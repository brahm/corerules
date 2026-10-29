//! The bridge between the Mac app and the Rust backend, exported to Swift
//! with UniFFI. The Swift interface only draws and reports; what it shows
//! and what it decides come from `cr-app` and `cr-store` through here, as
//! the GNOME and Windows apps take them directly. The bridge grows with
//! each screen ported to the Mac; it holds no rule of its own.

mod edit;
mod help;
mod import;
mod prompt;
mod sheet;
mod wizard;

use std::sync::{Arc, OnceLock};

use cr_app::library::{self, Commands, Filter};
use cr_app::sheets::Export;
use cr_store::Installation;
use cr_store::catalog::{Entry, Section};
use cr_store::userdb::Filter as Origins;
use cr_store::userdb::parts::{OTHER_SECTION, OTHER_TYPES};

pub use edit::{EditDetails, EditSession, SaveState};
pub use help::{HelpBook, HelpLinked, HelpRow, HelpSession, HelpTopic};
pub use import::{ImportCategory, ImportItem, ImportReport, ImportSession};
pub use prompt::{PromptInfo, PromptKind};
pub use sheet::{SheetOptions, SheetSectionInfo, SheetSession};
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

/// What the entry list shows of a section, as the other apps' search and
/// filters choose it.
#[derive(Debug, Clone, PartialEq, Eq, Default, uniffi::Record)]
pub struct ListFilter {
    pub search: String,
    /// All, Standard or Custom records, by its place in
    /// [`origin_names`].
    pub origins: u32,
    /// Every party (`None`), the characters in none (empty) or a party's.
    pub party: Option<String>,
    /// The type of the other parts shown, by its id, or every type.
    pub part_type: Option<String>,
    pub magic_only: bool,
}

/// A choice of a filter: what it shows, and its value.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FilterChoice {
    pub label: String,
    pub value: Option<String>,
}

/// The filters a section offers beside the search.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SectionFilters {
    /// Standard or custom records, in the user databases' sections.
    pub origins: bool,
    /// The parties, when the characters are in more than one.
    pub parties: Vec<FilterChoice>,
    /// The part types, in Other Parts.
    pub part_types: Vec<FilterChoice>,
    /// Magic Only, in the sections of parts.
    pub magic: bool,
}

/// The choices of the origin filter: All, Standard, Custom.
#[uniffi::export]
pub fn origin_names() -> Vec<String> {
    Origins::NAMES
        .iter()
        .map(|n| cr_i18n::tr(n).to_owned())
        .collect()
}

/// The commands an entry offers, as `cr_app::library` decides them.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EntryCommands {
    pub edit: bool,
    pub copy: bool,
    pub records: bool,
    pub sheet: bool,
    pub to_character: bool,
    pub to_npc: bool,
    pub delete: bool,
    pub generate: bool,
    pub view: bool,
    pub monster: bool,
    pub print: bool,
    pub read: bool,
}

impl EntryCommands {
    fn of(entry: &Entry, writable: bool) -> Self {
        let c = Commands::of(entry, writable);
        Self {
            edit: c.edit,
            copy: c.copy,
            records: c.records,
            sheet: c.sheet,
            to_character: c.to_character,
            to_npc: c.to_npc,
            delete: c.delete,
            generate: c.generate,
            view: c.view,
            monster: c.monster,
            print: c.print,
            read: c.read,
        }
    }
}

/// A picture of the original's as PNG.
fn png(image: &cr_formats::bmp::Image) -> Option<Vec<u8>> {
    let rgba = image::RgbaImage::from_raw(image.width, image.height, image.rgba.clone())?;
    let mut png = std::io::Cursor::new(Vec::new());
    rgba.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(png.into_inner())
}

/// A monster type's frame Next Image shows.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FrameImage {
    /// Its place among the type's frames, which the next Next Image takes.
    pub at: u32,
    pub png: Vec<u8>,
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
    /// An item's icon, as PNG.
    pub icon: Option<Vec<u8>>,
    /// Its picture, as PNG.
    pub picture: Option<Vec<u8>>,
    /// The picture is a monster's, with Full Screen.
    pub monster_picture: bool,
    /// A monster type with frames, which Next Image goes through.
    pub frames: bool,
    /// A character of the list.
    pub character: bool,
    pub commands: EntryCommands,
}

