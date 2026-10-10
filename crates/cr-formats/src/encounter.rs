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
        /// Frequency in percent (65/20/11/4), as in [`Monster`].
        frequency: i32,
        /// 0 to 28.
        unknown_0x338: i32,
        /// "Human", "Mammal".
        monster_type: String,
        notes: String,
        /// "Any Land".
        climate_terrain: String,
        /// 0/1 flags at +0x328, +0x32c, +0x330.
        unknown_0x328: [i32; 3],
        /// `CEncTemplateMonsterPropertiesOb`s.
        monsters: Vec<Object>,
        /// `CEncTemplateNPCPropertiesOb`s.
        npcs: Vec<Object>,
        /// `CTrapOb`s.
        traps: Vec<Object>,
        /// `CEncTemplateItemPropertiesOb`s.
        items: Vec<Object>,
        unknown_0x4: [i32; 26],
        /// Same shape as `Monster::unknown_0x4`.
        unknown_0x6c: [i32; 150],
    }
}

record! {
    /// A monster entry of a template (`CEncTemplateMonsterPropertiesOb`,
    /// schema 88, size 0x7c, Serialize 0x651f4a).
    pub struct TemplateMonsterProperties {
        /// Members +0x50 to +0x6c; the first two look like a min/max count
        /// (1/4, 1/6, 1/10).
        unknown_0x50: [i32; 8],
        /// 0/1 flags at +0x38, +0x3c, +0x40.
        unknown_0x38: [i32; 3],
        /// Mostly 100.
        unknown_0x70: i32,
        unknown_0x74: i32,
        unknown_0x78: i32,
        /// "Ogre", "Orc, Leader".
        monster_name: String,
        /// Empty or a second name ("Orc assistant").
        second_name: String,
        /// Empty or a third name ("Kuo-Toa").
        third_name: String,
        /// Usually all empty; else e.g. "Orc".
        monster_type: MonsterType,
        /// Full `CMonsterOb` copies (2207 in EncPlat.dat).
        monsters: Vec<Object>,
    }
}

record! {
    /// An NPC entry of a template (`CEncTemplateNPCPropertiesOb`, schema 88,
    /// size 0x104, Serialize 0x65295d).
    pub struct TemplateNpcProperties {
        /// Members +0xd4 to +0xf0.
        unknown_0xd4: [i32; 8],
        /// 0/1 flags at +0xc8, +0xcc, +0xd0.
        unknown_0xc8: [i32; 3],
        /// Mostly 100.
        unknown_0xf4: i32,
        unknown_0xf8: i32,
        unknown_0xfc: i32,
        /// Empty or a race ("Elf", "Dwarf, Hill").
        race: String,
        /// "Halfling", "Gnome".
        monster_type: MonsterType,
        unknown_0x18: EncounterNpc,
        /// Often empty strings with levels -1.
        unknown_0x60: EncounterNpc,
        /// `CEncounterNPCOb`s (571 in EncPlat.dat).
        npcs: Vec<Object>,
    }
}

record! {
    /// An item entry of a template (`CEncTemplateItemPropertiesOb`, schema
    /// 88, size 0xe18, Serialize 0x650cde).
    pub struct TemplateItemProperties {
        unknown_0xda0: i32,
        /// Members +0xdb0 to +0xdd0.
        unknown_0xdb0: [i32; 9],
        /// Members +0xda4 to +0xdac.
        unknown_0xda4: [i32; 3],
        monster: Box<Monster>,
        monster_type: MonsterType,
        /// The item ("Catapult, Heavy", "Cart").
        part: Box<Part>,
        /// "Orc, Leader", "Orc assistant".
        monster_names: Vec<String>,
        /// `CEncTemplateNumAppearOb`s.
        unknown_0xde8: Vec<Object>,
        /// `CEncTemplateNumAppearOb`s.
        unknown_0xdfc: Vec<Object>,
    }
}

