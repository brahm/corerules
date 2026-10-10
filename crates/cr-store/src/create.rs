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
//! Skills & Powers characters, rogue skill points and weapon
//! specialization are left to the original program.

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
}

impl RollMethod {
    pub const ALL: [RollMethod; 6] = [
        Self::InOrder,
        Self::BestOfTwo,
        Self::Arrange,
        Self::BestSixOfTwelve,
        Self::FourDice,
        Self::Enter,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::InOrder => "Method I",
            Self::BestOfTwo => "Method II",
            Self::Arrange => "Method III",
            Self::BestSixOfTwelve => "Method IV",
            Self::FourDice => "Method V",
            Self::Enter => "Enter Scores",
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
        }
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
            Self::Enter => [10; 6],
        };
        if self.arranged() {
            scores.sort_unstable_by(|a, b| b.cmp(a));
        }
        scores
    }
}

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
    /// Subraces, the default one first, with their movement rates.
    subraces: Vec<(String, i32)>,
    /// Multi-class combinations as class ids per group (Core Rules 2.0).
    multi_classes: Vec<[i32; 5]>,
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

/// Every choice made for a new character.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub setting: &'static Setting,
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
    /// Nonweapon proficiencies and their cost in slots.
    nonweapon: Vec<(String, u32)>,
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
                let entry = (sub.name.clone(), sub.movement);
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
                    rules.nonweapon.push((
                        part.head.name.clone(),
                        rules.parts.nonweapon_cost(&part.head.name, false),
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
        rules.nonweapon.sort_by_key(|n| n.0.to_lowercase());
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
            .filter(|(i, s)| setting.allows_subrace(&s.0, *i == 0))
            .map(|(_, s)| s.0.clone())
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

    /// Nonweapon proficiencies with their cost in slots.
    pub fn nonweapon_proficiencies(&self) -> &[(String, u32)] {
        &self.nonweapon
    }

    pub fn nonweapon_cost(&self, name: &str) -> u32 {
        self.nonweapon
            .iter()
            .find(|n| n.0 == name)
            .map_or(1, |n| n.1)
    }

    /// Slots the draft's nonweapon proficiencies take.
    pub fn nonweapon_spent(&self, draft: &Draft) -> u32 {
        draft
            .nonweapon_proficiencies
            .iter()
            .map(|n| self.nonweapon_cost(n))
            .sum()
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

    /// The native languages the wizard gives the race: the race's own, or
    /// the regional human dialect. Core Rules 2.0 leaves them empty.
    pub fn native_languages(&self, race: &str) -> Vec<String> {
        if !self.expansion {
            return Vec::new();
        }
        match self.race(race) {
            Some(r) if !r.languages.is_empty() => r.languages.clone(),
            _ if race == "Human" => vec![HUMAN_LANGUAGE.into()],
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
        let slots = self.nonweapon_slots(&draft.classes, adjusted[3]);
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
            d.strength = [adjusted[0]; 3];
            d.exceptional_strength = if self.has_exceptional_strength(&draft.classes, adjusted[0]) {
                draft.exceptional_strength.clamp(1, 100)
            } else {
                0
            };
            for (a, scores) in d.abilities.iter_mut().enumerate() {
                *scores = [adjusted[a + 1]; 3];
            }
            // The starting funds, in gold pieces.
            d.unknown_0x1778[4] = draft.gold;
            d.race.clone_from(&draft.race);
            d.subrace.clone_from(&draft.subrace);
            d.kit.clone_from(&draft.kit);
            d.alignment.clone_from(&draft.alignment);
            if self.expansion {
                let movement = race
                    .and_then(|r| r.subraces.iter().find(|s| s.0 == draft.subrace))
                    .map(|s| s.1)
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
                s.abilities = class
                    .default_abilities
                    .iter()
                    .filter(|a| self.abilities.contains(*a))
                    .cloned()
                    .collect();
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
                flag_0x28: 1,
                details: vec![
                    "Core Rules".into(),
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
                vec![("Hill dwarf".into(), 6), ("Gray dwarf (duergar)".into(), 6)]
            } else {
                Vec::new()
            },
            multi_classes: Vec::new(),
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
            nonweapon: vec![("Swimming".into(), 1), ("Tracking".into(), 2)],
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
        d.languages = rules.native_languages("Dwarf");
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
        for method in RollMethod::ALL {
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
