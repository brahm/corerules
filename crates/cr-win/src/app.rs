//! The window: a title bar over a NavigationView with the library's
//! sections and Settings, as Windows 11 apps are laid out. One component
//! holds the state and handles every message; the pages are drawn by the
//! functions of the modules below, from that state.

mod edit;
mod import;
mod library;
mod new_character;
mod settings;
mod sheet;

use std::path::PathBuf;
use std::sync::Arc;

use cr_app::action::{self, Action};
use cr_app::library::Filter;
use cr_app::sheets::Export;
use cr_formats::export::ExportFile;
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::catalog::{Entry, Section};
use cr_store::export::{Content, ImportReport};
use cr_store::sheet::Sheet;
use cr_store::userdb::parts::OTHER_TYPES;
use cr_win::text::plain;
use windows_reactor::*;

use crate::native::{self, Owner};
pub use edit::Edit;
use library::Pictures;
use sheet::{OptionsEdit, SheetOptions, SheetPage};

/// Starts the app; `cr-win <file>` opens a package, as File Explorer does
/// for a package the app is the default for.
pub fn run() {
    crate::prefs::apply_language();
    let file = std::env::args_os().nth(1).map(PathBuf::from);
    if App::run_component::<Shell>(file).is_err() {
        // Reactor has already said why (the Windows App Runtime missing).
        std::process::exit(1);
    }
}

fn can_write(install: &Installation) -> bool {
    !install.is_core20() && install.is_writable()
}

/// Whether the Windows app has `action` yet; the ones it lacks
/// ([`action::NOT_YET_ON_WINDOWS`]) are not offered.
pub fn available(action: Action) -> bool {
    !action::NOT_YET_ON_WINDOWS.contains(&action)
}

/// The navigation's tag for Settings; the sections are tagged by id.
const SETTINGS: &str = "Settings";

pub struct Shell {
    install: Option<Arc<Installation>>,
    /// Whether the installation takes changes: an Expansion installation
    /// the user may write to (Core Rules 2.0 is opened read-only).
    can_write: bool,
    /// The catalog, once read.
    sections: Option<Arc<Vec<Section>>>,
    /// The section shown, by id.
    section: String,
    filter: Filter,
    /// The choices of the party filter.
    parties: Vec<Option<String>>,
    /// The entry on the details, by key, and its pictures.
    selected: Option<String>,
    pictures: Pictures,
    /// Monster Full Screen is shown, over this background.
    full_screen: bool,
    background: Option<EncodedImage>,
    page: Page,
    /// The pages Back returns to.
    history: Vec<Page>,
    notice: Option<Notice>,
    confirm: Option<Confirm>,
    pane_open: bool,
    /// A dialog or a print job is under way.
    busy: bool,
}

/// What the window shows.
#[derive(Clone)]
pub enum Page {
    Library,
    Edit(Box<Edit>),
    NewCharacter {
        skills_and_powers: bool,
        setting: usize,
    },
    Import(Arc<Package>),
    Settings,
    Sheet(Arc<SheetPage>),
    SheetOptions(Box<SheetOptions>),
}

/// A package opened for import, with what it would bring in.
pub struct Package {
    pub path: PathBuf,
    pub file: ExportFile,
    pub preview: Vec<Content>,
    pub expansion: bool,
}

/// The InfoBar at the top of the page.
#[derive(Clone, PartialEq)]
pub struct Notice {
    pub severity: InfoBarSeverity,
    pub title: String,
    pub message: String,
}

impl Notice {
    fn new(
        severity: InfoBarSeverity,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            title: title.into(),
            message: message.into(),
        }
    }

    fn success(title: impl Into<String>) -> Self {
        Self::new(InfoBarSeverity::Success, title, "")
    }
}

/// A question a ContentDialog asks before something that cannot be
/// undone.
#[derive(Clone, PartialEq)]
pub enum Confirm {
    Delete {
        file: String,
        name: String,
    },
    /// Changes would be lost; then the section `section` is shown, or
    /// the page before.
    Discard {
        section: Option<String>,
    },
    ToNpc {
        file: String,
    },
    /// Character Sheet Options with no section.
    EmptySheet,
}

