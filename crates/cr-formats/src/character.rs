//! Character files: `Chars/index.chr` and one `<id>.dat` per character.
//!
//! A character file is a `CCharacterDoc` (`Serialize` at 0x41cd5b in
//! TSRP3.EXE). It opens with a version string, `TSRP3 V54` for the
//! Expansion and `TSRP2 V29` for Core Rules 2.0 (the Expansion also
//! recognises an older `TSRP V27`); the fields a version predates are
//! absent, with the conditions copied from the load code. Files from both
//! programs round-trip (tests/fixtures/characters and its `v20` folder).
//! Unnamed fields keep the member offset they have in the original object.

use crate::field::Field;
use crate::inventory::InventoryBundle;
use crate::{ArchiveReader, ArchiveWriter, ClassInfo, Error, Object, ObjectTag, Result, record};

const MINI_CLASS: &str = "CCharacterMiniOb";

/// The character layout the Expansion writes.
pub const CURRENT_VERSION: i32 = 54;

/// The version string of the files the Expansion writes.
pub const EXPANSION_VERSION: &str = "TSRP3 V54";

/// The version string of the files Core Rules 2.0 writes.
pub const CORE20_VERSION: &str = "TSRP2 V29";

/// One entry of `index.chr` (`CCharacterMiniOb`), the summary the
/// character list shows without opening each file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterEntry {
    pub name: String,
    /// File name inside `Chars/`, e.g. `90688519.dat`.
    pub file: String,
    pub race: String,
    /// Entry layout version, 3 in the Expansion. Versions 0 and 1 store
    /// only `unknown_0x24` and the flag.
    pub version: u32,
    pub extra: Option<EntryExtra>,
    pub unknown_0x24: u32,
    /// 1 for the Core Rules fixture, 0 for the Skills & Powers one.
    pub flag_0x28: u8,
    /// Display lines: rules system, player, party, race, class, level, kit.
    pub details: Vec<String>,
}

/// Fields of a [`CharacterEntry`] from layout version 2 on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryExtra {
    /// Read into a local and dropped by the Expansion (8 in both fixtures).
    pub unknown: u32,
    pub unknown_0x2c: String,
}

impl Field for CharacterEntry {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let name = r.read_string()?;
        let file = r.read_string()?;
        let race = r.read_string()?;
        let version = r.read_u32()?;
        let (unknown, unknown_0x24, flag_0x28, unknown_0x2c) = if version > 1 {
            (
                Some(r.read_u32()?),
                r.read_u32()?,
                r.read_u8()?,
                Some(r.read_string()?),
            )
        } else {
            (None, version, r.read_u8()?, None)
        };
        let extra = match (unknown, unknown_0x2c) {
            (Some(unknown), Some(unknown_0x2c)) => Some(EntryExtra {
                unknown,
                unknown_0x2c,
            }),
            _ => None,
        };
        Ok(Self {
            name,
            file,
            race,
            version,
            extra,
            unknown_0x24,
            flag_0x28,
            details: Field::read(r)?,
        })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_string(&self.name)?;
        w.write_string(&self.file)?;
        w.write_string(&self.race)?;
        // Layouts 0 and 1 have no separate flag: this u32 is the NPC mark
        // (+0x24), 1 for an NPC (Evermore 0x42c690).
        w.write_u32(if self.version > 1 {
            self.version
        } else {
            self.unknown_0x24
        });
        match (&self.extra, self.version > 1) {
            (Some(extra), true) => {
                w.write_u32(extra.unknown);
                w.write_u32(self.unknown_0x24);
                w.write_u8(self.flag_0x28);
                w.write_string(&extra.unknown_0x2c)?;
            }
            (None, false) => w.write_u8(self.flag_0x28),
            (_, expected) => {
                return Err(Error::FieldMismatch {
                    field: "extra",
                    expected,
                });
            }
        }
        self.details.write(w)
    }
}

/// `index.chr`: a `CObList` of [`CharacterEntry`] (`CCharacterFileList`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterIndex {
    pub schema: u16,
    pub entries: Vec<CharacterEntry>,
}

