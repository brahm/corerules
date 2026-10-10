//! Making a new character, step by step as the Expansion's creation wizard
//! does: ability scores, race, class, the details page (name, alignment,
//! kit, age, height, weight, hit points), proficiencies and languages,
//! spells, equipment. Every choice comes from the rules databases, narrowed
//! by the campaign [`Setting`]; the file is written in the layout of the
//! program that owns the installation (`TSRP3 V54` for the Expansion,
//! `TSRP2 V29` for Core Rules 2.0) with the values the original's wizard
//! writes into a new character, and gets an entry in `Chars/index.chr`.
//!
//! What the databases do not hold comes from the Player's Handbook, as the
//! original's wizard has it built in: the rolling methods, starting money
//! per class group, the bonus languages and spells a wizard may know for
//! Intelligence, and the social class roll (2d6 on the kit's table). Core
//! Rules 2.0 has no `AllowedU.dat`, so there the classes a race may take
//! follow the race's level limits and multi-class list, with the
//! specialist wizards of the Player's Handbook table.
//!
//! The Expansion also makes Player's Option characters (Skills & Powers):
//! subabilities, character points spent on racial and class abilities,
//! nonweapon proficiencies, traits and weapon proficiencies, disadvantages
//! that give points back, and weapon specialization. The point pools
//! come from the databases (race and class records, part costs); the
//! allotments the book gives per class group (nonweapon proficiencies:
//! warriors and rogues 6, priests and wizards 8; weapon proficiencies 2
//! points a slot for warriors, 3 for the others) are built in, as in the
//! original. Rogue skill points are left to the original program.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use cr_formats::Object;
use cr_formats::character::{
    CORE20_VERSION, Character, CharacterEntry, CharacterIndex, EXPANSION_VERSION, EntryExtra,
    SheetEntries, SheetOptions,
};
use cr_formats::inventory::InventoryBundle;
use cr_formats::parts::{Part, PartsFile};
use cr_formats::spells::SpellsFile;

use crate::dual::ClassRules;
use crate::edit::{CLASS_LINE, LEVEL_LINE, PLAYER_LINE, replace, with_suffix};
use crate::lists::{CARRIED, CharacterLists, Item, PartTable, carried_weight};
use crate::settings::Setting;
use crate::{Error, Installation, Result};

pub const ABILITIES: [&str; 6] = [
    "Strength",
    "Dexterity",
    "Constitution",
    "Intelligence",
    "Wisdom",
    "Charisma",
];

/// `inventory` indexes of the readied and worn items.
const READIED: usize = 3;
const WORN: usize = 5;

/// What the original's wizard buys for every new character: a backpack
/// holding a week of rations, flint and steel, three torches and a
/// wineskin.
pub const STANDARD_PACK: (&str, [(&str, u32); 4]) = (
    "Backpack",
    [
        ("Dry rations (1 week)", 1),
        ("Flint and steel", 1),
        ("Torch", 3),
        ("Wineskin", 1),
    ],
);

/// The `CCharacterMiniOb` schema both programs write.
const INDEX_SCHEMA: u16 = 95;

/// Wizards get their spellbook for free.
pub const SPELLBOOK: &str = "Spellbook";

/// The native language the original gives a race whose record names none.
const HUMAN_LANGUAGE: &str = "Human, Regional dialect";

/// Dice for the rolls; a small xorshift generator, so that tests can fix
/// the seed.
#[derive(Debug, Clone)]
pub struct Dice(u64);

impl Default for Dice {
    fn default() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        Self::seeded(nanos ^ (u64::from(std::process::id()) << 32))
    }
}

impl Dice {
    pub fn seeded(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// `count` dice of `sides` sides, added up.
    pub fn roll(&mut self, count: u32, sides: u32) -> u32 {
        if sides == 0 {
            return 0;
        }
        (0..count)
            .map(|_| (self.next() % u64::from(sides)) as u32 + 1)
            .sum()
    }
}

/// The ways the Player's Handbook rolls ability scores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollMethod {
    /// 3d6 for each ability, in order.
    InOrder,
    /// 3d6 twice for each ability, keeping the better, in order.
    BestOfTwo,
    /// 3d6 six times, arranged as the player likes.
    Arrange,
    /// 3d6 twelve times, the best six arranged.
    BestSixOfTwelve,
    /// 4d6 dropping the lowest die, six times, arranged.
    FourDice,
    /// Scores typed in.
    Enter,
    /// Skills & Powers Method VII: 75 points divided among the abilities.
    Points,
    /// Skills & Powers Method VIII: 24d6 shared out, 3 to 6 dice an
    /// ability, the best three of each kept.
    DicePool,
    /// Skills & Powers Method IX: 2d6 on a table sets the points to divide
    /// and the highest score.
    RolledPoints,
}

/// Method VII's points.
pub const METHOD_VII_POINTS: u32 = 75;

/// Method VIII's dice.
pub const METHOD_VIII_DICE: u32 = 24;

impl RollMethod {
    pub const ALL: [RollMethod; 6] = [
        Self::InOrder,
        Self::BestOfTwo,
        Self::Arrange,
        Self::BestSixOfTwelve,
        Self::FourDice,
        Self::Enter,
    ];

    /// What the Skills & Powers wizard adds.
    pub const SKILLS_AND_POWERS: [RollMethod; 3] =
        [Self::Points, Self::DicePool, Self::RolledPoints];

    /// The methods offered for the rules chosen.
    pub fn offered(skills_and_powers: bool) -> Vec<RollMethod> {
        let mut methods = Self::ALL.to_vec();
        if skills_and_powers {
            methods.splice(5..5, Self::SKILLS_AND_POWERS);
        }
        methods
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::InOrder => "Method I",
            Self::BestOfTwo => "Method II",
            Self::Arrange => "Method III",
            Self::BestSixOfTwelve => "Method IV",
            Self::FourDice => "Method V",
            Self::Enter => "Enter Scores",
            Self::Points => "Method VII",
            Self::DicePool => "Method VIII",
            Self::RolledPoints => "Method IX",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::InOrder => "3d6 for each ability, in order",
            Self::BestOfTwo => "3d6 twice for each ability, the better in order",
            Self::Arrange => "3d6 six times, arranged as you like",
            Self::BestSixOfTwelve => "3d6 twelve times, the best six arranged as you like",
            Self::FourDice => "4d6 six times dropping the lowest die, arranged as you like",
            Self::Enter => "Type in scores rolled elsewhere",
            Self::Points => "75 points divided among the abilities, 3 to 18 each",
            Self::DicePool => "24d6 shared out, 3 to 6 dice an ability, the best three kept",
            Self::RolledPoints => "2d6 on a table give the points to divide and the highest score",
        }
    }

    /// Whether the player divides points among the abilities.
    pub fn divided(self) -> bool {
        matches!(self, Self::Points | Self::RolledPoints)
    }

    /// Whether the player arranges the rolled scores.
    pub fn arranged(self) -> bool {
        matches!(self, Self::Arrange | Self::BestSixOfTwelve | Self::FourDice)
    }

    /// Scores for Str, Dex, Con, Int, Wis and Cha; for the methods that
    /// arrange them, highest first.
    pub fn roll(self, dice: &mut Dice) -> [u32; 6] {
        let mut scores = match self {
            Self::InOrder | Self::Arrange => std::array::from_fn(|_| dice.roll(3, 6)),
            Self::BestOfTwo => std::array::from_fn(|_| dice.roll(3, 6).max(dice.roll(3, 6))),
            Self::BestSixOfTwelve => {
                let mut rolls: Vec<u32> = (0..12).map(|_| dice.roll(3, 6)).collect();
                rolls.sort_unstable_by(|a, b| b.cmp(a));
                std::array::from_fn(|i| rolls[i])
            }
            Self::FourDice => std::array::from_fn(|_| {
                let mut four: [u32; 4] = std::array::from_fn(|_| dice.roll(1, 6));
                four.sort_unstable();
                four[1..].iter().sum()
            }),
            Self::Enter | Self::Points | Self::RolledPoints => [13, 13, 13, 12, 12, 12],
            Self::DicePool => roll_dice_pool([4; 6], dice),
        };
        if self.arranged() {
            scores.sort_unstable_by(|a, b| b.cmp(a));
        }
        scores
    }
}

/// Method VIII: for each ability the number of d6 given to it, the best
/// three of them added up.
pub fn roll_dice_pool(counts: [u32; 6], dice: &mut Dice) -> [u32; 6] {
    std::array::from_fn(|a| {
        let mut rolled: Vec<u32> = (0..counts[a].max(3)).map(|_| dice.roll(1, 6)).collect();
        rolled.sort_unstable_by(|x, y| y.cmp(x));
        rolled[..3].iter().sum()
    })
}

/// Method IX: the points to divide and the highest score for a 2d6 roll
/// (Skills & Powers, Point Generation table).
pub fn point_table(roll: u32) -> (u32, u32) {
    match roll {
        0..=2 => (68, 18),
        3 => (70, 18),
        4 => (72, 17),
        5 => (72, 18),
        6 => (74, 17),
        7 => (74, 18),
        8 => (76, 17),
        9 => (76, 16),
        10 => (78, 16),
        11 => (78, 15),
        _ => (80, 15),
    }
}

/// The two Skills & Powers subabilities of each ability, in the order the
/// character file keeps them.
pub const SUBABILITIES: [[&str; 2]; 6] = [
    ["Stamina", "Muscle"],
    ["Aim", "Balance"],
    ["Health", "Fitness"],
    ["Reason", "Knowledge"],
    ["Intuition", "Willpower"],
    ["Leadership", "Appearance"],
];

/// How far a subability may be from its ability score.
pub const SUBABILITY_SPREAD: i32 = 2;

/// Character points for nonweapon proficiencies per class group, and what
/// a weapon proficiency slot costs (Skills & Powers, chapters 6 and 7).
fn group_points(group: i32) -> u32 {
    match group {
        2 | 3 => 8,
        _ => 6,
    }
}

/// The most points disadvantages may give, and how many may be taken.
pub const DISADVANTAGE_POINTS: u32 = 15;
pub const MOST_DISADVANTAGES: usize = 2;

/// Weapon specialization at 1st level: only fighters, 2 points for a
/// single-classed one and 4 for a multi-classed one.
const SPECIALIZATION_COSTS: (u32, u32) = (2, 4);

/// A race as creation needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaceRules {
    pub name: String,
    /// Minimum and maximum base score per ability.
    pub limits: [[i32; 2]; 6],
    pub adjustments: [i32; 6],
    level_limits: [i32; 9],
    /// Male and female base height, height dice, male and female base
    /// weight, weight dice, base age, age dice.
    physical: [i32; 11],
    pub movement: i32,
    languages: Vec<String>,
    /// One of the races the Expansion lists with "Standard Races Only".
    pub standard: bool,
    /// Subraces, the default one first.
    subraces: Vec<SubraceRules>,
    /// Multi-class combinations as class ids per group (Core Rules 2.0).
    multi_classes: Vec<[i32; 5]>,
    /// Skills & Powers: points for racial abilities, how many may be kept,
    /// and the abilities a character of the race starts with.
    points: u32,
    saved: u32,
    abilities: Vec<String>,
}