record! {
    /// `CEncTemplateNumAppearOb` (schema 88, size 0x4fc, Serialize
    /// 0x6517d3).
    pub struct TemplateNumAppear {
        /// 1, 5, 10.
        unknown_0x4ec: i32,
        /// 1, 6.
        unknown_0x4f0: i32,
        unknown_0x4f4: i32,
        /// 0, 9.
        unknown_0x4f8: i32,
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
        unknown_0x7c: i32,
        /// "Bugbear", "Orc".
        monster_type: String,
        /// Name of the template it came from ("Orc, Lair (Huge city)").
        template: String,
        unknown_0x78: String,
        /// `CEncounterMonsterPackageOb`s.
        monsters: Vec<Object>,
        /// `CEncounterNPCPackageOb`s.
        npcs: Vec<Object>,
        /// `CTrapOb`s.
        traps: Vec<Object>,
        treasure: InventoryBundle,
        items: InventoryBundle,
        unknown_0x4: [i32; 26],
    }
}

record! {
    /// The monsters of one kind in an encounter
    /// (`CEncounterMonsterPackageOb`, schema 88, size 0x504, Serialize
    /// 0x6405b8).
    pub struct MonsterPackage {
        /// 1, 5, 544, 1081.
        unknown_0x4ec: i32,
        monster: Box<Monster>,
        /// `CEncounterMonHpOb`s, one per individual monster.
        individuals: Vec<Object>,
    }
}

record! {
    /// One monster of a [`MonsterPackage`] (`CEncounterMonHpOb`, schema 88,
    /// size 0xc, Serialize 0x650612).
    pub struct MonsterHp {
        /// 1 to 22.
        unknown_0x4: i32,
        /// 1 to 146; probably hit points.
        unknown_0x8: i32,
    }
}

record! {
    /// An NPC in an encounter (`CEncounterNPCPackageOb`, schema 88, size
    /// 0x21a4, Serialize 0x63bc0f); not in the shipped data.
    pub struct NpcPackage {
        unknown_0x218c: i32,
        /// A whole character, stored as in a `Chars/<id>.dat` file.
        character: Box<Character>,
        /// Element class not determined.
        unknown_0x2190: Vec<Object>,
    }
}

record! {
    /// One frequency table (`CEncounterFreqTableOb`, schema 1, size 0x28c,
    /// Serialize 0x63ff92); not in the shipped data.
    pub struct FrequencyTable {
        unknown_0x274: String,
        unknown_0x278: Vec<String>,
        table: FrequencyTableData,
    }
}

record! {
    /// The object embedded at `CEncounterFreqTableOb + 0x4` (size 0x270,
    /// Serialize 0x64fe45). Constructor values in the field docs.
    pub struct FrequencyTableData {
        /// 0.
        unknown_0x260: i32,
        /// 100.
        unknown_0x264: i32,
        /// 0.
        unknown_0x268: i32,
        unknown_0x25c: String,
        /// 0.
        unknown_0x26c: i32,
        /// All 1.
        unknown_0x4: [i32; 150],
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
                unknown_0x274: "Forest".into(),
                unknown_0x278: vec!["Orc".into()],
                table: FrequencyTableData {
                    unknown_0x260: 0,
                    unknown_0x264: 100,
                    unknown_0x268: 0,
                    unknown_0x25c: String::new(),
                    unknown_0x26c: 0,
                    unknown_0x4: [1; 150],
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
            unknown_0x78: String::new(),
            monsters: vec![],
            npcs: vec![Object::NpcPackage(Box::new(NpcPackage {
                unknown_0x218c: 1,
                character: Box::new(character),
                unknown_0x2190: vec![],
            }))],
            traps: vec![],
            treasure: empty.clone(),
            items: empty,
            unknown_0x4: [0; 26],
        };
        let list = ObjectList {
            objects: vec![Object::Encounter(Box::new(encounter))],
        };
        let bytes = list.to_bytes().unwrap();
        assert_eq!(ObjectList::parse(&bytes).unwrap(), list);
    }
}
