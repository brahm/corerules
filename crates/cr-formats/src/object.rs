//! Polymorphic objects: what `CArchive::WriteObject` stores in a `CObList`
//! or `CObArray`, a class tag followed by the object's fields.

use crate::field::Field;
use crate::{ArchiveReader, ArchiveWriter, ClassInfo, Error, ObjectTag, Result};

macro_rules! objects {
    ($( $(#[$meta:meta])* $variant:ident($ty:ty) = $class:literal, $schema:literal; )*) => {
        /// An object of any class this crate can read.
        #[derive(Debug, Clone, PartialEq)]
        pub enum Object {
            $( $(#[$meta])* $variant($ty), )*
        }

        impl Object {
            /// The MFC class name and schema written in the class tag.
            pub fn class(&self) -> (&'static str, u16) {
                match self {
                    $( Self::$variant(_) => ($class, $schema), )*
                }
            }

            fn read_body(class: &ClassInfo, r: &mut ArchiveReader) -> Result<Self> {
                match class.name.as_str() {
                    $( $class => Ok(Self::$variant(<$ty as Field>::read(r)?)), )*
                    _ => Err(Error::UnknownClass(class.name.clone())),
                }
            }

            fn write_body(&self, w: &mut ArchiveWriter) -> Result<()> {
                match self {
                    $( Self::$variant(v) => v.write(w), )*
                }
            }
        }
    };
}

objects! {
    /// `CStringArray` stored as an object.
    StringArray(Vec<String>) = "CStringArray", 0;
    Race(Box<crate::race::Race>) = "CRaceOb", 88;
    MultiClass(crate::race::MultiClass) = "CClassMultiClassOb", 94;
    Class(Box<crate::classes::Class>) = "CClassOb", 88;
    KitBonus(crate::classes::KitBonus) = "CPartKitBonusOb", 1;
    InventoryBundle(crate::inventory::InventoryBundle) = "CInventoryBundle", 1;
    TreasureTable(crate::treasure::TreasureTable) = "CTreasureSingleTableOb", 1;
    TreasureTableEntry(Box<crate::treasure::TreasureTableEntry>) = "CTreasureTableEntry", 1;
    Treasure(crate::treasure::Treasure) = "CTreasureOb", 95;
    PsionicPower(Box<crate::psionic::PsionicPower>) = "CPsionicPowerOb", 96;
    Trap(Box<crate::traps::Trap>) = "CTrapOb", 95;
    PhraseSpacing(Box<crate::phrase::PhraseSpacing>) = "CPhraseSpacingOb", 88;
}

impl Object {
    fn read_new(class: ClassInfo, r: &mut ArchiveReader) -> Result<Self> {
        let object = Self::read_body(&class, r)?;
        let (_, schema) = object.class();
        if class.schema != schema {
            return Err(Error::UnexpectedSchema {
                class: class.name,
                expected: schema,
                found: class.schema,
            });
        }
        Ok(object)
    }
}

impl Field for Object {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let offset = r.position();
        match r.read_object_tag()? {
            ObjectTag::New(class) => Self::read_new(class, r),
            tag => Err(Error::UnexpectedTag { offset, tag }),
        }
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        let (name, schema) = self.class();
        w.write_object_tag(&ClassInfo {
            name: name.into(),
            schema,
        });
        self.write_body(w)
    }
}

/// A pointer member that may be `NULL`.
impl Field for Option<Box<Object>> {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let offset = r.position();
        match r.read_object_tag()? {
            ObjectTag::Null => Ok(None),
            ObjectTag::New(class) => Object::read_new(class, r).map(|o| Some(Box::new(o))),
            tag => Err(Error::UnexpectedTag { offset, tag }),
        }
    }

    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        match self {
            None => {
                w.write_null_object();
                Ok(())
            }
            Some(object) => object.write(w),
        }
    }
}