/// A subrace record: its movement, its Skills & Powers points, ability
/// package and languages.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SubraceRules {
    name: String,
    movement: i32,
    points: u32,
    saved: u32,
    abilities: Vec<String>,
    languages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KitRules {
    name: String,
    /// Class titles allowed ("Fighter/Thief"); empty for any.
    classes: Vec<String>,
    /// Races and subraces allowed; empty for any.
    races: Vec<String>,
    excluded_races: Vec<String>,
    /// Social class per 2d6 roll, indexed by the roll.
    social_classes: Vec<String>,
}

/// Something the equipment page sells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ware {
    pub name: String,
    /// "Equipment", "Weapons" or "Armor".
    pub part_type: &'static str,
    /// Price in copper pieces.
    pub cost: u32,
}

/// A nonweapon proficiency: its cost in slots (Core Rules) and character
/// points (Skills & Powers), its groups (general, priest, rogue, warrior,
/// wizard, psionicist) and the subabilities it needs ("Wisdom/Willpower").
#[derive(Debug, Clone, PartialEq, Eq)]
struct NonweaponRules {
    name: String,
    slots: u32,
    points: u32,
    groups: [bool; 6],
    relevant: Vec<String>,
}

/// Every choice made for a new character.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub setting: &'static Setting,
    /// Player's Option (Skills & Powers) rather than the Core Rules.
    pub skills_and_powers: bool,
    /// Str, Dex, Con, Int, Wis and Cha before the racial adjustments.
    pub scores: [u32; 6],
    /// 1 to 100 for 18/01 to 18/00; only warriors with Strength 18.
    pub exceptional_strength: u32,
    pub race: String,
    pub subrace: String,
    pub classes: Vec<String>,
    pub name: String,
    pub player: String,
    pub male: bool,
    pub alignment: String,
    pub kit: String,
    pub social_class: String,
    pub age: u32,
    pub height: u32,
    pub weight: u32,
    /// The hit die rolled for each class.
    pub hit_dice: Vec<u32>,
    pub nonweapon_proficiencies: Vec<String>,
    /// Native languages.
    pub languages: Vec<String>,
    pub weapon_proficiencies: Vec<String>,
    /// Wizard spells known.
    pub spells: Vec<String>,
    /// Starting money, in gold pieces.
    pub gold: u32,
    /// Whether the standard pack is bought.
    pub pack: bool,
    /// Items bought, with their quantities.
    pub purchases: Vec<(String, u32)>,
    /// Skills & Powers: how far each ability's first subability is above
    /// its score; the second is as far below.
    pub subabilities: [i32; 6],
    pub racial_abilities: Vec<String>,
    /// Class abilities, per class.
    pub class_abilities: Vec<Vec<String>>,
    pub traits: Vec<String>,
    pub disadvantages: Vec<String>,
    /// Weapons the character specializes in.
    pub specializations: Vec<String>,
}

impl Draft {
    pub fn new(setting: &'static Setting) -> Self {
        Self {
            setting,
            scores: [10; 6],
            exceptional_strength: 0,
            race: String::new(),
            subrace: String::new(),
            classes: Vec::new(),
            name: String::new(),
            player: String::new(),
            male: true,
            alignment: String::new(),
            kit: "None".into(),
            social_class: String::new(),
            age: 0,
            height: 0,
            weight: 0,
            hit_dice: Vec::new(),
            nonweapon_proficiencies: Vec::new(),
            languages: Vec::new(),
            weapon_proficiencies: Vec::new(),
            spells: Vec::new(),
            gold: 0,
            pack: true,
            purchases: Vec::new(),
            skills_and_powers: false,
            subabilities: [0; 6],
            racial_abilities: Vec::new(),
            class_abilities: Vec::new(),
            traits: Vec::new(),
            disadvantages: Vec::new(),
            specializations: Vec::new(),
        }
    }

    pub fn class_title(&self) -> String {
        self.classes.join("/")
    }
}

/// What the databases say about making a character, loaded once.
#[derive(Debug)]
pub struct CreationRules {
    /// Whether the installation is the Expansion's (else Core Rules 2.0).
    pub expansion: bool,
    races: Vec<RaceRules>,
    classes: Vec<ClassRules>,
    /// Class combinations per race (`AllowedU.dat`, the Expansion only).
    combinations: HashMap<String, Vec<Vec<String>>>,
    kits: Vec<KitRules>,
    nonweapon: Vec<NonweaponRules>,
    /// Skills & Powers parts with their cost in character points: racial
    /// and class abilities, traits, and the points disadvantages give.
    racial_abilities: Vec<(String, i32)>,
    class_ability_costs: HashMap<String, i32>,
    traits: Vec<(String, u32)>,
    disadvantages: Vec<(String, u32)>,
    /// Weapons and the classes that may not use them.
    weapons: Vec<(String, Vec<String>)>,
    /// Wizard spells: name, level, schools.
    wizard_spells: Vec<(String, i32, Vec<String>)>,
    pub wares: Vec<Ware>,
    abilities: HashSet<String>,
    parts: PartTable,
}

impl Installation {
    pub fn creation_rules(&self) -> Result<CreationRules> {
        let mut classes: Vec<ClassRules> = Vec::new();
        let mut expansion = false;
        for object in self.list_objects("Class")? {
            if let Object::Class(class) = object {
                expansion |= matches!(*class, cr_formats::classes::Class::Expansion { .. });
                if !classes.iter().any(|c| c.name == class.name()) {
                    classes.push(ClassRules::of(&class));
                }
            }
        }

        let mut records = Vec::new();
        for object in self.list_objects("Race")? {
            if let Object::Race(race) = object
                && !records
                    .iter()
                    .any(|r: &cr_formats::race::Race| r.name == race.name)
            {
                records.push(*race);
            }
        }
        // A main race has ability limits; a subrace has none and names its
        // race ("Hill dwarf", "Dark (drow) elf", "Half-Stout Halfling").
        let is_main = |r: &cr_formats::race::Race| r.ability_limits.iter().any(|l| l[1] > 0);
        let mut races: Vec<RaceRules> = records
            .iter()
            .filter(|r| is_main(r))
            .map(|r| RaceRules {
                name: r.name.clone(),
                limits: r.ability_limits,
                adjustments: r.ability_adjustments,
                level_limits: r.level_limits,
                physical: r.physical,
                movement: r.movement,
                languages: r.languages.clone(),
                standard: r.unknown_0x1e0[1] != 0,
                subraces: Vec::new(),
                points: r.unknown_0xa8.max(0) as u32,
                saved: r.unknown_0xac.max(0) as u32,
                abilities: r.abilities.clone(),
                multi_classes: r
                    .multi_classes
                    .iter()
                    .filter_map(|m| match m {
                        Object::MultiClass(m) => Some(m.classes),
                        _ => None,
                    })
                    .collect(),
            })
            .collect();
        for sub in records.iter().filter(|r| !is_main(r)) {
            let words: Vec<String> = sub
                .name
                .split_whitespace()
                .map(|w| w.trim_matches(['(', ')']).to_lowercase())
                .collect();
            if let Some(race) = races
                .iter_mut()
                .find(|r| words.contains(&r.name.to_lowercase()))
            {
                let entry = SubraceRules {
                    name: sub.name.clone(),
                    movement: sub.movement,
                    points: sub.unknown_0xa8.max(0) as u32,
                    saved: sub.unknown_0xac.max(0) as u32,
                    abilities: sub.abilities.clone(),
                    languages: sub.languages.clone(),
                };
                if sub.unknown_0x1e0[0] != 0 {
                    race.subraces.insert(0, entry);
                } else {
                    race.subraces.push(entry);
                }
            }
        }

        let mut combinations: HashMap<String, Vec<Vec<String>>> = HashMap::new();
        for object in self.list_objects("Allowed")? {
            if let Object::ClassesAllowed(allowed) = object {
                let list = combinations.entry(allowed.race.clone()).or_default();
                for helper in &allowed.classes {
                    let names = helper_classes(helper);
                    if !names.is_empty() && !list.contains(&names) {
                        list.push(names);
                    }
                }
            }
        }

        let mut rules = CreationRules {
            expansion,
            races,
            classes,
            combinations,
            kits: Vec::new(),
            nonweapon: Vec::new(),
            racial_abilities: Vec::new(),
            class_ability_costs: HashMap::new(),
            traits: Vec::new(),
            disadvantages: Vec::new(),
            weapons: Vec::new(),
            wizard_spells: Vec::new(),
            wares: Vec::new(),
            abilities: self.part_names("Class Abilities")?.into_iter().collect(),
            parts: self.part_table()?,
        };
        let mut seen = HashSet::new();
        for relative in ["Database/Parts.dat", "UserDbas/PartsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file = PartsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                path: relative.into(),
                source,
            })?;
            let parts = |part_type: &str| {
                file.of_type(part_type)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|o| match o {
                        Object::Part(p) if !p.head.name.is_empty() => Some(&**p),
                        _ => None,
                    })
                    .collect::<Vec<&Part>>()
            };
            for part in parts("Character Kits") {
                if !seen.insert(("kit", part.head.name.clone())) {
                    continue;
                }
                let Some(Some(kit)) = &part.data.kit else {
                    continue;
                };
                let Object::PartKit(kit) = &**kit else {
                    continue;
                };
                let Some(data) = &kit.data else { continue };
                rules.kits.push(KitRules {
                    name: part.head.name.clone(),
                    classes: data
                        .allowed_classes
                        .iter()
                        .flatten()
                        .map(|h| helper_classes(h).join("/"))
                        .filter(|t| !t.is_empty())
                        .collect(),
                    races: data.unknown_0x10.clone(),
                    excluded_races: data.unknown_0xa8.clone(),
                    social_classes: data.unknown_0x10c.clone(),
                });
            }
            for part in parts("Non-Weapon Proficiencies") {
                if seen.insert(("nwp", part.head.name.clone())) {
                    let name = &part.head.name;
                    rules.nonweapon.push(NonweaponRules {
                        name: name.clone(),
                        slots: rules.parts.nonweapon_cost(name, false),
                        points: rules.parts.nonweapon_cost(name, true),
                        groups: part.data.unknown_0x48c.map(|g| g != 0),
                        relevant: [&part.head.unknown_0x458, &part.head.unknown_0x45c]
                            .into_iter()
                            .filter(|r| !r.is_empty())
                            .cloned()
                            .collect(),
                    });
                }
            }
            let cost = |part: &Part| match part.head.unknown_0x308 {
                0 => cost_in_name(&part.head.name),
                cost => cost,
            };
            for part in parts("Racial Abilities") {
                if seen.insert(("racial", part.head.name.clone())) {
                    rules
                        .racial_abilities
                        .push((part.head.name.clone(), cost(part)));
                }
            }
            for part in parts("Class Abilities") {
                rules
                    .class_ability_costs
                    .entry(part.head.name.clone())
                    .or_insert_with(|| cost(part));
            }
            for part in parts("Traits") {
                if seen.insert(("trait", part.head.name.clone())) {
                    rules
                        .traits
                        .push((part.head.name.clone(), cost(part).max(0) as u32));
                }
            }
            for part in parts("Disadvantages") {
                if seen.insert(("disadvantage", part.head.name.clone())) {
                    rules.disadvantages.push((
                        part.head.name.clone(),
                        part.data.unknown_0x220.max(0) as u32,
                    ));
                }
            }
            for part_type in ["Equipment", "Weapons", "Armor"] {
                for part in parts(part_type) {
                    // Magical items are not for sale.
                    if part.head.unknown_0x224 != 0
                        || !seen.insert(("ware", part.head.name.clone()))
                    {
                        continue;
                    }
                    if part_type == "Weapons" {
                        rules.weapons.push((
                            part.head.name.clone(),
                            part.data.allowed_classes.clone().unwrap_or_default(),
                        ));
                    }
                    let price = &part.head.unknown_0x1f8;
                    let cost = [500, 100, 10, 1]
                        .iter()
                        .zip(price)
                        .map(|(worth, n)| worth * n.max(&0))
                        .sum::<i32>() as u32;
                    rules.wares.push(Ware {
                        name: part.head.name.clone(),
                        part_type,
                        cost,
                    });
                }
            }
        }
        for relative in ["Database/Spells.dat", "UserDbas/SpellsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file =
                SpellsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                    path: relative.into(),
                    source,
                })?;
            for o in &file.wizard {
                if let Object::Spell(s) = o
                    && !s.name.is_empty()
                    && !rules.wizard_spells.iter().any(|w| w.0 == s.name)
                {
                    rules
                        .wizard_spells
                        .push((s.name.clone(), s.level, s.schools.clone()));
                }
            }
        }
        rules.nonweapon.sort_by_key(|n| n.name.to_lowercase());
        rules.racial_abilities.sort_by_key(|n| n.0.to_lowercase());
        rules.traits.sort_by_key(|n| n.0.to_lowercase());
        rules.disadvantages.sort_by_key(|n| n.0.to_lowercase());
        rules.weapons.sort_by_key(|n| n.0.to_lowercase());
        rules.wares.sort_by_key(|n| n.name.to_lowercase());
        rules.races.sort_by_key(|r| r.name.to_lowercase());
        Ok(rules)
    }

    /// Makes the character `draft` describes and adds it to the character
    /// list; returns its file name inside `Chars/`.
    pub fn create_character(&self, rules: &CreationRules, draft: &Draft) -> Result<String> {
        if let Some(problem) = rules.problem(draft) {
            return Err(Error::Invalid(problem));
        }
        let character = rules.build(draft)?;
        let format = |path: &str| {
            let path = path.to_owned();
            move |source| Error::Format {
                path: path.into(),
                source,
            }
        };
        let bytes = character.to_bytes().map_err(format("new character"))?;
        Character::parse(&bytes).map_err(format("new character"))?;

        let chars = match self.path("Chars") {
            Some(dir) => dir,
            None => {
                let dir = self.root().join("Chars");
                fs::create_dir_all(&dir).map_err(|source| Error::Io {
                    path: dir.clone(),
                    source,
                })?;
                dir
            }
        };
        let mut index = self.characters()?;
        if index.schema == 0 {
            // An empty list read back has no class tag to take it from.
            index.schema = INDEX_SCHEMA;
        }
        let file = new_file_name(&chars, &index);
        index.entries.push(rules.entry(draft, &character, &file));
        let index_bytes = index.to_bytes().map_err(format("Chars/index.chr"))?;
        CharacterIndex::parse(&index_bytes).map_err(format("Chars/index.chr"))?;

        write_new(&chars.join(&file), &bytes)?;
        match self.path("Chars/index.chr") {
            Some(path) => replace(&path, &index_bytes)?,
            None => write_new(&chars.join("index.chr"), &index_bytes)?,
        }
        Ok(file)
    }
}

