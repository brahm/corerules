//! The parts New Character's pages and the editor's draw alike: headings,
//! number boxes, lists with Add and Remove, choices of one and check
//! boxes. Each takes what makes the shell's message of a change, so that
//! the wizard's and the editor's inputs both fit.

use std::collections::HashMap;

use cr_app::create::{Choice, List, ListKind, MOST_ITEMS, Unarmed};
use cr_app::skills::{MOST_POINTS as MOST_SKILL_POINTS, SkillsForm};
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

/// Items' icons by name, made once; `None` for an item without one.
pub type Icons = HashMap<String, Option<EncodedImage>>;

/// What a list's rows show: the name and range of their number, and each
/// one's icon by its title.
#[derive(Clone, Copy)]
pub struct Rows<'a> {
    pub number: (&'a str, u32, u32),
    pub icons: Option<&'a Icons>,
}

impl Rows<'_> {
    /// As a list of `kind` shows them: how many of an item, or a
    /// proficiency's rank.
    pub fn of(kind: ListKind) -> Self {
        Rows {
            number: if kind == ListKind::Items {
                (tr("Quantity"), 1, MOST_ITEMS)
            } else {
                (tr("Rank"), 1, 10)
            },
            icons: None,
        }
    }
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
    list_with(
        kind,
        list,
        details,
        adding,
        Rows::of(kind),
        messages,
        context,
    )
}

