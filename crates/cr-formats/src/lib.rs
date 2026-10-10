//! Readers and writers for the data files of AD&D Core Rules 2.0.
//!
//! Every format here must round-trip byte for byte: reading a file and
//! writing it back produces the same bytes, so the original program keeps
//! working with files this crate has touched.

pub mod allowed;
pub mod archive;
pub mod character;
pub mod classes;
pub mod convert;
pub mod database;
pub mod encounter;
pub mod field;
pub mod inventory;
pub mod monster;
pub mod object;
pub mod parts;
pub mod phrase;
pub mod psionic;
pub mod race;
pub mod spells;
pub mod traps;
pub mod treasure;

pub use archive::{ArchiveReader, ArchiveWriter, ClassInfo, ObjectTag};
pub use field::Field;
pub use object::Object;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unexpected end of file at offset {offset:#x} (wanted {wanted} bytes)")]
    UnexpectedEof { offset: usize, wanted: usize },
    #[error("Unicode CString at offset {offset:#x}; Core Rules writes only ANSI strings")]
    UnicodeString { offset: usize },
    #[error("unknown class index {index} at offset {offset:#x}")]
    BadClassIndex { offset: usize, index: usize },
    #[error("expected class {expected}, found {found}")]
    UnexpectedClass {
        expected: &'static str,
        found: String,
    },
    #[error("unexpected object tag {tag:?} at offset {offset:#x}")]
    UnexpectedTag { offset: usize, tag: ObjectTag },
    #[error("{0:?} cannot be written in Windows-1252")]
    Unencodable(String),
    #[error("{trailing} unread bytes at the end of the file")]
    TrailingBytes { trailing: usize },
    #[error("class {0} is not supported yet")]
    UnknownClass(String),
    #[error("class {class} has schema {found}, expected {expected}")]
    UnexpectedSchema {
        class: String,
        expected: u16,
        found: u16,
    },
    #[error("field {field} must be {} for this file version", if *expected { "present" } else { "absent" })]
    FieldMismatch { field: &'static str, expected: bool },
}

pub type Result<T> = std::result::Result<T, Error>;

/// C `atol` as the original program uses it on version suffixes: optional
/// leading whitespace and sign, then decimal digits up to the first
/// non-digit; 0 when there are none.
pub(crate) fn atol(s: &str) -> i32 {
    let s = s.trim_start();
    let (negative, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let value = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i32, |n, d| {
            n.wrapping_mul(10).wrapping_add(i32::from(d - b'0'))
        });
    if negative {
        value.wrapping_neg()
    } else {
        value
    }
}
