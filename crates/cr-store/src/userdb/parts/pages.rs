//! The part editor's pages (IDD 155 and its property pages), described by
//! the offsets the original dialogs read and write (`+0x27c`).
//!
//! Which pages a part gets follows IDD 155's OnInitDialog: a switch on the
//! part type enables the page buttons, then Items Worn, Container, Carry
//! Me and Skills follow their checks on the main page for every type.
//! Non-weapon proficiencies open their own dialog (IDD 176) instead of
//! IDD 155, and weapon proficiencies cannot be edited here at all.
//!
//! Several Player's Handbook values fall back to the Skills & Powers ones
//! while they are empty (damage, speed, size, type and rate of fire, the
//! four coins, the weight): the pages show the fallback and their OK
//! stores it, so changing any control of such a page here stores the
//! fallbacks too ([`opened_weapon`], [`opened_costs`]).

use super::super::slots::{self, Slot, SlotMut};
use super::{PartRec, Value};
use crate::userdb::{FieldDesc, FieldKind, Flavor, Options};

/// The member at `off` (IDD offsets of `CPartOb`: the head first, then
/// the data; +0x728 is the Strength adjustment stored before the data).
pub(crate) fn slot(r: &PartRec, off: usize) -> Option<Slot<'_>> {
    if off == 0x728 {
        return Some(Slot::I32(&r.part.unknown_0x728));
    }
    slots::head(&r.part.head, off).or_else(|| slots::data(&r.part.data, off))
}

pub(crate) fn slot_mut(r: &mut PartRec, off: usize) -> Option<SlotMut<'_>> {
    let p = &mut r.part;
    if off == 0x728 {
        return Some(SlotMut::I32(&mut p.unknown_0x728));
    }
    match slots::head_mut(&mut p.head, off) {
        Some(s) => Some(s),
        None => slots::data_mut(&mut p.data, off),
    }
}

fn raw(r: &PartRec, off: usize) -> Value {
    slot(r, off).map_or(Value::Int(0), Slot::value)
}

/// The value a page shows for `off`, with the PHB fallbacks.
pub(crate) fn value(r: &PartRec, off: usize) -> Value {
    raw(r, phb_source(r, off))
}

pub(crate) fn store(r: &mut PartRec, off: usize, v: &Value) {
    if let Some(s) = slot_mut(r, off) {
        s.set(v);
    }
}

fn int(r: &PartRec, off: usize) -> i64 {
    raw(r, off).int()
}

fn empty(v: &Value) -> bool {
    match v {
        Value::Text(s) => s.is_empty(),
        v => v.int() == 0,
    }
}

/// Weapon groups of PHB members and the S&P members they fall back to
/// while all of them are empty (getters 0x6611cd to 0x6614d0).
const WEAPON_FALLBACKS: &[(&[usize], &[usize])] = &[
    (&[0x3fc, 0x3f8, 0x400], &[0x3d8, 0x3d4, 0x3dc]),
    (&[0x3e8, 0x3ec, 0x3f0], &[0x3c4, 0x3c8, 0x3cc]),
    (&[0x404], &[0x3e0]),
    (&[0x3f4], &[0x3d0]),
    (&[0x3e4], &[0x3b4]),
    (&[0x408], &[0x41c]),
];

/// PHB coins and the S&P ones (getters 0x6610a6 to 0x661151).
const COST_FALLBACKS: &[(&[usize], &[usize])] = &[
    (&[0x208], &[0x1f8]),
    (&[0x20c], &[0x1fc]),
    (&[0x210], &[0x200]),
    (&[0x214], &[0x204]),
];

fn phb_source(r: &PartRec, off: usize) -> usize {
    for (phb, base) in WEAPON_FALLBACKS.iter().chain(COST_FALLBACKS) {
        if let Some(i) = phb.iter().position(|o| *o == off) {
            return if phb.iter().all(|o| empty(&raw(r, *o))) {
                base[i]
            } else {
                off
            };
        }
    }
    off
}

fn materialize(r: &mut PartRec, groups: &[(&[usize], &[usize])]) {
    for (phb, _) in groups {
        for off in *phb {
            let v = value(r, *off);
            store(r, *off, &v);
        }
    }
}

/// What the weapon page's OK stores besides the changed control.
pub(crate) fn opened_weapon(r: &mut PartRec) {
    materialize(r, WEAPON_FALLBACKS);
}

pub(crate) fn opened_costs(r: &mut PartRec) {
    materialize(r, COST_FALLBACKS);
}

/// The PHB weight (+0x1f0), shown as the S&P one (+0x1ec) while it is not
/// above zero (0x66118a).
fn phb_weight(r: &PartRec) -> f32 {
    let phb = f32::from_bits(int(r, 0x1f0) as u32);
    if phb > 0.0 {
        phb
    } else {
        f32::from_bits(int(r, 0x1ec) as u32)
    }
}

/// The bulk (+0x1f4) as the General page shows it: 0 outside 0..1e7.
fn bulk(r: &PartRec) -> f32 {
    let b = f32::from_bits(int(r, 0x1f4) as u32);
    if (0.0..=10_000_000.0).contains(&b) {
        b
    } else {
        0.0
    }
}

pub(crate) fn opened_general(r: &mut PartRec) {
    let (w, b) = (phb_weight(r), bulk(r));
    store(r, 0x1f0, &Value::Int(i64::from(w.to_bits() as i32)));
    store(r, 0x1f4, &Value::Int(i64::from(b.to_bits() as i32)));
}

fn real_bits(r: &PartRec, off: usize) -> Value {
    Value::Real(f64::from(f32::from_bits(int(r, off) as u32)))
}

