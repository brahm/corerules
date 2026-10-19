//! Reads the files of an ISO 9660 CD image, such as the Core Rules CDs,
//! without mounting it.
//!
//! Names come from the Joliet tree when the image has one, else from the
//! plain ISO 9660 tree, shown as Linux shows a mounted CD: the `;1` version
//! and a trailing dot are dropped and plain names are lowercased
//! (`DATABASE/RACE.DAT;1` is `database/race.dat`).

use std::cell::RefCell;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// Sectors before the first volume descriptor.
const SYSTEM_AREA: u64 = 16;
const SECTOR: u64 = 2048;

/// A file or folder in the image.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    extent: u32,
    size: u32,
}

pub struct Image {
    file: RefCell<File>,
    block: u64,
    joliet: bool,
    root: Entry,
}

impl Image {
    /// Whether `path` starts like an ISO 9660 image.
    pub fn is_image(path: &Path) -> bool {
        let mut magic = [0u8; 6];
        File::open(path)
            .and_then(|mut f| {
                f.seek(SeekFrom::Start(SYSTEM_AREA * SECTOR))?;
                f.read_exact(&mut magic)
            })
            .is_ok()
            && &magic[1..] == b"CD001"
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut primary = None;
        let mut joliet = None;
        for n in SYSTEM_AREA.. {
            let mut d = [0u8; SECTOR as usize];
            file.seek(SeekFrom::Start(n * SECTOR))?;
            file.read_exact(&mut d)?;
            if &d[1..6] != b"CD001" {
                return Err(invalid("not an ISO 9660 image"));
            }
            match d[0] {
                1 if primary.is_none() => primary = Some(d),
                // A supplementary descriptor with a UCS-2 escape sequence.
                2 if d[88..90] == *b"%/" && matches!(d[90], b'@' | b'C' | b'E') => {
                    joliet.get_or_insert(d);
                }
                255 => break,
                _ => {}
            }
        }
        let is_joliet = joliet.is_some();
        let d = joliet
            .or(primary)
            .ok_or_else(|| invalid("no volume descriptor"))?;
        let block = u64::from(u16::from_le_bytes([d[128], d[129]]));
        if block == 0 {
            return Err(invalid("block size 0"));
        }
        let root = record(&d[156..190], is_joliet)
            .ok_or_else(|| invalid("bad root directory"))?
            .1;
        Ok(Self {
            file: RefCell::new(file),
            block,
            joliet: is_joliet,
            root: Entry {
                name: String::new(),
                ..root
            },
        })
    }

    pub fn root(&self) -> &Entry {
        &self.root
    }

    /// The contents of a file, or the raw records of a folder.
    pub fn read(&self, entry: &Entry) -> io::Result<Vec<u8>> {
        let mut file = self.file.borrow_mut();
        file.seek(SeekFrom::Start(u64::from(entry.extent) * self.block))?;
        let mut data = vec![0; entry.size as usize];
        file.read_exact(&mut data)?;
        Ok(data)
    }

    /// The entries of a folder, without `.` and `..`.
    pub fn list(&self, dir: &Entry) -> io::Result<Vec<Entry>> {
        let data = self.read(dir)?;
        let mut entries = Vec::new();
        let mut at = 0;
        while at < data.len() {
            let len = usize::from(data[at]);
            if len == 0 {
                // Records do not cross sectors; the rest of this one is padding.
                at = (at / SECTOR as usize + 1) * SECTOR as usize;
                continue;
            }
            let rec = data
                .get(at..at + len)
                .ok_or_else(|| invalid("directory record past its folder"))?;
            match record(rec, self.joliet) {
                Some((false, entry)) => entries.push(entry),
                Some((true, _)) => {}
                None => return Err(invalid("bad directory record")),
            }
            at += len;
        }
        Ok(entries)
    }
}

/// A directory record, with whether it is `.` or `..`.
fn record(rec: &[u8], joliet: bool) -> Option<(bool, Entry)> {
    let name_len = usize::from(*rec.get(32)?);
    let id = rec.get(33..33 + name_len)?;
    let extent = u32::from_le_bytes(rec.get(2..6)?.try_into().ok()?);
    let size = u32::from_le_bytes(rec.get(10..14)?.try_into().ok()?);
    let is_dir = rec[25] & 2 != 0;
    if id == [0] || id == [1] {
        let entry = Entry {
            name: String::new(),
            is_dir,
            extent,
            size,
        };
        return Some((true, entry));
    }
    let mut name = if joliet {
        let units: Vec<u16> = id
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        id.iter().map(|&b| char::from(b)).collect::<String>()
    };
    if !is_dir && let Some(semi) = name.rfind(';') {
        name.truncate(semi);
    }
    if !is_dir && name.ends_with('.') {
        name.pop();
    }
    if !joliet {
        name.make_ascii_lowercase();
    }
    Some((
        false,
        Entry {
            name,
            is_dir,
            extent,
            size,
        },
    ))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory record for `name`.
    fn rec(name: &[u8], extent: u32, size: u32, dir: bool) -> Vec<u8> {
        let mut r = vec![0u8; 33];
        r[2..6].copy_from_slice(&extent.to_le_bytes());
        r[10..14].copy_from_slice(&size.to_le_bytes());
        r[25] = if dir { 2 } else { 0 };
        r[32] = name.len() as u8;
        r.extend_from_slice(name);
        if r.len() % 2 == 1 {
            r.push(0);
        }
        r[0] = r.len() as u8;
        r
    }

    #[test]
    fn reads_a_small_image() {
        // Sector 16: primary descriptor, 17: terminator, 18: root, 19: file.
        let mut img = vec![0u8; 20 * SECTOR as usize];
        let pvd = 16 * SECTOR as usize;
        img[pvd] = 1;
        img[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
        img[pvd + 128..pvd + 130].copy_from_slice(&2048u16.to_le_bytes());
        let root = rec(&[0], 18, 2048, true);
        img[pvd + 156..pvd + 156 + root.len()].copy_from_slice(&root);
        let term = 17 * SECTOR as usize;
        img[term] = 255;
        img[term + 1..term + 6].copy_from_slice(b"CD001");
        let mut dir = rec(&[0], 18, 2048, true);
        dir.extend(rec(&[1], 18, 2048, true));
        dir.extend(rec(b"RACE.DAT;1", 19, 5, false));
        dir.extend(rec(b"BOOKS", 18, 2048, true));
        let at = 18 * SECTOR as usize;
        img[at..at + dir.len()].copy_from_slice(&dir);
        img[19 * SECTOR as usize..][..5].copy_from_slice(b"hello");

        let path = std::env::temp_dir().join(format!("cr-iso-test-{}.iso", std::process::id()));
        std::fs::write(&path, &img).unwrap();
        assert!(Image::is_image(&path));
        let image = Image::open(&path).unwrap();
        let entries = image.list(image.root()).unwrap();
        let names: Vec<_> = entries
            .iter()
            .map(|e| (e.name.as_str(), e.is_dir))
            .collect();
        assert_eq!(names, [("race.dat", false), ("books", true)]);
        assert_eq!(image.read(&entries[0]).unwrap(), b"hello");
        std::fs::remove_file(path).unwrap();
    }
}
