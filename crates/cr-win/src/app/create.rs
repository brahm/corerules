//! New Character on a page of the window: the wizard of
//! `cr_app::create`, one of its pages at a time, with Back and Next at the
//! foot and its questions as dialogs.

use cr_app::create::{
    AbilitiesPage, Answer, Choice, ClassAbilitiesPage, ClassPage, CombatPage, DetailsPage,
    EnteredClassPage, EnteredScoresPage, EquipmentPage, Input, List, ListKind, MOST_ITEMS, Page,
    PickGroup, ProficienciesPage, PromptKind, RacePage, RacialPage, RoguePage, SOCIAL_CLASSES,
    SettingPage, SpellsPage, Unarmed, WildPage, Wizard, none_label,
};
use cr_app::skills::{MOST_POINTS, SkillsForm};
use cr_i18n::{tr, trn};
use cr_store::create::{
    ABILITIES, ENTERED_SCORES, METHOD_VI_DICE, METHOD_X_STRENGTH_STEP, UNARMED,
};
use cr_store::settings::SETTINGS;
use cr_win::text::plain;
use windows_reactor::*;

use super::portrait::Shown;
use super::{Message, Shell};
use crate::ui;

/// A wizard input as the shell's message.
fn input(input: Input) -> Message {
    Message::Wizard(input)
}

/// What the box of each list has chosen for Add, until it is added.
pub type Adding = Vec<(ListKind, String)>;

/// The wizard's page; `portrait` is the one the details page shows.
pub fn view(
    wizard: &Wizard,
    adding: &Adding,
    portrait: Option<&Shown>,
    context: &mut ViewContext<Shell>,
) -> View {
    let page = wizard.page();
    let content: View = match page {
        Page::Setting(page) => setting(page, context),
        Page::Abilities(page) => abilities(page, context),
        Page::EnteredScores(page) => entered_scores(page, context),
        Page::Race(page) => race(page, context),
        Page::Racial(page) => racial(page, context),
        Page::Class(page) => class(page, context),
        Page::EnteredClass(page) => entered_class(page, context),
        Page::ClassAbilities(page) => class_abilities(page, context),
        Page::Details(page) => details(page, portrait, context),
        Page::Proficiencies(page) => proficiencies(page, adding, context),
        Page::Combat(page) => combat(page, adding, context),
        Page::Rogue(page) => rogue(page, context),
        Page::Spells(page) => spells(page, adding, context),
        Page::Wild(page) => wild(page, adding, context),
        Page::Psionics(page) => skills_form(&page.form, adding, context),
        Page::Equipment(page) => equipment(page, adding, context),
    };
    let error: View = match &wizard.error {
        Some(error) => InfoBar::new()
            .severity(InfoBarSeverity::Error)
            .title(error.clone())
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    };
    let first = wizard.depth() == 1;
    let buttons = StackPanel::new()
        .margin(Thickness::new(0.0, 16.0, 0.0, 0.0))
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Right)
        .children((
            Button::new()
                .on_click(context.message(Message::Back))
                .content(plain(if first { tr("_Cancel") } else { tr("Back") })),
            Button::new()
                .style(ButtonStyle::Accent)
                .is_enabled(page.can_go_on())
                .on_click(context.message(input(Input::Next)))
                .content(plain(if matches!(page, Page::Equipment(_)) {
                    tr("C_reate")
                } else {
                    tr("_Next")
                })),
        ));
    // Each question is a dialog of its own, so that one asked right after
    // another opens.
    let dialog: Vec<KeyedView> = wizard
        .prompt()
        .map(|prompt| {
            KeyedView::new(
                wizard.prompt_serial().to_string(),
                prompt_dialog(
                    prompt,
                    wizard.prompt_serial(),
                    Message::WizardAnswer,
                    context,
                ),
            )
        })
        .into_iter()
        .collect();
    Grid::new().children((
        ui::page(StackPanel::new().spacing(4.0).children((
            ui::title(page.title(wizard.entering())).margin(Thickness::new(0.0, 0.0, 0.0, 16.0)),
            content,
            error,
            buttons,
        ))),
        Grid::new().keyed_children(dialog),
    ))
}

/// The question in view as a ContentDialog, the wizard's or the
/// editor's; `answer` makes the message of its answers, which carry
/// `serial`, the prompt's number.
pub(super) fn prompt_dialog<T>(
    prompt: &cr_app::prompt::Prompt<T>,
    serial: u64,
    answer: fn(u64, Answer) -> Message,
    context: &mut ViewContext<Shell>,
) -> View {
    let text = ui::body(prompt.text.clone()).is_text_selection_enabled(true);
    let content: View = match &prompt.kind {
        PromptKind::RaiseOne { labels } => {
            let buttons: Vec<KeyedView> = labels
                .iter()
                .enumerate()
                .map(|(a, label)| {
                    KeyedView::new(
                        a.to_string(),
                        Button::new()
                            .horizontal_alignment(HorizontalAlignment::Stretch)
                            .on_click(context.message(answer(serial, Answer::Choice(a))))
                            .content(label.clone()),
                    )
                })
                .collect();
            StackPanel::new()
                .spacing(12.0)
                .children((text, StackPanel::new().spacing(6.0).keyed_children(buttons)))
        }
        PromptKind::CoreVsOption { off } => StackPanel::new().spacing(12.0).children((
            text,
            CheckBox::new()
                .is_checked(*off)
                .on_is_checked_changed(context.callback(|on| input(Input::PromptOff(on))))
                .content(plain(tr("_Do not show this window again"))),
        )),
        PromptKind::AllClasses { groups, chosen } => {
            let rows: Vec<KeyedView> = groups
                .iter()
                .zip(chosen)
                .enumerate()
                .filter(|(_, ((_, classes), _))| !classes.is_empty())
                .map(|(g, ((name, classes), &c))| {
                    let items =
                        std::iter::once(tr("None").to_owned()).chain(classes.iter().cloned());
                    KeyedView::new(
                        g.to_string(),
                        StackPanel::new().spacing(2.0).children((
                            ui::caption(tr(name)),
                            ComboBox::new()
                                .automation_name(tr(name))
                                .items_source(items)
                                .selected_index(c)
                                .min_width(240.0)
                                .on_selection_changed(context.callback(move |i: Option<usize>| {
                                    input(Input::PromptGroup(g, i.unwrap_or(0)))
                                })),
                        )),
                    )
                })
                .collect();
            StackPanel::new()
                .spacing(12.0)
                .children((text, StackPanel::new().spacing(8.0).keyed_children(rows)))
        }
        _ => text.into(),
    };
    let (primary, close) = match &prompt.kind {
        // A message closes with OK, a choice with its own buttons.
        PromptKind::Message | PromptKind::CoreVsOption { .. } => {
            (String::new(), plain(prompt.yes_label()))
        }
        PromptKind::RaiseOne { .. } => (String::new(), String::new()),
        _ => (
            plain(prompt.yes_label()),
            prompt.no_label().map(plain).unwrap_or_default(),
        ),
    };
    ContentDialog::new()
        .title(prompt.title.clone())
        .primary_button_text(primary)
        .close_button_text(close)
        .is_open(true)
        .on_closed(context.callback(move |result| {
            answer(
                serial,
                match result {
                    ContentDialogResult::Primary => Answer::Yes,
                    _ => Answer::No,
                },
            )
        }))
        .content(ScrollViewer::new().max_height(480.0).content(content))
}

