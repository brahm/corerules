//! Custom Data (`UserDbas/PartsU.dat`): IDD 139 "Parts", the generic part
//! editor IDD 155 and its property pages.
//!
//! The file holds one array per part type ([`PART_TYPES`]); a name must
//! be unique within its type. The Expansion writes each part with its
//! "TSRP3 V7" version block, Core Rules 2.0 without it (so the fields that
//! block switches on are absent).
//!
//! The pages ported here are the ones weapons, armor, equipment, magical
//! items and kits use: Main, Weapons, Proficiencies, Costs, Adjustments,
//! Saving Throws, Items Worn, Container, Restrict Class, Description and,
//! for Expansion kits, Kit (availability and starting money). Every other
//! member keeps the value the record had (a copy keeps its source's).

use cr_formats::parts::{
    KIT_CURRENT_VERSION, KIT_HEADER, PART_TYPES, Part, PartData, PartHead, PartKit, PartKitData,
    PartVersion, PartsFile,
};
use cr_formats::{ArchiveReader, Field, Object};

use super::{
    FieldDesc, FieldKind, Flavor, NameError, NewContext, Options, Origin, UserKind, Value,
    default_reason,
};
use crate::{Error, Installation, Result};

/// One part and its type (the index of its array).
#[derive(Debug, Clone, PartialEq)]
pub struct PartRec {
    pub part_type: usize,
    pub part: Part,
}

pub const WEAPONS: usize = 0;
pub const ARMOR: usize = 1;
pub const EQUIPMENT: usize = 7;
pub const MAGICAL_ITEMS: usize = 8;
pub const KITS: usize = 15;

/// The part types the Custom Data list offers ("Select the Type"),
/// alphabetically, with their array index. Core Rules 2.0 has no Class
/// Abilities type.
pub fn editable_types(flavor: Flavor) -> Vec<(usize, &'static str)> {
    let mut types: Vec<(usize, &'static str)> = PART_TYPES
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.filter(|t| *t != "NOT").map(|t| (i, t)))
        .filter(|(i, _)| flavor == Flavor::Expansion || *i != 14)
        .collect();
    types.sort_by_key(|(_, t)| *t);
    types
}

/// The array index of a part type name ("Weapons" is 0).
pub fn source_type(name: &str) -> Option<usize> {
    PART_TYPES.iter().position(|t| *t == Some(name))
}

pub fn type_name(part_type: usize) -> &'static str {
    PART_TYPES
        .get(part_type)
        .copied()
        .flatten()
        .unwrap_or("Parts")
}

/// The names of every part, standard and user, per type, sorted.
pub fn all_names(install: &Installation) -> Result<Vec<Vec<String>>> {
    Ok(names_by_origin(install)?
        .into_iter()
        .map(|names| {
            let mut names: Vec<String> = names.into_iter().map(|(n, _)| n).collect();
            names.sort_by_key(|n| n.to_lowercase());
            names.dedup();
            names
        })
        .collect())
}

/// The names of every part with where it comes from, per type, in file
/// order (standard first).
pub fn names_by_origin(install: &Installation) -> Result<Vec<Vec<(String, Origin)>>> {
    let mut names = vec![Vec::new(); PART_TYPES.len()];
    for (origin, relative) in [
        (Origin::Standard, "Database/Parts.dat"),
        (Origin::Custom, Parts::FILE),
    ] {
        let Some(path) = install.path(relative) else {
            continue;
        };
        let file = PartsFile::parse(&install.read(relative)?)
            .map_err(|source| Error::Format { path, source })?;
        for (t, array) in file.arrays.iter().enumerate() {
            for object in array {
                if let Object::Part(p) = object
                    && !p.head.name.is_empty()
                {
                    names[t].push((p.head.name.clone(), origin));
                }
            }
        }
    }
    Ok(names)
}

/// The weapon groups the standard weapons belong to ("Axes: Axes").
pub fn weapon_groups(install: &Installation) -> Result<Vec<String>> {
    let mut groups: Vec<String> = Parts::standard(install)?
        .into_iter()
        .filter(|r| r.part_type == WEAPONS)
        .flat_map(|r| r.part.data.unknown_0x1cc)
        .collect();
    groups.sort();
    groups.dedup();
    Ok(groups)
}

pub struct Parts;

/// Zero bytes to read default records from: empty strings and lists, 0
/// integers, null objects.
const ZEROS: [u8; 8192] = [0; 8192];

/// The version block the Expansion writes.
pub const VERSION_MARKER: i32 = -8889;
pub const VERSION_STRING: &str = "TSRP3 V7";

