//! A record of the user databases on a page of the window: the editor of
//! `cr_app::records::form`, its pages chosen above them, each field drawn
//! as its kind asks, with Save and Cancel under them, and the dialogs
//! some fields open: a group to choose from, a class level's free
//! spells, NWPs or ability, and a monster attack.

use cr_app::action;
use cr_app::bundle::BundleInput;
use cr_app::records::dialogs::{
    self, AbilityEdit, AttackEdit, AttackInput, Dialog, DialogInput, FreeSpellsEdit, GroupEdit,
    NwpsEdit,
};
use cr_app::records::form::{Field, GRID_ROWS, RecordForm, RecordInput};
use cr_i18n::tr;
use cr_store::userdb::classes::levels::FREQUENCIES;
use cr_store::userdb::monsters::{AREAS, SAVE_RESULTS, SAVES, TO_HIT, area_labels};
use cr_store::userdb::{FieldKind, Value};
use cr_win::text::plain;
use windows_reactor::*;

use super::bundle::{self, BundleMessages};
use super::create::prompt_dialog;
use super::forms::{heading, two_lines, with_buttons};
use super::{Message, Shell};
use crate::ui;

fn input(input: RecordInput) -> Message {
    Message::Record(input)
}

fn change(change: DialogInput) -> Message {
    input(RecordInput::Dialog(change))
}

fn attack(change: AttackInput) -> Message {
    self::change(DialogInput::Attack(change))
}

/// The items' edits.
const ITEMS: BundleMessages = BundleMessages {
    input: |i| input(RecordInput::Items(i)),
    answer: |serial, yes| input(RecordInput::ItemsAnswer(serial, yes)),
};

/// What a picture field shows for what the record keeps, by its kind:
/// the line under its label and the picture.
pub type PictureOf<'a> = &'a dyn Fn(FieldKind, &str) -> (String, Option<EncodedImage>);

