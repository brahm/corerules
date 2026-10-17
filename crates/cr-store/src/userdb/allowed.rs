//! Allowed Classes (IDD 189, Expansion only): the classes and multi-class
//! combinations each race may take, with a level limit, in
//! `UserDbas/AllowedU.dat`.
//!
//! The file holds one record per race, shipped entries (`flag_0x4` 1) and
//! the user's (0) mixed. Opening the dialog for a race without a record
//! appends an empty one and writes the file at once; OK replaces the
//! race's user entries, inserting them after the shipped multi-class
//! entries and before the shipped single-class ones, each at that same
//! place (so they end up in reverse order of the dialog's sorted list).

use cr_formats::Object;
use cr_formats::allowed::{ClassesAllowed, ClassesAllowedHelper, ClassesAllowedHelperData};

use super::{list_bytes, parse_list};
use crate::{Error, Installation, Result};

pub const FILE: &str = "UserDbas/AllowedU.dat";

pub const NO_SELECTION: &str = "You must first make a selection from the list on the right.";
pub const NO_AVAILABLE_TITLE: &str = "No \"Available\" Class Highlighted";
pub const NO_AVAILABLE: &str =
    "First click on an \"Available\" Class and then click the right arrow button.";
pub const DUAL_AND_MULTI: &str =
    "This Race is set to allow dual-class.\nA Race may not dual-class and multi-class.";
pub const NO_RACE: &str = "You must select a Race from the list";

/// The fewest class names a combination the original refuses holds
/// (`47cf69`: the count must be at most 5).
pub const MAX_COMBINED: usize = 6;

/// One allowed class or combination as the dialog shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowed {
    /// "Fighter", or "UdbClass/Thief" for a combination (group order).
    pub name: String,
    /// One class name per group, empty for none.
    pub classes: [String; 5],
    pub level_limit: i32,
    /// A shipped entry, kept as is.
    pub standard: bool,
}

impl Allowed {
    /// A user entry for `classes` (one name per group).
    pub fn new(classes: [String; 5], level_limit: i32) -> Self {
        let name = classes
            .iter()
            .filter(|c| !c.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join("/");
        Self {
            name,
            classes,
            level_limit,
            standard: false,
        }
    }

    /// The entry the original's add (`47cf69`, then `689761`) makes of a
    /// combination of class names: it refuses six names or more (a
    /// `CClassesAllowedHelperOb` holds one class per group, five in all);
    /// otherwise each name fills the slot of its class's group, the first
    /// name of a group winning and unknown names being skipped. `None`
    /// when refused or when no name is a known class.
    pub fn from_names(
        names: &[&str],
        classes: &[super::classes::ClassEntry],
        level_limit: i32,
    ) -> Option<Self> {
        if names.len() >= MAX_COMBINED {
            return None;
        }
        let mut slots: [String; 5] = Default::default();
        for name in names {
            let Some(group) = classes
                .iter()
                .find(|c| c.name == *name)
                .and_then(|c| c.group)
            else {
                continue;
            };
            if slots[group].is_empty() {
                (*name).clone_into(&mut slots[group]);
            }
        }
        slots
            .iter()
            .any(|s| !s.is_empty())
            .then(|| Self::new(slots, level_limit))
    }

    pub fn is_multiclass(&self) -> bool {
        self.classes.iter().filter(|c| !c.is_empty()).count() > 1
    }

    pub(crate) fn of(data: &ClassesAllowedHelperData) -> Self {
        let mut a = Self::new(
            [
                data.warrior.clone(),
                data.rogue.clone(),
                data.priest.clone(),
                data.wizard.clone(),
                data.psionicist.clone(),
            ],
            data.level_limit.unwrap_or(0),
        );
        a.standard = data.flag_0x4 != 0;
        a
    }

    pub(crate) fn helper(&self) -> Object {
        let [warrior, rogue, priest, wizard, psionicist] = self.classes.clone();
        Object::ClassesAllowedHelper(Box::new(ClassesAllowedHelper {
            version: 1,
            data: ClassesAllowedHelperData {
                warrior,
                rogue,
                priest,
                wizard,
                psionicist,
                level_limit: Some(self.level_limit.clamp(0, 100)),
                unknown_0x20: String::new(),
                flag_0x4: u8::from(self.standard),
                flag_0x24: 0,
            },
        }))
    }
}

/// The records of `AllowedU.dat`, in file order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AllowedDb {
    pub records: Vec<ClassesAllowed>,
}

fn helper_data(o: &Object) -> Option<&ClassesAllowedHelperData> {
    match o {
        Object::ClassesAllowedHelper(h) => Some(&h.data),
        _ => None,
    }
}

impl AllowedDb {
    /// Reads the file; a missing file is an empty list.
    pub fn load(install: &Installation) -> Result<Self> {
        let Some(path) = install.path(FILE) else {
            return Ok(Self::default());
        };
        let objects =
            parse_list(&install.read(FILE)?).map_err(|source| Error::Format { path, source })?;
        Ok(Self {
            records: objects
                .into_iter()
                .filter_map(|o| match o {
                    Object::ClassesAllowed(c) => Some(c),
                    _ => None,
                })
                .collect(),
        })
    }

