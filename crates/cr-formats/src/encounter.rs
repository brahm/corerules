//! Encounters: templates (`Database/EncPlat.dat`, `CEncTemplateOb`), saved
//! encounters (`Database/Encount.dat`, `UserDbas/EncountU.dat`,
//! `CEncounterOb`), each a `CObList` read with
//! [`ObjectList`](crate::database::ObjectList), and the frequency tables
//! of `UserDbas/EncTabU.dat` ([`FrequencyTables`]).
//!
//! No class here reads a version. Embedded members (a monster, a monster
//! type, a part, a character, an inventory) are written inline with no
//! class tag. Unnamed fields keep their member offset.

use crate::character::Character;
use crate::field::Field;
use crate::inventory::InventoryBundle;
use crate::monster::{Monster, MonsterType};
use crate::parts::Part;
use crate::{ArchiveReader, ArchiveWriter, Object, Result, record};

record! {
    /// An encounter template (`CEncTemplateOb`, schema 88, size 0x33c,
    /// Serialize 0x648648).
    pub struct EncounterTemplate {
        /// "Orc, Wandering"; unique.
        name: String,
        /// Always 0.
        unknown_0x2d0: i32,
        /// Frequency in percent (65/20/11/4, 0 = exempt from the 2-20
        /// tables), as in [`Monster`].
        frequency: i32,
        /// "Number Of Hit Dice" of the 2-20 tables, 0 to 28.
        hit_dice: i32,
        /// "Human", "Mammal".
        monster_type: String,
        /// The description copied into each encounter.
        notes: String,
        /// "Any Land".
        climate_terrain: String,
        /// One-hot (+0x328, +0x32c, +0x330): both lair and wandering, lair
        /// only, wandering only.
        placement: [i32; 3],
        /// `CEncTemplateMonsterPropertiesOb`s.
        monsters: Vec<Object>,
        /// `CEncTemplateNPCPropertiesOb`s.
        npcs: Vec<Object>,
        /// `CTrapOb`s.
        traps: Vec<Object>,
        /// `CEncTemplateItemPropertiesOb`s.
        items: Vec<Object>,
        /// Lair treasure: how many of each treasure type A to Z.
        lair_treasure: [i32; 26],
        /// Climate (0-4) and terrain (5-36) flags, as `Monster::unknown_0x4`.
        climates: [i32; 150],
    }
}

record! {
    /// A monster entry of a template (`CEncTemplateMonsterPropertiesOb`,
    /// schema 88, size 0x7c, Serialize 0x651f4a).
    pub struct TemplateMonsterProperties {
        /// Number appearing as N d S * M + B (+0x50 to +0x5c), then the
        /// second box (+0x60 to +0x6c).
        dice: [i32; 8],
        /// 0/1 flags at +0x38, +0x3c, +0x40: dependent, "and", "or".
        flags: [i32; 3],
        /// Percent chance; 100 rolls nothing.
        percent: i32,
        /// "for every [n]"; 0 is a one-time addition.
        per: i32,
        /// "beyond [n]".
        beyond: i32,
        /// "Ogre", "Orc, Leader".
        monster_name: String,
        /// The monster of the second box ("Orc assistant").
        second_name: String,
        /// The monster a dependent line counts ("Kuo-Toa"), when
        /// `monster_type` has no name.
        reference_name: String,
        /// The monster type a dependent line counts; usually all empty,
        /// else e.g. "Orc".
        monster_type: MonsterType,
        /// "randomly selected from the following list": full `CMonsterOb`
        /// copies (2207 in EncPlat.dat).
        monsters: Vec<Object>,
    }
}

record! {
    /// An NPC entry of a template (`CEncTemplateNPCPropertiesOb`, schema 88,
    /// size 0x104, Serialize 0x65295d).
    pub struct TemplateNpcProperties {
        /// Number appearing as N d S * M + B (+0xd4 to +0xe0), then the
        /// second box (+0xe4 to +0xf0).
        dice: [i32; 8],
        /// 0/1 flags at +0xc8, +0xcc, +0xd0: dependent, "and", "or".
        flags: [i32; 3],
        /// Percent chance; mostly 100.
        percent: i32,
        /// "for every [n]".
        per: i32,
        /// "beyond [n]".
        beyond: i32,
        /// The monster a dependent line counts ("Elf", "Dwarf, Hill"),
        /// when `monster_type` has no name.
        reference_name: String,
        /// The monster type a dependent line counts ("Halfling", "Gnome").
        monster_type: MonsterType,
        /// The NPC of the first box.
        npc: EncounterNpc,
        /// The NPC of the second box; often empty strings with levels -1.
        second_npc: EncounterNpc,
        /// The random list: `CEncounterNPCOb`s (571 in EncPlat.dat).
        npcs: Vec<Object>,
    }
}