fn store_real(r: &mut PartRec, off: usize, v: &Value) {
    let bits = (v.real() as f32).to_bits() as i32;
    store(r, off, &Value::Int(bits.into()));
}

/// A control on member `off`.
macro_rules! at {
    ($page:expr, $label:expr, $kind:expr, $off:expr) => {
        FieldDesc::new(
            $page,
            $label,
            $kind,
            |r: &PartRec, off| value(r, off),
            |r, off, v, _| store(r, off, v),
        )
        .arg($off)
    };
    ($page:expr, $label:expr, $kind:expr, $off:expr, $opened:path) => {
        FieldDesc::new(
            $page,
            $label,
            $kind,
            |r: &PartRec, off| value(r, off),
            |r, off, v, _| {
                $opened(r);
                store(r, off, v)
            },
        )
        .arg($off)
    };
}

macro_rules! int {
    ($page:expr, $label:expr, $min:expr, $max:expr, $off:expr $(, $opened:path)?) => {
        at!($page, $label, FieldKind::Int { min: $min, max: $max }, $off $(, $opened)?)
    };
}

macro_rules! check {
    ($page:expr, $label:expr, $off:expr $(, $opened:path)?) => {
        at!($page, $label, FieldKind::Check, $off $(, $opened)?)
    };
}

/// A "not limited" check with its number: on stores -1.
macro_rules! unlimited {
    ($page:expr, $label:expr, $max:expr, $off:expr) => {
        at!(
            $page,
            $label,
            FieldKind::OptInt {
                min: 0,
                max: $max,
                none: -1,
                start: 0,
                inverted: true
            },
            $off
        )
    };
}

/// Radios stored as one flag each: the first set flag picks the radio
/// after the "none" one ([`RADIOS`] index in `arg`).
const RADIOS: [&[usize]; 2] = [
    // Items Worn, Shield Types: Buckler, Small, Medium, Body.
    &[0x7d4, 0x7d8, 0x7dc, 0x7e0],
    // Adjustments, conditions: selected weapons, any weapon, any melee.
    &[0x510, 0x514, 0x518],
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
            let flags = RADIOS[w];
            Value::Int(
                flags
                    .iter()
                    .position(|o| int(r, *o) != 0)
                    .map_or(0, |i| i as i64 + 1),
            )
        },
        |r, w, v, _| {
            for (i, off) in RADIOS[w].iter().enumerate() {
                store(r, *off, &Value::Int(i64::from(v.int() == i as i64 + 1)));
            }
        },
    )
    .arg(which)
}

// Pages, one per original dialog.
pub(crate) const MAIN: &str = "Part";
const NWP: &str = "Nonweapon Proficiency";
const ABILITIES: &str = "Abilities";
const WORN: &str = "Items Worn";
const CONTAINER: &str = "Container";
const CARRY: &str = "Carry Me";
const COSTS: &str = "Costs";
const WEAPON: &str = "Weapon";
const PROFS: &str = "Weapon Proficiencies";
const ADJUST: &str = "Adjustments";
const SAVES: &str = "Saving Throws";
const SCHOOLS: &str = "Schools";
const SETTING: &str = "Setting";
const RESTRICT: &str = "Restrict Class";
const RACIAL: &str = "Racial";
const MAGIC_ITEMS: &str = "Magic Items";
const SKILLS: &str = "Skills";
const CLASS_ABILITIES: &str = "Class Abilities";
const GENERAL: &str = "General Info";
const TRAITS: &str = "Traits";
pub(crate) const DESCRIPTION: &str = "Description";

fn of(r: &PartRec, types: &[usize]) -> bool {
    types.contains(&r.part_type)
}

pub(crate) fn magic(r: &PartRec) -> bool {
    int(r, 0x224) != 0
}

// Page rules (IDD 155's OnInitDialog).
fn on_main(r: &PartRec) -> bool {
    r.part_type != 4
}
fn on_nwp(r: &PartRec) -> bool {
    r.part_type == 4
}
fn on_abilities(r: &PartRec) -> bool {
    of(r, &[5, 8]) || (r.part_type == 0 && magic(r))
}
fn on_worn(r: &PartRec) -> bool {
    on_main(r) && int(r, 0x250) != 0
}
fn on_container(r: &PartRec) -> bool {
    on_main(r) && int(r, 0x274) != 0
}
fn on_carry(r: &PartRec) -> bool {
    on_main(r) && int(r, 0x2b4) != 0
}
fn on_costs(r: &PartRec) -> bool {
    of(r, &[0, 1, 7, 8, 11, 12, 13, 16, 18])
}
fn on_weapon(r: &PartRec) -> bool {
    r.part_type == 0
}
fn on_adjust(r: &PartRec) -> bool {
    of(r, &[0, 1, 5, 8])
}
fn on_saves(r: &PartRec) -> bool {
    of(r, &[0, 1, 8, 15])
}
fn on_setting(r: &PartRec) -> bool {
    of(r, &[0, 1, 7, 8])
}
fn on_restrict(r: &PartRec) -> bool {
    of(r, &[0, 1, 8, 15])
}
fn on_magic_items(r: &PartRec) -> bool {
    of(r, &[0, 8])
}
fn on_skills(r: &PartRec) -> bool {
    on_main(r) && int(r, 0x598) != 0
}
fn on_general(r: &PartRec) -> bool {
    of(r, &[0, 7, 8, 13, 16])
}
fn on_schools(r: &PartRec) -> bool {
    r.part_type == 9
}
fn on_traits(r: &PartRec) -> bool {
    of(r, &[8, 11])
}
fn on_racial(r: &PartRec) -> bool {
    r.part_type == 5
}
fn on_class_abilities(r: &PartRec) -> bool {
    r.part_type == 14
}

