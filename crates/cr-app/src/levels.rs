//! A character's class levels with no toolkit in them: each class's level
//! and experience as a line, the Levels dialog (IDD 368) that Enter
//! Character's and Update Character's Name pages open alike, and its Add
//! Experience.

use cr_formats::character::Character;
use cr_i18n::tr;
use cr_store::advance::{AdvanceRules, ClassLevel, psionic_total};
use cr_store::create::Dice;

/// The highest level the Levels dialog takes.
pub const MOST_LEVEL: u32 = 100;
/// The experience Add Experience gives or takes at most.
pub const MOST_EXPERIENCE: i64 = 10_000_000;

/// "12,500", as the interface language groups the digits.
fn thousands(n: u32) -> String {
    cr_i18n::thousands(n.into())
}

/// "Level 3 · 2,500 XP · next level at 5,000", for a class.
pub fn level_text(class: &ClassLevel) -> String {
    let (level, xp) = (class.level, thousands(class.experience));
    match (class.next, class.limit) {
        (_, Some(limit)) if class.level >= limit => {
            tr!(
                "Level {level} · {xp} XP · level limit {limit}",
                level,
                xp,
                limit
            )
        }
        (Some(next), _) => tr!(
            "Level {level} · {xp} XP · next level at {next}",
            level,
            xp,
            next = thousands(next)
        ),
        (None, Some(limit)) => {
            tr!(
                "Level {level} · {xp} XP · level limit {limit}",
                level,
                xp,
                limit
            )
        }
        (None, None) => tr!("Level {level} · {xp} XP", level, xp),
    }
}

/// A class with its level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassRow {
    pub name: String,
    pub level: u32,
    /// Its level and experience, as [`level_text`] says them.
    pub text: String,
}

/// Each class of `character` with its level.
pub fn class_rows(advance: &AdvanceRules, character: &Character) -> Vec<ClassRow> {
    match advance.levels(character) {
        Ok(classes) => classes
            .iter()
            .map(|class| ClassRow {
                name: class.name.clone(),
                level: class.level,
                text: level_text(class),
            })
            .collect(),
        Err(_) => character
            .data
            .classes
            .names
            .iter()
            .map(|name| ClassRow {
                name: name.clone(),
                level: 1,
                text: String::new(),
            })
            .collect(),
    }
}

/// Whether a character has PSPs to show: a psionicist, or anyone with
/// some.
pub fn psionic(advance: &AdvanceRules, character: &Character) -> bool {
    character
        .data
        .classes
        .names
        .iter()
        .any(|c| advance.psionicist(c))
        || psionic_total(character) > 0
}

/// A change in the Levels dialog, or in its Add Experience.
#[derive(Debug, Clone, PartialEq)]
pub enum LevelsInput {
    /// A class's level, by its index.
    Level(usize, u32),
    PrimeBonus(bool),
    /// "Level Limits": none, the racial limits, or those and the prime
    /// requisite bonus.
    Limits(usize),
    /// Skills & Powers' unspent character points.
    CharacterPoints(u32),
    PointsPerLevel(u32),
    /// Opens Add Experience.
    AddExperience,
    Amount(i64),
    /// The class given the experience; then All Classes, then Split
    /// Between Classes.
    Target(usize),
    /// Add on Add Experience.
    Give,
    /// Cancel on Add Experience.
    Cancel,
}

/// The Levels dialog (0x616c00) as edited: each class's level from 1 to
/// 100, the prime requisite bonus, the level limits and, with Skills &
/// Powers, the character points.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelsForm {
    character: Box<Character>,
    pub classes: Vec<ClassRow>,
    pub prime_bonus: bool,
    pub points_per_level: u32,
    /// "Level Limits" is offered: the Expansion's databases.
    pub limits_shown: bool,
    pub limits: usize,
    pub skills_and_powers: bool,
    pub character_points: u32,
    /// What the last change said: the maximum level reached, or an error.
    pub notice: Option<String>,
    /// Add Experience, while it is open.
    pub experience: Option<ExperienceForm>,
}

/// Add Experience (0x6182a8): how much, and to which classes.
#[derive(Debug, Clone, PartialEq)]
pub struct ExperienceForm {
    pub amount: i64,
    /// The classes, then All Classes and Split Between Classes when there
    /// are several.
    pub targets: Vec<String>,
    pub target: usize,
    /// Why the experience could not be given.
    pub error: Option<String>,
    /// Its own prime requisite bonus and points per level, opened by
    /// itself; from the Levels dialog it takes the dialog's.
    pub settings: Option<ExperienceSettings>,
}

/// What Add Experience gives with when opened by itself, as the
/// editor's Add… opens it.
#[derive(Debug, Clone, PartialEq)]
pub struct ExperienceSettings {
    pub prime_bonus: bool,
    pub points_per_level: u32,
    /// The points per level are offered: Skills & Powers.
    pub skills_and_powers: bool,
}

impl ExperienceForm {
    /// Add Experience for `character`, 1,000 to give; several classes
    /// split it.
    pub fn new(character: &Character, settings: Option<ExperienceSettings>) -> Self {
        let names = &character.data.classes.names;
        let mut targets = names.clone();
        if names.len() > 1 {
            targets.push(tr("All Classes").into());
            targets.push(tr("Split Between Classes").into());
        }
        Self {
            amount: 1000,
            target: if names.len() > 1 { names.len() + 1 } else { 0 },
            targets,
            error: None,
            settings,
        }
    }

