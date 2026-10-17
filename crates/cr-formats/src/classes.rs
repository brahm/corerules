//! Classes: `Database/Class.dat` and `UserDbas/ClassU.dat`, a `CObList` of
//! `CClassOb` (schema 88, `CClassOb::Serialize` at 0x6644af in TSRP3.EXE).
//!
//! In the Expansion each class starts with its own version string, `TSRP3
//! V39`; older layouts lack the fields their version predates, with the
//! conditions copied from the load code. Core Rules 2.0 (`EVERMORE.EXE`,
//! `CClassOb::Serialize` at 0x48c090) writes no version string and a
//! shorter, fixed layout, [`Core20ClassData`]. Unnamed fields keep the
//! member offset they have in the original object (2.0 offsets for the 2.0
//! layout).

use crate::field::Field;
use crate::{ArchiveReader, ArchiveWriter, Object, Result, record};

/// The class layout the Expansion writes.
pub const CURRENT_VERSION: i32 = 39;

/// The class version a version string announces, the way the original
/// reads it: the current version on an exact match, otherwise the number
/// after the first seven characters, or the version before the current one
/// when there is nothing after them.
pub fn version_of(version_string: &str) -> i32 {
    if version_string == format!("TSRP3 V{CURRENT_VERSION}") {
        CURRENT_VERSION
    } else if version_string.len() > 7 {
        crate::atol(version_string.get(7..).unwrap_or(""))
    } else {
        CURRENT_VERSION - 1
    }
}

/// One class (`CClassOb`), in the layout of the program that wrote it.
#[derive(Debug, Clone, PartialEq)]
// Always boxed inside `Object`; boxing the variants too would only add
// indirection.
#[allow(clippy::large_enum_variant)]
pub enum Class {
    Expansion {
        /// `TSRP3 V39` for files the Expansion wrote.
        version: String,
        data: ClassData,
    },
    Core20(Core20ClassData),
}

impl Class {
    pub fn name(&self) -> &str {
        match self {
            Self::Expansion { data, .. } => &data.name,
            Self::Core20(data) => &data.name,
        }
    }

    /// Removes the [`SpellPoints`] objects, which the Expansion cannot
    /// read; returns how many there were.
    pub fn drop_spell_points(&mut self) -> usize {
        let arrays: Vec<&mut Vec<Object>> = match self {
            Self::Expansion { data, .. } => [
                &mut data.unknown_0x2f8c,
                &mut data.unknown_0x3514,
                &mut data.kit_bonuses,
                &mut data.unknown_0x4884,
            ]
            .into_iter()
            .chain(data.unknown_0x4acc.iter_mut())
            .collect(),
            Self::Core20(data) => data.unknown_0x26bc.iter_mut().collect(),
        };
        let mut dropped = 0;
        for array in arrays {
            let before = array.len();
            array.retain(|o| !matches!(o, Object::SpellPoints(_)));
            dropped += before - array.len();
        }
        dropped
    }
}

/// Both programs store `CClassOb` under schema 88, so the layout is told
/// apart by the first string: the Expansion's version string starts with
/// `TSRP`, a 2.0 class starts with its name.
impl Field for Class {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let first = r.read_string()?;
        if first.starts_with("TSRP") {
            let data = ClassData::read_fields(r, version_of(&first))?;
            Ok(Self::Expansion {
                version: first,
                data,
            })
        } else {
            Core20ClassData::read_after_name(r, first).map(Self::Core20)
        }
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        match self {
            Self::Expansion { version, data } => {
                w.write_string(version)?;
                data.write_fields(w, version_of(version))
            }
            Self::Core20(data) => {
                if data.name.starts_with("TSRP") {
                    return Err(crate::Error::Unencodable(data.name.clone()));
                }
                data.write(w)
            }
        }
    }
}

