//! Export…, New Package… and Edit Package…, as the Mac app draws them:
//! the user's records and characters to take into a package, checked one
//! by one, and the package editor, its items by category, each with
//! Remove, Add Items to put more in, and Save, from `cr_app::packages`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::packages::{self, ItemChooser, PackageEditor, category_label, file_name};
use cr_formats::export::ExportFile;
use cr_store::Installation;
use cr_store::export::Content;

use crate::CoreError;
use crate::import::{ImportCategory, ImportItem};

/// What Export… or Save did: the heading, and what it left out.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PackageDone {
    pub said: String,
    pub warnings: Vec<String>,
}

/// Export…, or the package editor's Add Items: the user's records and
/// characters, each checked or not.
#[derive(uniffi::Object)]
pub struct ExportSession {
    install: Arc<Installation>,
    chooser: Mutex<ItemChooser>,
}

impl ExportSession {
    pub(crate) fn open(install: Arc<Installation>) -> Result<Arc<Self>, CoreError> {
        let items = install.exportable()?;
        Ok(Arc::new(Self {
            install,
            chooser: Mutex::new(ItemChooser::new(items)),
        }))
    }

    fn chooser(&self) -> MutexGuard<'_, ItemChooser> {
        self.chooser.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[uniffi::export]
impl ExportSession {
    /// The items by category, sorted by label.
    pub fn categories(&self) -> Vec<ImportCategory> {
        let chooser = self.chooser();
        chooser
            .categories()
            .into_iter()
            .map(|(category, items)| ImportCategory {
                label: category_label(category).to_owned(),
                items: items
                    .into_iter()
                    .map(|i| ImportItem {
                        index: i as u32,
                        name: chooser.items[i].name.clone(),
                        note: String::new(),
                        checked: chooser.checked[i],
                    })
                    .collect(),
            })
            .collect()
    }

    /// Checks item `index`, or not.
    pub fn set(&self, index: u32, on: bool) {
        self.chooser().set(index as usize, on);
    }

    /// Select All, on or off.
    pub fn set_all(&self, on: bool) {
        self.chooser().set_all(on);
    }

    pub fn all(&self) -> bool {
        self.chooser().all()
    }

    pub fn any(&self) -> bool {
        self.chooser().any()
    }

    /// The extension of a new package of this installation.
    pub fn extension(&self) -> String {
        packages::extension(!self.install.is_core20()).to_owned()
    }

    /// Export…: the items checked, in a new package at `path`.
    pub fn export(&self, path: String) -> Result<PackageDone, CoreError> {
        let items = self.chooser().chosen();
        let path = Path::new(&path);
        let export = self.install.export(&items)?;
        packages::write(path, &export.file).map_err(CoreError::new)?;
        Ok(PackageDone {
            said: cr_i18n::tr!("Exported to {name}", name = file_name(path)),
            warnings: export.warnings,
        })
    }
}

/// An item of the package being edited.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PackageItem {
    /// What [`PackageSession::remove`] takes.
    pub id: u32,
    pub name: String,
}

/// The items of a category of the package.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PackageCategory {
    pub label: String,
    pub items: Vec<PackageItem>,
}

/// New Package… or Edit Package…, open in a sheet.
#[derive(uniffi::Object)]
pub struct PackageSession {
    install: Arc<Installation>,
    editor: Mutex<PackageEditor>,
    /// The items last listed, which the ids name.
    listed: Mutex<Vec<Content>>,
}

impl PackageSession {
    fn new(install: Arc<Installation>, editor: PackageEditor) -> Arc<Self> {
        Arc::new(Self {
            install,
            editor: Mutex::new(editor),
            listed: Mutex::default(),
        })
    }

    fn editor(&self) -> MutexGuard<'_, PackageEditor> {
        self.editor.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[uniffi::export]
impl PackageSession {
    /// The package's file name, or New Package.
    pub fn title(&self) -> String {
        self.editor().title()
    }

