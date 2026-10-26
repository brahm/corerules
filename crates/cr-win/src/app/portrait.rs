//! Choose Portrait and Choose Picture: a character's portrait among the
//! installation's, the race's own first; a monster type's or a race's
//! picture; an item's icon, with a search over the items that have it;
//! or none, or one made from a picture of the user's (BMP, JPEG, GIF or
//! PNG), as the GNOME app's dialogs offer them.

use std::path::Path;

use cr_app::pictures::{icon_group_title, picture_line};
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::pictures::{ICON_SIZE, Kind, PORTRAIT_HEIGHT, PORTRAIT_WIDTH, stored_path};
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::ui;

/// What the dialog chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choosing {
    /// The portrait of the character on New Character or Edit Character.
    Portrait,
    /// A picture of `kind` for the record editor's field with this index.
    Picture { field: usize, kind: Kind },
    /// An item's icon for the record editor's field with this index.
    Icon { field: usize },
}

impl Choosing {
    fn kind(self) -> Kind {
        match self {
            Self::Portrait => Kind::Portrait,
            Self::Picture { kind, .. } => kind,
            Self::Icon { .. } => Kind::Item,
        }
    }
}

/// A picture offered.
#[derive(Clone)]
pub struct Tile {
    /// What the tile is called under it.
    pub file: String,
    /// What is kept when it is chosen: a portrait's file, a picture's
    /// stored path, an icon's number or file.
    pub stored: String,
    /// The heading of its group.
    pub group: String,
    /// What the search looks in, in lower case: an icon's name and the
    /// items that have it.
    pub searched: String,
    pub image: Option<EncodedImage>,
}

/// The dialog while it is open: what it chooses, the pictures once they
/// are read, or why they could not be, and the icons' search.
pub struct Chooser {
    pub choosing: Choosing,
    pub tiles: Option<Result<Vec<Tile>, String>>,
    pub search: String,
}

impl Chooser {
    pub fn new(choosing: Choosing) -> Self {
        Self {
            choosing,
            tiles: None,
            search: String::new(),
        }
    }
}

/// The portrait a page shows, read again only when it changes.
pub struct Shown {
    pub stored: [String; 2],
    /// Its file, or None, or that the installation lacks it.
    pub subtitle: String,
    pub image: Option<EncodedImage>,
}

impl Shown {
    /// The portrait a character keeps, `stored`.
    pub fn read(install: Option<&Installation>, stored: &[String; 2]) -> Self {
        let path = install
            .filter(|_| !stored[1].is_empty())
            .and_then(|i| i.portrait_path(stored));
        let subtitle = match stored[1].as_str() {
            "" => tr("None").to_owned(),
            file if path.is_none() => tr!("{file} (not in this installation)", file = file),
            file => file.to_owned(),
        };
        Self {
            stored: stored.clone(),
            subtitle,
            image: path.as_deref().and_then(ui::picture),
        }
    }
}

/// The pictures Choose File… offers.
pub fn filters() -> Vec<(String, String)> {
    vec![(
        tr("Pictures (BMP, JPEG, GIF, PNG)").to_owned(),
        "*.bmp;*.jpg;*.jpeg;*.gif;*.png".to_owned(),
    )]
}

/// The pictures `choosing` offers in `install`, the race's portraits or
/// the user's pictures first, read in the background.
pub fn list(install: &Installation, choosing: Choosing, race: &str) -> Result<Vec<Tile>, String> {
    if choosing.kind() == Kind::Item {
        return icons(install);
    }
    let kind = choosing.kind();
    let choices = install
        .picture_choices(kind, race)
        .map_err(|e| e.to_string())?;
    let groups = match kind {
        Kind::Portrait if race.is_empty() => [String::new(), tr("Portraits").to_owned()],
        Kind::Portrait => [
            tr!("Portraits for {race}", race = race),
            tr("Other Portraits").to_owned(),
        ],
        Kind::Monster | Kind::Item => [
            tr("Your Pictures").to_owned(),
            tr("Monster Pictures").to_owned(),
        ],
    };
    let (first, rest): (Vec<_>, Vec<_>) = choices.into_iter().partition(|c| c.first);
    Ok(first
        .into_iter()
        .map(|c| (c, &groups[0]))
        .chain(rest.into_iter().map(|c| (c, &groups[1])))
        .map(|(c, group)| Tile {
            image: ui::thumbnail(&c.path, tile_side(kind)),
            stored: match choosing {
                Choosing::Portrait => c.file.clone(),
                _ => stored_path(install.root(), &c.path).unwrap_or_default(),
            },
            searched: c.file.to_lowercase(),
            file: c.file,
            group: group.clone(),
        })
        .collect())
}

/// The side of a tile's picture: portraits at their shipped size, monster
/// pictures at half theirs, icons at theirs.
fn tile_side(kind: Kind) -> u32 {
    match kind {
        Kind::Portrait => PORTRAIT_HEIGHT.max(PORTRAIT_WIDTH),
        Kind::Monster => kind.size().1 / 2,
        Kind::Item => ICON_SIZE,
    }
}

