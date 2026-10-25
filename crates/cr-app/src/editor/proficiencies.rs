//! The editor's Proficiencies page: the Expansion's Nonweapon
//! Proficiencies page of Update Character, where taking a proficiency
//! again adds a rank and removing takes one off, both asking and warning
//! as the original does, and the native languages.

use std::sync::Arc;

use cr_formats::character::Character;
use cr_i18n::{tr, trn};

use super::{EditInput, Editor, Prompt, Tab, Then};
use crate::create::{List, ListKind, points, slots};

impl Editor {
    /// What an entry took, with its ranks.
    fn nonweapon_subtitle(&self, index: usize) -> String {
        let rules = &self.needs.proficiencies;
        let character = &*self.advanced;
        let Some(entry) = self.nonweapon.entries.get(index) else {
            return String::new();
        };
        let cost = if character.skills_and_powers() {
            points(entry.spent.into())
        } else {
            slots(entry.spent)
        };
        let ranks = rules.ranks(character, &self.nonweapon, index);
        if ranks > 1 {
            trn!("{cost}, {n} rank", "{cost}, {n} ranks", ranks, cost)
        } else {
            cost
        }
    }

    /// The nonweapon proficiencies, each to take again for a rank, with
    /// the slots or points left.
    pub fn nonweapon_list(&self) -> List {
        let rules = &self.needs.proficiencies;
        let character = &*self.advanced;
        let description = match rules.left(&self.nonweapon_character(), &self.nonweapon) {
            Some(left) if character.skills_and_powers() => {
                tr!("{points} left", points = points(left))
            }
            Some(left) if left < 0 => trn!(
                "The character has {n} slot too many non-weapon proficiencies.",
                "The character has {n} slots too many non-weapon proficiencies.",
                left.unsigned_abs()
            ),
            Some(left) => trn!("{n} slot left", "{n} slots left", left),
            None => String::new(),
        };
        List {
            title: tr("Nonweapon Proficiencies").to_owned(),
            description,
            rows: (0..self.nonweapon.entries.len())
                .map(|i| {
                    let name = self.nonweapon.entries[i].name.clone();
                    (name, self.nonweapon_subtitle(i), None)
                })
                .collect(),
            choices: rules.names(),
            shown: true,
            fixed: false,
        }
    }

    /// The native languages.
    pub fn languages_list(&self) -> List {
        let taken = &self.lists.languages;
        List {
            title: tr("Native Languages").to_owned(),
            description: String::new(),
            rows: taken
                .iter()
                .map(|n| (n.clone(), String::new(), None))
                .collect(),
            choices: self
                .needs
                .languages
                .iter()
                .filter(|n| !taken.contains(n))
                .cloned()
                .collect(),
            shown: true,
            fixed: false,
        }
    }

    /// The character as the proficiency rules see it for the points
    /// left: with what the abilities and combat choices spend of the
    /// character points taken off.
    fn nonweapon_character(&self) -> Character {
        self.sharing_points(&self.advanced, self.nonweapon_points())
    }

    /// Takes a proficiency once the kit's warning is ignored and the
    /// question answered Yes, or says why not.
    fn add_nonweapon(&mut self, name: String) {
        let rules = self.needs.proficiencies.clone();
        if let Some(warning) = rules.barred_warning(&self.advanced, &self.nonweapon, &name) {
            self.show(Prompt::ignorable(
                tr("Kit Non-Weapon Proficiency Requirements"),
                &warning,
                Arc::new(move |e: &mut Editor| {
                    e.nonweapon.barred_ignored = true;
                    e.add_nonweapon(name.clone());
                }),
            ));
            return;
        }
        match rules.purchase(&self.nonweapon_character(), &self.nonweapon, &name) {
            Ok(purchase) => self.ask_each(
                purchase.question.clone().into_iter().collect(),
                Arc::new(move |e: &mut Editor| {
                    let rules = e.needs.proficiencies.clone();
                    rules.take(&mut e.nonweapon, &name, &purchase);
                }),
            ),
            Err(text) => self.message(tr("Warning"), text),
        }
    }

