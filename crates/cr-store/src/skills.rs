//! The Skills page of the Expansion's Update Character: the Rogue Skills,
//! Psionics, Wild Talents and Psionic Attacks and Defenses dialogs. None of
//! them costs Skills & Powers character points, and Core Rules characters
//! get the same rules.
//!
//! * Rogue skills keep the points put in at each level apart
//!   (`rogue_skills` entries, `[L-1]` for level L), and Update changes only
//!   the column of the rogue class's current level. A class with rogue
//!   skills (`flag_0x27a4`) has its 1st-level points (`unknown_0xb4[0]`,
//!   thief 60) and then its points per level (`unknown_0xb4[1]`, thief
//!   30) to put in; a class with `flag_0x278c` (thief) may put at most
//!   half of the points available at a level into one skill. Lowering a
//!   skill gives its points back. The original asks before going over that
//!   limit or past 95%, which the app only warns about; spending more
//!   points than there are is refused. Kit bonus points are not modelled.
//! * Psionicists choose disciplines, a primary discipline, sciences and
//!   devotions, as many as their psionicist level allows (the program's
//!   tables; a psychic adept, `flags_0x780[5]` on a class ability, gets
//!   more). Sciences and devotions need a primary discipline, must belong
//!   to a discipline taken, and may not be above the psionicist's level;
//!   the original refuses all of that, and dropping a discipline drops
//!   its powers there, so the app refuses dropping one whose powers are
//!   kept. Its other checks (prerequisites,
//!   twice as many devotions as sciences, sciences outside the primary)
//!   only ask, and the app leaves them to the player. Disciplines are
//!   saved sorted, powers in the order taken.
//! * Every other character may take wild talents, any power with a chance
//!   on the wild talent table (`unknown_0x40`), stored as the
//!   psionicist's sciences and devotions. Saving them gives the 1st-level
//!   PSPs their initial costs (`unknown_0x48`) instead of the old ones'; a
//!   character without PSPs first rolls 10 + 1d4 + the bonuses of
//!   Intuition, Fitness and Reason above 15. The original's Test and Roll
//!   buttons are not offered.
//! * Attack and defense modes are lists of the program's own names; their
//!   limits by level only ask, so the app leaves them to the player.

use std::collections::{HashMap, HashSet};

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::classes::Class;
use cr_formats::parts::PartsFile;
use cr_i18n::tr;

use crate::advance::AdvanceRules;
use crate::collate::sort_key;
use crate::create::Dice;
use crate::npc::magic::{bonus16, psi_dev, psi_disc, psi_sci};
use crate::sheet::SheetRules;
use crate::userdb::powers::DISCIPLINES;
use crate::{Error, Installation, Result};

/// The most a rogue skill may reach without the original asking.
const MOST_ROGUE_SCORE: i32 = 95;

/// Where the Psionic Attacks and Defenses dialog's names are in
/// `Program/Tsrp3.exe` (the strings its list boxes are filled with).
const ATTACK_TEXTS: [u32; 5] = [0x81a468, 0x81a478, 0x81a48c, 0x81a4a0, 0x81a4b4];
const DEFENSE_TEXTS: [u32; 5] = [0x81a4c8, 0x81a4e0, 0x81a4f4, 0x81a508, 0x81a51c];

/// The psionicist class group.
const PSIONICIST: i32 = 4;

#[derive(Debug, Clone, Copy, Default)]
struct ClassEntry {
    group: i32,
    /// Has rogue skills (`flag_0x27a4`).
    rogue: bool,
    /// Limits the points one skill gets per level (`flag_0x278c`).
    capped: bool,
    /// Rogue points at 1st level and per level after (`unknown_0xb4`).
    first: i64,
    per_level: i64,
}

/// A psionic power, as the dialogs see it.
#[derive(Debug, Clone)]
pub struct Power {
    pub name: String,
    /// Its discipline's name; empty for metapsionic powers.
    pub discipline: String,
    pub devotion: bool,
    pub level: u32,
    /// Its initial PSP cost (`unknown_0x48`).
    pub psp: u32,
    /// Its chance on the wild talent table (`unknown_0x40`).
    pub wild_chance: u32,
}

