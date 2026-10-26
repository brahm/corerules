//! New NPC and New NPC Group: the forms of `cr_app::npc` on a page of
//! the window, with Create and Cancel under them. New NPC chooses the
//! race and class combination, a level for each class, the rules and the
//! ability scores Ability Scores… rolls; New NPC Group adds lines of a
//! number, race, classes, kit, alignment and levels, made in the order of
//! the sorted list.

use cr_app::npc::{GROUPS, GroupForm, GroupInput, MOST_LEVEL, MOST_NUMBER, NpcForm, NpcInput};
use cr_i18n::tr;
use cr_store::create::ABILITIES;
use cr_win::text::plain;
use windows_reactor::*;

use super::forms::{self, heading};
use super::{Message, Shell};
use crate::ui;

fn npc(input: NpcInput) -> Message {
    Message::Npc(input)
}

fn group(input: GroupInput) -> Message {
    Message::Group(input)
}

/// A ComboBox of `items`; `chosen` makes the message of the one chosen.
fn combo(
    name: &str,
    items: Vec<String>,
    selected: usize,
    enabled: bool,
    chosen: impl Fn(usize) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> ComboBox {
    ComboBox::new()
        .automation_name(name)
        .items_source(items)
        .selected_index(Some(selected))
        .is_enabled(enabled)
        .min_width(240.0)
        .on_selection_changed(context.callback(move |i: Option<usize>| match i {
            Some(i) => chosen(i),
            None => Message::Nothing,
        }))
}

/// A ToggleSwitch as a settings card with a line under its name.
fn toggle(
    name: &str,
    description: &str,
    on: bool,
    toggled: impl Fn(bool) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    ui::setting(
        name,
        description,
        ToggleSwitch::new()
            .automation_name(name)
            .is_on(on)
            .on_toggled(context.callback(toggled)),
    )
}

/// The Rules choice: the Core Rules or Player's Option.
fn rules(
    players_option: bool,
    chosen: impl Fn(bool) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    ui::setting(
        tr("Rules"),
        "",
        combo(
            tr("Rules"),
            vec![tr("Core Rules").to_owned(), "Player's Option".to_owned()],
            usize::from(players_option),
            true,
            move |i| chosen(i == 1),
            context,
        ),
    )
}

/// What stopped the form, in a bar over its buttons.
fn error(error: &Option<String>) -> View {
    match error {
        Some(text) => InfoBar::new()
            .severity(InfoBarSeverity::Error)
            .title(text.clone())
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    }
}

/// Cancel and the button that creates, under the form.
fn buttons(create: &str, enabled: bool, context: &mut ViewContext<Shell>) -> View {
    StackPanel::new()
        .margin(Thickness::new(0.0, 16.0, 0.0, 0.0))
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Right)
        .children((
            Button::new()
                .on_click(context.message(Message::Back))
                .content(plain(tr("_Cancel"))),
            Button::new()
                .style(ButtonStyle::Accent)
                .is_enabled(enabled)
                .on_click(context.message(Message::CreateNpcs))
                .content(plain(create)),
        ))
}

/// The New NPC page.
pub fn view(form: &NpcForm, busy: bool, context: &mut ViewContext<Shell>) -> View {
    let character = StackPanel::new().spacing(4.0).children((
        heading(tr("Character"), None),
        ui::setting(
            tr("Race"),
            "",
            combo(
                tr("Race"),
                form.races().to_vec(),
                form.race_index(),
                true,
                |i| npc(NpcInput::Race(i)),
                context,
            ),
        ),
        toggle(
            tr("All Races"),
            tr("Include races that are not standard"),
            form.all_races,
            |on| npc(NpcInput::AllRaces(on)),
            context,
        ),
        ui::setting(
            tr("Class"),
            "",
            combo(
                tr("Class"),
                form.class_names(),
                form.class_index(),
                true,
                |i| npc(NpcInput::Class(i)),
                context,
            ),
        ),
        toggle(
            tr("All Classes"),
            tr("Include classes the race may not take"),
            form.all_classes,
            |on| npc(NpcInput::AllClasses(on)),
            context,
        ),
        rules(
            form.players_option,
            |on| npc(NpcInput::PlayersOption(on)),
            context,
        ),
    ));
    // A level for each class of the combination, named for it.
    let levels: Vec<KeyedView> = form
        .classes()
        .iter()
        .zip(&form.levels)
        .enumerate()
        .map(|(i, (class, level))| {
            KeyedView::new(
                format!("{i}\u{1}{class}"),
                ui::setting(
                    class,
                    "",
                    forms::number(
                        class,
                        (*level).max(1) as u32,
                        (1, MOST_LEVEL as u32),
                        move |v| npc(NpcInput::Level(i, v as i32)),
                        context,
                    ),
                ),
            )
        })
        .collect();
    let levels = StackPanel::new().spacing(4.0).children((
        heading(tr("Levels"), None),
        StackPanel::new().spacing(4.0).keyed_children(levels),
    ));
    let abilities: View = if form.template {
        View::empty()
    } else {
        abilities(form, context)
    };
    let party: View = if form.party_shown() {
        ui::setting(
            tr("Party"),
            "",
            TextBox::new()
                .automation_name(tr("Party"))
                .text(form.party.clone())
                .min_width(260.0)
                .on_text_changed(context.callback(|text: String| npc(NpcInput::Party(text)))),
        )
    } else {
        View::empty()
    };
    let create = if form.template {
        tr("Add to _Template")
    } else {
        tr("_Create")
    };
    ui::page(StackPanel::new().spacing(4.0).children((
        ui::title(tr("New NPC")).margin(Thickness::new(0.0, 0.0, 0.0, 8.0)),
        character,
        levels,
        abilities,
        party,
        error(&form.error),
        buttons(create, !busy, context),
    )))
}

