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
//! that give points back, weapon of choice, expertise, specialization,
//! unarmed combat, shield and armor proficiencies and fighting styles. The
//! point pools come from the databases (race and class records, part
//! costs); what the original has built in (the costs of the combat
//! options, the fighting styles each class group may learn, the rogue
//! skill tables) follows the books, as in the original.

use cr_i18n::{tr, tr_noop};
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
    tr_noop!("Strength"),
    tr_noop!("Dexterity"),
    tr_noop!("Constitution"),
    tr_noop!("Intelligence"),
    tr_noop!("Wisdom"),
    tr_noop!("Charisma"),
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
pub(crate) const INDEX_SCHEMA: u16 = 95;

/// Wizards get their spellbook for free.
pub const SPELLBOOK: &str = "Spellbook";

/// The native language the original gives a race whose record names none.
const HUMAN_LANGUAGE: &str = "Human, Regional dialect";
/// The races of human blood, which speak the regional dialect too.
const HUMAN_BLOOD: [&str; 4] = ["Human", "Half-orc", "Half-ogre", "Half-elf"];

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
    /// Method VI: every ability starts at 8 and 7d6 are added, each die
    /// whole to one ability, none above 18.
    SevenDice,
    /// Skills & Powers Method X: Method VII's 75 points, of which warriors
    /// may spend some on exceptional Strength, 10% a point.
    PointsAndStrength,
    /// Race and class first, then 3d6 in order raised or lowered into
    /// what they allow.
    PlayersChoice,
}

/// Method VII's points.
pub const METHOD_VII_POINTS: u32 = 75;

/// Method VIII's dice.
pub const METHOD_VIII_DICE: u32 = 24;

/// Method VI: the score every ability starts at, and the dice added.
pub const METHOD_VI_BASE: u32 = 8;
pub const METHOD_VI_DICE: usize = 7;

/// Method X: exceptional Strength bought per point.
pub const METHOD_X_STRENGTH_STEP: u32 = 10;

impl RollMethod {
    /// The Player's Handbook's methods.
    pub const ALL: [RollMethod; 7] = [
        Self::InOrder,
        Self::BestOfTwo,
        Self::Arrange,
        Self::BestSixOfTwelve,
        Self::FourDice,
        Self::SevenDice,
        Self::Enter,
    ];

    /// What the Expansion adds, for Core Rules and Player's Option
    /// characters alike, in its order: Methods VII to X before Enter
    /// Scores, Player's Choice after it.
    pub const SKILLS_AND_POWERS: [RollMethod; 4] = [
        Self::Points,
        Self::DicePool,
        Self::RolledPoints,
        Self::PointsAndStrength,
    ];

