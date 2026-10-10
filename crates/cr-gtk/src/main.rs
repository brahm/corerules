//! The GNOME app: browses the data of a Core Rules installation in three
//! panes (sections, entries, details) that collapse on narrow windows.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_store::Installation;
use cr_store::catalog::{Entry, Section};
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
                Some(install) => window.show_installation(&install),
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
    browser: Rc<Browser>,
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
        window.set_content(Some(&stack));
        let browser = Browser::new();
        browser.breakpoints(&window);
        stack.add_named(&browser.outer, Some("browser"));
        let this = Self {
            window,
            stack,
            browser: Rc::new(browser),
        };

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
            Ok(install) => self.show_installation(&install),
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

    fn show_installation(&self, install: &Installation) {
        self.browser.load(install.catalog());
        self.window.set_title(Some(&format!(
            "Core Rules — {}",
            install
                .root()
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default()
        )));
        self.stack.set_visible_child_name("browser");
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
        detail_view.add_top_bar(&adw::HeaderBar::new());
        detail_view.set_content(Some(&placeholder("Nothing Selected", "")));
        detail_page.set_child(Some(&detail_view));

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
            let (entries_page, inner_page, detail_view) = (
                entries_page.clone(),
                inner_page.clone(),
                detail_view.clone(),
            );
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
                detail_view.set_content(Some(&match &section.error {
                    Some(error) => {
                        placeholder_text("Could Not Read", "dialog-error-symbolic", error)
                    }
                    None if section.entries.is_empty() => {
                        placeholder(&format!("No {}", section.title), section.icon)
                    }
                    None => placeholder("Nothing Selected", section.icon),
                }));
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
            let (sections, inner, detail_page, detail_view) = (
                sections.clone(),
                inner.clone(),
                detail_page.clone(),
                detail_view.clone(),
            );
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
                detail_page.set_title(&entry.title);
                detail_view.set_content(Some(&detail(entry)));
                inner.set_show_content(true);
            }
        });
        Self {
            outer,
            inner,
            sidebar_list,
            sections,
        }
    }

    /// Shows the sections of a newly opened installation.
    fn load(&self, sections: Vec<Section>) {
        self.sidebar_list.remove_all();
        for section in &sections {
            self.sidebar_list.append(&section_row(section));
        }
        *self.sections.borrow_mut() = sections;
        if let Some(first) = self.sidebar_list.row_at_index(0) {
            self.sidebar_list.select_row(Some(&first));
        }
        self.outer.set_show_content(false);
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
