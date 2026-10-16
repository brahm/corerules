//! The Expansion's kit editor: IDD 194 "Character Kits" and the dialogs
//! its buttons open (195, 181, 198, 201, 212, 218, 219, 230 to 234, 238
//! to 243), described by the offsets of `CPartKitOb` they read and write.
//!
//! The originals' "check plus list" controls (Allow only these classes,
//! Free Equipment, Forever barred from list, ...) are [`FieldKind::Checked`]:
//! the check is on while the list has names and unchecking empties it, as
//! the click handlers do. Lists of `CPartIDOb`s carry a number per name
//! (the second member, which these dialogs use for a bonus or a level).

use cr_formats::Object;
use cr_formats::parts::{ListOfListsOfOptions, NumList, PartId, PartKitData, PartKitLevel};

use super::super::allowed::Allowed;
use super::super::slots::{self, Slot};
use super::{PartRec, Value, kit, kit_mut};
use crate::userdb::{FieldDesc, FieldKind, Options, Sources};

/// Kit members holding `CPartIDOb`s (name and number).
const ID_LISTS: [usize; 4] = [0x174, 0x244, 0x2f8, 0x470];

pub(crate) fn has_kit(r: &PartRec) -> bool {
    kit(r).is_some()
}

fn slot(k: &PartKitData, off: usize) -> Option<Slot<'_>> {
    // The program keeps the second copy of the recommended WPs on load.
    if off == 0x384 {
        return Some(Slot::List(
            k.unknown_0x384_again.as_ref().unwrap_or(&k.unknown_0x384),
        ));
    }
    slots::kit(k, off)
}

/// The value of kit member `off` as a control shows it.
pub(crate) fn value(r: &PartRec, off: usize) -> Value {
    let Some(k) = kit(r) else {
        return Value::Int(0);
    };
    match slot(k, off) {
        Some(Slot::Objs(objects)) if ID_LISTS.contains(&off) => Value::Pairs(
            objects
                .iter()
                .filter_map(|o| match o {
                    Object::PartId(p) => Some((p.name.clone(), i64::from(p.part_type))),
                    _ => None,
                })
                .collect(),
        ),
        Some(Slot::Objs(objects)) if off == 0x65c => Value::List(allowed_names(objects)),
        Some(Slot::Objs(objects)) if off == 0x634 => Value::List(
            objects
                .iter()
                .filter_map(|o| match o {
                    Object::Spell(s) => Some(s.name.clone()),
                    _ => None,
                })
                .collect(),
        ),
        Some(Slot::Objs(objects)) if off == 0x648 => Value::Grid(
            objects
                .iter()
                .filter_map(|o| match o {
                    Object::PartKitLevel(l) => {
                        Some(l.unknown_0x4.iter().map(|n| (*n).into()).collect())
                    }
                    _ => None,
                })
                .collect(),
        ),
        Some(Slot::Choose(c)) => Value::Groups(
            c.lists
                .iter()
                .filter_map(|o| match o {
                    Object::NumList(n) => Some((i64::from(n.count), n.options.clone())),
                    _ => None,
                })
                .collect(),
        ),
        Some(s) => s.value(),
        None => Value::Int(0),
    }
}

fn int(r: &PartRec, off: usize) -> i64 {
    value(r, off).int()
}

fn allowed_names(objects: &[Object]) -> Vec<String> {
    objects
        .iter()
        .filter_map(|o| match o {
            Object::ClassesAllowedHelper(h) => Some(Allowed::of(&h.data).name),
            _ => None,
        })
        .collect()
}

