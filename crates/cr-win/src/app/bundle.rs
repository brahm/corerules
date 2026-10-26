//! A treasure bundle as the roller and the Encounter page edit it: each
//! item with Remove, Move Into and Edit Spells, the spells of a scroll
//! with Remove and Copy To, the items an item holds under it, and the
//! dialog the edit waits on, from the bundle editor of `cr_app::bundle`.

use cr_app::bundle::{BundleEditor, BundleInput, Row, Step, scroll_limit_text};
use cr_i18n::tr;
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::ui;

/// What a bundle's page says: an edit, and the answer to its dialog.
#[derive(Clone, Copy)]
pub struct BundleMessages {
    pub input: fn(BundleInput) -> Message,
    pub answer: fn(u64, bool) -> Message,
}

/// The indent of an item another holds.
const INDENT: f64 = 24.0;

/// A button that is only a symbol, named for screen readers. It has no
/// tooltip, which WinUI refuses inside a dialog.
fn symbol_button(
    name: &str,
    symbol: Symbol,
    message: Message,
    context: &mut ViewContext<Shell>,
) -> View {
    Button::new()
        .automation_name(name)
        .on_click(context.message(message))
        .content(SymbolIcon::new().symbol(symbol))
}

fn buttons(buttons: Vec<KeyedView>) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(4.0)
        .keyed_children(buttons)
}

/// The bundle's items, each with its edits; `editable` is false where
/// they are only shown.
pub fn rows(
    rows: &[Row],
    editable: bool,
    messages: BundleMessages,
    context: &mut ViewContext<Shell>,
) -> View {
    let mut out = Vec::new();
    for row in rows {
        add_row(row, 0, editable, messages, &mut out, context);
    }
    if out.is_empty() {
        return ui::card(ui::body(tr("None")).opacity(0.6));
    }
    StackPanel::new().spacing(4.0).keyed_children(out)
}

fn add_row(
    row: &Row,
    depth: usize,
    editable: bool,
    messages: BundleMessages,
    out: &mut Vec<KeyedView>,
    context: &mut ViewContext<Shell>,
) {
    let key = format!("{:?}\u{1}{}", row.path, row.title);
    let edits: View = if editable {
        let mut list = vec![
            KeyedView::new(
                "remove",
                ui::icon_button(
                    &plain(tr("_Remove…")),
                    Symbol::Delete,
                    (messages.input)(BundleInput::Remove(row.path.clone())),
                    context,
                ),
            ),
            KeyedView::new(
                "move",
                ui::icon_button(
                    &plain(tr("_Move Into…")),
                    Symbol::MoveToFolder,
                    (messages.input)(BundleInput::MoveInto(row.path.clone())),
                    context,
                ),
            ),
        ];
        if row.edit_spells {
            list.push(KeyedView::new(
                "spells",
                ui::icon_button(
                    &plain(tr("Edit _Spells…")),
                    Symbol::Edit,
                    (messages.input)(BundleInput::EditSpells(row.path.clone())),
                    context,
                ),
            ));
        }
        buttons(list)
    } else {
        View::empty()
    };
    let card = ui::setting(&row.title, &row.subtitle, edits);
    let card: View = Border::new()
        .margin(Thickness::new(INDENT * depth as f64, 0.0, 0.0, 0.0))
        .opacity(if row.flag { 0.6 } else { 1.0 })
        .content(card);
    out.push(KeyedView::new(key.clone(), card));
    for (i, (name, pages)) in row.spells.iter().enumerate() {
        let edits: View = if editable {
            buttons(vec![
                KeyedView::new(
                    "remove",
                    ui::icon_button(
                        tr("Remove"),
                        Symbol::Delete,
                        (messages.input)(BundleInput::RemoveSpell(row.path.clone(), i)),
                        context,
                    ),
                ),
                KeyedView::new(
                    "copy",
                    ui::icon_button(
                        &plain(tr("_Copy To…")),
                        Symbol::Copy,
                        (messages.input)(BundleInput::CopySpell(row.path.clone(), i)),
                        context,
                    ),
                ),
            ])
        } else {
            View::empty()
        };
        out.push(KeyedView::new(
            format!("{key}\u{1}spell{i}\u{1}{name}"),
            Border::new()
                .margin(Thickness::new(INDENT * (depth + 1) as f64, 0.0, 0.0, 0.0))
                .content(ui::setting(name, pages, edits)),
        ));
    }
    for inner in &row.rows {
        add_row(inner, depth + 1, editable, messages, out, context);
    }
}

