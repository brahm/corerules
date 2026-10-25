//! Edit Character: the editor of `cr_app::editor` on a page of the
//! window, its pages chosen above them, with Save and Cancel under them.
//! The Details page has the Name page of Update Character, the classes
//! with Add Class, the experience with Levels… and Add…, the appearance,
//! the ability scores and the history; the Abilities page the subrace,
//! the racial and class abilities, schools and spheres; the Combat page
//! the weapon proficiencies and groups, the specializations, unarmed
//! combat, the kit's weapons, shields, armor, fighting styles, traits and
//! disadvantages; the Proficiencies page the nonweapon proficiencies and
//! the languages; the Skills page the rogue skills, the psionics and the
//! wild talents; the Spells page the kit's schools and spheres, the
//! spells known, the spellbooks and the spells memorized; the Equipment
//! page the spending money and the items of each bundle.

use cr_app::create::{ListKind, none_label, points};
use cr_app::editor::{
    COIN_NAMES, Checks, EditInput, Editor, MOST_AGE, MOST_COINS, MOST_HEIGHT, MOST_POINTS,
    MOST_QUANTITY, MOST_WEIGHT, Tab,
};
use cr_app::levels::LevelsInput;
use cr_i18n::{tr, tr_noop};
use cr_store::create::{ABILITIES, SUBABILITIES};
use cr_store::edit::ABILITY_RANGE;
use cr_win::text::plain;
use windows_reactor::*;

use super::create::prompt_dialog;
use super::forms::{
    self, Adding, Icons, ListMessages, Rows, SkillsMessages, heading, with_buttons,
};
use super::portrait::Shown;
use super::{Message, Shell};
use crate::ui;

const GENDERS: [&str; 2] = [tr_noop!("Male"), tr_noop!("Female")];

/// An editor input as the shell's message.
fn input(input: EditInput) -> Message {
    Message::Editor(input)
}

fn levels(change: LevelsInput) -> Message {
    input(EditInput::Levels(change))
}

fn experience(change: LevelsInput) -> Message {
    input(EditInput::Experience(change))
}

/// The editor's lists' messages.
const LISTS: ListMessages = ListMessages {
    add: |kind, name| input(EditInput::Add(kind, name)),
    remove: |kind, row| input(EditInput::Remove(kind, row)),
    times: |kind, row, n| input(EditInput::Times(kind, row, n)),
};

/// The Skills page's messages.
const SKILLS: SkillsMessages = SkillsMessages {
    points: |row, points| input(EditInput::SkillPoints(row, points)),
    bonus: |group, name| input(EditInput::KitBonus(group, name)),
    chosen: |name, on| input(EditInput::RogueChosen(name, on)),
    primary: |slot, index| input(EditInput::Primary(slot, index)),
    lists: LISTS,
};

/// A NumberBox for a whole number in `0..=max`.
fn number(
    name: &str,
    value: u32,
    (min, max): (u32, u32),
    changed: fn(u32) -> EditInput,
    context: &mut ViewContext<Shell>,
) -> NumberBox {
    NumberBox::new()
        .automation_name(name)
        .value(f64::from(value))
        .minimum(f64::from(min))
        .maximum(f64::from(max))
        .min_width(120.0)
        .on_value_changed(context.callback(move |v: Option<f64>| match v {
            Some(v) => input(changed((v.round().max(0.0) as u32).clamp(min, max))),
            None => Message::Nothing,
        }))
}

fn text(
    name: &str,
    value: &str,
    changed: fn(String) -> EditInput,
    context: &mut ViewContext<Shell>,
) -> TextBox {
    TextBox::new()
        .automation_name(name)
        .text(value)
        .min_width(260.0)
        .on_text_changed(context.callback(move |text: String| input(changed(text))))
}

fn combo(
    name: &str,
    items: Vec<String>,
    selected: Option<usize>,
    changed: fn(usize) -> EditInput,
    context: &mut ViewContext<Shell>,
) -> ComboBox {
    ComboBox::new()
        .automation_name(name)
        .items_source(items)
        .selected_index(selected)
        .min_width(220.0)
        .on_selection_changed(context.callback(move |i: Option<usize>| match i {
            Some(i) => input(changed(i)),
            None => Message::Nothing,
        }))
}

