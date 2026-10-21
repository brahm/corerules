//! Treasure tables and saved treasures.
//!
//! * `Database/Treasure.dat` and `UserDbas/Treasure.dat` (`CTreasureTablesDoc`):
//!   a `CObList` of [`TreasureTable`] (`CTreasureSingleTableOb`).
//! * `Database/TreaBun.dat` and `UserDbas/TreaBunU.dat` (`CTreasureDoc`): a
//!   `CObList` of [`Treasure`] (`CTreasureOb`); both ship empty.
//!
//! All of them are read with [`ObjectList`](crate::database::ObjectList).

use crate::inventory::{InventoryBundle, InventoryItem};
use crate::{Object, record};

record! {
    /// One treasure table (`CTreasureSingleTableOb`, schema 1, Serialize
    /// 0x546175).
    pub struct TreasureTable {
        /// `DMG 88: Magical Items`.
        name: String,
        /// A `BOOL` stored as a byte; 1 in every shipped table.
        flag_0x4: u8,
        /// The rows, `CTreasureTableEntry` objects.
        entries: Vec<Object>,
    }
}

record! {
    /// One row of a treasure table (`CTreasureTableEntry`, schema 1,
    /// Serialize 0x54963d). Names of the dice fields come from the DMG
    /// tables the rows reproduce.
    pub struct TreasureTableEntry {
        /// Relative chance; the weights of a table sum to 20, 100, 1296, ...
        weight: u32,
        /// 1 exactly when `item.name` names another table.
        is_table: u8,
        /// The resulting item, or the sub-table to roll on (inline
        /// `CInventoryItem`, no class tag).
        item: InventoryItem,
        /// Quantity as dice count, sides and bonus (`Dust of Appearance`:
        /// 5d10; 0d0+1 for a single item).
        quantity_dice: [u32; 3],
        /// Rolls on `sub_table_1`, as dice count, sides and bonus (`Rod of
        /// Absorption`: 0d0+1).
        sub_table_1_dice: [u32; 3],
        /// `DMG Treasure: Wand Charges`.
        sub_table_1: String,
        /// Rolls on `sub_table_2` (`Necklace of Prayer Beads`: 1d4+2).
        sub_table_2_dice: [u32; 3],
        sub_table_2: String,
        /// 25 on swords, 5 on other weapons, else 0: the percent chance of
        /// an intelligent weapon (0x5462c7).
        intelligent_chance: u8,
        /// Spell scrolls: lowest and highest spell level (1..4, 4..9) and
        /// the number of spells (1..7).
        spell_min_level: u32,
        spell_max_level: u32,
        spell_count: u32,
        /// 1 on wizard spell scroll rows, 0 on priest rows.
        spell_wizard: u8,
    }
}

record! {
    /// A saved treasure (`CTreasureOb`, schema 95, Serialize 0x53fc98).
    pub struct Treasure {
        name: String,
        /// Inline `CInventoryBundle`, no class tag.
        bundle: InventoryBundle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::ObjectList;

    fn item(name: &str) -> InventoryItem {
        InventoryItem {
            name: name.into(),
            quantity: 1,
            contents: None,
            unknown_0x8: 999,
            flag_0x4: 0,
            extra: None,
        }
    }

    fn entry(name: &str) -> Object {
        Object::TreasureTableEntry(Box::new(TreasureTableEntry {
            weight: 10,
            is_table: 0,
            item: item(name),
            quantity_dice: [0, 0, 1],
            sub_table_1_dice: [0, 0, 0],
            sub_table_1: String::new(),
            sub_table_2_dice: [1, 4, 2],
            sub_table_2: "Beads".into(),
            intelligent_chance: 0,
            spell_min_level: 0,
            spell_max_level: 0,
            spell_count: 0,
            spell_wizard: 0,
        }))
    }

    #[test]
    fn table_rows_share_one_class_tag() {
        let list = ObjectList {
            objects: vec![Object::TreasureTable(TreasureTable {
                name: "T".into(),
                flag_0x4: 1,
                entries: vec![entry("A"), entry("B")],
            })],
        };
        let bytes = list.to_bytes().unwrap();
        // The second row refers to the CTreasureTableEntry class by index.
        assert!(bytes.windows(2).any(|w| w == [0x03, 0x80]));
        assert_eq!(ObjectList::parse(&bytes).unwrap(), list);
    }

    #[test]
    fn saved_treasure_holds_an_inline_bundle() {
        let t = Object::Treasure(Treasure {
            name: "Hoard".into(),
            bundle: InventoryBundle {
                items: vec![item("Gem")],
            },
        });
        let list = ObjectList { objects: vec![t] };
        let bytes = list.to_bytes().unwrap();
        assert_eq!(ObjectList::parse(&bytes).unwrap(), list);
    }
}
