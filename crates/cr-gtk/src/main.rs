//! The GNOME app: browses the data of a Core Rules installation in three
//! panes (sections, entries, details) that collapse on narrow windows.

mod abilities;
mod books;
mod bundle;
mod combat;
mod create;
mod encounter;
mod enter;
mod extract;
mod help;
mod kit_magic;
mod lists;
mod nonweapon;
mod npc;
mod party;
mod picture;
mod prefs;
mod print;
mod sheet;
mod sheet_options;
mod skills;
mod tables;
mod transfer;
mod treasure;
mod userdb;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use adw::prelude::*;
use cr_app::action;
use cr_app::editor::{
    slots_fit, slots_text, social_class_shown, social_class_stored, spell_level_text,
};
use cr_app::library::{self, Commands};
use cr_app::records;
use cr_formats::character::Character;
use cr_i18n::{tr, tr_noop, trn};
use cr_store::Installation;
use cr_store::advance::{AdvanceRules, Advancement, adjust_psionic_points, psionic_total};
use cr_store::catalog::{ENCOUNTER_KEY, Entry, Section, TABLE_KEY, TREASURE_KEY};
use cr_store::choices::{Caster, caster};
use cr_store::dual::ClassTable;
use cr_store::edit::{ABILITY_RANGE, CharacterDetails};
use cr_store::help::topic;
use cr_store::learn::{LearnRules, Learner};
use cr_store::lists::{
    BUNDLE_NAMES, COINS, CharacterLists, ITEM_TYPES, Item, MONEY, Memorized, Spellbook,
};
use cr_store::npc::NpcRules;
use cr_store::pictures::{ICON_SIZE, ItemIcons};
use cr_store::purse;
use cr_store::treasure::Rand;
use cr_store::userdb::parts::{OTHER_SECTION, OTHER_TYPES, other_type};
use gtk::{gio, glib};
use lists::{ListEditor, RowList, RowSpec, pick};

const APP_ID: &str = "io.github.brahm.CoreRules";

/// The actions that make or change content, off for Core Rules 2.0.
const WRITE_ACTIONS: [&str; 21] = [
    "new-character",
    "enter-character",
    "new-npc",
    "new-npc-group",
    "roll-treasure",
    "new-encounter",
    "new-table",
    "export",
    "import",
    "import-core20",
    "new-package",
    "edit-package",
    "manage-parties",
    "udb-new",
    "udb-copy",
    "udb-edit",
    "udb-delete",
    "udb-subrace",
    "udb-schools",
    "udb-allowed",
    "udb-restore",
];

fn main() -> glib::ExitCode {
    prefs::apply_language();
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| add_icons());
    app.connect_activate(|app| {
        // The icon is installed under the application id; X11 window
        // managers read it from the window.
        gtk::Window::set_default_icon_name(APP_ID);
        let window = Window::new(app);
        let start = std::env::args_os().nth(1).map(PathBuf::from);
        match start {
            Some(dir) => window.open(dir),
            None => match Installation::discover() {
                Some(install) => window.show_installation(install),
                None => window.show_missing(None),
            },
        }
        window.window.present();
        if std::env::var_os("CORERULES_PRINT_TEXTS").is_some() {
            let window = window.window.clone();
            glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                print_texts(window.upcast_ref());
            });
        }
    });
    // The optional folder argument is handled above, not by GApplication.
    app.run_with_args(&std::env::args().take(1).collect::<Vec<_>>())
}

/// Prints the text of every label on screen under `widget`, one per line
/// after `text` and a tab, with line breaks written `\n`, so the package
/// tests can check the interface's language (`CORERULES_PRINT_TEXTS=1`, see
/// `packaging/smoke-test.sh`).
fn print_texts(widget: &gtk::Widget) {
    if !widget.is_mapped() {
        return;
    }
    if let Some(label) = widget.downcast_ref::<gtk::Label>() {
        let text = label.label();
        if !text.is_empty() {
            println!("text\t{}", text.replace('\n', "\\n"));
        }
    }
    let mut child = widget.first_child();
    while let Some(c) = child {
        print_texts(&c);
        child = c.next_sibling();
    }
}

/// Symbolic icons the icon theme lacks, by name.
const ICONS: [(&str, &str); 2] = [
    (
        "corerules-sword-symbolic",
        include_str!("../../../data/icons/symbolic/corerules-sword-symbolic.svg"),
    ),
    (
        "corerules-help-symbolic",
        include_str!("../../../data/icons/symbolic/corerules-help-symbolic.svg"),
    ),
];

/// Makes [`ICONS`] available to the icon theme. GTK finds an unthemed icon
/// in a search path folder by its file name, so they are written to the
/// cache folder once and that folder is added; nothing has to be installed.
fn add_icons() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let dir = glib::user_cache_dir().join("corerules").join("icons");
    let written = ICONS.iter().all(|(name, svg)| {
        let path = dir.join(format!("{name}.svg"));
        std::fs::read_to_string(&path).is_ok_and(|s| s == *svg)
            || (std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&path, svg).is_ok())
    });
    if written {
        gtk::IconTheme::for_display(&display).add_search_path(&dir);
    }
}

/// The window and the widgets that change while it is open.
#[derive(Clone)]
struct Window {
    window: adw::ApplicationWindow,
    /// Holds the browser, or a status page when there is nothing to show.
    stack: gtk::Stack,
    toasts: adw::ToastOverlay,
    browser: Rc<Browser>,
    install: Rc<RefCell<Option<Installation>>>,
    /// One random number stream for the session, as the originals seed
    /// theirs once at start.
    rand: Rc<RefCell<Rand>>,
    /// The NPC generator of the open installation, loaded on first use.
    npc_rules: Rc<RefCell<Option<Arc<NpcRules>>>>,
}

