//! The GNOME app: browses the data of a Core Rules installation in three
//! panes (sections, entries, details) that collapse on narrow windows.

mod abilities;
mod books;
mod bundle;
mod create;
mod encounter;
mod extract;
mod lists;
mod npc;
mod party;
mod picture;
mod prefs;
mod sheet;
mod tables;
mod transfer;
mod treasure;
mod userdb;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::character::Character;
use cr_i18n::{tr, tr_noop};
use cr_store::Installation;
use cr_store::advance::{AdvanceRules, Advancement};
use cr_store::catalog::{ENCOUNTER_KEY, Entry, Section, TABLE_KEY, TEMPLATE_KEY, TREASURE_KEY};
use cr_store::choices::{Caster, caster};
use cr_store::dual::ClassChoice;
use cr_store::edit::{ABILITY_RANGE, CharacterDetails};
use cr_store::lists::{
    BUNDLE_NAMES, COINS, CharacterLists, ITEM_TYPES, Item, MONEY, Memorized, Spellbook,
};
use cr_store::npc::NpcRules;
use cr_store::pictures::{ICON_SIZE, ItemIcons};
use cr_store::treasure::Rand;
use gtk::{gio, glib};
use lists::{ListEditor, RowList, RowSpec, pick};

const APP_ID: &str = "io.github.brahm.CoreRules";