/// A Core Rules installation or data package, and its catalog as it was
/// read; [`Library::reload`] reads it again after a change.
#[derive(uniffi::Object)]
pub struct Library {
    install: Arc<Installation>,
    sections: Vec<Section>,
    /// The help files, read as they are opened.
    help_files: OnceLock<Arc<cr_app::help::Files>>,
    /// The records' help, read once.
    items: OnceLock<Arc<cr_store::help::ItemHelp>>,
}

impl Library {
    fn new(install: Installation) -> Arc<Self> {
        let install = Arc::new(install);
        let sections = install.catalog();
        Arc::new(Self {
            install,
            sections,
            help_files: OnceLock::new(),
            items: OnceLock::new(),
        })
    }

    fn help_files(&self) -> Result<Arc<cr_app::help::Files>, CoreError> {
        if let Some(files) = self.help_files.get() {
            return Ok(files.clone());
        }
        let install = Installation::open(self.install.root())?;
        Ok(self
            .help_files
            .get_or_init(|| Arc::new(cr_app::help::Files::new(install)))
            .clone())
    }

    fn items(&self) -> Arc<cr_store::help::ItemHelp> {
        self.items
            .get_or_init(|| Arc::new(self.install.item_help()))
            .clone()
    }

    fn entry(&self, section: &str, index: u32) -> Option<&Entry> {
        self.section(section)?.entries.get(index as usize)
    }

    fn section(&self, id: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.id == id)
    }
}

/// What an entry's Help, or a monster's View and Print, show.
#[derive(uniffi::Enum)]
pub enum EntryHelp {
    /// A topic of the help, by its spec for [`HelpSession::open`].
    Open { spec: String },
    /// A popup topic, a record's own help or a summary, as text.
    Info { title: String, text: String },
    /// A page shown as it prints.
    Page { sheet: Arc<SheetSession> },
    /// Nothing: what the app says.
    Nothing { message: String },
}

/// The program help's topic of the library's section `section`, as Help
/// on This Screen opens it.
#[uniffi::export]
pub fn section_help(section: String) -> u32 {
    cr_store::help::topic::of_section(&section)
}

/// The program help's topics of the screens other than the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HelpScreen {
    MainMenu,
    CharacterSheet,
    Import,
}

#[uniffi::export]
pub fn screen_help(screen: HelpScreen) -> u32 {
    use cr_store::help::topic;
    match screen {
        HelpScreen::MainMenu => topic::MAIN_MENU,
        HelpScreen::CharacterSheet => topic::CHARACTER_SHEET,
        HelpScreen::Import => topic::IMPORT,
    }
}

/// What About shows of the app.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AboutInfo {
    pub version: String,
    /// The description, one paragraph each.
    pub paragraphs: Vec<String>,
    pub copyright: String,
    pub website: String,
    pub issues: String,
    pub developer: String,
    pub developer_url: String,
    pub assistant: String,
    pub assistant_url: String,
    /// The catalog's translator credits, one per line.
    pub translators: Option<String>,
    /// What's New: a line for each paragraph, "• " before a list's item.
    pub release_notes: Vec<String>,
    /// The notice on the original's content.
    pub legal: String,
}

#[uniffi::export]
pub fn about() -> AboutInfo {
    use cr_app::about;
    AboutInfo {
        version: about::VERSION.to_owned(),
        paragraphs: about::description(about::System::MacOs)
            .iter()
            .map(|p| (*p).to_owned())
            .collect(),
        copyright: about::COPYRIGHT.to_owned(),
        website: about::WEBSITE.to_owned(),
        issues: about::ISSUES.to_owned(),
        developer: about::DEVELOPER.to_owned(),
        developer_url: about::DEVELOPER_URL.to_owned(),
        assistant: about::ASSISTANT.to_owned(),
        assistant_url: about::ASSISTANT_URL.to_owned(),
        translators: about::translators().map(str::to_owned),
        release_notes: about::release_notes_text(),
        legal: about::legal().to_owned(),
    }
}