/// Stores `v` into kit member `off`.
pub(crate) fn store(r: &mut PartRec, off: usize, v: &Value, sources: &Sources) {
    let Some(k) = kit_mut(r) else {
        return;
    };
    if off == 0x384 {
        k.unknown_0x384 = v.list();
        if let Some(again) = k.unknown_0x384_again.as_mut() {
            *again = v.list();
        }
        return;
    }
    let Some(s) = slots::kit_mut(k, off) else {
        return;
    };
    match s {
        slots::SlotMut::Objs(objects) if ID_LISTS.contains(&off) => {
            *objects = v
                .pairs()
                .into_iter()
                .map(|(name, n)| {
                    Object::PartId(PartId {
                        name,
                        part_type: n as i32,
                    })
                })
                .collect();
        }
        slots::SlotMut::Objs(objects) if off == 0x65c => {
            let old = std::mem::take(objects);
            for name in v.list() {
                let kept = old
                    .iter()
                    .find(|o| allowed_names(std::slice::from_ref(o)) == [name.clone()]);
                if let Some(o) = kept {
                    objects.push(o.clone());
                    continue;
                }
                let parts: Vec<&str> = name.split('/').collect();
                if let Some(a) = Allowed::from_names(&parts, &sources.classes, 0) {
                    objects.push(a.helper());
                }
            }
        }
        slots::SlotMut::Objs(objects) if off == 0x634 => {
            let old = std::mem::take(objects);
            for name in v.list() {
                let kept = old
                    .iter()
                    .find(|o| matches!(o, Object::Spell(s) if s.name == name))
                    .cloned()
                    .or_else(|| {
                        sources
                            .spells
                            .iter()
                            .find(|s| s.name == name)
                            .map(|s| Object::Spell(Box::new(s.clone())))
                    });
                objects.extend(kept);
            }
        }
        slots::SlotMut::Objs(objects) if off == 0x648 => {
            *objects = v
                .grid()
                .into_iter()
                .map(|row| {
                    let mut cells = [0i32; 10];
                    for (c, n) in cells.iter_mut().zip(row) {
                        *c = n as i32;
                    }
                    Object::PartKitLevel(PartKitLevel {
                        version: 1,
                        unknown_0x4: cells,
                    })
                })
                .collect();
        }
        slots::SlotMut::Choose(c) => {
            *c = ListOfListsOfOptions {
                lists: v
                    .groups()
                    .into_iter()
                    .map(|(count, options)| {
                        Object::NumList(NumList {
                            count: count as i32,
                            options,
                        })
                    })
                    .collect(),
            };
        }
        s => s.set(v),
    }
}

/// A control on kit member `off`.
macro_rules! kat {
    ($page:expr, $label:expr, $kind:expr, $off:expr) => {
        FieldDesc::new(
            $page,
            $label,
            $kind,
            |r: &PartRec, off| value(r, off),
            |r, off, v, s| store(r, off, v, s),
        )
        .arg($off)
        .visible(has_kit)
    };
    ($page:expr, $label:expr, $kind:expr, $off:expr, $opened:path) => {
        FieldDesc::new(
            $page,
            $label,
            $kind,
            |r: &PartRec, off| value(r, off),
            |r, off, v, s| {
                $opened(r);
                store(r, off, v, s)
            },
        )
        .arg($off)
        .visible(has_kit)
    };
}

macro_rules! check {
    ($page:expr, $label:expr, $off:expr $(, $opened:path)?) => {
        kat!($page, $label, FieldKind::Check, $off $(, $opened)?)
    };
}

macro_rules! int {
    ($page:expr, $label:expr, $min:expr, $max:expr, $off:expr $(, $opened:path)?) => {
        kat!($page, $label, FieldKind::Int { min: $min, max: $max }, $off $(, $opened)?)
    };
}

macro_rules! list {
    ($page:expr, $label:expr, $source:expr, $off:expr) => {
        kat!(
            $page,
            $label,
            FieldKind::Dual(Options::Source($source)),
            $off
        )
    };
}

/// A check over a list: on while the list has names.
macro_rules! checked {
    ($page:expr, $label:expr, $source:expr, $off:expr) => {
        kat!(
            $page,
            $label,
            FieldKind::Checked(Options::Source($source)),
            $off
        )
    };
}

macro_rules! sentinel {
    ($page:expr, $label:expr, $min:expr, $max:expr, $none:expr, $start:expr, $off:expr) => {
        kat!(
            $page,
            $label,
            FieldKind::OptInt {
                min: $min,
                max: $max,
                none: $none,
                start: $start,
                inverted: false
            },
            $off
        )
    };
}

/// Radios stored as one flag each (index in `arg`): the first set flag
/// picks the radio after the first one.
const RADIOS: [&[usize]; 2] = [
    // IDD 232: in a melee weapon, in a missile weapon.
    &[0x520, 0x521],
    // IDD 233: melee, missile, hurled.
    &[0x4ad, 0x4ae, 0x4af],
];

fn radio(
    page: &'static str,
    label: &'static str,
    options: &'static [&'static str],
    which: usize,
) -> FieldDesc<PartRec> {
    FieldDesc::new(
        page,
        label,
        FieldKind::Choice(Options::Fixed(options)),
        |r: &PartRec, w| {
            Value::Int(
                RADIOS[w]
                    .iter()
                    .position(|o| int(r, *o) != 0)
                    .map_or(0, |i| i as i64 + 1),
            )
        },
        |r, w, v, s| {
            for (i, off) in RADIOS[w].iter().enumerate() {
                store(r, *off, &Value::Int(i64::from(v.int() == i as i64 + 1)), s);
            }
        },
    )
    .arg(which)
    .visible(has_kit)
}

