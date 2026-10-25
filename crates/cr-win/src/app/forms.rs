//! The parts New Character's pages and the editor's draw alike: headings,
//! number boxes, lists with Add and Remove, choices of one and check
//! boxes. Each takes what makes the shell's message of a change, so that
//! the wizard's and the editor's inputs both fit.

use cr_app::create::{Choice, List, ListKind, MOST_ITEMS, Unarmed};
use cr_i18n::tr;
use cr_store::create::UNARMED;
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::ui;

/// What the box of each list has chosen for Add, until it is added.
pub type Adding = Vec<(ListKind, String)>;

/// What a form's lists say: a name added, a row removed, a row's number
/// (an item's quantity or a proficiency's rank) changed.
#[derive(Clone, Copy)]
pub struct ListMessages {
    pub add: fn(ListKind, String) -> Message,
    pub remove: fn(ListKind, usize) -> Message,
    pub times: fn(ListKind, usize, u32) -> Message,
}

/// A heading over a group of cards, with the group's description.
pub fn heading(title: &str, description: Option<&str>) -> View {
    let margin = Thickness::new(0.0, 16.0, 0.0, 4.0);
    match description {
        Some(text) if !text.is_empty() => StackPanel::new()
            .margin(margin)
            .spacing(2.0)
            .children((ui::strong(title), ui::caption(text))),
        _ => ui::strong(title).margin(margin).into(),
    }
}

/// A title with a line under it, as a list row shows a choice.
pub fn two_lines(title: &str, subtitle: &str) -> View {
    if subtitle.is_empty() {
        return ui::body(title).into();
    }
    StackPanel::new()
        .spacing(2.0)
        .children((ui::body(title), ui::caption(subtitle)))
}

/// A whole number from a NumberBox, within `min..=max`.
pub fn whole(value: Option<f64>, min: u32, max: u32) -> Option<u32> {
    value.map(|v| (v.round().max(0.0) as u32).clamp(min, max))
}

/// A NumberBox for a whole number in `min..=max`; `changed` makes the
/// message of a new value.
pub fn number(
    name: &str,
    value: u32,
    (min, max): (u32, u32),
    changed: impl Fn(u32) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> NumberBox {
    NumberBox::new()
        .automation_name(name)
        .value(f64::from(value))
        .minimum(f64::from(min))
        .maximum(f64::from(max))
        .min_width(120.0)
        .on_value_changed(context.callback(move |v| match whole(v, min, max) {
            Some(v) => changed(v),
            None => Message::Nothing,
        }))
}

/// A NumberBox for a whole number in `min..=max` that may be below 0.
pub fn signed_number(
    name: &str,
    value: i32,
    (min, max): (i32, i32),
    changed: impl Fn(i32) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> NumberBox {
    NumberBox::new()
        .automation_name(name)
        .value(f64::from(value))
        .minimum(f64::from(min))
        .maximum(f64::from(max))
        .min_width(120.0)
        .on_value_changed(context.callback(move |v: Option<f64>| match v {
            Some(v) => changed((v.round() as i32).clamp(min, max)),
            None => Message::Nothing,
        }))
}

/// A control and the buttons after it, in a row.
pub fn with_buttons(control: impl Into<View>, buttons: impl Into<View>) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .children((control.into(), buttons.into()))
}

/// A list the user adds to, with a box of what may be added and Add, and
/// removes from by each row's Remove; a row's number changes in it. Each
/// of Add's choices has its line of `details`, by its index, if any.
pub fn list(
    kind: ListKind,
    list: &List,
    details: &[String],
    adding: &Adding,
    messages: ListMessages,
    context: &mut ViewContext<Shell>,
) -> View {
    if !list.shown {
        return View::empty();
    }
    let rows: Vec<KeyedView> = list
        .rows
        .iter()
        .enumerate()
        .map(|(i, (title, subtitle, count))| {
            let times = messages.times;
            let rank: View = match *count {
                // How many of an item, or a proficiency's rank.
                Some(n) if kind == ListKind::Items => number(
                    &format!("{title}: {}", tr("Quantity")),
                    n,
                    (1, MOST_ITEMS),
                    move |v| times(kind, i, v),
                    context,
                )
                .into(),
                Some(n) => number(
                    &format!("{title}: {}", tr("Rank")),
                    n,
                    (1, 10),
                    move |v| times(kind, i, v),
                    context,
                )
                .into(),
                None => View::empty(),
            };
            let remove: View = if list.fixed {
                View::empty()
            } else {
                ui::icon_button(
                    tr("Remove"),
                    Symbol::Delete,
                    (messages.remove)(kind, i),
                    context,
                )
            };
            KeyedView::new(
                format!("{i}\u{1}{title}"),
                ui::setting(title, subtitle, with_buttons(rank, remove)),
            )
        })
        .collect();
    let rows: View = if rows.is_empty() {
        ui::card(ui::body(tr("None")).opacity(0.6))
    } else {
        StackPanel::new().spacing(4.0).keyed_children(rows)
    };
    // Rows only rolled have nothing to add.
    if list.fixed {
        return StackPanel::new()
            .spacing(4.0)
            .children((heading(&list.title, Some(&list.description)), rows));
    }
    let chosen = adding
        .iter()
        .find(|(k, _)| *k == kind)
        .and_then(|(_, name)| list.choices.iter().position(|c| c == name));
    let add_to = tr!("Add to {list}", list = list.title.clone());
    let choices = list.choices.clone();
    let add: View = match chosen {
        Some(i) => Button::new()
            .on_click(context.message((messages.add)(kind, list.choices[i].clone())))
            .content(plain(tr("_Add"))),
        None => Button::new().is_enabled(false).content(plain(tr("_Add"))),
    };
    let items: Vec<String> = list
        .choices
        .iter()
        .enumerate()
        .map(|(i, choice)| match details.get(i) {
            Some(detail) if !detail.is_empty() => format!("{choice} · {detail}"),
            _ => choice.clone(),
        })
        .collect();
    let add = with_buttons(
        ComboBox::new()
            .automation_name(add_to.clone())
            .placeholder_text(add_to)
            .items_source(items)
            .selected_index(chosen)
            .is_enabled(!list.choices.is_empty())
            .min_width(280.0)
            .on_selection_changed(context.callback(move |i: Option<usize>| {
                match i.and_then(|i| choices.get(i)) {
                    Some(name) => Message::Adding(kind, name.clone()),
                    None => Message::Nothing,
                }
            })),
        add,
    );
    StackPanel::new().spacing(4.0).children((
        heading(&list.title, Some(&list.description)),
        rows,
        Border::new()
            .margin(Thickness::new(0.0, 4.0, 0.0, 0.0))
            .content(add),
    ))
}

/// A choice of one, "None" first, as a settings card named `label`.
pub fn choice(
    choice: &Choice,
    label: &str,
    chosen: impl Fn(usize) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    if !choice.shown {
        return View::empty();
    }
    StackPanel::new().spacing(4.0).children((
        heading(&choice.title, Some(&choice.description)),
        ui::setting(
            label,
            "",
            ComboBox::new()
                .automation_name(choice.title.clone())
                .items_source(choice.items.clone())
                .selected_index(choice.selected)
                .min_width(240.0)
                .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                    Some(i) => chosen(i),
                    None => Message::Nothing,
                })),
        ),
    ))
}

