//! Parts: `Database/Parts.dat` and `UserDbas/PartsU.dat` (`CPartDoc`,
//! `Serialize` at 0x67b10d in TSRP3.EXE), the weapons, armor, proficiencies,
//! kits and other "parts" a character is built from.
//!
//! The file is 21 consecutive `CObArray`s of `CPart` with no outer count;
//! the array index is the part type ([`PART_TYPES`]). A Character Kits part
//! carries a `CPartKitOb`. Unnamed fields keep the member offset they have
//! in the original object (`unknown_0x30c` is the member at `this + 0x30c`).

use crate::field::Field;
use crate::{ArchiveReader, ArchiveWriter, Error, Object, Result, record};

/// Number of part arrays in a parts file.
pub const PART_TYPE_COUNT: usize = 21;

/// Part type names by array index, from the table `CPartDoc` builds in
/// `DeleteContents` (0x677978). "NOT" marks the two unused slots; the last
/// two have no name. Type 17 ("Count") holds units such as "Charge".
pub const PART_TYPES: [Option<&str>; PART_TYPE_COUNT] = [
    Some("Weapons"),
    Some("Armor"),
    Some("NOT"),
    Some("Weapon Proficiencies"),
    Some("Non-Weapon Proficiencies"),
    Some("Racial Abilities"),
    Some("Languages"),
    Some("Equipment"),
    Some("Magical Items"),
    Some("Schools Of Magic"),
    Some("NOT"),
    Some("Traits"),
    Some("Disadvantages"),
    Some("Gems and Art"),
    Some("Class Abilities"),
    Some("Character Kits"),
    Some("Currency Types"),
    Some("Count"),
    Some("Spheres"),
    None,
    None,
];

/// A parts file: one array of parts per part type.
#[derive(Debug, Clone, PartialEq)]
pub struct PartsFile {
    pub arrays: [Vec<Object>; PART_TYPE_COUNT],
}

impl PartsFile {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = ArchiveReader::new(data);
        let arrays = Field::read(&mut r)?;
        r.finish()?;
        Ok(Self { arrays })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut w = ArchiveWriter::new();
        self.arrays.write(&mut w)?;
        Ok(w.into_bytes())
    }

    /// The parts of the type named `name` (see [`PART_TYPES`]).
    pub fn of_type(&self, name: &str) -> Option<&[Object]> {
        let index = PART_TYPES.iter().position(|t| *t == Some(name))?;
        Some(&self.arrays[index])
    }
}

/// The part layout the Expansion writes.
pub const CURRENT_VERSION: i32 = 7;

/// Any word below this where the version marker goes starts a version
/// string; the program writes -8889 (`0xffffdd47`).
const MARKER_LIMIT: i32 = -8888;

/// The part version a version string announces, the way the original reads
/// it: the current version on an exact match, otherwise the number after the
/// first seven characters, or the version before the current one when there
/// is nothing after them.
pub fn version_of(version_string: &str) -> i32 {
    if version_string == format!("TSRP3 V{CURRENT_VERSION}") {
        CURRENT_VERSION
    } else if version_string.len() > 7 {
        crate::atol(version_string.get(7..).unwrap_or(""))
    } else {
        CURRENT_VERSION - 1
    }
}

/// The version tag of a [`Part`].
#[derive(Debug, Clone, PartialEq)]
pub struct PartVersion {
    /// Below -8888; -8889 in every shipped part.
    pub marker: i32,
    /// `TSRP3 V7` for files the Expansion wrote.
    pub string: String,
}

/// One part (`CPart`, schema 93, `Serialize` at 0x65c3a0).
///
/// After the first block of fields comes either a version marker and a
/// version string, or (records from before versioning, version 0) the
/// value of `unknown_0x728` directly.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub head: PartHead,
    /// `None` for an unversioned (version 0) record.
    pub version: Option<PartVersion>,
    pub unknown_0x728: i32,
    pub data: PartData,
}

impl Part {
    /// The layout version the fields in `data` follow.
    pub fn layout_version(&self) -> i32 {
        self.version.as_ref().map_or(0, |v| version_of(&v.string))
    }
}