/// What Print and Save as PDF act on.
enum Printable {
    /// The sheet of the character in this file.
    Character(String),
    /// The page of the entry with this key.
    Page(String),
    Sheet(Arc<Sheet>),
}

impl Printable {
    fn sheet(self, install: &Installation) -> Result<Sheet, String> {
        match self {
            Self::Character(file) => install
                .character(&file)
                .and_then(|c| install.sheet(&c))
                .map_err(|e| e.to_string()),
            Self::Page(key) => install
                .page(&key)
                .ok_or_else(|| tr("Could Not Read").to_owned()),
            Self::Sheet(sheet) => Ok((*sheet).clone()),
        }
    }
}

#[derive(Clone)]
pub enum Message {
    /// The catalog was read; the entry with this key stays on the
    /// details.
    Loaded(Arc<Vec<Section>>, Option<String>),
    Navigate(Option<String>),
    Back,
    TogglePane,
    PaneOpenChanged(bool),
    Search(String),
    Origins(Option<usize>),
    Party(Option<usize>),
    PartType(Option<usize>),
    MagicOnly(bool),
    Select(Option<usize>),
    NextImage,
    FullScreen(bool),
    CloseNotice,
    /// A command, by the action that does the same in the GNOME app.
    Act(Action),
    FolderChosen(Result<Option<PathBuf>, String>),
    // An entry's commands.
    EditCharacter,
    EditLoaded(Result<Box<Edit>, String>),
    Edit(edit::Field),
    Save,
    Saved(Result<(String, String), String>),
    AskDelete,
    Answered(ContentDialogResult),
    Deleted(Result<String, String>),
    ToCharacter,
    AskToNpc,
    /// A conversion was made: the character's file now, and what to say.
    Converted(Result<(String, &'static str), String>),
    ShowSheet,
    ViewPage,
    SheetLoaded(Result<Arc<SheetPage>, String>),
    Printed(Result<bool, String>),
    PdfSaved(Result<Option<PathBuf>, String>),
    Exported(Result<Option<PathBuf>, String>),
    // Character Sheet Options.
    SheetOptions,
    Options(OptionsEdit),
    SaveOptions,
    OptionsSaved(Result<Sheet, String>),
    // New Character.
    Rules(Option<usize>),
    Setting(Option<usize>),
    Next,
    // Import.
    PackageChosen(Result<Option<Arc<Package>>, String>),
    Import,
    Imported(Result<Arc<ImportReport>, String>),
    // Settings.
    Language(Option<usize>),
}

impl Shell {
    /// Reads the catalog again; the entry with `key` stays on the
    /// details.
    fn load(&mut self, key: Option<String>, context: &ComponentContext<Self>) {
        if let Some(install) = self.install.clone() {
            context.spawn_background(move |_| Message::Loaded(Arc::new(install.catalog()), key));
        }
    }

    fn current_section(&self) -> Option<&Section> {
        self.sections
            .as_ref()?
            .iter()
            .find(|s| s.id == self.section)
    }

    /// The entries of the section that the filters let through.
    fn visible(&self) -> Vec<&Entry> {
        match self.current_section() {
            Some(section) => self
                .filter
                .visible(section)
                .into_iter()
                .map(|i| &section.entries[i])
                .collect(),
            None => Vec::new(),
        }
    }

    fn selected_entry(&self) -> Option<&Entry> {
        let key = self.selected.as_deref()?;
        self.current_section()?
            .entries
            .iter()
            .find(|e| e.key == key)
    }

    fn select(&mut self, key: Option<String>) {
        self.selected = key;
        self.pictures = self.selected_entry().map(Pictures::of).unwrap_or_default();
        self.full_screen = false;
    }

    /// The character on the details, by file.
    fn selected_character(&self) -> Option<String> {
        self.selected
            .clone()
            .filter(|key| cr_app::library::is_character(key))
    }

    fn go(&mut self, page: Page) {
        self.history.push(std::mem::replace(&mut self.page, page));
        self.notice = None;
    }

    fn back(&mut self) {
        self.page = self.history.pop().unwrap_or(Page::Library);
        self.notice = None;
    }

    /// Back to the library, at the section `id`.
    fn show_section(&mut self, id: String) {
        self.history.clear();
        self.page = Page::Library;
        if id != self.section {
            self.section = id;
            // A new section starts without a search, as in the GNOME app.
            self.filter.search.clear();
            self.select(None);
        }
    }

    /// Whether the page shown has changes that leaving it would lose.
    fn changed(&self) -> bool {
        match &self.page {
            Page::Edit(edit) => edit.changed(),
            Page::SheetOptions(options) => options.changed(),
            _ => false,
        }
    }

    fn show_error(&mut self, title: &str, error: impl Into<String>) {
        self.notice = Some(Notice::new(InfoBarSeverity::Error, title, error));
    }

    /// What Print and Save as PDF act on: the sheet shown, or the entry
    /// on the details.
    fn printable(&self) -> Option<(Printable, String)> {
        if let Page::Sheet(page) = &self.page {
            return Some((
                Printable::Sheet(page.sheet.clone()),
                page.sheet.title.clone(),
            ));
        }
        let entry = self.selected_entry()?;
        let title = entry.title.trim().to_owned();
        if cr_app::library::is_character(&entry.key) {
            Some((Printable::Character(entry.key.clone()), title))
        } else if cr_app::library::has_page(&entry.key) {
            Some((Printable::Page(entry.key.clone()), title))
        } else {
            None
        }
    }

    /// Does what `action` does in the GNOME app; the actions the Windows
    /// app does not have yet ([`action::NOT_YET_ON_WINDOWS`]) do nothing.
    fn act(&mut self, action: Action, context: &ComponentContext<Self>) {
        match action {
            action::OPEN => self.dialog(
                context,
                |owner| native::dialogs::open_folder(owner, &plain(tr("_Open Folder…"))),
                |r| Message::FolderChosen(r.map_err(|e| e.message())),
            ),
            action::NEW_CHARACTER => self.go(Page::NewCharacter {
                skills_and_powers: false,
                setting: 0,
            }),
            action::IMPORT => {
                let install = self.install.clone();
                let expansion = install.as_ref().is_some_and(|i| !i.is_core20());
                self.dialog(
                    context,
                    move |owner| {
                        let filters = import::filters(expansion);
                        match native::dialogs::open_file(owner, tr("Import Package"), &filters) {
                            Ok(Some(path)) => import::open(install.as_deref(), path).map(Some),
                            Ok(None) => Ok(None),
                            Err(e) => Err(e.message()),
                        }
                    },
                    Message::PackageChosen,
                );
            }
            action::SHEET_PRINT => {
                let Some(((printable, _), install)) = self.printable().zip(self.install.clone())
                else {
                    return;
                };
                self.dialog(
                    context,
                    move |owner| {
                        let sheet = printable.sheet(&install)?;
                        native::print::print_sheet(owner, &sheet).map_err(|e| e.message())
                    },
                    Message::Printed,
                );
            }
            action::SHEET_PDF => {
                let Some(((printable, title), install)) =
                    self.printable().zip(self.install.clone())
                else {
                    return;
                };
                let name = cr_app::file_name(&title, "pdf");
                self.dialog(
                    context,
                    move |owner| {
                        let filters = [(tr("PDF documents").to_owned(), "*.pdf".to_owned())];
                        let title = plain(tr("Save as _PDF…"));
                        let Some(path) =
                            native::dialogs::save_file(owner, &title, &name, "pdf", &filters)
                                .map_err(|e| e.message())?
                        else {
                            return Ok(None);
                        };
                        let sheet = printable.sheet(&install)?;
                        write(&path, &cr_app::pdf::sheet(&sheet, native::paper()))?;
                        Ok(Some(path))
                    },
                    Message::PdfSaved,
                );
            }
            action::SHEET_HTML => self.export(Export::Html, context),
            action::SHEET_TXT => self.export(Export::Text, context),
            action::SHEET_MD => self.export(Export::Markdown, context),
            // About is on the Settings page, as in Windows 11 apps.
            action::PREFERENCES | action::ABOUT if !matches!(self.page, Page::Settings) => {
                self.go(Page::Settings)
            }
            _ => {}
        }
    }

    /// Exports the character's sheet shown.
    fn export(&mut self, export: Export, context: &ComponentContext<Self>) {
        let Page::Sheet(page) = &self.page else {
            return;
        };
        if page.file.is_none() {
            return;
        }
        let sheet = page.sheet.clone();
        let extension = export.extension();
        let name = cr_app::file_name(&sheet.title, extension);
        self.dialog(
            context,
            move |owner| {
                let filters = [(export.kind().to_owned(), format!("*.{extension}"))];
                let title = plain(export.label());
                let Some(path) =
                    native::dialogs::save_file(owner, &title, &name, extension, &filters)
                        .map_err(|e| e.message())?
                else {
                    return Ok(None);
                };
                write(&path, &export.write(&sheet))?;
                Ok(Some(path))
            },
            Message::Exported,
        );
    }

    /// Runs a dialog of Windows on a thread of its own, with the window
    /// as its owner.
    fn dialog<T: Send + 'static>(
        &mut self,
        context: &ComponentContext<Self>,
        work: impl FnOnce(Owner) -> T + Send + 'static,
        done: impl FnOnce(T) -> Message + Send + 'static,
    ) {
        if self.busy {
            return;
        }
        self.busy = true;
        let owner = Owner::active();
        context.spawn_background(move |_| done(native::on_dialog_thread(move || work(owner))));
    }

    /// Runs `work` on the installation in the background.
    fn with_install(
        &self,
        context: &ComponentContext<Self>,
        work: impl FnOnce(&Installation) -> Message + Send + 'static,
    ) {
        if let Some(install) = self.install.clone() {
            context.spawn_background(move |_| work(&install));
        }
    }
}

/// Writes a file the user chose, saying which when it fails.
fn write(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| {
        tr!(
            "Could not save {file}: {error}",
            file = path.display(),
            error = e
        )
    })
}

