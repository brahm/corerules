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
use cr_i18n::tr_noop;

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
pub(crate) const MAIN: &str = tr_noop!("Part");
const NWP: &str = tr_noop!("Nonweapon Proficiency");
const ABILITIES: &str = tr_noop!("Abilities");
const WORN: &str = tr_noop!("Items Worn");
const CONTAINER: &str = tr_noop!("Container");
const CARRY: &str = tr_noop!("Carry Me");
const COSTS: &str = tr_noop!("Costs");
const WEAPON: &str = tr_noop!("Weapon");
const PROFS: &str = tr_noop!("Weapon Proficiencies");
const ADJUST: &str = tr_noop!("Adjustments");
const SAVES: &str = tr_noop!("Saving Throws");
const SCHOOLS: &str = tr_noop!("Schools");
const SETTING: &str = tr_noop!("Setting");
const RESTRICT: &str = tr_noop!("Restrict Class");
const RACIAL: &str = tr_noop!("Racial");
const MAGIC_ITEMS: &str = tr_noop!("Magic Items");
const SKILLS: &str = tr_noop!("Skills");
const CLASS_ABILITIES: &str = "Class Abilities";
const GENERAL: &str = tr_noop!("General Info");
const TRAITS: &str = tr_noop!("Traits");
pub(crate) const DESCRIPTION: &str = tr_noop!("Description");

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
        at!(MAIN, tr_noop!("Name"), FieldKind::Text { max: 50 }, 0x198).visible(on_main),
        check!(MAIN, tr_noop!("Magic"), 0x224)
            .visible(on_main)
            .enabled(|r| of(r, &[0, 1, 17])),
        int!(MAIN, tr_noop!("XP Value"), 0, i32::MAX as i64, 0x230)
            .visible(on_main)
            .enabled(magic),
        check!(
            MAIN,
            tr_noop!("This is something a character could wear (Jewelry not included)"),
            0x250
        )
        .visible(on_main)
        .enabled(|r| of(r, &[7, 8])),
        check!(
            MAIN,
            tr_noop!("This can be used to carry other items"),
            0x274
        )
        .visible(on_main)
        .enabled(|r| of(r, &[7, 8])),
        check!(MAIN, tr_noop!("This can be carried in a container"), 0x2b4)
            .visible(on_main)
            .enabled(|r| of(r, &[0, 7, 8])),
        check!(MAIN, tr_noop!("Thieving Skills are adjusted"), 0x598)
            .visible(on_main)
            .enabled(|r| of(r, &[1, 8, 12])),
    ]);
    let metal = check!(MAIN, tr_noop!("This item contains metal"), 0x228).visible(on_main);
    let silver = check!(MAIN, tr_noop!("This item contains silver"), 0x22c).visible(on_main);
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
        n(at!(
            NWP,
            tr_noop!("Name"),
            FieldKind::Text { max: 50 },
            0x198
        )),
        n(int!(
            NWP,
            tr_noop!("Cost in character points"),
            -1000,
            1000,
            0x478
        )
        .group(tr_noop!("Skills & Powers"))),
        n(int!(NWP, tr_noop!("Initial Rating"), -1000, 1000, 0x480)
            .group(tr_noop!("Skills & Powers"))),
        n(
            int!(NWP, tr_noop!("Additional Languages"), -1000, 1000, 0x470)
                .group(tr_noop!("Skills & Powers")),
        ),
        n(at!(NWP, tr_noop!("Relevant Ability Score"), sub(), 0x458)
            .group(tr_noop!("Skills & Powers"))),
        n(
            at!(NWP, tr_noop!("Second Relevant Ability Score"), sub(), 0x45c)
                .group(tr_noop!("Skills & Powers")),
        ),
        n(int!(NWP, tr_noop!("Cost in slots"), -1000, 1000, 0x47c).group(tr_noop!("Core Rules"))),
        n(int!(NWP, tr_noop!("Check Modifier"), -1000, 1000, 0x484).group(tr_noop!("Core Rules"))),
        n(at!(
            NWP,
            tr_noop!("Ability Score Base"),
            FieldKind::Combo {
                options: Options::Source("sub-abilities"),
                free: true
            },
            0x460
        )
        .group(tr_noop!("Core Rules"))),
        n(check!(
            NWP,
            tr_noop!("This Nonweapon Proficiency has Languages associated with it"),
            0x75c
        )),
    ]);
    for (label, off) in [
        (tr_noop!("General"), 0x48c),
        (tr_noop!("Priest"), 0x490),
        (tr_noop!("Rogue"), 0x494),
        (tr_noop!("Warrior"), 0x498),
        (tr_noop!("Wizard"), 0x49c),
        (tr_noop!("Psionicist"), 0x4a0),
    ] {
        f.push(n(check!(NWP, label, off).group(tr_noop!("Group(s)"))));
    }
    f.extend([
        n(at!(
            NWP,
            tr_noop!("Trait"),
            FieldKind::Combo {
                options: Options::Source("Traits"),
                free: false
            },
            0x468
        )
        .group(tr_noop!("Rating bonuses"))),
        n(
            int!(NWP, tr_noop!("Trait Bonus"), -1000, 1000, 0x488)
                .group(tr_noop!("Rating bonuses")),
        ),
        n(at!(
            NWP,
            tr_noop!("Proficiency"),
            FieldKind::Combo {
                options: Options::Source("Non-Weapon Proficiencies"),
                free: false
            },
            0x464
        )
        .group(tr_noop!("Rating bonuses"))),
        n(int!(NWP, tr_noop!("Proficiency Bonus"), -1000, 1000, 0x474)
            .group(tr_noop!("Rating bonuses"))),
        n(check!(NWP, tr_noop!("Allow Psionic Attacks"), 0x798)),
        n(check!(NWP, tr_noop!("MAC Bonus"), 0x79c)),
    ]);

    // IDD 156 Ability.
    let a = |d: FieldDesc<PartRec>| d.visible(on_abilities);
    for (group, members) in [
        (
            tr_noop!("Strength"),
            [
                (tr_noop!("Strength"), 0x728),
                (tr_noop!("Stamina"), 0x310),
                (tr_noop!("Muscle"), 0x314),
            ],
        ),
        (
            tr_noop!("Dexterity"),
            [
                (tr_noop!("Dexterity"), 0x72c),
                (tr_noop!("Aim"), 0x318),
                (tr_noop!("Balance"), 0x31c),
            ],
        ),
        (
            tr_noop!("Constitution"),
            [
                (tr_noop!("Constitution"), 0x730),
                (tr_noop!("Health"), 0x320),
                (tr_noop!("Fitness"), 0x324),
            ],
        ),
        (
            tr_noop!("Intelligence"),
            [
                (tr_noop!("Intelligence"), 0x734),
                (tr_noop!("Reason"), 0x328),
                (tr_noop!("Knowledge"), 0x32c),
            ],
        ),
        (
            tr_noop!("Wisdom"),
            [
                (tr_noop!("Wisdom"), 0x738),
                (tr_noop!("Intuition"), 0x330),
                (tr_noop!("Willpower"), 0x334),
            ],
        ),
        (
            tr_noop!("Charisma"),
            [
                (tr_noop!("Charisma"), 0x73c),
                (tr_noop!("Leadership"), 0x338),
                (tr_noop!("Appearance"), 0x33c),
            ],
        ),
    ] {
        for (label, off) in members {
            f.push(a(int!(ABILITIES, label, -25, 25, off).group(group)));
        }
        if group == "Strength" {
            f.push(a(int!(
                ABILITIES,
                tr_noop!("Extra Strength"),
                -25,
                100,
                0x740
            )
            .group(group)));
        }
    }
    f.push(a(at!(
        ABILITIES,
        tr_noop!("Effects on Ability Scores"),
        FieldKind::Choice(Options::Fixed(&[
            tr_noop!("Set to the amount indicated"),
            tr_noop!("Adjust by the Amount indicated"),
        ])),
        0x30c
    )));

    // IDD 157 Items Worn.
    let w = |d: FieldDesc<PartRec>| d.visible(on_worn);
    f.extend([
        w(at!(
            WORN,
            tr_noop!("Armor Type"),
            FieldKind::Choice(Options::Source("armor-types")),
            0x248
        )),
        w(at!(
            WORN,
            tr_noop!("This item has no Armor Class"),
            FieldKind::OptInt {
                min: -100,
                max: 99,
                none: 100,
                start: 0,
                inverted: true
            },
            0x270
        )
        .group(tr_noop!("Armor Class"))),
        w(
            check!(WORN, tr_noop!("This item sets AC; No bonuses apply"), 0x808)
                .group(tr_noop!("Armor Class")),
        ),
    ]);
    for (label, off) in [
        (tr_noop!("as a Robe?"), 0x254),
        (tr_noop!("as Gloves?"), 0x26c),
        (tr_noop!("as Shoes or Boots?"), 0x258),
        (tr_noop!("as a Hat or Helmet?"), 0x25c),
        (tr_noop!("as a Knife Sheath?"), 0x268),
        (tr_noop!("as a Quiver?"), 0x264),
        (tr_noop!("as a Scabbard?"), 0x260),
    ] {
        f.push(w(
            check!(WORN, label, off).group(tr_noop!("Can this item be worn..."))
        ));
    }
    f.extend([
        w(int!(WORN, tr_noop!("Number of hands needed"), 0, 2, 0x24c)),
        w(check!(
            WORN,
            tr_noop!("Include this Armor name as an Armor proficiency"),
            0x42c
        )
        .group(tr_noop!("Armor Proficiency Information"))),
        w(at!(
            WORN,
            tr_noop!("Armor Proficiency"),
            FieldKind::Combo {
                options: Options::Source("Armor"),
                free: true
            },
            0x430
        )
        .group(tr_noop!("Armor Proficiency Information"))
        .enabled(|r| int(r, 0x42c) == 0)),
        w(radio(
            WORN,
            tr_noop!("Shield Type"),
            &[
                tr_noop!("The following do not apply"),
                tr_noop!("Buckler"),
                tr_noop!("Small"),
                tr_noop!("Medium"),
                tr_noop!("Body"),
            ],
            0,
        )),
    ]);

    // IDD 160 Container Information.
    let c = |d: FieldDesc<PartRec>| d.visible(on_container);
    f.push(c(FieldDesc::new(
        CONTAINER,
        tr_noop!("Stowage Capacity in pounds"),
        FieldKind::Real {
            min: 0.0,
            max: 1_000_000.0,
            none: Some((-1.0, tr_noop!("Not limited by weight capacity"))),
        },
        |r: &PartRec, off| real_bits(r, off),
        |r, off, v, _| store_real(r, off, v),
    )
    .arg(0x278)));
    for (label, off) in [
        (tr_noop!("Arrows: not limited by number"), 0x298),
        (tr_noop!("Knives: not limited by number"), 0x284),
        (tr_noop!("Quarrels: not limited by number"), 0x27c),
        (tr_noop!("Swords: not limited by number"), 0x29c),
        (tr_noop!("Javelins: not limited by number"), 0x2a0),
        (tr_noop!("Staves: not limited by number"), 0x2a4),
    ] {
        f.push(c(unlimited!(CONTAINER, label, 1_000_000, off)
            .group(tr_noop!("This container can hold..."))));
    }
    for (label, off) in [
        (tr_noop!("a Hat"), 0x288),
        (tr_noop!("Boots or Shoes"), 0x280),
        (tr_noop!("Tack (Saddle)"), 0x294),
        (tr_noop!("a Pack Animal"), 0x290),
        (tr_noop!("a Transport"), 0x28c),
    ] {
        f.push(c(
            check!(CONTAINER, label, off).group(tr_noop!("This item is the same as..."))
        ));
    }
    f.extend([
        c(FieldDesc::new(
            CONTAINER,
            tr_noop!("Not limited by bulk"),
            FieldKind::Check,
            |r: &PartRec, _| Value::Bool(int(r, 0x2a8) < 0 && int(r, 0x2ac) == 0),
            |r, _, v, _| {
                store(r, 0x2a8, &Value::Int(if v.bool() { -1 } else { 0 }));
                store(r, 0x2ac, &Value::Int(0));
            },
        )
        .group(tr_noop!("Bulk Points"))),
        c(FieldDesc::new(
            CONTAINER,
            tr_noop!("Same as character's strength"),
            FieldKind::Check,
            |r: &PartRec, _| Value::Bool(int(r, 0x2ac) != 0),
            |r, _, v, _| {
                store(r, 0x2a8, &Value::Int(if v.bool() { -1 } else { 0 }));
                store(r, 0x2ac, &Value::Int(v.bool().into()));
            },
        )
        .group(tr_noop!("Bulk Points"))
        .enabled(|r| !(int(r, 0x2a8) < 0 && int(r, 0x2ac) == 0))),
        c(
            int!(CONTAINER, tr_noop!("Bulk Points"), 0, 10_000_000, 0x2a8)
                .group(tr_noop!("Bulk Points"))
                .enabled(|r| int(r, 0x2a8) >= 0),
        ),
        c(check!(
            CONTAINER,
            tr_noop!("Do Not Add to Encumbrance"),
            0x2b0
        )),
    ]);

    // IDD 161 Can Carry in Containers.
    for (label, off) in [
        (tr_noop!("Quiver (Arrows)"), 0x2c8),
        (tr_noop!("Knife Sheath (Knives)"), 0x2c0),
        (tr_noop!("Bolt Case (Quarrels)"), 0x2b8),
        (tr_noop!("Scabbard (Swords)"), 0x2cc),
        (tr_noop!("Javelin Case (Javelins)"), 0x2d0),
        (tr_noop!("Staff Case (Staves)"), 0x2d4),
        (tr_noop!("a Hat"), 0x2c4),
        (tr_noop!("Boots or Shoes"), 0x2bc),
    ] {
        f.push(
            check!(CARRY, label, off)
                .group(tr_noop!(
                    "Check the containers in which you may carry this item"
                ))
                .visible(on_carry),
        );
    }

    // IDD 163 Cost.
    let money = |r: &PartRec| on_costs(r) && of(r, &[0, 1, 7, 8, 13, 16]);
    for (book, base) in [("Player's Handbook", 0x208), ("Skills & Powers", 0x1f8)] {
        for (i, coin) in [
            tr_noop!("Platinum Pieces"),
            tr_noop!("Gold Pieces"),
            tr_noop!("Silver Pieces"),
            tr_noop!("Copper Pieces"),
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
        int!(COSTS, tr_noop!("Cost"), 0, 1_000_000_000, 0x308)
            .group(tr_noop!("Character Points"))
            .visible(|r| of(r, &[11, 18])),
        int!(COSTS, tr_noop!("Minor Access"), 0, 1_000_000, 0x218)
            .group(tr_noop!("Character Points"))
            .visible(|r| r.part_type == 18),
        int!(
            COSTS,
            tr_noop!("Character Points gained (Intended for use with Disadvantages)"),
            -1000,
            1000,
            0x220
        )
        .group(tr_noop!("Character Points"))
        .visible(|r| r.part_type == 12),
        at!(
            COSTS,
            tr_noop!("This race will recieve a one character point savings"),
            FieldKind::Combo {
                options: Options::Source("main-races"),
                free: false
            },
            0x21c
        )
        .group(tr_noop!("Character Points"))
        .visible(|r| r.part_type == 11),
        at!(
            COSTS,
            tr_noop!("This race receives 1 additional point"),
            FieldKind::Combo {
                options: Options::Source("main-races"),
                free: false
            },
            0x21c
        )
        .group(tr_noop!("Character Points"))
        .visible(|r| r.part_type == 12),
    ]);

    // IDD 164 Weapon Information (and 177, the weapons that fire it).
    let wp = |d: FieldDesc<PartRec>| d.visible(on_weapon);
    for (book, dice) in [("Player's Handbook", 0usize), ("Skills & Powers", 2)] {
        for (label, d) in [
            (tr_noop!("Damage, Small-Medium"), dice),
            (tr_noop!("Damage, Large"), dice + 1),
        ] {
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
            tr_noop!("Speed Factor"),
            -100,
            1000,
            if phb { 0x404 } else { 0x3e0 },
            opened_weapon
        )
        .group(book)));
        for (label, off, options) in [
            (
                tr_noop!("Weapon Size"),
                if phb { 0x3f4 } else { 0x3d0 },
                &["", "T", "S", "M", "L", "H", "G"][..],
            ),
            (
                tr_noop!("Weapon Type"),
                if phb { 0x3e4 } else { 0x3b4 },
                &["", "B", "P", "S", "P/B", "P/S", "S/B", "S/P"][..],
            ),
            (
                tr_noop!("ROF"),
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
        wp(
            int!(WEAPON, tr_noop!("Short"), 0, 10000, 0x414, opened_weapon)
                .group(tr_noop!("Range")),
        ),
        wp(
            int!(WEAPON, tr_noop!("Medium"), 0, 10000, 0x418, opened_weapon)
                .group(tr_noop!("Range")),
        ),
        wp(int!(WEAPON, tr_noop!("Long"), 0, 10000, 0x410, opened_weapon).group(tr_noop!("Range"))),
        wp(
            check!(WEAPON, tr_noop!("Missile Weapon"), 0x3bc, opened_weapon)
                .group(tr_noop!("Use"))
                .enabled(|r| int(r, 0x420) == 0),
        ),
        wp(check!(WEAPON, tr_noop!("Needs Ammo"), 0x3c0, opened_weapon)
            .group(tr_noop!("Use"))
            .enabled(|r| int(r, 0x3bc) != 0 && int(r, 0x420) == 0)),
        wp(
            check!(WEAPON, tr_noop!("Melee Weapon"), 0x3b8, opened_weapon)
                .group(tr_noop!("Use"))
                .enabled(|r| int(r, 0x420) == 0),
        ),
        wp(check!(WEAPON, tr_noop!("Ammunition"), 0x420, opened_weapon)
            .group(tr_noop!("Use"))
            .enabled(|r| int(r, 0x3b8) == 0 && int(r, 0x3bc) == 0)),
        wp(at!(
            WEAPON,
            tr_noop!("Weapons which can fire the current Ammo"),
            FieldKind::Dual(Options::Source("Weapons")),
            0x434,
            opened_weapon
        )
        .enabled(ammo)),
        wp(check!(
            WEAPON,
            tr_noop!("Do not apply low STR Penalty"),
            0x7b8,
            opened_weapon
        )
        .group(tr_noop!("Combat"))),
        wp(int!(
            WEAPON,
            tr_noop!("Max Damage Bonus from STR"),
            -100,
            100,
            0x7cc,
            opened_weapon
        )
        .group(tr_noop!("Combat"))),
        wp(int!(
            WEAPON,
            tr_noop!("# of hands needed"),
            0,
            2,
            0x24c,
            opened_weapon
        )
        .group(tr_noop!("Combat"))),
        wp(int!(
            WEAPON,
            tr_noop!("Knockdown (d)"),
            0,
            100,
            0x40c,
            opened_weapon
        )
        .group(tr_noop!("Combat"))),
    ]);
    if expansion {
        f.extend([
            wp(
                check!(WEAPON, tr_noop!("Thief Weapon"), 0x448, opened_weapon)
                    .group(tr_noop!("Combat")),
            ),
            wp(check!(
                WEAPON,
                tr_noop!("Applies to Cavalier horseman's bonus (Melee weapons only)"),
                0x724,
                opened_weapon
            )
            .group(tr_noop!("Use"))
            .enabled(|r| int(r, 0x3b8) != 0)),
        ]);
    }
    // IDD 413, opened from the weapon page.
    f.extend([
        wp(check!(
            PROFS,
            tr_noop!("Include this weapon name as a weapon proficiency"),
            0x424
        )),
        wp(at!(
            PROFS,
            tr_noop!("Proficiency needed"),
            FieldKind::Combo {
                options: Options::Source("Weapons"),
                free: true
            },
            0x428
        )
        .enabled(|r| int(r, 0x424) == 0)),
        wp(at!(
            PROFS,
            tr_noop!("Weapon Groups to which this item belongs"),
            FieldKind::Dual(Options::Source("tight-weapon-groups")),
            0x1cc
        )),
    ]);

    // IDD 166 Adjustment.
    let ad = |d: FieldDesc<PartRec>| d.visible(on_adjust);
    let selected = |r: &PartRec| int(r, 0x510) != 0;
    f.extend([
        ad(int!(ADJUST, tr_noop!("AC Adjustment"), -100, 100, 0x44c)),
        ad(
            int!(ADJUST, tr_noop!("To Hit Adjustment"), -100, 100, 0x454)
                .group(tr_noop!("Weapon Adjustments")),
        ),
        ad(
            int!(ADJUST, tr_noop!("To Damage Adjustment"), -100, 100, 0x450)
                .group(tr_noop!("Weapon Adjustments")),
        ),
        ad(radio(
            ADJUST,
            tr_noop!("Conditions"),
            &[
                tr_noop!("No Conditions"),
                tr_noop!("With Selected Weapon(s) below"),
                tr_noop!("With any Weapon of the user's choice"),
                tr_noop!("With any Melee Weapon of the user's choice"),
            ],
            1,
        )
        .group(tr_noop!("Weapon Adjustments"))),
        ad(at!(
            ADJUST,
            tr_noop!("Weapons which recieve the bonus"),
            FieldKind::Dual(Options::Source("Weapons")),
            0x4fc
        )
        .enabled(selected)),
    ]);
    for (label, check, rate) in [
        (tr_noop!("Changes Land Movement Rate"), 0x7ec, 0x7e8),
        (tr_noop!("Changes Swimming Movement Rate"), 0x800, 0x7f8),
        (tr_noop!("Changes Flying Movement Rate"), 0x804, 0x7fc),
    ] {
        f.push(ad(check!(ADJUST, label, check).group(tr_noop!("Movement"))));
        f.push(
            ad(int!(ADJUST, tr_noop!("Base Movement Rate"), 0, 1000, rate)
                .group(tr_noop!("Movement")))
            .arg(rate)
            .enabled_at(|r, rate| int(r, rate + MOVE_CHECK[(rate - 0x7e8) / 4]) != 0),
        );
    }

    // IDD 167 Saving Throw Adjustments.
    for (label, off) in [
        (tr_noop!("Paralization"), 0x2d8),
        (tr_noop!("Poison"), 0x2dc),
        (tr_noop!("Death Magic"), 0x2e0),
        (tr_noop!("Rod"), 0x2e4),
        (tr_noop!("Staff"), 0x2e8),
        (tr_noop!("Wand"), 0x2ec),
        (tr_noop!("Petrification"), 0x2f0),
        (tr_noop!("Polymorph"), 0x2f4),
        (tr_noop!("Spell"), 0x2f8),
        (tr_noop!("Breath Weapon"), 0x2fc),
    ] {
        f.push(int!(SAVES, label, -100, 100, off).visible(on_saves));
    }

    // IDD 170 Schools Of Magic and IDD 171 Setting.
    f.extend([
        at!(
            SCHOOLS,
            tr_noop!("Scheme for this School"),
            FieldKind::Combo {
                options: Options::Source("school-schemes"),
                free: false
            },
            0x1b4
        )
        .visible(on_schools),
        at!(
            SCHOOLS,
            tr_noop!("Schools of Opposition to the current School"),
            FieldKind::Dual(Options::Source("Schools Of Magic")),
            0x1a0
        )
        .visible(on_schools),
        at!(
            SETTING,
            tr_noop!("Settings"),
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
                tr_noop!("Classes which CANNOT use the current selection"),
                FieldKind::Dual(Options::Source("classes")),
                0x82c
            ))
            .visible(|r| on_restrict(r) && r.part.data.allowed_classes.is_some()),
        );
    }
    let special = tr_noop!("Special Case (Will be used in addition to the Selected List)");
    f.extend([
        rs(check!(RESTRICT, tr_noop!("Any Specialist"), 0x4c0).group(special)),
        rs(check!(RESTRICT, tr_noop!("Psionicist (5 CP Ability)"), 0x4c8).group(special)),
        rs(check!(RESTRICT, tr_noop!("Psionicist (10 CP Ability)"), 0x4cc).group(special)),
    ]);
    if expansion {
        f.push(rs(check!(
            RESTRICT,
            tr_noop!("Wizard must pay 3 extra CPs to be proficient with this weapon"),
            0x4d0
        )
        .group(special)));
    }

    // IDD 180 Racial Abilities.
    let ra = |d: FieldDesc<PartRec>| d.visible(on_racial);
    f.extend([
        ra(at!(
            RACIAL,
            tr_noop!("Race"),
            FieldKind::Combo {
                options: Options::Source("races"),
                free: false
            },
            0x580
        )),
        ra(int!(RACIAL, tr_noop!("CP Cost"), -1000, 1000, 0x308)),
        ra(int!(
            RACIAL,
            tr_noop!("Purchases will be at a discount of (percent)"),
            0,
            1000,
            0x51c
        )),
    ]);
    for (combo, level) in [(0x574, 0x53c), (0x578, 0x540)] {
        f.push(ra(at!(
            RACIAL,
            tr_noop!("Weapon Proficiency"),
            FieldKind::Combo {
                options: Options::Source("Weapons"),
                free: false
            },
            combo
        )
        .group(tr_noop!(
            "Weapon Proficiencies granted at the specified level"
        ))));
        f.push(ra(int!(RACIAL, tr_noop!("at level"), 0, 100, level).group(
            tr_noop!("Weapon Proficiencies granted at the specified level"),
        )));
    }
    f.extend([
        ra(at!(
            RACIAL,
            tr_noop!("Nonweapon Proficiency"),
            FieldKind::Combo {
                options: Options::Source("Non-Weapon Proficiencies"),
                free: false
            },
            0x57c
        )
        .group(tr_noop!(
            "Nonweapon Proficiency Bonus (You must have the proficiency to gain the bonus)"
        ))),
        ra(
            int!(RACIAL, tr_noop!("Bonus"), -1000, 1000, 0x530).group(tr_noop!(
                "Nonweapon Proficiency Bonus (You must have the proficiency to gain the bonus)"
            )),
        ),
        ra(int!(RACIAL, tr_noop!("Hit Points"), -1000, 1000, 0x520)
            .group(tr_noop!("Per level bonuses"))),
        ra(int!(
            RACIAL,
            tr_noop!("Experience Points (percent)"),
            -1000,
            1000,
            0x538
        )
        .group(tr_noop!("Per level bonuses"))),
    ]);
    let saves = tr_noop!("Saving Throw Bonuses (Lower is better)");
    for (label, off) in [
        (tr_noop!("Paralization"), 0x550),
        (tr_noop!("Poison"), 0x558),
        (tr_noop!("Death Magic"), 0x54c),
        (tr_noop!("Rod"), 0x560),
        (tr_noop!("Staff"), 0x568),
        (tr_noop!("Wand"), 0x56c),
        (tr_noop!("Petrification"), 0x554),
        (tr_noop!("Polymorph"), 0x55c),
        (tr_noop!("Spell"), 0x564),
        (tr_noop!("Breath"), 0x548),
    ] {
        f.push(ra(check!(RACIAL, label, off).group(saves)));
    }
    f.extend([
        ra(int!(
            RACIAL,
            tr_noop!("Bonus (zero uses the chart bonuses)"),
            -100,
            100,
            0x534
        )
        .group(saves)),
        ra(check!(
            RACIAL,
            tr_noop!("Give this Ability it's own value (As a percent)"),
            0x544
        )
        .group(tr_noop!("Creating a new value"))),
        ra(int!(RACIAL, tr_noop!("Starting Value"), -1000, 1000, 0x524)
            .group(tr_noop!("Creating a new value"))),
        ra(
            int!(RACIAL, tr_noop!("Increase Per Level"), -1000, 1000, 0x528)
                .group(tr_noop!("Creating a new value")),
        ),
    ]);

    // IDD 182 Magic Items.
    let mi = |d: FieldDesc<PartRec>| d.visible(on_magic_items);
    f.push(mi(at!(
        MAGIC_ITEMS,
        tr_noop!("Category for this item"),
        FieldKind::Choice(Options::Fixed(&[
            tr_noop!("None of the following"),
            tr_noop!("Potions and Oils"),
            tr_noop!("Scrolls"),
            tr_noop!("Rings"),
            tr_noop!("Rods"),
            tr_noop!("Staves"),
            tr_noop!("Wands"),
            tr_noop!("Miscellaneous Magic: Books and Tomes"),
            tr_noop!("Miscellaneous Magic: Jewels and Jewelry"),
            tr_noop!("Miscellaneous Magic: Cloaks and Robes"),
            tr_noop!("Miscellaneous Magic: Boots and Gloves"),
            tr_noop!("Miscellaneous Magic: Girdles and Helms"),
            tr_noop!("Miscellaneous Magic: Bags and Bottles"),
            tr_noop!("Miscellaneous Magic: Dusts and Stones"),
            tr_noop!("Miscellaneous Magic: Household Items and Tools"),
            tr_noop!("Miscellaneous Magic: Musical Instruments"),
            tr_noop!("Miscellaneous Magic: The Weird Stuff"),
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
        f.push(mi(
            check!(MAGIC_ITEMS, label, off).group(tr_noop!("Functions As..."))
        ));
    }

    // IDD 183 Thieving Skill adjustments.
    for (label, off) in [
        (tr_noop!("Pick Pockets"), 0x59c),
        (tr_noop!("Open Locks"), 0x5a0),
        (tr_noop!("Find/Remove Traps"), 0x5a4),
        (tr_noop!("Move Silently"), 0x5a8),
        (tr_noop!("Hide in Shadows"), 0x5ac),
        (tr_noop!("Detect Noise"), 0x5b0),
        (tr_noop!("Climb Walls"), 0x5b4),
        (tr_noop!("Read Languages"), 0x5b8),
        (tr_noop!("Detect Magic"), 0x5bc),
        (tr_noop!("Detect Illusion"), 0x5c0),
        (tr_noop!("Bribe"), 0x5c4),
        (tr_noop!("Tunneling"), 0x5c8),
        (tr_noop!("Escape Bonds"), 0x5cc),
    ] {
        f.push(
            int!(SKILLS, label, -250, 250, off)
                .group(tr_noop!("Thieving Skill Racial Adjustments (percent)"))
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
            tr_noop!("Only applies for selected race"),
            FieldKind::Combo {
                options: Options::Source("main-races"),
                free: true
            },
            0x1e0,
            opened_general
        )),
        g(FieldDesc::new(
            GENERAL,
            tr_noop!("Player's Handbook"),
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
        .group(tr_noop!("Weight (pounds)"))),
        g(FieldDesc::new(
            GENERAL,
            tr_noop!("Skills & Powers"),
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
        .group(tr_noop!("Weight (pounds)"))),
        g(FieldDesc::new(
            GENERAL,
            tr_noop!("Bulk Points"),
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
            tr_noop!("Pages for spells"),
            0,
            1_000_000,
            0x7e4,
            opened_general
        )
        .enabled(|r| !of(r, &[0, 1, 13, 16]))),
        g(at!(
            GENERAL,
            tr_noop!("Equipment Group"),
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
            tr_noop!("Demihuman Equipment"),
            FieldKind::Combo {
                options: Options::Source("racial-equipment"),
                free: true
            },
            0x244,
            opened_general
        )),
    ]);

    // IDD 364 Traits.
    f.push(
        check!(
            TRAITS,
            tr_noop!("Allow Character access to Wild Talents"),
            0x744
        )
        .visible(on_traits),
    );
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
            tr_noop!("Character Points gained (Intended for use with Disadvantages)"),
            -1000,
            1000,
            0x220
        )),
        ca(int!(
            P,
            tr_noop!("Character Point Cost"),
            -1000,
            1000,
            0x308
        )),
        ca(int!(
            P,
            tr_noop!("Turn Undead starting at level (0=never)"),
            0,
            100,
            0x778
        )),
    ];
    for (label, off) in [
        (tr_noop!("Multiple Specialization"), 0x783),
        (tr_noop!("Cleric Weapon Allowance"), 0x7a0),
        (tr_noop!("Melee Weapons Only"), 0x7c0),
        (tr_noop!("Cleric Weapons Only"), 0x7c4),
        (tr_noop!("Thief Weapons Only"), 0x7c8),
        (
            tr_noop!("Defense Bonus (+2 to AC if unarmored/unencumbered)"),
            0x780,
        ),
        (tr_noop!("Armor Use Lesser"), 0x824),
        (tr_noop!("Armor Use Major"), 0x828),
        (tr_noop!("Weapon Use Lesser"), 0x81c),
        (tr_noop!("Weapon Use Major"), 0x820),
    ] {
        f.push(ca(check!(P, label, off)));
    }
    f.extend([
        ca(at!(
            P,
            tr_noop!("Free NWP"),
            FieldKind::Combo {
                options: Options::Source("Non-Weapon Proficiencies"),
                free: true
            },
            0x718
        )),
        ca(check!(P, tr_noop!("Apply Ranger Bonus"), 0x71c)),
        ca(FieldDesc::new(
            P,
            tr_noop!("Value"),
            FieldKind::Choice(Options::Fixed(&[
                tr_noop!("No score"),
                tr_noop!("Dexterity / Level"),
                tr_noop!("Base and increment"),
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
        .group(tr_noop!("Creating a new value"))),
    ]);
    let based = |r: &PartRec| int(r, 0x77c) == 5;
    f.extend([
        ca(int!(P, tr_noop!("Starting Value"), -20, 100, 0x524)
            .group(tr_noop!("Creating a new value"))
            .enabled(based)),
        ca(int!(P, tr_noop!("Increase Per Level"), -20, 100, 0x528)
            .group(tr_noop!("Creating a new value"))
            .enabled(based)),
        ca(int!(P, tr_noop!("Maximum"), 0, 1000, 0x52c)
            .group(tr_noop!("Creating a new value"))
            .enabled(based)),
    ]);
    for (label, off) in [
        (tr_noop!("Psychic Adept"), 0x785),
        (tr_noop!("PSP Bonus"), 0x7bc),
        (tr_noop!("Contact (5)"), 0x794),
        (tr_noop!("Contact (10)"), 0x788),
        (tr_noop!("Mental Defense (5)"), 0x790),
        (tr_noop!("Mental Defense (10)"), 0x78c),
    ] {
        f.push(ca(check!(P, label, off).group(tr_noop!("Psionicist"))));
    }
    for (label, off) in [
        (tr_noop!("Gains benefits of high Con as a warrior"), 0x7b0),
        (tr_noop!("Allowed Exceptional Strength"), 0x7b4),
        (tr_noop!("Specialization"), 0x782),
        (tr_noop!("Access to improved spell tables"), 0x784),
        (tr_noop!("Increase hit dice (+2 to number of sides)"), 0x781),
    ] {
        f.push(ca(check!(P, label, off).group(tr_noop!("General"))));
    }
    for (label, off) in [
        (tr_noop!("Specialist Mage Bonus Spells"), 0x7ac),
        (tr_noop!("Specialist Mage Learning Bonus"), 0x7a8),
        (tr_noop!("Specialist Mage Learning Penalty"), 0x7a4),
    ] {
        f.push(ca(check!(P, label, off).group(tr_noop!("Specialist Mage"))));
    }
    f
}

/// The Description button of IDD 155 and IDD 176.
pub(crate) fn description() -> FieldDesc<PartRec> {
    at!(
        DESCRIPTION,
        tr_noop!("Description"),
        FieldKind::MultiLine,
        0x4
    )
}
