//! Custom Data (`UserDbas/PartsU.dat`): IDD 139 "Parts", the generic part
//! editor IDD 155 and its property pages.
//!
//! The file holds one array per part type ([`PART_TYPES`]); a name must
//! be unique within its type. The Expansion writes each part with its
//! "TSRP3 V7" version block, Core Rules 2.0 without it (so the fields that
//! block switches on are absent).
//!
//! The pages ported here cover every type the list offers but weapon
//! proficiencies: Main, Weapons, Proficiencies, Costs, Adjustments, Saving
//! Throws, Items Worn, Container, Carry Me, Skills, Abilities, Schools,
//! Setting, Racial, Traits, Class Abilities, Magic Items, General Info,
//! Restrict Class, Description and, for Expansion kits, Kit (availability
//! and starting money). Every other member keeps the value the record had
//! (a copy keeps its source's).

use cr_formats::parts::{
    KIT_CURRENT_VERSION, KIT_HEADER, PART_TYPES, Part, PartData, PartHead, PartKit, PartKitData,
    PartVersion, PartsFile,
};
use cr_formats::{ArchiveReader, Field, Object};
use cr_i18n::{tr, tr_noop};

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
pub const NONWEAPON_PROFICIENCIES: usize = 4;
pub const SCHOOLS: usize = 9;

/// The types whose user parts the Expansion checks against the standard
/// ones of the same type each time it opens the database (0x472b7b), in
/// its order.
pub const CHECKED_ON_OPEN: [usize; 4] = [NONWEAPON_PROFICIENCIES, KITS, SCHOOLS, MAGICAL_ITEMS];

/// The text of the original's Name Conflict (IDD 246).
pub const NAME_CONFLICT: &str = tr_noop!(
    "There is a conflict between the name of a Custom Item and the name of a Standard Item.  \
     You must enter a new name for your custom item before you may continue."
);

/// The user parts the Expansion renames when it opens the database
/// (0x47dffe): those of a [`CHECKED_ON_OPEN`] type with the name of a
/// standard part of that type, ignoring case and outer spaces. Each is
/// given its name trimmed, in capitals, with a "*" (the original
/// capitalizes the name it compares and keeps it). The index into the
/// user list and that name; when [`name_taken`] says it is taken, the
/// original's Name Conflict asks for another.
pub fn open_renames(db: &super::UserDb<Parts>) -> Vec<(usize, String)> {
    let key = |r: &PartRec| r.part.head.name.trim().to_uppercase();
    let mut renames = Vec::new();
    for t in CHECKED_ON_OPEN {
        for (i, r) in db.user.iter().enumerate() {
            if r.part_type == t
                && db
                    .standard
                    .iter()
                    .any(|s| s.part_type == t && key(s) == key(r))
            {
                renames.push((i, format!("{}*", key(r))));
            }
        }
    }
    renames
}

/// Whether the original's check (0x47e2f6) refuses `name` for a user part:
/// it is empty once trimmed, or a standard or user part of any type, or
/// one of `also`, has it, ignoring case and outer spaces.
pub fn name_taken(db: &super::UserDb<Parts>, name: &str, also: &[String]) -> bool {
    let key = |n: &str| n.trim().to_uppercase();
    let name = key(name);
    name.is_empty()
        || db
            .standard
            .iter()
            .chain(&db.user)
            .any(|r| key(&r.part.head.name) == name)
        || also.iter().any(|n| key(n) == name)
}

/// The part types the app lists together in one section, the ones without
/// a section of their own: the id their records' keys carry, their array
/// index and their name. Weapon proficiencies are left out: the files hold
/// none, the program makes them from the weapons.
pub const OTHER_TYPES: [(&str, usize, &str); 10] = [
    ("racial-abilities", 5, tr_noop!("Racial Abilities")),
    ("languages", 6, tr_noop!("Languages")),
    ("schools", 9, tr_noop!("Schools of Magic")),
    ("traits", 11, tr_noop!("Traits")),
    ("disadvantages", 12, tr_noop!("Disadvantages")),
    ("gems-and-art", 13, tr_noop!("Gems and Art")),
    (
        "class-abilities",
        CLASS_ABILITIES,
        tr_noop!("Class Abilities"),
    ),
    ("currency", 16, tr_noop!("Currency Types")),
    ("count", 17, tr_noop!("Count")),
    ("spheres", 18, tr_noop!("Spheres")),
];

/// The id of the section that lists [`OTHER_TYPES`].
pub const OTHER_SECTION: &str = "parts";

