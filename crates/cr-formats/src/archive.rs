//! Reader and writer for MFC `CArchive` serialization, the on-disk format of
//! every Core Rules data file.
//!
//! The layout follows MFC 4.x (Visual C++ 4/5, 1996-1998):
//!
//! * Integers are little-endian. `BOOL`/`int`/`long` are 4 bytes.
//! * Counts (`WriteCount`) are a `u16`, or `0xFFFF` followed by a `u32`.
//! * ANSI `CString`s carry a length prefix: a `u8` below `0xFF`, otherwise
//!   `0xFF` + `u16`, otherwise `0xFF 0xFFFF` + `u32`. Text is Windows-1252.
//! * Objects written with `WriteObject` start with a tag. `0x0000` is null,
//!   `0xFFFF` introduces a new class (`u16` schema, `u16` name length, name),
//!   `0x8000 | n` refers to class `n` already seen, and a plain index refers
//!   to an object already stored. Classes and objects share one index
//!   counter that starts at 1.

use encoding_rs::WINDOWS_1252;

use crate::{Error, Result};

const NULL_TAG: u16 = 0x0000;
const NEW_CLASS_TAG: u16 = 0xFFFF;
const CLASS_TAG: u16 = 0x8000;
const BIG_OBJECT_TAG: u16 = 0x7FFF;
const BIG_CLASS_TAG: u32 = 0x8000_0000;

/// A class as stored in the archive by `CRuntimeClass::Store`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassInfo {
    pub name: String,
    pub schema: u16,
}

/// What `ReadObject` found where an object was expected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectTag {
    Null,
    /// A new object follows; its fields come next in the stream.
    New(ClassInfo),
    /// A back-reference to an object that was already read.
    Ref(u32),
}

/// Cursor over an archive held in memory.
pub struct ArchiveReader<'a> {
    data: &'a [u8],
    pos: usize,
    /// Index map: slot 0 is unused (null), classes and objects follow in
    /// the order they were first stored. `None` marks an object slot.
    slots: Vec<Option<ClassInfo>>,
}

impl<'a> ArchiveReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            slots: vec![None],
        }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn is_at_end(&self) -> bool {
        self.pos == self.data.len()
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.remaining() < len {
            return Err(Error::UnexpectedEof {
                offset: self.pos,
                wanted: len,
            });
        }
        let bytes = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(bytes)
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read_bytes(1)?[0])
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read_bytes(2)?.try_into().unwrap()))
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read_bytes(4)?.try_into().unwrap()))
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        Ok(self.read_u32()? as i32)
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.read_u32()?))
    }

    /// `CArchive::ReadCount`.
    pub fn read_count(&mut self) -> Result<u32> {
        match self.read_u16()? {
            0xFFFF => self.read_u32(),
            n => Ok(u32::from(n)),
        }
    }

    /// `operator>>(CArchive&, CString&)` for ANSI builds.
    pub fn read_string(&mut self) -> Result<String> {
        let offset = self.pos;
        let len = match self.read_u8()? {
            0xFF => match self.read_u16()? {
                0xFFFE => return Err(Error::UnicodeString { offset }),
                0xFFFF => self.read_u32()? as usize,
                n => usize::from(n),
            },
            n => usize::from(n),
        };
        let (text, _, _) = WINDOWS_1252.decode(self.read_bytes(len)?);
        Ok(text.into_owned())
    }

    /// `CStringArray::Serialize`.
    pub fn read_string_array(&mut self) -> Result<Vec<String>> {
        let count = self.read_count()?;
        (0..count).map(|_| self.read_string()).collect()
    }

    /// The tag `CArchive::ReadObject` reads before an object's fields.
    pub fn read_object_tag(&mut self) -> Result<ObjectTag> {
        let offset = self.pos;
        let tag = self.read_u16()?;
        let index = match tag {
            NULL_TAG => return Ok(ObjectTag::Null),
            NEW_CLASS_TAG => {
                let schema = self.read_u16()?;
                let len = usize::from(self.read_u16()?);
                let name = String::from_utf8_lossy(self.read_bytes(len)?).into_owned();
                let class = ClassInfo { name, schema };
                self.slots.push(Some(class.clone()));
                self.slots.push(None);
                return Ok(ObjectTag::New(class));
            }
            BIG_OBJECT_TAG => {
                let big = self.read_u32()?;
                if big & BIG_CLASS_TAG == 0 {
                    return Ok(ObjectTag::Ref(big));
                }
                (big & !BIG_CLASS_TAG) as usize
            }
            t if t & CLASS_TAG == 0 => return Ok(ObjectTag::Ref(u32::from(t))),
            t => usize::from(t & !CLASS_TAG),
        };
        let class = self
            .slots
            .get(index)
            .cloned()
            .flatten()
            .ok_or(Error::BadClassIndex { offset, index })?;
        self.slots.push(None);
        Ok(ObjectTag::New(class))
    }
}

