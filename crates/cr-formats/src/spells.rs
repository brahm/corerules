//! Spells: `Database/Spells.dat` and `UserDbas/SpellsU.dat` (`CSpellsDoc`,
//! `Serialize` at 0x4f9d89 in TSRP3.EXE): the wizard spells, then the priest
//! spells, as two `CObArray`s of `CSpellsOb` with no outer count.
//!
//! `CSpellsOb` also appears inside kits (bonus spells) and parts.

use crate::field::Field;
use crate::{ArchiveReader, ArchiveWriter, Object, Result, record};

/// A spells file.
#[derive(Debug, Clone, PartialEq)]
pub struct SpellsFile {
    /// 480 in the shipped `Spells.dat`.
    pub wizard: Vec<Object>,
    /// 453 in the shipped `Spells.dat`.
    pub priest: Vec<Object>,
}

impl SpellsFile {
    pub fn parse(data: &[u8]) -> Result<Self> {
        // One reader for both arrays: the priest list refers back to the
        // class tags of the wizard list.
        let mut r = ArchiveReader::new(data);
        let wizard = Field::read(&mut r)?;
        let priest = Field::read(&mut r)?;
        r.finish()?;
        Ok(Self { wizard, priest })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut w = ArchiveWriter::new();
        self.wizard.write(&mut w)?;
        self.priest.write(&mut w)?;
        Ok(w.into_bytes())
    }
}

record! {
    /// One spell (`CSpellsOb`, schema 88), e.g. Infravision: wizard level
    /// 3, range "Touch", duration "2 hours + 1 hour/level".
    pub struct Spell {
        name: String,
        /// 0 in every shipped spell but one.
        unknown_0x60: i32,
        /// 1 on 48 wizard and 112 priest spells.
        unknown_0x64: i32,
        /// 1 on every wizard spell, 0 on priest spells.
        wizard: i32,
        /// 1 on every priest spell, 0 on wizard spells.
        priest: i32,
        /// Spell level (wizard 1-9, priest 0-7).
        level: i32,
        /// "10 ft radius", "Creature touched".
        area_of_effect: String,
        /// "1", "1 rd".
        casting_time: String,
        /// "V, S, M".
        components: String,
        /// Mostly empty; "None", "Medium (1 hit)".
        unknown_0x44: String,
        duration: String,
        /// Mostly empty; "None", "d8", "d10".
        unknown_0x4c: String,
        /// "5 yds/lvl", "Touch".
        range: String,
        /// "None".
        saving_throw: String,
        /// Mostly empty; "None", "Small visual".
        unknown_0x58: String,
        /// Mostly empty; "+2", "+5".
        unknown_0x5c: String,
        /// Database identifier (Affect Normal Fires 105, Bless 8030).
        id: i32,
        /// Mostly empty; a short note ("Restores 1d8 hit points.").
        unknown_0xc: String,
        /// Schools ("Alteration", "Elemental Fire").
        schools: Vec<String>,
        /// Priest spheres ("All", "Animal"); empty for wizard spells.
        spheres: Vec<String>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_user_file() {
        let file = SpellsFile::parse(&[0, 0, 0, 0]).unwrap();
        assert!(file.wizard.is_empty() && file.priest.is_empty());
        assert_eq!(file.to_bytes().unwrap(), [0, 0, 0, 0]);
    }

    #[test]
    fn priest_list_reuses_the_wizard_class_tag() {
        let spell = |name: &str| {
            Object::Spell(Box::new(Spell {
                name: name.into(),
                unknown_0x60: 0,
                unknown_0x64: 0,
                wizard: 0,
                priest: 0,
                level: 1,
                area_of_effect: String::new(),
                casting_time: String::new(),
                components: String::new(),
                unknown_0x44: String::new(),
                duration: String::new(),
                unknown_0x4c: String::new(),
                range: String::new(),
                saving_throw: String::new(),
                unknown_0x58: String::new(),
                unknown_0x5c: String::new(),
                id: 1,
                unknown_0xc: String::new(),
                schools: vec![],
                spheres: vec![],
            }))
        };
        let wizard_only = SpellsFile {
            wizard: vec![spell("Sleep")],
            priest: vec![],
        };
        let file = SpellsFile {
            priest: vec![spell("Bless")],
            ..wizard_only.clone()
        };
        let bytes = file.to_bytes().unwrap();
        // The second array opens with count 1 and the tag of class index 1.
        let priest = wizard_only.to_bytes().unwrap().len() - 2;
        assert_eq!(&bytes[priest..priest + 4], &[1, 0, 0x01, 0x80]);
        assert_eq!(SpellsFile::parse(&bytes).unwrap(), file);
    }
}