record! {
    /// A class as Core Rules 2.0 stores it. The first 33 values and the
    /// experience table match the Expansion's [`ClassData`] in order and
    /// sample values (Fighter: id 1000, Paladin minimum Str 12).
    pub struct Core20ClassData {
        name: String,
        /// Same order as the Expansion's values from `unknown_0x14` to
        /// `unknown_0xcc`.
        unknown_0x10: [i32; 33],
        experience: [i32; 101],
        unknown_0x264: [[i32; 11]; 101],
        unknown_0x13ec: [[i32; 11]; 101],
        unknown_0x8: i32,
        unknown_0x25f0: [i32; 8],
        unknown_0x25d8: String,
        /// Short name shown in multi-class titles ("F", "Pa"), like the
        /// Expansion's `abbreviation`.
        unknown_0x2610: [String; 9],
        unknown_0x2634: [i32; 9],
        unknown_0x25c4: [i32; 10],
        unknown_0x58: [Vec<String>; 12],
        unknown_0x26bc: [Vec<Object>; 10],
    }
}

impl Core20ClassData {
    fn read_after_name(r: &mut ArchiveReader, name: String) -> Result<Self> {
        Ok(Self {
            name,
            unknown_0x10: Field::read(r)?,
            experience: Field::read(r)?,
            unknown_0x264: Field::read(r)?,
            unknown_0x13ec: Field::read(r)?,
            unknown_0x8: Field::read(r)?,
            unknown_0x25f0: Field::read(r)?,
            unknown_0x25d8: Field::read(r)?,
            unknown_0x2610: Field::read(r)?,
            unknown_0x2634: Field::read(r)?,
            unknown_0x25c4: Field::read(r)?,
            unknown_0x58: Field::read(r)?,
            unknown_0x26bc: Field::read(r)?,
        })
    }
}