impl Window {
    fn new(app: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Core Rules")
            .default_width(1100)
            .default_height(720)
            .width_request(360)
            .height_request(294)
            .build();
        let stack = gtk::Stack::new();
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&stack));
        window.set_content(Some(&toasts));
        let browser = Browser::new();
        browser.breakpoints(&window);
        stack.add_named(&browser.outer, Some("browser"));
        let this = Self {
            window,
            stack,
            toasts,
            browser: Rc::new(browser),
            install: Rc::default(),
            rand: Rc::new(RefCell::new(Rand::from_clock())),
            npc_rules: Rc::default(),
        };
        this.browser.sheet_button.connect_clicked({
            let this = this.clone();
            move |_| {
                let key = this.browser.current_key.borrow().clone();
                this.show_sheet(&key);
            }
        });
        this.browser.edit_button.connect_clicked({
            let this = this.clone();
            move |_| {
                let key = this.browser.current_key.borrow().clone();
                let host = Rc::new(this.clone());
                if let Some(name) = key.strip_prefix(TREASURE_KEY) {
                    this.roll_treasure(Some(name))
                } else if let Some(name) = key.strip_prefix(ENCOUNTER_KEY) {
                    encounter::open_saved(&host, name)
                } else if let Some(name) = key.strip_prefix(TABLE_KEY) {
                    encounter::table_editor(&host, Some(name))
                } else if key.starts_with(cr_store::userdb::KEY) {
                    userdb::edit(&host, &key)
                } else {
                    this.edit_character(&key)
                }
            }
        });

        // Copy on a saved encounter or treasure bundle, as the original's
        // Encounters tab and List of Treasure Bundles have it.
        this.browser.copy_button.connect_clicked({
            let this = this.clone();
            move |_| {
                let key = this.browser.current_key.borrow().clone();
                if let Some(name) = key.strip_prefix(TREASURE_KEY) {
                    this.copy_treasure(name)
                } else if let Some(name) = key.strip_prefix(ENCOUNTER_KEY) {
                    encounter::copy_saved(&Rc::new(this.clone()), name)
                }
            }
        });

        this.browser.delete_button.connect_clicked({
            let this = this.clone();
            move |_| {
                let key = this.browser.current_key.borrow().clone();
                party::delete_character(&Rc::new(this.clone()), &key);
            }
        });

        this.browser.convert_button.connect_clicked({
            let this = this.clone();
            move |_| {
                let key = this.browser.current_key.borrow().clone();
                this.convert_to_character(&key);
            }
        });

        this.browser.npc_button.connect_clicked({
            let this = this.clone();
            move |_| {
                let key = this.browser.current_key.borrow().clone();
                this.convert_to_npc(&key);
            }
        });

        this.browser.generate_button.connect_clicked({
            let this = this.clone();
            move |_| {
                let key = this.browser.current_key.borrow().clone();
                let host = Rc::new(this.clone());
                if let Some(name) = library::template_name(&key) {
                    encounter::create_from_template(&host, name)
                } else if let Some(name) = key.strip_prefix(TABLE_KEY) {
                    encounter::roll_and_generate(&host, name)
                }
            }
        });

        for (button, print) in [
            (&this.browser.view_button, false),
            (&this.browser.print_button, true),
        ] {
            button.connect_clicked({
                let this = this.clone();
                move |_| {
                    let key = this.browser.current_key.borrow().clone();
                    let install = this.install.borrow();
                    let Some(install) = install.as_ref() else {
                        return;
                    };
                    let toasts = this.toasts.clone();
                    let toast = move |m: &str| toasts.add_toast(adw::Toast::new(m));
                    match library::monster_key(&key) {
                        Some((name, custom)) => {
                            help::monster(&this.window, install, name, custom, print, toast)
                        }
                        None => {
                            let Some(page) = install.page(&key) else {
                                return;
                            };
                            if print {
                                sheet::print_sheet(&this.window, &Rc::new(page), &toast);
                            } else {
                                sheet::preview(&this.window, page, toast);
                            }
                        }
                    }
                }
            });
        }

        let preferences = gio::ActionEntry::builder(action::PREFERENCES.name)
            .activate({
                let this = this.clone();
                move |_, _, _| {
                    let toasts = this.toasts.clone();
                    prefs::show(&this.window, move |m| toasts.add_toast(adw::Toast::new(m)))
                }
            })
            .build();
        let open = gio::ActionEntry::builder(action::OPEN.name)
            .activate({
                let this = this.clone();
                move |_, _, _| this.choose_folder()
            })
            .build();
        let extract_data = gio::ActionEntry::builder(action::EXTRACT_DATA.name)
            .activate({
                let this = this.clone();
                move |_, _, _| extract::choose_source(&Rc::new(this.clone()))
            })
            .build();
        let new_character = gio::ActionEntry::builder(action::NEW_CHARACTER.name)
            .activate({
                let this = this.clone();
                move |_, _, _| this.new_character(false)
            })
            .build();
        let enter_character = gio::ActionEntry::builder(action::ENTER_CHARACTER.name)
            .activate({
                let this = this.clone();
                move |_, _, _| this.new_character(true)
            })
            .build();
        let new_npc = gio::ActionEntry::builder(action::NEW_NPC.name)
            .activate({
                let this = this.clone();
                move |_, _, _| npc::new_npc(&Rc::new(this.clone()))
            })
            .build();
        let new_npc_group = gio::ActionEntry::builder(action::NEW_NPC_GROUP.name)
            .activate({
                let this = this.clone();
                move |_, _, _| npc::new_group(&Rc::new(this.clone()))
            })
            .build();
        let roll_treasure = gio::ActionEntry::builder(action::ROLL_TREASURE.name)
            .activate({
                let this = this.clone();
                move |_, _, _| this.roll_treasure(None)
            })
            .build();
        let new_encounter = gio::ActionEntry::builder(action::NEW_ENCOUNTER.name)
            .activate({
                let this = this.clone();
                move |_, _, _| encounter::choose_template(&Rc::new(this.clone()))
            })
            .build();
        let new_table = gio::ActionEntry::builder(action::NEW_TABLE.name)
            .activate({
                let this = this.clone();
                move |_, _, _| encounter::table_editor(&Rc::new(this.clone()), None)
            })
            .build();
        let open_book = gio::ActionEntry::builder(action::OPEN_BOOK.name)
            .parameter_type(Some(glib::VariantTy::STRING))
            .activate({
                let this = this.clone();
                move |_, _, name| {
                    if let Some(name) = name.and_then(|n| n.str()) {
                        this.open_book(name, None)
                    }
                }
            })
            .build();
        let help = gio::ActionEntry::builder(action::HELP.name)
            .parameter_type(Some(glib::VariantTy::STRING))
            .activate({
                let this = this.clone();
                move |_, _, spec| {
                    if let Some(spec) = spec.and_then(|s| s.str()) {
                        this.open_help(spec)
                    }
                }
            })
            .build();
        // The original's Help > Main Menu and F1: the program help of the
        // screen shown.
        let help_here = gio::ActionEntry::builder(action::HELP_HERE.name)
            .activate({
                let this = this.clone();
                move |_, _, _| {
                    let topic = topic::of_section(&userdb::Host::current_section(&this));
                    this.open_help(&cr_app::help::program(topic))
                }
            })
            .build();
        // A right-click on a control or a row: its screen and what it
        // reads.
        let help_control = gio::ActionEntry::builder(action::HELP_CONTROL.name)
            .parameter_type(Some(glib::VariantTy::STRING))
            .activate({
                let this = this.clone();
                move |_, _, spec| {
                    let Some(spec) = spec.and_then(|s| s.str()) else {
                        return;
                    };
                    let mut lines = spec.lines();
                    let Some(screen) = lines.next().and_then(|n| n.parse().ok()) else {
                        return;
                    };
                    let texts: Vec<&str> = lines.collect();
                    if let Some(install) = this.install.borrow().as_ref() {
                        let toasts = this.toasts.clone();
                        let toast = move |m: &str| toasts.add_toast(adw::Toast::new(m));
                        help::context_help(&this.window, install, screen, &texts, toast);
                    }
                }
            })
            .build();
        let about = gio::ActionEntry::builder(action::ABOUT.name)
            .activate({
                let this = this.clone();
                move |_, _, _| help::about(&this.window, this.install.borrow().as_ref())
            })
            .build();
        let roll_table = gio::ActionEntry::builder(action::ROLL_TABLE.name)
            .activate({
                let this = this.clone();
                move |_, _, _| this.roll_table()
            })
            .build();
        let manage_parties = gio::ActionEntry::builder(action::MANAGE_PARTIES.name)
            .activate({
                let this = this.clone();
                move |_, _, _| party::manage(&Rc::new(this.clone()))
            })
            .build();
        let export = gio::ActionEntry::builder(action::EXPORT.name)
            .activate({
                let this = this.clone();
                move |_, _, _| transfer::export(&Rc::new(this.clone()))
            })
            .build();
        let import = gio::ActionEntry::builder(action::IMPORT.name)
            .activate({
                let this = this.clone();
                move |_, _, _| transfer::import(&Rc::new(this.clone()))
            })
            .build();
        let import_core20 = gio::ActionEntry::builder(action::IMPORT_CORE20.name)
            .activate({
                let this = this.clone();
                move |_, _, _| transfer::import_core20(&Rc::new(this.clone()))
            })
            .build();
        let new_package = gio::ActionEntry::builder(action::NEW_PACKAGE.name)
            .activate({
                let this = this.clone();
                move |_, _, _| transfer::new_package(&Rc::new(this.clone()))
            })
            .build();
        let edit_package = gio::ActionEntry::builder(action::EDIT_PACKAGE.name)
            .activate({
                let this = this.clone();
                move |_, _, _| transfer::edit_package(&Rc::new(this.clone()))
            })
            .build();
        this.window
            .add_action_entries([help, help_here, help_control, about]);
        help::add_key(&this.window, {
            let this = this.clone();
            move || topic::of_section(&userdb::Host::current_section(&this))
        });
        this.window.add_action_entries([
            open_book,
            roll_table,
            export,
            import,
            new_package,
            edit_package,
            manage_parties,
            import_core20,
        ]);
        this.window.add_action_entries([
            preferences,
            open,
            extract_data,
            new_character,
            enter_character,
            new_npc,
            new_npc_group,
            roll_treasure,
            new_encounter,
            new_table,
        ]);
        app.set_accels_for_action(action::OPEN.detailed, &["<Control>o"]);
        app.set_accels_for_action(action::PREFERENCES.detailed, &["<Control>comma"]);
        app.set_accels_for_action(action::NEW_CHARACTER.detailed, &["<Control>n"]);
        app.set_accels_for_action(action::NEW_NPC.detailed, &["<Control><Shift>n"]);
        app.set_accels_for_action(action::ROLL_TREASURE.detailed, &["<Control>t"]);
        app.set_accels_for_action(action::ROLL_TABLE.detailed, &["<Control><Shift>t"]);
        app.set_accels_for_action(action::NEW_ENCOUNTER.detailed, &["<Control>e"]);
        userdb::add_actions(&this.window, Rc::new(this.clone()));
        this.set_can_create(false);
        this
    }

    fn choose_folder(&self) {
        let dialog = gtk::FileDialog::builder()
            .title(tr("Open Core Rules Folder"))
            .modal(true)
            .build();
        let this = self.clone();
        dialog.select_folder(Some(&self.window), gio::Cancellable::NONE, move |result| {
            if let Ok(folder) = result
                && let Some(path) = folder.path()
            {
                this.open(path);
            }
        });
    }

    fn open(&self, dir: PathBuf) {
        match Installation::open(&dir) {
            // A mounted CD or ISO image: what is made is saved into a copy
            // in the app's data folder instead. Core Rules 2.0 is only
            // browsed, so it opens from the CD.
            Ok(install) if !install.is_core20() && !install.is_writable() => {
                extract::open_local_copy(&Rc::new(self.clone()), dir)
            }
            Ok(install) => self.show_installation(install),
            Err(e) => self.show_missing(Some(&e.to_string())),
        }
    }

    fn show_missing(&self, error: Option<&str>) {
        let button = gtk::Button::builder()
            .label(tr("_Open Folder…"))
            .use_underline(true)
            .halign(gtk::Align::Center)
            .action_name(action::OPEN.detailed)
            .css_classes(["pill"])
            .build();
        let extract = gtk::Button::builder()
            .label(tr("Extract Data from _CD…"))
            .use_underline(true)
            .halign(gtk::Align::Center)
            .action_name(action::EXTRACT_DATA.detailed)
            .css_classes(["pill", "suggested-action"])
            .build();
        let buttons = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();
        buttons.append(&extract);
        buttons.append(&button);
        let page = adw::StatusPage::builder()
            .icon_name("folder-symbolic")
            .title(tr("No Core Rules Installation"))
            .description(match error {
                Some(error) => glib::markup_escape_text(error).to_string(),
                None => tr("Extract the data from your Core Rules 2.0 or Expansion CD \
                            (its ISO image or the mounted CD), or open the folder \
                            where AD&amp;D Core Rules 2.0 is installed or a data package")
                .to_owned(),
            })
            .child(&buttons)
            .build();
        let view = adw::ToolbarView::new();
        view.add_top_bar(&adw::HeaderBar::new());
        view.set_content(Some(&page));
        self.show_status(&view);
        self.set_can_create(false);
    }

    fn show_installation(&self, install: Installation) {
        self.browser.load(install.catalog(), None);
        self.window.set_title(Some(&format!(
            "Core Rules — {}",
            install
                .root()
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default()
        )));
        self.stack.set_visible_child_name("browser");
        // Core Rules 2.0 content is only imported into the Expansion:
        // a 2.0 installation is browsed, never changed.
        let read_only = install.is_core20();
        *self.browser.detail.full_screen.borrow_mut() = install.full_screen_background();
        *self.install.borrow_mut() = Some(install);
        self.npc_rules.borrow_mut().take();
        self.set_can_create(true);
        for name in WRITE_ACTIONS {
            if let Some(action) = self
                .window
                .lookup_action(name)
                .and_downcast::<gio::SimpleAction>()
            {
                action.set_enabled(!read_only);
            }
        }
        self.browser.set_read_only(read_only);
        if !read_only
            && self
                .install
                .borrow()
                .as_ref()
                .is_some_and(Installation::characters_damaged)
        {
            self.offer_rebuild();
        }
        if !read_only && let Some(install) = self.install.borrow().as_ref() {
            let this = self.clone();
            let reload = Rc::new(move || {
                if let Some(install) = this.install.borrow().as_ref() {
                    this.browser.load(install.catalog(), None);
                }
            });
            let parent = self.window.clone().upcast();
            if let Err(e) = userdb::rename_hidden_parts(&parent, install, reload) {
                self.toast(&e.to_string());
            }
        }
    }

    /// Offers to rebuild a damaged character list from the character files,
    /// as the original does (0x47a87f).
    fn offer_rebuild(&self) {
        let alert = adw::AlertDialog::builder()
            .heading(tr("Rebuild the Character List?"))
            .body(tr(
                "The list of your characters and NPCs cannot be read, or it lists a \
                 character whose file is missing or damaged. It can be rebuilt from \
                 the character files; with many characters this may take a while.",
            ))
            .default_response("rebuild")
            .close_response("cancel")
            .build();
        alert.add_responses(&[("cancel", tr("_Not Now")), ("rebuild", tr("_Rebuild"))]);
        alert.set_response_appearance("rebuild", adw::ResponseAppearance::Suggested);
        let this = self.clone();
        alert.connect_response(Some("rebuild"), move |_, _| {
            let result = this
                .install
                .borrow()
                .as_ref()
                .map(Installation::rebuild_characters);
            match result {
                Some(Ok(rebuilt)) => {
                    let mut message = trn!(
                        "The list was rebuilt with {n} character",
                        "The list was rebuilt with {n} characters",
                        rebuilt.characters
                    );
                    if !rebuilt.unreadable.is_empty() {
                        message = tr!(
                            "{message}; these files could not be read: {files}",
                            message,
                            files = rebuilt.unreadable.join(", ")
                        );
                    }
                    this.toast(&message);
                    this.reload("characters", None);
                }
                Some(Err(e)) => this.toast(&e.to_string()),
                None => {}
            }
        });
        alert.present(Some(&self.window));
    }

    fn set_can_create(&self, enabled: bool) {
        for name in [
            "new-character",
            "enter-character",
            "new-npc",
            "new-npc-group",
            "roll-treasure",
            "new-encounter",
            "new-table",
            "open-book",
            "roll-table",
            "export",
            "import",
            "new-package",
            "edit-package",
            "manage-parties",
            "import-core20",
        ] {
            if let Some(action) = self
                .window
                .lookup_action(name)
                .and_downcast::<gio::SimpleAction>()
            {
                action.set_enabled(enabled);
            }
        }
    }

    /// Opens the treasure roller, on the saved bundle `saved` if given.
    fn roll_treasure(&self, saved: Option<&str>) {
        self.open_treasure(saved, false);
    }

    /// The original's Copy… of a saved bundle (0x53f028): the roller on
    /// its items with no name, so that saving adds a new bundle.
    fn copy_treasure(&self, name: &str) {
        self.open_treasure(Some(name), true);
    }

    fn open_treasure(&self, saved: Option<&str>, copy: bool) {
        let loaded = match &*self.install.borrow() {
            Some(install) => install.treasure_rules().and_then(|rules| {
                let bundle = match saved {
                    Some(name) => install
                        .saved_treasures()?
                        .into_iter()
                        .find(|t| t.name == name)
                        .map(|t| t.bundle)
                        .unwrap_or_default(),
                    None => Default::default(),
                };
                Ok((rules, bundle))
            }),
            None => return,
        };
        let (rules, bundle) = match loaded {
            Ok(loaded) => loaded,
            Err(e) => return self.toast(&e.to_string()),
        };
        let store = self.treasure_store();
        let this = self.clone();
        treasure::roller(
            &self.window,
            Rc::new(rules),
            self.rand.clone(),
            treasure::Start {
                bundle,
                ..Default::default()
            },
            treasure::Mode::Free {
                saved: saved.filter(|_| !copy).map(str::to_owned),
            },
            store,
            move |message| this.toast(message),
        );
    }

    /// How the treasure roller saves and deletes bundles.
    fn treasure_store(&self) -> treasure::Store {
        treasure::Store {
            save: Box::new({
                let this = self.clone();
                move |old, name, bundle| {
                    {
                        let install = this.install.borrow();
                        let install = install
                            .as_ref()
                            .ok_or(tr("The installation is no longer open"))?;
                        match old {
                            Some(old) => install.replace_saved_treasure(old, name, bundle),
                            None => install.save_treasure(name, bundle),
                        }
                        .map_err(|e| e.to_string())?;
                    }
                    this.reload("treasure", Some(format!("{TREASURE_KEY}{name}")));
                    Ok(())
                }
            }),
            delete: Box::new({
                let this = self.clone();
                move |name| {
                    {
                        let install = this.install.borrow();
                        let install = install
                            .as_ref()
                            .ok_or(tr("The installation is no longer open"))?;
                        install
                            .delete_saved_treasure(name)
                            .map_err(|e| e.to_string())?;
                    }
                    this.reload("treasure", None);
                    Ok(())
                }
            }),
        }
    }

    /// Reloads the sections and shows the entry `key` of `section`.
    fn reload(&self, section: &str, key: Option<String>) {
        // A record saved changes what the NPC generator and the item
        // checks offer (a new kit, race or item).
        self.npc_rules.borrow_mut().take();
        let install = self.install.borrow();
        let Some(install) = install.as_ref() else {
            return;
        };
        // The other part types share one section.
        let section = match other_type(section) {
            Some(_) => OTHER_SECTION,
            None => section,
        };
        self.browser.show_section(section);
        self.browser.load(install.catalog(), key.as_deref());
        if key.is_none() {
            self.browser.show_section(section);
        }
    }

    /// The item check the Equipment pages use, read with the NPC
    /// generator's data and kept with it; `None`, without a word, when
    /// the program cannot be read.
    fn item_rules(&self) -> Option<Arc<NpcRules>> {
        if let Some(rules) = &*self.npc_rules.borrow() {
            return Some(rules.clone());
        }
        let rules = Arc::new(self.install.borrow().as_ref()?.npc_rules().ok()?);
        *self.npc_rules.borrow_mut() = Some(rules.clone());
        Some(rules)
    }

    /// Opens the New Character dialog, or with `entering` the Enter
    /// Character one (the original's Character Management button 1573).
    fn new_character(&self, entering: bool) {
        let (rules, root, backgrounds) = match &*self.install.borrow() {
            Some(install) => (
                install.creation_rules().and_then(|creation| {
                    let advance = Arc::new(install.advance_rules()?);
                    let learn = Arc::new(install.learn_rules()?);
                    // Without the Expansion's psionics there is no Wild
                    // Talents step.
                    let skills = creation
                        .expansion
                        .then(|| install.skill_rules().ok().map(Rc::new))
                        .flatten();
                    Ok((creation, advance, learn, skills))
                }),
                install.root().to_owned(),
                cr_store::edit::backgrounds(install).unwrap_or_default(),
            ),
            None => return,
        };
        let (rules, advance, learn, skills) = match rules {
            Ok(rules) => rules,
            Err(e) => return self.toast(&e.to_string()),
        };
        let this = self.clone();
        let mode = create::Mode {
            entering,
            advance,
            learn,
            items: self.item_rules(),
            skills,
            root,
            backgrounds,
        };
        create::new_character(&self.window, rules, mode, move |draft| {
            let install = this.install.borrow();
            let Some(install) = install.as_ref() else {
                return Err(tr("The installation is no longer open").into());
            };
            let rules = install.creation_rules().map_err(|e| e.to_string())?;
            let file = install
                .create_character(&rules, draft)
                .map_err(|e| e.to_string())?;
            this.browser.show_section("characters");
            this.browser.load(install.catalog(), Some(&file));
            // Enter Character's "Character Saved" (0x496b0b).
            this.toast(if draft.entered.is_some() {
                tr("Character saved")
            } else {
                tr("Character created")
            });
            Ok(())
        });
    }

    /// Convert to Character: the selected NPC becomes a player character.
    fn convert_to_character(&self, file: &str) {
        let result = match &*self.install.borrow() {
            Some(install) => install.convert_to_character(file),
            None => return,
        };
        match result {
            Ok(()) => {
                self.reload("characters", Some(file.to_owned()));
                self.toast(tr("Converted to a character"));
            }
            Err(e) => self.toast(&e.to_string()),
        }
    }

    /// Convert to NPC: after the original's question, the selected
    /// character moves to the DM Toolkit's NPCs.
    fn convert_to_npc(&self, file: &str) {
        let alert = adw::AlertDialog::builder()
            .heading(tr("Convert to NPC?"))
            .body(tr(
                "This will move the selected Player Character to the NPC List in the DM Toolkit.",
            ))
            .default_response("cancel")
            .close_response("cancel")
            .build();
        alert.add_responses(&[("cancel", tr("_Cancel")), ("convert", tr("_Convert"))]);
        alert.set_response_appearance("convert", adw::ResponseAppearance::Suggested);
        let this = self.clone();
        let file = file.to_owned();
        alert.connect_response(Some("convert"), move |_, _| {
            let result = match &*this.install.borrow() {
                Some(install) => install.convert_to_npc(&file),
                None => return,
            };
            match result {
                Ok(new) => {
                    this.reload("characters", Some(new));
                    this.toast(tr("Converted to an NPC"));
                }
                Err(e) => this.toast(&e.to_string()),
            }
        });
        alert.present(Some(&self.window));
    }

    /// A note at the bottom of the window; with a dialog open, which
    /// would hide it, an alert on the dialog instead.
    fn toast(&self, message: &str) {
        match self.window.visible_dialog() {
            Some(dialog) => create::message(&dialog, tr("Core Rules"), message),
            None => self.toasts.add_toast(adw::Toast::new(message)),
        }
    }

    /// Shows the character sheet of the character in `Chars/<file>`; an
    /// Expansion character's sections and border can be changed there.
    fn show_sheet(&self, file: &str) {
        let loaded = match &*self.install.borrow() {
            Some(install) => cr_app::sheets::character(install, file),
            None => return,
        };
        let (sheet, choice) = match loaded {
            Ok(loaded) => loaded,
            Err(e) => return self.toast(&e.to_string()),
        };
        let options = choice.map(|choice| sheet::Options {
            choice,
            save: Box::new({
                let (this, file) = (self.clone(), file.to_owned());
                move |choice| match &*this.install.borrow() {
                    Some(install) => {
                        cr_app::sheets::choose(install, &file, choice).map_err(|e| e.to_string())
                    }
                    None => Err(String::new()),
                }
            }),
        });
        let this = self.clone();
        sheet::show(&self.window, sheet, options, move |message| {
            this.toast(message)
        });
    }

    /// Opens the editor for the character stored in `Chars/<file>`.
    fn edit_character(&self, file: &str) {
        let loaded = match &*self.install.borrow() {
            Some(install) => install.character(file),
            None => return,
        };
        let character = match loaded {
            Ok(character) => character,
            Err(e) => return self.toast(&e.to_string()),
        };
        let choices = match &*self.install.borrow() {
            Some(install) => Choices::load(install, &character),
            None => return,
        };
        let mut choices = match choices {
            Ok(choices) => choices,
            Err(e) => return self.toast(&e.to_string()),
        };
        choices.item_rules = self.item_rules();
        let details = CharacterDetails::of(&character);
        let lists = CharacterLists::of(&character);
        let classes = character.data.classes.names.clone();
        let Some(root) = self.install.borrow().as_ref().map(|i| i.root().to_owned()) else {
            return;
        };
        let this = self.clone();
        let file = file.to_owned();
        character_editor(
            &self.window,
            root,
            &details,
            &lists,
            &classes,
            choices,
            move |details, lists| {
                let install = this.install.borrow();
                let Some(install) = install.as_ref() else {
                    return Err(tr("The installation is no longer open").into());
                };
                install
                    .save_character(&file, details, Some(lists))
                    .map_err(|e| e.to_string())?;
                this.browser.load(install.catalog(), Some(&file));
                this.toast(tr("Character saved"));
                Ok(())
            },
        );
    }

    /// Opens a book in its reader window, searching for `find` if given.
    fn open_book(&self, name: &str, find: Option<&str>) {
        let opened = match &*self.install.borrow() {
            Some(install) => books::open(&self.window, install, name, find),
            None => return,
        };
        if let Err(e) = opened {
            create::message(&self.window, tr("Could Not Open Book"), &e);
        }
    }

    /// Opens a help file of `Help/` in the help window: `FILE`, at its
    /// contents topic, or `FILE#CONTEXT` or `FILE#NUMBER` (a `[MAP]`
    /// number).
    fn open_help(&self, spec: &str) {
        let (file, target) = help::Target::parse(spec);
        let opened = match &*self.install.borrow() {
            Some(install) => help::open(&self.window, install, file, target),
            None => Err(tr("Open a Core Rules folder to read its help").to_owned()),
        };
        if let Err(e) = opened {
            self.toast(&e);
        }
    }

    /// Opens the Roll on a Table dialog.
    fn roll_table(&self) {
        let Some(root) = self.install.borrow().as_ref().map(|i| i.root().to_owned()) else {
            return;
        };
        let this = self.clone();
        let treasure = self.clone();
        tables::open(
            &self.window,
            root,
            self.rand.clone(),
            move |name, find| this.open_book(name, find),
            move || treasure.roll_treasure(None),
        );
    }

    fn show_status(&self, widget: &impl IsA<gtk::Widget>) {
        if let Some(old) = self.stack.child_by_name("status") {
            self.stack.remove(&old);
        }
        self.stack.add_named(widget, Some("status"));
        self.stack.set_visible_child_name("status");
    }
}

