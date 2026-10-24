//! The character editor's Skills page: the Expansion's Skills page of
//! Update Character, with its Rogue Skills, Psionics, Wild Talents and
//! Psionic Attacks and Defenses dialogs as groups.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::character::Character;
use cr_i18n::tr;
use cr_store::create::Dice;
use cr_store::skills::wild::WildMode;
use cr_store::skills::{CharacterSkills, SkillRules};

use crate::abilities::described_list;
use crate::combat::WeaponCombo;
use crate::create::ask_each;
use crate::lists::{RowList, RowSpec};

/// What Test needs from the rest of the editor: the character as edited
/// (its levels and abilities), and a way to change Constitution,
/// Intelligence and Wisdom (`data.abilities[1..4]`).
pub struct WildHooks {
    pub character: Rc<dyn Fn() -> Character>,
    pub set_abilities: Rc<dyn Fn([[u32; 3]; 3])>,
}

/// The page's widgets, cheap to clone into callbacks.
#[derive(Clone)]
struct State {
    rules: Rc<SkillRules>,
    original: Rc<CharacterSkills>,
    /// The kit's bonuses and chosen skills as edited.
    kit: Rc<RefCell<CharacterSkills>>,
    rogue: Vec<adw::SpinRow>,
    primary: Vec<WeaponCombo>,
    disciplines: Option<RowList<()>>,
    sciences: Option<RowList<()>>,
    devotions: Option<RowList<()>>,
    wild_talents: Option<RowList<()>>,
    attacks: RowList<()>,
    defenses: RowList<()>,
    /// Made a wild talent by a test, and tested at all.
    wild: Rc<Cell<(bool, bool)>>,
}

impl State {
    fn skills(&self) -> CharacterSkills {
        let titles = |list: &Option<RowList<()>>, old: &[String]| {
            list.as_ref()
                .map_or_else(|| old.to_vec(), RowList::rows_titles)
        };
        let o = &self.original;
        let mut primary = o.primary.clone();
        for (slot, combo) in primary.iter_mut().zip(&self.primary) {
            *slot = combo.value().unwrap_or_default();
        }
        let (mut sciences, mut devotions) = (
            titles(&self.sciences, &o.sciences),
            titles(&self.devotions, &o.devotions),
        );
        // Wild talents are stored as sciences and devotions.
        if let Some(list) = &self.wild_talents {
            (devotions, sciences) = list
                .rows_titles()
                .into_iter()
                .partition(|n| self.rules.power(n).is_some_and(|p| p.devotion));
        }
        let kit = self.kit.borrow();
        CharacterSkills {
            rogue: self.rogue.iter().map(|r| r.value() as i32).collect(),
            kit_bonuses: kit.kit_bonuses.clone(),
            kit_chosen: kit.kit_chosen.clone(),
            primary,
            disciplines: titles(&self.disciplines, &o.disciplines),
            sciences,
            devotions,
            attacks: self.attacks.rows_titles(),
            defenses: self.defenses.rows_titles(),
            wild_talent: self.wild.get().0,
            tested: self.wild.get().1,
        }
    }
}

pub struct SkillsEditor {
    pub page: adw::PreferencesPage,
    state: State,
}

/// "Telepathic devotion, from level 3", for a power.
pub(crate) fn power_text(rules: &SkillRules, name: &str) -> String {
    let Some(power) = rules.power(name) else {
        return String::new();
    };
    let discipline = if power.discipline.is_empty() {
        tr("Metapsionic")
    } else {
        tr(&power.discipline)
    };
    let kind = if power.devotion {
        tr!("{discipline} devotion", discipline)
    } else {
        tr!("{discipline} science", discipline)
    };
    if power.level > 0 {
        tr!("{kind}, from level {level}", kind, level = power.level)
    } else {
        kind
    }
}