record! {
    /// The fields of a class after its version string.
    pub struct ClassData(v) {
        name: String,
        unknown_0x14: [i32; 2],
        /// Minimum Str, Dex, Con, Int, Wis, Cha (Paladin: 12, 0, 9, 0, 13, 17).
        ability_requirements: [i32; 6],
        unknown_0x34: [i32; 6],
        unknown_0x4c: i32,
        /// Skills & Powers character points a 1st-level character of the
        /// class spends on class abilities (Fighter 15, Cleric 125), then
        /// the class's maximum level (+0x58, 30 for most classes).
        unknown_0x54: [i32; 2],
        unknown_0x84: [i32; 3],
        unknown_0xb4: [i32; 2],
        /// Database identifier: 1000s warriors, 2000s rogues, 3000s
        /// priests, 4000s wizards, 5000 psionicist (Fighter 1000, Paladin
        /// 1001). `unknown_0x14[0]` holds the group, 0 to 4, in the same
        /// order.
        id: i32,
        unknown_0x50: i32,
        unknown_0xbc: [i32; 3],
        unknown_0xcc: [i32; 6],
        /// Experience points needed for each level, indexed by level.
        experience: [i32; 101],
        unknown_0x27c: Option<[i32; 101]> = if v > 20,
        unknown_0x414: [[i32; 11]; 101],
        unknown_0x159c: [[i32; 11]; 101],
        unknown_0x3050: Option<[i32; 102]> = if v > 21,
        unknown_0x31e8: Option<[i32; 102]> = if v > 21,
        unknown_0x3380: Option<[i32; 101]> = if v > 22,
        unknown_0x48c4: Option<[i32; 101]> = if v > 29,
        /// Twelve per-level values for levels 0 to 100 (members 0x356c,
        /// 0x3700, ... 0x46c8, interleaved by level).
        unknown_0x356c: Option<[[i32; 12]; 101]> = if v > 34,
        flag_0x278c: u8,
        flag_0x278d: Option<u8> = if v > 1,
        flags_0x486e: Option<[u8; 5]> = if v > 2,
        flags_0x278e: Option<[u8; 2]> = if v > 24,
        flag_0x2790: Option<u8> = if v > 27,
        flag_0x2791: Option<u8> = if v > 3,
        flag_0x2792: Option<u8> = if v > 4,
        flags_0x2793: Option<[u8; 2]> = if v > 5,
        flag_0x279d: Option<u8> = if v > 6,
        flags_0x279e: Option<[u8; 2]> = if v > 7,
        flags_0x27a0: Option<[u8; 3]> = if v > 8,
        flag_0x27a4: Option<u8> = if v > 9,
        flags_0x2795: Option<[u8; 2]> = if v > 10,
        flag_0x2797: Option<u8> = if v > 11,
        flag_0x27a1: Option<u8> = if v > 13,
        flag_0x2ffc: Option<u8> = if v > 16,
        flag_0x3554: Option<u8> = if v > 17,
        flags_0x3044: Option<[u8; 5]> = if v > 18,
        flag_0x279c: Option<u8> = if v > 28,
        flags_0x3049: Option<[u8; 6]> = if v > 30,
        flags_0x4864: Option<[u8; 2]> = if v > 31,
        flag_0x486c: Option<u8> = if v > 33,
        unknown_0x27a8: Option<[[i32; 5]; 101]> = if v > 14,
        unknown_0xc: i32,
        unknown_0x2798: i32,
        unknown_0x2ff8: i32,
        unknown_0x3040: i32,
        unknown_0x2fc8: Option<[i32; 2]> = if v > 23,
        unknown_0x3550: i32,
        unknown_0x90: i32,
        unknown_0x4a70: Option<[i32; 12]> = if v > 25,
        unknown_0x485c: [i32; 2],
        unknown_0x94: Option<[i32; 3]> = if v > 32,
        unknown_0xc8: i32,
        /// Specialist school name ("Shadow", "Song"), empty otherwise.
        unknown_0x2788: String,
        /// Short name shown in multi-class titles ("F", "Pa", "R").
        abbreviation: String,
        unknown_0x353c: [String; 3],
        unknown_0x4868: String,
        unknown_0x3548: String,
        unknown_0x8: String,
        unknown_0x4aa0: [String; 2],
        unknown_0x4aa8: [i32; 9],
        unknown_0x2774: [i32; 5],
        unknown_0x4a5c: [i32; 5],
        /// Class abilities offered by Skills & Powers.
        abilities: Vec<String>,
        unknown_0x4870: Option<Vec<String>> = if v > 37,
        unknown_0x70: Vec<String>,
        /// Alignments the class allows.
        alignments: Vec<String>,
        /// Class abilities a new character starts with.
        default_abilities: Vec<String>,
        unknown_0x2738: Vec<String>,
        unknown_0x2760: Vec<String>,
        unknown_0x274c: Vec<String>,
        unknown_0x2fa0: Vec<String>,
        unknown_0x2fcc: Vec<String>,
        unknown_0x2fb4: Option<Vec<String>> = if v > 23,
        unknown_0x2fe0: Option<Vec<String>> = if v > 23,
        unknown_0x3558: Option<Vec<String>> = if v > 26,
        unknown_0x3004: Option<Vec<String>> = if v > 19,
        /// The weapon category combo: "Any" in most shipped classes, "Only
        /// from list below" in a few.
        unknown_0x3000: Option<String> = if v > 19,
        unknown_0x3018: Vec<String>,
        unknown_0x302c: Vec<String>,
        unknown_0x4898: Vec<String>,
        unknown_0x48b0: Option<Vec<String>> = if v > 38,
        unknown_0x2f8c: Vec<Object>,
        /// Before version 36 the program converts the stored objects into
        /// its newer form after loading; the file layout is the same.
        unknown_0x3514: Vec<Object>,
        /// Kit bonuses (`CPartKitBonusOb`); converted after loading below
        /// version 37, same file layout.
        kit_bonuses: Vec<Object>,
        unknown_0x4884: Vec<Object>,
        unknown_0x4acc: [Vec<Object>; 6],
    }
}

record! {
    /// A bonus a kit or class grants (`CPartKitBonusOb`, versionable
    /// schema 1), e.g. Ranger: `Tracking`.
    pub struct KitBonus {
        version: u32,
        name: String,
        unknown_0x8: i32,
        unknown_0xc: i32,
    }
}

/// A spell-like ability a class gets at a level (`CClassFreeSpellsOb`,
/// schema 96, `Serialize` at 0x6b70aa), in the class's `unknown_0x2f8c`.
/// The shipped classes have none; community specialty priests use them
/// ("Detect Evil", 1 "per day").
#[derive(Debug, Clone, PartialEq)]
pub struct ClassFreeSpells {
    /// 4 when the Expansion writes it.
    pub version: i32,
    /// Level the ability comes at.
    pub unknown_0x2c: i32,
    /// How many uses ("1" in "1 per day"); 1 for a new object.
    pub unknown_0x30: i32,
    /// The frequency text ("per day"); from version 3.
    pub unknown_0x34: Option<String>,
    pub flag_0x38: u8,
    pub unknown_0x4: Vec<String>,
    /// From version 2: the spells (`CPartKitBonusOb`, `name` the spell).
    /// Below version 4 the program converts the stored objects after
    /// loading; the file layout is the same.
    pub unknown_0x18: Option<Vec<Object>>,
}

