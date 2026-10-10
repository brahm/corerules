//! The GNOME app: browses the data of a Core Rules installation in three
//! panes (sections, entries, details) that collapse on narrow windows.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_store::Installation;
use cr_store::catalog::{Entry, Section};
use cr_store::edit::CharacterDetails;
use gtk::{gio, glib};

const APP_ID: &str = "io.github.brahm.CoreRules";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
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
        };
        this.browser.edit_button.connect_clicked({
            let this = this.clone();
            move |_| {
                let key = this.browser.current_key.borrow().clone();
                this.edit_character(&key);
            }
        });

        let open = gio::ActionEntry::builder("open")
            .activate({
                let this = this.clone();
                move |_, _, _| this.choose_folder()
            })
            .build();
        this.window.add_action_entries([open]);
        app.set_accels_for_action("win.open", &["<Control>o"]);
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
        *self.install.borrow_mut() = Some(install);
    }

    fn toast(&self, message: &str) {
        self.toasts.add_toast(adw::Toast::new(message));
    }

    /// Opens the editor for the character stored in `Chars/<file>`.
    fn edit_character(&self, file: &str) {
        let loaded = match &*self.install.borrow() {
            Some(install) => install.character(file),
            None => return,
        };
        let details = match loaded {
            Ok(character) => CharacterDetails::of(&character),
            Err(e) => return self.toast(&e.to_string()),
        };
        let this = self.clone();
        let file = file.to_owned();
        character_editor(&self.window, &details, move |details| {
            let install = this.install.borrow();
            let Some(install) = install.as_ref() else {
                return Err("The installation is no longer open".into());
            };
            install
                .save_character_details(&file, details)
                .map_err(|e| e.to_string())?;
            this.browser.load(install.catalog(), Some(&file));
            this.toast("Character saved");
            Ok(())
        });
    }

    fn show_status(&self, widget: &impl IsA<gtk::Widget>) {
        if let Some(old) = self.stack.child_by_name("status") {
            self.stack.remove(&old);
        }
        self.stack.add_named(widget, Some("status"));
        self.stack.set_visible_child_name("status");
    }
}

fn open_menu() -> gtk::MenuButton {
    let menu = gio::Menu::new();
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
    /// The key of the entry on the detail page.
    current_key: Rc<RefCell<String>>,
}

/// The right-hand pane.
#[derive(Clone)]
struct DetailPane {
    page: adw::NavigationPage,
    view: adw::ToolbarView,
    edit_button: gtk::Button,
    current_key: Rc<RefCell<String>>,
}

impl DetailPane {
    fn show(&self, entry: &Entry) {
        self.page.set_title(&entry.title);
        self.view.set_content(Some(&detail(entry)));
        self.edit_button.set_visible(!entry.key.is_empty());
        self.current_key.borrow_mut().clone_from(&entry.key);
    }

    fn show_placeholder(&self, widget: &gtk::Widget) {
        self.page.set_title("Details");
        self.view.set_content(Some(widget));
        self.edit_button.set_visible(false);
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
        let detail_header = adw::HeaderBar::new();
        detail_header.pack_end(&edit_button);
        detail_view.add_top_bar(&detail_header);
        detail_view.set_content(Some(&placeholder("Nothing Selected", "")));
        detail_page.set_child(Some(&detail_view));
        let current_key: Rc<RefCell<String>> = Rc::default();
        let detail = DetailPane {
            page: detail_page.clone(),
            view: detail_view,
            edit_button: edit_button.clone(),
            current_key: current_key.clone(),
        };

        // Entries.
        let model = gio::ListStore::new::<glib::BoxedAnyObject>();
        let search = gtk::SearchEntry::builder()
            .placeholder_text("Search")
            .hexpand(true)
            .build();
        let filter = gtk::CustomFilter::new({
            let search = search.clone();
            move |item| {
                let needle = search.text().to_lowercase();
                needle.is_empty() || {
                    let item = item.downcast_ref::<glib::BoxedAnyObject>().unwrap();
                    let row = item.borrow::<Row>();
                    row.title.to_lowercase().contains(&needle)
                        || row.subtitle.to_lowercase().contains(&needle)
                }
            }
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
        let entries_view = adw::ToolbarView::new();
        entries_view.add_top_bar(&adw::HeaderBar::new());
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
            let (entries_page, inner_page, detail) =
                (entries_page.clone(), inner_page.clone(), detail.clone());
            let current = current.clone();
            move |_, row| {
                let Some(row) = row else { return };
                let index = row.index() as usize;
                let sections = sections.borrow();
                let section = &sections[index];
                *current.borrow_mut() = index;
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
                show_empty(rows.len() as u32);
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
                .css_classes(["property"])
                .build();
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
    save: impl Fn(&CharacterDetails) -> Result<(), String> + 'static,
) {
    let entry_row =
        |title: &str, text: &str| adw::EntryRow::builder().title(title).text(text).build();
    let spin_row = |title: &str, subtitle: &str, value: u32, max: f64| {
        adw::SpinRow::builder()
            .title(title)
            .subtitle(subtitle)
            .adjustment(&gtk::Adjustment::new(
                value.into(),
                0.0,
                max,
                1.0,
                10.0,
                0.0,
            ))
            .build()
    };
    let name = entry_row("Name", &details.name);
    let player = entry_row("Player", &details.player);
    let social_class = entry_row("Social Class", &details.social_class);
    let age = spin_row("Age", "Years", details.age, 9999.0);
    let height = spin_row("Height", "Inches", details.height, 999.0);
    let weight = spin_row("Weight", "Pounds", details.weight, 99999.0);

    let character = adw::PreferencesGroup::new();
    character.add(&name);
    character.add(&player);
    character.add(&social_class);
    let appearance = adw::PreferencesGroup::builder().title("Appearance").build();
    appearance.add(&age);
    appearance.add(&height);
    appearance.add(&weight);
    let error = gtk::Label::builder()
        .wrap(true)
        .visible(false)
        .css_classes(["error"])
        .build();
    let page = adw::PreferencesPage::new();
    page.add(&character);
    page.add(&appearance);
    let errors = adw::PreferencesGroup::new();
    errors.add(&error);
    page.add(&errors);

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
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&page));
    let dialog = adw::Dialog::builder()
        .title("Edit Character")
        .content_width(440)
        .content_height(480)
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
            };
            match save(&details) {
                Ok(()) => {
                    dialog.close();
                }
                Err(message) => {
                    error.set_label(&message);
                    error.set_visible(true);
                }
            }
        }
    });
    dialog.present(Some(parent));
}
