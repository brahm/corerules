//! The editor's Combat page: the Expansion's Weapon Proficiencies page of
//! Update Character, with its Specialization, Shield and Armor and
//! Fighting Styles dialogs as groups, and the traits and disadvantages of
//! its Nonweapon Proficiencies page. Taking a weapon proficiency, a group,
//! a specialization or an unarmed style asks what the original asks and
//! gives the kit's warnings, as New Character does.

use std::collections::VecDeque;
use std::sync::Arc;

use cr_formats::character::Character;
use cr_i18n::{tr, trn};
use cr_store::combat::Purchase;
use cr_store::create::{Question, SHIELDS, UNARMED};

use super::{EditInput, Editor, Prompt, Then};
use crate::create::{Choice, List, ListKind, Unarmed, cost_text, index_of, item_at, with_none};

/// What the Combat page shows.
#[derive(Debug, Clone, PartialEq)]
pub struct CombatView {
    /// Skills & Powers: the character points left, which the Abilities
    /// page spends too.
    pub points_left: Option<i64>,
    pub description: String,
    pub weapons: List,
    pub broad: List,
    pub tight: List,
    pub weapon_of_choice: Choice,
    pub expertise: Choice,
    pub mastery: Choice,
    pub unarmed: Unarmed,
    pub specializations: List,
    /// The weapon the kit has the character focus in, and the ones it has
    /// the character pick.
    pub focus: Choice,
    pub picks: [Choice; 2],
    /// Skills & Powers: each shield with its cost, and whether it is
    /// taken.
    pub shields: Vec<(String, String, bool)>,
    pub armor: List,
    pub styles: List,
    pub traits: List,
    pub disadvantages: List,
}

/// What a taken purchase does with what it cost.
type Take = Arc<dyn Fn(&mut Editor, i64) + Send + Sync>;

impl Editor {
    /// The class levels as the form has them now.
    pub(super) fn levels(&self) -> Vec<u32> {
        self.needs
            .advance
            .levels(&self.advanced)
            .map_or_else(|_| vec![1], |l| l.iter().map(|c| c.level).collect())
    }