/// Check boxes in a card, each with a line under it and whether it may
/// change: `make` makes the message of one ticked or not.
pub fn checks(
    rows: &[(String, String, bool, bool)],
    make: impl Fn(usize, bool) -> Message + Clone + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    let checks: Vec<KeyedView> = rows
        .iter()
        .enumerate()
        .map(|(i, (title, subtitle, on, enabled))| {
            let make = make.clone();
            KeyedView::new(
                format!("{i}\u{1}{title}"),
                CheckBox::new()
                    .automation_name(title.clone())
                    .is_checked(*on)
                    .is_enabled(*enabled)
                    .on_is_checked_changed(context.callback(move |on| make(i, on)))
                    .content(two_lines(title, subtitle)),
            )
        })
        .collect();
    ui::card(StackPanel::new().spacing(4.0).keyed_children(checks))
}

/// The unarmed styles to tick, each with what it costs.
pub fn unarmed(
    unarmed: &Unarmed,
    make: impl Fn(usize, bool) -> Message + Clone + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    if !unarmed.shown {
        return View::empty();
    }
    let rows: Vec<(String, String, bool, bool)> = UNARMED
        .iter()
        .zip(unarmed.taken)
        .map(|(name, on)| ((*name).to_owned(), unarmed.cost.clone(), on, true))
        .collect();
    StackPanel::new().spacing(4.0).children((
        heading(
            tr("Unarmed Combat"),
            Some(tr(
                "Specialization in an unarmed style, not together with a weapon specialization",
            )),
        ),
        checks(&rows, make, context),
    ))
}

/// Skills & Powers: the character points left, kept for later levels.
pub fn points_left(points: &str, problem: Option<&str>) -> View {
    StackPanel::new().spacing(4.0).children((
        heading(tr("Character Points Left"), problem),
        ui::setting(
            tr("Character Points Left"),
            tr("Kept on the character for later levels"),
            ui::subtitle(points.to_owned()),
        ),
    ))
}
