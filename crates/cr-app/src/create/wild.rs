//! The steps before the equipment: the Wild Talents dialog (IDD 358) for
//! an Expansion character who is no psionicist, and the Psionics dialog
//! (IDD 357), Update Character's Skills page, for a psionicist.

use std::collections::VecDeque;
use std::sync::Arc;

use cr_formats::character::Character;
use cr_i18n::tr;
use cr_store::create::Dice;
use cr_store::skills::SkillRules;
use cr_store::skills::wild::{WildDraft, WildMode};

use super::{Input, List, ListKind, Page, Wizard};
use crate::skills::{SkillsForm, power_text};

#[derive(Debug, Clone, PartialEq)]
pub struct WildPage {
    /// New Character's instructions, for a Skills & Powers character not
    /// yet a wild talent (0x635177).
    pub description: String,
    /// The chance a test makes the character a wild talent.
    pub chance: String,
    /// Test is offered: the character was no wild talent as the page
    /// opened.
    pub test_shown: bool,
    pub can_test: bool,
    pub can_roll: bool,
    pub talents: List,
    pub attacks: List,
    pub defenses: List,
    /// The character as built from the draft, before a test's losses.
    base: Box<Character>,
    wild: WildDraft,
}

/// A psionicist's Skills page, as a step.
#[derive(Clone)]
pub struct PsionicsPage {
    pub form: SkillsForm,
}

impl Wizard {
    /// Wild Talents for an Expansion character who is no psionicist,
    /// Psionics for a psionicist, then the equipment.
    pub(super) fn before_equipment(&mut self) {
        let psionicist = self.rules.is_psionicist(&self.draft.classes);
        match self.needs.skills.clone() {
            Some(skills) if self.rules.expansion && !psionicist => {
                self.draft.psionics = None;
                self.wild_page(&skills);
            }
            Some(skills) if psionicist => {
                self.draft.wild = WildDraft::default();
                self.psionics_page(skills);
            }
            _ => {
                self.draft.wild = WildDraft::default();
                self.draft.psionics = None;
                self.equipment_page();
            }
        }
    }

    fn psionics_page(&mut self, skills: Arc<SkillRules>) {
        let Ok(mut character) = self.rules.build(&self.draft) else {
            self.equipment_page();
            return;
        };
        if let Some(psionics) = &self.draft.psionics {
            // Back from a later step: what was chosen before.
            let _ = skills.apply(&mut character, psionics, &mut Dice::default());
        }
        self.push(Page::Psionics(Box::new(PsionicsPage {
            form: SkillsForm::new(skills, character),
        })));
    }

    pub(super) fn psionics_input(&mut self, input: Input) {
        if input == Input::Next {
            let Page::Psionics(page) = self.page() else {
                return;
            };
            let (new, problem) = (page.form.skills(), page.form.problem());
            match problem {
                Some(problem) if !self.entering() => self.message(tr("Psionics"), problem),
                _ => {
                    self.draft.psionics = Some(new);
                    self.equipment_page();
                }
            }
            return;
        }
        let Page::Psionics(page) = self.page_mut() else {
            return;
        };
        let form = &mut page.form;
        match input {
            Input::SkillPoints(row, points) => {
                form.set_points(row, points);
            }
            Input::KitBonus(group, name) => form.pick_bonus(group, name),
            Input::RogueChosen(name, on) => form.tick_chosen(name, on),
            Input::Primary(slot, index) => form.set_primary(slot, index),
            Input::Add(kind, name) => form.add(kind, &name),
            Input::Remove(kind, row) => form.remove(kind, row),
            _ => {}
        }
    }

    /// The Wild Talents dialog as a step: New Character tests and, for a
    /// wild talent, rolls once; Enter Character also takes any talent
    /// from the table.
    fn wild_page(&mut self, skills: &SkillRules) {
        let Ok(base) = self.rules.build(&self.draft) else {
            self.equipment_page();
            return;
        };
        let mut wild = self.draft.wild.clone();
        let mut current = base.clone();
        if let Some(abilities) = wild.abilities {
            current.data.abilities[1..4].copy_from_slice(&abilities);
        }
        let wild_at_start = skills.is_wild_talent(&current, wild.wild_talent);
        let creating = !self.entering();
        // New Character starts a character not yet a wild talent over
        // (0x635177).
        if creating && !wild_at_start {
            wild.talents.clear();
            wild.attacks.clear();
            wild.defenses.clear();
        }
        let description = if creating && self.draft.skills_and_powers && !wild_at_start {
            skills.wild_instructions().to_owned()
        } else {
            String::new()
        };
        self.push(Page::Wild(Box::new(WildPage {
            description,
            chance: String::new(),
            test_shown: !wild_at_start,
            can_test: false,
            can_roll: false,
            talents: List::default(),
            attacks: List::default(),
            defenses: List::default(),
            base: Box::new(base),
            wild,
        })));
        self.refresh_wild();
    }

