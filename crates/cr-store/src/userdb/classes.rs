//! Custom Classes (`UserDbas/ClassU.dat`, Expansion only): IDD 191
//! "Classes" and the IDD 192 class editor with its group, ability score,
//! alignment and description pages; and the class names, identifiers and
//! groups the other editors need.
//!
//! The pages ported are the ones a new class needs to be usable: the main
//! page, the page of the class's group, Ability Scores, Alignment and
//! Description. The other pages (levels, proficiencies, spells, ...) keep
//! the values the class had; a new class gets the defaults of the
//! original's constructor.

use cr_formats::classes::{CURRENT_VERSION, Class, ClassData};
use cr_formats::{ArchiveReader, Object};

use super::{FieldDesc, FieldKind, Flavor, NameError, NewContext, Options, UserKind, Value};
use crate::{Installation, Result};

/// The class groups, by `unknown_0x14[0]` (and `id / 1000 - 1`).
pub const GROUPS: [&str; 5] = ["Warrior", "Rogue", "Priest", "Wizard", "Psionicist"];

/// The alignments a class allows, in the order the files list them.
pub const ALIGNMENTS: [&str; 9] = [
    "Lawful Good",
    "Lawful Neutral",
    "Lawful Evil",
    "Neutral Good",
    "True Neutral",
    "Neutral Evil",
    "Chaotic Good",
    "Chaotic Neutral",
    "Chaotic Evil",
];

pub const ABILITIES: [&str; 6] = [
    "Strength",
    "Dexterity",
    "Constitution",
    "Intelligence",
    "Wisdom",
    "Charisma",
];

/// Shown after a class is added (two spaces before the name, as in the
/// original).
pub fn added_message(name: &str) -> String {
    format!(
        "To access  {name} in character creation, you must go to Custom Races and assign this \
         class to the appropriate races from the Allowed Classes screen."
    )
}

/// Asked before the group of an existing class changes.
pub const GROUP_CHANGE_WARNING: &str = "Changing the Class group will remove all Race/Class \
     combination information.\nDo you wish to continue anyway?";

/// The title of the message for a `/` in a class name.
pub const SLASH_TITLE: &str = "Invalid Character In Class Name";

/// One class as the other editors see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassEntry {
    pub name: String,
    /// Fighter 1000; 0 for user classes.
    pub id: i32,
    /// Index into [`GROUPS`].
    pub group: Option<usize>,
}

/// The group index of a class identifier (Fighter 1000: 0, Thief 2000: 1).
pub fn group_of(id: i32) -> Option<usize> {
    let group = usize::try_from(id / 1000).ok()?.checked_sub(1)?;
    (group < GROUPS.len()).then_some(group)
}

/// The identifier of a class in either layout.
pub fn class_id(class: &Class) -> i32 {
    match class {
        Class::Expansion { data, .. } => data.id,
        // The 2.0 layout keeps the identifier at value 22.
        Class::Core20(data) => data.unknown_0x10[22],
    }
}

/// The group of a class in either layout.
pub fn class_group(class: &Class) -> Option<usize> {
    let stored = match class {
        Class::Expansion { data, .. } => data.unknown_0x14[0],
        Class::Core20(data) => data.unknown_0x10[0],
    };
    usize::try_from(stored)
        .ok()
        .filter(|g| *g < GROUPS.len())
        .or_else(|| group_of(class_id(class)))
}

/// Every class, standard then user, sorted by name.
pub fn class_list(install: &Installation) -> Result<Vec<ClassEntry>> {
    let mut list: Vec<ClassEntry> = install
        .list_objects("Class")?
        .iter()
        .filter_map(|o| match o {
            Object::Class(c) => Some(ClassEntry {
                name: c.name().to_owned(),
                id: class_id(c),
                group: class_group(c),
            }),
            _ => None,
        })
        .collect();
    list.sort_by_key(|c| c.name.to_lowercase());
    list.dedup_by(|a, b| a.name == b.name);
    Ok(list)
}

