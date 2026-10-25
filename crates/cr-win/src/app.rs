//! The window: a title bar over a NavigationView with the library's
//! sections and Settings, as Windows 11 apps are laid out. One component
//! holds the state and handles every message; the pages are drawn by the
//! functions of the modules below, from that state.

mod book;
mod bundle;
mod create;
mod edit;
mod encounter;
mod forms;
mod help;
mod import;
mod levels;
mod library;
mod npc;
mod parties;
mod portrait;
mod settings;
mod sheet;
mod tables;
mod treasure;
mod web;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};

use cr_app::action::{self, Action};
use cr_app::create::{Answer, Effect, Input, ListKind, Needs, Page as CreatePage, Wizard};
use cr_app::editor::{EditEffect, EditInput, EditNeeds, Editor, Tab as EditTab};
use cr_app::encounter::{
    EncounterEffect, EncounterForm, EncounterInput, TableEditInput, TableEditor, TableEffect,
};
use cr_app::help::{Files, Found, MonsterHelp, PROGRAM, Target};
use cr_app::library::Filter;
use cr_app::npc::{GroupForm, GroupInput, NpcForm, NpcInput};
use cr_app::parties::PartyLists;
use cr_app::sheets::Export;
use cr_app::tables::{TableInput, TableRoller};
use cr_app::treasure::{Roller, TreasureEffect, TreasureInput};
use cr_formats::character::Character;
use cr_formats::export::ExportFile;
use cr_i18n::{tr, trc, trn};
use cr_store::Installation;
use cr_store::catalog::{ENCOUNTER_KEY, Entry, Section, TABLE_KEY, TREASURE_KEY};
use cr_store::encounter::{Encounter, EncounterRules, check_name};
use cr_store::export::{Content, ImportReport};
use cr_store::help::{ItemHelp, topic};
use cr_store::npc::{Npc, NpcRules};
use cr_store::pictures::stored_portrait;
use cr_store::sheet::Sheet;
use cr_store::tables::{Leftovers, TableBook, TableRolls};
use cr_store::treasure::{Bundle, Rand, TreasureRules};
use cr_store::userdb::UserDb;
use cr_store::userdb::parts::OTHER_TYPES;
use cr_store::userdb::traps::Traps;
use cr_win::text::plain;
use windows_reactor::*;

use crate::native::keys::Key;
use crate::native::{self, Owner};
use book::{BookPage, BookView};
use encounter::{Chooser, GroupDialog};
use help::{HelpPage, Opened, Shown};
use library::Pictures;
use parties::PartiesPage;
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
    /// The icons of the items Edit Character's bundles have held.
    icons: forms::Icons,
    /// Choose Portrait, while it is open.
    portraits: Option<portrait::Chooser>,
    /// The portrait the details page shows, read when it changes.
    portrait: RefCell<Option<Rc<portrait::Shown>>>,
    /// A book is shown, whose keys the keyboard hook takes.
    reading: Rc<Cell<bool>>,
    /// The dice of the generators, one sequence for the session as in
    /// the GNOME app.
    rand: Arc<Mutex<Rand>>,
    /// The NPC generator of the installation, read once.
    npc_rules: Option<Arc<NpcRules>>,
    /// The treasure tables of the installation, read once.
    treasure_rules: Option<Arc<TreasureRules>>,
    /// The tables Roll on a Table rolls on, read once, the book and table
    /// chosen last, and what its closed sessions left for the next roll
    /// (the original's stack).
    table_rolls: Option<Arc<TableRolls>>,
    table_last: Option<(TableBook, usize)>,
    leftovers: Leftovers,
    /// The encounter templates, monsters and treasure, read once.
    encounter_rules: Option<Arc<EncounterRules>>,
    /// Group Encounter, while it asks.
    group: Option<GroupDialog>,
}

/// What the window shows.
#[derive(Clone)]
pub enum Page {
    Library,
    Edit(Box<Editor>),
    Create(Box<Wizard>),
    Import(Arc<Package>),
    Settings,
    Sheet(Arc<SheetPage>),
    SheetOptions(Box<SheetOptions>),
    Help(Box<HelpPage>),
    Book(Box<BookPage>),
    Npc(Box<NpcForm>),
    NpcGroup(Box<GroupForm>),
    Treasure(Box<Roller>),
    Table(Box<TableRoller>),
    Parties(Box<PartiesPage>),
    NewEncounter(Box<Chooser>),
    Encounter(Box<EncounterForm>),
    EncounterTable(Box<TableEditor>),
}

/// What an encounter page opens on.
enum Opening {
    /// New Encounter's templates.
    Choose,
    /// An encounter made from a template, saying `said` first.
    Template {
        name: String,
        said: Option<String>,
    },
    /// Roll and Generate on a saved 2-20 table.
    RollTable(String),
    /// A saved encounter; with `copy`, under no name.
    Saved {
        name: String,
        copy: bool,
    },
    /// The 2-20 table editor, on a saved table or a new one.
    Table(Option<String>),
    Group(GroupDialog),
}

