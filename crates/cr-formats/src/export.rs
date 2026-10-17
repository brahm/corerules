//! Export files: `.xpt` (Core Rules 2.0) and `.xpx` (the Expansion), what
//! the DM Toolkit's Export writes and its Import reads (`CImportExportDoc`,
//! `Serialize` at 0x49fc52 in TSRP3.EXE and 0x4e0130 in EVERMORE.EXE).
//!
//! The file is one archive, so class tags carry over from one section to
//! the next:
//!
//! * The Expansion starts with `-1` and the version string `TSRP3 V1`;
//!   Core Rules 2.0 has neither and starts with the character count.
//! * The characters, each stored exactly like a character file (the
//!   export flag at +0x1ea1 only changes the messages).
//! * The Campaign Mapper drawings: the file name, the size and the bytes of
//!   each `.FCW` file.
//! * The Expansion only, when the version string is `TSRP3 V1`: the
//!   race/class combinations (`AllowedU.dat` records) and the classes.
//! * Psionic powers, traps, races, wizard spells, priest spells, monsters,
//!   monster attacks, saved encounters, encounter templates, treasure
//!   bundles, an array nothing fills, and monster types, each a
//!   `CObArray`.
//! * The parts, laid out like a parts file (21 arrays, see
//!   [`crate::parts`]).
//! * The Map Maker II maps (`CMapData`), a `CObArray` this crate does not
//!   decode. They come last, so their bytes are kept as they are.

use crate::character::Character;
use crate::field::Field;
use crate::parts::PART_TYPE_COUNT;
use crate::{ArchiveReader, ArchiveWriter, Object, Result};

/// The version string of the files the Expansion writes.
pub const EXPANSION_VERSION: &str = "TSRP3 V1";

/// A Campaign Mapper drawing carried in an export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignMap {
    /// The drawing's file name (`Lordstown.FCW`); Import suggests it when
    /// it asks where to save the drawing.
    pub path: String,
    /// The `.FCW` file.
    pub data: Vec<u8>,
}

impl Field for CampaignMap {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let path = r.read_string()?;
        let len = r.read_u32()? as usize;
        let data = r.read_bytes(len)?.to_vec();
        Ok(Self { path, data })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_string(&self.path)?;
        w.write_u32(self.data.len() as u32);
        w.write_bytes(&self.data);
        Ok(())
    }
}

/// The Map Maker II maps, undecoded.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapMakerMaps {
    /// How many maps the array holds.
    pub count: u32,
    /// The maps' bytes after the count, to the end of the file. They may
    /// refer to class tags of the sections before them, so they are only
    /// valid behind the same sections.
    pub data: Vec<u8>,
}

/// An export file.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportFile {
    /// `Some("TSRP3 V1")` for the Expansion, `None` for Core Rules 2.0.
    pub version: Option<String>,
    pub characters: Vec<Character>,
    pub campaign_maps: Vec<CampaignMap>,
    /// `CClassesAllowedOb` records; present when the version string is
    /// [`EXPANSION_VERSION`].
    pub allowed: Option<Vec<Object>>,
    /// `CClassOb` records; present with `allowed`.
    pub classes: Option<Vec<Object>>,
    pub psionics: Vec<Object>,
    pub traps: Vec<Object>,
    pub races: Vec<Object>,
    pub wizard_spells: Vec<Object>,
    pub priest_spells: Vec<Object>,
    pub monsters: Vec<Object>,
    pub monster_attacks: Vec<Object>,
    pub encounters: Vec<Object>,
    pub templates: Vec<Object>,
    pub treasure: Vec<Object>,
    /// The array at +0x140, which neither program fills.
    pub unused: Vec<Object>,
    pub monster_types: Vec<Object>,
    /// The parts by type, as in a parts file.
    pub parts: [Vec<Object>; PART_TYPE_COUNT],
    pub map_maker: MapMakerMaps,
}