impl npc::Host for Window {
    fn with_install<T>(
        &self,
        run: impl FnOnce(&Installation) -> cr_store::Result<T>,
    ) -> Option<cr_store::Result<T>> {
        self.install.borrow().as_ref().map(run)
    }

    fn npc_rules(&self) -> Option<Arc<NpcRules>> {
        if let Some(rules) = &*self.npc_rules.borrow() {
            return Some(rules.clone());
        }
        let loaded = self.install.borrow().as_ref()?.npc_rules();
        match loaded {
            Ok(rules) => {
                let rules = Arc::new(rules);
                *self.npc_rules.borrow_mut() = Some(rules.clone());
                Some(rules)
            }
            Err(e) => {
                self.toast(&e.to_string());
                None
            }
        }
    }

    fn rand(&self) -> Rc<RefCell<Rand>> {
        self.rand.clone()
    }

    fn toast(&self, message: &str) {
        Window::toast(self, message)
    }

    fn reload(&self, section: &str, key: Option<String>) {
        Window::reload(self, section, key)
    }

    fn parent(&self) -> gtk::Widget {
        self.window.clone().upcast()
    }
}

impl extract::Host for Window {
    fn open(&self, dir: PathBuf) {
        Window::open(self, dir)
    }

    fn add_toast(&self, toast: adw::Toast) {
        self.toasts.add_toast(toast);
    }

    fn parent(&self) -> gtk::Widget {
        self.window.clone().upcast()
    }
}

impl transfer::Host for Window {
    fn installation(&self) -> std::cell::Ref<'_, Option<Installation>> {
        self.install.borrow()
    }

    fn toast(&self, message: &str) {
        Window::toast(self, message)
    }

    fn reloaded(&self) {
        // Imported races and classes change what the NPC generator offers.
        self.npc_rules.borrow_mut().take();
        if let Some(install) = self.install.borrow().as_ref() {
            self.browser.load(install.catalog(), None);
        }
    }

    fn parent(&self) -> gtk::Widget {
        self.window.clone().upcast()
    }
}

impl party::Host for Window {
    fn with_install<T>(
        &self,
        run: impl FnOnce(&Installation) -> cr_store::Result<T>,
    ) -> Option<cr_store::Result<T>> {
        self.install.borrow().as_ref().map(run)
    }

    fn toast(&self, message: &str) {
        Window::toast(self, message)
    }

    fn add_toast(&self, toast: adw::Toast) {
        self.toasts.add_toast(toast);
    }

    fn reload(&self, section: &str, key: Option<String>) {
        Window::reload(self, section, key)
    }

    fn parent(&self) -> gtk::Widget {
        self.window.clone().upcast()
    }

    fn current_key(&self) -> String {
        self.browser.current_key.borrow().clone()
    }

    fn group_encounter(&self, parent: &gtk::Widget, party: &str, members: Vec<String>) {
        encounter::group_encounter(&Rc::new(self.clone()), parent, party, members)
    }
}

impl userdb::Host for Window {
    fn with_install<T>(
        &self,
        run: impl FnOnce(&Installation) -> cr_store::Result<T>,
    ) -> Option<cr_store::Result<T>> {
        self.install.borrow().as_ref().map(run)
    }

    fn toast(&self, message: &str) {
        Window::toast(self, message)
    }

    fn reload(&self, section: &str, key: Option<String>) {
        Window::reload(self, section, key)
    }

    fn parent(&self) -> gtk::Widget {
        self.window.clone().upcast()
    }

    fn current_key(&self) -> String {
        self.browser.current_key.borrow().clone()
    }

    fn current_section(&self) -> String {
        let index = self.browser.sidebar_list.selected_row().map(|r| r.index());
        let sections = self.browser.sections.borrow();
        index
            .and_then(|i| sections.get(i as usize))
            .map(|s| s.id.to_owned())
            .unwrap_or_default()
    }

    fn template_npc(&self, parent: &gtk::Widget, chosen: npc::Chosen) {
        npc::template_npc(&Rc::new(self.clone()), parent, chosen)
    }
}

impl encounter::Host for Window {
    fn with_install<T>(
        &self,
        run: impl FnOnce(&Installation) -> cr_store::Result<T>,
    ) -> Option<cr_store::Result<T>> {
        self.install.borrow().as_ref().map(run)
    }

    fn rand(&self) -> Rc<RefCell<Rand>> {
        self.rand.clone()
    }

    fn toast(&self, message: &str) {
        Window::toast(self, message)
    }

    fn reload(&self, section: &str, key: Option<String>) {
        Window::reload(self, section, key)
    }

    fn treasure_store(&self) -> treasure::Store {
        Window::treasure_store(self)
    }

    fn user_record(
        &self,
        section: &str,
        action: &str,
        origin: cr_store::userdb::Origin,
        name: &str,
    ) {
        userdb::record_action(&Rc::new(self.clone()), section, action, origin, name)
    }

    fn new_user_record(&self, section: &str, saved: Box<dyn Fn(&str)>) {
        userdb::new_record_then(&Rc::new(self.clone()), section, saved)
    }

    fn parent(&self) -> gtk::Widget {
        self.window.clone().upcast()
    }
}

/// The template a catalog key names.
/// The name of the monster with detail key `key`, and whether it is the
/// user's.
fn open_menu() -> gtk::MenuButton {
    let menu = gio::Menu::new();
    menu.append(
        Some(tr("_New Character…")),
        Some(action::NEW_CHARACTER.detailed),
    );
    menu.append(
        Some(tr("E_nter Character…")),
        Some(action::ENTER_CHARACTER.detailed),
    );
    menu.append(Some(tr("New N_PC…")), Some(action::NEW_NPC.detailed));
    menu.append(
        Some(tr("New NPC _Group…")),
        Some(action::NEW_NPC_GROUP.detailed),
    );
    let item = gio::MenuItem::new(
        Some(tr("Manage _Parties…")),
        Some(action::MANAGE_PARTIES.detailed),
    );
    item.set_attribute_value("hidden-when", Some(&"action-disabled".to_variant()));
    menu.append_item(&item);
    menu.append(
        Some(tr("New _Encounter…")),
        Some(action::NEW_ENCOUNTER.detailed),
    );
    menu.append(
        Some(tr("New Encounter _Table…")),
        Some(action::NEW_TABLE.detailed),
    );
    menu.append(
        Some(tr("_Roll Treasure…")),
        Some(action::ROLL_TREASURE.detailed),
    );
    menu.append(
        Some(tr("Roll on a _Table…")),
        Some(action::ROLL_TABLE.detailed),
    );
    let packages = gio::Menu::new();
    packages.append(Some(tr("_Import…")), Some(action::IMPORT.detailed));
    let item = gio::MenuItem::new(
        Some(tr("Import Core _Rules 2.0…")),
        Some(action::IMPORT_CORE20.detailed),
    );
    item.set_attribute_value("hidden-when", Some(&"action-disabled".to_variant()));
    packages.append_item(&item);
    packages.append(Some(tr("E_xport…")), Some(action::EXPORT.detailed));
    packages.append(
        Some(tr("New P_ackage…")),
        Some(action::NEW_PACKAGE.detailed),
    );
    packages.append(
        Some(tr("E_dit Package…")),
        Some(action::EDIT_PACKAGE.detailed),
    );
    menu.append_section(None, &packages);
    let app = gio::Menu::new();
    app.append(Some(tr("_Open Folder…")), Some(action::OPEN.detailed));
    app.append(
        Some(tr("Extract Data from _CD…")),
        Some(action::EXTRACT_DATA.detailed),
    );
    app.append(Some(tr("_Preferences")), Some(action::PREFERENCES.detailed));
    menu.append_section(None, &app);
    let help = gio::Menu::new();
    let item = gio::MenuItem::new(Some(tr("_Help")), Some(action::HELP_HERE.detailed));
    // F1 itself is handled by the window and by each dialog.
    item.set_attribute_value("accel", Some(&"F1".to_variant()));
    help.append_item(&item);
    help.append(
        Some(tr("_Online Manual")),
        Some(&*format!(
            "{}::{}",
            action::HELP.detailed,
            cr_app::help::MANUAL
        )),
    );
    help.append(
        Some(tr("_Tome of Help")),
        Some(&*format!(
            "{}::{}",
            action::HELP.detailed,
            cr_app::help::TOME
        )),
    );
    help.append(Some(tr("_About Core Rules")), Some(action::ABOUT.detailed));
    menu.append_section(None, &help);
    gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text(tr("Main Menu"))
        .menu_model(&menu)
        .primary(true)
        .build()
}

/// Sections in the sidebar, the selected section's entries in the middle,
/// and the activated entry on the right.
struct Browser {
    outer: adw::NavigationSplitView,
    inner: adw::NavigationSplitView,
    sidebar_list: gtk::ListBox,
    sections: Rc<RefCell<Vec<Section>>>,
    selection: gtk::SingleSelection,
    detail: DetailPane,
    /// Shown on the detail page of entries that can be edited.
    edit_button: gtk::Button,
    /// Copies a saved encounter or treasure bundle.
    copy_button: gtk::Button,
    sheet_button: gtk::Button,
    /// Clears the NPC mark of a generated character.
    convert_button: gtk::Button,
    /// Sets it on a player character.
    npc_button: gtk::Button,
    /// Deletes a character.
    delete_button: gtk::Button,
    /// Creates an encounter from a template or a 2-20 table.
    generate_button: gtk::Button,
    /// View and Print, as on the original's screens of monsters,
    /// encounters, templates, 2-20 tables, saved treasure and treasure
    /// tables.
    view_button: gtk::Button,
    print_button: gtk::Button,
    /// The key of the entry on the detail page.
    current_key: Rc<RefCell<String>>,
    /// The Characters section's party filter and what each choice shows:
    /// everyone (`None`), no party (empty) or a party.
    party_filter: gtk::DropDown,
    party_choices: Rc<RefCell<Vec<Option<String>>>>,
    /// Says that a Core Rules 2.0 installation is only read.
    read_only_banner: adw::Banner,
}

/// The right-hand pane.
#[derive(Clone)]
struct DetailPane {
    page: adw::NavigationPage,
    view: adw::ToolbarView,
    edit_button: gtk::Button,
    copy_button: gtk::Button,
    sheet_button: gtk::Button,
    convert_button: gtk::Button,
    npc_button: gtk::Button,
    delete_button: gtk::Button,
    generate_button: gtk::Button,
    view_button: gtk::Button,
    print_button: gtk::Button,
    current_key: Rc<RefCell<String>>,
    /// Copy, New and Delete for standard and custom records.
    udb_actions: userdb::DetailActions,
    /// Core Rules 2.0: entries are shown, not edited.
    read_only: Rc<std::cell::Cell<bool>>,
    /// The Monster Full Screen dialog's background.
    full_screen: Rc<RefCell<Option<std::path::PathBuf>>>,
}

impl DetailPane {
    fn show(&self, entry: &Entry) {
        self.page.set_title(&entry.title);
        self.view
            .set_content(Some(&detail(entry, self.full_screen.borrow().clone())));
        let key = &entry.key;
        let writable = !self.read_only.get();
        let commands = Commands::of(entry, writable);
        self.edit_button.set_visible(commands.edit);
        self.copy_button.set_visible(commands.copy);
        self.udb_actions
            .update(if commands.records { key } else { "" });
        self.sheet_button.set_visible(commands.sheet);
        self.convert_button.set_visible(commands.to_character);
        self.npc_button.set_visible(commands.to_npc);
        self.delete_button.set_visible(commands.delete);
        self.generate_button.set_visible(commands.generate);
        self.generate_button
            .set_tooltip_text(Some(library::generate_label(key)));
        self.view_button.set_visible(commands.view);
        self.view_button.set_icon_name(if commands.monster {
            "help-browser-symbolic"
        } else {
            "document-print-preview-symbolic"
        });
        self.print_button.set_visible(commands.print);
        self.current_key.borrow_mut().clone_from(&entry.key);
    }

    fn show_placeholder(&self, widget: &gtk::Widget) {
        self.page.set_title(tr("Details"));
        self.view.set_content(Some(widget));
        self.edit_button.set_visible(false);
        self.copy_button.set_visible(false);
        self.sheet_button.set_visible(false);
        self.convert_button.set_visible(false);
        self.npc_button.set_visible(false);
        self.delete_button.set_visible(false);
        self.generate_button.set_visible(false);
        self.view_button.set_visible(false);
        self.print_button.set_visible(false);
        self.udb_actions.update("");
        self.current_key.borrow_mut().clear();
    }
}