/// What an encounter page was opened on.
#[derive(Clone)]
pub enum EncounterOpened {
    Chooser(Box<Chooser>),
    /// The Encounter page, with what generating it said.
    Form(Box<EncounterForm>, Vec<String>),
    Table(Box<TableEditor>),
    /// Group Encounter's sheet, to preview or to print.
    Sheet(Arc<Sheet>, bool),
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
    DeleteTreasure {
        name: String,
    },
    /// Delete Members… on the party chosen in Manage Parties.
    DeleteParty {
        party: String,
    },
    DeleteEncounter {
        name: String,
    },
    DeleteTable {
        name: String,
    },
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
    EditLoaded(Result<Box<Editor>, String>),
    Editor(EditInput),
    /// The answer to the editor's prompt with this number.
    EditorAnswer(u64, Answer),
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
    /// The character New Character wrote, by its file, or why it could
    /// not; whether it was entered.
    Created(Result<String, String>, bool),
    Wizard(Input),
    /// The answer to the prompt with this number.
    WizardAnswer(u64, Answer),
    /// What the box of a list has chosen for Add.
    Adding(ListKind, String),
    /// Choose… on the Portrait row.
    ChoosePortrait,
    PortraitsListed(Result<Vec<portrait::Tile>, String>),
    /// A portrait chosen by its file, or none.
    PortraitChosen(Option<String>),
    /// Choose File… on Choose Portrait.
    PortraitFile,
    /// The portrait made from the user's picture, if one was chosen.
    PortraitAdded(Result<Option<String>, String>),
    ClosePortraits,
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
    // NPC Generation.
    /// The NPC generator was read, for New NPC or (true) New NPC Group.
    NpcRulesLoaded(Result<Arc<NpcRules>, String>, bool),
    Npc(NpcInput),
    Group(GroupInput),
    /// Create on New NPC or New NPC Group.
    CreateNpcs,
    NpcsSaved(Result<NpcsSaved, String>),
    // Roll Treasure.
    TreasureLoaded(Result<Box<Roller>, String>),
    Treasure(TreasureInput),
    /// Use Treasure, on an encounter's treasure.
    UseTreasure,
    /// The treasure was saved under this name.
    TreasureSaved(Result<String, String>),
    /// The saved treasure with this name was deleted.
    TreasureDeleted(Result<String, String>),
    /// Edit on the details of a saved treasure.
    EditEntry,
    /// Copy on the details of a saved treasure.
    CopyEntry,
    // Roll on a Table.
    TableRollsLoaded(Result<Arc<TableRolls>, String>),
    Table(TableInput),
    CopyResults,
    /// Read in Book: the table rolled on, in its book.
    ReadTable,
    /// Open Treasure Generator, which a roll on DMG Table 84 opens.
    TableTreasure,
    // Manage Parties.
    PartiesRead(Result<PartyLists, String>),
    PartyChosen(String),
    PartyName(String),
    NewParty,
    /// The lists with the party made, or why its name was refused.
    NewPartyMade(Result<PartyLists, String>),
    /// The character in this file goes to that party (none when empty).
    MoveToParty(String, String),
    /// The character moved: its file before and after, and the lists.
    PartyMoved(Result<(String, String, PartyLists), String>),
    AskDeleteParty,
    /// How many characters of the party were deleted.
    PartyDeleted(Result<(usize, String), String>),
    // Encounters.
    EncounterOpened(Result<(Arc<EncounterRules>, EncounterOpened), String>),
    TemplateSearch(String),
    /// A template chosen on New Encounter; empty for the empty encounter.
    ChooseTemplate(String),
    CreateEncounter,
    Encounter(EncounterInput),
    EncounterSaved(Result<String, String>),
    EncounterDeleted(Result<(), String>),
    AddNpc,
    /// The saved characters, by name and file, for Add NPC.
    NpcsListed(Result<Vec<(String, String)>, String>),
    NpcLoaded(Result<Box<Character>, String>),
    AddTrap,
    TrapsLoaded(Result<Arc<UserDb<Traps>>, String>),
    /// View NPC, or Print NPC, on the encounter's NPC package with this
    /// index.
    NpcSheet(usize, bool),
    NpcSheetMade(Result<Arc<Sheet>, String>, bool),
    /// Roll Treasure on the encounter.
    EncounterTreasure,
    EncounterTable(TableEditInput),
    TableSaved(Result<String, String>),
    AskDeleteTable,
    TableDeleted(Result<(), String>),
    /// Create Encounter on the details of a template or a 2-20 table.
    GenerateEntry,
    /// Group Encounter on Manage Parties.
    GroupEncounter,
    GroupPrint(bool),
    GroupCondensed(bool),
    GroupPreview(bool),
    GroupAnswered(bool),
}