/// A heading over a group of cards, with the group's description.
pub(super) fn heading(title: &str, description: Option<&str>) -> View {
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
fn two_lines(title: &str, subtitle: &str) -> View {
    if subtitle.is_empty() {
        return ui::body(title).into();
    }
    StackPanel::new()
        .spacing(2.0)
        .children((ui::body(title), ui::caption(subtitle)))
}

/// One radio button per choice, in a card; `chosen` makes the message of
/// the one checked.
fn choices(
    group: &str,
    rows: &[(String, String)],
    selected: usize,
    chosen: impl Fn(usize) -> Input + Clone + 'static,
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
                    .on_checked(context.callback(move |on: bool| {
                        if on {
                            input(chosen(i))
                        } else {
                            Message::Nothing
                        }
                    }))
                    .content(two_lines(title, subtitle)),
            )
        })
        .collect();
    ui::card(StackPanel::new().spacing(4.0).keyed_children(buttons))
}

/// A whole number from a NumberBox, within `min..=max`.
fn whole(value: Option<f64>, min: u32, max: u32) -> Option<u32> {
    value.map(|v| (v.round().max(0.0) as u32).clamp(min, max))
}

/// A NumberBox for a whole number in `min..=max`; `changed` makes the
/// message of a new value.
fn number(
    name: &str,
    value: u32,
    (min, max): (u32, u32),
    changed: impl Fn(u32) -> Input + 'static,
    context: &mut ViewContext<Shell>,
) -> NumberBox {
    NumberBox::new()
        .automation_name(name)
        .value(f64::from(value))
        .minimum(f64::from(min))
        .maximum(f64::from(max))
        .min_width(120.0)
        .on_value_changed(context.callback(move |v| match whole(v, min, max) {
            Some(v) => input(changed(v)),
            None => Message::Nothing,
        }))
}

/// A button with the Refresh symbol that rolls again.
fn roll(name: &str, what: Input, context: &mut ViewContext<Shell>) -> View {
    ui::icon_button(name, Symbol::Refresh, input(what), context)
}

/// A control and the buttons after it, in a row.
pub(super) fn with_buttons(control: impl Into<View>, buttons: impl Into<View>) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .children((control.into(), buttons.into()))
}

fn setting(page: &SettingPage, context: &mut ViewContext<Shell>) -> View {
    let rules_text = if page.skills_and_powers {
        tr(
            "Skills & Powers: subabilities, and character points for racial and \
            class abilities, proficiencies and traits",
        )
    } else {
        tr("Player's Handbook: proficiency slots")
    };
    // Core Rules 2.0 has no Player's Option.
    let rules: View = if page.expansion {
        StackPanel::new().spacing(4.0).children((
            ui::strong(tr("Rules")),
            ui::caption(tr("The original's Core Rules and Player's Option buttons")),
            ui::card(
                StackPanel::new().spacing(8.0).children((
                    RadioButtons::new()
                        .items_source([plain(tr("Core Rules")), plain(tr("Player's Option"))])
                        .selected_index(usize::from(page.skills_and_powers))
                        .on_selection_changed(context.callback(|i: Option<usize>| match i {
                            Some(i) => input(Input::Rules(i)),
                            None => Message::Nothing,
                        })),
                    ui::caption(rules_text),
                )),
            ),
        ))
    } else {
        View::empty()
    };
    let chosen = &SETTINGS[page.setting.min(SETTINGS.len() - 1)];
    StackPanel::new().spacing(20.0).children((
        rules,
        StackPanel::new().spacing(4.0).children((
            ui::strong(tr("Campaign Setting")),
            ui::caption(tr(
                "The setting limits the races and classes offered on the next pages. \
                 The original program has no settings; these follow each setting's \
                 campaign book.",
            )),
            ui::card(
                StackPanel::new().spacing(8.0).children((
                    RadioButtons::new()
                        .items_source(SETTINGS.iter().map(|s| tr(s.name).to_owned()))
                        .selected_index(page.setting)
                        .on_selection_changed(context.callback(|i: Option<usize>| match i {
                            Some(i) => input(Input::Setting(i)),
                            None => Message::Nothing,
                        })),
                    ui::caption(tr(chosen.description)),
                )),
            ),
        )),
    ))
}