impl Browser {
    fn new() -> Self {
        let sections: Rc<RefCell<Vec<Section>>> = Rc::default();
        let outer = adw::NavigationSplitView::builder()
            .min_sidebar_width(200.0)
            .max_sidebar_width(260.0)
            .build();
        let inner = adw::NavigationSplitView::builder()
            .min_sidebar_width(280.0)
            .max_sidebar_width(400.0)
            .sidebar_width_fraction(0.4)
            .build();

        // Details.
        let detail_page = adw::NavigationPage::builder()
            .title(tr("Details"))
            .tag("detail")
            .build();
        let detail_view = adw::ToolbarView::new();
        let edit_button = gtk::Button::builder()
            .icon_name("document-edit-symbolic")
            .tooltip_text(tr("Edit"))
            .visible(false)
            .build();
        let sheet_button = gtk::Button::builder()
            .icon_name("x-office-document-symbolic")
            .tooltip_text(tr("Character Sheet"))
            .visible(false)
            .build();
        let convert_button = gtk::Button::builder()
            .icon_name("avatar-default-symbolic")
            .tooltip_text(tr("Convert to Character"))
            .visible(false)
            .build();
        let npc_button = gtk::Button::builder()
            .icon_name("system-switch-user-symbolic")
            .tooltip_text(tr("Convert to NPC"))
            .visible(false)
            .build();
        let delete_button = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .tooltip_text(tr("Delete Character"))
            .visible(false)
            .build();
        let copy_button = gtk::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text(tr("Copy"))
            .visible(false)
            .build();
        let generate_button = gtk::Button::builder()
            .icon_name("media-playlist-shuffle-symbolic")
            .tooltip_text(tr("Create Encounter"))
            .visible(false)
            .build();
        let view_button = gtk::Button::builder()
            .icon_name("help-browser-symbolic")
            .tooltip_text(tr("View"))
            .visible(false)
            .build();
        let print_button = gtk::Button::builder()
            .icon_name("printer-symbolic")
            .tooltip_text(tr("Print"))
            .visible(false)
            .build();
        let detail_header = adw::HeaderBar::new();
        detail_header.pack_start(&delete_button);
        detail_header.pack_end(&print_button);
        detail_header.pack_end(&view_button);
        detail_header.pack_end(&generate_button);
        detail_header.pack_end(&copy_button);
        detail_header.pack_end(&edit_button);
        detail_header.pack_end(&sheet_button);
        detail_header.pack_end(&convert_button);
        detail_header.pack_end(&npc_button);
        let udb_actions = userdb::DetailActions::new();
        udb_actions.pack(&detail_header);
        detail_view.add_top_bar(&detail_header);
        detail_view.set_content(Some(&placeholder(tr("Nothing Selected"), "")));
        detail_page.set_child(Some(&detail_view));
        let current_key: Rc<RefCell<String>> = Rc::default();
        let detail = DetailPane {
            page: detail_page.clone(),
            view: detail_view,
            edit_button: edit_button.clone(),
            copy_button: copy_button.clone(),
            sheet_button: sheet_button.clone(),
            convert_button: convert_button.clone(),
            npc_button: npc_button.clone(),
            delete_button: delete_button.clone(),
            generate_button: generate_button.clone(),
            view_button: view_button.clone(),
            print_button: print_button.clone(),
            current_key: current_key.clone(),
            udb_actions,
            read_only: Rc::default(),
            full_screen: Rc::default(),
        };

        // Entries.
        let model = gio::ListStore::new::<glib::BoxedAnyObject>();
        let search = gtk::SearchEntry::builder()
            .placeholder_text(tr("Search"))
            .hexpand(true)
            .build();
        let origin_filter = userdb::origin_filter();
        let party_filter = gtk::DropDown::builder()
            .tooltip_text(tr("Show a Party"))
            .visible(false)
            .build();
        let party_choices: Rc<RefCell<Vec<Option<String>>>> = Rc::default();
        // The type of the other parts, in their section.
        let type_filter = gtk::DropDown::builder()
            .model(&gtk::StringList::new(
                &std::iter::once(tr("All Types"))
                    .chain(OTHER_TYPES.iter().map(|(_, _, name)| tr(name)))
                    .collect::<Vec<_>>(),
            ))
            .tooltip_text(tr("Show a Part Type"))
            .visible(false)
            .build();
        // The Parts list's Magic Only (ctl 1277, 0x68fe59).
        let magic_filter = gtk::ToggleButton::builder()
            .icon_name("starred-symbolic")
            .tooltip_text(tr("Magic Only"))
            .visible(false)
            .build();
        // The section shown, by its place in `sections`.
        let current = Rc::new(RefCell::new(0usize));
        let filter = gtk::CustomFilter::new({
            let (type_filter, magic_filter) = (type_filter.clone(), magic_filter.clone());
            let (search, origin_filter) = (search.clone(), origin_filter.clone());
            let (party_filter, party_choices) = (party_filter.clone(), party_choices.clone());
            let (sections, current) = (sections.clone(), current.clone());
            move |item| {
                let item = item.downcast_ref::<glib::BoxedAnyObject>().unwrap();
                let row = item.borrow::<Row>();
                let filter = library::Filter {
                    search: search.text().into(),
                    origins: cr_store::userdb::Filter::of(origin_filter.selected()),
                    party: party_choices
                        .borrow()
                        .get(party_filter.selected() as usize)
                        .cloned()
                        .flatten(),
                    part_type: (type_filter.selected() as usize)
                        .checked_sub(1)
                        .and_then(|i| OTHER_TYPES.get(i))
                        .map(|(id, ..)| *id),
                    magic_only: magic_filter.is_active(),
                };
                let sections = sections.borrow();
                sections
                    .get(*current.borrow())
                    .and_then(|s| Some((s.id, s.entries.get(row.index)?)))
                    .is_some_and(|(id, entry)| filter.shows(id, entry))
            }
        });
        origin_filter.connect_selected_notify({
            let filter = filter.clone();
            move |_| filter.changed(gtk::FilterChange::Different)
        });
        party_filter.connect_selected_notify({
            let filter = filter.clone();
            move |_| filter.changed(gtk::FilterChange::Different)
        });
        type_filter.connect_selected_notify({
            let filter = filter.clone();
            move |_| filter.changed(gtk::FilterChange::Different)
        });
        magic_filter.connect_toggled({
            let filter = filter.clone();
            move |_| filter.changed(gtk::FilterChange::Different)
        });
        search.connect_search_changed({
            let filter = filter.clone();
            move |_| filter.changed(gtk::FilterChange::Different)
        });
        let filtered = gtk::FilterListModel::new(Some(model.clone()), Some(filter));
        let selection = gtk::SingleSelection::builder()
            .model(&filtered)
            .autoselect(false)
            .can_unselect(true)
            .build();
        let list = gtk::ListView::builder()
            .model(&selection)
            .factory(&row_factory())
            .single_click_activate(true)
            .css_classes(["navigation-sidebar"])
            .build();
        let entries_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&list)
            .build();
        let empty = adw::StatusPage::builder()
            .icon_name("edit-find-symbolic")
            .css_classes(["compact"])
            .build();
        let entries_stack = gtk::Stack::new();
        entries_stack.add_named(&entries_scroll, Some("list"));
        entries_stack.add_named(&empty, Some("empty"));
        // An empty list says whether the search or the section is empty.
        let section_title = Rc::new(RefCell::new(String::new()));
        let show_empty = Rc::new({
            let (entries_stack, empty, search, section_title) = (
                entries_stack.clone(),
                empty.clone(),
                search.clone(),
                section_title.clone(),
            );
            move |shown: u32| {
                if shown > 0 {
                    entries_stack.set_visible_child_name("list");
                    return;
                }
                if search.text().is_empty() {
                    empty.set_title(&tr!("No {section}", section = section_title.borrow()));
                    empty.set_icon_name(Some("folder-symbolic"));
                } else {
                    empty.set_title(tr("No Results"));
                    empty.set_icon_name(Some("edit-find-symbolic"));
                }
                entries_stack.set_visible_child_name("empty");
            }
        });
        filtered.connect_items_changed({
            let show_empty = show_empty.clone();
            move |m, _, _, _| show_empty(m.n_items())
        });
        let search_bar = gtk::SearchBar::builder()
            .child(&search)
            .search_mode_enabled(true)
            .build();
        // The bar stays: Esc only clears the search.
        search_bar.connect_search_mode_enabled_notify(|bar| {
            if !bar.is_search_mode() {
                bar.set_search_mode(true);
            }
        });
        search.connect_stop_search(|entry| entry.set_text(""));
        // Shown in the Characters section.
        let new_button = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text(tr("New Character"))
            .action_name(action::NEW_CHARACTER.detailed)
            .visible(false)
            .build();
        let entries_header = adw::HeaderBar::new();
        entries_header.pack_start(&new_button);
        entries_header.pack_end(&origin_filter);
        entries_header.pack_end(&party_filter);
        entries_header.pack_end(&type_filter);
        entries_header.pack_end(&magic_filter);
        let read_only_banner = adw::Banner::builder()
            .title(tr("Core Rules 2.0 is read-only"))
            .tooltip_text(tr(
                "Open an Expansion installation and import this content with Import Core Rules 2.0…",
            ))
            .revealed(true)
            .visible(false)
            .build();
        let entries_view = adw::ToolbarView::new();
        entries_view.add_top_bar(&entries_header);
        entries_view.add_top_bar(&read_only_banner);
        entries_view.add_top_bar(&search_bar);
        entries_view.set_content(Some(&entries_stack));
        let entries_page = adw::NavigationPage::builder()
            .title(tr("Entries"))
            .tag("entries")
            .child(&entries_view)
            .build();

        inner.set_sidebar(Some(&entries_page));
        inner.set_content(Some(&detail_page));
        let inner_page = adw::NavigationPage::builder()
            .title(tr("Entries"))
            .child(&inner)
            .build();

        // Sections.
        let sidebar_list = gtk::ListBox::builder()
            .css_classes(["navigation-sidebar"])
            .build();
        let sidebar_header = adw::HeaderBar::new();
        sidebar_header.pack_end(&open_menu());
        let sidebar_view = adw::ToolbarView::new();
        sidebar_view.add_top_bar(&sidebar_header);
        sidebar_view.set_content(Some(
            &gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .child(&sidebar_list)
                .build(),
        ));
        let sidebar_page = adw::NavigationPage::builder()
            .title("Core Rules")
            .child(&sidebar_view)
            .build();
        outer.set_sidebar(Some(&sidebar_page));
        outer.set_content(Some(&inner_page));

        // Behaviour.
        sidebar_list.connect_row_selected({
            let sections = sections.clone();
            let (model, search, inner) = (model.clone(), search.clone(), inner.clone());
            let filtered = filtered.clone();
            let (entries_page, inner_page, detail) =
                (entries_page.clone(), inner_page.clone(), detail.clone());
            let current = current.clone();
            let (party_filter, party_choices) = (party_filter.clone(), party_choices.clone());
            move |_, row| {
                let Some(row) = row else { return };
                let index = row.index() as usize;
                let sections = sections.borrow();
                let section = &sections[index];
                *current.borrow_mut() = index;
                let new = library::new_command(section.id);
                new_button.set_visible(new.is_some());
                if let Some((action, tooltip)) = new {
                    new_button.set_action_name(Some(action.detailed));
                    new_button.set_tooltip_text(Some(&tooltip));
                }
                origin_filter.set_visible(records::has_origins(section.id));
                party_filter
                    .set_visible(section.id == "characters" && party_choices.borrow().len() > 1);
                type_filter.set_visible(section.id == OTHER_SECTION);
                magic_filter.set_visible(library::is_parts(section.id));
                entries_page.set_title(section.title);
                inner_page.set_title(section.title);
                search.set_text("");
                let rows: Vec<glib::BoxedAnyObject> = section
                    .entries
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        glib::BoxedAnyObject::new(Row {
                            index: i,
                            title: e.title.clone(),
                            subtitle: e.subtitle.clone(),
                        })
                    })
                    .collect();
                *section_title.borrow_mut() = section.title.to_owned();
                model.splice(0, model.n_items(), &rows);
                show_empty(filtered.n_items());
                detail.show_placeholder(&match &section.error {
                    Some(error) => {
                        placeholder_text(tr("Could Not Read"), "dialog-error-symbolic", error)
                    }
                    None if section.entries.is_empty() => {
                        placeholder(&tr!("No {section}", section = section.title), section.icon)
                    }
                    None => placeholder(tr("Nothing Selected"), section.icon),
                });
                inner.set_show_content(false);
            }
        });
        // Selecting with the keyboard only changes the list; clicking or
        // pressing Enter also moves to it when the panes are collapsed.
        sidebar_list.connect_row_activated({
            let outer = outer.clone();
            move |_, _| outer.set_show_content(true)
        });
        list.connect_activate({
            let (sections, inner, detail) = (sections.clone(), inner.clone(), detail.clone());
            move |list, position| {
                let Some(item) = list
                    .model()
                    .and_then(|m| m.item(position))
                    .and_downcast::<glib::BoxedAnyObject>()
                else {
                    return;
                };
                let sections = sections.borrow();
                let entry = &sections[*current.borrow()].entries[item.borrow::<Row>().index];
                detail.show(entry);
                inner.set_show_content(true);
            }
        });
        Self {
            outer,
            inner,
            sidebar_list,
            sections,
            selection,
            detail,
            edit_button,
            copy_button,
            sheet_button,
            convert_button,
            npc_button,
            delete_button,
            generate_button,
            view_button,
            print_button,
            current_key,
            party_filter,
            party_choices,
            read_only_banner,
        }
    }

    /// Shows a Core Rules 2.0 installation without the ways to change it.
    fn set_read_only(&self, read_only: bool) {
        self.detail.read_only.set(read_only);
        // Shown without the reveal animation, which leaves the list
        // under the search bar until the next relayout.
        self.read_only_banner.set_visible(read_only);
    }

    /// Shows the sections of an installation. With `key`, the installation
    /// was reloaded after an edit: the same section stays selected and the
    /// entry with that key stays on the detail page.
    fn load(&self, sections: Vec<Section>, key: Option<&str>) {
        let selected = self
            .sidebar_list
            .selected_row()
            .map_or(0, |row| row.index());
        self.sidebar_list.remove_all();
        for section in &sections {
            self.sidebar_list.append(&section_row(section));
        }
        self.load_parties(&sections);
        *self.sections.borrow_mut() = sections;
        let selected = if key.is_some() { selected } else { 0 };
        if let Some(row) = self.sidebar_list.row_at_index(selected) {
            self.sidebar_list.select_row(Some(&row));
        }
        let Some(key) = key else {
            self.outer.set_show_content(false);
            return;
        };
        let sections = self.sections.borrow();
        let entries = sections.get(selected as usize).map(|s| &s.entries[..]);
        if let Some((position, entry)) = entries
            .unwrap_or_default()
            .iter()
            .enumerate()
            .find(|(_, e)| e.key == key)
        {
            // The search was cleared, but the type and origin filters may
            // still hide entries before it.
            let shown = (0..self.selection.n_items()).find(|&i| {
                self.selection
                    .item(i)
                    .and_downcast::<glib::BoxedAnyObject>()
                    .is_some_and(|item| item.borrow::<Row>().index == position)
            });
            if let Some(shown) = shown {
                self.selection.set_selected(shown);
            }
            self.detail.show(entry);
            self.inner.set_show_content(true);
        }
    }

    /// Fills the party filter with the characters' parties, keeping the
    /// choice when its party is still there.
    fn load_parties(&self, sections: &[Section]) {
        let choices = library::parties(sections);
        let chosen = self
            .party_choices
            .borrow()
            .get(self.party_filter.selected() as usize)
            .cloned()
            .flatten();
        let position = library::party_position(&choices, chosen.as_deref());
        let names: Vec<&str> = choices
            .iter()
            .map(|c| library::party_label(c.as_deref()))
            .collect();
        let model = gtk::StringList::new(&names);
        *self.party_choices.borrow_mut() = choices;
        self.party_filter.set_model(Some(&model));
        self.party_filter.set_selected(position as u32);
    }

    /// Selects the section with this id.
    fn show_section(&self, id: &str) {
        let index = self.sections.borrow().iter().position(|s| s.id == id);
        if let Some(row) = index.and_then(|i| self.sidebar_list.row_at_index(i as i32)) {
            self.sidebar_list.select_row(Some(&row));
        }
    }

    /// Collapses the entry list over the details first, then the sidebar.
    fn breakpoints(&self, window: &adw::ApplicationWindow) {
        let narrow = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            860.0,
            adw::LengthUnit::Sp,
        ));
        narrow.add_setter(&self.inner, "collapsed", Some(&true.to_value()));
        let phone = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            560.0,
            adw::LengthUnit::Sp,
        ));
        phone.add_setter(&self.inner, "collapsed", Some(&true.to_value()));
        phone.add_setter(&self.outer, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(narrow);
        window.add_breakpoint(phone);
    }
}

