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
use cr_i18n::{tr, tr_noop};

use super::{Origin, parts};
use crate::{Error, Installation, Result};

pub const USER_FILE: &str = "UserDbas/Treasure.dat";
pub const STANDARD_FILE: &str = "Database/Treasure.dat";

/// The message for an empty name.
pub const EMPTY_NAME: &str = tr_noop!("You must specify a name for the table.");
/// The name a new table starts with.
pub const NEW_TABLE: &str = tr_noop!("New Table");
pub const DELETE_WARNING: &str =
    tr_noop!("This will delete the selected table.  You will lose any customizations.");
pub const RESTORE_WARNING: &str = tr_noop!(
    "This will revert the selected table to match the Dungeon Master's Guide. You will lose any customizations."
);

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
            name: tr(NEW_TABLE).into(),
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
            return Err(tr(EMPTY_NAME).into());
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
            return Err(tr("Standard tables cannot be deleted; restore them instead.").into());
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
            .ok_or_else(|| tr!("{name} is not in the Dungeon Master's Guide tables.", name))?;
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
        intelligent_chance: 0,
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

/// The table names a row's Random Contents can roll (IDD 262, 0x54d025):
/// every table of the database, sorted.
pub fn table_names(db: &TableDb) -> Vec<String> {
    let mut names: Vec<String> = db.tables.iter().map(|t| t.name.clone()).collect();
    names.sort_by_cached_key(|n| crate::collate::sort_key(n));
    names.dedup();
    names
}

/// Where Always Contains picks its parts: [`source_names`] without
/// "Tables" (IDD 262's category combo, 1881).
pub fn contents_sources() -> Vec<String> {
    source_names()
        .into_iter()
        .filter(|s| s != "Tables")
        .collect()
}

/// The Always Contains items of a row: the top level of its item's
/// contents, in order.
pub fn contained(entry: &TreasureTableEntry) -> &[InventoryItem] {
    match entry.item.contents.as_deref() {
        Some(Object::InventoryBundle(b)) => &b.items,
        _ => &[],
    }
}

/// How the Always Contains list shows an item (0x4aadba): its name, with
/// " xN" when there are several.
pub fn contained_line(item: &InventoryItem) -> String {
    if item.quantity == 1 {
        item.name.clone()
    } else {
        format!("{} x{}", item.name, item.quantity)
    }
}

/// Add (0x54a794): one more of `name` in `items`, added to an item of the
/// same name that holds nothing.
pub fn add_contained(items: &mut Vec<InventoryItem>, name: &str) {
    if let Some(item) = items
        .iter_mut()
        .find(|i| i.name == name && i.contents.is_none() && i.extra.is_none())
    {
        item.quantity = item.quantity.saturating_add(1);
        return;
    }
    items.push(InventoryItem {
        name: name.into(),
        quantity: 1,
        contents: None,
        unknown_0x8: 999,
        flag_0x4: 0,
        extra: None,
    });
}

/// Remove (0x54a5b5): `count` of the item at `index`, the item itself
/// when that is all of it.
pub fn remove_contained(items: &mut Vec<InventoryItem>, index: usize, count: u32) {
    let Some(item) = items.get_mut(index) else {
        return;
    };
    if count >= item.quantity {
        items.remove(index);
    } else {
        item.quantity -= count;
    }
}

/// OK on the entry details (0x54a84c): the item contains `items`. It gets
/// a contents bundle even when that is empty, as in the original, which
/// keeps rolled copies of it from merging.
pub fn set_contained(entry: &mut TreasureTableEntry, items: Vec<InventoryItem>) {
    entry.item.contents = Some(Box::new(Object::InventoryBundle(
        cr_formats::inventory::InventoryBundle { items },
    )));
}

/// The highest spell level of a scroll row: 9 for wizard spells, 7 for
/// priest spells (IDD 365, 0x53e0f6).
pub fn max_spell_level(wizard: bool) -> u32 {
    if wizard { 9 } else { 7 }
}

/// Wizard or Priest in Scroll Details (0x53e1ee, 0x53e21d): choosing
/// priest spells lowers both levels to 7 at most.
pub fn set_spell_wizard(entry: &mut TreasureTableEntry, wizard: bool) {
    entry.spell_wizard = u8::from(wizard);
    if !wizard {
        entry.spell_min_level = entry.spell_min_level.min(7);
        entry.spell_max_level = entry.spell_max_level.min(7);
    }
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
    tr_noop!("Armor"),
    tr_noop!("Equipment"),
    tr_noop!("Gems and Art"),
    tr_noop!("Magical Items"),
    tr_noop!("Weapons"),
];