fn abilities(page: &AbilitiesPage, context: &mut ViewContext<Shell>) -> View {
    let names: Vec<String> = page
        .methods
        .iter()
        .map(|m| tr(m.name()).to_owned())
        .collect();
    let method = ui::card(
        StackPanel::new().spacing(8.0).children((
            ComboBox::new()
                .automation_name(tr("Method"))
                .items_source(names)
                .selected_index(page.methods.iter().position(|m| *m == page.method))
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .on_selection_changed(context.callback(|i: Option<usize>| match i {
                    Some(i) => input(Input::Method(i)),
                    None => Message::Nothing,
                })),
            ui::caption(tr(page.method.description())),
        )),
    );
    if page.later() {
        return method;
    }
    let roll_again: View = if page.rolls() {
        Button::new()
            .on_click(context.message(input(Input::Roll)))
            .content(tr("Roll Again"))
    } else {
        View::empty()
    };
    let rows: Vec<KeyedView> = (0..6)
        .flat_map(|a| {
            let name = tr(ABILITIES[a]);
            let control: View = if page.typed_in() {
                number(
                    name,
                    page.typed[a],
                    (3, page.highest()),
                    move |v| Input::Score(a, v),
                    context,
                )
                .into()
            } else if page.pooled() {
                let score = if page.pool_rolled {
                    page.scores[a].to_string()
                } else {
                    "–".to_owned()
                };
                with_buttons(
                    number(
                        &trn!("{n} die", "{n} dice", page.pool[a]),
                        page.pool[a],
                        (3, 6),
                        move |v| Input::PoolDice(a, v),
                        context,
                    ),
                    ui::subtitle(score).min_width(40.0),
                )
            } else {
                let arrows: View = if page.method.arranged() {
                    StackPanel::new()
                        .orientation(Orientation::Horizontal)
                        .spacing(4.0)
                        .children((
                            arrow(tr("Move Up"), Symbol::Up, a, a.wrapping_sub(1), context),
                            arrow(tr("Move Down"), Symbol::Download, a, a + 1, context),
                        ))
                } else {
                    View::empty()
                };
                with_buttons(
                    ui::subtitle(page.scores[a].to_string()).min_width(40.0),
                    arrows,
                )
            };
            let description = if page.pooled() {
                trn!("{n} die", "{n} dice", page.pool[a])
            } else {
                String::new()
            };
            let mut rows = vec![KeyedView::new(
                format!("a{a}"),
                ui::setting(name, &description, control),
            )];
            if a == 0 && page.exceptional_shown() {
                rows.push(KeyedView::new(
                    "exceptional",
                    ui::setting(
                        tr("Exceptional Strength"),
                        tr("For warriors with Strength 18, 10% a point"),
                        number(
                            tr("Exceptional Strength"),
                            page.exceptional,
                            (0, 100 / METHOD_X_STRENGTH_STEP),
                            Input::Exceptional,
                            context,
                        ),
                    ),
                ));
            }
            rows
        })
        .collect();
    let seven: View = if page.method == cr_store::create::RollMethod::SevenDice {
        let ability_names: Vec<String> = ABILITIES.iter().map(|a| tr(a).to_owned()).collect();
        let dice: Vec<KeyedView> = (0..METHOD_VI_DICE)
            .map(|d| {
                let die = tr!("Die {number}", number = d + 1);
                KeyedView::new(
                    d.to_string(),
                    ui::setting(
                        &die,
                        &tr!("Rolled {roll}", roll = page.seven[d]),
                        ComboBox::new()
                            .automation_name(die.clone())
                            .items_source(ability_names.clone())
                            .selected_index(page.seven_to[d])
                            .min_width(160.0)
                            .on_selection_changed(context.callback(
                                move |i: Option<usize>| match i {
                                    Some(a) => input(Input::SevenDie(d, a)),
                                    None => Message::Nothing,
                                },
                            )),
                    ),
                )
            })
            .collect();
        StackPanel::new().spacing(4.0).children((
            heading(
                tr("Dice"),
                Some(tr("Each die is added to the ability chosen for it")),
            ),
            StackPanel::new().spacing(4.0).keyed_children(dice),
        ))
    } else {
        View::empty()
    };
    StackPanel::new().spacing(4.0).children((
        method,
        Grid::new()
            .columns([GridLength::STAR, GridLength::Auto])
            .children((
                heading(tr("Ability Scores"), page.description.as_deref()),
                Border::new()
                    .grid_column(1)
                    .vertical_alignment(VerticalAlignment::Bottom)
                    .margin(Thickness::new(0.0, 0.0, 0.0, 4.0))
                    .content(roll_again),
            )),
        StackPanel::new().spacing(4.0).keyed_children(rows),
        seven,
    ))
}

/// Move Up or Move Down on a rolled score.
fn arrow(
    name: &str,
    symbol: Symbol,
    from: usize,
    to: usize,
    context: &mut ViewContext<Shell>,
) -> View {
    let shown = to < 6;
    Button::new()
        .automation_name(name)
        .is_enabled(shown)
        .on_click(context.message(input(Input::Swap(from, to.min(5)))))
        .content(SymbolIcon::new().symbol(symbol))
        .tooltip(name)
}

fn entered_scores(page: &EnteredScoresPage, context: &mut ViewContext<Shell>) -> View {
    let rows: Vec<KeyedView> = (0..6)
        .flat_map(|a| {
            let name = tr(ABILITIES[a]);
            let mut rows = vec![KeyedView::new(
                format!("a{a}"),
                ui::setting(
                    name,
                    "",
                    number(
                        name,
                        page.scores[a],
                        ENTERED_SCORES,
                        move |v| Input::Score(a, v),
                        context,
                    ),
                ),
            )];
            if a == 0 && page.exceptional_shown() {
                rows.push(KeyedView::new(
                    "exceptional",
                    ui::setting(
                        tr("Exceptional Strength"),
                        tr("1 to 99 for 18/01 to 18/99, 100 for 18/00, 0 for none"),
                        number(
                            tr("Exceptional Strength"),
                            page.exceptional,
                            (0, 100),
                            Input::Exceptional,
                            context,
                        ),
                    ),
                ));
            }
            rows
        })
        .collect();
    StackPanel::new().spacing(4.0).children((
        heading(
            tr("Set your ability scores"),
            Some(tr(
                "The scores are kept as entered, with no racial adjustments",
            )),
        ),
        StackPanel::new().spacing(4.0).keyed_children(rows),
    ))
}