impl Component for Shell {
    type Input = Option<PathBuf>;
    type Message = Message;

    fn create(file: &Option<PathBuf>, context: &ComponentContext<Self>) -> Self {
        let install = Installation::discover().map(Arc::new);
        let mut shell = Self {
            can_write: install.as_deref().is_some_and(can_write),
            install,
            sections: None,
            section: cr_app::library::CHARACTERS.to_owned(),
            filter: Filter::default(),
            parties: vec![None],
            selected: None,
            pictures: Pictures::default(),
            full_screen: false,
            background: None,
            page: Page::Library,
            history: Vec::new(),
            notice: None,
            confirm: None,
            pane_open: true,
            busy: false,
        };
        shell.load(None, context);
        if let Some(path) = file.clone() {
            shell.busy = true;
            let install = shell.install.clone();
            context.spawn_background(move |_| {
                Message::PackageChosen(import::open(install.as_deref(), path).map(Some))
            });
        }
        shell
    }

    fn update(&mut self, message: Message, context: &ComponentContext<Self>) {
        match message {
            Message::Loaded(sections, key) => {
                self.parties = cr_app::library::parties(&sections);
                if cr_app::library::party_position(&self.parties, self.filter.party.as_deref()) == 0
                {
                    self.filter.party = None;
                }
                if !sections.iter().any(|s| s.id == self.section)
                    && let Some(first) = sections.first()
                {
                    self.section = first.id.to_owned();
                }
                self.sections = Some(sections);
                let key = key.or_else(|| self.selected.clone());
                let key = key.filter(|k| {
                    self.current_section()
                        .is_some_and(|s| s.entries.iter().any(|e| &e.key == k))
                });
                self.select(key);
            }
            Message::Navigate(Some(tag)) => {
                if tag == SETTINGS {
                    self.act(action::PREFERENCES, context);
                } else if self.changed() {
                    self.confirm = Some(Confirm::Discard { section: Some(tag) });
                } else if tag != self.section || !matches!(self.page, Page::Library) {
                    self.show_section(tag);
                }
            }
            Message::Navigate(None) => {}
            Message::Back => {
                if self.changed() {
                    self.confirm = Some(Confirm::Discard { section: None });
                } else {
                    self.back();
                }
            }
            Message::TogglePane => self.pane_open = !self.pane_open,
            Message::PaneOpenChanged(open) => self.pane_open = open,
            Message::Search(text) => self.filter.search = text,
            Message::Origins(Some(index)) => {
                self.filter.origins = cr_store::userdb::Filter::of(index as u32)
            }
            Message::Party(Some(index)) => {
                if let Some(choice) = self.parties.get(index) {
                    self.filter.party = choice.clone();
                }
            }
            Message::PartType(Some(index)) => {
                self.filter.part_type = index
                    .checked_sub(1)
                    .and_then(|i| OTHER_TYPES.get(i))
                    .map(|(id, ..)| *id);
            }
            Message::Origins(None) | Message::Party(None) | Message::PartType(None) => {}
            Message::MagicOnly(on) => self.filter.magic_only = on,
            Message::Select(index) => {
                if let Some(key) = index.and_then(|i| self.visible().get(i).map(|e| e.key.clone()))
                {
                    self.select(Some(key));
                }
            }
            Message::NextImage => self.pictures.next(),
            Message::FullScreen(true) => {
                if self.background.is_none() {
                    self.background = self
                        .install
                        .as_ref()
                        .and_then(|i| i.full_screen_background())
                        .and_then(|p| crate::ui::picture(&p));
                }
                self.full_screen = true;
            }
            Message::FullScreen(false) => self.full_screen = false,
            Message::CloseNotice => self.notice = None,
            Message::Act(action) => self.act(action, context),
            Message::FolderChosen(chosen) => {
                self.busy = false;
                match chosen {
                    Ok(Some(path)) => match Installation::open(&path) {
                        Ok(install) => {
                            self.can_write = can_write(&install);
                            self.install = Some(Arc::new(install));
                            self.sections = None;
                            self.background = None;
                            self.history.clear();
                            self.page = Page::Library;
                            self.select(None);
                            self.notice = None;
                            self.load(None, context);
                        }
                        Err(e) => self.show_error(tr("No Core Rules Installation"), e.to_string()),
                    },
                    Ok(None) => {}
                    Err(e) => self.show_error(tr("No Core Rules Installation"), e),
                }
            }
            Message::EditCharacter => {
                if let Some(file) = self.selected_character() {
                    self.with_install(context, move |install| {
                        Message::EditLoaded(Edit::load(install, &file).map(Box::new))
                    });
                }
            }
            Message::EditLoaded(Ok(edit)) => self.go(Page::Edit(edit)),
            Message::EditLoaded(Err(e)) => self.show_error(tr("Edit Character"), e),
            Message::Edit(field) => {
                if let Page::Edit(edit) = &mut self.page {
                    edit.set(field);
                }
            }
            Message::Save => {
                if let (Page::Edit(edit), Some(install)) = (&mut self.page, self.install.clone())
                    && edit.problem().is_none()
                    && !edit.saving
                {
                    edit.saving = true;
                    let (file, details) = (edit.file.clone(), edit.details.clone());
                    context.spawn_background(move |_| {
                        Message::Saved(
                            install
                                .save_character_details(&file, &details)
                                .map(|()| (file, details.name.trim().to_owned()))
                                .map_err(|e| e.to_string()),
                        )
                    });
                }
            }
            Message::Saved(Ok((file, name))) => {
                self.back();
                self.notice = Some(Notice::success(tr!("Saved {name}", name)));
                self.load(Some(file), context);
            }
            Message::Saved(Err(e)) => {
                if let Page::Edit(edit) = &mut self.page {
                    edit.saving = false;
                }
                self.show_error(tr("Edit Character"), e);
            }
            Message::AskDelete => {
                if let Some(entry) = self.selected_entry() {
                    self.confirm = Some(Confirm::Delete {
                        file: entry.key.clone(),
                        name: entry.title.trim().to_owned(),
                    });
                }
            }
            Message::AskToNpc => {
                if let Some(file) = self.selected_character() {
                    self.confirm = Some(Confirm::ToNpc { file });
                }
            }
            Message::ToCharacter => {
                if let Some(file) = self.selected_character() {
                    self.with_install(context, move |install| {
                        Message::Converted(
                            install
                                .convert_to_character(&file)
                                .map(|()| (file, tr("Converted to a character")))
                                .map_err(|e| e.to_string()),
                        )
                    });
                }
            }
            Message::Answered(answer) => match (self.confirm.take(), answer) {
                (Some(Confirm::Delete { file, .. }), ContentDialogResult::Primary) => {
                    self.with_install(context, move |install| {
                        // Without the GTK app's Undo, the kept file goes at
                        // once, as when its toast closes.
                        let deleted = install
                            .delete_character(&file)
                            .and_then(|d| {
                                install.forget_deleted(std::slice::from_ref(&d))?;
                                Ok(d.name.trim().to_owned())
                            })
                            .map_err(|e| e.to_string());
                        Message::Deleted(deleted)
                    });
                }
                (Some(Confirm::ToNpc { file }), ContentDialogResult::Primary) => {
                    self.with_install(context, move |install| {
                        Message::Converted(
                            install
                                .convert_to_npc(&file)
                                .map(|new| (new, tr("Converted to an NPC")))
                                .map_err(|e| e.to_string()),
                        )
                    });
                }
                (Some(Confirm::Discard { section }), ContentDialogResult::Primary) => match section
                {
                    Some(id) => self.show_section(id),
                    None => self.back(),
                },
                (Some(Confirm::EmptySheet), ContentDialogResult::Primary) => {
                    self.save_options(context)
                }
                _ => {}
            },
            Message::Deleted(Ok(name)) => {
                self.select(None);
                self.notice = Some(Notice::success(tr!("“{name}” deleted", name)));
                self.load(None, context);
            }
            Message::Deleted(Err(e)) => self.show_error(tr("Delete Character"), e),
            Message::Converted(Ok((file, said))) => {
                self.notice = Some(Notice::success(said));
                self.load(Some(file), context);
            }
            Message::Converted(Err(e)) => self.show_error(&e, ""),
            Message::ShowSheet => {
                if let Some(file) = self.selected_character() {
                    self.with_install(context, move |install| {
                        Message::SheetLoaded(
                            cr_app::sheets::character(install, &file)
                                .map(|(sheet, choice)| {
                                    Arc::new(SheetPage::new(sheet, Some(file), choice))
                                })
                                .map_err(|e| e.to_string()),
                        )
                    });
                }
            }
            Message::ViewPage => {
                if let Some(key) = self.selected.clone() {
                    self.with_install(context, move |install| {
                        Message::SheetLoaded(
                            install
                                .page(&key)
                                .map(|sheet| Arc::new(SheetPage::new(sheet, None, None)))
                                .ok_or_else(|| tr("Could Not Read").to_owned()),
                        )
                    });
                }
            }
            Message::SheetLoaded(Ok(page)) => self.go(Page::Sheet(page)),
            Message::SheetLoaded(Err(e)) => self.show_error(&e, ""),
            Message::Printed(result) => {
                self.busy = false;
                match result {
                    Ok(true) => self.notice = Some(Notice::success(tr("Sent to the printer"))),
                    Ok(false) => {}
                    Err(e) => self.show_error(&tr!("Could not print: {error}", error = e), ""),
                }
            }
            Message::PdfSaved(result) => {
                self.busy = false;
                match result {
                    Ok(Some(_)) => self.notice = Some(Notice::success(tr("Sheet saved as PDF"))),
                    Ok(None) => {}
                    Err(e) => self.show_error(&e, ""),
                }
            }
            Message::Exported(result) => {
                self.busy = false;
                match result {
                    Ok(Some(_)) => self.notice = Some(Notice::success(tr("Sheet exported"))),
                    Ok(None) => {}
                    Err(e) => self.show_error(&e, ""),
                }
            }
            Message::SheetOptions => {
                if let Page::Sheet(page) = &self.page
                    && let (Some(file), Some(choice)) = (page.file.clone(), page.choice.clone())
                {
                    self.go(Page::SheetOptions(Box::new(SheetOptions::new(
                        file, choice,
                    ))));
                }
            }
            Message::Options(edit) => {
                if let Page::SheetOptions(options) = &mut self.page {
                    options.edit(edit);
                }
            }
            Message::SaveOptions => match &self.page {
                // An empty sheet is asked about first.
                Page::SheetOptions(options) if options.sections.is_empty() => {
                    self.confirm = Some(Confirm::EmptySheet)
                }
                _ => self.save_options(context),
            },
            Message::OptionsSaved(Ok(sheet)) => {
                let Page::SheetOptions(options) = &self.page else {
                    return;
                };
                let (file, choice) = (options.file.clone(), options.choice());
                self.history.pop();
                self.page = Page::Sheet(Arc::new(SheetPage::new(sheet, Some(file), Some(choice))));
                self.notice = None;
            }
            Message::OptionsSaved(Err(e)) => {
                if let Page::SheetOptions(options) = &mut self.page {
                    options.saving = false;
                }
                self.show_error(tr("Character Sheet Options"), e);
            }
            Message::Rules(Some(index)) => {
                if let Page::NewCharacter {
                    skills_and_powers, ..
                } = &mut self.page
                {
                    *skills_and_powers = index == 1;
                }
            }
            Message::Setting(Some(index)) => {
                if let Page::NewCharacter { setting, .. } = &mut self.page {
                    *setting = index;
                }
            }
            Message::Rules(None) | Message::Setting(None) => {}
            Message::Next => {
                self.notice = Some(Notice::new(
                    InfoBarSeverity::Informational,
                    tr("Not in this version yet"),
                    tr(
                        "The other steps of New Character come to the Windows app in a later version.",
                    ),
                ));
            }
            Message::PackageChosen(chosen) => {
                self.busy = false;
                match chosen {
                    Ok(Some(package)) => self.go(Page::Import(package)),
                    Ok(None) => {}
                    Err(e) => self.show_error(tr("Not a Package"), e),
                }
            }
            Message::Import => {
                if let (Page::Import(package), Some(install)) = (&self.page, self.install.clone())
                    && !self.busy
                {
                    self.busy = true;
                    let package = package.clone();
                    context.spawn_background(move |_| {
                        Message::Imported(
                            install
                                .import(&package.file, None)
                                .map(Arc::new)
                                .map_err(|e| e.to_string()),
                        )
                    });
                }
            }
            Message::Imported(result) => {
                self.busy = false;
                match result {
                    Ok(report) => {
                        self.history.clear();
                        self.page = Page::Library;
                        self.notice = Some(import::report(&report));
                        self.load(None, context);
                    }
                    Err(e) => self.show_error(tr("Import Failed"), e),
                }
            }
            Message::Language(Some(index)) => {
                let language = index
                    .checked_sub(1)
                    .and_then(|i| cr_i18n::Language::ALL.get(i));
                if language.copied() == crate::prefs::chosen_language() {
                    return;
                }
                self.notice = Some(match crate::prefs::save_language(language.copied()) {
                    Ok(()) => Notice::new(
                        InfoBarSeverity::Informational,
                        tr("Restart Core Rules to use the new language"),
                        "",
                    ),
                    Err(e) => Notice::new(
                        InfoBarSeverity::Error,
                        tr!("Could not save the preferences: {error}", error = e),
                        "",
                    ),
                });
            }
            Message::Language(None) => {}
        }
    }

