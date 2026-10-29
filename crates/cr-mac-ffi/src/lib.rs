//! The bridge between the Mac app and the Rust backend, exported to Swift
//! with UniFFI. The Swift interface only draws and reports; what it shows
//! and what it decides come from `cr-app` and `cr-store` through here, as
//! the GNOME and Windows apps take them directly. The bridge grows with
//! each screen ported to the Mac; it holds no rule of its own.

mod edit;
mod import;
mod prompt;
mod wizard;

use std::sync::Arc;

use cr_app::library::Filter;
use cr_app::sheets::Export;
use cr_store::Installation;
use cr_store::catalog::Section;

pub use edit::{EditDetails, EditSession, SaveState};
pub use import::{ImportCategory, ImportItem, ImportReport, ImportSession};
pub use prompt::{PromptInfo, PromptKind};
pub use wizard::{SettingInfo, WizardPage, WizardSession};

uniffi::setup_scaffolding!();

/// What went wrong, as a sentence for the user.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CoreError {
    #[error("{message}")]
    Failed { message: String },
}

impl CoreError {
    fn new(message: impl ToString) -> Self {
        CoreError::Failed {
            message: message.to_string(),
        }
    }
}

impl From<cr_store::Error> for CoreError {
    fn from(e: cr_store::Error) -> Self {
        CoreError::new(e)
    }
}

/// The app's version, as the other apps show it.
#[uniffi::export]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// A language of the interface.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Language {
    /// The catalog's code: `pt_BR`.
    pub code: String,
    /// Its name in itself: `Português (Brasil)`.
    pub native_name: String,
}

/// The languages of the interface, in the order the language choice
/// lists them.
#[uniffi::export]
pub fn languages() -> Vec<Language> {
    cr_i18n::Language::ALL
        .into_iter()
        .map(|l| Language {
            code: l.code().to_owned(),
            native_name: l.native_name().to_owned(),
        })
        .collect()
}

/// The code of the first of `preferred` that is one of ours, English
/// otherwise. The Mac app passes the languages chosen in System Settings
/// (`Locale.preferredLanguages`, as `pt-BR`), which macOS does not put in
/// `LANG`.
#[uniffi::export]
pub fn language_for(preferred: Vec<String>) -> String {
    preferred
        .iter()
        .find_map(|l| cr_i18n::Language::from_locale(l))
        .unwrap_or(cr_i18n::Language::English)
        .code()
        .to_owned()
}

/// Sets the interface language by its code, before the first translated
/// text; an unknown code leaves English.
#[uniffi::export]
pub fn set_language(code: String) {
    if let Some(language) = cr_i18n::Language::from_code(&code) {
        cr_i18n::set_language(language);
    }
}

/// The translation of an interface text, or the text itself.
#[uniffi::export]
pub fn tr(msgid: String) -> String {
    cr_i18n::tr(&msgid).to_owned()
}

/// An action of the app, as `cr_app::action` names it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ActionInfo {
    pub group: String,
    pub name: String,
    /// The name with its group: `win.new-character`.
    pub detailed: String,
    /// The name of its constant: `NEW_CHARACTER`.
    pub constant: String,
}

/// Every action of the app.
#[uniffi::export]
pub fn actions() -> Vec<ActionInfo> {
    cr_app::action::ALL
        .iter()
        .map(|a| ActionInfo {
            group: a.group.to_owned(),
            name: a.name.to_owned(),
            detailed: a.detailed.to_owned(),
            constant: a.constant.to_owned(),
        })
        .collect()
}

/// Where Extract Data from CD… puts the data package, the way texts for
/// the user name it.
#[uniffi::export]
pub fn data_package_shown() -> String {
    cr_store::DATA_PACKAGE_SHOWN.to_owned()
}

/// A section of the library, without its entries.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SectionInfo {
    pub id: String,
    pub title: String,
    pub count: u32,
    /// Why its files could not be read, when they could not.
    pub error: Option<String>,
}

/// A row of a section's list.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EntryRow {
    /// Its position in the section, which [`Library::details`] takes.
    pub index: u32,
    pub key: String,
    pub title: String,
    pub subtitle: String,
    /// A character of the list, which can be edited and deleted.
    pub character: bool,
}