record! {
    /// An item entry of a template (`CEncTemplateItemPropertiesOb`, schema
    /// 88, size 0xe18, Serialize 0x650cde).
    pub struct TemplateItemProperties {
        /// Percent chance.
        percent: i32,
        /// Number of items N d S * M + B (+0xdb0 to +0xdbc), the coin
        /// (+0xdc0: 0 platinum, 1 gold, 2 silver, 3 copper) and the coins
        /// N d S * M + B (+0xdc4 to +0xdd0).
        dice: [i32; 9],
        /// Dependent (+0xda4), "for every" (+0xda8), "beyond" (+0xdac).
        dependency: [i32; 3],
        /// The monster a dependent item counts, by name.
        monster: Box<Monster>,
        /// The monster type a dependent item counts.
        monster_type: MonsterType,
        /// The item ("Catapult, Heavy", "Cart").
        part: Box<Part>,
        /// Monsters whose number the item doubles ("Orc, Leader").
        monster_names: Vec<String>,
        /// `CEncTemplateNumAppearOb`s that appear with the items.
        escorts: Vec<Object>,
        /// `CEncTemplateNumAppearOb`s that appear for each item.
        per_item: Vec<Object>,
    }
}

record! {
    /// `CEncTemplateNumAppearOb` (schema 88, size 0x4fc, Serialize
    /// 0x6517d3).
    pub struct TemplateNumAppear {
        /// Number appearing as N d S * M + B (1d6, 10d6, 1d1+9).
        dice: [i32; 4],
        monster: Box<Monster>,
    }
}

record! {
    /// An NPC description (`CEncounterNPCOb`, schema 95, size 0x48,
    /// Serialize 0x6531e0).
    pub struct EncounterNpc {
        /// "Bob", "Norm".
        name: String,
        /// "Human", "Elf".
        race: String,
        /// "Fighter", then further classes or empty.
        classes: [String; 5],
        /// Level in each class; -1 = none.
        levels: [i32; 5],
        /// `CPartIDOb`s; empty in all shipped data.
        unknown_0x30: Vec<Object>,
    }
}

record! {
    /// A saved encounter (`CEncounterOb`, schema 88, size 0x194, Serialize
    /// 0x63c659).
    pub struct Encounter {
        /// "Wandering Bugbear", "Huge Orc City".
        name: String,
        /// Always 0.
        unknown_0x7c: i32,
        /// "Bugbear", "Orc"; "~All" when made from the "~All" list.
        monster_type: String,
        /// Name of the template it came from ("Orc, Lair (Huge city)").
        template: String,
        description: String,
        /// `CEncounterMonsterPackageOb`s.
        monsters: Vec<Object>,
        /// `CEncounterNPCPackageOb`s.
        npcs: Vec<Object>,
        /// `CTrapOb`s.
        traps: Vec<Object>,
        treasure: InventoryBundle,
        items: InventoryBundle,
        /// The template's lair treasure types, A to Z.
        lair_treasure: [i32; 26],
    }
}

record! {
    /// The monsters of one kind in an encounter
    /// (`CEncounterMonsterPackageOb`, schema 88, size 0x504, Serialize
    /// 0x6405b8).
    pub struct MonsterPackage {
        /// How many (1, 5, 544, 1081).
        count: i32,
        monster: Box<Monster>,
        /// `CEncounterMonHpOb`s, one per distinct hit point value.
        hit_points: Vec<Object>,
    }
}

record! {
    /// Monsters of a [`MonsterPackage`] with the same hit points
    /// (`CEncounterMonHpOb`, schema 88, size 0xc, Serialize 0x650612).
    pub struct MonsterHp {
        hit_points: i32,
        /// How many have them.
        count: i32,
    }
}

record! {
    /// NPCs in an encounter (`CEncounterNPCPackageOb`, schema 88, size
    /// 0x21a4, 2.0 0x2028, Serialize 0x63bc0f); not in the shipped data.
    pub struct NpcPackage {
        /// How many.
        count: i32,
        /// A whole character, stored as in a `Chars/<id>.dat` file
        /// (`TSRP3 V54` or `TSRP2 V29`); every individual shares it.
        character: Box<Character>,
        /// `CPartIDOb`s, one per individual, with the hit points in
        /// `part_type`.
        hit_points: Vec<Object>,
    }
}

