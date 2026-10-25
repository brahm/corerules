//! The library: a section's entries with the search and filters on the
//! left, the selected entry's details on the right and the commands above
//! them, as Windows lays out a list with details. What is listed and
//! which commands an entry has come from [`cr_app::library`], as in the
//! GNOME app.

use std::collections::HashMap;

use cr_app::action;
use cr_app::library::{self, Commands};
use cr_app::records;
use cr_i18n::tr;
use cr_store::catalog::{Entry, Frames, Section};
use cr_store::userdb::Filter as Origins;
use cr_store::userdb::parts::{OTHER_SECTION, OTHER_TYPES};
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell, available};
use crate::ui;

/// The pictures of the entry on the details, made once when it is
/// selected.
#[derive(Clone, Default)]
pub struct Pictures {
    /// An item's icon.
    pub icon: Option<EncodedImage>,
    pub picture: Option<EncodedImage>,
    /// A monster type's frames and the one shown, for Next Image.
    pub frames: Option<(Frames, u32)>,
    /// The picture is a monster's, with Full Screen.
    pub monster: bool,
}

impl Pictures {
    pub fn of(entry: &Entry) -> Self {
        let icon = entry
            .icon
            .as_ref()
            .and_then(cr_app::pictures::icon)
            .and_then(|i| ui::png(&i));
        if library::is_monster(&entry.key)
            && let Some(path) = entry.picture.as_deref()
        {
            return Self {
                icon,
                picture: cr_app::pictures::monster(path, entry.frames.as_ref())
                    .and_then(|i| ui::png(&i)),
                frames: entry.frames.clone().map(|f| (f, 0)),
                monster: true,
            };
        }
        Self {
            icon,
            picture: entry.picture.as_deref().and_then(ui::picture),
            frames: None,
            monster: false,
        }
    }

    /// Next Image: the type's next frame, back to the first after the
    /// last.
    pub fn next(&mut self) {
        let Some((frames, at)) = &mut self.frames else {
            return;
        };
        let next = cr_app::pictures::next_frame(frames, *at);
        if let Some(image) =
            cr_app::pictures::frame(&frames.page, frames.first + next).and_then(|i| ui::png(&i))
        {
            *at = next;
            self.picture = Some(image);
        }
    }
}

/// The GNOME app's sword, which Segoe Fluent Icons has no glyph for:
/// its 16-pixel drawing turned 45 degrees, as path data.
const SWORD: &str = "F1 M14.93,1.07 L14.43,3.69 L7.79,10.33 L5.67,8.21 L12.31,1.57 Z \
    M4.04,6.59 L9.41,11.96 A1,1 0 0 1 8,13.37 L2.63,8 A1,1 0 0 1 4.04,6.59 Z \
    M4.61,9.98 L6.02,11.39 L4.04,13.37 L2.63,11.96 Z \
    M1.64,12.38 A1.4,1.4 0 1 1 3.62,14.36 A1.4,1.4 0 1 1 1.64,12.38 Z";

/// The section's icon in the navigation, as the GNOME app gives each its
/// own.
pub fn icon(section: &str) -> View {
    if section == "weapons" {
        return PathIcon::new().data(SWORD).into();
    }
    SymbolIcon::new().symbol(symbol(section)).into()
}

fn symbol(section: &str) -> Symbol {
    match section {
        library::CHARACTERS => Symbol::People,
        "races" => Symbol::Contact,
        "classes" => Symbol::Manage,
        "kits" => Symbol::ContactInfo,
        "armor" => Symbol::Permissions,
        "equipment" => Symbol::Shop,
        "magic-items" => Symbol::SolidStar,
        "nonweapon" => Symbol::List,
        OTHER_SECTION => Symbol::AllApps,
        "wizard-spells" | "priest-spells" => Symbol::Highlight,
        "psionics" => Symbol::Emoji2,
        "monsters" => Symbol::Emoji,
        "monster-types" => Symbol::ViewAll,
        "traps" => Symbol::Important,
        "encounters" => Symbol::MapPin,
        "templates" => Symbol::Bullets,
        "tables" => Symbol::ShowResults,
        "treasure" => Symbol::Bookmarks,
        "treasure-tables" => Symbol::Calculator,
        "books" => Symbol::Library,
        _ => Symbol::Document,
    }
}

