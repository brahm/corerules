//! `Database/Convert.dat` (`CConversionMapDoc`, Serialize 0x4561bd): eight
//! `CMapStringToOb` written back to back with one archive, mapping names
//! from older data to the current names.
//!
//! In the shipped file every value is a `CStringArray` object. The maps are
//! kept as lists in file order: MFC writes them in hash-bucket order and
//! reverses each bucket's chain on load, so only the order as read
//! round-trips byte for byte.

use crate::field::Field;
use crate::{ArchiveReader, ArchiveWriter, Object, Result};

/// Number of maps in the document (members +0x50, +0x6c, ..., +0x114).
pub const MAP_COUNT: usize = 8;

/// One `CMapStringToOb` (Serialize 0x73754e): count, then key and object
/// per entry.
pub type StringToObjectMap = Vec<(String, Object)>;

/// The conversion maps, by position in the document (contents inferred
/// from the data).
#[derive(Debug, Clone, PartialEq)]
pub struct ConversionMaps {
    /// 0: item names -> Parts names (`Sling stone` -> `Sling, sling stone`).
    /// 1: monster names. 2: races (`Dwarf` -> `Hill dwarf`). 3: wizard
    /// spells. 4: priest spells. 5: monster names (`Whale, Narwhal 4 HD` ->
    /// `Whale, Narwhal`). 6: nonweapon proficiencies. 7: languages ->
    /// dialects.
    pub maps: [StringToObjectMap; MAP_COUNT],
}

impl ConversionMaps {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = ArchiveReader::new(data);
        let maps = Field::read(&mut r)?;
        r.finish()?;
        Ok(Self { maps })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut w = ArchiveWriter::new();
        self.maps.write(&mut w)?;
        Ok(w.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_array_class_is_defined_once_across_maps() {
        let value = |s: &str| Object::StringArray(vec![s.into()]);
        let mut maps: [StringToObjectMap; MAP_COUNT] = Default::default();
        maps[0] = vec![("Sling stone".into(), value("Sling, sling stone"))];
        maps[2] = vec![
            ("Dwarf".into(), value("Hill dwarf")),
            ("Elf".into(), Object::StringArray(vec![])),
        ];
        let doc = ConversionMaps { maps };
        let bytes = doc.to_bytes().unwrap();
        let name = b"CStringArray";
        assert_eq!(bytes.windows(name.len()).filter(|w| w == name).count(), 1);
        assert!(bytes.windows(2).any(|w| w == [0x01, 0x80]));
        assert_eq!(ConversionMaps::parse(&bytes).unwrap(), doc);
    }
}