/// A new kit (`CPartKitOb`) as the Expansion's constructor leaves it.
pub fn new_kit() -> PartKit {
    let v = KIT_CURRENT_VERSION as i32;
    let mut d = PartKitData::read_fields(&mut ArchiveReader::new(&ZEROS), v)
        .expect("zero bytes read as an empty kit");
    d.flags_0x4 = [1; 11];
    d.flag_0x4b4 = Some(1);
    // The six "25"s are the ability maxima of the requirements page.
    for i in 0..6 {
        d.unknown_0x4c[1 + 6 * i] = 25;
    }
    d.unknown_0x88 = [0, 0, 100, 0, i32::MIN];
    d.flags_0x73 = Some([1, 1]);
    d.unknown_0x120 = Some([-1, -1]);
    d.unknown_0x18c = Some([101, 10, 0, -10, 0]);
    d.unknown_0x1f4 = Some([10, 0]);
    d.unknown_0x200 = Some([50, 50]);
    d.unknown_0x318 = Some(-1);
    d.unknown_0x450 = Some([99, -1, -1, -1, -1]);
    d.unknown_0x51c = Some(101);
    d.unknown_0x564 = Some(205);
    d.unknown_0x5e0 = Some([-1, -1, 0, 0, 0, 0, 0]);
    PartKit {
        header: KIT_HEADER.into(),
        version: KIT_CURRENT_VERSION,
        data: Some(d),
    }
}

/// A new part of `part_type` as each program's constructor leaves it
/// (goldens `PartsU_01_*`).
pub fn new_part(flavor: Flavor, part_type: usize) -> Part {
    let mut head =
        PartHead::read(&mut ArchiveReader::new(&ZEROS)).expect("zero bytes read as a part");
    head.unknown_0x30c[0] = 1;
    head.unknown_0x2a8 = -1;
    // Armor class: 100 is none.
    head.unknown_0x250[8] = 100;
    head.unknown_0x278 = (-1.0f32).to_bits() as i32;
    head.unknown_0x27c = [-1, 0, -1, 0, 0, 0, 0, -1, -1, -1, -1];
    let expansion = flavor == Flavor::Expansion;
    let version = expansion.then(|| PartVersion {
        marker: VERSION_MARKER,
        string: VERSION_STRING.into(),
    });
    let v = if expansion {
        cr_formats::parts::CURRENT_VERSION
    } else {
        0
    };
    let mut d = PartData::read_fields(&mut ArchiveReader::new(&ZEROS), v)
        .expect("zero bytes read as part data");
    d.unknown_0x420 = [0, 1];
    d.unknown_0x42c = 1;
    if expansion {
        d.unknown_0x52c = Some(100);
    } else {
        d.unknown_0x788 = 1;
        d.unknown_0x78c = 1;
        d.unknown_0x790 = 1;
        d.unknown_0x794 = 1;
    }
    d.unknown_0x79c = 1;
    d.unknown_0x7cc[0] = 14;
    if part_type == KITS {
        d.unknown_0x5e4 = [-1; 24];
        d.unknown_0x644 = -1;
        if expansion {
            d.kit = Some(Some(Box::new(Object::PartKit(Box::new(new_kit())))));
        }
    } else {
        d.unknown_0x660 = i32::MIN;
        d.unknown_0x65c = 100;
    }
    Part {
        head,
        version,
        unknown_0x728: 0,
        data: d,
    }
}

fn kit(r: &PartRec) -> Option<&PartKitData> {
    match r.part.data.kit.as_ref()?.as_ref()?.as_ref() {
        Object::PartKit(k) => k.data.as_ref(),
        _ => None,
    }
}

fn kit_mut(r: &mut PartRec) -> Option<&mut PartKitData> {
    match r.part.data.kit.as_mut()?.as_mut()?.as_mut() {
        Object::PartKit(k) => k.data.as_mut(),
        _ => None,
    }
}

fn has_kit(r: &PartRec) -> bool {
    kit(r).is_some()
}

fn is_type(r: &PartRec, types: &[usize]) -> bool {
    types.contains(&r.part_type)
}

fn weapon(r: &PartRec) -> bool {
    r.part_type == WEAPONS
}

fn magic(r: &PartRec) -> bool {
    r.part.head.unknown_0x224 != 0
}

/// Shorthand for a descriptor on `part.head` / `part.data` members.
macro_rules! int {
    ($page:expr, $label:expr, $min:expr, $max:expr, $($f:tt)+) => {
        FieldDesc::new(
            $page,
            $label,
            FieldKind::Int { min: $min, max: $max },
            |r: &PartRec, _| Value::Int(r.part.$($f)+ as i64),
            |r, _, v, _| r.part.$($f)+ = v.int() as _,
        )
    };
}

macro_rules! check {
    ($page:expr, $label:expr, $($f:tt)+) => {
        FieldDesc::new(
            $page,
            $label,
            FieldKind::Check,
            |r: &PartRec, _| Value::Bool(r.part.$($f)+ != 0),
            |r, _, v, _| r.part.$($f)+ = v.bool() as _,
        )
    };
}

