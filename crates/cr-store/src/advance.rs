//! Experience, levels, hit points and spell slots, as the original's
//! Level... dialog (Expansion 0x618999, 2.0 0x43d920) and character sheet
//! compute them.
//!
//! Only experience, the hit point roll of each level gained, the PSPs of a
//! psionicist and, with Skills & Powers, the character points are stored;
//! levels, hit point totals and spell slots are worked out whenever they
//! are shown. Losing levels leaves the rolls in place, and the original
//! reuses them if the levels come back. Kits that scale experience or add
//! hit points (none of the shipped ones) are not taken into account.

use std::collections::{HashMap, HashSet};

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::classes::Class;
use cr_formats::parts::PartsFile;

use crate::create::Dice;
use crate::{Error, Installation, Result};

/// Spell levels the sheet shows, 1st to 9th.
pub const SPELL_LEVELS: usize = 9;

/// What advancement needs to know about a class.
#[derive(Debug, Clone)]
struct ClassAdvance {
    group: i32,
    prime: [bool; 6],
    /// Experience needed for each level, indexed by level.
    experience: Vec<i64>,
    hit_die: i32,
    /// The last level with a hit die; flat hit points after it.
    last_die_level: i32,
    flat: i32,
    /// Spell slots per class level and spell level (column 0 unused).
    slots: Vec<[i32; 11]>,
    /// The alternative table a class ability selects (rangers, bards).
    alternative_slots: Vec<[i32; 11]>,
    /// Uses the warriors' Constitution table.
    warrior_constitution: bool,
    /// Level-1 rolls of a multi-class character are not divided.
    keep_first_roll: bool,
    /// Doubles the Wisdom bonus spells.
    double_wisdom: bool,
    /// Casts priest spells, casts wizard spells.
    casts: (bool, bool),
}

impl ClassAdvance {
    fn of(class: &Class) -> Self {
        let table = |rows: &[[i32; 11]]| rows.to_vec();
        match class {
            Class::Expansion { data, .. } => Self {
                group: data.unknown_0x14[0],
                prime: data.unknown_0x34.map(|p| p != 0),
                experience: data.experience.iter().map(|&x| i64::from(x)).collect(),
                hit_die: data.unknown_0x84[0],
                last_die_level: data.unknown_0x84[1],
                flat: data.unknown_0x84[2],
                slots: table(&data.unknown_0x414),
                alternative_slots: table(&data.unknown_0x159c),
                warrior_constitution: data.flag_0x279c.unwrap_or(0) != 0,
                keep_first_roll: data.flags_0x4864.is_some_and(|f| f[0] != 0),
                double_wisdom: data.flag_0x2790.unwrap_or(0) != 0,
                casts: (data.unknown_0x2774[0] != 0, data.unknown_0x2774[1] != 0),
            },
            // The 2.0 layout keeps the same values in one array.
            Class::Core20(data) => {
                let v = &data.unknown_0x10;
                let slots = table(&data.unknown_0x264);
                let alternative_slots = table(&data.unknown_0x13ec);
                let any = |t: &[[i32; 11]]| t.iter().flatten().any(|&s| s > 0);
                let group = v[0];
                Self {
                    group,
                    prime: std::array::from_fn(|i| v[8 + i] != 0),
                    experience: data.experience.iter().map(|&x| i64::from(x)).collect(),
                    hit_die: v[17],
                    last_die_level: v[18],
                    flat: v[19],
                    casts: (
                        group == 2 || ((any(&slots) || any(&alternative_slots)) && group == 0),
                        group == 3 || ((any(&slots) || any(&alternative_slots)) && group == 1),
                    ),
                    slots,
                    alternative_slots,
                    warrior_constitution: false,
                    keep_first_roll: false,
                    double_wisdom: false,
                }
            }
        }
    }

    /// The level `xp` reaches: the first level whose next threshold is
    /// above it (0 experience is level 1).
    fn level(&self, xp: i64) -> u32 {
        (0..100)
            .find(|&i| self.experience.get(i + 1).is_some_and(|&t| t > xp))
            .unwrap_or(100) as u32
    }