/// The editor; `adding` is what the lists' boxes chose for Add, `icons`
/// those of the items in the bundles, and `portrait` the one the Details
/// page shows.
pub fn view(
    editor: &Editor,
    adding: &Adding,
    icons: &Icons,
    portrait: &Shown,
    context: &mut ViewContext<Shell>,
) -> View {
    // The pages to choose from, once there are several.
    let tabs: View = if Tab::ALL.len() > 1 {
        let items: Vec<KeyedView> = Tab::ALL
            .iter()
            .map(|tab| {
                KeyedView::new(
                    tab.title(),
                    SelectorBarItem::new()
                        .text(tr(tab.title()))
                        .is_selected(*tab == editor.tab),
                )
            })
            .collect();
        SelectorBar::new()
            .margin(Thickness::new(0.0, 0.0, 0.0, 12.0))
            .on_selected_text_changed(context.callback(|text: Option<String>| {
                match Tab::ALL
                    .iter()
                    .find(|t| text.as_deref() == Some(tr(t.title())))
                {
                    Some(tab) => input(EditInput::Tab(*tab)),
                    None => Message::Nothing,
                }
            }))
            .slots([SlotView::collection(SelectorBarSlot::Items, items)])
    } else {
        View::empty()
    };
    let content = match editor.tab {
        Tab::Details => details(editor, portrait, context),
        Tab::Abilities => abilities_tab(editor, context),
        Tab::Combat => combat_tab(editor, adding, context),
        Tab::Proficiencies => StackPanel::new().spacing(4.0).children((
            forms::list(
                ListKind::Nonweapon,
                &editor.nonweapon_list(),
                &[],
                adding,
                LISTS,
                context,
            ),
            forms::list(
                ListKind::Learned,
                &editor.languages_list(),
                &[],
                adding,
                LISTS,
                context,
            ),
        )),
        Tab::Skills => skills_tab(editor, adding, context),
        Tab::Spells => spells_tab(editor, adding, context),
        Tab::Equipment => equipment_tab(editor, adding, icons, context),
    };
    let problem: View = match editor.error.clone().or_else(|| editor.problem()) {
        Some(problem) => InfoBar::new()
            .severity(if editor.error.is_some() {
                InfoBarSeverity::Error
            } else {
                InfoBarSeverity::Warning
            })
            .title(problem)
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    };
    let buttons = StackPanel::new()
        .margin(Thickness::new(0.0, 16.0, 0.0, 0.0))
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Right)
        .children((
            Button::new()
                .is_enabled(!editor.saving)
                .on_click(context.message(Message::Back))
                .content(plain(tr("_Cancel"))),
            Button::new()
                .style(ButtonStyle::Accent)
                .is_enabled(editor.can_save())
                .on_click(context.message(input(EditInput::Save)))
                .content(plain(tr("_Save"))),
        ));
    // Each question is a dialog of its own, so that one asked right after
    // another opens; so is each try of Add Experience.
    let serial = editor.prompt_serial();
    let mut dialogs: Vec<KeyedView> = editor
        .prompt()
        .map(|prompt| {
            KeyedView::new(
                format!("prompt{serial}"),
                prompt_dialog(prompt, serial, Message::EditorAnswer, context),
            )
        })
        .into_iter()
        .collect();
    if let Some(form) = &editor.levels {
        dialogs.push(KeyedView::new(
            "levels",
            super::levels::dialog(form, levels, input(EditInput::CloseLevels), context),
        ));
    }
    if let Some(form) = &editor.experience {
        dialogs.push(KeyedView::new(
            format!("experience{}", editor.experience_serial),
            super::levels::experience_dialog(form, experience, context),
        ));
    }
    Grid::new().children((
        ui::page(StackPanel::new().spacing(4.0).children((
            ui::title(tr("Edit Character")).margin(Thickness::new(0.0, 0.0, 0.0, 16.0)),
            tabs,
            content,
            problem,
            buttons,
        ))),
        Grid::new().keyed_children(dialogs),
    ))
}

