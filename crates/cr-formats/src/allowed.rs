//! Classes allowed per race: `UserDbas/AllowedU.dat` (`CClassesAllowedDoc`,
//! `Serialize` at 0x680e21 in TSRP3.EXE), one `CObArray` of
//! `CClassesAllowedOb`, read with [`ObjectList`](crate::database::ObjectList).
//!
//! `CClassesAllowedHelperOb` also lists the classes a kit allows.

use crate::field::Field;
use crate::{ArchiveReader, ArchiveWriter, Object, Result, record};

record! {
    /// The classes one race may take (`CClassesAllowedOb`, schema 95).
    pub struct ClassesAllowed {
        /// 0 in every shipped record; the program does not test it.
        version: i32,
        /// "Aarakocra".
        race: String,
        /// Empty in every shipped record.
        unknown_0x1c: String,
        /// 0 in every shipped record.
        flag_0x20: u8,
        /// Allowed classes and multi-class combinations
        /// (`CClassesAllowedHelperOb`).
        classes: Vec<Object>,
    }
}

/// One allowed class or multi-class combination
/// (`CClassesAllowedHelperOb`, schema 95, `Serialize` at 0x671f50).
#[derive(Debug, Clone, PartialEq)]
pub struct ClassesAllowedHelper {
    /// 1 in every shipped record; `level_limit` exists from version 1 on.
    pub version: i32,
    pub data: ClassesAllowedHelperData,
}

impl Field for ClassesAllowedHelper {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let version = r.read_i32()?;
        let data = ClassesAllowedHelperData::read_fields(r, version)?;
        Ok(Self { version, data })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_i32(self.version);
        self.data.write_fields(w, self.version)
    }
}

record! {
    /// The fields of a [`ClassesAllowedHelper`] after its version: one class
    /// name per class group, empty for none.
    pub struct ClassesAllowedHelperData(v) {
        /// "Fighter", "Ranger", "Paladin".
        warrior: String,
        /// "Thief", "Bard".
        rogue: String,
        /// "Cleric", "Druid".
        priest: String,
        /// "Mage", "Illusionist".
        wizard: String,
        /// "Psionicist".
        psionicist: String,
        /// Maximum level in `AllowedU.dat` (Aarakocra: Cleric 7, Fighter
        /// 11; Psionicist 0); 0 in kits.
        level_limit: Option<i32> = if v > 0,
        /// Empty in every shipped record.
        unknown_0x20: String,
        /// 1 in `AllowedU.dat`, 0 in kits.
        flag_0x4: u8,
        /// 0 in every shipped record.
        flag_0x24: u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_0_helper_has_no_level_limit() {
        // version 0, five class names, "" , two flags
        let bytes = [0, 0, 0, 0, 1, b'F', 0, 0, 0, 0, 0, 1, 0];
        let helper = ClassesAllowedHelper::read(&mut ArchiveReader::new(&bytes)).unwrap();
        assert_eq!(helper.data.warrior, "F");
        assert_eq!(helper.data.level_limit, None);
        assert_eq!(helper.data.flag_0x4, 1);
        let mut w = ArchiveWriter::new();
        helper.write(&mut w).unwrap();
        assert_eq!(w.into_bytes(), bytes);
    }

    #[test]
    fn version_1_helper_has_a_level_limit() {
        let bytes = [1, 0, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 0, 1, 0];
        let helper = ClassesAllowedHelper::read(&mut ArchiveReader::new(&bytes)).unwrap();
        assert_eq!(helper.data.level_limit, Some(11));
        let mut w = ArchiveWriter::new();
        helper.write(&mut w).unwrap();
        assert_eq!(w.into_bytes(), bytes);
    }
}