impl SkillsEditor {
    pub fn new(rules: Rc<SkillRules>, character: &Character, hooks: WildHooks) -> Self {
        let original = Rc::new(rules.skills_of(character));
        let wild = Rc::new(Cell::new((original.wild_talent, original.tested)));
        let page = adw::PreferencesPage::new();
        let character = Rc::new(character.clone());

        let points = gtk::Label::builder().css_classes(["numeric"]).build();
        let warnings = gtk::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .css_classes(["warning"])
            .visible(false)
            .build();
        let mut rogue = Vec::new();
        let kit = Rc::new(RefCell::new((*original).clone()));
        let mut rogue_group = None;
        let skills = rules.rogue_skills(&character);
        if rules.rogue_level(&character).is_some() {
            let group = adw::PreferencesGroup::builder()
                .title(tr("Rogue Skills"))
                .build();
            let row = adw::ActionRow::builder().title(tr("Points Left")).build();
            row.add_suffix(&points);
            group.add(&row);
            for (skill, &value) in skills.iter().zip(&original.rogue) {
                let row = adw::SpinRow::builder()
                    .title(&skill.label)
                    .adjustment(&gtk::Adjustment::new(
                        f64::from(value),
                        f64::from(value.min(0)),
                        250.0,
                        1.0,
                        10.0,
                        0.0,
                    ))
                    .build();
                group.add(&row);
                rogue.push(row);
            }
            group.add(&warnings);
            page.add(&group);
            rogue_group = Some(group);
            for group in kit_rogue_groups(&rules, &character, &original, &kit, &rogue) {
                page.add(&group);
            }
        }

        let (mut primary, mut disciplines, mut sciences, mut devotions, mut wild_talents) =
            (Vec::new(), None, None, None, None);
        let psionic_level = rules.psionic_level(&character);
        if let Some(level) = psionic_level {
            let (most_disciplines, most_sciences, most_devotions) = rules.allowed(&character);
            let group = adw::PreferencesGroup::builder()
                .title(tr("Psionics"))
                .description(tr!(
                    "At psionicist level {level}: up to {disciplines} disciplines, \
                     {sciences} sciences and {devotions} devotions.",
                    level,
                    disciplines = most_disciplines,
                    sciences = most_sciences,
                    devotions = most_devotions
                ))
                .build();
            let titles = [tr("Primary Discipline"), tr("Second Primary Discipline")];
            let count = if rules.adept(&character) { 2 } else { 1 };
            for (title, selected) in titles.iter().zip(&original.primary).take(count) {
                let combo = WeaponCombo::new(
                    title,
                    "",
                    original.disciplines.clone(),
                    Some(selected).filter(|s| !s.is_empty()),
                );
                group.add(&combo.row);
                primary.push(combo);
            }
            page.add(&group);

            let offer = rules.discipline_names();
            let list = described_list(
                tr("Disciplines"),
                &original.disciplines,
                |_| String::new(),
                move |list| {
                    if list.rows_titles().len() < most_disciplines {
                        offer.clone()
                    } else {
                        Vec::new()
                    }
                },
            );
            page.add(&list.group);
            let powers = |title: &str, names: &[String], devotion: bool, most: usize| {
                let (text, offer) = (rules.clone(), rules.clone());
                let (character, disciplines) = (character.clone(), list.clone());
                let list = described_list(
                    title,
                    names,
                    move |name| power_text(&text, name),
                    move |list| {
                        if list.rows_titles().len() < most {
                            offer.power_choices(&character, &disciplines.rows_titles(), devotion)
                        } else {
                            Vec::new()
                        }
                    },
                );
                page.add(&list.group);
                Some(list)
            };
            sciences = powers(tr("Sciences"), &original.sciences, false, most_sciences);
            devotions = powers(tr("Devotions"), &original.devotions, true, most_devotions);
            disciplines = Some(list);
        } else {
            let (text, offer) = (rules.clone(), rules.wild_talent_choices());
            let taken: Vec<String> = original
                .sciences
                .iter()
                .chain(&original.devotions)
                .cloned()
                .collect();
            let list = described_list(
                tr("Your Wild Talents"),
                &taken,
                move |name| power_text(&text, name),
                move |_| offer.clone(),
            );
            list.group.set_description(Some(tr(
                "Any character but a psionicist may have wild talents. A character \
                 without PSPs rolls them when the first is taken.",
            )));
            page.add(&wild_group(
                &rules, &character, &original, &list, &wild, hooks,
            ));
            page.add(&list.group);
            wild_talents = Some(list);
        }

        let modes = |title: &str, names: &[String], offer: &[String]| {
            let offer = offer.to_vec();
            let list = described_list(title, names, |_| String::new(), move |_| offer.clone());
            page.add(&list.group);
            list
        };
        let attacks = modes(
            tr("Psionic Attack Modes"),
            &original.attacks,
            rules.attack_names(),
        );
        let defenses = modes(
            tr("Psionic Defense Modes"),
            &original.defenses,
            rules.defense_names(),
        );

        let state = State {
            rules: rules.clone(),
            original,
            kit,
            rogue,
            primary,
            disciplines,
            sciences,
            devotions,
            wild_talents,
            attacks,
            defenses,
            wild,
        };
        let refresh = Rc::new({
            let (state, rules, character) = (state.clone(), rules.clone(), character.clone());
            move || {
                let new = state.skills();
                if let Some((group, (_, level))) =
                    rogue_group.as_ref().zip(rules.rogue_level(&character))
                {
                    let mut description = tr!(
                        "The points put in at level {level}; lowering a skill gives its points \
                         back.",
                        level
                    );
                    if let Some(limit) = rules.rogue_limit(&character, &new) {
                        description.push(' ');
                        description
                            .push_str(&tr!("One skill should get at most {limit} of them.", limit));
                    }
                    group.set_description(Some(&description));
                }
                let minimums = rules.rogue_minimums(&character, &new);
                for ((row, &least), &value) in state.rogue.iter().zip(&minimums).zip(&new.rogue) {
                    row.adjustment().set_lower(f64::from(least.min(value)));
                }
                let kit_points = rules.rogue_kit_points(&character, &new);
                let left = rules.rogue_points_left(&character, &new);
                points.set_label(&left.to_string());
                if left < 0 {
                    points.add_css_class("error");
                } else {
                    points.remove_css_class("error");
                }
                for (((row, skill), value), (adjustment, levels)) in state
                    .rogue
                    .iter()
                    .zip(&skills)
                    .zip(&new.rogue)
                    .zip(&kit_points)
                {
                    let mut subtitle = tr!("Score {score}%", score = skill.base_score + *value);
                    if *adjustment != 0 {
                        subtitle.push_str(" · ");
                        subtitle.push_str(&tr!(
                            "kit modifier {points}",
                            points = format!("{adjustment:+}")
                        ));
                    }
                    for (level, bonus) in levels {
                        subtitle.push_str(" · ");
                        subtitle.push_str(&tr!(
                            "level {level}: {points}",
                            level = *level,
                            points = format!("{bonus:+}")
                        ));
                    }
                    row.set_subtitle(&subtitle);
                }
                let found = rules.rogue_warnings(&character, &new);
                warnings.set_label(&found.join("\n"));
                warnings.set_visible(!found.is_empty());
                // A non-psionicist has attack and defense modes with wild
                // talents.
                let modes = psionic_level.is_some()
                    || !new.attacks.is_empty()
                    || !new.defenses.is_empty()
                    || !new.sciences.is_empty()
                    || !new.devotions.is_empty();
                state.attacks.group.set_visible(modes);
                state.defenses.group.set_visible(modes);
            }
        });
        for (index, row) in state.rogue.iter().enumerate() {
            // Points past what is left, past the most one skill may get
            // at this level, or past a 95% score are refused: the row goes
            // back.
            let before = Rc::new(Cell::new(row.value()));
            let (refresh, state, rules, character) = (
                refresh.clone(),
                state.clone(),
                rules.clone(),
                character.clone(),
            );
            row.connect_value_notify(move |row| {
                let over = || {
                    let new = state.skills();
                    let mut old = new.clone();
                    if let Some(points) = old.rogue.get_mut(index) {
                        *points = before.get() as i32;
                    }
                    rules.rogue_points_left(&character, &new) < 0
                        || rules.rogue_warnings(&character, &new).len()
                            > rules.rogue_warnings(&character, &old).len()
                };
                if row.value() > before.get() && over() {
                    row.set_value(before.get());
                    row.error_bell();
                    return;
                }
                before.set(row.value());
                refresh();
            });
        }

        // The primary disciplines are among the disciplines taken.
        if let Some(list) = &state.disciplines {
            let (state, refresh) = (state.clone(), refresh.clone());
            list.connect_changed(move || {
                let taken = state
                    .disciplines
                    .as_ref()
                    .map(RowList::rows_titles)
                    .unwrap_or_default();
                for combo in &state.primary {
                    combo.refill(taken.clone());
                }
                refresh();
            });
        }
        for list in [&state.sciences, &state.devotions, &state.wild_talents]
            .into_iter()
            .flatten()
            .chain([&state.attacks, &state.defenses])
        {
            let refresh = refresh.clone();
            list.connect_changed(move || refresh());
        }
        refresh();
        Self { page, state }
    }