/// Writes archives byte-for-byte the way MFC does.
#[derive(Default)]
pub struct ArchiveWriter {
    data: Vec<u8>,
    /// Class name to index in the shared index map.
    classes: Vec<(String, u32)>,
    next_index: u32,
}

impl ArchiveWriter {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            classes: Vec::new(),
            next_index: 1,
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    pub fn write_u8(&mut self, v: u8) {
        self.data.push(v);
    }

    pub fn write_u16(&mut self, v: u16) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_u32(&mut self, v: u32) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_i32(&mut self, v: i32) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_f32(&mut self, v: f32) {
        self.write_u32(v.to_bits());
    }

    pub fn write_count(&mut self, n: u32) {
        if n < 0xFFFF {
            self.write_u16(n as u16);
        } else {
            self.write_u16(0xFFFF);
            self.write_u32(n);
        }
    }

    pub fn write_string(&mut self, s: &str) -> Result<()> {
        let (bytes, _, unmappable) = WINDOWS_1252.encode(s);
        if unmappable {
            return Err(Error::Unencodable(s.to_owned()));
        }
        let len = bytes.len();
        if len < 0xFF {
            self.write_u8(len as u8);
        } else if len < 0xFFFE {
            self.write_u8(0xFF);
            self.write_u16(len as u16);
        } else {
            self.write_u8(0xFF);
            self.write_u16(0xFFFF);
            self.write_u32(len as u32);
        }
        self.write_bytes(&bytes);
        Ok(())
    }

    pub fn write_string_array(&mut self, items: &[String]) -> Result<()> {
        self.write_count(items.len() as u32);
        items.iter().try_for_each(|s| self.write_string(s))
    }

    pub fn write_null_object(&mut self) {
        self.write_u16(NULL_TAG);
    }

    /// Writes the tag for a new object of `class`; the caller writes its
    /// fields next.
    pub fn write_object_tag(&mut self, class: &ClassInfo) {
        let known = self
            .classes
            .iter()
            .find(|(name, _)| *name == class.name)
            .map(|(_, i)| *i);
        match known {
            Some(index) if index < u32::from(BIG_OBJECT_TAG) => {
                self.write_u16(CLASS_TAG | index as u16);
            }
            Some(index) => {
                self.write_u16(BIG_OBJECT_TAG);
                self.write_u32(BIG_CLASS_TAG | index);
            }
            None => {
                self.write_u16(NEW_CLASS_TAG);
                self.write_u16(class.schema);
                self.write_u16(class.name.len() as u16);
                self.write_bytes(class.name.as_bytes());
                self.classes.push((class.name.clone(), self.next_index));
                self.next_index += 1;
            }
        }
        self.next_index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_round_trip_with_every_length_prefix() {
        for len in [0usize, 1, 254, 255, 300, 0xFFFD, 0x1_0000] {
            let text = "a".repeat(len);
            let mut w = ArchiveWriter::new();
            w.write_string(&text).unwrap();
            let bytes = w.into_bytes();
            let mut r = ArchiveReader::new(&bytes);
            assert_eq!(r.read_string().unwrap(), text);
            assert!(r.is_at_end());
        }
    }

    #[test]
    fn windows_1252_text_is_preserved() {
        let mut w = ArchiveWriter::new();
        w.write_string("Ça café – naïve").unwrap();
        let bytes = w.into_bytes();
        assert_eq!(bytes[0] as usize, bytes.len() - 1);
        assert_eq!(
            ArchiveReader::new(&bytes).read_string().unwrap(),
            "Ça café – naïve"
        );
    }

    #[test]
    fn second_object_of_a_class_uses_a_class_reference() {
        let class = ClassInfo {
            name: "CTrapOb".into(),
            schema: 95,
        };
        let mut w = ArchiveWriter::new();
        w.write_count(2);
        w.write_object_tag(&class);
        w.write_string("Deadfall").unwrap();
        w.write_object_tag(&class);
        w.write_string("Pit").unwrap();
        let bytes = w.into_bytes();
        assert_eq!(&bytes[..4], &[0x02, 0x00, 0xFF, 0xFF]);
        assert_eq!(&bytes[24..26], &[0x01, 0x80]);

        let mut r = ArchiveReader::new(&bytes);
        assert_eq!(r.read_count().unwrap(), 2);
        assert_eq!(r.read_object_tag().unwrap(), ObjectTag::New(class.clone()));
        assert_eq!(r.read_string().unwrap(), "Deadfall");
        assert_eq!(r.read_object_tag().unwrap(), ObjectTag::New(class));
        assert_eq!(r.read_string().unwrap(), "Pit");
        assert!(r.is_at_end());
    }
}
