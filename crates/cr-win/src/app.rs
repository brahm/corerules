//! The window: a title bar over a NavigationView with the library's
//! sections and Settings, as Windows 11 apps are laid out. One component
//! holds the state and handles every message; the pages are drawn by the
//! functions of the modules below, from that state.

mod book;
mod create;
mod edit;
mod help;
mod import;
mod library;
mod settings;
mod sheet;
mod web;

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use cr_app::action::{self, Action};
use cr_app::create::{Answer, Effect, Input, ListKind, Needs, Wizard};
use cr_app::help::{Files, Found, MonsterHelp, PROGRAM, Target};
use cr_app::library::Filter;
use cr_app::sheets::Export;
use cr_formats::export::ExportFile;
use cr_i18n::{tr, trc};
use cr_store::Installation;
use cr_store::catalog::{Entry, Section};
use cr_store::export::{Content, ImportReport};
use cr_store::help::{ItemHelp, topic};
use cr_store::sheet::Sheet;
use cr_store::userdb::parts::OTHER_TYPES;
use cr_win::text::plain;
use windows_reactor::*;

use crate::native::keys::Key;
use crate::native::{self, Owner};
use book::{BookPage, BookView};
pub use edit::Edit;
use help::{HelpPage, Opened, Shown};
use library::Pictures;
use sheet::{OptionsEdit, SheetOptions, SheetPage};
use web::WebHost;

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

/// The navigation's tags for Settings and Help; the sections are tagged
/// by id.
const SETTINGS: &str = "Settings";
const HELP: &str = "Help";

/// The width of the navigation, open; closed, only its icons show.
const PANE: f64 = 240.0;
const PANE_CLOSED: f64 = 48.0;

pub struct Shell {
    install: Option<Arc<Installation>>,
    /// Whether the installation takes changes: an Expansion installation
    /// the user may write to (Core Rules 2.0 is opened read-only).
    can_write: bool,
    /// The catalog, once read, and how many times it was.
    sections: Option<Arc<Vec<Section>>>,
    loads: u64,
    /// The section shown, by id.
    section: String,
    filter: Filter,
    /// The choices of the party filter.
    parties: Vec<Option<String>>,
    /// The entry on the details, by key, and its pictures.
    selected: Option<String>,
    /// Which of the entries with that key, when the data repeats one.
    selected_copy: usize,
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
    /// The window's width, once Reactor has said it; 0 before.
    width: f64,
    /// A dialog or a print job is under way.
    busy: bool,
    /// The installation's help files and its records' help, read once.
    help_files: Option<Arc<Files>>,
    items: Arc<OnceLock<Arc<ItemHelp>>>,
    help_view: WebHost,
    /// The sections of About that are open.
    about_open: Vec<settings::Section>,
    book_view: BookView,
    /// Counts the times a page asked again for its WebView2's
    /// CoreWebView2, for it to ask once more.
    web_tries: u64,
    /// What the boxes of New Character's lists have chosen for Add.
    adding: create::Adding,
    /// A book is shown, whose keys the keyboard hook takes.
    reading: Rc<Cell<bool>>,
}

