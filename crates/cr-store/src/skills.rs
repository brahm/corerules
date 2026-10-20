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
//!   points than there are is refused.
//! * The kit (62bc54 to 62cc45) may replace the class's points at 1st
//!   level and per level (+0x120, +0x124). What it gives is not spent:
//!   its fixed adjustments (+0x174) and its bonus of each level, which
//!   the player puts into one skill of a list (+0x13c from +0x140 at 1st
//!   level, +0x154 from +0x158 later) for every level still without one,
//!   and which may not be taken back. It may also have the player choose
//!   skills (+0x16c of them, for a +0x170% bonus) until they are chosen.
//!   Both are kept as `CPartKitBonusOb`s (+0x15e4) with the rogue level
//!   they were given at.
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

/// What a kit does to the rogue skills.
#[derive(Debug, Clone, Default)]
struct KitRogue {
    /// Points at 1st level and per level after, when not −1.
    first: i32,
    per_level: i32,
    /// Fixed adjustments (+0x174): skill, amount.
    adjust: Vec<(String, i32)>,
    /// The bonus at 1st level and its skills, then at each later level.
    first_bonus: (i32, Vec<String>),
    later_bonus: (i32, Vec<String>),
    /// How many skills to choose and their bonus.
    chosen: (i32, i32),
}

/// A kit's bonus to a rogue skill (`CPartKitBonusOb`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitBonus {
    /// The skill's class ability.
    pub name: String,
    /// The rogue level it was given at.
    pub level: i32,
    pub bonus: i32,
}

/// A level whose kit bonus is still to go to a skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RogueBonusPick {
    pub level: u32,
    pub amount: i32,
    /// The skills it may go to.
    pub names: Vec<String>,
}

/// The skills the kit has the character choose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChosenPick {
    /// How many more may be chosen.
    pub count: usize,
    pub bonus: i32,
    pub names: Vec<String>,
}

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
    kits: HashMap<String, KitRogue>,
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
        let kits = self
            .kit_parts()?
            .into_iter()
            .map(|(name, kit)| {
                let rogue = KitRogue {
                    first: kit.i_or(0x120, -1),
                    per_level: kit.i_or(0x124, -1),
                    adjust: kit.rogue_adjust.clone(),
                    first_bonus: (kit.i(0x13c), kit.sa(0x140).to_vec()),
                    later_bonus: (kit.i(0x154), kit.sa(0x158).to_vec()),
                    chosen: (kit.i(0x16c), kit.i(0x170)),
                };
                (name, rogue)
            })
            .collect();
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
            kits,
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
    /// The kit's bonus of each level (`[L-1]`; none for a level without
    /// one yet) and the skills it had chosen.
    pub kit_bonuses: Vec<Option<KitBonus>>,
    pub kit_chosen: Vec<KitBonus>,
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

fn kit_bonus(object: &Object) -> Option<KitBonus> {
    match object {
        Object::KitBonus(b) => Some(KitBonus {
            name: b.name.clone(),
            level: b.unknown_0x8,
            bonus: b.unknown_0xc,
        }),
        _ => None,
    }
}