/// What the entry list shows for one entry.
struct Row {
    /// Its place in its section.
    index: usize,
    title: String,
    subtitle: String,
}

fn row_factory() -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let title = gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let subtitle = gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["caption", "dim-label"])
            .build();
        let column = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();
        column.append(&title);
        column.append(&subtitle);
        item.set_child(Some(&column));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = item.item().and_downcast::<glib::BoxedAnyObject>().unwrap();
        let row = row.borrow::<Row>();
        let column = item.child().and_downcast::<gtk::Box>().unwrap();
        let title = column.first_child().and_downcast::<gtk::Label>().unwrap();
        let subtitle = column.last_child().and_downcast::<gtk::Label>().unwrap();
        title.set_label(&row.title);
        subtitle.set_label(&row.subtitle);
        subtitle.set_visible(!row.subtitle.is_empty());
    });
    factory
}

fn section_row(section: &Section) -> gtk::ListBoxRow {
    let content = gtk::Box::builder().spacing(12).build();
    content.append(&gtk::Image::from_icon_name(section.icon));
    content.append(
        &gtk::Label::builder()
            .label(section.title)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build(),
    );
    let count = if section.error.is_some() {
        "!".to_owned()
    } else {
        section.entries.len().to_string()
    };
    content.append(
        &gtk::Label::builder()
            .label(count)
            .css_classes(["dim-label", "numeric"])
            .build(),
    );
    gtk::ListBoxRow::builder().child(&content).build()
}

fn placeholder(title: &str, icon: &str) -> gtk::Widget {
    adw::StatusPage::builder()
        .title(title)
        .icon_name(icon)
        .css_classes(["compact"])
        .build()
        .upcast()
}

fn placeholder_text(title: &str, icon: &str, text: &str) -> gtk::Widget {
    adw::StatusPage::builder()
        .title(title)
        .icon_name(icon)
        .description(glib::markup_escape_text(text).as_str())
        .build()
        .upcast()
}

fn detail(entry: &Entry, full_screen: Option<std::path::PathBuf>) -> gtk::Widget {
    let page = adw::PreferencesPage::new();

    let heading = adw::PreferencesGroup::new();
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .build();
    column.append(
        &gtk::Label::builder()
            .label(&entry.title)
            .xalign(0.0)
            .wrap(true)
            .css_classes(["title-1"])
            .build(),
    );
    if !entry.subtitle.is_empty() {
        column.append(
            &gtk::Label::builder()
                .label(&entry.subtitle)
                .xalign(0.0)
                .wrap(true)
                .css_classes(["dim-label"])
                .build(),
        );
    }
    match entry.icon.as_ref().and_then(picture::icon_texture) {
        Some(texture) => {
            let row = gtk::Box::builder().spacing(18).build();
            let icon = gtk::Image::builder()
                .paintable(&texture)
                .pixel_size(ICON_SIZE as i32)
                .valign(gtk::Align::Start)
                .build();
            icon.update_property(&[gtk::accessible::Property::Label(tr("Icon"))]);
            row.append(&icon);
            column.set_hexpand(true);
            column.set_valign(gtk::Align::Center);
            row.append(&column);
            heading.add(&row);
        }
        None => heading.add(&column),
    }
    page.add(&heading);
    if library::is_monster(&entry.key)
        && let Some(group) = entry
            .picture
            .as_deref()
            .and_then(|p| picture::monster_group(p, entry.frames.as_ref(), full_screen))
    {
        page.add(&group);
    } else if let Some(picture) = entry
        .picture
        .as_deref()
        .and_then(|p| sheet::picture(p, 240))
    {
        let group = adw::PreferencesGroup::new();
        group.add(&picture);
        page.add(&group);
    }
    if let Some(group) = books::detail_group(&entry.key) {
        page.add(&group);
    }

    for group in &entry.groups {
        if group.rows.is_empty() {
            continue;
        }
        let widget = adw::PreferencesGroup::builder()
            .title(glib::markup_escape_text(&group.title).as_str())
            .build();
        for (key, value) in &group.rows {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(key).as_str())
                .subtitle(glib::markup_escape_text(value).as_str())
                .subtitle_selectable(true)
                .build();
            // A row without a value is a list item, not a property.
            if !value.is_empty() {
                row.add_css_class("property");
            }
            widget.add(&row);
        }
        page.add(&widget);
    }

    let text = library::clean_text(&entry.text);
    if !text.is_empty() {
        let group = adw::PreferencesGroup::builder()
            .title(tr("Description"))
            .build();
        group.add(
            &gtk::Label::builder()
                .label(&text)
                .xalign(0.0)
                .wrap(true)
                .wrap_mode(gtk::pango::WrapMode::WordChar)
                .selectable(true)
                .css_classes(["body"])
                .build(),
        );
        page.add(&group);
    }
    page.upcast()
}

