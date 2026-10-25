//! The editor's Details page: Update Character's Name page (name, player,
//! portrait, gender, alignment, kit, background and history), the
//! character's classes with Add Class for dual-classing, the experience
//! with the Levels dialog and Add Experience, the hit points and PSPs, the
//! appearance and the ability scores.

use cr_formats::character::Character;
use cr_i18n::tr;
use cr_store::advance::{adjust_psionic_points, psionic_total};
use cr_store::dual::{ClassChoice, ClassTable};
use cr_store::edit::ABILITY_RANGE;

use super::{EditInput, Editor};
use crate::create::SOCIAL_CLASSES;
use crate::levels::{
    ClassRow, ExperienceForm, ExperienceSettings, LevelsForm, class_rows, psionic,
};

/// The most years, inches and pounds the original takes.
pub const MOST_AGE: u32 = 9999;
pub const MOST_HEIGHT: u32 = 999;
pub const MOST_WEIGHT: u32 = 99_999;
/// The most hit points and PSPs the editor's boxes take.
pub const MOST_POINTS: u32 = 99_999;

/// The classes a character may add, or why none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassOffer {
    /// What dual-classing does, or why the character cannot.
    pub description: String,
    pub names: Vec<String>,
}

/// The classes `character` may add, by the dual-class rules.
pub fn class_offer(table: &ClassTable, character: &Character) -> ClassOffer {
    match table.choices(character) {
        ClassChoice::Refused(why) => ClassOffer {
            description: why,
            names: Vec::new(),
        },
        ClassChoice::Allowed(names) if names.is_empty() => ClassOffer {
            description: tr("No class meets the dual-class requirements of these abilities")
                .to_owned(),
            names,
        },
        ClassChoice::Allowed(names) => ClassOffer {
            description: tr(
                "Dual-classing: the new class starts at level 1 and the earlier class stops \
                 advancing",
            )
            .to_owned(),
            names,
        },
    }
}

/// The social class as the Social Class box shows it: one of the
/// original's classes in the language chosen, any other text as written.
pub fn social_class_shown(stored: &str) -> String {
    SOCIAL_CLASSES
        .iter()
        .find(|class| **class == stored)
        .map_or(stored, |class| tr(class))
        .to_owned()
}

/// The social class as the character keeps it: one of the original's
/// classes, shown in the language chosen, back in English, so that the
/// sheet and the kits' tables still know it; any other text as written.
pub fn social_class_stored(shown: &str) -> String {
    SOCIAL_CLASSES
        .iter()
        .find(|class| tr(class) == shown)
        .map_or(shown, |class| class)
        .to_owned()
}

impl Editor {
    /// The character with the ability scores as edited, for the classes
    /// it may add.
    fn edited(&self) -> Character {
        let mut character = (*self.advanced).clone();
        character.data.strength[0] = self.details.abilities[0][0];
        for (slot, scores) in self.details.abilities.iter().enumerate().skip(1) {
            character.data.abilities[slot - 1][0] = scores[0];
        }
        character
    }

    /// The classes Add Class offers, by the abilities as edited; none
    /// once one is added, since the next depends on it.
    pub fn class_offer(&self) -> ClassOffer {
        class_offer(&self.needs.class_table, &self.edited())
    }

    /// Each class with its level and experience.
    pub fn class_rows(&self) -> Vec<ClassRow> {
        class_rows(&self.needs.advance, &self.advanced)
    }

    /// The hit points the rolls give, when they can be read.
    pub fn hit_points(&self) -> Option<u32> {
        self.needs
            .advance
            .hit_points(&self.advanced)
            .ok()
            .map(|hp| hp.max(0) as u32)
    }

    /// The PSPs, for a psionicist or anyone with some.
    pub fn psp(&self) -> Option<u32> {
        psionic(&self.needs.advance, &self.advanced).then(|| psionic_total(&self.advanced))
    }

    /// Skills & Powers' unspent character points.
    pub fn character_points(&self) -> Option<u32> {
        self.advanced
            .skills_and_powers()
            .then(|| self.advanced.data.unknown_0x1778[0])
    }

    /// Whether the character may have exceptional strength: a warrior
    /// with Strength 18.
    pub fn exceptional_allowed(&self) -> bool {
        self.needs.creation.has_exceptional_strength(
            &self.advanced.data.classes.names,
            self.details.abilities[0][0],
        )
    }