fn race(page: &RacePage, context: &mut ViewContext<Shell>) -> View {
    let races: View = if page.races.is_empty() {
        ui::card(two_lines(
            tr("No race allows these scores"),
            tr("Go back and roll again"),
        ))
    } else {
        choices("race", &page.races, page.chosen, Input::Race, context)
    };
    let subraces: View = if page.subraces_shown() {
        ui::setting(
            tr("Subrace"),
            "",
            ComboBox::new()
                .automation_name(tr("Subrace"))
                .items_source(page.subraces.clone())
                .selected_index(page.subrace)
                .min_width(220.0)
                .on_selection_changed(context.callback(|i: Option<usize>| match i {
                    Some(i) => input(Input::Subrace(i)),
                    None => Message::Nothing,
                })),
        )
    } else {
        View::empty()
    };
    StackPanel::new().spacing(4.0).children((
        ui::setting(
            tr("Standard Races Only"),
            tr("Leave out the monster races, as the original's switch does"),
            ToggleSwitch::new()
                .automation_name(tr("Standard Races Only"))
                .is_on(page.standard_only)
                .on_toggled(context.callback(|on| input(Input::StandardOnly(on)))),
        ),
        heading(tr("Race"), Some(&page.description)),
        races,
        subraces,
    ))
}

/// Check boxes for the abilities of a group, with their costs.
fn ability_checks(
    slot: usize,
    rows: &[(String, String)],
    choices: &[(String, i32)],
    chosen: &[String],
    context: &mut ViewContext<Shell>,
) -> View {
    let checks: Vec<KeyedView> = rows
        .iter()
        .zip(choices)
        .enumerate()
        .map(|(i, ((title, cost), (name, _)))| {
            KeyedView::new(
                format!("{i}\u{1}{name}"),
                CheckBox::new()
                    .automation_name(title.clone())
                    .is_checked(chosen.contains(name))
                    .on_is_checked_changed(
                        context.callback(move |on| input(Input::Ability(slot, i, on))),
                    )
                    .content(two_lines(title, cost)),
            )
        })
        .collect();
    ui::card(StackPanel::new().spacing(4.0).keyed_children(checks))
}

fn racial(page: &RacialPage, context: &mut ViewContext<Shell>) -> View {
    let list: View = if page.fixed() {
        let names: Vec<KeyedView> = page
            .chosen
            .iter()
            .enumerate()
            .map(|(i, name)| {
                KeyedView::new(i.to_string(), ui::body(cr_app::create::ability_title(name)))
            })
            .collect();
        if names.is_empty() {
            ui::card(ui::body(tr("None")).opacity(0.6))
        } else {
            ui::card(StackPanel::new().spacing(6.0).keyed_children(names))
        }
    } else {
        ability_checks(0, &page.rows, &page.choices, &page.chosen, context)
    };
    StackPanel::new().spacing(4.0).children((
        heading(tr("Racial Abilities"), Some(&page.description)),
        list,
    ))
}

fn class(page: &ClassPage, context: &mut ViewContext<Shell>) -> View {
    let list: View = if page.options.is_empty() {
        ui::card(two_lines(
            tr("No class allows these scores"),
            tr("Go back and choose another race or roll again"),
        ))
    } else {
        choices("class", &page.rows, page.chosen, Input::Class, context)
    };
    let scores: View = if page.choice && !page.options.is_empty() {
        StackPanel::new().spacing(4.0).children((
            Grid::new()
                .columns([GridLength::STAR, GridLength::Auto])
                .children((
                    heading(tr("Ability Scores"), None),
                    Border::new()
                        .grid_column(1)
                        .vertical_alignment(VerticalAlignment::Bottom)
                        .margin(Thickness::new(0.0, 0.0, 0.0, 4.0))
                        .content(
                            Button::new()
                                .on_click(context.message(input(Input::Roll)))
                                .content(tr("Roll Again")),
                        ),
                )),
            ui::card(two_lines(&page.rolled_text.0, &page.rolled_text.1)),
        ))
    } else {
        View::empty()
    };
    StackPanel::new().spacing(4.0).children((
        heading(tr("Class"), Some(&page.description)),
        list,
        scores,
    ))
}

fn entered_class(page: &EnteredClassPage, context: &mut ViewContext<Shell>) -> View {
    StackPanel::new().spacing(4.0).children((
        Grid::new()
            .columns([GridLength::STAR, GridLength::Auto])
            .children((
                heading(tr("Class"), Some(&page.race)),
                Border::new()
                    .grid_column(1)
                    .vertical_alignment(VerticalAlignment::Bottom)
                    .margin(Thickness::new(0.0, 0.0, 0.0, 4.0))
                    .content(
                        Button::new()
                            .on_click(context.message(input(Input::AllClasses)))
                            .content(tr("All Classes…"))
                            .tooltip(tr("Any combination, one class of each group")),
                    ),
            )),
        choices("class", &page.rows, page.chosen, Input::Class, context),
    ))
}

fn class_abilities(page: &ClassAbilitiesPage, context: &mut ViewContext<Shell>) -> View {
    let groups: Vec<KeyedView> = page
        .groups
        .iter()
        .enumerate()
        .map(|(slot, group)| {
            let chosen = page.chosen.get(slot).map(Vec::as_slice).unwrap_or_default();
            KeyedView::new(
                format!("{slot}\u{1}{}", group.title),
                StackPanel::new().spacing(4.0).children((
                    heading(&group.title, Some(&group.description)),
                    ability_checks(slot, &group.rows, &group.choices, chosen, context),
                )),
            )
        })
        .collect();
    StackPanel::new().spacing(4.0).keyed_children(groups)
}