/// The rules of the Skills page, read from the databases.
#[derive(Debug)]
pub struct SkillRules {
    advance: AdvanceRules,
    sheet: SheetRules,
    classes: HashMap<String, ClassEntry>,
    powers: Vec<Power>,
    /// Class abilities that make a psychic adept.
    adept_abilities: HashSet<String>,
    attacks: Vec<String>,
    defenses: Vec<String>,
}

impl Installation {
    pub fn skill_rules(&self) -> Result<SkillRules> {
        let mut classes = HashMap::new();
        for object in self.list_objects("Class")? {
            if let Object::Class(class) = object
                && let Class::Expansion { data, .. } = *class
            {
                classes.entry(data.name.clone()).or_insert(ClassEntry {
                    group: data.unknown_0x14[0],
                    rogue: data.flag_0x27a4.unwrap_or(0) != 0,
                    capped: data.flag_0x278c != 0,
                    first: data.unknown_0xb4[0].into(),
                    per_level: data.unknown_0xb4[1].into(),
                });
            }
        }
        let mut powers: Vec<Power> = Vec::new();
        if self.path("Database/Psionic.dat").is_some() {
            for object in self.list_objects("Psionic")? {
                if let Object::PsionicPower(p) = object
                    && !p.name.is_empty()
                    && !powers.iter().any(|q| q.name == p.name)
                {
                    powers.push(Power {
                        discipline: DISCIPLINES
                            .get(p.unknown_0x28 as usize)
                            .map_or_else(String::new, |d| (*d).to_owned()),
                        devotion: p.unknown_0x2c != 0,
                        level: p.unknown_0x44,
                        psp: p.unknown_0x48,
                        wild_chance: p.unknown_0x40,
                        name: p.name,
                    });
                }
            }
        }
        powers.sort_by_cached_key(|p| sort_key(&p.name));
        let mut adept_abilities = HashSet::new();
        for relative in ["Database/Parts.dat", "UserDbas/PartsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file = PartsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                path: relative.into(),
                source,
            })?;
            for object in file.of_type("Class Abilities").unwrap_or_default() {
                if let Object::Part(p) = object
                    && p.data.flags_0x780[5] != 0
                {
                    adept_abilities.insert(p.head.name.clone());
                }
            }
        }
        let image = crate::exe::Image::open(self, crate::treasure::Flavor::Expansion).ok();
        let texts = |vas: &[u32]| -> Vec<String> {
            image.as_ref().map_or_else(Vec::new, |i| {
                vas.iter().filter_map(|&va| i.text(va)).collect()
            })
        };
        Ok(SkillRules {
            advance: self.advance_rules()?,
            sheet: self.sheet_rules()?,
            classes,
            powers,
            adept_abilities,
            attacks: texts(&ATTACK_TEXTS),
            defenses: texts(&DEFENSE_TEXTS),
        })
    }
}

/// A character's choices on the Skills page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterSkills {
    /// The points put into each rogue skill (in `rogue_skills` order) at
    /// the rogue class's current level.
    pub rogue: Vec<u32>,
    /// The primary discipline, and a psychic adept's second.
    pub primary: [String; 2],
    pub disciplines: Vec<String>,
    /// A psionicist's powers, or a wild talent's.
    pub sciences: Vec<String>,
    pub devotions: Vec<String>,
    pub attacks: Vec<String>,
    pub defenses: Vec<String>,
}

/// One rogue skill as the dialog shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RogueSkill {
    /// The entry's name (the class ability's).
    pub name: String,
    /// The name as the dialog shows it.
    pub label: String,
    /// Points put in at earlier levels.
    pub earlier: u32,
    /// The score without the points of the current level.
    pub base_score: i32,
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort_by_cached_key(|n| sort_key(n));
    names
}

impl SkillRules {
    fn classes<'a>(&'a self, character: &'a Character) -> impl Iterator<Item = &'a ClassEntry> {
        character
            .data
            .classes
            .names
            .iter()
            .filter_map(|c| self.classes.get(c))
    }

    /// The class slot with rogue skills and its level.
    pub fn rogue_level(&self, character: &Character) -> Option<(usize, u32)> {
        let slot = character
            .data
            .classes
            .names
            .iter()
            .position(|c| self.classes.get(c).is_some_and(|c| c.rogue))?;
        let level = self.advance.level_of(character, slot).ok()?.max(1);
        Some((slot, level))
    }