/// The icons an item may have, by the groups of their items.
fn icons(install: &Installation) -> Result<Vec<Tile>, String> {
    let choices = install.icon_choices().map_err(|e| e.to_string())?;
    // The strips the icons are cut from, each decoded once.
    let mut strips: Vec<(std::path::PathBuf, Option<cr_formats::bmp::Image>)> = Vec::new();
    Ok(choices
        .into_iter()
        .map(|c| {
            let name = cr_app::pictures::icon_name(&c.stored);
            let image = match c.icon.left {
                None => ui::picture(&c.icon.path),
                Some(left) => {
                    let at = match strips.iter().position(|(p, _)| *p == c.icon.path) {
                        Some(at) => at,
                        None => {
                            let strip = cr_app::pictures::load(&c.icon.path);
                            strips.push((c.icon.path.clone(), strip));
                            strips.len() - 1
                        }
                    };
                    strips[at]
                        .1
                        .as_ref()
                        .and_then(|strip| cr_app::pictures::icon_in(strip, left))
                        .and_then(|i| ui::png(&i))
                }
            };
            Tile {
                file: c.names.first().cloned().unwrap_or_else(|| name.clone()),
                searched: std::iter::once(name)
                    .chain(c.names.iter().cloned())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .to_lowercase(),
                stored: c.stored,
                group: icon_group_title(c.group).to_owned(),
                image,
            }
        })
        .collect())
}

/// Makes a picture for `choosing` from the user's picture at `path`, and
/// gives what is kept for it.
pub fn add(install: &Installation, choosing: Choosing, path: &Path) -> Result<String, String> {
    if let Choosing::Icon { .. } = choosing {
        return install.add_icon(path).map_err(|e| e.to_string());
    }
    let made = install
        .add_picture(choosing.kind(), path)
        .map_err(|e| e.to_string())?;
    Ok(match choosing {
        Choosing::Portrait => made
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default(),
        _ => stored_path(install.root(), &made).unwrap_or_default(),
    })
}

/// What a record's picture field shows for what it keeps: the line under
/// its label and the picture, `icon` for an item's.
pub fn look(
    install: Option<&Installation>,
    kind: Kind,
    icon: bool,
    stored: &str,
) -> (String, Option<EncodedImage>) {
    let image = if icon {
        install
            .and_then(|i| i.icon(stored.trim()))
            .and_then(|i| cr_app::pictures::icon(&i))
            .and_then(|i| ui::png(&i))
    } else {
        install
            .and_then(|i| i.picture_path(stored, kind))
            .as_deref()
            .and_then(ui::picture)
    };
    (picture_line(stored, icon, image.is_some()), image)
}

/// A portrait's thumbnail, or a placeholder.
pub fn thumbnail(image: Option<&EncodedImage>, height: f64) -> View {
    let width = height * f64::from(PORTRAIT_WIDTH) / f64::from(PORTRAIT_HEIGHT);
    match image {
        Some(image) => Image::new()
            .source_data(image.clone())
            .stretch(Stretch::Uniform)
            .width(width)
            .height(height)
            .automation_name(tr("Portrait"))
            .into(),
        None => Viewbox::new()
            .width(width)
            .height(height / 2.0)
            .margin(Thickness::new(0.0, height / 4.0, 0.0, height / 4.0))
            .opacity(0.5)
            .stretch(Stretch::Uniform)
            .slots([SlotView::new(
                ViewboxSlot::Child,
                SymbolIcon::new().symbol(Symbol::Contact),
            )]),
    }
}

/// The Portrait row: the one chosen, Choose… and None.
pub fn row(shown: &Shown, context: &mut ViewContext<Shell>) -> View {
    let clear: View = if shown.stored[1].is_empty() {
        View::empty()
    } else {
        ui::icon_button(
            tr("No Portrait"),
            Symbol::Clear,
            Message::PortraitChosen(None),
            context,
        )
    };
    ui::card(
        Grid::new()
            .column_spacing(16.0)
            .columns([GridLength::Auto, GridLength::STAR, GridLength::Auto])
            .children((
                thumbnail(shown.image.as_ref(), 48.0),
                Border::new()
                    .grid_column(1)
                    .vertical_alignment(VerticalAlignment::Center)
                    .content(StackPanel::new().spacing(2.0).children((
                        ui::body(tr("Portrait")),
                        ui::caption(shown.subtitle.clone()),
                    ))),
                StackPanel::new()
                    .grid_column(2)
                    .orientation(Orientation::Horizontal)
                    .spacing(8.0)
                    .vertical_alignment(VerticalAlignment::Center)
                    .children((
                        clear,
                        Button::new()
                            .on_click(context.message(Message::ChoosePortrait))
                            .content(plain(tr("_Choose…"))),
                    )),
            )),
    )
}