/// The Details page.
fn details(editor: &Editor, portrait: &Shown, context: &mut ViewContext<Shell>) -> View {
    let d = &editor.details;
    let who = StackPanel::new().spacing(4.0).children((
        ui::setting(
            tr("Name"),
            "",
            text(tr("Name"), &d.name, EditInput::Name, context),
        ),
        ui::setting(
            tr("Player"),
            "",
            text(tr("Player"), &d.player, EditInput::Player, context),
        ),
        ui::setting(
            tr("Social Class"),
            "",
            text(
                tr("Social Class"),
                &d.social_class,
                EditInput::SocialClass,
                context,
            ),
        ),
        super::portrait::row(portrait, context),
        ui::setting(
            tr("Gender"),
            "",
            combo(
                tr("Gender"),
                GENDERS.iter().map(|g| tr(g).to_owned()).collect(),
                Some(usize::from(!d.male)),
                EditInput::Gender,
                context,
            ),
        ),
    ));
    let background_event: View = if editor.backgrounds.is_empty() {
        View::empty()
    } else {
        ui::setting(
            tr("Background Event"),
            "",
            with_buttons(
                combo(
                    tr("Background Event"),
                    editor.backgrounds.iter().map(|b| none_label(b)).collect(),
                    editor.background_index(),
                    EditInput::Background,
                    context,
                ),
                ui::icon_button(
                    tr("Select Randomly"),
                    Symbol::Refresh,
                    input(EditInput::RandomBackground),
                    context,
                ),
            ),
        )
    };
    let background = StackPanel::new().spacing(4.0).children((
        heading(tr("Background"), None),
        ui::setting(
            tr("Alignment"),
            "",
            combo(
                tr("Alignment"),
                editor.alignments.iter().map(|a| tr(a).to_owned()).collect(),
                editor.alignment_index(),
                EditInput::Alignment,
                context,
            ),
        ),
        ui::setting(
            tr("Kit"),
            tr("A new kit rolls the social class on its table"),
            combo(
                tr("Kit"),
                editor.kits.iter().map(|k| none_label(k)).collect(),
                editor.kit_index(),
                EditInput::Kit,
                context,
            ),
        ),
        background_event,
    ));
    let appearance = StackPanel::new().spacing(4.0).children((
        heading(tr("Appearance"), None),
        ui::setting(
            tr("Age"),
            tr("Years"),
            number(tr("Age"), d.age, (0, MOST_AGE), EditInput::Age, context),
        ),
        ui::setting(
            tr("Height"),
            tr("Inches"),
            number(
                tr("Height"),
                d.height,
                (0, MOST_HEIGHT),
                EditInput::Height,
                context,
            ),
        ),
        ui::setting(
            tr("Weight"),
            tr("Pounds"),
            number(
                tr("Weight"),
                d.weight,
                (0, MOST_WEIGHT),
                EditInput::Weight,
                context,
            ),
        ),
    ));
    let history: View = if editor.backgrounds.is_empty() && d.history.is_empty() {
        View::empty()
    } else {
        StackPanel::new().spacing(4.0).children((
            heading(tr("History"), None),
            TextBox::new()
                .automation_name(tr("History"))
                .text(d.history.clone())
                .accepts_return(true)
                .text_wrapping(TextWrapping::Wrap)
                .min_height(96.0)
                .on_text_changed(context.callback(|text: String| input(EditInput::History(text)))),
        ))
    };
    StackPanel::new().spacing(4.0).children((
        who,
        background,
        classes(editor, context),
        experience_group(editor, context),
        appearance,
        abilities(editor, context),
        history,
    ))
}