    /// The rogue skills, in `rogue_skills` order.
    pub fn rogue_skills(&self, character: &Character) -> Vec<RogueSkill> {
        let Some((_, level)) = self.rogue_level(character) else {
            return Vec::new();
        };
        let column = level as usize - 1;
        let scores = self.sheet.rogue_scores(character).unwrap_or_default();
        character
            .data
            .rogue_skills
            .entries
            .iter()
            .map(|(name, values)| {
                let current = values.get(column).copied().unwrap_or(0);
                let earlier = values[..column.min(values.len())].iter().sum();
                let score = scores.iter().find(|s| &s.0 == name).map_or(0, |s| s.1);
                RogueSkill {
                    name: name.clone(),
                    label: crate::sheet::ability_display_name(name),
                    earlier,
                    base_score: score - current as i32,
                }
            })
            .collect()
    }

    /// The rogue points of the levels so far: the best 1st-level points of
    /// the rogue classes, and their points per level after.
    fn rogue_budget(&self, character: &Character, level: u32) -> i64 {
        let rogue = || self.classes(character).filter(|c| c.rogue);
        let first = rogue().map(|c| c.first).max().unwrap_or(0);
        let per_level = rogue().map(|c| c.per_level).max().unwrap_or(0);
        first + per_level * i64::from(level.saturating_sub(1))
    }

    /// The rogue points left with `new`; negative when it puts in more
    /// than there are.
    pub fn rogue_points_left(&self, character: &Character, new: &CharacterSkills) -> i64 {
        let Some((_, level)) = self.rogue_level(character) else {
            return 0;
        };
        let earlier: i64 = self
            .rogue_skills(character)
            .iter()
            .map(|s| i64::from(s.earlier))
            .sum();
        let current: i64 = new.rogue.iter().map(|&p| i64::from(p)).sum();
        self.rogue_budget(character, level) - earlier - current
    }

    /// The most one skill should get at this level, for a class that
    /// limits it: half of the points available at this level.
    pub fn rogue_limit(&self, character: &Character) -> Option<u32> {
        let (slot, level) = self.rogue_level(character)?;
        let class = self.classes.get(&character.data.classes.names[slot])?;
        if !class.capped {
            return None;
        }
        let earlier: i64 = self
            .rogue_skills(character)
            .iter()
            .map(|s| i64::from(s.earlier))
            .sum();
        Some(((self.rogue_budget(character, level) - earlier).max(0) / 2) as u32)
    }

    /// What the original would ask about before taking `new`'s rogue
    /// points: a skill over the limit for one level, or past 95%.
    pub fn rogue_warnings(&self, character: &Character, new: &CharacterSkills) -> Vec<String> {
        let limit = self.rogue_limit(character);
        let mut warnings = Vec::new();
        for (skill, &points) in self.rogue_skills(character).iter().zip(&new.rogue) {
            let name = &skill.label;
            if limit.is_some_and(|l| points > l) {
                warnings.push(tr!(
                    "{skill} gets more than the {limit} points one skill may get at this level",
                    skill = name.as_str(),
                    limit = limit.unwrap_or(0)
                ));
            }
            if skill.base_score + points as i32 > MOST_ROGUE_SCORE {
                warnings.push(tr!(
                    "{skill} goes past {most}%",
                    skill = name.as_str(),
                    most = MOST_ROGUE_SCORE
                ));
            }
        }
        warnings
    }

    /// The psionicist level, for a character with a psionicist class.
    pub fn psionic_level(&self, character: &Character) -> Option<u32> {
        let slot = character
            .data
            .classes
            .names
            .iter()
            .position(|c| self.classes.get(c).is_some_and(|c| c.group == PSIONICIST))?;
        self.advance.level_of(character, slot).ok()
    }

    /// Whether a class ability makes the character a psychic adept, with
    /// two primary disciplines and more powers.
    pub fn adept(&self, character: &Character) -> bool {
        character
            .data
            .class_slots
            .iter()
            .flat_map(|s| &s.abilities)
            .any(|a| self.adept_abilities.contains(a))
    }