pub fn view(shell: &Shell, context: &mut ViewContext<Shell>) -> View {
    if shell.install.is_none() {
        return no_installation(context);
    }
    let Some(section) = shell.current_section() else {
        return ProgressRing::new()
            .is_active(true)
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .into();
    };
    let visible = shell.visible();
    // An entry the filters hide is not shown either.
    let selected = shell
        .selected_entry()
        .filter(|entry| visible.iter().any(|e| e.key == entry.key));

    let mut commands = Vec::new();
    if let Some((new, tooltip)) = library::new_command(section.id)
        && available(new)
    {
        commands.push(KeyedView::new(
            "new",
            ui::command(
                &tooltip,
                Symbol::Add,
                shell.can_write,
                context.message(Message::Act(new)),
            ),
        ));
    }
    commands.push(KeyedView::new(
        "import",
        ui::command(
            &plain(tr("_Import…")),
            Symbol::Import,
            shell.can_write && !shell.busy,
            context.message(Message::Act(action::IMPORT)),
        ),
    ));
    commands.push(KeyedView::new(
        "open",
        ui::command(
            &plain(tr("_Open Folder…")),
            Symbol::OpenLocal,
            !shell.busy,
            context.message(Message::Act(action::OPEN)),
        ),
    ));
    let header = Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .margin(Thickness::new(36.0, 24.0, 24.0, 12.0))
        .children((
            ui::title(section.title).vertical_alignment(VerticalAlignment::Center),
            CommandBar::new()
                .horizontal_alignment(HorizontalAlignment::Right)
                .grid_column(1)
                .collection_slot(CommandBarSlot::PrimaryCommands, commands),
        ));

    let master = Grid::new()
        .rows([GridLength::Auto, GridLength::Auto, GridLength::STAR])
        .row_spacing(8.0)
        .children((
            AutoSuggestBox::new()
                .text(shell.filter.search.clone())
                .placeholder_text(plain(tr("Search")))
                .automation_name(plain(tr("Search")))
                .on_text_changed(context.callback(Message::Search)),
            Border::new()
                .grid_row(1)
                .content(filters(shell, section, context)),
            // A new list for each section and filter: the ListView keeps
            // its selection on the item rather than on the index it is
            // given, so after the filter changes it would mark another
            // entry.
            Grid::new().grid_row(2).keyed_children([KeyedView::new(
                list_key(shell, section),
                list(shell, section, &visible, context),
            )]),
        ));

    let details =
        match selected {
            Some(entry) => details(shell, entry, context),
            None => {
                let centered = StackPanel::new()
                    .spacing(6.0)
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .vertical_alignment(VerticalAlignment::Center);
                match &section.error {
                    Some(error) => centered
                        .children((ui::subtitle(tr("Could Not Read")), ui::body(error.clone()))),
                    None if section.entries.is_empty() => centered
                        .children((ui::caption(tr!("No {section}", section = section.title)),)),
                    None => centered.children((ui::caption(tr("Nothing Selected")),)),
                }
            }
        };
    let read_only: View = if shell.install.as_ref().is_some_and(|i| i.is_core20()) {
        InfoBar::new()
            .severity(InfoBarSeverity::Informational)
            .title(tr("Core Rules 2.0 is read-only"))
            .message(tr(
                "Open an Expansion installation and import this content with Import Core Rules 2.0…",
            ))
            .is_open(true)
            .is_closable(false)
            .margin(Thickness::new(36.0, 0.0, 24.0, 12.0))
            .into()
    } else {
        View::empty()
    };
    Grid::new()
        .rows([GridLength::Auto, GridLength::Auto, GridLength::STAR])
        .children((
            header,
            Border::new().grid_row(1).content(read_only),
            Grid::new()
                .grid_row(2)
                .columns([GridLength::Pixel(320.0), GridLength::STAR])
                .column_spacing(16.0)
                .margin(Thickness::new(36.0, 0.0, 24.0, 24.0))
                .children((master, Border::new().grid_column(1).content(details))),
        ))
}

