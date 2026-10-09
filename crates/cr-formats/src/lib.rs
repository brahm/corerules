//! Readers and writers for the data files of AD&D Core Rules 2.0.
//!
//! Every format here must round-trip byte for byte: reading a file and
//! writing it back produces the same bytes, so the original program keeps
//! working with files this crate has touched.

pub mod archive;
pub mod character;
pub mod database;

pub use archive::{ArchiveReader, ArchiveWriter, ClassInfo, ObjectTag};

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
}

pub type Result<T> = std::result::Result<T, Error>;
