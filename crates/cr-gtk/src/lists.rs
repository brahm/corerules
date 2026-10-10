//! Editing a list of names chosen from a rules database: a group of rows
//! with remove buttons, and an Add button that opens a searchable picker.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

/// A group of names the user can add to and remove from.
#[derive(Clone)]
pub struct ListEditor {
    pub group: adw::PreferencesGroup,
    items: Rc<RefCell<Vec<String>>>,
    rows: Rc<RefCell<Vec<adw::ActionRow>>>,
    empty: adw::ActionRow,
}

impl ListEditor {
    /// `choices` are offered by the Add button; names already in the list
    /// are left out of the picker.
    pub fn new(title: &str, items: &[String], choices: Rc<Vec<String>>) -> Self {
        let add = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text(format!("Add to {title}"))
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let group = adw::PreferencesGroup::builder()
            .title(glib::markup_escape_text(title).as_str())
            .header_suffix(&add)
            .build();
        let empty = adw::ActionRow::builder()
            .title("None")
            .css_classes(["dim-label"])
            .build();
        group.add(&empty);
        let editor = Self {
            group,
            items: Rc::default(),
            rows: Rc::default(),
            empty,
        };
        for item in items {
            editor.push(item.clone());
        }
        editor.update_empty();

        add.connect_clicked({
            let editor = editor.clone();
            let title = title.to_owned();
            move |button| {
                let taken = editor.items.borrow().clone();
                let offered: Vec<String> = choices
                    .iter()
                    .filter(|c| !taken.contains(c))
                    .cloned()
                    .collect();
                let editor = editor.clone();
                pick(button, &title, offered, move |name| {
                    editor.push(name);
                    editor.update_empty();
                });
            }
        });
        editor
    }

    pub fn items(&self) -> Vec<String> {
        self.items.borrow().clone()
    }

    fn push(&self, name: String) {
        let remove = gtk::Button::builder()
            .icon_name("list-remove-symbolic")
            .tooltip_text("Remove")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&name).as_str())
            .build();
        row.add_suffix(&remove);
        self.group.add(&row);
        self.items.borrow_mut().push(name);
        self.rows.borrow_mut().push(row.clone());
        remove.connect_clicked({
            let editor = self.clone();
            move |_| {
                let position = editor.rows.borrow().iter().position(|r| *r == row);
                if let Some(position) = position {
                    editor.rows.borrow_mut().remove(position);
                    editor.items.borrow_mut().remove(position);
                    editor.group.remove(&row);
                    editor.update_empty();
                }
            }
        });
    }

    fn update_empty(&self) {
        self.empty.set_visible(self.items.borrow().is_empty());
    }
}

/// A dialog listing `choices` with a search entry; `chosen` gets the name
/// the user activates.
fn pick(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    choices: Vec<String>,
    chosen: impl Fn(String) + 'static,
) {
    let model = gtk::StringList::new(&choices.iter().map(String::as_str).collect::<Vec<_>>());
    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search")
        .hexpand(true)
        .build();
    let filter = gtk::StringFilter::builder()
        .expression(gtk::PropertyExpression::new(
            gtk::StringObject::static_type(),
            gtk::Expression::NONE,
            "string",
        ))
        .match_mode(gtk::StringFilterMatchMode::Substring)
        .ignore_case(true)
        .build();
    search
        .bind_property("text", &filter, "search")
        .sync_create()
        .build();
    let filtered = gtk::FilterListModel::new(Some(model), Some(filter));
    // Names that start with the search come first, so Enter picks "Torch"
    // rather than "Lantern/torchbearer".
    let sorter = gtk::CustomSorter::new({
        let search = search.clone();
        move |a, b| {
            let needle = search.text().to_lowercase();
            let rank = |o: &glib::Object| {
                let text = o.downcast_ref::<gtk::StringObject>().unwrap().string();
                !text.to_lowercase().starts_with(&needle)
            };
            rank(a).cmp(&rank(b)).into()
        }
    });
    search.connect_search_changed({
        let sorter = sorter.clone();
        move |_| sorter.changed(gtk::SorterChange::Different)
    });
    let filtered = gtk::SortListModel::new(Some(filtered), Some(sorter));
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let label = gtk::Label::builder()
            .xalign(0.0)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(6)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        item.downcast_ref::<gtk::ListItem>()
            .unwrap()
            .set_child(Some(&label));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let text = item.item().and_downcast::<gtk::StringObject>().unwrap();
        let label = item.child().and_downcast::<gtk::Label>().unwrap();
        label.set_label(&text.string());
    });
    let list = gtk::ListView::builder()
        .model(&gtk::SingleSelection::new(Some(filtered)))
        .factory(&factory)
        .single_click_activate(true)
        .css_classes(["navigation-sidebar"])
        .build();
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&list)
        .build();
    let search_bar = gtk::SearchBar::builder()
        .child(&search)
        .search_mode_enabled(true)
        .build();
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.add_top_bar(&search_bar);
    view.set_content(Some(&scroll));
    let dialog = adw::Dialog::builder()
        .title(format!("Add to {title}"))
        .content_width(360)
        .content_height(480)
        .child(&view)
        .build();
    list.connect_activate({
        let dialog = dialog.clone();
        move |list, position| {
            let Some(text) = list
                .model()
                .and_then(|m| m.item(position))
                .and_downcast::<gtk::StringObject>()
            else {
                return;
            };
            chosen(text.string().into());
            dialog.close();
        }
    });
    // Enter in the search entry picks the first match.
    search.connect_activate({
        let list = list.clone();
        move |_| {
            if list.model().is_some_and(|m| m.n_items() > 0) {
                list.emit_by_name::<()>("activate", &[&0u32]);
            }
        }
    });
    dialog.present(Some(parent));
    search.grab_focus();
}