/// The actions that make or change content, off for Core Rules 2.0.
const WRITE_ACTIONS: [&str; 19] = [
    "new-character",
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
const ICONS: [(&str, &str); 1] = [(
    "corerules-sword-symbolic",
    include_str!("../../../data/icons/symbolic/corerules-sword-symbolic.svg"),
)];

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
    npc_rules: Rc<RefCell<Option<Rc<NpcRules>>>>,
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
                if let Some(name) = key.strip_prefix(TEMPLATE_KEY) {
                    encounter::create_from_template(&host, name)
                } else if let Some(name) = key.strip_prefix(TABLE_KEY) {
                    encounter::roll_and_generate(&host, name)
                }
            }
        });

        let preferences = gio::ActionEntry::builder("preferences")
            .activate({
                let this = this.clone();
                move |_, _, _| {
                    let toasts = this.toasts.clone();
                    prefs::show(&this.window, move |m| toasts.add_toast(adw::Toast::new(m)))
                }
            })
            .build();
        let open = gio::ActionEntry::builder("open")
            .activate({
                let this = this.clone();
                move |_, _, _| this.choose_folder()
            })
            .build();
        let extract_data = gio::ActionEntry::builder("extract-data")
            .activate({
                let this = this.clone();
                move |_, _, _| extract::choose_source(&Rc::new(this.clone()))
            })
            .build();
        let new_character = gio::ActionEntry::builder("new-character")
            .activate({
                let this = this.clone();
                move |_, _, _| this.new_character()
            })
            .build();
        let new_npc = gio::ActionEntry::builder("new-npc")
            .activate({
                let this = this.clone();
                move |_, _, _| npc::new_npc(&Rc::new(this.clone()))
            })
            .build();
        let new_npc_group = gio::ActionEntry::builder("new-npc-group")
            .activate({
                let this = this.clone();
                move |_, _, _| npc::new_group(&Rc::new(this.clone()))
            })
            .build();
        let roll_treasure = gio::ActionEntry::builder("roll-treasure")
            .activate({
                let this = this.clone();
                move |_, _, _| this.roll_treasure(None)
            })
            .build();
        let new_encounter = gio::ActionEntry::builder("new-encounter")
            .activate({
                let this = this.clone();
                move |_, _, _| encounter::choose_template(&Rc::new(this.clone()))
            })
            .build();
        let new_table = gio::ActionEntry::builder("new-table")
            .activate({
                let this = this.clone();
                move |_, _, _| encounter::table_editor(&Rc::new(this.clone()), None)
            })
            .build();
        let open_book = gio::ActionEntry::builder("open-book")
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
        let roll_table = gio::ActionEntry::builder("roll-table")
            .activate({
                let this = this.clone();
                move |_, _, _| this.roll_table()
            })
            .build();
        let manage_parties = gio::ActionEntry::builder("manage-parties")
            .activate({
                let this = this.clone();
                move |_, _, _| party::manage(&Rc::new(this.clone()))
            })
            .build();
        let export = gio::ActionEntry::builder("export")
            .activate({
                let this = this.clone();
                move |_, _, _| transfer::export(&Rc::new(this.clone()))
            })
            .build();
        let import = gio::ActionEntry::builder("import")
            .activate({
                let this = this.clone();
                move |_, _, _| transfer::import(&Rc::new(this.clone()))
            })
            .build();
        let import_core20 = gio::ActionEntry::builder("import-core20")
            .activate({
                let this = this.clone();
                move |_, _, _| transfer::import_core20(&Rc::new(this.clone()))
            })
            .build();
        let new_package = gio::ActionEntry::builder("new-package")
            .activate({
                let this = this.clone();
                move |_, _, _| transfer::new_package(&Rc::new(this.clone()))
            })
            .build();
        let edit_package = gio::ActionEntry::builder("edit-package")
            .activate({
                let this = this.clone();
                move |_, _, _| transfer::edit_package(&Rc::new(this.clone()))
            })
            .build();
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
            new_npc,
            new_npc_group,
            roll_treasure,
            new_encounter,
            new_table,
        ]);
        app.set_accels_for_action("win.open", &["<Control>o"]);
        app.set_accels_for_action("win.preferences", &["<Control>comma"]);
        app.set_accels_for_action("win.new-character", &["<Control>n"]);
        app.set_accels_for_action("win.new-npc", &["<Control><Shift>n"]);
        app.set_accels_for_action("win.roll-treasure", &["<Control>t"]);
        app.set_accels_for_action("win.roll-table", &["<Control><Shift>t"]);
        app.set_accels_for_action("win.new-encounter", &["<Control>e"]);
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
            Ok(install) => self.show_installation(install),
            Err(e) => self.show_missing(Some(&e.to_string())),
        }
    }

    fn show_missing(&self, error: Option<&str>) {
        let button = gtk::Button::builder()
            .label(tr("_Open Folder…"))
            .use_underline(true)
            .halign(gtk::Align::Center)
            .action_name("win.open")
            .css_classes(["pill"])
            .build();
        let extract = gtk::Button::builder()
            .label(tr("Extract Data from _CD…"))
            .use_underline(true)
            .halign(gtk::Align::Center)
            .action_name("win.extract-data")
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
    }

    fn set_can_create(&self, enabled: bool) {
        for name in [
            "new-character",
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
                saved: saved.map(str::to_owned),
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
        let install = self.install.borrow();
        let Some(install) = install.as_ref() else {
            return;
        };
        self.browser.show_section(section);
        self.browser.load(install.catalog(), key.as_deref());
        if key.is_none() {
            self.browser.show_section(section);
        }
    }

    /// Opens the New Character dialog.
    fn new_character(&self) {
        let (rules, root) = match &*self.install.borrow() {
            Some(install) => (install.creation_rules(), install.root().to_owned()),
            None => return,
        };
        let rules = match rules {
            Ok(rules) => rules,
            Err(e) => return self.toast(&e.to_string()),
        };
        let this = self.clone();
        create::new_character(&self.window, rules, root, move |draft| {
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
            this.toast(tr("Character created"));
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

    fn toast(&self, message: &str) {
        self.toasts.add_toast(adw::Toast::new(message));
    }

    /// Shows the character sheet of the character in `Chars/<file>`.
    fn show_sheet(&self, file: &str) {
        let sheet = match &*self.install.borrow() {
            Some(install) => install.character(file).and_then(|c| install.sheet(&c)),
            None => return,
        };
        match sheet {
            Ok(sheet) => {
                let this = self.clone();
                sheet::show(&self.window, sheet, move |message| this.toast(message));
            }
            Err(e) => self.toast(&e.to_string()),
        }
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
        let choices = match choices {
            Ok(choices) => choices,
            Err(e) => return self.toast(&e.to_string()),
        };
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

    fn npc_rules(&self) -> Option<Rc<NpcRules>> {
        if let Some(rules) = &*self.npc_rules.borrow() {
            return Some(rules.clone());
        }
        let loaded = self.install.borrow().as_ref()?.npc_rules();
        match loaded {
            Ok(rules) => {
                let rules = Rc::new(rules);
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

    fn trap_record(&self, action: &str, origin: cr_store::userdb::Origin, name: &str) {
        userdb::trap_action(&Rc::new(self.clone()), action, origin, name)
    }

    fn parent(&self) -> gtk::Widget {
        self.window.clone().upcast()
    }
}

fn open_menu() -> gtk::MenuButton {
    let menu = gio::Menu::new();
    menu.append(Some(tr("_New Character…")), Some("win.new-character"));
    menu.append(Some(tr("New N_PC…")), Some("win.new-npc"));
    menu.append(Some(tr("New NPC _Group…")), Some("win.new-npc-group"));
    let item = gio::MenuItem::new(Some(tr("Manage _Parties…")), Some("win.manage-parties"));
    item.set_attribute_value("hidden-when", Some(&"action-disabled".to_variant()));
    menu.append_item(&item);
    menu.append(Some(tr("New _Encounter…")), Some("win.new-encounter"));
    menu.append(Some(tr("New Encounter _Table…")), Some("win.new-table"));
    menu.append(Some(tr("_Roll Treasure…")), Some("win.roll-treasure"));
    menu.append(Some(tr("Roll on a _Table…")), Some("win.roll-table"));
    let packages = gio::Menu::new();
    packages.append(Some(tr("_Import…")), Some("win.import"));
    let item = gio::MenuItem::new(
        Some(tr("Import Core _Rules 2.0…")),
        Some("win.import-core20"),
    );
    item.set_attribute_value("hidden-when", Some(&"action-disabled".to_variant()));
    packages.append_item(&item);
    packages.append(Some(tr("E_xport…")), Some("win.export"));
    packages.append(Some(tr("New P_ackage…")), Some("win.new-package"));
    packages.append(Some(tr("E_dit Package…")), Some("win.edit-package"));
    menu.append_section(None, &packages);
    let app = gio::Menu::new();
    app.append(Some(tr("_Open Folder…")), Some("win.open"));
    app.append(Some(tr("Extract Data from _CD…")), Some("win.extract-data"));
    app.append(Some(tr("_Preferences")), Some("win.preferences"));
    menu.append_section(None, &app);
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
    sheet_button: gtk::Button,
    /// Clears the NPC mark of a generated character.
    convert_button: gtk::Button,
    /// Sets it on a player character.
    npc_button: gtk::Button,
    /// Deletes a character.
    delete_button: gtk::Button,
    /// Creates an encounter from a template or a 2-20 table.
    generate_button: gtk::Button,
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
    sheet_button: gtk::Button,
    convert_button: gtk::Button,
    npc_button: gtk::Button,
    delete_button: gtk::Button,
    generate_button: gtk::Button,
    current_key: Rc<RefCell<String>>,
    /// Copy, New and Delete for standard and custom records.
    udb_actions: userdb::DetailActions,
    /// Core Rules 2.0: entries are shown, not edited.
    read_only: Rc<std::cell::Cell<bool>>,
}

impl DetailPane {
    fn show(&self, entry: &Entry) {
        self.page.set_title(&entry.title);
        self.view.set_content(Some(&detail(entry)));
        let key = &entry.key;
        let character = !key.is_empty() && !key.contains(':');
        let writable = !self.read_only.get();
        self.edit_button.set_visible(
            writable
                && (character
                    || userdb::editable(key)
                    || [TREASURE_KEY, ENCOUNTER_KEY, TABLE_KEY]
                        .iter()
                        .any(|p| key.starts_with(p))),
        );
        self.udb_actions.update(if writable { key } else { "" });
        self.sheet_button.set_visible(character);
        self.convert_button
            .set_visible(writable && character && entry.npc);
        self.npc_button
            .set_visible(writable && character && !entry.npc);
        self.delete_button.set_visible(writable && character);
        self.generate_button
            .set_visible(writable && (key.starts_with(TEMPLATE_KEY) || key.starts_with(TABLE_KEY)));
        self.generate_button
            .set_tooltip_text(Some(if key.starts_with(TABLE_KEY) {
                tr("Roll and Create Encounter")
            } else {
                tr("Create Encounter")
            }));
        self.current_key.borrow_mut().clone_from(&entry.key);
    }

    fn show_placeholder(&self, widget: &gtk::Widget) {
        self.page.set_title(tr("Details"));
        self.view.set_content(Some(widget));
        self.edit_button.set_visible(false);
        self.sheet_button.set_visible(false);
        self.convert_button.set_visible(false);
        self.npc_button.set_visible(false);
        self.delete_button.set_visible(false);
        self.generate_button.set_visible(false);
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
        let generate_button = gtk::Button::builder()
            .icon_name("media-playlist-shuffle-symbolic")
            .tooltip_text(tr("Create Encounter"))
            .visible(false)
            .build();
        let detail_header = adw::HeaderBar::new();
        detail_header.pack_start(&delete_button);
        detail_header.pack_end(&generate_button);
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
            sheet_button: sheet_button.clone(),
            convert_button: convert_button.clone(),
            npc_button: npc_button.clone(),
            delete_button: delete_button.clone(),
            generate_button: generate_button.clone(),
            current_key: current_key.clone(),
            udb_actions,
            read_only: Rc::default(),
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
        let filter = gtk::CustomFilter::new({
            let (search, origin_filter) = (search.clone(), origin_filter.clone());
            let (party_filter, party_choices) = (party_filter.clone(), party_choices.clone());
            move |item| {
                let needle = search.text().to_lowercase();
                let item = item.downcast_ref::<glib::BoxedAnyObject>().unwrap();
                let row = item.borrow::<Row>();
                let party = party_choices
                    .borrow()
                    .get(party_filter.selected() as usize)
                    .cloned()
                    .flatten();
                let in_party = match (&row.party, party) {
                    (Some(theirs), Some(wanted)) => theirs.eq_ignore_ascii_case(&wanted),
                    _ => true,
                };
                userdb::passes(&origin_filter, row.origin)
                    && in_party
                    && (needle.is_empty()
                        || row.title.to_lowercase().contains(&needle)
                        || row.subtitle.to_lowercase().contains(&needle))
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
        // Shown in the Characters section.
        let new_button = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text(tr("New Character"))
            .action_name("win.new-character")
            .visible(false)
            .build();
        let entries_header = adw::HeaderBar::new();
        entries_header.pack_start(&new_button);
        entries_header.pack_end(&origin_filter);
        entries_header.pack_end(&party_filter);
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
        let current = Rc::new(RefCell::new(0usize));
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
                let new = match section.id {
                    "characters" => Some(("win.new-character", tr("New Character").into())),
                    "encounters" => Some(("win.new-encounter", tr("New Encounter").into())),
                    "tables" => Some(("win.new-table", tr("New Encounter Table").into())),
                    "treasure" => Some(("win.roll-treasure", tr("Roll Treasure").into())),
                    id if userdb::can_create(id) => Some(("win.udb-new", userdb::new_tooltip(id))),
                    _ => None,
                };
                new_button.set_visible(new.is_some());
                if let Some((action, tooltip)) = new {
                    new_button.set_action_name(Some(action));
                    new_button.set_tooltip_text(Some(&tooltip));
                }
                origin_filter.set_visible(userdb::has_origins(section.id));
                party_filter
                    .set_visible(section.id == "characters" && party_choices.borrow().len() > 1);
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
                            origin: userdb::origin_of(&e.key),
                            party: (section.id == "characters").then(|| e.party.clone()),
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
            sheet_button,
            convert_button,
            npc_button,
            delete_button,
            generate_button,
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
            // The search was cleared, so list positions are entry indexes.
            self.selection.set_selected(position as u32);
            self.detail.show(entry);
            self.inner.set_show_content(true);
        }
    }

    /// Fills the party filter with the characters' parties, keeping the
    /// choice when its party is still there.
    fn load_parties(&self, sections: &[Section]) {
        let mut parties: Vec<String> = Vec::new();
        let mut unassigned = false;
        for entry in sections
            .iter()
            .filter(|s| s.id == "characters")
            .flat_map(|s| &s.entries)
        {
            if entry.party.is_empty() {
                unassigned = true;
            } else if !parties.iter().any(|p| p.eq_ignore_ascii_case(&entry.party)) {
                parties.push(entry.party.clone());
            }
        }
        parties.sort_by_cached_key(|p| cr_store::collate::sort_key(p));
        let mut choices = vec![None];
        if !parties.is_empty() {
            choices.extend(parties.into_iter().map(Some));
            if unassigned {
                choices.push(Some(String::new()));
            }
        }
        let chosen = self
            .party_choices
            .borrow()
            .get(self.party_filter.selected() as usize)
            .cloned()
            .flatten();
        let position = chosen
            .and_then(|c| {
                choices
                    .iter()
                    .position(|p| p.as_ref().is_some_and(|p| p.eq_ignore_ascii_case(&c)))
            })
            .unwrap_or(0);
        let names: Vec<&str> = choices
            .iter()
            .map(|c| match c.as_deref() {
                None => tr("All Parties"),
                Some("") => tr("No Party"),
                Some(party) => party,
            })
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
    index: usize,
    title: String,
    subtitle: String,
    /// Standard or custom, for the records of the user databases.
    origin: Option<cr_store::userdb::Origin>,
    /// The party of a character, empty for none.
    party: Option<String>,
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

/// Text from the original's Windows edit boxes, cleaned up for a label.
fn clean_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "    ")
        .trim()
        .to_owned()
}

fn detail(entry: &Entry) -> gtk::Widget {
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
    if let Some(picture) = entry
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

    let text = clean_text(&entry.text);
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
    let social_class = entry_row(tr("Social Class"), &details.social_class);
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
            let rolled =
                rules.roll_social_class(&create::combo_value(kit), &mut Default::default());
            social_class.set_text(&rolled);
        }
    });
    let event = create::combo_row(
        tr("Background Event"),
        &with(choices.backgrounds.clone(), &details.background),
        &details.background,
    );
    let history = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(8)
        .right_margin(8)
        .accepts_tab(false)
        .height_request(96)
        .css_classes(["card"])
        .build();
    history.buffer().set_text(&details.history);
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
    let (class_group, new_classes) = classes_group(classes, &choices.new_classes);
    let advanced = Rc::new(RefCell::new(choices.character.clone()));
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

    let editors = ListEditors::new(lists, &choices, classes);
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
                    .set_description(slots_text(slots, &list.values()).as_deref());
            }
        };
        show();
        list.connect_changed(show.clone());
        level_listeners.borrow_mut().push(Box::new(show));
    }
    let ability_editor =
        abilities::AbilityEditor::new(choices.abilities.clone(), &choices.character);
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
        &editors.proficiencies_page(),
        Some("proficiencies"),
        tr("Proficiencies"),
        "view-list-symbolic",
    );
    stack.add_titled_with_icon(
        &editors.spells_page(),
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
        .content_width(480)
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
    save_button.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            let details = CharacterDetails {
                name: name.text().trim().to_owned(),
                player: player.text().trim().to_owned(),
                social_class: social_class.text().trim().to_owned(),
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
                history: {
                    let buffer = history.buffer();
                    let (start, end) = buffer.bounds();
                    buffer.text(&start, &end, false).to_string()
                },
            };
            let mut lists = editors.lists();
            lists.new_classes = new_classes.borrow().clone();
            let edited = ability_editor.abilities();
            if edited != ability_rules.abilities_of(&original) {
                lists.abilities = Some(edited);
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
    dialog.present(Some(parent));
}

/// The character's classes, with an Add button for one more class when
/// the race may dual-class. Race and classes themselves cannot change.
fn classes_group(
    classes: &[String],
    choice: &ClassChoice,
) -> (adw::PreferencesGroup, Rc<RefCell<Vec<String>>>) {
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
    let offered = match choice {
        ClassChoice::Refused(why) => {
            group.set_description(Some(&glib::markup_escape_text(why)));
            add.set_sensitive(false);
            return (group, added);
        }
        ClassChoice::Allowed(names) if names.is_empty() => {
            group.set_description(Some(tr(
                "No class meets the dual-class requirements of these abilities",
            )));
            add.set_sensitive(false);
            return (group, added);
        }
        ClassChoice::Allowed(names) => names.clone(),
    };
    group.set_description(Some(tr(
        "Dual-classing: the new class starts at level 1 and the earlier class stops advancing",
    )));
    add.connect_clicked({
        let (group, added) = (group.clone(), added.clone());
        move |button| {
            let (group, added, button) = (group.clone(), added.clone(), button.clone());
            pick(
                &button.clone(),
                tr("Classes"),
                offered.clone(),
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
                    // One class at a time: the next one depends on this one.
                    button.set_sensitive(false);
                    remove.connect_clicked({
                        let (group, added, button) = (group.clone(), added.clone(), button.clone());
                        move |_| {
                            group.remove(&row);
                            added.borrow_mut().clear();
                            button.set_sensitive(true);
                        }
                    });
                },
            );
        }
    });
    (group, added)
}

/// Experience, levels and hit points, with a button to give or take away
/// experience as the original's Level... dialog does. Changes go to
/// `advanced`, a copy of the character that the save compares with the
/// character as read.
fn experience_group(
    rules: &Rc<AdvanceRules>,
    advanced: &Rc<RefCell<Character>>,
    listeners: &Listeners,
) -> adw::PreferencesGroup {
    let add = gtk::Button::builder()
        .label(tr("Add…"))
        .tooltip_text(tr("Add or take away experience"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let group = adw::PreferencesGroup::builder()
        .title(tr("Experience"))
        .header_suffix(&add)
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
    let showing = Rc::new(std::cell::Cell::new(false));
    group.add(&hit_points);
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
        let (hit_points, showing) = (hit_points.clone(), showing.clone());
        move || {
            for listener in listeners.borrow().iter() {
                listener();
            }
            let character = advanced.borrow();
            match rules.levels(&character) {
                Ok(levels) => {
                    for (row, class) in class_rows.iter().zip(levels) {
                        let (level, xp) = (class.level, thousands(class.experience));
                        let text = match (class.next, class.limit) {
                            (_, Some(limit)) if class.level >= limit => {
                                tr!(
                                    "Level {level} · {xp} XP · level limit {limit}",
                                    level,
                                    xp,
                                    limit
                                )
                            }
                            (Some(next), _) => tr!(
                                "Level {level} · {xp} XP · next level at {next}",
                                level,
                                xp,
                                next = thousands(next)
                            ),
                            (None, Some(limit)) => {
                                tr!(
                                    "Level {level} · {xp} XP · level limit {limit}",
                                    level,
                                    xp,
                                    limit
                                )
                            }
                            (None, None) => tr!("Level {level} · {xp} XP", level, xp),
                        };
                        row.set_subtitle(&text);
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
            showing.set(false);
            if let Some(points) = &points {
                points.set_label(&character.data.unknown_0x1778[0].to_string());
            }
        }
    });
    refresh();
    hit_points.connect_value_notify({
        let (rules, advanced, refresh) = (rules.clone(), advanced.clone(), refresh.clone());
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
    if rules.expansion() {
        let limits = adw::ComboRow::builder()
            .title(tr("Level Limits"))
            .model(&gtk::StringList::new(&[
                tr("None"),
                tr("Racial Limits"),
                tr("Racial Limits and Prime Requisite Bonus"),
            ]))
            .selected(character.data.unknown_0x1dc4.min(2))
            .build();
        limits.connect_selected_notify({
            let (advanced, refresh) = (advanced.clone(), refresh.clone());
            move |limits| {
                advanced.borrow_mut().data.unknown_0x1dc4 = limits.selected();
                refresh();
            }
        });
        group.add(&limits);
    }
    add.connect_clicked({
        let (rules, advanced) = (rules.clone(), advanced.clone());
        move |button| {
            let refresh = refresh.clone();
            experience_dialog(button, &rules, &advanced, move || refresh());
        }
    });
    group
}

/// Callbacks run when the character's levels change.
type Listeners = Rc<RefCell<Vec<Box<dyn Fn()>>>>;

/// "1st: 2 of 3 · 2nd: 0 of 1" for the memorized spells (values are their
/// spell levels) against the slots, marking levels over the limit.
fn slots_text(slots: Option<[u32; 9]>, memorized: &[(u32, Option<u32>)]) -> Option<String> {
    let slots = slots?;
    let parts: Vec<String> = (1..=9u32)
        .filter_map(|level| {
            let used = memorized.iter().filter(|(l, _)| *l == level).count() as u32;
            let have = slots[level as usize - 1];
            if have == 0 && used == 0 {
                return None;
            }
            let level = ordinal(level);
            Some(if used > have {
                tr!("{level}: {used} of {have} (too many)", level, used, have)
            } else {
                tr!("{level}: {used} of {have}", level, used, have)
            })
        })
        .collect();
    Some(if parts.is_empty() {
        tr("No spell slots at this level").into()
    } else {
        parts.join(" · ")
    })
}

fn group_error(rows: &[adw::ActionRow], message: &str) {
    for row in rows {
        row.set_subtitle(&glib::markup_escape_text(message));
    }
}

/// 12345 as "12,345", as the original shows experience.
fn thousands(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Asks how much experience to give to which classes, then gives it.
fn experience_dialog(
    parent: &impl IsA<gtk::Widget>,
    rules: &Rc<AdvanceRules>,
    advanced: &Rc<RefCell<Character>>,
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
        .active(true)
        .build();
    let points = adw::SpinRow::builder()
        .title(tr("Character Points per Level"))
        .numeric(true)
        .adjustment(&gtk::Adjustment::new(3.0, 0.0, 10.0, 1.0, 1.0, 0.0))
        .visible(character.skills_and_powers())
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
            let n = names.len();
            let amount = amount.value() as i64;
            let selected = target.selected() as usize;
            let (classes, amount) = match selected {
                g if g < n => (vec![g], amount),
                g if g == n => ((0..n).collect(), amount),
                _ => ((0..n).collect(), amount / n as i64),
            };
            let mut dice = cr_store::create::Dice::default();
            let mut character = advanced.borrow().clone();
            let result = classes.into_iter().rev().try_for_each(|g| {
                rules.add_experience(
                    &mut character,
                    g,
                    amount,
                    prime.is_active(),
                    points.value() as u32,
                    &mut dice,
                )
            });
            match result {
                Ok(()) => {
                    *advanced.borrow_mut() = character;
                    changed();
                    dialog.close();
                }
                Err(e) => {
                    error.set_title(&glib::markup_escape_text(&e.to_string()));
                    error.set_revealed(true);
                }
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
    weapons: Rc<Vec<String>>,
    nonweapon: Rc<Vec<String>>,
    languages: Rc<Vec<String>>,
    wizard_spells: Rc<Vec<String>>,
    priest_spells: Rc<Vec<String>>,
    items: Rc<Vec<String>>,
    /// The items' icons, drawn on the equipment rows.
    icons: Rc<ItemIcons>,
    wizard_levels: Rc<HashMap<String, u32>>,
    priest_levels: Rc<HashMap<String, u32>>,
    class_ids: HashMap<String, i32>,
    /// Classes the character may add, or why none.
    new_classes: ClassChoice,
    advance: Rc<AdvanceRules>,
    /// The character as read, for experience and levels.
    character: Character,
    /// Alignments, kits and the social class tables.
    creation: Rc<cr_store::create::CreationRules>,
    /// The Background… dialog's events, "NONE" first.
    backgrounds: Vec<String>,
    /// Subraces, racial and class abilities, schools and spheres.
    abilities: Rc<cr_store::abilities::AbilityRules>,
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
        Ok(Self {
            weapons: Rc::new(install.weapon_proficiency_names()?),
            nonweapon: Rc::new(install.part_names("Non-Weapon Proficiencies")?),
            languages: Rc::new(install.part_names("Languages")?),
            wizard_spells: Rc::new(install.spell_names(true)?),
            priest_spells: Rc::new(install.spell_names(false)?),
            items: Rc::new(items),
            icons: Rc::new(install.item_icons()?),
            wizard_levels: Rc::new(install.spell_levels(true)?),
            priest_levels: Rc::new(install.spell_levels(false)?),
            class_ids: install.class_ids()?,
            new_classes: install.class_table()?.choices(character),
            advance: Rc::new(install.advance_rules()?),
            character: character.clone(),
            creation: Rc::new(install.creation_rules()?),
            backgrounds: cr_store::edit::backgrounds(install)?,
            abilities: Rc::new(install.ability_rules()?),
        })
    }
}

/// The editors of a character's lists.
struct ListEditors {
    weapon_proficiencies: ListEditor,
    nonweapon_proficiencies: ListEditor,
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
}

/// Ordinal for a spell level ("1st").
fn ordinal(level: u32) -> String {
    match (level % 10, level % 100) {
        (1, n) if n != 11 => tr!("{level}st", level),
        (2, n) if n != 12 => tr!("{level}nd", level),
        (3, n) if n != 13 => tr!("{level}rd", level),
        _ => tr!("{level}th", level),
    }
}

impl ListEditors {
    fn new(lists: &CharacterLists, choices: &Choices, classes: &[String]) -> Self {
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
                            subtitle: tr!("{ordinal} level", ordinal = ordinal(level as u32)),
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
            let list = RowList::new(
                &tr!("Memorized ({class})", class),
                rows,
                offer,
                move |name| {
                    let level = levels.get(&name).copied()?;
                    Some(RowSpec {
                        value: level,
                        subtitle: tr!("{ordinal} level", ordinal = ordinal(level)),
                        title: name,
                        number: None,
                    })
                },
            );
            memorized.push((slot, wizard, list));
        }

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
                let items = choices.items.clone();
                let list = RowList::new(
                    tr(BUNDLE_NAMES[b]),
                    rows,
                    move || items.to_vec(),
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
            weapon_proficiencies: ListEditor::new(
                tr("Weapon Proficiencies"),
                &lists.weapon_proficiencies,
                choices.weapons.clone(),
            ),
            nonweapon_proficiencies: ListEditor::new(
                tr("Nonweapon Proficiencies"),
                &lists.nonweapon_proficiencies,
                choices.nonweapon.clone(),
            ),
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
        }
    }

    fn proficiencies_page(&self) -> adw::PreferencesPage {
        let page = adw::PreferencesPage::new();
        page.add(&self.weapon_proficiencies.group);
        page.add(&self.nonweapon_proficiencies.group);
        page.add(&self.languages.group);
        page
    }

    fn spells_page(&self) -> adw::PreferencesPage {
        let page = adw::PreferencesPage::new();
        page.add(&self.wizard_spells.group);
        page.add(&self.priest_spells.group);
        for (_, list) in &self.spellbooks {
            page.add(&list.group);
        }
        for (_, _, list) in &self.memorized {
            page.add(&list.group);
        }
        page
    }

    fn equipment_page(&self) -> adw::PreferencesPage {
        let page = adw::PreferencesPage::new();
        let money = adw::PreferencesGroup::builder()
            .title(tr("Spending Money"))
            .build();
        for row in &self.coins {
            money.add(row);
        }
        page.add(&money);
        for (_, list) in &self.equipment {
            page.add(&list.group);
        }
        page
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
        CharacterLists {
            weapon_proficiencies: self.weapon_proficiencies.items(),
            nonweapon_proficiencies: self.nonweapon_proficiencies.items(),
            languages: self.languages.items(),
            wizard_spells: self.wizard_spells.items(),
            priest_spells: self.priest_spells.items(),
            memorized,
            equipment,
            coins: std::array::from_fn(|c| self.coins[c].value() as u32),
            new_classes: Vec::new(),
            advancement: None,
            abilities: None,
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