/// The fields of every page but the kit's.
pub(crate) fn fields(flavor: Flavor) -> Vec<FieldDesc<PartRec>> {
    let expansion = flavor == Flavor::Expansion;
    let mut f: Vec<FieldDesc<PartRec>> = Vec::new();

    // IDD 155: name, the type's checks.
    f.extend([
        at!(MAIN, "Name", FieldKind::Text { max: 50 }, 0x198).visible(on_main),
        check!(MAIN, "Magic", 0x224)
            .visible(on_main)
            .enabled(|r| of(r, &[0, 1, 17])),
        int!(MAIN, "XP Value", 0, i32::MAX as i64, 0x230)
            .visible(on_main)
            .enabled(magic),
        check!(
            MAIN,
            "This is something a character could wear (Jewelry not included)",
            0x250
        )
        .visible(on_main)
        .enabled(|r| of(r, &[7, 8])),
        check!(MAIN, "This can be used to carry other items", 0x274)
            .visible(on_main)
            .enabled(|r| of(r, &[7, 8])),
        check!(MAIN, "This can be carried in a container", 0x2b4)
            .visible(on_main)
            .enabled(|r| of(r, &[0, 7, 8])),
        check!(MAIN, "Thieving Skills are adjusted", 0x598)
            .visible(on_main)
            .enabled(|r| of(r, &[1, 8, 12])),
    ]);
    let metal = check!(MAIN, "This item contains metal", 0x228).visible(on_main);
    let silver = check!(MAIN, "This item contains silver", 0x22c).visible(on_main);
    if expansion {
        f.push(metal.enabled(|r| of(r, &[0, 1, 7, 8])));
        f.push(silver.enabled(|r| of(r, &[0, 1, 7, 8])));
    } else {
        // Core Rules 2.0 disables them for weapons.
        f.push(metal.enabled(|r| of(r, &[1, 7, 8])));
        f.push(silver.enabled(|r| of(r, &[1, 7, 8])));
    }

    // IDD 176, the Non-Weapon Proficiencies dialog (type 4 only).
    let n = |d: FieldDesc<PartRec>| d.visible(on_nwp);
    let sub = || FieldKind::Combo {
        options: Options::Source("sub-abilities"),
        free: false,
    };
    f.extend([
        n(at!(NWP, "Name", FieldKind::Text { max: 50 }, 0x198)),
        n(int!(NWP, "Cost in character points", -1000, 1000, 0x478).group("Skills & Powers")),
        n(int!(NWP, "Initial Rating", -1000, 1000, 0x480).group("Skills & Powers")),
        n(int!(NWP, "Additional Languages", -1000, 1000, 0x470).group("Skills & Powers")),
        n(at!(NWP, "Relevant Ability Score", sub(), 0x458).group("Skills & Powers")),
        n(at!(NWP, "Second Relevant Ability Score", sub(), 0x45c).group("Skills & Powers")),
        n(int!(NWP, "Cost in slots", -1000, 1000, 0x47c).group("Core Rules")),
        n(int!(NWP, "Check Modifier", -1000, 1000, 0x484).group("Core Rules")),
        n(at!(
            NWP,
            "Ability Score Base",
            FieldKind::Combo {
                options: Options::Source("sub-abilities"),
                free: true
            },
            0x460
        )
        .group("Core Rules")),
        n(check!(
            NWP,
            "This Nonweapon Proficiency has Languages associated with it",
            0x75c
        )),
    ]);
    for (label, off) in [
        ("General", 0x48c),
        ("Priest", 0x490),
        ("Rogue", 0x494),
        ("Warrior", 0x498),
        ("Wizard", 0x49c),
        ("Psionicist", 0x4a0),
    ] {
        f.push(n(check!(NWP, label, off).group("Group(s)")));
    }
    f.extend([
        n(at!(
            NWP,
            "Trait",
            FieldKind::Combo {
                options: Options::Source("Traits"),
                free: false
            },
            0x468
        )
        .group("Rating bonuses")),
        n(int!(NWP, "Trait Bonus", -1000, 1000, 0x488).group("Rating bonuses")),
        n(at!(
            NWP,
            "Proficiency",
            FieldKind::Combo {
                options: Options::Source("Non-Weapon Proficiencies"),
                free: false
            },
            0x464
        )
        .group("Rating bonuses")),
        n(int!(NWP, "Proficiency Bonus", -1000, 1000, 0x474).group("Rating bonuses")),
        n(check!(NWP, "Allow Psionic Attacks", 0x798)),
        n(check!(NWP, "MAC Bonus", 0x79c)),
    ]);

    // IDD 156 Ability.
    let a = |d: FieldDesc<PartRec>| d.visible(on_abilities);
    for (group, members) in [
        (
            "Strength",
            [("Strength", 0x728), ("Stamina", 0x310), ("Muscle", 0x314)],
        ),
        (
            "Dexterity",
            [("Dexterity", 0x72c), ("Aim", 0x318), ("Balance", 0x31c)],
        ),
        (
            "Constitution",
            [
                ("Constitution", 0x730),
                ("Health", 0x320),
                ("Fitness", 0x324),
            ],
        ),
        (
            "Intelligence",
            [
                ("Intelligence", 0x734),
                ("Reason", 0x328),
                ("Knowledge", 0x32c),
            ],
        ),
        (
            "Wisdom",
            [
                ("Wisdom", 0x738),
                ("Intuition", 0x330),
                ("Willpower", 0x334),
            ],
        ),
        (
            "Charisma",
            [
                ("Charisma", 0x73c),
                ("Leadership", 0x338),
                ("Appearance", 0x33c),
            ],
        ),
    ] {
        for (label, off) in members {
            f.push(a(int!(ABILITIES, label, -25, 25, off).group(group)));
        }
        if group == "Strength" {
            f.push(a(
                int!(ABILITIES, "Extra Strength", -25, 100, 0x740).group(group)
            ));
        }
    }
    f.push(a(at!(
        ABILITIES,
        "Effects on Ability Scores",
        FieldKind::Choice(Options::Fixed(&[
            "Set to the amount indicated",
            "Adjust by the Amount indicated",
        ])),
        0x30c
    )));

    // IDD 157 Items Worn.
    let w = |d: FieldDesc<PartRec>| d.visible(on_worn);
    f.extend([
        w(at!(
            WORN,
            "Armor Type",
            FieldKind::Choice(Options::Source("armor-types")),
            0x248
        )),
        w(at!(
            WORN,
            "This item has no Armor Class",
            FieldKind::OptInt {
                min: -100,
                max: 99,
                none: 100,
                start: 0,
                inverted: true
            },
            0x270
        )
        .group("Armor Class")),
        w(check!(WORN, "This item sets AC; No bonuses apply", 0x808).group("Armor Class")),
    ]);
    for (label, off) in [
        ("as a Robe?", 0x254),
        ("as Gloves?", 0x26c),
        ("as Shoes or Boots?", 0x258),
        ("as a Hat or Helmet?", 0x25c),
        ("as a Knife Sheath?", 0x268),
        ("as a Quiver?", 0x264),
        ("as a Scabbard?", 0x260),
    ] {
        f.push(w(check!(WORN, label, off).group("Can this item be worn...")));
    }
    f.extend([
        w(int!(WORN, "Number of hands needed", 0, 2, 0x24c)),
        w(check!(
            WORN,
            "Include this Armor name as an Armor proficiency",
            0x42c
        )
        .group("Armor Proficiency Information")),
        w(at!(
            WORN,
            "Armor Proficiency",
            FieldKind::Combo {
                options: Options::Source("Armor"),
                free: true
            },
            0x430
        )
        .group("Armor Proficiency Information")
        .enabled(|r| int(r, 0x42c) == 0)),
        w(radio(
            WORN,
            "Shield Type",
            &[
                "The following do not apply",
                "Buckler",
                "Small",
                "Medium",
                "Body",
            ],
            0,
        )),
    ]);

    // IDD 160 Container Information.
    let c = |d: FieldDesc<PartRec>| d.visible(on_container);
    f.push(c(FieldDesc::new(
        CONTAINER,
        "Stowage Capacity in pounds",
        FieldKind::Real {
            min: 0.0,
            max: 1_000_000.0,
            none: Some((-1.0, "Not limited by weight capacity")),
        },
        |r: &PartRec, off| real_bits(r, off),
        |r, off, v, _| store_real(r, off, v),
    )
    .arg(0x278)));
    for (label, off) in [
        ("Arrows: not limited by number", 0x298),
        ("Knives: not limited by number", 0x284),
        ("Quarrels: not limited by number", 0x27c),
        ("Swords: not limited by number", 0x29c),
        ("Javelins: not limited by number", 0x2a0),
        ("Staves: not limited by number", 0x2a4),
    ] {
        f.push(c(
            unlimited!(CONTAINER, label, 1_000_000, off).group("This container can hold...")
        ));
    }
    for (label, off) in [
        ("a Hat", 0x288),
        ("Boots or Shoes", 0x280),
        ("Tack (Saddle)", 0x294),
        ("a Pack Animal", 0x290),
        ("a Transport", 0x28c),
    ] {
        f.push(c(
            check!(CONTAINER, label, off).group("This item is the same as...")
        ));
    }
    f.extend([
        c(FieldDesc::new(
            CONTAINER,
            "Not limited by bulk",
            FieldKind::Check,
            |r: &PartRec, _| Value::Bool(int(r, 0x2a8) < 0 && int(r, 0x2ac) == 0),
            |r, _, v, _| {
                store(r, 0x2a8, &Value::Int(if v.bool() { -1 } else { 0 }));
                store(r, 0x2ac, &Value::Int(0));
            },
        )
        .group("Bulk Points")),
        c(FieldDesc::new(
            CONTAINER,
            "Same as character's strength",
            FieldKind::Check,
            |r: &PartRec, _| Value::Bool(int(r, 0x2ac) != 0),
            |r, _, v, _| {
                store(r, 0x2a8, &Value::Int(if v.bool() { -1 } else { 0 }));
                store(r, 0x2ac, &Value::Int(v.bool().into()));
            },
        )
        .group("Bulk Points")
        .enabled(|r| !(int(r, 0x2a8) < 0 && int(r, 0x2ac) == 0))),
        c(int!(CONTAINER, "Bulk Points", 0, 10_000_000, 0x2a8)
            .group("Bulk Points")
            .enabled(|r| int(r, 0x2a8) >= 0)),
        c(check!(CONTAINER, "Do Not Add to Encumbrance", 0x2b0)),
    ]);

    // IDD 161 Can Carry in Containers.
    for (label, off) in [
        ("Quiver (Arrows)", 0x2c8),
        ("Knife Sheath (Knives)", 0x2c0),
        ("Bolt Case (Quarrels)", 0x2b8),
        ("Scabbard (Swords)", 0x2cc),
        ("Javelin Case (Javelins)", 0x2d0),
        ("Staff Case (Staves)", 0x2d4),
        ("a Hat", 0x2c4),
        ("Boots or Shoes", 0x2bc),
    ] {
        f.push(
            check!(CARRY, label, off)
                .group("Check the containers in which you may carry this item")
                .visible(on_carry),
        );
    }

    // IDD 163 Cost.
    let money = |r: &PartRec| on_costs(r) && of(r, &[0, 1, 7, 8, 13, 16]);
    for (book, base) in [("Player's Handbook", 0x208), ("Skills & Powers", 0x1f8)] {
        for (i, coin) in [
            "Platinum Pieces",
            "Gold Pieces",
            "Silver Pieces",
            "Copper Pieces",
        ]
        .into_iter()
        .enumerate()
        {
            f.push(
                int!(COSTS, coin, 0, 1_000_000_000, base + 4 * i, opened_costs)
                    .group(book)
                    .visible(money),
            );
        }
    }
    f.extend([
        int!(COSTS, "Cost", 0, 1_000_000_000, 0x308)
            .group("Character Points")
            .visible(|r| of(r, &[11, 18])),
        int!(COSTS, "Minor Access", 0, 1_000_000, 0x218)
            .group("Character Points")
            .visible(|r| r.part_type == 18),
        int!(
            COSTS,
            "Character Points gained (Intended for use with Disadvantages)",
            -1000,
            1000,
            0x220
        )
        .group("Character Points")
        .visible(|r| r.part_type == 12),
        at!(
            COSTS,
            "This race will recieve a one character point savings",
            FieldKind::Combo {
                options: Options::Source("main-races"),
                free: false
            },
            0x21c
        )
        .group("Character Points")
        .visible(|r| r.part_type == 11),
        at!(
            COSTS,
            "This race receives 1 additional point",
            FieldKind::Combo {
                options: Options::Source("main-races"),
                free: false
            },
            0x21c
        )
        .group("Character Points")
        .visible(|r| r.part_type == 12),
    ]);

    // IDD 164 Weapon Information (and 177, the weapons that fire it).
    let wp = |d: FieldDesc<PartRec>| d.visible(on_weapon);
    for (book, dice) in [("Player's Handbook", 0usize), ("Skills & Powers", 2)] {
        for (label, d) in [("Damage, Small-Medium", dice), ("Damage, Large", dice + 1)] {
            f.push(wp(FieldDesc::new(
                WEAPON,
                label,
                FieldKind::Dice,
                |r: &PartRec, d| Value::Dice(DICE[d].map(|off| value(r, off).int())),
                |r, d, v, _| {
                    opened_weapon(r);
                    for (off, x) in DICE[d].into_iter().zip(v.dice()) {
                        store(r, off, &Value::Int(x));
                    }
                },
            )
            .arg(d)
            .group(book)));
        }
        let phb = dice == 0;
        f.push(wp(int!(
            WEAPON,
            "Speed Factor",
            -100,
            1000,
            if phb { 0x404 } else { 0x3e0 },
            opened_weapon
        )
        .group(book)));
        for (label, off, options) in [
            (
                "Weapon Size",
                if phb { 0x3f4 } else { 0x3d0 },
                &["", "T", "S", "M", "L", "H", "G"][..],
            ),
            (
                "Weapon Type",
                if phb { 0x3e4 } else { 0x3b4 },
                &["", "B", "P", "S", "P/B", "P/S", "S/B", "S/P"][..],
            ),
            (
                "ROF",
                if phb { 0x408 } else { 0x41c },
                &["", "1", "2", "3/2", "2/1", "1/2", "3/1", "4/1", "5/1"][..],
            ),
        ] {
            let options: &'static [&'static str] = options;
            f.push(wp(at!(
                WEAPON,
                label,
                FieldKind::Combo {
                    options: Options::Fixed(options),
                    free: phb
                },
                off,
                opened_weapon
            )
            .group(book)));
        }
    }
    let ammo = |r: &PartRec| int(r, 0x420) != 0;
    f.extend([
        wp(int!(WEAPON, "Short", 0, 10000, 0x414, opened_weapon).group("Range")),
        wp(int!(WEAPON, "Medium", 0, 10000, 0x418, opened_weapon).group("Range")),
        wp(int!(WEAPON, "Long", 0, 10000, 0x410, opened_weapon).group("Range")),
        wp(check!(WEAPON, "Missile Weapon", 0x3bc, opened_weapon)
            .group("Use")
            .enabled(|r| int(r, 0x420) == 0)),
        wp(check!(WEAPON, "Needs Ammo", 0x3c0, opened_weapon)
            .group("Use")
            .enabled(|r| int(r, 0x3bc) != 0 && int(r, 0x420) == 0)),
        wp(check!(WEAPON, "Melee Weapon", 0x3b8, opened_weapon)
            .group("Use")
            .enabled(|r| int(r, 0x420) == 0)),
        wp(check!(WEAPON, "Ammunition", 0x420, opened_weapon)
            .group("Use")
            .enabled(|r| int(r, 0x3b8) == 0 && int(r, 0x3bc) == 0)),
        wp(at!(
            WEAPON,
            "Weapons which can fire the current Ammo",
            FieldKind::Dual(Options::Source("Weapons")),
            0x434,
            opened_weapon
        )
        .enabled(ammo)),
        wp(check!(WEAPON, "Do not apply low STR Penalty", 0x7b8, opened_weapon).group("Combat")),
        wp(int!(
            WEAPON,
            "Max Damage Bonus from STR",
            -100,
            100,
            0x7cc,
            opened_weapon
        )
        .group("Combat")),
        wp(int!(WEAPON, "# of hands needed", 0, 2, 0x24c, opened_weapon).group("Combat")),
        wp(int!(WEAPON, "Knockdown (d)", 0, 100, 0x40c, opened_weapon).group("Combat")),
    ]);
    if expansion {
        f.extend([
            wp(check!(WEAPON, "Thief Weapon", 0x448, opened_weapon).group("Combat")),
            wp(check!(
                WEAPON,
                "Applies to Cavalier horseman's bonus (Melee weapons only)",
                0x724,
                opened_weapon
            )
            .group("Use")
            .enabled(|r| int(r, 0x3b8) != 0)),
        ]);
    }
    // IDD 413, opened from the weapon page.
    f.extend([
        wp(check!(
            PROFS,
            "Include this weapon name as a weapon proficiency",
            0x424
        )),
        wp(at!(
            PROFS,
            "Proficiency needed",
            FieldKind::Combo {
                options: Options::Source("Weapons"),
                free: true
            },
            0x428
        )
        .enabled(|r| int(r, 0x424) == 0)),
        wp(at!(
            PROFS,
            "Weapon Groups to which this item belongs",
            FieldKind::Dual(Options::Source("tight-weapon-groups")),
            0x1cc
        )),
    ]);

    // IDD 166 Adjustment.
    let ad = |d: FieldDesc<PartRec>| d.visible(on_adjust);
    let selected = |r: &PartRec| int(r, 0x510) != 0;
    f.extend([
        ad(int!(ADJUST, "AC Adjustment", -100, 100, 0x44c)),
        ad(int!(ADJUST, "To Hit Adjustment", -100, 100, 0x454).group("Weapon Adjustments")),
        ad(int!(ADJUST, "To Damage Adjustment", -100, 100, 0x450).group("Weapon Adjustments")),
        ad(radio(
            ADJUST,
            "Conditions",
            &[
                "No Conditions",
                "With Selected Weapon(s) below",
                "With any Weapon of the user's choice",
                "With any Melee Weapon of the user's choice",
            ],
            1,
        )
        .group("Weapon Adjustments")),
        ad(at!(
            ADJUST,
            "Weapons which recieve the bonus",
            FieldKind::Dual(Options::Source("Weapons")),
            0x4fc
        )
        .enabled(selected)),
    ]);
    for (label, check, rate) in [
        ("Changes Land Movement Rate", 0x7ec, 0x7e8),
        ("Changes Swimming Movement Rate", 0x800, 0x7f8),
        ("Changes Flying Movement Rate", 0x804, 0x7fc),
    ] {
        f.push(ad(check!(ADJUST, label, check).group("Movement")));
        f.push(
            ad(int!(ADJUST, "Base Movement Rate", 0, 1000, rate).group("Movement"))
                .arg(rate)
                .enabled_at(|r, rate| int(r, rate + MOVE_CHECK[(rate - 0x7e8) / 4]) != 0),
        );
    }

    // IDD 167 Saving Throw Adjustments.
    for (label, off) in [
        ("Paralization", 0x2d8),
        ("Poison", 0x2dc),
        ("Death Magic", 0x2e0),
        ("Rod", 0x2e4),
        ("Staff", 0x2e8),
        ("Wand", 0x2ec),
        ("Petrification", 0x2f0),
        ("Polymorph", 0x2f4),
        ("Spell", 0x2f8),
        ("Breath Weapon", 0x2fc),
    ] {
        f.push(int!(SAVES, label, -100, 100, off).visible(on_saves));
    }

    // IDD 170 Schools Of Magic and IDD 171 Setting.
    f.extend([
        at!(
            SCHOOLS,
            "Scheme for this School",
            FieldKind::Combo {
                options: Options::Source("school-schemes"),
                free: false
            },
            0x1b4
        )
        .visible(on_schools),
        at!(
            SCHOOLS,
            "Schools of Opposition to the current School",
            FieldKind::Dual(Options::Source("Schools Of Magic")),
            0x1a0
        )
        .visible(on_schools),
        at!(
            SETTING,
            "Settings",
            FieldKind::Dual(Options::Source("settings")),
            0x1b8
        )
        .visible(on_setting),
    ]);

    // IDD 179 Class Restrictions.
    let rs = |d: FieldDesc<PartRec>| d.visible(on_restrict);
    if expansion {
        f.push(
            rs(at!(
                RESTRICT,
                "Classes which CANNOT use the current selection",
                FieldKind::Dual(Options::Source("classes")),
                0x82c
            ))
            .visible(|r| on_restrict(r) && r.part.data.allowed_classes.is_some()),
        );
    }
    let special = "Special Case (Will be used in addition to the Selected List)";
    f.extend([
        rs(check!(RESTRICT, "Any Specialist", 0x4c0).group(special)),
        rs(check!(RESTRICT, "Psionicist (5 CP Ability)", 0x4c8).group(special)),
        rs(check!(RESTRICT, "Psionicist (10 CP Ability)", 0x4cc).group(special)),
    ]);
    if expansion {
        f.push(rs(check!(
            RESTRICT,
            "Wizard must pay 3 extra CPs to be proficient with this weapon",
            0x4d0
        )
        .group(special)));
    }

    // IDD 180 Racial Abilities.
    let ra = |d: FieldDesc<PartRec>| d.visible(on_racial);
    f.extend([
        ra(at!(
            RACIAL,
            "Race",
            FieldKind::Combo {
                options: Options::Source("races"),
                free: false
            },
            0x580
        )),
        ra(int!(RACIAL, "CP Cost", -1000, 1000, 0x308)),
        ra(int!(
            RACIAL,
            "Purchases will be at a discount of (percent)",
            0,
            1000,
            0x51c
        )),
    ]);
    for (combo, level) in [(0x574, 0x53c), (0x578, 0x540)] {
        f.push(ra(at!(
            RACIAL,
            "Weapon Proficiency",
            FieldKind::Combo {
                options: Options::Source("Weapons"),
                free: false
            },
            combo
        )
        .group("Weapon Proficiencies granted at the specified level")));
        f.push(ra(int!(RACIAL, "at level", 0, 100, level)
            .group("Weapon Proficiencies granted at the specified level")));
    }
    f.extend([
        ra(at!(
            RACIAL,
            "Nonweapon Proficiency",
            FieldKind::Combo {
                options: Options::Source("Non-Weapon Proficiencies"),
                free: false
            },
            0x57c
        )
        .group("Nonweapon Proficiency Bonus (You must have the proficiency to gain the bonus)")),
        ra(int!(RACIAL, "Bonus", -1000, 1000, 0x530).group(
            "Nonweapon Proficiency Bonus (You must have the proficiency to gain the bonus)",
        )),
        ra(int!(RACIAL, "Hit Points", -1000, 1000, 0x520).group("Per level bonuses")),
        ra(
            int!(RACIAL, "Experience Points (percent)", -1000, 1000, 0x538)
                .group("Per level bonuses"),
        ),
    ]);
    let saves = "Saving Throw Bonuses (Lower is better)";
    for (label, off) in [
        ("Paralization", 0x550),
        ("Poison", 0x558),
        ("Death Magic", 0x54c),
        ("Rod", 0x560),
        ("Staff", 0x568),
        ("Wand", 0x56c),
        ("Petrification", 0x554),
        ("Polymorph", 0x55c),
        ("Spell", 0x564),
        ("Breath", 0x548),
    ] {
        f.push(ra(check!(RACIAL, label, off).group(saves)));
    }
    f.extend([
        ra(int!(
            RACIAL,
            "Bonus (zero uses the chart bonuses)",
            -100,
            100,
            0x534
        )
        .group(saves)),
        ra(check!(
            RACIAL,
            "Give this Ability it's own value (As a percent)",
            0x544
        )
        .group("Creating a new value")),
        ra(int!(RACIAL, "Starting Value", -1000, 1000, 0x524).group("Creating a new value")),
        ra(int!(RACIAL, "Increase Per Level", -1000, 1000, 0x528).group("Creating a new value")),
    ]);

    // IDD 182 Magic Items.
    let mi = |d: FieldDesc<PartRec>| d.visible(on_magic_items);
    f.push(mi(at!(
        MAGIC_ITEMS,
        "Category for this item",
        FieldKind::Choice(Options::Fixed(&[
            "None of the following",
            "Potions and Oils",
            "Scrolls",
            "Rings",
            "Rods",
            "Staves",
            "Wands",
            "Miscellaneous Magic: Books and Tomes",
            "Miscellaneous Magic: Jewels and Jewelry",
            "Miscellaneous Magic: Cloaks and Robes",
            "Miscellaneous Magic: Boots and Gloves",
            "Miscellaneous Magic: Girdles and Helms",
            "Miscellaneous Magic: Bags and Bottles",
            "Miscellaneous Magic: Dusts and Stones",
            "Miscellaneous Magic: Household Items and Tools",
            "Miscellaneous Magic: Musical Instruments",
            "Miscellaneous Magic: The Weird Stuff",
        ])),
        0x584
    )));
    for (label, off) in [
        ("Staff Of Power", 0x594),
        ("Ring Of Protection", 0x590),
        ("Cloak Of Protection", 0x58c),
        ("Bracers Of Defense", 0x588),
        ("Boots Of Speed", 0x7f0),
        ("Bracers Of Archery", 0x7f4),
        ("Ring of Clumsiness", 0x80c),
        ("Holy Avenger", 0x810),
        ("Girdle of Dwarvenkind", 0x814),
        ("Gauntlets of Dexterity", 0x818),
    ] {
        f.push(mi(check!(MAGIC_ITEMS, label, off).group("Functions As...")));
    }

    // IDD 183 Thieving Skill adjustments.
    for (label, off) in [
        ("Pick Pockets", 0x59c),
        ("Open Locks", 0x5a0),
        ("Find/Remove Traps", 0x5a4),
        ("Move Silently", 0x5a8),
        ("Hide in Shadows", 0x5ac),
        ("Detect Noise", 0x5b0),
        ("Climb Walls", 0x5b4),
        ("Read Languages", 0x5b8),
        ("Detect Magic", 0x5bc),
        ("Detect Illusion", 0x5c0),
        ("Bribe", 0x5c4),
        ("Tunneling", 0x5c8),
        ("Escape Bonds", 0x5cc),
    ] {
        f.push(
            int!(SKILLS, label, -250, 250, off)
                .group("Thieving Skill Racial Adjustments (percent)")
                .visible(on_skills),
        );
    }

    // IDD 332 Class Abilities (the Expansion only).
    if expansion {
        f.extend(class_abilities());
    }

    // IDD 363 General Information.
    let g = |d: FieldDesc<PartRec>| d.visible(on_general);
    f.extend([
        g(at!(
            GENERAL,
            "Only applies for selected race",
            FieldKind::Combo {
                options: Options::Source("main-races"),
                free: true
            },
            0x1e0,
            opened_general
        )),
        g(FieldDesc::new(
            GENERAL,
            "Player's Handbook",
            FieldKind::Real {
                min: 0.0,
                max: 1_000_000.0,
                none: None,
            },
            |r: &PartRec, _| Value::Real(f64::from(phb_weight(r))),
            |r, _, v, _| {
                opened_general(r);
                store_real(r, 0x1f0, v);
            },
        )
        .group("Weight (pounds)")),
        g(FieldDesc::new(
            GENERAL,
            "Skills & Powers",
            FieldKind::Real {
                min: 0.0,
                max: 1_000_000.0,
                none: None,
            },
            |r: &PartRec, off| real_bits(r, off),
            |r, off, v, _| {
                opened_general(r);
                store_real(r, off, v);
            },
        )
        .arg(0x1ec)
        .group("Weight (pounds)")),
        g(FieldDesc::new(
            GENERAL,
            "Bulk Points",
            FieldKind::Real {
                min: 0.0,
                max: 10_000_000.0,
                none: None,
            },
            |r: &PartRec, _| Value::Real(f64::from(bulk(r))),
            |r, off, v, _| {
                opened_general(r);
                store_real(r, off, v);
            },
        )
        .arg(0x1f4)),
        g(int!(
            GENERAL,
            "Pages for spells",
            0,
            1_000_000,
            0x7e4,
            opened_general
        )
        .enabled(|r| !of(r, &[0, 1, 13, 16]))),
        g(at!(
            GENERAL,
            "Equipment Group",
            FieldKind::Combo {
                options: Options::Source("equipment-categories"),
                free: true
            },
            0x240,
            opened_general
        )
        .enabled(|r| r.part_type == 7)),
        g(at!(
            GENERAL,
            "Demihuman Equipment",
            FieldKind::Combo {
                options: Options::Source("racial-equipment"),
                free: true
            },
            0x244,
            opened_general
        )),
    ]);

    // IDD 364 Traits.
    f.push(check!(TRAITS, "Allow Character access to Wild Talents", 0x744).visible(on_traits));
    f
}