/// The classes, and Add Class for one more when the race may dual-class.
fn classes(editor: &Editor, context: &mut ViewContext<Shell>) -> View {
    let offer = editor.class_offer();
    let rows: Vec<KeyedView> = editor
        .character()
        .data
        .classes
        .names
        .iter()
        .enumerate()
        .map(|(g, name)| KeyedView::new(format!("{g}\u{1}{name}"), ui::card(ui::body(name))))
        .collect();
    let added: View = match &editor.new_class {
        Some(name) => ui::setting(
            name,
            tr("New, level 1"),
            ui::icon_button(
                tr("Remove"),
                Symbol::Delete,
                input(EditInput::RemoveClass),
                context,
            ),
        ),
        // One class at a time: the next one depends on this one.
        None if !offer.names.is_empty() => ui::setting(
            tr("Add Class"),
            "",
            ComboBox::new()
                .automation_name(tr("Add Class"))
                .placeholder_text(tr("Choose a class"))
                .items_source(offer.names.clone())
                .min_width(220.0)
                .on_selection_changed(context.callback(|i: Option<usize>| match i {
                    Some(i) => input(EditInput::AddClass(i)),
                    None => Message::Nothing,
                })),
        ),
        None => View::empty(),
    };
    StackPanel::new().spacing(4.0).children((
        heading(tr("Classes"), Some(&offer.description)),
        StackPanel::new().spacing(4.0).keyed_children(rows),
        added,
    ))
}

/// Experience, levels, hit points and PSPs, with Levels… and Add….
fn experience_group(editor: &Editor, context: &mut ViewContext<Shell>) -> View {
    let header = Grid::new()
        .margin(Thickness::new(0.0, 16.0, 0.0, 4.0))
        .columns([GridLength::STAR, GridLength::Auto])
        .children((
            ui::strong(tr("Experience")).vertical_alignment(VerticalAlignment::Center),
            StackPanel::new()
                .grid_column(1)
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .children((
                    Button::new()
                        .on_click(context.message(input(EditInput::OpenLevels)))
                        .content(plain(tr("Levels…"))),
                    Button::new()
                        .on_click(context.message(input(EditInput::OpenExperience)))
                        .content(plain(tr("Add…")))
                        .tooltip(tr("Add or take away experience")),
                )),
        ));
    let rows: Vec<KeyedView> = editor
        .class_rows()
        .into_iter()
        .enumerate()
        .map(|(g, class)| {
            KeyedView::new(
                format!("{g}\u{1}{}", class.name),
                ui::setting(&class.name, &class.text, View::empty()),
            )
        })
        .collect();
    let hit_points: View = match editor.hit_points() {
        Some(hp) => ui::setting(
            tr("Hit Points"),
            tr("Changes the rolls of the highest level"),
            number(
                tr("Hit Points"),
                hp,
                (0, MOST_POINTS),
                EditInput::HitPoints,
                context,
            ),
        ),
        None => View::empty(),
    };
    let psp: View = match editor.psp() {
        Some(psp) => ui::setting(
            tr("PSPs"),
            tr("Changes those of the highest level that has them"),
            number(tr("PSPs"), psp, (0, 999), EditInput::Psp, context),
        ),
        None => View::empty(),
    };
    let points: View = match editor.character_points() {
        Some(points) => ui::setting(
            tr("Character Points"),
            tr("Unspent"),
            ui::body(points.to_string()),
        ),
        None => View::empty(),
    };
    StackPanel::new().spacing(4.0).children((
        header,
        StackPanel::new().spacing(4.0).keyed_children(rows),
        hit_points,
        psp,
        points,
    ))
}

