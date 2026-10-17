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

use super::{FieldDesc, Flavor, NameError, NewContext, Origin, UserKind, Value, default_reason};
use crate::{Error, Installation, Result};

mod kit;
mod pages;

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
pub const CLASS_ABILITIES: usize = 14;
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

/// Every part, standard (`Database/Parts.dat`) then user, in file order.
fn every_part(install: &Installation) -> Result<Vec<PartRec>> {
    let mut records = Parts::standard(install)?;
    if let Some(path) = install.path(Parts::FILE) {
        let file = PartsFile::parse(&install.read(Parts::FILE)?)
            .map_err(|source| Error::Format { path, source })?;
        records.extend(records_of(file));
    }
    Ok(records)
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.retain(|n| !n.is_empty());
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup();
    names
}

/// The weapon proficiencies: the files hold none (type 3 is empty), the
/// program makes them from the weapons, by the weapon's own name when
/// "Include this weapon name as a weapon proficiency" (+0x424) is set and
/// by "Proficiency needed" (+0x428) otherwise.
pub fn weapon_proficiencies(install: &Installation) -> Result<Vec<String>> {
    Ok(sorted(
        every_part(install)?
            .into_iter()
            .filter(|r| r.part_type == WEAPONS)
            .map(|r| {
                if r.part.data.unknown_0x420[1] != 0 {
                    r.part.head.name
                } else {
                    r.part.data.unknown_0x428
                }
            })
            .collect(),
    ))
}

/// The rogue skills: Class Abilities whose score is "rogue skill"
/// (+0x77c == 6), what the kit's Rogue Skills dialog lists.
pub fn rogue_skills(install: &Installation) -> Result<Vec<String>> {
    Ok(sorted(
        every_part(install)?
            .into_iter()
            .filter(|r| r.part_type == CLASS_ABILITIES && r.part.data.unknown_0x778[1] == 6)
            .map(|r| r.part.head.name)
            .collect(),
    ))
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
    /// Also what IDD 155's OnInitDialog forces: armor is wearable and
    /// magical items are magic.
    fn prepare(r: &mut PartRec) {
        match r.part_type {
            ARMOR => r.part.head.unknown_0x250[0] = 1,
            MAGICAL_ITEMS => r.part.head.unknown_0x224 = 1,
            _ => {}
        }
        if r.part.data.unknown_0x428.is_empty() {
            r.part.data.unknown_0x428.clone_from(&r.part.head.name);
        }
    }

    fn fields(flavor: Flavor) -> Vec<FieldDesc<PartRec>> {
        let mut f = pages::fields(flavor);
        // Kits (IDD 194 and the dialogs its buttons open).
        if flavor == Flavor::Expansion {
            f.extend(kit::fields());
        }
        f.push(pages::description());
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
    use crate::userdb::{FieldKind, Sources, UserDb};

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
        assert!(kit::has_kit(&kit));
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
    fn every_number_and_check_reaches_its_member() {
        let sources = Sources::default();
        for flavor in [Flavor::Expansion, Flavor::Core20] {
            for field in Parts::fields(flavor) {
                let mut r = Parts::new_record(&context(flavor, KITS));
                let probe = match field.kind {
                    FieldKind::Int { min, max } => {
                        Value::Int(if min <= 2 && 2 <= max { 2 } else { min })
                    }
                    FieldKind::Check => Value::Int(1),
                    FieldKind::Text { .. } | FieldKind::MultiLine => Value::Text("X".into()),
                    _ => continue,
                };
                field.set(&mut r, &probe, &sources);
                let got = field.get(&r);
                // Core Rules 2.0 parts lack the members of the version
                // block; their fields read 0.
                if flavor == Flavor::Core20 && got.int() == 0 && got.text().is_empty() {
                    continue;
                }
                match probe {
                    Value::Text(_) => assert_eq!(got.text(), probe.text(), "{}", field.label),
                    _ => assert_eq!(got.int(), probe.int(), "{} {:#x}", field.label, field.arg),
                }
            }
        }
    }

    #[test]
    fn types_offered() {
        let t3 = editable_types(Flavor::Expansion);
        assert_eq!(t3.len(), 17);
        assert_eq!(t3[0], (ARMOR, "Armor"));
        assert_eq!(editable_types(Flavor::Core20).len(), 16);
    }
}