/// The ability scores: Roll and Clear, and the scores rolled to change.
fn abilities(form: &NpcForm, context: &mut ViewContext<Shell>) -> View {
    let description = if form.abilities.is_some() {
        tr("The NPC keeps these scores")
    } else {
        tr("Rolled with the NPC unless you roll them now")
    };
    let mut buttons: Vec<KeyedView> = vec![KeyedView::new(
        "roll",
        Button::new()
            .on_click(context.message(npc(NpcInput::Roll)))
            .content(plain(tr("_Roll"))),
    )];
    if form.abilities.is_some() {
        buttons.push(KeyedView::new(
            "clear",
            Button::new()
                .on_click(context.message(npc(NpcInput::Clear)))
                .content(plain(tr("_Clear"))),
        ));
    }
    let scores: View = match form.abilities {
        Some(rolled) => {
            let mut rows: Vec<KeyedView> = ABILITIES
                .iter()
                .zip(form.ability_ranges())
                .enumerate()
                .map(|(i, (name, (lo, hi)))| {
                    KeyedView::new(
                        *name,
                        ui::setting(
                            tr(name),
                            "",
                            forms::signed_number(
                                tr(name),
                                rolled.scores[i],
                                (lo, hi.max(lo)),
                                move |v| npc(NpcInput::Ability(i, v)),
                                context,
                            ),
                        ),
                    )
                })
                .collect();
            if form.exceptional_shown() {
                rows.push(KeyedView::new(
                    "exceptional",
                    ui::setting(
                        tr("Exceptional Strength"),
                        tr("1 to 100 for 18/01 to 18/00"),
                        forms::signed_number(
                            tr("Exceptional Strength"),
                            rolled.exceptional,
                            (1, 100),
                            |v| npc(NpcInput::Exceptional(v)),
                            context,
                        ),
                    ),
                ));
            }
            StackPanel::new().spacing(4.0).keyed_children(rows)
        }
        None => View::empty(),
    };
    StackPanel::new().spacing(4.0).children((
        heading(tr("Ability Scores"), Some(description)),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .keyed_children(buttons),
        scores,
    ))
}