impl Field for ClassFreeSpells {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let version = i32::read(r)?;
        Ok(Self {
            version,
            unknown_0x2c: Field::read(r)?,
            unknown_0x30: Field::read(r)?,
            unknown_0x34: (version > 2).then(|| r.read_string()).transpose()?,
            flag_0x38: Field::read(r)?,
            unknown_0x4: Field::read(r)?,
            unknown_0x18: (version > 1).then(|| Field::read(r)).transpose()?,
        })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        // The program writes version 4 and every member; an object read
        // from an older version is written back in its own layout.
        let gated = |present: bool, expected: bool, field| {
            if present == expected {
                Ok(())
            } else {
                Err(crate::Error::FieldMismatch { field, expected })
            }
        };
        gated(
            self.unknown_0x34.is_some(),
            self.version > 2,
            "unknown_0x34",
        )?;
        gated(
            self.unknown_0x18.is_some(),
            self.version > 1,
            "unknown_0x18",
        )?;
        self.version.write(w)?;
        self.unknown_0x2c.write(w)?;
        self.unknown_0x30.write(w)?;
        if let Some(text) = &self.unknown_0x34 {
            w.write_string(text)?;
        }
        self.flag_0x38.write(w)?;
        self.unknown_0x4.write(w)?;
        if let Some(objects) = &self.unknown_0x18 {
            objects.write(w)?;
        }
        Ok(())
    }
}

record! {
    /// Spell points per level (`CSpellPointsOb`, schema 94), found in
    /// community classes made with a later build of the Expansion: the
    /// `TSRP3.EXE` of the Expansion CD does not have the class and stops
    /// reading a file that holds one. The layout comes from those files
    /// (41 bytes each); importing drops them, see `cr_store::export`.
    pub struct SpellPoints {
        /// 1 in every file.
        version: u32,
        /// Level, spell points, highest spell level (Druid of Chauntea at
        /// level 1: 4 points, 3rd level), then values that are mostly 0.
        values: [i32; 9],
        flag: u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_strings() {
        assert_eq!(version_of("TSRP3 V39"), 39);
        assert_eq!(version_of("TSRP3 V21"), 21);
        assert_eq!(version_of("Fighter"), 38);
    }

    fn round_trip<T: Field + PartialEq + std::fmt::Debug>(value: &T) -> Vec<u8> {
        let mut w = ArchiveWriter::new();
        value.write(&mut w).unwrap();
        let bytes = w.into_bytes();
        let mut r = ArchiveReader::new(&bytes);
        assert_eq!(&T::read(&mut r).unwrap(), value);
        assert!(r.is_at_end());
        bytes
    }

    #[test]
    fn free_spells_members_follow_the_version() {
        let mut free = ClassFreeSpells {
            version: 4,
            unknown_0x2c: 1,
            unknown_0x30: 1,
            unknown_0x34: Some("per day".into()),
            flag_0x38: 0,
            unknown_0x4: vec![],
            unknown_0x18: Some(vec![]),
        };
        let v4 = round_trip(&free);
        // Version, level, uses, "per day", flag, two empty arrays (a count
        // each).
        assert_eq!(v4.len(), 4 + 4 + 4 + 8 + 1 + 2 + 2);

        free.version = 1;
        assert!(free.write(&mut ArchiveWriter::new()).is_err());
        free.unknown_0x34 = None;
        free.unknown_0x18 = None;
        assert_eq!(round_trip(&free).len(), 4 + 4 + 4 + 1 + 2);
    }

    #[test]
    fn spell_points_are_41_bytes() {
        let points = SpellPoints {
            version: 1,
            values: [1, 4, 3, 0, 0, 0, 0, 0, 0],
            flag: 0,
        };
        assert_eq!(round_trip(&points).len(), 41);
    }
}