macro_rules! text {
    ($page:expr, $label:expr, $kind:expr, $($f:tt)+) => {
        FieldDesc::new(
            $page,
            $label,
            $kind,
            |r: &PartRec, _| Value::Text(r.part.$($f)+.clone()),
            |r, _, v, _| r.part.$($f)+ = v.text(),
        )
    };
}

/// An optional byte flag (present in the Expansion's layout only).
macro_rules! flag {
    ($page:expr, $label:expr, $($f:tt)+) => {
        FieldDesc::new(
            $page,
            $label,
            FieldKind::Check,
            |r: &PartRec, _| Value::Bool(r.part.$($f)+.unwrap_or(0) != 0),
            |r, _, v, _| {
                if let Some(flag) = r.part.$($f)+.as_mut() {
                    *flag = u8::from(v.bool());
                }
            },
        )
    };
}

/// A member of the kit.
macro_rules! kit_field {
    ($label:expr, $kind:expr, |$k:ident| $get:expr, |$km:ident, $v:ident| $set:expr) => {
        FieldDesc::new(
            "Kit",
            $label,
            $kind,
            |r: &PartRec, _| match kit(r) {
                Some($k) => $get,
                None => Value::Int(0),
            },
            |r, _, $v, _| {
                if let Some($km) = kit_mut(r) {
                    $set;
                }
            },
        )
        .visible(has_kit)
    };
}

const SAVES: [&str; 10] = [
    "Breath Weapon",
    "Death Magic",
    "Paralyzation",
    "Petrification",
    "Poison",
    "Polymorph",
    "Rod",
    "Spell",
    "Staff",
    "Wand",
];

/// Costs: Skills & Powers pp/gp/sp/cp (`unknown_0x1f8[0..4]`), then the
/// Player's Handbook's (`[4..8]`; PHB gold is +0x20c).
const COINS: [&str; 4] = ["Platinum", "Gold", "Silver", "Copper"];

/// Items Worn: where the item goes (+0x254 to +0x26c).
const WORN: [(&str, usize); 7] = [
    ("As a Robe", 1),
    ("As Shoes or Boots", 2),
    ("As a Hat or Helmet", 3),
    ("As a Scabbard", 4),
    ("As a Quiver", 5),
    ("As a Knife Sheath", 6),
    ("As Gloves", 7),
];

/// Can Carry in Containers (+0x2b8 to +0x2d4).
const CARRY: [&str; 8] = [
    "Bolt Case",
    "Boots or Shoes",
    "Knife Sheath",
    "A Hat",
    "Quiver",
    "Scabbard",
    "Javelin Case",
    "Staff Case",
];

impl UserKind for Parts {
    type Rec = PartRec;
    const NOUN: &'static str = "Part";
    const FILE: &'static str = "UserDbas/PartsU.dat";

    fn name(r: &PartRec) -> &str {
        &r.part.head.name
    }

    fn set_name(r: &mut PartRec, name: &str) {
        name.clone_into(&mut r.part.head.name);
    }

    fn scope(r: &PartRec) -> usize {
        r.part_type
    }

    fn standard(install: &Installation) -> Result<Vec<PartRec>> {
        let relative = "Database/Parts.dat";
        let Some(path) = install.path(relative) else {
            return Ok(Vec::new());
        };
        let file = PartsFile::parse(&install.read(relative)?)
            .map_err(|source| Error::Format { path, source })?;
        Ok(records_of(file))
    }

    fn parse_user(data: &[u8]) -> std::result::Result<Vec<PartRec>, cr_formats::Error> {
        PartsFile::parse(data).map(records_of)
    }

    fn user_bytes(records: &[PartRec]) -> std::result::Result<Vec<u8>, cr_formats::Error> {
        let mut arrays: [Vec<Object>; PART_TYPES.len()] = Default::default();
        for r in records {
            arrays[r.part_type].push(Object::Part(Box::new(r.part.clone())));
        }
        PartsFile { arrays }.to_bytes()
    }

    fn new_record(context: &NewContext) -> PartRec {
        PartRec {
            part_type: context.scope,
            part: new_part(context.flavor, context.scope),
        }
    }

    /// The weapon proficiency name defaults to the part's name (the
    /// Proficiencies page fills it in, and OK stores it).
    fn prepare(r: &mut PartRec) {
        if r.part.data.unknown_0x428.is_empty() {
            r.part.data.unknown_0x428.clone_from(&r.part.head.name);
        }
    }

