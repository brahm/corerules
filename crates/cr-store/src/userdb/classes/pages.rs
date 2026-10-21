//! The class editor IDD 192 and the dialogs its buttons open, described by
//! the offsets of `CClassOb` their accessors read and write: the group
//! pages (133 to 137), Ability Scores (158), Alignment (255), Weapons
//! (159), Armor (168), Major and Minor Spheres and the More page's
//! Abilities and Schools (338), Magical Items, Required and Bonus NWPs
//! (178), Crossover Penalty (152), Class Abilities (207), More (336) with
//! Reminders (175), Limit Spells and Kits, Non-standard Bonuses (244),
//! Levels (335) with its Ability Score bonus (221), and Description.
//!
//! Levels edits one level at a time in the original (a spin control picks
//! the level); here the per-level numbers are tables with one row per
//! level up to the class's maximum level.

use cr_formats::Object;
use cr_formats::classes::{ClassData, KitBonus};

use super::super::slots;
use super::levels::{self, AutoFill};
use super::{ABILITIES, ALIGNMENTS, Class, GROUPS, data, data_mut, group};
use crate::userdb::{FieldDesc, FieldKind, Options, UserKind, Value};
use cr_i18n::tr_noop;

pub(crate) fn value(r: &Class, off: usize) -> Value {
    data(r)
        .and_then(|d| slots::class(d, off))
        .map_or(Value::Int(0), slots::Slot::value)
}

fn int(r: &Class, off: usize) -> i64 {
    value(r, off).int()
}

pub(crate) fn store(r: &mut Class, off: usize, v: &Value) {
    if let Some(s) = data_mut(r).and_then(|d| slots::class_mut(d, off)) {
        s.set(v);
    }
}

/// A control on class member `off`.
macro_rules! at {
    ($page:expr, $label:expr, $kind:expr, $off:expr) => {
        FieldDesc::new(
            $page,
            $label,
            $kind,
            |r: &Class, off| value(r, off),
            |r, off, v, _| store(r, off, v),
        )
        .arg($off)
    };
}

macro_rules! check {
    ($page:expr, $label:expr, $off:expr) => {
        at!($page, $label, FieldKind::Check, $off)
    };
}

macro_rules! int {
    ($page:expr, $label:expr, $min:expr, $max:expr, $off:expr) => {
        at!(
            $page,
            $label,
            FieldKind::Int {
                min: $min,
                max: $max
            },
            $off
        )
    };
}

macro_rules! list {
    ($page:expr, $label:expr, $source:expr, $off:expr) => {
        at!(
            $page,
            $label,
            FieldKind::Dual(Options::Source($source)),
            $off
        )
    };
}

const MAIN: &str = tr_noop!("Class");
const ABILITY: &str = tr_noop!("Ability Scores");
const ALIGNMENT: &str = tr_noop!("Alignment");
const WEAPONS: &str = tr_noop!("Weapons");
const ARMOR: &str = tr_noop!("Armor");
const MAJOR: &str = tr_noop!("Major Spheres");
const MINOR: &str = tr_noop!("Minor Spheres");
const ITEMS: &str = tr_noop!("Magical Items");
const REQUIRED: &str = tr_noop!("Req. Profs");
const BONUS: &str = tr_noop!("Bonus Profs");
const CROSSOVER: &str = tr_noop!("Crossover Penalty");
const LEVELS: &str = tr_noop!("Levels");
const ABILITIES_PAGE: &str = tr_noop!("Class Abilities");
const MORE: &str = tr_noop!("More");
const DESCRIPTION: &str = tr_noop!("Description");
const NWP: &str = "Non-Weapon Proficiencies";

/// IDD 168's radio (+0x3040).
const ARMOR_CHOICES: [&str; 3] = [
    tr_noop!("Any"),
    tr_noop!("Best Armor Class Allowed"),
    tr_noop!("Only Allow Armor From List Below"),
];

/// IDD 159's category combo (+0x3000, a drop-down list, unsorted): these
/// two (0x6f925c), then the weapon damage types the program builds at
/// start-up (0x678788, its list +0x20c). A class whose text is in none of
/// them shows "Any", as OnInitDialog (0x6f8c5d) selects it.
const WEAPON_CATEGORIES: [&str; 10] = [
    "Any",
    "Only from list below",
    "None",
    "P",
    "S",
    "B",
    "P/B",
    "P/S",
    "B/S",
    "P/S/B",
];

const GENDERS: [&str; 3] = [
    tr_noop!("Male and Female"),
    tr_noop!("Male Only"),
    tr_noop!("Female Only"),
];

