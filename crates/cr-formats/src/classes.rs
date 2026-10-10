//! Classes: `Database/Class.dat` and `UserDbas/ClassU.dat`, a `CObList` of
//! `CClassOb` (schema 88, `CClassOb::Serialize` at 0x6644af in TSRP3.EXE).
//!
//! Each class starts with its own version string, `TSRP3 V39` in the
//! Expansion. Older layouts lack the fields their version predates; the
//! conditions below are copied from the load code. Unnamed fields keep the
//! member offset they have in the original object.

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

/// One class (`CClassOb`).
#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    /// `TSRP3 V39` for files the Expansion wrote.
    pub version: String,
    pub data: ClassData,
}

impl Field for Class {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let version = r.read_string()?;
        let data = ClassData::read_fields(r, version_of(&version))?;
        Ok(Self { version, data })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_string(&self.version)?;
        self.data.write_fields(w, version_of(&self.version))
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
        unknown_0x54: [i32; 2],
        unknown_0x84: [i32; 3],
        unknown_0xb4: [i32; 2],
        /// Database identifier: 1000s warriors, 2000s wizards, 3000s
        /// priests, 4000s rogues (Fighter 1000, Paladin 1001).
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
        /// "Any" in every shipped class.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_strings() {
        assert_eq!(version_of("TSRP3 V39"), 39);
        assert_eq!(version_of("TSRP3 V21"), 21);
        assert_eq!(version_of("Fighter"), 38);
    }
}
