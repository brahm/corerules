//! Export…, New Package… and Edit Package…: the user's records and
//! characters to take into a package, checked one by one, and the
//! package editor, its items by category, each with the button that
//! takes it out, and Add Items to put more in.

use cr_app::packages::{ItemChooser, PackageEditor, category_label};
use cr_i18n::tr;
use cr_win::text::plain;
use windows_reactor::*;

use super::action;
use super::forms::heading;
use super::{Message, Shell};
use crate::ui;

/// Export…, or the editor's Add Items.
#[derive(Clone)]
pub struct ExportPage {
    pub chooser: ItemChooser,
    /// The items go into the package editor below, rather than into a new
    /// package.
    pub adding: bool,
}

/// The Save dialog's file types for a package of the Expansion or of Core
/// Rules 2.0.
pub fn save_filters(expansion: bool) -> Vec<(String, String)> {
    vec![if expansion {
        (tr("Expansion Packages (*.xpx)").to_owned(), "*.xpx".into())
    } else {
        (
            tr("Core Rules 2.0 Packages (*.xpt)").to_owned(),
            "*.xpt".into(),
        )
    }]
}

/// A page's title, with Help beside it, and its other commands.
fn header(title: &str, commands: Vec<KeyedView>, context: &mut ViewContext<Shell>) -> View {
    let mut commands = commands;
    commands.push(KeyedView::new(
        "help",
        ui::command(
            tr("Help"),
            Symbol::Help,
            true,
            context.message(Message::Act(action::HELP_HERE)),
        ),
    ));
    Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .column_spacing(16.0)
        .margin(Thickness::new(0.0, 0.0, 0.0, 8.0))
        .children((
            ui::title(title.to_owned()).vertical_alignment(VerticalAlignment::Center),
            CommandBar::new()
                .grid_column(1)
                .collection_slot(CommandBarSlot::PrimaryCommands, commands),
        ))
}

/// The buttons at the foot of a page: Cancel, which goes back, and the
/// action.
fn buttons(
    action: &str,
    enabled: bool,
    message: Message,
    busy: bool,
    context: &mut ViewContext<Shell>,
) -> View {
    let progress: View = if busy {
        ProgressRing::new()
            .is_active(true)
            .width(24.0)
            .height(24.0)
            .into()
    } else {
        View::empty()
    };
    StackPanel::new()
        .margin(Thickness::new(0.0, 16.0, 0.0, 0.0))
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Right)
        .children((
            progress,
            Button::new()
                .is_enabled(!busy)
                .on_click(context.message(Message::Back))
                .content(plain(tr("_Cancel"))),
            Button::new()
                .style(ButtonStyle::Accent)
                .is_enabled(enabled && !busy)
                .on_click(context.message(message))
                .content(plain(action)),
        ))
}

pub fn export_view(page: &ExportPage, busy: bool, context: &mut ViewContext<Shell>) -> View {
    let chooser = &page.chooser;
    let (title, action) = if page.adding {
        (tr("Add Items"), tr("_Add"))
    } else {
        (tr("Export"), tr("_Export…"))
    };
    let content: View = if chooser.items.is_empty() {
        ui::card(StackPanel::new().spacing(2.0).children((
            ui::strong(tr("No Records")),
            ui::caption(tr("Packages take the records you made and your characters")),
        )))
    } else {
        let categories: Vec<KeyedView> = chooser
            .categories()
            .into_iter()
            .map(|(category, items)| {
                let rows: Vec<KeyedView> = items
                    .iter()
                    .map(|&i| {
                        let name = chooser.items[i].name.as_str();
                        KeyedView::new(
                            format!("{i}\u{1}{name}"),
                            CheckBox::new()
                                .automation_name(name)
                                .is_checked(chooser.checked[i])
                                .on_is_checked_changed(
                                    context.callback(move |on| Message::ExportCheck(i, on)),
                                )
                                .content(ui::body(name)),
                        )
                    })
                    .collect();
                KeyedView::new(
                    category.label(),
                    StackPanel::new().spacing(4.0).children((
                        heading(category_label(category), None),
                        ui::card(StackPanel::new().spacing(4.0).keyed_children(rows)),
                    )),
                )
            })
            .collect();
        StackPanel::new().spacing(8.0).children((
            ui::card(
                CheckBox::new()
                    .automation_name(plain(tr("Select _All")))
                    .is_checked(chooser.all())
                    .on_is_checked_changed(context.callback(Message::ExportAll))
                    .content(ui::body(plain(tr("Select _All")))),
            ),
            StackPanel::new().spacing(8.0).keyed_children(categories),
        ))
    };
    ui::page(StackPanel::new().spacing(4.0).children((
        header(title, Vec::new(), context),
        content,
        buttons(action, chooser.any(), Message::ExportChosen, busy, context),
    )))
}

pub fn editor_view(editor: &PackageEditor, busy: bool, context: &mut ViewContext<Shell>) -> View {
    let add = vec![KeyedView::new(
        "add",
        ui::command(
            tr("Add Items"),
            Symbol::Add,
            editor.same_layout() && !busy,
            context.message(Message::PackageAdd),
        ),
    )];
    let note: View = match editor.layout_note() {
        Some(note) => InfoBar::new()
            .severity(InfoBarSeverity::Informational)
            .message(note)
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    };
    let items = editor.items();
    let content: View = if items.is_empty() {
        ui::card(StackPanel::new().spacing(2.0).children((
            ui::strong(tr("Empty Package")),
            ui::caption(tr("Add records and characters with the + button")),
        )))
    } else {
        let categories: Vec<KeyedView> = items
            .into_iter()
            .map(|(category, items)| {
                let rows: Vec<KeyedView> = items
                    .into_iter()
                    .map(|item| {
                        let key = format!("{}\u{1}{}", item.index, item.name);
                        let name = item.name.clone();
                        KeyedView::new(
                            key,
                            ui::setting(
                                &name,
                                "",
                                ui::icon_button(
                                    tr("Remove"),
                                    Symbol::Delete,
                                    Message::PackageRemove(Box::new(item)),
                                    context,
                                ),
                            ),
                        )
                    })
                    .collect();
                KeyedView::new(
                    category.label(),
                    StackPanel::new().spacing(4.0).children((
                        heading(category_label(category), None),
                        StackPanel::new().spacing(4.0).keyed_children(rows),
                    )),
                )
            })
            .collect();
        StackPanel::new().spacing(8.0).keyed_children(categories)
    };
    let maps: View = match editor.maps_note() {
        Some(note) => ui::caption(note).into(),
        None => View::empty(),
    };
    let error: View = match &editor.error {
        Some(error) => InfoBar::new()
            .severity(InfoBarSeverity::Error)
            .title(tr("Package Not Saved"))
            .message(error.clone())
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    };
    ui::page(StackPanel::new().spacing(8.0).children((
        header(&editor.title(), add, context),
        note,
        content,
        maps,
        error,
        buttons(
            tr("_Save"),
            !editor.saving,
            Message::PackageSave,
            busy || editor.saving,
            context,
        ),
    )))
}
