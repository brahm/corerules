//! Rule database files (`Database/*.dat`, `UserDbas/*U.dat`).
//!
//! Each one is a `CObList` of objects of one main class. Field layouts are
//! being mapped class by class; until a class is mapped, [`survey`] reports
//! what a file holds without decoding its records.

use crate::{ArchiveReader, ClassInfo, ObjectTag, Result};

/// What a database file contains, read from its first record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Survey {
    pub records: u32,
    /// Class of the first record; `None` for an empty list.
    pub class: Option<ClassInfo>,
}

pub fn survey(data: &[u8]) -> Result<Survey> {
    if data.is_empty() {
        return Ok(Survey {
            records: 0,
            class: None,
        });
    }
    let mut r = ArchiveReader::new(data);
    let records = r.read_count()?;
    let class = match records {
        0 => None,
        _ => match r.read_object_tag()? {
            ObjectTag::New(class) => Some(class),
            _ => None,
        },
    };
    Ok(Survey { records, class })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_user_database() {
        assert_eq!(
            survey(&[0, 0]).unwrap(),
            Survey {
                records: 0,
                class: None
            }
        );
        assert_eq!(
            survey(&[]).unwrap(),
            Survey {
                records: 0,
                class: None
            }
        );
    }

    #[test]
    fn first_class_is_reported() {
        let data = b"\x30\x00\xff\xff\x58\x00\x07\x00CRaceOb";
        let s = survey(data).unwrap();
        assert_eq!(s.records, 48);
        assert_eq!(
            s.class,
            Some(ClassInfo {
                name: "CRaceOb".into(),
                schema: 88
            })
        );
    }
}