/// What IDD 242's OnInitDialog does to the social rank table: a table
/// that does not have 13 entries gets 13, rolls 2 to 12 empty.
pub(crate) fn opened_other(r: &mut PartRec) {
    if let Some(k) = kit_mut(r)
        && k.unknown_0x10c.len() != 13
    {
        k.unknown_0x10c.resize(13, String::new());
        for roll in &mut k.unknown_0x10c[2..] {
            roll.clear();
        }
    }
}

/// "Heaviest starting armor": Any (i32::MIN), None (10), then 9 to -9.
fn armor_index(n: i64) -> i64 {
    match n {
        n if n < -9 => 0,
        n if n >= 10 => 1,
        n => 11 - n,
    }
}

fn armor_value(index: i64) -> i64 {
    match index {
        0 => i32::MIN.into(),
        1 => 10,
        i => 11 - i,
    }
}

const ARMOR_LIMITS: &[&str] = &[
    "Any", "None", "9", "8", "7", "6", "5", "4", "3", "2", "1", "0", "-1", "-2", "-3", "-4", "-5",
    "-6", "-7", "-8", "-9",
];

// Pages.
const KIT: &str = "Kit";
const REQ: &str = "Kit Requirements";
const ALIGN: &str = "Kit Alignment/Gender";
const COMBAT: &str = "Kit Combat";
const ROGUE: &str = "Kit Rogue Skills";
const MONEY: &str = "Kit Money";
const LANG: &str = "Kit Languages";
const ARMOR: &str = "Kit Armor";
const NWPS: &str = "Kit NWPs";
const WPS: &str = "Kit WPs";
const SPEC: &str = "Kit Specialization";
const FOCUS: &str = "Kit Weapon Focus";
const RESTR: &str = "Kit WP Restrictions";
const CLASS: &str = "Kit Class Abilities";
const SPELLS: &str = "Kit Spells";
const EQUIP: &str = "Kit Equipment";
const OTHER: &str = "Kit Other";

const NWP: &str = "Non-Weapon Proficiencies";
const WP: &str = "weapon-proficiencies";