/// The ability scores, with Skills & Powers' subabilities, and
/// exceptional strength.
fn abilities(editor: &Editor, context: &mut ViewContext<Shell>) -> View {
    let d = &editor.details;
    let (low, high) = (*ABILITY_RANGE.start(), *ABILITY_RANGE.end());
    let score =
        move |name: &str, value: u32, a: usize, part: usize, context: &mut ViewContext<Shell>| {
            NumberBox::new()
                .automation_name(name)
                .value(f64::from(value))
                .minimum(f64::from(low))
                .maximum(f64::from(high))
                .min_width(100.0)
                .on_value_changed(context.callback(move |v: Option<f64>| match v {
                    Some(v) => input(EditInput::Ability(
                        a,
                        part,
                        (v.round().max(0.0) as u32).clamp(low, high),
                    )),
                    None => Message::Nothing,
                }))
        };
    let rows: Vec<KeyedView> = (0..6)
        .map(|a| {
            let scores = d.abilities[a];
            let control: View = if d.skills_and_powers {
                let labeled = |label: &str, control: NumberBox| {
                    StackPanel::new()
                        .spacing(2.0)
                        .children((ui::caption(label), control))
                };
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(8.0)
                    .children((
                        labeled(
                            tr("Score"),
                            score(tr(ABILITIES[a]), scores[0], a, 0, context),
                        ),
                        labeled(
                            tr(SUBABILITIES[a][0]),
                            score(tr(SUBABILITIES[a][0]), scores[1], a, 1, context),
                        ),
                        labeled(
                            tr(SUBABILITIES[a][1]),
                            score(tr(SUBABILITIES[a][1]), scores[2], a, 2, context),
                        ),
                    ))
            } else {
                score(tr(ABILITIES[a]), scores[0], a, 0, context).into()
            };
            KeyedView::new(a, ui::setting(tr(ABILITIES[a]), "", control))
        })
        .collect();
    StackPanel::new().spacing(4.0).children((
        heading(tr("Abilities"), None),
        StackPanel::new().spacing(4.0).keyed_children(rows),
        ui::setting(
            tr("Exceptional Strength"),
            tr("18/01 to 18/00, 0 for none"),
            NumberBox::new()
                .automation_name(tr("Exceptional Strength"))
                .value(f64::from(d.exceptional_strength))
                .minimum(0.0)
                .maximum(100.0)
                .min_width(120.0)
                .is_enabled(editor.exceptional_allowed())
                .on_value_changed(context.callback(|v: Option<f64>| match v {
                    Some(v) => input(EditInput::Exceptional(v.round().max(0.0) as u32)),
                    None => Message::Nothing,
                })),
        ),
    ))
}

/// The Abilities page: the subrace and its package, then each group of
/// abilities, schools and spheres to tick.
fn abilities_tab(editor: &Editor, context: &mut ViewContext<Shell>) -> View {
    let package: Vec<KeyedView> = editor
        .package()
        .into_iter()
        .enumerate()
        .map(|(i, name)| KeyedView::new(format!("{i}\u{1}{name}"), ui::body(name)))
        .collect();
    StackPanel::new().spacing(4.0).children((
        forms::points_left(&points(editor.points_left()), None),
        heading(tr("Race"), Some(editor.subrace_description())),
        ui::setting(
            tr("Subrace"),
            "",
            combo(
                tr("Subrace"),
                editor.subraces(),
                editor.subrace_index(),
                EditInput::Subrace,
                context,
            ),
        ),
        heading(tr("Racial Package"), None),
        ui::card(StackPanel::new().spacing(6.0).keyed_children(package)),
        check_groups(editor.ability_groups(), EditInput::Tick, context),
    ))
}

/// Groups of check boxes, each under its heading; `tick` makes the input
/// of one ticked or not.
fn check_groups<G: Copy + 'static>(
    groups: Vec<Checks<G>>,
    tick: fn(G, usize, bool) -> EditInput,
    context: &mut ViewContext<Shell>,
) -> View {
    let groups: Vec<KeyedView> = groups
        .into_iter()
        .map(|group| {
            let rows: Vec<(String, String, bool, bool)> = group
                .checks
                .into_iter()
                .map(|c| (c.title, c.subtitle, c.on, c.enabled))
                .collect();
            let which = group.group;
            let checks = if rows.is_empty() {
                ui::card(ui::body(tr("None")).opacity(0.6))
            } else {
                forms::checks(&rows, move |i, on| input(tick(which, i, on)), context)
            };
            KeyedView::new(
                group.title.clone(),
                StackPanel::new()
                    .spacing(4.0)
                    .children((heading(&group.title, Some(&group.description)), checks)),
            )
        })
        .collect();
    StackPanel::new().spacing(4.0).keyed_children(groups)
}