/// The dialog the bundle editor waits on, a ContentDialog of its own for
/// each one opened.
pub fn dialog(
    editor: &BundleEditor,
    messages: BundleMessages,
    context: &mut ViewContext<Shell>,
) -> View {
    let Some(step) = editor.step() else {
        return View::empty();
    };
    let serial = editor.serial();
    let input = messages.input;
    let (title, content, primary, close): (String, View, String, String) = match step {
        Step::Add(add) => {
            let parts = add.shown();
            let chosen = parts.clone();
            let selected = add
                .chosen
                .as_ref()
                .and_then(|c| parts.iter().position(|p| p == c));
            let list: View = if parts.is_empty() {
                ui::caption(tr("No Results")).margin(12.0).into()
            } else {
                ListView::new()
                    .automation_name(tr("Add Item"))
                    .max_height(360.0)
                    .selected_index(selected)
                    .on_selection_changed(context.callback(move |i: Option<usize>| {
                        match i.and_then(|i| chosen.get(i)) {
                            Some(name) => input(BundleInput::Part(name.clone())),
                            None => Message::Nothing,
                        }
                    }))
                    .collection_slot(
                        ListViewSlot::Items,
                        parts.iter().enumerate().map(|(i, name)| {
                            KeyedView::new(
                                format!("{i}\u{1}{name}"),
                                ListViewItem::new().content(ui::body(name.clone())),
                            )
                        }),
                    )
            };
            let content = StackPanel::new().spacing(8.0).children((
                ComboBox::new()
                    .automation_name(tr("Category"))
                    .items_source(add.category_names())
                    .selected_index(Some(add.category))
                    .min_width(240.0)
                    .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                        Some(i) => input(BundleInput::Category(i)),
                        None => Message::Nothing,
                    })),
                TextBox::new()
                    .automation_name(tr("Search"))
                    .placeholder_text(tr("Search"))
                    .text(add.search.clone())
                    .on_text_changed(
                        context.callback(move |text: String| input(BundleInput::Search(text))),
                    ),
                // A new list for each category and search, which would
                // otherwise keep a selection made in another.
                Grid::new().keyed_children([KeyedView::new(
                    format!("{}\u{1}{}", add.category, add.search),
                    list,
                )]),
            ));
            // Add goes on once a part is chosen.
            let add = if selected.is_some() {
                plain(tr("_Add"))
            } else {
                String::new()
            };
            (
                tr("Add Item").to_owned(),
                content,
                add,
                plain(tr("_Cancel")),
            )
        }
        Step::HowMany(how) => (
            how.title.to_owned(),
            StackPanel::new().spacing(4.0).children((
                ui::caption(tr("How many?")),
                NumberBox::new()
                    .automation_name(tr("How many?"))
                    .value(f64::from(how.value))
                    .minimum(f64::from(how.min))
                    .maximum(f64::from(how.max))
                    .min_width(160.0)
                    .on_value_changed(context.callback(move |v: Option<f64>| match v {
                        Some(v) => input(BundleInput::Count(v.round().max(0.0) as u32)),
                        None => Message::Nothing,
                    })),
            )),
            plain(how.action),
            plain(tr("_Cancel")),
        ),
        Step::Choose(choose) => {
            (
                choose.title.to_owned(),
                ListView::new()
                    .automation_name(choose.title)
                    .max_height(400.0)
                    .selected_index(choose.chosen)
                    .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                        Some(i) => input(BundleInput::Choose(i)),
                        None => Message::Nothing,
                    }))
                    .collection_slot(
                        ListViewSlot::Items,
                        choose.nodes.iter().enumerate().map(|(i, node)| {
                            KeyedView::new(
                                format!("{i}\u{1}{}", node.label),
                                ListViewItem::new().content(ui::body(node.label.clone()).margin(
                                    Thickness::new(INDENT * node.depth as f64, 0.0, 0.0, 0.0),
                                )),
                            )
                        }),
                    ),
                if choose.chosen.is_some() {
                    plain(choose.action)
                } else {
                    String::new()
                },
                plain(tr("_Cancel")),
            )
        }
        Step::Notice { title, text } => (
            title.clone(),
            ui::body(text.clone()).into(),
            String::new(),
            plain(tr("_OK")),
        ),
        Step::Ask(ask) => (
            ask.title.to_owned(),
            ui::body(ask.text).into(),
            plain(tr("_Yes")),
            plain(tr("_No")),
        ),
        Step::Scroll(scroll) => {
            let selected: Vec<KeyedView> = scroll
                .selected
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    KeyedView::new(
                        format!("{i}\u{1}{name}"),
                        Grid::new()
                            .columns([GridLength::STAR, GridLength::Auto])
                            .column_spacing(8.0)
                            .children((
                                ui::body(name.clone())
                                    .vertical_alignment(VerticalAlignment::Center),
                                Border::new().grid_column(1).content(symbol_button(
                                    tr("Remove"),
                                    Symbol::Delete,
                                    input(BundleInput::ScrollRemove(i)),
                                    context,
                                )),
                            )),
                    )
                })
                .collect();
            let choices = scroll.choices.clone();
            let adding = scroll
                .adding
                .as_ref()
                .and_then(|a| choices.iter().position(|c| c == a));
            // Past the scroll's limit, Add asks first.
            let asking: View = if scroll.asking {
                StackPanel::new().spacing(8.0).children((
                    InfoBar::new()
                        .severity(InfoBarSeverity::Warning)
                        .title(tr("Warning"))
                        .message(scroll_limit_text())
                        .is_open(true)
                        .is_closable(false),
                    StackPanel::new()
                        .orientation(Orientation::Horizontal)
                        .spacing(8.0)
                        .children((
                            Button::new()
                                .on_click(context.message(input(BundleInput::ScrollLimit(true))))
                                .content(plain(tr("_Yes"))),
                            Button::new()
                                .on_click(context.message(input(BundleInput::ScrollLimit(false))))
                                .content(plain(tr("_No"))),
                        )),
                ))
            } else {
                View::empty()
            };
            let content = StackPanel::new().spacing(8.0).children((
                ui::strong(scroll.heading()),
                ui::caption(tr("Select Spells that will be in the scroll.")),
                StackPanel::new().spacing(4.0).keyed_children(selected),
                asking,
                Grid::new()
                    .columns([GridLength::STAR, GridLength::Auto])
                    .column_spacing(8.0)
                    .children((
                        ComboBox::new()
                            .automation_name(plain(tr("Add Spell…")))
                            .placeholder_text(plain(tr("Add Spell…")))
                            .items_source(scroll.choices.clone())
                            .selected_index(adding)
                            .horizontal_alignment(HorizontalAlignment::Stretch)
                            .on_selection_changed(context.callback(move |i: Option<usize>| {
                                match i.and_then(|i| choices.get(i)) {
                                    Some(name) => input(BundleInput::ScrollAdding(name.clone())),
                                    None => Message::Nothing,
                                }
                            })),
                        Border::new().grid_column(1).content(
                            Button::new()
                                .is_enabled(adding.is_some() && !scroll.asking)
                                .on_click(context.message(input(BundleInput::ScrollAdd)))
                                .content(plain(tr("_Add"))),
                        ),
                    )),
            ));
            (
                scroll.title.clone(),
                content,
                plain(tr("_Done")),
                plain(tr("_Cancel")),
            )
        }
    };
    let answer = messages.answer;
    Grid::new().keyed_children([KeyedView::new(
        format!("bundle{serial}"),
        ContentDialog::new()
            .title(title)
            .primary_button_text(primary)
            .close_button_text(close)
            .is_open(true)
            .on_closed(
                context
                    .callback(move |result| answer(serial, result == ContentDialogResult::Primary)),
            )
            .content(ScrollViewer::new().max_height(480.0).content(content)),
    )])
}