fn kit_bonus_object(bonus: &KitBonus) -> Box<Object> {
    Box::new(Object::KitBonus(cr_formats::classes::KitBonus {
        version: 1,
        name: bonus.name.clone(),
        unknown_0x8: bonus.level,
        unknown_0xc: bonus.bonus,
    }))
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

    fn kit(&self, character: &Character) -> Option<&KitRogue> {
        self.kits.get(&character.data.kit)
    }

    /// The rogue points of the levels so far: the best 1st-level points of
    /// the rogue classes, and their points per level after, unless the
    /// kit has its own (62bc54).
    fn rogue_budget(&self, character: &Character, level: u32) -> i64 {
        let rogue = || self.classes(character).filter(|c| c.rogue);
        let mut first = rogue().map(|c| c.first).max().unwrap_or(0);
        let mut per_level = rogue().map(|c| c.per_level).max().unwrap_or(0);
        if let Some(kit) = self.kit(character) {
            if kit.first != -1 {
                first = kit.first.into();
            }
            if kit.per_level != -1 {
                per_level = kit.per_level.into();
            }
        }
        first + per_level * i64::from(level.saturating_sub(1))
    }

    /// The kit's fixed adjustment to a skill (62c689).
    fn kit_adjustment(&self, character: &Character, name: &str) -> i64 {
        self.kit(character)
            .and_then(|k| k.adjust.iter().find(|(n, _)| n == name))
            .map_or(0, |a| a.1.into())
    }

    /// What the kit gave a skill, which is not spent (62d1c7): its fixed
    /// adjustment and its bonuses of every level.
    fn kit_points(&self, character: &Character, new: &CharacterSkills, name: &str) -> i64 {
        self.kit_adjustment(character, name)
            + new
                .kit_bonuses
                .iter()
                .flatten()
                .filter(|b| b.name == name)
                .map(|b| i64::from(b.bonus))
                .sum::<i64>()
    }

    /// The kit's bonus of `level` to a skill, if it went there (62d0bc).
    fn bonus_at(new: &CharacterSkills, level: u32, name: &str) -> i32 {
        new.kit_bonuses
            .get(level as usize - 1)
            .and_then(Option::as_ref)
            .filter(|b| b.name == name)
            .map_or(0, |b| b.bonus)
    }

    /// The points each skill has at the levels before the current one,
    /// with the bonuses `new` gives them.
    fn earlier(&self, character: &Character, new: &CharacterSkills) -> Vec<i64> {
        let old = self.skills_of(character);
        let level = self.rogue_level(character).map_or(1, |l| l.1);
        self.rogue_skills(character)
            .iter()
            .map(|skill| {
                let added: i64 = (1..level)
                    .filter(|&l| {
                        old.kit_bonuses
                            .get(l as usize - 1)
                            .is_none_or(Option::is_none)
                    })
                    .map(|l| i64::from(Self::bonus_at(new, l, &skill.name)))
                    .sum();
                i64::from(skill.earlier) + added
            })
            .collect()
    }

    /// The rogue points left with `new`; negative when it puts in more
    /// than there are. What the kit gave is not spent.
    pub fn rogue_points_left(&self, character: &Character, new: &CharacterSkills) -> i64 {
        let Some((_, level)) = self.rogue_level(character) else {
            return 0;
        };
        let earlier = self.earlier(character, new);
        let spent: i64 = self
            .rogue_skills(character)
            .iter()
            .zip(&earlier)
            .zip(&new.rogue)
            .map(|((skill, earlier), &current)| {
                earlier + i64::from(current) - self.kit_points(character, new, &skill.name)
            })
            .sum();
        self.rogue_budget(character, level) - spent
    }

    /// The most one skill should get at this level, for a class that
    /// limits it: half of the points available at this level, what the
    /// kit gave counted in (62bef6).
    pub fn rogue_limit(&self, character: &Character, new: &CharacterSkills) -> Option<u32> {
        let (slot, level) = self.rogue_level(character)?;
        let class = self.classes.get(&character.data.classes.names[slot])?;
        if !class.capped {
            return None;
        }
        let mut available = self.rogue_budget(character, level);
        if level > 1 {
            for (skill, earlier) in self
                .rogue_skills(character)
                .iter()
                .zip(self.earlier(character, new))
            {
                available += self.kit_points(character, new, &skill.name) - earlier;
            }
        }
        Some((available.max(0) / 2) as u32)
    }

    /// The fewest points each skill may have at this level: the kit's
    /// bonus of the level, and at 1st level its fixed adjustment
    /// (62c7e3).
    pub fn rogue_minimums(&self, character: &Character, new: &CharacterSkills) -> Vec<u32> {
        let Some((_, level)) = self.rogue_level(character) else {
            return Vec::new();
        };
        self.rogue_skills(character)
            .iter()
            .map(|skill| {
                let mut least = i64::from(Self::bonus_at(new, level, &skill.name));
                if level == 1 {
                    least += self.kit_adjustment(character, &skill.name);
                }
                least.max(0) as u32
            })
            .collect()
    }

    /// What the kit gave each skill, as the dialog's description lists
    /// it: its fixed adjustment, then each level's bonus.
    pub fn rogue_kit_points(
        &self,
        character: &Character,
        new: &CharacterSkills,
    ) -> Vec<(i32, Vec<(u32, i32)>)> {
        self.rogue_skills(character)
            .iter()
            .map(|skill| {
                let levels = new
                    .kit_bonuses
                    .iter()
                    .zip(1..)
                    .filter_map(|(b, l)| {
                        b.as_ref()
                            .filter(|b| b.name == skill.name)
                            .map(|b| (l, b.bonus))
                    })
                    .collect();
                (self.kit_adjustment(character, &skill.name) as i32, levels)
            })
            .collect()
    }

    /// The levels up to the rogue level whose kit bonus is still to go to
    /// a skill, with the skills of the kit's list the character has
    /// (62c05c, 62cc45).
    pub fn rogue_bonus_picks(&self, character: &Character) -> Vec<RogueBonusPick> {
        let (Some(kit), Some((_, level))) = (self.kit(character), self.rogue_level(character))
        else {
            return Vec::new();
        };
        let old = self.skills_of(character);
        let skills = self.rogue_skills(character);
        (1..=level)
            .filter(|&l| {
                old.kit_bonuses
                    .get(l as usize - 1)
                    .is_none_or(Option::is_none)
            })
            .filter_map(|l| {
                let (amount, list) = if l == 1 {
                    &kit.first_bonus
                } else {
                    &kit.later_bonus
                };
                let names: Vec<String> = list
                    .iter()
                    .filter(|n| skills.iter().any(|s| &s.name == *n))
                    .cloned()
                    .collect();
                (*amount != 0 && !names.is_empty()).then_some(RogueBonusPick {
                    level: l,
                    amount: *amount,
                    names,
                })
            })
            .collect()
    }

    /// Puts the kit's bonus of a level into a skill, or none, in the
    /// skill's points of that level.
    pub fn set_rogue_bonus(
        &self,
        character: &Character,
        new: &mut CharacterSkills,
        level: u32,
        name: Option<&str>,
    ) {
        let Some(pick) = self
            .rogue_bonus_picks(character)
            .into_iter()
            .find(|p| p.level == level)
        else {
            return;
        };
        let Some((_, current)) = self.rogue_level(character) else {
            return;
        };
        let skills = self.rogue_skills(character);
        let at = level as usize - 1;
        if new.kit_bonuses.len() <= at {
            new.kit_bonuses.resize(at + 1, None);
        }
        let points = |new: &mut CharacterSkills, name: &str, amount: i32| {
            if level == current
                && let Some(i) = skills.iter().position(|s| s.name == name)
                && let Some(value) = new.rogue.get_mut(i)
            {
                *value = value.saturating_add_signed(amount);
            }
        };
        if let Some(old) = new.kit_bonuses[at].take() {
            points(new, &old.name, -old.bonus);
        }
        if let Some(name) = name.filter(|n| pick.names.iter().any(|p| p == n)) {
            points(new, name, pick.amount);
            new.kit_bonuses[at] = Some(KitBonus {
                name: name.to_owned(),
                level: current as i32,
                bonus: pick.amount,
            });
        }
    }

    /// The skills the kit still has the character choose (62c8e6).
    pub fn chosen_pick(&self, character: &Character) -> Option<ChosenPick> {
        let kit = self.kit(character)?;
        self.rogue_level(character)?;
        let (count, bonus) = kit.chosen;
        let had = self.skills_of(character).kit_chosen;
        let names: Vec<String> = self
            .rogue_skills(character)
            .into_iter()
            .map(|s| s.name)
            .filter(|n| !had.iter().any(|c| &c.name == n))
            .collect();
        let count = (count.max(0) as usize).saturating_sub(had.len());
        (count > 0 && !names.is_empty()).then_some(ChosenPick {
            count,
            bonus,
            names,
        })
    }

    /// Chooses `names` besides the skills the character had chosen.
    pub fn set_chosen(&self, character: &Character, new: &mut CharacterSkills, names: &[String]) {
        let mut chosen = self.skills_of(character).kit_chosen;
        if let (Some(pick), Some((_, level))) =
            (self.chosen_pick(character), self.rogue_level(character))
        {
            chosen.extend(
                names
                    .iter()
                    .filter(|n| pick.names.contains(n))
                    .take(pick.count)
                    .map(|n| KitBonus {
                        name: n.clone(),
                        level: level as i32,
                        bonus: pick.bonus,
                    }),
            );
        }
        new.kit_chosen = chosen;
    }

    /// What the original would ask about before taking `new`'s rogue
    /// points: a skill over the limit for one level, the kit's bonus left
    /// out (62b489), or past 95%.
    pub fn rogue_warnings(&self, character: &Character, new: &CharacterSkills) -> Vec<String> {
        let limit = self.rogue_limit(character, new);
        let level = self.rogue_level(character).map_or(1, |l| l.1);
        let mut warnings = Vec::new();
        for (skill, &points) in self.rogue_skills(character).iter().zip(&new.rogue) {
            let name = &skill.label;
            let own = i64::from(points) - i64::from(Self::bonus_at(new, level, &skill.name));
            if limit.is_some_and(|l| own > i64::from(l)) {
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
            kit_bonuses: d
                .unknown_0x15e4
                .as_ref()
                .map(|l| {
                    l[0].iter()
                        .map(|o| o.as_deref().and_then(kit_bonus))
                        .collect()
                })
                .unwrap_or_default(),
            kit_chosen: d
                .unknown_0x15e4
                .as_ref()
                .map(|l| l[1].iter().flatten().filter_map(|o| kit_bonus(o)).collect())
                .unwrap_or_default(),
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
        let level = column.map_or(1, |c| c as u32 + 1);
        let d = &mut character.data;
        if let Some(column) = column {
            for (entry, &points) in d.rogue_skills.entries.iter_mut().zip(&new.rogue) {
                if let Some(value) = entry.1.get_mut(column) {
                    *value = points;
                }
                // The kit's bonuses newly given to earlier levels.
                for l in 1..level {
                    let at = l as usize - 1;
                    if old.kit_bonuses.get(at).is_none_or(Option::is_none) {
                        let bonus = Self::bonus_at(new, l, &entry.0);
                        entry.1[at] = entry.1[at].wrapping_add_signed(bonus);
                    }
                }
            }
        }
        if (new.kit_bonuses != old.kit_bonuses || new.kit_chosen != old.kit_chosen)
            && let Some(lists) = &mut d.unknown_0x15e4
        {
            lists[0] = new
                .kit_bonuses
                .iter()
                .map(|b| b.as_ref().map(kit_bonus_object))
                .collect();
            lists[1] = new
                .kit_chosen
                .iter()
                .map(|b| Some(kit_bonus_object(b)))
                .collect();
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