/// A NumberBox for a whole number in `min..=max`.
pub fn number(
    name: &str,
    value: i64,
    (min, max): (i64, i64),
    enabled: bool,
    changed: impl Fn(i64) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> NumberBox {
    NumberBox::new()
        .automation_name(name)
        .value(value as f64)
        .minimum(min as f64)
        .maximum(max as f64)
        .min_width(120.0)
        .is_enabled(enabled)
        .on_value_changed(context.callback(move |v: Option<f64>| match v {
            Some(v) => changed((v.round() as i64).clamp(min, max)),
            None => Message::Nothing,
        }))
}

/// Three NumberBoxes for dice: count d sides + bonus.
pub fn dice(
    name: &str,
    values: [i64; 3],
    ranges: [(i64, i64); 3],
    enabled: bool,
    changed: impl Fn(usize, i64) -> Message + Clone + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    let parts = [tr("Dice"), tr("Sides"), tr("Bonus")];
    let mut children: Vec<KeyedView> = Vec::new();
    for k in 0..3 {
        if k > 0 {
            children.push(KeyedView::new(
                format!("sep{k}"),
                ui::body(if k == 1 { "d" } else { "+" })
                    .vertical_alignment(VerticalAlignment::Center),
            ));
        }
        let changed = changed.clone();
        children.push(KeyedView::new(
            format!("n{k}"),
            number(
                &format!("{name}: {}", parts[k]),
                values[k],
                ranges[k],
                enabled,
                move |v| changed(k, v),
                context,
            )
            .min_width(96.0),
        ));
    }
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .keyed_children(children)
}

/// A button with a label, which, unlike a symbol's, needs no tooltip:
/// WinUI refuses one inside a dialog.
pub fn text_button(
    label: &str,
    enabled: bool,
    message: Message,
    context: &mut ViewContext<Shell>,
) -> View {
    Button::new()
        .is_enabled(enabled)
        .on_click(context.message(message))
        .content(plain(label))
}

/// A row of a list: its text and the buttons after it.
fn row(title: &str, subtitle: &str, buttons: impl Into<View>) -> View {
    ui::setting(title, subtitle, buttons)
}

/// A box of names to add, with Add after it; `choose` makes the message
/// of a name chosen, `add` that of Add.
pub fn add_box(
    name: &str,
    choices: &[String],
    chosen: Option<&str>,
    enabled: bool,
    choose: impl Fn(String) -> Message + 'static,
    add: Message,
    context: &mut ViewContext<Shell>,
) -> View {
    let selected = chosen.and_then(|c| choices.iter().position(|n| n == c));
    let items = choices.to_vec();
    Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .column_spacing(8.0)
        .max_width(560.0)
        .horizontal_alignment(HorizontalAlignment::Left)
        .children((
            ComboBox::new()
                .automation_name(name)
                .placeholder_text(name)
                .items_source(choices.to_vec())
                .selected_index(selected)
                .is_enabled(enabled && !choices.is_empty())
                .min_width(200.0)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .on_selection_changed(context.callback(move |i: Option<usize>| {
                    match i.and_then(|i| items.get(i)) {
                        Some(name) => choose(name.clone()),
                        None => Message::Nothing,
                    }
                })),
            Border::new().grid_column(1).content(text_button(
                tr("_Add"),
                enabled && selected.is_some(),
                add,
                context,
            )),
        ))
}

/// The editor; `picture` gives the pictures its fields show.
pub fn view(form: &RecordForm, picture: PictureOf, context: &mut ViewContext<Shell>) -> View {
    let pages = form.pages();
    let shown: Vec<usize> = (0..pages.len()).filter(|&p| form.page_visible(p)).collect();
    // The pages to choose from, once there are several.
    let tabs: View = if shown.len() > 1 {
        let items: Vec<KeyedView> = shown
            .iter()
            .map(|&p| {
                KeyedView::new(
                    pages[p].name,
                    SelectorBarItem::new()
                        .text(tr(pages[p].name))
                        .is_selected(p == form.page),
                )
            })
            .collect();
        let names: Vec<(usize, String)> = shown
            .iter()
            .map(|&p| (p, tr(pages[p].name).to_owned()))
            .collect();
        ScrollViewer::new()
            .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)
            .vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled)
            .margin(Thickness::new(0.0, 0.0, 0.0, 4.0))
            .content(
                Border::new()
                    .padding(Thickness::new(0.0, 0.0, 0.0, 8.0))
                    .content(
                        SelectorBar::new()
                            .on_selected_text_changed(context.callback(
                                move |text: Option<String>| {
                                    match names.iter().find(|(_, n)| text.as_deref() == Some(n)) {
                                        Some((p, _)) => input(RecordInput::Page(*p)),
                                        None => Message::Nothing,
                                    }
                                },
                            ))
                            .slots([SlotView::collection(SelectorBarSlot::Items, items)]),
                    ),
            )
    } else {
        View::empty()
    };
    let groups: Vec<KeyedView> = pages
        .get(form.page)
        .map(|page| {
            page.groups
                .iter()
                .enumerate()
                .filter(|(_, g)| form.group_visible(g))
                .map(|(g, group)| {
                    let title: View = if group.title.is_empty() {
                        View::empty()
                    } else {
                        heading(tr(group.title), None)
                    };
                    let fields: Vec<KeyedView> = group
                        .fields
                        .iter()
                        .map(|&f| {
                            KeyedView::new(f.to_string(), field_view(form, f, picture, context))
                        })
                        .collect();
                    KeyedView::new(
                        format!("{}\u{1}{g}", page.name),
                        StackPanel::new().spacing(4.0).children((
                            title,
                            StackPanel::new().spacing(4.0).keyed_children(fields),
                        )),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let problem = problem(form.error.as_deref());
    let buttons = buttons(
        !form.saving,
        form.can_save(),
        input(RecordInput::Save),
        context,
    );
    let header = header(form.title(), context);
    // Each question is a dialog of its own, so that one asked right after
    // another opens; so is each opening of a field's dialog.
    let serial = form.prompt_serial();
    // WinUI opens one ContentDialog at a time: a question first.
    let dialog: Option<KeyedView> = if let Some(prompt) = form.prompt() {
        Some(KeyedView::new(
            format!("prompt{serial}"),
            prompt_dialog(prompt, serial, Message::RecordAnswer, context),
        ))
    } else if let Some(dialog) = &form.dialog {
        Some(KeyedView::new(
            format!("dialog{}", form.dialog_serial),
            dialog_view(form, dialog, context),
        ))
    } else {
        form.items.as_ref().map(|items| {
            KeyedView::new(
                format!("items{}", items.serial()),
                bundle::dialog(items, ITEMS, context),
            )
        })
    };
    let dialogs: Vec<KeyedView> = dialog.into_iter().collect();
    Grid::new().children((
        ui::page(StackPanel::new().spacing(4.0).children((
            header,
            tabs,
            StackPanel::new().spacing(4.0).keyed_children(groups),
            problem,
            buttons,
        ))),
        Grid::new().keyed_children(dialogs),
    ))
}

/// Why an editor could not save, if it could not.
pub fn problem(error: Option<&str>) -> View {
    match error {
        Some(error) => InfoBar::new()
            .severity(InfoBarSeverity::Error)
            .title(tr("Not Saved"))
            .message(error.to_owned())
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    }
}

/// An editor's Cancel, which goes back, and Save.
pub fn buttons(
    cancel: bool,
    save: bool,
    saving: Message,
    context: &mut ViewContext<Shell>,
) -> View {
    StackPanel::new()
        .margin(Thickness::new(0.0, 16.0, 0.0, 0.0))
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Right)
        .children((
            Button::new()
                .is_enabled(cancel)
                .on_click(context.message(Message::Back))
                .content(plain(tr("_Cancel"))),
            Button::new()
                .style(ButtonStyle::Accent)
                .is_enabled(save)
                .on_click(context.message(saving))
                .content(plain(tr("_Save"))),
        ))
}

/// An editor's title, with Help beside it.
pub fn header(title: &str, context: &mut ViewContext<Shell>) -> View {
    Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .column_spacing(16.0)
        .margin(Thickness::new(0.0, 0.0, 0.0, 16.0))
        .children((
            ui::title(title.to_owned()).vertical_alignment(VerticalAlignment::Center),
            CommandBar::new().grid_column(1).collection_slot(
                CommandBarSlot::PrimaryCommands,
                vec![KeyedView::new(
                    "help",
                    ui::command(
                        tr("Help"),
                        Symbol::Help,
                        true,
                        context.message(Message::Act(action::HELP_HERE)),
                    ),
                )],
            ),
        ))
}

/// One field, as its kind draws it.
fn field_view(
    form: &RecordForm,
    index: usize,
    picture: PictureOf,
    context: &mut ViewContext<Shell>,
) -> View {
    let field = form.field(index);
    if !field.visible {
        return View::empty();
    }
    let label = tr(field.label);
    let enabled = field.enabled;
    let set = move |value: Value| input(RecordInput::Set(index, value));
    match field.kind {
        FieldKind::Text { .. } => ui::setting(
            label,
            "",
            TextBox::new()
                .automation_name(label)
                .text(field.value.text())
                .min_width(260.0)
                .is_enabled(enabled)
                .on_text_changed(context.callback(move |text: String| set(Value::Text(text)))),
        ),
        FieldKind::MultiLine => TextBox::new()
            .automation_name(label)
            .text(field.value.text())
            .accepts_return(true)
            .text_wrapping(TextWrapping::Wrap)
            .min_height(160.0)
            .is_enabled(enabled)
            .on_text_changed(context.callback(move |text: String| set(Value::Text(text))))
            .into(),
        FieldKind::Int { min, max } => {
            let n = number(
                label,
                field.value.int(),
                (min, max),
                enabled,
                move |v| set(Value::Int(v)),
                context,
            );
            let control: View = match field.suggest {
                Some(suggest) => with_buttons(
                    n,
                    text_button(
                        tr(suggest),
                        enabled,
                        input(RecordInput::Suggest(index)),
                        context,
                    ),
                ),
                None => n.into(),
            };
            ui::setting(label, "", control)
        }
        FieldKind::Check => ui::card(
            CheckBox::new()
                .automation_name(label)
                .is_checked(field.value.bool())
                .is_enabled(enabled)
                .on_is_checked_changed(context.callback(move |on| set(Value::Bool(on))))
                .content(ui::body(label)),
        ),
        FieldKind::Choice(_) | FieldKind::Combo { free: false, .. } => {
            let (choices, selected) = form.choices(index);
            ui::setting(
                label,
                "",
                ComboBox::new()
                    .automation_name(label)
                    .items_source(choices)
                    .selected_index(selected)
                    .min_width(240.0)
                    .is_enabled(enabled)
                    .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                        Some(i) => input(RecordInput::Choose(index, i)),
                        None => Message::Nothing,
                    })),
            )
        }
        FieldKind::Combo {
            options,
            free: true,
        } => {
            let choices = form.options(options);
            let text = TextBox::new()
                .automation_name(label)
                .text(field.value.text())
                .min_width(220.0)
                .is_enabled(enabled)
                .on_text_changed(context.callback(move |text: String| set(Value::Text(text))));
            let control: View = if choices.is_empty() {
                text.into()
            } else {
                let choose = tr!("Choose {field}", field = label);
                let items = choices.clone();
                with_buttons(
                    text,
                    ComboBox::new()
                        .automation_name(choose.clone())
                        .placeholder_text(plain(tr("_Choose…")))
                        .items_source(choices)
                        .selected_index(None)
                        .min_width(140.0)
                        .is_enabled(enabled)
                        .on_selection_changed(context.callback(move |i: Option<usize>| {
                            match i.and_then(|i| items.get(i)) {
                                Some(choice) => set(Value::Text(choice.clone())),
                                None => Message::Nothing,
                            }
                        })),
                )
            };
            ui::setting(label, "", control)
        }
        FieldKind::Dual(_) | FieldKind::Pairs { .. } => names(form, index, &field, context),
        FieldKind::Checked(_) => StackPanel::new().spacing(4.0).children((
            ui::card(
                CheckBox::new()
                    .automation_name(label)
                    .is_checked(field.checked)
                    .is_enabled(enabled)
                    .on_is_checked_changed(
                        context.callback(move |on| input(RecordInput::Check(index, on))),
                    )
                    .content(ui::body(label)),
            ),
            if field.checked {
                names(form, index, &field, context)
            } else {
                View::empty()
            },
        )),
        FieldKind::OptInt { min, max, .. } => ui::setting(
            label,
            "",
            with_buttons(
                CheckBox::new()
                    .automation_name(label)
                    .is_checked(field.checked)
                    .is_enabled(enabled)
                    .min_width(0.0)
                    .on_is_checked_changed(
                        context.callback(move |on| input(RecordInput::Check(index, on))),
                    ),
                number(
                    label,
                    field.value.int(),
                    (min, max),
                    field.number_open(),
                    move |v| set(Value::Int(v)),
                    context,
                ),
            ),
        ),
        FieldKind::Real { min, max, none } => {
            let value = field.value.real();
            let n = NumberBox::new()
                .automation_name(label)
                .value(if field.number_open() || none.is_none() {
                    value
                } else {
                    0.0
                })
                .minimum(min)
                .maximum(max)
                .min_width(120.0)
                .is_enabled(field.number_open())
                .on_value_changed(context.callback(move |v: Option<f64>| match v {
                    Some(v) => set(Value::Real(v.clamp(min, max))),
                    None => Message::Nothing,
                }));
            let control: View = match none {
                Some((_, check)) => with_buttons(
                    CheckBox::new()
                        .is_checked(field.checked)
                        .is_enabled(enabled)
                        .on_is_checked_changed(
                            context.callback(move |on| input(RecordInput::Check(index, on))),
                        )
                        .content(ui::body(tr(check))),
                    n,
                ),
                None => n.into(),
            };
            ui::setting(label, "", control)
        }
        FieldKind::Groups(_) => entries(form, index, enabled, tr("Add a Group…"), context),
        FieldKind::FreeSpells => entries(form, index, enabled, tr("Add Free Spells…"), context),
        FieldKind::LevelNwps(_) => entries(form, index, enabled, tr("Add NWPs…"), context),
        FieldKind::LevelAbilities(_) => {
            entries(form, index, enabled, tr("Add a Class Ability…"), context)
        }
        FieldKind::Attacks => attacks(form, index, enabled, context),
        FieldKind::Items => {
            let bundle = field.value.items();
            StackPanel::new().spacing(4.0).children((
                bundle::rows(&cr_app::bundle::rows(&bundle), enabled, ITEMS, context),
                Border::new()
                    .margin(Thickness::new(0.0, 4.0, 0.0, 0.0))
                    .content(text_button(
                        tr("_Add Item…"),
                        enabled,
                        input(RecordInput::Items(BundleInput::Add)),
                        context,
                    )),
            ))
        }
        FieldKind::Grid {
            columns,
            first,
            min,
            max,
        } => grid(form, index, &field, (columns, first, min, max), context),
        FieldKind::AutoFill => {
            let classes = form.auto_fill_classes();
            let chosen = form.adding(index).map(str::to_owned);
            let copy = match chosen {
                Some(class) => input(RecordInput::AutoFill(index, class)),
                None => Message::Nothing,
            };
            let selected = form
                .adding(index)
                .and_then(|c| classes.iter().position(|n| n == c));
            let items = classes.clone();
            ui::setting(
                tr("Copy from another class…"),
                tr("Fills this page with what another class has on it"),
                with_buttons(
                    ComboBox::new()
                        .automation_name(tr("Select a class to copy"))
                        .placeholder_text(tr("Select a class to copy"))
                        .items_source(classes)
                        .selected_index(selected)
                        .min_width(200.0)
                        .is_enabled(enabled)
                        .on_selection_changed(context.callback(move |i: Option<usize>| {
                            match i.and_then(|i| items.get(i)) {
                                Some(class) => input(RecordInput::Adding(index, class.clone())),
                                None => Message::Nothing,
                            }
                        })),
                    text_button(tr("Copy"), enabled && selected.is_some(), copy, context),
                ),
            )
        }
        FieldKind::Dice => {
            let values = field.value.dice();
            ui::setting(
                label,
                "",
                dice(
                    label,
                    values,
                    [(0, 1000), (0, 1000), (-1000, 100_000)],
                    enabled,
                    move |k, v| {
                        let mut d = values;
                        d[k] = v;
                        set(Value::Dice(d))
                    },
                    context,
                ),
            )
        }
        FieldKind::Picture(_) | FieldKind::Icon => {
            let stored = field.value.text();
            let (subtitle, image) = picture(field.kind, &stored);
            let side = if field.kind == FieldKind::Icon {
                f64::from(cr_store::pictures::ICON_SIZE)
            } else {
                64.0
            };
            let shown: View = match image {
                Some(image) => Image::new()
                    .source_data(image)
                    .stretch(Stretch::Uniform)
                    .max_width(side * 2.0)
                    .height(side)
                    .automation_name(label)
                    .into(),
                None => View::empty(),
            };
            let clear: View = if cr_store::pictures::split_stored(&stored)
                .1
                .trim()
                .is_empty()
            {
                View::empty()
            } else {
                ui::icon_button(
                    tr("No Picture"),
                    Symbol::Clear,
                    set(Value::Text(String::new())),
                    context,
                )
            };
            ui::card(
                Grid::new()
                    .column_spacing(16.0)
                    .columns([GridLength::Auto, GridLength::STAR, GridLength::Auto])
                    .children((
                        shown,
                        Border::new()
                            .grid_column(1)
                            .vertical_alignment(VerticalAlignment::Center)
                            .content(two_lines(label, &subtitle)),
                        StackPanel::new()
                            .grid_column(2)
                            .orientation(Orientation::Horizontal)
                            .spacing(8.0)
                            .vertical_alignment(VerticalAlignment::Center)
                            .children((
                                clear,
                                Button::new()
                                    .is_enabled(enabled)
                                    .on_click(context.message(Message::ChoosePicture(index)))
                                    .content(plain(tr("_Choose…"))),
                            )),
                    )),
            )
        }
    }
}