    /// The methods the installation's wizard offers.
    pub fn offered(expansion: bool) -> Vec<RollMethod> {
        let mut methods = Self::ALL.to_vec();
        if expansion {
            methods.splice(6..6, Self::SKILLS_AND_POWERS);
            methods.push(Self::PlayersChoice);
        }
        methods
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::InOrder => tr("Method I"),
            Self::BestOfTwo => tr("Method II"),
            Self::Arrange => tr("Method III"),
            Self::BestSixOfTwelve => tr("Method IV"),
            Self::FourDice => tr("Method V"),
            Self::Enter => tr("Enter Scores"),
            Self::Points => tr("Method VII"),
            Self::DicePool => tr("Method VIII"),
            Self::RolledPoints => tr("Method IX"),
            Self::SevenDice => tr("Method VI"),
            Self::PointsAndStrength => tr("Method X"),
            Self::PlayersChoice => tr("Player's Choice"),
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::InOrder => tr("3d6 for each ability, in order"),
            Self::BestOfTwo => tr("3d6 twice for each ability, the better in order"),
            Self::Arrange => tr("3d6 six times, arranged as you like"),
            Self::BestSixOfTwelve => tr("3d6 twelve times, the best six arranged as you like"),
            Self::FourDice => tr("4d6 six times dropping the lowest die, arranged as you like"),
            Self::Enter => tr("Type in scores rolled elsewhere"),
            Self::Points => tr("75 points divided among the abilities, 3 to 18 each"),
            Self::DicePool => tr("24d6 shared out, 3 to 6 dice an ability, the best three kept"),
            Self::RolledPoints => {
                tr("2d6 on a table give the points to divide and the highest score")
            }
            Self::SevenDice => tr("Scores start at 8; 7d6 added, a die an ability"),
            Self::PointsAndStrength => tr("75 points; a warrior may buy exceptional Strength"),
            Self::PlayersChoice => tr("Race and class first; 3d6 then fitted to them"),
        }
    }

    /// Whether the player divides points among the abilities.
    pub fn divided(self) -> bool {
        matches!(
            self,
            Self::Points | Self::RolledPoints | Self::PointsAndStrength
        )
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
            Self::Enter | Self::Points | Self::RolledPoints | Self::PointsAndStrength => {
                [13, 13, 13, 12, 12, 12]
            }
            Self::DicePool => roll_dice_pool([4; 6], dice),
            Self::SevenDice => {
                // Each die to the lowest score it fits.
                let rolled = roll_seven_dice(dice);
                let mut assignment = [0; METHOD_VI_DICE];
                let mut scores = [METHOD_VI_BASE; 6];
                for (d, &die) in rolled.iter().enumerate() {
                    let a = (0..6)
                        .filter(|&a| scores[a] + die <= 18)
                        .min_by_key(|&a| scores[a])
                        .unwrap_or(0);
                    assignment[d] = a;
                    scores[a] = (scores[a] + die).min(18);
                }
                seven_dice_scores(&rolled, &assignment).unwrap_or(scores)
            }
            Self::PlayersChoice => std::array::from_fn(|_| dice.roll(3, 6)),
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

/// Method VI: the seven dice.
pub fn roll_seven_dice(dice: &mut Dice) -> [u32; METHOD_VI_DICE] {
    std::array::from_fn(|_| dice.roll(1, 6))
}

/// Method VI: the scores when each die goes to the ability `assignment`
/// gives it (0 for Strength); `None` when a score would pass 18.
pub fn seven_dice_scores(
    rolled: &[u32; METHOD_VI_DICE],
    assignment: &[usize; METHOD_VI_DICE],
) -> Option<[u32; 6]> {
    let mut scores = [METHOD_VI_BASE; 6];
    for (&die, &a) in rolled.iter().zip(assignment) {
        *scores.get_mut(a)? += die;
    }
    scores.iter().all(|&s| s <= 18).then_some(scores)
}

/// Method X: points 18/01 and better cost on top of the 18; `None` when
/// the Strength is not 18.
pub fn exceptional_strength_points(strength: u32, exceptional: u32) -> Option<u32> {
    (strength == 18).then(|| exceptional.div_ceil(METHOD_X_STRENGTH_STEP))
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
    [tr_noop!("Stamina"), tr_noop!("Muscle")],
    [tr_noop!("Aim"), tr_noop!("Balance")],
    [tr_noop!("Health"), tr_noop!("Fitness")],
    [tr_noop!("Reason"), tr_noop!("Knowledge")],
    [tr_noop!("Intuition"), tr_noop!("Willpower")],
    [tr_noop!("Leadership"), tr_noop!("Appearance")],
];

/// How far a subability may be from its ability score.
pub const SUBABILITY_SPREAD: i32 = 2;

/// Character points for nonweapon proficiencies per class group, for a
/// class record that has none (Skills & Powers, chapter 6).
fn group_points(group: i32) -> u32 {
    match group {
        2 | 3 => 8,
        _ => 6,
    }
}

/// The most points disadvantages may give, and how many may be taken.
pub const DISADVANTAGE_POINTS: u32 = 15;
pub const MOST_DISADVANTAGES: usize = 2;

/// Weapon specialization at 1st level: only single-classed fighters,
/// for 2 points. A multi-classed fighter is not of high enough level, but
/// may take an unarmed style for twice that.
const SPECIALIZATION_COST: u32 = 2;

/// The unarmed combat styles a fighter may specialize in, in the order of
/// the character file's flags (`flags_0x1e54`).
pub const UNARMED: [&str; 3] = ["Punching", "Wrestling", "Martial Arts"];

/// Shield proficiencies, in the order of the file's flags
/// (`unknown_0x134`).
pub const SHIELDS: [&str; 4] = ["Buckler", "Small shield", "Medium shield", "Body shield"];

/// The fighting styles and the numbers the file keeps them by
/// (`classes.unknown_0x50`), and the class groups that may learn each
/// (warrior, rogue, priest, wizard, psionicist; Skills & Powers, Table
/// 52). The "(+2)" styles are the specializations in them.
pub const FIGHTING_STYLES: [(&str, u32, [bool; 5]); 9] = [
    ("One-handed Weapon", 1993, [true; 5]),
    ("Weapon and Shield", 1994, [true, false, true, false, false]),
    ("Two-handed Weapon", 1995, [true, false, true, true, false]),
    ("Two Weapon", 1996, [true, true, false, false, false]),
    ("Missile", 1997, [true, true, false, false, false]),
    ("Horse Archer", 1998, [true, true, false, false, false]),
    ("Thrown Weapon/Sling", 1999, [true; 5]),
    ("Two Weapon (+2)", 2000, [true, true, false, false, false]),
    ("One-handed Weapon (+2)", 2001, [true; 5]),
];

/// How many fighting styles the file has room for.
pub const MOST_FIGHTING_STYLES: usize = 10;

/// The rogue skills in the order of the races' adjustments, their base
/// scores and what wearing no armor adds (Player's Handbook, Tables 26
/// and 29).
pub const ROGUE_SKILLS: [(&str, i32, i32); 8] = [
    (tr_noop!("Pick pockets"), 15, 5),
    (tr_noop!("Open locks"), 10, 0),
    (tr_noop!("Find/remove traps"), 5, 0),
    (tr_noop!("Move silently"), 10, 10),
    (tr_noop!("Hide in shadows"), 5, 5),
    (tr_noop!("Detect noise"), 15, 0),
    (tr_noop!("Climb walls"), 60, 10),
    (tr_noop!("Read languages"), 0, 0),
];

/// Discretionary rogue skill points at 1st level and the most one skill
/// may get: thieves, and bards.
pub const THIEF_POINTS: (u32, u32) = (60, 30);
pub const BARD_POINTS: (u32, u32) = (20, 20);

/// Dexterity's adjustments to pick pockets, open locks, find/remove traps,
/// move silently and hide in shadows (Player's Handbook, Table 28).
fn dexterity_skill_adjustments(dexterity: u32) -> [i32; 5] {
    match dexterity {
        0..=9 => [-15, -10, -10, -20, -10],
        10 => [-10, -5, -10, -15, -5],
        11 => [-5, 0, -5, -10, 0],
        12 => [0, 0, 0, -5, 0],
        13..=15 => [0; 5],
        16 => [0, 5, 0, 0, 0],
        17 => [5, 10, 0, 5, 5],
        18 => [10, 15, 5, 10, 10],
        _ => [15, 20, 10, 15, 15],
    }
}

/// The rogue skill a class ability gives: "Thief, Pick pockets* (10)" is
/// pick pockets. The star marks the skills that take points; a ranger's
/// "Hide in shadows (5)" is not one.
pub(crate) fn rogue_skill_of(ability: &str) -> Option<usize> {
    let name = ability.split_once(", ").map_or(ability, |(_, n)| n);
    let (name, _) = name.split_once('*')?;
    let name = name.trim();
    ROGUE_SKILLS
        .iter()
        .position(|(skill, ..)| skill.eq_ignore_ascii_case(name))
}

/// Whether a nonweapon proficiency teaches languages, one a slot or rank.
pub fn teaches_languages(proficiency: &str) -> bool {
    proficiency.ends_with("Languages")
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
    /// Rogue skill adjustments, in the order of [`ROGUE_SKILLS`].
    thief_adjustments: [i32; 8],
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
    /// How the scores were rolled.
    pub roll_method: RollMethod,
    /// Languages learned through Modern Languages and the like, one per
    /// slot (Core Rules) or rank (Skills & Powers), in the order of the
    /// proficiencies.
    pub learned_languages: Vec<String>,
    /// Discretionary points added to each rogue skill, in the order of
    /// [`ROGUE_SKILLS`].
    pub rogue_points: [u32; 8],
    /// Skills & Powers combat options.
    pub weapon_of_choice: Option<String>,
    pub expertise: Option<String>,
    pub unarmed: Vec<String>,
    /// Buckler, small, medium and body shield.
    pub shields: [bool; 4],
    pub armor_proficiencies: Vec<String>,
    /// In the order learned.
    pub fighting_styles: Vec<String>,
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
            roll_method: RollMethod::FourDice,
            learned_languages: Vec::new(),
            rogue_points: [0; 8],
            weapon_of_choice: None,
            expertise: None,
            unarmed: Vec::new(),
            shields: [false; 4],
            armor_proficiencies: Vec::new(),
            fighting_styles: Vec::new(),
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
    /// Every language, for Modern Languages and the like.
    languages: Vec<String>,
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
                thief_adjustments: r.thief_adjustments,
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
            languages: Vec::new(),
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
            for part in parts("Languages") {
                if seen.insert(("language", part.head.name.clone())) {
                    rules.languages.push(part.head.name.clone());
                }
            }
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
        rules
            .nonweapon
            .sort_by_cached_key(|n| crate::collate::sort_key(&n.name));
        rules
            .racial_abilities
            .sort_by_cached_key(|n| crate::collate::sort_key(&n.0));
        rules
            .traits
            .sort_by_cached_key(|n| crate::collate::sort_key(&n.0));
        rules
            .disadvantages
            .sort_by_cached_key(|n| crate::collate::sort_key(&n.0));
        rules
            .weapons
            .sort_by_cached_key(|n| crate::collate::sort_key(&n.0));
        rules
            .wares
            .sort_by_cached_key(|n| crate::collate::sort_key(&n.name));
        rules
            .languages
            .sort_by_cached_key(|n| crate::collate::sort_key(n));
        rules
            .races
            .sort_by_cached_key(|r| crate::collate::sort_key(&r.name));
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
pub(crate) fn new_file_name(chars: &Path, index: &CharacterIndex) -> String {
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
pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
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
        self.races_where(setting, standard_only, Some(scores))
    }

    /// The races the setting has, whatever the scores: Player's Choice
    /// picks the race before rolling.
    pub fn all_races(&self, setting: &Setting, standard_only: bool) -> Vec<String> {
        self.races_where(setting, standard_only, None)
    }

    fn races_where(
        &self,
        setting: &Setting,
        standard_only: bool,
        scores: Option<&[u32; 6]>,
    ) -> Vec<String> {
        self.races
            .iter()
            .filter(|r| setting.allows_race(&r.name) && (!standard_only || r.standard))
            .filter(|r| {
                scores.is_none_or(|scores| {
                    r.limits
                        .iter()
                        .zip(scores)
                        .all(|(l, &s)| (l[0]..=l[1]).contains(&(s as i32)))
                })
            })
            .filter(|r| !self.combinations_of(setting, &r.name).is_empty())
            .map(|r| r.name.clone())
            .collect()
    }

    /// The highest base scores the race allows: Player's Choice offers the
    /// classes these reach.
    pub fn best_scores(&self, race: &str) -> [u32; 6] {
        let limits = self.race(race).map_or([[3, 18]; 6], |r| r.limits);
        std::array::from_fn(|a| limits[a][1].clamp(3, 18) as u32)
    }

    /// Player's Choice: 3d6 in order, each raised to what the race and the
    /// classes need and lowered to the race's limit.
    pub fn players_choice(&self, race: &str, classes: &[String], dice: &mut Dice) -> [u32; 6] {
        let (limits, adjustments) = self
            .race(race)
            .map_or(([[3, 18]; 6], [0; 6]), |r| (r.limits, r.adjustments));
        std::array::from_fn(|a| {
            let needed = classes
                .iter()
                .filter_map(|c| self.class(c))
                .map(|c| c.requirements[a] - adjustments[a])
                .max()
                .unwrap_or(3);
            let low = limits[a][0].max(needed).max(3);
            let high = limits[a][1].clamp(3, 18).max(low);
            (dice.roll(3, 6) as i32).clamp(low, high) as u32
        })
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
        options.sort_by_cached_key(|c| (c.len(), crate::collate::sort_key(&c.join("/"))));
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
        kits.sort_by_cached_key(|k| crate::collate::sort_key(k));
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
    /// draft's nonweapon proficiencies take. A proficiency taken again
    /// takes its slots again with the Core Rules; with Skills & Powers
    /// each further rank costs 1 point.
    pub fn nonweapon_spent(&self, draft: &Draft) -> u32 {
        if draft.skills_and_powers {
            self.nonweapon_ranks(draft)
                .iter()
                .map(|(n, ranks)| self.nonweapon_rank_cost(draft, n, *ranks))
                .sum()
        } else {
            draft
                .nonweapon_proficiencies
                .iter()
                .map(|n| self.nonweapon_cost(draft, n))
                .sum()
        }
    }

    /// Each nonweapon proficiency once, in the order first taken, with how
    /// many times it was taken.
    pub fn nonweapon_ranks(&self, draft: &Draft) -> Vec<(String, u32)> {
        let mut ranks: Vec<(String, u32)> = Vec::new();
        for name in &draft.nonweapon_proficiencies {
            match ranks.iter_mut().find(|(n, _)| n == name) {
                Some((_, count)) => *count += 1,
                None => ranks.push((name.clone(), 1)),
            }
        }
        ranks
    }

    /// Skills & Powers: what a proficiency costs with this many ranks.
    pub fn nonweapon_rank_cost(&self, draft: &Draft, name: &str, ranks: u32) -> u32 {
        self.nonweapon_cost(draft, name) + ranks.saturating_sub(1)
    }

    /// Every language, for the proficiencies that teach them.
    pub fn languages(&self) -> &[String] {
        &self.languages
    }

    /// How many languages the proficiencies teach: one a slot or rank of
    /// Modern Languages and the like.
    pub fn language_slots(&self, draft: &Draft) -> usize {
        draft
            .nonweapon_proficiencies
            .iter()
            .filter(|n| teaches_languages(n))
            .count()
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
            .map(|c| match c.nonweapon_points {
                points if points > 0 => points as u32,
                _ => group_points(c.group),
            })
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

    /// What a weapon proficiency costs a Skills & Powers character: the
    /// cheapest of the classes' (2 points for warriors, 3 for the others).
    pub fn weapon_cost(&self, classes: &[String]) -> u32 {
        let group_cost = |c: &ClassRules| match c.weapon_cost {
            cost if cost > 0 => cost as u32,
            _ if c.group == 0 => 2,
            _ => 3,
        };
        classes
            .iter()
            .filter_map(|c| self.class(c))
            .map(group_cost)
            .min()
            .unwrap_or(3)
    }

    /// The points a Skills & Powers character gets for weapon
    /// proficiencies: the best of the classes' allotments.
    pub fn weapon_points(&self, classes: &[String]) -> u32 {
        let cost = self.weapon_cost(classes);
        classes
            .iter()
            .filter_map(|c| self.class(c))
            .map(|c| match c.weapon_points {
                points if points > 0 => points as u32,
                _ => c.weapon_slots.max(0) as u32 * cost,
            })
            .max()
            .unwrap_or(0)
    }

    /// The class groups among the classes: warrior, rogue, priest, wizard,
    /// psionicist.
    fn class_groups(&self, classes: &[String]) -> [bool; 5] {
        let mut groups = [false; 5];
        for class in classes.iter().filter_map(|c| self.class(c)) {
            if let Some(g) = groups.get_mut(class.group.clamp(0, 4) as usize) {
                *g = true;
            }
        }
        groups
    }

    /// The cheapest of the classes' groups for a per-group price list.
    fn cheapest(&self, classes: &[String], prices: [u32; 5]) -> u32 {
        let groups = self.class_groups(classes);
        (0..5)
            .filter(|&g| groups[g])
            .map(|g| prices[g])
            .min()
            .unwrap_or(prices[4])
    }

    /// What specializing in one weapon costs, for a single-classed fighter
    /// who has the Weapon specialization ability; `None` when the
    /// character cannot specialize at 1st level.
    pub fn specialization_cost(&self, draft: &Draft) -> Option<u32> {
        if draft.classes != ["Fighter"] {
            return None;
        }
        let has_ability = draft
            .class_abilities
            .first()
            .is_some_and(|a| a.iter().any(|a| a.contains("Weapon specialization")));
        has_ability.then_some(SPECIALIZATION_COST)
    }

    /// Weapon of choice: warriors 2 points, wizards 4, the others 3.
    pub fn weapon_of_choice_cost(&self, classes: &[String]) -> u32 {
        self.cheapest(classes, [2, 3, 3, 4, 3])
    }

    /// Weapon expertise: warriors 2 points, wizards 5, the others 4; 1
    /// less in the weapon of choice.
    pub fn expertise_cost(&self, draft: &Draft) -> u32 {
        let cost = self.cheapest(&draft.classes, [2, 4, 4, 5, 4]);
        if draft.expertise.is_some() && draft.expertise == draft.weapon_of_choice {
            cost - 1
        } else {
            cost
        }
    }

    /// Specializing in an unarmed style: fighters only, at what weapon
    /// specialization costs, twice that when multi-classed.
    pub fn unarmed_cost(&self, classes: &[String]) -> Option<u32> {
        classes
            .iter()
            .any(|c| c == "Fighter")
            .then_some(if classes.len() == 1 {
                SPECIALIZATION_COST
            } else {
                2 * SPECIALIZATION_COST
            })
    }

    /// A shield or armor proficiency: a weapon proficiency's cost for
    /// warriors, twice that for the others.
    pub fn shield_cost(&self, classes: &[String]) -> u32 {
        let cost = self.weapon_cost(classes);
        if self.class_groups(classes)[0] {
            cost
        } else {
            2 * cost
        }
    }

    /// The armor a character may become proficient with.
    pub fn armors(&self) -> Vec<String> {
        self.wares
            .iter()
            .filter(|w| w.part_type == "Armor")
            .filter(|w| {
                let name = w.name.to_lowercase();
                !name.contains("shield") && !name.contains("buckler")
            })
            .map(|w| w.name.clone())
            .collect()
    }

    /// What a fighting style costs, or `None` when none of the classes may
    /// learn it: a weapon proficiency's cost, 1 more for Two Weapon unless
    /// a ranger, and the "(+2)" specializations that again.
    pub fn style_cost(&self, classes: &[String], style: &str) -> Option<u32> {
        let (_, _, allowed) = FIGHTING_STYLES.iter().find(|s| s.0 == style)?;
        let groups = self.class_groups(classes);
        if !(0..5).any(|g| groups[g] && allowed[g]) {
            return None;
        }
        let cost = self.weapon_cost(classes);
        let base = style.trim_end_matches(" (+2)");
        let two_weapon = base == "Two Weapon" && !classes.iter().any(|c| c == "Ranger");
        let cost = cost + u32::from(two_weapon);
        Some(if base == style {
            cost
        } else {
            cost + self.weapon_cost(classes)
        })
    }

    /// The fighting styles the classes may learn.
    pub fn styles(&self, classes: &[String]) -> Vec<String> {
        FIGHTING_STYLES
            .iter()
            .filter(|s| self.style_cost(classes, s.0).is_some())
            .map(|s| s.0.to_owned())
            .collect()
    }

    /// What the combat options cost: weapon of choice, expertise,
    /// specializations, unarmed styles, shields, armor and fighting styles.
    pub fn combat_spent(&self, draft: &Draft) -> u32 {
        let classes = &draft.classes;
        let shields = draft.shields.iter().filter(|&&s| s).count() as u32
            + draft.armor_proficiencies.len() as u32;
        let styles: u32 = draft
            .fighting_styles
            .iter()
            .filter_map(|s| self.style_cost(classes, s))
            .sum();
        draft
            .weapon_of_choice
            .as_ref()
            .map_or(0, |_| self.weapon_of_choice_cost(classes))
            + draft
                .expertise
                .as_ref()
                .map_or(0, |_| self.expertise_cost(draft))
            + draft.specializations.len() as u32 * self.specialization_cost(draft).unwrap_or(0)
            + draft.unarmed.len() as u32 * self.unarmed_cost(classes).unwrap_or(0)
            + shields * self.shield_cost(classes)
            + styles
    }

    /// Character points left at the end, kept on the character for later
    /// levels: what the proficiencies leave, plus the weapon allotment,
    /// less the weapon proficiencies and the combat options.
    pub fn points_left(&self, draft: &Draft) -> i64 {
        let cost = i64::from(self.weapon_cost(&draft.classes));
        let allotment = i64::from(self.weapon_points(&draft.classes));
        self.proficiency_points_left(draft) + allotment
            - draft.weapon_proficiencies.len() as i64 * cost
            - i64::from(self.combat_spent(draft))
    }

    /// The class abilities the character file gets per class: those bought
    /// with Skills & Powers, the defaults with the Core Rules.
    fn chosen_class_abilities(&self, draft: &Draft, slot: usize) -> Vec<String> {
        let mut chosen = if draft.skills_and_powers {
            draft.class_abilities.get(slot).cloned().unwrap_or_default()
        } else {
            draft
                .classes
                .get(slot)
                .map_or_else(Vec::new, |c| self.default_class_abilities(c))
        };
        chosen.sort_by(|x, y| x.as_bytes().cmp(y.as_bytes()));
        chosen
    }

    /// The class abilities that are rogue skills, sorted as the file keeps
    /// them, with the skill each gives.
    pub fn rogue_skills(&self, draft: &Draft) -> Vec<(String, usize)> {
        let mut skills: Vec<(String, usize)> = (0..draft.classes.len())
            .flat_map(|slot| self.chosen_class_abilities(draft, slot))
            .filter_map(|a| rogue_skill_of(&a).map(|s| (a, s)))
            .collect();
        skills.sort_by(|x, y| x.0.as_bytes().cmp(y.0.as_bytes()));
        skills.dedup_by_key(|s| s.1);
        skills
    }

    /// Discretionary rogue skill points at 1st level and the most for one
    /// skill: none without rogue skills, a bard's when the bard is the
    /// only class with them.
    pub fn rogue_points(&self, draft: &Draft) -> (u32, u32) {
        if self.rogue_skills(draft).is_empty() {
            (0, 0)
        } else if draft.classes.iter().any(|c| c == "Bard")
            && !draft.classes.iter().any(|c| c == "Thief")
        {
            BARD_POINTS
        } else {
            THIEF_POINTS
        }
    }

    pub fn rogue_points_left(&self, draft: &Draft) -> i64 {
        let (points, _) = self.rogue_points(draft);
        i64::from(points)
            - draft
                .rogue_points
                .iter()
                .map(|&p| i64::from(p))
                .sum::<i64>()
    }

    /// A rogue skill's score with the points added, in armor and without.
    pub fn rogue_skill_score(&self, draft: &Draft, skill: usize) -> (i32, i32) {
        let Some(&(_, base, unarmored)) = ROGUE_SKILLS.get(skill) else {
            return (0, 0);
        };
        let racial = self
            .race(&draft.race)
            .map_or(0, |r| r.thief_adjustments[skill]);
        let dexterity = self.adjusted(&draft.race, &draft.scores)[1];
        let dexterity = dexterity_skill_adjustments(dexterity)
            .get(skill)
            .copied()
            .unwrap_or(0);
        let score = base + racial + dexterity + draft.rogue_points[skill] as i32;
        (score, score + unarmored)
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
        spells.sort_by_cached_key(|s| crate::collate::sort_key(s));
        spells
    }

    pub fn is_wizard(&self, classes: &[String]) -> bool {
        classes
            .iter()
            .any(|c| self.class(c).is_some_and(|c| c.group == 3))
    }

    /// The native languages the wizard gives: with Skills & Powers the
    /// subrace's package; otherwise the race's own language. The races of
    /// human blood add the regional human dialect, and a race with no
    /// language gets Common. Core Rules 2.0 leaves them empty.
    pub fn native_languages(&self, draft: &Draft) -> Vec<String> {
        if !self.expansion {
            return Vec::new();
        }
        let race = self.race(&draft.race);
        let package = race
            .and_then(|r| r.subraces.iter().find(|s| s.name == draft.subrace))
            .map(|s| &s.languages)
            .filter(|l| draft.skills_and_powers && !l.is_empty());
        let mut languages = match (package, race) {
            (Some(languages), _) => languages.clone(),
            (None, Some(r)) => r.languages.clone(),
            _ => Vec::new(),
        };
        // The races of human blood speak the regional dialect as well
        // (Tsrp3.exe lists them next to the language's name).
        if HUMAN_BLOOD.contains(&draft.race.as_str())
            && !languages.iter().any(|l| l == HUMAN_LANGUAGE)
        {
            languages.push(HUMAN_LANGUAGE.into());
            languages.sort_by_cached_key(|n| crate::collate::sort_key(n));
        }
        if languages.is_empty() {
            languages.push("Common".into());
        }
        languages
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
        if let Some(problem) = self.languages_problem(draft) {
            return Some(problem);
        }
        if let Some(problem) = self.rogue_problem(draft) {
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

    /// The learned languages fit the proficiencies that teach them.
    fn languages_problem(&self, draft: &Draft) -> Option<String> {
        let slots = self.language_slots(draft);
        if draft.learned_languages.len() > slots {
            return Some(format!(
                "{} languages are learned; the proficiencies teach {slots}",
                draft.learned_languages.len()
            ));
        }
        let mut seen = HashSet::new();
        for language in &draft.learned_languages {
            if !self.languages.contains(language) {
                return Some(format!("{language} is not a language"));
            }
            if draft.languages.contains(language) || !seen.insert(language) {
                return Some(format!("{language} is known already"));
            }
        }
        None
    }

    /// The rogue skill points are spent within the limits.
    fn rogue_problem(&self, draft: &Draft) -> Option<String> {
        let (_, most) = self.rogue_points(draft);
        let skills = self.rogue_skills(draft);
        for (skill, &added) in draft.rogue_points.iter().enumerate() {
            if added == 0 {
                continue;
            }
            if !skills.iter().any(|s| s.1 == skill) {
                return Some(tr!(
                    "The character has no {skill} skill",
                    skill = tr(ROGUE_SKILLS[skill].0)
                ));
            }
            if added > most {
                return Some(tr!(
                    "{skill} may get at most {most} points",
                    skill = tr(ROGUE_SKILLS[skill].0),
                    most
                ));
            }
        }
        let left = self.rogue_points_left(draft);
        (left < 0).then(|| {
            tr!(
                "The rogue skills get {count} points too many",
                count = -left
            )
        })
    }

    /// Skills & Powers: the combat options are open to the character.
    pub fn combat_problem(&self, draft: &Draft) -> Option<String> {
        let classes = &draft.classes;
        for weapon in [&draft.weapon_of_choice, &draft.expertise]
            .into_iter()
            .flatten()
        {
            if !draft.weapon_proficiencies.contains(weapon) {
                return Some(tr!("{weapon} needs its weapon proficiency", weapon));
            }
        }
        if !draft.unarmed.is_empty() {
            if self.unarmed_cost(classes).is_none() {
                return Some(tr("Only fighters may specialize in unarmed combat").into());
            }
            if !draft.specializations.is_empty() {
                return Some(
                    tr("Unarmed combat and weapon specialization do not go together").into(),
                );
            }
            if let Some(u) = draft
                .unarmed
                .iter()
                .find(|u| !UNARMED.contains(&u.as_str()))
            {
                return Some(tr!("{u} is not an unarmed style", u));
            }
        }
        let armors = self.armors();
        if let Some(a) = draft
            .armor_proficiencies
            .iter()
            .find(|a| !armors.contains(a))
        {
            return Some(tr!("{a} is not armor", a));
        }
        if draft.fighting_styles.len() > MOST_FIGHTING_STYLES {
            return Some(tr!(
                "A character may know at most {count} fighting styles",
                count = MOST_FIGHTING_STYLES
            ));
        }
        let mut seen = HashSet::new();
        for style in &draft.fighting_styles {
            if self.style_cost(classes, style).is_none() {
                return Some(tr!(
                    "A {class} cannot learn {style}",
                    class = draft.class_title(),
                    style
                ));
            }
            if !seen.insert(style) {
                return Some(tr!("{style} is taken twice", style));
            }
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
                return Some(
                    "Only a single-classed fighter with Weapon specialization may specialize"
                        .into(),
                );
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
        if let Some(problem) = self.combat_problem(draft) {
            return Some(problem);
        }
        let left = self.points_left(draft);
        if left < 0 {
            return Some(format!(
                "The weapon proficiencies and combat options cost {} points more than there are",
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
            d.exceptional_strength = if !self.has_exceptional_strength(&draft.classes, adjusted[0])
            {
                0
            } else if draft.roll_method == RollMethod::PointsAndStrength {
                // Method X: only what was bought.
                draft.exceptional_strength.min(100)
            } else {
                draft.exceptional_strength.clamp(1, 100)
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
                s.abilities = self.chosen_class_abilities(draft, slot);
                s.unknown_0x270.clone_from(&class.schools);
                s.unknown_0x2d4.clone_from(&class.minor_spheres);
                s.unknown_0x338.clone_from(&class.major_spheres);
                d.unknown_0xc60[slot][0] = draft.hit_dice.get(slot).copied().unwrap_or(1);
            }
            // Each rogue skill the classes give, with the points added at
            // 1st level.
            d.rogue_skills.entries = self
                .rogue_skills(draft)
                .into_iter()
                .map(|(name, skill)| {
                    let mut values = [0; 100];
                    values[0] = draft.rogue_points[skill];
                    (name, values)
                })
                .collect();
        }

        // Proficiencies, languages, spells and equipment go in as the
        // lists editor adds them.
        // A Skills & Powers proficiency taken again is one entry with more
        // ranks; a Core Rules one is another entry.
        let ranks = self.nonweapon_ranks(draft);
        let nonweapon: Vec<String> = if draft.skills_and_powers {
            ranks.iter().map(|(n, _)| n.clone()).collect()
        } else {
            draft.nonweapon_proficiencies.clone()
        };
        let mut lists = CharacterLists {
            weapon_proficiencies: draft.weapon_proficiencies.clone(),
            nonweapon_proficiencies: nonweapon.clone(),
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
        {
            // The learned languages, one per slot or rank, go with the
            // proficiencies that teach them.
            let mut learned = draft.learned_languages.iter();
            character.data.nonweapon_details = nonweapon
                .iter()
                .map(|n| {
                    let count = if !teaches_languages(n) {
                        0
                    } else if draft.skills_and_powers {
                        ranks.iter().find(|r| &r.0 == n).map_or(1, |r| r.1)
                    } else {
                        1
                    };
                    Object::StringArray(learned.by_ref().take(count as usize).cloned().collect())
                })
                .collect();
        }
        if draft.skills_and_powers {
            // The lists went in as Core Rules ones; now the points.
            let d = &mut character.data;
            d.skills_and_powers = 1;
            d.nonweapon_costs = ranks
                .iter()
                .map(|(n, count)| self.nonweapon_rank_cost(draft, n, *count))
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
            d.unknown_0xac[4] = sorted(&draft.armor_proficiencies);
            d.unknown_0x124 = [
                draft.weapon_of_choice.clone().unwrap_or_default(),
                draft.expertise.clone().unwrap_or_default(),
                String::new(),
            ];
            d.unknown_0x134 = draft.shields.map(u32::from);
            if let Some(flags) = d.flags_0x1e54.as_mut() {
                for (flag, style) in flags.iter_mut().zip(UNARMED) {
                    *flag = u8::from(draft.unarmed.iter().any(|u| u == style));
                }
            }
            for (slot, style) in d.classes.unknown_0x50.iter_mut().zip(
                draft
                    .fighting_styles
                    .iter()
                    .filter_map(|s| FIGHTING_STYLES.iter().find(|f| f.0 == s)),
            ) {
                *slot = style.1;
            }
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
            default_abilities: if name == "Thief" {
                vec![
                    "Thief, Climb walls* (5)".into(),
                    "Thief, Pick pockets* (10)".into(),
                ]
            } else {
                vec![format!("{name}, Followers (5)")]
            },
            abilities: if name == "Fighter" {
                vec![
                    "Fighter, Leadership (5)".into(),
                    "Fighter, Weapon specialization (5)".into(),
                ]
            } else if name == "Thief" {
                vec!["Thief, Open locks* (10)".into()]
            } else {
                Vec::new()
            },
            points: if name == "Fighter" { 15 } else { 30 },
            // Warriors as the Expansion's records have them; the others
            // fall back to the book's numbers.
            nonweapon_points: if group == 0 { 6 } else { 0 },
            weapon_points: if group == 0 { 8 } else { 0 },
            weapon_cost: if group == 0 { 2 } else { 0 },
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
            thief_adjustments: if name == "Dwarf" {
                [0, 10, 15, 0, 0, 0, -10, -5]
            } else {
                [0; 8]
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
            (
                "Dwarf",
                vec![
                    vec!["Fighter"],
                    vec!["Thief"],
                    vec!["Fighter", "Mage"],
                    vec!["Fighter", "Thief"],
                ],
            ),
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
                class("Thief", 2000, 1, 6, [0, 9, 0, 0, 0, 0]),
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
                nonweapon("Modern Languages", 1, 2, 0, "Intelligence/Knowledge"),
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
                Ware {
                    name: "Full armor, banded mail".into(),
                    part_type: "Armor",
                    cost: 50_000,
                },
                Ware {
                    name: "Shield, small".into(),
                    part_type: "Armor",
                    cost: 300,
                },
            ],
            abilities: [
                "Fighter, Followers (5)",
                "Thief, Climb walls* (5)",
                "Thief, Pick pockets* (10)",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            languages: ["Centaur", "Dwarf", "Elf", "Gnome"]
                .into_iter()
                .map(String::from)
                .collect(),
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
                vec!["Thief".into()],
                vec!["Fighter".into(), "Mage".into()],
                vec!["Fighter".into(), "Thief".into()]
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
        assert_eq!(RollMethod::offered(true).len(), 12);
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
        // Each further rank costs 1: 5 + 19.
        d.nonweapon_proficiencies = vec!["Spellcraft".into(); 20];
        assert!(rules.problem(&d).unwrap().contains("traits cost"));
        let mut d = skills_and_powers_draft(&rules);
        d.specializations = vec!["Sword, long".into()];
        assert!(rules.problem(&d).unwrap().contains("proficiency"));
        let mut d = skills_and_powers_draft(&rules);
        d.class_abilities[0].retain(|a| !a.contains("specialization"));
        assert!(rules.problem(&d).unwrap().contains("fighter"));
        let mut d = skills_and_powers_draft(&rules);
        // 11 points left from the proficiencies, 8 for weapons.
        d.nonweapon_proficiencies = vec!["Spellcraft".into(); 3];
        d.weapon_proficiencies = vec!["Dagger".into(); 10];
        assert!(rules.problem(&d).unwrap().contains("combat options cost"));
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

    #[test]
    fn methods_six_and_ten_and_players_choice() {
        let rules = rules(true);
        let mut dice = Dice::seeded(7);
        let rolled = [6, 6, 5, 1, 2, 3, 4];
        assert_eq!(
            seven_dice_scores(&rolled, &[0, 1, 2, 3, 4, 5, 0]),
            Some([18, 14, 13, 9, 10, 11])
        );
        // Strength 8 + 6 + 6 + 5 is over 18.
        assert_eq!(seven_dice_scores(&rolled, &[0, 0, 0, 2, 3, 4, 5]), None);
        assert_eq!(exceptional_strength_points(18, 30), Some(3));
        assert_eq!(exceptional_strength_points(18, 100), Some(10));
        assert_eq!(exceptional_strength_points(17, 30), None);
        assert!(RollMethod::PointsAndStrength.divided());
        assert!(!RollMethod::offered(false).contains(&RollMethod::PlayersChoice));
        assert_eq!(
            RollMethod::offered(true).last(),
            Some(&RollMethod::PlayersChoice)
        );
        for _ in 0..50 {
            let scores = rules.players_choice("Human", &["Paladin".into()], &mut dice);
            assert!(scores[0] >= 12 && scores[2] >= 9 && scores[4] >= 13);
            assert!((17..=18).contains(&scores[5]));
            let scores = rules.players_choice("Dwarf", &["Fighter".into()], &mut dice);
            assert!(scores[0] >= 9 && scores[1] <= 17);
            assert!(
                rules
                    .races(&SETTINGS[0], &scores, false)
                    .contains(&"Dwarf".to_owned())
            );
        }
        assert_eq!(rules.best_scores("Dwarf"), [18, 17, 18, 18, 18, 18]);
        assert_eq!(rules.all_races(&SETTINGS[0], false), ["Dwarf", "Human"]);
    }

    #[test]
    fn languages_go_with_the_proficiencies_that_teach_them() {
        let (root, install) = installation("languages");
        let rules = rules(true);
        let mut d = draft(&rules);
        d.nonweapon_proficiencies = vec!["Modern Languages".into(); 2];
        d.learned_languages = vec!["Centaur".into(), "Elf".into()];
        assert_eq!(rules.language_slots(&d), 2);
        assert_eq!(rules.nonweapon_spent(&d), 2);
        assert_eq!(rules.problem(&d), None);
        let core = rules.build(&d).unwrap();
        assert_eq!(core.data.nonweapon_proficiencies.len(), 2);
        assert_eq!(
            core.data.nonweapon_details,
            [
                Object::StringArray(vec!["Centaur".into()]),
                Object::StringArray(vec!["Elf".into()])
            ]
        );
        let mut extra = d.clone();
        extra.learned_languages.push("Gnome".into());
        assert!(rules.problem(&extra).unwrap().contains("teach 2"));
        let mut known = d.clone();
        known.learned_languages = vec!["Dwarf".into()];
        assert!(rules.problem(&known).unwrap().contains("known already"));

        // Skills & Powers: one entry, 2 points and 1 for the second rank.
        let mut sp = skills_and_powers_draft(&rules);
        sp.nonweapon_proficiencies = vec!["Modern Languages".into(); 2];
        sp.learned_languages = vec!["Centaur".into(), "Elf".into()];
        assert_eq!(rules.nonweapon_spent(&sp), 3);
        assert_eq!(rules.problem(&sp), None);
        let file = install.create_character(&rules, &sp).unwrap();
        let c = install.character(&file).unwrap();
        assert_eq!(c.data.nonweapon_proficiencies, ["Modern Languages"]);
        assert_eq!(c.data.nonweapon_costs, [3]);
        assert_eq!(
            c.data.nonweapon_details,
            [Object::StringArray(vec!["Centaur".into(), "Elf".into()])]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rogue_skills_get_their_points() {
        let rules = rules(true);
        let mut d = draft(&rules);
        d.classes = vec!["Thief".into()];
        d.hit_dice = vec![4];
        d.spells.clear();
        d.weapon_proficiencies.clear();
        d.rogue_points[0] = 30;
        d.rogue_points[6] = 25;
        let skills = rules.rogue_skills(&d);
        assert_eq!(
            skills,
            [
                ("Thief, Climb walls* (5)".to_owned(), 6),
                ("Thief, Pick pockets* (10)".to_owned(), 0)
            ]
        );
        assert_eq!(rules.rogue_points(&d), THIEF_POINTS);
        assert_eq!(rules.rogue_points_left(&d), 5);
        assert_eq!(rules.problem(&d), None);
        // Pick pockets 15, Dexterity 14 adds nothing; 20 without armor.
        assert_eq!(rules.rogue_skill_score(&d, 0), (45, 50));
        // The dwarf's -10 to climbing walls.
        assert_eq!(rules.rogue_skill_score(&d, 6), (75, 85));
        let c = rules.build(&d).unwrap();
        let entries: Vec<(&str, u32)> = c
            .data
            .rogue_skills
            .entries
            .iter()
            .map(|(n, v)| (n.as_str(), v[0]))
            .collect();
        assert_eq!(
            entries,
            [
                ("Thief, Climb walls* (5)", 25),
                ("Thief, Pick pockets* (10)", 30)
            ]
        );
        let mut d2 = d.clone();
        d2.rogue_points[0] = 31;
        assert!(rules.problem(&d2).unwrap().contains("at most 30"));
        let mut d3 = d.clone();
        d3.rogue_points[1] = 5;
        assert!(rules.problem(&d3).unwrap().contains("Open locks"));
        let mut d4 = d.clone();
        d4.rogue_points[6] = 30;
        d4.rogue_points[0] = 30;
        d4.rogue_points[1] = 0;
        assert_eq!(rules.rogue_points_left(&d4), 0);
        // Skills & Powers: only the skills bought.
        let mut sp = skills_and_powers_draft(&rules);
        sp.classes = vec!["Thief".into()];
        sp.class_abilities = vec![vec!["Thief, Open locks* (10)".into()]];
        sp.specializations.clear();
        assert_eq!(
            rules.rogue_skills(&sp),
            [("Thief, Open locks* (10)".to_owned(), 1)]
        );
    }

    #[test]
    fn combat_options_cost_and_are_written_as_in_the_original() {
        let (root, install) = installation("combat");
        let rules = rules(true);
        let mut d = skills_and_powers_draft(&rules);
        d.weapon_proficiencies = vec!["Dagger".into(), "Sword, long".into()];
        d.weapon_of_choice = Some("Dagger".into());
        d.expertise = Some("Dagger".into());
        d.shields = [false, true, false, false];
        d.armor_proficiencies = vec!["Full armor, banded mail".into()];
        d.fighting_styles = vec!["Missile".into(), "Weapon and Shield".into()];
        let classes = d.classes.clone();
        assert_eq!(rules.weapon_of_choice_cost(&classes), 2);
        // Expertise in the weapon of choice costs 1 less.
        assert_eq!(rules.expertise_cost(&d), 1);
        assert_eq!(rules.shield_cost(&classes), 2);
        assert_eq!(rules.style_cost(&classes, "Two Weapon"), Some(3));
        assert_eq!(
            rules.style_cost(&classes, "One-handed Weapon (+2)"),
            Some(4)
        );
        assert_eq!(rules.style_cost(&["Mage".into()], "Missile"), None);
        assert_eq!(
            rules.style_cost(&["Mage".into()], "Two-handed Weapon"),
            Some(3)
        );
        // Dagger specialization 2, choice 2, expertise 1, shield and armor
        // 2 each, styles 2 each.
        assert_eq!(rules.combat_spent(&d), 2 + 2 + 1 + 4 + 4);
        assert_eq!(rules.armors(), ["Full armor, banded mail"]);
        assert_eq!(rules.problem(&d), None);
        let file = install.create_character(&rules, &d).unwrap();
        let data = install.character(&file).unwrap().data;
        assert_eq!(
            data.unknown_0x124,
            ["Dagger".to_owned(), "Dagger".into(), String::new()]
        );
        assert_eq!(data.unknown_0x134, [0, 1, 0, 0]);
        assert_eq!(data.unknown_0xac[4], ["Full armor, banded mail"]);
        assert_eq!(data.classes.unknown_0x50[..3], [1997, 1994, 0]);
        assert_eq!(data.flags_0x1e54, Some([0; 5]));

        // A multi-classed fighter may not specialize in a weapon at 1st
        // level, but may in punching, at 4 points.
        let mut multi = d.clone();
        multi.classes = vec!["Fighter".into(), "Thief".into()];
        multi.class_abilities = vec![d.class_abilities[0].clone(), Vec::new()];
        multi.hit_dice = vec![7, 3];
        assert_eq!(rules.specialization_cost(&multi), None);
        assert!(rules.problem(&multi).unwrap().contains("single-classed"));
        multi.specializations.clear();
        multi.unarmed = vec!["Punching".into()];
        assert_eq!(rules.unarmed_cost(&multi.classes), Some(4));
        assert_eq!(rules.problem(&multi), None);
        let file = install.create_character(&rules, &multi).unwrap();
        let data = install.character(&file).unwrap().data;
        assert_eq!(data.flags_0x1e54, Some([1, 0, 0, 0, 0]));

        let mut bad = d.clone();
        bad.weapon_of_choice = Some("Club".into());
        assert!(rules.problem(&bad).unwrap().contains("Club"));
        let mut bad = d.clone();
        bad.unarmed = vec!["Wrestling".into()];
        assert!(rules.problem(&bad).unwrap().contains("do not go together"));
        let mut bad = d.clone();
        bad.fighting_styles.push("Missile".into());
        assert!(rules.problem(&bad).unwrap().contains("twice"));
        fs::remove_dir_all(root).unwrap();
    }
}