impl Field for Part {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let head = PartHead::read(r)?;
        let word = r.read_i32()?;
        let (version, unknown_0x728) = if word < MARKER_LIMIT {
            let string = r.read_string()?;
            let value = r.read_i32()?;
            (
                Some(PartVersion {
                    marker: word,
                    string,
                }),
                value,
            )
        } else {
            (None, word)
        };
        let v = version.as_ref().map_or(0, |v| version_of(&v.string));
        let data = PartData::read_fields(r, v)?;
        Ok(Self {
            head,
            version,
            unknown_0x728,
            data,
        })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        self.head.write(w)?;
        match &self.version {
            Some(version) => {
                if version.marker >= MARKER_LIMIT {
                    return Err(Error::FieldMismatch {
                        field: "marker",
                        expected: false,
                    });
                }
                w.write_i32(version.marker);
                w.write_string(&version.string)?;
            }
            // The value would read back as a version marker.
            None if self.unknown_0x728 < MARKER_LIMIT => {
                return Err(Error::FieldMismatch {
                    field: "version",
                    expected: true,
                });
            }
            None => {}
        }
        w.write_i32(self.unknown_0x728);
        self.data.write_fields(w, self.layout_version())
    }
}

record! {
    /// The fields of a part before its version marker.
    pub struct PartHead {
        /// "Belaying pin".
        name: String,
        unknown_0x300: String,
        unknown_0x308: i32,
        unknown_0x230: i32,
        /// Rules text.
        description: String,
        unknown_0x30c: [i32; 19],
        unknown_0x394: [i32; 3],
        unknown_0x2ac: i32,
        unknown_0x80c: [i32; 4],
        /// -1 default.
        unknown_0x2a8: i32,
        unknown_0x4cc: i32,
        unknown_0x470: [i32; 4],
        /// Equipment category ("Misc","Clothing","Animal").
        unknown_0x240: String,
        /// NWP relevant ability ("Intelligence").
        unknown_0x460: String,
        /// NWP sub-ability ("Wisdom/Intuition").
        unknown_0x45c: String,
        /// Equipment source group ("Elven Equipment").
        unknown_0x244: String,
        /// School family ("School of Effects").
        unknown_0x1b4: String,
        /// Related NWP name.
        unknown_0x464: String,
        unknown_0x1e4: i32,
        unknown_0x1e8: String,
        /// `f32` bit pattern (0x3dcccccd = 0.1 most common).
        unknown_0x1ec: i32,
        /// `f32` bit pattern.
        unknown_0x1f0: i32,
        unknown_0x1f8: [i32; 8],
        unknown_0x224: i32,
        /// CString holding a number ("209").
        unknown_0x234: String,
        unknown_0x238: [i32; 2],
        unknown_0x24c: i32,
        unknown_0x248: i32,
        unknown_0x250: [i32; 10],
        /// `f32` bit pattern (default -1.0).
        unknown_0x278: i32,
        unknown_0x27c: [i32; 11],
        unknown_0x2b4: [i32; 9],
        /// Ability/sub-ability ("Dexterity/Aim").
        unknown_0x458: String,
        /// Weapon damage type ("P","S","B").
        unknown_0x3b4: String,
        /// Second-mode damage type.
        unknown_0x3e4: String,
        unknown_0x3b8: [i32; 4],
        unknown_0x3e8: i32,
        unknown_0x3c8: i32,
        unknown_0x3ec: i32,
        unknown_0x3cc: i32,
        unknown_0x3f0: i32,
        /// Weapon size ("S","M","L").
        unknown_0x3d0: String,
        /// Second-mode size.
        unknown_0x3f4: String,
        unknown_0x3d4: i32,
        unknown_0x3f8: i32,
        unknown_0x3d8: i32,
        unknown_0x3fc: i32,
        unknown_0x3dc: i32,
        unknown_0x400: i32,
        unknown_0x3e0: i32,
        unknown_0x404: i32,
        unknown_0x40c: [i32; 4],
        /// Rate of fire ("2/rnd").
        unknown_0x41c: String,
        /// Rate of fire (second mode).
        unknown_0x408: String,
        unknown_0x44c: [i32; 3],
        unknown_0x2d8: [i32; 10],
    }
}