/// A dialog for the details of a character. `save` is called with the
/// edited details; the dialog closes when it succeeds and shows its error
/// otherwise.
fn character_editor(
    parent: &impl IsA<gtk::Widget>,
    root: PathBuf,
    details: &CharacterDetails,
    lists: &CharacterLists,
    classes: &[String],
    choices: Choices,
    save: impl Fn(&CharacterDetails, &CharacterLists) -> Result<(), String> + 'static,
) {
    let entry_row =
        |title: &str, text: &str| adw::EntryRow::builder().title(title).text(text).build();
    let ranged_row = |title: &str, subtitle: &str, value: u32, min: u32, max: u32| {
        adw::SpinRow::builder()
            .title(title)
            .subtitle(subtitle)
            .numeric(true)
            .adjustment(&gtk::Adjustment::new(
                value.into(),
                min.into(),
                max.into(),
                1.0,
                5.0,
                0.0,
            ))
            .build()
    };
    let spin_row = |title: &str, subtitle: &str, value: u32, max: u32| {
        ranged_row(title, subtitle, value, 0, max)
    };
    let name = entry_row(tr("Name"), &details.name);
    let player = entry_row(tr("Player"), &details.player);
    let social_class = entry_row(
        tr("Social Class"),
        &social_class_shown(&details.social_class),
    );
    let age = spin_row(tr("Age"), tr("Years"), details.age, 9999);
    let height = spin_row(tr("Height"), tr("Inches"), details.height, 999);
    let weight = spin_row(tr("Weight"), tr("Pounds"), details.weight, 99999);

    // With the Core Rules one row sets the score and both subabilities, as
    // the original does; Skills & Powers characters get all three.
    let (low, high) = (*ABILITY_RANGE.start(), *ABILITY_RANGE.end());
    let abilities = adw::PreferencesGroup::builder()
        .title(tr("Abilities"))
        .build();
    let ability_rows: Vec<[adw::SpinRow; 3]> = ABILITY_NAMES
        .iter()
        .zip(details.abilities)
        .map(|(names, values)| {
            let rows = [0, 1, 2].map(|i| ranged_row(tr(names[i]), "", values[i], low, high));
            if details.skills_and_powers {
                let expander = adw::ExpanderRow::builder().title(tr(names[0])).build();
                let summarize = {
                    let (expander, rows) = (expander.clone(), rows.clone());
                    move || {
                        let [score, first, second] = rows.each_ref().map(|r| r.value() as u32);
                        expander.set_subtitle(&format!(
                            "{score} · {} {first}, {} {second}",
                            tr(names[1]),
                            tr(names[2])
                        ));
                    }
                };
                summarize();
                for row in &rows {
                    expander.add_row(row);
                    let summarize = summarize.clone();
                    row.connect_value_notify(move |_| summarize());
                }
                abilities.add(&expander);
            } else {
                abilities.add(&rows[0]);
            }
            rows
        })
        .collect();
    let exceptional = spin_row(
        tr("Exceptional Strength"),
        tr("18/01 to 18/00, 0 for none"),
        details.exceptional_strength,
        100,
    );
    abilities.add(&exceptional);
    // Only a warrior with Strength 18 has exceptional strength.
    let fit_exceptional = {
        let (rules, classes) = (
            choices.creation.clone(),
            choices.character.data.classes.names.clone(),
        );
        let (strength, exceptional) = (ability_rows[0][0].clone(), exceptional.clone());
        move || {
            let allowed = rules.has_exceptional_strength(&classes, strength.value() as u32);
            exceptional.set_sensitive(allowed);
            if !allowed {
                exceptional.set_value(0.0);
            }
        }
    };
    fit_exceptional();
    ability_rows[0][0].connect_value_notify(move |_| fit_exceptional());

    let (portrait_row, portrait) = picture::portrait_row(
        root,
        choices.character.data.race.clone(),
        details.portrait.clone(),
    );
    let character = adw::PreferencesGroup::new();
    character.add(&name);
    character.add(&player);
    character.add(&social_class);
    character.add(&portrait_row);

    // The Name page of Update Character: sex, alignment, kit (a new kit
    // rolls the social class again, on the kit's table) and Background….
    let with = |mut items: Vec<String>, current: &str| {
        if !items.iter().any(|i| i == current) {
            items.push(current.to_owned());
        }
        items
    };
    let sexes = [tr("Male").to_owned(), tr("Female").to_owned()];
    let sex = create::combo_row(tr("Gender"), &sexes, &sexes[usize::from(!details.male)]);
    let alignment = create::combo_row(
        tr("Alignment"),
        &with(choices.creation.alignments(classes), &details.alignment),
        &details.alignment,
    );
    let kit = create::combo_row(
        tr("Kit"),
        &with(choices.creation.all_kits(), &details.kit),
        &details.kit,
    );
    kit.set_enable_search(true);
    kit.set_expression(Some(gtk::PropertyExpression::new(
        gtk::StringObject::static_type(),
        None::<gtk::Expression>,
        "string",
    )));
    kit.connect_selected_notify({
        let (rules, social_class) = (choices.creation.clone(), social_class.clone());
        move |kit| {
            let chosen = create::combo_value(kit);
            let rolled = rules.roll_social_class(&chosen, &mut Default::default());
            social_class.set_text(&social_class_shown(&rolled));
            create::kit_information(kit, &rules, &chosen);
        }
    });
    let (event, history) =
        create::background_rows(&choices.backgrounds, &details.background, &details.history);
    character.add(&sex);
    let background = adw::PreferencesGroup::builder()
        .title(tr("Background"))
        .build();
    background.add(&alignment);
    background.add(&kit);
    background.add(&event);
    let history_group = adw::PreferencesGroup::builder()
        .title(tr("History"))
        .build();
    history_group.add(&history);
    let appearance = adw::PreferencesGroup::builder()
        .title(tr("Appearance"))
        .build();
    appearance.add(&age);
    appearance.add(&height);
    appearance.add(&weight);
    let (class_group, new_classes, refresh_classes) =
        classes_group(classes, &choices.class_table, &choices.character);
    let advanced = Rc::new(RefCell::new(choices.character.clone()));
    // Add Class follows the abilities as edited, not as saved.
    {
        let edited = Rc::new({
            let (advanced, rows) = (advanced.clone(), ability_rows.clone());
            move || {
                let mut character = advanced.borrow().clone();
                character.data.strength[0] = rows[0][0].value() as u32;
                for (slot, rows) in rows.iter().enumerate().skip(1) {
                    character.data.abilities[slot - 1][0] = rows[0].value() as u32;
                }
                character
            }
        });
        for rows in &ability_rows {
            let (edited, refresh) = (edited.clone(), refresh_classes.clone());
            rows[0].connect_value_notify(move |_| refresh(&edited()));
        }
    }
    let level_listeners: Listeners = Rc::default();
    let experience = experience_group(&choices.advance, &advanced, &level_listeners);
    let page = adw::PreferencesPage::new();
    page.add(&character);
    page.add(&background);
    page.add(&class_group);
    page.add(&experience);
    page.add(&appearance);
    page.add(&abilities);
    page.add(&history_group);

    let kit_magic_editor = kit_magic::KitMagicEditor::new(
        choices.kit_magic.clone(),
        &choices.character,
        Rc::new({
            let kit = kit.clone();
            move || create::combo_value(&kit)
        }),
    );
    kit.connect_selected_notify({
        let editor = kit_magic_editor.clone();
        move |_| editor.rebuild()
    });
    let editors = ListEditors::new(lists, &choices, classes, &advanced, &kit_magic_editor);
    level_listeners.borrow_mut().push(Box::new({
        let editor = editors.nonweapon_proficiencies.clone();
        move || editor.levels_changed()
    }));
    // Memorized spells against the slots of the levels the character has
    // now, as the original's memorize dialog counts them.
    for (slot, _, list) in &editors.memorized {
        let show = {
            let (list, rules, advanced, slot) = (
                list.clone(),
                choices.advance.clone(),
                advanced.clone(),
                *slot,
            );
            move || {
                let slots = rules.spell_slots(&advanced.borrow(), slot).ok().flatten();
                list.group
                    .set_description(slots_text(slots, &spell_levels(&list)).as_deref());
            }
        };
        show();
        list.connect_changed(show.clone());
        level_listeners.borrow_mut().push(Box::new(show));
        // A spell past the level's slots is refused.
        list.set_limit(
            {
                let (list, rules, advanced, slot) = (
                    list.clone(),
                    choices.advance.clone(),
                    advanced.clone(),
                    *slot,
                );
                move || {
                    let slots = rules.spell_slots(&advanced.borrow(), slot).ok().flatten();
                    slots_fit(slots, &spell_levels(&list))
                }
            },
            {
                let group = list.group.clone();
                move || {
                    create::message(
                        &group,
                        tr("Not Allowed"),
                        tr("There are no more spell slots of this level to memorize it in."),
                    )
                }
            },
        );
    }
    let ability_editor = Rc::new(abilities::AbilityEditor::new(
        choices.abilities.clone(),
        &choices.character,
    ));
    let show_notices = ability_editor.show_kit_notices(Rc::new({
        let editor = kit_magic_editor.clone();
        move |list, taken| editor.notices(list, taken)
    }));
    show_notices();
    kit_magic_editor.connect_changed(move || show_notices());
    let levels: combat::Levels = Rc::new({
        let (rules, advanced) = (choices.advance.clone(), advanced.clone());
        move || {
            rules
                .levels(&advanced.borrow())
                .map_or_else(|_| vec![1], |l| l.iter().map(|c| c.level).collect())
        }
    });
    let combat_editor =
        combat::CombatEditor::new(choices.combat.clone(), &choices.character, levels.clone());
    // Abilities and Combat spend the same character points.
    {
        let combat = combat_editor.clone();
        let abilities = Rc::downgrade(&ability_editor);
        *ability_editor.link.others.borrow_mut() = Rc::new({
            let combat = combat.clone();
            move || combat.spent()
        });
        *ability_editor.link.changed.borrow_mut() = Rc::new(move || combat.show_points());
        *combat_editor.link.others.borrow_mut() = Rc::new({
            let abilities = abilities.clone();
            move || abilities.upgrade().map_or(0, |a| a.spent())
        });
        *combat_editor.link.changed.borrow_mut() = Rc::new(move || {
            if let Some(a) = abilities.upgrade() {
                a.show_points();
            }
        });
        ability_editor.show_points();
        combat_editor.show_points();
    }
    kit.connect_selected_notify({
        let combat = combat_editor.clone();
        move |kit| combat.kit_changed(&create::combo_value(kit))
    });
    level_listeners.borrow_mut().push(Box::new({
        let editor = combat_editor.clone();
        move || editor.levels_changed()
    }));
    // Test for Wild Talents reads the abilities as edited and may lower
    // Constitution, Intelligence or Wisdom (`data.abilities[1..4]`, rows 2
    // to 4).
    let wild_hooks = skills::WildHooks {
        character: Rc::new({
            let (advanced, rows) = (advanced.clone(), ability_rows.clone());
            let skills_and_powers = details.skills_and_powers;
            move || {
                let mut character = advanced.borrow().clone();
                for (slot, [score, first, second]) in rows.iter().enumerate().skip(1) {
                    let score = score.value() as u32;
                    character.data.abilities[slot - 1] = if skills_and_powers {
                        [score, first.value() as u32, second.value() as u32]
                    } else {
                        [score; 3]
                    };
                }
                character
            }
        }),
        set_abilities: Rc::new({
            let rows = ability_rows.clone();
            move |abilities| {
                for (rows, values) in rows[2..5].iter().zip(abilities) {
                    for (row, value) in rows.iter().zip(values) {
                        row.set_value(f64::from(value));
                    }
                }
            }
        }),
    };
    let skills_editor =
        skills::SkillsEditor::new(choices.skills.clone(), &choices.character, wild_hooks);
    // A spell picked is checked as the original's Learn does (0x62dc71),
    // for the first class that casts spells of its kind.
    let dice = Rc::new(RefCell::new(cr_store::create::Dice::default()));
    for (list, wizard) in [
        (&editors.wizard_spells, true),
        (&editors.priest_spells, false),
    ] {
        let learner = Rc::new({
            let (advanced, levels, kit) = (advanced.clone(), levels.clone(), kit.clone());
            let (rules, ids, abilities) = (
                choices.advance.clone(),
                choices.class_ids.clone(),
                ability_editor.clone(),
            );
            let (wizard_spells, priest_spells) =
                (editors.wizard_spells.clone(), editors.priest_spells.clone());
            move || {
                let character = advanced.borrow();
                let kind = if wizard {
                    Caster::Wizard
                } else {
                    Caster::Priest
                };
                let names = &character.data.classes.names;
                let slot = names
                    .iter()
                    .position(|n| ids.get(n).copied().map(caster) == Some(kind));
                let (class, level, class_abilities) = match slot {
                    Some(g) => (
                        names[g].clone(),
                        levels().get(g).copied().unwrap_or(1),
                        abilities
                            .abilities()
                            .classes
                            .get(g)
                            .map(|c| c.abilities.clone())
                            .unwrap_or_default(),
                    ),
                    None => (String::new(), 0, Vec::new()),
                };
                let (reason, knowledge) = Learner::intelligence(&character);
                Learner {
                    castable: rules.castable(&class, level, &class_abilities),
                    class,
                    abilities: class_abilities,
                    kit: create::combo_value(&kit),
                    reason,
                    knowledge,
                    wizard_spells: wizard_spells.items(),
                    priest_spells: priest_spells.items(),
                    entering: false,
                }
            }
        });
        // The chance to learn each spell offered (0x630052).
        list.set_detail({
            let (rules, learner) = (choices.learn.clone(), learner.clone());
            move |name| rules.chance_text(&learner(), name, wizard)
        });
        list.connect_add({
            let (rules, dice, group) = (choices.learn.clone(), dice.clone(), list.group.clone());
            move |name, add| {
                create::learn_spell(
                    group.upcast_ref(),
                    &rules,
                    &learner(),
                    (name, wizard),
                    &mut dice.borrow_mut(),
                    add,
                );
            }
        });
    }
    // Buying an item the character may not use asks first, as the
    // original's Update Character does (0x43d7bc in its mode).
    let item_check = if let Some(check) = choices.item_rules.clone() {
        let user = Rc::new({
            let (advanced, levels, kit) = (advanced.clone(), levels.clone(), kit.clone());
            let (ability_editor, combat_editor) = (ability_editor.clone(), combat_editor.clone());
            move || {
                let character = advanced.borrow();
                let d = &character.data;
                cr_store::npc::ItemUser {
                    race: d.race.clone(),
                    classes: d
                        .classes
                        .names
                        .iter()
                        .cloned()
                        .zip(levels().into_iter().map(|l| l as i32))
                        .collect(),
                    skills_and_powers: character.skills_and_powers(),
                    kit: create::combo_value(&kit),
                    class_abilities: ability_editor
                        .abilities()
                        .classes
                        .into_iter()
                        .map(|c| c.abilities)
                        .collect(),
                    weapon_proficiencies: combat_editor.combat().weapons,
                    focus_weapon: d
                        .unknown_0x1e68
                        .as_ref()
                        .map(|t| t[6].clone())
                        .unwrap_or_default(),
                }
            }
        });
        Some(
            Rc::new(move |group: &adw::PreferencesGroup, name: &str, add| {
                match check.item_problem(&user(), name, false) {
                    Some(text) => create::item_error(group, &text, add),
                    None => add(),
                }
            }) as ItemCheck,
        )
    } else {
        None
    };
    editors.connect_trade(choices.creation.clone(), kit.clone(), item_check);
    let stack = adw::ViewStack::new();
    stack.add_titled_with_icon(
        &page,
        Some("details"),
        tr("Details"),
        "avatar-default-symbolic",
    );
    stack.add_titled_with_icon(
        &ability_editor.page,
        Some("abilities"),
        tr("Abilities"),
        "emblem-system-symbolic",
    );
    stack.add_titled_with_icon(
        &combat_editor.page,
        Some("combat"),
        tr("Combat"),
        "security-high-symbolic",
    );
    stack.add_titled_with_icon(
        &editors.proficiencies_page(),
        Some("proficiencies"),
        tr("Proficiencies"),
        "view-list-symbolic",
    );
    stack.add_titled_with_icon(
        &skills_editor.page,
        Some("skills"),
        tr("Skills"),
        "system-run-symbolic",
    );
    kit_magic_editor.set_rest(editors.spell_groups());
    stack.add_titled_with_icon(
        &kit_magic_editor.page,
        Some("spells"),
        tr("Spells"),
        "starred-symbolic",
    );
    stack.add_titled_with_icon(
        &editors.equipment_page(),
        Some("equipment"),
        tr("Equipment"),
        "package-x-generic-symbolic",
    );
    // The kit trims the schools and spheres once the Spells page is shown,
    // as the original's Spells and Memorize pages do.
    let spells_shown = Rc::new(std::cell::Cell::new(false));
    // The kit's text before the character is equipped (494875), once.
    let equipped = std::cell::Cell::new(false);
    stack.connect_visible_child_name_notify({
        let (spells_shown, rules, kit) =
            (spells_shown.clone(), choices.creation.clone(), kit.clone());
        move |stack| match stack.visible_child_name().as_deref() {
            Some("spells") => spells_shown.set(true),
            Some("equipment") if !equipped.replace(true) => {
                let name = create::combo_value(&kit);
                if let Some(text) = rules.kit_equipment_text(&name) {
                    create::message(stack, &name, text);
                }
            }
            _ => {}
        }
    });
    let error = adw::Banner::new("");

    let cancel = gtk::Button::with_mnemonic(tr("_Cancel"));
    let save_button = gtk::Button::builder()
        .label(tr("_Save"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let header = adw::HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .build();
    header.pack_start(&cancel);
    header.pack_end(&save_button);
    // The original's Update Character pages, by the page shown.
    help::add_button(&header, &stack, {
        let stack = stack.clone();
        move || match stack.visible_child_name().as_deref() {
            Some("abilities") => topic::ABILITY_SCORES,
            Some("combat") => topic::WEAPON_PROFICIENCIES,
            Some("proficiencies") => topic::NONWEAPON_PROFICIENCIES,
            Some("skills") => topic::SKILLS,
            Some("spells") => topic::CHARACTER_SPELLS,
            Some("equipment") => topic::EQUIPMENT,
            _ => topic::CHARACTER_INFORMATION,
        }
    });
    let switcher = adw::ViewSwitcherBar::builder()
        .stack(&stack)
        .reveal(true)
        .build();
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.add_top_bar(&error);
    view.set_content(Some(&stack));
    view.add_bottom_bar(&switcher);
    let dialog = adw::Dialog::builder()
        .title(tr("Edit Character"))
        .content_width(560)
        .content_height(600)
        .child(&view)
        .default_widget(&save_button)
        .build();

    name.connect_changed({
        let save_button = save_button.clone();
        move |name| save_button.set_sensitive(!name.text().trim().is_empty())
    });
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    let skills_and_powers = details.skills_and_powers;
    let original = choices.character.clone();
    let ability_rules = choices.abilities.clone();
    let combat_rules = choices.combat.clone();
    let skill_rules = choices.skills.clone();
    let kit_magic_rules = choices.kit_magic.clone();
    // Leaving the original's Weapon Proficiencies page checks the kit's
    // specializations first.
    // Leaving the original's Nonweapon Proficiencies page asks about the
    // proficiency the kit has the character learn by a level; Yes stays.
    let nonweapon_editor = editors.nonweapon_proficiencies.clone();
    let save_all: Rc<dyn Fn()> = Rc::new({
        let dialog = dialog.clone();
        let combat_editor = combat_editor.clone();
        move || {
            let details = CharacterDetails {
                name: name.text().trim().to_owned(),
                player: player.text().trim().to_owned(),
                social_class: social_class_stored(social_class.text().trim()),
                age: age.value() as u32,
                height: height.value() as u32,
                weight: weight.value() as u32,
                abilities: std::array::from_fn(|i| {
                    let [score, first, second] = &ability_rows[i];
                    if skills_and_powers {
                        [score, first, second].map(|row| row.value() as u32)
                    } else {
                        [score.value() as u32; 3]
                    }
                }),
                exceptional_strength: exceptional.value() as u32,
                skills_and_powers,
                portrait: portrait.borrow().clone(),
                alignment: create::combo_value(&alignment),
                male: sex.selected() == 0,
                kit: create::combo_value(&kit),
                background: create::combo_value(&event),
                history: create::text_of(&history),
            };
            let mut lists = editors.lists();
            lists.new_classes = new_classes.borrow().clone();
            let edited = ability_editor.abilities();
            if edited != ability_rules.abilities_of(&original) {
                lists.abilities = Some(edited);
            }
            let edited = combat_editor.combat();
            if edited != combat_rules.combat_of(&original) {
                lists.weapon_proficiencies.clone_from(&edited.weapons);
                lists.combat = Some(edited);
            }
            let edited = skills_editor.skills();
            if edited != skill_rules.skills_of(&original) {
                lists.skills = Some(edited);
            }
            let magic = kit_magic_editor.magic();
            if spells_shown.get() || magic != kit_magic_rules.magic_of(&original) {
                lists.kit_magic = Some(magic);
            }
            let advancement = Advancement::between(&original, &advanced.borrow());
            if advancement != Advancement::of(&original) {
                lists.advancement = Some(advancement);
            }
            match save(&details, &lists) {
                Ok(()) => {
                    dialog.close();
                }
                Err(message) => {
                    error.set_title(&glib::markup_escape_text(&message));
                    error.set_revealed(true);
                }
            }
        }
    });
    let stay: Rc<dyn Fn()> = Rc::new({
        let stack = stack.clone();
        move || stack.set_visible_child_name("proficiencies")
    });
    save_button.connect_clicked(move |_| {
        let (nonweapon_editor, stay, save_all) =
            (nonweapon_editor.clone(), stay.clone(), save_all.clone());
        combat_editor.check_leaving(Rc::new(move || {
            nonweapon_editor.check_leaving(stay.clone(), save_all.clone())
        }))
    });
    dialog.present(Some(parent));
}

/// Checks the classes offered again against a character as edited.
type Recheck = Rc<dyn Fn(&Character)>;

/// The character's classes, with an Add button for one more class when
/// the race may dual-class. Race and classes themselves cannot change.
/// The returned closure checks the dual-class rules again against the
/// abilities as edited, and takes back an added class they no longer
/// allow.
fn classes_group(
    classes: &[String],
    table: &Rc<ClassTable>,
    character: &Character,
) -> (adw::PreferencesGroup, Rc<RefCell<Vec<String>>>, Recheck) {
    let add = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text(tr("Add Class"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let group = adw::PreferencesGroup::builder()
        .title(tr("Classes"))
        .header_suffix(&add)
        .build();
    for class in classes {
        group.add(
            &adw::ActionRow::builder()
                .title(glib::markup_escape_text(class).as_str())
                .build(),
        );
    }
    let added: Rc<RefCell<Vec<String>>> = Rc::default();
    let added_row: Rc<RefCell<Option<adw::ActionRow>>> = Rc::default();
    let offered: Rc<RefCell<Vec<String>>> = Rc::default();
    let refresh: Recheck = Rc::new({
        let (table, group, add) = (table.clone(), group.clone(), add.clone());
        let (added, added_row, offered) = (added.clone(), added_row.clone(), offered.clone());
        move |character| {
            let offer = cr_app::editor::class_offer(&table, character);
            group.set_description(Some(&glib::markup_escape_text(&offer.description)));
            let names = offer.names;
            let kept = added.borrow().iter().all(|n| names.contains(n));
            if !kept {
                added.borrow_mut().clear();
                if let Some(row) = added_row.borrow_mut().take() {
                    group.remove(&row);
                }
            }
            add.set_sensitive(!names.is_empty() && added.borrow().is_empty());
            *offered.borrow_mut() = names;
        }
    });
    refresh(character);
    add.connect_clicked({
        let (group, added, added_row) = (group.clone(), added.clone(), added_row.clone());
        move |button| {
            let (group, added, added_row, button) = (
                group.clone(),
                added.clone(),
                added_row.clone(),
                button.clone(),
            );
            pick(
                &button.clone(),
                tr("Classes"),
                offered.borrow().clone(),
                move |name| {
                    let remove = gtk::Button::builder()
                        .icon_name("list-remove-symbolic")
                        .tooltip_text(tr("Remove"))
                        .valign(gtk::Align::Center)
                        .css_classes(["flat"])
                        .build();
                    let row = adw::ActionRow::builder()
                        .title(glib::markup_escape_text(&name).as_str())
                        .subtitle(tr("New, level 1"))
                        .build();
                    row.add_suffix(&remove);
                    group.add(&row);
                    added.borrow_mut().push(name);
                    *added_row.borrow_mut() = Some(row.clone());
                    // One class at a time: the next one depends on this one.
                    button.set_sensitive(false);
                    remove.connect_clicked({
                        let (group, added, added_row, button) = (
                            group.clone(),
                            added.clone(),
                            added_row.clone(),
                            button.clone(),
                        );
                        move |_| {
                            group.remove(&row);
                            added.borrow_mut().clear();
                            added_row.borrow_mut().take();
                            button.set_sensitive(true);
                        }
                    });
                },
            );
        }
    });
    (group, added, refresh)
}

/// Experience, levels, hit points and PSPs, with the Name page's
/// "Level..." button (1613), which opens the original's Levels dialog in
/// Update too, and a button to give or take away experience. Changes go
/// to `advanced`, a copy of the character that the save compares with the
/// character as read.
fn experience_group(
    rules: &Arc<AdvanceRules>,
    advanced: &Rc<RefCell<Character>>,
    listeners: &Listeners,
) -> adw::PreferencesGroup {
    let levels = gtk::Button::builder()
        .label(tr("Levels…"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let add = gtk::Button::builder()
        .label(tr("Add…"))
        .tooltip_text(tr("Add or take away experience"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.append(&levels);
    buttons.append(&add);
    let group = adw::PreferencesGroup::builder()
        .title(tr("Experience"))
        .header_suffix(&buttons)
        .build();
    let character = advanced.borrow().clone();
    let class_rows: Vec<adw::ActionRow> = character
        .data
        .classes
        .names
        .iter()
        .map(|name| {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(name).as_str())
                .build();
            group.add(&row);
            row
        })
        .collect();
    // The Name page's Hit Points arrows: a change moves the stored rolls
    // (see AdvanceRules::adjust_hit_points), then shows the hit points
    // they give.
    let hit_points = adw::SpinRow::builder()
        .title(tr("Hit Points"))
        .numeric(true)
        .adjustment(&gtk::Adjustment::new(0.0, 0.0, 99_999.0, 1.0, 10.0, 0.0))
        .build();
    // The Name page's PSP arrows (2373, 0 to 999): a change goes to the
    // highest level that has PSPs.
    let psionic = adw::SpinRow::builder()
        .title(tr("PSPs"))
        .subtitle(tr("Changes those of the highest level that has them"))
        .numeric(true)
        .adjustment(&gtk::Adjustment::new(0.0, 0.0, 999.0, 1.0, 10.0, 0.0))
        .build();
    let showing = Rc::new(std::cell::Cell::new(false));
    group.add(&hit_points);
    group.add(&psionic);
    let points = character.skills_and_powers().then(|| {
        let label = gtk::Label::builder()
            .css_classes(["numeric"])
            .valign(gtk::Align::Center)
            .build();
        let row = adw::ActionRow::builder()
            .title(tr("Character Points"))
            .subtitle(tr("Unspent"))
            .build();
        row.add_suffix(&label);
        group.add(&row);
        label
    });
    let refresh = Rc::new({
        let (rules, advanced, listeners) = (rules.clone(), advanced.clone(), listeners.clone());
        let (hit_points, psionic, showing) = (hit_points.clone(), psionic.clone(), showing.clone());
        move || {
            for listener in listeners.borrow().iter() {
                listener();
            }
            let character = advanced.borrow();
            match rules.levels(&character) {
                Ok(levels) => {
                    for (row, class) in class_rows.iter().zip(levels) {
                        row.set_subtitle(&level_text(&class));
                    }
                }
                Err(e) => group_error(&class_rows, &e.to_string()),
            }
            showing.set(true);
            match rules.hit_points(&character) {
                Ok(hp) => {
                    hit_points.set_sensitive(true);
                    hit_points.set_value(f64::from(hp));
                }
                Err(_) => hit_points.set_sensitive(false),
            }
            psionic.set_value(f64::from(psionic_total(&character)));
            showing.set(false);
            psionic.set_visible(
                character
                    .data
                    .classes
                    .names
                    .iter()
                    .any(|c| rules.psionicist(c))
                    || psionic_total(&character) > 0,
            );
            if let Some(points) = &points {
                points.set_label(&character.data.unknown_0x1778[0].to_string());
            }
        }
    });
    refresh();
    hit_points.connect_value_notify({
        let (rules, advanced, refresh) = (rules.clone(), advanced.clone(), refresh.clone());
        let showing = showing.clone();
        move |row| {
            if showing.get() {
                return;
            }
            let wanted = row.value() as i32;
            let changed = {
                let mut character = advanced.borrow_mut();
                rules
                    .hit_points(&character)
                    .and_then(|hp| rules.adjust_hit_points(&mut character, wanted - hp))
            };
            if changed.is_ok() {
                refresh();
            }
        }
    });
    psionic.connect_value_notify({
        let (advanced, refresh) = (advanced.clone(), refresh.clone());
        move |row| {
            if showing.get() {
                return;
            }
            {
                let mut character = advanced.borrow_mut();
                let delta = row.value() as i64 - i64::from(psionic_total(&character));
                adjust_psionic_points(&mut character, delta as i32);
            }
            refresh();
        }
    });
    // The Levels dialog opens with the prime requisite bonus on (0x617a71)
    // and 3 character points a level, every time.
    levels.connect_clicked({
        let (rules, advanced, refresh) = (rules.clone(), advanced.clone(), refresh.clone());
        move |button| {
            let refresh = refresh.clone();
            enter::levels_dialog(button, &rules, &advanced, (true, 3), move |_, _| refresh());
        }
    });
    add.connect_clicked({
        let (rules, advanced) = (rules.clone(), advanced.clone());
        move |button| {
            let refresh = refresh.clone();
            experience_dialog(button, &rules, &advanced, None, move || refresh());
        }
    });
    group
}

pub(crate) use cr_app::levels::level_text;
use cr_app::levels::{ExperienceForm, LevelsInput};

/// Callbacks run when the character's levels change.
type Listeners = Rc<RefCell<Vec<Box<dyn Fn()>>>>;

/// The levels of the spells a memorized list holds.
fn spell_levels(list: &RowList<u32>) -> Vec<u32> {
    list.values().into_iter().map(|(level, _)| level).collect()
}

fn group_error(rows: &[adw::ActionRow], message: &str) {
    for row in rows {
        row.set_subtitle(&glib::markup_escape_text(message));
    }
}

/// Asks how much experience to give to which classes, then gives it;
/// `fixed` are the prime requisite bonus and the character points per
/// level when the Levels dialog sets them.
pub(crate) fn experience_dialog(
    parent: &impl IsA<gtk::Widget>,
    rules: &Arc<AdvanceRules>,
    advanced: &Rc<RefCell<Character>>,
    fixed: Option<(bool, u32)>,
    changed: impl Fn() + 'static,
) {
    let character = advanced.borrow().clone();
    let names = character.data.classes.names.clone();
    let amount = adw::SpinRow::builder()
        .title(tr("Experience"))
        .subtitle(tr("Negative to take experience away"))
        .numeric(true)
        .adjustment(&gtk::Adjustment::new(
            1000.0,
            -10_000_000.0,
            10_000_000.0,
            100.0,
            1000.0,
            0.0,
        ))
        .build();
    let mut targets: Vec<String> = names.clone();
    if names.len() > 1 {
        targets.push(tr("All Classes").into());
        targets.push(tr("Split Between Classes").into());
    }
    let target = adw::ComboRow::builder()
        .title(tr("Give To"))
        .model(&gtk::StringList::new(
            &targets.iter().map(String::as_str).collect::<Vec<_>>(),
        ))
        .build();
    if names.len() > 1 {
        target.set_selected(names.len() as u32 + 1);
    }
    let prime = adw::SwitchRow::builder()
        .title(tr("Prime Requisite Bonus"))
        .subtitle(tr(
            "10% more for a class whose prime requisites are all 16 or more",
        ))
        .active(fixed.is_none_or(|f| f.0))
        .visible(fixed.is_none())
        .build();
    let points = adw::SpinRow::builder()
        .title(tr("Character Points per Level"))
        .numeric(true)
        .adjustment(&gtk::Adjustment::new(
            fixed.map_or(3.0, |f| f.1.into()),
            0.0,
            10.0,
            1.0,
            1.0,
            0.0,
        ))
        .visible(character.skills_and_powers() && fixed.is_none())
        .build();
    let group = adw::PreferencesGroup::builder()
        .description(tr(
            "Each level gained rolls its hit points. Levels lost keep their rolls, as in the original.",
        ))
        .build();
    group.add(&amount);
    group.add(&target);
    group.add(&prime);
    group.add(&points);
    let page = adw::PreferencesPage::new();
    page.add(&group);
    let cancel = gtk::Button::with_mnemonic(tr("_Cancel"));
    let give = gtk::Button::builder()
        .label(tr("_Add"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let header = adw::HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .build();
    header.pack_start(&cancel);
    header.pack_end(&give);
    let error = adw::Banner::new("");
    let view = adw::ToolbarView::new();
    help::add_button(&header, &view, || topic::LEVELS);
    view.add_top_bar(&header);
    view.add_top_bar(&error);
    view.set_content(Some(&page));
    let dialog = adw::Dialog::builder()
        .title(tr("Add Experience"))
        .content_width(420)
        .child(&view)
        .default_widget(&give)
        .build();
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    give.connect_clicked({
        let (dialog, rules, advanced) = (dialog.clone(), rules.clone(), advanced.clone());
        move |_| {
            let mut character = advanced.borrow().clone();
            let mut form = ExperienceForm::new(&character, None);
            form.input(LevelsInput::Amount(amount.value() as i64));
            form.input(LevelsInput::Target(target.selected() as usize));
            let settings = (prime.is_active(), points.value() as u32);
            let mut dice = cr_store::create::Dice::default();
            if form.give(&rules, &mut character, settings, &mut dice) {
                *advanced.borrow_mut() = character;
                changed();
                dialog.close();
            } else if let Some(e) = form.error {
                error.set_title(&glib::markup_escape_text(&e));
                error.set_revealed(true);
            }
        }
    });
    dialog.present(Some(parent));
}

/// Each ability with its two Skills & Powers subabilities.
const ABILITY_NAMES: [[&str; 3]; 6] = [
    [
        tr_noop!("Strength"),
        tr_noop!("Stamina"),
        tr_noop!("Muscle"),
    ],
    [tr_noop!("Dexterity"), tr_noop!("Aim"), tr_noop!("Balance")],
    [
        tr_noop!("Constitution"),
        tr_noop!("Health"),
        tr_noop!("Fitness"),
    ],
    [
        tr_noop!("Intelligence"),
        tr_noop!("Reason"),
        tr_noop!("Knowledge"),
    ],
    [
        tr_noop!("Wisdom"),
        tr_noop!("Intuition"),
        tr_noop!("Willpower"),
    ],
    [
        tr_noop!("Charisma"),
        tr_noop!("Leadership"),
        tr_noop!("Appearance"),
    ],
];

/// Names the list editors offer, from the installation's databases.
struct Choices {
    languages: Rc<Vec<String>>,
    wizard_spells: Rc<Vec<String>>,
    priest_spells: Rc<Vec<String>>,
    items: Rc<Vec<String>>,
    /// The items' icons, drawn on the equipment rows.
    icons: Rc<ItemIcons>,
    wizard_levels: Rc<HashMap<String, u32>>,
    priest_levels: Rc<HashMap<String, u32>>,
    class_ids: HashMap<String, i32>,
    /// The dual-class rules, for the classes the character may add.
    class_table: Rc<ClassTable>,
    advance: Arc<AdvanceRules>,
    /// The character as read, for experience and levels.
    character: Character,
    /// Alignments, kits and the social class tables.
    creation: Arc<cr_store::create::CreationRules>,
    /// The Background… dialog's events, "NONE" first.
    backgrounds: Vec<String>,
    /// Subraces, racial and class abilities, schools and spheres.
    abilities: Rc<cr_store::abilities::AbilityRules>,
    combat: Rc<cr_store::combat::CombatRules>,
    /// Nonweapon proficiencies' costs and the kit's questions about them.
    proficiencies: Rc<cr_store::proficiencies::ProficiencyRules>,
    /// Rogue skills, psionic powers, wild talents, attack and defense modes.
    skills: Rc<cr_store::skills::SkillRules>,
    /// The kit's schools, spheres and spell use.
    kit_magic: Rc<cr_store::kit_magic::KitMagicRules>,
    /// The spell pickers' checks.
    learn: Rc<LearnRules>,
    /// The item check of the Equipment page, when the program was read.
    item_rules: Option<Arc<NpcRules>>,
}

impl Choices {
    fn load(install: &Installation, character: &Character) -> cr_store::Result<Self> {
        let mut items = Vec::new();
        for part_type in ITEM_TYPES {
            items.extend(install.part_names(part_type)?);
        }
        // Coins are edited as money.
        items.retain(|n| !COINS.iter().any(|c| c.0 == n));
        items.sort_by_cached_key(|n| cr_store::collate::sort_key(n));
        items.dedup();
        let advance = Arc::new(install.advance_rules()?);
        let creation = Arc::new(install.creation_rules()?);
        Ok(Self {
            languages: Rc::new(install.part_names("Languages")?),
            wizard_spells: Rc::new(install.spell_names(true)?),
            priest_spells: Rc::new(install.spell_names(false)?),
            items: Rc::new(items),
            icons: Rc::new(install.item_icons()?),
            wizard_levels: Rc::new(install.spell_levels(true)?),
            priest_levels: Rc::new(install.spell_levels(false)?),
            class_ids: install.class_ids()?,
            class_table: Rc::new(install.class_table()?),
            combat: Rc::new(install.combat_rules_with(creation.clone(), advance.clone())?),
            proficiencies: Rc::new(
                install.proficiency_rules_with(creation.clone(), advance.clone()),
            ),
            advance,
            character: character.clone(),
            creation,
            backgrounds: cr_store::edit::backgrounds(install)?,
            abilities: Rc::new(install.ability_rules()?),
            skills: Rc::new(install.skill_rules()?),
            kit_magic: Rc::new(install.kit_magic_rules()?),
            learn: Rc::new(install.learn_rules()?),
            item_rules: None,
        })
    }
}

/// The editors of a character's lists.
struct ListEditors {
    /// Edited on the Combat page.
    weapon_proficiencies: Vec<String>,
    nonweapon_proficiencies: nonweapon::NonweaponEditor,
    /// The nonweapon proficiencies as read, to tell whether they changed.
    original_nonweapon: cr_store::proficiencies::CharacterNonweapon,
    languages: ListEditor,
    wizard_spells: ListEditor,
    priest_spells: ListEditor,
    /// Per spellbook: what it was read as, and its spells with pages.
    spellbooks: Vec<(Spellbook, RowList<bool>)>,
    /// Per casting class: its slot, whether it casts wizard spells, and
    /// the memorized spells with their levels.
    memorized: Vec<(usize, bool, RowList<u32>)>,
    original_memorized: Memorized,
    /// One per bundle but the spending money.
    equipment: Vec<(usize, RowList<Item>)>,
    coins: [adw::SpinRow; 5],
    /// Whether adding and removing items buys and sells them (the
    /// original's Buy/Sell mode, Free when Update opens).
    trading: adw::SwitchRow,
}

/// Asked before an item is added, with the list's group, the item and
/// what adds it.
type ItemCheck = Rc<dyn Fn(&adw::PreferencesGroup, &str, Box<dyn FnOnce()>)>;

/// A list's items and how many of each.
type Stock = Vec<(String, u32)>;

/// What `list` holds now.
fn stock(list: &RowList<Item>) -> Stock {
    list.values()
        .into_iter()
        .map(|(item, n)| (item.name, n.unwrap_or(item.quantity)))
        .collect()
}

impl ListEditors {
    /// `advanced` is the character at the levels the editor has now.
    fn new(
        lists: &CharacterLists,
        choices: &Choices,
        classes: &[String],
        advanced: &Rc<RefCell<Character>>,
        kit_magic: &kit_magic::KitMagicEditor,
    ) -> Self {
        let wizard_spells = ListEditor::new(
            tr("Wizard Spells Known"),
            &lists.wizard_spells,
            choices.wizard_spells.clone(),
        );
        let priest_spells = ListEditor::new(
            tr("Priest Spells Known"),
            &lists.priest_spells,
            choices.priest_spells.clone(),
        );

        // Spellbooks take the wizard spells the character knows.
        let spellbooks = lists
            .spellbooks
            .iter()
            .map(|book| {
                let levels = choices.wizard_levels.clone();
                let capacity = book.capacity.max(1);
                let rows = book
                    .spells
                    .iter()
                    .map(|(name, pages, wizard)| RowSpec {
                        value: *wizard,
                        title: name.clone(),
                        subtitle: String::new(),
                        number: Some((*pages, 1, capacity)),
                    })
                    .collect();
                let known = wizard_spells.clone();
                let list = RowList::new(
                    &format!("{} ({})", book.name, tr(BUNDLE_NAMES[book.bundle])),
                    rows,
                    move || known.items(),
                    move |name| {
                        // One page per spell level, as the rules have it.
                        let pages = levels.get(&name).copied().unwrap_or(1).max(1);
                        Some(RowSpec {
                            value: true,
                            title: name,
                            subtitle: String::new(),
                            number: Some((pages, 1, capacity)),
                        })
                    },
                );
                list.connect_changed({
                    let (list, total) = (list.clone(), book.capacity);
                    move || {
                        let used: u32 = list.values().iter().filter_map(|v| v.1).sum();
                        list.group.set_description(Some(&tr!(
                            "{used} of {total} pages used",
                            used,
                            total
                        )));
                    }
                });
                (book.clone(), list)
            })
            .collect();

        // Memorized spells, per class that casts.
        let mut memorized = Vec::new();
        for (slot, class) in classes.iter().enumerate().take(5) {
            let kind = choices.class_ids.get(class).copied().map(caster);
            let wizard = match kind {
                Some(Caster::Wizard) => true,
                Some(Caster::Priest) => false,
                _ => continue,
            };
            let rows = (0..11)
                .flat_map(|level| {
                    let (w, p) = &lists.memorized[slot][level];
                    (if wizard { w } else { p })
                        .iter()
                        .map(move |name| RowSpec {
                            value: level as u32,
                            title: name.clone(),
                            subtitle: spell_level_text(level as u32, wizard),
                            number: None,
                        })
                })
                .collect();
            let levels = if wizard {
                choices.wizard_levels.clone()
            } else {
                choices.priest_levels.clone()
            };
            let offer: Box<dyn Fn() -> Vec<String>> = if wizard {
                let known = wizard_spells.clone();
                Box::new(move || known.items())
            } else {
                let all = choices.priest_spells.clone();
                Box::new(move || all.to_vec())
            };
            // The kit's warning comes first, which needs the list.
            let this: Rc<RefCell<Option<RowList<u32>>>> = Rc::default();
            let list = RowList::new(&tr!("Memorized ({class})", class), rows, offer, {
                let (this, kit_magic) = (this.clone(), kit_magic.clone());
                move |name| {
                    let level = levels.get(&name).copied()?;
                    let list = this.borrow().clone()?;
                    kit_magic.check_spell_use(&list.group, {
                        let list = list.clone();
                        move || {
                            list.add(RowSpec {
                                value: level,
                                subtitle: spell_level_text(level, wizard),
                                title: name.clone(),
                                number: None,
                            })
                        }
                    });
                    None
                }
            });
            *this.borrow_mut() = Some(list.clone());
            memorized.push((slot, wizard, list));
        }

        let trading = adw::SwitchRow::builder()
            .title(tr("Buy and Sell"))
            .subtitle(tr(
                "Items added are paid for with the spending money, items removed are sold",
            ))
            .build();
        // What can be bought: no magical items, gems or coins (5d9331).
        let wares: Rc<Vec<String>> = Rc::new({
            let mut names: Vec<String> = choices
                .creation
                .wares
                .iter()
                .map(|w| w.name.clone())
                .collect();
            names.sort_by_cached_key(|n| cr_store::collate::sort_key(n));
            names
        });
        let equipment = (0..9)
            .filter(|&b| b != MONEY)
            .map(|b| {
                let rows = lists.equipment[b]
                    .iter()
                    .map(|item| RowSpec {
                        title: item.name.clone(),
                        subtitle: if item.contents.is_empty() {
                            String::new()
                        } else {
                            tr!("Holds {items}", items = item.contents.join(", "))
                        },
                        number: Some((item.quantity, 1, 99_999)),
                        value: item.clone(),
                    })
                    .collect();
                let (items, wares, trading) =
                    (choices.items.clone(), wares.clone(), trading.clone());
                let list = RowList::new(
                    tr(BUNDLE_NAMES[b]),
                    rows,
                    move || {
                        if trading.is_active() {
                            wares.to_vec()
                        } else {
                            items.to_vec()
                        }
                    },
                    |name| {
                        Some(RowSpec {
                            value: Item::new(&name),
                            title: name,
                            subtitle: String::new(),
                            number: Some((1, 1, 99_999)),
                        })
                    },
                );
                list.show_icons(choices.icons.clone());
                (b, list)
            })
            .collect();

        let coins = std::array::from_fn(|c| {
            adw::SpinRow::builder()
                .title(
                    [
                        tr("Copper"),
                        tr("Silver"),
                        tr("Electrum"),
                        tr("Gold"),
                        tr("Platinum"),
                    ][c],
                )
                .numeric(true)
                .adjustment(&gtk::Adjustment::new(
                    lists.coins[c].into(),
                    0.0,
                    9_999_999.0,
                    1.0,
                    10.0,
                    0.0,
                ))
                .build()
        });

        Self {
            weapon_proficiencies: lists.weapon_proficiencies.clone(),
            nonweapon_proficiencies: nonweapon::NonweaponEditor::new(
                choices.proficiencies.clone(),
                advanced.clone(),
            ),
            original_nonweapon: choices.proficiencies.nonweapon_of(&choices.character),
            languages: ListEditor::new(
                tr("Native Languages"),
                &lists.languages,
                choices.languages.clone(),
            ),
            wizard_spells,
            priest_spells,
            spellbooks,
            memorized,
            original_memorized: lists.memorized.clone(),
            equipment,
            coins,
            trading,
        }
    }

    fn proficiencies_page(&self) -> adw::PreferencesPage {
        let page = adw::PreferencesPage::new();
        page.add(&self.nonweapon_proficiencies.group);
        page.add(&self.languages.group);
        page
    }

    /// The Spells page's groups, after the kit's.
    fn spell_groups(&self) -> Vec<adw::PreferencesGroup> {
        [&self.wizard_spells.group, &self.priest_spells.group]
            .into_iter()
            .chain(self.spellbooks.iter().map(|(_, list)| &list.group))
            .chain(self.memorized.iter().map(|(_, _, list)| &list.group))
            .cloned()
            .collect()
    }

    fn equipment_page(&self) -> adw::PreferencesPage {
        let page = adw::PreferencesPage::new();
        let money = adw::PreferencesGroup::builder()
            .title(tr("Spending Money"))
            .build();
        money.add(&self.trading);
        for row in &self.coins {
            money.add(row);
        }
        page.add(&money);
        for (_, list) in &self.equipment {
            page.add(&list.group);
        }
        page
    }

    /// Has the equipment lists ask `check` before an item is added and,
    /// while Buy and Sell is on, pay for what is added and give back the
    /// price of what is removed or lessened, at the kit's percentage
    /// (5d9331, 5dde7f, 5ddd19).
    fn connect_trade(
        &self,
        rules: Arc<cr_store::create::CreationRules>,
        kit: adw::ComboRow,
        check: Option<ItemCheck>,
    ) {
        let lists: Rc<Vec<RowList<Item>>> =
            Rc::new(self.equipment.iter().map(|(_, l)| l.clone()).collect());
        let paid: Rc<RefCell<Vec<Stock>>> =
            Rc::new(RefCell::new(lists.iter().map(stock).collect()));
        let coins = Rc::new(self.coins.clone());
        let price = Rc::new({
            let (rules, kit) = (rules.clone(), kit.clone());
            move |name: &str, n: u32| rules.price(&create::combo_value(&kit), name, n)
        });
        // Why `cost` cannot be paid, if it cannot (5d9705): the spending
        // money is short, but maybe not the money kept elsewhere.
        let short = Rc::new({
            let (coins, lists) = (coins.clone(), lists.clone());
            move |cost: u64| -> Option<&'static str> {
                let spending = purse::worth(&std::array::from_fn(|c| coins[c].value() as u32));
                if spending >= cost {
                    return None;
                }
                let kept: u64 = lists
                    .iter()
                    .flat_map(stock)
                    .filter_map(|(name, n)| {
                        COINS
                            .iter()
                            .find(|c| c.0 == name)
                            .map(|c| u64::from(n) * u64::from(c.1))
                    })
                    .sum();
                Some(if spending + kept >= cost {
                    tr(
                        "You must move some of your money into Spending Money before you can \
                        purchase this item.",
                    )
                } else {
                    tr("This item is too expensive.")
                })
            }
        });
        for (i, list) in lists.iter().enumerate() {
            // What each item costs, as the picker offers it.
            list.set_detail({
                let (trading, price) = (self.trading.clone(), price.clone());
                move |name| {
                    if trading.is_active() {
                        create::money(price(name, 1))
                    } else {
                        String::new()
                    }
                }
            });
            list.connect_add({
                let (check, group, trading) =
                    (check.clone(), list.group.clone(), self.trading.clone());
                let (price, short) = (price.clone(), short.clone());
                move |name, add| {
                    let buy: Box<dyn FnOnce()> = if trading.is_active() {
                        let (group, short) = (group.clone(), short.clone());
                        let cost = price(name, 1);
                        Box::new(move || match short(cost.into()) {
                            Some(text) => create::message(&group, tr("Error Message!"), text),
                            None => add(),
                        })
                    } else {
                        add
                    };
                    match &check {
                        Some(check) => check(&group, name, buy),
                        None => buy(),
                    }
                }
            });
            list.also_connect_changed({
                let (list, paid, trading) = (list.clone(), paid.clone(), self.trading.clone());
                let (coins, price, short) = (coins.clone(), price.clone(), short.clone());
                move || {
                    let now = stock(&list);
                    let before = std::mem::replace(&mut paid.borrow_mut()[i], now.clone());
                    if !trading.is_active() {
                        return;
                    }
                    let cost = |rows: &[(String, u32)]| -> u64 {
                        rows.iter()
                            .map(|(name, n)| u64::from(price(name, *n)))
                            .sum()
                    };
                    let (was, is) = (cost(&before), cost(&now));
                    let mut purse: purse::Coins = std::array::from_fn(|c| coins[c].value() as u32);
                    if is > was {
                        let more = u32::try_from(is - was).unwrap_or(u32::MAX);
                        if !purse::pay(&mut purse, more) {
                            let text = short(more.into())
                                .unwrap_or_else(|| tr("This item is too expensive."));
                            // Only more of an item gets here: it goes back
                            // to what was paid for.
                            paid.borrow_mut()[i] = before.clone();
                            let changed =
                                (0..now.len().min(before.len())).find(|&j| now[j] != before[j]);
                            if let Some(j) = changed {
                                list.set_number(j, before[j].1);
                            }
                            create::message(&list.group, tr("Error Message!"), text);
                            return;
                        }
                    } else if was > is {
                        purse::refund(&mut purse, u32::try_from(was - is).unwrap_or(u32::MAX));
                    }
                    for (row, n) in coins.iter().zip(purse) {
                        row.set_value(n.into());
                    }
                }
            });
        }
        // That the kit changes what items cost, as the mode is chosen
        // (5e2ba2).
        self.trading.connect_active_notify(move |row| {
            let name = create::combo_value(&kit);
            if row.is_active()
                && let Some(text) = rules.kit_price_notice(&name)
            {
                create::message(row, &name, text);
            }
        });
    }

    fn lists(&self) -> CharacterLists {
        let mut memorized = self.original_memorized.clone();
        for (slot, wizard, _) in &self.memorized {
            for level in &mut memorized[*slot] {
                if *wizard {
                    level.0.clear();
                } else {
                    level.1.clear();
                }
            }
        }
        for (slot, wizard, list) in &self.memorized {
            let names = list.rows_titles();
            for ((level, _), name) in list.values().into_iter().zip(names) {
                let entry = &mut memorized[*slot][level as usize];
                if *wizard {
                    entry.0.push(name);
                } else {
                    entry.1.push(name);
                }
            }
        }
        let mut equipment: [Vec<Item>; 9] = Default::default();
        for (b, list) in &self.equipment {
            equipment[*b] = list
                .values()
                .into_iter()
                .map(|(mut item, quantity)| {
                    item.quantity = quantity.unwrap_or(item.quantity);
                    item
                })
                .collect();
        }
        let nonweapon = self.nonweapon_proficiencies.nonweapon();
        CharacterLists {
            weapon_proficiencies: self.weapon_proficiencies.clone(),
            nonweapon_proficiencies: nonweapon.entries.iter().map(|e| e.name.clone()).collect(),
            languages: self.languages.items(),
            wizard_spells: self.wizard_spells.items(),
            priest_spells: self.priest_spells.items(),
            memorized,
            equipment,
            coins: std::array::from_fn(|c| self.coins[c].value() as u32),
            new_classes: Vec::new(),
            advancement: None,
            abilities: None,
            combat: None,
            skills: None,
            nonweapon: (nonweapon != self.original_nonweapon).then_some(nonweapon),
            kit_magic: None,
            spellbooks: self
                .spellbooks
                .iter()
                .map(|(book, list)| Spellbook {
                    spells: list
                        .rows_titles()
                        .into_iter()
                        .zip(list.values())
                        .map(|(name, (wizard, pages))| (name, pages.unwrap_or(1), wizard))
                        .collect(),
                    ..book.clone()
                })
                .collect(),
        }
    }
}