    fn view(&self, _file: &Option<PathBuf>, context: &mut ViewContext<Self>) -> View {
        context.window_title("Core Rules");
        context.window_visuals(
            WindowVisuals::new()
                .backdrop(WindowBackdrop::Mica)
                .client_size(1100.0, 720.0),
        );

        let content = match &self.page {
            Page::Library => library::view(self, context),
            Page::Edit(edit) => edit::view(edit, context),
            Page::NewCharacter {
                skills_and_powers,
                setting,
            } => new_character::view(self, *skills_and_powers, *setting, context),
            Page::Import(package) => import::view(self, package, context),
            Page::Settings => settings::view(self, context),
            Page::Sheet(page) => sheet::view(self, page, context),
            Page::SheetOptions(options) => sheet::options_view(options, context),
        };
        let notice = match &self.notice {
            Some(notice) => InfoBar::new()
                .severity(notice.severity)
                .title(notice.title.clone())
                .message(notice.message.clone())
                .is_open(true)
                .is_closable(true)
                .on_closed(context.message(Message::CloseNotice))
                .margin(Thickness::new(36.0, 12.0, 36.0, 0.0))
                .into(),
            None => View::empty(),
        };
        let dialog = self.confirm_dialog(context);

        let selected_tag = match self.page {
            Page::Settings => SETTINGS,
            _ => self.section.as_str(),
        };
        let menu: Vec<KeyedView> = self
            .sections
            .iter()
            .flat_map(|sections| sections.iter())
            .map(|section| {
                KeyedView::new(
                    section.id,
                    NavigationViewItem::new()
                        .tag(section.id)
                        .is_selected(selected_tag == section.id)
                        .selects_on_invoked(true)
                        .slots([
                            SlotView::new(NavigationViewItemSlot::Content, section.title),
                            SlotView::new(
                                NavigationViewItemSlot::Icon,
                                SymbolIcon::new().symbol(library::symbol(section.id)),
                            ),
                        ]),
                )
            })
            .collect();
        let navigation = NavigationView::new()
            .pane_display_mode(NavigationViewPaneDisplayMode::Left)
            .is_pane_toggle_button_visible(false)
            .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
            .is_settings_visible(true)
            .always_show_header(false)
            .is_pane_open(self.pane_open)
            .open_pane_length(240.0)
            .on_is_pane_open_changed(context.callback(Message::PaneOpenChanged))
            .on_selected_tag_changed(context.callback(Message::Navigate))
            .grid_row(1)
            .slots([
                SlotView::collection(NavigationViewSlot::MenuItems, menu),
                SlotView::new(
                    NavigationViewSlot::Content,
                    Grid::new()
                        .rows([GridLength::Auto, GridLength::STAR])
                        .children((notice, Border::new().grid_row(1).content(content), dialog)),
                ),
            ]);
        let title_bar = TitleBar::new()
            .preferred_height(WindowTitleBarHeight::Standard)
            .title("Core Rules")
            .is_back_button_visible(true)
            .is_back_button_enabled(!self.history.is_empty())
            .is_pane_toggle_button_visible(true)
            .on_back_requested(context.message(Message::Back))
            .on_pane_toggle_requested(context.message(Message::TogglePane))
            .grid_row(0);
        let full_screen = if self.full_screen && matches!(self.page, Page::Library) {
            library::full_screen(self, context)
        } else {
            View::empty()
        };
        Grid::new()
            .rows([GridLength::Auto, GridLength::STAR])
            .children((title_bar, navigation, full_screen))
    }
}