    /// The highest level the experience table reaches.
    fn max_level(&self) -> u32 {
        self.experience
            .iter()
            .enumerate()
            .skip(2)
            .take_while(|(_, t)| **t > 0)
            .last()
            .map_or(1, |(i, _)| i as u32)
    }

    fn threshold(&self, level: u32) -> i64 {
        self.experience.get(level as usize).copied().unwrap_or(0)
    }
}

/// The databases' view of advancement, loaded once per installation.
#[derive(Debug, Default)]
pub struct AdvanceRules {
    classes: HashMap<String, ClassAdvance>,
    dual_races: HashSet<String>,
    /// Level limits from `UserDbas/AllowedU.dat` by race and class.
    limits: HashMap<(String, String), i32>,
    /// Class Abilities parts that select the alternative spell table, add
    /// a specialist slot, or give the warriors' Constitution table.
    alternative_slots: HashSet<String>,
    specialist: HashSet<String>,
    warrior_abilities: HashSet<String>,
    /// The Expansion (level limits, Wisdom bonus only with Skills &
    /// Powers) rather than 2.0.
    expansion: bool,
}

impl Installation {
    pub fn advance_rules(&self) -> Result<AdvanceRules> {
        let mut rules = AdvanceRules {
            expansion: self.path("UserDbas/AllowedU.dat").is_some(),
            ..Default::default()
        };
        for object in self.list_objects("Class")? {
            if let Object::Class(class) = object {
                rules
                    .classes
                    .entry(class.name().to_owned())
                    .or_insert_with(|| ClassAdvance::of(&class));
            }
        }
        for object in self.list_objects("Race")? {
            if let Object::Race(race) = object
                && race.unknown_0x1e0[2] != 0
            {
                rules.dual_races.insert(race.name.clone());
            }
        }
        for object in self.list_objects("Allowed")? {
            if let Object::ClassesAllowed(allowed) = object {
                for helper in &allowed.classes {
                    if let Object::ClassesAllowedHelper(h) = helper {
                        let d = &h.data;
                        let names: Vec<_> =
                            [&d.warrior, &d.rogue, &d.priest, &d.wizard, &d.psionicist]
                                .into_iter()
                                .filter(|n| !n.is_empty())
                                .collect();
                        if let ([name], Some(limit)) = (names.as_slice(), d.level_limit) {
                            rules
                                .limits
                                .entry((allowed.race.clone(), (*name).clone()))
                                .or_insert(limit);
                        }
                    }
                }
            }
        }
        for relative in ["Database/Parts.dat", "UserDbas/PartsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file = PartsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                path: relative.into(),
                source,
            })?;
            for object in file.of_type("Class Abilities").unwrap_or_default() {
                if let Object::Part(part) = object {
                    let name = part.head.name.clone();
                    if part.data.flags_0x780[4] != 0 {
                        rules.alternative_slots.insert(name.clone());
                    }
                    if part.data.unknown_0x7a4[2] != 0 {
                        rules.specialist.insert(name.clone());
                    }
                    if part.data.unknown_0x7a4[3] != 0 {
                        rules.warrior_abilities.insert(name);
                    }
                }
            }
        }
        Ok(rules)
    }
}

/// Str, Dex, Con, Int, Wis, Cha as the rules read them: the score, or with
/// Skills & Powers the subability each rule uses.
fn ability(character: &Character, ability: usize, subability: usize) -> i32 {
    let d = &character.data;
    let scores = if ability == 0 {
        d.strength
    } else {
        d.abilities[ability - 1]
    };
    if character.skills_and_powers() {
        scores[subability] as i32
    } else {
        scores[0] as i32
    }
}

const CON: usize = 2;
const INT: usize = 3;
const WIS: usize = 4;

fn constitution(character: &Character) -> i32 {
    // Fitness with Skills & Powers.
    ability(character, CON, 2)
}