/// A label and its value in an entry's details.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Row {
    pub label: String,
    pub value: String,
}

/// A titled group of rows in an entry's details.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Group {
    pub title: String,
    pub rows: Vec<Row>,
}

/// A file a character's sheet is exported to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SheetExport {
    Html,
    Text,
    Markdown,
}

impl SheetExport {
    fn of(export: Export) -> Self {
        match export {
            Export::Html => Self::Html,
            Export::Text => Self::Text,
            Export::Markdown => Self::Markdown,
        }
    }

    fn export(self) -> Export {
        match self {
            Self::Html => Export::Html,
            Self::Text => Export::Text,
            Self::Markdown => Export::Markdown,
        }
    }
}

/// The file name's extension of an export: `html`.
#[uniffi::export]
pub fn export_extension(export: SheetExport) -> String {
    export.export().extension().to_owned()
}

/// The menu item of an export: "Export as _HTML…".
#[uniffi::export]
pub fn export_label(export: SheetExport) -> String {
    export.export().label().to_owned()
}

/// The name the original suggests for a file of `title`:
/// `Borin_Pedraforte.pdf`.
#[uniffi::export]
pub fn file_name(title: String, extension: String) -> String {
    cr_app::file_name(title.trim(), &extension)
}

/// What the details pane shows of an entry.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EntryDetails {
    pub title: String,
    pub subtitle: String,
    pub groups: Vec<Group>,
    /// Long rules text, shown below the groups.
    pub text: String,
    /// The path of its picture, when it has one.
    pub picture: Option<String>,
    /// A character of the list, which has a sheet and can be edited and
    /// deleted.
    pub character: bool,
}

/// A Core Rules installation or data package, and its catalog as it was
/// read; [`Library::reload`] reads it again after a change.
#[derive(uniffi::Object)]
pub struct Library {
    install: Arc<Installation>,
    sections: Vec<Section>,
}

impl Library {
    fn new(install: Installation) -> Arc<Self> {
        let install = Arc::new(install);
        let sections = install.catalog();
        Arc::new(Self { install, sections })
    }

    fn section(&self, id: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.id == id)
    }
}

/// The data package Extract Data from CD… made, else an installation in
/// the usual Wine prefix, when there is one.
#[uniffi::export]
pub fn discover() -> Option<Arc<Library>> {
    Installation::discover().map(Library::new)
}

#[uniffi::export]
impl Library {
    /// The installation, CD or data package in folder `path`.
    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Arc<Self>, CoreError> {
        Ok(Self::new(Installation::open(path)?))
    }

    /// The same folder, its catalog read again.
    pub fn reload(&self) -> Result<Arc<Self>, CoreError> {
        Self::open(self.root())
    }

    /// The folder it was opened from.
    pub fn root(&self) -> String {
        self.install.root().display().to_string()
    }

    /// Whether changes can be saved into it (a CD cannot take them).
    pub fn is_writable(&self) -> bool {
        self.install.is_writable()
    }

    /// The sections of the catalog, in the order the apps list them.
    pub fn sections(&self) -> Vec<SectionInfo> {
        self.sections
            .iter()
            .map(|s| SectionInfo {
                id: s.id.to_owned(),
                title: s.title.to_owned(),
                count: s.entries.len() as u32,
                error: s.error.clone(),
            })
            .collect()
    }

    /// The rows of section `section` that `search` matches, as the other
    /// apps' search box filters them.
    pub fn entries(&self, section: String, search: String) -> Vec<EntryRow> {
        let Some(section) = self.section(&section) else {
            return Vec::new();
        };
        let filter = Filter {
            search,
            ..Filter::default()
        };
        filter
            .visible(section)
            .into_iter()
            .map(|i| {
                let e = &section.entries[i];
                EntryRow {
                    index: i as u32,
                    key: e.key.clone(),
                    title: e.title.clone(),
                    subtitle: e.subtitle.clone(),
                    character: cr_app::library::is_character(&e.key),
                }
            })
            .collect()
    }