/// The names a list field has chosen, each with its number for pairs and
/// Remove, and the box of those it may add.
fn names(form: &RecordForm, index: usize, field: &Field, context: &mut ViewContext<Shell>) -> View {
    let enabled = field.number_open();
    let number = match field.kind {
        FieldKind::Pairs {
            number, min, max, ..
        } => Some((tr(number), min, max)),
        _ => None,
    };
    let rows: Vec<KeyedView> = field
        .value
        .pairs()
        .iter()
        .enumerate()
        .map(|(i, (name, n))| {
            let count: View = match number {
                Some((label, min, max)) => number_of(
                    &format!("{name}: {label}"),
                    *n,
                    (min, max),
                    enabled,
                    move |v| input(RecordInput::Number(index, i, v)),
                    context,
                ),
                None => View::empty(),
            };
            let remove = ui::icon_button(
                tr("Remove"),
                Symbol::Delete,
                input(RecordInput::Remove(index, i)),
                context,
            );
            KeyedView::new(
                format!("{i}\u{1}{name}"),
                row(
                    name,
                    "",
                    with_buttons(count, if enabled { remove } else { View::empty() }),
                ),
            )
        })
        .collect();
    let rows: View = if rows.is_empty() {
        ui::card(ui::body(tr("None")).opacity(0.6))
    } else {
        StackPanel::new().spacing(4.0).keyed_children(rows)
    };
    let add_to = tr!("Add to {list}", list = tr(field.label));
    StackPanel::new().spacing(4.0).children((
        rows,
        Border::new()
            .margin(Thickness::new(0.0, 4.0, 0.0, 0.0))
            .content(add_box(
                &add_to,
                &form.list_choices(index),
                form.adding(index),
                enabled,
                move |name| input(RecordInput::Adding(index, name)),
                input(RecordInput::Add(index)),
                context,
            )),
    ))
}