/// The part type of an [`OTHER_TYPES`] id.
pub fn other_type(id: &str) -> Option<usize> {
    OTHER_TYPES
        .iter()
        .find(|(i, ..)| *i == id)
        .map(|(_, t, _)| *t)
}

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
            names.sort_by_cached_key(|n| crate::collate::sort_key(n));
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
    names.sort_by_cached_key(|n| crate::collate::sort_key(n));
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
    const NOUN: &'static str = tr_noop!("Part");
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

    /// With what IDD 155's OnInitDialog forces already in place, so the
    /// editor opens as the original's does (a new magical item's XP Value
    /// can be typed at once).
    fn new_record(context: &NewContext) -> PartRec {
        let mut r = PartRec {
            part_type: context.scope,
            part: new_part(context.flavor, context.scope),
        };
        forced_on_open(&mut r);
        r
    }

    /// The weapon proficiency name defaults to the part's name (the
    /// Proficiencies page fills it in, and OK stores it).
    /// Also what IDD 155's OnInitDialog forces: armor is wearable and
    /// magical items are magic.
    fn prepare(r: &mut PartRec) {
        forced_on_open(r);
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

    /// Copy (IDD 139, 0x6907e2) refuses the class abilities whose value
    /// is one the program computes itself, such as the rogue skills: only
    /// No score, Dexterity / Level and Base and increment (+0x77c 0, 1
    /// and 5) are copied.
    fn copy_refusal(r: &PartRec) -> Option<String> {
        (r.part_type == CLASS_ABILITIES && ![0, 1, 5].contains(&r.part.data.unknown_0x778[1]))
            .then(|| tr("This class ability may not be copied.  Returning.").to_owned())
    }

    fn name_message(error: &NameError, r: &PartRec, _adding: bool) -> String {
        let reason = match error {
            NameError::Used | NameError::UsedByStandard => {
                tr!(
                    "Name already used in catagory {category}.",
                    category = type_name(r.part_type)
                )
            }
            e => default_reason(e),
        };
        tr!("Part not added: Error. {reason}", reason)
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
            _ if price > 0 => tr!("{price} gp", price),
            _ if h.unknown_0x230 > 0 => tr!("{xp} XP", xp = h.unknown_0x230),
            _ => String::new(),
        }
    }
}

/// The part editor's fields, for [`shown_values`].
pub fn shown_fields(flavor: Flavor) -> Vec<FieldDesc<PartRec>> {
    Parts::fields(flavor)
}

/// What the part editor's pages hold for `part`, as its catalog page lists
/// it: the set values of the fields its type shows, by page and group.
/// Names, pictures, icons, the description and the values that need the
/// installation's lists are left out.
pub fn shown_values(
    fields: &[FieldDesc<PartRec>],
    part_type: usize,
    part: &Part,
) -> Vec<(String, Vec<(String, String)>)> {
    use super::{FieldKind, Options};
    let r = PartRec {
        part_type,
        part: part.clone(),
    };
    let mut shown: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for f in fields {
        if f.page == pages::DESCRIPTION
            || f.source_getter.is_some()
            || f.visible.is_some_and(|v| !v(&r))
            || f.enabled.is_some_and(|e| !e(&r))
            || f.enabled_at.is_some_and(|e| !e(&r, f.arg))
        {
            continue;
        }
        let value = (f.getter)(&r, f.arg);
        let text = match f.kind {
            FieldKind::MultiLine | FieldKind::Picture(_) | FieldKind::Icon => continue,
            FieldKind::Text { .. } if value.text() == part.head.name => continue,
            FieldKind::Check => {
                if !value.bool() {
                    continue;
                }
                tr("Yes").to_owned()
            }
            FieldKind::Choice(Options::Fixed(options)) => match value.int() {
                i @ 1.. => options
                    .get(i as usize)
                    .map(|o| tr(o).to_owned())
                    .unwrap_or_default(),
                _ => continue,
            },
            FieldKind::Choice(_) => continue,
            FieldKind::Int { .. } if value.int() == 0 => continue,
            FieldKind::OptInt { none, inverted, .. } => {
                let n = value.int();
                let is_none = n == none || (none < 0 && n < 0);
                match (inverted, is_none) {
                    (true, true) => tr("Yes").to_owned(),
                    (false, true) => continue,
                    (_, false) if n == 0 => continue,
                    (_, false) => n.to_string(),
                }
            }
            FieldKind::Real { none, .. } => match (none, value) {
                (Some((x, label)), Value::Real(v)) if v == x => tr(label).to_owned(),
                (_, Value::Real(0.0)) => continue,
                (_, v) => v.text(),
            },
            FieldKind::Dice if value.dice() == [0; 3] => continue,
            _ => value.text(),
        };
        if text.is_empty() {
            continue;
        }
        let title = match f.group {
            "" => tr(f.page).to_owned(),
            group => format!("{} – {}", tr(f.page), tr(group)),
        };
        let row = (tr(f.label).to_owned(), text);
        match shown.iter_mut().find(|(t, _)| *t == title) {
            Some((_, rows)) => rows.push(row),
            None => shown.push((title, vec![row])),
        }
    }
    shown
}