impl ExportFile {
    /// An empty export in the layout of the Expansion (`expansion`) or of
    /// Core Rules 2.0.
    pub fn new(expansion: bool) -> Self {
        Self {
            version: expansion.then(|| EXPANSION_VERSION.to_owned()),
            characters: Vec::new(),
            campaign_maps: Vec::new(),
            allowed: expansion.then(Vec::new),
            classes: expansion.then(Vec::new),
            psionics: Vec::new(),
            traps: Vec::new(),
            races: Vec::new(),
            wizard_spells: Vec::new(),
            priest_spells: Vec::new(),
            monsters: Vec::new(),
            monster_attacks: Vec::new(),
            encounters: Vec::new(),
            templates: Vec::new(),
            treasure: Vec::new(),
            unused: Vec::new(),
            monster_types: Vec::new(),
            parts: Default::default(),
            map_maker: MapMakerMaps::default(),
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = ArchiveReader::new(data);
        let first = r.read_i32()?;
        let (version, count) = if first < 0 {
            let version = r.read_string()?;
            (Some(version), r.read_u32()?)
        } else {
            (None, first as u32)
        };
        // A version string other than this one gets the "Dazed and
        // confused" message and is read like a Core Rules 2.0 file.
        let expansion = version.as_deref() == Some(EXPANSION_VERSION);
        let characters = (0..count)
            .map(|_| Character::read(&mut r))
            .collect::<Result<_>>()?;
        let maps = r.read_u32()?;
        let campaign_maps = (0..maps)
            .map(|_| CampaignMap::read(&mut r))
            .collect::<Result<_>>()?;
        let (allowed, classes) = if expansion {
            (Some(Field::read(&mut r)?), Some(Field::read(&mut r)?))
        } else {
            (None, None)
        };
        let mut file = Self {
            version,
            characters,
            campaign_maps,
            allowed,
            classes,
            psionics: Field::read(&mut r)?,
            traps: Field::read(&mut r)?,
            races: Field::read(&mut r)?,
            wizard_spells: Field::read(&mut r)?,
            priest_spells: Field::read(&mut r)?,
            monsters: Field::read(&mut r)?,
            monster_attacks: Field::read(&mut r)?,
            encounters: Field::read(&mut r)?,
            templates: Field::read(&mut r)?,
            treasure: Field::read(&mut r)?,
            unused: Field::read(&mut r)?,
            monster_types: Field::read(&mut r)?,
            parts: Field::read(&mut r)?,
            map_maker: MapMakerMaps::default(),
        };
        file.map_maker.count = r.read_count()?;
        file.map_maker.data = r.read_bytes(r.remaining())?.to_vec();
        Ok(file)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut w = ArchiveWriter::new();
        if let Some(version) = &self.version {
            w.write_i32(-1);
            w.write_string(version)?;
        }
        w.write_u32(self.characters.len() as u32);
        for character in &self.characters {
            character.write(&mut w)?;
        }
        w.write_u32(self.campaign_maps.len() as u32);
        for map in &self.campaign_maps {
            map.write(&mut w)?;
        }
        let expansion = self.version.as_deref() == Some(EXPANSION_VERSION);
        match (&self.allowed, &self.classes) {
            (Some(allowed), Some(classes)) if expansion => {
                allowed.write(&mut w)?;
                classes.write(&mut w)?;
            }
            (None, None) if !expansion => {}
            _ => {
                return Err(crate::Error::FieldMismatch {
                    field: "allowed and classes",
                    expected: expansion,
                });
            }
        }
        for array in [
            &self.psionics,
            &self.traps,
            &self.races,
            &self.wizard_spells,
            &self.priest_spells,
            &self.monsters,
            &self.monster_attacks,
            &self.encounters,
            &self.templates,
            &self.treasure,
            &self.unused,
            &self.monster_types,
        ] {
            array.write(&mut w)?;
        }
        self.parts.write(&mut w)?;
        w.write_count(self.map_maker.count);
        w.write_bytes(&self.map_maker.data);
        Ok(w.into_bytes())
    }

    /// Whether the file has the Expansion's layout.
    pub fn is_expansion(&self) -> bool {
        self.version.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_files_round_trip() {
        for expansion in [false, true] {
            let file = ExportFile::new(expansion);
            let bytes = file.to_bytes().unwrap();
            assert_eq!(ExportFile::parse(&bytes).unwrap(), file);
        }
        // Core Rules 2.0: no characters, no drawings, 12 + 21 + 1 empty
        // arrays.
        let bytes = ExportFile::new(false).to_bytes().unwrap();
        assert_eq!(bytes.len(), 8 + 2 * 34);
        assert_eq!(&bytes[..8], &[0; 8]);
    }
}