fn number_of(
    name: &str,
    value: i64,
    range: (i64, i64),
    enabled: bool,
    changed: impl Fn(i64) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    number(name, value, range, enabled, changed, context).into()
}

/// The entries of a field listed with Edit and Remove (groups, levels),
/// and the button that adds one.
fn entries(
    form: &RecordForm,
    index: usize,
    enabled: bool,
    add: &str,
    context: &mut ViewContext<Shell>,
) -> View {
    let rows: Vec<KeyedView> = form
        .entries(index)
        .iter()
        .enumerate()
        .map(|(i, (title, subtitle))| {
            let buttons: View = if enabled {
                with_buttons(
                    ui::icon_button(
                        tr("Edit"),
                        Symbol::Edit,
                        input(RecordInput::Open(index, Some(i))),
                        context,
                    ),
                    ui::icon_button(
                        tr("Remove"),
                        Symbol::Delete,
                        input(RecordInput::RemoveEntry(index, i)),
                        context,
                    ),
                )
            } else {
                View::empty()
            };
            KeyedView::new(format!("{i}\u{1}{title}"), row(title, subtitle, buttons))
        })
        .collect();
    let rows: View = if rows.is_empty() {
        ui::card(ui::body(tr("None")).opacity(0.6))
    } else {
        StackPanel::new().spacing(4.0).keyed_children(rows)
    };
    StackPanel::new().spacing(4.0).children((
        rows,
        Border::new()
            .margin(Thickness::new(0.0, 4.0, 0.0, 0.0))
            .content(text_button(
                add,
                enabled,
                input(RecordInput::Open(index, None)),
                context,
            )),
    ))
}