/// What IDD 155's OnInitDialog forces: armor is wearable and magical
/// items are magic.
fn forced_on_open(r: &mut PartRec) {
    match r.part_type {
        ARMOR => r.part.head.unknown_0x250[0] = 1,
        MAGICAL_ITEMS => r.part.head.unknown_0x224 = 1,
        _ => {}
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
    fn user_parts_named_like_standard_ones_are_renamed_on_open() {
        let part = |t: usize, name: &str| {
            let mut r = Parts::new_record(&context(Flavor::Expansion, t));
            Parts::set_name(&mut r, name);
            r
        };
        let db = UserDb::<Parts>::new(
            Flavor::Expansion,
            vec![part(KITS, "Gladiator"), part(WEAPONS, "Dagger")],
            vec![
                part(WEAPONS, "dagger"),
                part(KITS, " gladiator "),
                part(KITS, "GLADIATOR*"),
                part(MAGICAL_ITEMS, "Gladiator"),
            ],
        );
        // Only a type checked on open, against the same type.
        assert_eq!(open_renames(&db), [(1, "GLADIATOR*".to_owned())]);
        assert!(name_taken(&db, "gladiator*", &[]));
        assert!(name_taken(&db, "  ", &[]));
        assert!(name_taken(&db, "Brute", &["brute".to_owned()]));
        assert!(!name_taken(&db, "Brute", &[]));
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
    fn items_keep_their_picture_as_the_original_writes_it() {
        let fields = Parts::fields(Flavor::Expansion);
        let picture = fields.iter().find(|f| f.label == "Picture").unwrap();
        assert!(matches!(picture.kind, FieldKind::Icon));
        let mut sword = Parts::new_record(&context(Flavor::Expansion, WEAPONS));
        // The original's new items have none.
        assert_eq!(sword.part.head.unknown_0x234, "");
        assert!(picture.is_visible(&sword));
        for stored in ["12", "CRE_espada.bmp", ""] {
            picture.set(&mut sword, &Value::Text(stored.into()), &Sources::default());
            assert_eq!(sword.part.head.unknown_0x234, stored);
            assert_eq!(picture.get(&sword), Value::Text(stored.into()));
        }
        // Armor has no General Information page.
        let armor = Parts::new_record(&context(Flavor::Expansion, ARMOR));
        assert!(!picture.is_visible(&armor));
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

    /// The other types and the six with sections of their own are every
    /// type the original's list offers but weapon proficiencies.
    #[test]
    fn other_types_complete_the_list() {
        let own = [WEAPONS, ARMOR, 4, EQUIPMENT, MAGICAL_ITEMS, KITS];
        let mut listed: Vec<usize> = OTHER_TYPES.iter().map(|(_, t, _)| *t).collect();
        listed.extend(own);
        listed.sort_unstable();
        let mut offered: Vec<usize> = editable_types(Flavor::Expansion)
            .into_iter()
            .map(|(t, _)| t)
            .filter(|t| *t != 3)
            .collect();
        offered.sort_unstable();
        assert_eq!(listed, offered);
        for (id, t, _) in OTHER_TYPES {
            assert_eq!(other_type(id), Some(t));
        }
        assert_eq!(other_type(OTHER_SECTION), None);
    }

    #[test]
    fn computed_class_abilities_are_not_copied() {
        let mut ability = Parts::new_record(&context(Flavor::Expansion, CLASS_ABILITIES));
        for (value, refused) in [(0, false), (1, false), (5, false), (2, true), (6, true)] {
            ability.part.data.unknown_0x778[1] = value;
            assert_eq!(Parts::copy_refusal(&ability).is_some(), refused, "{value}");
        }
        let mut trait_ = Parts::new_record(&context(Flavor::Expansion, 11));
        trait_.part.data.unknown_0x778[1] = 6;
        assert_eq!(Parts::copy_refusal(&trait_), None);
    }
}
