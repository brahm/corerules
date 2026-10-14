//! Treasure tables (`UserDbas/Treasure.dat`): IDD 257 "Treasure Tables",
//! the IDD 258 editor and the IDD 262 entry details.
//!
//! Unlike the other user files, `UserDbas/Treasure.dat` is a full copy of
//! every table, standard and user: the standard tables (flag 1) can be
//! edited and restored from `Database/Treasure.dat`, user tables (flag 0)
//! can be deleted. New tables are appended. A table name is not checked
//! for uniqueness; it only must not be empty.

use cr_formats::Object;
use cr_formats::database::ObjectList;
use cr_formats::inventory::InventoryItem;
use cr_formats::treasure::{TreasureTable, TreasureTableEntry};

use super::{Origin, parts};
use crate::{Error, Installation, Result};

pub const USER_FILE: &str = "UserDbas/Treasure.dat";
pub const STANDARD_FILE: &str = "Database/Treasure.dat";

/// The message for an empty name.
pub const EMPTY_NAME: &str = "You must specify a name for the table.";
/// The name a new table starts with.
pub const NEW_TABLE: &str = "New Table";
pub const DELETE_WARNING: &str =
    "This will delete the selected table.  You will lose any customizations.";
pub const RESTORE_WARNING: &str = "This will revert the selected table to match the Dungeon Master's Guide. You will lose any customizations.";

/// All treasure tables, as the treasure-table list holds them.
#[derive(Debug, Clone, PartialEq)]
pub struct TableDb {
    /// The tables of `UserDbas/Treasure.dat` (of `Database/Treasure.dat`
    /// when there is no user copy yet), in file order.
    pub tables: Vec<TreasureTable>,
    /// `Database/Treasure.dat`, for Restore.
    pub shipped: Vec<TreasureTable>,
}

fn tables_in(install: &Installation, relative: &str) -> Result<Vec<TreasureTable>> {
    let Some(path) = install.path(relative) else {
        return Ok(Vec::new());
    };
    let list = ObjectList::parse(&install.read(relative)?)
        .map_err(|source| Error::Format { path, source })?;
    Ok(list
        .objects
        .into_iter()
        .filter_map(|o| match o {
            Object::TreasureTable(t) => Some(t),
            _ => None,
        })
        .collect())
}

/// Whether a table is one of the shipped ones (Restore instead of
/// Delete).
pub fn is_standard(table: &TreasureTable) -> bool {
    table.flag_0x4 != 0
}

pub fn origin(table: &TreasureTable) -> Origin {
    if is_standard(table) {
        Origin::Standard
    } else {
        Origin::Custom
    }
}

impl TableDb {
    pub fn load(install: &Installation) -> Result<Self> {
        let shipped = tables_in(install, STANDARD_FILE)?;
        let tables = if install.path(USER_FILE).is_some() {
            tables_in(install, USER_FILE)?
        } else {
            shipped.clone()
        };
        Ok(Self { tables, shipped })
    }

    pub fn position(&self, name: &str) -> Option<usize> {
        self.tables.iter().position(|t| t.name == name)
    }

    /// A new, empty user table.
    pub fn new_table() -> TreasureTable {
        TreasureTable {
            name: NEW_TABLE.into(),
            flag_0x4: 0,
            entries: Vec::new(),
        }
    }

    /// Copy…: a user table with the same entries.
    pub fn copy_of(table: &TreasureTable) -> TreasureTable {
        TreasureTable {
            name: String::new(),
            flag_0x4: 0,
            entries: table.entries.clone(),
        }
    }

    /// Checks a table before it is stored (its name trimmed).
    pub fn check(table: &mut TreasureTable) -> std::result::Result<(), String> {
        table.name = table.name.trim().to_owned();
        if table.name.is_empty() {
            return Err(EMPTY_NAME.into());
        }
        Ok(())
    }

    /// Appends a table; returns its index.
    pub fn add(&mut self, mut table: TreasureTable) -> std::result::Result<usize, String> {
        Self::check(&mut table)?;
        self.tables.push(table);
        Ok(self.tables.len() - 1)
    }

    /// Replaces table `index` in place; a standard table stays standard.
    pub fn replace(
        &mut self,
        index: usize,
        mut table: TreasureTable,
    ) -> std::result::Result<(), String> {
        Self::check(&mut table)?;
        table.flag_0x4 = self.tables[index].flag_0x4;
        self.tables[index] = table;
        Ok(())
    }

    /// Deletes a user table; standard tables can only be restored.
    pub fn delete(&mut self, index: usize) -> std::result::Result<TreasureTable, String> {
        if is_standard(&self.tables[index]) {
            return Err("Standard tables cannot be deleted; restore them instead.".into());
        }
        Ok(self.tables.remove(index))
    }

    /// Restore: the standard table `index` as `Database/Treasure.dat` has
    /// it.
    pub fn restore(&mut self, index: usize) -> std::result::Result<(), String> {
        let name = &self.tables[index].name;
        let shipped = self
            .shipped
            .iter()
            .find(|t| &t.name == name)
            .ok_or_else(|| format!("{name} is not in the Dungeon Master's Guide tables."))?;
        self.tables[index] = shipped.clone();
        Ok(())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        ObjectList {
            objects: self
                .tables
                .iter()
                .cloned()
                .map(Object::TreasureTable)
                .collect(),
        }
        .to_bytes()
        .map_err(|source| Error::Format {
            path: USER_FILE.into(),
            source,
        })
    }

    /// OK on the list: writes `UserDbas/Treasure.dat`.
    pub fn save(&self, install: &Installation) -> Result<()> {
        crate::encounter::replace_or_create(install, USER_FILE, "Treasure.dat", &self.to_bytes()?)
    }
}