/// A monster's attacks: each with Edit and Remove, the attack types to
/// add, and New….
fn attacks(
    form: &RecordForm,
    index: usize,
    enabled: bool,
    context: &mut ViewContext<Shell>,
) -> View {
    let types = form.attack_types();
    let chosen = form.adding(index).map(str::to_owned);
    let add = match chosen {
        Some(name) => input(RecordInput::AddAttack(index, name)),
        None => Message::Nothing,
    };
    StackPanel::new().spacing(4.0).children((
        entries_rows(form, index, enabled, context),
        Border::new()
            .margin(Thickness::new(0.0, 4.0, 0.0, 0.0))
            .content(add_box(
                tr("Attacks"),
                &types,
                form.adding(index),
                enabled,
                move |name| input(RecordInput::Adding(index, name)),
                add,
                context,
            )),
        Border::new().content(text_button(
            tr("New…"),
            enabled,
            input(RecordInput::Open(index, None)),
            context,
        )),
    ))
}

fn entries_rows(
    form: &RecordForm,
    index: usize,
    enabled: bool,
    context: &mut ViewContext<Shell>,
) -> View {
    let rows: Vec<KeyedView> = form
        .entries(index)
        .iter()
        .enumerate()
        .map(|(i, (title, subtitle))| {
            let buttons: View = if enabled {
                with_buttons(
                    ui::icon_button(
                        tr("Edit"),
                        Symbol::Edit,
                        input(RecordInput::Open(index, Some(i))),
                        context,
                    ),
                    ui::icon_button(
                        tr("Remove"),
                        Symbol::Delete,
                        input(RecordInput::RemoveEntry(index, i)),
                        context,
                    ),
                )
            } else {
                View::empty()
            };
            KeyedView::new(format!("{i}\u{1}{title}"), row(title, subtitle, buttons))
        })
        .collect();
    if rows.is_empty() {
        ui::card(ui::body(tr("None")).opacity(0.6))
    } else {
        StackPanel::new().spacing(4.0).keyed_children(rows)
    }
}