    /// The character as read with the kit as chosen, which the kit's
    /// focus and picks follow.
    fn kit_character(&self) -> std::borrow::Cow<'_, Character> {
        if self.details.kit == self.original.data.kit {
            std::borrow::Cow::Borrowed(&self.original)
        } else {
            let mut character = (*self.original).clone();
            character.data.kit.clone_from(&self.details.kit);
            std::borrow::Cow::Owned(character)
        }
    }

    /// Points are shown for Skills & Powers characters only; Core Rules
    /// ones pay slots, which the original only counts.
    fn cost_line(&self, points: i64) -> String {
        if self.original.skills_and_powers() {
            cost_text(points)
        } else {
            String::new()
        }
    }

    /// What a weapon or group cost when it was taken here, else what it
    /// costs now.
    fn charged(&self, name: &str) -> Option<i64> {
        self.combat
            .bought
            .iter()
            .rev()
            .find(|b| b.0 == name)
            .map(|b| b.1)
    }

    /// The weapons held, the groups' too.
    fn covered_weapons(&self) -> Vec<String> {
        self.needs
            .combat
            .covered_weapons(&self.original, &self.combat, &self.levels())
    }

    pub fn combat_view(&self) -> CombatView {
        let rules = &self.needs.combat;
        let character = &*self.original;
        let c = &self.combat;
        let levels = self.levels();
        let skills_and_powers = character.skills_and_powers();
        let list = |title: &str, rows: Vec<(String, String, Option<u32>)>, choices, shown| List {
            title: title.to_owned(),
            description: String::new(),
            rows,
            choices,
            shown,
            fixed: false,
        };
        let not_taken = |offer: Vec<String>, taken: &[String]| -> Vec<String> {
            offer.into_iter().filter(|n| !taken.contains(n)).collect()
        };

        let original = &self.original_combat;
        let mut weapons = list(
            tr("Weapon Proficiencies"),
            c.weapons
                .iter()
                .map(|name| {
                    let cost = self
                        .charged(name)
                        .unwrap_or_else(|| rules.weapon_price(character, original, &levels, name));
                    (name.clone(), self.cost_line(cost), None)
                })
                .collect(),
            not_taken(rules.weapon_names(), &c.weapons),
            true,
        );
        if let Some(left) = rules.core_slots_left(character, c, &levels) {
            weapons.description = trn!("{n} slot left", "{n} slots left", left);
        }

        let group = |broad: bool| {
            let taken = if broad {
                &c.broad_groups
            } else {
                &c.tight_groups
            };
            let mut group = list(
                if broad {
                    tr("Broad Weapon Groups")
                } else {
                    tr("Tight Weapon Groups")
                },
                taken
                    .iter()
                    .map(|name| {
                        let weapons = rules.group_weapons(broad, name).join(", ");
                        let line = match self.charged(name) {
                            Some(cost) => format!("{}: {weapons}", cost_text(cost)),
                            None => weapons,
                        };
                        (name.clone(), line, None)
                    })
                    .collect(),
                rules.group_choices(character, c, &levels, broad),
                self.groups_shown,
            );
            group.description = cost_text(if broad { 6 } else { 4 });
            group
        };

        let covered = with_none(self.covered_weapons());
        let weapon_choice = |title: &str, cost: i64, items: &Vec<String>, chosen| Choice {
            title: title.to_owned(),
            description: cost_text(cost),
            selected: index_of(items, chosen),
            items: items.clone(),
            shown: skills_and_powers,
        };
        let specialized = with_none(c.specializations.clone());
        let specialization = rules.specialization_cost(character);

        let kit_character = self.kit_character();
        let focus = match rules.focus_choices(&kit_character, c, &levels) {
            Some((prompt, weapons)) => {
                let items = with_none(weapons);
                Choice {
                    title: tr("Weapon Focus").to_owned(),
                    description: prompt,
                    selected: index_of(&items, c.focus.as_ref()),
                    items,
                    shown: true,
                }
            }
            None => Choice::default(),
        };
        let kit_picks = rules.kit_picks(&kit_character, c, &levels);
        let picks = std::array::from_fn(|slot| match kit_picks.iter().find(|p| p.slot == slot) {
            Some(pick) => {
                let items = with_none(pick.choices.clone());
                Choice {
                    title: pick.title.clone(),
                    description: pick.text.clone(),
                    selected: index_of(&items, c.kit_picks[slot].as_ref()),
                    items,
                    shown: true,
                }
            }
            None => Choice::default(),
        });

        let shield_cost = cost_text(rules.shield_cost(character));
        let armor_cost = cost_text(rules.armor_cost(character));
        let single_style = rules.single_style(character);
        let mut styles = list(
            tr("Fighting Styles"),
            c.styles
                .iter()
                .map(|name| {
                    let cost = cost_text(rules.style_cost(character, name));
                    (name.clone(), cost, None)
                })
                .collect(),
            if single_style && !c.styles.is_empty() {
                Vec::new()
            } else {
                not_taken(rules.style_names(), &c.styles)
            },
            skills_and_powers,
        );
        if single_style {
            styles.description =
                tr("A character without a warrior class may have a single style.").to_owned();
        }

        CombatView {
            points_left: skills_and_powers.then(|| self.points_left()),
            description: if skills_and_powers {
                tr(
                    "Removing a weapon proficiency or weapon group refunds nothing; everything \
                     else on this page is refunded when removed.",
                )
                .to_owned()
            } else {
                String::new()
            },
            weapons,
            broad: group(true),
            tight: group(false),
            weapon_of_choice: weapon_choice(
                tr("Weapon of Choice"),
                rules.weapon_of_choice_cost(character),
                &covered,
                c.weapon_of_choice.as_ref(),
            ),
            expertise: weapon_choice(
                tr("Weapon Expertise"),
                rules.expertise_cost(character),
                &covered,
                c.expertise.as_ref(),
            ),
            mastery: weapon_choice(
                tr("Weapon Mastery"),
                rules.mastery_cost(character),
                &specialized,
                c.mastery.as_ref(),
            ),
            unarmed: Unarmed {
                cost: if skills_and_powers {
                    cost_text(specialization)
                } else {
                    trn!("{n} slot", "{n} slots", 2)
                },
                taken: c.unarmed,
                shown: true,
            },
            specializations: list(
                tr("Weapon Specializations"),
                c.specializations
                    .iter()
                    .map(|name| (name.clone(), self.cost_line(specialization), None))
                    .collect(),
                not_taken(self.covered_weapons(), &c.specializations),
                true,
            ),
            focus,
            picks,
            shields: if skills_and_powers {
                SHIELDS
                    .iter()
                    .zip(c.shields)
                    .map(|(name, on)| ((*name).to_owned(), shield_cost.clone(), on))
                    .collect()
            } else {
                Vec::new()
            },
            armor: list(
                tr("Armor Proficiencies"),
                c.armor
                    .iter()
                    .map(|name| (name.clone(), armor_cost.clone(), None))
                    .collect(),
                not_taken(rules.armor_names().to_vec(), &c.armor),
                skills_and_powers,
            ),
            styles,
            traits: list(
                tr("Traits"),
                c.traits
                    .iter()
                    .map(|name| {
                        let cost = cost_text(rules.trait_cost(character, name));
                        (name.clone(), cost, None)
                    })
                    .collect(),
                not_taken(rules.trait_names(), &c.traits),
                skills_and_powers,
            ),
            disadvantages: list(
                tr("Disadvantages"),
                c.disadvantages
                    .iter()
                    .map(|name| {
                        let cost = cost_text(-rules.disadvantage_points(character, name));
                        (name.clone(), cost, None)
                    })
                    .collect(),
                not_taken(rules.disadvantage_names(), &c.disadvantages),
                skills_and_powers,
            ),
        }
    }

    /// Asks `questions` in turn, then runs `then` once each is answered
    /// Yes or closed.
    pub(super) fn ask_each(&mut self, mut questions: VecDeque<Question>, then: Then) {
        let Some(question) = questions.pop_front() else {
            then(self);
            return;
        };
        let next: Then =
            Arc::new(move |e: &mut Editor| e.ask_each(questions.clone(), then.clone()));
        self.show(if question.notice {
            Prompt::notice(&question.title, question.text, next)
        } else {
            Prompt::ask(&question.title, question.text, next, None)
        });
    }

    /// Asks a purchase's questions, then `take`s it for what it costs or,
    /// when the points run short and the user says so, for nothing.
    fn buy(&mut self, purchase: Purchase, take: Take) {
        let Purchase {
            questions,
            free,
            cost,
        } = purchase;
        let then: Then = Arc::new(move |e: &mut Editor| match &free {
            Some(free) => {
                let take = take.clone();
                e.show(Prompt::ask(
                    &free.title,
                    free.text.clone(),
                    Arc::new(move |e: &mut Editor| take(e, 0)),
                    None,
                ));
            }
            None => take(e, cost),
        });
        self.ask_each(questions.into(), then);
    }

    /// Takes a weapon proficiency once the questions are answered Yes.
    fn add_weapon(&mut self, name: String) {
        let purchase =
            self.needs
                .combat
                .weapon_purchase(&self.original, &self.combat, &self.levels(), &name);
        self.buy(
            purchase,
            Arc::new(move |e: &mut Editor, cost| e.weapon_warned(name.clone(), cost)),
        );
    }

    /// Takes a weapon proficiency for `cost` once each of the kit's
    /// warnings is ignored.
    fn weapon_warned(&mut self, name: String, cost: i64) {
        let rules = self.needs.combat.clone();
        let levels = self.levels();
        let warning = rules
            .weapon_warnings(&self.original, &self.combat, &levels, &name)
            .into_iter()
            .next();
        if let Some(warning) = warning {
            let check = warning.check;
            self.show(Prompt::ignorable(
                tr("Kit Weapon Proficiency Requirements"),
                &warning.text,
                Arc::new(move |e: &mut Editor| {
                    e.combat.ignored.push(check);
                    e.weapon_warned(name.clone(), cost);
                }),
            ));
            return;
        }
        rules.bought(&self.original, &mut self.combat, &levels, &name, cost);
        self.combat.weapons.push(name);
        self.combat_changed();
    }

    /// Buys a broad (else tight) weapon group once the kit's warning is
    /// ignored and the questions are answered Yes, or says why the
    /// character may not.
    fn add_group(&mut self, broad: bool, name: String) {
        let rules = self.needs.combat.clone();
        let levels = self.levels();
        let warning = rules
            .group_warnings(&self.original, &self.combat, &levels, broad, &name)
            .into_iter()
            .next();
        if let Some(warning) = warning {
            let check = warning.check;
            self.show(Prompt::ignorable(
                tr("Kit Weapon Proficiency Requirements"),
                &warning.text,
                Arc::new(move |e: &mut Editor| {
                    e.combat.ignored.push(check);
                    e.add_group(broad, name.clone());
                }),
            ));
            return;
        }
        match rules.group_purchase(&self.original, &self.combat, &levels, broad) {
            Ok(purchase) => self.buy(
                purchase,
                Arc::new(move |e: &mut Editor, cost| {
                    let rules = e.needs.combat.clone();
                    rules.group_bought(&mut e.combat, &name, cost);
                    if broad {
                        e.combat.broad_groups.push(name.clone());
                    } else {
                        e.combat.tight_groups.push(name.clone());
                    }
                    e.combat_changed();
                }),
            ),
            Err(text) => self.message(tr("Weapon Groups"), text),
        }
    }

    /// Specializes in a weapon once the questions are answered Yes and
    /// the kit's warnings ignored.
    fn specialize(&mut self, name: String) {
        let questions = self.needs.combat.specialization_questions(
            &self.original,
            &self.combat,
            &self.levels(),
            &name,
        );
        self.ask_each(
            questions.into(),
            Arc::new(move |e: &mut Editor| e.specialization_warned(name.clone())),
        );
    }

    fn specialization_warned(&mut self, name: String) {
        let warning = self
            .needs
            .combat
            .specialization_warnings(&self.original, &self.combat, &self.levels(), &name)
            .into_iter()
            .next();
        if let Some(warning) = warning {
            let check = warning.check;
            self.show(Prompt::ignorable(
                tr("Kit Specialization Requirements"),
                &warning.text,
                Arc::new(move |e: &mut Editor| {
                    e.combat.ignored.push(check);
                    e.specialization_warned(name.clone());
                }),
            ));
            return;
        }
        self.combat.specializations.push(name);
        self.combat_changed();
    }

    /// Turns unarmed style `i` on once the questions are answered Yes.
    fn take_unarmed(&mut self, i: usize) {
        let purchase =
            self.needs
                .combat
                .unarmed_purchase(&self.original, &self.combat, &self.levels());
        self.buy(
            purchase,
            Arc::new(move |e: &mut Editor, cost| {
                let (rules, levels) = (e.needs.combat.clone(), e.levels());
                rules.bought(&e.original, &mut e.combat, &levels, UNARMED[i], cost);
                e.combat.unarmed[i] = true;
                e.combat_changed();
            }),
        );
    }

    /// Takes row `i` out of a list; with Skills & Powers a weapon
    /// proficiency or group refunds nothing, which the original says once,
    /// and a group stays while there is specialization information.
    fn remove_combat(&mut self, kind: ListKind, i: usize) {
        let refunds_nothing = matches!(kind, ListKind::Weapons | ListKind::Broad | ListKind::Tight)
            && self.original.skills_and_powers();
        if refunds_nothing {
            let broad = match kind {
                ListKind::Broad => Some(true),
                ListKind::Tight => Some(false),
                _ => None,
            };
            if let Some(broad) = broad
                && let Some(text) = self.needs.combat.group_removal(
                    &self.original,
                    &self.combat,
                    &self.levels(),
                    broad,
                )
            {
                self.message(tr("Weapon Groups"), text);
                return;
            }
            if !std::mem::replace(&mut self.refund_told, true) {
                self.show(Prompt::notice(
                    tr("Warning"),
                    tr(
                        "In Update, character points are not returned for removing Weapon \
                         Proficiencies, Tight Groups, or Broad Groups.",
                    ),
                    Arc::new(move |e: &mut Editor| e.take_out(kind, i)),
                ));
                return;
            }
        }
        self.take_out(kind, i);
    }

    fn take_out(&mut self, kind: ListKind, i: usize) {
        let c = &mut self.combat;
        let list = match kind {
            ListKind::Weapons => &mut c.weapons,
            ListKind::Broad => &mut c.broad_groups,
            ListKind::Tight => &mut c.tight_groups,
            ListKind::Specializations => &mut c.specializations,
            ListKind::Armor => &mut c.armor,
            ListKind::Styles => &mut c.styles,
            ListKind::Traits => &mut c.traits,
            ListKind::Disadvantages => &mut c.disadvantages,
            _ => return,
        };
        if i < list.len() {
            list.remove(i);
            self.combat_changed();
        }
    }

    /// The choices that depend on the weapons held, the specializations,
    /// the kit and the levels follow them: one no longer offered goes back
    /// to none, as the GNOME editor's combos do.
    fn combat_changed(&mut self) {
        let covered = self.covered_weapons();
        let c = &mut self.combat;
        for chosen in [&mut c.weapon_of_choice, &mut c.expertise] {
            if chosen.as_ref().is_some_and(|w| !covered.contains(w)) {
                *chosen = None;
            }
        }
        if c.mastery
            .as_ref()
            .is_some_and(|w| !c.specializations.contains(w))
        {
            c.mastery = None;
        }
        self.kit_weapons_changed();
    }

    /// The kit's focus and picks follow the kit chosen, the levels and
    /// the weapons held.
    pub(super) fn kit_weapons_changed(&mut self) {
        let rules = self.needs.combat.clone();
        let levels = self.levels();
        let kit_character = self.kit_character().into_owned();
        let c = &mut self.combat;
        match rules.focus_choices(&kit_character, c, &levels) {
            Some((_, weapons)) => {
                if c.focus.as_ref().is_some_and(|w| !weapons.contains(w)) {
                    c.focus = None;
                }
            }
            None => c.focus.clone_from(&self.original_combat.focus),
        }
        let picks = rules.kit_picks(&kit_character, c, &levels);
        for slot in 0..2 {
            if self.original_combat.kit_picks[slot].is_some() {
                continue;
            }
            let kept = picks.iter().find(|p| p.slot == slot).is_some_and(|p| {
                c.kit_picks[slot]
                    .as_ref()
                    .is_some_and(|w| p.choices.contains(w))
            });
            if !kept {
                c.kit_picks[slot] = None;
            }
        }
    }

    /// The kit's warnings about the specializations, as leaving the
    /// original's page gives them, then `then` once each is ignored.
    pub(super) fn check_leaving_combat(&mut self, then: Then) {
        let warning = self
            .needs
            .combat
            .leaving_warnings(&self.original, &self.combat, &self.levels())
            .into_iter()
            .next();
        let Some(warning) = warning else {
            then(self);
            return;
        };
        let check = warning.check;
        self.show(Prompt::ignorable(
            tr("Kit Weapon Proficiency Requirements"),
            &warning.text,
            Arc::new(move |e: &mut Editor| {
                e.combat.ignored.push(check);
                e.check_leaving_combat(then.clone());
            }),
        ));
    }

    pub(super) fn combat_input(&mut self, input: EditInput) {
        let view = self.combat_view();
        match input {
            EditInput::Add(kind, name) => {
                let offered = |list: &List| list.shown && list.choices.contains(&name);
                match kind {
                    ListKind::Weapons if offered(&view.weapons) => self.add_weapon(name),
                    ListKind::Broad if offered(&view.broad) => self.add_group(true, name),
                    ListKind::Tight if offered(&view.tight) => self.add_group(false, name),
                    ListKind::Specializations if offered(&view.specializations) => {
                        self.specialize(name);
                    }
                    ListKind::Armor if offered(&view.armor) => self.combat.armor.push(name),
                    ListKind::Styles if offered(&view.styles) => self.combat.styles.push(name),
                    ListKind::Traits if offered(&view.traits) => self.combat.traits.push(name),
                    ListKind::Disadvantages if offered(&view.disadvantages) => {
                        self.combat.disadvantages.push(name);
                    }
                    _ => {}
                }
            }
            EditInput::Remove(kind, i) => self.remove_combat(kind, i),
            EditInput::WeaponOfChoice(i) => {
                self.combat.weapon_of_choice = item_at(&view.weapon_of_choice.items, i);
            }
            EditInput::Expertise(i) => {
                self.combat.expertise = item_at(&view.expertise.items, i);
            }
            EditInput::Mastery(i) => self.combat.mastery = item_at(&view.mastery.items, i),
            EditInput::Focus(i) if view.focus.shown => {
                self.combat.focus = item_at(&view.focus.items, i);
            }
            EditInput::KitPick(slot, i) if slot < 2 && view.picks[slot].shown => {
                self.combat.kit_picks[slot] = item_at(&view.picks[slot].items, i);
            }
            EditInput::Unarmed(i, on) if i < UNARMED.len() => {
                if !on {
                    self.combat.unarmed[i] = false;
                } else if !self.combat.unarmed[i] {
                    self.take_unarmed(i);
                }
            }
            EditInput::Shield(i, on) if i < SHIELDS.len() && !view.shields.is_empty() => {
                self.combat.shields[i] = on;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::AbilityGroup;
    use super::super::tests::{editor, editor_where};
    use super::*;
    use crate::prompt::Answer;

    /// Answers Yes (OK, Ignore) to whatever is asked.
    fn yes_to_all(e: &mut Editor) {
        for _ in 0..20 {
            if e.prompt().is_none() {
                return;
            }
            e.answer(Answer::Yes);
        }
        panic!("too many questions");
    }

    #[test]
    fn a_weapon_proficiency_is_taken_once_asked_and_then_specialized_in() {
        for skills_and_powers in [false, true] {
            if let Some(e) = editor_where(|c| c.skills_and_powers() == skills_and_powers) {
                take_and_specialize(e);
            }
        }
    }

    fn take_and_specialize(mut e: Editor) {
        let view = e.combat_view();
        let Some(weapon) = view.weapons.choices.first().cloned() else {
            return;
        };
        e.input(EditInput::Add(ListKind::Weapons, weapon.clone()));
        yes_to_all(&mut e);
        assert!(e.combat.weapons.contains(&weapon));
        assert!(e.changed());
        let view = e.combat_view();
        assert!(view.specializations.choices.contains(&weapon));
        e.input(EditInput::Add(ListKind::Specializations, weapon.clone()));
        yes_to_all(&mut e);
        assert!(e.combat.specializations.contains(&weapon));
        if e.original.skills_and_powers() {
            let view = e.combat_view();
            let i = view
                .mastery
                .items
                .iter()
                .position(|w| *w == weapon)
                .unwrap();
            e.input(EditInput::Mastery(i));
            assert_eq!(e.combat.mastery.as_ref(), Some(&weapon));
        }
        // Taking the weapon away takes away what needs it.
        let row = e.combat.weapons.iter().position(|w| *w == weapon).unwrap();
        e.input(EditInput::Remove(ListKind::Weapons, row));
        yes_to_all(&mut e);
        assert!(!e.combat.weapons.contains(&weapon));
        let row = e.combat.specializations.iter().position(|w| *w == weapon);
        if let Some(row) = row {
            e.input(EditInput::Remove(ListKind::Specializations, row));
        }
        assert_eq!(e.combat.mastery, None);
    }

    #[test]
    fn skills_and_powers_choices_spend_the_points_the_abilities_share() {
        let Some(mut e) = editor_where(Character::skills_and_powers) else {
            return;
        };
        let before = e.points_left();
        let Some(shield) = e.combat.shields.iter().position(|on| !on) else {
            return;
        };
        e.input(EditInput::Shield(shield, true));
        let cost = e.needs.combat.shield_cost(&e.original);
        assert_eq!(e.points_left(), before - cost);
        e.input(EditInput::Shield(shield, false));
        assert_eq!(e.points_left(), before);
        // Ticking every class ability spends more than there is.
        for group in e.ability_groups() {
            if matches!(group.group, AbilityGroup::Class(_)) {
                for (i, check) in group.checks.iter().enumerate() {
                    if !check.on {
                        e.input(EditInput::Tick(group.group, i, true));
                    }
                }
            }
        }
        if e.points_left() < 0 {
            assert!(e.problem().is_some());
            assert!(!e.can_save());
        }
    }

    #[test]
    fn a_name_not_offered_is_not_added() {
        let Some(mut e) = editor() else {
            return;
        };
        e.input(EditInput::Add(ListKind::Weapons, "No Such Weapon".into()));
        assert!(e.prompt().is_none());
        assert!(!e.changed());
    }
}
