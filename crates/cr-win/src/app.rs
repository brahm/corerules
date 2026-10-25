//! The window: a title bar over a NavigationView with the characters and
//! Settings, as Windows 11 apps are laid out. One component holds the
//! state and handles every message; the pages are drawn by the functions
//! of the modules below, from that state.

mod characters;
mod edit;
mod import;
mod new_character;
mod settings;

use std::path::PathBuf;
use std::sync::Arc;

use cr_formats::export::ExportFile;
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::catalog::Entry;
use cr_store::export::{Content, ImportReport};
use cr_win::text::plain;
use windows_reactor::*;

use crate::native::{self, Owner};
pub use edit::Edit;

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

/// The navigation's tags.
const CHARACTERS: &str = "characters";
const SETTINGS: &str = "Settings";

pub struct Shell {
    install: Option<Arc<Installation>>,
    /// Whether the installation takes changes: an Expansion installation
    /// the user may write to (Core Rules 2.0 is opened read-only).
    can_write: bool,
    /// The characters, once read, or why they could not be.
    characters: Option<Result<Vec<Entry>, String>>,
    search: String,
    /// The selected character's file, and its portrait.
    selected: Option<String>,
    portrait: Option<EncodedImage>,
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
    Characters,
    Edit(Box<Edit>),
    NewCharacter {
        skills_and_powers: bool,
        setting: usize,
    },
    Import(Arc<Package>),
    Settings,
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
}

/// A question a ContentDialog asks before something that cannot be
/// undone.
#[derive(Clone, PartialEq)]
pub enum Confirm {
    Delete { file: String, name: String },
    Discard,
}

#[derive(Clone)]
pub enum Message {
    Loaded(Result<Vec<Entry>, String>),
    Navigate(Option<String>),
    Back,
    TogglePane,
    PaneOpenChanged(bool),
    Search(String),
    Select(Option<usize>),
    CloseNotice,
    OpenFolder,
    FolderChosen(Result<Option<PathBuf>, String>),
    // The characters' commands.
    NewCharacter,
    EditCharacter,
    EditLoaded(Result<Box<Edit>, String>),
    Edit(edit::Field),
    Save,
    Saved(Result<String, String>),
    AskDelete,
    Answered(ContentDialogResult),
    Deleted(Result<String, String>),
    Print,
    Printed(Result<bool, String>),
    // New Character.
    Rules(Option<usize>),
    Setting(Option<usize>),
    Next,
    // Import.
    OpenPackage,
    PackageChosen(Result<Option<Arc<Package>>, String>),
    Import,
    Imported(Result<Arc<ImportReport>, String>),
    // Settings.
    Language(Option<usize>),
}

impl Shell {
    fn load(&mut self, context: &ComponentContext<Self>) {
        self.characters = None;
        if let Some(install) = self.install.clone() {
            context.spawn_background(move |_| Message::Loaded(install.character_entries()));
        }
    }

