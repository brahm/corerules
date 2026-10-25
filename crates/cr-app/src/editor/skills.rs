//! The editor's Skills page: the Expansion's Skills page of Update
//! Character, with its Rogue Skills, Psionics, Wild Talents and Psionic
//! Attacks and Defenses dialogs as groups, as [`SkillsForm`] has them.
//! Test for Wild Talents reads the abilities as edited and may lower
//! Constitution, Intelligence or Wisdom on the Details page.

use std::sync::Arc;

use cr_formats::character::Character;

use super::{EditInput, Editor};
use crate::create::ListKind;

impl Editor {
    /// `character` with the ability scores as edited.
    pub(super) fn with_abilities(&self, mut character: Character) -> Character {
        let abilities = &self.details.abilities;
        character.data.strength = abilities[0];
        character.data.abilities.copy_from_slice(&abilities[1..]);
        character
    }

    /// The Skills page sees the abilities as edited.
    pub(super) fn abilities_changed(&mut self) {
        let character = self.with_abilities((*self.original).clone());
        self.skills.set_character(character);
    }

    /// Whether the page's choices changed.
    pub(super) fn skills_changed(&self) -> bool {
        self.skills.skills() != self.original_skills
    }

    pub(super) fn skills_input(&mut self, input: EditInput) {
        match input {
            EditInput::SkillPoints(row, points) => {
                self.skills.set_points(row, points);
            }
            EditInput::KitBonus(group, name) => self.skills.pick_bonus(group, name),
            EditInput::RogueChosen(name, on) => self.skills.tick_chosen(name, on),
            EditInput::Primary(slot, index) => self.skills.set_primary(slot, index),
            EditInput::Add(kind, name) => {
                if !self.skills.list(kind).choices.contains(&name) {
                    return;
                }
                // A talent not rolled asks first (0x6363c7).
                self.ask_each(
                    self.skills.add_question(kind).into_iter().collect(),
                    Arc::new(move |e: &mut Editor| e.skills.add(kind, &name)),
                );
            }
            EditInput::Remove(kind, row) => self.skills.remove(kind, row),
            EditInput::TestWild if self.skills.wild().is_some_and(|w| w.can_test) => {
                self.abilities_changed();
                self.ask_each(
                    self.skills.test_questions().into(),
                    Arc::new(|e: &mut Editor| {
                        let test = e.skills.test(&mut e.dice);
                        // The test's losses show on the Details page.
                        e.details.abilities[2..5].copy_from_slice(&test.abilities);
                        e.ask_each(test.notices.into(), Arc::new(|_: &mut Editor| {}));
                    }),
                );
            }
            EditInput::RollWild if self.skills.wild().is_some() => {
                self.ask_each(
                    self.skills.roll_questions().into(),
                    Arc::new(|e: &mut Editor| {
                        let notices = e.skills.roll(&mut e.dice);
                        e.ask_each(notices.into(), Arc::new(|_: &mut Editor| {}));
                    }),
                );
            }
            _ => {}
        }
    }

    /// Whether an input is the Skills page's: its lists are the
    /// disciplines, powers, wild talents and psionic modes.
    pub(super) fn is_skills_input(input: &EditInput) -> bool {
        match input {
            EditInput::Add(kind, _) | EditInput::Remove(kind, _) => matches!(
                kind,
                ListKind::Disciplines
                    | ListKind::Sciences
                    | ListKind::Devotions
                    | ListKind::WildTalents
                    | ListKind::Attacks
                    | ListKind::Defenses
            ),
            EditInput::SkillPoints(..)
            | EditInput::KitBonus(..)
            | EditInput::RogueChosen(..)
            | EditInput::Primary(..)
            | EditInput::TestWild
            | EditInput::RollWild => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::editor;
    use super::*;
    use crate::prompt::Answer;

    #[test]
    fn a_test_for_wild_talents_shows_its_losses_on_the_details() {
        let Some(mut e) = editor() else {
            return;
        };
        let Some(wild) = e.skills.wild() else {
            return;
        };
        if !wild.can_test {
            return;
        }
        e.input(EditInput::TestWild);
        let mut guard = 0;
        while e.prompt().is_some() && guard < 20 {
            e.answer(Answer::Yes);
            guard += 1;
        }
        assert!(e.skills.skills().tested);
        assert!(e.changed());
        let tested = e.skills.character().data.abilities;
        assert_eq!(e.details.abilities[2..5], tested[1..4]);
    }

    #[test]
    fn a_rogue_skill_takes_points_while_some_are_left() {
        let Some(mut e) = editor() else {
            return;
        };
        let Some(rogue) = e.skills.rogue() else {
            return;
        };
        if rogue.left <= 0 || rogue.rows.is_empty() {
            return;
        }
        let points = rogue.rows[0].points;
        e.input(EditInput::SkillPoints(0, points + 1));
        assert_eq!(e.skills.rogue().unwrap().left, rogue.left - 1);
        assert!(e.changed());
    }
}