/// A table of numbers, one row per level, shown ten levels at a time.
fn grid(
    form: &RecordForm,
    index: usize,
    field: &Field,
    (columns, first, min, max): (&'static [&'static str], i64, i64, i64),
    context: &mut ViewContext<Shell>,
) -> View {
    let rows = field.value.grid();
    if rows.is_empty() {
        return ui::card(ui::body(tr("None")).opacity(0.6));
    }
    let start = form.grid_first(index).min(rows.len().saturating_sub(1)) / GRID_ROWS * GRID_ROWS;
    let ranges: Vec<String> = (0..rows.len())
        .step_by(GRID_ROWS)
        .map(|s| {
            let last = (s + GRID_ROWS).min(rows.len()) - 1;
            tr!(
                "Levels {from} to {to}",
                from = first + s as i64,
                to = first + last as i64
            )
        })
        .collect();
    let pager: View = if ranges.len() > 1 {
        ComboBox::new()
            .automation_name(tr("Levels"))
            .items_source(ranges)
            .selected_index(Some(start / GRID_ROWS))
            .min_width(200.0)
            .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                Some(i) => input(RecordInput::GridRows(index, i * GRID_ROWS)),
                None => Message::Nothing,
            }))
            .into()
    } else {
        View::empty()
    };
    let width = columns.len() + 1;
    let mut cells: Vec<KeyedView> = Vec::new();
    cells.push(KeyedView::new(
        "level",
        ui::strong(tr("Level")).vertical_alignment(VerticalAlignment::Bottom),
    ));
    for (c, title) in columns.iter().enumerate() {
        cells.push(KeyedView::new(
            format!("h{c}"),
            ui::strong(tr(title))
                .horizontal_alignment(HorizontalAlignment::Center)
                .vertical_alignment(VerticalAlignment::Bottom)
                .grid_column(c as i32 + 1),
        ));
    }
    let shown = &rows[start..(start + GRID_ROWS).min(rows.len())];
    for (r, values) in shown.iter().enumerate() {
        let level = first + (start + r) as i64;
        cells.push(KeyedView::new(
            format!("l{r}"),
            ui::body(level.to_string())
                .vertical_alignment(VerticalAlignment::Center)
                .grid_row(r as i32 + 1),
        ));
        for (c, value) in values.iter().enumerate().take(columns.len()) {
            let row = start + r;
            cells.push(KeyedView::new(
                format!("c{r}\u{1}{c}"),
                number(
                    &format!("{} {level}: {}", tr("Level"), tr(columns[c])),
                    *value,
                    (min, max),
                    field.enabled,
                    move |v| input(RecordInput::Cell(index, row, c, v)),
                    context,
                )
                .min_width(76.0)
                .grid_row(r as i32 + 1)
                .grid_column(c as i32 + 1),
            ));
        }
    }
    let table = Grid::new()
        .columns(std::iter::repeat_n(GridLength::Auto, width).collect::<Vec<_>>())
        .rows(std::iter::repeat_n(GridLength::Auto, shown.len() + 1).collect::<Vec<_>>())
        .column_spacing(6.0)
        .row_spacing(4.0)
        .keyed_children(cells);
    StackPanel::new().spacing(8.0).children((
        pager,
        ui::card(
            ScrollViewer::new()
                .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)
                .vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                .content(
                    Border::new()
                        .padding(Thickness::new(0.0, 0.0, 0.0, 12.0))
                        .content(table),
                ),
        ),
    ))
}

/// The dialog a field opened.
fn dialog_view(form: &RecordForm, dialog: &Dialog, context: &mut ViewContext<Shell>) -> View {
    let (title, content): (String, View) = match dialog {
        Dialog::Group(d) => (
            tr(form.field(d.field).label).to_owned(),
            group_view(form, d, context),
        ),
        Dialog::FreeSpells(d) => (
            tr("Free Spells").to_owned(),
            free_spells_view(form, d, context),
        ),
        Dialog::Nwps(d) => (tr("NWPs").to_owned(), nwps_view(form, d, context)),
        Dialog::Ability(d) => (tr("Class Abilities").to_owned(), ability_view(d, context)),
        Dialog::Attack(d) => (tr("Monster Attack").to_owned(), attack_view(d, context)),
    };
    ContentDialog::new()
        .title(title)
        .primary_button_text(plain(tr("_OK")))
        .close_button_text(plain(tr("_Cancel")))
        .is_open(true)
        .on_closed(context.callback(|result| {
            input(RecordInput::CloseDialog(
                result == ContentDialogResult::Primary,
            ))
        }))
        .content(
            ScrollViewer::new().max_height(560.0).content(
                Border::new()
                    .padding(Thickness::new(0.0, 0.0, 16.0, 0.0))
                    .content(content),
            ),
        )
}

/// A setting inside a dialog: the label over its control, as narrow
/// dialogs lay them out.
fn labelled(label: &str, control: impl Into<View>) -> View {
    StackPanel::new()
        .spacing(4.0)
        .children((ui::caption(label), control.into()))
}

/// A dialog's list of names, each with its number if it has one and
/// Remove, and the box of those to add.
fn dialog_list(
    list: usize,
    rows: &[(String, Option<i64>)],
    number: Option<(&str, i64, i64)>,
    choices: &[String],
    adding: Option<&str>,
    context: &mut ViewContext<Shell>,
) -> View {
    let shown: Vec<KeyedView> = rows
        .iter()
        .enumerate()
        .map(|(i, (name, n))| {
            let count: View = match (number, n) {
                (Some((label, min, max)), Some(n)) => number_of(
                    &format!("{name}: {label}"),
                    *n,
                    (min, max),
                    true,
                    move |v| change(DialogInput::Number(list, i, v)),
                    context,
                ),
                _ => View::empty(),
            };
            KeyedView::new(
                format!("{i}\u{1}{name}"),
                Grid::new()
                    .columns([GridLength::STAR, GridLength::Auto])
                    .column_spacing(8.0)
                    .children((
                        ui::body(name.clone()).vertical_alignment(VerticalAlignment::Center),
                        Border::new().grid_column(1).content(with_buttons(
                            count,
                            text_button(
                                tr("Remove"),
                                true,
                                change(DialogInput::Remove(list, i)),
                                context,
                            ),
                        )),
                    )),
            )
        })
        .collect();
    let shown: View = if shown.is_empty() {
        ui::caption(tr("None")).into()
    } else {
        StackPanel::new().spacing(4.0).keyed_children(shown)
    };
    StackPanel::new().spacing(8.0).children((
        shown,
        add_box(
            tr("Add"),
            choices,
            adding,
            true,
            move |name| change(DialogInput::Adding(list, name)),
            change(DialogInput::Add(list)),
            context,
        ),
    ))
}

