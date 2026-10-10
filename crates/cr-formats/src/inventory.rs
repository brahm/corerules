//! Equipment: `CInventoryBundle` (versionable schema 1) holds a list of
//! `CInventoryItem`s, each of which may hold a bundle of its own (a
//! backpack and what is in it). Characters store nine bundles inline;
//! monsters, treasures and items store them as objects.

use crate::field::Field;
use crate::{ArchiveReader, ArchiveWriter, Object, Result, record};

record! {
    /// `CInventoryBundle::Serialize` (0x4aa81b): the item list at +0x50,
    /// count and items, items inline without class tags.
    pub struct InventoryBundle {
        items: Vec<InventoryItem>,
    }
}

/// `CInventoryItem::Serialize` (0x4acb0e).
#[derive(Debug, Clone, PartialEq)]
pub struct InventoryItem {
    pub name: String,
    /// How many (3 torches).
    pub quantity: u32,
    /// What the item contains, as a `CInventoryBundle` object.
    pub contents: Option<Box<Object>>,
    /// Member +0x08; the constructor sets 999 and every shipped item keeps
    /// it.
    pub unknown_0x8: u32,
    pub flag_0x4: u8,
    /// Extra data some items carry; a presence byte precedes it.
    pub extra: Option<InventoryExtra>,
}

impl Field for InventoryItem {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        Ok(Self {
            name: r.read_string()?,
            quantity: r.read_u32()?,
            contents: Field::read(r)?,
            unknown_0x8: r.read_u32()?,
            flag_0x4: r.read_u8()?,
            extra: match r.read_u8()? {
                0 => None,
                _ => Some(InventoryExtra::read(r)?),
            },
        })
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_string(&self.name)?;
        w.write_u32(self.quantity);
        self.contents.write(w)?;
        w.write_u32(self.unknown_0x8);
        w.write_u8(self.flag_0x4);
        match &self.extra {
            None => w.write_u8(0),
            Some(extra) => {
                w.write_u8(1);
                extra.write(w)?;
            }
        }
        Ok(())
    }
}

record! {
    /// The optional 0x50-byte object behind `CInventoryItem + 0x18`
    /// (Serialize 0x4adc02).
    pub struct InventoryExtra {
        unknown_0x4: [u32; 3],
        unknown_0x10: Vec<String>,
        /// A `CDWordArray`.
        unknown_0x24: Vec<u32>,
        /// A `CByteArray`.
        unknown_0x38: Vec<u8>,
    }
}