record! {
    /// The fields of a part after its version.
    pub struct PartData(v) {
        unknown_0x72c: [i32; 6],
        unknown_0x480: [i32; 3],
        unknown_0x420: [i32; 2],
        unknown_0x42c: i32,
        /// Class flags of records older than version 1, in the order Bard,
        /// Cleric, Druid, Fighter, Mage, Paladin, Ranger, (not converted;
        /// Psionicist?), Thief. On load the program turns them into names in
        /// `allowed_classes` and clears them.
        old_class_flags: [i32; 9],
        unknown_0x4c8: i32,
        unknown_0x510: [i32; 7],
        unknown_0x52c: Option<i32> = if v > 6,
        unknown_0x530: [i32; 16],
        unknown_0x48c: [i32; 6],
        unknown_0x584: [i32; 5],
        unknown_0x220: i32,
        /// Armor: thief skill adjustments (-40, ...).
        unknown_0x59c: [i32; 13],
        unknown_0x598: i32,
        unknown_0x4: Option<[i32; 101]> = if v > 5,
        unknown_0x570: i32,
        unknown_0x5e4: [i32; 24],
        unknown_0x664: [i32; 2],
        unknown_0x678: i32,
        unknown_0x648: [i32; 2],
        unknown_0x644: i32,
        unknown_0x228: i32,
        unknown_0x19c: i32,
        unknown_0x2b0: i32,
        unknown_0x22c: i32,
        unknown_0x218: i32,
        unknown_0x75c: [i32; 2],
        flag_0x724: Option<u8> = if v > 4,
        unknown_0x868: [String; 10],
        unknown_0x468: String,
        /// NWP group ("GENERAL","PRIEST").
        unknown_0x46c: String,
        /// Related item name ("Sword, short").
        unknown_0x428: String,
        /// Related armor name.
        unknown_0x430: String,
        unknown_0x574: [String; 3],
        /// Race name (racial abilities).
        unknown_0x580: String,
        /// Race name (traits/disadv.).
        unknown_0x21c: String,
        /// Race name (racial abilities).
        unknown_0x1e0: String,
        /// `f32` bit pattern.
        unknown_0x1f4: i32,
        unknown_0x744: i32,
        unknown_0x778: [i32; 2],
        flags_0x780: [u8; 6],
        unknown_0x7a0: i32,
        unknown_0x66c: [i32; 3],
        unknown_0x7b8: i32,
        unknown_0x7a4: [i32; 4],
        unknown_0x7bc: i32,
        unknown_0x71c: i32,
        unknown_0x78c: i32,
        unknown_0x788: i32,
        unknown_0x794: i32,
        unknown_0x79c: i32,
        unknown_0x790: i32,
        unknown_0x7cc: [i32; 7],
        unknown_0x7b4: i32,
        unknown_0x4f0: i32,
        unknown_0x4e4: i32,
        unknown_0x7e8: [i32; 9],
        unknown_0x7c0: [i32; 3],
        unknown_0x81c: [i32; 4],
        unknown_0x798: i32,
        unknown_0x710: [String; 2],
        unknown_0x650: i32,
        flag_0x67e: u8,
        flags_0x680: [u8; 2],
        unknown_0x654: i32,
        flag_0x67f: u8,
        /// Class ability name ("Tracking").
        unknown_0x718: String,
        /// 0x80000000 on all non-kit parts, 0 on kits.
        unknown_0x660: i32,
        flag_0x67d: u8,
        unknown_0x720: String,
        /// 100 on non-kits, 0 on kits.
        unknown_0x65c: i32,
        flag_0x67c: u8,
        unknown_0x658: i32,
        flag_0x4d0: Option<u8> = if v > 1,
        flag_0x448: Option<u8> = if v > 2,
        /// The kit (`CPartKitOb`) of a Character Kits part, `None` for every
        /// other part.
        kit: Option<Option<Box<Object>>> = if v > 3,
        unknown_0x6e8: Vec<String>,
        unknown_0x358: Vec<String>,
        unknown_0x36c: Vec<String>,
        unknown_0x380: Vec<String>,
        unknown_0x3a0: Vec<String>,
        /// School names (Schools of Magic).
        unknown_0x1a0: Vec<String>,
        /// Historical eras.
        unknown_0x1b8: Vec<String>,
        /// Weapon groups ("Swords: Large").
        unknown_0x1cc: Vec<String>,
        unknown_0x840: Vec<String>,
        unknown_0x854: Vec<String>,
        /// Ammo/launcher names.
        unknown_0x434: Vec<String>,
        /// Weapon names.
        unknown_0x4fc: Vec<String>,
        unknown_0x5d0: Vec<String>,
        unknown_0x684: Vec<String>,
        unknown_0x698: Vec<String>,
        unknown_0x6c0: Vec<String>,
        unknown_0x6ac: Vec<String>,
        unknown_0x6d4: Vec<String>,
        unknown_0x6fc: Vec<String>,
        /// Class ability names.
        unknown_0x764: Vec<String>,

