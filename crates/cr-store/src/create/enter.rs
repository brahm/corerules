//! Enter Character (0x44aeab): the creation wizard in its second mode
//! (+0x60 = 0x379), for a character that already exists on paper. The
//! scores are typed in from 1 to 25 and kept as typed, with no racial
//! adjustment; any race and any class combination may be chosen; and the
//! Name page's Levels dialog (IDD 368, 0x616c00) gives each class its
//! experience, rolling hit points and PSPs for each level gained.

use cr_formats::character::Character;

use super::{CreationRules, Draft};
use crate::Result;
use crate::settings::Setting;
use cr_i18n::tr;

/// The lowest and highest score the Enter scores page allows (0x5b1cd5,
/// 0x5b215d), and the score it starts from (0x5b194d).
pub const ENTERED_SCORES: (u32, u32) = (1, 25);
pub const ENTERED_DEFAULT_SCORE: u32 = 12;

/// The Name page's alignments when they are set freely (0x5beb8f), in the
/// original's order (0x4c209c).
pub const ALIGNMENTS: [&str; 9] = [
    "Lawful Good",
    "Lawful Neutral",
    "Lawful Evil",
    "Neutral Good",
    "True Neutral",
    "Neutral Evil",
    "Chaotic Good",
    "Chaotic Neutral",
    "Chaotic Evil",
];

/// What the Levels dialog gives an entered character, kept from the Name
/// page to the save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entered {
    /// Each class's experience (0x448090).
    pub experience: [u32; 5],
    /// The hit point roll of each class and level (+0xc60).
    pub rolls: [[u32; 100]; 5],
    /// The PSPs of each level (+0x1430).
    pub psionic: [u32; 100],
    /// Skills & Powers: unspent character points (+0x1778), zeroed when
    /// the classes are chosen (0x5b4678).
    pub character_points: u32,
    /// "Level Limits:" None, Apply limits or Prime Requisite Bonuses
    /// (+0x1dc4: 0, 1 or 2).
    pub level_limits: u32,
    /// Each class's abilities, with those its levels grant (0x61990a).
    pub class_abilities: Vec<Vec<String>>,
    /// "Apply Prime Requisite experience bonuses?" (+0x2074).
    pub prime_bonus: bool,
    /// Skills & Powers: the character points each level gained adds
    /// (spin 1997, 0 to 10).
    pub points_per_level: u32,
}

impl Default for Entered {
    fn default() -> Self {
        Self {
            experience: [0; 5],
            rolls: [[0; 100]; 5],
            psionic: [0; 100],
            character_points: 0,
            level_limits: 0,
            class_abilities: Vec::new(),
            prime_bonus: true,
            points_per_level: 3,
        }
    }
}

impl Entered {
    /// Takes what the Levels dialog changed from `character`.
    pub fn take(&mut self, character: &Character) {
        let d = &character.data;
        self.experience = d.unknown_0x1618;
        self.rolls = d.unknown_0xc60;
        self.psionic = d.unknown_0x1430;
        self.character_points = d.unknown_0x1778[0];
        self.level_limits = d.unknown_0x1dc4;
        self.class_abilities = d
            .class_slots
            .iter()
            .take(d.classes.names.len())
            .map(|s| s.abilities.clone())
            .collect();
    }

    /// Writes it to `character`; class abilities it has are added to
    /// those the character has.
    pub fn apply(&self, character: &mut Character) {
        let d = &mut character.data;
        d.unknown_0x1618 = self.experience;
        d.unknown_0xc60 = self.rolls;
        d.unknown_0x1430 = self.psionic;
        if character.skills_and_powers() {
            character.data.unknown_0x1778[0] = self.character_points;
        }
        let d = &mut character.data;
        d.unknown_0x1dc4 = self.level_limits;
        for (slot, abilities) in d.class_slots.iter_mut().zip(&self.class_abilities) {
            for name in abilities {
                if !slot.abilities.contains(name) {
                    slot.abilities.push(name.clone());
                }
            }
        }
    }
}

impl CreationRules {
    /// The scores the character has: as typed with Enter Character, else
    /// after the race's adjustments.
    pub fn draft_scores(&self, draft: &Draft) -> [u32; 6] {
        if draft.entered.is_some() {
            let (low, high) = ENTERED_SCORES;
            draft.scores.map(|s| s.clamp(low, high))
        } else {
            self.adjusted(&draft.race, &draft.scores)
        }
    }

    /// Enter Character's classes for the race: every combination it may
    /// take in the setting, the class minimums not checked (0x5b5f22).
    pub fn entered_class_options(&self, setting: &Setting, race: &str) -> Vec<Vec<String>> {
        let mut options = self.combinations_of(setting, race);
        options.sort_by_cached_key(|c| (c.len(), crate::collate::sort_key(&c.join("/"))));
        options
    }

    /// The classes "All Classes..." (IDD 193) offers in each group, one of
    /// which may be picked: Warriors, Rogues, Priests, Wizards and
    /// Psionicists.
    pub fn classes_by_group(&self, setting: &Setting) -> [Vec<String>; 5] {
        let mut groups: [Vec<String>; 5] = Default::default();
        for class in &self.classes {
            let Ok(g) = usize::try_from(class.group) else {
                continue;
            };
            if g < 5 && setting.allows_class(&class.name, class.group == 3) {
                groups[g].push(class.name.clone());
            }
        }
        for group in &mut groups {
            group.sort_by_cached_key(|n| crate::collate::sort_key(n));
        }
        groups
    }

    /// Each class's level: as the Levels dialog left it with Enter
    /// Character and in Update Character, 1st in New Character.
    pub(crate) fn draft_levels(&self, draft: &Draft) -> Vec<u32> {
        let n = draft.classes.len();
        if draft.class_levels.len() == n {
            draft
                .class_levels
                .iter()
                .map(|l| (*l).clamp(1, 100))
                .collect()
        } else {
            vec![1; n]
        }
    }

    /// What keeps an entered character from being saved, of what New
    /// Character checks on its first pages: classes the databases have,
    /// at most one of each group, and an alignment.
    pub(crate) fn entered_problem(&self, draft: &Draft) -> Option<String> {
        if draft.classes.is_empty() {
            return Some(tr("Choose a class").into());
        }
        let mut groups = [false; 5];
        for name in &draft.classes {
            let Some(class) = self.class(name) else {
                return Some(tr!("{name} is not a class in the databases", name));
            };
            if let Some(taken) = usize::try_from(class.group)
                .ok()
                .and_then(|g| groups.get_mut(g))
            {
                if *taken {
                    return Some(tr!(
                        "A {class} has two classes of the same group",
                        class = draft.class_title()
                    ));
                }
                *taken = true;
            }
        }
        if !ALIGNMENTS.contains(&draft.alignment.as_str()) {
            return Some(tr("Choose an alignment").into());
        }
        None
    }

    /// Sets the entered character at 1st level in the classes chosen, as
    /// the Name page first finds it: the hit dice and the PSPs New
    /// Character gives, no experience and no character points. The
    /// dialog's own choices are kept.
    pub fn start_entered(&self, draft: &mut Draft) -> Result<()> {
        let Some(previous) = draft.entered.clone() else {
            return Ok(());
        };
        draft.class_levels = vec![1; draft.classes.len()];
        draft.level = 1;
        let character = self.build_new(draft)?;
        let mut entered = Entered {
            level_limits: previous.level_limits,
            prime_bonus: previous.prime_bonus,
            points_per_level: previous.points_per_level,
            ..Entered::default()
        };
        entered.take(&character);
        entered.character_points = 0;
        draft.entered = Some(entered);
        Ok(())
    }
}