record! {
    /// One frequency table (`CEncounterFreqTableOb`, schema 1, size 0x28c,
    /// Serialize 0x63ff92); not in the shipped data.
    pub struct FrequencyTable {
        name: String,
        /// The template names for the rolls 2 to 20.
        slots: Vec<String>,
        table: FrequencyTableData,
    }
}

record! {
    /// The object embedded at `CEncounterFreqTableOb + 0x4` (size 0x270,
    /// Serialize 0x64fe45). Constructor values in the field docs.
    pub struct FrequencyTableData {
        /// Hit dice range of the filter, 0 to 100.
        hit_dice_min: i32,
        hit_dice_max: i32,
        /// Templates: 0 all, 1 standard, 2 custom.
        source: i32,
        /// Climate/terrain category; "Any" takes every template.
        category: String,
        /// 0 lair and wandering, 1 lair or wandering, 2 wandering, 3 lair.
        lair: i32,
        /// Climate and terrain flags, all 1 from the constructor.
        climates: [i32; 150],
    }
}

/// `UserDbas/EncTabU.dat` (frequency-table document, Serialize 0x63ef52).
#[derive(Debug, Clone, PartialEq)]
pub struct FrequencyTables {
    /// The program writes `TSRP3/FreqDoc/0001`; the shipped file has
    /// `TSRP2/FreqDoc/0001`. Any other value only raises a warning.
    pub version: String,
    /// `CEncounterFreqTableOb`s.
    pub tables: Vec<Object>,
}

impl FrequencyTables {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = ArchiveReader::new(data);
        let version = r.read_string()?;
        let tables = Field::read(&mut r)?;
        r.finish()?;
        Ok(Self { version, tables })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut w = ArchiveWriter::new();
        w.write_string(&self.version)?;
        self.tables.write(&mut w)?;
        Ok(w.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_tables_round_trip() {
        let doc = FrequencyTables {
            version: "TSRP3/FreqDoc/0001".into(),
            tables: vec![Object::FrequencyTable(Box::new(FrequencyTable {
                name: "Forest".into(),
                slots: vec!["Orc".into()],
                table: FrequencyTableData {
                    hit_dice_min: 0,
                    hit_dice_max: 100,
                    source: 0,
                    category: String::new(),
                    lair: 0,
                    climates: [1; 150],
                },
            }))],
        };
        let bytes = doc.to_bytes().unwrap();
        // Version string, count 1, then a new-class tag for the table.
        assert_eq!(&bytes[..19], b"\x12TSRP3/FreqDoc/0001");
        assert_eq!(&bytes[19..25], b"\x01\x00\xff\xff\x01\x00");
        assert_eq!(FrequencyTables::parse(&bytes).unwrap(), doc);
    }

    #[test]
    fn empty_frequency_tables() {
        let data = b"\x12TSRP2/FreqDoc/0001\x00\x00";
        let doc = FrequencyTables::parse(data).unwrap();
        assert!(doc.tables.is_empty());
        assert_eq!(doc.to_bytes().unwrap(), data);
    }

    #[test]
    fn npc_round_trips() {
        let npc = EncounterNpc {
            name: "Bob".into(),
            race: "Human".into(),
            classes: [
                "Fighter".into(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            levels: [3, -1, -1, -1, -1],
            unknown_0x30: vec![],
        };
        let mut w = ArchiveWriter::new();
        npc.write(&mut w).unwrap();
        let bytes = w.into_bytes();
        let mut r = ArchiveReader::new(&bytes);
        assert_eq!(EncounterNpc::read(&mut r).unwrap(), npc);
        assert!(r.is_at_end());
    }

    /// An NPC package embeds a whole character file; class tags inside it
    /// continue the numbering of the surrounding archive.
    #[test]
    fn npc_package_embeds_a_character() {
        use crate::database::ObjectList;
        const BORIN: &[u8] = include_bytes!("../../../tests/fixtures/characters/90688519.dat");
        let character = Character::parse(BORIN).unwrap();
        let empty = InventoryBundle { items: vec![] };
        let encounter = Encounter {
            name: "Wandering Dwarf".into(),
            unknown_0x7c: 0,
            monster_type: "Dwarf".into(),
            template: String::new(),
            description: String::new(),
            monsters: vec![],
            npcs: vec![Object::NpcPackage(Box::new(NpcPackage {
                count: 1,
                character: Box::new(character),
                hit_points: vec![],
            }))],
            traps: vec![],
            treasure: empty.clone(),
            items: empty,
            lair_treasure: [0; 26],
        };
        let list = ObjectList {
            objects: vec![Object::Encounter(Box::new(encounter))],
        };
        let bytes = list.to_bytes().unwrap();
        assert_eq!(ObjectList::parse(&bytes).unwrap(), list);
    }
}
