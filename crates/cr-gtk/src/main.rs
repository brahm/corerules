//! The GNOME app: browses the data of a Core Rules installation in three
//! panes (sections, entries, details) that collapse on narrow windows.

mod books;
mod create;
mod encounter;
mod lists;
mod npc;
mod sheet;
mod tables;
mod treasure;
mod userdb;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::character::Character;
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
use cr_store::treasure::Rand;
use gtk::{gio, glib};
use lists::{ListEditor, RowList, RowSpec, pick};

const APP_ID: &str = "io.github.brahm.CoreRules";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
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
    });
    // The optional folder argument is handled above, not by GApplication.
    app.run_with_args(&std::env::args().take(1).collect::<Vec<_>>())
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

        this.browser.convert_button.connect_clicked({
            let this = this.clone();
            move |_| {
                let key = this.browser.current_key.borrow().clone();
                this.convert_to_character(&key);
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

        let open = gio::ActionEntry::builder("open")
            .activate({
                let this = this.clone();
                move |_, _, _| this.choose_folder()
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
        this.window.add_action_entries([open_book, roll_table]);
        this.window.add_action_entries([
            open,
            new_character,
            new_npc,
            new_npc_group,
            roll_treasure,
            new_encounter,
            new_table,
        ]);
        app.set_accels_for_action("win.open", &["<Control>o"]);
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
            .title("Open Core Rules Folder")
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
            .label("_Open Folder…")
            .use_underline(true)
            .halign(gtk::Align::Center)
            .action_name("win.open")
            .css_classes(["pill", "suggested-action"])
            .build();
        let page = adw::StatusPage::builder()
            .icon_name("folder-symbolic")
            .title("No Core Rules Installation")
            .description(match error {
                Some(error) => glib::markup_escape_text(error).to_string(),
                None => "Open the folder where AD&amp;D Core Rules 2.0 is installed, \
                         or the root of its CD"
                    .to_owned(),
            })
            .child(&button)
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
        // Create Multiple NPCs draws from the race/class combinations of
        // AllowedU.dat, which only the Expansion has.
        let groups = install.path("UserDbas/AllowedU.dat").is_some();
        *self.install.borrow_mut() = Some(install);
        self.npc_rules.borrow_mut().take();
        self.set_can_create(true);
        userdb::set_flavor(&self.window, groups);
        if let Some(action) = self
            .window
            .lookup_action("new-npc-group")
            .and_downcast::<gio::SimpleAction>()
        {
            action.set_enabled(groups);
        }
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
                            .ok_or("The installation is no longer open")?;
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
                            .ok_or("The installation is no longer open")?;
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
        let rules = match &*self.install.borrow() {
            Some(install) => install.creation_rules(),
            None => return,
        };
        let rules = match rules {
            Ok(rules) => rules,
            Err(e) => return self.toast(&e.to_string()),
        };
        let this = self.clone();
        create::new_character(&self.window, rules, move |draft| {
            let install = this.install.borrow();
            let Some(install) = install.as_ref() else {
                return Err("The installation is no longer open".into());
            };
            let rules = install.creation_rules().map_err(|e| e.to_string())?;
            let file = install
                .create_character(&rules, draft)
                .map_err(|e| e.to_string())?;
            this.browser.show_section("characters");
            this.browser.load(install.catalog(), Some(&file));
            this.toast("Character created");
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
                self.toast("Converted to a character");
            }
            Err(e) => self.toast(&e.to_string()),
        }
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
        let this = self.clone();
        let file = file.to_owned();
        character_editor(
            &self.window,
            &details,
            &lists,
            &classes,
            choices,
            move |details, lists| {
                let install = this.install.borrow();
                let Some(install) = install.as_ref() else {
                    return Err("The installation is no longer open".into());
                };
                install
                    .save_character(&file, details, Some(lists))
                    .map_err(|e| e.to_string())?;
                this.browser.load(install.catalog(), Some(&file));
                this.toast("Character saved");
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

    fn parent(&self) -> gtk::Widget {
        self.window.clone().upcast()
    }
}

fn open_menu() -> gtk::MenuButton {
    let menu = gio::Menu::new();
    menu.append(Some("_New Character…"), Some("win.new-character"));
    menu.append(Some("New N_PC…"), Some("win.new-npc"));
    menu.append(Some("New NPC _Group…"), Some("win.new-npc-group"));
    menu.append(Some("New _Encounter…"), Some("win.new-encounter"));
    menu.append(Some("New Encounter _Table…"), Some("win.new-table"));
    menu.append(Some("_Roll Treasure…"), Some("win.roll-treasure"));
    menu.append(Some("Roll on a _Table…"), Some("win.roll-table"));
    menu.append(Some("_Open Folder…"), Some("win.open"));
    gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Main Menu")
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
    /// Creates an encounter from a template or a 2-20 table.
    generate_button: gtk::Button,
    /// The key of the entry on the detail page.
    current_key: Rc<RefCell<String>>,
}

/// The right-hand pane.
#[derive(Clone)]
struct DetailPane {
    page: adw::NavigationPage,
    view: adw::ToolbarView,
    edit_button: gtk::Button,
    sheet_button: gtk::Button,
    convert_button: gtk::Button,
    generate_button: gtk::Button,
    current_key: Rc<RefCell<String>>,
    /// Copy, New and Delete for standard and custom records.
    udb_actions: userdb::DetailActions,
}

impl DetailPane {
    fn show(&self, entry: &Entry) {
        self.page.set_title(&entry.title);
        self.view.set_content(Some(&detail(entry)));
        let key = &entry.key;
        let character = !key.is_empty() && !key.contains(':');
        self.edit_button.set_visible(
            character
                || userdb::editable(key)
                || [TREASURE_KEY, ENCOUNTER_KEY, TABLE_KEY]
                    .iter()
                    .any(|p| key.starts_with(p)),
        );
        self.udb_actions.update(key);
        self.sheet_button.set_visible(character);
        self.convert_button.set_visible(character && entry.npc);
        self.generate_button
            .set_visible(key.starts_with(TEMPLATE_KEY) || key.starts_with(TABLE_KEY));
        self.generate_button
            .set_tooltip_text(Some(if key.starts_with(TABLE_KEY) {
                "Roll and Create Encounter"
            } else {
                "Create Encounter"
            }));
        self.current_key.borrow_mut().clone_from(&entry.key);
    }

    fn show_placeholder(&self, widget: &gtk::Widget) {
        self.page.set_title("Details");
        self.view.set_content(Some(widget));
        self.edit_button.set_visible(false);
        self.sheet_button.set_visible(false);
        self.convert_button.set_visible(false);
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
            .title("Details")
            .tag("detail")
            .build();
        let detail_view = adw::ToolbarView::new();
        let edit_button = gtk::Button::builder()
            .icon_name("document-edit-symbolic")
            .tooltip_text("Edit")
            .visible(false)
            .build();
        let sheet_button = gtk::Button::builder()
            .icon_name("x-office-document-symbolic")
            .tooltip_text("Character Sheet")
            .visible(false)
            .build();
        let convert_button = gtk::Button::builder()
            .icon_name("avatar-default-symbolic")
            .tooltip_text("Convert to Character")
            .visible(false)
            .build();
        let generate_button = gtk::Button::builder()
            .icon_name("media-playlist-shuffle-symbolic")
            .tooltip_text("Create Encounter")
            .visible(false)
            .build();
        let detail_header = adw::HeaderBar::new();
        detail_header.pack_end(&generate_button);
        detail_header.pack_end(&edit_button);
        detail_header.pack_end(&sheet_button);
        detail_header.pack_end(&convert_button);
        let udb_actions = userdb::DetailActions::new();
        udb_actions.pack(&detail_header);
        detail_view.add_top_bar(&detail_header);
        detail_view.set_content(Some(&placeholder("Nothing Selected", "")));
        detail_page.set_child(Some(&detail_view));
        let current_key: Rc<RefCell<String>> = Rc::default();
        let detail = DetailPane {
            page: detail_page.clone(),
            view: detail_view,
            edit_button: edit_button.clone(),
            sheet_button: sheet_button.clone(),
            convert_button: convert_button.clone(),
            generate_button: generate_button.clone(),
            current_key: current_key.clone(),
            udb_actions,
        };

        // Entries.
        let model = gio::ListStore::new::<glib::BoxedAnyObject>();
        let search = gtk::SearchEntry::builder()
            .placeholder_text("Search")
            .hexpand(true)
            .build();
        let origin_filter = userdb::origin_filter();
        let filter = gtk::CustomFilter::new({
            let (search, origin_filter) = (search.clone(), origin_filter.clone());
            move |item| {
                let needle = search.text().to_lowercase();
                let item = item.downcast_ref::<glib::BoxedAnyObject>().unwrap();
                let row = item.borrow::<Row>();
                userdb::passes(&origin_filter, row.origin)
                    && (needle.is_empty()
                        || row.title.to_lowercase().contains(&needle)
                        || row.subtitle.to_lowercase().contains(&needle))
            }
        });
        origin_filter.connect_selected_notify({
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
                    empty.set_title(&format!("No {}", section_title.borrow()));
                    empty.set_icon_name(Some("folder-symbolic"));
                } else {
                    empty.set_title("No Results");
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
            .tooltip_text("New Character")
            .action_name("win.new-character")
            .visible(false)
            .build();
        let entries_header = adw::HeaderBar::new();
        entries_header.pack_start(&new_button);
        entries_header.pack_end(&origin_filter);
        let entries_view = adw::ToolbarView::new();
        entries_view.add_top_bar(&entries_header);
        entries_view.add_top_bar(&search_bar);
        entries_view.set_content(Some(&entries_stack));
        let entries_page = adw::NavigationPage::builder()
            .title("Entries")
            .tag("entries")
            .child(&entries_view)
            .build();

        inner.set_sidebar(Some(&entries_page));
        inner.set_content(Some(&detail_page));
        let inner_page = adw::NavigationPage::builder()
            .title("Entries")
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
            move |_, row| {
                let Some(row) = row else { return };
                let index = row.index() as usize;
                let sections = sections.borrow();
                let section = &sections[index];
                *current.borrow_mut() = index;
                let new = match section.id {
                    "characters" => Some(("win.new-character", "New Character".into())),
                    "encounters" => Some(("win.new-encounter", "New Encounter".into())),
                    "tables" => Some(("win.new-table", "New Encounter Table".into())),
                    "treasure" => Some(("win.roll-treasure", "Roll Treasure".into())),
                    id if userdb::can_create(id) => Some(("win.udb-new", userdb::new_tooltip(id))),
                    _ => None,
                };
                new_button.set_visible(new.is_some());
                if let Some((action, tooltip)) = new {
                    new_button.set_action_name(Some(action));
                    new_button.set_tooltip_text(Some(&tooltip));
                }
                origin_filter.set_visible(userdb::has_origins(section.id));
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
                        })
                    })
                    .collect();
                *section_title.borrow_mut() = section.title.to_owned();
                model.splice(0, model.n_items(), &rows);
                show_empty(filtered.n_items());
                detail.show_placeholder(&match &section.error {
                    Some(error) => {
                        placeholder_text("Could Not Read", "dialog-error-symbolic", error)
                    }
                    None if section.entries.is_empty() => {
                        placeholder(&format!("No {}", section.title), section.icon)
                    }
                    None => placeholder("Nothing Selected", section.icon),
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
            generate_button,
            current_key,
        }
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
    heading.add(&column);
    page.add(&heading);
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
            .title("Description")
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
    let name = entry_row("Name", &details.name);
    let player = entry_row("Player", &details.player);
    let social_class = entry_row("Social Class", &details.social_class);
    let age = spin_row("Age", "Years", details.age, 9999);
    let height = spin_row("Height", "Inches", details.height, 999);
    let weight = spin_row("Weight", "Pounds", details.weight, 99999);

    // With the Core Rules one row sets the score and both subabilities, as
    // the original does; Skills & Powers characters get all three.
    let (low, high) = (*ABILITY_RANGE.start(), *ABILITY_RANGE.end());
    let abilities = adw::PreferencesGroup::builder().title("Abilities").build();
    let ability_rows: Vec<[adw::SpinRow; 3]> = ABILITY_NAMES
        .iter()
        .zip(details.abilities)
        .map(|(names, values)| {
            let rows = [0, 1, 2].map(|i| ranged_row(names[i], "", values[i], low, high));
            if details.skills_and_powers {
                let expander = adw::ExpanderRow::builder().title(names[0]).build();
                let summarize = {
                    let (expander, rows) = (expander.clone(), rows.clone());
                    move || {
                        let [score, first, second] = rows.each_ref().map(|r| r.value() as u32);
                        expander.set_subtitle(&format!(
                            "{score} · {} {first}, {} {second}",
                            names[1], names[2]
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
        "Exceptional Strength",
        "18/01 to 18/00, 0 for none",
        details.exceptional_strength,
        100,
    );
    abilities.add(&exceptional);

    let character = adw::PreferencesGroup::new();
    character.add(&name);
    character.add(&player);
    character.add(&social_class);
    let appearance = adw::PreferencesGroup::builder().title("Appearance").build();
    appearance.add(&age);
    appearance.add(&height);
    appearance.add(&weight);
    let (class_group, new_classes) = classes_group(classes, &choices.new_classes);
    let advanced = Rc::new(RefCell::new(choices.character.clone()));
    let level_listeners: Listeners = Rc::default();
    let experience = experience_group(&choices.advance, &advanced, &level_listeners);
    let page = adw::PreferencesPage::new();
    page.add(&character);
    page.add(&class_group);
    page.add(&experience);
    page.add(&appearance);
    page.add(&abilities);

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
    let stack = adw::ViewStack::new();
    stack.add_titled_with_icon(&page, Some("details"), "Details", "avatar-default-symbolic");
    stack.add_titled_with_icon(
        &editors.proficiencies_page(),
        Some("proficiencies"),
        "Proficiencies",
        "view-list-symbolic",
    );
    stack.add_titled_with_icon(
        &editors.spells_page(),
        Some("spells"),
        "Spells",
        "starred-symbolic",
    );
    stack.add_titled_with_icon(
        &editors.equipment_page(),
        Some("equipment"),
        "Equipment",
        "package-x-generic-symbolic",
    );
    let error = adw::Banner::new("");

    let cancel = gtk::Button::with_mnemonic("_Cancel");
    let save_button = gtk::Button::builder()
        .label("_Save")
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
        .title("Edit Character")
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
            };
            let mut lists = editors.lists();
            lists.new_classes = new_classes.borrow().clone();
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
        .tooltip_text("Add Class")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let group = adw::PreferencesGroup::builder()
        .title("Classes")
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
            group.set_description(Some(
                "No class meets the dual-class requirements of these abilities",
            ));
            add.set_sensitive(false);
            return (group, added);
        }
        ClassChoice::Allowed(names) => names.clone(),
    };
    group.set_description(Some(
        "Dual-classing: the new class starts at level 1 and the earlier class stops advancing",
    ));
    add.connect_clicked({
        let (group, added) = (group.clone(), added.clone());
        move |button| {
            let (group, added, button) = (group.clone(), added.clone(), button.clone());
            pick(&button.clone(), "Classes", offered.clone(), move |name| {
                let remove = gtk::Button::builder()
                    .icon_name("list-remove-symbolic")
                    .tooltip_text("Remove")
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&name).as_str())
                    .subtitle("New, level 1")
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
            });
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
        .label("Add…")
        .tooltip_text("Add or take away experience")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let group = adw::PreferencesGroup::builder()
        .title("Experience")
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
    let hit_points = gtk::Label::builder()
        .css_classes(["numeric"])
        .valign(gtk::Align::Center)
        .build();
    let hp_row = adw::ActionRow::builder().title("Hit Points").build();
    hp_row.add_suffix(&hit_points);
    group.add(&hp_row);
    let points = character.skills_and_powers().then(|| {
        let label = gtk::Label::builder()
            .css_classes(["numeric"])
            .valign(gtk::Align::Center)
            .build();
        let row = adw::ActionRow::builder()
            .title("Character Points")
            .subtitle("Unspent")
            .build();
        row.add_suffix(&label);
        group.add(&row);
        label
    });
    let refresh = Rc::new({
        let (rules, advanced, listeners) = (rules.clone(), advanced.clone(), listeners.clone());
        move || {
            for listener in listeners.borrow().iter() {
                listener();
            }
            let character = advanced.borrow();
            match rules.levels(&character) {
                Ok(levels) => {
                    for (row, class) in class_rows.iter().zip(levels) {
                        let mut text =
                            format!("Level {} · {} XP", class.level, thousands(class.experience));
                        match (class.next, class.limit) {
                            (_, Some(limit)) if class.level >= limit => {
                                text.push_str(&format!(" · level limit {limit}"))
                            }
                            (Some(next), _) => {
                                text.push_str(&format!(" · next level at {}", thousands(next)))
                            }
                            (None, Some(limit)) => {
                                text.push_str(&format!(" · level limit {limit}"))
                            }
                            (None, None) => {}
                        }
                        row.set_subtitle(&text);
                    }
                }
                Err(e) => group_error(&class_rows, &e.to_string()),
            }
            hit_points.set_label(
                &rules
                    .hit_points(&character)
                    .map_or_else(|_| "?".into(), |hp| hp.to_string()),
            );
            if let Some(points) = &points {
                points.set_label(&character.data.unknown_0x1778[0].to_string());
            }
        }
    });
    refresh();
    if rules.expansion() {
        let limits = adw::ComboRow::builder()
            .title("Level Limits")
            .model(&gtk::StringList::new(&[
                "None",
                "Racial Limits",
                "Racial Limits and Prime Requisite Bonus",
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
            let over = if used > have { " (too many)" } else { "" };
            Some(format!("{}: {used} of {have}{over}", ordinal(level)))
        })
        .collect();
    Some(if parts.is_empty() {
        "No spell slots at this level".into()
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
        .title("Experience")
        .subtitle("Negative to take experience away")
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
        targets.push("All Classes".into());
        targets.push("Split Between Classes".into());
    }
    let target = adw::ComboRow::builder()
        .title("Give To")
        .model(&gtk::StringList::new(
            &targets.iter().map(String::as_str).collect::<Vec<_>>(),
        ))
        .build();
    if names.len() > 1 {
        target.set_selected(names.len() as u32 + 1);
    }
    let prime = adw::SwitchRow::builder()
        .title("Prime Requisite Bonus")
        .subtitle("10% more for a class whose prime requisites are all 16 or more")
        .active(true)
        .build();
    let points = adw::SpinRow::builder()
        .title("Character Points per Level")
        .numeric(true)
        .adjustment(&gtk::Adjustment::new(3.0, 0.0, 10.0, 1.0, 1.0, 0.0))
        .visible(character.skills_and_powers())
        .build();
    let group = adw::PreferencesGroup::builder()
        .description("Each level gained rolls its hit points. Levels lost keep their rolls, as in the original.")
        .build();
    group.add(&amount);
    group.add(&target);
    group.add(&prime);
    group.add(&points);
    let page = adw::PreferencesPage::new();
    page.add(&group);
    let cancel = gtk::Button::with_mnemonic("_Cancel");
    let give = gtk::Button::builder()
        .label("_Add")
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
        .title("Add Experience")
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
    ["Strength", "Stamina", "Muscle"],
    ["Dexterity", "Aim", "Balance"],
    ["Constitution", "Health", "Fitness"],
    ["Intelligence", "Reason", "Knowledge"],
    ["Wisdom", "Intuition", "Willpower"],
    ["Charisma", "Leadership", "Appearance"],
];

/// Names the list editors offer, from the installation's databases.
struct Choices {
    weapons: Rc<Vec<String>>,
    nonweapon: Rc<Vec<String>>,
    languages: Rc<Vec<String>>,
    wizard_spells: Rc<Vec<String>>,
    priest_spells: Rc<Vec<String>>,
    items: Rc<Vec<String>>,
    wizard_levels: Rc<HashMap<String, u32>>,
    priest_levels: Rc<HashMap<String, u32>>,
    class_ids: HashMap<String, i32>,
    /// Classes the character may add, or why none.
    new_classes: ClassChoice,
    advance: Rc<AdvanceRules>,
    /// The character as read, for experience and levels.
    character: Character,
}

impl Choices {
    fn load(install: &Installation, character: &Character) -> cr_store::Result<Self> {
        let mut items = Vec::new();
        for part_type in ITEM_TYPES {
            items.extend(install.part_names(part_type)?);
        }
        // Coins are edited as money.
        items.retain(|n| !COINS.iter().any(|c| c.0 == n));
        items.sort_by_key(|n| n.to_lowercase());
        items.dedup();
        Ok(Self {
            weapons: Rc::new(install.weapon_proficiency_names()?),
            nonweapon: Rc::new(install.part_names("Non-Weapon Proficiencies")?),
            languages: Rc::new(install.part_names("Languages")?),
            wizard_spells: Rc::new(install.spell_names(true)?),
            priest_spells: Rc::new(install.spell_names(false)?),
            items: Rc::new(items),
            wizard_levels: Rc::new(install.spell_levels(true)?),
            priest_levels: Rc::new(install.spell_levels(false)?),
            class_ids: install.class_ids()?,
            new_classes: install.class_table()?.choices(character),
            advance: Rc::new(install.advance_rules()?),
            character: character.clone(),
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
    let suffix = match (level % 10, level % 100) {
        (1, n) if n != 11 => "st",
        (2, n) if n != 12 => "nd",
        (3, n) if n != 13 => "rd",
        _ => "th",
    };
    format!("{level}{suffix}")
}

impl ListEditors {
    fn new(lists: &CharacterLists, choices: &Choices, classes: &[String]) -> Self {
        let wizard_spells = ListEditor::new(
            "Wizard Spells Known",
            &lists.wizard_spells,
            choices.wizard_spells.clone(),
        );
        let priest_spells = ListEditor::new(
            "Priest Spells Known",
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
                    &format!("{} ({})", book.name, BUNDLE_NAMES[book.bundle]),
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
                        list.group
                            .set_description(Some(&format!("{used} of {total} pages used")));
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
                            subtitle: format!("{} level", ordinal(level as u32)),
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
            let list = RowList::new(&format!("Memorized ({class})"), rows, offer, move |name| {
                let level = levels.get(&name).copied()?;
                Some(RowSpec {
                    value: level,
                    subtitle: format!("{} level", ordinal(level)),
                    title: name,
                    number: None,
                })
            });
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
                            format!("Holds {}", item.contents.join(", "))
                        },
                        number: Some((item.quantity, 1, 99_999)),
                        value: item.clone(),
                    })
                    .collect();
                let items = choices.items.clone();
                let list = RowList::new(
                    BUNDLE_NAMES[b],
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
                (b, list)
            })
            .collect();

        let coins = std::array::from_fn(|c| {
            adw::SpinRow::builder()
                .title(["Copper", "Silver", "Electrum", "Gold", "Platinum"][c])
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
                "Weapon Proficiencies",
                &lists.weapon_proficiencies,
                choices.weapons.clone(),
            ),
            nonweapon_proficiencies: ListEditor::new(
                "Nonweapon Proficiencies",
                &lists.nonweapon_proficiencies,
                choices.nonweapon.clone(),
            ),
            languages: ListEditor::new(
                "Native Languages",
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
            .title("Spending Money")
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
