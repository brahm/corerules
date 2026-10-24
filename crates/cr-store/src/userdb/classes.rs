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
use cr_i18n::{tr, tr_noop};

use super::{FieldDesc, Flavor, NameError, NewContext, UserKind};
use crate::{Installation, Result};

pub mod levels;
mod pages;

/// The class groups, by `unknown_0x14[0]` (and `id / 1000 - 1`).
pub const GROUPS: [&str; 5] = [
    tr_noop!("Warrior"),
    tr_noop!("Rogue"),
    tr_noop!("Priest"),
    tr_noop!("Wizard"),
    tr_noop!("Psionicist"),
];

/// The alignments a class allows, in the order the files list them.
pub const ALIGNMENTS: [&str; 9] = [
    tr_noop!("Lawful Good"),
    tr_noop!("Lawful Neutral"),
    tr_noop!("Lawful Evil"),
    tr_noop!("Neutral Good"),
    tr_noop!("True Neutral"),
    tr_noop!("Neutral Evil"),
    tr_noop!("Chaotic Good"),
    tr_noop!("Chaotic Neutral"),
    tr_noop!("Chaotic Evil"),
];

pub const ABILITIES: [&str; 6] = [
    tr_noop!("Strength"),
    tr_noop!("Dexterity"),
    tr_noop!("Constitution"),
    tr_noop!("Intelligence"),
    tr_noop!("Wisdom"),
    tr_noop!("Charisma"),
];

/// Shown after a class is added (two spaces before the name, as in the
/// original).
pub fn added_message(name: &str) -> String {
    tr!(
        "To access  {name} in character creation, you must go to Custom Races and assign this \
         class to the appropriate races from the Allowed Classes screen.",
        name
    )
}

/// Asked before the group of an existing class changes.
pub const GROUP_CHANGE_WARNING: &str = tr_noop!(
    "Changing the Class group will remove all Race/Class \
     combination information.\nDo you wish to continue anyway?"
);

/// The title of the message for a `/` in a class name.
pub const SLASH_TITLE: &str = tr_noop!("Invalid Character In Class Name");

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
    Ok(class_entries(&install.list_objects("Class")?))
}

/// [`class_list`] of the class records `objects`, standard then user.
pub fn class_entries(objects: &[Object]) -> Vec<ClassEntry> {
    let mut list: Vec<ClassEntry> = objects
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
    list.sort_by_cached_key(|c| crate::collate::sort_key(&c.name));
    list.dedup_by(|a, b| a.name == b.name);
    list
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
    names.sort_by_cached_key(|n| crate::collate::sort_key(n));
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

impl UserKind for Classes {
    type Rec = Class;
    const NOUN: &'static str = tr_noop!("Class");
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
        pages::fields()
    }

    fn name_message(error: &NameError, _r: &Class, _adding: bool) -> String {
        match error {
            NameError::Slash => super::default_reason(error),
            NameError::UsedByStandard => tr("Class not added: Error. The name you chose for your \
                 class is identical to a standard class name.  Please choose a unique name.")
            .into(),
            e => tr!(
                "Class not added: Error. {reason}",
                reason = super::default_reason(e)
            ),
        }
    }

    fn delete_warning(_r: &Class) -> Option<String> {
        Some(
            tr("Any character or NPC created from the deleted Class will no longer function correctly")
                .into(),
        )
    }

    fn change_question(stored: &Class, edited: &Class) -> Option<&'static str> {
        (group(stored) != group(edited)).then_some(GROUP_CHANGE_WARNING)
    }

    /// The original's Remove (0x6920dc) and group change (0x6cfbec) take
    /// the class out of every race's allowed classes (0x47d777).
    fn detach(install: &Installation, r: &Class) -> Result<()> {
        let mut db = super::allowed::AllowedDb::load(install)?;
        if db.remove_class(r.name()) {
            db.save(install)?;
        }
        Ok(())
    }

    fn summary(r: &Class) -> String {
        tr(GROUPS[group(r)]).into()
    }

    /// New Character offers only classes with an alignment, and a class
    /// whose hit die has no sides starts with 1 hit point.
    fn save_warning(r: &Class) -> Option<String> {
        let d = data(r)?;
        let mut missing = Vec::new();
        if d.alignments.is_empty() {
            missing.push(tr(
                "No alignment is allowed, so New Character will not offer the class.",
            ));
        }
        if d.unknown_0x84[0] <= 0 {
            missing.push(tr(
                "The starting hit die has no sides, so characters start with 1 hit point.",
            ));
        }
        (!missing.is_empty()).then(|| missing.join("\n\n"))
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
    use crate::userdb::{Sources, UserDb, Value};

    fn entry(name: &str, id: i32) -> ClassEntry {
        ClassEntry {
            name: name.into(),
            id,
            group: group_of(id),
        }
    }

    #[test]
    fn a_class_without_alignments_or_hit_die_is_warned_about() {
        let mut class = new_class();
        assert_eq!(
            Classes::save_warning(&class).map(|w| w.matches("\n\n").count()),
            Some(1)
        );
        let d = data_mut(&mut class).unwrap();
        d.alignments = vec![ALIGNMENTS[0].into()];
        d.unknown_0x84[0] = 8;
        assert_eq!(Classes::save_warning(&class), None);
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
    fn a_group_change_asks_and_takes_the_class_out_of_the_races() {
        use crate::userdb::allowed::{Allowed, AllowedDb};
        let mut class = new_class();
        Classes::set_name(&mut class, "UdbClass");
        let mut rogue = class.clone();
        data_mut(&mut rogue).unwrap().unknown_0x14[0] = 1;
        assert_eq!(Classes::change_question(&class, &class), None);
        assert_eq!(
            Classes::change_question(&class, &rogue),
            Some(GROUP_CHANGE_WARNING)
        );
        let mut allowed = AllowedDb::default();
        let (i, _) = allowed.ensure("Dwarf");
        let slots = ["UdbClass", "", "", "", ""].map(String::from);
        allowed.records[i].classes = vec![Allowed::new(slots, 12).helper()];
        let t = TempInstall::new(
            "classes-detach",
            &[("UserDbas/AllowedU.dat", allowed.to_bytes().unwrap())],
        );
        Classes::detach(&t.install, &class).unwrap();
        assert!(
            AllowedDb::load(&t.install)
                .unwrap()
                .entries("Dwarf")
                .is_empty()
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
