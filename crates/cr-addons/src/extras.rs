//! The add-ons' pictures and texts, built into the program beside the
//! package: the pictures go where the records look for them, and the
//! texts (the add-ons' readme files, as plain text) are shown on request.
//!
//! `addons/Comunidade-extras.xz` is, before xz, a text header and then the
//! files' bytes in the header's order:
//!
//! ```text
//! CRX1
//! F<TAB>size<TAB>path        pictures/..., texts/....txt, addons.tsv
//! P<TAB>picture<TAB>place    a picture's place in the installation
//! (blank line)
//! ```
//!
//! `addons.tsv` holds one block per add-on folder: `A<TAB>folder`, then
//! `K<TAB>package<TAB>items in it` for each package, `I<TAB>category<TAB>name`
//! for the items it brought and `T<TAB>text<TAB>file name` for its texts.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use cr_i18n::tr;
use cr_store::Installation;

static EXTRAS: &[u8] = include_bytes!("../../../addons/Comunidade-extras.xz");

pub struct Extras {
    data: Vec<u8>,
    /// Each file's bytes in `data`, by its path.
    files: HashMap<String, (usize, usize)>,
    /// Pictures in the order of the header.
    pictures: Vec<String>,
    /// (picture, place in the installation).
    placements: Vec<(String, String)>,
}

/// One add-on folder of the community's archive.
#[derive(Debug, Default)]
pub struct Addon {
    pub folder: String,
    pub packages: Vec<Package>,
    /// (path in the extras, file name).
    pub texts: Vec<(String, String)>,
}

#[derive(Debug, Default)]
pub struct Package {
    pub path: String,
    /// Items in the package, before newer add-ons replaced some.
    pub total: usize,
    /// (category, name) of the items it brought.
    pub items: Vec<(String, String)>,
}

impl Addon {
    pub fn name(&self) -> &str {
        self.folder.rsplit('/').next().unwrap_or(&self.folder)
    }

    /// Items in its packages.
    pub fn total(&self) -> usize {
        self.packages.iter().map(|p| p.total).sum()
    }
}

/// The built-in pictures and texts, unpacked on first use.
pub fn get() -> Result<&'static Extras, String> {
    static EXTRAS: std::sync::OnceLock<Result<Extras, String>> = std::sync::OnceLock::new();
    EXTRAS
        .get_or_init(Extras::load)
        .as_ref()
        .map_err(Clone::clone)
}

impl Extras {
    /// The built-in pictures and texts, unpacked.
    fn load() -> Result<Self, String> {
        let mut data = Vec::new();
        lzma_rs::xz_decompress(&mut io::Cursor::new(EXTRAS), &mut data)
            .map_err(|e| tr!("The built-in pictures are damaged: {e}", e))?;
        Self::parse(data).ok_or_else(|| tr("The built-in pictures are damaged.").to_string())
    }

    fn parse(data: Vec<u8>) -> Option<Self> {
        let end = data.windows(2).position(|w| w == b"\n\n")?;
        let header = std::str::from_utf8(&data[..end]).ok()?;
        let mut lines = header.lines();
        if lines.next()? != "CRX1" {
            return None;
        }
        let mut at = end + 2;
        let (mut files, mut pictures, mut placements) = (HashMap::new(), Vec::new(), Vec::new());
        for line in lines {
            let mut fields = line.split('\t');
            match (fields.next(), fields.next(), fields.next()) {
                (Some("F"), Some(size), Some(path)) => {
                    let size: usize = size.parse().ok()?;
                    files.insert(path.to_owned(), (at, size));
                    if path.starts_with("pictures/") {
                        pictures.push(path.to_owned());
                    }
                    at += size;
                }
                (Some("P"), Some(picture), Some(place)) => {
                    placements.push((picture.to_owned(), place.to_owned()));
                }
                _ => return None,
            }
        }
        (at <= data.len()).then_some(Self {
            data,
            files,
            pictures,
            placements,
        })
    }

    pub fn file(&self, path: &str) -> Option<&[u8]> {
        let &(at, size) = self.files.get(path)?;
        self.data.get(at..at + size)
    }

    pub fn text(&self, path: &str) -> String {
        self.file(path)
            .map(|t| String::from_utf8_lossy(t).into_owned())
            .unwrap_or_default()
    }