        /// `CSpellsOb`s; empty in every shipped part.
        unknown_0x748: Vec<Object>,
        /// Classes allowed to take the part ("Cleric", "Druid", "Mage").
        allowed_classes: Option<Vec<String>> = if v > 0,
    }
}

/// Header string every `CPartKitOb` starts with.
pub const KIT_HEADER: &str = "CPKO";

/// The kit layout the Expansion writes.
pub const KIT_CURRENT_VERSION: u32 = 30;

/// A character kit (`CPartKitOb`, schema 1, `Serialize` at 0x66ce22), hung
/// off a Character Kits part.
#[derive(Debug, Clone, PartialEq)]
pub struct PartKit {
    /// [`KIT_HEADER`] in every valid kit.
    pub header: String,
    /// 30 in every shipped kit.
    pub version: u32,
    /// `None` when the header is not [`KIT_HEADER`]: the program then reports
    /// an error and reads nothing more.
    pub data: Option<PartKitData>,
}

impl Field for PartKit {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let header = r.read_string()?;
        let version = r.read_u32()?;
        let data = (header == KIT_HEADER)
            .then(|| PartKitData::read_fields(r, version as i32))
            .transpose()?;
        Ok(Self {
            header,
            version,
            data,
        })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_string(&self.header)?;
        w.write_u32(self.version);
        match (&self.data, self.header == KIT_HEADER) {
            // The program compares the version as a signed number.
            (Some(data), true) => data.write_fields(w, self.version as i32),
            (None, false) => Ok(()),
            (_, expected) => Err(Error::FieldMismatch {
                field: "data",
                expected,
            }),
        }
    }
}

