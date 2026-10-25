//! A character's class levels on Windows: Enter Character's levels on its
//! details page, with the hit point and PSP boxes that stand in for the
//! rolls, and the Levels dialog with its Add Experience.

use cr_app::create::{EnteredLevels, Input, MOST_HIT_POINTS};
use cr_app::levels::{ExperienceForm, LevelsForm, LevelsInput, MOST_EXPERIENCE, MOST_LEVEL};
use cr_i18n::tr;
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::ui;

/// A wizard input as the shell's message.
fn input(input: Input) -> Message {
    Message::Wizard(input)
}

/// A NumberBox for a whole number in `min..=max`.
fn number(
    name: &str,
    value: i64,
    (min, max): (i64, i64),
    changed: impl Fn(i64) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> NumberBox {
    NumberBox::new()
        .automation_name(name)
        .value(value as f64)
        .minimum(min as f64)
        .maximum(max as f64)
        .min_width(120.0)
        .on_value_changed(context.callback(move |v: Option<f64>| match v {
            Some(v) => changed((v.round() as i64).clamp(min, max)),
            None => Message::Nothing,
        }))
}

/// Enter Character's Levels group: each class with its level, Levels…,
/// and the hit points and PSPs.
pub fn group(levels: &EnteredLevels, context: &mut ViewContext<Shell>) -> View {
    let open = Button::new()
        .is_enabled(levels.error.is_none())
        .on_click(context.message(input(Input::OpenLevels)))
        .content(plain(tr("Levels…")));
    let heading = Grid::new()
        .margin(Thickness::new(0.0, 16.0, 0.0, 4.0))
        .columns([GridLength::STAR, GridLength::Auto])
        .children((
            ui::strong(tr("Levels")).vertical_alignment(VerticalAlignment::Center),
            Border::new().grid_column(1).content(open),
        ));
    if let Some(error) = &levels.error {
        return StackPanel::new()
            .spacing(4.0)
            .children((heading, ui::card(ui::body(error.clone()).opacity(0.6))));
    }
    let classes: Vec<KeyedView> = levels
        .classes
        .iter()
        .enumerate()
        .map(|(g, class)| {
            KeyedView::new(
                format!("{g}\u{1}{}", class.name),
                ui::setting(&class.name, &class.text, View::empty()),
            )
        })
        .collect();
    let hit_points: View = match levels.hit_points {
        Some(hp) => ui::setting(
            tr("Hit Points"),
            tr("Changes the rolls of the highest level"),
            number(
                tr("Hit Points"),
                hp.into(),
                (1, MOST_HIT_POINTS.into()),
                |v| input(Input::HitPoints(v as u32)),
                context,
            ),
        ),
        None => View::empty(),
    };
    let psp: View = match levels.psp {
        Some(psp) => ui::setting(
            tr("PSPs"),
            tr("Changes those of the highest level that has them"),
            number(
                tr("PSPs"),
                psp.into(),
                (0, MOST_HIT_POINTS.into()),
                |v| input(Input::Psp(v as u32)),
                context,
            ),
        ),
        None => View::empty(),
    };
    let dialog: View = match &levels.dialog {
        Some(form) => dialog(
            form,
            |change| input(Input::Levels(change)),
            input(Input::CloseLevels),
            context,
        ),
        None => View::empty(),
    };
    StackPanel::new().spacing(4.0).children((
        heading,
        StackPanel::new().spacing(4.0).keyed_children(classes),
        hit_points,
        psp,
        dialog,
    ))
}

/// The Levels dialog (IDD 368): each class's level, the prime requisite
/// bonus, the level limits and, with Skills & Powers, the character
/// points; Add Experience opens in it. `map` makes the message of a
/// change, `close` the one of Close.
pub fn dialog(
    form: &LevelsForm,
    map: fn(LevelsInput) -> Message,
    close: Message,
    context: &mut ViewContext<Shell>,
) -> View {
    let classes: Vec<KeyedView> = form
        .classes
        .iter()
        .enumerate()
        .map(|(g, class)| {
            KeyedView::new(
                format!("{g}\u{1}{}", class.name),
                ui::setting(
                    &class.name,
                    &class.text,
                    number(
                        &class.name,
                        class.level.into(),
                        (1, MOST_LEVEL.into()),
                        move |v| map(LevelsInput::Level(g, v as u32)),
                        context,
                    ),
                ),
            )
        })
        .collect();
    let limits: View = if form.limits_shown {
        ui::setting(
            tr("Level Limits"),
            "",
            ComboBox::new()
                .automation_name(tr("Level Limits"))
                .items_source(vec![
                    tr("None").to_owned(),
                    tr("Racial Limits").to_owned(),
                    tr("Racial Limits and Prime Requisite Bonus").to_owned(),
                ])
                .selected_index(form.limits)
                .min_width(240.0)
                .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                    Some(i) => map(LevelsInput::Limits(i)),
                    None => Message::Nothing,
                })),
        )
    } else {
        View::empty()
    };
    let points: View = if form.skills_and_powers {
        StackPanel::new().spacing(4.0).children((
            ui::setting(
                tr("Character Points"),
                tr("Unspent"),
                number(
                    tr("Character Points"),
                    form.character_points.into(),
                    (0, 9999),
                    move |v| map(LevelsInput::CharacterPoints(v as u32)),
                    context,
                ),
            ),
            ui::setting(
                tr("Character Points per Level"),
                "",
                number(
                    tr("Character Points per Level"),
                    form.points_per_level.into(),
                    (0, 10),
                    move |v| map(LevelsInput::PointsPerLevel(v as u32)),
                    context,
                ),
            ),
        ))
    } else {
        View::empty()
    };
    let notice: View = match &form.notice {
        Some(text) => InfoBar::new()
            .severity(InfoBarSeverity::Informational)
            .message(text.clone())
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    };
    let experience: View = match &form.experience {
        Some(experience) => ui::card(
            StackPanel::new().spacing(8.0).children((
                ui::strong(tr("Add Experience")),
                experience_fields(experience, map, context),
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(8.0)
                    .horizontal_alignment(HorizontalAlignment::Right)
                    .children((
                        Button::new()
                            .on_click(context.message(map(LevelsInput::Cancel)))
                            .content(plain(tr("_Cancel"))),
                        Button::new()
                            .style(ButtonStyle::Accent)
                            .on_click(context.message(map(LevelsInput::Give)))
                            .content(plain(tr("_Add"))),
                    )),
            )),
        ),
        None => Button::new()
            .on_click(context.message(map(LevelsInput::AddExperience)))
            .content(plain(tr("Add Experience…"))),
    };
    let content = StackPanel::new().spacing(4.0).children((
        ui::caption(tr(
            "A level puts the class at the experience it takes; each level gained rolls its hit \
             points and PSPs",
        )),
        StackPanel::new().spacing(4.0).keyed_children(classes),
        experience,
        notice,
        ui::setting(
            tr("Apply Prime Requisite Experience Bonuses"),
            tr("10% more for a class whose prime requisites are all 16 or more"),
            ToggleSwitch::new()
                .automation_name(tr("Apply Prime Requisite Experience Bonuses"))
                .is_on(form.prime_bonus)
                .on_toggled(context.callback(move |on| map(LevelsInput::PrimeBonus(on)))),
        ),
        limits,
        points,
    ));
    ContentDialog::new()
        .title(tr("Levels"))
        .close_button_text(plain(tr("_Close")))
        .is_open(true)
        .on_closed(context.callback(move |_| close.clone()))
        .content(ScrollViewer::new().max_height(560.0).content(content))
}