pub(crate) fn fields() -> Vec<FieldDesc<Class>> {
    let mut f = vec![
        FieldDesc::new(
            MAIN,
            tr_noop!("Name"),
            FieldKind::Text { max: 18 },
            |r: &Class, _| Value::Text(r.name().to_owned()),
            |r, _, v, _| super::Classes::set_name(r, &v.text()),
        ),
        FieldDesc::new(
            MAIN,
            tr_noop!("Group"),
            FieldKind::Choice(Options::Fixed(&GROUPS)),
            |r: &Class, _| Value::Int(group(r) as i64),
            |r, _, v, _| store(r, 0x14, &Value::Int(v.int().clamp(0, 4))),
        )
        .group(tr_noop!("Select a Group")),
        int!(MAIN, tr_noop!("Character Points"), 0, 100_000, 0x54)
            .group(tr_noop!("Character Points (Starting)")),
        check!(MAIN, tr_noop!("Allow any magical Items"), 0x48ac),
    ];
    group_pages(&mut f);
    ability_pages(&mut f);
    equipment_pages(&mut f);
    proficiency_pages(&mut f);
    more_page(&mut f);
    levels_page(&mut f);
    f.push(
        FieldDesc::new(
            DESCRIPTION,
            tr_noop!("Description"),
            FieldKind::MultiLine,
            |r: &Class, off| value(r, off),
            |r, off, v, _| store(r, off, v),
        )
        .arg(0x8),
    );
    f
}