/// [`list`] with its rows shown as `shown` has them.
pub fn list_with(
    kind: ListKind,
    list: &List,
    details: &[String],
    adding: &Adding,
    shown: Rows,
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
            let (label, least, most) = shown.number;
            let rank: View = match *count {
                Some(n) => number(
                    &format!("{title}: {label}"),
                    n,
                    (least, most),
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
            let buttons = with_buttons(rank, remove);
            let icon = shown.icons.and_then(|icons| icons.get(title)?.as_ref());
            KeyedView::new(
                format!("{i}\u{1}{title}"),
                match icon {
                    Some(icon) => ui::icon_setting(icon, title, subtitle, buttons),
                    None => ui::setting(title, subtitle, buttons),
                },
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

/// One radio button per choice, in a card; `chosen` makes the message of
/// the one checked.
pub fn choices(
    group: &str,
    rows: &[(String, String)],
    selected: usize,
    chosen: impl Fn(usize) -> Message + Clone + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    let buttons: Vec<KeyedView> = rows
        .iter()
        .enumerate()
        .map(|(i, (title, subtitle))| {
            let chosen = chosen.clone();
            KeyedView::new(
                format!("{i}\u{1}{title}"),
                RadioButton::new()
                    .group_name(group)
                    .automation_name(title.clone())
                    .is_checked(i == selected)
                    .on_checked(context.callback(
                        move |on: bool| {
                            if on { chosen(i) } else { Message::Nothing }
                        },
                    ))
                    .content(two_lines(title, subtitle)),
            )
        })
        .collect();
    ui::card(StackPanel::new().spacing(4.0).keyed_children(buttons))
}

/// The text of warnings that do not stop the form.
pub fn warnings(texts: &[String]) -> View {
    if texts.is_empty() {
        return View::empty();
    }
    InfoBar::new()
        .severity(InfoBarSeverity::Warning)
        .message(texts.join("\n"))
        .is_open(true)
        .is_closable(false)
        .into()
}

/// What the Skills form says: a rogue skill's points, the kit's bonus
/// given, one of its chosen skills ticked, a primary discipline, and its
/// lists' changes.
#[derive(Clone, Copy)]
pub struct SkillsMessages {
    pub points: fn(usize, i32) -> Message,
    pub bonus: fn(usize, usize) -> Message,
    pub chosen: fn(usize, bool) -> Message,
    pub primary: fn(usize, usize) -> Message,
    pub lists: ListMessages,
}

/// The Skills form: the rogue skills, the kit's bonuses and chosen
/// skills, the psionics, the disciplines and powers, then `wild` (the
/// Wild Talents dialog's buttons, if any) with the wild talents, and the
/// psionic attack and defense modes.
pub fn skills(
    form: &SkillsForm,
    adding: &Adding,
    messages: SkillsMessages,
    wild: View,
    context: &mut ViewContext<Shell>,
) -> View {
    let rogue: View = match form.rogue() {
        Some(group) => {
            let rows: Vec<KeyedView> = group
                .rows
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    KeyedView::new(
                        format!("{i}\u{1}{}", row.label),
                        ui::setting(
                            &row.label,
                            &row.subtitle,
                            signed_number(
                                &row.label,
                                row.points,
                                (row.least, MOST_SKILL_POINTS),
                                move |v| (messages.points)(i, v),
                                context,
                            ),
                        ),
                    )
                })
                .collect();
            StackPanel::new().spacing(4.0).children((
                heading(tr("Rogue Skills"), Some(&group.description)),
                ui::setting(
                    tr("Points Left"),
                    "",
                    ui::subtitle(group.left.to_string().replace('-', "−")),
                ),
                StackPanel::new().spacing(4.0).keyed_children(rows),
                warnings(&group.warnings),
            ))
        }
        None => View::empty(),
    };
    let bonuses: Vec<KeyedView> = form
        .kit_bonuses()
        .into_iter()
        .enumerate()
        .map(|(g, group)| {
            let rows: Vec<(String, String)> = group
                .names
                .iter()
                .map(|n| (n.clone(), String::new()))
                .collect();
            KeyedView::new(
                format!("{g}\u{1}{}", group.title),
                StackPanel::new().spacing(4.0).children((
                    heading(&group.title, Some(&group.description)),
                    choices(
                        &format!("kit-bonus-{g}"),
                        &rows,
                        group.picked.unwrap_or(usize::MAX),
                        move |i| (messages.bonus)(g, i),
                        context,
                    ),
                )),
            )
        })
        .collect();
    let chosen: View = match form.chosen() {
        Some(group) => {
            let full = group.full;
            let rows: Vec<(String, String, bool, bool)> = group
                .names
                .iter()
                .zip(&group.ticked)
                .map(|(n, &on)| (n.clone(), String::new(), on, on || !full))
                .collect();
            StackPanel::new().spacing(4.0).children((
                heading(&group.title, Some(&group.description)),
                checks(&rows, messages.chosen, context),
            ))
        }
        None => View::empty(),
    };
    let psionics: View = match form.psionics() {
        Some(description) => {
            let primary: Vec<KeyedView> = form
                .primary()
                .into_iter()
                .enumerate()
                .map(|(slot, choice)| {
                    KeyedView::new(
                        slot.to_string(),
                        ui::setting(
                            &choice.title,
                            "",
                            ComboBox::new()
                                .automation_name(choice.title.clone())
                                .items_source(choice.items.clone())
                                .selected_index(choice.selected)
                                .min_width(240.0)
                                .on_selection_changed(context.callback(move |i: Option<usize>| {
                                    match i {
                                        Some(i) => (messages.primary)(slot, i),
                                        None => Message::Nothing,
                                    }
                                })),
                        ),
                    )
                })
                .collect();
            StackPanel::new().spacing(4.0).children((
                heading(tr("Psionics"), Some(&description)),
                StackPanel::new().spacing(4.0).keyed_children(primary),
            ))
        }
        None => View::empty(),
    };
    let lists = |kinds: &[ListKind], context: &mut ViewContext<Shell>| -> View {
        let lists: Vec<KeyedView> = kinds
            .iter()
            .map(|&kind| {
                KeyedView::new(
                    format!("{kind:?}"),
                    list(kind, &form.list(kind), &[], adding, messages.lists, context),
                )
            })
            .collect();
        StackPanel::new().spacing(4.0).keyed_children(lists)
    };
    let powers = lists(
        &[
            ListKind::Disciplines,
            ListKind::Sciences,
            ListKind::Devotions,
        ],
        context,
    );
    let talents = lists(&[ListKind::WildTalents], context);
    let modes = lists(&[ListKind::Attacks, ListKind::Defenses], context);
    StackPanel::new().spacing(4.0).children((
        rogue,
        StackPanel::new().spacing(4.0).keyed_children(bonuses),
        chosen,
        psionics,
        powers,
        wild,
        talents,
        modes,
    ))
}
