//! The Nonweapon Proficiencies page of the Expansion's Update Character
//! (IDD 307): an existing character's nonweapon proficiencies, what each
//! one took, and the kit's warnings and questions about them.
//!
//! The page charges as New Character does (0x43776b, see
//! [`crate::create`]), the character seen at its class levels, so the
//! kit's free proficiencies come with the levels:
//!
//! * a proficiency taken again is another rank of the same entry for 1
//!   more slot or point (5c30df); a Core Rules language one is another
//!   entry at its full cost;
//! * removing takes one rank off, or the entry with its last one, and
//!   refunds what it took; removing the last rank of a required one asks
//!   first (5c2856);
//! * a proficiency the kit bars gives the kit's warning, which may be
//!   ignored for good (+0x1dea);
//! * the Core Rules slots are counted as Update Character counts them
//!   (5c458c), and taking more than are left asks first; with Skills &
//!   Powers the character points are spent, and the app refuses where the
//!   original lets them go below 0 after asking, as on the other pages;
//! * leaving the page asks about the proficiency the kit has the
//!   character learn by a level, until answered No (5c8e84).

use std::sync::Arc;

use cr_formats::character::Character;
use cr_i18n::tr;

use crate::Installation;
use crate::advance::AdvanceRules;
use crate::create::{CreationRules, Draft, Question, required_removal};

/// The rules of the page, from New Character's.
#[derive(Debug, Clone, Default)]
pub struct ProficiencyRules {
    creation: Option<Arc<CreationRules>>,
    /// For the class levels and whether the character is dual-classed.
    advance: Option<Arc<AdvanceRules>>,
}

/// A nonweapon proficiency the character has, with the slots or
/// character points it took (`nonweapon_costs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonweaponEntry {
    pub name: String,
    pub spent: u32,
}

/// A character's choices on the page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterNonweapon {
    /// In the file's order, new ones last.
    pub entries: Vec<NonweaponEntry>,
    /// The kit's warning about the proficiencies it bars was ignored
    /// (`flags_0x1dea[0]`).
    pub barred_ignored: bool,
    /// The kit's question about the proficiency to learn by a level was
    /// answered No (the byte of `unknown_0x1df0`).
    pub by_level_declined: bool,
}

/// What taking a proficiency costs and asks first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonweaponPurchase {
    /// The entry it adds a rank to, or none for a new entry.
    pub rank_of: Option<usize>,
    /// The slots or character points it takes.
    pub cost: u32,
    /// Asked first when the Core Rules slots run short: No does not take
    /// it.
    pub question: Option<Question>,
}

impl Installation {
    /// [`ProficiencyRules`] with New Character's and the advancement rules
    /// already loaded.
    pub fn proficiency_rules_with(
        &self,
        creation: Arc<CreationRules>,
        advance: Arc<AdvanceRules>,
    ) -> ProficiencyRules {
        ProficiencyRules {
            creation: Some(creation),
            advance: Some(advance),
        }
    }
}

impl ProficiencyRules {
    pub fn nonweapon_of(&self, character: &Character) -> CharacterNonweapon {
        let d = &character.data;
        CharacterNonweapon {
            entries: d
                .nonweapon_proficiencies
                .iter()
                .enumerate()
                .map(|(i, name)| NonweaponEntry {
                    name: name.clone(),
                    spent: d.nonweapon_costs.get(i).copied().unwrap_or(0),
                })
                .collect(),
            barred_ignored: d.flags_0x1dea.is_some_and(|f| f[0] != 0),
            by_level_declined: d.unknown_0x1df0.is_some_and(|f| f.1 != 0),
        }
    }

    /// Every nonweapon proficiency, to take or take again.
    pub fn names(&self) -> Vec<String> {
        self.creation
            .as_ref()
            .map(|c| c.nonweapon_proficiencies())
            .unwrap_or_default()
    }

    /// The character's class levels as stored.
    fn levels(&self, character: &Character) -> Vec<u32> {
        self.advance
            .as_ref()
            .and_then(|a| a.levels(character).ok())
            .map_or_else(|| vec![1], |l| l.iter().map(|c| c.level).collect())
    }

    /// The character with `new` chosen, as New Character's rules see it,
    /// at its class levels.
    fn draft(
        &self,
        character: &Character,
        new: &CharacterNonweapon,
    ) -> Option<(&CreationRules, Draft)> {
        let rules = self.creation.as_deref()?;
        let mut draft = rules.update_draft(character, &self.levels(character));
        draft.nonweapon_proficiencies = new.entries.iter().map(|e| e.name.clone()).collect();
        draft.barred_nonweapon_ignored = new.barred_ignored;
        draft.by_level_declined = new.by_level_declined;
        Some((rules, draft))
    }

