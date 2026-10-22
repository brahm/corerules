//! Editing a list of names chosen from a rules database: a group of rows
//! with remove buttons, and an Add button that opens a searchable picker.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::tr;
use cr_store::pictures::ItemIcons;
use gtk::glib;

/// A group of names the user can add to and remove from.
#[derive(Clone)]
pub struct ListEditor {
    pub group: adw::PreferencesGroup,
    items: Rc<RefCell<Vec<String>>>,
    rows: Rc<RefCell<Vec<adw::ActionRow>>>,
    empty: adw::ActionRow,
    add_check: AddCheck,
    detail: Rc<RefCell<Option<Detail>>>,
}

impl ListEditor {
    /// `choices` are offered by the Add button; names already in the list
    /// are left out of the picker.
    pub fn new(title: &str, items: &[String], choices: Rc<Vec<String>>) -> Self {
        let add = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text(tr!("Add to {list}", list = title))
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let group = adw::PreferencesGroup::builder()
            .title(glib::markup_escape_text(title).as_str())
            .header_suffix(&add)
            .build();
        let empty = adw::ActionRow::builder()
            .title(tr("None"))
            .css_classes(["dim-label"])
            .build();
        group.add(&empty);
        let editor = Self {
            group,
            items: Rc::default(),
            rows: Rc::default(),
            empty,
            add_check: Rc::default(),
            detail: Rc::default(),
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
                let detail = editor.detail.borrow().clone();
                let editor = editor.clone();
                let title = tr!("Add to {list}", list = title);
                pick_detailed(button, &title, offered, detail, move |name| {
                    let add: Box<dyn FnOnce()> = {
                        let (editor, name) = (editor.clone(), name.clone());
                        Box::new(move || {
                            editor.push(name);
                            editor.update_empty();
                        })
                    };
                    match &*editor.add_check.borrow() {
                        Some(check) => check(&name, add),
                        None => add(),
                    }
                });
            }
        });
        editor
    }

    pub fn items(&self) -> Vec<String> {
        self.items.borrow().clone()
    }

    /// Asks `f` before a name picked with Add joins the list, with the
    /// name and what adds it.
    pub fn connect_add(&self, f: impl Fn(&str, Box<dyn FnOnce()>) + 'static) {
        *self.add_check.borrow_mut() = Some(Box::new(f));
    }

    /// Shows `f`'s line under each name Add offers.
    pub fn set_detail(&self, f: impl Fn(&str) -> String + 'static) {
        *self.detail.borrow_mut() = Some(Rc::new(f));
    }

    fn push(&self, name: String) {
        let remove = gtk::Button::builder()
            .icon_name("list-remove-symbolic")
            .tooltip_text(tr("Remove"))
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
/// the user activates. The dialog is titled "Add to `list`".
pub fn pick(
    parent: &impl IsA<gtk::Widget>,
    list: &str,
    choices: Vec<String>,
    chosen: impl Fn(String) + 'static,
) {
    pick_titled(parent, &tr!("Add to {list}", list), choices, chosen);
}

/// [`pick`] under a title of its own.
pub fn pick_titled(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    choices: Vec<String>,
    chosen: impl Fn(String) + 'static,
) {
    pick_detailed(parent, title, choices, None, chosen);
}

/// A line under each name the picker shows, from the name.
pub type Detail = Rc<dyn Fn(&str) -> String>;

/// [`pick_titled`] with `detail` under each name.
fn pick_detailed(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    choices: Vec<String>,
    detail: Option<Detail>,
    chosen: impl Fn(String) + 'static,
) {
    let model = gtk::StringList::new(&choices.iter().map(String::as_str).collect::<Vec<_>>());
    let search = gtk::SearchEntry::builder()
        .placeholder_text(tr("Search"))
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
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let detail = gtk::Label::builder()
            .xalign(0.0)
            .visible(false)
            .css_classes(["dim-label", "caption"])
            .build();
        let column = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(6)
            .build();
        column.append(&label);
        column.append(&detail);
        item.downcast_ref::<gtk::ListItem>()
            .unwrap()
            .set_child(Some(&column));
    });
    factory.connect_bind(move |_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let text = item.item().and_downcast::<gtk::StringObject>().unwrap();
        let column = item.child().and_downcast::<gtk::Box>().unwrap();
        let label = column.first_child().and_downcast::<gtk::Label>().unwrap();
        let line = column.last_child().and_downcast::<gtk::Label>().unwrap();
        label.set_label(&text.string());
        let below = detail
            .as_ref()
            .map(|d| d(&text.string()))
            .unwrap_or_default();
        line.set_visible(!below.is_empty());
        line.set_label(&below);
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
        .title(title)
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

/// A row a [`RowList`] shows for one value.
pub struct RowSpec<T> {
    pub value: T,
    pub title: String,
    pub subtitle: String,
    /// A number the user can change (a quantity, pages), with its range.
    pub number: Option<(u32, u32, u32)>,
}

type Rows<T> = Rc<RefCell<Vec<(T, adw::ActionRow, Option<adw::SpinRow>)>>>;
type Callback = Rc<RefCell<Option<Box<dyn Fn()>>>>;
/// Asked before a row is removed, with the row's value and what removes
/// it.
type RemoveCheck<T> = Rc<RefCell<Option<Box<dyn Fn(&T, Box<dyn FnOnce()>)>>>>;

/// Asked before a chosen name becomes a row, with the name and what adds
/// it.
type AddCheck = Rc<RefCell<Option<Box<dyn Fn(&str, Box<dyn FnOnce()>)>>>>;

/// How big an item's icon is drawn at the start of its row.
const ROW_ICON_SIZE: i32 = 40;

/// A group of rows, each holding a value of type `T` and maybe a number,
/// with remove buttons and an Add button whose picker offers names that
/// `make` turns into rows.
pub struct RowList<T: 'static> {
    pub group: adw::PreferencesGroup,
    rows: Rows<T>,
    empty: adw::ActionRow,
    /// Called after every change.
    changed: Callback,
    remove_check: RemoveCheck<T>,
    add_check: AddCheck,
    detail: Rc<RefCell<Option<Detail>>>,
    /// The icons drawn at the start of the rows, by title (item name).
    icons: Rc<RefCell<Option<Rc<ItemIcons>>>>,
}