/// The filters the section offers: standard or custom records, a party,
/// a part type, Magic Only.
fn filters(shell: &Shell, section: &Section, context: &mut ViewContext<Shell>) -> View {
    let mut filters = Vec::new();
    if records::has_origins(section.id) {
        filters.push(KeyedView::new(
            "origins",
            ComboBox::new()
                .items_source(Origins::NAMES.map(tr))
                .selected_index(Some(match shell.filter.origins {
                    Origins::All => 0,
                    Origins::Standard => 1,
                    Origins::Custom => 2,
                }))
                .automation_name(tr("Show Standard or Custom Records"))
                .on_selection_changed(context.callback(Message::Origins)),
        ));
    }
    if section.id == library::CHARACTERS && shell.parties.len() > 1 {
        filters.push(KeyedView::new(
            "party",
            ComboBox::new()
                .items_source(
                    shell
                        .parties
                        .iter()
                        .map(|p| library::party_label(p.as_deref()).to_owned()),
                )
                .selected_index(Some(library::party_position(
                    &shell.parties,
                    shell.filter.party.as_deref(),
                )))
                .automation_name(tr("Show a Party"))
                .on_selection_changed(context.callback(Message::Party)),
        ));
    }
    if section.id == OTHER_SECTION {
        filters.push(KeyedView::new(
            "type",
            ComboBox::new()
                .items_source(
                    std::iter::once(tr("All Types"))
                        .chain(OTHER_TYPES.iter().map(|(_, _, name)| tr(name))),
                )
                .selected_index(Some(shell.filter.part_type.map_or(0, |t| {
                    OTHER_TYPES
                        .iter()
                        .position(|(id, ..)| *id == t)
                        .map_or(0, |i| i + 1)
                })))
                .automation_name(tr("Show a Part Type"))
                .on_selection_changed(context.callback(Message::PartType)),
        ));
    }
    let magic: Option<View> = library::is_parts(section.id).then(|| {
        ToggleButton::new()
            .is_checked(shell.filter.magic_only)
            .on_is_checked_changed(context.callback(Message::MagicOnly))
            .content(tr("Magic Only"))
    });
    let row = |filters: Vec<KeyedView>| {
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .keyed_children(filters)
    };
    match magic {
        // Two lists and Magic Only do not fit beside the list's width.
        Some(magic) if filters.len() > 1 => StackPanel::new()
            .spacing(8.0)
            .children((row(filters), magic)),
        Some(magic) => {
            filters.push(KeyedView::new("magic", magic));
            row(filters)
        }
        None if filters.is_empty() => View::empty(),
        None => row(filters),
    }
}

/// What the list depends on, so that a change makes it anew: a new
/// catalog too, whose rows may have new keys (a converted character's
/// file) and whose items would lose the selection.
fn list_key(shell: &Shell, section: &Section) -> String {
    let filter = &shell.filter;
    format!(
        "{}\u{1}{}\u{1}{}\u{1}{:?}\u{1}{:?}\u{1}{:?}\u{1}{}",
        shell.loads,
        section.id,
        filter.search,
        filter.origins,
        filter.party,
        filter.part_type,
        filter.magic_only
    )
}

fn list(
    shell: &Shell,
    section: &Section,
    visible: &[&Entry],
    context: &mut ViewContext<Shell>,
) -> View {
    if section.error.is_some() || section.entries.is_empty() {
        return View::empty();
    }
    if visible.is_empty() {
        return ui::caption(tr("No Results")).margin(12.0).into();
    }
    // The standard data repeats some records (Garnet, Pearl and Spinel
    // in Gems and Art), and Reactor needs each row's key once.
    let mut seen: HashMap<&str, usize> = HashMap::new();
    ListView::new()
        .automation_name(section.title)
        .selected_index(
            shell
                .selected_entry()
                .and_then(|chosen| visible.iter().position(|e| std::ptr::eq(*e, chosen))),
        )
        .on_selection_changed(context.callback(Message::Select))
        .collection_slot(
            ListViewSlot::Items,
            visible.iter().map(|entry| {
                let text: View = if entry.subtitle.is_empty() {
                    ui::body(entry.title.clone()).into()
                } else {
                    StackPanel::new().spacing(2.0).children((
                        ui::body(entry.title.clone()),
                        ui::caption(entry.subtitle.clone()),
                    ))
                };
                let repeat = seen.entry(entry.key.as_str()).or_default();
                let key = match *repeat {
                    0 => entry.key.clone(),
                    n => format!("{}\u{1}{n}", entry.key),
                };
                *repeat += 1;
                KeyedView::new(
                    key,
                    ListViewItem::new().tag(entry.key.clone()).content(
                        Border::new()
                            .padding(Thickness::new(0.0, 6.0, 0.0, 6.0))
                            .content(text),
                    ),
                )
            }),
        )
}