record! {
    /// The fields of a kit after its header and version. After loading, the
    /// program clears `flags_0x30c[2]` when `flags_0x44c[1]` is set.
    pub struct PartKitData(v) {
        flags_0x4: [u8; 11],
        flags_0x71: [u8; 2],
        flag_0x444: u8,
        flag_0x4b4: Option<u8> = if v > 29,
        flags_0x75: [u8; 8],
        flag_0x84: u8,
        /// Raw bytes, not booleans (25, 25, ..., 10, ..., 12 in the
        /// shipped kits: ability-score-like values).
        unknown_0x4c: [u8; 37],
        unknown_0x80: i32,
        /// Part type of `old_id_names` (local; before version 3).
        old_id_type: Option<i32> = if v < 3,
        unknown_0x88: [i32; 5],
        /// The first two hold long description texts.
        unknown_0x9c: [String; 3],
        unknown_0x670: Option<i32> = if v > 19,
        /// "Fighter".
        unknown_0x674: Option<String> = if v > 19,
        /// Part names the program turns into `CPartIDOb`s with
        /// `old_id_type` in `unknown_0x244` (local; before version 3).
        old_id_names: Option<Vec<String>> = if v < 3,
        /// Race names ("Deep dwarf","Dwarf").
        unknown_0x10: Vec<String>,
        unknown_0x24: Vec<String>,
        unknown_0x38: Vec<String>,
        /// Race names (monster races "Aarakocra",...).
        unknown_0xa8: Vec<String>,
        /// Weapon names ("Quarterstaff").
        unknown_0x384: Vec<String>,
        unknown_0xbc: Vec<String>,
        unknown_0xd0: Vec<String>,
        /// NWP names ("Disguise","Etiquette",...).
        unknown_0x258: Vec<String>,
        /// Trait names ("Alertness","Ambidexterity").
        unknown_0xe4: Vec<String>,
        /// ("Net").
        unknown_0xf8: Vec<String>,
        /// 13 social-class strings indexed by roll ("", "", "Lower class", ...).
        unknown_0x10c: Vec<String>,
        /// `CPartIDOb`s.
        unknown_0x244: Option<Vec<Object>> = if v >= 3,
        flags_0x73: Option<[u8; 2]> = if v >= 4,
        unknown_0x120: Option<[i32; 2]> = if v >= 5,
        unknown_0x13c: Option<i32> = if v >= 5,
        unknown_0x154: Option<i32> = if v >= 5,
        unknown_0x16c: Option<[i32; 2]> = if v >= 5,
        unknown_0x128: Option<Vec<String>> = if v >= 5,
        unknown_0x140: Option<Vec<String>> = if v >= 5,
        unknown_0x158: Option<Vec<String>> = if v >= 5,
        /// `CPartIDOb`s ("Bard, Climb walls...").
        unknown_0x174: Option<Vec<Object>> = if v >= 5,
        flags_0x188: Option<[u8; 2]> = if v >= 6,
        unknown_0x18c: Option<[i32; 5]> = if v >= 6,
        unknown_0x1c8: Option<Vec<String>> = if v >= 7,
        unknown_0x1dc: Option<Vec<String>> = if v >= 7,
        unknown_0x208: Option<Vec<String>> = if v >= 7,
        flag_0x1f0: Option<u8> = if v >= 7,
        flag_0x1fc: Option<u8> = if v >= 7,
        unknown_0x1f4: Option<[i32; 2]> = if v >= 7,
        unknown_0x200: Option<[i32; 2]> = if v >= 7,
        unknown_0x26c: Option<Vec<String>> = if v >= 8,
        unknown_0x280: Option<Vec<String>> = if v >= 8,
        unknown_0x294: Option<Vec<String>> = if v >= 8,
        unknown_0x2a8: Option<Vec<String>> = if v >= 8,
        unknown_0x2bc: Option<Vec<String>> = if v >= 8,
        unknown_0x2d0: Option<Vec<String>> = if v >= 8,
        unknown_0x2e4: Option<Vec<String>> = if v >= 8,
        /// `CPartIDOb`s.
        unknown_0x2f8: Option<Vec<Object>> = if v >= 8,
        flags_0x30c: Option<[u8; 9]> = if v >= 8,
        unknown_0x318: Option<i32> = if v >= 8,
        unknown_0x320: Option<i32> = if v >= 8,
        unknown_0x32c: Option<i32> = if v >= 8,
        unknown_0x31c: Option<String> = if v >= 8,
        unknown_0x324: Option<[String; 2]> = if v >= 8,
        /// Choose-N-of option lists.
        unknown_0x330: Option<ListOfListsOfOptions> = if v >= 9,
        flag_0x348: Option<u8> = if v >= 10,
        flag_0x380: Option<u8> = if v >= 10,
        flags_0x44c: Option<[u8; 3]> = if v >= 10,
        flags_0x464: Option<[u8; 2]> = if v >= 10,
        flags_0x4ac: Option<[u8; 8]> = if v >= 10,
        flag_0x508: Option<u8> = if v >= 10,
        flag_0x518: Option<u8> = if v >= 10,
        flags_0x520: Option<[u8; 2]> = if v >= 10,
        flag_0x538: Option<u8> = if v >= 10,
        unknown_0x34c: Option<[i32; 2]> = if v >= 10,
        unknown_0x368: Option<i32> = if v >= 10,
        unknown_0x448: Option<i32> = if v >= 10,
        unknown_0x450: Option<[i32; 5]> = if v >= 10,
        unknown_0x468: Option<[i32; 2]> = if v >= 10,
        unknown_0x50c: Option<[i32; 3]> = if v >= 10,
        unknown_0x51c: Option<i32> = if v >= 10,
        unknown_0x564: Option<i32> = if v > 23,
        /// Weapons as `CPartIDOb`s ("Long bow").
        unknown_0x470: Option<Vec<Object>> = if v >= 10,
        unknown_0x354: Option<Vec<String>> = if v >= 10,
        unknown_0x36c: Option<Vec<String>> = if v >= 10,
        /// `unknown_0x384` serialized a second time; the program keeps
        /// this copy on load.
        unknown_0x384_again: Option<Vec<String>> = if v >= 10,
        unknown_0x484: Option<Vec<String>> = if v >= 10,
        unknown_0x498: Option<Vec<String>> = if v >= 10,
        unknown_0x4b8: Option<Vec<String>> = if v >= 10,
        unknown_0x4cc: Option<Vec<String>> = if v >= 10,
        unknown_0x4e0: Option<Vec<String>> = if v >= 10,
        unknown_0x4f4: Option<Vec<String>> = if v >= 10,
        unknown_0x524: Option<Vec<String>> = if v >= 10,
        unknown_0x53c: Option<Vec<String>> = if v >= 10,
        unknown_0x550: Option<Vec<String>> = if v > 23,
        unknown_0x568: Option<ListOfListsOfOptions> = if v >= 11,
        unknown_0x1a0: Option<Vec<String>> = if v >= 12,
        unknown_0x398: Option<Vec<String>> = if v >= 12,
        unknown_0x3ac: Option<Vec<String>> = if v >= 12,
        unknown_0x3c0: Option<Vec<String>> = if v >= 12,
        unknown_0x3d4: Option<Vec<String>> = if v >= 12,
        unknown_0x3e8: Option<Vec<String>> = if v >= 13,
        unknown_0x3fc: Option<Vec<String>> = if v >= 13,
        unknown_0x410: Option<Vec<String>> = if v >= 13,
        unknown_0x424: Option<Vec<String>> = if v >= 13,
        unknown_0x1b4: Option<Vec<String>> = if v >= 13,
        flags_0x438: Option<[u8; 2]> = if v >= 13,
        flags_0x5d4: Option<[u8; 6]> = if v >= 13,
        unknown_0x43c: Option<[i32; 2]> = if v >= 13,
        unknown_0x5e0: Option<[i32; 7]> = if v >= 13,
        unknown_0x61c: Option<String> = if v >= 13,
        unknown_0x5fc: Option<[i32; 2]> = if v >= 13,
        /// Equipment names ("Quiver").
        unknown_0x598: Option<Vec<String>> = if v >= 13,
        unknown_0x608: Option<[i32; 4]> = if v >= 14,
        /// Bonus spells (`CSpellsOb`, "Detect Magic").
        bonus_spells: Option<Vec<Object>> = if v >= 15,
        flag_0x5da: Option<u8> = if v >= 16,
        /// Per-level table (`CPartKitLevelOb`).
        levels: Option<Vec<Object>> = if v >= 16,
        flag_0x580: Option<u8> = if v >= 17,
        /// Starting equipment ("Backpack").
        starting_equipment: Option<Vec<String>> = if v >= 17,
        flag_0x5dc: Option<u8> = if v >= 18,
        flag_0x618: Option<u8> = if v >= 19,
        unknown_0x604: Option<i32> = if v >= 21,
        unknown_0x620: Option<Vec<String>> = if v >= 22,
        /// Allowed classes and multi-class combinations for the kit
        /// (`CClassesAllowedHelperOb`).
        allowed_classes: Option<Vec<Object>> = if v >= 23,
        /// Spheres ("Animal").
        unknown_0x21c: Option<Vec<String>> = if v >= 25,
        /// ("Necromantic").
        unknown_0x230: Option<Vec<String>> = if v >= 26,
        flags_0x5dd: Option<[u8; 2]> = if v >= 27,
        flag_0x5df: Option<u8> = if v >= 28,
        unknown_0x5ac: Option<Vec<String>> = if v >= 29,
        unknown_0x5c0: Option<Vec<String>> = if v >= 29,
    }
}

