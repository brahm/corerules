//! Opens an AD&D Core Rules 2.0 installation folder.
//!
//! The original runs on Windows, so file names inside an installation have
//! inconsistent case (`Database` from the installer, `DATABASE` on the CD,
//! `index.chr` next to `INDEX.CHR`). Every lookup here ignores case.

pub mod advance;
pub mod books;
pub mod catalog;
pub mod choices;
pub mod create;
pub mod dual;
pub mod edit;
pub mod encounter;
pub mod lists;
pub mod npc;
pub mod settings;
pub mod sheet;
pub mod tables;
pub mod treasure;
pub mod userdb;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use cr_formats::character::CharacterIndex;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} is not a Core Rules installation (no Database folder)")]
    NotAnInstallation(PathBuf),
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{0}")]
    Invalid(String),
    #[error("{path}: {source}")]
    Format {
        path: PathBuf,
        source: cr_formats::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Where `packaging/extract-data.sh --install` puts the data package.
pub fn data_package_dir() -> Option<PathBuf> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local/share")))?;
    Some(data.join("corerules").join("data"))
}

/// Folder where the installer puts Core Rules inside a Wine prefix.
pub const DEFAULT_WINE_PATH: &str = "drive_c/Program Files/AD&D Core Rules 2.0";

pub struct Installation {
    root: PathBuf,
}

impl Installation {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if find_entry(&root, "Database").is_none() {
            return Err(Error::NotAnInstallation(root));
        }
        Ok(Self { root })
    }

    /// Looks for the data package made by `extract-data.sh` in the app's
    /// data folder (`$XDG_DATA_HOME/corerules/data`, else
    /// `~/.local/share/corerules/data`), then for an installation in the
    /// usual Wine prefix (`$WINEPREFIX`, else `~/.wine`).
    pub fn discover() -> Option<Self> {
        if let Some(dir) = data_package_dir()
            && let Ok(install) = Self::open(dir)
        {
            return Some(install);
        }
        let prefix = std::env::var_os("WINEPREFIX")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".wine")))?;
        Self::open(prefix.join(DEFAULT_WINE_PATH)).ok()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolves a relative path such as `Chars/index.chr`, matching each
    /// component without regard to case.
    pub fn path(&self, relative: &str) -> Option<PathBuf> {
        relative
            .split(['/', '\\'])
            .filter(|c| !c.is_empty())
            .try_fold(self.root.clone(), |dir, name| find_entry(&dir, name))
    }

    pub fn read(&self, relative: &str) -> Result<Vec<u8>> {
        let path = self
            .path(relative)
            .unwrap_or_else(|| self.root.join(relative));
        fs::read(&path).map_err(|source| Error::Io { path, source })
    }

    /// The character list (`Chars/index.chr`). A missing index means no characters.
    pub fn characters(&self) -> Result<CharacterIndex> {
        let Some(path) = self.path("Chars/index.chr") else {
            return Ok(CharacterIndex {
                schema: 95,
                entries: Vec::new(),
            });
        };
        let data = fs::read(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        CharacterIndex::parse(&data).map_err(|source| Error::Format { path, source })
    }

    /// `.dat` files in a folder such as `Database` or `UserDbas`, sorted by name.
    pub fn data_files(&self, folder: &str) -> Result<Vec<PathBuf>> {
        let Some(dir) = self.path(folder) else {
            return Ok(Vec::new());
        };
        let entries = fs::read_dir(&dir).map_err(|source| Error::Io {
            path: dir.clone(),
            source,
        })?;
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("dat")))
            .collect();
        files.sort_by_key(|p| p.file_name().map(|n| n.to_ascii_lowercase()));
        Ok(files)
    }
}

fn find_entry(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_resolve_without_regard_to_case() {
        let root = std::env::temp_dir().join(format!("cr-store-test-{}", std::process::id()));
        fs::create_dir_all(root.join("DATABASE")).unwrap();
        fs::create_dir_all(root.join("Chars")).unwrap();
        fs::write(root.join("Chars/INDEX.CHR"), [0, 0]).unwrap();

        let install = Installation::open(&root).unwrap();
        assert_eq!(
            install.path("chars/index.chr"),
            Some(root.join("Chars/INDEX.CHR"))
        );
        assert!(install.characters().unwrap().entries.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