/// What the window shows.
#[derive(Clone)]
pub enum Page {
    Library,
    Edit(Box<Edit>),
    Create(Box<Wizard>),
    Import(Arc<Package>),
    Settings,
    Sheet(Arc<SheetPage>),
    SheetOptions(Box<SheetOptions>),
    Help(Box<HelpPage>),
    Book(Box<BookPage>),
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
    /// What a record says, or a popup of the program help: only read.
    Info {
        title: String,
        text: String,
    },
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
    WindowSized(WindowSize),
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
    /// A control's change that does nothing.
    Nothing,
    // New Character.
    WizardLoaded(Result<Box<Wizard>, String>),
    Wizard(Input),
    /// The answer to the prompt with this number.
    WizardAnswer(u64, Answer),
    /// What the box of a list has chosen for Add.
    WizardAdding(ListKind, String),
    // Import.
    PackageChosen(Result<Option<Arc<Package>>, String>),
    Import,
    Imported(Result<Arc<ImportReport>, String>),
    // Settings.
    Language(Option<usize>),
    CoreVsOption(bool),
    CopyDebugInfo,
    AboutSection(settings::Section, bool),
    // Help.
    HelpShown(Result<Box<Shown>, String>),
    HelpBook(Option<usize>),
    /// A row of the tree, by its title.
    HelpRow(String),
    HelpSearch(String),
    HelpRows(String, Arc<Vec<cr_app::help::Row>>),
    HelpLink(String),
    HelpOpened(Opened),
    HelpBack,
    HelpPrint,
    /// What a record's help or a monster's View and Print show, and
    /// whether to print it.
    ItemHelpFound(Result<ItemHelpShown, String>, bool),
    MonsterHelp(bool),
    // A book.
    /// The book asked for by this name was read.
    BookOpened(String, Result<Arc<book::Opened>, String>),
    /// What the book's page posted.
    BookPosted(String),
    BookHeading(Option<usize>),
    /// The contents laid over the text were closed.
    BookContentsClosed,
    BookFind(String),
    /// The match after (1) or before (-1) the one shown.
    BookMatch(i32),
    BookFindClosed,
    // The WebView2 of the help or a book.
    /// Its control was not there: the page asks for it again.
    WebRetry,
    /// Its CoreWebView2 could not be made, for this reason.
    WebFailed(String),
    /// Time to see whether it started.
    WebWatch,
}