/// The tiles of a group, a button each with its name under it: the
/// dialog cannot hold tooltips, which windows-reactor refuses inside one.
fn tiles(tiles: &[&Tile], kind: Kind, context: &mut ViewContext<Shell>) -> View {
    // Portraits at their shipped size, monster pictures at half theirs,
    // icons at theirs, whatever the bitmap's.
    let (width, height) = match kind {
        Kind::Portrait => (
            96.0 * f64::from(PORTRAIT_WIDTH) / f64::from(PORTRAIT_HEIGHT),
            96.0,
        ),
        Kind::Monster => {
            let side = f64::from(kind.size().1 / 2);
            (side, side)
        }
        Kind::Item => (f64::from(ICON_SIZE), f64::from(ICON_SIZE)),
    };
    let label_width = width.max(96.0);
    let buttons: Vec<KeyedView> = tiles
        .iter()
        .map(|tile| {
            let picture: View = match (kind, &tile.image) {
                (Kind::Portrait, image) => thumbnail(image.as_ref(), height),
                (_, Some(image)) => Image::new()
                    .source_data(image.clone())
                    .stretch(Stretch::Uniform)
                    .width(width)
                    .height(height)
                    .into(),
                (_, None) => Border::new().width(width).height(height).into(),
            };
            KeyedView::new(
                tile.stored.clone(),
                Button::new()
                    .margin(Thickness::new(0.0, 0.0, 6.0, 6.0))
                    .automation_name(tile.file.clone())
                    .on_click(context.message(Message::PortraitChosen(Some(tile.stored.clone()))))
                    .content(
                        StackPanel::new().spacing(4.0).children((
                            Border::new()
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .content(picture),
                            ui::caption(tile.file.clone())
                                .text_wrapping(TextWrapping::NoWrap)
                                .text_trimming(TextTrimming::CharacterEllipsis)
                                .max_width(label_width)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )),
                    ),
            )
        })
        .collect();
    VariableSizedWrapGrid::new()
        .orientation(Orientation::Horizontal)
        .keyed_children(buttons)
}

/// Choose Portrait or Choose Picture as a dialog.
pub fn dialog(chooser: &Chooser, context: &mut ViewContext<Shell>) -> View {
    let kind = chooser.choosing.kind();
    let portrait = kind == Kind::Portrait;
    let content: View = match &chooser.tiles {
        None => ProgressRing::new().is_active(true).into(),
        Some(Err(e)) => InfoBar::new()
            .severity(InfoBarSeverity::Error)
            .title(if portrait {
                tr("Could Not List the Portraits")
            } else {
                tr("Could Not List the Pictures")
            })
            .message(e.clone())
            .is_open(true)
            .is_closable(false)
            .into(),
        Some(Ok(all)) if all.is_empty() => StackPanel::new().spacing(4.0).children((
            ui::strong(if portrait {
                tr("No Portraits")
            } else {
                tr("No Pictures")
            }),
            ui::caption(tr("Choose a picture of yours to make one")),
        )),
        Some(Ok(all)) => {
            let needle = chooser.search.trim().to_lowercase();
            let mut groups: Vec<(&str, Vec<&Tile>)> = Vec::new();
            for tile in all.iter().filter(|t| t.searched.contains(&needle)) {
                match groups.last_mut() {
                    Some((group, members)) if *group == tile.group => members.push(tile),
                    _ => groups.push((&tile.group, vec![tile])),
                }
            }
            let shown: Vec<KeyedView> = groups
                .into_iter()
                .map(|(title, group)| {
                    let heading: View = if title.is_empty() {
                        View::empty()
                    } else {
                        ui::strong(title.to_owned()).into()
                    };
                    KeyedView::new(
                        format!("{title}\u{1}{needle}"),
                        StackPanel::new()
                            .spacing(6.0)
                            .children((heading, tiles(&group, kind, context))),
                    )
                })
                .collect();
            let shown: View = if shown.is_empty() {
                ui::caption(tr("No Results")).into()
            } else {
                StackPanel::new().spacing(12.0).keyed_children(shown)
            };
            // An item's icons are many: a search by the items that have
            // them, as in the GNOME app.
            if matches!(chooser.choosing, Choosing::Icon { .. }) {
                StackPanel::new().spacing(12.0).children((
                    TextBox::new()
                        .automation_name(tr("Search"))
                        .placeholder_text(tr("Search"))
                        .text(chooser.search.clone())
                        .on_text_changed(context.callback(Message::PictureSearch)),
                    shown,
                ))
            } else {
                shown
            }
        }
    };
    let choosing = chooser.choosing;
    ContentDialog::new()
        .title(if portrait {
            tr("Choose Portrait")
        } else {
            tr("Choose Picture")
        })
        .primary_button_text(plain(tr("Choose _File…")))
        .close_button_text(plain(tr("_Cancel")))
        .is_open(true)
        .on_closed(context.callback(move |result| match result {
            ContentDialogResult::Primary => Message::PortraitFile(choosing),
            _ => Message::ClosePortraits,
        }))
        .content(
            ScrollViewer::new()
                .max_height(520.0)
                .min_width(480.0)
                .content(content),
        )
}