/// The selected entry: its icon, title and picture, the groups of values
/// the GNOME app shows and its description, with the commands that act
/// on it.
fn details(shell: &Shell, entry: &Entry, context: &mut ViewContext<Shell>) -> View {
    let commands = Commands::of(entry, shell.can_write);
    let busy = shell.busy;
    let mut buttons = Vec::new();
    let mut add = |key: &'static str, label: &str, symbol: Symbol, enabled: bool, message| {
        buttons.push(KeyedView::new(
            key,
            ui::command(label, symbol, enabled, context.message(message)),
        ));
    };
    if commands.read {
        add(
            "read",
            &plain(tr("_Read Book")),
            Symbol::Read,
            !busy,
            Message::Act(action::OPEN_BOOK),
        );
    }
    if commands.sheet {
        add(
            "sheet",
            tr("Character Sheet"),
            Symbol::Document,
            !busy,
            Message::ShowSheet,
        );
    }
    if commands.edit && library::is_character(&entry.key) {
        add(
            "edit",
            tr("Edit Character"),
            Symbol::Edit,
            !busy,
            Message::EditCharacter,
        );
    }
    if commands.view && !commands.monster {
        add(
            "view",
            tr("View"),
            Symbol::Preview,
            !busy,
            Message::ViewPage,
        );
    }
    // The Monsters screen's View and Print: a shipped monster's help
    // topic, a monster of the user database as it prints.
    if commands.monster {
        add(
            "view",
            tr("View"),
            Symbol::Preview,
            !busy,
            Message::MonsterHelp(false),
        );
        add(
            "print",
            tr("Print"),
            Symbol::Print,
            !busy,
            Message::MonsterHelp(true),
        );
    }
    if commands.sheet || (commands.print && !commands.monster) {
        add(
            "print",
            tr("Print"),
            Symbol::Print,
            !busy,
            Message::Act(action::SHEET_PRINT),
        );
    }
    if commands.sheet {
        add(
            "pdf",
            &plain(tr("Save as _PDF…")),
            Symbol::Save,
            !busy,
            Message::Act(action::SHEET_PDF),
        );
    }
    if commands.to_character {
        add(
            "to-character",
            tr("Convert to Character"),
            Symbol::Contact,
            !busy,
            Message::ToCharacter,
        );
    }
    if commands.to_npc {
        add(
            "to-npc",
            tr("Convert to NPC"),
            Symbol::OtherUser,
            !busy,
            Message::AskToNpc,
        );
    }
    if commands.delete {
        add(
            "delete",
            tr("Delete Character"),
            Symbol::Delete,
            !busy,
            Message::AskDelete,
        );
    }
    // What a right-click on its row shows in the original.
    add(
        "help",
        tr("Help"),
        Symbol::Help,
        true,
        Message::Act(action::HELP_CONTROL),
    );
    let pictures = &shell.pictures;
    let heading = StackPanel::new()
        .spacing(2.0)
        .vertical_alignment(VerticalAlignment::Center)
        .children((
            ui::subtitle(entry.title.clone()),
            ui::caption(entry.subtitle.clone()),
        ));
    let heading: View = match &pictures.icon {
        Some(icon) => StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(18.0)
            .children((
                Image::new()
                    .source_data(icon.clone())
                    .width(f64::from(cr_store::pictures::ICON_SIZE))
                    .height(f64::from(cr_store::pictures::ICON_SIZE))
                    .automation_name(tr("Icon")),
                heading,
            )),
        None => heading,
    };
    let picture: View = match &pictures.picture {
        Some(image) => {
            let image = Image::new()
                .source_data(image.clone())
                .stretch(Stretch::Uniform)
                .max_height(240.0)
                .horizontal_alignment(HorizontalAlignment::Left);
            if pictures.monster {
                let mut buttons = Vec::new();
                if pictures.frames.is_some() {
                    buttons.push(KeyedView::new(
                        "next",
                        ui::icon_button(
                            tr("Next Image"),
                            Symbol::Forward,
                            Message::NextImage,
                            context,
                        ),
                    ));
                }
                buttons.push(KeyedView::new(
                    "full",
                    ui::icon_button(
                        tr("Full Screen"),
                        Symbol::FullScreen,
                        Message::FullScreen(true),
                        context,
                    ),
                ));
                StackPanel::new().spacing(6.0).children((
                    StackPanel::new()
                        .orientation(Orientation::Horizontal)
                        .spacing(6.0)
                        .keyed_children(buttons),
                    image,
                ))
            } else {
                image.into()
            }
        }
        None => View::empty(),
    };
    let groups = entry
        .groups
        .iter()
        .enumerate()
        .filter(|(_, g)| !g.rows.is_empty())
        .map(|(i, group)| {
            let rows = group.rows.iter().enumerate().map(|(r, (label, value))| {
                // A row without a value is a list item, not a property.
                let row: View = if value.is_empty() {
                    ui::body(label.clone())
                        .is_text_selection_enabled(true)
                        .into()
                } else {
                    Grid::new()
                        .columns([GridLength::Pixel(180.0), GridLength::STAR])
                        .column_spacing(12.0)
                        .children((
                            ui::caption(label.clone()),
                            ui::body(value.clone())
                                .is_text_selection_enabled(true)
                                .grid_column(1),
                        ))
                };
                KeyedView::new(r, row)
            });
            let body = StackPanel::new().spacing(6.0).keyed_children(rows);
            let content: View = if group.title.is_empty() {
                body
            } else {
                StackPanel::new()
                    .spacing(8.0)
                    .children((ui::strong(group.title.clone()), body))
            };
            KeyedView::new(i, ui::card(content))
        });
    let text = library::clean_text(&entry.text);
    let text: View = if text.is_empty() {
        View::empty()
    } else {
        ui::card(StackPanel::new().spacing(8.0).children((
            ui::strong(tr("Description")),
            ui::body(text).is_text_selection_enabled(true),
        )))
    };
    ScrollViewer::new().content(
        StackPanel::new()
            .spacing(12.0)
            .margin(Thickness::new(0.0, 0.0, 12.0, 0.0))
            .children((
                heading,
                // Under the heading, where the commands that do not fit
                // the pane's width go to its More menu.
                CommandBar::new()
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .collection_slot(CommandBarSlot::PrimaryCommands, buttons),
                picture,
                StackPanel::new().spacing(8.0).keyed_children(groups),
                text,
            )),
    )
}