    fn fields(flavor: Flavor) -> Vec<FieldDesc<PartRec>> {
        const MAIN: &str = "Part";
        const WEAPON: &str = "Weapon";
        const PROFS: &str = "Proficiencies";
        const COSTS: &str = "Costs";
        const ADJUST: &str = "Adjustments";
        const SAVE: &str = "Saving Throws";
        const WORN_PAGE: &str = "Items Worn";
        const CONTAINER: &str = "Container";
        const RESTRICT: &str = "Restrict Class";
        const DESCRIPTION: &str = "Description";
        let expansion = flavor == Flavor::Expansion;
        let mut f = vec![
            text!(MAIN, "Name", FieldKind::Text { max: 50 }, head.name),
            check!(MAIN, "Magic", head.unknown_0x224),
            int!(MAIN, "XP Value", 0, i32::MAX as i64, head.unknown_0x230).enabled(magic),
            check!(
                MAIN,
                "This is something a character could wear (jewelry not included)",
                head.unknown_0x250[0]
            )
            .enabled(|r| !is_type(r, &[WEAPONS, KITS])),
            check!(
                MAIN,
                "This can be used to carry other items",
                head.unknown_0x250[9]
            )
            .enabled(|r| !is_type(r, &[WEAPONS, ARMOR, KITS])),
            check!(
                MAIN,
                "This can be carried in a container",
                head.unknown_0x2b4[0]
            )
            .enabled(|r| r.part_type != KITS),
            check!(MAIN, "Thieving skills are adjusted", data.unknown_0x598)
                .enabled(|r| is_type(r, &[ARMOR, MAGICAL_ITEMS])),
        ];
        let metal = check!(MAIN, "This item contains metal", data.unknown_0x228);
        let silver = check!(MAIN, "This item contains silver", data.unknown_0x22c);
        if expansion {
            f.push(metal.enabled(|r| r.part_type != KITS));
            f.push(silver.enabled(|r| r.part_type != KITS));
        } else {
            // Core Rules 2.0 disables them for weapons.
            f.push(metal.enabled(|r| !is_type(r, &[WEAPONS, KITS])));
            f.push(silver.enabled(|r| !is_type(r, &[WEAPONS, KITS])));
        }

        // Weapons (IDD 164).
        let w = |d: FieldDesc<PartRec>| d.visible(weapon);
        let books = [("Player's Handbook", 0usize), ("Skills & Powers", 1usize)];
        for (book, b) in books {
            f.push(w(FieldDesc::new(
                WEAPON,
                "Damage, Small-Medium",
                FieldKind::Dice,
                |r: &PartRec, b| {
                    let h = &r.part.head;
                    Value::Dice(if b == 0 {
                        [
                            h.unknown_0x3fc.into(),
                            h.unknown_0x3f8.into(),
                            h.unknown_0x400.into(),
                        ]
                    } else {
                        [
                            h.unknown_0x3b8[3].into(),
                            h.unknown_0x3c8.into(),
                            h.unknown_0x3cc.into(),
                        ]
                    })
                },
                |r, b, v, _| {
                    let [n, s, m] = v.dice().map(|x| x as i32);
                    let h = &mut r.part.head;
                    if b == 0 {
                        (h.unknown_0x3fc, h.unknown_0x3f8, h.unknown_0x400) = (n, s, m);
                    } else {
                        (h.unknown_0x3b8[3], h.unknown_0x3c8, h.unknown_0x3cc) = (n, s, m);
                    }
                },
            )
            .arg(b)
            .group(book)));
            f.push(w(FieldDesc::new(
                WEAPON,
                "Damage, Large",
                FieldKind::Dice,
                |r: &PartRec, b| {
                    let h = &r.part.head;
                    Value::Dice(if b == 0 {
                        [
                            h.unknown_0x3e8.into(),
                            h.unknown_0x3ec.into(),
                            h.unknown_0x3f0.into(),
                        ]
                    } else {
                        [
                            h.unknown_0x3d8.into(),
                            h.unknown_0x3d4.into(),
                            h.unknown_0x3dc.into(),
                        ]
                    })
                },
                |r, b, v, _| {
                    let [n, s, m] = v.dice().map(|x| x as i32);
                    let h = &mut r.part.head;
                    if b == 0 {
                        (h.unknown_0x3e8, h.unknown_0x3ec, h.unknown_0x3f0) = (n, s, m);
                    } else {
                        (h.unknown_0x3d8, h.unknown_0x3d4, h.unknown_0x3dc) = (n, s, m);
                    }
                },
            )
            .arg(b)
            .group(book)));
            f.push(w(FieldDesc::new(
                WEAPON,
                "Speed Factor",
                FieldKind::Int {
                    min: -100,
                    max: 1000,
                },
                |r: &PartRec, b| {
                    let h = &r.part.head;
                    Value::Int(
                        if b == 0 {
                            h.unknown_0x404
                        } else {
                            h.unknown_0x3e0
                        }
                        .into(),
                    )
                },
                |r, b, v, _| {
                    let h = &mut r.part.head;
                    *if b == 0 {
                        &mut h.unknown_0x404
                    } else {
                        &mut h.unknown_0x3e0
                    } = v.int() as i32;
                },
            )
            .arg(b)
            .group(book)));
            for (label, which) in [("Weapon Size", 0usize), ("Weapon Type", 1), ("ROF", 2)] {
                f.push(w(FieldDesc::new(
                    WEAPON,
                    label,
                    FieldKind::Combo {
                        options: Options::Fixed(match which {
                            0 => &["", "T", "S", "M", "L", "H", "G"],
                            1 => &["", "B", "P", "S", "P/B", "P/S", "S/B", "S/P"],
                            _ => &["", "1", "2", "3/2", "2/1", "1/2", "3/1", "4/1", "5/1"],
                        }),
                        free: true,
                    },
                    |r: &PartRec, arg| Value::Text(weapon_text(r, arg).clone()),
                    |r, arg, v, _| *weapon_text_mut(r, arg) = v.text(),
                )
                .arg(b * 3 + which)
                .group(book)));
            }
        }
        f.extend([
            w(int!(WEAPON, "Short", 0, 10000, head.unknown_0x40c[2]).group("Range")),
            w(int!(WEAPON, "Medium", 0, 10000, head.unknown_0x40c[3]).group("Range")),
            w(int!(WEAPON, "Long", 0, 10000, head.unknown_0x40c[1]).group("Range")),
            w(int!(WEAPON, "Knockdown (d)", 0, 100, head.unknown_0x40c[0]).group("Combat")),
            w(int!(WEAPON, "# of Hands Needed", 0, 2, head.unknown_0x24c).group("Combat")),
            w(int!(
                WEAPON,
                "Max Damage Bonus from STR",
                -100,
                100,
                data.unknown_0x7cc[0]
            )
            .group("Combat")),
            w(check!(WEAPON, "Do Not Apply Low STR Penalty", data.unknown_0x7b8).group("Combat")),
            w(check!(WEAPON, "Melee Weapon", head.unknown_0x3b8[0]).group("Use")),
            w(check!(WEAPON, "Missile Weapon", head.unknown_0x3b8[1]).group("Use")),
            w(check!(WEAPON, "Needs Ammo", head.unknown_0x3b8[2])
                .group("Use")
                .enabled(|r| r.part.head.unknown_0x3b8[1] != 0)),
            w(check!(WEAPON, "Ammunition", data.unknown_0x420[0]).group("Use")),
        ]);
        if expansion {
            f.push(w(
                flag!(WEAPON, "Thief Weapon", data.flag_0x448).group("Use")
            ));
            f.push(w(flag!(
                WEAPON,
                "Applies to Cavalier Horseman's Bonus (melee weapons only)",
                data.flag_0x724
            )
            .group("Use")
            .enabled(|r| r.part.head.unknown_0x3b8[0] != 0)));
        }
        // Weapon Proficiencies (IDD 413).
        f.extend([
            w(check!(
                PROFS,
                "Include This Weapon Name as a Weapon Proficiency",
                data.unknown_0x420[1]
            )),
            w(text!(
                PROFS,
                "Proficiency Needed",
                FieldKind::Combo {
                    options: Options::Source("Weapons"),
                    free: true
                },
                data.unknown_0x428
            )
            .enabled(|r| r.part.data.unknown_0x420[1] == 0)),
            w(FieldDesc::new(
                PROFS,
                "Weapon Groups",
                FieldKind::Dual(Options::Source("weapon-groups")),
                |r: &PartRec, _| Value::List(r.part.data.unknown_0x1cc.clone()),
                |r, _, v, _| r.part.data.unknown_0x1cc = v.list(),
            )),
        ]);

        // Costs (IDD 163).
        let costs = |r: &PartRec| is_type(r, &[WEAPONS, ARMOR, EQUIPMENT, MAGICAL_ITEMS, 13]);
        for (book, base) in [("Player's Handbook", 4usize), ("Skills & Powers", 0)] {
            for (i, coin) in COINS.iter().enumerate() {
                f.push(
                    FieldDesc::new(
                        COSTS,
                        coin,
                        FieldKind::Int {
                            min: 0,
                            max: 1_000_000_000,
                        },
                        |r: &PartRec, i| Value::Int(r.part.head.unknown_0x1f8[i].into()),
                        |r, i, v, _| r.part.head.unknown_0x1f8[i] = v.int() as i32,
                    )
                    .arg(base + i)
                    .group(book)
                    .visible(costs),
                );
            }
        }

        // Adjustments (IDD 166) and Saving Throw Adjustments (IDD 167).
        let adjusted = |r: &PartRec| is_type(r, &[WEAPONS, ARMOR, MAGICAL_ITEMS]);
        f.extend([
            int!(ADJUST, "Armor Class", -100, 100, head.unknown_0x44c[0]).visible(adjusted),
            int!(ADJUST, "Attack", -100, 100, head.unknown_0x44c[1]).visible(adjusted),
            int!(ADJUST, "Damage", -100, 100, head.unknown_0x44c[2]).visible(adjusted),
        ]);
        for (i, save) in SAVES.iter().enumerate() {
            f.push(
                FieldDesc::new(
                    SAVE,
                    save,
                    FieldKind::Int {
                        min: -100,
                        max: 100,
                    },
                    |r: &PartRec, i| Value::Int(r.part.head.unknown_0x2d8[i].into()),
                    |r, i, v, _| r.part.head.unknown_0x2d8[i] = v.int() as i32,
                )
                .arg(i)
                .visible(|r| is_type(r, &[WEAPONS, ARMOR, MAGICAL_ITEMS, KITS])),
            );
        }

        // Items Worn (IDD 157) for armor and wearable items.
        let worn = |r: &PartRec| is_type(r, &[ARMOR, EQUIPMENT, MAGICAL_ITEMS]);
        f.push(
            int!(
                WORN_PAGE,
                "Armor Class (100: none)",
                -100,
                100,
                head.unknown_0x250[8]
            )
            .visible(worn),
        );
        for (label, i) in WORN {
            f.push(
                FieldDesc::new(
                    WORN_PAGE,
                    label,
                    FieldKind::Check,
                    |r: &PartRec, i| Value::Bool(r.part.head.unknown_0x250[i] != 0),
                    |r, i, v, _| r.part.head.unknown_0x250[i] = v.bool().into(),
                )
                .arg(i)
                .group("Worn")
                .visible(worn)
                .enabled(|r| r.part.head.unknown_0x250[0] != 0),
            );
        }
        f.extend([
            check!(
                WORN_PAGE,
                "Include This Armor Name as an Armor Proficiency",
                data.unknown_0x42c
            )
            .group("Proficiency")
            .visible(|r| r.part_type == ARMOR),
            text!(
                WORN_PAGE,
                "Armor Proficiency",
                FieldKind::Combo {
                    options: Options::Source("Armor"),
                    free: true
                },
                data.unknown_0x430
            )
            .group("Proficiency")
            .visible(|r| r.part_type == ARMOR)
            .enabled(|r| r.part.data.unknown_0x42c == 0),
        ]);

        // Can Carry in Containers (IDD 161). The Container page (IDD 160)
        // is not ported: its members are not mapped well enough yet.
        for (i, label) in CARRY.iter().enumerate() {
            f.push(
                FieldDesc::new(
                    CONTAINER,
                    label,
                    FieldKind::Check,
                    |r: &PartRec, i| Value::Bool(carry(r, i) != 0),
                    |r, i, v, _| *carry_mut(r, i) = v.bool().into(),
                )
                .arg(i)
                .group("Can Be Carried In")
                .visible(|r| r.part_type != KITS)
                .enabled(|r| r.part.head.unknown_0x2b4[0] != 0),
            );
        }

        // Class Restrictions (IDD 179); the Expansion's layout only.
        if expansion {
            f.push(
                FieldDesc::new(
                    RESTRICT,
                    "Classes",
                    FieldKind::Dual(Options::Source("classes")),
                    |r: &PartRec, _| {
                        Value::List(r.part.data.allowed_classes.clone().unwrap_or_default())
                    },
                    |r, _, v, _| {
                        if let Some(list) = r.part.data.allowed_classes.as_mut() {
                            *list = v.list();
                        }
                    },
                )
                .visible(|r| r.part.data.allowed_classes.is_some()),
            );
        }

        // Kits (IDD 194) and Kit Money (IDD 238).
        f.extend([
            kit_field!(
                "Available to Core Rules Characters",
                FieldKind::Check,
                |k| Value::Bool(k.flags_0x73.unwrap_or_default()[0] != 0),
                |k, v| {
                    if let Some(flags) = k.flags_0x73.as_mut() {
                        flags[0] = v.bool().into();
                    }
                }
            )
            .group("Availability"),
            kit_field!(
                "Available to Skills & Powers Characters",
                FieldKind::Check,
                |k| Value::Bool(k.flags_0x73.unwrap_or_default()[1] != 0),
                |k, v| {
                    if let Some(flags) = k.flags_0x73.as_mut() {
                        flags[1] = v.bool().into();
                    }
                }
            )
            .group("Availability"),
            kit_field!(
                "Assign Maximum Starting Funds Allowed for Class",
                FieldKind::Check,
                |k| Value::Bool(k.flag_0x84 != 0),
                |k, v| k.flag_0x84 = v.bool().into()
            )
            .group("Starting Money"),
            kit_field!(
                "Maximum Retained Funds During Character Generation",
                FieldKind::Check,
                |k| Value::Bool(k.flag_0x5dc.unwrap_or(0) != 0),
                |k, v| k.flag_0x5dc = k.flag_0x5dc.map(|_| v.bool().into())
            )
            .group("Starting Money"),
            kit_field!(
                "Starting Money = Dice × Multiplier + Add",
                FieldKind::Check,
                |k| Value::Bool(k.flag_0x618.unwrap_or(0) != 0),
                |k, v| k.flag_0x618 = k.flag_0x618.map(|_| v.bool().into())
            )
            .group("Starting Money"),
            kit_field!(
                "Dice",
                FieldKind::Dice,
                |k| {
                    let m = k.unknown_0x608.unwrap_or_default();
                    Value::Dice([m[0].into(), m[1].into(), m[3].into()])
                },
                |k, v| {
                    if let Some(m) = k.unknown_0x608.as_mut() {
                        let [n, s, a] = v.dice();
                        (m[0], m[1], m[3]) = (n as i32, s as i32, a as i32);
                    }
                }
            )
            .group("Starting Money")
            .enabled(|r| kit(r).and_then(|k| k.flag_0x618).unwrap_or(0) != 0),
            kit_field!(
                "Multiplier",
                FieldKind::Int {
                    min: 0,
                    max: 1_000_000
                },
                |k| Value::Int(k.unknown_0x608.unwrap_or_default()[2].into()),
                |k, v| {
                    if let Some(m) = k.unknown_0x608.as_mut() {
                        m[2] = v.int() as i32;
                    }
                }
            )
            .group("Starting Money")
            .enabled(|r| kit(r).and_then(|k| k.flag_0x618).unwrap_or(0) != 0),
        ]);

        f.push(text!(
            DESCRIPTION,
            "Description",
            FieldKind::MultiLine,
            head.description
        ));
        f
    }