    /// The choices as edited.
    pub fn skills(&self) -> CharacterSkills {
        self.state.skills()
    }
}

/// Asks the questions in turn, then runs `then` once.
pub(crate) fn ask_then(
    parent: &gtk::Widget,
    questions: Vec<cr_store::create::Question>,
    then: Box<dyn FnOnce()>,
) {
    let then = RefCell::new(Some(then));
    ask_each(
        parent,
        VecDeque::from(questions),
        Rc::new(move || {
            if let Some(then) = then.borrow_mut().take() {
                then();
            }
        }),
    );
}

/// The Wild Talents dialog's Test and Roll buttons (IDD 358, 0x6366b7 and
/// 0x635aa3), and its question before Add (0x6363c7).
fn wild_group(
    rules: &Rc<SkillRules>,
    character: &Rc<Character>,
    original: &CharacterSkills,
    list: &RowList<()>,
    wild: &Rc<Cell<(bool, bool)>>,
    hooks: WildHooks,
) -> adw::PreferencesGroup {
    let dice = Rc::new(RefCell::new(Dice::default()));
    let group = adw::PreferencesGroup::builder()
        .title(tr("Wild Talents"))
        .build();
    let button = |label: &str| {
        gtk::Button::builder()
            .label(label)
            .valign(gtk::Align::Center)
            .build()
    };
    let (test, roll) = (button(tr("Test")), button(tr("Roll")));
    let test_row = adw::ActionRow::builder()
        .title(tr("Test for Wild Talents"))
        .build();
    test_row.add_suffix(&test);
    // A wild talent already does not test, as in the original.
    test_row.set_visible(!rules.is_wild_talent(character, original.wild_talent));
    let roll_row = adw::ActionRow::builder()
        .title(tr("Roll for Wild Talents"))
        .subtitle(tr("Table 83: Wild Talents"))
        .build();
    roll_row.add_suffix(&roll);
    group.add(&test_row);
    group.add(&roll_row);
    let hooks = Rc::new(hooks);
    let show_chance = {
        let (rules, hooks, row) = (rules.clone(), hooks.clone(), test_row.clone());
        move || {
            let chance = rules.wild_chance(&(hooks.character)());
            row.set_subtitle(&tr!("A {chance}% chance to become a wild talent", chance));
        }
    };
    show_chance();
    // The abilities change on another page.
    test_row.connect_map(move |_| show_chance());

    test.connect_clicked({
        let (rules, wild, dice, hooks) = (rules.clone(), wild.clone(), dice.clone(), hooks.clone());
        move |button| {
            let character = (hooks.character)();
            let questions = rules.test_questions(&character, wild.get().1);
            let parent = button.clone().upcast::<gtk::Widget>();
            let (rules, wild, dice, hooks, button) = (
                rules.clone(),
                wild.clone(),
                dice.clone(),
                hooks.clone(),
                button.clone(),
            );
            ask_then(
                &parent.clone(),
                questions,
                Box::new(move || {
                    let test = rules.test_wild_talent(&character, &mut dice.borrow_mut());
                    ask_then(
                        &parent,
                        test.notices.clone(),
                        Box::new(move || {
                            wild.set((wild.get().0 || test.wild_talent, true));
                            if test.wild_talent {
                                button.set_sensitive(false);
                            }
                            (hooks.set_abilities)(test.abilities);
                        }),
                    );
                }),
            );
        }
    });

    roll.connect_clicked({
        let (rules, list, dice) = (rules.clone(), list.clone(), dice.clone());
        move |button| {
            let parent = button.clone().upcast::<gtk::Widget>();
            let (rules, list, dice) = (rules.clone(), list.clone(), dice.clone());
            ask_then(
                &parent.clone(),
                rules
                    .roll_questions(&list.rows_titles(), WildMode::Update)
                    .unwrap_or_default(),
                Box::new(move || {
                    let (names, notices) = rules.roll_wild_talents(&mut dice.borrow_mut());
                    list.clear();
                    for name in names {
                        list.add(RowSpec {
                            value: (),
                            subtitle: power_text(&rules, &name),
                            title: name,
                            number: None,
                        });
                    }
                    ask_then(&parent, notices, Box::new(|| {}));
                }),
            );
        }
    });

    list.connect_add({
        let (rules, group) = (rules.clone(), group.clone());
        move |_, add| ask_then(group.upcast_ref(), vec![rules.add_question()], add)
    });
    group
}

