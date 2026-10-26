//! The window: a title bar over a NavigationView with the library's
//! sections and Settings, as Windows 11 apps are laid out. One component
//! holds the state and handles every message; the pages are drawn by the
//! functions of the modules below, from that state.

mod allowed;
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
mod packages;
mod parties;
mod portrait;
mod record;
mod settings;
mod sheet;
mod tables;
mod templates;
mod treasure;
mod treasure_tables;
mod web;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
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
use cr_app::packages::{ItemChooser, PackageEditor, Renaming, Step, file_name};
use cr_app::parties::PartyLists;
use cr_app::records::actions::{self as record_actions, Act, Opened as Acted};
use cr_app::records::allowed::{AllowedEffect, AllowedForm, AllowedInput};
use cr_app::records::form::{RecordEffect, RecordForm, RecordInput, Saved};
use cr_app::records::tables::{
    TableEffect as TreasureTableEffect, TableForm, TableInput as TreasureTableInput,
};
use cr_app::records::templates::{
    NpcFor, TemplateEffect, TemplateForm, TemplateInput, template_npc,
};
use cr_app::sheets::Export;
use cr_app::tables::{TableInput, TableRoller};
use cr_app::treasure::{Roller, TreasureEffect, TreasureInput};
use cr_formats::character::Character;
use cr_formats::export::ExportFile;
use cr_i18n::{tr, trc, trn};
use cr_store::Installation;
use cr_store::catalog::{ENCOUNTER_KEY, Entry, Section, TABLE_KEY, TREASURE_KEY};
use cr_store::encounter::{Encounter, EncounterRules, check_name};
use cr_store::export::{Content, Exportable, ImportReport};
use cr_store::help::{ItemHelp, topic};
use cr_store::npc::{Npc, NpcRules};
use cr_store::pictures::stored_portrait;
use cr_store::sheet::Sheet;
use cr_store::tables::{Leftovers, TableBook, TableRolls};
use cr_store::treasure::{Bundle, Rand, TreasureRules};
use cr_store::userdb::parts::{OTHER_TYPES, Parts};
use cr_store::userdb::traps::Traps;
use cr_store::userdb::{FieldKind, Origin, UserDb, Value};
use cr_win::text::plain;
use windows_reactor::*;

use crate::native::keys::Key;
use crate::native::{self, Owner};
use book::{BookPage, BookView};
use encounter::{Chooser, GroupDialog};
use help::{HelpPage, Opened, Shown};
use library::Pictures;
use parties::PartiesPage;
use portrait::Choosing;
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
    /// The keys of the Expanders opened on the Encounter page or the 2-20
    /// table editor.
    open_rows: Vec<String>,
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
    /// What follows saving the record on the record editor.
    record_then: Then,
    /// A record just saved, shown when the catalog has been read again.
    revealing: Option<Revealing>,
    /// What the record editor's picture fields show, by kind and what
    /// they keep, read once.
    record_pictures: RefCell<HashMap<String, (String, Option<EncodedImage>)>>,
    /// New in the other parts' section, while it asks for the type: the
    /// types by section id and name, and the one chosen.
    new_part: Option<(Vec<(&'static str, &'static str)>, usize)>,
    /// What the NPC that NPC Creation makes in template mode goes to.
    npc_for: Option<NpcFor>,
}

/// What the window shows.
#[derive(Clone)]
pub enum Page {
    Library,
    Edit(Box<Editor>),
    Create(Box<Wizard>),
    Import(Box<import::ImportPage>),
    /// Export…, or Add Items on the package editor.
    Export(Box<packages::ExportPage>),
    /// New Package… or Edit Package….
    Package(Box<PackageEditor>),
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
    /// A record of the user databases.
    Record(Box<RecordForm>),
    TreasureTable(Box<TableForm>),
    Allowed(Box<AllowedForm>),
    Template(Box<TemplateForm>),
}

/// What follows saving a record of the user databases.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Then {
    /// The library shows the record.
    #[default]
    Show,
    /// The trap New Trap made goes in the encounter, as the original's.
    AddTrap,
    /// The monster New Monster made goes in the encounter.
    AddMonster,
}

/// A record just saved, which the library shows once the catalog has
/// been read again: its editor stays until then, so that the library
/// comes back with the record selected rather than with the list as it
/// was, whose selection follows the keyboard focus to the record that
/// was edited.
pub struct Revealing {
    section: &'static str,
    key: String,
    notice: Notice,
    told: Option<(String, String)>,
}