/// The score a proficiency's relevant ability names: a subability
/// ("Wisdom/Willpower") or a whole ability ("Intelligence").
fn subability_score(scores: &[[u32; 3]; 6], relevant: &str) -> Option<u32> {
    let (ability, sub) = relevant.split_once('/').unwrap_or((relevant, ""));
    let a = ABILITIES
        .iter()
        .position(|n| n.eq_ignore_ascii_case(ability.trim()))?;
    let s = SUBABILITIES[a]
        .iter()
        .position(|n| n.eq_ignore_ascii_case(sub.trim()))
        .map_or(0, |s| s + 1);
    Some(scores[a][s])
}

/// The cost a racial or class ability's name ends with: 5 for "Dwarf, Axe
/// bonus (5)", -5 for "Warrior, Limited armor (-5)", 0 without one.
fn cost_in_name(name: &str) -> i32 {
    name.rsplit_once('(')
        .and_then(|(_, rest)| rest.strip_suffix(')'))
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// The class names of a `CClassesAllowedHelperOb`, in group order.
fn helper_classes(object: &Object) -> Vec<String> {
    match object {
        Object::ClassesAllowedHelper(h) => {
            let d = &h.data;
            [&d.warrior, &d.rogue, &d.priest, &d.wizard, &d.psionicist]
                .into_iter()
                .filter(|n| !n.is_empty())
                .cloned()
                .collect()
        }
        _ => Vec::new(),
    }
}

/// An eight-digit name like the original's (`92385619.dat`) that no
/// character uses yet.
fn new_file_name(chars: &Path, index: &CharacterIndex) -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let mut n = seconds % 90_000_000 + 10_000_000;
    loop {
        let name = format!("{n}.dat");
        let taken = index
            .entries
            .iter()
            .any(|e| e.file.eq_ignore_ascii_case(&name))
            || fs::read_dir(chars).is_ok_and(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(&name))
            });
        if !taken {
            return name;
        }
        n = if n >= 99_999_999 { 10_000_000 } else { n + 1 };
    }
}