    /// Takes a rank off entry `index`, or the entry with its last rank,
    /// once the question, if any, is answered Yes.
    fn remove_nonweapon(&mut self, index: usize) {
        let rules = self.needs.proficiencies.clone();
        let take_off: Then = Arc::new(move |e: &mut Editor| {
            let rules = e.needs.proficiencies.clone();
            rules.remove(&e.advanced, &mut e.nonweapon, index);
        });
        match rules.removal_question(&self.advanced, &self.nonweapon, index) {
            Some(q) => self.show(Prompt::ask(&q.title, q.text, take_off, None)),
            None => take_off(self),
        }
    }

    /// Asks about the proficiency the kit has the character learn by a
    /// level, as leaving the original's page does: Yes stays on the page,
    /// No runs `then`.
    pub(super) fn check_leaving_nonweapon(&mut self, then: Then) {
        let question = self
            .needs
            .proficiencies
            .leaving_question(&self.advanced, &self.nonweapon);
        let Some(question) = question else {
            then(self);
            return;
        };
        self.show(Prompt::ask(
            &question.title,
            question.text,
            Arc::new(|e: &mut Editor| e.tab = Tab::Proficiencies),
            Some(Arc::new(move |e: &mut Editor| {
                e.nonweapon.by_level_declined = true;
                then(e);
            })),
        ));
    }

    pub(super) fn proficiencies_input(&mut self, input: EditInput) {
        match input {
            EditInput::Add(ListKind::Nonweapon, name) => {
                if self.needs.proficiencies.names().contains(&name) {
                    self.add_nonweapon(name);
                }
            }
            EditInput::Remove(ListKind::Nonweapon, i) if i < self.nonweapon.entries.len() => {
                self.remove_nonweapon(i);
            }
            EditInput::Add(ListKind::Learned, name) => {
                if self.languages_list().choices.contains(&name) {
                    self.lists.languages.push(name);
                }
            }
            EditInput::Remove(ListKind::Learned, i) if i < self.lists.languages.len() => {
                self.lists.languages.remove(i);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::editor_where;
    use super::*;
    use crate::prompt::Answer;

    #[test]
    fn a_proficiency_taken_twice_has_two_ranks_and_loses_them_one_at_a_time() {
        for skills_and_powers in [false, true] {
            let Some(mut e) = editor_where(|c| c.skills_and_powers() == skills_and_powers) else {
                continue;
            };
            let taken: Vec<String> = e.nonweapon.entries.iter().map(|n| n.name.clone()).collect();
            let Some(name) = e
                .nonweapon_list()
                .choices
                .into_iter()
                .find(|n| !taken.contains(n))
            else {
                continue;
            };
            let count = e.nonweapon.entries.len();
            for _ in 0..2 {
                e.input(EditInput::Add(ListKind::Nonweapon, name.clone()));
                while e.prompt().is_some() {
                    e.answer(Answer::Yes);
                }
            }
            if e.nonweapon.entries.len() == count {
                // Not enough points for it.
                continue;
            }
            assert_eq!(e.nonweapon.entries.len(), count + 1, "one entry");
            assert!(e.changed());
            let rows = e.nonweapon_list().rows;
            assert!(!rows[count].1.is_empty());
            e.input(EditInput::Remove(ListKind::Nonweapon, count));
            e.input(EditInput::Remove(ListKind::Nonweapon, count));
            while e.prompt().is_some() {
                e.answer(Answer::Yes);
            }
            assert_eq!(e.nonweapon.entries.len(), count);
        }
    }

    #[test]
    fn languages_offered_are_those_not_spoken() {
        let Some(mut e) = editor_where(|_| true) else {
            return;
        };
        let Some(language) = e.languages_list().choices.first().cloned() else {
            return;
        };
        e.input(EditInput::Add(ListKind::Learned, language.clone()));
        assert!(e.lists.languages.contains(&language));
        assert!(!e.languages_list().choices.contains(&language));
        let row = e.lists.languages.len() - 1;
        e.input(EditInput::Remove(ListKind::Learned, row));
        assert!(!e.changed());
    }
}