    fn name_message(error: &NameError, r: &PartRec, _adding: bool) -> String {
        let reason = match error {
            NameError::Used | NameError::UsedByStandard => {
                format!("Name already used in catagory {}.", type_name(r.part_type))
            }
            e => default_reason(e),
        };
        format!("Part not added: Error. {reason}")
    }

    fn summary(r: &PartRec) -> String {
        let h = &r.part.head;
        let price = h.unknown_0x1f8[5].max(h.unknown_0x1f8[1]);
        match r.part_type {
            WEAPONS if h.unknown_0x3fc > 0 => format!(
                "{} / {}",
                super::dice_text(
                    h.unknown_0x3fc.into(),
                    h.unknown_0x3f8.into(),
                    h.unknown_0x400.into()
                ),
                super::dice_text(
                    h.unknown_0x3e8.into(),
                    h.unknown_0x3ec.into(),
                    h.unknown_0x3f0.into()
                )
            ),
            _ if price > 0 => format!("{price} gp"),
            _ if h.unknown_0x230 > 0 => format!("{} XP", h.unknown_0x230),
            _ => String::new(),
        }
    }
}

/// Weapon size, type and rate of fire of the PHB (0..3) and S&P (3..6)
/// columns.
fn weapon_text(r: &PartRec, which: usize) -> &String {
    let h = &r.part.head;
    match which {
        0 => &h.unknown_0x3f4,
        1 => &h.unknown_0x3e4,
        2 => &h.unknown_0x408,
        3 => &h.unknown_0x3d0,
        4 => &h.unknown_0x3b4,
        _ => &h.unknown_0x41c,
    }
}