/// What a record's help shows.
#[derive(Clone)]
pub enum ItemHelpShown {
    /// The help at a topic.
    Open(String),
    Info {
        title: String,
        text: String,
    },
    Page(Sheet),
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
        let mut copies = self
            .current_section()?
            .entries
            .iter()
            .filter(|e| e.key == key);
        let first = copies.next();
        match self.selected_copy {
            0 => first,
            n => copies.nth(n - 1).or(first),
        }
    }

    fn select(&mut self, key: Option<String>) {
        self.select_copy(key, 0);
    }

    /// Selects the `copy`th entry with `key` (the standard data repeats
    /// some records).
    fn select_copy(&mut self, key: Option<String>, copy: usize) {
        self.selected = key;
        self.selected_copy = copy;
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
            // Past its first page the wizard has the choices made.
            Page::Create(wizard) => wizard.depth() > 1,
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
            action::NEW_CHARACTER => self.new_character(context),
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
                if let Some((printable, _)) = self.printable() {
                    self.print(printable, context);
                }
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
            action::HELP => self.open_help(PROGRAM.to_owned(), false, context),
            // The original's Help > Main Menu and F1: the program help of
            // the page shown.
            action::HELP_HERE => {
                if !matches!(self.page, Page::Help(_)) {
                    self.open_help(cr_app::help::program(self.here()), false, context);
                }
            }
            action::HELP_CONTROL => self.item_help(context),
            // Read Book, on the book on the details.
            action::OPEN_BOOK => {
                if let Some(name) = self
                    .selected_entry()
                    .and_then(|e| cr_app::library::book_name(&e.key))
                {
                    self.open_book(name.to_owned(), None, context);
                }
            }
            action::BOOK_FIND => {
                if let Page::Book(page) = &mut self.page
                    && page.book.is_some()
                {
                    page.finding = true;
                    page.find_asked += 1;
                }
            }
            action::BOOK_ZOOM_IN => self.book_zoom(1),
            action::BOOK_ZOOM_OUT => self.book_zoom(-1),
            action::BOOK_ZOOM_RESET => self.book_zoom(0),
            action::BOOK_CONTENTS => {
                let narrow = book::narrow(self);
                if let Page::Book(page) = &mut self.page {
                    // Over the text in a narrow window, beside it otherwise.
                    if narrow {
                        page.overlay = !page.overlay;
                    } else {
                        page.contents = !page.contents;
                    }
                }
            }
            action::BOOK_CLOSE => {
                if matches!(self.page, Page::Book(_)) {
                    self.back();
                }
            }
            _ => {}
        }
    }

    /// Opens the book `name` on the book page, or goes to it if shown;
    /// with `find`, Find starts with it.
    fn open_book(&mut self, name: String, find: Option<String>, context: &ComponentContext<Self>) {
        let Some(install) = self.install.clone() else {
            return;
        };
        if let Page::Book(page) = &mut self.page
            && page.name.eq_ignore_ascii_case(&name)
        {
            if let Some(find) = find {
                page.finding = true;
                page.find_asked += 1;
                page.skip_first = true;
                let scripts = page.search(find);
                self.run_in_book(&scripts);
            }
            return;
        }
        self.go(Page::Book(Box::new(BookPage::new(name.clone(), find))));
        context.spawn_background(move |_| {
            let opened = book::open(&install, &name).map(Arc::new);
            Message::BookOpened(name, opened)
        });
    }

    /// Sees, once WebView2 has had time to start, whether the page's
    /// did.
    fn watch_web(context: &ComponentContext<Self>) {
        context.spawn_background(|_| {
            std::thread::sleep(web::WAIT);
            Message::WebWatch
        });
    }

    /// Starts New Character once its rules are read.
    fn new_character(&mut self, context: &ComponentContext<Self>) {
        let Some(install) = self.install.clone() else {
            return;
        };
        if self.busy {
            return;
        }
        self.busy = true;
        let core_vs_option = crate::prefs::core_vs_option_shown();
        context.spawn_background(move |_| {
            Message::WizardLoaded(
                Needs::load(&install, false, None, core_vs_option)
                    .map(|(rules, needs)| Box::new(Wizard::new(rules, needs)))
                    .map_err(|e| e.to_string()),
            )
        });
    }

    /// Does what the wizard asks of the app.
    fn wizard_effects(&mut self) {
        let Page::Create(wizard) = &mut self.page else {
            return;
        };
        for effect in wizard.take_effects() {
            match effect {
                Effect::HideCoreVsOption => {
                    if let Err(e) = crate::prefs::set_core_vs_option_shown(false) {
                        self.show_error(
                            &tr!("Could not save the preferences: {error}", error = e),
                            "",
                        );
                    }
                }
            }
        }
    }

    /// Zooms the book shown in, out (`direction` -1) or back to 1 (0).
    fn book_zoom(&mut self, direction: i32) {
        if let Page::Book(page) = &mut self.page
            && let Some(script) = page.zoom(direction)
        {
            self.run_in_book(&[script]);
        }
    }

    /// Runs `scripts` in the book's page, once it has loaded.
    fn run_in_book(&self, scripts: &[String]) {
        if let Page::Book(page) = &self.page {
            for script in scripts {
                self.book_view.run(page, script);
            }
        }
    }

    /// Prints a sheet through the Windows print dialog.
    fn print(&mut self, printable: Printable, context: &ComponentContext<Self>) {
        let Some(install) = self.install.clone() else {
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

    /// The topic of the program help for the page shown, as the
    /// original's screens open it.
    fn here(&self) -> u32 {
        match &self.page {
            Page::Library => topic::of_section(&self.section),
            Page::Edit(_) => topic::CHARACTER_INFORMATION,
            Page::Create(wizard) => wizard.help(),
            Page::Import(_) => topic::IMPORT,
            Page::Sheet(_) | Page::SheetOptions(_) => topic::CHARACTER_SHEET,
            Page::Settings | Page::Help(_) => topic::MAIN_MENU,
            Page::Book(_) => topic::BOOKS,
        }
    }

    /// The installation's help files, read as they are opened.
    fn help_files(&mut self) -> Option<Arc<Files>> {
        if self.help_files.is_none() {
            let root = self.install.as_ref()?.root().to_owned();
            self.help_files = Installation::open(&root)
                .ok()
                .map(|i| Arc::new(Files::new(i)));
        }
        self.help_files.clone()
    }

    /// The records' help, read once.
    fn items(&self) -> impl FnOnce() -> Arc<ItemHelp> + Send + 'static {
        let (items, install) = (self.items.clone(), self.install.clone());
        move || {
            items
                .get_or_init(|| Arc::new(install.map(|i| i.item_help()).unwrap_or_default()))
                .clone()
        }
    }

    /// Opens the help at `spec` (`Program.hlp#1234`) on the help page;
    /// `print` prints the topic once it shows.
    fn open_help(&mut self, spec: String, print: bool, context: &ComponentContext<Self>) {
        let Some(files) = self.help_files() else {
            self.show_error(tr("Open a Core Rules folder to read its help"), "");
            return;
        };
        if !matches!(self.page, Page::Help(_)) {
            self.go(Page::Help(Box::default()));
        }
        let Page::Help(page) = &mut self.page else {
            return;
        };
        page.print = print;
        let (listed, rows) = (page.listed.clone(), page.rows.clone());
        context.spawn_background(move |_| {
            let (file, target) = Target::parse(&spec);
            Message::HelpShown(help::open(&files, file, &target, &listed, &rows))
        });
    }

    /// Shows topic `place` of the help page in the background.
    fn show_topic(
        &mut self,
        place: Option<cr_app::help::Place>,
        remember: bool,
        context: &ComponentContext<Self>,
    ) {
        let Some(files) = self.help_files() else {
            return;
        };
        let Page::Help(page) = &self.page else {
            return;
        };
        let (listed, rows) = (page.listed.clone(), page.rows.clone());
        context.spawn_background(move |_| {
            let shown = place
                .ok_or_else(|| tr("That topic is not in this installation").to_owned())
                .and_then(|place| help::show(&files, place, &listed, &rows, remember));
            Message::HelpShown(shown)
        });
    }

    /// The help of the entry on the details, as a right-click on its row
    /// in the original's list opens it.
    fn item_help(&mut self, context: &ComponentContext<Self>) {
        let (Some(entry), Some(install)) = (self.selected_entry(), self.install.clone()) else {
            return;
        };
        let screen = topic::of_section(&self.section);
        let texts = [entry.title.trim().to_owned(), entry.subtitle.clone()];
        let name = entry.title.trim().to_owned();
        let (items, files) = (self.items(), self.help_files());
        context.spawn_background(move |_| {
            let controls = install.screen_help(screen);
            let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
            let shown = match cr_app::help::context_help(&install, screen, &texts, &controls, items)
            {
                Some(Found::Topic(found)) if found.popup => files
                    .as_ref()
                    .and_then(|files| popup_text(files, found.file, found.number))
                    .map(|(title, text)| ItemHelpShown::Info { title, text })
                    .ok_or_else(|| tr!("{name} has no help topic", name = name)),
                Some(Found::Topic(found)) => Ok(ItemHelpShown::Open(format!(
                    "{}#{}",
                    found.file, found.number
                ))),
                Some(Found::Custom { title, text } | Found::Text { title, text }) => {
                    Ok(ItemHelpShown::Info { title, text })
                }
                Some(Found::Page(sheet)) => Ok(ItemHelpShown::Page(sheet)),
                None => Err(tr!("{name} has no help topic", name = name)),
            };
            Message::ItemHelpFound(shown, false)
        });
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

/// The title and text of topic `number` of `file`, as a popup shows it.
fn popup_text(files: &Files, file: &str, number: u32) -> Option<(String, String)> {
    let help = files.file(file)?;
    let topic = help.topics.get(help.topic_of_number(number)?)?;
    let text = topic
        .paragraphs
        .iter()
        .map(|p| p.text())
        .collect::<Vec<_>>()
        .join("\n");
    Some((cr_app::help::title(&help, topic), text.trim().to_owned()))
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
            loads: 0,
            section: cr_app::library::CHARACTERS.to_owned(),
            filter: Filter::default(),
            parties: vec![None],
            selected: None,
            selected_copy: 0,
            pictures: Pictures::default(),
            full_screen: false,
            background: None,
            page: Page::Library,
            history: Vec::new(),
            notice: None,
            confirm: None,
            pane_open: true,
            width: 0.0,
            busy: false,
            help_files: None,
            items: Arc::default(),
            help_view: WebHost::default(),
            about_open: Vec::new(),
            book_view: BookView::default(),
            web_tries: 0,
            adding: Vec::new(),
            reading: Rc::default(),
        };
        let sender = context.sender();
        let reading = shell.reading.clone();
        native::keys::on_key(move |key| {
            let action = match key {
                Key::Help => action::HELP_HERE,
                _ if !reading.get() => return false,
                Key::Contents => action::BOOK_CONTENTS,
                Key::Find => action::BOOK_FIND,
                Key::ZoomIn => action::BOOK_ZOOM_IN,
                Key::ZoomOut => action::BOOK_ZOOM_OUT,
                Key::ZoomReset => action::BOOK_ZOOM_RESET,
                Key::Close => action::BOOK_CLOSE,
            };
            sender.send(Message::Act(action));
            true
        });
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
                self.loads += 1;
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
                } else if tag == HELP {
                    if !matches!(self.page, Page::Help(_)) {
                        self.act(action::HELP_HERE, context);
                    }
                } else if self.changed() {
                    self.confirm = Some(Confirm::Discard { section: Some(tag) });
                } else if tag != self.section || !matches!(self.page, Page::Library) {
                    self.show_section(tag);
                }
            }
            Message::Navigate(None) => {}
            Message::Back => {
                if let Page::Create(wizard) = &mut self.page
                    && wizard.back()
                {
                    return;
                }
                if self.changed() {
                    self.confirm = Some(Confirm::Discard { section: None });
                } else {
                    self.back();
                }
            }
            Message::TogglePane => self.pane_open = !self.pane_open,
            Message::PaneOpenChanged(open) => self.pane_open = open,
            Message::WindowSized(size) => self.width = size.width,
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
                let chosen = index.and_then(|i| {
                    let entry = *self.visible().get(i)?;
                    let copy = self
                        .current_section()?
                        .entries
                        .iter()
                        .take_while(|e| !std::ptr::eq(*e, entry))
                        .filter(|e| e.key == entry.key)
                        .count();
                    Some((entry.key.clone(), copy))
                });
                if let Some((key, copy)) = chosen {
                    self.select_copy(Some(key), copy);
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
                            self.help_files = None;
                            self.items = Arc::default();
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
            Message::Nothing => {}
            Message::WizardLoaded(loaded) => {
                self.busy = false;
                match loaded {
                    Ok(wizard) => {
                        self.adding.clear();
                        self.go(Page::Create(wizard));
                    }
                    Err(e) => self.show_error(tr("New Character"), e),
                }
            }
            Message::Wizard(input) => {
                if let Page::Create(wizard) = &mut self.page {
                    // Added, the choice leaves the box.
                    if let Input::Add(kind, _) = &input {
                        self.adding.retain(|(k, _)| k != kind);
                    }
                    wizard.input(input);
                    self.wizard_effects();
                }
            }
            Message::WizardAdding(kind, name) => {
                self.adding.retain(|(k, _)| *k != kind);
                self.adding.push((kind, name));
            }
            Message::WizardAnswer(serial, answer) => {
                if let Page::Create(wizard) = &mut self.page
                    && wizard.prompt_serial() == serial
                {
                    wizard.answer(answer);
                    self.wizard_effects();
                }
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
            Message::CoreVsOption(shown) => {
                if let Err(e) = crate::prefs::set_core_vs_option_shown(shown) {
                    self.show_error(
                        &tr!("Could not save the preferences: {error}", error = e),
                        "",
                    );
                }
            }
            Message::AboutSection(section, open) => {
                self.about_open.retain(|s| *s != section);
                if open {
                    self.about_open.push(section);
                }
            }
            Message::CopyDebugInfo => match native::copy(&settings::debug_info(self)) {
                Ok(()) => self.notice = Some(Notice::success(tr("Copied to the clipboard"))),
                Err(e) => self.show_error(&e.message(), ""),
            },
            Message::HelpShown(Ok(shown)) | Message::HelpOpened(Opened::Shown(shown)) => {
                let Page::Help(page) = &mut self.page else {
                    return;
                };
                if shown.remember
                    && let Some(current) = &page.shown
                    && current.place != shown.place
                {
                    page.history.push(current.place.clone());
                }
                if let Some(rows) = shown.rows.clone() {
                    page.rows = rows;
                    page.listed = shown.place.file.clone();
                    page.query.clear();
                }
                // A search's rows are not the ones it was found in.
                let shown_row = if page.query.is_empty() {
                    shown.row
                } else {
                    None
                };
                page.shown = Some(Arc::from(shown));
                page.generation += 1;
                if self.help_view.web().is_none() {
                    Self::watch_web(context);
                }
                page.printing = std::mem::take(&mut page.print);
                page.row = shown_row;
                page.popup = None;
            }
            Message::HelpShown(Err(e)) => {
                if let Page::Help(page) = &mut self.page {
                    page.print = false;
                }
                self.show_error(&e, "");
            }
            Message::HelpBook(Some(index)) => {
                if let (Some(file), Page::Help(page)) = (help::book(index), &self.page)
                    && !page.listed.eq_ignore_ascii_case(file)
                {
                    self.open_help(file.to_owned(), false, context);
                }
            }
            Message::HelpBook(None) => {}
            Message::HelpRow(title) => {
                let (Some(files), Page::Help(page)) = (self.help_files(), &self.page) else {
                    return;
                };
                // The tree says which row by its title; a heading only
                // opens and closes its branch.
                if let Some(row) = page.rows.iter().find(|r| r.title == title && r.is_topic()) {
                    let place = files.row_place(row);
                    self.show_topic(place, true, context);
                }
            }
            Message::HelpSearch(query) => {
                let (Some(files), Page::Help(page)) = (self.help_files(), &mut self.page) else {
                    return;
                };
                if page.query == query {
                    return;
                }
                page.query = query.clone();
                let listed = page.listed.clone();
                context.spawn_background(move |_| {
                    let rows = Arc::new(files.rows(&listed, &query));
                    Message::HelpRows(query, rows)
                });
            }
            Message::HelpRows(query, rows) => {
                if let Page::Help(page) = &mut self.page
                    && page.query == query
                {
                    page.rows = rows;
                    page.row = None;
                }
            }
            // The page asks to print once it has loaded.
            Message::HelpLink(id) if id == help::PRINT => {
                if let Some(web) = self.help_view.web() {
                    web.print();
                }
            }
            Message::HelpLink(id) => {
                let (Some(files), Page::Help(page)) = (self.help_files(), &self.page) else {
                    return;
                };
                let (from, links, index) = match id.strip_prefix(cr_win::html::POPUP) {
                    Some(index) => match &page.popup {
                        Some((file, links)) => (file.clone(), links, index),
                        None => return,
                    },
                    None => match &page.shown {
                        Some(shown) => (shown.place.file.clone(), &shown.page.links, id.as_str()),
                        None => return,
                    },
                };
                let Some(link) = index
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| links.get(i))
                    .cloned()
                else {
                    return;
                };
                let (listed, rows) = (page.listed.clone(), page.rows.clone());
                context.spawn_background(move |_| {
                    Message::HelpOpened(help::follow(&files, &from, &link, &listed, &rows))
                });
            }
            Message::HelpOpened(Opened::Popup(file, popup)) => {
                if let Page::Help(page) = &mut self.page
                    && let Some(web) = self.help_view.web()
                {
                    web.run(&format!(
                        "showPopup({})",
                        cr_win::html::js_string(&popup.html)
                    ));
                    page.popup = Some((file, popup.links));
                }
            }
            Message::HelpOpened(Opened::Web(uri)) => native::open_uri(&uri),
            Message::HelpOpened(Opened::Missing) => {
                self.show_error(tr("That topic is not in this installation"), "")
            }
            Message::HelpBack => {
                if let Page::Help(page) = &mut self.page
                    && let Some(place) = page.history.pop()
                {
                    self.show_topic(Some(place), false, context);
                }
            }
            Message::HelpPrint => {
                if let Some(web) = self.help_view.web() {
                    web.print();
                }
            }
            Message::MonsterHelp(print) => {
                let (Some(entry), Some(install)) = (self.selected_entry(), self.install.clone())
                else {
                    return;
                };
                let Some((name, custom)) = cr_app::library::monster_key(&entry.key) else {
                    return;
                };
                let name = name.to_owned();
                let items = self.items();
                context.spawn_background(move |_| {
                    let shown = match cr_app::help::monster(&install, &name, custom, items) {
                        MonsterHelp::Page(sheet) => Ok(ItemHelpShown::Page(sheet)),
                        MonsterHelp::Topic(found) => Ok(ItemHelpShown::Open(format!(
                            "{}#{}",
                            found.file, found.number
                        ))),
                        MonsterHelp::None => Err(tr!("{name} has no help topic", name = name)),
                    };
                    Message::ItemHelpFound(shown, print)
                });
            }
            Message::ItemHelpFound(Ok(ItemHelpShown::Open(spec)), print) => {
                self.open_help(spec, print, context)
            }
            Message::ItemHelpFound(Ok(ItemHelpShown::Info { title, text }), _) => {
                self.confirm = Some(Confirm::Info { title, text })
            }
            Message::ItemHelpFound(Ok(ItemHelpShown::Page(sheet)), true) => {
                self.print(Printable::Sheet(Arc::new(sheet)), context)
            }
            Message::ItemHelpFound(Ok(ItemHelpShown::Page(sheet)), false) => {
                self.go(Page::Sheet(Arc::new(SheetPage::new(sheet, None, None))))
            }
            Message::ItemHelpFound(Err(e), _) => {
                self.notice = Some(Notice::new(InfoBarSeverity::Informational, e, ""))
            }
            Message::BookOpened(name, opened) => {
                let Page::Book(page) = &mut self.page else {
                    return;
                };
                if page.name != name {
                    return;
                }
                match opened {
                    Ok(book) => {
                        page.opened(book);
                        Self::watch_web(context);
                    }
                    Err(e) => page.error = Some(e),
                }
            }
            Message::WebRetry => self.web_tries += 1,
            // The page's WebView2 should have started by now.
            Message::WebWatch => {
                let waiting = match &self.page {
                    Page::Book(page) => page.book.is_some() && self.book_view.host.web().is_none(),
                    Page::Help(page) => page.shown.is_some() && self.help_view.web().is_none(),
                    _ => false,
                };
                if waiting && self.notice.is_none() {
                    self.show_error(
                        tr("The Page Could Not Be Shown"),
                        tr!("WebView2 did not start ({reason}).", reason = "no answer"),
                    );
                }
            }
            Message::WebFailed(reason) => self.show_error(
                tr("The Page Could Not Be Shown"),
                tr!("WebView2 did not start ({reason}).", reason),
            ),
            Message::BookPosted(message) => {
                let Page::Book(page) = &mut self.page else {
                    return;
                };
                if message == cr_win::book::READY {
                    // The page shows anew: where the reader was, as zoomed,
                    // with what Find found.
                    page.ready = true;
                    let mut scripts = vec![cr_win::book::start(page.zoom, page.top)];
                    if !page.query.is_empty() {
                        let query = page.query.clone();
                        scripts.extend(page.search(query));
                    }
                    self.run_in_book(&scripts);
                } else if let Some(top) = message.strip_prefix(cr_win::book::TOP) {
                    if let Ok(top) = top.parse() {
                        page.moved(top);
                    }
                } else if let Some(key) = message.strip_prefix(cr_win::book::KEY) {
                    let action = match key {
                        "find" => action::BOOK_FIND,
                        "zoom-in" => action::BOOK_ZOOM_IN,
                        "zoom-out" => action::BOOK_ZOOM_OUT,
                        "zoom-reset" => action::BOOK_ZOOM_RESET,
                        "contents" => action::BOOK_CONTENTS,
                        "close" => action::BOOK_CLOSE,
                        "help" => action::HELP_HERE,
                        _ => return,
                    };
                    self.act(action, context);
                }
            }
            Message::BookHeading(Some(index)) => {
                let Page::Book(page) = &mut self.page else {
                    return;
                };
                // The contents follow the text, which selects them too.
                if page.heading == Some(index) {
                    return;
                }
                let Some(paragraph) = page
                    .book
                    .as_ref()
                    .and_then(|b| b.headings.get(index))
                    .map(|h| h.paragraph)
                else {
                    return;
                };
                page.heading = Some(index);
                // Contents laid over the text give way to it.
                page.overlay = false;
                self.run_in_book(&[cr_win::book::go(paragraph)]);
            }
            Message::BookHeading(None) => {}
            Message::BookContentsClosed => {
                if let Page::Book(page) = &mut self.page {
                    page.overlay = false;
                }
            }
            Message::BookFind(query) => {
                if let Page::Book(page) = &mut self.page
                    && page.query != query
                {
                    let scripts = page.search(query);
                    self.run_in_book(&scripts);
                }
            }
            Message::BookMatch(direction) => {
                let Page::Book(page) = &mut self.page else {
                    return;
                };
                let script = match page.step_match(direction) {
                    Some(script) => vec![script],
                    // Nothing found yet: Enter searches again.
                    None if !page.query.is_empty() => {
                        let query = page.query.clone();
                        page.search(query)
                    }
                    None => return,
                };
                self.run_in_book(&script);
            }
            Message::BookFindClosed => {
                if let Page::Book(page) = &mut self.page {
                    page.finding = false;
                    page.query.clear();
                    page.matches = Arc::default();
                    page.current = None;
                }
                self.run_in_book(&[cr_win::book::UNMARK.to_owned()]);
            }
        }
    }

    fn view(&self, _file: &Option<PathBuf>, context: &mut ViewContext<Self>) -> View {
        context.window_title("Core Rules");
        let sized = context.callback(Message::WindowSized);
        context.on_window_size(sized);
        context.window_visuals(
            WindowVisuals::new()
                .backdrop(WindowBackdrop::Mica)
                .client_size(1100.0, 720.0),
        );

        let content = match &self.page {
            Page::Library => library::view(self, context),
            Page::Edit(edit) => edit::view(edit, context),
            Page::Create(wizard) => create::view(wizard, &self.adding, context),
            Page::Import(package) => import::view(self, package, context),
            Page::Settings => settings::view(self, context),
            Page::Sheet(page) => sheet::view(self, page, context),
            Page::SheetOptions(options) => sheet::options_view(options, context),
            Page::Help(page) => help::view(self, page, context),
            Page::Book(page) => book::view(self, page, context),
        };
        // The keyboard hook takes the reader's keys only while a book
        // shows.
        self.reading.set(matches!(self.page, Page::Book(_)));
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
            Page::Help(_) => HELP,
            _ => self.section.as_str(),
        };
        let mut menu: Vec<KeyedView> = self
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
                            SlotView::new(NavigationViewItemSlot::Icon, library::icon(section.id)),
                        ]),
                )
            })
            .collect();
        menu.push(KeyedView::new(
            HELP,
            NavigationViewItem::new()
                .tag(HELP)
                .is_selected(selected_tag == HELP)
                .selects_on_invoked(true)
                .slots([
                    SlotView::new(NavigationViewItemSlot::Content, plain(tr("_Help"))),
                    SlotView::new(
                        NavigationViewItemSlot::Icon,
                        SymbolIcon::new().symbol(Symbol::Help),
                    ),
                ]),
        ));
        // Not NavigationView's own Settings item, which is named in the
        // language of Windows rather than the app's.
        menu.push(KeyedView::new(
            SETTINGS,
            NavigationViewItem::new()
                .tag(SETTINGS)
                .is_selected(selected_tag == SETTINGS)
                .selects_on_invoked(true)
                .slots([
                    SlotView::new(
                        NavigationViewItemSlot::Content,
                        trc("preferences", "Settings"),
                    ),
                    SlotView::new(
                        NavigationViewItemSlot::Icon,
                        SymbolIcon::new().symbol(Symbol::Setting),
                    ),
                ]),
        ));
        let navigation = NavigationView::new()
            .pane_display_mode(NavigationViewPaneDisplayMode::Left)
            .is_pane_toggle_button_visible(false)
            .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
            .is_settings_visible(false)
            .always_show_header(false)
            .is_pane_open(self.pane_open)
            .open_pane_length(PANE)
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
            Some(Confirm::Info { title, text }) => (title.clone(), text.clone(), String::new()),
            None => (String::new(), String::new(), String::new()),
        };
        // Only read: the dialog only closes.
        let close = if matches!(self.confirm, Some(Confirm::Info { .. })) {
            plain(tr("_Close"))
        } else {
            plain(tr("_Cancel"))
        };
        ContentDialog::new()
            .title(title)
            .primary_button_text(action)
            .close_button_text(close)
            .is_open(self.confirm.is_some())
            .on_closed(context.callback(Message::Answered))
            .content(
                ScrollViewer::new()
                    .max_height(480.0)
                    .content(crate::ui::body(text).is_text_selection_enabled(true)),
            )
    }
}
