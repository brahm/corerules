//! Import… and Import Core Rules 2.0…, as the Mac app draws them: what a
//! package, or a Core Rules 2.0 installation, brings in, each item taken
//! or left, from `cr_app::packages`. The parts whose names clash
//! with the standard ones get the name with "*" the original gives; the
//! Name Conflict question comes with the rest of the import.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::packages::{self, ImportChoice};
use cr_store::Installation;

use crate::CoreError;

/// An item of the package.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ImportItem {
    /// Its place in the package, which [`ImportSession::set`] takes.
    pub index: u32,
    pub name: String,
    /// What importing it does about a name it shares.
    pub note: String,
    pub checked: bool,
}

/// The items of a category.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ImportCategory {
    pub label: String,
    pub items: Vec<ImportItem>,
}

/// What an import did: the heading and the lines under it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ImportReport {
    pub heading: String,
    pub lines: Vec<String>,
}

/// A package open for Import….
#[derive(uniffi::Object)]
pub struct ImportSession {
    install: Arc<Installation>,
    /// The package's file name, or the installation's folder.
    file_name: String,
    core20: bool,
    choice: Mutex<ImportChoice>,
}

impl ImportSession {
    pub(crate) fn open(install: Arc<Installation>, path: &str) -> Result<Arc<Self>, CoreError> {
        let path = Path::new(path);
        let file = packages::read(path).map_err(CoreError::new)?;
        Self::of(install, file, packages::file_name(path), false)
    }

    /// What the Core Rules 2.0 installation in `folder` hands over;
    /// `None` when it has nothing of its own.
    pub(crate) fn open_core20(
        install: Arc<Installation>,
        folder: &str,
    ) -> Result<Option<Arc<Self>>, CoreError> {
        let export = Installation::open(Path::new(folder))?.core20_package()?;
        if cr_store::export::contents(&export.file).is_empty() {
            return Ok(None);
        }
        Self::of(install, export.file, folder.to_owned(), true).map(Some)
    }

    fn of(
        install: Arc<Installation>,
        file: cr_formats::export::ExportFile,
        file_name: String,
        core20: bool,
    ) -> Result<Arc<Self>, CoreError> {
        let preview = install.import_preview(&file)?;
        let expansion = !install.is_core20();
        Ok(Arc::new(Self {
            file_name,
            core20,
            choice: Mutex::new(ImportChoice::new(file, preview, expansion)),
            install,
        }))
    }

    fn choice(&self) -> MutexGuard<'_, ImportChoice> {
        self.choice.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[uniffi::export]
impl ImportSession {
    /// The package's file name, or the installation's folder.
    pub fn file_name(&self) -> String {
        self.file_name.clone()
    }

    /// The sheet's title.
    pub fn title(&self) -> String {
        if self.core20 {
            cr_i18n::tr("Import Core Rules 2.0 Installation")
        } else {
            cr_i18n::tr("Import Package")
        }
        .to_owned()
    }

    /// The items by category, sorted by label.
    pub fn categories(&self) -> Vec<ImportCategory> {
        let choice = self.choice();
        choice
            .categories()
            .into_iter()
            .map(|(category, items)| ImportCategory {
                label: packages::category_label(category).to_owned(),
                items: items
                    .into_iter()
                    .map(|n| ImportItem {
                        index: n as u32,
                        name: choice.preview[n].name.clone(),
                        note: choice.note(n).to_owned(),
                        checked: choice.checked[n],
                    })
                    .collect(),
            })
            .collect()
    }

    /// Takes or leaves item `index`.
    pub fn set(&self, index: u32, on: bool) {
        self.choice().set(index as usize, on);
    }

    /// Why the package cannot be imported here: what the Expansion made,
    /// into Core Rules 2.0.
    pub fn refused(&self) -> Option<String> {
        self.choice()
            .refused()
            .then(|| packages::core20_only().1.to_owned())
    }

    /// "The package also holds 2 Map Maker II maps, …", when it does.
    pub fn maps_note(&self) -> Option<String> {
        self.choice().maps_note()
    }

    /// Whether Import may be pressed.
    pub fn can_import(&self) -> bool {
        self.choice().can_import()
    }

    /// Imports the items taken.
    pub fn import(&self) -> Result<ImportReport, CoreError> {
        let choice = self.choice();
        let report = self.install.import(&choice.file, Some(&choice.chosen()))?;
        let (heading, lines) = packages::report(&report);
        Ok(ImportReport { heading, lines })
    }
}
