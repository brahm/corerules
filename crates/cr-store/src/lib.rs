//! Opens an AD&D Core Rules 2.0 installation folder.
//!
//! The original runs on Windows, so file names inside an installation have
//! inconsistent case (`Database` from the installer, `DATABASE` on the CD,
//! `index.chr` next to `INDEX.CHR`). Every lookup here ignores case.

pub mod abilities;
pub mod advance;
pub mod books;
pub mod catalog;
pub mod choices;
pub mod collate;
pub mod combat;
pub mod create;
pub mod dual;
pub mod edit;
pub mod encounter;
mod exe;
pub mod export;
pub mod extract;
pub mod groups;
pub mod help;
mod iso9660;
pub mod kit_magic;
pub mod learn;
pub mod lists;
pub mod npc;
pub mod party;
pub mod pictures;
pub mod proficiencies;
pub mod purse;
pub mod settings;
pub mod sheet;
pub mod skills;
pub mod summary;
pub mod tables;
pub mod treasure;
pub mod userdb;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use cr_formats::character::CharacterIndex;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{}", not_an_installation(.0))]
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

fn not_an_installation(path: &Path) -> String {
    cr_i18n::tr!(
        "{path} is not a Core Rules installation (no Database folder)",
        path = path.display()
    )
}

pub type Result<T> = std::result::Result<T, Error>;

/// Where Extract Data from CD… (`cr extract-data`) puts the data package:
/// `$XDG_DATA_HOME/corerules/data`, else `~/.local/share/corerules/data`.
#[cfg(not(windows))]
pub fn data_package_dir() -> Option<PathBuf> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local/share")))?;
    Some(data.join("corerules").join("data"))
}

/// Where Extract Data from CD… (`cr extract-data`) puts the data package:
/// `Core Rules\data` in the user's Documents folder. A packaged (MSIX) app
/// writes there directly, so the characters stay when the app is
/// uninstalled, where AppData would be removed with it.
#[cfg(windows)]
pub fn data_package_dir() -> Option<PathBuf> {
    Some(documents_dir()?.join("Core Rules").join("data"))
}

/// The Documents folder as Windows knows it, which may have been moved
/// (into OneDrive, say) and whose name may be translated.
#[cfg(windows)]
fn documents_dir() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{
        FOLDERID_Documents, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
    };

    let mut path = std::ptr::null_mut();
    // SAFETY: on success `path` is a NUL-terminated UTF-16 string; it is
    // freed with CoTaskMemFree whether the call succeeds or not, as the
    // documentation of SHGetKnownFolderPath asks.
    unsafe {
        let result = SHGetKnownFolderPath(
            &FOLDERID_Documents,
            KF_FLAG_DEFAULT as _,
            std::ptr::null_mut(),
            &mut path,
        );
        let dir = (result == 0 && !path.is_null()).then(|| {
            let len = (0..).take_while(|&i| *path.add(i) != 0).count();
            PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(
                path, len,
            )))
        });
        CoTaskMemFree(path as _);
        dir
    }
}

/// [`data_package_dir`] the way texts for the user name it.
#[cfg(not(windows))]
pub const DATA_PACKAGE_SHOWN: &str = "~/.local/share/corerules/data";
/// [`data_package_dir`] the way texts for the user name it.
#[cfg(windows)]
pub const DATA_PACKAGE_SHOWN: &str = r"Documents\Core Rules\data";

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

    /// Looks for the data package made by Extract Data from CD… in the app's
    /// data folder ([`data_package_dir`]), then, except on Windows, for an
    /// installation in the usual Wine prefix (`$WINEPREFIX`, else
    /// `~/.wine`).
    pub fn discover() -> Option<Self> {
        if let Some(dir) = data_package_dir()
            && let Ok(install) = Self::open(dir)
        {
            return Some(install);
        }
        if cfg!(windows) {
            return None;
        }
        let prefix = std::env::var_os("WINEPREFIX")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".wine")))?;
        Self::open(prefix.join(DEFAULT_WINE_PATH)).ok()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Whether the app can save into this installation: false for a
    /// mounted CD or ISO image, a folder shared read-only with the Flatpak,
    /// or any folder the user may not write to. Everything the app makes
    /// is saved under the installation's folders, so this is tried by
    /// making and removing a file in each of them: permission bits do not
    /// show a read-only mount.
    pub fn is_writable(&self) -> bool {
        std::iter::once(self.root.clone())
            .chain(
                ["Chars", "UserDbas", "Database"]
                    .into_iter()
                    .filter_map(|folder| self.path(folder)),
            )
            .all(|dir| can_write_in(&dir))
    }

    /// Whether this is Core Rules 2.0 (Evermore), whose content is only
    /// imported into the Expansion: everything new is made in the
    /// Expansion's layout, so 2.0 installations are opened read-only.
    pub fn is_core20(&self) -> bool {
        self.path("UserDbas/AllowedU.dat").is_none()
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

fn can_write_in(dir: &Path) -> bool {
    let probe = dir.join(format!(".corerules-write-test-{}", std::process::id()));
    let made = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&probe)
        .is_ok();
    made && fs::remove_file(&probe).is_ok()
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

    #[cfg(unix)]
    #[test]
    fn a_read_only_folder_is_not_writable() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("cr-store-ro-{}", std::process::id()));
        fs::create_dir_all(root.join("Database")).unwrap();
        fs::create_dir_all(root.join("Chars")).unwrap();
        let install = Installation::open(&root).unwrap();
        assert!(install.is_writable());
        assert_eq!(fs::read_dir(root.join("Chars")).unwrap().count(), 0);

        let chars = root.join("Chars");
        fs::set_permissions(&chars, fs::Permissions::from_mode(0o555)).unwrap();
        // Root writes into any folder; the check is then left to CI.
        let bypassed = fs::write(chars.join("probe"), b"").is_ok();
        if !bypassed {
            assert!(!install.is_writable());
        }
        fs::set_permissions(&chars, fs::Permissions::from_mode(0o755)).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
