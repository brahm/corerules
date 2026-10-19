//! The pictures records name: a monster type's bitmap, a race's or a
//! character's portrait.
//!
//! The original stores the Windows path the picture had on the computer
//! that made the record (`C:\Program Files\AD&D Core Rules 2.0\Bitmaps\
//! Monsters\vaprak.bmp`, `G:\Bitmaps\Portrait\MALE1.bmp` on the CD, `C:\AD&D
//! Pics\troll.bmp`), or only a file name for the shipped pictures
//! (`6.bmp`). [`relative`] turns either into a path inside the
//! installation, the same one the add-ons installer writes the picture to.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Where a bare file name is looked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `Bitmaps/Monsters`, the monster types' pictures.
    Monster,
    /// `Bitmaps/Portrait`, the races' and characters' portraits.
    Portrait,
}

/// The installation folder's name in the stored paths, whatever the drive
/// or the localized `Program Files` before it.
const ROOT_NAME: &str = "AD&D Core Rules 2.0";

/// The path inside an installation, with `/` between the parts, of the
/// picture a record stores:
///
/// * a path inside a Core Rules installation, on any drive, keeps what
///   follows the installation folder (`Bitmaps/Monsters/vaprak.bmp`);
/// * a path into the CD's `Bitmaps` keeps it (`Bitmaps/Portrait/MALE1.bmp`);
/// * any other path goes under `UserBMP`, without the drive
///   (`UserBMP/AD&D Pics/troll.bmp`);
/// * a bare file name is in the shipped folder for `kind`.
///
/// `None` for an empty path.
pub fn relative(stored: &str, kind: Kind) -> Option<String> {
    let parts: Vec<&str> = stored
        .trim()
        .split(['\\', '/'])
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    let (&file, folders) = parts.split_last()?;
    let mut out: Vec<&str> = match folders.first() {
        None => vec![
            "Bitmaps",
            match kind {
                Kind::Monster => "Monsters",
                Kind::Portrait => "Portrait",
            },
        ],
        Some(drive) if drive.ends_with(':') => {
            let rest = &folders[1..];
            if let Some(at) = rest.iter().position(|p| p.eq_ignore_ascii_case(ROOT_NAME)) {
                rest[at + 1..].to_vec()
            } else if rest
                .first()
                .is_some_and(|p| p.eq_ignore_ascii_case("Bitmaps"))
            {
                rest.to_vec()
            } else {
                std::iter::once("UserBMP")
                    .chain(rest.iter().copied())
                    .collect()
            }
        }
        Some(_) => folders.to_vec(),
    };
    out.push(file);
    Some(out.join("/"))
}

/// The picture a record stores, combined from a folder and a file name as
/// races and characters keep them; either may be empty.
pub fn joined(folder: &str, file: &str) -> String {
    match (folder.trim_end_matches(['\\', '/']), file) {
        (_, "") => String::new(),
        ("", file) => file.to_owned(),
        (folder, file) => format!("{folder}\\{file}"),
    }
}

/// Finds pictures in an installation without regard to case, listing
/// each folder once.
pub struct Finder {
    root: PathBuf,
    /// Each folder looked in, by its relative path in lower case: its
    /// files by name in lower case.
    folders: HashMap<String, Option<HashMap<String, PathBuf>>>,
}

impl Finder {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_owned(),
            folders: HashMap::new(),
        }
    }

    /// The file for a path [`relative`] made, when it is there.
    pub fn find(&mut self, relative: &str) -> Option<PathBuf> {
        let (folder, file) = relative.rsplit_once('/').unwrap_or(("", relative));
        let root = &self.root;
        self.folders
            .entry(folder.to_lowercase())
            .or_insert_with(|| {
                let dir = folder
                    .split('/')
                    .filter(|p| !p.is_empty())
                    .try_fold(root.clone(), |dir, name| crate::find_entry(&dir, name))?;
                Some(
                    fs::read_dir(dir)
                        .ok()?
                        .filter_map(|e| e.ok())
                        .map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.path()))
                        .collect(),
                )
            })
            .as_ref()?
            .get(&file.to_lowercase())
            .cloned()
    }

    /// The file of a picture a record stores, when it is there.
    pub fn picture(&mut self, stored: &str, kind: Kind) -> Option<PathBuf> {
        self.find(&relative(stored, kind)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_paths_map_into_the_installation() {
        let cases = [
            (
                "C:\\Program Files\\AD&D Core Rules 2.0\\Bitmaps\\Monsters\\vaprak.bmp",
                Kind::Monster,
                "Bitmaps/Monsters/vaprak.bmp",
            ),
            (
                "C:\\Ohjelmatiedostot\\AD&D Core Rules 2.0\\Bitmaps\\Portrait\\\\NA.bmp",
                Kind::Portrait,
                "Bitmaps/Portrait/NA.bmp",
            ),
            (
                "F:\\AD&D\\AD&D Core Rules 2.0\\Bitmaps\\Monsters\\MCA3\\x.bmp",
                Kind::Monster,
                "Bitmaps/Monsters/MCA3/x.bmp",
            ),
            (
                "C:\\Program Files\\AD&D Core Rules 2.0\\0.BMP",
                Kind::Monster,
                "0.BMP",
            ),
            (
                "G:\\Bitmaps\\Portrait\\MALE1.bmp",
                Kind::Portrait,
                "Bitmaps/Portrait/MALE1.bmp",
            ),
            (
                "Q:\\BITMAPS\\MONSTERS\\191.bmp",
                Kind::Portrait,
                "BITMAPS/MONSTERS/191.bmp",
            ),
            (
                "C:\\AD&D Pics\\troll.bmp",
                Kind::Monster,
                "UserBMP/AD&D Pics/troll.bmp",
            ),
            ("6.bmp", Kind::Monster, "Bitmaps/Monsters/6.bmp"),
            (
                "Aarakocr.bmp",
                Kind::Portrait,
                "Bitmaps/Portrait/Aarakocr.bmp",
            ),
        ];
        for (stored, kind, want) in cases {
            assert_eq!(relative(stored, kind).as_deref(), Some(want), "{stored}");
        }
        assert_eq!(relative("", Kind::Monster), None);
        assert_eq!(
            joined("G:\\Bitmaps\\Portrait\\", "MALE1.bmp"),
            "G:\\Bitmaps\\Portrait\\MALE1.bmp"
        );
        assert_eq!(joined("G:\\Bitmaps\\Portrait\\", ""), "");
    }

    #[test]
    fn pictures_are_found_without_regard_to_case() {
        let root = std::env::temp_dir().join(format!("cr-pictures-{}", std::process::id()));
        fs::create_dir_all(root.join("BITMAPS/Monsters")).unwrap();
        fs::write(root.join("BITMAPS/Monsters/Vaprak.BMP"), b"BM").unwrap();
        let mut finder = Finder::new(&root);
        assert_eq!(
            finder.picture(
                "C:\\Program Files\\AD&D Core Rules 2.0\\Bitmaps\\Monsters\\vaprak.bmp",
                Kind::Monster
            ),
            Some(root.join("BITMAPS/Monsters/Vaprak.BMP"))
        );
        assert_eq!(finder.picture("6.bmp", Kind::Monster), None);
        assert_eq!(finder.picture("x.bmp", Kind::Portrait), None);
        fs::remove_dir_all(root).unwrap();
    }
}