fn group_view(form: &RecordForm, d: &GroupEdit, context: &mut ViewContext<Shell>) -> View {
    let rows: Vec<(String, Option<i64>)> = d.names.iter().map(|n| (n.clone(), None)).collect();
    StackPanel::new().spacing(12.0).children((
        labelled(
            tr("Number to choose"),
            number(
                tr("Number to choose"),
                d.count,
                (1, dialogs::MOST_COUNT),
                true,
                |v| change(DialogInput::Count(v)),
                context,
            ),
        ),
        dialog_list(
            0,
            &rows,
            None,
            &form.dialog_choices(0),
            d.adding.as_deref(),
            context,
        ),
    ))
}

fn level_box(level: i64, context: &mut ViewContext<Shell>) -> View {
    labelled(
        tr("Level"),
        number(
            tr("Level"),
            level,
            (1, dialogs::MOST_LEVEL),
            true,
            |v| change(DialogInput::Level(v)),
            context,
        ),
    )
}

fn free_spells_view(
    form: &RecordForm,
    d: &FreeSpellsEdit,
    context: &mut ViewContext<Shell>,
) -> View {
    let level = &d.level;
    let frequencies: Vec<String> = FREQUENCIES.iter().map(|f| tr(f).to_owned()).collect();
    let rows: Vec<KeyedView> = level
        .spells
        .iter()
        .enumerate()
        .map(|(i, spell)| {
            KeyedView::new(
                format!("{i}\u{1}{}\u{1}{}", spell.name, spell.wizard),
                Grid::new()
                    .columns([GridLength::STAR, GridLength::Auto])
                    .column_spacing(8.0)
                    .children((
                        Border::new()
                            .vertical_alignment(VerticalAlignment::Center)
                            .content(two_lines(&spell.name, FreeSpellsEdit::spell_kind(spell))),
                        Border::new().grid_column(1).content(with_buttons(
                            number(
                                &format!("{}: {}", spell.name, tr("Uses")),
                                spell.uses.into(),
                                (0, dialogs::MOST_USES),
                                true,
                                move |v| change(DialogInput::Number(0, i, v)),
                                context,
                            ),
                            text_button(
                                tr("Remove"),
                                true,
                                change(DialogInput::Remove(0, i)),
                                context,
                            ),
                        )),
                    )),
            )
        })
        .collect();
    let spells: View = if rows.is_empty() {
        ui::caption(tr("None")).into()
    } else {
        StackPanel::new().spacing(4.0).keyed_children(rows)
    };
    StackPanel::new().spacing(12.0).children((
        level_box(level.level.into(), context),
        labelled(
            tr("The spells may be cast this often"),
            number(
                tr("The spells may be cast this often"),
                level.uses.into(),
                (0, dialogs::MOST_USES),
                true,
                |v| change(DialogInput::Uses(v)),
                context,
            ),
        ),
        labelled(
            tr("Frequency"),
            with_buttons(
                TextBox::new()
                    .automation_name(tr("Frequency"))
                    .text(level.frequency.clone())
                    .min_width(160.0)
                    .on_text_changed(
                        context.callback(|t: String| change(DialogInput::Frequency(t))),
                    ),
                ComboBox::new()
                    .automation_name(tr("Choose"))
                    .placeholder_text(tr("Choose"))
                    .items_source(frequencies)
                    .selected_index(None)
                    .min_width(120.0)
                    .on_selection_changed(context.callback(|i: Option<usize>| {
                        // Stored as the original writes them.
                        match i.and_then(|i| FREQUENCIES.get(i)) {
                            Some(f) => change(DialogInput::Frequency((*f).to_owned())),
                            None => Message::Nothing,
                        }
                    })),
            ),
        ),
        CheckBox::new()
            .is_checked(level.pick_one)
            .on_is_checked_changed(context.callback(|on| change(DialogInput::PickOne(on))))
            .content(ui::body(tr("Must select one spell from the list"))),
        ui::strong(tr("Spells")),
        spells,
        labelled(
            tr("Spell type"),
            ComboBox::new()
                .automation_name(tr("Spell type"))
                .items_source(vec![tr("Priest").to_owned(), tr("Wizard").to_owned()])
                .selected_index(Some(usize::from(d.wizard)))
                .min_width(160.0)
                .on_selection_changed(context.callback(|i: Option<usize>| match i {
                    Some(i) => change(DialogInput::Wizard(i == 1)),
                    None => Message::Nothing,
                })),
        ),
        labelled(
            tr("Spell level (0 for all)"),
            number(
                tr("Spell level"),
                d.spell_level,
                (0, dialogs::MOST_SPELL_LEVEL),
                true,
                |v| change(DialogInput::SpellLevel(v)),
                context,
            ),
        ),
        add_box(
            tr("Add a Spell…"),
            &form.dialog_choices(0),
            d.adding.as_deref(),
            true,
            |name| change(DialogInput::Adding(0, name)),
            change(DialogInput::Add(0)),
            context,
        ),
    ))
}