/// A new row for `item` (a part, or with `is_table` another table) with
/// weight `weight`: one of it, nothing else rolled.
pub fn new_entry(item: &str, is_table: bool, weight: u32) -> TreasureTableEntry {
    TreasureTableEntry {
        weight,
        is_table: u8::from(is_table),
        item: InventoryItem {
            name: item.into(),
            quantity: 1,
            contents: None,
            unknown_0x8: 999,
            flag_0x4: 0,
            extra: None,
        },
        quantity_dice: [0, 0, 1],
        sub_table_1_dice: [0, 0, 0],
        sub_table_1: String::new(),
        sub_table_2_dice: [0, 0, 0],
        sub_table_2: String::new(),
        unknown_0x54: 0,
        spell_min_level: 0,
        spell_max_level: 0,
        spell_count: 0,
        spell_wizard: 0,
    }
}

/// The rows of a table.
pub fn entries(table: &TreasureTable) -> Vec<&TreasureTableEntry> {
    table
        .entries
        .iter()
        .filter_map(|o| match o {
            Object::TreasureTableEntry(e) => Some(&**e),
            _ => None,
        })
        .collect()
}

pub fn entries_mut(table: &mut TreasureTable) -> Vec<&mut TreasureTableEntry> {
    table
        .entries
        .iter_mut()
        .filter_map(|o| match o {
            Object::TreasureTableEntry(e) => Some(&mut **e),
            _ => None,
        })
        .collect()
}

pub fn push_entry(table: &mut TreasureTable, entry: TreasureTableEntry) {
    table
        .entries
        .push(Object::TreasureTableEntry(Box::new(entry)));
}

/// The roll ranges the editor shows for the weights: "1-3", "4", ...
pub fn ranges(table: &TreasureTable) -> Vec<String> {
    let mut low = 1u64;
    entries(table)
        .iter()
        .map(|e| {
            let high = low + u64::from(e.weight);
            let text = match e.weight {
                0 => "-".to_owned(),
                1 => low.to_string(),
                _ => format!("{low}-{}", high - 1),
            };
            low = high;
            text
        })
        .collect()
}

/// The categories of the editor's source combo.
pub const CATEGORIES: [&str; 5] = [
    "Armor",
    "Equipment",
    "Gems and Art",
    "Magical Items",
    "Weapons",
];

/// What the source combo can show: each category × All/Custom/Standard,
/// then Count and Tables.
pub fn source_names() -> Vec<String> {
    let mut names = Vec::new();
    for category in CATEGORIES {
        for which in ["All", "Custom", "Standard"] {
            names.push(format!("{category}, {which}"));
        }
    }
    names.push("Count".into());
    names.push("Tables".into());
    names
}

/// The items the source list shows for `source` (one of
/// [`source_names`]), sorted; the flag says whether they are tables.
pub fn source_items(
    install: &Installation,
    db: &TableDb,
    source: &str,
) -> Result<(Vec<String>, bool)> {
    if source == "Tables" {
        let mut names: Vec<String> = db.tables.iter().map(|t| t.name.clone()).collect();
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup();
        return Ok((names, true));
    }
    let (category, which) = source.split_once(", ").unwrap_or((source, "All"));
    let category = if category == "Magical Items" {
        "Magical Items"
    } else {
        category
    };
    let Some(t) = parts::source_type(category) else {
        return Ok((Vec::new(), false));
    };
    let by_origin = parts::names_by_origin(install)?;
    let mut names: Vec<String> = by_origin[t]
        .iter()
        .filter(|(_, o)| match which {
            "Custom" => *o == Origin::Custom,
            "Standard" => *o == Origin::Standard,
            _ => true,
        })
        .map(|(n, _)| n.clone())
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup();
    Ok((names, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userdb::testing::TempInstall;

    fn shipped() -> Vec<u8> {
        let mut table = TreasureTable {
            name: "DMG 88".into(),
            flag_0x4: 1,
            entries: Vec::new(),
        };
        push_entry(&mut table, new_entry("Potion", false, 20));
        ObjectList {
            objects: vec![Object::TreasureTable(table)],
        }
        .to_bytes()
        .unwrap()
    }

    #[test]
    fn tables_are_copied_edited_restored_and_appended() {
        let t = TempInstall::new("tables", &[("Database/Treasure.dat", shipped())]);
        let mut db = TableDb::load(&t.install).unwrap();
        assert_eq!(db.tables.len(), 1);
        // Editing a standard table keeps it standard.
        let mut edited = db.tables[0].clone();
        entries_mut(&mut edited)[0].weight = 5;
        edited.flag_0x4 = 0;
        db.replace(0, edited).unwrap();
        assert!(is_standard(&db.tables[0]));
        assert!(db.delete(0).is_err());
        db.restore(0).unwrap();
        assert_eq!(entries(&db.tables[0])[0].weight, 20);

        let mut table = TableDb::new_table();
        table.name = "  ".into();
        assert_eq!(db.add(table.clone()), Err(EMPTY_NAME.into()));
        table.name = "Udb table".into();
        push_entry(&mut table, new_entry("Udb sword", false, 3));
        push_entry(&mut table, new_entry("DMG 88", true, 1));
        assert_eq!(ranges(&table), ["1-3", "4"]);
        assert_eq!(db.add(table), Ok(1));
        db.save(&t.install).unwrap();
        let reread = TableDb::load(&t.install).unwrap();
        assert_eq!(reread.tables.len(), 2);
        assert_eq!(origin(&reread.tables[1]), Origin::Custom);
        assert_eq!(source_names().len(), 17);
    }
}