fn details(page: &DetailsPage, portrait: Option<&Shown>, context: &mut ViewContext<Shell>) -> View {
    let text =
        |name: &str, value: &str, make: fn(String) -> Input, context: &mut ViewContext<Shell>| {
            TextBox::new()
                .automation_name(name)
                .text(value)
                .min_width(260.0)
                .on_text_changed(context.callback(move |text: String| input(make(text))))
        };
    let combo = |name: &str,
                 items: Vec<String>,
                 selected: Option<usize>,
                 make: fn(usize) -> Input,
                 context: &mut ViewContext<Shell>| {
        ComboBox::new()
            .automation_name(name)
            .items_source(items)
            .selected_index(selected)
            .min_width(220.0)
            .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                Some(i) => input(make(i)),
                None => Message::Nothing,
            }))
    };
    let name_box = text(tr("Name"), &page.name, Input::Name, context);
    let name: View = if page.random_names {
        with_buttons(
            name_box,
            roll(tr("Random Name"), Input::RandomName, context),
        )
    } else {
        name_box.into()
    };
    let player = text(tr("Player"), &page.player, Input::Player, context);
    let genders = vec![tr("Male").to_owned(), tr("Female").to_owned()];
    let gender = combo(
        tr("Gender"),
        genders,
        Some(usize::from(!page.male)),
        Input::Gender,
        context,
    );
    let alignment = combo(
        tr("Alignment"),
        page.alignments.iter().map(|a| tr(a).to_owned()).collect(),
        page.alignment_index(),
        Input::Alignment,
        context,
    );
    let kit = combo(
        tr("Kit"),
        page.kits.iter().map(|k| none_label(k)).collect(),
        page.kit_index(),
        Input::Kit,
        context,
    );
    let social = with_buttons(
        combo(
            tr("Social Class"),
            SOCIAL_CLASSES.iter().map(|s| tr(s).to_owned()).collect(),
            Some(page.social_class),
            Input::SocialClass,
            context,
        ),
        roll(tr("Roll Social Class"), Input::RollSocialClass, context),
    );
    let background: View = if page.backgrounds.is_empty() {
        View::empty()
    } else {
        ui::setting(
            tr("Background Event"),
            "",
            with_buttons(
                combo(
                    tr("Background Event"),
                    page.backgrounds.iter().map(|b| none_label(b)).collect(),
                    page.background_index(),
                    Input::Background,
                    context,
                ),
                roll(tr("Select Randomly"), Input::RandomBackground, context),
            ),
        )
    };
    let history: View = if page.backgrounds.is_empty() {
        View::empty()
    } else {
        StackPanel::new().spacing(4.0).children((
            heading(tr("History"), None),
            TextBox::new()
                .automation_name(tr("History"))
                .text(page.history.clone())
                .accepts_return(true)
                .text_wrapping(TextWrapping::Wrap)
                .min_height(96.0)
                .on_text_changed(context.callback(|text: String| input(Input::History(text)))),
        ))
    };
    let measure = |name: &str,
                   unit: &str,
                   value: u32,
                   max: u32,
                   make: fn(u32) -> Input,
                   again: (&str, Input),
                   context: &mut ViewContext<Shell>| {
        ui::setting(
            name,
            unit,
            with_buttons(
                number(name, value, (1, max), make, context),
                roll(again.0, again.1, context),
            ),
        )
    };
    let age = measure(
        tr("Age"),
        tr("Years"),
        page.age,
        9999,
        Input::Age,
        (tr("Roll Age"), Input::RollAge),
        context,
    );
    let height = measure(
        tr("Height"),
        tr("Inches"),
        page.height,
        999,
        Input::Height,
        (tr("Roll Height"), Input::RollHeight),
        context,
    );
    let weight = measure(
        tr("Weight"),
        tr("Pounds"),
        page.weight,
        9999,
        Input::Weight,
        (tr("Roll Weight"), Input::RollWeight),
        context,
    );
    let hit_points: View = match &page.hit_dice_text {
        Some(rolls) => ui::setting(
            tr("Hit Die Roll"),
            rolls,
            roll(tr("Roll Hit Points"), Input::RollHitPoints, context),
        ),
        None => View::empty(),
    };
    // Enter Character's levels stand in for the rolls.
    let levels: View = match &page.levels {
        Some(levels) => super::levels::group(levels, context),
        None => View::empty(),
    };
    let exceptional: View = if !page.exceptional_shown {
        View::empty()
    } else if page.exceptional_bought {
        ui::setting(
            tr("Exceptional Strength"),
            tr("Bought with the ability points"),
            NumberBox::new()
                .automation_name(tr("Exceptional Strength"))
                .value(f64::from(page.exceptional))
                .is_enabled(false)
                .min_width(120.0),
        )
    } else {
        ui::setting(
            tr("Exceptional Strength"),
            tr("18/01 to 18/00"),
            with_buttons(
                number(
                    tr("Exceptional Strength"),
                    page.exceptional,
                    (1, 100),
                    Input::Exceptional,
                    context,
                ),
                roll(
                    tr("Roll Exceptional Strength"),
                    Input::RollExceptional,
                    context,
                ),
            ),
        )
    };
    let subabilities: View = match page.subabilities {
        Some(spreads) => {
            let rows: Vec<KeyedView> = (0..6)
                .map(|a| {
                    let (title, subtitle) = &page.subability_rows[a];
                    KeyedView::new(
                        a.to_string(),
                        ui::setting(
                            title,
                            subtitle,
                            NumberBox::new()
                                .automation_name(tr(ABILITIES[a]))
                                .value(f64::from(spreads[a]))
                                .minimum(-2.0)
                                .maximum(2.0)
                                .min_width(120.0)
                                .on_value_changed(context.callback(
                                    move |v: Option<f64>| match v {
                                        Some(v) => input(Input::Subability(a, v.round() as i32)),
                                        None => Message::Nothing,
                                    },
                                )),
                        ),
                    )
                })
                .collect();
            StackPanel::new().spacing(4.0).children((
                heading(
                    tr("Subabilities"),
                    Some(tr("Raising one subability lowers the other as much")),
                ),
                StackPanel::new().spacing(4.0).keyed_children(rows),
            ))
        }
        None => View::empty(),
    };
    fn group(children: impl IntoViews) -> View {
        StackPanel::new().spacing(4.0).children(children)
    }
    StackPanel::new().spacing(4.0).children((
        group((
            heading(&page.heading, None),
            ui::setting(tr("Name"), "", name),
            ui::setting(tr("Player"), "", player),
            ui::setting(tr("Gender"), "", gender),
            match portrait {
                Some(shown) => super::portrait::row(shown, context),
                None => View::empty(),
            },
        )),
        group((
            heading(tr("Background"), None),
            ui::setting(tr("Alignment"), "", alignment),
            ui::setting(tr("Kit"), "", kit),
            ui::setting(tr("Social Class"), "", social),
            background,
        )),
        group((heading(tr("Appearance"), None), age, height, weight)),
        levels,
        group((
            heading(tr("Combat"), None),
            hit_points,
            exceptional,
            subabilities,
        )),
        history,
    ))
}