    /// The characters the search lets through.
    fn visible(&self) -> Vec<&Entry> {
        match &self.characters {
            Some(Ok(entries)) => entries
                .iter()
                .filter(|e| cr_win::text::matches(&self.search, &e.title, &e.subtitle))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn selected_entry(&self) -> Option<&Entry> {
        let file = self.selected.as_deref()?;
        match &self.characters {
            Some(Ok(entries)) => entries.iter().find(|e| e.key == file),
            _ => None,
        }
    }

    fn select(&mut self, file: Option<String>) {
        self.selected = file;
        self.portrait = self
            .selected_entry()
            .and_then(|e| e.picture.as_deref())
            .and_then(characters::portrait);
    }

    fn go(&mut self, page: Page) {
        self.history.push(std::mem::replace(&mut self.page, page));
        self.notice = None;
    }

    fn back(&mut self) {
        self.page = self.history.pop().unwrap_or(Page::Characters);
        self.notice = None;
    }

    fn show_error(&mut self, title: &str, error: impl Into<String>) {
        self.notice = Some(Notice::new(InfoBarSeverity::Error, title, error));
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
}

impl Component for Shell {
    type Input = Option<PathBuf>;
    type Message = Message;

    fn create(file: &Option<PathBuf>, context: &ComponentContext<Self>) -> Self {
        let install = Installation::discover().map(Arc::new);
        let mut shell = Self {
            can_write: install.as_deref().is_some_and(can_write),
            install,
            characters: None,
            search: String::new(),
            selected: None,
            portrait: None,
            page: Page::Characters,
            history: Vec::new(),
            notice: None,
            confirm: None,
            pane_open: true,
            busy: false,
        };
        shell.load(context);
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
            Message::Loaded(characters) => {
                let first = match &characters {
                    Ok(entries)
                        if !entries
                            .iter()
                            .any(|e| Some(&e.key) == self.selected.as_ref()) =>
                    {
                        Some(entries.first().map(|e| e.key.clone()))
                    }
                    _ => None,
                };
                self.characters = Some(characters);
                let selected = first.unwrap_or_else(|| self.selected.clone());
                self.select(selected);
            }
            Message::Navigate(Some(tag)) => match tag.as_str() {
                CHARACTERS if matches!(&self.page, Page::Edit(edit) if edit.changed()) => {
                    self.confirm = Some(Confirm::Discard);
                }
                CHARACTERS if !matches!(self.page, Page::Characters) => {
                    self.history.clear();
                    self.page = Page::Characters;
                }
                SETTINGS if !matches!(self.page, Page::Settings) => self.go(Page::Settings),
                _ => {}
            },
            Message::Navigate(None) => {}
            Message::Back => match &self.page {
                Page::Edit(edit) if edit.changed() => self.confirm = Some(Confirm::Discard),
                _ => self.back(),
            },
            Message::TogglePane => self.pane_open = !self.pane_open,
            Message::PaneOpenChanged(open) => self.pane_open = open,
            Message::Search(text) => self.search = text,
            Message::Select(index) => {
                if let Some(file) = index.and_then(|i| self.visible().get(i).map(|e| e.key.clone()))
                {
                    self.select(Some(file));
                }
            }
            Message::CloseNotice => self.notice = None,
            Message::OpenFolder => self.dialog(
                context,
                |owner| native::dialogs::open_folder(owner, &plain(tr("_Open Folder…"))),
                |r| Message::FolderChosen(r.map_err(|e| e.message())),
            ),
            Message::FolderChosen(chosen) => {
                self.busy = false;
                match chosen {
                    Ok(Some(path)) => match Installation::open(&path) {
                        Ok(install) => {
                            self.can_write = can_write(&install);
                            self.install = Some(Arc::new(install));
                            self.select(None);
                            self.notice = None;
                            self.load(context);
                        }
                        Err(e) => self.show_error(tr("No Core Rules Installation"), e.to_string()),
                    },
                    Ok(None) => {}
                    Err(e) => self.show_error(tr("No Core Rules Installation"), e),
                }
            }
            Message::NewCharacter => self.go(Page::NewCharacter {
                skills_and_powers: false,
                setting: 0,
            }),
            Message::EditCharacter => {
                if let (Some(install), Some(file)) = (self.install.clone(), self.selected.clone()) {
                    context.spawn_background(move |_| {
                        Message::EditLoaded(Edit::load(&install, &file).map(Box::new))
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
                                .map(|()| details.name.trim().to_owned())
                                .map_err(|e| e.to_string()),
                        )
                    });
                }
            }
            Message::Saved(Ok(name)) => {
                self.back();
                self.notice = Some(Notice::new(
                    InfoBarSeverity::Success,
                    tr!("Saved {name}", name),
                    "",
                ));
                self.load(context);
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
            Message::Answered(answer) => match (self.confirm.take(), answer) {
                (Some(Confirm::Delete { file, .. }), ContentDialogResult::Primary) => {
                    if let Some(install) = self.install.clone() {
                        context.spawn_background(move |_| {
                            // Without the GTK app's Undo, the kept file
                            // goes at once, as when its toast closes.
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
                }
                (Some(Confirm::Discard), ContentDialogResult::Primary) => self.back(),
                _ => {}
            },
            Message::Deleted(Ok(name)) => {
                self.select(None);
                self.notice = Some(Notice::new(
                    InfoBarSeverity::Success,
                    tr!("“{name}” deleted", name),
                    "",
                ));
                self.load(context);
            }
            Message::Deleted(Err(e)) => self.show_error(tr("Delete Character"), e),
            Message::Print => {
                let (Some(install), Some(file)) = (self.install.clone(), self.selected.clone())
                else {
                    return;
                };
                self.dialog(
                    context,
                    move |owner| {
                        let sheet = install
                            .character(&file)
                            .and_then(|c| install.sheet(&c))
                            .map_err(|e| e.to_string())?;
                        native::print::print_sheet(owner, &sheet).map_err(|e| e.message())
                    },
                    Message::Printed,
                );
            }
            Message::Printed(result) => {
                self.busy = false;
                match result {
                    Ok(true) => {
                        self.notice = Some(Notice::new(
                            InfoBarSeverity::Success,
                            tr("Sent to the printer"),
                            "",
                        ))
                    }
                    Ok(false) => {}
                    Err(e) => self.show_error(&tr!("Could not print: {error}", error = e), ""),
                }
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
            Message::OpenPackage => {
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
                        self.page = Page::Characters;
                        self.notice = Some(import::report(&report));
                        self.load(context);
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
            Page::Characters => characters::view(self, context),
            Page::Edit(edit) => edit::view(edit, context),
            Page::NewCharacter {
                skills_and_powers,
                setting,
            } => new_character::view(self, *skills_and_powers, *setting, context),
            Page::Import(package) => import::view(self, package, context),
            Page::Settings => settings::view(self, context),
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
            _ => CHARACTERS,
        };
        let menu = vec![KeyedView::new(
            CHARACTERS,
            NavigationViewItem::new()
                .tag(CHARACTERS)
                .is_selected(selected_tag == CHARACTERS)
                .selects_on_invoked(true)
                .slots([
                    SlotView::new(NavigationViewItemSlot::Content, plain(tr("Characters"))),
                    SlotView::new(
                        NavigationViewItemSlot::Icon,
                        SymbolIcon::new().symbol(Symbol::People),
                    ),
                ]),
        )];
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
        Grid::new()
            .rows([GridLength::Auto, GridLength::STAR])
            .children((title_bar, navigation))
    }
}

impl Shell {
    /// The ContentDialog of the question being asked, closed when none is.
    fn confirm_dialog(&self, context: &mut ViewContext<Self>) -> View {
        let (title, text, action) = match &self.confirm {
            Some(Confirm::Delete { name, .. }) => (
                tr!("Delete {name}?", name),
                tr("The character leaves the character list and its file is removed.").to_owned(),
                plain(tr("_Delete")),
            ),
            Some(Confirm::Discard) => (
                plain(tr("Discard Changes?")),
                tr("The character has changes that are not saved.").to_owned(),
                plain(tr("_Discard")),
            ),
            None => (String::new(), String::new(), String::new()),
        };
        ContentDialog::new()
            .title(title)
            .primary_button_text(action)
            .close_button_text(plain(tr("_Cancel")))
            .is_open(self.confirm.is_some())
            .on_closed(context.callback(Message::Answered))
            .content(ui_text(text))
    }
}

fn ui_text(text: String) -> View {
    crate::ui::body(text).into()
}