    /// How many disciplines, sciences and devotions the psionicist may
    /// have.
    pub fn allowed(&self, character: &Character) -> (usize, usize, usize) {
        let level = self.psionic_level(character).unwrap_or(0) as i32;
        let adept = self.adept(character);
        let count = |n: i32| n.max(0) as usize;
        (
            count(psi_disc(level, adept)),
            count(psi_sci(level, adept)),
            count(psi_dev(level, adept)),
        )
    }

    pub fn discipline_names(&self) -> Vec<String> {
        DISCIPLINES.iter().map(|d| (*d).to_owned()).collect()
    }

    pub fn power(&self, name: &str) -> Option<&Power> {
        self.powers.iter().find(|p| p.name == name)
    }

    /// The sciences or devotions a psionicist may take: those of the
    /// disciplines taken up to the psionicist's level, sorted.
    pub fn power_choices(
        &self,
        character: &Character,
        disciplines: &[String],
        devotion: bool,
    ) -> Vec<String> {
        let level = self.psionic_level(character).unwrap_or(0);
        self.powers
            .iter()
            .filter(|p| p.devotion == devotion && p.level <= level)
            .filter(|p| disciplines.contains(&p.discipline))
            .map(|p| p.name.clone())
            .collect()
    }

    /// The powers on the wild talent table, sorted.
    pub fn wild_talent_choices(&self) -> Vec<String> {
        self.powers
            .iter()
            .filter(|p| p.wild_chance > 0)
            .map(|p| p.name.clone())
            .collect()
    }

    pub fn attack_names(&self) -> &[String] {
        &self.attacks
    }

    pub fn defense_names(&self) -> &[String] {
        &self.defenses
    }

    /// The character's choices as stored.
    pub fn skills_of(&self, character: &Character) -> CharacterSkills {
        let d = &character.data;
        let column = self.rogue_level(character).map(|(_, l)| l as usize - 1);
        CharacterSkills {
            rogue: column.map_or_else(Vec::new, |c| {
                d.rogue_skills
                    .entries
                    .iter()
                    .map(|e| e.1.get(c).copied().unwrap_or(0))
                    .collect()
            }),
            primary: d.unknown_0x1980.clone(),
            disciplines: d.unknown_0x196c[0].clone(),
            sciences: d.unknown_0x196c[1].clone(),
            devotions: d.unknown_0x196c[2].clone(),
            attacks: d.unknown_0xac[6].clone(),
            defenses: d.unknown_0x19c4.clone(),
        }
    }

    /// Why `new` cannot be saved, if it cannot.
    pub fn problem(&self, character: &Character, new: &CharacterSkills) -> Option<String> {
        let left = self.rogue_points_left(character, new);
        if left < 0 {
            return Some(tr!(
                "Not enough rogue skill points: the skills need {more} more",
                more = -left
            ));
        }
        let old = self.skills_of(character);
        let added = || {
            let new_of = |new: &'_ [String], old: &'_ [String], devotion| {
                new.iter()
                    .filter(|p| !old.contains(p))
                    .map(move |p| (p.clone(), devotion))
                    .collect::<Vec<_>>()
            };
            let mut added = new_of(&new.sciences, &old.sciences, false);
            added.extend(new_of(&new.devotions, &old.devotions, true));
            added
        };
        let Some(level) = self.psionic_level(character) else {
            // Anyone else's powers are wild talents.
            return added()
                .into_iter()
                .find_map(|(name, devotion)| match self.power(&name) {
                    Some(p) if p.wild_chance > 0 && p.devotion == devotion => None,
                    _ => Some(tr!("{power} is not a wild talent", power = name.as_str())),
                });
        };
        let (disciplines, sciences, devotions) = self.allowed(character);
        if new.disciplines.len() > disciplines.max(old.disciplines.len()) {
            return Some(tr!(
                "Too many disciplines for a psionicist of this level (at most {count})",
                count = disciplines
            ));
        }
        if new.sciences.len() > sciences.max(old.sciences.len()) {
            return Some(tr!(
                "Too many sciences for a psionicist of this level (at most {count})",
                count = sciences
            ));
        }
        if new.devotions.len() > devotions.max(old.devotions.len()) {
            return Some(tr!(
                "Too many devotions for a psionicist of this level (at most {count})",
                count = devotions
            ));
        }
        let primaries = if self.adept(character) { 2 } else { 1 };
        if let Some(primary) = new.primary[..primaries]
            .iter()
            .find(|p| !p.is_empty() && !new.disciplines.contains(p))
        {
            return Some(tr!(
                "The primary discipline {discipline} is not among the disciplines",
                discipline = primary.as_str()
            ));
        }
        // Dropping a discipline drops its powers in the original.
        if new.disciplines != old.disciplines
            && let Some((name, power)) = new
                .sciences
                .iter()
                .chain(&new.devotions)
                .filter_map(|n| Some((n, self.power(n)?)))
                .find(|(_, p)| !p.discipline.is_empty() && !new.disciplines.contains(&p.discipline))
        {
            return Some(tr!(
                "{power} needs the {discipline} discipline",
                power = name.as_str(),
                discipline = power.discipline.as_str()
            ));
        }
        for (name, devotion) in added() {
            if new.primary[..primaries].iter().any(String::is_empty) {
                return Some(tr("Choose a primary discipline before any power").to_owned());
            }
            let power = match self.power(&name) {
                Some(p) if p.devotion == devotion => p,
                _ if devotion => {
                    return Some(tr!("{power} is not a devotion", power = name.as_str()));
                }
                _ => return Some(tr!("{power} is not a science", power = name.as_str())),
            };
            if !power.discipline.is_empty() && !new.disciplines.contains(&power.discipline) {
                return Some(tr!(
                    "{power} needs the {discipline} discipline",
                    power = name.as_str(),
                    discipline = power.discipline.as_str()
                ));
            }
            if power.level > level {
                return Some(tr!(
                    "You must be at least level {level} to use {power}",
                    level = power.level,
                    power = name.as_str()
                ));
            }
        }
        None
    }