    pub fn position(&self, race: &str) -> Option<usize> {
        self.records.iter().position(|r| r.race == race)
    }

    /// The record of `race`, appending an empty one when there is none;
    /// the flag says whether it was created (the original then writes the
    /// file at once).
    pub fn ensure(&mut self, race: &str) -> (usize, bool) {
        if let Some(i) = self.position(race) {
            return (i, false);
        }
        self.records.push(ClassesAllowed {
            version: 0,
            race: race.to_owned(),
            unknown_0x1c: String::new(),
            flag_0x20: 0,
            classes: Vec::new(),
        });
        (self.records.len() - 1, true)
    }

    /// The entries of `race`, in file order.
    pub fn entries(&self, race: &str) -> Vec<Allowed> {
        self.position(race)
            .map(|i| {
                self.records[i]
                    .classes
                    .iter()
                    .filter_map(helper_data)
                    .map(Allowed::of)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// OK on the dialog: replaces the user entries of `race` with `user`,
    /// given in the dialog's order.
    pub fn set_user(&mut self, race: &str, user: &[Allowed]) {
        let (i, _) = self.ensure(race);
        let list = &mut self.records[i].classes;
        list.retain(|o| helper_data(o).is_none_or(|d| d.flag_0x4 != 0));
        let at = list
            .iter()
            .rposition(|o| helper_data(o).is_some_and(|d| Allowed::of(d).is_multiclass()))
            .map_or(0, |p| p + 1);
        for entry in user {
            let mut entry = entry.clone();
            entry.standard = false;
            list.insert(at, entry.helper());
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        list_bytes(
            self.records
                .iter()
                .cloned()
                .map(Object::ClassesAllowed)
                .collect(),
        )
        .map_err(|source| Error::Format {
            path: FILE.into(),
            source,
        })
    }

    pub fn save(&self, install: &Installation) -> Result<()> {
        crate::encounter::replace_or_create(install, FILE, "AllowedU.dat", &self.to_bytes()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(warrior: &str, rogue: &str) -> [String; 5] {
        [
            warrior.into(),
            rogue.into(),
            String::new(),
            String::new(),
            String::new(),
        ]
    }

    #[test]
    fn user_entries_go_between_shipped_multis_and_singles() {
        let mut db = AllowedDb::default();
        let mut shipped = vec![
            Allowed::new(class("Fighter", "Thief"), 0),
            Allowed::new(class("", "Thief"), 12),
        ];
        for s in &mut shipped {
            s.standard = true;
        }
        // Shipped entries are kept in their order.
        db.set_user("Dwarf", &[]);
        let (i, created) = db.ensure("Dwarf");
        assert!(!created);
        db.records[i].classes = shipped.iter().map(Allowed::helper).collect();
        let user = [
            Allowed::new(class("UdbClass", ""), 12),
            Allowed::new(class("UdbClass", "Thief"), 0),
        ];
        db.set_user("Dwarf", &user);
        let names: Vec<_> = db
            .entries("Dwarf")
            .iter()
            .map(|a| (a.name.clone(), a.level_limit, a.standard))
            .collect();
        assert_eq!(
            names,
            [
                ("Fighter/Thief".to_owned(), 0, true),
                ("UdbClass/Thief".to_owned(), 0, false),
                ("UdbClass".to_owned(), 12, false),
                ("Thief".to_owned(), 12, true),
            ]
        );
        // OK again replaces the user entries; limits are clamped.
        db.set_user("Dwarf", &[Allowed::new(class("UdbClass", ""), 300)]);
        assert_eq!(db.entries("Dwarf")[1].level_limit, 100);
        assert_eq!(db.entries("Dwarf").len(), 3);
        let bytes = db.to_bytes().unwrap();
        assert_eq!(
            AllowedDb {
                records: parse_list(&bytes)
                    .unwrap()
                    .into_iter()
                    .filter_map(|o| match o {
                        Object::ClassesAllowed(c) => Some(c),
                        _ => None,
                    })
                    .collect()
            },
            db
        );
    }

    #[test]
    fn combinations_fill_one_slot_per_group() {
        use crate::userdb::classes::{ClassEntry, group_of};
        let classes = [
            ("Fighter", 1000),
            ("Paladin", 1001),
            ("Thief", 2000),
            ("Cleric", 3000),
            ("Mage", 4000),
            ("Psionicist", 5000),
        ]
        .map(|(n, id)| ClassEntry {
            name: n.into(),
            id,
            group: group_of(id),
        });
        // The first name of a group wins and unknown names are skipped.
        let a = Allowed::from_names(&["Thief", "Fighter", "Nobody", "Paladin"], &classes, 7);
        assert_eq!(
            a.map(|a| (a.name, a.level_limit)),
            Some(("Fighter/Thief".to_owned(), 7))
        );
        let five = ["Fighter", "Thief", "Cleric", "Mage", "Psionicist"];
        assert!(Allowed::from_names(&five, &classes, 0).is_some());
        // Six names are refused even when one would be dropped.
        let six = [
            "Fighter",
            "Paladin",
            "Thief",
            "Cleric",
            "Mage",
            "Psionicist",
        ];
        assert_eq!(Allowed::from_names(&six, &classes, 0), None);
        assert_eq!(Allowed::from_names(&["Nobody"], &classes, 0), None);
    }
}