fn nwps_view(form: &RecordForm, d: &NwpsEdit, context: &mut ViewContext<Shell>) -> View {
    let granted: Vec<(String, Option<i64>)> =
        d.level.granted.iter().map(|n| (n.clone(), None)).collect();
    let modified: Vec<(String, Option<i64>)> = d
        .level
        .modifications
        .iter()
        .map(|(n, m)| (n.clone(), Some(i64::from(*m))))
        .collect();
    StackPanel::new().spacing(12.0).children((
        level_box(d.level.level.into(), context),
        ui::strong(tr("Bonus NWPs granted at this level")),
        dialog_list(
            0,
            &granted,
            None,
            &form.dialog_choices(0),
            d.adding[0].as_deref(),
            context,
        ),
        CheckBox::new()
            .is_checked(d.level.pick_one)
            .on_is_checked_changed(context.callback(|on| change(DialogInput::PickOne(on))))
            .content(ui::body(tr("May only pick one from the list above"))),
        ui::strong(tr("NWP score modifications")),
        dialog_list(
            1,
            &modified,
            Some((tr("Modification"), 0, dialogs::MOST_MODIFICATION)),
            &form.dialog_choices(1),
            d.adding[1].as_deref(),
            context,
        ),
    ))
}

fn ability_view(d: &AbilityEdit, context: &mut ViewContext<Shell>) -> View {
    StackPanel::new().spacing(12.0).children((
        ui::body(d.prompt()),
        level_box(d.level, context),
        labelled(
            tr("Class Ability"),
            ComboBox::new()
                .automation_name(tr("Class Ability"))
                .items_source(d.choices.clone())
                .selected_index(d.chosen())
                .min_width(260.0)
                .on_selection_changed(context.callback(|i: Option<usize>| match i {
                    Some(i) => change(DialogInput::Ability(i)),
                    None => Message::Nothing,
                })),
        ),
    ))
}

fn combo(
    name: &str,
    items: &[&str],
    selected: usize,
    changed: fn(usize) -> AttackInput,
    context: &mut ViewContext<Shell>,
) -> View {
    labelled(
        name,
        ComboBox::new()
            .automation_name(name)
            .items_source(items.iter().map(|i| tr(i).to_owned()).collect::<Vec<_>>())
            .selected_index(Some(selected))
            .min_width(240.0)
            .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                Some(i) => attack(changed(i)),
                None => Message::Nothing,
            })),
    )
}

fn attack_number(
    name: &str,
    value: i32,
    min: i64,
    changed: fn(i32) -> AttackInput,
    context: &mut ViewContext<Shell>,
) -> View {
    labelled(
        name,
        number(
            name,
            value.into(),
            (min, dialogs::MOST_ATTACK_NUMBER.into()),
            true,
            move |v| attack(changed(v as i32)),
            context,
        ),
    )
}

fn attack_view(d: &AttackEdit, context: &mut ViewContext<Shell>) -> View {
    let a = &d.attack;
    let problem: View = match d.problem {
        Some(problem) => InfoBar::new()
            .severity(InfoBarSeverity::Error)
            .title(problem)
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    };
    let to_hit: Vec<&str> = TO_HIT.iter().map(|(_, l)| *l).collect();
    let most = i64::from(dialogs::MOST_ATTACK_NUMBER);
    let measures: Vec<KeyedView> = area_labels(d.area() as i32)
        .iter()
        .zip(d.measures())
        .enumerate()
        .filter_map(|(k, (label, value))| {
            let label = tr((*label)?).trim_end_matches(':').to_owned();
            Some(KeyedView::new(
                format!("{k}\u{1}{label}"),
                labelled(
                    &tr!("{measure} (feet)", measure = label.clone()),
                    number(
                        &label,
                        value.into(),
                        (0, most),
                        true,
                        move |v| attack(AttackInput::Measure(k, v as i32)),
                        context,
                    ),
                ),
            ))
        })
        .collect();
    StackPanel::new().spacing(12.0).children((
        problem,
        labelled(
            tr("Name"),
            TextBox::new()
                .automation_name(tr("Name"))
                .text(a.name.clone())
                .min_width(260.0)
                .on_text_changed(context.callback(|t: String| attack(AttackInput::Name(t)))),
        ),
        attack_number(tr("Help ID"), a.help_id, 0, AttackInput::HelpId, context),
        labelled(
            tr("Damage"),
            dice(
                tr("Damage"),
                [a.damage_dice, a.damage_sides, a.damage_bonus].map(i64::from),
                [(0, most); 3],
                true,
                |k, v| attack(AttackInput::Damage(k, v as i32)),
                context,
            ),
        ),
        combo(
            tr("Must make a to hit roll against"),
            &to_hit,
            d.to_hit(),
            AttackInput::ToHit,
            context,
        ),
        labelled(
            tr("Special Effect"),
            TextBox::new()
                .automation_name(tr("Special Effect"))
                .text(a.special_effect.clone())
                .min_width(260.0)
                .on_text_changed(context.callback(|t: String| attack(AttackInput::Effect(t)))),
        ),
        labelled(
            tr("Bonus Hit Dice (used to calculate modified HD for experience)"),
            number(
                tr("Bonus Hit Dice"),
                a.bonus_hit_dice.into(),
                (-most, most),
                true,
                |v| attack(AttackInput::HitDice(v as i32)),
                context,
            ),
        ),
        ui::strong(tr("Area of Effect")),
        combo(tr("Type"), &AREAS, d.area(), AttackInput::Area, context),
        StackPanel::new().spacing(12.0).keyed_children(measures),
        ui::strong(tr("Saving Throws")),
        combo(
            tr("Save Against"),
            &SAVES,
            d.save(),
            AttackInput::Save,
            context,
        ),
        attack_number(
            tr("Modified by"),
            a.save_modifier,
            0,
            AttackInput::Modifier,
            context,
        ),
        combo(
            tr("To gain this"),
            &SAVE_RESULTS,
            d.result(),
            AttackInput::Result,
            context,
        ),
    ))
}