impl CharacterIndex {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = ArchiveReader::new(data);
        let count = r.read_count()?;
        let mut schema = 0;
        let mut entries = Vec::with_capacity(count.min(1024) as usize);
        for _ in 0..count {
            let offset = r.position();
            match r.read_object_tag()? {
                ObjectTag::New(class) if class.name == MINI_CLASS => schema = class.schema,
                ObjectTag::New(class) => {
                    return Err(Error::UnexpectedClass {
                        expected: MINI_CLASS,
                        found: class.name,
                    });
                }
                tag => return Err(Error::UnexpectedTag { offset, tag }),
            }
            entries.push(CharacterEntry::read(&mut r)?);
        }
        r.finish()?;
        Ok(Self { schema, entries })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let class = ClassInfo {
            name: MINI_CLASS.into(),
            schema: self.schema,
        };
        let mut w = ArchiveWriter::new();
        w.write_count(self.entries.len() as u32);
        for e in &self.entries {
            w.write_object_tag(&class);
            e.write(&mut w)?;
        }
        Ok(w.into_bytes())
    }
}

/// The character version a version string announces, the way the original
/// reads it.
pub fn version_of(version_string: &str) -> i32 {
    match version_string {
        "TSRP V27" => 27,
        "TSRP3 V28" => 28,
        s if s == format!("TSRP3 V{CURRENT_VERSION}") => CURRENT_VERSION,
        s => crate::atol(s.find('V').map_or(s, |i| &s[i + 1..])),
    }
}

/// A character file (`CCharacterDoc`).
#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    /// `TSRP3 V54` for files the Expansion wrote.
    pub version: String,
    pub data: CharacterData,
}

/// A character embedded in another object (an NPC in an encounter) is
/// stored exactly like a character file.
impl Field for Character {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let version = r.read_string()?;
        let data = CharacterData::read_fields(r, version_of(&version))?;
        Ok(Self { version, data })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_string(&self.version)?;
        self.data.write_fields(w, version_of(&self.version))
    }
}

impl Character {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = ArchiveReader::new(data);
        let character = Self::read(&mut r)?;
        r.finish()?;
        Ok(character)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut w = ArchiveWriter::new();
        self.write(&mut w)?;
        Ok(w.into_bytes())
    }

    /// A character with every string and list empty and every number 0,
    /// in the layout `version_string` announces: what a new character
    /// starts from before the creation steps fill it in.
    pub fn blank(version_string: &str) -> Result<Self> {
        // Zero bytes read as empty strings, empty lists and zeros.
        let zeros = vec![0u8; 1 << 16];
        let mut r = ArchiveReader::new(&zeros);
        let data = CharacterData::read_fields(&mut r, version_of(version_string))?;
        Ok(Self {
            version: version_string.into(),
            data,
        })
    }

    /// Whether the character was built with Skills & Powers rather than
    /// the Core Rules.
    pub fn skills_and_powers(&self) -> bool {
        self.data.skills_and_powers != 0
    }
}