impl<T: Clone + 'static> Clone for RowList<T> {
    fn clone(&self) -> Self {
        Self {
            group: self.group.clone(),
            rows: self.rows.clone(),
            empty: self.empty.clone(),
            changed: self.changed.clone(),
            remove_check: self.remove_check.clone(),
            add_check: self.add_check.clone(),
            detail: self.detail.clone(),
            icons: self.icons.clone(),
        }
    }
}

impl<T: Clone + 'static> RowList<T> {
    /// `choices` gives the names to offer when Add is pressed (it may
    /// depend on other lists); `make` turns a chosen name into a row.
    pub fn new(
        title: &str,
        rows: Vec<RowSpec<T>>,
        choices: impl Fn() -> Vec<String> + 'static,
        make: impl Fn(String) -> Option<RowSpec<T>> + 'static,
    ) -> Self {
        let add = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text(tr!("Add to {list}", list = title))
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let group = adw::PreferencesGroup::builder()
            .title(glib::markup_escape_text(title).as_str())
            .header_suffix(&add)
            .build();
        let empty = adw::ActionRow::builder()
            .title(tr("None"))
            .css_classes(["dim-label"])
            .build();
        group.add(&empty);
        let list = Self {
            group,
            rows: Rc::default(),
            empty,
            changed: Rc::default(),
            remove_check: Rc::default(),
            add_check: Rc::default(),
            detail: Rc::default(),
            icons: Rc::default(),
        };
        for row in rows {
            list.push(row);
        }
        list.update();
        let title = title.to_owned();
        let make = Rc::new(make);
        add.connect_clicked({
            let list = list.clone();
            move |button| {
                let (list, make) = (list.clone(), make.clone());
                let detail = list.detail.borrow().clone();
                let heading = tr!("Add to {list}", list = title);
                pick_detailed(button, &heading, choices(), detail, move |name| {
                    let add: Box<dyn FnOnce()> = {
                        let (list, make, name) = (list.clone(), make.clone(), name.clone());
                        Box::new(move || {
                            if let Some(spec) = make(name) {
                                list.push(spec);
                                list.update();
                            }
                        })
                    };
                    match &*list.add_check.borrow() {
                        Some(check) => check(&name, add),
                        None => add(),
                    }
                });
            }
        });
        list
    }

    pub fn push(&self, spec: RowSpec<T>) {
        let remove = gtk::Button::builder()
            .icon_name("list-remove-symbolic")
            .tooltip_text(tr("Remove"))
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let (row, spin) = match spec.number {
            Some((value, min, max)) => {
                let spin = adw::SpinRow::builder()
                    .numeric(true)
                    .adjustment(&gtk::Adjustment::new(
                        value.into(),
                        min.into(),
                        max.into(),
                        1.0,
                        5.0,
                        0.0,
                    ))
                    .build();
                spin.connect_value_notify({
                    let changed = self.changed.clone();
                    move |_| {
                        if let Some(changed) = &*changed.borrow() {
                            changed();
                        }
                    }
                });
                (spin.clone().upcast::<adw::ActionRow>(), Some(spin))
            }
            None => (adw::ActionRow::new(), None),
        };
        row.set_title(&glib::markup_escape_text(&spec.title));
        if !spec.subtitle.is_empty() {
            row.set_subtitle(&glib::markup_escape_text(&spec.subtitle));
        }
        if let Some(icons) = &*self.icons.borrow() {
            add_icon(&row, icons);
        }
        row.add_suffix(&remove);
        self.group.add(&row);
        self.rows.borrow_mut().push((spec.value, row.clone(), spin));
        remove.connect_clicked({
            let list = self.clone();
            move |_| {
                let remove: Box<dyn FnOnce()> = Box::new({
                    let (list, row) = (list.clone(), row.clone());
                    move || {
                        let position = list.rows.borrow().iter().position(|r| r.1 == row);
                        if let Some(position) = position {
                            list.rows.borrow_mut().remove(position);
                            list.group.remove(&row);
                            list.update();
                        }
                    }
                });
                let value = list
                    .rows
                    .borrow()
                    .iter()
                    .find(|r| r.1 == row)
                    .map(|r| r.0.clone());
                match (&*list.remove_check.borrow(), value) {
                    (Some(check), Some(value)) => check(&value, remove),
                    _ => remove(),
                }
            }
        });
    }

    /// Draws, at the start of each row, the icon of the item its title
    /// names, the rows there now and those added later.
    pub fn show_icons(&self, icons: Rc<ItemIcons>) {
        for (_, row, _) in self.rows.borrow().iter() {
            add_icon(row, &icons);
        }
        *self.icons.borrow_mut() = Some(icons);
    }

    /// The values and their numbers, in order.
    pub fn values(&self) -> Vec<(T, Option<u32>)> {
        self.rows
            .borrow()
            .iter()
            .map(|(value, _, spin)| (value.clone(), spin.as_ref().map(|s| s.value() as u32)))
            .collect()
    }

    /// The row titles (names), in order.
    pub fn rows_titles(&self) -> Vec<String> {
        self.rows
            .borrow()
            .iter()
            .map(|(_, row, _)| plain_title(row))
            .collect()
    }

    /// Removes every row.
    pub fn clear(&self) {
        for (_, row, _) in self.rows.borrow_mut().drain(..) {
            self.group.remove(&row);
        }
        self.update();
    }

    /// Adds a row as Add does.
    pub fn add(&self, spec: RowSpec<T>) {
        self.push(spec);
        self.update();
    }

    /// Rewrites each row's subtitle from its value and number.
    pub fn set_subtitles(&self, f: impl Fn(&T, Option<u32>) -> String) {
        for (value, row, spin) in self.rows.borrow().iter() {
            let subtitle = f(value, spin.as_ref().map(|s| s.value() as u32));
            row.set_subtitle(&glib::markup_escape_text(&subtitle));
        }
    }

    /// Has `f` decide whether a row goes: it gets the row's value and what
    /// removes it.
    pub fn connect_remove(&self, f: impl Fn(&T, Box<dyn FnOnce()>) + 'static) {
        *self.remove_check.borrow_mut() = Some(Box::new(f));
    }

    /// Shows `f`'s line under each name Add offers.
    pub fn set_detail(&self, f: impl Fn(&str) -> String + 'static) {
        *self.detail.borrow_mut() = Some(Rc::new(f));
    }

    /// Asks `f` before a name picked with Add becomes a row, with the name
    /// and what adds it.
    pub fn connect_add(&self, f: impl Fn(&str, Box<dyn FnOnce()>) + 'static) {
        *self.add_check.borrow_mut() = Some(Box::new(f));
    }

    /// Calls `f` after every change.
    pub fn connect_changed(&self, f: impl Fn() + 'static) {
        *self.changed.borrow_mut() = Some(Box::new(f));
        self.update();
    }

    /// Calls `f` after every change too, after what is called already.
    pub fn also_connect_changed(&self, f: impl Fn() + 'static) {
        let before = self.changed.borrow_mut().take();
        *self.changed.borrow_mut() = Some(Box::new(move || {
            if let Some(before) = &before {
                before();
            }
            f();
        }));
        self.update();
    }

    fn update(&self) {
        self.empty.set_visible(self.rows.borrow().is_empty());
        if let Some(changed) = &*self.changed.borrow() {
            changed();
        }
    }
}

/// `row`'s title without its markup.
fn plain_title(row: &adw::ActionRow) -> String {
    let title = row.title();
    gtk::pango::parse_markup(&title, '\0').map_or(title.to_string(), |m| m.1.into())
}

/// The icon of the item `row` names, at its start; a dimmed stand-in when
/// the item has none, so the titles line up.
fn add_icon(row: &adw::ActionRow, icons: &ItemIcons) {
    let image = gtk::Image::builder()
        .pixel_size(ROW_ICON_SIZE)
        .valign(gtk::Align::Center)
        .build();
    match icons
        .get(&plain_title(row))
        .and_then(crate::picture::icon_texture)
    {
        Some(texture) => image.set_paintable(Some(&texture)),
        None => {
            image.set_icon_name(Some("package-x-generic-symbolic"));
            image.add_css_class("dim-label");
        }
    }
    row.add_prefix(&image);
}