/// A list the user adds to, with a box of what may be added and Add,
/// and removes from by each row's Remove; a nonweapon proficiency's rank
/// changes in its row.
fn list(kind: ListKind, list: &List, adding: &Adding, context: &mut ViewContext<Shell>) -> View {
    described_list(kind, list, &[], adding, context)
}

/// [`list`] with a line about each choice of Add, by its index.
fn described_list(
    kind: ListKind,
    list: &List,
    details: &[String],
    adding: &Adding,
    context: &mut ViewContext<Shell>,
) -> View {
    if !list.shown {
        return View::empty();
    }
    let rows: Vec<KeyedView> = list
        .rows
        .iter()
        .enumerate()
        .map(|(i, (title, subtitle, times))| {
            let rank: View = match *times {
                // How many of an item, or a proficiency's rank.
                Some(times) if kind == ListKind::Items => number(
                    &format!("{title}: {}", tr("Quantity")),
                    times,
                    (1, MOST_ITEMS),
                    move |v| Input::Times(i, v),
                    context,
                )
                .into(),
                Some(times) => number(
                    &format!("{title}: {}", tr("Rank")),
                    times,
                    (1, 10),
                    move |v| Input::Times(i, v),
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
                    input(Input::Remove(kind, i)),
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
            .on_click(context.message(input(Input::Add(kind, list.choices[i].clone()))))
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
                    Some(name) => Message::WizardAdding(kind, name.clone()),
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

/// A choice of one, "None" first, as a settings card.
fn choice(choice: &Choice, chosen: fn(usize) -> Input, context: &mut ViewContext<Shell>) -> View {
    if !choice.shown {
        return View::empty();
    }
    StackPanel::new().spacing(4.0).children((
        heading(&choice.title, Some(&choice.description)),
        ui::setting(
            tr("Weapon"),
            "",
            ComboBox::new()
                .automation_name(choice.title.clone())
                .items_source(choice.items.clone())
                .selected_index(choice.selected)
                .min_width(240.0)
                .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                    Some(i) => input(chosen(i)),
                    None => Message::Nothing,
                })),
        ),
    ))
}

/// Check boxes, each with a line under it: `make` makes the input of one
/// ticked or not; the ones `enabled` refuses are off.
fn checks(
    rows: &[(String, String, bool)],
    enabled: impl Fn(bool) -> bool,
    make: impl Fn(usize, bool) -> Input + Clone + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    let checks: Vec<KeyedView> = rows
        .iter()
        .enumerate()
        .map(|(i, (title, subtitle, on))| {
            let make = make.clone();
            KeyedView::new(
                format!("{i}\u{1}{title}"),
                CheckBox::new()
                    .automation_name(title.clone())
                    .is_checked(*on)
                    .is_enabled(enabled(*on))
                    .on_is_checked_changed(context.callback(move |on| input(make(i, on))))
                    .content(two_lines(title, subtitle)),
            )
        })
        .collect();
    ui::card(StackPanel::new().spacing(4.0).keyed_children(checks))
}

/// A list the classes or the kit have the user pick from: once as many
/// are ticked as may be, the others are off.
fn pick_group(g: usize, group: &PickGroup, context: &mut ViewContext<Shell>) -> View {
    let rows: Vec<(String, String, bool)> = group
        .names
        .iter()
        .zip(&group.picked)
        .map(|(name, &on)| (name.clone(), String::new(), on))
        .collect();
    let full = group.full;
    StackPanel::new().spacing(4.0).children((
        heading(&group.title, Some(&group.description)),
        checks(
            &rows,
            move |on| on || !full,
            move |i, on| Input::Pick(g, i, on),
            context,
        ),
    ))
}

/// The unarmed styles to tick, each with what it costs.
fn unarmed(unarmed: &Unarmed, context: &mut ViewContext<Shell>) -> View {
    if !unarmed.shown {
        return View::empty();
    }
    let rows: Vec<(String, String, bool)> = UNARMED
        .iter()
        .zip(unarmed.taken)
        .map(|(name, on)| ((*name).to_owned(), unarmed.cost.clone(), on))
        .collect();
    StackPanel::new().spacing(4.0).children((
        heading(
            tr("Unarmed Combat"),
            Some(tr(
                "Specialization in an unarmed style, not together with a weapon specialization",
            )),
        ),
        checks(&rows, |_| true, Input::Unarmed, context),
    ))
}

/// Skills & Powers: the character points left, kept for later levels.
fn points_left(points: &str, problem: Option<&str>) -> View {
    StackPanel::new().spacing(4.0).children((
        heading(tr("Character Points Left"), problem),
        ui::setting(
            tr("Character Points Left"),
            tr("Kept on the character for later levels"),
            ui::subtitle(points.to_owned()),
        ),
    ))
}

fn proficiencies(
    page: &ProficienciesPage,
    adding: &Adding,
    context: &mut ViewContext<Shell>,
) -> View {
    fn group(children: impl IntoViews) -> View {
        StackPanel::new().spacing(4.0).children(children)
    }
    let picks: Vec<KeyedView> = page
        .picks
        .iter()
        .enumerate()
        .map(|(g, pick)| {
            KeyedView::new(
                format!("{g}\u{1}{}", pick.title),
                pick_group(g, pick, context),
            )
        })
        .collect();
    let native: Vec<KeyedView> = page
        .native
        .iter()
        .enumerate()
        .map(|(i, name)| KeyedView::new(format!("{i}\u{1}{name}"), ui::body(name.clone())))
        .collect();
    let native = if native.is_empty() {
        ui::card(ui::body(tr("None")).opacity(0.6))
    } else {
        ui::card(StackPanel::new().spacing(6.0).keyed_children(native))
    };
    let requirements: View = if page.requirements.is_empty() {
        View::empty()
    } else {
        let rows: Vec<KeyedView> = page
            .requirements
            .iter()
            .enumerate()
            .map(|(i, text)| {
                KeyedView::new(
                    i.to_string(),
                    ui::body(text.clone()).is_text_selection_enabled(true),
                )
            })
            .collect();
        group((
            heading(
                tr("Kit Weapon Proficiency Requirements"),
                Some(&page.requirements_text),
            ),
            ui::card(StackPanel::new().spacing(6.0).keyed_children(rows)),
        ))
    };
    let lending: View = match &page.lending {
        Some(lending) => group((
            heading(
                tr("Nonweapon Slots for Weapons"),
                Some(&lending.description),
            ),
            ui::setting(
                tr("Nonweapon slots used"),
                "",
                number(
                    tr("Nonweapon slots used"),
                    lending.value,
                    (lending.least, lending.most),
                    Input::Lent,
                    context,
                ),
            ),
        )),
        None => View::empty(),
    };
    let points: View = match &page.points_left {
        Some(points) => points_left(points, None),
        None => View::empty(),
    };
    StackPanel::new().spacing(4.0).children((
        group((
            list(ListKind::Nonweapon, &page.nonweapon, adding, context),
            StackPanel::new().spacing(4.0).keyed_children(picks),
            list(ListKind::Traits, &page.traits, adding, context),
            list(
                ListKind::Disadvantages,
                &page.disadvantages,
                adding,
                context,
            ),
        )),
        group((
            heading(tr("Native Languages"), None),
            native,
            list(ListKind::Learned, &page.learned, adding, context),
        )),
        group((
            requirements,
            lending,
            list(ListKind::Weapons, &page.weapons, adding, context),
            list(ListKind::Broad, &page.broad, adding, context),
            list(ListKind::Tight, &page.tight, adding, context),
            choice(&page.focus, Input::Focus, context),
        )),
        group((
            list(
                ListKind::Specializations,
                &page.specializations,
                adding,
                context,
            ),
            choice(&page.free, Input::FreeSpecialization, context),
            unarmed(&page.unarmed, context),
            points,
        )),
    ))
}

fn combat(page: &CombatPage, adding: &Adding, context: &mut ViewContext<Shell>) -> View {
    let weapon =
        |choice: &Choice, chosen: fn(usize) -> Input, context: &mut ViewContext<Shell>| -> View {
            if !choice.shown {
                return View::empty();
            }
            ui::setting(
                &choice.title,
                &choice.description,
                ComboBox::new()
                    .automation_name(choice.title.clone())
                    .items_source(choice.items.clone())
                    .selected_index(choice.selected)
                    .min_width(240.0)
                    .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                        Some(i) => input(chosen(i)),
                        None => Message::Nothing,
                    })),
            )
        };
    let shields = checks(&page.shields, |_| true, Input::Shield, context);
    StackPanel::new().spacing(4.0).children((
        heading(tr("Weapons"), Some(&page.weapons_text)),
        weapon(&page.weapon_of_choice, Input::WeaponOfChoice, context),
        weapon(&page.expertise, Input::Expertise, context),
        weapon(&page.specialization, Input::Specialization, context),
        unarmed(&page.unarmed, context),
        heading(tr("Shield Proficiencies"), None),
        shields,
        list(ListKind::Armor, &page.armor, adding, context),
        list(ListKind::Styles, &page.styles, adding, context),
        points_left(&page.points_left, page.problem.as_deref()),
    ))
}