    /// Changes the amount, the classes or, with its own, the settings.
    pub fn input(&mut self, input: LevelsInput) {
        match input {
            LevelsInput::Amount(n) => self.amount = n.clamp(-MOST_EXPERIENCE, MOST_EXPERIENCE),
            LevelsInput::Target(t) if t < self.targets.len() => self.target = t,
            LevelsInput::PrimeBonus(on) => {
                if let Some(settings) = &mut self.settings {
                    settings.prime_bonus = on;
                }
            }
            LevelsInput::PointsPerLevel(n) => {
                if let Some(settings) = &mut self.settings {
                    settings.points_per_level = n.min(10);
                }
            }
            _ => {}
        }
    }

    /// Gives the amount to its classes of `character`, with `prime_bonus`
    /// and `points_per_level` unless the form has its own; false, with
    /// the error, if it could not.
    pub fn give(
        &mut self,
        advance: &AdvanceRules,
        character: &mut Character,
        (prime_bonus, points_per_level): (bool, u32),
        dice: &mut Dice,
    ) -> bool {
        let (prime_bonus, points_per_level) = match &self.settings {
            Some(s) => (s.prime_bonus, s.points_per_level),
            None => (prime_bonus, points_per_level),
        };
        let n = character.data.classes.names.len();
        let (classes, amount) = match self.target {
            g if g < n => (vec![g], self.amount),
            g if g == n => ((0..n).collect(), self.amount),
            _ => ((0..n).collect(), self.amount / n.max(1) as i64),
        };
        let mut changed = character.clone();
        let result = classes.into_iter().rev().try_for_each(|g| {
            advance.add_experience(&mut changed, g, amount, prime_bonus, points_per_level, dice)
        });
        match result {
            Ok(()) => {
                *character = changed;
                true
            }
            Err(e) => {
                self.error = Some(e.to_string());
                false
            }
        }
    }
}

impl LevelsForm {
    /// The dialog over `character`, with the prime requisite bonus and
    /// the character points per level it opens with.
    pub fn new(
        advance: &AdvanceRules,
        character: Character,
        prime_bonus: bool,
        points_per_level: u32,
    ) -> Self {
        let mut form = Self {
            classes: Vec::new(),
            prime_bonus,
            points_per_level,
            limits_shown: advance.expansion(),
            limits: 0,
            skills_and_powers: character.skills_and_powers(),
            character_points: 0,
            notice: None,
            experience: None,
            character: Box::new(character),
        };
        form.refresh(advance);
        form
    }

    /// The character as the dialog has changed it.
    pub fn character(&self) -> &Character {
        &self.character
    }

    fn refresh(&mut self, advance: &AdvanceRules) {
        self.classes = class_rows(advance, &self.character);
        self.limits = self.character.data.unknown_0x1dc4.min(2) as usize;
        self.character_points = self.character.data.unknown_0x1778[0];
    }

    /// Makes a change in the dialog or in its Add Experience.
    pub fn input(&mut self, advance: &AdvanceRules, input: LevelsInput, dice: &mut Dice) {
        match input {
            LevelsInput::Level(g, level) if g < self.classes.len() => {
                let result = advance.set_level(
                    &mut self.character,
                    g,
                    level.clamp(1, MOST_LEVEL),
                    self.points_per_level,
                    dice,
                );
                self.notice = match result {
                    Ok(message) => message,
                    Err(e) => Some(e.to_string()),
                };
            }
            LevelsInput::PrimeBonus(on) => self.prime_bonus = on,
            LevelsInput::Limits(n) if n <= 2 && self.limits_shown => {
                self.character.data.unknown_0x1dc4 = n as u32;
            }
            LevelsInput::CharacterPoints(n) if self.skills_and_powers => {
                self.character.data.unknown_0x1778[0] = n.min(9999);
            }
            LevelsInput::PointsPerLevel(n) => self.points_per_level = n.min(10),
            LevelsInput::AddExperience => {
                self.experience = Some(ExperienceForm::new(&self.character, None));
            }
            LevelsInput::Amount(_) | LevelsInput::Target(_) => {
                if let Some(experience) = &mut self.experience {
                    experience.input(input);
                }
            }
            LevelsInput::Give => self.give(advance, dice),
            LevelsInput::Cancel => self.experience = None,
            _ => {}
        }
        self.refresh(advance);
    }

    /// Gives Add Experience's amount to its classes, with the dialog's
    /// prime requisite bonus and points per level; closes it if it could.
    fn give(&mut self, advance: &AdvanceRules, dice: &mut Dice) {
        let Some(experience) = &mut self.experience else {
            return;
        };
        let settings = (self.prime_bonus, self.points_per_level);
        if experience.give(advance, &mut self.character, settings, dice) {
            self.experience = None;
        }
    }

    /// Closes the dialog: each class gets the abilities its levels grant
    /// (0x61990a); gives the character.
    pub fn close(mut self, advance: &AdvanceRules) -> Character {
        advance.grant_level_abilities(&mut self.character);
        *self.character
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_are_grouped() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(2500), "2,500");
        assert_eq!(thousands(1_250_000), "1,250,000");
    }

    #[test]
    fn a_level_says_the_next_or_the_limit() {
        let class = ClassLevel {
            name: "Fighter".into(),
            experience: 2500,
            level: 2,
            next: Some(4000),
            limit: None,
        };
        assert_eq!(
            level_text(&class),
            "Level 2 · 2,500 XP · next level at 4,000"
        );
        let limited = ClassLevel {
            limit: Some(2),
            ..class
        };
        assert_eq!(level_text(&limited), "Level 2 · 2,500 XP · level limit 2");
    }
}