/// The Combat page.
fn combat_tab(editor: &Editor, adding: &Adding, context: &mut ViewContext<Shell>) -> View {
    let v = editor.combat_view();
    let list = |kind: ListKind, list: &cr_app::create::List, context: &mut ViewContext<Shell>| {
        forms::list(kind, list, &[], adding, LISTS, context)
    };
    let weapon = tr("Weapon");
    let points_left: View = match v.points_left {
        Some(left) => forms::points_left(&points(left), Some(&v.description)),
        None => View::empty(),
    };
    let shields: View = if v.shields.is_empty() {
        View::empty()
    } else {
        let rows: Vec<(String, String, bool, bool)> = v
            .shields
            .iter()
            .map(|(name, cost, on)| (name.clone(), cost.clone(), *on, true))
            .collect();
        StackPanel::new().spacing(4.0).children((
            heading(tr("Shield Proficiencies"), None),
            forms::checks(&rows, |i, on| input(EditInput::Shield(i, on)), context),
        ))
    };
    let group = |children: (View, View, View, View, View)| -> View {
        StackPanel::new().spacing(4.0).children(children)
    };
    let weapons = group((
        points_left,
        list(ListKind::Weapons, &v.weapons, context),
        list(ListKind::Broad, &v.broad, context),
        list(ListKind::Tight, &v.tight, context),
        View::empty(),
    ));
    let specialization = group((
        forms::choice(
            &v.weapon_of_choice,
            weapon,
            |i| input(EditInput::WeaponOfChoice(i)),
            context,
        ),
        forms::choice(
            &v.expertise,
            weapon,
            |i| input(EditInput::Expertise(i)),
            context,
        ),
        list(ListKind::Specializations, &v.specializations, context),
        forms::choice(
            &v.mastery,
            weapon,
            |i| input(EditInput::Mastery(i)),
            context,
        ),
        forms::unarmed(
            &v.unarmed,
            |i, on| input(EditInput::Unarmed(i, on)),
            context,
        ),
    ));
    let kit = group((
        forms::choice(&v.focus, weapon, |i| input(EditInput::Focus(i)), context),
        forms::choice(
            &v.picks[0],
            weapon,
            |i| input(EditInput::KitPick(0, i)),
            context,
        ),
        forms::choice(
            &v.picks[1],
            weapon,
            |i| input(EditInput::KitPick(1, i)),
            context,
        ),
        View::empty(),
        View::empty(),
    ));
    let rest = group((
        shields,
        list(ListKind::Armor, &v.armor, context),
        list(ListKind::Styles, &v.styles, context),
        list(ListKind::Traits, &v.traits, context),
        list(ListKind::Disadvantages, &v.disadvantages, context),
    ));
    StackPanel::new()
        .spacing(4.0)
        .children((weapons, specialization, kit, rest))
}

/// The Skills page, with the Wild Talents dialog's Test and Roll for a
/// character who is no psionicist.
fn skills_tab(editor: &Editor, adding: &Adding, context: &mut ViewContext<Shell>) -> View {
    let wild: View = match editor.skills.wild() {
        Some(wild) => StackPanel::new().spacing(4.0).children((
            heading(tr("Wild Talents"), None),
            ui::setting(
                tr("Test for Wild Talents"),
                &wild.chance,
                Button::new()
                    .is_enabled(wild.can_test)
                    .on_click(context.message(input(EditInput::TestWild)))
                    .content(tr("Test")),
            ),
            ui::setting(
                tr("Roll for Wild Talents"),
                tr("Table 83: Wild Talents"),
                Button::new()
                    .on_click(context.message(input(EditInput::RollWild)))
                    .content(tr("Roll")),
            ),
        )),
        None => View::empty(),
    };
    forms::skills(&editor.skills, adding, SKILLS, wild, context)
}