    fn wild_mode(&self) -> WildMode {
        if self.entering() {
            WildMode::Enter
        } else {
            WildMode::Create
        }
    }

    /// The character as tested so far.
    fn wild_character(page: &WildPage) -> Character {
        let mut character = (*page.base).clone();
        if let Some(abilities) = page.wild.abilities {
            character.data.abilities[1..4].copy_from_slice(&abilities);
        }
        character
    }

    fn refresh_wild(&mut self) {
        let Some(skills) = self.needs.skills.clone() else {
            return;
        };
        let entering = self.entering();
        let Page::Wild(page) = self.page_mut() else {
            return;
        };
        let character = Self::wild_character(page);
        let wild = skills.is_wild_talent(&character, page.wild.wild_talent);
        page.chance = tr!(
            "A {chance}% chance to become a wild talent",
            chance = skills.wild_chance(&character)
        );
        page.can_test = !wild;
        page.can_roll = entering || wild;
        let taken = |names: &[String], offer: Vec<String>| -> Vec<String> {
            offer.into_iter().filter(|n| !names.contains(n)).collect()
        };
        // New Character's talents are only rolled; Enter Character's may
        // also be added and removed.
        page.talents = List {
            title: tr("Your Wild Talents").to_owned(),
            description: String::new(),
            rows: page
                .wild
                .talents
                .iter()
                .map(|n| (n.clone(), power_text(&skills, n), None))
                .collect(),
            choices: if entering {
                taken(&page.wild.talents, skills.wild_talent_choices())
            } else {
                Vec::new()
            },
            shown: true,
            fixed: !entering,
        };
        // Psionic Attacks and Defenses opens for a wild talent (0x6366b7).
        let modes = |title: &str, names: &[String], offer: &[String]| List {
            title: title.to_owned(),
            description: String::new(),
            rows: names
                .iter()
                .map(|n| (n.clone(), String::new(), None))
                .collect(),
            choices: taken(names, offer.to_vec()),
            shown: entering || wild,
            fixed: false,
        };
        page.attacks = modes(
            tr("Psionic Attack Modes"),
            &page.wild.attacks,
            skills.attack_names(),
        );
        page.defenses = modes(
            tr("Psionic Defense Modes"),
            &page.wild.defenses,
            skills.defense_names(),
        );
    }

    /// The page's wild talents, attacks or defenses.
    fn wild_names(&mut self, kind: ListKind) -> Option<&mut Vec<String>> {
        let Page::Wild(page) = self.page_mut() else {
            return None;
        };
        match kind {
            ListKind::WildTalents => Some(&mut page.wild.talents),
            ListKind::Attacks => Some(&mut page.wild.attacks),
            ListKind::Defenses => Some(&mut page.wild.defenses),
            _ => None,
        }
    }