fn rogue(page: &RoguePage, context: &mut ViewContext<Shell>) -> View {
    let rows: Vec<KeyedView> = page
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            KeyedView::new(
                format!("{i}\u{1}{}", row.name),
                ui::setting(
                    &row.name,
                    &row.scores,
                    number(
                        &row.name,
                        row.points,
                        (0, page.most),
                        move |v| Input::RoguePoints(i, v),
                        context,
                    ),
                ),
            )
        })
        .collect();
    let bonus: View = if page.bonus.shown {
        let rows: Vec<(String, String)> = page
            .bonus
            .names
            .iter()
            .map(|n| (n.clone(), String::new()))
            .collect();
        let selected = page
            .bonus
            .picked
            .iter()
            .position(|p| *p)
            .unwrap_or(usize::MAX);
        StackPanel::new().spacing(4.0).children((
            heading(&page.bonus.title, Some(&page.bonus.description)),
            choices("rogue-bonus", &rows, selected, Input::RogueBonus, context),
        ))
    } else {
        View::empty()
    };
    let chosen: View = if page.chosen.shown {
        let rows: Vec<(String, String, bool)> = page
            .chosen
            .names
            .iter()
            .zip(&page.chosen.picked)
            .map(|(n, &on)| (n.clone(), String::new(), on))
            .collect();
        let full = page.chosen.full;
        StackPanel::new().spacing(4.0).children((
            heading(&page.chosen.title, Some(&page.chosen.description)),
            checks(&rows, move |on| on || !full, Input::RogueChosen, context),
        ))
    } else {
        View::empty()
    };
    StackPanel::new().spacing(4.0).children((
        heading(tr("Rogue Skills"), Some(&page.description)),
        StackPanel::new().spacing(4.0).keyed_children(rows),
        bonus,
        chosen,
    ))
}

