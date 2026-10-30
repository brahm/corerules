//! Import Add-ons…, as the Mac app draws it: what a folder of add-ons
//! holds, said in one text from `cr_app::addons`, and the import of all
//! of it at once.

use std::path::Path;
use std::sync::Arc;

use cr_app::addons::{self, AddonFolder};
use cr_store::Installation;

use crate::CoreError;
use crate::import::ImportReport;

/// A folder read for Import Add-ons….
#[derive(uniffi::Object)]
pub struct AddonsSession {
    install: Arc<Installation>,
    found: AddonFolder,
}

impl AddonsSession {
    /// Reads the add-ons in `folder`. Takes a while with large archives;
    /// the app calls it off the main thread.
    pub(crate) fn open(install: Arc<Installation>, folder: &str) -> Result<Arc<Self>, CoreError> {
        let found =
            addons::scan(Path::new(folder), !install.is_core20()).map_err(CoreError::new)?;
        Ok(Arc::new(Self { install, found }))
    }
}

#[uniffi::export]
impl AddonsSession {
    /// The title of the question before the import.
    pub fn title(&self) -> String {
        self.found.title().to_owned()
    }

    /// What the folder holds, what comes in and what does not.
    pub fn text(&self) -> String {
        self.found.summary().1
    }

    pub fn can_import(&self) -> bool {
        self.found.can_import()
    }

    /// Imports every add-on that can be, after a backup of the user
    /// databases and characters. Takes a while; the app calls it off the
    /// main thread.
    pub fn import(&self) -> Result<ImportReport, CoreError> {
        let done = self
            .found
            .import(&self.install, &|_, _, _| {})
            .map_err(CoreError::new)?;
        let (heading, text) = done.report();
        Ok(ImportReport {
            heading,
            lines: text.lines().map(str::to_owned).collect(),
        })
    }
}