/// Hit point adjustment for Constitution, warriors' or everyone else's.
pub fn constitution_adjustment(con: i32, warrior: bool) -> i32 {
    match con {
        ..=1 => -3,
        2..=3 => -2,
        4..=6 => -1,
        7..=14 => 0,
        15 => 1,
        16 => 2,
        _ if !warrior => 2,
        17 => 3,
        18 => 4,
        19..=20 => 5,
        21..=23 => 6,
        _ => 7,
    }
}

/// The lowest hit point roll Constitution allows.
fn constitution_minimum(con: i32) -> u32 {
    match con {
        20 => 2,
        21..=22 => 3,
        23.. => 4,
        _ => 0,
    }
}

/// Wisdom bonus priest spells for spell levels 1 to 7.
pub fn wisdom_bonus(wis: i32) -> [u32; 7] {
    let bonus: &[u32] = match wis {
        ..=12 => &[],
        13 => &[1],
        14 => &[2],
        15 => &[2, 1],
        16 => &[2, 2],
        17 => &[2, 2, 1],
        18 => &[2, 2, 1, 1],
        19 => &[3, 2, 2, 1],
        20 => &[3, 3, 2, 2],
        21 => &[3, 3, 3, 2, 1],
        22 => &[3, 3, 3, 3, 2],
        23 => &[4, 3, 3, 3, 2, 1],
        24 => &[4, 3, 3, 3, 3, 2],
        _ => &[4, 3, 3, 3, 3, 3, 1],
    };
    std::array::from_fn(|i| bonus.get(i).copied().unwrap_or(0))
}

/// One class of a character as advancement sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassLevel {
    pub name: String,
    pub experience: u32,
    pub level: u32,
    /// Experience for the next level, if there is one.
    pub next: Option<u32>,
    /// The level limit that holds the class back, if any.
    pub limit: Option<u32>,
}

impl AdvanceRules {
    fn class(&self, character: &Character, g: usize) -> Result<&ClassAdvance> {
        let name = &character.data.classes.names[g];
        self.classes
            .get(name)
            .ok_or_else(|| Error::Invalid(format!("{name} is not a class in the databases")))
    }

    pub fn dual_classed(&self, character: &Character) -> bool {
        character.data.classes.names.len() >= 2 && self.dual_races.contains(&character.data.race)
    }

    /// The level limit of class `g` (Expansion, when the character applies
    /// limits: `unknown_0x1dc4` 1, or 2 with prime requisite bonuses).
    fn limit(&self, character: &Character, g: usize, class: &ClassAdvance) -> Option<u32> {
        let mode = character.data.unknown_0x1dc4;
        if !self.expansion || mode == 0 {
            return None;
        }
        let name = &character.data.classes.names[g];
        let class_max = class.max_level() as i32;
        let mut limit = self
            .limits
            .get(&(character.data.race.clone(), name.clone()))
            .copied()
            .unwrap_or(0);
        if class_max < limit || limit < 1 {
            limit = class_max;
        }
        limit = limit.min(100);
        if mode == 2 {
            let bonus = (0..6)
                .filter(|&a| class.prime[a])
                .map(|a| ability(character, a, 0))
                .filter(|&s| s >= 14)
                .map(|s| match s {
                    14..=15 => 1,
                    16..=17 => 2,
                    18 => 3,
                    19 => 4,
                    _ => 5,
                })
                .min()
                .unwrap_or(0);
            limit = (limit + bonus).min(100);
        }
        Some(limit.max(1) as u32)
    }

    fn level_of(&self, character: &Character, g: usize) -> Result<u32> {
        let class = self.class(character, g)?;
        let level = class.level(i64::from(character.data.unknown_0x1618[g]));
        Ok(match self.limit(character, g, class) {
            Some(limit) => level.min(limit),
            None => level,
        })
    }