/// The class names of each group, sorted.
pub fn by_group(classes: &[ClassEntry]) -> [Vec<String>; 5] {
    let mut groups: [Vec<String>; 5] = Default::default();
    for c in classes {
        if let Some(g) = c.group {
            groups[g].push(c.name.clone());
        }
    }
    groups
}

/// Every combination of two or three classes from different groups,
/// named in group order ("Fighter/Thief", "Fighter/Thief/Mage").
pub fn multiclass_names(classes: &[ClassEntry]) -> Vec<String> {
    let groups = by_group(classes);
    let mut names = Vec::new();
    for a in 0..5 {
        for b in a + 1..5 {
            for x in &groups[a] {
                for y in &groups[b] {
                    names.push(format!("{x}/{y}"));
                    for group in &groups[b + 1..] {
                        for z in group {
                            names.push(format!("{x}/{y}/{z}"));
                        }
                    }
                }
            }
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    names
}

pub struct Classes;

fn data(r: &Class) -> Option<&ClassData> {
    match r {
        Class::Expansion { data, .. } => Some(data),
        Class::Core20(_) => None,
    }
}

fn data_mut(r: &mut Class) -> Option<&mut ClassData> {
    match r {
        Class::Expansion { data, .. } => Some(data),
        Class::Core20(_) => None,
    }
}

/// The group index of a class being edited.
pub fn group(r: &Class) -> usize {
    data(r).map_or(0, |d| d.unknown_0x14[0].clamp(0, 4) as usize)
}

/// A new class as the Expansion's constructor (0x662281) leaves it.
pub fn new_class() -> Class {
    let zeros = vec![0u8; 1 << 16];
    let mut d = ClassData::read_fields(&mut ArchiveReader::new(&zeros), CURRENT_VERSION)
        .expect("zero bytes read as a class");
    d.unknown_0x3050 = Some([1; 102]);
    d.unknown_0x31e8 = Some([1; 102]);
    d.unknown_0x3380 = Some([10; 101]);
    let mut row = [0; 12];
    row[6..].fill(25);
    d.unknown_0x356c = Some([row; 101]);
    d.unknown_0x4a70 = Some(row);
    d.flags_0x3044 = Some([1, 0, 0, 0, 0]);
    d.unknown_0x3000 = Some("Any".into());
    d.unknown_0x2ff8 = -10;
    d.unknown_0x2fc8 = Some([1, 1]);
    d.unknown_0x94 = Some([1, 0, 0]);
    d.unknown_0xc8 = 1;
    Class::Expansion {
        version: format!("TSRP3 V{CURRENT_VERSION}"),
        data: d,
    }
}

/// A plain member of the class data.
macro_rules! cint {
    ($page:expr, $label:expr, $min:expr, $max:expr, $($f:tt)+) => {
        FieldDesc::new(
            $page,
            $label,
            FieldKind::Int { min: $min, max: $max },
            |r: &Class, _| Value::Int(data(r).map_or(0, |d| d.$($f)+ as i64)),
            |r, _, v, _| {
                if let Some(d) = data_mut(r) {
                    d.$($f)+ = v.int() as _;
                }
            },
        )
    };
}

/// A byte flag: plain (`byte`), present from some version on (`Option<u8>`)
/// or one of an optional array.
macro_rules! cflag {
    ($page:expr, $label:expr, byte $f:ident) => {
        FieldDesc::new(
            $page,
            $label,
            FieldKind::Check,
            |r: &Class, _| Value::Bool(data(r).is_some_and(|d| d.$f != 0)),
            |r, _, v, _| {
                if let Some(d) = data_mut(r) {
                    d.$f = u8::from(v.bool());
                }
            },
        )
    };
    ($page:expr, $label:expr, $f:ident) => {
        FieldDesc::new(
            $page,
            $label,
            FieldKind::Check,
            |r: &Class, _| Value::Bool(data(r).and_then(|d| d.$f).unwrap_or(0) != 0),
            |r, _, v, _| {
                if let Some(flag) = data_mut(r).and_then(|d| d.$f.as_mut()) {
                    *flag = u8::from(v.bool());
                }
            },
        )
    };
    ($page:expr, $label:expr, $f:ident[$i:expr]) => {
        FieldDesc::new(
            $page,
            $label,
            FieldKind::Check,
            |r: &Class, _| Value::Bool(data(r).and_then(|d| d.$f).map_or(0, |f| f[$i]) != 0),
            |r, _, v, _| {
                if let Some(flags) = data_mut(r).and_then(|d| d.$f.as_mut()) {
                    flags[$i] = u8::from(v.bool());
                }
            },
        )
    };
}

impl UserKind for Classes {
    type Rec = Class;
    const NOUN: &'static str = "Class";
    const FILE: &'static str = "UserDbas/ClassU.dat";
    const MAX_NAME: usize = 18;

    fn name(r: &Class) -> &str {
        r.name()
    }

    fn set_name(r: &mut Class, name: &str) {
        match r {
            Class::Expansion { data, .. } => name.clone_into(&mut data.name),
            Class::Core20(data) => name.clone_into(&mut data.name),
        }
    }

    fn standard(install: &Installation) -> Result<Vec<Class>> {
        super::standard_list(install, "Class", pick)
    }

    fn parse_user(bytes: &[u8]) -> std::result::Result<Vec<Class>, cr_formats::Error> {
        super::parse_list(bytes).map(|l| l.into_iter().filter_map(pick).collect())
    }

    fn user_bytes(records: &[Class]) -> std::result::Result<Vec<u8>, cr_formats::Error> {
        super::list_bytes(
            records
                .iter()
                .cloned()
                .map(|c| Object::Class(Box::new(c)))
                .collect(),
        )
    }

    fn new_record(_context: &NewContext) -> Class {
        new_class()
    }

    fn copied(r: &mut Class) {
        if let Some(d) = data_mut(r) {
            d.id = 0;
        }
    }

    fn check_name(name: &str) -> Option<NameError> {
        name.contains('/').then_some(NameError::Slash)
    }

    fn fields(_flavor: Flavor) -> Vec<FieldDesc<Class>> {
        const MAIN: &str = "Class";
        let mut f = vec![
            FieldDesc::new(
                MAIN,
                "Name",
                FieldKind::Text { max: 18 },
                |r: &Class, _| Value::Text(r.name().to_owned()),
                |r, _, v, _| Self::set_name(r, &v.text()),
            ),
            FieldDesc::new(
                MAIN,
                "Group",
                FieldKind::Choice(Options::Fixed(&GROUPS)),
                |r: &Class, _| Value::Int(group(r) as i64),
                |r, _, v, _| {
                    if let Some(d) = data_mut(r) {
                        d.unknown_0x14[0] = v.int().clamp(0, 4) as i32;
                    }
                },
            ),
            cint!(MAIN, "Character Points", 0, 100_000, unknown_0x54[0]),
        ];

        const WARRIOR: &str = "Warrior";
        f.extend(
            [
                cflag!(
                    WARRIOR,
                    "Weapon Specialization CP cost of 2 when single classed",
                    flag_0x2791
                ),
                cflag!(
                    WARRIOR,
                    "Allow Weapon Specialization at first level, second at ...",
                    flag_0x2792
                ),
                cflag!(
                    WARRIOR,
                    "Weapon Mastery CP cost of 2 when single classed",
                    flags_0x2793[0]
                ),
                cflag!(
                    WARRIOR,
                    "Allow Weapon Mastery at fifth level, sixth level ...",
                    flags_0x2793[1]
                ),
                cflag!(
                    WARRIOR,
                    "Ignore additional CP cost for Two Weapon Fighting",
                    flags_0x2795[0]
                ),
                cflag!(WARRIOR, "Allow unarmed combat (Fighter)", flags_0x2795[1]),
                cflag!(
                    WARRIOR,
                    "Allow Weapon Specialization for Core Rules characters",
                    flag_0x2797
                ),
                cflag!(
                    WARRIOR,
                    "Receive the special Warrior Constitution Hit Point bonus",
                    flag_0x279c
                ),
                cint!(WARRIOR, "Gold", 0, 1_000_000_000, unknown_0x2798),
            ]
            .map(|d| d.visible(|r| group(r) == 0)),
        );
        const ROGUE: &str = "Rogue";
        f.extend(
            [
                cflag!(
                    ROGUE,
                    "Only half of the points allocated per level may go to one skill",
                    byte flag_0x278c
                ),
                cflag!(ROGUE, "Candidate for Rogue Skills", flag_0x27a4),
            ]
            .map(|d| d.visible(|r| group(r) == 1)),
        );
        const PRIEST: &str = "Priest";
        f.extend(
            [
                cflag!(
                    PRIEST,
                    "Cannot use an edged weapon (Cleric)",
                    flags_0x27a0[0]
                ),
                cflag!(
                    PRIEST,
                    "Cannot purchase Weapon Groups (Cleric)",
                    flag_0x27a1
                ),
                cflag!(
                    PRIEST,
                    "Gain access to 8th level wizard spells at 16th level",
                    flags_0x278e[0]
                ),
                cflag!(
                    PRIEST,
                    "No more than three-quarters of spells available",
                    flags_0x278e[1]
                ),
                cflag!(
                    PRIEST,
                    "Receive double the bonus spells they are entitled to",
                    flag_0x2790
                ),
                cflag!(
                    PRIEST,
                    "Always use the priest's THAC0 for attack rolls",
                    flag_0x486c
                ),
                cint!(PRIEST, "Percentage", 0, 100, unknown_0x485c[0]),
            ]
            .map(|d| d.visible(|r| group(r) == 2)),
        );
        const WIZARD: &str = "Wizard";
        f.extend(
            [
                cflag!(
                    WIZARD,
                    "Is a Specialist (Applied to item restrictions)",
                    flag_0x279d
                ),
                FieldDesc::new(
                    WIZARD,
                    "School",
                    FieldKind::Combo {
                        options: Options::Source("Schools Of Magic"),
                        free: true,
                    },
                    |r: &Class, _| {
                        Value::Text(
                            data(r)
                                .map(|d| d.unknown_0x2788.clone())
                                .unwrap_or_default(),
                        )
                    },
                    |r, _, v, _| {
                        if let Some(d) = data_mut(r) {
                            d.unknown_0x2788 = v.text();
                        }
                    },
                ),
            ]
            .map(|d| d.visible(|r| group(r) == 3)),
        );
        const PSIONICIST: &str = "Psionicist";
        f.extend(
            [
                cflag!(
                    PSIONICIST,
                    "Apply Item restriction as 5 CP",
                    flags_0x279e[0]
                ),
                cflag!(
                    PSIONICIST,
                    "Apply Item restriction as 10 CP",
                    flags_0x279e[1]
                ),
            ]
            .map(|d| d.visible(|r| group(r) == 4)),
        );

        const ABILITY: &str = "Ability Scores";
        for (i, ability) in ABILITIES.iter().enumerate() {
            f.push(
                FieldDesc::new(
                    ABILITY,
                    "Minimum",
                    FieldKind::Int { min: 0, max: 25 },
                    |r: &Class, i| {
                        Value::Int(data(r).map_or(0, |d| d.ability_requirements[i].into()))
                    },
                    |r, i, v, _| {
                        if let Some(d) = data_mut(r) {
                            d.ability_requirements[i] = v.int() as i32;
                        }
                    },
                )
                .arg(i)
                .group(ability),
            );
            f.push(
                FieldDesc::new(
                    ABILITY,
                    "Prime Requisite",
                    FieldKind::Check,
                    |r: &Class, i| Value::Bool(data(r).is_some_and(|d| d.unknown_0x34[i] != 0)),
                    |r, i, v, _| {
                        if let Some(d) = data_mut(r) {
                            d.unknown_0x34[i] = i32::from(v.bool());
                        }
                    },
                )
                .arg(i)
                .group(ability),
            );
        }
        f.extend([
            cflag!(ABILITY, "Allow Exceptional Strength", flag_0x3554).group("Strength"),
            cflag!(
                ABILITY,
                "Permanently add up to 2 points to STR (or 20 to exceptional STR)",
                flags_0x486e[0]
            )
            .group("Strength"),
        ]);
        for (i, alignment) in ALIGNMENTS.iter().enumerate() {
            f.push(
                FieldDesc::new(
                    "Alignment",
                    alignment,
                    FieldKind::Check,
                    |r: &Class, i| {
                        Value::Bool(
                            data(r)
                                .is_some_and(|d| d.alignments.iter().any(|a| a == ALIGNMENTS[i])),
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
                .group("Alignments Allowed"),
            );
        }
        f.push(FieldDesc::new(
            "Description",
            "Description",
            FieldKind::MultiLine,
            |r: &Class, _| Value::Text(data(r).map(|d| d.unknown_0x8.clone()).unwrap_or_default()),
            |r, _, v, _| {
                if let Some(d) = data_mut(r) {
                    d.unknown_0x8 = v.text();
                }
            },
        ));
        f
    }

    fn name_message(error: &NameError, _r: &Class, _adding: bool) -> String {
        match error {
            NameError::Slash => super::default_reason(error),
            NameError::UsedByStandard => "Class not added: Error. The name you chose for your \
                 class is identical to a standard class name.  Please choose a unique name."
                .into(),
            e => format!("Class not added: Error. {}", super::default_reason(e)),
        }
    }

    fn delete_warning(_r: &Class) -> Option<String> {
        Some(
            "Any character or NPC created from the deleted Class will no longer function correctly"
                .into(),
        )
    }

    fn summary(r: &Class) -> String {
        GROUPS[group(r)].into()
    }
}

fn pick(o: Object) -> Option<Class> {
    match o {
        Object::Class(c) => Some(*c),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userdb::testing::TempInstall;
    use crate::userdb::{Sources, UserDb};

    fn entry(name: &str, id: i32) -> ClassEntry {
        ClassEntry {
            name: name.into(),
            id,
            group: group_of(id),
        }
    }

    #[test]
    fn groups_and_multiclass_names() {
        assert_eq!(group_of(1001), Some(0));
        assert_eq!(group_of(5000), Some(4));
        assert_eq!(group_of(0), None);
        let classes = [
            entry("Fighter", 1000),
            entry("Thief", 2000),
            entry("Mage", 4000),
        ];
        assert_eq!(
            multiclass_names(&classes),
            [
                "Fighter/Mage",
                "Fighter/Thief",
                "Fighter/Thief/Mage",
                "Thief/Mage"
            ]
        );
    }

    #[test]
    fn class_names_are_short_and_slash_free() {
        let mut fighter = new_class();
        Classes::set_name(&mut fighter, "Fighter");
        let t = TempInstall::new(
            "classes",
            &[
                (
                    "Database/Class.dat",
                    Classes::user_bytes(&[fighter]).unwrap(),
                ),
                ("UserDbas/AllowedU.dat", vec![0, 0]),
            ],
        );
        let mut db = UserDb::<Classes>::load(&t.install).unwrap();
        let mut c = db.new_record(0, "");
        for (name, error) in [
            ("fighter", NameError::UsedByStandard),
            ("Fighter/Thief", NameError::Slash),
            ("A name of nineteen.", NameError::TooLong(18)),
        ] {
            Classes::set_name(&mut c, name);
            assert_eq!(db.add(c.clone()), Err(error));
        }
        assert!(
            Classes::name_message(&NameError::UsedByStandard, &c, true)
                .contains("identical to a standard class name")
        );
        let fields = Classes::fields(Flavor::Expansion);
        let gold = fields.iter().find(|f| f.label == "Gold").unwrap();
        assert!(gold.is_visible(&c));
        let lg = fields.iter().find(|f| f.label == "Lawful Good").unwrap();
        let tn = fields.iter().find(|f| f.label == "True Neutral").unwrap();
        tn.set(&mut c, &Value::Bool(true), &Sources::default());
        lg.set(&mut c, &Value::Bool(true), &Sources::default());
        assert_eq!(
            data(&c).unwrap().alignments,
            ["Lawful Good", "True Neutral"]
        );
        Classes::set_name(&mut c, "UdbClass");
        db.add(c).unwrap();
        db.save(&t.install).unwrap();
        assert_eq!(UserDb::<Classes>::load(&t.install).unwrap().user.len(), 1);
    }
}