/// Add Experience's amount, the classes it goes to, its own prime
/// requisite bonus and points per level when it has them, and why it
/// could not be given.
fn experience_fields(
    experience: &ExperienceForm,
    map: fn(LevelsInput) -> Message,
    context: &mut ViewContext<Shell>,
) -> View {
    let error: View = match &experience.error {
        Some(text) => InfoBar::new()
            .severity(InfoBarSeverity::Error)
            .message(text.clone())
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    };
    let settings: View = match &experience.settings {
        Some(settings) => {
            let points: View = if settings.skills_and_powers {
                ui::setting(
                    tr("Character Points per Level"),
                    "",
                    number(
                        tr("Character Points per Level"),
                        settings.points_per_level.into(),
                        (0, 10),
                        move |v| map(LevelsInput::PointsPerLevel(v as u32)),
                        context,
                    ),
                )
            } else {
                View::empty()
            };
            StackPanel::new().spacing(4.0).children((
                ui::setting(
                    tr("Prime Requisite Bonus"),
                    tr("10% more for a class whose prime requisites are all 16 or more"),
                    ToggleSwitch::new()
                        .automation_name(tr("Prime Requisite Bonus"))
                        .is_on(settings.prime_bonus)
                        .on_toggled(context.callback(move |on| map(LevelsInput::PrimeBonus(on)))),
                ),
                points,
            ))
        }
        None => View::empty(),
    };
    StackPanel::new().spacing(4.0).children((
        ui::caption(tr(
            "Each level gained rolls its hit points. Levels lost keep their rolls, as in the \
             original.",
        )),
        ui::setting(
            tr("Experience"),
            tr("Negative to take experience away"),
            number(
                tr("Experience"),
                experience.amount,
                (-MOST_EXPERIENCE, MOST_EXPERIENCE),
                move |v| map(LevelsInput::Amount(v)),
                context,
            ),
        ),
        ui::setting(
            tr("Give To"),
            "",
            ComboBox::new()
                .automation_name(tr("Give To"))
                .items_source(experience.targets.clone())
                .selected_index(experience.target)
                .min_width(220.0)
                .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                    Some(i) => map(LevelsInput::Target(i)),
                    None => Message::Nothing,
                })),
        ),
        settings,
        error,
    ))
}

/// Add Experience as a dialog of its own, as the editor's Add… opens it:
/// Add gives the experience, Cancel closes it.
pub fn experience_dialog(
    experience: &ExperienceForm,
    map: fn(LevelsInput) -> Message,
    context: &mut ViewContext<Shell>,
) -> View {
    ContentDialog::new()
        .title(tr("Add Experience"))
        .primary_button_text(plain(tr("_Add")))
        .close_button_text(plain(tr("_Cancel")))
        .is_open(true)
        .on_closed(context.callback(move |result| match result {
            ContentDialogResult::Primary => map(LevelsInput::Give),
            _ => map(LevelsInput::Cancel),
        }))
        .content(
            ScrollViewer::new()
                .max_height(560.0)
                .content(experience_fields(experience, map, context)),
        )
}