    /// The details of entry `index` of section `section`.
    pub fn details(&self, section: String, index: u32) -> Option<EntryDetails> {
        let e = self.section(&section)?.entries.get(index as usize)?;
        Some(EntryDetails {
            title: e.title.clone(),
            subtitle: e.subtitle.clone(),
            groups: e
                .groups
                .iter()
                .map(|g| Group {
                    title: g.title.clone(),
                    rows: g
                        .rows
                        .iter()
                        .map(|(label, value)| Row {
                            label: label.clone(),
                            value: value.clone(),
                        })
                        .collect(),
                })
                .collect(),
            text: e.text.clone(),
            picture: e.picture.as_ref().map(|p| p.display().to_string()),
            character: cr_app::library::is_character(&e.key),
        })
    }

    /// Removes the character of `Chars/<file>` from the list and its
    /// file, as Delete does without the GNOME app's Undo; gives its name.
    pub fn delete_character(&self, file: String) -> Result<String, CoreError> {
        let deleted = self.install.delete_character(&file)?;
        self.install
            .forget_deleted(std::slice::from_ref(&deleted))?;
        Ok(deleted.name.trim().to_owned())
    }

    /// The sheet of the character in `Chars/<file>` as a PDF, on Letter
    /// paper or A4.
    pub fn sheet_pdf(&self, file: String, letter: bool) -> Result<Vec<u8>, CoreError> {
        let (sheet, _) = cr_app::sheets::character(&self.install, &file)?;
        let paper = if letter {
            cr_app::pdf::Paper::Letter
        } else {
            cr_app::pdf::Paper::A4
        };
        Ok(cr_app::pdf::sheet(&sheet, paper))
    }

    /// The files a sheet is exported to in this installation.
    pub fn sheet_exports(&self) -> Vec<SheetExport> {
        self.install
            .sheet_rules()
            .map(|rules| Export::of(rules.flavor()))
            .unwrap_or_default()
            .into_iter()
            .map(SheetExport::of)
            .collect()
    }

    /// The sheet of the character in `Chars/<file>` as `export` writes it.
    pub fn sheet_export(&self, file: String, export: SheetExport) -> Result<Vec<u8>, CoreError> {
        let (sheet, _) = cr_app::sheets::character(&self.install, &file)?;
        Ok(export.export().write(&sheet))
    }

    /// Import…: what the package in `path` would bring in.
    pub fn import_package(&self, path: String) -> Result<Arc<ImportSession>, CoreError> {
        ImportSession::open(self.install.clone(), &path)
    }

    /// New Character, from its first page. `core_vs_option` says whether
    /// the Expansion's Core Rules Vs Player's Option message is still
    /// shown.
    pub fn new_character(&self, core_vs_option: bool) -> Result<Arc<WizardSession>, CoreError> {
        WizardSession::open(&self.install, core_vs_option)
    }

    /// Edit Character on the character in `Chars/<file>`.
    pub fn edit_character(&self, file: String) -> Result<Arc<EditSession>, CoreError> {
        EditSession::open(self.install.clone(), &file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_languages_of_macos_are_ours() {
        assert_eq!(language_for(vec!["pt-BR".into()]), "pt_BR");
        assert_eq!(language_for(vec!["ja-JP".into(), "de-DE".into()]), "de");
        assert_eq!(language_for(vec!["ja-JP".into()]), "en_US");
        assert_eq!(language_for(Vec::new()), "en_US");
    }

    #[test]
    fn every_action_crosses_the_bridge() {
        let all = actions();
        assert_eq!(all.len(), cr_app::action::ALL.len());
        assert!(
            all.iter()
                .any(|a| a.constant == "NEW_CHARACTER" && a.detailed == "win.new-character")
        );
    }

    #[test]
    fn a_folder_without_a_database_is_refused() {
        let empty = std::env::temp_dir().join(format!("cr-mac-ffi-{}", std::process::id()));
        std::fs::create_dir_all(&empty).unwrap();
        let refused = Library::open(empty.display().to_string());
        std::fs::remove_dir_all(&empty).unwrap();
        assert!(matches!(refused, Err(CoreError::Failed { .. })));
    }
}