    pub fn alignment_index(&self) -> Option<usize> {
        self.alignments
            .iter()
            .position(|a| *a == self.details.alignment)
    }

    pub fn kit_index(&self) -> Option<usize> {
        self.kits.iter().position(|k| *k == self.details.kit)
    }

    pub fn background_index(&self) -> Option<usize> {
        self.backgrounds
            .iter()
            .position(|b| *b == self.details.background)
    }

    /// An added class the abilities as edited no longer allow is taken
    /// back.
    fn recheck_class(&mut self) {
        if let Some(class) = &self.new_class
            && !self.class_offer().names.contains(class)
        {
            self.new_class = None;
        }
        if !self.exceptional_allowed() {
            self.details.exceptional_strength = 0;
        }
    }

    pub(super) fn details_input(&mut self, input: EditInput) {
        let advance = self.needs.advance.clone();
        let d = &mut self.details;
        match input {
            EditInput::Name(text) => d.name = text,
            EditInput::Player(text) => d.player = text,
            EditInput::SocialClass(text) => d.social_class = social_class_stored(&text),
            EditInput::Portrait(portrait) => d.portrait = portrait,
            EditInput::Gender(i) => d.male = i == 0,
            EditInput::Alignment(i) => {
                if let Some(a) = self.alignments.get(i) {
                    d.alignment.clone_from(a);
                }
            }
            EditInput::Kit(i) => {
                let Some(kit) = self.kits.get(i).cloned() else {
                    return;
                };
                if kit == d.kit {
                    return;
                }
                // A new kit rolls the social class again, on its table,
                // and shows its information, as the original does.
                d.social_class = self.needs.creation.roll_social_class(&kit, &mut self.dice);
                d.kit.clone_from(&kit);
                self.advanced.data.kit.clone_from(&kit);
                self.kit_weapons_changed();
                self.settle_kit_magic();
                if let Some(text) = self.needs.creation.kit_information(&kit) {
                    let text = text.to_owned();
                    self.message(tr("Kit Information"), text);
                }
            }
            EditInput::Background(i) => {
                if let Some(event) = self.backgrounds.get(i) {
                    d.background.clone_from(event);
                }
            }
            EditInput::RandomBackground => {
                if let Some(event) =
                    cr_store::edit::random_background(&mut self.rand, &self.backgrounds)
                {
                    d.background = event.to_owned();
                }
            }
            EditInput::History(text) => d.history = text,
            EditInput::Age(v) => d.age = v.min(super::MOST_AGE),
            EditInput::Height(v) => d.height = v.min(super::MOST_HEIGHT),
            EditInput::Weight(v) => d.weight = v.min(super::MOST_WEIGHT),
            EditInput::Ability(a, part, v) if a < 6 && part < 3 => {
                let v = v.clamp(*ABILITY_RANGE.start(), *ABILITY_RANGE.end());
                // With the Core Rules the score sets both subabilities, as
                // the original keeps them.
                if d.skills_and_powers {
                    d.abilities[a][part] = v;
                } else {
                    d.abilities[a] = [v; 3];
                }
                self.recheck_class();
                self.abilities_changed();
            }
            EditInput::Exceptional(v) => {
                let allowed = self.exceptional_allowed();
                self.details.exceptional_strength = if allowed { v.min(100) } else { 0 };
            }
            EditInput::AddClass(i) => {
                if self.new_class.is_none()
                    && let Some(name) = self.class_offer().names.get(i)
                {
                    self.new_class = Some(name.clone());
                }
            }
            EditInput::RemoveClass => self.new_class = None,
            // The Levels dialog opens with the prime requisite bonus on
            // (0x617a71) and 3 character points a level, every time.
            EditInput::OpenLevels => {
                self.experience = None;
                self.levels = Some(Box::new(LevelsForm::new(
                    &advance,
                    (*self.advanced).clone(),
                    true,
                    3,
                )));
            }
            EditInput::Levels(change) => {
                if let Some(levels) = &mut self.levels {
                    levels.input(&advance, change, &mut self.dice);
                }
            }
            EditInput::CloseLevels => {
                if let Some(levels) = self.levels.take() {
                    *self.advanced = levels.close(&advance);
                    self.kit_weapons_changed();
                }
            }
            EditInput::OpenExperience => {
                self.levels = None;
                let settings = ExperienceSettings {
                    prime_bonus: true,
                    points_per_level: 3,
                    skills_and_powers: self.advanced.skills_and_powers(),
                };
                self.experience = Some(ExperienceForm::new(&self.advanced, Some(settings)));
                self.experience_serial += 1;
            }
            EditInput::Experience(crate::levels::LevelsInput::Cancel) => self.experience = None,
            EditInput::Experience(crate::levels::LevelsInput::Give) => {
                if let Some(experience) = &mut self.experience {
                    if experience.give(&advance, &mut self.advanced, (true, 3), &mut self.dice) {
                        self.experience = None;
                        self.kit_weapons_changed();
                    } else {
                        self.experience_serial += 1;
                    }
                }
            }
            EditInput::Experience(change) => {
                if let Some(experience) = &mut self.experience {
                    experience.input(change);
                }
            }
            // The Name page's Hit Points arrows: a change moves the
            // stored rolls (see AdvanceRules::adjust_hit_points).
            EditInput::HitPoints(wanted) => {
                let wanted = wanted.min(MOST_POINTS) as i32;
                let character = &mut self.advanced;
                let _ = advance
                    .hit_points(character)
                    .and_then(|hp| advance.adjust_hit_points(character, wanted - hp));
            }
            // The PSP arrows (2373): a change goes to the highest level
            // that has PSPs.
            EditInput::Psp(wanted) => {
                let delta = i64::from(wanted.min(999)) - i64::from(psionic_total(&self.advanced));
                adjust_psionic_points(&mut self.advanced, delta as i32);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::editor;
    use super::*;
    use crate::levels::LevelsInput;

    #[test]
    fn the_social_class_is_kept_in_english_and_shown_translated() {
        for class in SOCIAL_CLASSES {
            assert_eq!(social_class_shown(class), tr(class));
            assert_eq!(social_class_stored(&social_class_shown(class)), class);
        }
        assert_eq!(social_class_shown("Gentry"), "Gentry");
        assert_eq!(social_class_stored("Gentry"), "Gentry");
    }

    #[test]
    fn a_new_kit_rolls_the_social_class() {
        let Some(mut e) = editor() else {
            return;
        };
        let Some(other) = e.kits.iter().position(|k| *k != e.details.kit) else {
            return;
        };
        e.input(EditInput::Kit(other));
        assert_eq!(e.details.kit, e.kits[other]);
        assert!(!e.details.social_class.is_empty());
        if let Some(prompt) = e.prompt() {
            assert_eq!(prompt.title, "Kit Information");
            e.answer(crate::prompt::Answer::Yes);
        }
        assert!(e.prompt().is_none());
    }

    #[test]
    fn experience_and_hit_points_change_the_levels() {
        let Some(mut e) = editor() else {
            return;
        };
        let before = e.class_rows();
        e.input(EditInput::OpenExperience);
        e.input(EditInput::Experience(LevelsInput::Amount(1_000_000)));
        e.input(EditInput::Experience(LevelsInput::Give));
        assert!(e.experience.is_none(), "{:?}", e.experience);
        let after = e.class_rows();
        assert!(after.iter().zip(&before).any(|(a, b)| a.level > b.level));
        assert!(e.changed());
        if let Some(hp) = e.hit_points() {
            e.input(EditInput::HitPoints(hp + 1));
            assert_eq!(e.hit_points(), Some(hp + 1));
        }
        e.input(EditInput::Save);
        let effects = e.take_effects();
        let [super::super::EditEffect::Save(saving)] = effects.as_slice() else {
            panic!("{effects:?}");
        };
        assert!(saving.lists.advancement.is_some());
    }

    #[test]
    fn the_levels_dialog_sets_a_level() {
        let Some(mut e) = editor() else {
            return;
        };
        e.input(EditInput::OpenLevels);
        e.input(EditInput::Levels(LevelsInput::Level(0, 5)));
        e.input(EditInput::CloseLevels);
        assert!(e.levels.is_none());
        assert_eq!(e.class_rows()[0].level, 5);
    }

    #[test]
    fn exceptional_strength_needs_a_warrior_with_18() {
        let Some(mut e) = editor() else {
            return;
        };
        e.input(EditInput::Ability(0, 0, 12));
        e.input(EditInput::Exceptional(50));
        assert_eq!(e.details.exceptional_strength, 0);
    }
}