record! {
    /// The fields of a character after its version string.
    pub struct CharacterData(v) {
        /// 1 when the character uses Skills & Powers.
        skills_and_powers: u8,
        unknown_0x1dac: u32,
        player: String,
        name: String,
        /// Class as displayed, e.g. "Fighter/Mage".
        class_title: String,
        /// "NONE" in both fixtures.
        unknown_0x90: String,
        unknown_0x94: String,
        /// Social class, e.g. "Lower middle class".
        social_class: String,
        unknown_0x1618: [u32; 5],
        /// Height in inches, weight in pounds, age in years.
        height: u32,
        weight: u32,
        age: u32,
        unknown_0x163c: u32,
        /// Strength and its Skills & Powers subabilities, Stamina and
        /// Muscle (members 0x172c, 0x1734, 0x1738). The score includes the
        /// racial adjustment; with the Core Rules the program writes the
        /// same value to all three.
        strength: [u32; 3],
        /// 1 to 100 for 18/01 to 18/00 (member 0x1730).
        exceptional_strength: u32,
        /// Dexterity (Aim, Balance), Constitution (Health, Fitness),
        /// Intelligence (Reason, Knowledge), Wisdom (Intuition, Willpower)
        /// and Charisma (Leadership, Appearance): the score and its two
        /// subabilities, like strength (members 0x173c to 0x1774).
        abilities: [[u32; 3]; 5],
        unknown_0x1980: [String; 2],
        unknown_0x162c: u32,
        /// Unspent Skills & Powers character points (the nonweapon
        /// proficiency page also writes its "Points Left" here for Core
        /// Rules characters), then the copper,
        /// silver, electrum, gold and platinum totals the equipment page
        /// keeps; the creation wizard stores the starting funds here and
        /// the page recomputes them from the coin items in `inventory[6]`.
        unknown_0x1778: [u32; 6],
        /// Movement rates cached for the equipment page (`[0]` is the
        /// base rate after armor, 12, 9 or 6); the sheet recomputes them.
        unknown_0x17bc: [u32; 3],
        /// Weight carried in pounds, an `f32` bit pattern. The program
        /// trusts it (it sets encumbrance, AC and THAC0 penalties), so it
        /// must change with the equipment.
        carried_weight: u32,
        unknown_0x196c: [Vec<String>; 3],
        race: String,
        /// "Hill dwarf", "High elf".
        subrace: String,
        /// "None" when the character has no kit.
        kit: String,
        /// Skills & Powers weapon of choice and weapon expertise, then a
        /// third name empty in every file seen.
        unknown_0x124: [String; 3],
        /// Compared with 1990, 1991 and 1992 after loading.
        unknown_0x130: u32,
        /// Skills & Powers shield proficiencies, 1 when held: buckler,
        /// small, medium and body shield.
        unknown_0x134: [u32; 4],
        unknown_0x144: String,
        unknown_0x148: [u32; 6],
        unknown_0x1da4: u32,
        unknown_0x186c: u32,
        unknown_0x1874: u32,
        unknown_0x1870: u32,
        unknown_0x1dc4: u32,
        flags_0x1ddc: Option<[u8; 2]> = if v > 30,
        flag_0x1dde: Option<u8> = if v > 31,
        flags_0x1ddf: Option<[u8; 6]> = if v > 32,
        flag_0x1de1: Option<u8> = if v > 33,
        flags_0x1de6: Option<[u8; 2]> = if v > 36,
        flags_0x1de8: Option<[u8; 2]> = if v > 40,
        flags_0x1dea: Option<[u8; 3]> = if v > 34,
        unknown_0x1df0: Option<(u32, u8)> = if v > 37,
        unknown_0x1df8: Option<u32> = if v > 41,
        unknown_0x1dfc: Option<u32> = if v > 44,
        unknown_0x39c: Option<String> = if v > 46,
        unknown_0x1e50: u32,
        /// Skills & Powers unarmed combat specializations, 1 when held:
        /// punching, wrestling and martial arts, then two unused flags.
        flags_0x1e54: Option<[u8; 5]> = if v > 53,
        unknown_0x1e5c: [u32; 3],
        alignment: String,
        unknown_0x1db8: [String; 2],
        unknown_0x1e20: String,
        unknown_0x1e88: String,
        unknown_0x1e68: Option<[String; 13]> = if v > 27,
        sheet_options: Option<SheetOptions> = if v > 28,
        classes: ClassList,
        weapon_proficiencies: Vec<String>,
        /// Skills & Powers weapon lists; `[3]` holds the weapons the
        /// character specializes in, `[4]` the armor proficiencies.
        unknown_0xac: [Vec<String>; 7],
        unknown_0x19c4: Vec<String>,
        racial_abilities: Vec<String>,
        /// Wizard spells known, e.g. "Burning Hands".
        spells: Vec<String>,
        /// Priest spells known.
        priest_spells: Vec<String>,
        /// Memorized spells, one entry per copy: per class (index into
        /// `classes.names`) and spell level 0 to 10 (0 unused), the wizard
        /// spells (member 0x3c8) and the priest spells (member 0x814).
        /// What is written in a spellbook belongs to the spellbook item.
        unknown_0x3c8: [[(Vec<String>, Vec<String>); 11]; 5],
        unknown_0x1878: Vec<String>,
        /// Thief and bard skills, named as the class abilities that grant
        /// them ("Thief, Pick pockets* (10)"), with the discretionary
        /// points added in `values[0]`, for Core Rules and Skills & Powers
        /// characters alike.
        rogue_skills: RogueSkills,
        unknown_0xc60: [[u32; 100]; 5],
        unknown_0x1430: [u32; 100],
        /// What each nonweapon proficiency cost (slots, or Skills & Powers
        /// character points), parallel to `nonweapon_proficiencies`.
        nonweapon_costs: Vec<u32>,
        /// Bonus ranks per nonweapon proficiency, normally 0; parallel.
        nonweapon_bonuses: Option<Vec<u32>> = if v > 50,
        /// Per class slot: its abilities and three more lists.
        class_slots: [ClassSlot; 5],
        /// Armor and shield, e.g. "Full armor, brigandine": the Armor
        /// parts among the worn items, rebuilt by the program from
        /// `inventory[5]` whenever it needs them.
        armor: Vec<String>,
        /// The other worn magical items, rebuilt the same way.
        unknown_0x17a4: Vec<String>,
        /// Names of Non-Weapon Proficiencies parts, in the order taken.
        /// `nonweapon_costs`, `nonweapon_bonuses` and `nonweapon_details`
        /// have one entry each per proficiency.
        nonweapon_proficiencies: Vec<String>,
        /// Skills & Powers traits (`[0]`, Traits parts) and disadvantages
        /// (`[1]`, Disadvantages parts).
        unknown_0x1830: [Vec<String>; 3],
        unknown_0x98: Vec<String>,
        /// One `CStringArray` object per nonweapon proficiency: the
        /// languages learned through Modern Languages and the like, empty
        /// for the others.
        nonweapon_details: Vec<Object>,
        /// Native languages, names of Languages parts.
        languages: Vec<String>,
        unknown_0x1930: Option<[Vec<String>; 2]> = if v > 29,
        unknown_0x1dc8: Option<Vec<String>> = if v > 35,
        unknown_0x1958: Option<Vec<String>> = if v > 38,
        /// Nine equipment bundles: animals, daily food and lodging, items
        /// carried, readied, stored and worn, spending money (coins named
        /// "* Gold Pieces" and so on), services, transport. Each is sorted
        /// by name in byte order.
        inventory: [InventoryBundle; 9],
        unknown_0x1ea4: Option<(u8, [Vec<String>; 2])> = if v > 39,
        flag_0x1610: Option<u8> = if v > 42,
        unknown_0x160c: Option<u32> = if v >= 44,
        unknown_0x15e4: Option<[Vec<Object>; 2]> = if v >= 46,
        unknown_0x1e28: Option<Vec<Object>> = if v > 47,
        unknown_0x1e3c: Option<Vec<Object>> = if v > 49,
        flag_0x1ea8: Option<u8> = if v > 48,
        unknown_0x1e00: Option<Vec<String>> = if v > 51,
        unknown_0x2174: Option<Vec<Object>> = if v > 52,
    }
}