record! {
    /// A part named together with its type (`CPartIDOb`, schema 94), e.g.
    /// "Tracking", 14 (Class Abilities).
    pub struct PartId {
        name: String,
        /// Index into [`PART_TYPES`].
        part_type: i32,
    }
}

record! {
    /// One row of a kit's per-level table (`CPartKitLevelOb`, versionable
    /// schema 1).
    pub struct PartKitLevel {
        /// 1 in every shipped kit; the program does not test it.
        version: u32,
        unknown_0x4: [i32; 10],
    }
}

record! {
    /// Pick `count` of `options` (`CNumList`, schema 1).
    pub struct NumList {
        /// How many to pick (1, 2).
        count: i32,
        options: Vec<String>,
    }
}

record! {
    /// `CListOfListsOfOptions`, embedded in a kit rather than stored as an
    /// object: an array of `CNumList`s.
    pub struct ListOfListsOfOptions {
        lists: Vec<Object>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Enough zero bytes for any record below: empty strings, empty arrays,
    /// zero integers and null objects.
    const ZEROS: [u8; 4096] = [0; 4096];

    fn round_trip<T: Field + PartialEq + std::fmt::Debug>(value: &T) -> Vec<u8> {
        let mut w = ArchiveWriter::new();
        value.write(&mut w).unwrap();
        let bytes = w.into_bytes();
        let mut r = ArchiveReader::new(&bytes);
        assert_eq!(&T::read(&mut r).unwrap(), value);
        assert!(r.is_at_end());
        bytes
    }