    /// Each class with its experience and level.
    pub fn levels(&self, character: &Character) -> Result<Vec<ClassLevel>> {
        let d = &character.data;
        (0..d.classes.names.len())
            .map(|g| {
                let class = self.class(character, g)?;
                let level = self.level_of(character, g)?;
                let limit = self.limit(character, g, class);
                let next = class.threshold(level + 1);
                Ok(ClassLevel {
                    name: d.classes.names[g].clone(),
                    experience: d.unknown_0x1618[g],
                    level,
                    next: (next > 0 && limit.is_none_or(|l| level < l)).then_some(next as u32),
                    limit,
                })
            })
            .collect()
    }

    /// Whether every prime requisite of class `g` is 16 or more, which
    /// earns 10% more experience.
    pub fn prime_bonus(&self, character: &Character, g: usize) -> Result<bool> {
        let class = self.class(character, g)?;
        Ok((0..6)
            .filter(|&a| class.prime[a])
            .all(|a| ability(character, a, 0) >= 16))
    }

    /// Adds `amount` experience (negative to take it away) to class `g`,
    /// with the prime requisite bonus when `prime_bonus` is set, rolling
    /// hit points and PSPs for each level gained and, with Skills &
    /// Powers, adding `points_per_level` character points per level.
    pub fn add_experience(
        &self,
        character: &mut Character,
        g: usize,
        amount: i64,
        prime_bonus: bool,
        points_per_level: u32,
        dice: &mut Dice,
    ) -> Result<()> {
        let mut amount = amount;
        if prime_bonus {
            let percent = if self.prime_bonus(character, g)? { 110 } else { 100 };
            // C integer division truncates toward zero, as here.
            amount = amount * percent / 100;
        }
        let old = self.level_of(character, g)?;
        let xp = (i64::from(character.data.unknown_0x1618[g]) + amount).clamp(0, u32::MAX.into());
        character.data.unknown_0x1618[g] = xp as u32;
        let new = self.level_of(character, g)?;
        for level in old + 1..=new {
            let roll = self.roll_hit_points(character, g, level, dice)?;
            character.data.unknown_0xc60[g][level as usize - 1] = roll;
            if self.class(character, g)?.group == 4 {
                character.data.unknown_0x1430[level as usize - 1] =
                    psionic_points(character, level, dice);
            }
        }
        if character.skills_and_powers() {
            let change = (i64::from(new) - i64::from(old)) * i64::from(points_per_level);
            let points = &mut character.data.unknown_0x1778[0];
            *points = (i64::from(*points) + change).max(0) as u32;
        }
        Ok(())
    }

    /// The hit point roll of `level` (2 or more) in class `g`.
    fn roll_hit_points(
        &self,
        character: &Character,
        g: usize,
        level: u32,
        dice: &mut Dice,
    ) -> Result<u32> {
        let class = self.class(character, g)?;
        let n = character.data.classes.names.len() as u32;
        if level as i32 > class.last_die_level {
            return Ok((class.flat.max(0) as u32 / n).max(1));
        }
        let mut roll = dice
            .roll(1, class.hit_die.max(1) as u32)
            .max(constitution_minimum(constitution(character)));
        if level > 1 && !self.dual_classed(character) {
            roll = (roll / n).max(1);
        }
        Ok(roll)
    }

    fn constitution_adjustment(&self, character: &Character, g: usize) -> Result<i32> {
        let class = self.class(character, g)?;
        let warrior_ability = character.data.class_slots[g]
            .abilities
            .iter()
            .any(|a| self.warrior_abilities.contains(a));
        let warrior = class.warrior_constitution
            || warrior_ability
            || if self.dual_classed(character) {
                class.group == 0
            } else {
                (0..character.data.classes.names.len())
                    .any(|c| self.class(character, c).is_ok_and(|k| k.group == 0))
            };
        Ok(constitution_adjustment(constitution(character), warrior))
    }

