//! The characters: the list with its search on the left, the selected
//! character's details on the right, and the commands above them, as
//! Windows lays out a list with details.

use cr_i18n::tr;
use cr_store::catalog::Entry;
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::ui;

pub fn view(shell: &Shell, context: &mut ViewContext<Shell>) -> View {
    if shell.install.is_none() {
        return no_installation(context);
    }
    let can_write = shell.can_write;
    let visible = shell.visible();
    // A character the search hides is not shown either.
    let selected = shell
        .selected_entry()
        .filter(|entry| visible.iter().any(|e| e.key == entry.key));
    let commands = CommandBar::new()
        .horizontal_alignment(HorizontalAlignment::Right)
        .grid_column(1)
        .collection_slot(
            CommandBarSlot::PrimaryCommands,
            [
                KeyedView::new(
                    "new",
                    ui::command(
                        &plain(tr("_New Character…")),
                        Symbol::Add,
                        can_write,
                        context.message(Message::NewCharacter),
                    ),
                ),
                KeyedView::new(
                    "import",
                    ui::command(
                        &plain(tr("_Import…")),
                        Symbol::Import,
                        can_write && !shell.busy,
                        context.message(Message::OpenPackage),
                    ),
                ),
                KeyedView::new(
                    "open",
                    ui::command(
                        &plain(tr("_Open Folder…")),
                        Symbol::OpenLocal,
                        !shell.busy,
                        context.message(Message::OpenFolder),
                    ),
                ),
            ],
        );
    let header = Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .margin(Thickness::new(36.0, 24.0, 24.0, 12.0))
        .children((
            ui::title(plain(tr("Characters"))).vertical_alignment(VerticalAlignment::Center),
            commands,
        ));

    let list: View = match &shell.characters {
        None => ProgressRing::new()
            .is_active(true)
            .horizontal_alignment(HorizontalAlignment::Center)
            .margin(24.0)
            .into(),
        Some(Err(e)) => ui::body(e.clone()).margin(12.0).into(),
        Some(Ok(entries)) if entries.is_empty() => {
            ui::caption(tr("No entries")).margin(12.0).into()
        }
        Some(Ok(_)) => ListView::new()
            .automation_name(plain(tr("Characters")))
            .selected_index(
                shell
                    .selected
                    .as_ref()
                    .and_then(|file| visible.iter().position(|e| &e.key == file)),
            )
            .on_selection_changed(context.callback(Message::Select))
            .collection_slot(
                ListViewSlot::Items,
                visible.iter().map(|entry| {
                    KeyedView::new(
                        entry.key.clone(),
                        ListViewItem::new().tag(entry.key.clone()).content(
                            StackPanel::new()
                                .spacing(2.0)
                                .margin(Thickness::new(0.0, 6.0, 0.0, 6.0))
                                .children((
                                    ui::body(entry.title.clone()),
                                    ui::caption(entry.subtitle.clone()),
                                )),
                        ),
                    )
                }),
            ),
    };
    let master = Grid::new()
        .rows([GridLength::Auto, GridLength::STAR])
        .row_spacing(8.0)
        .children((
            AutoSuggestBox::new()
                .text(shell.search.clone())
                .placeholder_text(plain(tr("Search")))
                .automation_name(plain(tr("Search")))
                .on_text_changed(context.callback(Message::Search)),
            // A new list for each search: the ListView keeps its selection
            // on the item rather than on the index it is given, so after
            // the search is cleared it would mark another character.
            Grid::new()
                .grid_row(1)
                .keyed_children([KeyedView::new(shell.search.clone(), list)]),
        ));

    let details = match selected {
        Some(entry) => details(
            entry,
            shell.portrait.clone(),
            can_write,
            shell.busy,
            context,
        ),
        None => ui::caption(tr("Nothing Selected"))
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
    };
    Grid::new()
        .rows([GridLength::Auto, GridLength::STAR])
        .children((
            header,
            Grid::new()
                .grid_row(1)
                .columns([GridLength::Pixel(320.0), GridLength::STAR])
                .column_spacing(16.0)
                .margin(Thickness::new(36.0, 0.0, 24.0, 24.0))
                .children((master, Border::new().grid_column(1).content(details))),
        ))
}

/// The selected character: its portrait, name and the groups of values
/// the GTK app shows, with the commands that act on it.
fn details(
    entry: &Entry,
    portrait: Option<EncodedImage>,
    can_write: bool,
    busy: bool,
    context: &mut ViewContext<Shell>,
) -> View {
    let commands = CommandBar::new().collection_slot(
        CommandBarSlot::PrimaryCommands,
        [
            KeyedView::new(
                "edit",
                ui::command(
                    &plain(tr("Edit Character")),
                    Symbol::Edit,
                    can_write,
                    context.message(Message::EditCharacter),
                ),
            ),
            KeyedView::new(
                "print",
                ui::command(
                    &plain(tr("Print")),
                    Symbol::Print,
                    !busy,
                    context.message(Message::Print),
                ),
            ),
            KeyedView::new(
                "delete",
                ui::command(
                    &plain(tr("Delete Character")),
                    Symbol::Delete,
                    can_write,
                    context.message(Message::AskDelete),
                ),
            ),
        ],
    );
    let portrait: View = match portrait {
        Some(image) => Image::new()
            .source_data(image)
            .stretch(Stretch::Uniform)
            .max_height(160.0)
            .horizontal_alignment(HorizontalAlignment::Left)
            .into(),
        None => View::empty(),
    };
    let groups = entry.groups.iter().enumerate().map(|(i, group)| {
        let rows = group.rows.iter().enumerate().map(|(r, (label, value))| {
            KeyedView::new(
                r,
                Grid::new()
                    .columns([GridLength::Pixel(180.0), GridLength::STAR])
                    .column_spacing(12.0)
                    .children((
                        ui::caption(label.clone()),
                        ui::body(value.clone()).grid_column(1),
                    )),
            )
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
    let text: View = if entry.text.is_empty() {
        View::empty()
    } else {
        ui::card(ui::body(entry.text.clone()))
    };
    ScrollViewer::new().content(
        StackPanel::new()
            .spacing(12.0)
            .margin(Thickness::new(0.0, 0.0, 12.0, 0.0))
            .children((
                Grid::new()
                    .columns([GridLength::STAR, GridLength::Auto])
                    .children((
                        StackPanel::new().spacing(2.0).children((
                            ui::subtitle(entry.title.clone()),
                            ui::caption(entry.subtitle.clone()),
                        )),
                        Border::new().grid_column(1).content(commands),
                    )),
                portrait,
                StackPanel::new().spacing(8.0).keyed_children(groups),
                text,
            )),
    )
}

/// A portrait of the original as a PNG, since WinUI's decoders do not
/// read every bitmap the original wrote.
pub fn portrait(path: &std::path::Path) -> Option<EncodedImage> {
    let image = cr_formats::bmp::decode(&std::fs::read(path).ok()?)?;
    let rgba = image::RgbaImage::from_raw(image.width, image.height, image.rgba)?;
    let mut png = std::io::Cursor::new(Vec::new());
    rgba.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(EncodedImage::new(png.into_inner()))
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
                .on_click(context.message(Message::OpenFolder))
                .content(plain(tr("_Open Folder…"))),
        )),
    )
}