fn weapon_text_mut(r: &mut PartRec, which: usize) -> &mut String {
    let h = &mut r.part.head;
    match which {
        0 => &mut h.unknown_0x3f4,
        1 => &mut h.unknown_0x3e4,
        2 => &mut h.unknown_0x408,
        3 => &mut h.unknown_0x3d0,
        4 => &mut h.unknown_0x3b4,
        _ => &mut h.unknown_0x41c,
    }
}

/// Can Carry flags in [`CARRY`] order: +0x2b8, +0x2bc, +0x2c0, +0x2c4,
/// +0x2c8, +0x2cc, +0x2d0, +0x2d4 (`unknown_0x2b4[1..9]`).
fn carry(r: &PartRec, i: usize) -> i32 {
    r.part.head.unknown_0x2b4[carry_index(i)]
}

fn carry_mut(r: &mut PartRec, i: usize) -> &mut i32 {
    &mut r.part.head.unknown_0x2b4[carry_index(i)]
}

fn carry_index(i: usize) -> usize {
    // Bolt case +0x2b8, boots +0x2bc, knife sheath +0x2c0, hat +0x2c4,
    // quiver +0x2c8, scabbard +0x2cc, javelin case +0x2d0, staff case
    // +0x2d4.
    1 + i
}

fn records_of(file: PartsFile) -> Vec<PartRec> {
    file.arrays
        .into_iter()
        .enumerate()
        .flat_map(|(t, array)| {
            array.into_iter().filter_map(move |o| match o {
                Object::Part(p) => Some(PartRec {
                    part_type: t,
                    part: *p,
                }),
                _ => None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userdb::testing::TempInstall;
    use crate::userdb::{Sources, UserDb};

    fn context(flavor: Flavor, scope: usize) -> NewContext {
        NewContext {
            flavor,
            scope,
            parent: String::new(),
        }
    }

    #[test]
    fn new_parts_follow_the_program() {
        let t3 = Parts::new_record(&context(Flavor::Expansion, WEAPONS));
        assert_eq!(t3.part.layout_version(), 7);
        assert_eq!(t3.part.data.unknown_0x52c, Some(100));
        assert_eq!(t3.part.data.kit, Some(None));
        let ev = Parts::new_record(&context(Flavor::Core20, WEAPONS));
        assert_eq!(ev.part.version, None);
        assert_eq!(ev.part.data.allowed_classes, None);
        assert_eq!(ev.part.data.unknown_0x788, 1);
        let kit = Parts::new_record(&context(Flavor::Expansion, KITS));
        assert!(has_kit(&kit));
        assert_eq!(kit.part.data.unknown_0x660, 0);
        // Both round-trip through the file.
        let bytes = Parts::user_bytes(&[t3.clone(), kit.clone()]).unwrap();
        assert_eq!(Parts::parse_user(&bytes).unwrap(), [t3, kit]);
        let bytes = Parts::user_bytes(std::slice::from_ref(&ev)).unwrap();
        assert_eq!(Parts::parse_user(&bytes).unwrap(), [ev]);
    }

    #[test]
    fn names_are_unique_per_type() {
        let mut sword = Parts::new_record(&context(Flavor::Expansion, WEAPONS));
        sword.part.head.name = "Sword".into();
        let t = TempInstall::new(
            "parts",
            &[
                (
                    "Database/Parts.dat",
                    Parts::user_bytes(std::slice::from_ref(&sword)).unwrap(),
                ),
                ("UserDbas/PartsU.dat", vec![0; 42]),
                // An Expansion installation, which has kit data.
                ("UserDbas/AllowedU.dat", vec![0, 0]),
            ],
        );
        let mut db = UserDb::<Parts>::load(&t.install).unwrap();
        let mut kit = db.new_record(KITS, "");
        kit.part.head.name = "sword".into();
        assert_eq!(db.add(kit.clone()), Ok(0));
        assert_eq!(db.user[0].part.data.unknown_0x428, "sword");
        let mut weapon = db.new_record(WEAPONS, "");
        weapon.part.head.name = "SWORD".into();
        let error = db.add(weapon.clone()).unwrap_err();
        assert_eq!(
            Parts::name_message(&error, &weapon, true),
            "Part not added: Error. Name already used in catagory Weapons."
        );
        let fields = Parts::fields(Flavor::Expansion);
        let xp = fields.iter().find(|f| f.label == "XP Value").unwrap();
        assert!(!xp.is_enabled(&weapon));
        let field = |label: &str| fields.iter().find(|f| f.label == label).unwrap();
        field("Magic").set(&mut weapon, &Value::Bool(true), &Sources::default());
        assert!(xp.is_enabled(&weapon));
        let dice = fields
            .iter()
            .find(|f| f.label == "Damage, Small-Medium" && f.arg == 0)
            .unwrap();
        dice.set(&mut weapon, &Value::Dice([1, 8, 0]), &Sources::default());
        assert_eq!(
            (
                weapon.part.head.unknown_0x3fc,
                weapon.part.head.unknown_0x3f8
            ),
            (1, 8)
        );
        assert!(!field("Multiplier").is_visible(&weapon));
        assert!(field("Multiplier").is_visible(&kit));
        db.save(&t.install).unwrap();
        assert_eq!(UserDb::<Parts>::load(&t.install).unwrap().user.len(), 1);
        assert_eq!(
            names_by_origin(&t.install).unwrap()[KITS],
            [("sword".to_owned(), Origin::Custom)]
        );
    }

    #[test]
    fn types_offered() {
        let t3 = editable_types(Flavor::Expansion);
        assert_eq!(t3.len(), 17);
        assert_eq!(t3[0], (ARMOR, "Armor"));
        assert_eq!(editable_types(Flavor::Core20).len(), 16);
    }
}