/// The New NPC Group page.
pub fn group_view(form: &GroupForm, busy: bool, context: &mut ViewContext<Shell>) -> View {
    let about = StackPanel::new().spacing(4.0).children((
        heading(tr("Group"), None),
        ui::setting(
            tr("Group Name"),
            "",
            TextBox::new()
                .automation_name(tr("Group Name"))
                .text(form.name.clone())
                .min_width(260.0)
                .on_text_changed(context.callback(|text: String| group(GroupInput::Name(text)))),
        ),
        ui::setting(
            tr("Race"),
            "",
            combo(
                tr("Race"),
                form.race_items(),
                form.race_index(),
                true,
                |i| group(GroupInput::Race(i)),
                context,
            ),
        ),
        toggle(
            tr("All Races"),
            tr("Include races that are not standard"),
            form.all_races,
            |on| group(GroupInput::AllRaces(on)),
            context,
        ),
        ui::setting(
            tr("Fewest"),
            "",
            forms::number(
                tr("Fewest"),
                form.number.0.max(1) as u32,
                (1, MOST_NUMBER as u32),
                |v| group(GroupInput::Fewest(v as i32)),
                context,
            ),
        ),
        ui::setting(
            tr("Most"),
            "",
            forms::number(
                tr("Most"),
                form.number.1.max(1) as u32,
                (1, MOST_NUMBER as u32),
                |v| group(GroupInput::Most(v as i32)),
                context,
            ),
        ),
        rules(
            form.players_option,
            |on| group(GroupInput::PlayersOption(on)),
            context,
        ),
    ));
    let classes: Vec<KeyedView> = GROUPS
        .iter()
        .enumerate()
        .map(|(g, name)| {
            KeyedView::new(
                *name,
                ui::setting(
                    tr(name),
                    "",
                    combo(
                        tr(name),
                        form.class_items(g),
                        form.class_index(g),
                        form.classes_enabled(),
                        move |i| group(GroupInput::Class(g, i)),
                        context,
                    ),
                ),
            )
        })
        .collect();
    let classes = StackPanel::new().spacing(4.0).children((
        heading(tr("Classes"), Some(tr("A class of each group, or none"))),
        toggle(
            tr("Allowed Combination"),
            tr("Each NPC takes a class combination its race allows"),
            form.allowed,
            |on| group(GroupInput::Allowed(on)),
            context,
        ),
        StackPanel::new().spacing(4.0).keyed_children(classes),
        ui::setting(
            tr("Kit"),
            "",
            combo(
                tr("Kit"),
                form.kit_items(),
                form.kit_index(),
                true,
                |i| group(GroupInput::Kit(i)),
                context,
            ),
        ),
        toggle(
            tr("All Kits"),
            tr("Include kits the race, classes or alignment may not take"),
            form.all_kits,
            |on| group(GroupInput::AllKits(on)),
            context,
        ),
        ui::setting(
            tr("Alignment"),
            "",
            combo(
                tr("Alignment"),
                form.alignment_items(),
                form.alignment_index(),
                form.alignment_enabled(),
                |i| group(GroupInput::Alignment(i)),
                context,
            ),
        ),
        toggle(
            tr("Any Alignment"),
            tr("Include alignments the classes, race or kit do not allow"),
            form.any_alignment,
            |on| group(GroupInput::AnyAlignment(on)),
            context,
        ),
    ));
    let levels: Vec<KeyedView> = GROUPS
        .iter()
        .zip(form.levels)
        .enumerate()
        .map(|(g, (name, (low, high)))| {
            let name = tr(name);
            KeyedView::new(
                g.to_string(),
                ui::setting(
                    name,
                    "",
                    StackPanel::new()
                        .orientation(Orientation::Horizontal)
                        .spacing(8.0)
                        .children((
                            forms::number(
                                &tr!("Lowest {name} Level", name),
                                low.max(1) as u32,
                                (1, MOST_LEVEL as u32),
                                move |v| group(GroupInput::Low(g, v as i32)),
                                context,
                            ),
                            ui::caption(tr("to")).vertical_alignment(VerticalAlignment::Center),
                            forms::number(
                                &tr!("Highest {name} Level", name),
                                high.max(1) as u32,
                                (1, MOST_LEVEL as u32),
                                move |v| group(GroupInput::High(g, v as i32)),
                                context,
                            ),
                        )),
                ),
            )
        })
        .collect();
    let levels = StackPanel::new().spacing(4.0).children((
        heading(
            tr("Levels"),
            Some(tr("Lowest and highest level for each class group")),
        ),
        StackPanel::new().spacing(4.0).keyed_children(levels),
    ));
    let lines: Vec<KeyedView> = form
        .lines()
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            KeyedView::new(
                format!("{i}\u{1}{line}"),
                ui::card(ui::body(line).is_text_selection_enabled(true)),
            )
        })
        .collect();
    let lines = StackPanel::new().spacing(4.0).children((
        heading(
            tr("Lines"),
            Some(tr("Without lines, Create makes the line above")),
        ),
        StackPanel::new().spacing(4.0).keyed_children(lines),
        Button::new()
            .margin(Thickness::new(0.0, 4.0, 0.0, 0.0))
            .on_click(context.message(group(GroupInput::AddLine)))
            .content(plain(tr("_Add Line"))),
    ));
    ui::page(StackPanel::new().spacing(4.0).children((
        ui::title(tr("New NPC Group")).margin(Thickness::new(0.0, 0.0, 0.0, 8.0)),
        about,
        classes,
        levels,
        lines,
        error(&form.error),
        buttons(tr("_Create"), !busy, context),
    )))
}