record! {
    /// What the character holds per class slot (members 0x1a8, 0x270,
    /// 0x2d4 and 0x338, 0x14 bytes per slot).
    pub struct ClassSlot {
        /// Class abilities, e.g. "Fighter, Followers (5)".
        abilities: Vec<String>,
        unknown_0x270: Vec<String>,
        unknown_0x2d4: Vec<String>,
        unknown_0x338: Vec<String>,
    }
}

/// The character's classes: a `u32` count, ten values at member 0x50, then
/// one class name per count (resolved to `CClassOb`s by name).
#[derive(Debug, Clone, PartialEq)]
pub struct ClassList {
    /// Skills & Powers fighting styles, in the order they were learned,
    /// 0 for none: 1993 one-handed weapon, 1994 weapon and shield, 1995
    /// two-handed weapon, 1996 two weapon, 1997 missile, 1998 horse
    /// archer, 1999 thrown weapon/sling, 2000 two weapon (+2), 2001
    /// one-handed weapon (+2).
    pub unknown_0x50: [u32; 10],
    pub names: Vec<String>,
}

impl Field for ClassList {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let count = r.read_u32()?;
        let unknown_0x50 = Field::read(r)?;
        let mut names = Vec::with_capacity(count.min(16) as usize);
        for _ in 0..count {
            names.push(r.read_string()?);
        }
        Ok(Self {
            unknown_0x50,
            names,
        })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_u32(self.names.len() as u32);
        self.unknown_0x50.write(w)?;
        self.names.iter().try_for_each(|n| w.write_string(n))
    }
}

/// `CCharacterDocRogueSkillsOb` (Serialize 0x59a8ab), stored inline: a
/// `u32` count, then per entry a name and 100 values.
#[derive(Debug, Clone, PartialEq)]
pub struct RogueSkills {
    pub entries: Vec<(String, [u32; 100])>,
}