    pub(super) fn wild_input(&mut self, input: Input) {
        let (Some(skills), Page::Wild(page)) = (self.needs.skills.clone(), self.page()) else {
            return;
        };
        let page = page.as_ref().clone();
        match input {
            Input::Add(kind, name) => {
                let offered = match kind {
                    ListKind::WildTalents => &page.talents.choices,
                    ListKind::Attacks => &page.attacks.choices,
                    ListKind::Defenses => &page.defenses.choices,
                    _ => return,
                };
                if !offered.contains(&name) {
                    return;
                }
                let add = Arc::new(move |w: &mut Wizard| {
                    if let Some(names) = w.wild_names(kind) {
                        names.push(name.clone());
                    }
                    w.refresh_wild();
                });
                // A talent not rolled asks first (0x6363c7).
                let questions = if kind == ListKind::WildTalents {
                    VecDeque::from([skills.add_question()])
                } else {
                    VecDeque::new()
                };
                self.ask_each(questions, add);
            }
            Input::Remove(kind, row) => {
                if kind == ListKind::WildTalents && page.talents.fixed {
                    return;
                }
                if let Some(names) = self.wild_names(kind)
                    && row < names.len()
                {
                    names.remove(row);
                }
                self.refresh_wild();
            }
            Input::Test if page.test_shown && page.can_test => {
                let character = Self::wild_character(&page);
                let questions = skills.test_questions(&character, page.wild.tested);
                self.ask_each(
                    questions.into(),
                    Arc::new(move |w: &mut Wizard| {
                        let Some(skills) = w.needs.skills.clone() else {
                            return;
                        };
                        let test = skills.test_wild_talent(&character, &mut w.dice);
                        if let Page::Wild(page) = w.page_mut() {
                            page.wild.tested = true;
                            page.wild.wild_talent |= test.wild_talent;
                            page.wild.abilities = Some(test.abilities);
                        }
                        w.refresh_wild();
                        w.ask_each(test.notices.into(), Arc::new(|_: &mut Wizard| {}));
                    }),
                );
            }
            Input::Roll if page.can_roll => {
                let questions = match skills.roll_questions(&page.wild.talents, self.wild_mode()) {
                    Ok(questions) => questions,
                    Err(refusal) => {
                        self.message(&refusal.title, refusal.text);
                        return;
                    }
                };
                self.ask_each(
                    questions.into(),
                    Arc::new(|w: &mut Wizard| {
                        let Some(skills) = w.needs.skills.clone() else {
                            return;
                        };
                        let (names, notices) = skills.roll_wild_talents(&mut w.dice);
                        if let Page::Wild(page) = w.page_mut() {
                            page.wild.talents = names;
                        }
                        w.refresh_wild();
                        w.ask_each(notices.into(), Arc::new(|_: &mut Wizard| {}));
                    }),
                );
            }
            Input::Next => {
                self.draft.wild = page.wild;
                self.equipment_page();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Answer;
    use super::super::tests::wizard;
    use super::*;

    /// The wizard past the spells, for a human of `class`.
    fn before_equipment(class: &str) -> Option<Wizard> {
        let mut w = wizard(false)?;
        w.draft.race = "Human".into();
        w.draft.classes = vec![class.to_owned()];
        w.draft.class_abilities = vec![Vec::new()];
        w.draft.scores = [16; 6];
        w.before_equipment();
        Some(w)
    }

    #[test]
    fn a_psionicist_chooses_disciplines_before_the_equipment() {
        let Some(mut w) = before_equipment("Psionicist") else {
            return;
        };
        let Page::Psionics(page) = w.page() else {
            panic!("not psionics");
        };
        let discipline = page.form.list(ListKind::Disciplines).choices[0].clone();
        w.input(Input::Add(ListKind::Disciplines, discipline.clone()));
        w.input(Input::Primary(0, 1));
        w.input(Input::Next);
        // Too few powers for the level are refused.
        if let Some(prompt) = w.prompt() {
            assert_eq!(prompt.title, tr("Psionics"));
            return;
        }
        assert!(matches!(w.page(), Page::Equipment(_)));
        let psionics = w.draft().psionics.clone().unwrap();
        assert_eq!(psionics.disciplines, std::slice::from_ref(&discipline));
        assert_eq!(psionics.primary[0], discipline);
        assert_eq!(w.draft().wild, WildDraft::default());
    }

    #[test]
    fn enter_character_takes_any_wild_talent() {
        let Some(mut w) = wizard(true) else {
            return;
        };
        w.draft.race = "Human".into();
        w.draft.classes = vec!["Fighter".into()];
        w.draft.class_abilities = vec![Vec::new()];
        w.before_equipment();
        let Page::Wild(page) = w.page() else {
            panic!("not wild talents");
        };
        assert!(page.can_roll && !page.talents.fixed && page.attacks.shown);
        let talent = page.talents.choices[0].clone();
        w.input(Input::Add(ListKind::WildTalents, talent.clone()));
        // "You should roll for Wild Talents."
        assert!(w.prompt().is_some());
        w.answer(Answer::Yes);
        let Page::Wild(page) = w.page() else {
            panic!();
        };
        assert_eq!(page.talents.rows[0].0, talent);
        assert!(!page.talents.rows[0].1.is_empty());
        w.input(Input::Next);
        assert_eq!(w.draft().wild.talents, [talent]);
    }
}