    /// Why items cannot be added, when they cannot.
    pub fn layout_note(&self) -> Option<String> {
        self.editor().layout_note().map(str::to_owned)
    }

    /// "The package also holds 2 Map Maker II maps, …", when it does.
    pub fn maps_note(&self) -> Option<String> {
        self.editor().maps_note()
    }

    /// Whether Add Items may add to it.
    pub fn can_add(&self) -> bool {
        self.editor().same_layout()
    }

    /// The items, by category.
    pub fn categories(&self) -> Vec<PackageCategory> {
        let mut listed = Vec::new();
        let categories = self
            .editor()
            .items()
            .into_iter()
            .map(|(category, items)| PackageCategory {
                label: category_label(category).to_owned(),
                items: items
                    .into_iter()
                    .map(|item| {
                        let id = listed.len() as u32;
                        let name = item.name.clone();
                        listed.push(item);
                        PackageItem { id, name }
                    })
                    .collect(),
            })
            .collect();
        *self.listed.lock().unwrap_or_else(|e| e.into_inner()) = listed;
        categories
    }

    /// Takes the item listed with `id` out.
    pub fn remove(&self, id: u32) {
        let item = self
            .listed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id as usize)
            .cloned();
        if let Some(item) = item {
            self.editor().remove(&item);
        }
    }

    /// Add Items' Add: the items checked in `chosen`.
    pub fn add(&self, chosen: Arc<ExportSession>) -> Result<(), CoreError> {
        let items = chosen.chooser().chosen();
        let mut editor = self.editor();
        let mut file = editor.file.clone();
        let warnings = self.install.add_to(&mut file, &items)?;
        editor.file = file;
        editor.added(warnings);
        Ok(())
    }

    /// Whether anything was changed since it was opened or saved.
    pub fn changed(&self) -> bool {
        self.editor().changed
    }

    /// Whether it was saved before, so that Save needs no place.
    pub fn has_path(&self) -> bool {
        self.editor().path.is_some()
    }

    /// The extension of the package.
    pub fn extension(&self) -> String {
        packages::extension(self.editor().file.is_expansion()).to_owned()
    }

    /// Save: where it is, or at `path` for a new package.
    pub fn save(&self, path: Option<String>) -> Result<PackageDone, CoreError> {
        let mut editor = self.editor();
        let Some(path) = path.map(PathBuf::from).or_else(|| editor.path.clone()) else {
            return Err(CoreError::new(cr_i18n::tr("Package Not Saved")));
        };
        packages::write(&path, &editor.file).map_err(CoreError::new)?;
        let (said, warnings) = editor.saved(path);
        Ok(PackageDone { said, warnings })
    }
}

/// What Edit Package… opened.
#[derive(uniffi::Enum)]
pub enum PackageOpened {
    Editor {
        session: Arc<PackageSession>,
    },
    /// A package of Core Rules 2.0, which is only imported: what to say.
    Core20 {
        title: String,
        text: String,
    },
}

/// New Package….
pub(crate) fn new_package(install: &Arc<Installation>) -> Arc<PackageSession> {
    let expansion = !install.is_core20();
    PackageSession::new(
        install.clone(),
        PackageEditor::new(ExportFile::new(expansion), None, expansion),
    )
}

/// Edit Package… on the package at `path`.
pub(crate) fn edit_package(
    install: &Arc<Installation>,
    path: &str,
) -> Result<PackageOpened, CoreError> {
    let path = PathBuf::from(path);
    let file = packages::read(&path).map_err(CoreError::new)?;
    // 2.0 content is only imported, converted to the Expansion's layout
    // on the way in.
    if !file.is_expansion() {
        let (title, text) = packages::core20_only();
        return Ok(PackageOpened::Core20 {
            title: title.to_owned(),
            text: text.to_owned(),
        });
    }
    let expansion = !install.is_core20();
    Ok(PackageOpened::Editor {
        session: PackageSession::new(
            install.clone(),
            PackageEditor::new(file, Some(path), expansion),
        ),
    })
}