/// What Create on New NPC or New NPC Group made and saved.
#[derive(Clone)]
pub struct NpcsSaved {
    /// The file of the last NPC saved.
    file: Option<String>,
    said: String,
    /// The generator's notes on what it could not do.
    messages: Option<String>,
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
        if let Page::Table(roller) = &mut self.page {
            roller.close(&mut self.leftovers);
        }
        self.page = self.history.pop().unwrap_or(Page::Library);
        self.notice = None;
    }

    /// Back to the library, at the section `id`.
    fn show_section(&mut self, id: String) {
        self.close_tables();
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
            action::NEW_CHARACTER => self.new_character(false, context),
            action::ENTER_CHARACTER => self.new_character(true, context),
            action::NEW_NPC => self.new_npc(false, context),
            action::NEW_NPC_GROUP => self.new_npc(true, context),
            action::ROLL_TREASURE => self.open_treasure(None, false, context),
            action::ROLL_TABLE => self.open_tables(context),
            action::NEW_ENCOUNTER => self.open_encounter(Opening::Choose, context),
            action::NEW_TABLE => self.open_encounter(Opening::Table(None), context),
            action::ENCOUNTER_PRINT => {
                if let Page::Encounter(form) = &self.page {
                    let sheet = Arc::new(form.sheet());
                    self.print(Printable::Sheet(sheet), context);
                }
            }
            action::ENCOUNTER_PDF => {
                if let Page::Encounter(form) = &self.page {
                    let (sheet, title) = (Arc::new(form.sheet()), form.title());
                    self.save_pdf(Printable::Sheet(sheet), &title, context);
                }
            }
            action::ENCOUNTER_DELETE => {
                if let Page::Encounter(form) = &self.page
                    && let Some(name) = form.saved.clone()
                {
                    self.confirm = Some(Confirm::DeleteEncounter { name });
                }
            }
            action::MANAGE_PARTIES => {
                if self.install.is_some() && !matches!(self.page, Page::Parties(_)) {
                    self.go(Page::Parties(Box::default()));
                    self.read_parties(String::new(), Vec::new(), context);
                }
            }
            action::TREASURE_SAVE => {
                if matches!(self.page, Page::Treasure(_)) {
                    self.update(Message::Treasure(TreasureInput::Save), context);
                }
            }
            action::TREASURE_PRINT => {
                if let Page::Treasure(roller) = &self.page {
                    let sheet = Arc::new(roller.sheet());
                    self.print(Printable::Sheet(sheet), context);
                }
            }
            action::TREASURE_PDF => {
                if let Page::Treasure(roller) = &self.page {
                    let (sheet, title) = (Arc::new(roller.sheet()), roller.title());
                    self.save_pdf(Printable::Sheet(sheet), &title, context);
                }
            }
            action::TREASURE_DELETE => {
                if let Page::Treasure(roller) = &self.page
                    && let Some(name) = roller.saved.clone()
                {
                    self.confirm = Some(Confirm::DeleteTreasure { name });
                }
            }
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
                if let Some((printable, title)) = self.printable() {
                    self.save_pdf(printable, &title, context);
                }
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

    /// Starts New Character, or with `entering` Enter Character, once its
    /// rules are read.
    fn new_character(&mut self, entering: bool, context: &ComponentContext<Self>) {
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
                Needs::load(&install, entering, None, core_vs_option)
                    .map(|(rules, needs)| Box::new(Wizard::new(rules, needs)))
                    .map_err(|e| e.to_string()),
            )
        });
    }

    /// Opens New NPC, or with `group` New NPC Group, once the generator
    /// is read.
    fn new_npc(&mut self, group: bool, context: &ComponentContext<Self>) {
        if let Some(rules) = self.npc_rules.clone() {
            self.update(Message::NpcRulesLoaded(Ok(rules), group), context);
            return;
        }
        let Some(install) = self.install.clone() else {
            return;
        };
        if self.busy {
            return;
        }
        self.busy = true;
        context.spawn_background(move |_| {
            Message::NpcRulesLoaded(
                install.npc_rules().map(Arc::new).map_err(|e| e.to_string()),
                group,
            )
        });
    }

    /// Create on New NPC or New NPC Group: the NPCs are made and saved in
    /// the background.
    fn create_npcs(&mut self, context: &ComponentContext<Self>) {
        if self.busy {
            return;
        }
        enum Form {
            One(Box<NpcForm>),
            Group(Box<GroupForm>),
        }
        let form = match &self.page {
            Page::Npc(form) => Form::One(form.clone()),
            Page::NpcGroup(form) => Form::Group(form.clone()),
            _ => return,
        };
        let Some(install) = self.install.clone() else {
            return;
        };
        self.busy = true;
        let rand = self.rand.clone();
        context.spawn_background(move |_| {
            let mut rand = rand.lock().unwrap_or_else(|e| e.into_inner());
            let (npcs, party) = match &form {
                Form::One(form) => (
                    form.generate(&mut rand).map(|npc| vec![npc]),
                    form.party.trim().to_owned(),
                ),
                Form::Group(form) => (form.generate(&mut rand), form.name.trim().to_owned()),
            };
            drop(rand);
            let saved = npcs.and_then(|npcs: Vec<Npc>| {
                let mut file = None;
                for npc in &npcs {
                    file = Some(install.save_npc(npc, &party).map_err(|e| e.to_string())?);
                }
                let said = match (&form, npcs.first()) {
                    (Form::One(_), Some(npc)) => {
                        tr!("{name} was created", name = npc.character.data.name.clone())
                    }
                    _ => cr_app::npc::created_text(npcs.len()),
                };
                Ok(NpcsSaved {
                    file,
                    said,
                    messages: cr_app::npc::messages(&npcs),
                })
            });
            Message::NpcsSaved(saved)
        });
    }

    /// Opens Roll Treasure, on the saved bundle `saved` if given; with
    /// `copy`, on its items under no name, for Save to add a new bundle
    /// (the original's Copy…, 0x53f028).
    fn open_treasure(
        &mut self,
        saved: Option<String>,
        copy: bool,
        context: &ComponentContext<Self>,
    ) {
        let Some(install) = self.install.clone() else {
            return;
        };
        if self.busy {
            return;
        }
        self.busy = true;
        let rules = self.treasure_rules.clone();
        context.spawn_background(move |_| {
            let rules = match rules {
                Some(rules) => Ok(rules),
                None => install.treasure_rules().map(Arc::new),
            };
            let roller = rules.and_then(|rules| {
                let Some(name) = saved else {
                    return Ok(Roller::new(rules, [0; 26], Bundle::default()));
                };
                let bundle = install
                    .saved_treasures()?
                    .into_iter()
                    .find(|t| t.name == name)
                    .map(|t| t.bundle)
                    .unwrap_or_default();
                Ok(if copy {
                    Roller::new(rules, [0; 26], bundle)
                } else {
                    Roller::saved(rules, &name, bundle)
                })
            });
            Message::TreasureLoaded(roller.map(Box::new).map_err(|e| e.to_string()))
        });
    }

    /// Opens Roll on a Table once the tables are read.
    fn open_tables(&mut self, context: &ComponentContext<Self>) {
        if let Some(rolls) = self.table_rolls.clone() {
            self.update(Message::TableRollsLoaded(Ok(rolls)), context);
            return;
        }
        let Some(install) = self.install.clone() else {
            return;
        };
        if self.busy {
            return;
        }
        self.busy = true;
        context.spawn_background(move |_| {
            Message::TableRollsLoaded(
                install
                    .table_rolls()
                    .map(Arc::new)
                    .map_err(|e| e.to_string()),
            )
        });
    }

    /// Opens an encounter page on `opening` once the rules are read.
    fn open_encounter(&mut self, opening: Opening, context: &ComponentContext<Self>) {
        let Some(install) = self.install.clone() else {
            return;
        };
        if self.busy {
            return;
        }
        self.busy = true;
        let (cached, rand) = (self.encounter_rules.clone(), self.rand.clone());
        context.spawn_background(move |_| {
            let error = |e: cr_store::Error| e.to_string();
            let opened = (|| {
                let rules = match cached {
                    Some(rules) => rules,
                    None => Arc::new(install.encounter_rules().map_err(error)?),
                };
                let made = |name: &str, said: Option<String>| {
                    let mut rand = rand.lock().unwrap_or_else(|e| e.into_inner());
                    let made = cr_app::encounter::from_template(&rules, name, &mut rand);
                    let mut messages: Vec<String> = said.into_iter().collect();
                    messages.extend(made.messages);
                    let form =
                        EncounterForm::new(rules.clone(), made.encounter, None, made.pending);
                    EncounterOpened::Form(Box::new(form), messages)
                };
                let opened = match opening {
                    Opening::Choose => {
                        EncounterOpened::Chooser(Box::new(Chooser::new(rules.clone())))
                    }
                    Opening::Template { name, said } => made(&name, said),
                    Opening::RollTable(name) => {
                        let tables = install.frequency_tables().map_err(error)?;
                        let table = tables
                            .table(&name)
                            .ok_or_else(|| tr("Could Not Read").to_owned())?;
                        let (template, said) = {
                            let mut rand = rand.lock().unwrap_or_else(|e| e.into_inner());
                            cr_app::encounter::roll_and_generate(&rules, table, &mut rand)?
                        };
                        made(&template, Some(said))
                    }
                    Opening::Saved { name, copy } => {
                        let mut encounter = install
                            .saved_encounters()
                            .map_err(error)?
                            .into_iter()
                            .find(|e| e.name == name)
                            .ok_or_else(|| tr("Could Not Read").to_owned())?;
                        // The original's Copy (0x694117): no name, so that
                        // Save adds a new one once it is named.
                        if copy {
                            encounter.name.clear();
                        }
                        let saved = (!copy).then_some(name);
                        let form = EncounterForm::new(rules.clone(), encounter, saved, Vec::new());
                        EncounterOpened::Form(Box::new(form), Vec::new())
                    }
                    Opening::Table(name) => {
                        let table = match name {
                            Some(name) => install
                                .frequency_tables()
                                .map_err(error)?
                                .table(&name)
                                .cloned(),
                            None => None,
                        };
                        EncounterOpened::Table(Box::new(TableEditor::new(rules.clone(), table)))
                    }
                    Opening::Group(group) => {
                        let members = group
                            .members
                            .iter()
                            .map(|file| install.character(file))
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(error)?;
                        let mut encounter = Encounter::of_group(members);
                        if group.print {
                            encounter.name = group.party.clone();
                            let sheet = rules.group_sheet(&encounter, group.condensed);
                            EncounterOpened::Sheet(Arc::new(sheet), group.preview)
                        } else {
                            // The name is left for the user to give
                            // (0x44e759).
                            let form =
                                EncounterForm::new(rules.clone(), encounter, None, Vec::new());
                            EncounterOpened::Form(Box::new(form), Vec::new())
                        }
                    }
                };
                Ok((rules, opened))
            })();
            Message::EncounterOpened(opened)
        });
    }

    /// Does what the Encounter page asks of the app, and says what it
    /// could not do.
    fn encounter_effects(&mut self, context: &ComponentContext<Self>) {
        let Page::Encounter(form) = &mut self.page else {
            return;
        };
        let notice = form.notice.take();
        for effect in form.take_effects() {
            match effect {
                EncounterEffect::AddNpc(file) => self.with_install(context, move |install| {
                    Message::NpcLoaded(
                        install
                            .character(&file)
                            .map(Box::new)
                            .map_err(|e| e.to_string()),
                    )
                }),
                EncounterEffect::Save { old, encounter } => {
                    self.with_install(context, move |install| {
                        let saved = (|| {
                            let mut list = install.saved_encounters().map_err(|e| e.to_string())?;
                            let except = old
                                .as_ref()
                                .and_then(|o| list.iter().position(|e| &e.name == o));
                            check_name(&list, &encounter.name, except)
                                .map_err(|e| e.to_string())?;
                            let name = encounter.name.clone();
                            match except {
                                Some(i) => list[i] = *encounter,
                                None => list.push(*encounter),
                            }
                            install
                                .write_saved_encounters(&list)
                                .map_err(|e| e.to_string())?;
                            Ok(name)
                        })();
                        Message::EncounterSaved(saved)
                    })
                }
            }
        }
        if let Some((title, text)) = notice {
            self.notice = Some(Notice::new(InfoBarSeverity::Warning, title, text));
        }
    }

    /// Does what the 2-20 table editor asks of the app.
    fn table_effects(&mut self, context: &ComponentContext<Self>) {
        let Page::EncounterTable(editor) = &mut self.page else {
            return;
        };
        for effect in editor.take_effects() {
            match effect {
                TableEffect::Notice(text) => {
                    self.notice = Some(Notice::new(InfoBarSeverity::Informational, text, ""))
                }
                TableEffect::Generate { template, said } => self.open_encounter(
                    Opening::Template {
                        name: template,
                        said: Some(said),
                    },
                    context,
                ),
                TableEffect::Save { old, table } => self.with_install(context, move |install| {
                    let saved = (|| {
                        let mut tables = install.frequency_tables().map_err(|e| e.to_string())?;
                        let clash = tables.tables.iter().any(|x| {
                            x.name.eq_ignore_ascii_case(&table.name)
                                && Some(&x.name) != old.as_ref()
                        });
                        if clash {
                            return Err(tr("Name already used.").to_owned());
                        }
                        let name = table.name.clone();
                        match old
                            .as_ref()
                            .and_then(|o| tables.tables.iter().position(|x| &x.name == o))
                        {
                            Some(i) => tables.tables[i] = *table,
                            None => tables.tables.push(*table),
                        }
                        install
                            .write_frequency_tables(&tables)
                            .map_err(|e| e.to_string())?;
                        Ok(name)
                    })();
                    Message::TableSaved(saved)
                }),
            }
        }
    }

    /// Shows `page` in place of New Encounter or the 2-20 table editor,
    /// which close for it as the original's dialogs do, or over the page
    /// shown.
    fn open_page(&mut self, page: Page) {
        if matches!(self.page, Page::NewEncounter(_) | Page::EncounterTable(_)) {
            self.page = page;
            self.notice = None;
        } else {
            self.go(page);
        }
    }

    /// Reads Manage Parties' lists again, on the party `selected`, with
    /// the parties `new` made there.
    fn read_parties(&self, selected: String, new: Vec<String>, context: &ComponentContext<Self>) {
        self.with_install(context, move |install| {
            Message::PartiesRead(
                PartyLists::read(install, &selected, &new).map_err(|e| e.to_string()),
            )
        });
    }

    /// Ends the Roll Results session of Roll on a Table, shown or under
    /// the page shown, as closing the original's dialog does.
    fn close_tables(&mut self) {
        for page in std::iter::once(&mut self.page).chain(&mut self.history) {
            if let Page::Table(roller) = page {
                roller.close(&mut self.leftovers);
            }
        }
    }

    /// Does what the treasure roller asks of the app.
    fn treasure_effects(&mut self, context: &ComponentContext<Self>) {
        let Page::Treasure(roller) = &mut self.page else {
            return;
        };
        for effect in roller.take_effects() {
            match effect {
                TreasureEffect::Save { old, name, bundle } => {
                    let Some(install) = self.install.clone() else {
                        roller.saving = false;
                        continue;
                    };
                    context.spawn_background(move |_| {
                        let saved = match &old {
                            Some(old) => install.replace_saved_treasure(old, &name, &bundle),
                            None => install.save_treasure(&name, &bundle),
                        };
                        Message::TreasureSaved(saved.map(|()| name).map_err(|e| e.to_string()))
                    });
                }
            }
        }
    }

    /// Does what the wizard asks of the app.
    fn wizard_effects(&mut self, context: &ComponentContext<Self>) {
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
                // Create pressed again while it writes creates nothing
                // more.
                Effect::Create(draft) => {
                    let Some(install) = self.install.clone() else {
                        continue;
                    };
                    if self.busy {
                        continue;
                    }
                    self.busy = true;
                    context.spawn_background(move |_| {
                        let entered = draft.entered.is_some();
                        let created = install
                            .creation_rules()
                            .and_then(|rules| install.create_character(&rules, &draft))
                            .map_err(|e| e.to_string());
                        Message::Created(created, entered)
                    });
                }
            }
        }
    }

    /// Gives the details page the portrait of `file`, or none.
    fn set_portrait(&mut self, file: Option<String>, context: &ComponentContext<Self>) {
        let stored = file.as_deref().map(stored_portrait).unwrap_or_default();
        match &mut self.page {
            Page::Create(wizard) => {
                wizard.input(Input::Portrait(stored));
                self.wizard_effects(context);
            }
            Page::Edit(editor) => {
                editor.input(EditInput::Portrait(stored));
                self.editor_effects(context);
            }
            _ => {}
        }
    }

    /// The portrait `stored` as the details page shows it, read again
    /// only when it changed.
    fn portrait_shown(&self, stored: &[String; 2]) -> Rc<portrait::Shown> {
        let mut cached = self.portrait.borrow_mut();
        match cached.as_ref() {
            Some(shown) if shown.stored == *stored => shown.clone(),
            _ => {
                let shown = Rc::new(portrait::Shown::read(self.install.as_deref(), stored));
                *cached = Some(shown.clone());
                shown
            }
        }
    }

    /// Does what the editor asks of the app.
    fn editor_effects(&mut self, context: &ComponentContext<Self>) {
        let Page::Edit(editor) = &mut self.page else {
            return;
        };
        for effect in editor.take_effects() {
            match effect {
                EditEffect::Save(saving) => {
                    let Some(install) = self.install.clone() else {
                        editor.failed(tr("The installation is no longer open").to_owned());
                        continue;
                    };
                    context.spawn_background(move |_| {
                        let name = saving.details.name.clone();
                        Message::Saved(
                            install
                                .save_character(&saving.file, &saving.details, Some(&saving.lists))
                                .map(|()| (saving.file.clone(), name))
                                .map_err(|e| e.to_string()),
                        )
                    });
                }
            }
        }
        self.editor_icons();
    }

    /// Makes the icons of the items the editor's bundles hold, once each.
    fn editor_icons(&mut self) {
        let Page::Edit(editor) = &self.page else {
            return;
        };
        let icons = &editor.needs().icons;
        for item in editor.equipment().iter().flatten() {
            if !self.icons.contains_key(&item.name) {
                let icon = icons
                    .get(&item.name)
                    .and_then(cr_app::pictures::icon)
                    .and_then(|i| crate::ui::png(&i));
                self.icons.insert(item.name.clone(), icon);
            }
        }
    }

    /// `page` with Choose Portrait over it while it is open.
    fn with_portraits(&self, page: View, context: &mut ViewContext<Self>) -> View {
        match &self.portraits {
            Some(chooser) => Grid::new().children((page, portrait::dialog(chooser, context))),
            None => page,
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

    /// Saves a sheet as a PDF the user names, after `title`.
    fn save_pdf(&mut self, printable: Printable, title: &str, context: &ComponentContext<Self>) {
        let Some(install) = self.install.clone() else {
            return;
        };
        let name = cr_app::file_name(title, "pdf");
        self.dialog(
            context,
            move |owner| {
                let filters = [(tr("PDF documents").to_owned(), "*.pdf".to_owned())];
                let title = plain(tr("Save as _PDF…"));
                let Some(path) = native::dialogs::save_file(owner, &title, &name, "pdf", &filters)
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
            Page::Edit(editor) => editor.help(),
            Page::Create(wizard) => wizard.help(),
            Page::Import(_) => topic::IMPORT,
            Page::Sheet(_) | Page::SheetOptions(_) => topic::CHARACTER_SHEET,
            Page::Settings | Page::Help(_) => topic::MAIN_MENU,
            Page::Book(_) => topic::BOOKS,
            Page::Npc(_) => topic::NPC_CREATION,
            Page::NpcGroup(_) => topic::NPC_GROUP,
            Page::Treasure(_) => topic::ROLL_TREASURE,
            Page::Table(_) => topic::TABLES,
            Page::Parties(_) => topic::PARTY_MANAGEMENT,
            Page::NewEncounter(_) => topic::ENCOUNTERS,
            Page::Encounter(_) => topic::ENCOUNTER,
            Page::EncounterTable(_) => topic::ENCOUNTER_TABLE,
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
            icons: forms::Icons::new(),
            portraits: None,
            portrait: RefCell::new(None),
            reading: Rc::default(),
            rand: Arc::new(Mutex::new(Rand::from_clock())),
            npc_rules: None,
            treasure_rules: None,
            table_rolls: None,
            table_last: None,
            leftovers: Leftovers::default(),
            encounter_rules: None,
            group: None,
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
                            self.npc_rules = None;
                            self.treasure_rules = None;
                            self.table_rolls = None;
                            self.encounter_rules = None;
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
                        let loaded = install.character(&file).and_then(|character| {
                            let needs = EditNeeds::load(install)?;
                            Ok(Editor::new(Arc::new(needs), &file, character))
                        });
                        Message::EditLoaded(loaded.map(Box::new).map_err(|e| e.to_string()))
                    });
                }
            }
            Message::EditLoaded(Ok(editor)) => {
                self.adding.clear();
                self.icons.clear();
                self.go(Page::Edit(editor));
                self.editor_icons();
            }
            Message::EditLoaded(Err(e)) => self.show_error(tr("Edit Character"), e),
            Message::Editor(change) => {
                if let Page::Edit(editor) = &mut self.page {
                    // Added, the choice leaves the box.
                    if let EditInput::Add(kind, _) = &change {
                        self.adding.retain(|(k, _)| k != kind);
                    }
                    editor.input(change);
                    self.editor_effects(context);
                }
            }
            Message::EditorAnswer(serial, answer) => {
                if let Page::Edit(editor) = &mut self.page
                    && editor.prompt_serial() == serial
                {
                    editor.answer(answer);
                    self.editor_effects(context);
                }
            }
            Message::Saved(Ok((file, name))) => {
                self.back();
                self.notice = Some(Notice::success(tr!("Saved {name}", name)));
                self.load(Some(file), context);
            }
            Message::Saved(Err(e)) => {
                if let Page::Edit(editor) = &mut self.page {
                    editor.failed(e);
                }
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
                (Some(Confirm::DeleteParty { party }), ContentDialogResult::Primary) => {
                    self.with_install(context, move |install| {
                        // Without the GTK app's Undo, the kept files go at
                        // once, as when its toast closes.
                        let deleted = install
                            .delete_party(&party)
                            .and_then(|deleted| {
                                install.forget_deleted(&deleted)?;
                                Ok((deleted.len(), party))
                            })
                            .map_err(|e| e.to_string());
                        Message::PartyDeleted(deleted)
                    });
                }
                (Some(Confirm::DeleteEncounter { name }), ContentDialogResult::Primary) => {
                    self.with_install(context, move |install| {
                        let deleted = install.saved_encounters().and_then(|mut list| {
                            list.retain(|e| e.name != name);
                            install.write_saved_encounters(&list)
                        });
                        Message::EncounterDeleted(deleted.map_err(|e| e.to_string()))
                    });
                }
                (Some(Confirm::DeleteTable { name }), ContentDialogResult::Primary) => {
                    self.with_install(context, move |install| {
                        let deleted = install.frequency_tables().and_then(|mut tables| {
                            tables.tables.retain(|t| t.name != name);
                            install.write_frequency_tables(&tables)
                        });
                        Message::TableDeleted(deleted.map_err(|e| e.to_string()))
                    });
                }
                (Some(Confirm::DeleteTreasure { name }), ContentDialogResult::Primary) => {
                    self.with_install(context, move |install| {
                        Message::TreasureDeleted(
                            install
                                .delete_saved_treasure(&name)
                                .map(|()| name)
                                .map_err(|e| e.to_string()),
                        )
                    });
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
                    self.wizard_effects(context);
                }
            }
            Message::Created(Ok(file), entered) => {
                self.busy = false;
                self.show_section(cr_app::library::CHARACTERS.to_owned());
                // Enter Character's "Character Saved" (0x496b0b).
                self.notice = Some(Notice::success(if entered {
                    tr("Character saved")
                } else {
                    tr("Character created")
                }));
                self.load(Some(file), context);
            }
            Message::Created(Err(e), _) => {
                self.busy = false;
                if let Page::Create(wizard) = &mut self.page {
                    wizard.failed(e);
                }
            }
            Message::ChoosePortrait => {
                // The race whose portraits come first.
                let race = match &self.page {
                    Page::Create(wizard) if matches!(wizard.page(), CreatePage::Details(_)) => {
                        Some(wizard.draft().race.clone())
                    }
                    Page::Edit(editor) if editor.tab == EditTab::Details => {
                        Some(editor.character().data.race.clone())
                    }
                    _ => None,
                };
                if let Some(race) = race
                    && self.portraits.is_none()
                {
                    self.portraits = Some(portrait::Chooser {
                        race: race.clone(),
                        tiles: None,
                    });
                    self.with_install(context, move |install| {
                        Message::PortraitsListed(portrait::list(install, &race))
                    });
                }
            }
            Message::PortraitsListed(tiles) => {
                if let Some(chooser) = &mut self.portraits {
                    chooser.tiles = Some(tiles);
                }
            }
            Message::PortraitChosen(file) => {
                self.portraits = None;
                self.set_portrait(file, context);
            }
            Message::PortraitFile => {
                self.portraits = None;
                let Some(install) = self.install.clone() else {
                    return;
                };
                self.dialog(
                    context,
                    move |owner| {
                        let filters = portrait::filters();
                        match native::dialogs::open_file(owner, tr("Choose a Picture"), &filters) {
                            Ok(Some(path)) => portrait::add(&install, &path).map(Some),
                            Ok(None) => Ok(None),
                            Err(e) => Err(e.message()),
                        }
                    },
                    Message::PortraitAdded,
                );
            }
            Message::PortraitAdded(added) => {
                self.busy = false;
                match added {
                    Ok(Some(file)) => self.set_portrait(Some(file), context),
                    Ok(None) => {}
                    Err(e) => self.show_error(tr("Could Not Use the Picture"), e),
                }
            }
            Message::ClosePortraits => self.portraits = None,
            Message::Adding(kind, name) => {
                self.adding.retain(|(k, _)| *k != kind);
                self.adding.push((kind, name));
            }
            Message::WizardAnswer(serial, answer) => {
                if let Page::Create(wizard) = &mut self.page
                    && wizard.prompt_serial() == serial
                {
                    wizard.answer(answer);
                    self.wizard_effects(context);
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
                        // Imported races and classes change what the NPC
                        // generator offers, imported items the treasure.
                        self.npc_rules = None;
                        self.treasure_rules = None;
                        self.encounter_rules = None;
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
            Message::NpcRulesLoaded(loaded, group) => {
                self.busy = false;
                match loaded {
                    Ok(rules) => {
                        self.npc_rules = Some(rules.clone());
                        self.go(if group {
                            Page::NpcGroup(Box::new(GroupForm::new(rules)))
                        } else {
                            Page::Npc(Box::new(NpcForm::new(rules, false)))
                        });
                    }
                    Err(e) => self.show_error(tr("NPC Generator"), e),
                }
            }
            Message::Npc(input) => {
                if let Page::Npc(form) = &mut self.page {
                    let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
                    form.input(input, &mut rand);
                }
            }
            Message::Group(input) => {
                if let Page::NpcGroup(form) = &mut self.page {
                    form.input(input);
                }
            }
            Message::CreateNpcs => self.create_npcs(context),
            Message::NpcsSaved(Ok(saved)) => {
                self.busy = false;
                self.show_section(cr_app::library::CHARACTERS.to_owned());
                self.notice = Some(Notice::success(saved.said));
                if let Some(text) = saved.messages {
                    self.confirm = Some(Confirm::Info {
                        title: tr("NPC Generator").to_owned(),
                        text,
                    });
                }
                self.load(saved.file, context);
            }
            Message::NpcsSaved(Err(e)) => {
                self.busy = false;
                match &mut self.page {
                    Page::Npc(form) => form.error = Some(e),
                    Page::NpcGroup(form) => form.error = Some(e),
                    _ => self.show_error(tr("NPC Generator"), e),
                }
            }
            Message::TreasureLoaded(loaded) => {
                self.busy = false;
                match loaded {
                    Ok(roller) => {
                        self.treasure_rules = Some(roller.rules().clone());
                        self.go(Page::Treasure(roller));
                    }
                    Err(e) => self.show_error(tr("Roll Treasure"), e),
                }
            }
            Message::Treasure(input) => {
                if let Page::Treasure(roller) = &mut self.page {
                    let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
                    roller.input(input, &mut rand);
                    drop(rand);
                    self.treasure_effects(context);
                }
            }
            Message::UseTreasure => {
                let Page::Treasure(roller) = &self.page else {
                    return;
                };
                if !roller.pick {
                    return;
                }
                let bundle = roller.bundle.clone();
                self.back();
                self.update(
                    Message::Encounter(EncounterInput::Treasure(bundle)),
                    context,
                );
            }
            Message::TreasureSaved(Ok(name)) => {
                if let Page::Treasure(roller) = &mut self.page
                    && roller.saving
                {
                    roller.saved_as(name.clone());
                }
                self.notice = Some(Notice::success(tr("Treasure saved")));
                self.load(Some(format!("{TREASURE_KEY}{name}")), context);
            }
            Message::TreasureSaved(Err(e)) => {
                if let Page::Treasure(roller) = &mut self.page {
                    roller.saving = false;
                }
                self.show_error(tr("Save Treasure"), e);
            }
            Message::TreasureDeleted(Ok(_)) => {
                if matches!(self.page, Page::Treasure(_)) {
                    self.back();
                }
                self.notice = Some(Notice::success(tr("Saved treasure deleted")));
                self.load(None, context);
            }
            Message::TreasureDeleted(Err(e)) => {
                self.show_error(&plain(tr("_Delete Saved Treasure")), e)
            }
            Message::EditEntry | Message::CopyEntry => {
                let copy = matches!(message, Message::CopyEntry);
                let Some(key) = self.selected.clone() else {
                    return;
                };
                if let Some(name) = key.strip_prefix(TREASURE_KEY) {
                    self.open_treasure(Some(name.to_owned()), copy, context);
                } else if let Some(name) = key.strip_prefix(ENCOUNTER_KEY) {
                    let name = name.to_owned();
                    self.open_encounter(Opening::Saved { name, copy }, context);
                } else if let Some(name) = key.strip_prefix(TABLE_KEY)
                    && !copy
                {
                    self.open_encounter(Opening::Table(Some(name.to_owned())), context);
                }
            }
            Message::GenerateEntry => {
                let Some(key) = self.selected.clone() else {
                    return;
                };
                if let Some(name) = cr_app::library::template_name(&key) {
                    let name = name.to_owned();
                    self.open_encounter(Opening::Template { name, said: None }, context);
                } else if let Some(name) = key.strip_prefix(TABLE_KEY) {
                    self.open_encounter(Opening::RollTable(name.to_owned()), context);
                }
            }
            Message::EncounterOpened(opened) => {
                self.busy = false;
                match opened {
                    Ok((rules, opened)) => {
                        self.encounter_rules = Some(rules);
                        match opened {
                            EncounterOpened::Chooser(chooser) => {
                                self.go(Page::NewEncounter(chooser))
                            }
                            EncounterOpened::Form(form, messages) => {
                                self.open_page(Page::Encounter(form));
                                if !messages.is_empty() {
                                    self.notice = Some(Notice::new(
                                        InfoBarSeverity::Informational,
                                        messages.join("\n"),
                                        "",
                                    ));
                                }
                            }
                            EncounterOpened::Table(editor) => self.go(Page::EncounterTable(editor)),
                            EncounterOpened::Sheet(sheet, true) => self.go(Page::Sheet(Arc::new(
                                SheetPage::new((*sheet).clone(), None, None),
                            ))),
                            EncounterOpened::Sheet(sheet, false) => {
                                self.print(Printable::Sheet(sheet), context)
                            }
                        }
                    }
                    Err(e) => self.show_error(&e, ""),
                }
            }
            Message::TemplateSearch(search) => {
                if let Page::NewEncounter(chooser) = &mut self.page {
                    chooser.search = search;
                    chooser.chosen = None;
                }
            }
            Message::ChooseTemplate(name) => {
                if let Page::NewEncounter(chooser) = &mut self.page {
                    chooser.chosen = Some(name);
                }
            }
            Message::CreateEncounter => {
                let Page::NewEncounter(chooser) = &self.page else {
                    return;
                };
                match chooser.chosen.clone() {
                    Some(name) if name.is_empty() => {
                        let form = EncounterForm::new(
                            chooser.rules.clone(),
                            Encounter::default(),
                            None,
                            Vec::new(),
                        );
                        self.open_page(Page::Encounter(Box::new(form)));
                    }
                    Some(name) => {
                        self.open_encounter(Opening::Template { name, said: None }, context)
                    }
                    None => {}
                }
            }
            Message::Encounter(input) => {
                if let Page::Encounter(form) = &mut self.page {
                    let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
                    form.input(input, &mut rand);
                    drop(rand);
                    self.encounter_effects(context);
                }
            }
            Message::EncounterSaved(Ok(name)) => {
                if let Page::Encounter(form) = &mut self.page
                    && form.saving
                {
                    form.saved_as(name.clone());
                }
                self.notice = Some(Notice::success(tr("Encounter saved")));
                self.load(Some(format!("{ENCOUNTER_KEY}{name}")), context);
            }
            Message::EncounterSaved(Err(e)) => {
                if let Page::Encounter(form) = &mut self.page {
                    form.saving = false;
                }
                self.show_error(&e, "");
            }
            Message::EncounterDeleted(Ok(())) => {
                if matches!(self.page, Page::Encounter(_)) {
                    self.back();
                }
                self.notice = Some(Notice::success(tr("Encounter deleted")));
                self.load(None, context);
            }
            Message::EncounterDeleted(Err(e)) => self.show_error(&e, ""),
            Message::AddNpc => self.with_install(context, |install| {
                Message::NpcsListed(
                    install
                        .characters()
                        .map(|index| {
                            index
                                .entries
                                .into_iter()
                                .map(|e| (e.name, e.file))
                                .collect()
                        })
                        .map_err(|e| e.to_string()),
                )
            }),
            Message::NpcsListed(listed) => match listed {
                Ok(list) => {
                    if let Page::Encounter(form) = &mut self.page {
                        form.open_npcs(list);
                    }
                }
                Err(e) => self.show_error(tr("Add NPC"), e),
            },
            Message::NpcLoaded(loaded) => match loaded {
                Ok(character) => {
                    if let Page::Encounter(form) = &mut self.page {
                        form.add_npc(*character);
                        self.encounter_effects(context);
                    }
                }
                Err(e) => self.show_error(tr("Add NPC"), e),
            },
            Message::AddTrap => self.with_install(context, |install| {
                Message::TrapsLoaded(
                    UserDb::<Traps>::load(install)
                        .map(Arc::new)
                        .map_err(|e| e.to_string()),
                )
            }),
            Message::TrapsLoaded(loaded) => match loaded {
                Ok(db) => {
                    if let Page::Encounter(form) = &mut self.page {
                        form.open_traps(db);
                    }
                }
                Err(e) => self.show_error(tr("Add Trap"), e),
            },
            Message::NpcSheet(i, print) => {
                let Page::Encounter(form) = &self.page else {
                    return;
                };
                let Some(character) = form.encounter.npcs.get(i).map(|p| p.character.clone())
                else {
                    return;
                };
                self.with_install(context, move |install| {
                    Message::NpcSheetMade(
                        install
                            .npc_sheet(&character)
                            .map(Arc::new)
                            .map_err(|e| e.to_string()),
                        print,
                    )
                });
            }
            Message::NpcSheetMade(Ok(sheet), true) => self.print(Printable::Sheet(sheet), context),
            Message::NpcSheetMade(Ok(sheet), false) => self.go(Page::Sheet(Arc::new(
                SheetPage::new((*sheet).clone(), None, None),
            ))),
            Message::NpcSheetMade(Err(e), _) => self.show_error(&e, ""),
            Message::EncounterTreasure => {
                if let Page::Encounter(form) = &self.page {
                    let roller = Roller::pick(
                        form.treasure_rules().clone(),
                        form.treasure_counts(),
                        form.encounter.treasure.clone(),
                    );
                    self.go(Page::Treasure(Box::new(roller)));
                }
            }
            Message::EncounterTable(input) => {
                if let Page::EncounterTable(editor) = &mut self.page {
                    let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
                    editor.input(input, &mut rand);
                    drop(rand);
                    self.table_effects(context);
                }
            }
            Message::TableSaved(Ok(name)) => {
                if matches!(self.page, Page::EncounterTable(_)) {
                    self.back();
                }
                self.notice = Some(Notice::success(tr("Table saved")));
                self.load(Some(format!("{TABLE_KEY}{name}")), context);
            }
            Message::TableSaved(Err(e)) => {
                if let Page::EncounterTable(editor) = &mut self.page {
                    editor.saving = false;
                    editor.error = Some(e);
                }
            }
            Message::AskDeleteTable => {
                if let Page::EncounterTable(editor) = &self.page
                    && let Some(name) = editor.saved.clone()
                {
                    self.confirm = Some(Confirm::DeleteTable { name });
                }
            }
            Message::TableDeleted(Ok(())) => {
                if matches!(self.page, Page::EncounterTable(_)) {
                    self.back();
                }
                self.notice = Some(Notice::success(tr("Table deleted")));
                self.load(None, context);
            }
            Message::TableDeleted(Err(e)) => self.show_error(&e, ""),
            Message::GroupEncounter => {
                if let Page::Parties(page) = &self.page
                    && let Some(lists) = &page.lists
                    && !lists.selected.is_empty()
                {
                    self.group = Some(GroupDialog {
                        party: lists.selected.clone(),
                        members: lists.npcs.clone(),
                        print: false,
                        condensed: false,
                        preview: false,
                    });
                }
            }
            Message::GroupPrint(print) => {
                if let Some(group) = &mut self.group {
                    group.print = print;
                }
            }
            Message::GroupCondensed(on) => {
                if let Some(group) = &mut self.group {
                    group.condensed = on;
                }
            }
            Message::GroupPreview(on) => {
                if let Some(group) = &mut self.group {
                    group.preview = on;
                }
            }
            Message::GroupAnswered(yes) => {
                if let Some(group) = self.group.take()
                    && yes
                {
                    self.open_encounter(Opening::Group(group), context);
                }
            }
            Message::TableRollsLoaded(loaded) => {
                self.busy = false;
                match loaded {
                    Ok(rolls) => {
                        self.table_rolls = Some(rolls.clone());
                        self.go(Page::Table(Box::new(TableRoller::new(
                            rolls,
                            self.table_last,
                        ))));
                    }
                    Err(e) => self.show_error(tr("Could Not Read the Tables"), e),
                }
            }
            Message::Table(input) => {
                if let Page::Table(roller) = &mut self.page {
                    let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
                    roller.input(input, &mut self.leftovers, &mut rand);
                    self.table_last = roller.last();
                }
            }
            Message::CopyResults => {
                if let Page::Table(roller) = &self.page
                    && let Some(result) = &roller.result
                {
                    match native::copy(&result.to_text()) {
                        Ok(()) => {
                            self.notice = Some(Notice::success(tr("Copied to the clipboard")))
                        }
                        Err(e) => self.show_error(&e.message(), ""),
                    }
                }
            }
            Message::ReadTable => {
                let Page::Table(roller) = &mut self.page else {
                    return;
                };
                if let Some((book, number)) = roller.result.as_ref().and_then(|r| r.reference) {
                    roller.close(&mut self.leftovers);
                    self.open_book(
                        book.short_name().to_owned(),
                        Some(format!("Table {number}:")),
                        context,
                    );
                }
            }
            Message::PartiesRead(read) => match read {
                Ok(lists) => {
                    if let Page::Parties(page) = &mut self.page {
                        page.lists = Some(lists);
                    }
                }
                Err(e) => self.show_error(tr("Manage Parties"), e),
            },
            Message::PartyChosen(party) => {
                if let Page::Parties(page) = &self.page
                    && !party.eq_ignore_ascii_case(page.selected())
                {
                    let new = page.new_parties();
                    self.read_parties(party, new, context);
                }
            }
            Message::PartyName(name) => {
                if let Page::Parties(page) = &mut self.page {
                    page.name = name;
                    page.problem = None;
                }
            }
            Message::NewParty => {
                let Page::Parties(page) = &self.page else {
                    return;
                };
                let name = page.name.trim().to_owned();
                let mut new = page.new_parties();
                self.with_install(context, move |install| {
                    let made = match install.party_name_problem(&name, &new) {
                        Ok(None) => {
                            new.push(name.clone());
                            PartyLists::read(install, &name, &new).map_err(|e| e.to_string())
                        }
                        Ok(Some(problem)) => Err(problem),
                        Err(e) => Err(e.to_string()),
                    };
                    Message::NewPartyMade(made)
                });
            }
            Message::NewPartyMade(made) => {
                if let Page::Parties(page) = &mut self.page {
                    match made {
                        Ok(lists) => {
                            page.lists = Some(lists);
                            page.name.clear();
                            page.problem = None;
                        }
                        Err(problem) => page.problem = Some(problem),
                    }
                }
            }
            Message::MoveToParty(file, party) => {
                let Page::Parties(page) = &self.page else {
                    return;
                };
                let (selected, new) = (page.selected().to_owned(), page.new_parties());
                self.with_install(context, move |install| {
                    let moved = install.set_party(&file, &party).and_then(|moved| {
                        let lists = PartyLists::read(install, &selected, &new)?;
                        Ok((file, moved, lists))
                    });
                    Message::PartyMoved(moved.map_err(|e| e.to_string()))
                });
            }
            Message::PartyMoved(Ok((before, after, lists))) => {
                if let Page::Parties(page) = &mut self.page {
                    page.lists = Some(lists);
                }
                // The character on the details keeps its place there under
                // its new file.
                let key = self
                    .selected
                    .as_deref()
                    .is_some_and(|k| k.eq_ignore_ascii_case(&before))
                    .then_some(after);
                self.load(key, context);
            }
            Message::PartyMoved(Err(e)) => self.show_error(tr("Manage Parties"), e),
            Message::AskDeleteParty => {
                if let Page::Parties(page) = &self.page
                    && !page.selected().is_empty()
                {
                    self.confirm = Some(Confirm::DeleteParty {
                        party: page.selected().to_owned(),
                    });
                }
            }
            Message::PartyDeleted(Ok((n, party))) => {
                if matches!(self.page, Page::Parties(_)) {
                    self.back();
                }
                self.notice = Some(Notice::success(trn!(
                    "{n} character of “{party}” deleted",
                    "{n} characters of “{party}” deleted",
                    n,
                    party
                )));
                self.load(None, context);
            }
            Message::PartyDeleted(Err(e)) => self.show_error(tr("Manage Parties"), e),
            Message::TableTreasure => {
                if let Page::Table(roller) = &mut self.page {
                    roller.close(&mut self.leftovers);
                    self.open_treasure(None, false, context);
                }
            }
        }
    }

    fn view(&self, _file: &Option<PathBuf>, context: &mut ViewContext<Self>) -> View {
        context.window_title("Core Rules");
        let sized = context.callback(Message::WindowSized);
        context.on_window_size(sized);
        crate::ui::set_room(if self.width > 0.0 {
            book::room(self)
        } else {
            f64::INFINITY
        });
        context.window_visuals(
            WindowVisuals::new()
                .backdrop(WindowBackdrop::Mica)
                .client_size(1100.0, 720.0),
        );

        let content = match &self.page {
            Page::Library => library::view(self, context),
            Page::Create(wizard) => {
                let portrait = match wizard.page() {
                    CreatePage::Details(page) => Some(self.portrait_shown(&page.portrait)),
                    _ => None,
                };
                let page = create::view(wizard, &self.adding, portrait.as_deref(), context);
                self.with_portraits(page, context)
            }
            Page::Edit(editor) => {
                let portrait = self.portrait_shown(&editor.details.portrait);
                let page = edit::view(editor, &self.adding, &self.icons, &portrait, context);
                self.with_portraits(page, context)
            }
            Page::Import(package) => import::view(self, package, context),
            Page::Settings => settings::view(self, context),
            Page::Sheet(page) => sheet::view(self, page, context),
            Page::SheetOptions(options) => sheet::options_view(options, context),
            Page::Help(page) => help::view(self, page, context),
            Page::Book(page) => book::view(self, page, context),
            Page::Npc(form) => npc::view(form, self.busy, context),
            Page::NpcGroup(form) => npc::group_view(form, self.busy, context),
            Page::Treasure(roller) => treasure::view(roller, self.busy, context),
            Page::Table(roller) => tables::view(roller, context),
            Page::Parties(page) => {
                let page = parties::view(page, self.busy, context);
                match &self.group {
                    Some(group) => {
                        Grid::new().children((page, encounter::group_dialog(group, context)))
                    }
                    None => page,
                }
            }
            Page::NewEncounter(chooser) => encounter::chooser_view(chooser, self.busy, context),
            Page::Encounter(form) => encounter::view(form, self.busy, context),
            Page::EncounterTable(editor) => encounter::table_view(editor, self.busy, context),
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

        // Over a page of its own (a character's sheet, New Character, a
        // book) no section is selected, so that choosing the section
        // shown goes back to its list.
        let selected_tag = match self.page {
            Page::Settings => SETTINGS,
            Page::Help(_) => HELP,
            Page::Library => self.section.as_str(),
            _ => "",
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
            Some(Confirm::DeleteParty { party }) => (
                tr("Delete Party Members?").to_owned(),
                tr!(
                    "Every character in “{party}” will be deleted, and the party with them.",
                    party = party.clone()
                ),
                plain(tr("_Delete")),
            ),
            Some(Confirm::DeleteEncounter { name }) => (
                tr("Delete Encounter?").to_owned(),
                tr!(
                    "“{name}” will be removed from the saved encounters",
                    name = name.clone()
                ),
                plain(tr("_Delete")),
            ),
            Some(Confirm::DeleteTable { name }) => (
                tr("Delete Encounter Table?").to_owned(),
                tr!("“{name}” will be removed", name = name.clone()),
                plain(tr("_Delete")),
            ),
            Some(Confirm::DeleteTreasure { name }) => (
                tr("Delete Saved Treasure?").to_owned(),
                tr!("“{name}” will be removed from Saved Treasure", name = name.clone()),
                plain(tr("_Delete")),
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