impl Shell {
    /// Saves Character Sheet Options.
    fn save_options(&mut self, context: &ComponentContext<Self>) {
        let (Page::SheetOptions(options), Some(install)) = (&mut self.page, self.install.clone())
        else {
            return;
        };
        if options.saving {
            return;
        }
        options.saving = true;
        let (file, choice) = (options.file.clone(), options.choice());
        context.spawn_background(move |_| {
            Message::OptionsSaved(
                cr_app::sheets::choose(&install, &file, &choice).map_err(|e| e.to_string()),
            )
        });
    }

    /// The ContentDialog of the question being asked, closed when none is.
    fn confirm_dialog(&self, context: &mut ViewContext<Self>) -> View {
        let (title, text, action) = match &self.confirm {
            Some(Confirm::Delete { name, .. }) => (
                tr!("Delete {name}?", name),
                tr("The character leaves the character list and its file is removed.").to_owned(),
                plain(tr("_Delete")),
            ),
            Some(Confirm::Discard { .. }) => (
                plain(tr("Discard Changes?")),
                match &self.page {
                    Page::SheetOptions(_) => tr("The options have changes that are not saved."),
                    _ => tr("The character has changes that are not saved."),
                }
                .to_owned(),
                plain(tr("_Discard")),
            ),
            Some(Confirm::ToNpc { .. }) => (
                tr("Convert to NPC?").to_owned(),
                tr("This will move the selected Player Character to the NPC List in the DM Toolkit.")
                    .to_owned(),
                plain(tr("_Convert")),
            ),
            Some(Confirm::EmptySheet) => (
                tr("No Sections").to_owned(),
                tr("No sections: the sheet will be empty").to_owned(),
                plain(tr("_Save")),
            ),
            None => (String::new(), String::new(), String::new()),
        };
        ContentDialog::new()
            .title(title)
            .primary_button_text(action)
            .close_button_text(plain(tr("_Cancel")))
            .is_open(self.confirm.is_some())
            .on_closed(context.callback(Message::Answered))
            .content(crate::ui::body(text))
    }
}