    /// What the proficiency costs the character, one rank (0x43776b).
    pub fn cost(&self, character: &Character, new: &CharacterNonweapon, name: &str) -> u32 {
        self.draft(character, new)
            .map_or(1, |(rules, draft)| rules.nonweapon_cost(&draft, name))
    }

    /// The Core Rules slots or the Skills & Powers character points left;
    /// none for a Core Rules 2.0 character, whose slots are not counted.
    pub fn left(&self, character: &Character, new: &CharacterNonweapon) -> Option<i64> {
        let spent: i64 = new.entries.iter().map(|e| i64::from(e.spent)).sum();
        if character.skills_and_powers() {
            let old: i64 = character
                .data
                .nonweapon_costs
                .iter()
                .map(|&c| i64::from(c))
                .sum();
            return Some(i64::from(character.data.unknown_0x1778[0]) + old - spent);
        }
        let (rules, draft) = self.draft(character, new)?;
        let dual = self
            .advance
            .as_ref()
            .is_some_and(|a| a.dual_classed(character));
        rules
            .expansion
            .then(|| rules.core_nonweapon_slots(&draft, dual) - spent)
    }

    /// The kit's warning about the proficiency, unless ignored.
    pub fn barred_warning(
        &self,
        character: &Character,
        new: &CharacterNonweapon,
        name: &str,
    ) -> Option<String> {
        let (rules, draft) = self.draft(character, new)?;
        rules.barred_nonweapon_warning(&draft, name)
    }

    /// What taking the proficiency costs and asks, or why the character
    /// may not.
    pub fn purchase(
        &self,
        character: &Character,
        new: &CharacterNonweapon,
        name: &str,
    ) -> Result<NonweaponPurchase, String> {
        let separate = !character.skills_and_powers()
            && self
                .creation
                .as_ref()
                .is_some_and(|c| c.language_nonweapon(name));
        let rank_of = (!separate)
            .then(|| new.entries.iter().position(|e| e.name == name))
            .flatten();
        let cost = if rank_of.is_some() {
            1
        } else {
            self.cost(character, new, name)
        };
        let mut question = None;
        if let Some(left) = self.left(character, new)
            && i64::from(cost) > left
        {
            if character.skills_and_powers() {
                return Err(tr!(
                    "The selected ability requires {cost} character points, only {left} are \
                     available.",
                    cost,
                    left = left.max(0)
                ));
            }
            question = Some(Question::new(
                tr("Warning"),
                tr!(
                    "The selected ability requires {cost} slots, only {left} are available. \
                     Take ability anyway?",
                    cost,
                    left
                ),
            ));
        }
        Ok(NonweaponPurchase {
            rank_of,
            cost,
            question,
        })
    }

    /// Takes a proficiency as `purchase` says.
    pub fn take(&self, new: &mut CharacterNonweapon, name: &str, purchase: &NonweaponPurchase) {
        match purchase.rank_of.and_then(|i| new.entries.get_mut(i)) {
            Some(entry) => entry.spent += purchase.cost,
            None => new.entries.push(NonweaponEntry {
                name: name.to_owned(),
                spent: purchase.cost,
            }),
        }
    }

    /// The ranks of entry `index`: one, and one more for each slot or
    /// point beyond what one rank costs.
    pub fn ranks(&self, character: &Character, new: &CharacterNonweapon, index: usize) -> u32 {
        new.entries.get(index).map_or(0, |e| {
            1 + e.spent.saturating_sub(self.cost(character, new, &e.name))
        })
    }

    /// What removing a rank of entry `index` asks first: removing the last
    /// rank of a proficiency the classes, the kit or the file require.
    pub fn removal_question(
        &self,
        character: &Character,
        new: &CharacterNonweapon,
        index: usize,
    ) -> Option<Question> {
        let entry = new.entries.get(index)?;
        if self.ranks(character, new, index) > 1 {
            return None;
        }
        let (rules, draft) = self.draft(character, new)?;
        // The file's own required ones (+0x1930) count too.
        let held = character.data.unknown_0x1930.as_ref();
        let required = rules.required_nonweapon(&draft).contains(&entry.name)
            || held.is_some_and(|l| l[0].contains(&entry.name));
        required.then(required_removal)
    }

    /// Removes a rank of entry `index`, or the entry with its last one.
    pub fn remove(&self, character: &Character, new: &mut CharacterNonweapon, index: usize) {
        if self.ranks(character, new, index) > 1 {
            new.entries[index].spent -= 1;
        } else if index < new.entries.len() {
            new.entries.remove(index);
        }
    }

    /// What leaving the page asks, until answered No: the proficiency the
    /// kit has the character learn by a level.
    pub fn leaving_question(
        &self,
        character: &Character,
        new: &CharacterNonweapon,
    ) -> Option<Question> {
        let (rules, draft) = self.draft(character, new)?;
        rules.nonweapon_by_level(&draft)
    }
}