/// IDD 133 to 137, shown for the class's group.
fn group_pages(f: &mut Vec<FieldDesc<Class>>) {
    const W: &str = tr_noop!("Warrior");
    f.extend(
        [
            check!(
                W,
                tr_noop!("Weapon Specialization CP cost of 2 when single classed.  Allows specialization \
                 at first level. (Fighter)"),
                0x2791
            ),
            check!(
                W,
                tr_noop!("Allow Weapon Specialization at first level, second level for multi-classed \
                 (Fighter)"),
                0x2792
            ),
            check!(
                W,
                tr_noop!("Allow Weapon Mastery at fifth level, sixth level for multi-classed (Fighter)"),
                0x2794
            ),
            check!(
                W,
                tr_noop!("Weapon Mastery CP cost of 2 when single classed (Fighter)"),
                0x2793
            ),
            check!(
                W,
                tr_noop!("Ignore additional CP cost for Two Weapon Fighting style (Ranger)"),
                0x2795
            ),
            check!(W, tr_noop!("Allow unarmed combat (Fighter)"), 0x2796),
            check!(
                W,
                tr_noop!("Allow Weapon Specialization for Core Rules character (Fighter)"),
                0x2797
            ),
            check!(
                W,
                tr_noop!("Receive the special Warrior Constitution Hit Point adjustment"),
                0x279c
            ),
            int!(W, tr_noop!("Gold"), 0, 1_000_000_000, 0x2798).group(
                tr_noop!("Enter the maximum amount of gold this class may keep, 0 for no limit.  (Paladin)"),
            ),
        ]
        .map(|d| d.visible(|r| group(r) == 0)),
    );
    const R: &str = tr_noop!("Rogue");
    f.extend(
        [
            check!(
                R,
                tr_noop!(
                    "Only half of the points allocated per level may be used on any one skill"
                ),
                0x278c
            ),
            check!(R, tr_noop!("Candidate for Rogue Skills"), 0x27a4),
            check!(
                R,
                tr_noop!("Use the Thief Average Ability Table from the DMG for base chance"),
                0x27a5
            ),
        ]
        .map(|d| d.visible(|r| group(r) == 1)),
    );
    const P: &str = tr_noop!("Priest");
    f.extend(
        [
            check!(P, tr_noop!("Cannot use an edged weapon (Cleric)"), 0x27a0),
            check!(
                P,
                tr_noop!("Cannot purchase Weapon Groups (Cleric)"),
                0x27a1
            ),
            check!(
                P,
                tr_noop!(
                    "Gain access to 8th level wizard spells at 16th level and 9th level wizard \
                 spells at 18th level"
                ),
                0x278e
            ),
            check!(
                P,
                tr_noop!(
                    "No more than three-quarters of spells available can be taken as wizard spells"
                ),
                0x278f
            ),
            check!(
                P,
                tr_noop!(
                    "Receive double the bonus spells they are entitled to for a high Wisdom score"
                ),
                0x2790
            ),
            int!(
                P,
                tr_noop!("Turn undead as if this many level(s) higher than the actual level"),
                0,
                100,
                0x485c
            ),
            check!(
                P,
                tr_noop!("Always use the priest's THAC0 for attack rolls"),
                0x486c
            ),
        ]
        .map(|d| d.visible(|r| group(r) == 2)),
    );
    const Z: &str = tr_noop!("Wizard");
    let specialist = |r: &Class| int(r, 0x279d) != 0;
    f.extend(
        [
            check!(Z, tr_noop!("Cannot wear armor (Mage)"), 0x27a2),
            check!(
                Z,
                tr_noop!("Allow the effects of the class ability \"Armored Wizard\" (Mage)"),
                0x27a3
            ),
            list!(
                Z,
                tr_noop!("Wizards may not cast spells from selected schools"),
                "Schools Of Magic",
                0x4898
            ),
            // Unchecking empties the school combo (6f5cd7), which OK
            // then stores.
            FieldDesc::new(
                Z,
                tr_noop!("Is a Specialist (Applied to item restrictions)"),
                FieldKind::Check,
                |r: &Class, off| value(r, off),
                |r, off, v, _| {
                    store(r, off, v);
                    if !v.bool() {
                        store(r, 0x2788, &Value::Text(String::new()));
                    }
                },
            )
            .arg(0x279d)
            .group(tr_noop!("Specialist Information")),
            FieldDesc::new(
                Z,
                tr_noop!("Non-standard bonuses and penalties to learning spells"),
                FieldKind::Pairs {
                    options: Options::Source("Schools Of Magic"),
                    number: tr_noop!("% adjustment"),
                    min: -100,
                    max: 100,
                },
                |r: &Class, _| {
                    Value::Pairs(
                        data(r)
                            .map(|d| {
                                d.unknown_0x4884
                                    .iter()
                                    .filter_map(|o| match o {
                                        Object::KitBonus(b) => {
                                            Some((b.name.clone(), i64::from(b.unknown_0xc)))
                                        }
                                        _ => None,
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    )
                },
                |r, _, v, _| {
                    if let Some(d) = data_mut(r) {
                        d.unknown_0x4884 = v
                            .pairs()
                            .into_iter()
                            .map(|(name, n)| {
                                Object::KitBonus(KitBonus {
                                    version: 1,
                                    name,
                                    unknown_0x8: 0,
                                    unknown_0xc: n as i32,
                                })
                            })
                            .collect();
                    }
                },
            )
            .group(tr_noop!("Specialist Information"))
            .enabled(specialist),
            at!(
                Z,
                tr_noop!("Specialist School"),
                FieldKind::Combo {
                    options: Options::Source("Schools Of Magic"),
                    free: false
                },
                0x2788
            )
            .group(tr_noop!("Specialist Information"))
            .enabled(specialist),
        ]
        .map(|d| d.visible(|r| group(r) == 3)),
    );
    const S: &str = tr_noop!("Psionicist");
    f.extend(
        [
            check!(S, tr_noop!("Apply Item restriction as 5 CP"), 0x279e),
            check!(S, tr_noop!("Apply Item restriction as 10 CP"), 0x279f),
        ]
        .map(|d| d.visible(|r| group(r) == 4)),
    );
}

/// IDD 158 Ability Score Requirements, 255 Alignments Allowed and 152
/// NWP crossover.
fn ability_pages(f: &mut Vec<FieldDesc<Class>>) {
    for (i, ability) in ABILITIES.iter().enumerate() {
        f.push(int!(ABILITY, tr_noop!("Minimum"), 0, 25, 0x1c + 4 * i).group(ability));
        f.push(check!(ABILITY, tr_noop!("Prime Requisite"), 0x34 + 4 * i).group(ability));
    }
    f.extend([
        check!(ABILITY, tr_noop!("Allow Exceptional Strength"), 0x3554).group(tr_noop!("Strength")),
        check!(
            ABILITY,
            tr_noop!("Permanently add up to 2 points to STR (or 20 to XSTR) at first level"),
            0x486e
        )
        .group(tr_noop!("Strength")),
    ]);
    for (i, alignment) in ALIGNMENTS.iter().enumerate() {
        f.push(
            FieldDesc::new(
                ALIGNMENT,
                alignment,
                FieldKind::Check,
                |r: &Class, i| {
                    Value::Bool(
                        data(r).is_some_and(|d| d.alignments.iter().any(|a| a == ALIGNMENTS[i])),
                    )
                },
                |r, i, v, _| {
                    if let Some(d) = data_mut(r) {
                        let mut on: Vec<bool> = ALIGNMENTS
                            .iter()
                            .map(|a| d.alignments.iter().any(|x| x == a))
                            .collect();
                        on[i] = v.bool();
                        d.alignments = ALIGNMENTS
                            .iter()
                            .zip(on)
                            .filter(|(_, on)| *on)
                            .map(|(a, _)| (*a).to_owned())
                            .collect();
                    }
                },
            )
            .arg(i)
            .group(tr_noop!("Alignments Allowed")),
        );
    }
    for (i, g) in GROUPS.iter().enumerate() {
        f.push(check!(CROSSOVER, g, 0x3044 + i).group(tr_noop!("Class Groups")));
    }
    for (i, a) in ABILITIES.iter().enumerate() {
        f.push(check!(CROSSOVER, a, 0x3049 + i).group(tr_noop!("Relevant Ability Scores")));
    }
}

/// The armor radio: Any and Best AC empty the list (after a question
/// the GUI does not ask), Any and List put -10 back in the AC.
fn set_armor(r: &mut Class, off: usize, v: &Value, _: &crate::userdb::Sources) {
    let choice = v.int().clamp(0, 2);
    store(r, off, &Value::Int(choice));
    if choice != 2 {
        store(r, 0x302c, &Value::List(Vec::new()));
    }
    if choice != 1 {
        store(r, 0x2ff8, &Value::Int(-10));
    }
}

/// IDD 159 Weapons, 168 Armor, 338 (spheres) and the Magical Items
/// AddRemove.
fn equipment_pages(f: &mut Vec<FieldDesc<Class>>) {
    f.extend([
        at!(
            WEAPONS,
            tr_noop!("Allow wepons from the selected category"),
            FieldKind::Combo {
                options: Options::Fixed(&WEAPON_CATEGORIES),
                free: false
            },
            0x3000
        ),
        list!(
            WEAPONS,
            tr_noop!("And always allow the following"),
            "Weapons",
            0x3004
        ),
        check!(ARMOR, tr_noop!("Do not allow shield"), 0x2ffc),
        FieldDesc::new(
            ARMOR,
            tr_noop!("Select Allowed Armor"),
            FieldKind::Choice(Options::Fixed(&ARMOR_CHOICES)),
            |r: &Class, off| value(r, off),
            set_armor,
        )
        .arg(0x3040),
        int!(ARMOR, tr_noop!("Best Armor Class Allowed"), -10, 10, 0x2ff8)
            .enabled(|r| int(r, 0x3040) == 1),
        list!(
            ARMOR,
            tr_noop!("And always allow the following"),
            "Armor",
            0x302c
        )
        .enabled(|r| int(r, 0x3040) == 2),
        list!(MAJOR, tr_noop!("Major Spheres"), "Spheres", 0x2738),
        list!(MINOR, tr_noop!("Minor Spheres"), "Spheres", 0x274c),
        list!(
            ITEMS,
            tr_noop!("Same as the following classes"),
            "standard-classes",
            0x3018
        )
        .enabled(|r| int(r, 0x48ac) == 0),
        list!(
            ABILITIES_PAGE,
            tr_noop!("Selected Abilities"),
            "Class Abilities",
            0x5c
        ),
    ]);
}

/// IDD 178 for required (2200) and free (1026) NWPs.
fn proficiency_pages(f: &mut Vec<FieldDesc<Class>>) {
    f.extend([
        list!(
            REQUIRED,
            tr_noop!("All of these NWPs must be purchased"),
            NWP,
            0x2fcc
        ),
        list!(
            REQUIRED,
            tr_noop!("Must purchase as many as indicated below"),
            NWP,
            0x2fe0
        ),
        int!(
            REQUIRED,
            tr_noop!("Pick from the list above"),
            1,
            100,
            0x2ff4
        ),
        list!(
            BONUS,
            tr_noop!("All of these NWPs will be free"),
            NWP,
            0x2fa0
        ),
        list!(
            BONUS,
            tr_noop!("Allowed to pick as many as indicated below"),
            NWP,
            0x2fb4
        ),
        int!(BONUS, tr_noop!("Pick from the list above"), 1, 100, 0x2fc8),
    ]);
}

/// IDD 336 More, with Abilities and Schools (338), Reminders (175),
/// Limit Spells and Kits.
fn more_page(f: &mut Vec<FieldDesc<Class>>) {
    const HP: &str = tr_noop!("Hit Points (Use the first Hit Die as the per level Hit Die)");
    const NWPS: &str = tr_noop!("Non-Weapon Proficiencies");
    const WPS: &str = tr_noop!("Weapon Proficiencies");
    const MAGIC: &str = tr_noop!("Magic");
    const ROGUE: &str = tr_noop!("Rogue Skills");
    const REMINDERS: &str = tr_noop!("Reminders");
    f.push(auto_fill(MORE, AutoFill::More));
    f.extend([
        int!(MORE, tr_noop!("Starting HP: dice"), 1, 100, 0x94).group(HP),
        int!(
            MORE,
            tr_noop!("Starting HP: die sides"),
            -1_000_000,
            1_000_000,
            0x84
        )
        .group(HP),
        int!(MORE, tr_noop!("Plus dice"), 0, 100, 0x9c).group(HP),
        int!(MORE, tr_noop!("Plus die sides"), 0, 100, 0x98).group(HP),
        int!(MORE, tr_noop!("Plus"), 0, 1000, 0x90).group(HP),
        check!(MORE, tr_noop!("Do not average HPs if Multiclass"), 0x4864).group(HP),
        int!(
            MORE,
            tr_noop!("Bonus hit points at each level"),
            -10000,
            10000,
            0x4860
        )
        .group(HP),
        int!(
            MORE,
            tr_noop!("Maximum hit dice rolled"),
            -1_000_000,
            1_000_000,
            0x88
        )
        .group(HP),
        int!(
            MORE,
            tr_noop!("Hit point adds"),
            -1_000_000,
            1_000_000,
            0x8c
        )
        .group(HP),
        int!(
            MORE,
            tr_noop!("Starting Character Points"),
            -1_000_000,
            1_000_000,
            0xbc
        )
        .group(NWPS),
        check!(MORE, tr_noop!("Combine Slots for Multi-classed"), 0x486d).group(NWPS),
        int!(
            MORE,
            tr_noop!("Starting Slots"),
            -1_000_000,
            1_000_000,
            0xc0
        )
        .group(NWPS),
        int!(MORE, tr_noop!("Extra slot(s)"), 0, 100, 0xc8).group(NWPS),
        int!(
            MORE,
            tr_noop!("Every # levels"),
            -1_000_000,
            1_000_000,
            0xc4
        )
        .group(NWPS),
        int!(
            MORE,
            tr_noop!("Starting Character Points"),
            -1_000_000,
            1_000_000,
            0xcc
        )
        .group(WPS),
        int!(
            MORE,
            tr_noop!("Character Point cost of a slot"),
            -1_000_000,
            1_000_000,
            0xd0
        )
        .group(WPS),
        int!(
            MORE,
            tr_noop!("Starting Slots"),
            -1_000_000,
            1_000_000,
            0xd4
        )
        .group(WPS),
        int!(
            MORE,
            tr_noop!("Extra slot every # levels"),
            -1_000_000,
            1_000_000,
            0xd8
        )
        .group(WPS),
        int!(MORE, tr_noop!("Non-proficiency penalty"), -20, 20, 0xdc).group(WPS),
        int!(
            MORE,
            tr_noop!("Familiarity penalty"),
            -1_000_000,
            1_000_000,
            0xe0
        )
        .group(WPS),
        check!(MORE, tr_noop!("Wizard"), 0x2778).group(MAGIC),
        check!(MORE, tr_noop!("Priest"), 0x2774).group(MAGIC),
        check!(MORE, tr_noop!("Priestly Wizard"), 0x277c).group(MAGIC),
        check!(MORE, tr_noop!("Wizardly Priest"), 0x2780).group(MAGIC),
        check!(MORE, tr_noop!("May not change default Spheres"), 0x2784).group(MAGIC),
        at!(
            MORE,
            tr_noop!("Receive one extra spell per spell level from the following sphere"),
            FieldKind::Combo {
                options: Options::Source("Spheres"),
                free: false
            },
            0x4868
        )
        .group(MAGIC),
        check!(MORE, tr_noop!("Double starting gold"), 0x4865),
        at!(
            MORE,
            tr_noop!("Gender"),
            FieldKind::Choice(Options::Fixed(&GENDERS)),
            0x3550
        ),
        int!(
            MORE,
            tr_noop!("Starting Points"),
            -1_000_000,
            1_000_000,
            0xb4
        )
        .group(ROGUE),
        int!(MORE, tr_noop!("Per Level"), -1_000_000, 1_000_000, 0xb8).group(ROGUE),
        at!(
            MORE,
            tr_noop!("Abbreviation"),
            FieldKind::Text { max: 2 },
            0x354c
        ),
        list!(
            MORE,
            tr_noop!("Default abilities"),
            "Class Abilities",
            0x2724
        )
        .group(tr_noop!("Default")),
        list!(MORE, tr_noop!("Schools"), "Schools Of Magic", 0x2760).group(tr_noop!("Default")),
        list!(
            MORE,
            tr_noop!("Priest spells which this class may not cast"),
            "priest-spells",
            0x3558
        ),
        list!(
            MORE,
            tr_noop!("Standard Kits which characters of this class may not select"),
            "Character Kits",
            0x48b0
        ),
    ]);
    for (label, off) in [
        (
            tr_noop!(
                "Display the following message when entering the Non-Weapon Proficiency screen"
            ),
            0x3540,
        ),
        (
            tr_noop!("Display the following message when entering the Weapon Proficiency screen"),
            0x3544,
        ),
        (
            tr_noop!("Display the following message when entering the Inventory screen"),
            0x353c,
        ),
        (
            tr_noop!("Display the following message when entering the Language screen"),
            0x3548,
        ),
    ] {
        f.push(at!(MORE, label, FieldKind::MultiLine, off).group(REMINDERS));
    }
}

/// The highest level the Levels tables show: the class's maximum level
/// (+0x58), at least 1.
fn max_level(d: &ClassData) -> usize {
    d.unknown_0x54[1].clamp(1, 100) as usize
}

/// What Levels' OK (6f2163) does to the experience table: level 1 needs
/// 0, and each level needs at least one point more than the one before.
fn fix_experience(d: &mut ClassData) {
    d.experience[1] = 0;
    for level in 2..=100 {
        if d.experience[level] <= d.experience[level - 1] {
            d.experience[level] = d.experience[level - 1] + 1;
        }
    }
}

const LEVEL_COLUMNS: [&str; 10] = [
    tr_noop!("Required XP"),
    tr_noop!("THAC0"),
    tr_noop!("AC"),
    tr_noop!("Attacks"),
    tr_noop!("per rounds"),
    tr_noop!("Paralyzation, Poison, or Death Magic"),
    tr_noop!("Rod, Staff, or Wand"),
    tr_noop!("Petrification or Polymorph"),
    tr_noop!("Breath Weapon"),
    tr_noop!("Spell"),
];

/// The cells of the Levels table for `level`.
fn level_row(d: &ClassData, level: usize) -> Vec<i64> {
    let at = |a: Option<&[i32]>| a.map_or(0, |a| i64::from(a[level]));
    let mut row = vec![
        i64::from(d.experience[level]),
        at(d.unknown_0x27c.as_ref().map(|a| &a[..])),
        at(d.unknown_0x3380.as_ref().map(|a| &a[..])),
        at(d.unknown_0x3050.as_ref().map(|a| &a[..])),
        at(d.unknown_0x31e8.as_ref().map(|a| &a[..])),
    ];
    let saves = d.unknown_0x27a8.map_or([0; 5], |s| s[level]);
    row.extend(saves.map(i64::from));
    row
}

fn set_level_row(d: &mut ClassData, level: usize, row: &[i64]) {
    let cell = |c: usize| row.get(c).copied().unwrap_or(0) as i32;
    d.experience[level] = cell(0);
    if let Some(a) = d.unknown_0x27c.as_mut() {
        a[level] = cell(1);
    }
    if let Some(a) = d.unknown_0x3380.as_mut() {
        a[level] = cell(2);
    }
    if let Some(a) = d.unknown_0x3050.as_mut() {
        a[level] = cell(3);
    }
    if let Some(a) = d.unknown_0x31e8.as_mut() {
        a[level] = cell(4);
    }
    if let Some(s) = d.unknown_0x27a8.as_mut() {
        for (i, save) in s[level].iter_mut().enumerate() {
            *save = cell(5 + i);
        }
    }
}

const SPELL_COLUMNS: [&str; 20] = [
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "10",
    tr_noop!("Alt. 1"),
    tr_noop!("Alt. 2"),
    tr_noop!("Alt. 3"),
    tr_noop!("Alt. 4"),
    tr_noop!("Alt. 5"),
    tr_noop!("Alt. 6"),
    tr_noop!("Alt. 7"),
    tr_noop!("Alt. 8"),
    tr_noop!("Alt. 9"),
    tr_noop!("Alt. 10"),
];

/// Ability score bonus (IDD 221) columns, Strength to Charisma, and where
/// each sits in a level's twelve numbers (alphabetical: Cha, Con, Dex,
/// Int, Str, Wis adjustments, then the same maxima).
const BONUS_COLUMNS: [&str; 12] = [
    tr_noop!("Str"),
    tr_noop!("Dex"),
    tr_noop!("Con"),
    tr_noop!("Int"),
    tr_noop!("Wis"),
    tr_noop!("Cha"),
    tr_noop!("Str max"),
    tr_noop!("Dex max"),
    tr_noop!("Con max"),
    tr_noop!("Int max"),
    tr_noop!("Wis max"),
    tr_noop!("Cha max"),
];
const BONUS_INDEX: [usize; 6] = [4, 2, 1, 3, 5, 0];

fn bonus_index(column: usize) -> usize {
    BONUS_INDEX[column % 6] + 6 * (column / 6)
}

/// A Levels table: `get` reads a level's cells, `set` writes them; every
/// change goes through Levels' OK.
macro_rules! table {
    ($label:expr, $columns:expr, $min:expr, $max:expr, $get:expr, $set:expr) => {
        FieldDesc::new(
            LEVELS,
            $label,
            FieldKind::Grid {
                columns: &$columns,
                first: 1,
                min: $min,
                max: $max,
            },
            |r: &Class, _| {
                Value::Grid(
                    data(r)
                        .map(|d| (1..=max_level(d)).map(|l| $get(d, l)).collect())
                        .unwrap_or_default(),
                )
            },
            |r, _, v, _| {
                if let Some(d) = data_mut(r) {
                    for (i, row) in v.grid().iter().enumerate().take(100) {
                        $set(d, i + 1, row);
                    }
                    fix_experience(d);
                }
            },
        )
    };
}

/// IDD 335 Level Details and 221 Ability score bonus.
/// Auto Fill: copies `fill`'s page from the class the setter is given.
fn auto_fill(page: &'static str, fill: AutoFill) -> FieldDesc<Class> {
    FieldDesc::new(
        page,
        tr_noop!("Auto Fill"),
        FieldKind::AutoFill,
        |_: &Class, _| Value::Text(String::new()),
        |r, arg, v, sources| {
            let fill = if arg == 0 {
                AutoFill::Levels
            } else {
                AutoFill::More
            };
            let name = v.text();
            let from = sources.class_records.iter().find(|c| c.name() == name);
            if let (Some(to), Some(from)) = (data_mut(r), from.and_then(data)) {
                levels::auto_fill(to, from, fill);
            }
        },
    )
    .arg(match fill {
        AutoFill::Levels => 0,
        AutoFill::More => 1,
    })
}

/// What Level Details edits beyond its numbers: free spells, NWPs and
/// class abilities by level.
fn level_details(f: &mut Vec<FieldDesc<Class>>) {
    f.extend([
        FieldDesc::new(
            LEVELS,
            tr_noop!("Free Spells"),
            FieldKind::FreeSpells,
            |r: &Class, _| {
                Value::FreeSpells(
                    r.name().to_owned(),
                    data(r).map(levels::free_spells).unwrap_or_default(),
                )
            },
            |r, _, v, _| {
                if let Some(d) = data_mut(r) {
                    levels::set_free_spells(d, v.free_spells());
                }
            },
        ),
        FieldDesc::new(
            LEVELS,
            tr_noop!("NWPs by level"),
            FieldKind::LevelNwps(Options::Source("Non-Weapon Proficiencies")),
            |r: &Class, _| Value::LevelNwps(data(r).map(levels::level_nwps).unwrap_or_default()),
            |r, _, v, _| {
                if let Some(d) = data_mut(r) {
                    levels::set_level_nwps(d, v.level_nwps());
                }
            },
        ),
        FieldDesc::new(
            LEVELS,
            tr_noop!("Class abilities by level"),
            FieldKind::LevelAbilities(Options::Source("Class Abilities")),
            |r: &Class, _| Value::Pairs(data(r).map(levels::level_abilities).unwrap_or_default()),
            |r, _, v, _| {
                if let Some(d) = data_mut(r) {
                    levels::set_level_abilities(d, &v.pairs());
                }
            },
        )
        .visible(|r| data(r).is_some_and(|d| d.level_abilities.is_some())),
    ]);
}

fn levels_page(f: &mut Vec<FieldDesc<Class>>) {
    f.push(auto_fill(LEVELS, AutoFill::Levels));
    f.extend([
        FieldDesc::new(
            LEVELS,
            tr_noop!("Max Level"),
            FieldKind::Int { min: 0, max: 100 },
            |r: &Class, off| value(r, off),
            |r, off, v, _| {
                store(r, off, v);
                if let Some(d) = data_mut(r) {
                    fix_experience(d);
                }
            },
        )
        .arg(0x58),
        table!(
            tr_noop!("Levels"),
            LEVEL_COLUMNS,
            -1_000_000,
            i64::from(i32::MAX),
            level_row,
            set_level_row
        ),
        table!(
            tr_noop!("Spells progression by level"),
            SPELL_COLUMNS,
            -1000,
            1000,
            |d: &ClassData, l: usize| {
                (1..=10)
                    .map(|s| i64::from(d.unknown_0x414[l][s]))
                    .chain((1..=10).map(|s| i64::from(d.unknown_0x159c[l][s])))
                    .collect::<Vec<i64>>()
            },
            |d: &mut ClassData, l: usize, row: &[i64]| {
                for s in 0..10 {
                    d.unknown_0x414[l][s + 1] = row.get(s).copied().unwrap_or(0) as i32;
                    d.unknown_0x159c[l][s + 1] = row.get(10 + s).copied().unwrap_or(0) as i32;
                }
            }
        ),
        table!(
            tr_noop!("Ability score bonus"),
            BONUS_COLUMNS,
            0,
            25,
            |d: &ClassData, l: usize| {
                let row = d.unknown_0x356c.map_or([0; 12], |t| t[l]);
                (0..12)
                    .map(|c| i64::from(row[bonus_index(c)]))
                    .collect::<Vec<i64>>()
            },
            |d: &mut ClassData, l: usize, cells: &[i64]| {
                if let Some(t) = d.unknown_0x356c.as_mut() {
                    for c in 0..12 {
                        t[l][bonus_index(c)] = cells.get(c).copied().unwrap_or(0) as i32;
                    }
                }
            }
        ),
        FieldDesc::new(
            LEVELS,
            tr_noop!("Maximum attacks"),
            FieldKind::Int { min: 0, max: 10000 },
            |r: &Class, off| value(r, off),
            |r, off, v, _| {
                store(r, off, v);
                if let Some(d) = data_mut(r) {
                    fix_experience(d);
                }
            },
        )
        .arg(0x31e4)
        .group(tr_noop!("Attacks Per Round")),
        FieldDesc::new(
            LEVELS,
            tr_noop!("per rounds"),
            FieldKind::Int { min: 1, max: 1000 },
            |r: &Class, off| value(r, off),
            |r, off, v, _| {
                store(r, off, v);
                if let Some(d) = data_mut(r) {
                    fix_experience(d);
                }
            },
        )
        .arg(0x337c)
        .group(tr_noop!("Attacks Per Round")),
    ]);
    level_details(f);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bonus_columns_follow_the_dialog() {
        // Strength's adjustment is the fifth array (0x3bbc), Charisma's
        // maximum the seventh (0x3ee4).
        assert_eq!(bonus_index(0), 4);
        assert_eq!(bonus_index(11), 6);
        let mut seen: Vec<usize> = (0..12).map(bonus_index).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..12).collect::<Vec<_>>());
    }

    #[test]
    fn every_number_and_check_reaches_its_member() {
        let sources = crate::userdb::Sources::default();
        for field in fields() {
            let mut c = super::super::new_class();
            let probe = match field.kind {
                FieldKind::Int { min, max } => {
                    Value::Int(if min <= 2 && 2 <= max { 2 } else { min })
                }
                FieldKind::Check => Value::Int(1),
                FieldKind::Text { .. } | FieldKind::MultiLine => Value::Text("X".into()),
                _ => continue,
            };
            field.set(&mut c, &probe, &sources);
            let got = field.get(&c);
            match probe {
                Value::Text(_) => assert_eq!(got.text(), probe.text(), "{}", field.label),
                _ => assert_eq!(got.int(), probe.int(), "{}", field.label),
            }
        }
    }

    #[test]
    fn levels_ok_orders_experience() {
        let mut c = super::super::new_class();
        let d = data_mut(&mut c).unwrap();
        d.experience[1] = 5;
        d.experience[3] = 2000;
        fix_experience(d);
        assert_eq!(&d.experience[..5], &[0, 0, 1, 2000, 2001]);
    }

    #[test]
    fn level_details_fields_reach_the_class() {
        let mut sources = crate::userdb::Sources::default();
        let mut from = super::super::new_class();
        data_mut(&mut from).unwrap().experience[2] = 1234;
        sources.class_records.push(from);
        let fields = fields();
        let find = |kind: fn(&FieldKind) -> bool| {
            fields
                .iter()
                .find(|f| f.page == LEVELS && kind(&f.kind))
                .unwrap()
        };
        let mut c = super::super::new_class();

        let spells = find(|k| matches!(k, FieldKind::FreeSpells));
        let mut level = levels::FreeSpellLevel::new(3);
        level.spells.push(levels::FreeSpell {
            name: "Light".into(),
            uses: 2,
            wizard: false,
        });
        spells.set(
            &mut c,
            &Value::FreeSpells(String::new(), vec![level.clone()]),
            &sources,
        );
        assert_eq!(spells.get(&c).free_spells(), vec![level]);

        let nwps = find(|k| matches!(k, FieldKind::LevelNwps(_)));
        let entry = levels::LevelNwps {
            level: 2,
            granted: vec!["Swimming".into()],
            pick_one: true,
            modifications: vec![("Riding".into(), 5)],
        };
        nwps.set(&mut c, &Value::LevelNwps(vec![entry.clone()]), &sources);
        assert_eq!(nwps.get(&c).level_nwps(), vec![entry]);

        let abilities = find(|k| matches!(k, FieldKind::LevelAbilities(_)));
        let pairs = vec![("Turn Undead".to_owned(), 4)];
        abilities.set(&mut c, &Value::Pairs(pairs.clone()), &sources);
        assert_eq!(abilities.get(&c).pairs(), pairs);

        let fill = find(|k| matches!(k, FieldKind::AutoFill));
        fill.set(&mut c, &Value::Text(String::new()), &sources);
        assert_eq!(data(&c).unwrap().experience[2], 1234);
        // Level Details' Auto Fill brings the other class's free spells too.
        assert!(spells.get(&c).free_spells().is_empty());
    }
}