/// What a category's source takes: every item, the user's or the
/// standard ones.
const WHICH: [&str; 3] = [tr_noop!("All"), tr_noop!("Custom"), tr_noop!("Standard")];

/// What the source combo can show: each category × All/Custom/Standard,
/// then Count and Tables.
pub fn source_names() -> Vec<String> {
    let mut names = Vec::new();
    for category in CATEGORIES {
        for which in WHICH {
            names.push(format!("{category}, {which}"));
        }
    }
    names.push(tr_noop!("Count").into());
    names.push(tr_noop!("Tables").into());
    names
}

/// How the editor shows `source`, one of [`source_names`], translated:
/// "Armor, Custom" as each of its words is.
pub fn source_label(source: &str) -> String {
    match source.split_once(", ") {
        Some((category, which)) => format!("{}, {}", tr(category), tr(which)),
        None => tr(source).to_owned(),
    }
}

/// The items the source list shows for `source` (one of
/// [`source_names`]), sorted; the flag says whether they are tables.
pub fn source_items(
    install: &Installation,
    db: &TableDb,
    source: &str,
) -> Result<(Vec<String>, bool)> {
    if source == "Tables" {
        return Ok(items_of(&[], db, source));
    }
    Ok(items_of(&parts::names_by_origin(install)?, db, source))
}

/// What every source shows, as [`source_items`] gives it, the parts read
/// once: each of [`source_names`] with its items and whether they are
/// tables.
pub fn all_source_items(
    install: &Installation,
    db: &TableDb,
) -> Result<Vec<(String, Vec<String>, bool)>> {
    let by_origin = parts::names_by_origin(install)?;
    Ok(source_names()
        .into_iter()
        .map(|source| {
            let (items, tables) = items_of(&by_origin, db, &source);
            (source, items, tables)
        })
        .collect())
}

/// [`source_items`] with the parts' names by origin already read.
fn items_of(
    by_origin: &[Vec<(String, Origin)>],
    db: &TableDb,
    source: &str,
) -> (Vec<String>, bool) {
    if source == "Tables" {
        let mut names: Vec<String> = db.tables.iter().map(|t| t.name.clone()).collect();
        names.sort_by_cached_key(|n| crate::collate::sort_key(n));
        names.dedup();
        return (names, true);
    }
    let (category, which) = source.split_once(", ").unwrap_or((source, "All"));
    let Some(names) = parts::source_type(category).and_then(|t| by_origin.get(t)) else {
        return (Vec::new(), false);
    };
    let mut names: Vec<String> = names
        .iter()
        .filter(|(_, o)| match which {
            "Custom" => *o == Origin::Custom,
            "Standard" => *o == Origin::Standard,
            _ => true,
        })
        .map(|(n, _)| n.clone())
        .collect();
    names.sort_by_cached_key(|n| crate::collate::sort_key(n));
    names.dedup();
    (names, false)
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

    #[test]
    fn entry_details_edit_the_contents_and_the_scroll() {
        let mut entry = new_entry("Bag", false, 1);
        assert!(contained(&entry).is_empty());
        assert!(!contents_sources().contains(&"Tables".to_string()));
        assert_eq!(contents_sources().len(), source_names().len() - 1);

        // Add merges by name; Remove takes some or all.
        let mut items = Vec::new();
        add_contained(&mut items, "Torch");
        add_contained(&mut items, "Rope");
        add_contained(&mut items, "Torch");
        assert_eq!(items.len(), 2);
        assert_eq!(contained_line(&items[0]), "Torch x2");
        assert_eq!(contained_line(&items[1]), "Rope");
        remove_contained(&mut items, 0, 1);
        assert_eq!(items[0].quantity, 1);
        remove_contained(&mut items, 0, 1);
        assert_eq!(items.len(), 1);

        // OK always leaves a contents bundle, even an empty one.
        set_contained(&mut entry, Vec::new());
        assert!(entry.item.contents.is_some());
        assert!(contained(&entry).is_empty());
        set_contained(&mut entry, items);
        assert_eq!(contained(&entry).len(), 1);

        // Priest spells go up to 7th level.
        entry.spell_min_level = 4;
        entry.spell_max_level = 9;
        set_spell_wizard(&mut entry, true);
        assert_eq!((entry.spell_wizard, entry.spell_max_level), (1, 9));
        set_spell_wizard(&mut entry, false);
        assert_eq!(
            (
                entry.spell_wizard,
                entry.spell_min_level,
                entry.spell_max_level
            ),
            (0, 4, 7)
        );
        assert_eq!((max_spell_level(true), max_spell_level(false)), (9, 7));
    }
}