    /// Writes `new` into the character, as the dialogs do when closed:
    /// only what changed is rewritten. A wild talent without PSPs rolls
    /// them with `dice`.
    pub fn apply(
        &self,
        character: &mut Character,
        new: &CharacterSkills,
        dice: &mut Dice,
    ) -> Result<()> {
        if let Some(problem) = self.problem(character, new) {
            return Err(Error::Invalid(problem));
        }
        let old = self.skills_of(character);
        let psionicist = self.psionic_level(character).is_some();
        let column = self.rogue_level(character).map(|(_, l)| l as usize - 1);
        let wild_psp = |names: &[String]| -> u32 {
            names
                .iter()
                .filter_map(|n| self.power(n))
                .map(|p| p.psp)
                .sum()
        };
        let d = &mut character.data;
        if let Some(column) = column {
            for (entry, &points) in d.rogue_skills.entries.iter_mut().zip(&new.rogue) {
                if let Some(value) = entry.1.get_mut(column) {
                    *value = points;
                }
            }
        }
        if psionicist && (new.primary != old.primary || new.disciplines != old.disciplines) {
            d.unknown_0x1980.clone_from(&new.primary);
            d.unknown_0x196c[0] = sorted(new.disciplines.clone());
        }
        if new.sciences != old.sciences || new.devotions != old.devotions {
            if !psionicist {
                let powers: Vec<String> =
                    new.sciences.iter().chain(&new.devotions).cloned().collect();
                let old_powers: Vec<String> =
                    old.sciences.iter().chain(&old.devotions).cloned().collect();
                let total: u32 = d.unknown_0x1430.iter().sum();
                if !powers.is_empty() && total < 1 {
                    let [_, constitution, intelligence, wisdom, _] = d.abilities;
                    let bonus = |v: u32| bonus16(v as i32) as u32;
                    d.unknown_0x1430[0] = 10
                        + bonus(wisdom[1])
                        + bonus(constitution[2])
                        + bonus(intelligence[1])
                        + dice.roll(1, 4);
                }
                d.unknown_0x1430[0] =
                    d.unknown_0x1430[0].saturating_sub(wild_psp(&old_powers)) + wild_psp(&powers);
            }
            d.unknown_0x196c[1].clone_from(&new.sciences);
            d.unknown_0x196c[2].clone_from(&new.devotions);
        }
        if new.attacks != old.attacks {
            d.unknown_0xac[6].clone_from(&new.attacks);
        }
        if new.defenses != old.defenses {
            d.unknown_0x19c4.clone_from(&new.defenses);
        }
        Ok(())
    }
}