    /// The add-on folders, in the archive's order.
    pub fn addons(&self) -> Vec<Addon> {
        let mut addons: Vec<Addon> = Vec::new();
        for line in self.text("addons.tsv").lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields[..] {
                ["A", folder] => addons.push(Addon {
                    folder: folder.to_owned(),
                    ..Default::default()
                }),
                ["K", path, total] => {
                    if let Some(a) = addons.last_mut() {
                        a.packages.push(Package {
                            path: path.to_owned(),
                            total: total.parse().unwrap_or_default(),
                            items: Vec::new(),
                        });
                    }
                }
                ["I", category, name] => {
                    if let Some(p) = addons.last_mut().and_then(|a| a.packages.last_mut()) {
                        p.items.push((category.to_owned(), name.to_owned()));
                    }
                }
                ["T", path, name] => {
                    if let Some(a) = addons.last_mut() {
                        a.texts.push((path.to_owned(), name.to_owned()));
                    }
                }
                _ => {}
            }
        }
        addons
    }

    /// Where each picture goes in `install`: the places the records name,
    /// then every picture under `UserBMP/`, in its add-on's folder, where
    /// the original's picture dialogs open. Inside Wine, a picture the
    /// records look for elsewhere on drive C: (`C:\AD&D Pics`) goes there
    /// too.
    fn targets(&self, install: &Installation) -> Vec<(&str, PathBuf)> {
        let root = install.root();
        let drive_c = root
            .ancestors()
            .find(|a| a.file_name().is_some_and(|n| n == "drive_c"));
        let mut out = Vec::new();
        for (picture, place) in &self.placements {
            out.push((picture.as_str(), join(root, place)));
            if let (Some(c), Some(rest)) = (drive_c, place.strip_prefix("UserBMP/")) {
                out.push((picture.as_str(), join(c, rest)));
            }
        }
        for picture in &self.pictures {
            let place = format!("UserBMP/{}", picture.trim_start_matches("pictures/"));
            out.push((picture.as_str(), join(root, &place)));
        }
        out
    }

    /// Writes the pictures into `install`, never over a file that is
    /// already there; returns the files written, for the log.
    pub fn install_pictures(&self, install: &Installation) -> Result<Vec<PathBuf>, String> {
        let mut written = Vec::new();
        for (picture, target) in self.targets(install) {
            let Some(bytes) = self.file(picture) else {
                continue;
            };
            let target = existing_case(&target);
            if target.exists() {
                continue;
            }
            let write = || -> io::Result<()> {
                if let Some(dir) = target.parent() {
                    fs::create_dir_all(dir)?;
                }
                fs::write(&target, bytes)
            };
            write().map_err(|e| tr!("Could not write {file}: {e}", file = target.display(), e))?;
            written.push(target);
        }
        Ok(written)
    }
}

fn join(root: &Path, relative: &str) -> PathBuf {
    relative.split('/').fold(root.to_owned(), |p, c| p.join(c))
}

/// `path`, with each part that is already there in another case spelled
/// as it is on disk: the original's folders are `BITMAPS` on one
/// computer and `Bitmaps` on the next, and its file names ignore case.
fn existing_case(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        let next = out.join(part);
        if next.exists() || out.as_os_str().is_empty() {
            out = next;
            continue;
        }
        let name = part.as_os_str().to_string_lossy();
        let found = fs::read_dir(&out).ok().and_then(|entries| {
            entries
                .filter_map(|e| e.ok())
                .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(&name))
                .map(|e| e.path())
        });
        out = found.unwrap_or(next);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_extras_read() {
        let extras = get().unwrap();
        let addons = extras.addons();
        assert!(addons.len() > 250);
        assert!(
            addons
                .iter()
                .all(|a| !a.folder.starts_with("Official Addons"))
        );
        let udb = addons
            .iter()
            .find(|a| a.folder == "AD&D Core Rules User Database")
            .unwrap();
        assert!(udb.packages[0].items.len() > 30_000);
        assert!(udb.total() >= udb.packages[0].items.len());
        assert!(extras.pictures.len() > 600);
        assert!(
            extras
                .pictures
                .iter()
                .all(|p| extras.file(p).is_some_and(|b| b.starts_with(b"BM")))
        );
        for (path, _) in addons.iter().flat_map(|a| &a.texts) {
            assert!(extras.file(path).is_some(), "{path}");
        }
    }

    #[test]
    fn pictures_go_where_the_records_look() {
        let data = b"CRX1\nF\t2\tpictures/M/a.bmp\nP\tpictures/M/a.bmp\tBitmaps/Monsters/A.bmp\n\
                     P\tpictures/M/a.bmp\tUserBMP/AD&D Pics/a.bmp\n\nBM"
            .to_vec();
        let extras = Extras::parse(data).unwrap();
        let base = std::env::temp_dir().join(format!("cr-addons-pictures-{}", std::process::id()));
        let root = base.join("drive_c/Program Files/AD&D Core Rules 2.0");
        fs::create_dir_all(root.join("Database")).unwrap();
        fs::create_dir_all(root.join("BITMAPS/MONSTERS")).unwrap();
        let install = Installation::open(&root).unwrap();
        let written = extras.install_pictures(&install).unwrap();
        assert_eq!(
            written,
            [
                root.join("BITMAPS/MONSTERS/A.bmp"),
                root.join("UserBMP/AD&D Pics/a.bmp"),
                base.join("drive_c/AD&D Pics/a.bmp"),
                root.join("UserBMP/M/a.bmp"),
            ]
        );
        // A second install leaves them as they are.
        assert!(extras.install_pictures(&install).unwrap().is_empty());
        fs::remove_dir_all(base).unwrap();
    }
}
