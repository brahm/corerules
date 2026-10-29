//! A character's sheet, or the page of an encounter, encounter table,
//! template, treasure table or saved treasure, as the Mac app shows it:
//! the PDF `cr-app` draws for printing, in a window of its own. A
//! character's sheet is exported from here too, and its sections and
//! border chosen in Character Sheet Options.

use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::sheets::{Export, offered, section_name};
use cr_store::Installation;
use cr_store::sheet::{Border, Sheet, SheetChoice};

use crate::{CoreError, SheetExport};

/// A section Character Sheet Options offers to add.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SheetSectionInfo {
    pub id: u32,
    pub name: String,
}

/// What Character Sheet Options chooses: the sections in order (any may
/// repeat) and the border, by its place in [`border_labels`].
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SheetOptions {
    pub sections: Vec<u32>,
    pub border: u32,
}

/// Every section the options offer, the page break first.
#[uniffi::export]
pub fn sheet_sections_offered() -> Vec<SheetSectionInfo> {
    offered()
        .into_iter()
        .map(|id| SheetSectionInfo {
            id,
            name: section_name(id),
        })
        .collect()
}

/// The name of section `id` in the options' list.
#[uniffi::export]
pub fn sheet_section_name(id: u32) -> String {
    section_name(id)
}

/// The Border Style choices, in order.
#[uniffi::export]
pub fn border_labels() -> Vec<String> {
    Border::ALL.iter().map(|b| b.label().to_owned()).collect()
}

/// A sheet open in its window.
#[derive(uniffi::Object)]
pub struct SheetSession {
    install: Arc<Installation>,
    sheet: Mutex<Sheet>,
    /// The character's file, for a character's sheet.
    file: Option<String>,
    /// Its options, when they can be changed.
    choice: Mutex<Option<SheetChoice>>,
    writable: bool,
}

impl SheetSession {
    pub(crate) fn character(
        install: Arc<Installation>,
        file: &str,
        writable: bool,
    ) -> Result<Arc<Self>, CoreError> {
        let (sheet, choice) = cr_app::sheets::character(&install, file)?;
        Ok(Arc::new(Self {
            install,
            sheet: Mutex::new(sheet),
            file: Some(file.to_owned()),
            choice: Mutex::new(choice),
            writable,
        }))
    }

    pub(crate) fn page(install: Arc<Installation>, key: &str) -> Result<Arc<Self>, CoreError> {
        let sheet = install
            .page(key)
            .ok_or_else(|| CoreError::new(cr_i18n::tr("Could Not Read")))?;
        Ok(Self::of(install, sheet))
    }

    /// A page made elsewhere: a monster of the user database, a summary.
    pub(crate) fn of(install: Arc<Installation>, sheet: Sheet) -> Arc<Self> {
        Arc::new(Self {
            install,
            sheet: Mutex::new(sheet),
            file: None,
            choice: Mutex::new(None),
            writable: false,
        })
    }

    fn sheet(&self) -> MutexGuard<'_, Sheet> {
        self.sheet.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn choice(&self) -> MutexGuard<'_, Option<SheetChoice>> {
        self.choice.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[uniffi::export]
impl SheetSession {
    /// The character's name, or the page's title.
    pub fn title(&self) -> String {
        self.sheet().title.trim().to_owned()
    }

    /// The character's file, for a character's sheet.
    pub fn file(&self) -> Option<String> {
        self.file.clone()
    }

    /// The sheet as a PDF, on Letter paper or A4.
    pub fn pdf(&self, letter: bool) -> Vec<u8> {
        let paper = if letter {
            cr_app::pdf::Paper::Letter
        } else {
            cr_app::pdf::Paper::A4
        };
        cr_app::pdf::sheet(&self.sheet(), paper)
    }

    /// The files a character's sheet is exported to; none for a page.
    pub fn exports(&self) -> Vec<SheetExport> {
        if self.file.is_none() {
            return Vec::new();
        }
        Export::of(self.sheet().flavor)
            .into_iter()
            .map(SheetExport::of)
            .collect()
    }

    /// The sheet as `export` writes it.
    pub fn export(&self, export: SheetExport) -> Vec<u8> {
        export.export().write(&self.sheet())
    }

    /// Character Sheet Options as they are, when they can be changed.
    pub fn options(&self) -> Option<SheetOptions> {
        if !self.writable {
            return None;
        }
        self.choice().as_ref().map(|c| SheetOptions {
            sections: c.sections.clone(),
            border: Border::ALL.iter().position(|b| *b == c.border).unwrap_or(0) as u32,
        })
    }

    /// Keeps new options and makes the sheet again with them.
    pub fn choose(&self, options: SheetOptions) -> Result<(), CoreError> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        let choice = SheetChoice {
            sections: options.sections,
            border: Border::ALL
                .get(options.border as usize)
                .copied()
                .unwrap_or(Border::Normal),
        };
        let sheet = cr_app::sheets::choose(&self.install, file, &choice)?;
        *self.sheet() = sheet;
        *self.choice() = Some(choice);
        Ok(())
    }
}