/// The kit's bonus of each level still to go to a skill, and the skills
/// it has the character choose, as groups of the original's pickers
/// (62cc45, 62c8e6). Each pick goes into `kit` and the rows' points.
fn kit_rogue_groups(
    rules: &Rc<SkillRules>,
    character: &Rc<Character>,
    original: &Rc<CharacterSkills>,
    kit: &Rc<RefCell<CharacterSkills>>,
    rows: &[adw::SpinRow],
) -> Vec<adw::PreferencesGroup> {
    let current = {
        let (kit, rows, original) = (kit.clone(), rows.to_vec(), original.clone());
        move || {
            let kit = kit.borrow();
            CharacterSkills {
                rogue: rows.iter().map(|r| r.value() as i32).collect(),
                kit_bonuses: kit.kit_bonuses.clone(),
                kit_chosen: kit.kit_chosen.clone(),
                ..(*original).clone()
            }
        }
    };
    let skills = rules.rogue_skills(character);
    let label = |name: &str| {
        skills
            .iter()
            .find(|s| s.name == name)
            .map_or_else(|| name.to_owned(), |s| s.label.clone())
    };
    let mut groups = Vec::new();
    for pick in rules.rogue_bonus_picks(character) {
        let description = tr!(
            "The selected kit allows you to add {points} point(s) to one rogue skill from the \
             following list at level {level}.",
            points = pick.amount,
            level = pick.level
        );
        let group = adw::PreferencesGroup::builder()
            .title(tr!("Kit Bonus at Level {level}", level = pick.level))
            .description(&description)
            .build();
        let mut first: Option<gtk::CheckButton> = None;
        let picked = original
            .kit_bonuses
            .get(pick.level as usize - 1)
            .and_then(Option::as_ref)
            .map(|b| b.name.clone());
        for name in &pick.names {
            let check = gtk::CheckButton::builder()
                .active(picked.as_ref() == Some(name))
                .valign(gtk::Align::Center)
                .build();
            check.set_group(first.as_ref());
            first.get_or_insert_with(|| check.clone());
            let row = adw::ActionRow::builder()
                .title(label(name))
                .activatable_widget(&check)
                .build();
            row.add_prefix(&check);
            group.add(&row);
            check.connect_toggled({
                let (rules, character, kit, rows, current, name) = (
                    rules.clone(),
                    character.clone(),
                    kit.clone(),
                    rows.to_vec(),
                    current.clone(),
                    name.clone(),
                );
                move |check| {
                    if !check.is_active() {
                        return;
                    }
                    let mut new = current();
                    rules.set_rogue_bonus(&character, &mut new, pick.level, Some(&name));
                    kit.borrow_mut().kit_bonuses = new.kit_bonuses.clone();
                    for (row, &value) in rows.iter().zip(&new.rogue) {
                        row.adjustment().set_lower(f64::from(value.min(0)));
                        row.set_value(f64::from(value));
                    }
                }
            });
        }
        groups.push(group);
    }
    if let Some(pick) = rules.chosen_pick(character) {
        let group = adw::PreferencesGroup::builder()
            .title(tr("Chosen Rogue Skills"))
            .description(tr!(
                "The selected kit allows you to select {count} skills as 'chosen' for a \
                 {bonus}% bonus.",
                count = pick.count,
                bonus = pick.bonus
            ))
            .build();
        let checks: Rc<RefCell<Vec<(String, gtk::CheckButton)>>> = Rc::default();
        let limit = {
            let checks = checks.clone();
            move || {
                let checks = checks.borrow();
                let full = checks.iter().filter(|(_, c)| c.is_active()).count() >= pick.count;
                for (_, check) in checks.iter() {
                    check.set_sensitive(check.is_active() || !full);
                }
            }
        };
        for name in &pick.names {
            let check = gtk::CheckButton::builder()
                .valign(gtk::Align::Center)
                .build();
            let row = adw::ActionRow::builder()
                .title(label(name))
                .activatable_widget(&check)
                .build();
            row.add_prefix(&check);
            group.add(&row);
            checks.borrow_mut().push((name.clone(), check.clone()));
            check.connect_toggled({
                let (rules, character, kit, current, checks, limit) = (
                    rules.clone(),
                    character.clone(),
                    kit.clone(),
                    current.clone(),
                    checks.clone(),
                    limit.clone(),
                );
                move |_| {
                    let names: Vec<String> = checks
                        .borrow()
                        .iter()
                        .filter(|(_, c)| c.is_active())
                        .map(|(n, _)| n.clone())
                        .collect();
                    let mut new = current();
                    rules.set_chosen(&character, &mut new, &names);
                    kit.borrow_mut().kit_chosen = new.kit_chosen;
                    limit();
                }
            });
        }
        groups.push(group);
    }
    groups
}