/// The record an action on the user databases is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordTarget {
    section: String,
    name: String,
    then: Then,
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
    /// Delete on a record of the user databases, asking `title` and
    /// `text`.
    DeleteRecord {
        target: RecordTarget,
        title: String,
        text: String,
    },
    /// Restore on a standard treasure table.
    RestoreTable {
        name: String,
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
    /// Choose File… on Choose Portrait or Choose Picture.
    PortraitFile(Choosing),
    /// What is kept for the picture made from the user's, if one was
    /// chosen.
    PortraitAdded(Result<Option<String>, String>, Choosing),
    ClosePortraits,
    // Import.
    PackageChosen(Result<Option<Box<import::ImportPage>>, String>),
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
    /// An Expander of the Encounter page or the table editor opened or
    /// closed, by its key.
    RowOpen(String, bool),
    TableDeleted(Result<(), String>),
    /// Create Encounter on the details of a template or a 2-20 table.
    GenerateEntry,
    /// Group Encounter on Manage Parties.
    GroupEncounter,
    GroupPrint(bool),
    GroupCondensed(bool),
    GroupPreview(bool),
    GroupAnswered(bool),
    // The user databases.
    /// What an action on a record led to, and the record.
    RecordOpened(Result<Acted, String>, RecordTarget),
    Record(RecordInput),
    /// The answer to the record editor's prompt with this number.
    RecordAnswer(u64, Answer),
    RecordSaved(Result<Saved, String>),
    /// The record was deleted, with what to say, or why it was not.
    RecordDeleted(Result<String, String>, RecordTarget),
    /// Choose… on the record editor's picture field with this index.
    ChoosePicture(usize),
    /// The search over Choose Picture's icons.
    PictureSearch(String),
    /// The type chosen for New in the other parts' section, and Create
    /// (true) or Cancel on it.
    NewPartType(Option<usize>),
    NewPartAnswered(bool),
    TreasureTable(TreasureTableInput),
    TreasureTableAnswer(u64, Answer),
    TreasureTableSaved(Result<Saved, String>),
    /// The standard table with this name was restored, or why not.
    TableRestored(Result<String, String>, String),
    AllowedOpened(Result<Box<AllowedForm>, String>),
    Allowed(AllowedInput),
    AllowedAnswer(u64, Answer),
    AllowedSaved(Result<(), String>),
    /// New Trap on Add Trap, and New Monster on Add Monster.
    NewTrap,
    NewMonster,
    /// What New Trap or New Monster made, by its name, with the
    /// databases read again.
    NewTrapRead(Result<Arc<UserDb<Traps>>, String>, String),
    NewMonsterRead(Result<Arc<EncounterRules>, String>, String),
    Template(TemplateInput),
    TemplateAnswer(u64, Answer),
    TemplateSaved(Result<Saved, String>),
    /// What NPC Creation made in template mode, or why it made nothing.
    TemplateNpcMade(Result<Box<Npc>, String>),
    /// Import's check on an item, by index into the preview, or on every
    /// item of a large category, by its place among the categories.
    ImportCheck(usize, bool),
    ImportCategory(usize, bool),
    /// The parts, read for Name Conflict's questions.
    ImportParts(Result<Arc<UserDb<Parts>>, String>),
    /// The name typed in Name Conflict, and its Close or Help (true) on
    /// the dialog with this number.
    ConflictName(String),
    ConflictAnswered(u64, bool),
    /// What a Core Rules 2.0 installation hands over, none when it has
    /// nothing, or why it cannot.
    Core20Read(Result<Option<Option<Box<import::ImportPage>>>, String>),
    /// What Export or Add Items offers, and whether it adds to the
    /// package editor.
    ExportOffered(Result<Vec<Exportable>, String>, bool),
    ExportCheck(usize, bool),
    ExportAll(bool),
    ExportChosen,
    /// Where the package was exported, with the warnings, or why not.
    PackageExported(Result<Option<(PathBuf, Vec<String>)>, String>),
    PackageAdd,
    /// The editor's package with the items added, and the warnings.
    PackageAdded(Result<(Box<ExportFile>, Vec<String>), String>),
    PackageRemove(Box<Content>),
    PackageSave,
    PackageSaved(Result<Option<PathBuf>, String>),
    /// The package Edit Package… opened, or why not.
    PackageOpened(Result<Option<(PathBuf, Box<ExportFile>)>, String>),
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

    /// Makes `id` the library's section under the page shown, as the
    /// GNOME app shows the section of a record just saved.
    fn reveal(&mut self, id: &str) {
        if id != self.section {
            self.section = id.to_owned();
            self.filter.search.clear();
        }
    }

    /// Whether the open installation is the Expansion's.
    fn expansion(&self) -> bool {
        self.install.as_ref().is_some_and(|i| !i.is_core20())
    }

    /// Whether the page shown has changes that leaving it would lose.
    fn changed(&self) -> bool {
        match &self.page {
            Page::Edit(edit) => edit.changed(),
            Page::SheetOptions(options) => options.changed(),
            // Past its first page the wizard has the choices made.
            Page::Create(wizard) => wizard.depth() > 1,
            Page::Encounter(form) => form.changed(),
            Page::EncounterTable(editor) => editor.changed(),
            Page::Record(form) => form.changed(),
            Page::TreasureTable(form) => form.changed(),
            Page::Allowed(form) => form.changed(),
            Page::Template(form) => form.changed(),
            Page::Package(editor) => editor.changed,
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
            action::NEW_NPC => {
                self.npc_for = None;
                self.new_npc(false, context)
            }
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
            action::IMPORT_CORE20 => {
                let Some(install) = self.install.clone() else {
                    return;
                };
                self.dialog(
                    context,
                    move |owner| {
                        let title = tr("Import Core Rules 2.0 Installation");
                        match native::dialogs::open_folder(owner, title) {
                            Ok(Some(path)) => import::open_core20(&install, path).map(Some),
                            Ok(None) => Ok(None),
                            Err(e) => Err(e.message()),
                        }
                    },
                    Message::Core20Read,
                );
            }
            action::EXPORT => self.offer_exports(false, context),
            action::NEW_PACKAGE => {
                if let Some(install) = &self.install {
                    let expansion = !install.is_core20();
                    self.go(Page::Package(Box::new(PackageEditor::new(
                        ExportFile::new(expansion),
                        None,
                        expansion,
                    ))));
                }
            }
            action::EDIT_PACKAGE => {
                let expansion = self.expansion();
                self.dialog(
                    context,
                    move |owner| {
                        let filters = import::filters(expansion);
                        match native::dialogs::open_file(owner, tr("Edit Package"), &filters) {
                            Ok(Some(path)) => cr_app::packages::read(&path)
                                .map(|file| Some((path, Box::new(file)))),
                            Ok(None) => Ok(None),
                            Err(e) => Err(e.message()),
                        }
                    },
                    Message::PackageOpened,
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
            // The user databases: New on the section shown, the rest on
            // the record on the details.
            action::UDB_NEW => self.open_record(
                self.section.clone(),
                Origin::Standard,
                String::new(),
                Act::New,
                Then::Show,
                context,
            ),
            action::UDB_COPY => self.act_on_selected(Act::Copy, context),
            action::UDB_EDIT => self.act_on_selected(Act::Edit, context),
            action::UDB_DELETE => self.act_on_selected(Act::Delete, context),
            action::UDB_SUBRACE => self.act_on_selected(Act::NewSubrace, context),
            action::UDB_SCHOOLS => self.act_on_selected(Act::Schools, context),
            action::UDB_ALLOWED => {
                let race = self
                    .selected
                    .as_deref()
                    .and_then(cr_store::userdb::parse_key)
                    .map(|(_, _, name)| name.to_owned());
                if let Some(race) = race
                    && !self.busy
                {
                    self.busy = true;
                    self.with_install(context, move |install| {
                        Message::AllowedOpened(
                            AllowedForm::open(install, &race)
                                .map(Box::new)
                                .map_err(|e| e.to_string()),
                        )
                    });
                }
            }
            action::UDB_RESTORE => {
                if let Some((_, _, name)) = self
                    .selected
                    .as_deref()
                    .and_then(cr_store::userdb::parse_key)
                {
                    self.confirm = Some(Confirm::RestoreTable {
                        name: name.to_owned(),
                    });
                }
            }
            // Add Trap's Edit, Copy and Delete, on the trap chosen there.
            action::TRAP_EDIT => self.act_on_trap(Act::Edit, context),
            action::TRAP_COPY => self.act_on_trap(Act::Copy, context),
            action::TRAP_DELETE => self.act_on_trap(Act::Delete, context),
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
        // In template mode the NPC goes to the template, not to a file.
        if let Form::One(form) = &form
            && form.template
        {
            let form = form.clone();
            context.spawn_background(move |_| {
                let mut rand = rand.lock().unwrap_or_else(|e| e.into_inner());
                Message::TemplateNpcMade(form.generate(&mut rand).map(Box::new))
            });
            return;
        }
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
        self.open_rows.clear();
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
            Page::Export(_) | Page::Package(_) => topic::EXPORT,
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
            // A monster attack's dialog has a topic of its own.
            Page::Record(form) => form
                .dialog
                .as_ref()
                .and_then(|d| d.help())
                .unwrap_or_else(|| form.help()),
            Page::TreasureTable(form) => form.help(),
            Page::Allowed(form) => form.help(),
            Page::Template(form) => form.help(),
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

    /// Acts on the record `name` of `section` that comes from `origin`
    /// (New takes neither), reading the databases in the background;
    /// `then` is what follows saving it.
    fn open_record(
        &mut self,
        section: String,
        origin: Origin,
        name: String,
        act: Act,
        then: Then,
        context: &ComponentContext<Self>,
    ) {
        let Some(install) = self.install.clone() else {
            return;
        };
        if self.busy {
            return;
        }
        self.busy = true;
        // New in the other parts' section starts on the shown part's type,
        // and in the templates' on the shown template's monster type.
        let shown = self.selected.clone();
        let target = RecordTarget {
            section: section.clone(),
            name: name.clone(),
            then,
        };
        context.spawn_background(move |_| {
            let opened =
                record_actions::open(&install, &section, origin, &name, act, shown.as_deref());
            Message::RecordOpened(opened, target)
        });
    }

    /// Acts on the record on the details.
    fn act_on_selected(&mut self, act: Act, context: &ComponentContext<Self>) {
        let Some(key) = self.selected.clone() else {
            return;
        };
        if let Some((section, origin, name)) = cr_store::userdb::parse_key(&key) {
            let (section, name) = (section.to_owned(), name.to_owned());
            self.open_record(section, origin, name, act, Then::Show, context);
        }
    }

    /// Edit, Copy or Delete on the trap Add Trap chose, which closes for
    /// it as the GNOME app's does.
    fn act_on_trap(&mut self, act: Act, context: &ComponentContext<Self>) {
        let Page::Encounter(form) = &mut self.page else {
            return;
        };
        let Some((name, origin)) = form.picker.as_ref().and_then(|p| p.chosen_trap()) else {
            return;
        };
        form.picker = None;
        self.open_record("traps".to_owned(), origin, name, act, Then::Show, context);
    }

    /// New Trap or New Monster from the encounter's Add Trap or Add
    /// Monster, which closes for it.
    fn new_for_encounter(&mut self, section: &str, then: Then, context: &ComponentContext<Self>) {
        if let Page::Encounter(form) = &mut self.page {
            form.picker = None;
        }
        self.open_record(
            section.to_owned(),
            Origin::Custom,
            String::new(),
            Act::New,
            then,
            context,
        );
    }

    /// The databases changed: what was read of them is read again when
    /// it is needed.
    fn forget_rules(&mut self) {
        self.npc_rules = None;
        self.treasure_rules = None;
        self.table_rolls = None;
        self.encounter_rules = None;
        self.items = Arc::default();
    }

    /// Deletes the record `target`.
    fn delete_record(&self, target: RecordTarget, context: &ComponentContext<Self>) {
        self.with_install(context, move |install| {
            let deleted = record_actions::delete(install, &target.section, &target.name);
            Message::RecordDeleted(deleted, target)
        });
    }

    /// Does what the record editor asks of the app.
    fn record_effects(&mut self, context: &ComponentContext<Self>) {
        let Page::Record(form) = &mut self.page else {
            return;
        };
        let effects = form.take_effects();
        for effect in effects {
            match effect {
                RecordEffect::Save(job) => {
                    self.with_install(context, move |install| Message::RecordSaved(job(install)))
                }
                RecordEffect::Done {
                    section,
                    key,
                    name,
                    said,
                    told,
                } => {
                    let then = std::mem::take(&mut self.record_then);
                    self.forget_rules();
                    if then == Then::Show {
                        if let Page::Record(form) = &mut self.page {
                            form.saving = true;
                        }
                        self.revealing = Some(Revealing {
                            section,
                            key: key.clone(),
                            notice: Notice::success(said),
                            told,
                        });
                        self.load(Some(key), context);
                        return;
                    }
                    self.back();
                    match then {
                        Then::Show => {}
                        Then::AddTrap => self.with_install(context, move |install| {
                            let read = UserDb::<Traps>::load(install)
                                .map(Arc::new)
                                .map_err(|e| e.to_string());
                            Message::NewTrapRead(read, name)
                        }),
                        Then::AddMonster => self.with_install(context, move |install| {
                            let read = install
                                .encounter_rules()
                                .map(Arc::new)
                                .map_err(|e| e.to_string());
                            Message::NewMonsterRead(read, name)
                        }),
                    }
                    self.notice = Some(Notice::success(said));
                    if let Some((title, text)) = told {
                        self.confirm = Some(Confirm::Info { title, text });
                    }
                    self.load(Some(key), context);
                    return;
                }
            }
        }
    }

    /// Does what the treasure table editor asks of the app.
    fn treasure_table_effects(&mut self, context: &ComponentContext<Self>) {
        let Page::TreasureTable(form) = &mut self.page else {
            return;
        };
        let effects = form.take_effects();
        for effect in effects {
            match effect {
                TreasureTableEffect::Save { editing, table } => {
                    self.with_install(context, move |install| {
                        Message::TreasureTableSaved(cr_app::records::tables::save(
                            install, editing, *table,
                        ))
                    })
                }
                TreasureTableEffect::Done { key, said } => {
                    if let Page::TreasureTable(form) = &mut self.page {
                        form.saving = true;
                    }
                    self.forget_rules();
                    self.revealing = Some(Revealing {
                        section: cr_app::records::tables::SECTION,
                        key: key.clone(),
                        notice: Notice::success(said),
                        told: None,
                    });
                    self.load(Some(key), context);
                    return;
                }
            }
        }
    }

    /// Does what Allowed Classes asks of the app.
    fn allowed_effects(&mut self, context: &ComponentContext<Self>) {
        let Page::Allowed(form) = &mut self.page else {
            return;
        };
        for effect in form.take_effects() {
            match effect {
                AllowedEffect::Save { race, entries } => {
                    self.with_install(context, move |install| {
                        Message::AllowedSaved(cr_app::records::allowed::save(
                            install, &race, &entries,
                        ))
                    })
                }
                AllowedEffect::Done { said } => {
                    self.back();
                    self.forget_rules();
                    self.notice = Some(Notice::success(said));
                    self.load(None, context);
                    return;
                }
            }
        }
    }

    /// Does what the encounter template editor asks of the app.
    fn template_effects(&mut self, context: &ComponentContext<Self>) {
        let Page::Template(form) = &mut self.page else {
            return;
        };
        let effects = form.take_effects();
        for effect in effects {
            match effect {
                TemplateEffect::Npc(purpose) => {
                    self.npc_for = Some(purpose);
                    self.new_npc(false, context);
                    return;
                }
                TemplateEffect::Save { editing, template } => {
                    self.with_install(context, move |install| {
                        Message::TemplateSaved(cr_app::records::templates::save(
                            install, editing, *template,
                        ))
                    })
                }
                TemplateEffect::Done { key, said } => {
                    if let Page::Template(form) = &mut self.page {
                        form.saving = true;
                    }
                    self.forget_rules();
                    self.revealing = Some(Revealing {
                        section: cr_app::records::templates::SECTION,
                        key: key.clone(),
                        notice: Notice::success(said),
                        told: None,
                    });
                    self.load(Some(key), context);
                    return;
                }
            }
        }
    }

    /// Goes on with the names an import asks for: Name Conflict on the
    /// next part that needs one, or the import once every one has its
    /// name.
    fn rename_next(
        &mut self,
        mut renaming: Renaming,
        parts: Arc<UserDb<Parts>>,
        serial: u64,
        context: &ComponentContext<Self>,
    ) {
        let Page::Import(page) = &mut self.page else {
            return;
        };
        match renaming.next(&page.choice.preview, &parts) {
            Step::Ask { n, name } => {
                page.asking = Some(import::Asking {
                    renaming,
                    parts,
                    n,
                    name,
                    serial,
                })
            }
            Step::Import(names) => self.run_import(names, context),
        }
    }

    /// Imports the items chosen on the Import page, with `names` for the
    /// parts Name Conflict named.
    fn run_import(&mut self, names: HashMap<usize, String>, context: &ComponentContext<Self>) {
        let (Page::Import(page), Some(install)) = (&self.page, self.install.clone()) else {
            return;
        };
        self.busy = true;
        let (file, chosen) = (page.choice.file.clone(), page.choice.chosen());
        context.spawn_background(move |_| {
            Message::Imported(
                install
                    .import_named(&file, Some(&chosen), &names)
                    .map(Arc::new)
                    .map_err(|e| e.to_string()),
            )
        });
    }

    /// Export…, or (`adding`) Add Items on the package editor: the user's
    /// records and characters, read in the background.
    fn offer_exports(&mut self, adding: bool, context: &ComponentContext<Self>) {
        if self.busy || self.install.is_none() {
            return;
        }
        self.busy = true;
        self.with_install(context, move |install| {
            Message::ExportOffered(install.exportable().map_err(|e| e.to_string()), adding)
        });
    }

    /// Export's Export…: where to save the package, then the package; or
    /// Add Items' Add, into the package editor's.
    fn export_chosen(&mut self, context: &ComponentContext<Self>) {
        let Page::Export(page) = &self.page else {
            return;
        };
        let items = page.chooser.chosen();
        let Some(install) = self.install.clone() else {
            return;
        };
        if items.is_empty() || self.busy {
            return;
        }
        if page.adding {
            let Some(Page::Package(editor)) = self.history.last() else {
                return;
            };
            let mut file = editor.file.clone();
            self.busy = true;
            context.spawn_background(move |_| {
                Message::PackageAdded(
                    install
                        .add_to(&mut file, &items)
                        .map(|warnings| (Box::new(file), warnings))
                        .map_err(|e| e.to_string()),
                )
            });
            return;
        }
        let expansion = !install.is_core20();
        self.dialog(
            context,
            move |owner| {
                let Some(path) = ask_package_path(owner, expansion)? else {
                    return Ok(None);
                };
                let export = install.export(&items).map_err(|e| e.to_string())?;
                cr_app::packages::write(&path, &export.file)?;
                Ok(Some((path, export.warnings)))
            },
            Message::PackageExported,
        );
    }

    /// Save on the package editor: where it is, or where the Save dialog
    /// says for a new package.
    fn save_package(&mut self, context: &ComponentContext<Self>) {
        let Page::Package(editor) = &mut self.page else {
            return;
        };
        if editor.saving || self.busy {
            return;
        }
        editor.saving = true;
        editor.error = None;
        let (file, path, expansion) = (editor.file.clone(), editor.path.clone(), editor.expansion);
        let save = move |path: PathBuf| -> Result<Option<PathBuf>, String> {
            cr_app::packages::write(&path, &file)?;
            Ok(Some(path))
        };
        match path {
            Some(path) => {
                self.busy = true;
                context.spawn_background(move |_| Message::PackageSaved(save(path)));
            }
            None => self.dialog(
                context,
                move |owner| match ask_package_path(owner, expansion)? {
                    Some(path) => save(path),
                    None => Ok(None),
                },
                Message::PackageSaved,
            ),
        }
    }

    /// Keeps what Choose Portrait or Choose Picture chose: a character's
    /// portrait by its file, a record's picture or icon as it is kept.
    fn picked(
        &mut self,
        choosing: Choosing,
        stored: Option<String>,
        context: &ComponentContext<Self>,
    ) {
        match choosing {
            Choosing::Portrait => self.set_portrait(stored, context),
            Choosing::Picture { field, .. } | Choosing::Icon { field } => self.update(
                Message::Record(RecordInput::Set(
                    field,
                    Value::Text(stored.unwrap_or_default()),
                )),
                context,
            ),
        }
    }

    /// What the record editor's picture field of `kind` shows for what it
    /// keeps, `stored`, read once.
    fn record_picture(&self, kind: FieldKind, stored: &str) -> (String, Option<EncodedImage>) {
        let (picture, icon) = match kind {
            FieldKind::Picture(picture) => (picture, false),
            _ => (cr_store::pictures::Kind::Item, true),
        };
        let key = format!("{picture:?}\u{1}{icon}\u{1}{stored}");
        if let Some(found) = self.record_pictures.borrow().get(&key) {
            return found.clone();
        }
        let found = portrait::look(self.install.as_deref(), picture, icon, stored);
        self.record_pictures.borrow_mut().insert(key, found.clone());
        found
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

/// New in the other parts' section: the type of the new part ("Select
/// the Type" of IDD 139).
fn new_part_dialog(
    types: &[(&'static str, &'static str)],
    chosen: usize,
    context: &mut ViewContext<Shell>,
) -> View {
    let names: Vec<String> = types.iter().map(|(_, n)| (*n).to_owned()).collect();
    ContentDialog::new()
        .title(tr("New Part"))
        .primary_button_text(plain(tr("C_reate")))
        .close_button_text(plain(tr("_Cancel")))
        .is_open(true)
        .on_closed(
            context.callback(|result| {
                Message::NewPartAnswered(result == ContentDialogResult::Primary)
            }),
        )
        .content(
            StackPanel::new().spacing(12.0).children((
                crate::ui::body(tr("Select the type of the new part.")),
                ComboBox::new()
                    .automation_name(tr("New Part"))
                    .items_source(names)
                    .selected_index(Some(chosen))
                    .min_width(240.0)
                    .on_selection_changed(context.callback(Message::NewPartType)),
            )),
        )
}

/// The title and text of topic `number` of `file`, as a popup shows it.
/// Asks where to save a new package; the extension is added when the name
/// has none.
fn ask_package_path(owner: Owner, expansion: bool) -> Result<Option<PathBuf>, String> {
    let extension = cr_app::packages::extension(expansion);
    let filters = packages::save_filters(expansion);
    let chosen = native::dialogs::save_file(
        owner,
        &plain(tr("Save Package")),
        &format!("export.{extension}"),
        extension,
        &filters,
    )
    .map_err(|e| e.message())?;
    Ok(chosen.map(|mut path| {
        if path.extension().is_none() {
            path.set_extension(extension);
        }
        path
    }))
}

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
            open_rows: Vec::new(),
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
            record_then: Then::Show,
            revealing: None,
            record_pictures: RefCell::default(),
            new_part: None,
            npc_for: None,
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
                if let Some(revealing) = self
                    .revealing
                    .take_if(|r| key.as_deref() == Some(r.key.as_str()))
                {
                    if matches!(
                        self.page,
                        Page::Record(_) | Page::TreasureTable(_) | Page::Template(_)
                    ) {
                        self.back();
                    }
                    self.reveal(revealing.section);
                    self.notice = Some(revealing.notice);
                    if let Some((title, text)) = revealing.told {
                        self.confirm = Some(Confirm::Info { title, text });
                    }
                }
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
                (Some(Confirm::DeleteRecord { target, .. }), ContentDialogResult::Primary) => {
                    self.delete_record(target, context)
                }
                (Some(Confirm::RestoreTable { name }), ContentDialogResult::Primary) => self
                    .with_install(context, move |install| {
                        let restored = cr_app::records::tables::restore(install, &name);
                        Message::TableRestored(restored, name)
                    }),
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
                    Ok(Some(_)) => self.notice = Some(Notice::success(tr("Saved as PDF"))),
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
                    self.portraits = Some(portrait::Chooser::new(Choosing::Portrait));
                    self.with_install(context, move |install| {
                        Message::PortraitsListed(portrait::list(install, Choosing::Portrait, &race))
                    });
                }
            }
            Message::PortraitsListed(tiles) => {
                if let Some(chooser) = &mut self.portraits {
                    chooser.tiles = Some(tiles);
                }
            }
            Message::PortraitChosen(stored) => {
                // The Portrait row's None chooses with no dialog open.
                let choosing = self
                    .portraits
                    .take()
                    .map_or(Choosing::Portrait, |c| c.choosing);
                self.picked(choosing, stored, context);
            }
            Message::PortraitFile(choosing) => {
                self.portraits = None;
                let Some(install) = self.install.clone() else {
                    return;
                };
                self.dialog(
                    context,
                    move |owner| {
                        let filters = portrait::filters();
                        match native::dialogs::open_file(owner, tr("Choose a Picture"), &filters) {
                            Ok(Some(path)) => portrait::add(&install, choosing, &path).map(Some),
                            Ok(None) => Ok(None),
                            Err(e) => Err(e.message()),
                        }
                    },
                    move |added| Message::PortraitAdded(added, choosing),
                );
            }
            Message::PortraitAdded(added, choosing) => {
                self.busy = false;
                match added {
                    Ok(Some(stored)) => self.picked(choosing, Some(stored), context),
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
                let Page::Import(page) = &self.page else {
                    return;
                };
                if self.busy || !page.choice.can_import() {
                    return;
                }
                if import::renames(&page.choice) {
                    self.busy = true;
                    self.with_install(context, |install| {
                        Message::ImportParts(
                            UserDb::<Parts>::load(install)
                                .map(Arc::new)
                                .map_err(|e| e.to_string()),
                        )
                    });
                } else {
                    self.run_import(HashMap::new(), context);
                }
            }
            Message::ImportCheck(n, on) => {
                if let Page::Import(page) = &mut self.page {
                    page.choice.set(n, on);
                }
            }
            Message::ImportCategory(c, on) => {
                if let Page::Import(page) = &mut self.page
                    && let Some((_, items)) = page.choice.categories().get(c)
                {
                    page.choice.set_many(items, on);
                }
            }
            Message::ImportParts(Ok(parts)) => {
                self.busy = false;
                if let Page::Import(page) = &self.page {
                    let renaming = Renaming::new(page.choice.chosen());
                    self.rename_next(renaming, parts, 0, context);
                }
            }
            Message::ImportParts(Err(e)) => {
                self.busy = false;
                self.show_error(tr("Import Failed"), e);
            }
            Message::ConflictName(name) => {
                if let Page::Import(page) = &mut self.page
                    && let Some(asking) = &mut page.asking
                {
                    asking.name = name;
                }
            }
            Message::ConflictAnswered(serial, help) => {
                let Page::Import(page) = &mut self.page else {
                    return;
                };
                let Some(asking) = page.asking.as_mut().filter(|a| a.serial == serial) else {
                    return;
                };
                // Asked again after Help, and while a part has the name.
                if help || asking.renaming.taken(&asking.name, &asking.parts) {
                    asking.serial += 1;
                    if help {
                        self.open_help(cr_app::help::program(topic::NAME_CONFLICT), false, context);
                    }
                    return;
                }
                let Some(mut asking) = page.asking.take() else {
                    return;
                };
                asking.renaming.named(asking.n, asking.name.clone());
                self.rename_next(asking.renaming, asking.parts, asking.serial + 1, context);
            }
            Message::Imported(result) => {
                self.busy = false;
                match result {
                    Ok(report) => {
                        // Imported races and classes change what the NPC
                        // generator offers, imported items the treasure.
                        self.forget_rules();
                        self.history.clear();
                        self.page = Page::Library;
                        self.notice = Some(import::report(&report));
                        self.load(None, context);
                    }
                    Err(e) => self.show_error(tr("Import Failed"), e),
                }
            }
            Message::Core20Read(read) => {
                self.busy = false;
                match read {
                    Ok(Some(Some(page))) => self.go(Page::Import(page)),
                    Ok(Some(None)) => {
                        self.confirm = Some(Confirm::Info {
                            title: tr("Nothing to Import").to_owned(),
                            text: tr(
                                "This Core Rules 2.0 installation has no characters or records of its own.",
                            )
                            .to_owned(),
                        })
                    }
                    Ok(None) => {}
                    Err(e) => self.show_error(tr("Cannot Import"), e),
                }
            }
            Message::ExportOffered(offered, adding) => {
                self.busy = false;
                match offered {
                    Ok(items) => self.go(Page::Export(Box::new(packages::ExportPage {
                        chooser: ItemChooser::new(items),
                        adding,
                    }))),
                    Err(e) => self.show_error(tr("Export Failed"), e),
                }
            }
            Message::ExportCheck(i, on) => {
                if let Page::Export(page) = &mut self.page {
                    page.chooser.set(i, on);
                }
            }
            Message::ExportAll(on) => {
                if let Page::Export(page) = &mut self.page {
                    page.chooser.set_all(on);
                }
            }
            Message::ExportChosen => self.export_chosen(context),
            Message::PackageExported(exported) => {
                self.busy = false;
                match exported {
                    Ok(Some((path, warnings))) => {
                        self.back();
                        let said = tr!("Exported to {name}", name = file_name(&path));
                        self.notice = Some(Notice::new(
                            if warnings.is_empty() {
                                InfoBarSeverity::Success
                            } else {
                                InfoBarSeverity::Warning
                            },
                            said,
                            warnings.join("\n"),
                        ));
                    }
                    Ok(None) => {}
                    Err(e) => self.show_error(tr("Export Failed"), e),
                }
            }
            Message::PackageAdd => {
                if let Page::Package(editor) = &self.page
                    && editor.same_layout()
                {
                    self.offer_exports(true, context);
                }
            }
            Message::PackageAdded(added) => {
                self.busy = false;
                match added {
                    Ok((file, warnings)) => {
                        self.back();
                        if let Page::Package(editor) = &mut self.page {
                            editor.file = *file;
                            editor.added(warnings);
                        }
                    }
                    Err(e) => self.show_error(tr("Items Not Added"), e),
                }
            }
            Message::PackageRemove(item) => {
                if let Page::Package(editor) = &mut self.page {
                    editor.remove(&item);
                }
            }
            Message::PackageSave => self.save_package(context),
            Message::PackageSaved(saved) => {
                self.busy = false;
                let Page::Package(editor) = &mut self.page else {
                    return;
                };
                match saved {
                    Ok(Some(path)) => {
                        let (said, warnings) = editor.saved(path);
                        self.back();
                        self.notice = Some(Notice::new(
                            if warnings.is_empty() {
                                InfoBarSeverity::Success
                            } else {
                                InfoBarSeverity::Warning
                            },
                            said,
                            warnings.join("\n"),
                        ));
                    }
                    Ok(None) => editor.saving = false,
                    Err(e) => {
                        editor.saving = false;
                        editor.error = Some(e);
                    }
                }
            }
            Message::PackageOpened(opened) => {
                self.busy = false;
                match opened {
                    // 2.0 content is only imported, converted to the
                    // Expansion's layout on the way in.
                    Ok(Some((path, file))) if file.is_expansion() => {
                        let expansion = self.expansion();
                        self.go(Page::Package(Box::new(PackageEditor::new(
                            *file,
                            Some(path),
                            expansion,
                        ))));
                    }
                    Ok(Some(_)) => {
                        let (title, text) = cr_app::packages::core20_only();
                        self.confirm = Some(Confirm::Info {
                            title: title.to_owned(),
                            text: text.to_owned(),
                        });
                    }
                    Ok(None) => {}
                    Err(e) => self.show_error(tr("Not a Package"), e),
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
            Message::RowOpen(key, open) => {
                self.open_rows.retain(|k| *k != key);
                if open {
                    self.open_rows.push(key);
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
                            Page::Npc(Box::new(NpcForm::new(rules, self.npc_for.is_some())))
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
                self.reveal("treasure");
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
                            EncounterOpened::Table(editor) => {
                                self.open_rows.clear();
                                self.go(Page::EncounterTable(editor))
                            }
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
                self.reveal("encounters");
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
                    self.show_section("tables".to_owned());
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
            Message::RecordOpened(opened, target) => {
                self.busy = false;
                match opened {
                    Ok(Acted::Form(form)) => {
                        self.record_then = target.then;
                        self.record_pictures.borrow_mut().clear();
                        self.go(Page::Record(form));
                    }
                    Ok(Acted::Table(form)) => self.go(Page::TreasureTable(form)),
                    Ok(Acted::Template(form)) => self.go(Page::Template(form)),
                    Ok(Acted::Refused(why)) => {
                        self.notice = Some(Notice::new(InfoBarSeverity::Warning, why, ""))
                    }
                    Ok(Acted::Delete(Some((title, text)))) => {
                        self.confirm = Some(Confirm::DeleteRecord {
                            target,
                            title,
                            text,
                        })
                    }
                    // Some kinds are deleted at once, as in the original.
                    Ok(Acted::Delete(None)) => self.delete_record(target, context),
                    Ok(Acted::ChooseType { types, chosen }) => {
                        self.new_part = Some((types, chosen))
                    }
                    Ok(Acted::Nothing) => {}
                    Err(e) => self.show_error(&e, ""),
                }
            }
            Message::Record(change) => {
                if let Page::Record(form) = &mut self.page {
                    form.input(change);
                    self.record_effects(context);
                }
            }
            Message::RecordAnswer(serial, answer) => {
                if let Page::Record(form) = &mut self.page
                    && form.prompt_serial() == serial
                {
                    form.answer(answer);
                    self.record_effects(context);
                }
            }
            Message::RecordSaved(saved) => {
                if let Page::Record(form) = &mut self.page {
                    form.saved(saved);
                    self.record_effects(context);
                }
            }
            Message::RecordDeleted(Ok(said), target) => {
                let key = cr_store::userdb::key(&target.section, Origin::Custom, &target.name);
                if self.selected.as_deref() == Some(key.as_str()) {
                    self.select(None);
                }
                self.forget_rules();
                self.notice = Some(Notice::success(said));
                self.load(None, context);
            }
            Message::RecordDeleted(Err(e), _) => self.show_error(&plain(tr("_Delete")), e),
            Message::ChoosePicture(field) => {
                let Page::Record(form) = &self.page else {
                    return;
                };
                let choosing = match form.field(field).kind {
                    FieldKind::Picture(kind) => Choosing::Picture { field, kind },
                    FieldKind::Icon => Choosing::Icon { field },
                    _ => return,
                };
                if self.portraits.is_none() {
                    self.portraits = Some(portrait::Chooser::new(choosing));
                    self.with_install(context, move |install| {
                        Message::PortraitsListed(portrait::list(install, choosing, ""))
                    });
                }
            }
            Message::PictureSearch(search) => {
                if let Some(chooser) = &mut self.portraits {
                    chooser.search = search;
                }
            }
            Message::NewPartType(Some(index)) => {
                if let Some((_, chosen)) = &mut self.new_part {
                    *chosen = index;
                }
            }
            Message::NewPartType(None) => {}
            Message::NewPartAnswered(yes) => {
                if let Some((types, chosen)) = self.new_part.take()
                    && yes
                    && let Some((id, _)) = types.get(chosen)
                {
                    self.open_record(
                        (*id).to_owned(),
                        Origin::Standard,
                        String::new(),
                        Act::New,
                        Then::Show,
                        context,
                    );
                }
            }
            Message::TreasureTable(change) => {
                if let Page::TreasureTable(form) = &mut self.page {
                    form.input(change);
                    self.treasure_table_effects(context);
                }
            }
            Message::TreasureTableAnswer(serial, answer) => {
                if let Page::TreasureTable(form) = &mut self.page
                    && form.prompt_serial() == serial
                {
                    form.answer(answer);
                    self.treasure_table_effects(context);
                }
            }
            Message::TreasureTableSaved(saved) => {
                if let Page::TreasureTable(form) = &mut self.page {
                    form.saved(saved);
                    self.treasure_table_effects(context);
                }
            }
            Message::Template(change) => {
                if let Page::Template(form) = &mut self.page {
                    form.input(change);
                    self.template_effects(context);
                }
            }
            Message::TemplateAnswer(serial, answer) => {
                if let Page::Template(form) = &mut self.page
                    && form.prompt_serial() == serial
                {
                    form.answer(answer);
                    self.template_effects(context);
                }
            }
            Message::TemplateSaved(saved) => {
                if let Page::Template(form) = &mut self.page {
                    form.saved(saved);
                    self.template_effects(context);
                }
            }
            Message::TemplateNpcMade(Ok(npc)) => {
                self.busy = false;
                let purpose = self.npc_for.take().unwrap_or(NpcFor::Line);
                self.back();
                if let Page::Template(form) = &mut self.page {
                    form.npc_made(purpose, template_npc(&npc));
                }
                if let Some(text) = cr_app::npc::messages([npc.as_ref()]) {
                    self.confirm = Some(Confirm::Info {
                        title: tr("NPC Generator").to_owned(),
                        text,
                    });
                }
            }
            Message::TemplateNpcMade(Err(e)) => {
                self.busy = false;
                if let Page::Npc(form) = &mut self.page {
                    form.error = Some(e);
                }
            }
            Message::TableRestored(Ok(said), name) => {
                self.forget_rules();
                self.notice = Some(Notice::success(said));
                self.load(Some(cr_app::records::tables::restored_key(&name)), context);
            }
            Message::TableRestored(Err(e), _) => self.show_error(&plain(tr("_Restore")), e),
            Message::AllowedOpened(opened) => {
                self.busy = false;
                match opened {
                    Ok(form) => self.go(Page::Allowed(form)),
                    Err(e) => self.show_error(&plain(tr("_Allowed Classes…")), e),
                }
            }
            Message::Allowed(change) => {
                if let Page::Allowed(form) = &mut self.page {
                    form.input(change);
                    self.allowed_effects(context);
                }
            }
            Message::AllowedAnswer(serial, answer) => {
                if let Page::Allowed(form) = &mut self.page
                    && form.prompt_serial() == serial
                {
                    form.answer(answer);
                    self.allowed_effects(context);
                }
            }
            Message::AllowedSaved(saved) => {
                if let Page::Allowed(form) = &mut self.page {
                    form.saved(saved);
                    self.allowed_effects(context);
                }
            }
            Message::NewTrap => self.new_for_encounter("traps", Then::AddTrap, context),
            Message::NewMonster => self.new_for_encounter("monsters", Then::AddMonster, context),
            Message::NewTrapRead(read, name) => match read {
                Ok(db) => {
                    if let Page::Encounter(form) = &mut self.page {
                        form.add_new_trap(&db, &name);
                        self.encounter_effects(context);
                    }
                }
                Err(e) => self.show_error(tr("Add Trap"), e),
            },
            Message::NewMonsterRead(read, name) => match read {
                Ok(rules) => {
                    self.encounter_rules = Some(rules.clone());
                    if let Page::Encounter(form) = &mut self.page {
                        let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
                        form.add_new_monster(rules, &name, &mut rand);
                        drop(rand);
                        self.encounter_effects(context);
                    }
                }
                Err(e) => self.show_error(tr("Add Monster"), e),
            },
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
            Page::Import(page) => import::view(page, self, context),
            Page::Export(page) => packages::export_view(page, self.busy, context),
            Page::Package(editor) => packages::editor_view(editor, self.busy, context),
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
            Page::Encounter(form) => encounter::view(form, &self.open_rows, self.busy, context),
            Page::EncounterTable(editor) => {
                encounter::table_view(editor, &self.open_rows, self.busy, context)
            }
            Page::Record(form) => {
                let picture = |kind: FieldKind, stored: &str| self.record_picture(kind, stored);
                let page = record::view(form, &picture, context);
                self.with_portraits(page, context)
            }
            Page::TreasureTable(form) => treasure_tables::view(form, context),
            Page::Allowed(form) => allowed::view(form, context),
            Page::Template(form) => templates::view(form, context),
        };
        // New in the other parts' section asks for the type over the
        // library.
        let content = match &self.new_part {
            Some((types, chosen)) => {
                Grid::new().children((content, new_part_dialog(types, *chosen, context)))
            }
            None => content,
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
                    Page::Encounter(_) => tr("The encounter has changes that are not saved."),
                    Page::EncounterTable(_) | Page::TreasureTable(_) => {
                        tr("The table has changes that are not saved.")
                    }
                    Page::Record(_) | Page::Template(_) => {
                        tr("The record has changes that are not saved.")
                    }
                    Page::Package(_) => tr("The package has changes that are not saved."),
                    Page::Allowed(_) => {
                        tr("The allowed classes have changes that are not saved.")
                    }
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
            Some(Confirm::DeleteRecord { title, text, .. }) => {
                (title.clone(), text.clone(), plain(tr("_Delete")))
            }
            Some(Confirm::RestoreTable { .. }) => {
                let (title, text) = cr_app::records::tables::restore_question();
                (title, text, plain(tr("_Restore")))
            }
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