impl Field for RogueSkills {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let count = r.read_u32()?;
        let mut entries = Vec::with_capacity(count.min(64) as usize);
        for _ in 0..count {
            entries.push(Field::read(r)?);
        }
        Ok(Self { entries })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_u32(self.entries.len() as u32);
        self.entries.iter().try_for_each(|e| e.write(w))
    }
}

/// `CCharacterSheetOptions` (Serialize 0x599eb8), stored inline. Its own
/// version is 2 in the Expansion; version 1 stored the entries as a
/// `CObArray`.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetOptions {
    pub version: u32,
    pub unknown_0x260: u32,
    pub unknown_0x25c: u32,
    pub entries: SheetEntries,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SheetEntries {
    /// Version 1.
    Objects(Vec<Object>),
    /// Later versions: a `u32` count and (name, value) pairs.
    Pairs(Vec<(String, u32)>),
}

impl Field for SheetOptions {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let version = r.read_u32()?;
        let unknown_0x260 = r.read_u32()?;
        let unknown_0x25c = r.read_u32()?;
        let entries = if version == 1 {
            SheetEntries::Objects(Field::read(r)?)
        } else {
            let count = r.read_u32()?;
            let mut pairs = Vec::with_capacity(count.min(256) as usize);
            for _ in 0..count {
                pairs.push(Field::read(r)?);
            }
            SheetEntries::Pairs(pairs)
        };
        Ok(Self {
            version,
            unknown_0x260,
            unknown_0x25c,
            entries,
        })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_u32(self.version);
        w.write_u32(self.unknown_0x260);
        w.write_u32(self.unknown_0x25c);
        match (&self.entries, self.version == 1) {
            (SheetEntries::Objects(objects), true) => objects.write(w),
            (SheetEntries::Pairs(pairs), false) => {
                w.write_u32(pairs.len() as u32);
                pairs.iter().try_for_each(|p| p.write(w))
            }
            (_, expected) => Err(Error::FieldMismatch {
                field: "sheet_options.entries",
                expected,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &[u8] = include_bytes!("../../../tests/fixtures/characters/index.chr");
    const BORIN: &[u8] = include_bytes!("../../../tests/fixtures/characters/90688519.dat");
    const LIRAEL: &[u8] = include_bytes!("../../../tests/fixtures/characters/90988519.dat");

    #[test]
    fn index_reads_both_characters() {
        let index = CharacterIndex::parse(INDEX).unwrap();
        assert_eq!(index.schema, 95);
        let names: Vec<_> = index.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Borin Pedraforte", "Lirael Folhaverde"]);
        assert_eq!(index.entries[1].file, "90988519.dat");
        assert_eq!(index.entries[1].version, 3);
        assert_eq!(index.entries[1].details[0], "Skills & Powers");
        assert_eq!(index.entries[1].details[4], "Class: Fighter/Mage");
    }

    #[test]
    fn index_round_trips_byte_for_byte() {
        let index = CharacterIndex::parse(INDEX).unwrap();
        assert_eq!(index.to_bytes().unwrap(), INDEX);
    }

    #[test]
    fn old_layout_writes_the_npc_mark() {
        let mut index = CharacterIndex::parse(INDEX).unwrap();
        let entry = &mut index.entries[0];
        entry.version = 1;
        entry.extra = None;
        entry.unknown_0x24 = 1;
        let read = CharacterIndex::parse(&index.to_bytes().unwrap()).unwrap();
        assert_eq!(read.entries[0].unknown_0x24, 1);
        let entry = &mut index.entries[0];
        entry.unknown_0x24 = 0;
        let read = CharacterIndex::parse(&index.to_bytes().unwrap()).unwrap();
        assert_eq!(read.entries[0].unknown_0x24, 0);
    }

    #[test]
    fn version_strings() {
        assert_eq!(version_of("TSRP3 V54"), 54);
        assert_eq!(version_of("TSRP V27"), 27);
        assert_eq!(version_of("TSRP2 V29"), 29);
        assert_eq!(version_of("TSRP3 V41"), 41);
    }

    #[test]
    fn characters_read_completely() {
        let borin = Character::parse(BORIN).unwrap();
        assert_eq!(borin.version, "TSRP3 V54");
        assert!(!borin.skills_and_powers());
        let d = &borin.data;
        assert_eq!(d.player, "Fixture One");
        assert_eq!(d.name, "Borin Pedraforte");
        assert_eq!(
            (d.race.as_str(), d.subrace.as_str()),
            ("Dwarf", "Hill dwarf")
        );
        assert_eq!((d.height, d.weight, d.age), (47, 164, 63));
        assert_eq!(d.strength, [17; 3]);
        assert_eq!(d.abilities[4], [8; 3]);
        assert_eq!(d.alignment, "Lawful Good");
        assert_eq!(d.classes.names, ["Fighter"]);
        assert_eq!(d.weapon_proficiencies, ["Axe, battle"]);
        assert_eq!(d.class_slots[0].abilities[0], "Fighter, Followers (5)");
        let pack = &d.inventory[2].items[0];
        assert_eq!(pack.name, "Backpack");
        assert!(pack.contents.is_some());

        let lirael = Character::parse(LIRAEL).unwrap();
        assert!(lirael.skills_and_powers());
        assert_eq!(lirael.data.class_title, "Fighter/Mage");
        assert_eq!(lirael.data.classes.names.len(), 2);
        assert_eq!(lirael.data.spells, ["Burning Hands", "Armor"]);
    }

    #[test]
    fn blank_characters_have_the_layout_of_their_version() {
        let expansion = Character::blank(EXPANSION_VERSION).unwrap();
        assert!(expansion.data.flags_0x1e54.is_some());
        assert!(expansion.data.name.is_empty());
        assert_eq!(
            Character::parse(&expansion.to_bytes().unwrap()).unwrap(),
            expansion
        );
        let core20 = Character::blank(CORE20_VERSION).unwrap();
        assert_eq!(core20.data.nonweapon_bonuses, None);
        assert!(core20.data.sheet_options.is_some());
        assert_eq!(
            Character::parse(&core20.to_bytes().unwrap()).unwrap(),
            core20
        );
    }

    #[test]
    fn characters_round_trip_byte_for_byte() {
        for data in [BORIN, LIRAEL] {
            let character = Character::parse(data).unwrap();
            assert_eq!(character.to_bytes().unwrap(), data);
        }
    }

    const V20_INDEX: &[u8] = include_bytes!("../../../tests/fixtures/characters/v20/index.chr");
    const TORDEK: &[u8] = include_bytes!("../../../tests/fixtures/characters/v20/88010619.dat");
    const MIALEE: &[u8] = include_bytes!("../../../tests/fixtures/characters/v20/99610619.dat");
    const TORDEK_FIRST_SAVE: &[u8] =
        include_bytes!("../../../tests/fixtures/characters/v20/first-save/88010619.dat");
    const V20_FIRST_INDEX: &[u8] =
        include_bytes!("../../../tests/fixtures/characters/v20/first-save/index.chr");

    /// Files Core Rules 2.0 (without the Expansion) wrote.
    #[test]
    fn core20_characters() {
        for data in [V20_INDEX, V20_FIRST_INDEX] {
            let index = CharacterIndex::parse(data).unwrap();
            assert_eq!(index.to_bytes().unwrap(), data);
        }
        let index = CharacterIndex::parse(V20_INDEX).unwrap();
        let names: Vec<_> = index.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Tordek Martelo", "Mialee Luar"]);
        assert_eq!(index.entries[1].details[2], "Class: Fighter/Mage");

        for data in [TORDEK, MIALEE, TORDEK_FIRST_SAVE] {
            let character = Character::parse(data).unwrap();
            assert_eq!(character.version, "TSRP2 V29");
            assert_eq!(character.to_bytes().unwrap(), data);
        }
        let tordek = Character::parse(TORDEK).unwrap().data;
        assert_eq!(tordek.player, "Fixture Three");
        assert_eq!(tordek.race, "Dwarf");
        assert_eq!(tordek.classes.names, ["Fighter"]);
        assert_eq!(tordek.abilities[1], [16; 3]);
        let mialee = Character::parse(MIALEE).unwrap().data;
        assert_eq!(mialee.class_title, "Fighter/Mage");
        assert_eq!(mialee.alignment, "Chaotic Good");
    }
}