/// Land, swim and fly rates (+0x7e8, +0x7f8, +0x7fc) and the offset from
/// each to its check (+0x7ec, +0x800, +0x804), by `(rate - 0x7e8) / 4`.
const MOVE_CHECK: [usize; 6] = [4, 0, 0, 0, 8, 8];

/// Count, sides and bonus of the damage dice: PHB S-M, PHB L, S&P S-M,
/// S&P L.
const DICE: [[usize; 3]; 4] = [
    [0x3fc, 0x3f8, 0x400],
    [0x3e8, 0x3ec, 0x3f0],
    [0x3d8, 0x3d4, 0x3dc],
    [0x3c4, 0x3c8, 0x3cc],
];

/// IDD 332: the part type Class Abilities.
fn class_abilities() -> Vec<FieldDesc<PartRec>> {
    const P: &str = CLASS_ABILITIES;
    let ca = |d: FieldDesc<PartRec>| d.visible(on_class_abilities);
    let mut f = vec![
        ca(int!(
            P,
            "Character Points gained (Intended for use with Disadvantages)",
            -1000,
            1000,
            0x220
        )),
        ca(int!(P, "Character Point Cost", -1000, 1000, 0x308)),
        ca(int!(
            P,
            "Turn Undead starting at level (0=never)",
            0,
            100,
            0x778
        )),
    ];
    for (label, off) in [
        ("Multiple Specialization", 0x783),
        ("Cleric Weapon Allowance", 0x7a0),
        ("Melee Weapons Only", 0x7c0),
        ("Cleric Weapons Only", 0x7c4),
        ("Thief Weapons Only", 0x7c8),
        ("Defense Bonus (+2 to AC if unarmored/unencumbered)", 0x780),
        ("Armor Use Lesser", 0x824),
        ("Armor Use Major", 0x828),
        ("Weapon Use Lesser", 0x81c),
        ("Weapon Use Major", 0x820),
    ] {
        f.push(ca(check!(P, label, off)));
    }
    f.extend([
        ca(at!(
            P,
            "Free NWP",
            FieldKind::Combo {
                options: Options::Source("Non-Weapon Proficiencies"),
                free: true
            },
            0x718
        )),
        ca(check!(P, "Apply Ranger Bonus", 0x71c)),
        ca(FieldDesc::new(
            P,
            "Value",
            FieldKind::Choice(Options::Fixed(&[
                "No score",
                "Dexterity / Level",
                "Base and increment",
            ])),
            |r: &PartRec, _| {
                Value::Int(match int(r, 0x77c) {
                    1 => 1,
                    5 => 2,
                    _ => 0,
                })
            },
            |r, _, v, _| {
                let n = [0, 1, 5][v.int().clamp(0, 2) as usize];
                store(r, 0x77c, &Value::Int(n));
            },
        )
        .group("Creating a new value")),
    ]);
    let based = |r: &PartRec| int(r, 0x77c) == 5;
    f.extend([
        ca(int!(P, "Starting Value", -20, 100, 0x524)
            .group("Creating a new value")
            .enabled(based)),
        ca(int!(P, "Increase Per Level", -20, 100, 0x528)
            .group("Creating a new value")
            .enabled(based)),
        ca(int!(P, "Maximum", 0, 1000, 0x52c)
            .group("Creating a new value")
            .enabled(based)),
    ]);
    for (label, off) in [
        ("Psychic Adept", 0x785),
        ("PSP Bonus", 0x7bc),
        ("Contact (5)", 0x794),
        ("Contact (10)", 0x788),
        ("Mental Defense (5)", 0x790),
        ("Mental Defense (10)", 0x78c),
    ] {
        f.push(ca(check!(P, label, off).group("Psionicist")));
    }
    for (label, off) in [
        ("Gains benefits of high Con as a warrior", 0x7b0),
        ("Allowed Exceptional Strength", 0x7b4),
        ("Specialization", 0x782),
        ("Access to improved spell tables", 0x784),
        ("Increase hit dice (+2 to number of sides)", 0x781),
    ] {
        f.push(ca(check!(P, label, off).group("General")));
    }
    for (label, off) in [
        ("Specialist Mage Bonus Spells", 0x7ac),
        ("Specialist Mage Learning Bonus", 0x7a8),
        ("Specialist Mage Learning Penalty", 0x7a4),
    ] {
        f.push(ca(check!(P, label, off).group("Specialist Mage")));
    }
    f
}

/// The Description button of IDD 155 and IDD 176.
pub(crate) fn description() -> FieldDesc<PartRec> {
    at!(DESCRIPTION, "Description", FieldKind::MultiLine, 0x4)
}