/// Monster Full Screen: the picture in the frame of the original's
/// dialog when its background is there, over the whole window, until a
/// click.
pub fn full_screen(shell: &Shell, context: &mut ViewContext<Shell>) -> View {
    let Some(picture) = shell.pictures.picture.clone() else {
        return View::empty();
    };
    let (width, height, left, top, side) = cr_app::pictures::FULL_SCREEN;
    let content: View = match &shell.background {
        Some(background) => Viewbox::new()
            .stretch(Stretch::Uniform)
            .slots([SlotView::new(
                ViewboxSlot::Child,
                Canvas::new().width(width).height(height).children((
                    Image::new()
                        .source_data(background.clone())
                        .width(width)
                        .height(height)
                        .stretch(Stretch::Fill),
                    Image::new()
                        .source_data(picture)
                        .width(side)
                        .height(side)
                        .stretch(Stretch::Uniform)
                        .canvas_left(left)
                        .canvas_top(top),
                )),
            )]),
        None => Image::new()
            .source_data(picture)
            .stretch(Stretch::Uniform)
            .into(),
    };
    Border::new()
        .background(Color::rgb(0, 0, 0))
        .grid_row_span(2)
        .automation_name(tr("Full Screen"))
        .on_pointer_pressed(context.callback(|_: PointerEventInfo| Message::FullScreen(false)))
        .content(content)
}

/// No installation was found: where the app looks, and Open Folder….
fn no_installation(context: &mut ViewContext<Shell>) -> View {
    ui::page(
        StackPanel::new().spacing(12.0).children((
            ui::title(tr("No Core Rules Installation")),
            ui::body(tr!(
                "Put the data package made by Extract Data from CD… in {folder}, or open the \
                 folder where AD&D Core Rules 2.0 is installed.",
                folder = cr_store::DATA_PACKAGE_SHOWN
            )),
            Button::new()
                .style(ButtonStyle::Accent)
                .on_click(context.message(Message::Act(action::OPEN)))
                .content(plain(tr("_Open Folder…"))),
        )),
    )
}