fn spells(page: &SpellsPage, adding: &Adding, context: &mut ViewContext<Shell>) -> View {
    let schools: View = match &page.eliminated {
        Some(schools) => {
            let rows: Vec<(String, String, bool)> = schools
                .names
                .iter()
                .zip(&schools.ticked)
                .map(|(n, &on)| (n.clone(), String::new(), on))
                .collect();
            let full = schools.full;
            StackPanel::new().spacing(4.0).children((
                heading(&schools.title, Some(&schools.description)),
                checks(&rows, move |on| on || !full, Input::School, context),
            ))
        }
        None => View::empty(),
    };
    StackPanel::new().spacing(4.0).children((
        schools,
        described_list(
            ListKind::Spells,
            &page.spells,
            &page.chances,
            adding,
            context,
        ),
    ))
}

/// A NumberBox for a whole number in `min..=max` that may be below 0.
fn signed_number(
    name: &str,
    value: i32,
    (min, max): (i32, i32),
    changed: impl Fn(i32) -> Input + 'static,
    context: &mut ViewContext<Shell>,
) -> NumberBox {
    NumberBox::new()
        .automation_name(name)
        .value(f64::from(value))
        .minimum(f64::from(min))
        .maximum(f64::from(max))
        .min_width(120.0)
        .on_value_changed(context.callback(move |v: Option<f64>| match v {
            Some(v) => input(changed((v.round() as i32).clamp(min, max))),
            None => Message::Nothing,
        }))
}

/// A button with a label of its own.
fn button(label: &str, enabled: bool, what: Input, context: &mut ViewContext<Shell>) -> View {
    Button::new()
        .is_enabled(enabled)
        .on_click(context.message(input(what)))
        .content(plain(label))
}

/// The text of warnings that do not stop Next.
fn warnings(texts: &[String]) -> View {
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

/// The Wild Talents step: Test and Roll, the talents and, for a wild
/// talent, the psionic attack and defense modes.
fn wild(page: &WildPage, adding: &Adding, context: &mut ViewContext<Shell>) -> View {
    let description: View = if page.description.is_empty() {
        View::empty()
    } else {
        ui::card(ui::body(page.description.clone()))
    };
    let test: View = if page.test_shown {
        ui::setting(
            tr("Test for Wild Talents"),
            &page.chance,
            button(tr("Test"), page.can_test, Input::Test, context),
        )
    } else {
        View::empty()
    };
    StackPanel::new().spacing(4.0).children((
        description,
        heading(tr("Wild Talents"), None),
        test,
        ui::setting(
            tr("Roll for Wild Talents"),
            tr("Table 83: Wild Talents"),
            button(tr("Roll"), page.can_roll, Input::Roll, context),
        ),
        described_list(ListKind::WildTalents, &page.talents, &[], adding, context),
        described_list(ListKind::Attacks, &page.attacks, &[], adding, context),
        described_list(ListKind::Defenses, &page.defenses, &[], adding, context),
    ))
}

/// The Skills page: the rogue skills of the current level, the kit's
/// bonuses and chosen skills, a psionicist's disciplines and powers, and
/// the psionic attack and defense modes.
fn skills_form(form: &SkillsForm, adding: &Adding, context: &mut ViewContext<Shell>) -> View {
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
                                (row.least, MOST_POINTS),
                                move |v| Input::SkillPoints(i, v),
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
                        move |i| Input::KitBonus(g, i),
                        context,
                    ),
                )),
            )
        })
        .collect();
    let chosen: View = match form.chosen() {
        Some(group) => {
            let rows: Vec<(String, String, bool)> = group
                .names
                .iter()
                .zip(&group.ticked)
                .map(|(n, &on)| (n.clone(), String::new(), on))
                .collect();
            let full = group.full;
            StackPanel::new().spacing(4.0).children((
                heading(&group.title, Some(&group.description)),
                checks(&rows, move |on| on || !full, Input::RogueChosen, context),
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
                                        Some(i) => input(Input::Primary(slot, i)),
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
    let lists: Vec<KeyedView> = [
        ListKind::Disciplines,
        ListKind::Sciences,
        ListKind::Devotions,
        ListKind::Attacks,
        ListKind::Defenses,
    ]
    .into_iter()
    .map(|kind| {
        KeyedView::new(
            format!("{kind:?}"),
            described_list(kind, &form.list(kind), &[], adding, context),
        )
    })
    .collect();
    StackPanel::new().spacing(4.0).children((
        rogue,
        StackPanel::new().spacing(4.0).keyed_children(bonuses),
        chosen,
        psionics,
        StackPanel::new().spacing(4.0).keyed_children(lists),
    ))
}

/// The last step: the starting money, the standard pack, what comes free
/// and what the money buys.
fn equipment(page: &EquipmentPage, adding: &Adding, context: &mut ViewContext<Shell>) -> View {
    let free: Vec<KeyedView> = page
        .free
        .iter()
        .enumerate()
        .map(|(i, (name, why))| {
            KeyedView::new(
                format!("{i}\u{1}{name}"),
                ui::setting(name, why, View::empty()),
            )
        })
        .collect();
    StackPanel::new().spacing(4.0).children((
        heading(tr("Money"), None),
        ui::setting(
            tr("Starting Money"),
            &page.money,
            roll(tr("Roll Starting Money"), Input::RollMoney, context),
        ),
        ui::setting(
            tr("Standard Pack"),
            &page.pack_text,
            ToggleSwitch::new()
                .automation_name(tr("Standard Pack"))
                .is_on(page.pack)
                .on_toggled(context.callback(|on| input(Input::Pack(on)))),
        ),
        StackPanel::new().spacing(4.0).keyed_children(free),
        described_list(ListKind::Items, &page.items, &[], adding, context),
    ))
}