    fn zero_part(version: Option<PartVersion>) -> Part {
        let head = PartHead::read(&mut ArchiveReader::new(&ZEROS)).unwrap();
        let v = version.as_ref().map_or(0, |v| version_of(&v.string));
        let data = PartData::read_fields(&mut ArchiveReader::new(&ZEROS), v).unwrap();
        Part {
            head,
            version,
            unknown_0x728: 5,
            data,
        }
    }

    #[test]
    fn version_strings() {
        assert_eq!(version_of("TSRP3 V7"), 7);
        assert_eq!(version_of("TSRP3 V3"), 3);
        assert_eq!(version_of("TSRP3 "), 6);
    }

    #[test]
    fn unversioned_part_stores_unknown_0x728_in_place_of_the_marker() {
        let old = zero_part(None);
        assert_eq!(old.layout_version(), 0);
        assert_eq!(old.data.kit, None);
        assert_eq!(old.data.allowed_classes, None);
        let old_bytes = round_trip(&old);

        let new = zero_part(Some(PartVersion {
            marker: -8889,
            string: "TSRP3 V7".into(),
        }));
        assert_eq!(new.data.kit, Some(None));
        let new_bytes = round_trip(&new);
        // Marker and version string, then the fields of versions 1 to 7:
        // allowed classes (2), two flags, a null kit (2), unknown_0x52c,
        // unknown_0x4, flag_0x724.
        assert_eq!(
            new_bytes.len() - old_bytes.len(),
            4 + 9 + 2 + 2 + 2 + 4 + 404 + 1
        );
    }

    #[test]
    fn unversioned_part_cannot_hold_a_marker_value() {
        let mut part = zero_part(None);
        part.unknown_0x728 = -8889;
        assert!(part.write(&mut ArchiveWriter::new()).is_err());
    }

    #[test]
    fn old_kit_keeps_its_id_list() {
        let mut bytes = vec![4, b'C', b'P', b'K', b'O', 2, 0, 0, 0];
        bytes.extend_from_slice(&ZEROS);
        let mut r = ArchiveReader::new(&bytes);
        let kit = PartKit::read(&mut r).unwrap();
        let data = kit.data.as_ref().unwrap();
        assert_eq!(data.old_id_type, Some(0));
        assert_eq!(data.old_id_names, Some(vec![]));
        assert_eq!(data.unknown_0x244, None);
        assert_eq!(data.flag_0x4b4, None);
        let written = round_trip(&kit);
        assert_eq!(&written[..], &bytes[..r.position()]);
    }

    #[test]
    fn kit_with_a_wrong_header_stops_after_the_version() {
        let bytes = [4, b'C', b'P', b'K', b'X', 30, 0, 0, 0];
        let kit = PartKit::read(&mut ArchiveReader::new(&bytes)).unwrap();
        assert_eq!(kit.data, None);
        assert_eq!(round_trip(&kit), bytes);
    }

    #[test]
    fn empty_user_file() {
        let file = PartsFile::parse(&[0; 42]).unwrap();
        assert!(file.arrays.iter().all(Vec::is_empty));
        assert_eq!(file.of_type("Spheres"), Some(&[][..]));
        assert_eq!(file.to_bytes().unwrap(), [0; 42]);
    }
}