/// Every page of the Expansion's kit editor.
pub(crate) fn fields() -> Vec<FieldDesc<PartRec>> {
    let mut f = vec![
        // IDD 194.
        check!(KIT, "Available to Core Rules Characters", 0x73).group("Availability"),
        check!(KIT, "Available to Skills & Powers Characters", 0x74).group("Availability"),
        list!(KIT, "No races except these", "races", 0x10)
            .enabled(|r| value(r, 0xa8).list().is_empty()),
        list!(KIT, "All races except these", "races", 0xa8)
            .enabled(|r| value(r, 0x10).list().is_empty()),
        checked!(KIT, "Allow only these classes", "class-combinations", 0x65c),
        list!(KIT, "Recommended Traits", "Traits", 0xe4),
        list!(KIT, "Weapon (THAC0 Bonus)", WP, 0xf8),
        kat!(KIT, "Display on Selection", FieldKind::MultiLine, 0x9c),
        kat!(KIT, "Display on Equip", FieldKind::MultiLine, 0xa0),
    ];

    // IDD 195 Kit Requirements: min and max of each score.
    for (label, off) in [
        ("Strength", 0x4c),
        ("Stamina", 0x4e),
        ("Muscle", 0x50),
        ("Dexterity", 0x52),
        ("Aim", 0x54),
        ("Balance", 0x56),
        ("Constitution", 0x58),
        ("Health", 0x5a),
        ("Fitness", 0x5c),
        ("Intelligence", 0x5e),
        ("Reason", 0x60),
        ("Knowledge", 0x62),
        ("Wisdom", 0x64),
        ("Intuition", 0x66),
        ("Willpower", 0x68),
        ("Charisma", 0x6a),
        ("Leadership", 0x6c),
        ("Appearance", 0x6e),
    ] {
        f.push(int!(REQ, "Min", 0, 25, off).group(label));
        f.push(int!(REQ, "Max", 0, 25, off + 1).group(label));
    }
    f.extend([
        int!(
            REQ,
            "Must have a minimum score in their class' prime requisite",
            0,
            25,
            0x70
        ),
        check!(REQ, "Add % to XP requirements", 0x5de),
        int!(REQ, "% to XP requirements", -1000, 1000, 0x5ec).enabled(|r| int(r, 0x5de) != 0),
        check!(REQ, "Maximum % XP bonus for high stats", 0x5dd),
        int!(REQ, "% XP bonus for high stats", -1000, 1000, 0x5e4).enabled(|r| int(r, 0x5dd) != 0),
    ]);

    // IDD 181 Alignment and Gender.
    for (label, off) in [
        ("Lawful Good", 0x6),
        ("Lawful Neutral", 0x7),
        ("Lawful Evil", 0x8),
        ("Neutral Good", 0x9),
        ("True Neutral", 0xa),
        ("Neutral Evil", 0xb),
        ("Chaotic Good", 0xc),
        ("Chaotic Neutral", 0xd),
        ("Chaotic Evil", 0xe),
    ] {
        f.push(check!(ALIGN, label, off).group("Alignments available to this kit"));
    }
    f.push(check!(ALIGN, "Male", 0x4).group("Genders available to this kit"));
    f.push(check!(ALIGN, "Female", 0x5).group("Genders available to this kit"));

    // IDD 231 Combat.
    f.extend([
        int!(
            COMBAT,
            "THAC0 Mod (negative values better your THAC0)",
            -100,
            100,
            0x34c
        ),
        int!(COMBAT, "THAC0 Mod to this list", -100, 100, 0x350).group("THAC0 List"),
        list!(COMBAT, "Weapons with the THAC0 modifier", WP, 0x354).group("THAC0 List"),
        int!(COMBAT, "Damage mod to this list", -100, 100, 0x368).group("Damage List"),
        list!(COMBAT, "Weapons with the damage modifier", WP, 0x36c).group("Damage List"),
        check!(COMBAT, "Gains two weapon style specialist bonus", 0x348),
        check!(COMBAT, "Cavalier to hit bonus", 0x380),
        check!(
            COMBAT,
            "Negate all penalties for fighting with 2 weapons",
            0x5d8
        ),
        check!(
            COMBAT,
            "Use Sharpshooter Attacks Table (Applies only to crossbows)",
            0x5df
        ),
    ]);

    // IDD 201 Rogue Skills.
    f.extend([
        check!(ROGUE, "Move Silently as Ranger", 0x5d5),
        sentinel!(
            ROGUE,
            "Starts with non-standard number of skill points",
            0,
            1000,
            -1,
            60,
            0x120
        ),
        sentinel!(
            ROGUE,
            "Gains non-standard number of skill points per level",
            0,
            1000,
            -1,
            30,
            0x124
        ),
        checked!(
            ROGUE,
            "Starting skill points may only be spent on these abilities",
            "rogue-skills",
            0x128
        ),
        int!(ROGUE, "Add to one skill at level 1", 0, 500, 0x13c).group("At level 1"),
        list!(ROGUE, "from these abilities", "rogue-skills", 0x140).group("At level 1"),
        int!(
            ROGUE,
            "Add to one skill at levels beyond 1st",
            0,
            500,
            0x154
        )
        .group("At levels beyond 1st"),
        list!(ROGUE, "from these abilities", "rogue-skills", 0x158).group("At levels beyond 1st"),
        int!(ROGUE, "Choose abilities", 0, 50, 0x16c).group("Bonus"),
        int!(ROGUE, "for a bonus of", 0, 500, 0x170).group("Bonus"),
        kat!(
            ROGUE,
            "Modifiers to Rogue Skill Scores",
            FieldKind::Pairs {
                options: Options::Source("rogue-skills"),
                number: "Modifier",
                min: -100,
                max: 100
            },
            0x174
        ),
    ]);

    // IDD 238 Money.
    let dice = |r: &PartRec| int(r, 0x618) != 0;
    f.extend([
        check!(
            MONEY,
            "Maximum retained funds during character generation",
            0x5dc
        ),
        int!(MONEY, "Maximum retained funds", 0, 1_000_000, 0x5e0).enabled(|r| int(r, 0x5dc) != 0),
        check!(MONEY, "Starting money = dice * multiplier + add", 0x618).group("Starting Money"),
        int!(MONEY, "# of dice", 0, 1000, 0x608)
            .group("Starting Money")
            .enabled(dice),
        int!(MONEY, "# of sides", 0, 1000, 0x60c)
            .group("Starting Money")
            .enabled(dice),
        int!(MONEY, "Multiplier", 0, 1_000_000, 0x610)
            .group("Starting Money")
            .enabled(dice),
        int!(MONEY, "Add", 0, 1_000_000, 0x614)
            .group("Starting Money")
            .enabled(dice),
        check!(
            MONEY,
            "Assign maximum starting funds allowed for class",
            0x84
        )
        .group("Wealth"),
        int!(
            MONEY,
            "Percent of class starting funds assigned",
            0,
            1000,
            0x8c
        )
        .group("Wealth"),
        int!(
            MONEY,
            "Percent added to normal starting funds",
            0,
            1000,
            0x88
        )
        .group("Wealth"),
        int!(
            MONEY,
            "Percent of actual price paid for equipment",
            0,
            1000,
            0x90
        )
        .group("Wealth"),
    ]);

    // IDD 240 Languages.
    f.extend([
        check!(LANG, "Languages cost half", 0x5d6),
        int!(LANG, "Gain 1 language for each (levels)", 0, 100, 0x5fc),
        int!(LANG, "starting at level", 0, 100, 0x600),
    ]);

    // IDD 219 Armor.
    f.extend([
        check!(
            ARMOR,
            "No armor restrictions (overriding class restrictions)",
            0x188
        ),
        check!(
            ARMOR,
            "Can wear out of class armor with +1 AC penalty",
            0x189
        ),
        check!(ARMOR, "Light armor -2 AC Bonus", 0x7c),
        checked!(ARMOR, "Limited to armor from", "Armor", 0x1a0),
        checked!(
            ARMOR,
            "Limited to armor from (during generation)",
            "Armor",
            0x1b4
        ),
        int!(ARMOR, "Add to AC", -20, 20, 0x19c),
        int!(ARMOR, "At level", 1, 101, 0x18c).group("Base armor class"),
        int!(ARMOR, "base armor class becomes", -20, 20, 0x190).group("Base armor class"),
        int!(ARMOR, "improving by", -20, 20, 0x194).group("Base armor class"),
        int!(
            ARMOR,
            "each level until reaching a minimum of",
            -20,
            20,
            0x198
        )
        .group("Base armor class"),
    ]);

    // IDD 212 Non-Weapon Proficiencies.
    f.extend([
        list!(NWPS, "Required NWPs", NWP, 0x280),
        list!(NWPS, "Recommended NWPs", NWP, 0x258),
        kat!(
            NWPS,
            "Bonus NWPs",
            FieldKind::Groups(Options::Source(NWP)),
            0x330
        ),
        kat!(
            NWPS,
            "Gain bonus NWPs at specific levels",
            FieldKind::Pairs {
                options: Options::Source(NWP),
                number: "Level",
                min: 0,
                max: 100
            },
            0x2f8
        ),
        kat!(
            NWPS,
            "NWP Modifiers",
            FieldKind::Pairs {
                options: Options::Source(NWP),
                number: "Modifier",
                min: -100,
                max: 100
            },
            0x244
        ),
        list!(NWPS, "Pick NWP from list for +2 bonus", NWP, 0x2a8),
        list!(NWPS, "Barred during creation", NWP, 0x26c),
        checked!(
            NWPS,
            "Can pick secondary skill from",
            "secondary-skills",
            0x2d0
        ),
        check!(NWPS, "No secondary skills", 0x314).group("Secondary Skills"),
        list!(NWPS, "Required secondary skills", "secondary-skills", 0x2e4),
    ]);
    for (label, off) in [
        ("Warrior", 0x30f),
        ("Priest", 0x310),
        ("Rogue", 0x312),
        ("Wizard", 0x311),
        ("Psionicist", 0x313),
    ] {
        f.push(check!(NWPS, label, off).group("No out of group penalty"));
    }
    f.extend([
        list!(NWPS, "No out of class penalty", NWP, 0x2bc),
        check!(
            NWPS,
            "Replace NWP advancement with 1 NWP at levels 1, 3, and 6",
            0x30c
        ),
        kat!(NWPS, "May use WPs as NWPs", FieldKind::Check, 0x30e).enabled(|r| int(r, 0x44d) == 0),
        check!(NWPS, "Might receive bonus NWP each level", 0x30d),
        sentinel!(
            NWPS,
            "Start with non-standard number of NWP slots in Core Rules",
            0,
            100,
            -1,
            0,
            0x318
        ),
        kat!(
            NWPS,
            "Must learn NWP",
            FieldKind::Combo {
                options: Options::Source(NWP),
                free: true
            },
            0x31c
        )
        .group("Must learn"),
        int!(NWPS, "by level", 0, 100, 0x320).group("Must learn"),
        kat!(
            NWPS,
            "If has NWP",
            FieldKind::Combo {
                options: Options::Source(NWP),
                free: true
            },
            0x324
        )
        .group("Rating bonus"),
        kat!(
            NWPS,
            "then NWP",
            FieldKind::Combo {
                options: Options::Source(NWP),
                free: true
            },
            0x328
        )
        .group("Rating bonus"),
        int!(NWPS, "receives bonus of", -100, 100, 0x32c).group("Rating bonus"),
    ]);

    // IDD 230 Weapon Proficiencies.
    f.extend([
        int!(
            WPS,
            "Bonus WP Slots (CPs for Skills & Powers)",
            -100,
            100,
            0x448
        ),
        check!(
            WPS,
            "Replace WP advancement with 1 WP at levels 1, 3, and 6",
            0x44c
        ),
        check!(
            WPS,
            "Consider all class allowed weapons to be recommended",
            0x444
        ),
        FieldDesc::new(
            WPS,
            "May spend NWP slots as WPs",
            FieldKind::Check,
            |r: &PartRec, _| Value::Bool(int(r, 0x44d) != 0),
            |r, _, v, s| {
                store(r, 0x44d, v, s);
                if v.bool() {
                    // "Character Kits do not support both NWP slots as WP
                    // slots and WP slots as NWP slots": the other goes.
                    store(r, 0x30e, &Value::Int(0), s);
                } else {
                    store(r, 0x44e, &Value::Int(0), s);
                }
            },
        )
        .visible(has_kit),
        check!(
            WPS,
            "Must spend at least half of NWP slots in this way",
            0x44e
        )
        .enabled(|r| int(r, 0x44d) != 0),
        check!(
            WPS,
            "Limited to recommended and required WPs at start",
            0x76
        ),
        check!(WPS, "Must take one wooden WP", 0x465),
        sentinel!(
            WPS,
            "Different penalty for non-proficiency",
            -100,
            100,
            99,
            0,
            0x450
        ),
        sentinel!(WPS, "Start with nonstandard WP slots", 0, 100, -1, 0, 0x454),
        sentinel!(WPS, "Gain WP slots every X levels", 0, 100, -1, 0, 0x458),
        sentinel!(
            WPS,
            "Future WPs gained starting at level",
            0,
            100,
            -1,
            0,
            0x45c
        ),
        sentinel!(
            WPS,
            "Can learn out of class WPs by paying this many more slots",
            0,
            100,
            -1,
            0,
            0x460
        ),
        int!(WPS, "Bonus WP slots", 0, 100, 0x468).group("Bonus WP slots at level"),
        int!(WPS, "at level", 0, 100, 0x46c).group("Bonus WP slots at level"),
        list!(WPS, "Recommended WPs", WP, 0x384),
        kat!(
            WPS,
            "Required WPs",
            FieldKind::Groups(Options::Source(WP)),
            0x568
        ),
        kat!(
            WPS,
            "Bonus WPs at given levels",
            FieldKind::Pairs {
                options: Options::Source(WP),
                number: "Level",
                min: 0,
                max: 100
            },
            0x470
        ),
        list!(WPS, "These WPs are never out of class", WP, 0x484),
    ]);

    // IDD 232 Specialization.
    let by_level = |r: &PartRec| int(r, 0x51c) != 101;
    f.extend([
        check!(
            SPEC,
            "May specialize (even if normally not allowed by class)",
            0x518
        ),
        check!(
            SPEC,
            "May select Unarmed Combat (even if normally not allowed by class)",
            0x4b4
        ),
        sentinel!(SPEC, "Must specialize by level", 1, 100, 101, 1, 0x51c),
        radio(
            SPEC,
            "Specialize",
            &[
                "in any weapon proficiency",
                "in a melee weapon",
                "in a missile weapon",
            ],
            0,
        )
        .enabled(by_level),
        checked!(SPEC, "May only specialize in WPs from", WP, 0x524),
        checked!(SPEC, "Must specialize in 1 WP from", WP, 0x3e8),
        check!(
            SPEC,
            "May not specialize in a melee weapon at first level",
            0x538
        ),
        check!(SPEC, "Bow specialization costs -1 slots", 0x5d4),
        list!(SPEC, "Free to specialize in all of these", WP, 0x53c).group("Free specialization"),
        list!(
            SPEC,
            "Allowed to pick as many as indicated below",
            WP,
            0x550
        )
        .group("Free specialization"),
        int!(SPEC, "Number to pick", 0, 1000, 0x564).group("Free specialization"),
    ]);

    // IDD 233 Weapon Focus.
    let focus = |r: &PartRec| int(r, 0x4ac) != 0;
    f.push(check!(
        FOCUS,
        "This kit chooses a weapon to focus on",
        0x4ac
    ));
    f.push(radio(FOCUS, "Weapon", &["Any", "Melee", "Missile", "Hurled"], 1).enabled(focus));
    for (label, off) in [
        ("Use fighter THAC0 with this weapon", 0x4b0),
        ("Additional -1 THAC0 penalty for other weapons", 0x4b1),
        ("May only use magical weapons of this type", 0x4b2),
        (
            "May specialize in this weapon (even if class normally does not allow)",
            0x4b3,
        ),
    ] {
        f.push(check!(FOCUS, label, off).enabled(focus));
    }
    f.extend([
        check!(FOCUS, "+1 THAC0 to chosen melee weapon", 0x7a),
        check!(FOCUS, "+1 THAC0 and damage to chosen missile weapon", 0x7b),
    ]);

    // IDD 234 WP Restrictions.
    f.extend([
        check!(
            RESTR,
            "Odd WPs (1, 3, 5 ...) must be missile or thrown",
            0x438
        ),
        check!(
            RESTR,
            "No ranged weapons during character generation",
            0x508
        ),
        list!(RESTR, "Barred WPs during creation", WP, 0xbc),
        list!(RESTR, "Not recommended WPs", WP, 0xd0),
        checked!(RESTR, "Must always take WPs from list", WP, 0x4e0),
        checked!(
            RESTR,
            "During generation, limited to WPs from list",
            WP,
            0x4b8
        ),
        checked!(RESTR, "Half of all WPs must come from list", WP, 0x4f4),
        checked!(
            RESTR,
            "At least every second WP must come from list",
            WP,
            0x410
        ),
        checked!(RESTR, "Forever barred from list", WP, 0x3ac),
        checked!(
            RESTR,
            "Always restricted to WP from list and all thrown/hurled weapons",
            WP,
            0x3fc
        ),
        checked!(RESTR, "First sword must be from list", "Weapons", 0x398),
        checked!(
            RESTR,
            "Forever barred from broad groups in list",
            "broad-weapon-groups",
            0x3c0
        ),
        checked!(
            RESTR,
            "Forever barred from tight groups in list",
            "tight-weapon-groups",
            0x3d4
        ),
        int!(RESTR, "Must fill", 0, 100, 0x43c).group("Must fill X of first Y WPs from list"),
        int!(RESTR, "of first", 0, 100, 0x440).group("Must fill X of first Y WPs from list"),
        list!(RESTR, "WPs from", WP, 0x424).group("Must fill X of first Y WPs from list"),
        int!(
            RESTR,
            "Required slots of missile WPs at start",
            0,
            100,
            0x50c
        ),
        int!(
            RESTR,
            "Required different melee WPs at start",
            0,
            100,
            0x514
        ),
        int!(
            RESTR,
            "Required different missile WPs at start",
            0,
            100,
            0x510
        ),
    ]);

    // IDD 218 Class Abilities.
    f.extend([
        check!(CLASS, "Never uses spells", 0x5d7),
        int!(
            CLASS,
            "Add to my level when calculating turning undead",
            -100,
            100,
            0x5e8
        ),
        list!(CLASS, "Barred class abilities", "Class Abilities", 0x1c8),
        int!(CLASS, "Maximum level of spell", 0, 10, 0x1f4).group("Wizards"),
        list!(CLASS, "Barred schools of magic", "Schools Of Magic", 0x1dc),
        check!(CLASS, "Is limited to 5 schools", 0x1fc).group("Militant Wizard"),
        check!(CLASS, "Different oppositional schools", 0x1f0).group("Militant Wizard"),
        int!(
            CLASS,
            "Intelligence is modified by X in regards to wizard spell casting",
            -25,
            25,
            0x1f8
        )
        .group("Militant Wizard"),
        int!(CLASS, "Maximum number of Minor Spheres", 0, 50, 0x200).group("Priests"),
        int!(CLASS, "Maximum number of Major Spheres", 0, 50, 0x204).group("Priests"),
        list!(CLASS, "Limited to these spheres", "spheres", 0x21c),
        list!(CLASS, "Choose 1 free major sphere", "spheres", 0x208),
        list!(CLASS, "Receive free minor spheres", "spheres", 0x230),
    ]);

    // IDD 239 Spells.
    f.extend([
        list!(
            SPELLS,
            "Automatically knows spells from this list",
            "spells",
            0x634
        ),
        int!(
            SPELLS,
            "Add % to learn spells up to a max. of 95%",
            -100,
            95,
            0x5f0
        ),
        FieldDesc::new(
            SPELLS,
            "Use custom spell progression",
            FieldKind::Check,
            |r: &PartRec, _| Value::Bool(int(r, 0x5da) != 0),
            |r, _, v, s| {
                store(r, 0x5da, v, s);
                // Checking makes 100 empty levels; unchecking drops them.
                let rows = if v.bool() {
                    vec![vec![0; 10]; 100]
                } else {
                    Vec::new()
                };
                store(r, 0x648, &Value::Grid(rows), s);
            },
        )
        .visible(has_kit),
        kat!(
            SPELLS,
            "Spells per spell level",
            FieldKind::Grid {
                columns: &["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"],
                first: 1,
                min: 0,
                max: 100
            },
            0x648
        )
        .enabled(|r| int(r, 0x5da) != 0),
    ]);

    // IDD 241 Equipment.
    f.extend([
        checked!(
            EQUIP,
            "Must purchase equipment during character generation",
            "Equipment",
            0x584
        ),
        checked!(EQUIP, "Free Equipment", "Equipment", 0x598),
        checked!(EQUIP, "Free Weapons", "Weapons", 0x5ac),
        checked!(EQUIP, "Free Magic Items", "Magical Items", 0x5c0),
        checked!(EQUIP, "Forbidden Weapons", "Weapons", 0x620),
        check!(
            EQUIP,
            "Must purchase chosen weapon during character generation",
            0x580
        ),
    ]);

    // IDD 242 Other Kit Information.
    for roll in 2..=12usize {
        f.push(
            FieldDesc::new(
                OTHER,
                ROLLS[roll - 2],
                FieldKind::Combo {
                    options: Options::Source("social-classes"),
                    free: false,
                },
                |r: &PartRec, roll| {
                    Value::Text(
                        value(r, 0x10c)
                            .list()
                            .get(roll)
                            .cloned()
                            .unwrap_or_default(),
                    )
                },
                |r, roll, v, _| {
                    opened_other(r);
                    if let Some(k) = kit_mut(r) {
                        k.unknown_0x10c[roll] = v.text();
                    }
                },
            )
            .arg(roll)
            .group("Social Rank (2d6 roll)")
            .visible(has_kit),
        );
    }
    f.extend([
        kat!(
            OTHER,
            "Free NWP (selected from the recommended list)",
            FieldKind::Combo {
                options: Options::Source(NWP),
                free: true
            },
            0xa4,
            opened_other
        ),
        FieldDesc::new(
            OTHER,
            "Allow one free non-weapon proficiency",
            FieldKind::Check,
            |r: &PartRec, _| Value::Bool(int(r, 0x72) != 0),
            |r, _, v, s| {
                opened_other(r);
                store(r, 0x72, v, s);
                store(r, 0x78, v, s);
            },
        )
        .visible(has_kit),
        check!(OTHER, "Acquire one weapon at no cost", 0x71, opened_other),
        check!(
            OTHER,
            "No metal armor or weapons at start",
            0x75,
            opened_other
        ),
        check!(OTHER, "No fired WPs at start", 0x79, opened_other),
        check!(
            OTHER,
            "Apply Acrobatic skill penalties for heavy armor",
            0x77,
            opened_other
        ),
        int!(OTHER, "Bonus", 0, 100, 0x670, opened_other).group("Bonus to Level Limits"),
        kat!(
            OTHER,
            "to class",
            FieldKind::Combo {
                options: Options::Source("classes"),
                free: true
            },
            0x674,
            opened_other
        )
        .group("Bonus to Level Limits"),
        int!(OTHER, "Add to all damage", -100, 100, 0x80, opened_other),
        int!(
            OTHER,
            "Player picks this many NWPs for +1 score",
            0,
            100,
            0x94,
            opened_other
        ),
        int!(
            OTHER,
            "Additional HP per level",
            -100,
            100,
            0x604,
            opened_other
        ),
        FieldDesc::new(
            OTHER,
            "Heaviest starting armor",
            FieldKind::Choice(Options::Fixed(ARMOR_LIMITS)),
            |r: &PartRec, _| Value::Int(armor_index(int(r, 0x98))),
            |r, _, v, s| {
                opened_other(r);
                store(r, 0x98, &Value::Int(armor_value(v.int())), s);
            },
        )
        .visible(has_kit),
    ]);
    f
}

const ROLLS: [&str; 11] = ["2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_limits_round_trip() {
        for i in 0..ARMOR_LIMITS.len() as i64 {
            assert_eq!(armor_index(armor_value(i)), i);
        }
        assert_eq!(ARMOR_LIMITS[armor_index(-3) as usize], "-3");
        assert_eq!(armor_index(i32::MIN.into()), 0);
    }
}