/// The Spells page: the kit's barred schools, the schools to eliminate,
/// the spheres to keep and the free sphere; the wizard and priest spells
/// known, each offered with the chance to learn it; the spellbooks; and
/// the spells each class that casts has memorized.
fn spells_tab(editor: &Editor, adding: &Adding, context: &mut ViewContext<Shell>) -> View {
    let magic = editor.kit_magic_view();
    let barred: View = match &magic.barred {
        Some((warning, ignored)) => StackPanel::new().spacing(4.0).children((
            heading(tr("Kit Class Ability Requirements"), Some(warning)),
            ui::setting(
                tr("Ignore"),
                tr("Keeps these schools; otherwise they are removed when the character is saved."),
                ToggleSwitch::new()
                    .automation_name(tr("Ignore"))
                    .is_on(*ignored)
                    .on_toggled(context.callback(|on| input(EditInput::IgnoreBarred(on)))),
            ),
        )),
        None => View::empty(),
    };
    let lists = |lists: Vec<KeyedView>| StackPanel::new().spacing(4.0).keyed_children(lists);
    let known: Vec<KeyedView> = [(ListKind::Spells, true), (ListKind::PriestSpells, false)]
        .into_iter()
        .map(|(kind, wizard)| {
            let (list, chances) = editor.spells_known(wizard);
            KeyedView::new(
                format!("{kind:?}"),
                forms::list(kind, &list, &chances, adding, LISTS, context),
            )
        })
        .collect();
    let books: Vec<KeyedView> = editor
        .spellbooks()
        .into_iter()
        .enumerate()
        .map(|(b, (list, most))| {
            let kind = ListKind::Spellbook(b);
            let rows = Rows {
                number: (tr("Pages"), 1, most),
                icons: None,
            };
            KeyedView::new(
                format!("{kind:?}"),
                forms::list_with(kind, &list, &[], adding, rows, LISTS, context),
            )
        })
        .collect();
    let memorized: Vec<KeyedView> = editor
        .memorized()
        .into_iter()
        .map(|(slot, list)| {
            let kind = ListKind::Memorized(slot);
            KeyedView::new(
                format!("{kind:?}"),
                forms::list(kind, &list, &[], adding, LISTS, context),
            )
        })
        .collect();
    StackPanel::new().spacing(4.0).children((
        barred,
        check_groups(magic.groups, EditInput::MagicTick, context),
        forms::choice(
            &magic.free_sphere,
            tr("Selected Free Sphere"),
            |i| input(EditInput::FreeSphere(i)),
            context,
        ),
        lists(known),
        lists(books),
        lists(memorized),
    ))
}

/// The Equipment page: the spending money with Buy and Sell, then the
/// items of each other bundle with their icons, `icons`.
fn equipment_tab(
    editor: &Editor,
    adding: &Adding,
    icons: &Icons,
    context: &mut ViewContext<Shell>,
) -> View {
    let coins = editor.coins();
    let coins: Vec<KeyedView> = COIN_NAMES
        .iter()
        .enumerate()
        .map(|(c, name)| {
            KeyedView::new(
                *name,
                ui::setting(
                    tr(name),
                    "",
                    forms::number(
                        tr(name),
                        coins[c],
                        (0, MOST_COINS),
                        move |n| input(EditInput::Coin(c, n)),
                        context,
                    ),
                ),
            )
        })
        .collect();
    let bundles: Vec<KeyedView> = editor
        .bundles()
        .into_iter()
        .map(|(b, list, prices)| {
            let kind = ListKind::Bundle(b);
            let rows = Rows {
                number: (tr("Quantity"), 1, MOST_QUANTITY),
                icons: Some(icons),
            };
            KeyedView::new(
                format!("{kind:?}"),
                forms::list_with(kind, &list, &prices, adding, rows, LISTS, context),
            )
        })
        .collect();
    StackPanel::new().spacing(4.0).children((
        heading(tr("Spending Money"), None),
        ui::setting(
            tr("Buy and Sell"),
            tr("Items added are paid for with the spending money, items removed are sold"),
            ToggleSwitch::new()
                .automation_name(tr("Buy and Sell"))
                .is_on(editor.trading)
                .on_toggled(context.callback(|on| input(EditInput::Trading(on)))),
        ),
        StackPanel::new().spacing(4.0).keyed_children(coins),
        StackPanel::new().spacing(4.0).keyed_children(bundles),
    ))
}