/// The original program's About box and Credits window, as the
/// installation has them.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OriginalInfo {
    /// The About box: the product, then what it says and its license.
    pub about_title: Option<String>,
    pub about_text: String,
    /// The Credits window's title and lines.
    pub credits_title: Option<String>,
    pub credits: Vec<String>,
}

/// What a bug report needs: the versions, the language and the data
/// opened, as the other apps give them.
#[uniffi::export]
pub fn debug_info(data: Option<String>, macos: String) -> String {
    format!(
        "Core Rules {}\nmacOS {macos}\nLanguage: {}\nData: {}\n",
        cr_app::about::VERSION,
        cr_i18n::language().code(),
        data.unwrap_or_else(|| "none".to_owned()),
    )
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

    /// Whether changes can be saved into it: a CD cannot take them, and
    /// Core Rules 2.0 is only read.
    pub fn is_writable(&self) -> bool {
        !self.install.is_core20() && self.install.is_writable()
    }

    /// Whether it is Core Rules 2.0, which is only read.
    pub fn is_core20(&self) -> bool {
        self.install.is_core20()
    }

    /// The filters section `section` offers.
    pub fn filters(&self, section: String) -> SectionFilters {
        let parties = if section == library::CHARACTERS {
            let parties = library::parties(&self.sections);
            if parties.len() > 1 {
                parties
                    .iter()
                    .map(|p| FilterChoice {
                        label: library::party_label(p.as_deref()).to_owned(),
                        value: p.clone(),
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        let part_types = if section == OTHER_SECTION {
            std::iter::once(FilterChoice {
                label: cr_i18n::tr("All Types").to_owned(),
                value: None,
            })
            .chain(OTHER_TYPES.iter().map(|(id, _, name)| FilterChoice {
                label: cr_i18n::tr(name).to_owned(),
                value: Some((*id).to_owned()),
            }))
            .collect()
        } else {
            Vec::new()
        };
        SectionFilters {
            origins: cr_app::records::has_origins(&section),
            parties,
            part_types,
            magic: library::is_parts(&section),
        }
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
    pub fn entries(&self, section: String, filter: ListFilter) -> Vec<EntryRow> {
        let Some(section) = self.section(&section) else {
            return Vec::new();
        };
        let filter = Filter {
            search: filter.search,
            origins: Origins::of(filter.origins),
            party: filter.party,
            part_type: filter.part_type.and_then(|t| {
                OTHER_TYPES
                    .iter()
                    .find(|(id, ..)| *id == t)
                    .map(|(id, ..)| *id)
            }),
            magic_only: filter.magic_only,
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
            text: library::clean_text(&e.text),
            icon: e
                .icon
                .as_ref()
                .and_then(cr_app::pictures::icon)
                .and_then(|i| png(&i)),
            picture: e.picture.as_deref().and_then(|path| {
                if library::is_monster(&e.key) {
                    cr_app::pictures::monster(path, e.frames.as_ref())
                } else {
                    cr_app::pictures::load(path)
                }
                .and_then(|i| png(&i))
            }),
            monster_picture: library::is_monster(&e.key) && e.picture.is_some(),
            frames: library::is_monster(&e.key) && e.frames.is_some(),
            character: cr_app::library::is_character(&e.key),
            commands: EntryCommands::of(e, self.is_writable()),
        })
    }

    /// Next Image on entry `index` of section `section`, whose frame
    /// shown is `at`: the next frame, back to the first after the last.
    pub fn next_frame(&self, section: String, index: u32, at: u32) -> Option<FrameImage> {
        let e = self.section(&section)?.entries.get(index as usize)?;
        let frames = e.frames.as_ref()?;
        let next = cr_app::pictures::next_frame(frames, at);
        let image = cr_app::pictures::frame(&frames.page, frames.first + next)?;
        Some(FrameImage {
            at: next,
            png: png(&image)?,
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

    /// The original program's About box and Credits window.
    pub fn original(&self) -> OriginalInfo {
        let original = cr_app::about::Original::of(&self.install.original_about());
        let (about_title, about_text) = match original.about {
            Some(about) => (
                Some(about.name),
                [about.rest, about.license.unwrap_or_default()]
                    .into_iter()
                    .filter(|t| !t.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            ),
            None => (None, String::new()),
        };
        let (credits_title, credits) = match original.credits {
            Some((title, lines)) => (Some(title), lines),
            None => (None, Vec::new()),
        };
        OriginalInfo {
            about_title,
            about_text,
            credits_title,
            credits,
        }
    }

    /// The help, to open with [`HelpSession::open`].
    pub fn help(&self) -> Result<Arc<HelpSession>, CoreError> {
        Ok(HelpSession::new(self.help_files()?))
    }

    /// What a right-click on the row of entry `index` of section
    /// `section` shows in the original.
    pub fn entry_help(&self, section: String, index: u32) -> EntryHelp {
        use cr_app::help::Found;
        let Some(entry) = self.entry(&section, index) else {
            return EntryHelp::Nothing {
                message: String::new(),
            };
        };
        let name = entry.title.trim().to_owned();
        let none = || EntryHelp::Nothing {
            message: cr_i18n::tr!("{name} has no help topic", name = name.clone()),
        };
        let screen = cr_store::help::topic::of_section(&section);
        let controls = self.install.screen_help(screen);
        let texts = [entry.title.trim(), entry.subtitle.as_str()];
        match cr_app::help::context_help(&self.install, screen, &texts, &controls, || {
            self.items()
        }) {
            Some(Found::Topic(found)) if found.popup => self
                .help_files()
                .ok()
                .and_then(|files| help::popup_text(&files, found.file, found.number))
                .map_or_else(none, |(title, text)| EntryHelp::Info { title, text }),
            Some(Found::Topic(found)) => EntryHelp::Open {
                spec: format!("{}#{}", found.file, found.number),
            },
            Some(Found::Custom { title, text } | Found::Text { title, text }) => {
                EntryHelp::Info { title, text }
            }
            Some(Found::Page(sheet)) => EntryHelp::Page {
                sheet: SheetSession::of(self.install.clone(), sheet),
            },
            None => none(),
        }
    }

    /// The Monsters screen's View and Print on entry `index` of section
    /// `section`: a shipped monster's help topic, a monster of the user
    /// database as it prints.
    pub fn monster_help(&self, section: String, index: u32) -> EntryHelp {
        use cr_app::help::MonsterHelp;
        let Some((name, custom)) = self
            .entry(&section, index)
            .and_then(|e| library::monster_key(&e.key))
        else {
            return EntryHelp::Nothing {
                message: String::new(),
            };
        };
        match cr_app::help::monster(&self.install, name, custom, || self.items()) {
            MonsterHelp::Page(sheet) => EntryHelp::Page {
                sheet: SheetSession::of(self.install.clone(), sheet),
            },
            MonsterHelp::Topic(found) => EntryHelp::Open {
                spec: format!("{}#{}", found.file, found.number),
            },
            MonsterHelp::None => EntryHelp::Nothing {
                message: cr_i18n::tr!("{name} has no help topic", name = name.to_owned()),
            },
        }
    }

    /// The sheet of the character in `Chars/<file>`.
    pub fn character_sheet(&self, file: String) -> Result<Arc<SheetSession>, CoreError> {
        SheetSession::character(self.install.clone(), &file, self.is_writable())
    }

    /// The files a character's sheet is exported to in this
    /// installation.
    pub fn sheet_exports(&self) -> Vec<SheetExport> {
        self.install
            .sheet_rules()
            .map(|rules| Export::of(rules.flavor()))
            .unwrap_or_default()
            .into_iter()
            .map(SheetExport::of)
            .collect()
    }

    /// The page that View shows of the entry with key `key`: an
    /// encounter, encounter table, template, treasure table or saved
    /// treasure.
    pub fn page(&self, key: String) -> Result<Arc<SheetSession>, CoreError> {
        SheetSession::page(self.install.clone(), &key)
    }

    /// Convert to Character, on a character the NPC generator made: what
    /// the app says once it is done.
    pub fn convert_to_character(&self, file: String) -> Result<String, CoreError> {
        self.install.convert_to_character(&file)?;
        Ok(cr_i18n::tr("Converted to a character").to_owned())
    }

    /// Convert to NPC, on a player character: the new file of the NPC.
    pub fn convert_to_npc(&self, file: String) -> Result<String, CoreError> {
        Ok(self.install.convert_to_npc(&file)?)
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