    /// Hit points, from the stored rolls (0x42e0ab).
    pub fn hit_points(&self, character: &Character) -> Result<i32> {
        let d = &character.data;
        let n = d.classes.names.len();
        if n == 0 {
            return Ok(0);
        }
        let levels = (0..n)
            .map(|g| self.level_of(character, g))
            .collect::<Result<Vec<_>>>()?;
        let roll = |g: usize, level: u32| d.unknown_0xc60[g][level as usize - 1] as i32;
        if self.dual_classed(character) {
            let mut total = 0;
            let mut counted = 0;
            for (g, &level) in levels.iter().enumerate() {
                let class = self.class(character, g)?;
                let adjustment = self.constitution_adjustment(character, g)?;
                for l in counted + 1..=level {
                    total += roll(g, l);
                    if l as i32 <= class.last_die_level {
                        total += adjustment;
                    }
                }
                counted = counted.max(level);
            }
            return Ok(total);
        }
        let adjustment = self.constitution_adjustment(character, 0)?;
        let mut first: i32 = (0..n).map(|g| roll(g, 1)).sum();
        let keep = (0..n).any(|g| self.class(character, g).is_ok_and(|c| c.keep_first_roll));
        if !keep {
            first /= n as i32;
        }
        let mut total = f64::from((first + adjustment).max(1));
        for (g, &level) in levels.iter().enumerate() {
            let class = self.class(character, g)?;
            for l in 2..=level {
                let mut t = f64::from(roll(g, l));
                if l as i32 <= class.last_die_level {
                    t += f64::from(adjustment) / n as f64;
                }
                total += t.max(1.0);
            }
        }
        Ok(total as i32)
    }

    /// Spell slots of class `g` for spell levels 1 to 9, with Wisdom bonus
    /// spells (Expansion: Skills & Powers only; 2.0: always) and the
    /// specialist's extra slot. Empty for a class that casts no spells.
    pub fn spell_slots(&self, character: &Character, g: usize) -> Result<Option<[u32; 9]>> {
        let class = self.class(character, g)?;
        if !class.casts.0 && !class.casts.1 {
            return Ok(None);
        }
        let level = self.level_of(character, g)? as usize;
        let abilities = &character.data.class_slots[g].abilities;
        let alternative = abilities
            .iter()
            .any(|a| self.alternative_slots.contains(a))
            || class.slots.iter().flatten().all(|&s| s <= 0);
        let table = if alternative {
            &class.alternative_slots
        } else {
            &class.slots
        };
        let row = table.get(level).copied().unwrap_or_default();
        let wisdom = if class.group == 2 && (!self.expansion || character.skills_and_powers()) {
            // Intuition with Skills & Powers.
            wisdom_bonus(ability(character, WIS, 1))
        } else {
            [0; 7]
        };
        let specialist = abilities.iter().any(|a| self.specialist.contains(a));
        let factor = if class.double_wisdom { 2 } else { 1 };
        Ok(Some(std::array::from_fn(|i| {
            let base = row[i + 1].max(0) as u32;
            if base == 0 {
                return 0;
            }
            let bonus = wisdom.get(i).copied().unwrap_or(0) * factor;
            base + bonus + u32::from(specialist)
        })))
    }
}

/// PSPs a psionicist gains at `level` (0x42e916).
fn psionic_points(character: &Character, level: u32, dice: &mut Dice) -> u32 {
    let bonus = |a: usize, s: usize| (ability(character, a, s) - 15).max(0) as u32;
    let bonuses = bonus(WIS, 1) + bonus(INT, 1) + bonus(CON, 2);
    match level {
        1 => 15 + dice.roll(1, 6) + bonuses,
        2..=8 => dice.roll(1, 6) + bonuses,
        _ => bonus(WIS, 1) + 3,
    }
}

/// The highest wizard spell level Intelligence allows.
pub fn max_spell_level(int: i32) -> u32 {
    match int {
        ..=8 => 0,
        9 => 4,
        10..=11 => 5,
        12..=13 => 6,
        14..=15 => 7,
        16..=17 => 8,
        _ => 9,
    }
}
