//! Character files: `Chars/index.chr` and one `<id>.dat` per character.

use crate::{ArchiveReader, ArchiveWriter, ClassInfo, Error, ObjectTag, Result};

const MINI_CLASS: &str = "CCharacterMiniOb";

/// One entry of `index.chr` (`CCharacterMiniOb`), the summary the
/// character list shows without opening each file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterEntry {
    pub name: String,
    /// File name inside `Chars/`, e.g. `90688519.dat`.
    pub file: String,
    pub race: String,
    /// Fields not identified yet, kept verbatim for round-tripping.
    pub unknown_a: i32,
    pub unknown_b: i32,
    pub unknown_c: i32,
    pub unknown_d: u16,
    /// Display lines: rules system, player, party, race, class, level, kit.
    pub details: Vec<String>,
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
        let mut entries = Vec::with_capacity(count as usize);
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
            entries.push(CharacterEntry {
                name: r.read_string()?,
                file: r.read_string()?,
                race: r.read_string()?,
                unknown_a: r.read_i32()?,
                unknown_b: r.read_i32()?,
                unknown_c: r.read_i32()?,
                unknown_d: r.read_u16()?,
                details: r.read_string_array()?,
            });
        }
        if !r.is_at_end() {
            return Err(Error::TrailingBytes {
                trailing: r.remaining(),
            });
        }
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
            w.write_string(&e.name)?;
            w.write_string(&e.file)?;
            w.write_string(&e.race)?;
            w.write_i32(e.unknown_a);
            w.write_i32(e.unknown_b);
            w.write_i32(e.unknown_c);
            w.write_u16(e.unknown_d);
            w.write_string_array(&e.details)?;
        }
        Ok(w.into_bytes())
    }
}

/// Which rule set a character was built with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulesSystem {
    CoreRules,
    SkillsAndPowers,
    Other(i32),
}

impl From<i32> for RulesSystem {
    fn from(v: i32) -> Self {
        match v {
            0 => Self::CoreRules,
            1 => Self::SkillsAndPowers,
            n => Self::Other(n),
        }
    }
}

/// The identified start of a character file (`CCharacterDoc::Serialize`).
/// The rest of the file is not mapped yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterHeader {
    /// Program and file version, e.g. `TSRP3 V54` for the Expansion.
    pub version: String,
    pub rules: RulesSystem,
    /// Empty in both fixtures; inferred to be the party name because the
    /// index shows an empty "Party:" line for the same characters.
    pub party: String,
    pub player: String,
    pub name: String,
    pub class: String,
}

impl CharacterHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = ArchiveReader::new(data);
        Ok(Self {
            version: r.read_string()?,
            rules: r.read_i32()?.into(),
            party: r.read_string()?,
            player: r.read_string()?,
            name: r.read_string()?,
            class: r.read_string()?,
        })
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
        assert_eq!(index.entries[1].details[0], "Skills & Powers");
        assert_eq!(index.entries[1].details[4], "Class: Fighter/Mage");
    }

    #[test]
    fn index_round_trips_byte_for_byte() {
        let index = CharacterIndex::parse(INDEX).unwrap();
        assert_eq!(index.to_bytes().unwrap(), INDEX);
    }

    #[test]
    fn character_headers() {
        let borin = CharacterHeader::parse(BORIN).unwrap();
        assert_eq!(borin.version, "TSRP3 V54");
        assert_eq!(borin.rules, RulesSystem::CoreRules);
        assert_eq!(borin.player, "Fixture One");
        assert_eq!(borin.name, "Borin Pedraforte");
        assert_eq!(borin.class, "Fighter");

        let lirael = CharacterHeader::parse(LIRAEL).unwrap();
        assert_eq!(lirael.rules, RulesSystem::SkillsAndPowers);
        assert_eq!(lirael.class, "Fighter/Mage");
    }
}