/// Writes a file that does not exist yet, through a temporary name.
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = with_suffix(path, ".tmp");
    fs::write(&temporary, bytes).map_err(|source| Error::Io {
        path: temporary.clone(),
        source,
    })?;
    fs::rename(&temporary, path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Languages a character knows for Intelligence (Player's Handbook,
/// Table 4); they add to the nonweapon proficiency slots.
pub fn bonus_languages(intelligence: u32) -> u32 {
    match intelligence {
        0 | 1 => 0,
        2..=8 => 1,
        9..=11 => 2,
        12 | 13 => 3,
        14 | 15 => 4,
        16 => 5,
        17 => 6,
        18 => 7,
        19 => 8,
        20 => 9,
        21 => 10,
        22 => 11,
        23 => 12,
        24 => 15,
        _ => 20,
    }
}

/// The most spells of one level a wizard can know for Intelligence (Player's
/// Handbook, Table 4).
pub fn max_spells_per_level(intelligence: u32) -> usize {
    match intelligence {
        0..=8 => 0,
        9 => 6,
        10..=12 => 7,
        13 | 14 => 9,
        15 | 16 => 11,
        17 => 14,
        18 => 18,
        _ => usize::MAX,
    }
}

/// Starting money per class group in gold pieces (Player's Handbook, Table
/// 43): dice, sides, added, multiplier.
fn starting_money(group: i32) -> (u32, u32, u32, u32) {
    match group {
        0 => (5, 4, 0, 10),
        1 => (2, 6, 0, 10),
        2 => (3, 6, 0, 10),
        3 => (1, 4, 1, 10),
        _ => (3, 4, 0, 10),
    }
}

/// The specialist wizards a demihuman may be (Player's Handbook, Table
/// 22); only for Core Rules 2.0, whose databases do not say.
fn handbook_specialists(race: &str) -> &'static [&'static str] {
    match race {
        "Elf" => &["Diviner", "Enchanter"],
        "Half-elf" => &["Conjurer", "Diviner", "Enchanter", "Transmuter"],
        "Gnome" => &["Illusionist"],
        _ => &[],
    }
}

impl CreationRules {
    pub fn race(&self, name: &str) -> Option<&RaceRules> {
        self.races.iter().find(|r| r.name == name)
    }

    pub fn class(&self, name: &str) -> Option<&ClassRules> {
        self.classes.iter().find(|c| c.name == name)
    }

    /// The races the setting has whose limits the base scores meet.
    pub fn races(&self, setting: &Setting, scores: &[u32; 6], standard_only: bool) -> Vec<String> {
        self.races
            .iter()
            .filter(|r| setting.allows_race(&r.name) && (!standard_only || r.standard))
            .filter(|r| {
                r.limits
                    .iter()
                    .zip(scores)
                    .all(|(l, &s)| (l[0]..=l[1]).contains(&(s as i32)))
            })
            .filter(|r| !self.combinations_of(setting, &r.name).is_empty())
            .map(|r| r.name.clone())
            .collect()
    }

    /// Why a race the setting has is not offered for these scores.
    pub fn race_problem(&self, race: &str, scores: &[u32; 6]) -> Option<String> {
        let r = self.race(race)?;
        (0..6).find_map(|a| {
            let [low, high] = r.limits[a];
            let s = scores[a] as i32;
            (s < low || s > high).then(|| format!("{race} needs {} {low} to {high}", ABILITIES[a]))
        })
    }

    /// The subraces the setting has, the default first; the race itself
    /// when it has none.
    pub fn subraces(&self, setting: &Setting, race: &str) -> Vec<String> {
        let Some(r) = self.race(race) else {
            return Vec::new();
        };
        let names: Vec<String> = r
            .subraces
            .iter()
            .enumerate()
            .filter(|(i, s)| setting.allows_subrace(&s.name, *i == 0))
            .map(|(_, s)| s.name.clone())
            .collect();
        if names.is_empty() {
            vec![race.to_owned()]
        } else {
            names
        }
    }

    /// Scores after the race's adjustments.
    pub fn adjusted(&self, race: &str, scores: &[u32; 6]) -> [u32; 6] {
        let adjustments = self.race(race).map_or([0; 6], |r| r.adjustments);
        std::array::from_fn(|a| (scores[a] as i32 + adjustments[a]).clamp(3, 25) as u32)
    }

    /// Every class combination the race may take in the setting.
    fn combinations_of(&self, setting: &Setting, race: &str) -> Vec<Vec<String>> {
        let all = match self.combinations.get(race) {
            Some(list) => list.clone(),
            None => self.core20_combinations(race),
        };
        all.into_iter()
            .filter(|combination| {
                combination.iter().all(|name| {
                    self.class(name)
                        .is_some_and(|c| setting.allows_class(name, c.group == 3))
                })
            })
            .collect()
    }

    /// Core Rules 2.0: single classes by level limit, then the race's
    /// multi-class combinations.
    fn core20_combinations(&self, race: &str) -> Vec<Vec<String>> {
        let Some(r) = self.race(race) else {
            return Vec::new();
        };
        const SLOTS: [&str; 8] = [
            "Fighter", "Ranger", "Paladin", "Thief", "Bard", "Cleric", "Druid", "Mage",
        ];
        let unlimited = r.level_limits.iter().all(|&l| l == 0);
        let mut singles: Vec<Vec<String>> = Vec::new();
        for class in &self.classes {
            let slot = SLOTS.iter().position(|s| *s == class.name);
            let allowed = unlimited
                || class.group == 4
                || slot.is_some_and(|s| r.level_limits[s] > 0)
                || (race == "Half-elf" && class.name == "Bard")
                || handbook_specialists(race).contains(&class.name.as_str());
            if allowed {
                singles.push(vec![class.name.clone()]);
            }
        }
        for ids in &r.multi_classes {
            let names: Option<Vec<String>> = ids
                .iter()
                .filter(|&&id| id != 0)
                .map(|&id| {
                    self.classes
                        .iter()
                        .find(|c| c.id == id)
                        .map(|c| c.name.clone())
                })
                .collect();
            if let Some(names) = names
                && names.len() > 1
            {
                singles.push(names);
            }
        }
        singles
    }

    /// The class combinations the race may take in the setting with these
    /// (adjusted) scores, as the race page lists them.
    pub fn class_options(
        &self,
        setting: &Setting,
        race: &str,
        adjusted: &[u32; 6],
    ) -> Vec<Vec<String>> {
        let mut options: Vec<Vec<String>> = self
            .combinations_of(setting, race)
            .into_iter()
            .filter(|combination| {
                combination.iter().all(|name| {
                    self.class(name)
                        .is_some_and(|c| (0..6).all(|a| adjusted[a] as i32 >= c.requirements[a]))
                }) && !self.alignments(combination).is_empty()
            })
            .collect();
        options.sort_by_key(|c| (c.len(), c.join("/").to_lowercase()));
        options
    }

    /// Alignments every one of the classes allows, in the first class's
    /// order.
    pub fn alignments(&self, classes: &[String]) -> Vec<String> {
        let mut lists = classes.iter().filter_map(|c| self.class(c));
        let Some(first) = lists.next() else {
            return Vec::new();
        };
        let rest: Vec<&ClassRules> = lists.collect();
        first
            .alignments
            .iter()
            .filter(|a| rest.iter().all(|c| c.alignments.contains(a)))
            .cloned()
            .collect()
    }

    /// Kits for the race (or subrace) and classes, "None" first.
    pub fn kits(&self, race: &str, subrace: &str, classes: &[String]) -> Vec<String> {
        let title = classes.join("/");
        let mut kits: Vec<String> = self
            .kits
            .iter()
            .filter(|k| k.name != "None")
            .filter(|k| k.classes.is_empty() || k.classes.contains(&title))
            .filter(|k| k.races.is_empty() || k.races.iter().any(|r| r == race || r == subrace))
            .filter(|k| !k.excluded_races.iter().any(|r| r == race))
            .map(|k| k.name.clone())
            .collect();
        kits.sort_by_key(|k| k.to_lowercase());
        kits.insert(0, "None".into());
        kits
    }

    /// 2d6 on the kit's social class table (the "None" kit's for no kit).
    pub fn roll_social_class(&self, kit: &str, dice: &mut Dice) -> String {
        let table = self
            .kits
            .iter()
            .find(|k| k.name == kit && k.social_classes.len() >= 13)
            .or_else(|| self.kits.iter().find(|k| k.name == "None"));
        let roll = dice.roll(2, 6) as usize;
        table
            .and_then(|k| k.social_classes.get(roll))
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| {
                match roll {
                    0..=3 => "Lower class",
                    4..=7 => "Lower middle class",
                    8..=10 => "Upper middle class",
                    _ => "Upper class",
                }
                .into()
            })
    }

    fn physical(&self, race: &str) -> [i32; 11] {
        self.race(race).map_or([0; 11], |r| r.physical)
    }

    pub fn roll_age(&self, race: &str, dice: &mut Dice) -> u32 {
        let p = self.physical(race);
        (p[8].max(0) as u32) + dice.roll(p[9].max(0) as u32, p[10].max(0) as u32)
    }

    pub fn roll_height(&self, race: &str, male: bool, dice: &mut Dice) -> u32 {
        let p = self.physical(race);
        let base = if male { p[0] } else { p[1] };
        base.max(0) as u32 + dice.roll(p[2].max(0) as u32, p[3].max(0) as u32)
    }

    pub fn roll_weight(&self, race: &str, male: bool, dice: &mut Dice) -> u32 {
        let p = self.physical(race);
        let base = if male { p[4] } else { p[5] };
        base.max(0) as u32 + dice.roll(p[6].max(0) as u32, p[7].max(0) as u32)
    }

    /// The hit die of each class, rolled.
    pub fn roll_hit_dice(&self, classes: &[String], dice: &mut Dice) -> Vec<u32> {
        classes
            .iter()
            .map(|c| dice.roll(1, self.class(c).map_or(0, |c| c.hit_die.max(0) as u32)))
            .collect()
    }

    /// Exceptional strength applies to warriors with Strength 18.
    pub fn has_exceptional_strength(&self, classes: &[String], strength: u32) -> bool {
        strength == 18
            && classes
                .iter()
                .any(|c| self.class(c).is_some_and(|c| c.group == 0))
    }

    /// Starting money in gold pieces, by the most generous of the classes'
    /// groups.
    pub fn roll_gold(&self, classes: &[String], dice: &mut Dice) -> u32 {
        let best = classes
            .iter()
            .filter_map(|c| self.class(c))
            .map(|c| starting_money(c.group))
            .max_by_key(|&(n, sides, add, times)| (n * (sides + 1) + 2 * add) * times)
            .unwrap_or((0, 0, 0, 0));
        let (n, sides, add, times) = best;
        (dice.roll(n, sides) + add) * times
    }

    /// Nonweapon proficiency slots: the best of the classes, plus the
    /// languages Intelligence gives.
    pub fn nonweapon_slots(&self, classes: &[String], intelligence: u32) -> u32 {
        let class = classes
            .iter()
            .filter_map(|c| self.class(c))
            .map(|c| c.nonweapon_slots.max(0) as u32)
            .max()
            .unwrap_or(0);
        class + bonus_languages(intelligence)
    }

    pub fn weapon_slots(&self, classes: &[String]) -> u32 {
        classes
            .iter()
            .filter_map(|c| self.class(c))
            .map(|c| c.weapon_slots.max(0) as u32)
            .max()
            .unwrap_or(0)
    }

    /// The nonweapon proficiencies, by name.
    pub fn nonweapon_proficiencies(&self) -> Vec<String> {
        self.nonweapon.iter().map(|n| n.name.clone()).collect()
    }

    /// What a nonweapon proficiency costs the drafted character: slots with
    /// the Core Rules; with Skills & Powers its character points, 2 more
    /// when it belongs to none of the character's groups (nor the general
    /// one), and 1 more for each point a relevant subability is below 9.
    pub fn nonweapon_cost(&self, draft: &Draft, name: &str) -> u32 {
        let Some(n) = self.nonweapon.iter().find(|n| n.name == name) else {
            return 1;
        };
        if !draft.skills_and_powers {
            return n.slots;
        }
        let groups = self.proficiency_groups(&draft.classes);
        let foreign = !n.groups.iter().zip(groups).any(|(&has, own)| has && own);
        let subabilities = self.subability_scores(draft);
        let shortfall: u32 = n
            .relevant
            .iter()
            .filter_map(|r| subability_score(&subabilities, r))
            .map(|score| 9u32.saturating_sub(score))
            .sum();
        n.points + if foreign { 2 } else { 0 } + shortfall
    }

    /// The nonweapon proficiency groups open to the classes at the normal
    /// cost: general, and the classes' own.
    fn proficiency_groups(&self, classes: &[String]) -> [bool; 6] {
        let mut groups = [true, false, false, false, false, false];
        for class in classes.iter().filter_map(|c| self.class(c)) {
            let index = match class.group {
                0 => 3,
                1 => 2,
                2 => 1,
                3 => 4,
                _ => 5,
            };
            groups[index] = true;
        }
        groups
    }

    /// Slots (Core Rules) or character points (Skills & Powers) the
    /// draft's nonweapon proficiencies take.
    pub fn nonweapon_spent(&self, draft: &Draft) -> u32 {
        draft
            .nonweapon_proficiencies
            .iter()
            .map(|n| self.nonweapon_cost(draft, n))
            .sum()
    }

    /// Each ability's score and its two subabilities, after the racial
    /// adjustments; with the Core Rules the subabilities equal the score.
    pub fn subability_scores(&self, draft: &Draft) -> [[u32; 3]; 6] {
        let adjusted = self.adjusted(&draft.race, &draft.scores);
        std::array::from_fn(|a| {
            let score = adjusted[a] as i32;
            let spread = if draft.skills_and_powers {
                draft.subabilities[a].clamp(-SUBABILITY_SPREAD, SUBABILITY_SPREAD)
            } else {
                0
            };
            let sub = |s: i32| (s.clamp(1, 25)) as u32;
            [score as u32, sub(score + spread), sub(score - spread)]
        })
    }

    /// The racial abilities a character of the subrace starts with: its
    /// package, or a monster race's fixed abilities. Both programs write
    /// them for Core Rules characters as well.
    pub fn racial_package(&self, race: &str, subrace: &str) -> Vec<String> {
        let Some(r) = self.race(race) else {
            return Vec::new();
        };
        r.subraces
            .iter()
            .find(|s| s.name == subrace)
            .map_or_else(|| r.abilities.clone(), |s| s.abilities.clone())
    }

    /// Skills & Powers points for racial abilities, and how many of those
    /// left over the character may keep.
    pub fn racial_points(&self, race: &str, subrace: &str) -> (u32, u32) {
        let Some(r) = self.race(race) else {
            return (0, 0);
        };
        r.subraces
            .iter()
            .find(|s| s.name == subrace)
            .map_or((r.points, r.saved), |s| (s.points, s.saved))
    }

    /// The racial abilities a Skills & Powers character of the subrace
    /// may have, with their cost: the race's own ("Dwarf, Axe bonus (5)")
    /// and the subrace's package.
    pub fn racial_ability_choices(&self, race: &str, subrace: &str) -> Vec<(String, i32)> {
        let package = self.racial_package(race, subrace);
        let (points, _) = self.racial_points(race, subrace);
        let prefix = format!("{}, ", race.to_lowercase());
        self.racial_abilities
            .iter()
            .filter(|(name, _)| {
                package.contains(name)
                    || (points > 0
                        && !name.contains('[')
                        && name.to_lowercase().starts_with(&prefix))
            })
            .cloned()
            .collect()
    }

    pub fn racial_ability_cost(&self, name: &str) -> i32 {
        self.racial_abilities
            .iter()
            .find(|r| r.0 == name)
            .map_or_else(|| cost_in_name(name), |r| r.1)
    }

    /// Racial points not spent; negative when overspent.
    pub fn racial_points_left(&self, draft: &Draft) -> i64 {
        let (points, _) = self.racial_points(&draft.race, &draft.subrace);
        let spent: i64 = draft
            .racial_abilities
            .iter()
            .map(|a| i64::from(self.racial_ability_cost(a)))
            .sum();
        i64::from(points) - spent
    }

    /// The class abilities a Skills & Powers character of the class may
    /// buy, the defaults included.
    pub fn class_ability_choices(&self, class: &str) -> Vec<(String, i32)> {
        let Some(c) = self.class(class) else {
            return Vec::new();
        };
        let mut names: Vec<String> = c.abilities.clone();
        for name in &c.default_abilities {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        names
            .into_iter()
            .map(|n| {
                let cost = self.class_ability_cost(&n);
                (n, cost)
            })
            .collect()
    }

    pub fn class_ability_cost(&self, name: &str) -> i32 {
        self.class_ability_costs
            .get(name)
            .copied()
            .unwrap_or_else(|| cost_in_name(name))
    }

    /// The class abilities a new character starts with.
    pub fn default_class_abilities(&self, class: &str) -> Vec<String> {
        self.class(class).map_or_else(Vec::new, |c| {
            c.default_abilities
                .iter()
                .filter(|a| self.abilities.contains(*a))
                .cloned()
                .collect()
        })
    }

    /// The points of the class at `slot` not spent on its abilities.
    pub fn class_points_left(&self, draft: &Draft, slot: usize) -> i64 {
        let Some(class) = draft.classes.get(slot).and_then(|c| self.class(c)) else {
            return 0;
        };
        let spent: i64 = draft
            .class_abilities
            .get(slot)
            .into_iter()
            .flatten()
            .map(|a| i64::from(self.class_ability_cost(a)))
            .sum();
        i64::from(class.points.max(0)) - spent
    }

    /// Points kept from the racial and class abilities.
    fn kept_points(&self, draft: &Draft) -> i64 {
        let (_, saved) = self.racial_points(&draft.race, &draft.subrace);
        let racial = self.racial_points_left(draft).clamp(0, i64::from(saved));
        let classes: i64 = (0..draft.classes.len())
            .map(|slot| self.class_points_left(draft, slot).max(0))
            .sum();
        racial + classes
    }

    pub fn traits(&self) -> &[(String, u32)] {
        &self.traits
    }

    pub fn disadvantages(&self) -> &[(String, u32)] {
        &self.disadvantages
    }

    pub fn trait_cost(&self, name: &str) -> u32 {
        self.traits.iter().find(|t| t.0 == name).map_or(0, |t| t.1)
    }

    pub fn disadvantage_points(&self, name: &str) -> u32 {
        self.disadvantages
            .iter()
            .find(|t| t.0 == name)
            .map_or(0, |t| t.1)
    }

    /// Points for nonweapon proficiencies and traits: those kept from the
    /// abilities, the best of the classes' allotments, the bonus for
    /// Knowledge, and what disadvantages give.
    pub fn proficiency_points(&self, draft: &Draft) -> u32 {
        let allotment = draft
            .classes
            .iter()
            .filter_map(|c| self.class(c))
            .map(|c| group_points(c.group))
            .max()
            .unwrap_or(0);
        let knowledge = self.subability_scores(draft)[3][2];
        let disadvantages: u32 = draft
            .disadvantages
            .iter()
            .map(|d| self.disadvantage_points(d))
            .sum();
        self.kept_points(draft) as u32
            + allotment
            + bonus_languages(knowledge)
            + disadvantages.min(DISADVANTAGE_POINTS)
    }

    /// Points the nonweapon proficiencies and traits leave; negative when
    /// they cost more than there is.
    pub fn proficiency_points_left(&self, draft: &Draft) -> i64 {
        let traits: u32 = draft.traits.iter().map(|t| self.trait_cost(t)).sum();
        i64::from(self.proficiency_points(draft))
            - i64::from(self.nonweapon_spent(draft))
            - i64::from(traits)
    }

    /// What a weapon proficiency costs a Skills & Powers character: 2
    /// points for warriors, 3 for the others.
    pub fn weapon_cost(&self, classes: &[String]) -> u32 {
        if classes
            .iter()
            .any(|c| self.class(c).is_some_and(|c| c.group == 0))
        {
            2
        } else {
            3
        }
    }

    /// What specializing in one weapon costs, for a fighter who has the
    /// Weapon specialization ability; `None` when the character cannot
    /// specialize at 1st level.
    pub fn specialization_cost(&self, draft: &Draft) -> Option<u32> {
        let slot = draft.classes.iter().position(|c| c == "Fighter")?;
        let has_ability = draft
            .class_abilities
            .get(slot)
            .is_some_and(|a| a.iter().any(|a| a.contains("Weapon specialization")));
        has_ability.then_some(if draft.classes.len() == 1 {
            SPECIALIZATION_COSTS.0
        } else {
            SPECIALIZATION_COSTS.1
        })
    }

    /// Character points left at the end, kept on the character for later
    /// levels: what the proficiencies leave, plus the weapon slots'
    /// allotment, less the weapon proficiencies and specializations.
    pub fn points_left(&self, draft: &Draft) -> i64 {
        let cost = i64::from(self.weapon_cost(&draft.classes));
        let allotment = i64::from(self.weapon_slots(&draft.classes)) * cost;
        let specializations = draft.specializations.len() as i64
            * i64::from(self.specialization_cost(draft).unwrap_or(0));
        self.proficiency_points_left(draft) + allotment
            - draft.weapon_proficiencies.len() as i64 * cost
            - specializations
    }
    /// Weapons one of the classes may use.
    pub fn weapons(&self, classes: &[String]) -> Vec<String> {
        self.weapons
            .iter()
            .filter(|(_, forbidden)| classes.iter().any(|c| !forbidden.contains(c)))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// The 1st-level wizard spells of the wizard class among `classes`'
    /// schools; empty when none of them is a wizard.
    pub fn spells(&self, classes: &[String]) -> Vec<String> {
        let Some(wizard) = classes
            .iter()
            .filter_map(|c| self.class(c))
            .find(|c| c.group == 3)
        else {
            return Vec::new();
        };
        let mut spells: Vec<String> = self
            .wizard_spells
            .iter()
            .filter(|(_, level, schools)| {
                *level == 1
                    && (wizard.schools.is_empty()
                        || schools.is_empty()
                        || schools
                            .iter()
                            .any(|s| wizard.schools.contains(s) || s.contains("Lesser Divination")))
            })
            .map(|s| s.0.clone())
            .collect();
        spells.sort_by_key(|s| s.to_lowercase());
        spells
    }

    pub fn is_wizard(&self, classes: &[String]) -> bool {
        classes
            .iter()
            .any(|c| self.class(c).is_some_and(|c| c.group == 3))
    }

    /// The native languages the wizard gives: with Skills & Powers the
    /// subrace's package; otherwise the race's own language, or the
    /// regional human dialect. Core Rules 2.0 leaves them empty.
    pub fn native_languages(&self, draft: &Draft) -> Vec<String> {
        if !self.expansion {
            return Vec::new();
        }
        let race = self.race(&draft.race);
        let package = race
            .and_then(|r| r.subraces.iter().find(|s| s.name == draft.subrace))
            .map(|s| &s.languages)
            .filter(|l| draft.skills_and_powers && !l.is_empty());
        match (package, race) {
            (Some(languages), _) => languages.clone(),
            (None, Some(r)) if !r.languages.is_empty() => r.languages.clone(),
            _ if draft.race == "Human" => vec![HUMAN_LANGUAGE.into()],
            _ => vec!["Common".into()],
        }
    }

    pub fn ware(&self, name: &str) -> Option<&Ware> {
        self.wares.iter().find(|w| w.name == name)
    }

    /// What the draft's purchases cost, in copper pieces.
    pub fn spent(&self, draft: &Draft) -> u32 {
        let price = |name: &str| self.ware(name).map_or(0, |w| w.cost);
        let pack = if draft.pack {
            price(STANDARD_PACK.0)
                + STANDARD_PACK
                    .1
                    .iter()
                    .map(|(name, n)| price(name) * n)
                    .sum::<u32>()
        } else {
            0
        };
        pack + draft
            .purchases
            .iter()
            .map(|(name, n)| price(name) * n)
            .sum::<u32>()
    }

    /// Why the character cannot be made as drafted, if it cannot.
    pub fn problem(&self, draft: &Draft) -> Option<String> {
        if draft.name.trim().is_empty() {
            return Some("A character needs a name".into());
        }
        if self.race(&draft.race).is_none() {
            return Some("Choose a race".into());
        }
        if !self
            .races(draft.setting, &draft.scores, false)
            .contains(&draft.race)
        {
            return Some(
                self.race_problem(&draft.race, &draft.scores)
                    .unwrap_or_else(|| {
                        format!("{} is not a race of {}", draft.race, draft.setting.name)
                    }),
            );
        }
        let adjusted = self.adjusted(&draft.race, &draft.scores);
        if !self
            .class_options(draft.setting, &draft.race, &adjusted)
            .contains(&draft.classes)
        {
            return Some(format!(
                "A {} cannot be a {} here",
                draft.race,
                draft.class_title()
            ));
        }
        if !self.alignments(&draft.classes).contains(&draft.alignment) {
            return Some(format!(
                "A {} cannot be {}",
                draft.class_title(),
                draft.alignment
            ));
        }
        if draft.kit != "None"
            && !self
                .kits(&draft.race, &draft.subrace, &draft.classes)
                .contains(&draft.kit)
        {
            return Some(format!("The {} kit is not for this character", draft.kit));
        }
        if let Some(problem) = if draft.skills_and_powers {
            self.points_problem(draft)
        } else {
            self.slots_problem(draft, adjusted[3])
        } {
            return Some(problem);
        }
        let most = max_spells_per_level(adjusted[3]);
        if draft.spells.len() > most {
            return Some(format!(
                "Intelligence {} allows {most} spells of a level",
                adjusted[3]
            ));
        }
        if let Some((name, _)) = draft.purchases.iter().find(|(n, _)| self.ware(n).is_none()) {
            return Some(format!("{name} is not for sale"));
        }
        if self.spent(draft) > draft.gold * 100 {
            return Some(format!(
                "The equipment costs more than the {} gold pieces there are",
                draft.gold
            ));
        }
        None
    }

    /// Core Rules: the proficiencies fit the slots.
    fn slots_problem(&self, draft: &Draft, intelligence: u32) -> Option<String> {
        let slots = self.nonweapon_slots(&draft.classes, intelligence);
        let spent = self.nonweapon_spent(draft);
        if spent > slots {
            return Some(format!(
                "The nonweapon proficiencies take {spent} slots; there are {slots}"
            ));
        }
        let slots = self.weapon_slots(&draft.classes);
        if draft.weapon_proficiencies.len() as u32 > slots {
            return Some(format!("There are {slots} weapon proficiency slots"));
        }
        None
    }

    /// Skills & Powers: every choice is on offer and the points cover it.
    fn points_problem(&self, draft: &Draft) -> Option<String> {
        if !self.expansion {
            return Some("Player's Option characters need the Expansion".into());
        }
        if let Some(a) = draft
            .subabilities
            .iter()
            .find(|a| a.abs() > SUBABILITY_SPREAD)
        {
            return Some(format!(
                "A subability may be at most {SUBABILITY_SPREAD} from its score, not {}",
                a.abs()
            ));
        }
        let choices = self.racial_ability_choices(&draft.race, &draft.subrace);
        if let Some(a) = draft
            .racial_abilities
            .iter()
            .find(|a| !choices.iter().any(|c| &c.0 == *a))
        {
            return Some(format!(
                "{a} is not a racial ability of the {}",
                draft.subrace
            ));
        }
        let left = self.racial_points_left(draft);
        if left < 0 {
            return Some(format!(
                "The racial abilities cost {} points more than the race has",
                -left
            ));
        }
        for (slot, class) in draft.classes.iter().enumerate() {
            let choices = self.class_ability_choices(class);
            if let Some(a) = draft
                .class_abilities
                .get(slot)
                .into_iter()
                .flatten()
                .find(|a| !choices.iter().any(|c| &c.0 == *a))
            {
                return Some(format!("{a} is not a {class} ability"));
            }
            let left = self.class_points_left(draft, slot);
            if left < 0 {
                return Some(format!(
                    "The {class} abilities cost {} points more than the class has",
                    -left
                ));
            }
        }
        if draft.disadvantages.len() > MOST_DISADVANTAGES {
            return Some(format!(
                "A character may have at most {MOST_DISADVANTAGES} disadvantages"
            ));
        }
        if let Some(t) = draft
            .traits
            .iter()
            .find(|t| !self.traits.iter().any(|k| &k.0 == *t))
        {
            return Some(format!("{t} is not a trait"));
        }
        if let Some(t) = draft
            .disadvantages
            .iter()
            .find(|t| !self.disadvantages.iter().any(|k| &k.0 == *t))
        {
            return Some(format!("{t} is not a disadvantage"));
        }
        let left = self.proficiency_points_left(draft);
        if left < 0 {
            return Some(format!(
                "The nonweapon proficiencies and traits cost {} points more than there are",
                -left
            ));
        }
        if !draft.specializations.is_empty() {
            if self.specialization_cost(draft).is_none() {
                return Some("Only a fighter with Weapon specialization may specialize".into());
            }
            if let Some(w) = draft
                .specializations
                .iter()
                .find(|w| !draft.weapon_proficiencies.contains(w))
            {
                return Some(format!("Specializing in {w} needs its proficiency"));
            }
            let multiple = draft
                .class_abilities
                .iter()
                .flatten()
                .any(|a| a.contains("Multiple specialization"));
            if draft.specializations.len() > 1 && !multiple {
                return Some(
                    "Specializing in more than one weapon needs Multiple specialization".into(),
                );
            }
        }
        let left = self.points_left(draft);
        if left < 0 {
            return Some(format!(
                "The weapon proficiencies cost {} points more than there are",
                -left
            ));
        }
        None
    }

    /// The character file, filled in as the original's wizard fills it.
    pub fn build(&self, draft: &Draft) -> Result<Character> {
        let format = |source| Error::Format {
            path: "new character".into(),
            source,
        };
        let version = if self.expansion {
            EXPANSION_VERSION
        } else {
            CORE20_VERSION
        };
        let mut character = Character::blank(version).map_err(format)?;
        let adjusted = self.adjusted(&draft.race, &draft.scores);
        let race = self.race(&draft.race);
        {
            let d = &mut character.data;
            d.player.clone_from(&draft.player);
            d.name = draft.name.trim().to_owned();
            d.class_title = draft.class_title();
            d.unknown_0x90 = "NONE".into();
            d.social_class.clone_from(&draft.social_class);
            d.height = draft.height;
            d.weight = draft.weight;
            d.age = draft.age;
            // 0 for female characters; the original writes other values
            // (1 to 3) for males.
            d.unknown_0x163c = u32::from(draft.male);
            let scores = self.subability_scores(draft);
            d.strength = scores[0];
            d.exceptional_strength = if self.has_exceptional_strength(&draft.classes, adjusted[0]) {
                draft.exceptional_strength.clamp(1, 100)
            } else {
                0
            };
            for (a, ability) in d.abilities.iter_mut().enumerate() {
                *ability = scores[a + 1];
            }
            d.racial_abilities = if draft.skills_and_powers {
                draft.racial_abilities.clone()
            } else {
                self.racial_package(&draft.race, &draft.subrace)
            };
            d.racial_abilities
                .sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
            // The starting funds, in gold pieces.
            d.unknown_0x1778[4] = draft.gold;
            d.race.clone_from(&draft.race);
            d.subrace.clone_from(&draft.subrace);
            d.kit.clone_from(&draft.kit);
            d.alignment.clone_from(&draft.alignment);
            if self.expansion {
                let movement = race
                    .and_then(|r| r.subraces.iter().find(|s| s.name == draft.subrace))
                    .map(|s| s.movement)
                    .or(race.map(|r| r.movement))
                    .unwrap_or(12);
                d.unknown_0x17bc[0] = movement.max(0) as u32;
                d.flags_0x1ddc = Some([1, 0]);
                d.unknown_0x1df0 = Some((u32::MAX, 0));
            }
            d.sheet_options = Some(SheetOptions {
                version: 2,
                unknown_0x260: 0,
                unknown_0x25c: if self.expansion { 0 } else { 1822 },
                entries: SheetEntries::Pairs(Vec::new()),
            });
            d.classes.names.clone_from(&draft.classes);
            for (slot, name) in draft.classes.iter().enumerate().take(5) {
                let Some(class) = self.class(name) else {
                    continue;
                };
                let s = &mut d.class_slots[slot];
                s.abilities = if draft.skills_and_powers {
                    let mut chosen = draft.class_abilities.get(slot).cloned().unwrap_or_default();
                    chosen.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
                    chosen
                } else {
                    self.default_class_abilities(name)
                };
                s.unknown_0x270.clone_from(&class.schools);
                s.unknown_0x2d4.clone_from(&class.minor_spheres);
                s.unknown_0x338.clone_from(&class.major_spheres);
                d.unknown_0xc60[slot][0] = draft.hit_dice.get(slot).copied().unwrap_or(1);
            }
        }

        // Proficiencies, languages, spells and equipment go in as the
        // lists editor adds them.
        let mut lists = CharacterLists {
            weapon_proficiencies: draft.weapon_proficiencies.clone(),
            nonweapon_proficiencies: draft.nonweapon_proficiencies.clone(),
            languages: draft.languages.clone(),
            wizard_spells: draft.spells.clone(),
            ..CharacterLists::default()
        };
        let mut add = |bundle: usize, name: &str, quantity: u32| {
            let mut item = Item::new(name);
            item.quantity = quantity;
            lists.equipment[bundle].push(item);
        };
        if draft.pack && self.ware(STANDARD_PACK.0).is_some() {
            add(CARRIED, STANDARD_PACK.0, 1);
        }
        if self.is_wizard(&draft.classes) && self.parts.pages(SPELLBOOK).is_some() {
            add(CARRIED, SPELLBOOK, 1);
        }
        for (name, quantity) in &draft.purchases {
            let bundle = match self.ware(name).map(|w| w.part_type) {
                Some("Weapons") => READIED,
                Some("Armor") => WORN,
                _ => CARRIED,
            };
            add(bundle, name, *quantity);
        }
        let left = (draft.gold * 100).saturating_sub(self.spent(draft));
        lists.coins = [left % 10, left / 10 % 10, 0, left / 100, 0];
        lists.apply(&mut character, &self.parts)?;
        if draft.skills_and_powers {
            // The lists went in as Core Rules ones; now the points.
            let d = &mut character.data;
            d.skills_and_powers = 1;
            d.nonweapon_costs = draft
                .nonweapon_proficiencies
                .iter()
                .map(|n| self.nonweapon_cost(draft, n))
                .collect();
            d.unknown_0x1778[0] = self.points_left(draft).max(0) as u32;
            let sorted = |list: &[String]| {
                let mut list = list.to_vec();
                list.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
                list
            };
            d.unknown_0x1830[0] = sorted(&draft.traits);
            d.unknown_0x1830[1] = sorted(&draft.disadvantages);
            d.unknown_0xac[3] = sorted(&draft.specializations);
        }

        // The pack's contents go into the backpack.
        if draft.pack {
            let mut contents = InventoryBundle { items: Vec::new() };
            for (name, quantity) in STANDARD_PACK.1 {
                if self.ware(name).is_some() {
                    contents.items.push(self.parts.new_item(name, quantity));
                }
            }
            contents
                .items
                .sort_by(|x, y| x.name.as_bytes().cmp(y.name.as_bytes()));
            if let Some(pack) = character.data.inventory[CARRIED]
                .items
                .iter_mut()
                .find(|i| i.name == STANDARD_PACK.0)
            {
                pack.contents = Some(Box::new(Object::InventoryBundle(contents)));
            }
        }
        if self.expansion {
            let weight = carried_weight(&character, &self.parts) as f32;
            character.data.carried_weight = weight.to_bits();
        } else {
            character.data.carried_weight = 0;
        }
        Ok(character)
    }

    /// The character list entry the original writes for a new character.
    fn entry(&self, draft: &Draft, character: &Character, file: &str) -> CharacterEntry {
        let level = vec!["1"; draft.classes.len().max(1)].join("/");
        let race = format!("Race: {}", draft.race);
        let class = format!("{CLASS_LINE}{}", character.data.class_title);
        let level = format!("{LEVEL_LINE}{level}");
        if self.expansion {
            CharacterEntry {
                name: character.data.name.clone(),
                file: file.into(),
                race: draft.race.clone(),
                version: 3,
                extra: Some(EntryExtra {
                    unknown: 8,
                    unknown_0x2c: String::new(),
                }),
                unknown_0x24: 0,
                flag_0x28: u8::from(!draft.skills_and_powers),
                details: vec![
                    if draft.skills_and_powers {
                        "Skills & Powers"
                    } else {
                        "Core Rules"
                    }
                    .into(),
                    format!("{PLAYER_LINE}{}", draft.player),
                    "Party: ".into(),
                    race,
                    class,
                    level,
                    format!("Kit: {}", draft.kit),
                ],
            }
        } else {
            CharacterEntry {
                name: character.data.name.clone(),
                file: file.into(),
                race: draft.race.clone(),
                version: 0,
                extra: None,
                unknown_0x24: 0,
                flag_0x28: 1,
                details: vec!["Core Rules".into(), race, class, level],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SETTINGS;

    fn class(name: &str, id: i32, group: i32, hit_die: i32, requirements: [i32; 6]) -> ClassRules {
        let any = ["Lawful Good", "True Neutral", "Chaotic Evil"];
        ClassRules {
            name: name.into(),
            id,
            group,
            hit_die,
            weapon_slots: if group == 0 { 4 } else { 1 },
            nonweapon_slots: if group == 0 { 3 } else { 4 },
            alignments: match name {
                "Paladin" => vec!["Lawful Good".into()],
                _ => any.iter().map(|a| a.to_string()).collect(),
            },
            requirements,
            prime: [false; 6],
            default_abilities: vec![format!("{name}, Followers (5)")],
            abilities: if name == "Fighter" {
                vec![
                    "Fighter, Leadership (5)".into(),
                    "Fighter, Weapon specialization (5)".into(),
                ]
            } else {
                Vec::new()
            },
            points: if name == "Fighter" { 15 } else { 30 },
            major_spheres: Vec::new(),
            minor_spheres: Vec::new(),
            schools: if group == 3 {
                vec!["Alteration".into(), "Invocation/Evocation".into()]
            } else {
                Vec::new()
            },
        }
    }

    fn race(name: &str, limits: [[i32; 2]; 6], adjustments: [i32; 6]) -> RaceRules {
        RaceRules {
            name: name.into(),
            limits,
            adjustments,
            level_limits: [0; 9],
            physical: [60, 59, 2, 10, 140, 100, 6, 10, 15, 1, 4],
            movement: if name == "Dwarf" { 6 } else { 12 },
            languages: if name == "Dwarf" {
                vec!["Dwarf".into()]
            } else {
                Vec::new()
            },
            standard: true,
            subraces: if name == "Dwarf" {
                ["Hill dwarf", "Gray dwarf (duergar)"]
                    .into_iter()
                    .map(|sub| SubraceRules {
                        name: sub.into(),
                        movement: 6,
                        points: 45,
                        saved: 5,
                        abilities: vec!["Dwarf, Infravision (10)".into()],
                        languages: vec!["Dwarf".into(), "Gnome".into()],
                    })
                    .collect()
            } else {
                Vec::new()
            },
            multi_classes: Vec::new(),
            points: if name == "Dwarf" { 45 } else { 10 },
            saved: if name == "Dwarf" { 5 } else { 10 },
            abilities: Vec::new(),
        }
    }

    fn nonweapon(
        name: &str,
        slots: u32,
        points: u32,
        group: usize,
        relevant: &str,
    ) -> NonweaponRules {
        let mut groups = [false; 6];
        groups[group] = true;
        NonweaponRules {
            name: name.into(),
            slots,
            points,
            groups,
            relevant: vec![relevant.into()],
        }
    }

    fn rules(expansion: bool) -> CreationRules {
        let open = [[3, 18]; 6];
        let mut dwarf = open;
        dwarf[1] = [3, 17];
        let combinations = [
            (
                "Human",
                vec![
                    vec!["Fighter"],
                    vec!["Paladin"],
                    vec!["Mage"],
                    vec!["Psionicist"],
                ],
            ),
            ("Dwarf", vec![vec!["Fighter"], vec!["Fighter", "Mage"]]),
        ]
        .into_iter()
        .map(|(race, list)| {
            (
                race.to_owned(),
                list.into_iter()
                    .map(|c| c.into_iter().map(String::from).collect())
                    .collect(),
            )
        })
        .collect();
        CreationRules {
            expansion,
            races: vec![
                race("Dwarf", dwarf, [0, 0, 1, 0, 0, -1]),
                race("Human", open, [0; 6]),
            ],
            classes: vec![
                class("Fighter", 1000, 0, 10, [9, 0, 0, 0, 0, 0]),
                class("Paladin", 1001, 0, 10, [12, 0, 9, 0, 13, 17]),
                class("Mage", 4000, 3, 4, [0, 0, 0, 9, 0, 0]),
                class("Psionicist", 5000, 4, 6, [0, 0, 11, 12, 15, 0]),
            ],
            combinations: if expansion {
                combinations
            } else {
                HashMap::new()
            },
            kits: vec![KitRules {
                name: "None".into(),
                classes: Vec::new(),
                races: Vec::new(),
                excluded_races: Vec::new(),
                social_classes: vec!["Upper class".into(); 13],
            }],
            nonweapon: vec![
                nonweapon("Swimming", 1, 1, 0, "Strength/Stamina"),
                nonweapon("Tracking", 2, 4, 3, "Wisdom/Intuition"),
                nonweapon("Spellcraft", 1, 3, 4, "Intelligence/Reason"),
            ],
            racial_abilities: vec![
                ("Dwarf, Axe bonus (5)".into(), 5),
                ("Dwarf, Infravision (10)".into(), 10),
                ("Dwarf, Infravision [Deep Dwarf]".into(), 0),
            ],
            class_ability_costs: [
                ("Fighter, Followers (5)", 5),
                ("Fighter, Leadership (5)", 5),
                ("Fighter, Weapon specialization (5)", 5),
            ]
            .into_iter()
            .map(|(n, c)| (n.to_owned(), c))
            .collect(),
            traits: vec![("Allure".into(), 4)],
            disadvantages: vec![("Allergies".into(), 3), ("Greed".into(), 7)],
            weapons: vec![
                ("Dagger".into(), Vec::new()),
                ("Sword, long".into(), vec!["Mage".into()]),
            ],
            wizard_spells: vec![
                (
                    "Magic Missile".into(),
                    1,
                    vec!["Invocation/Evocation".into()],
                ),
                ("Sleep".into(), 1, vec!["Enchantment/Charm".into()]),
                ("Fireball".into(), 3, vec!["Invocation/Evocation".into()]),
            ],
            wares: vec![
                Ware {
                    name: "Backpack".into(),
                    part_type: "Equipment",
                    cost: 200,
                },
                Ware {
                    name: "Dagger".into(),
                    part_type: "Weapons",
                    cost: 200,
                },
                Ware {
                    name: "Torch".into(),
                    part_type: "Equipment",
                    cost: 1,
                },
            ],
            abilities: ["Fighter, Followers (5)".to_owned()].into_iter().collect(),
            parts: PartTable::default(),
        }
    }

    fn draft(rules: &CreationRules) -> Draft {
        let mut d = Draft::new(&SETTINGS[0]);
        d.scores = [16, 14, 15, 12, 10, 9];
        d.race = "Dwarf".into();
        d.subrace = "Hill dwarf".into();
        d.classes = vec!["Fighter".into(), "Mage".into()];
        d.name = "Durin Teste".into();
        d.player = "Fixture Four".into();
        d.alignment = "Lawful Good".into();
        d.social_class = "Upper class".into();
        d.age = 60;
        d.height = 48;
        d.weight = 150;
        d.hit_dice = vec![7, 3];
        d.nonweapon_proficiencies = vec!["Tracking".into()];
        d.languages = rules.native_languages(&d);
        d.weapon_proficiencies = vec!["Dagger".into()];
        d.spells = vec!["Magic Missile".into()];
        d.gold = 50;
        d.purchases = vec![("Dagger".into(), 1)];
        d
    }

    #[test]
    fn choices_follow_the_scores_race_and_setting() {
        let rules = rules(true);
        let generic = Setting::by_id("generic").unwrap();
        let scores = [16, 18, 15, 12, 10, 9];
        // Dexterity 18 is over the dwarf's limit.
        assert_eq!(rules.races(generic, &scores, false), ["Human"]);
        assert!(
            rules
                .race_problem("Dwarf", &scores)
                .unwrap()
                .contains("Dexterity")
        );
        let adjusted = rules.adjusted("Dwarf", &[16, 14, 15, 12, 10, 9]);
        assert_eq!(adjusted, [16, 14, 16, 12, 10, 8]);
        assert_eq!(
            rules.class_options(generic, "Dwarf", &adjusted),
            [
                vec!["Fighter".to_owned()],
                vec!["Fighter".into(), "Mage".into()]
            ]
        );
        // The paladin needs Charisma 17; the psionicist Wisdom 15.
        let human = rules.class_options(generic, "Human", &[16, 14, 15, 12, 10, 9]);
        assert_eq!(human, [vec!["Fighter".to_owned()], vec!["Mage".into()]]);
        let dragonlance = Setting::by_id("dragonlance").unwrap();
        assert!(
            rules
                .class_options(dragonlance, "Human", &[16, 14, 15, 12, 15, 9])
                .iter()
                .all(|c| c[0] != "Psionicist")
        );
        assert_eq!(rules.subraces(dragonlance, "Dwarf"), ["Hill dwarf"]);
        assert_eq!(rules.subraces(generic, "Human"), ["Human"]);
        assert_eq!(
            rules.alignments(&["Paladin".into(), "Mage".into()]),
            ["Lawful Good"]
        );
        assert_eq!(
            rules.spells(&["Fighter".into(), "Mage".into()]),
            ["Magic Missile"]
        );
        assert_eq!(rules.weapons(&["Mage".into()]), ["Dagger"]);
        assert_eq!(
            rules.nonweapon_slots(&["Fighter".into(), "Mage".into()], 12),
            4 + 3
        );
        assert_eq!(rules.weapon_slots(&["Fighter".into(), "Mage".into()]), 4);
    }

    #[test]
    fn dice_and_methods_roll_within_range() {
        let mut dice = Dice::seeded(42);
        assert_eq!(RollMethod::offered(false), RollMethod::ALL);
        assert_eq!(RollMethod::offered(true).len(), 9);
        assert_eq!(
            RollMethod::Points.roll(&mut dice).iter().sum::<u32>(),
            METHOD_VII_POINTS
        );
        assert_eq!(point_table(4), (72, 17));
        assert_eq!(point_table(12), (80, 15));
        for _ in 0..50 {
            let scores = roll_dice_pool([3, 6, 3, 4, 4, 4], &mut dice);
            assert!(scores.iter().all(|s| (3..=18).contains(s)));
        }
        for method in RollMethod::offered(true) {
            for _ in 0..50 {
                let scores = method.roll(&mut dice);
                assert!(scores.iter().all(|s| (3..=18).contains(s)), "{method:?}");
                if method.arranged() {
                    assert!(scores.windows(2).all(|w| w[0] >= w[1]));
                }
            }
        }
        let rules = rules(true);
        for _ in 0..50 {
            let gold = rules.roll_gold(&["Mage".into()], &mut dice);
            assert!((20..=50).contains(&gold) && gold.is_multiple_of(10));
            let social = rules.roll_social_class("None", &mut dice);
            assert_eq!(social, "Upper class");
        }
    }

    #[test]
    fn problems_are_reported_before_anything_is_written() {
        let rules = rules(true);
        let mut d = draft(&rules);
        assert_eq!(rules.problem(&d), None);
        d.alignment = "Chaotic Neutral".into();
        assert!(rules.problem(&d).is_some());
        let mut d = draft(&rules);
        d.nonweapon_proficiencies = vec!["Tracking".into(); 4];
        assert!(rules.problem(&d).unwrap().contains("slots"));
        let mut d = draft(&rules);
        d.purchases = vec![("Dagger".into(), 30)];
        assert!(rules.problem(&d).unwrap().contains("gold"));
        let mut d = draft(&rules);
        d.purchases = vec![("Leather armor".into(), 1)];
        assert!(rules.problem(&d).unwrap().contains("not for sale"));
        let mut d = draft(&rules);
        d.name = " ".into();
        assert!(rules.problem(&d).is_some());
    }

    fn installation(name: &str) -> (std::path::PathBuf, Installation) {
        let root = std::env::temp_dir().join(format!("cr-create-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Database")).unwrap();
        let install = Installation::open(&root).unwrap();
        (root, install)
    }

    #[test]
    fn expansion_characters_are_written_as_the_expansion_writes_them() {
        let (root, install) = installation("expansion");
        let rules = rules(true);
        let d = draft(&rules);
        let file = install.create_character(&rules, &d).unwrap();
        assert_eq!(file.len(), "12345678.dat".len());

        let c = install.character(&file).unwrap();
        assert_eq!(c.version, EXPANSION_VERSION);
        let data = &c.data;
        assert_eq!(data.name, "Durin Teste");
        assert_eq!(data.class_title, "Fighter/Mage");
        assert_eq!(
            (data.race.as_str(), data.subrace.as_str()),
            ("Dwarf", "Hill dwarf")
        );
        assert_eq!(data.strength, [16; 3]);
        assert_eq!(data.abilities[1], [16; 3]);
        assert_eq!(data.abilities[4], [8; 3]);
        assert_eq!(data.unknown_0x163c, 1);
        assert_eq!(data.unknown_0x17bc, [6, 0, 0]);
        assert_eq!(data.flags_0x1ddc, Some([1, 0]));
        assert_eq!(data.unknown_0xc60[0][0], 7);
        assert_eq!(data.unknown_0xc60[1][0], 3);
        assert_eq!(data.class_slots[0].abilities, ["Fighter, Followers (5)"]);
        assert!(data.class_slots[1].abilities.is_empty());
        assert_eq!(data.class_slots[1].unknown_0x270.len(), 2);
        // apply() takes the cost from the part table, which is empty here.
        assert_eq!(data.nonweapon_costs, [1]);
        assert_eq!(data.nonweapon_bonuses, Some(vec![0]));
        assert_eq!(data.languages, ["Dwarf"]);
        // Core Rules characters get the subrace's package too.
        assert_eq!(data.racial_abilities, ["Dwarf, Infravision (10)"]);
        assert_eq!(data.skills_and_powers, 0);
        assert_eq!(data.spells, ["Magic Missile"]);
        assert_eq!(data.unknown_0x1778[4], 50);
        // The pack holds its contents; 50 gp less the pack and the dagger.
        let pack = &data.inventory[CARRIED].items[0];
        assert_eq!(pack.name, "Backpack");
        let Some(Object::InventoryBundle(contents)) = pack.contents.as_deref() else {
            panic!("the backpack holds nothing");
        };
        assert_eq!(contents.items[0].name, "Torch");
        assert_eq!(contents.items[0].quantity, 3);
        assert_eq!(data.inventory[READIED].items[0].name, "Dagger");
        let coins: Vec<_> = data.inventory[6]
            .items
            .iter()
            .map(|i| (i.name.as_str(), i.quantity))
            .collect();
        assert_eq!(
            coins,
            [
                ("* Copper Pieces", 7),
                ("* Gold Pieces", 45),
                ("* Silver Pieces", 9)
            ]
        );

        let index = install.characters().unwrap();
        assert_eq!(index.schema, 95);
        let entry = &index.entries[0];
        assert_eq!(entry.file, file);
        assert_eq!(entry.version, 3);
        assert_eq!(
            entry.details,
            [
                "Core Rules",
                "Player's Name: Fixture Four",
                "Party: ",
                "Race: Dwarf",
                "Class: Fighter/Mage",
                "Level: 1/1",
                "Kit: None"
            ]
        );

        // A second character joins the list under another name.
        let mut d = draft(&rules);
        d.name = "Dwalin Teste".into();
        let second = install.create_character(&rules, &d).unwrap();
        assert_ne!(second, file);
        let index = install.characters().unwrap();
        assert_eq!(index.entries.len(), 2);
        assert!(root.join("Chars/index.chr.bak").exists());
        fs::remove_dir_all(root).unwrap();
    }

    fn skills_and_powers_draft(rules: &CreationRules) -> Draft {
        let mut d = draft(rules);
        d.skills_and_powers = true;
        d.classes = vec!["Fighter".into()];
        d.hit_dice = vec![7];
        d.spells.clear();
        d.subabilities = [2, 0, -1, 0, 0, 0];
        d.racial_abilities = vec![
            "Dwarf, Infravision (10)".into(),
            "Dwarf, Axe bonus (5)".into(),
        ];
        d.class_abilities = vec![vec![
            "Fighter, Weapon specialization (5)".into(),
            "Fighter, Followers (5)".into(),
        ]];
        d.traits = vec!["Allure".into()];
        d.disadvantages = vec!["Allergies".into()];
        d.specializations = vec!["Dagger".into()];
        d.languages = rules.native_languages(&d);
        d
    }

    #[test]
    fn skills_and_powers_points_add_up_as_in_the_original() {
        let rules = rules(true);
        let d = skills_and_powers_draft(&rules);
        assert_eq!(rules.problem(&d), None);
        // 45 racial points, 15 spent, 5 of the 30 left may be kept; the
        // fighter's 15 class points less 10 leave 5.
        assert_eq!(rules.racial_points_left(&d), 30);
        assert_eq!(rules.class_points_left(&d, 0), 5);
        // Kept 10, warriors' 6, Knowledge 12's 3, Allergies' 3.
        assert_eq!(rules.proficiency_points(&d), 22);
        // Tracking is a warrior proficiency: 4; Spellcraft is not: 3 + 2.
        assert_eq!(rules.nonweapon_cost(&d, "Tracking"), 4);
        assert_eq!(rules.nonweapon_cost(&d, "Spellcraft"), 5);
        assert_eq!(rules.proficiency_points_left(&d), 22 - 4 - 4);
        // Four weapon slots at 2 points; the dagger 2, specializing 2.
        assert_eq!(rules.points_left(&d), 14 + 8 - 2 - 2);
        assert_eq!(rules.subability_scores(&d)[0], [16, 18, 14]);
        assert_eq!(rules.subability_scores(&d)[2], [16, 15, 17]);
        assert_eq!(rules.native_languages(&d), ["Dwarf", "Gnome"]);
        // A subability may differ from its score by 1 or 2 only.
        let mut low = d.clone();
        low.scores[4] = 7;
        assert_eq!(rules.nonweapon_cost(&low, "Tracking"), 4 + 2);
        assert_eq!(
            rules.racial_ability_choices("Dwarf", "Hill dwarf"),
            [
                ("Dwarf, Axe bonus (5)".to_owned(), 5),
                ("Dwarf, Infravision (10)".to_owned(), 10)
            ]
        );
    }

    #[test]
    fn skills_and_powers_problems_name_what_is_short() {
        let rules = rules(true);
        let mut d = skills_and_powers_draft(&rules);
        d.subabilities[1] = 3;
        assert!(rules.problem(&d).unwrap().contains("subability"));
        let mut d = skills_and_powers_draft(&rules);
        d.class_abilities[0].push("Fighter, Leadership (5)".into());
        d.class_abilities[0].push("Fighter, Leadership (5)".into());
        assert!(rules.problem(&d).unwrap().contains("Fighter abilities"));
        let mut d = skills_and_powers_draft(&rules);
        d.disadvantages = vec!["Allergies".into(), "Greed".into(), "Greed".into()];
        assert!(rules.problem(&d).unwrap().contains("disadvantages"));
        let mut d = skills_and_powers_draft(&rules);
        d.nonweapon_proficiencies = vec!["Spellcraft".into(); 4];
        assert!(rules.problem(&d).unwrap().contains("traits cost"));
        let mut d = skills_and_powers_draft(&rules);
        d.specializations = vec!["Sword, long".into()];
        assert!(rules.problem(&d).unwrap().contains("proficiency"));
        let mut d = skills_and_powers_draft(&rules);
        d.class_abilities[0].retain(|a| !a.contains("specialization"));
        assert!(rules.problem(&d).unwrap().contains("fighter"));
        let mut d = skills_and_powers_draft(&rules);
        // 3 points left from the proficiencies, 8 for the weapon slots.
        d.nonweapon_proficiencies = vec!["Spellcraft".into(); 3];
        d.weapon_proficiencies = vec!["Dagger".into(); 5];
        assert!(
            rules
                .problem(&d)
                .unwrap()
                .contains("weapon proficiencies cost")
        );
    }

    #[test]
    fn skills_and_powers_characters_are_written_with_their_points() {
        let (root, install) = installation("skills");
        let rules = rules(true);
        let d = skills_and_powers_draft(&rules);
        let file = install.create_character(&rules, &d).unwrap();
        let c = install.character(&file).unwrap();
        let data = &c.data;
        assert_eq!(data.skills_and_powers, 1);
        assert_eq!(data.strength, [16, 18, 14]);
        assert_eq!(data.abilities[1], [16, 15, 17]);
        assert_eq!(
            data.racial_abilities,
            ["Dwarf, Axe bonus (5)", "Dwarf, Infravision (10)"]
        );
        assert_eq!(
            data.class_slots[0].abilities,
            [
                "Fighter, Followers (5)",
                "Fighter, Weapon specialization (5)"
            ]
        );
        assert_eq!(data.nonweapon_proficiencies, ["Tracking"]);
        assert_eq!(data.nonweapon_costs, [4]);
        assert_eq!(data.unknown_0x1778[0], 18);
        assert_eq!(data.unknown_0x1830[0], ["Allure"]);
        assert_eq!(data.unknown_0x1830[1], ["Allergies"]);
        assert_eq!(data.unknown_0xac[3], ["Dagger"]);
        assert_eq!(data.languages, ["Dwarf", "Gnome"]);
        let entry = &install.characters().unwrap().entries[0];
        assert_eq!(entry.details[0], "Skills & Powers");
        assert_eq!(entry.flag_0x28, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn core20_characters_are_written_as_core_rules_2_0_writes_them() {
        let (root, install) = installation("core20");
        fs::create_dir_all(root.join("Chars")).unwrap();
        // The index Core Rules 2.0 leaves when it has no characters.
        fs::write(root.join("Chars/index.chr"), [0, 0]).unwrap();
        let rules = rules(false);
        let mut d = draft(&rules);
        d.classes = vec!["Fighter".into()];
        d.hit_dice = vec![7];
        d.spells.clear();
        let file = install.create_character(&rules, &d).unwrap();

        let c = install.character(&file).unwrap();
        assert_eq!(c.version, CORE20_VERSION);
        assert_eq!(c.data.flags_0x1ddc, None);
        assert_eq!(c.data.nonweapon_bonuses, None);
        assert_eq!(c.data.carried_weight, 0);
        assert_eq!(c.data.unknown_0x17bc, [0; 3]);
        assert!(c.data.languages.is_empty());

        let index = install.characters().unwrap();
        assert_eq!(index.schema, 95);
        let entry = &index.entries[0];
        assert_eq!((entry.version, entry.extra.is_none()), (0, true));
        assert_eq!(
            entry.details,
            ["Core Rules", "Race: Dwarf", "Class: Fighter", "Level: 1"]
        );
        fs::remove_dir_all(root).unwrap();
    }
}
