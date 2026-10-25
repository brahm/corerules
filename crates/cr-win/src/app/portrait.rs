//! Choose Portrait: a character's portrait among the installation's, the
//! race's own first, or none, or one made from a picture of the user's
//! (BMP, JPEG, GIF or PNG), as the GNOME app's dialog offers them.

use std::path::Path;

use cr_i18n::tr;
use cr_store::Installation;
use cr_store::pictures::{Kind, PORTRAIT_HEIGHT, PORTRAIT_WIDTH};
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::ui;

/// A portrait offered.
#[derive(Clone)]
pub struct Tile {
    pub file: String,
    /// One the race offers.
    pub first: bool,
    pub image: Option<EncodedImage>,
}

/// The dialog while it is open: the race's name, and the portraits once
/// they are read, or why they could not be.
pub struct Chooser {
    pub race: String,
    pub tiles: Option<Result<Vec<Tile>, String>>,
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

/// The portraits of `install`, the race's first, read in the background.
pub fn list(install: &Installation, race: &str) -> Result<Vec<Tile>, String> {
    let choices = install
        .picture_choices(Kind::Portrait, race)
        .map_err(|e| e.to_string())?;
    Ok(choices
        .into_iter()
        .map(|c| Tile {
            image: ui::picture(&c.path),
            file: c.file,
            first: c.first,
        })
        .collect())
}

/// Makes a portrait from the user's picture at `path` and gives its file
/// name.
pub fn add(install: &Installation, path: &Path) -> Result<String, String> {
    let made = install
        .add_picture(Kind::Portrait, path)
        .map_err(|e| e.to_string())?;
    Ok(made
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default())
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

/// The tiles of a group, a button each with its file under it: the dialog
/// cannot hold tooltips, which windows-reactor refuses inside one.
fn tiles(tiles: &[&Tile], context: &mut ViewContext<Shell>) -> View {
    let height = 96.0;
    let width = height * f64::from(PORTRAIT_WIDTH) / f64::from(PORTRAIT_HEIGHT);
    let buttons: Vec<KeyedView> = tiles
        .iter()
        .map(|tile| {
            KeyedView::new(
                tile.file.clone(),
                Button::new()
                    .margin(Thickness::new(0.0, 0.0, 6.0, 6.0))
                    .automation_name(tile.file.clone())
                    .on_click(context.message(Message::PortraitChosen(Some(tile.file.clone()))))
                    .content(
                        StackPanel::new().spacing(4.0).children((
                            thumbnail(tile.image.as_ref(), height),
                            ui::caption(tile.file.clone())
                                .text_wrapping(TextWrapping::NoWrap)
                                .text_trimming(TextTrimming::CharacterEllipsis)
                                .max_width(width)
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

/// Choose Portrait as a dialog.
pub fn dialog(chooser: &Chooser, context: &mut ViewContext<Shell>) -> View {
    let content: View = match &chooser.tiles {
        None => ProgressRing::new().is_active(true).into(),
        Some(Err(e)) => InfoBar::new()
            .severity(InfoBarSeverity::Error)
            .title(tr("Could Not List the Portraits"))
            .message(e.clone())
            .is_open(true)
            .is_closable(false)
            .into(),
        Some(Ok(all)) if all.is_empty() => StackPanel::new().spacing(4.0).children((
            ui::strong(tr("No Portraits")),
            ui::caption(tr("Choose a picture of yours to make one")),
        )),
        Some(Ok(all)) => {
            let (first, rest): (Vec<&Tile>, Vec<&Tile>) = all.iter().partition(|t| t.first);
            let groups = if chooser.race.is_empty() {
                [(String::new(), first), (tr("Portraits").to_owned(), rest)]
            } else {
                [
                    (
                        tr!("Portraits for {race}", race = chooser.race.clone()),
                        first,
                    ),
                    (tr("Other Portraits").to_owned(), rest),
                ]
            };
            let shown: Vec<KeyedView> = groups
                .into_iter()
                .filter(|(_, group)| !group.is_empty())
                .map(|(title, group)| {
                    let heading: View = if title.is_empty() {
                        View::empty()
                    } else {
                        ui::strong(title.clone()).into()
                    };
                    KeyedView::new(
                        title,
                        StackPanel::new()
                            .spacing(6.0)
                            .children((heading, tiles(&group, context))),
                    )
                })
                .collect();
            StackPanel::new().spacing(12.0).keyed_children(shown)
        }
    };
    ContentDialog::new()
        .title(tr("Choose Portrait"))
        .primary_button_text(plain(tr("Choose _File…")))
        .close_button_text(plain(tr("_Cancel")))
        .is_open(true)
        .on_closed(context.callback(|result| match result {
            ContentDialogResult::Primary => Message::PortraitFile,
            _ => Message::ClosePortraits,
        }))
        .content(
            ScrollViewer::new()
                .max_height(520.0)
                .min_width(480.0)
                .content(content),
        )
}
