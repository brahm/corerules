//! The Encounter Template editor (IDD 289) of the Encounter Templates
//! section: one page per button of the original's dialog (Monsters, NPC's,
//! Items, Traps, Treasure Types, the 2-20 table information), with the
//! Properties dialogs of each line. Standard templates are only copied,
//! as in the original; Delete asks nothing.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::Object;
use cr_formats::encounter::{
    EncounterNpc, EncounterTemplate, TemplateItemProperties, TemplateMonsterProperties,
    TemplateNpcProperties, TemplateNumAppear,
};
use cr_formats::monster::{Monster, MonsterType};
use cr_formats::traps::Trap;
use cr_i18n::tr;
use cr_store::catalog::TEMPLATES;
use cr_store::help::topic;
use cr_store::treasure::LETTERS;
use cr_store::userdb::monsters::{CLIMATES, CUSTOM_TERRAIN, MonsterTypes, Monsters, TERRAINS};
use cr_store::userdb::parts::{self, Parts};
use cr_store::userdb::templates::{self as tpl, Combo, Templates};
use cr_store::userdb::traps::Traps;
use cr_store::userdb::{self as udb, Filter, Flavor, Origin, UserDb};
use cr_store::{Installation, Result};
use gtk::glib;

use super::{Action, Host, Redraw, Shared, Target, act, errors, finish, position};

/// What the editor offers to add: the monsters, monster types, traps and
/// parts of both databases, and the climate/terrain categories.
struct Data {
    flavor: Flavor,
    monsters: UserDb<Monsters>,
    types: UserDb<MonsterTypes>,
    traps: UserDb<Traps>,
    parts: UserDb<Parts>,
    terrains: Vec<String>,
}

impl Data {
    fn load(install: &Installation) -> Result<Self> {
        Ok(Self {
            flavor: cr_store::encounter::flavor_of(install),
            monsters: UserDb::load(install)?,
            types: UserDb::load(install)?,
            traps: UserDb::load(install)?,
            parts: UserDb::load(install)?,
            terrains: udb::monsters::distinct(install, "monster-terrain")?,
        })
    }

    /// The type names of IDD 279's list, sorted.
    fn type_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .types
            .list(Filter::All)
            .iter()
            .map(|e| e.record.name.clone())
            .collect();
        names.dedup();
        names
    }
}

/// New, Copy and Edit open the editor; Delete removes a user template at
/// once.
pub(super) fn action<H: Host + 'static>(host: &Shared<H>, target: &Target, action: Action) {
    if action == Action::Delete {
        return act::<Templates, H>(host, target, 0, action);
    }
    let Some(db) = errors(&**host, host.with_install(UserDb::<Templates>::load)) else {
        return;
    };
    let found = position(&db, target.origin, 0, &target.name);
    let record = found.and_then(|i| db.get(target.origin, i)).cloned();
    match action {
        Action::New | Action::NewSubrace => {
            // The original starts from the monster type selected in its
            // list; here, the shown template's.
            let shown = udb::parse_key(&host.current_key())
                .filter(|(section, ..)| *section == TEMPLATES)
                .and_then(|(_, origin, name)| {
                    position(&db, origin, 0, name).and_then(|i| db.get(origin, i))
                })
                .map(|t| t.monster_type.clone())
                .unwrap_or_default();
            let record = db.new_record(0, &shown);
            editor(host, None, tr("New Encounter Template").to_owned(), record);
        }
        Action::Copy => {
            if let Some(record) = record {
                let copy = db.copy_of(&record);
                editor(host, None, tr!("Copy of {name}", name = target.name), copy);
            }
        }
        Action::Edit => {
            let (Some(record), Some(index)) = (record, found) else {
                return;
            };
            if target.origin == Origin::Standard {
                return host.toast(tr("Standard records cannot be edited; copy them instead"));
            }
            editor(host, Some(index), target.name.clone(), record);
        }
        Action::Delete => {}
    }
}

// ---- small widgets ----

fn spin_button(value: i32, min: i32, max: i32) -> gtk::SpinButton {
    gtk::SpinButton::builder()
        .adjustment(&gtk::Adjustment::new(
            value.clamp(min, max) as f64,
            min as f64,
            max as f64,
            1.0,
            10.0,
            0.0,
        ))
        .numeric(true)
        .width_chars(5)
        .valign(gtk::Align::Center)
        .build()
}

fn spin_row(title: &str, value: i32, min: i32, max: i32) -> adw::SpinRow {
    adw::SpinRow::builder()
        .title(title)
        .adjustment(&gtk::Adjustment::new(
            value.clamp(min, max) as f64,
            min as f64,
            max as f64,
            1.0,
            10.0,
            0.0,
        ))
        .numeric(true)
        .build()
}

fn flat_button(icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
}

fn label_button(label: &str) -> gtk::Button {
    gtk::Button::builder()
        .label(label)
        .use_underline(true)
        .valign(gtk::Align::Center)
        .build()
}

fn escaped(text: &str) -> glib::GString {
    glib::markup_escape_text(text)
}

/// "N d S * M + B" as the originals' four boxes, under a title.
struct DiceRow {
    row: adw::PreferencesRow,
    title: gtk::Label,
    spins: [gtk::SpinButton; 4],
}

impl DiceRow {
    fn new(title: &str, dice: &[i32], ranges: [(i32, i32); 4]) -> Self {
        let spins: [gtk::SpinButton; 4] =
            std::array::from_fn(|i| spin_button(dice[i], ranges[i].0, ranges[i].1));
        let boxed = gtk::Box::builder().spacing(6).build();
        for (i, spin) in spins.iter().enumerate() {
            if i > 0 {
                boxed.append(&gtk::Label::new(Some(["d", "*", "+"][i - 1])));
            }
            spin.set_hexpand(true);
            boxed.append(spin);
        }
        let label = gtk::Label::builder().label(title).xalign(0.0).build();
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();
        content.append(&label);
        content.append(&boxed);
        let row = adw::PreferencesRow::builder()
            .title(title)
            .activatable(false)
            .selectable(false)
            .child(&content)
            .build();
        Self {
            row,
            title: label,
            spins,
        }
    }

    fn values(&self) -> [i32; 4] {
        std::array::from_fn(|i| self.spins[i].value() as i32)
    }
}

/// A list of lines with an Add button: rows that open their line and
/// remove it.
fn list_group(
    title: &str,
    description: &str,
    add: &gtk::Button,
) -> (adw::PreferencesGroup, gtk::ListBox) {
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    list.set_placeholder(Some(
        &gtk::Label::builder()
            .label(tr("None"))
            .margin_top(12)
            .margin_bottom(12)
            .css_classes(["dim-label"])
            .build(),
    ));
    let group = adw::PreferencesGroup::builder()
        .title(escaped(title))
        .description(escaped(description))
        .header_suffix(add)
        .build();
    group.add(&list);
    (group, list)
}

fn clear(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

/// A row with a remove button (returned with it); `open` runs on
/// activation when given.
fn line_row(
    title: &str,
    subtitle: &str,
    remove_tooltip: &str,
    remove: impl Fn() + 'static,
    open: Option<Box<dyn Fn()>>,
) -> (adw::ActionRow, gtk::Button) {
    let row = adw::ActionRow::builder()
        .title(escaped(title).as_str())
        .activatable(open.is_some())
        .build();
    if !subtitle.is_empty() {
        row.set_subtitle(escaped(subtitle).as_str());
    }
    let button = flat_button("list-remove-symbolic", remove_tooltip);
    button.connect_clicked(move |_| remove());
    row.add_suffix(&button);
    if let Some(open) = open {
        row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
        row.connect_activated(move |_| open());
    }
    (row, button)
}

/// A dialog with Cancel and an action button, whose Help opens `help`.
fn sub_dialog(
    title: &str,
    page: &adw::PreferencesPage,
    action: &str,
    help: u32,
) -> (adw::Dialog, gtk::Button) {
    let cancel = gtk::Button::builder()
        .label(tr("_Cancel"))
        .use_underline(true)
        .build();
    let ok = gtk::Button::builder()
        .label(action)
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let header = adw::HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .build();
    header.pack_start(&cancel);
    header.pack_end(&ok);
    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, move || help);
    view.add_top_bar(&header);
    view.set_content(Some(page));
    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(640)
        .content_height(680)
        .width_request(360)
        .height_request(294)
        .child(&view)
        .build();
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    (dialog, ok)
}

// ---- pickers ----

/// What IDD 302 and 306 hand back.
enum Picked {
    Monster(Box<Monster>),
    Type(MonsterType),
}

/// One entry of a picker: its title, subtitle and whether it is the
/// user's.
struct Choice {
    title: String,
    subtitle: String,
    custom: bool,
}

/// A long list shown lazily: `entries` filtered by `accept`, activating
/// one calls `activated` with its index.
fn lazy_list(
    entries: Rc<Vec<Choice>>,
    accept: impl Fn(usize) -> bool + 'static,
    activated: impl Fn(usize) + 'static,
) -> (gtk::ScrolledWindow, gtk::CustomFilter) {
    let indices: Vec<String> = (0..entries.len()).map(|i| i.to_string()).collect();
    let model = gtk::StringList::new(&indices.iter().map(String::as_str).collect::<Vec<_>>());
    let index_of = |o: &glib::Object| -> usize {
        o.downcast_ref::<gtk::StringObject>()
            .and_then(|s| s.string().parse().ok())
            .unwrap_or(usize::MAX)
    };
    let filter = gtk::CustomFilter::new(move |o| accept(index_of(o)));
    let filtered = gtk::FilterListModel::new(Some(model), Some(filter.clone()));
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let title = gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let subtitle = gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["dim-label", "caption"])
            .build();
        let texts = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        texts.append(&title);
        texts.append(&subtitle);
        let icon = gtk::Image::from_icon_name("document-edit-symbolic");
        let row = gtk::Box::builder()
            .spacing(12)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(6)
            .margin_end(6)
            .build();
        row.append(&texts);
        row.append(&icon);
        item.downcast_ref::<gtk::ListItem>()
            .unwrap()
            .set_child(Some(&row));
    });
    factory.connect_bind({
        let entries = entries.clone();
        move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let Some(i) = item.item().map(|o| index_of(&o)) else {
                return;
            };
            let Some(entry) = entries.get(i) else { return };
            let row = item.child().unwrap();
            let texts = row.first_child().unwrap();
            let title = texts.first_child().and_downcast::<gtk::Label>().unwrap();
            let subtitle = title.next_sibling().and_downcast::<gtk::Label>().unwrap();
            title.set_label(&entry.title);
            subtitle.set_label(&entry.subtitle);
            subtitle.set_visible(!entry.subtitle.is_empty());
            if let Some(icon) = texts.next_sibling() {
                icon.set_visible(entry.custom);
            }
        }
    });
    let view = gtk::ListView::builder()
        .model(&gtk::SingleSelection::new(Some(filtered)))
        .factory(&factory)
        .single_click_activate(true)
        .css_classes(["navigation-sidebar"])
        .build();
    view.connect_activate(move |view, position| {
        if let Some(o) = view.model().and_then(|m| m.item(position)) {
            activated(index_of(&o));
        }
    });
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&view)
        .build();
    (scroll, filter)
}

/// A picker dialog: `options` above the list, the search bar and `start`
/// in the header.
fn picker_dialog(
    title: &str,
    help: u32,
    start: &impl IsA<gtk::Widget>,
    search: &gtk::SearchEntry,
    options: Option<&gtk::ListBox>,
    lists: &[&gtk::ScrolledWindow],
) -> adw::Dialog {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    if let Some(options) = options {
        content.append(options);
    }
    for list in lists {
        content.append(*list);
    }
    let header = adw::HeaderBar::new();
    header.pack_start(start);
    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, move || help);
    view.add_top_bar(&header);
    view.add_top_bar(
        &gtk::SearchBar::builder()
            .child(search)
            .search_mode_enabled(true)
            .build(),
    );
    view.set_content(Some(&content));
    adw::Dialog::builder()
        .title(title)
        .content_width(520)
        .content_height(680)
        .width_request(360)
        .height_request(294)
        .child(&view)
        .build()
}

/// IDD 302 / 306: a monster from either database, filtered by origin and
/// type, or (with `types`, "Select Type instead of Monster") a monster
/// type.
fn pick_monster(
    parent: &impl IsA<gtk::Widget>,
    data: &Rc<Data>,
    title: &str,
    help: u32,
    start_type: &str,
    types: bool,
    chosen: impl Fn(Picked) + 'static,
) {
    let origin = gtk::DropDown::builder()
        .model(&gtk::StringList::new(&Filter::NAMES.map(tr)))
        .tooltip_text(tr("Show Standard or Custom Records"))
        .build();
    let mut type_names = vec!["~All".to_owned()];
    type_names.extend(data.type_names());
    let type_row = adw::ComboRow::builder()
        .title(tr("Monster Types"))
        .model(&gtk::StringList::new(
            &type_names.iter().map(String::as_str).collect::<Vec<_>>(),
        ))
        .enable_search(true)
        .expression(gtk::PropertyExpression::new(
            gtk::StringObject::static_type(),
            None::<gtk::Expression>,
            "string",
        ))
        .build();
    type_row.set_selected(type_names.iter().position(|t| t == start_type).unwrap_or(0) as u32);
    let instead = adw::SwitchRow::builder()
        .title(tr("Select Type instead of Monster"))
        .visible(types)
        .build();
    let options = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    options.append(&instead);
    options.append(&type_row);
    let search = gtk::SearchEntry::builder()
        .placeholder_text(tr("Search"))
        .hexpand(true)
        .build();

    let monsters: Rc<Vec<(Origin, Monster)>> = Rc::new(
        data.monsters
            .list(Filter::All)
            .iter()
            .map(|e| (e.origin, e.record.clone()))
            .collect(),
    );
    let type_records: Rc<Vec<(Origin, MonsterType)>> = Rc::new(
        data.types
            .list(Filter::All)
            .iter()
            .map(|e| (e.origin, e.record.clone()))
            .collect(),
    );
    let monster_choices = Rc::new(
        monsters
            .iter()
            .map(|(o, m)| Choice {
                title: m.name.clone(),
                subtitle: m.monster_type.clone(),
                custom: *o == Origin::Custom,
            })
            .collect::<Vec<_>>(),
    );
    let type_choices = Rc::new(
        type_records
            .iter()
            .map(|(o, t)| Choice {
                title: t.name.clone(),
                subtitle: String::new(),
                custom: *o == Origin::Custom,
            })
            .collect::<Vec<_>>(),
    );
    let type_names = Rc::new(type_names);
    let dialog_cell: Rc<glib::WeakRef<adw::Dialog>> = Rc::default();
    let chosen = Rc::new(chosen);
    let close = {
        let dialog_cell = dialog_cell.clone();
        move || {
            if let Some(d) = dialog_cell.upgrade() {
                d.close();
            }
        }
    };
    let (monster_list, monster_filter) = lazy_list(
        monster_choices,
        {
            let (monsters, origin, type_row, search, type_names) = (
                monsters.clone(),
                origin.clone(),
                type_row.clone(),
                search.clone(),
                type_names.clone(),
            );
            move |i| {
                let Some((o, m)) = monsters.get(i) else {
                    return false;
                };
                let wanted = &type_names[(type_row.selected() as usize).min(type_names.len() - 1)];
                Filter::of(origin.selected()).accepts(*o)
                    && (wanted == "~All" || m.monster_type == *wanted)
                    && m.name
                        .to_lowercase()
                        .contains(&search.text().to_lowercase())
            }
        },
        {
            let (monsters, chosen, close) = (monsters.clone(), chosen.clone(), close.clone());
            move |i| {
                if let Some((_, m)) = monsters.get(i) {
                    close();
                    chosen(Picked::Monster(Box::new(m.clone())));
                }
            }
        },
    );
    let (type_list, type_filter) = lazy_list(
        type_choices,
        {
            let (type_records, origin, search) =
                (type_records.clone(), origin.clone(), search.clone());
            move |i| {
                let Some((o, t)) = type_records.get(i) else {
                    return false;
                };
                Filter::of(origin.selected()).accepts(*o)
                    && t.name
                        .to_lowercase()
                        .contains(&search.text().to_lowercase())
            }
        },
        move |i| {
            if let Some((_, t)) = type_records.get(i) {
                close();
                chosen(Picked::Type(t.clone()));
            }
        },
    );
    type_list.set_visible(false);
    let refilter = {
        let (monster_filter, type_filter) = (monster_filter.clone(), type_filter.clone());
        move || {
            monster_filter.changed(gtk::FilterChange::Different);
            type_filter.changed(gtk::FilterChange::Different);
        }
    };
    origin.connect_selected_notify({
        let refilter = refilter.clone();
        move |_| refilter()
    });
    type_row.connect_selected_notify({
        let refilter = refilter.clone();
        move |_| refilter()
    });
    search.connect_search_changed(move |_| refilter());
    instead.connect_active_notify({
        let (monster_list, type_list, type_row) =
            (monster_list.clone(), type_list.clone(), type_row.clone());
        move |row| {
            monster_list.set_visible(!row.is_active());
            type_row.set_visible(!row.is_active());
            type_list.set_visible(row.is_active());
        }
    });
    let dialog = picker_dialog(
        title,
        help,
        &origin,
        &search,
        Some(&options),
        &[&monster_list, &type_list],
    );
    dialog_cell.set(Some(&dialog));
    dialog.present(Some(parent));
    search.grab_focus();
}

/// IDD 314: a part of one category, from either database.
fn pick_item(
    parent: &impl IsA<gtk::Widget>,
    data: &Rc<Data>,
    chosen: impl Fn(cr_formats::parts::Part) + 'static,
) {
    let categories = Rc::new(parts::editable_types(data.flavor));
    let category =
        gtk::DropDown::from_strings(&categories.iter().map(|(_, n)| tr(n)).collect::<Vec<_>>());
    category.set_tooltip_text(Some(tr("Part Type")));
    let search = gtk::SearchEntry::builder()
        .placeholder_text(tr("Search"))
        .hexpand(true)
        .build();
    let entries: Rc<Vec<(usize, cr_formats::parts::Part)>> = Rc::new(
        data.parts
            .list(Filter::All)
            .iter()
            .map(|e| (e.record.part_type, e.record.part.clone()))
            .collect(),
    );
    let choices = Rc::new(
        data.parts
            .list(Filter::All)
            .iter()
            .map(|e| Choice {
                title: e.record.part.head.name.clone(),
                subtitle: String::new(),
                custom: e.origin == Origin::Custom,
            })
            .collect::<Vec<_>>(),
    );
    let dialog_cell: Rc<glib::WeakRef<adw::Dialog>> = Rc::default();
    let (list, filter) = lazy_list(
        choices,
        {
            let (entries, category, search, categories) = (
                entries.clone(),
                category.clone(),
                search.clone(),
                categories.clone(),
            );
            move |i| {
                let Some((t, part)) = entries.get(i) else {
                    return false;
                };
                categories.get(category.selected() as usize).map(|(c, _)| c) == Some(t)
                    && part
                        .head
                        .name
                        .to_lowercase()
                        .contains(&search.text().to_lowercase())
            }
        },
        {
            let dialog_cell = dialog_cell.clone();
            move |i| {
                if let Some((_, part)) = entries.get(i) {
                    if let Some(d) = dialog_cell.upgrade() {
                        d.close();
                    }
                    chosen(part.clone());
                }
            }
        },
    );
    category.connect_selected_notify({
        let filter = filter.clone();
        move |_| filter.changed(gtk::FilterChange::Different)
    });
    search.connect_search_changed(move |_| filter.changed(gtk::FilterChange::Different));
    let dialog = picker_dialog(
        tr("Add this Item to the Encounter Template"),
        topic::TEMPLATE_ADD_ITEM,
        &category,
        &search,
        None,
        &[&list],
    );
    dialog_cell.set(Some(&dialog));
    dialog.present(Some(parent));
    search.grab_focus();
}

// ---- the editor ----

/// The template being edited and the lists its pages show.
struct Editor<H: Host + 'static> {
    host: Shared<H>,
    data: Rc<Data>,
    record: Rc<RefCell<EncounterTemplate>>,
    dialog: adw::Dialog,
    monsters: gtk::ListBox,
    npcs: gtk::ListBox,
    items: gtk::ListBox,
    traps: gtk::ListBox,
}

fn editor<H: Host + 'static>(
    host: &Shared<H>,
    editing: Option<usize>,
    title: String,
    record: EncounterTemplate,
) {
    let Some(data) = errors(&**host, host.with_install(Data::load)) else {
        return;
    };
    let data = Rc::new(data);
    let record = Rc::new(RefCell::new(record));
    let mut pages: Vec<(&'static str, u32, gtk::Widget)> = Vec::new();

    // Template: name, type and notes.
    let page = adw::PreferencesPage::new();
    let main = adw::PreferencesGroup::new();
    let name = adw::EntryRow::builder()
        .title(tr("Encounter Template Name"))
        .text(&record.borrow().name)
        .build();
    name.connect_changed({
        let record = record.clone();
        move |row| {
            row.text()
                .as_str()
                .clone_into(&mut record.borrow_mut().name)
        }
    });
    main.add(&name);
    let mut types = data.type_names();
    let current = record.borrow().monster_type.clone();
    if !types.contains(&current) {
        types.insert(0, current.clone());
    }
    let type_row = adw::ComboRow::builder()
        .title(tr("Monster Type"))
        .model(&gtk::StringList::new(
            &types.iter().map(String::as_str).collect::<Vec<_>>(),
        ))
        .enable_search(true)
        .expression(gtk::PropertyExpression::new(
            gtk::StringObject::static_type(),
            None::<gtk::Expression>,
            "string",
        ))
        .selected(types.iter().position(|t| *t == current).unwrap_or(0) as u32)
        .build();
    type_row.connect_selected_notify({
        let record = record.clone();
        move |row| {
            if let Some(t) = types.get(row.selected() as usize) {
                t.clone_into(&mut record.borrow_mut().monster_type);
            }
        }
    });
    main.add(&type_row);
    page.add(&main);
    let notes = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(12)
        .bottom_margin(12)
        .left_margin(12)
        .right_margin(12)
        .build();
    notes.buffer().set_text(&record.borrow().notes);
    notes.buffer().connect_changed({
        let record = record.clone();
        move |b| {
            let (start, end) = b.bounds();
            record.borrow_mut().notes = b.text(&start, &end, false).into();
        }
    });
    let notes_group = adw::PreferencesGroup::builder()
        .title(tr("Additional Notes for this Encounter"))
        .build();
    notes_group.add(
        &gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_height(220)
            .child(&notes)
            .css_classes(["card"])
            .build(),
    );
    page.add(&notes_group);
    pages.push((tr("Template"), topic::ENCOUNTER_TEMPLATE, page.upcast()));

    // The lists.
    let add_monster = flat_button("list-add-symbolic", tr("Add Monster"));
    let add_npc = flat_button("list-add-symbolic", tr("Add NPC"));
    let add_item = flat_button("list-add-symbolic", tr("Add Item"));
    let add_trap = flat_button("list-add-symbolic", tr("Add Trap"));
    let lists = [
        (
            tr("Monsters"),
            topic::TEMPLATE_MONSTERS,
            tr("Monsters associated with this Encounter Template"),
            &add_monster,
        ),
        (
            tr("NPC's"),
            topic::TEMPLATE_NPCS,
            tr("NPCs associated with this Encounter Template"),
            &add_npc,
        ),
        (
            tr("Items"),
            topic::TEMPLATE_ITEMS,
            tr("Items associated with this Encounter Template"),
            &add_item,
        ),
        (tr("Traps"), topic::TRAPS, tr("Traps"), &add_trap),
    ]
    .map(|(page_name, help, title, add)| {
        let (group, list) = list_group(title, "", add);
        let page = adw::PreferencesPage::new();
        page.add(&group);
        pages.push((page_name, help, page.upcast()));
        (group, list)
    });
    let [(_, monsters), (_, npcs), (_, items), (_, traps)] = lists;

    pages.push((
        tr("Treasure Types"),
        topic::TREASURE_TYPES,
        treasure_page(&record).upcast(),
    ));
    pages.push((
        tr("2 - 20 Encounter Table"),
        topic::TEMPLATE_TABLE,
        table_page(host, &data, &record).upcast(),
    ));

    let cancel = gtk::Button::builder()
        .label(tr("_Cancel"))
        .use_underline(true)
        .build();
    let save = gtk::Button::builder()
        .label(tr("_Save"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let header = adw::HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .build();
    header.pack_start(&cancel);
    header.pack_end(&save);
    let view = adw::ToolbarView::new();
    let stack = adw::ViewStack::new();
    let sidebar = gtk::ListBox::builder()
        .css_classes(["navigation-sidebar"])
        .build();
    let helps: Rc<Vec<(String, u32)>> = Rc::new(
        pages
            .iter()
            .map(|(name, help, _)| ((*name).to_owned(), *help))
            .collect(),
    );
    for (page_name, _, widget) in &pages {
        stack.add_titled(widget, Some(page_name), page_name);
        let row = gtk::ListBoxRow::builder()
            .child(
                &gtk::Label::builder()
                    .label(*page_name)
                    .xalign(0.0)
                    .margin_top(6)
                    .margin_bottom(6)
                    .margin_start(6)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .build(),
            )
            .build();
        row.set_widget_name(page_name);
        sidebar.append(&row);
    }
    crate::help::add_button(&header, &view, {
        let stack = stack.clone();
        move || {
            let shown = stack.visible_child_name().unwrap_or_default();
            helps
                .iter()
                .find(|(name, _)| *name == shown.as_str())
                .map_or(topic::ENCOUNTER_TEMPLATE, |(_, help)| *help)
        }
    });
    view.add_top_bar(&header);
    let split = adw::OverlaySplitView::builder()
        .sidebar(
            &gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .child(&sidebar)
                .build(),
        )
        .content(&stack)
        .min_sidebar_width(150.0)
        .max_sidebar_width(220.0)
        .build();
    sidebar.connect_row_selected({
        let stack = stack.clone();
        move |_, row| {
            if let Some(row) = row {
                stack.set_visible_child_name(&row.widget_name());
            }
        }
    });
    sidebar.select_row(sidebar.row_at_index(0).as_ref());
    let toggle = gtk::ToggleButton::builder()
        .icon_name("sidebar-show-symbolic")
        .tooltip_text(tr("Pages"))
        .active(true)
        .build();
    toggle
        .bind_property("active", &split, "show-sidebar")
        .bidirectional()
        .sync_create()
        .build();
    split
        .bind_property("collapsed", &toggle, "visible")
        .sync_create()
        .build();
    header.pack_start(&toggle);
    view.set_content(Some(&split));
    let dialog = super::dialog_for(&title, &view, true);
    let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        560.0,
        adw::LengthUnit::Sp,
    ));
    breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
    dialog.add_breakpoint(breakpoint);

    let editor = Rc::new(Editor {
        host: host.clone(),
        data,
        record: record.clone(),
        dialog: dialog.clone(),
        monsters,
        npcs,
        items,
        traps,
    });
    add_monster.connect_clicked({
        let editor = editor.clone();
        move |_| editor.add_monster()
    });
    add_npc.connect_clicked({
        let editor = editor.clone();
        move |_| editor.add_npc()
    });
    add_item.connect_clicked({
        let editor = editor.clone();
        move |_| editor.add_item()
    });
    add_trap.connect_clicked({
        let editor = editor.clone();
        move |_| editor.add_trap()
    });
    editor.redraw();
    finish::<Templates, H>(
        host,
        dialog,
        TEMPLATES,
        editing,
        record,
        (cancel, save),
        Rc::default(),
    );
}

/// IDD 276 in template mode: the lair treasure, 0 to 100 of each letter.
fn treasure_page(record: &Rc<RefCell<EncounterTemplate>>) -> adw::PreferencesPage {
    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(6)
        .min_children_per_line(2)
        .column_spacing(12)
        .row_spacing(6)
        .homogeneous(true)
        .build();
    for (i, letter) in LETTERS.iter().enumerate() {
        let spin = spin_button(record.borrow().lair_treasure[i], 0, 100);
        spin.update_property(&[gtk::accessible::Property::Label(&letter.to_string())]);
        spin.connect_value_notify({
            let record = record.clone();
            move |s| record.borrow_mut().lair_treasure[i] = s.value() as i32
        });
        let cell = gtk::Box::builder().spacing(6).build();
        cell.append(
            &gtk::Label::builder()
                .label(letter.to_string())
                .width_chars(2)
                .css_classes(["heading"])
                .build(),
        );
        cell.append(&spin);
        flow.insert(&cell, -1);
    }
    let group = adw::PreferencesGroup::builder()
        .title(tr("Treasure Types"))
        .description(tr("How many of each treasure type the lair holds"))
        .build();
    group.add(&flow);
    let page = adw::PreferencesPage::new();
    page.add(&group);
    page
}

/// IDD 325: where and how often the template comes up in the 2-20 tables,
/// its hit dice and its climate and terrain (IDD 273).
fn table_page<H: Host + 'static>(
    host: &Shared<H>,
    data: &Rc<Data>,
    record: &Rc<RefCell<EncounterTemplate>>,
) -> adw::PreferencesPage {
    let r = record.borrow().clone();
    let placement = adw::ComboRow::builder()
        .title(tr("Encountered"))
        .model(&gtk::StringList::new(&tpl::PLACEMENT_NAMES.map(tr)))
        .selected(tpl::placement_choice(&r.placement) as u32)
        .build();
    placement.connect_selected_notify({
        let record = record.clone();
        move |row| record.borrow_mut().placement = tpl::placement_of(row.selected() as usize)
    });
    let frequency = adw::ComboRow::builder()
        .title(tr("Frequency"))
        .model(&gtk::StringList::new(&tpl::FREQUENCY_NAMES.map(tr)))
        .selected(tpl::frequency_choice(r.frequency) as u32)
        .build();
    frequency.connect_selected_notify({
        let record = record.clone();
        move |row| {
            record.borrow_mut().frequency = tpl::FREQUENCIES[(row.selected() as usize).min(4)]
        }
    });
    let hit_dice = spin_row(tr("Number Of Hit Dice"), r.hit_dice, 0, 300_000_000);
    hit_dice.connect_value_notify({
        let record = record.clone();
        move |row| record.borrow_mut().hit_dice = row.value() as i32
    });
    let category = adw::EntryRow::builder()
        .title(tr("Category"))
        .text(&r.climate_terrain)
        .build();
    let choose = flat_button("view-list-symbolic", tr("Choose"));
    choose.connect_clicked({
        let (category, host, choices) = (category.clone(), host.clone(), data.terrains.clone());
        move |_| {
            let category = category.clone();
            crate::lists::pick(
                &host.parent(),
                &tr!("Choose {field}", field = tr("Category")),
                choices.clone(),
                move |c| category.set_text(&c),
            );
        }
    });
    category.add_suffix(&choose);
    let places = adw::ExpanderRow::builder()
        .title(tr("Climate and Terrain"))
        .subtitle(tr("Chosen one by one in the ~Custom category"))
        .build();
    let mut checks = Vec::new();
    for (i, place) in CLIMATES.iter().chain(TERRAINS.iter()).enumerate() {
        let check = gtk::CheckButton::builder()
            .active(r.climates[i] != 0)
            .valign(gtk::Align::Center)
            .build();
        check.connect_active_notify({
            let record = record.clone();
            move |c| record.borrow_mut().climates[i] = i32::from(c.is_active())
        });
        let row = adw::ActionRow::builder()
            .title(tr(place))
            .activatable_widget(&check)
            .build();
        row.add_prefix(&check);
        places.add_row(&row);
        checks.push(row);
    }
    let custom = move |text: &str| {
        for row in &checks {
            row.set_sensitive(text == CUSTOM_TERRAIN);
        }
    };
    custom(&r.climate_terrain);
    category.connect_changed({
        let record = record.clone();
        move |row| {
            row.text()
                .as_str()
                .clone_into(&mut record.borrow_mut().climate_terrain);
            custom(&row.text());
        }
    });
    let where_group = adw::PreferencesGroup::builder()
        .title(tr(
            "Select the category under which this Encounter Template will appear",
        ))
        .build();
    where_group.add(&placement);
    where_group.add(&frequency);
    where_group.add(&hit_dice);
    let climate_group = adw::PreferencesGroup::builder()
        .title(tr("Climate and Terrain"))
        .build();
    climate_group.add(&category);
    climate_group.add(&places);
    let page = adw::PreferencesPage::new();
    page.add(&where_group);
    page.add(&climate_group);
    page
}

fn monster_names(objects: &[Object]) -> Vec<String> {
    objects
        .iter()
        .filter_map(|o| match o {
            Object::Monster(m) => Some(m.name.clone()),
            _ => None,
        })
        .collect()
}

fn npc_list(objects: &[Object]) -> Vec<EncounterNpc> {
    objects
        .iter()
        .filter_map(|o| match o {
            Object::EncounterNpc(n) => Some((**n).clone()),
            _ => None,
        })
        .collect()
}

impl<H: Host + 'static> Editor<H> {
    fn redraw(self: &Rc<Self>) {
        self.redraw_monsters();
        self.redraw_npcs();
        self.redraw_items();
        self.redraw_traps();
    }

    /// IDD 303's list, sorted by its text.
    fn redraw_monsters(self: &Rc<Self>) {
        clear(&self.monsters);
        let mut lines: Vec<(String, usize)> = self
            .record
            .borrow()
            .monsters
            .iter()
            .enumerate()
            .filter_map(|(i, o)| match o {
                Object::TemplateMonsterProperties(p) => Some((tpl::monster_line_text(p), i)),
                _ => None,
            })
            .collect();
        lines.sort_by_cached_key(|(text, _)| text.to_lowercase());
        for (text, i) in lines {
            let remove = {
                let editor = self.clone();
                move || {
                    editor.record.borrow_mut().monsters.remove(i);
                    editor.redraw_monsters();
                }
            };
            let open = {
                let editor = self.clone();
                move || editor.monster_properties(i)
            };
            self.monsters.append(
                &line_row(
                    &text,
                    "",
                    tr("Delete Monster"),
                    remove,
                    Some(Box::new(open)),
                )
                .0,
            );
        }
    }

    fn redraw_npcs(self: &Rc<Self>) {
        clear(&self.npcs);
        let lines: Vec<String> = self
            .record
            .borrow()
            .npcs
            .iter()
            .map(|o| match o {
                Object::TemplateNpcProperties(p) => tpl::npc_line_text(p),
                _ => String::new(),
            })
            .collect();
        for (i, text) in lines.into_iter().enumerate() {
            let remove = {
                let editor = self.clone();
                move || {
                    editor.record.borrow_mut().npcs.remove(i);
                    editor.redraw_npcs();
                }
            };
            let open = {
                let editor = self.clone();
                move || editor.npc_properties(i)
            };
            self.npcs
                .append(&line_row(&text, "", tr("Delete NPC"), remove, Some(Box::new(open))).0);
        }
    }

    /// IDD 315's list: the items' names.
    fn redraw_items(self: &Rc<Self>) {
        clear(&self.items);
        let lines: Vec<(String, String)> = self
            .record
            .borrow()
            .items
            .iter()
            .map(|o| match o {
                Object::TemplateItemProperties(p) => (
                    p.part.head.name.clone(),
                    format!(
                        "{}d{} * {} + {} · {}%",
                        p.dice[0], p.dice[1], p.dice[2], p.dice[3], p.percent
                    ),
                ),
                _ => Default::default(),
            })
            .collect();
        for (i, (name, subtitle)) in lines.into_iter().enumerate() {
            let remove = {
                let editor = self.clone();
                move || {
                    editor.record.borrow_mut().items.remove(i);
                    editor.redraw_items();
                }
            };
            let open = {
                let editor = self.clone();
                move || editor.item_properties(i)
            };
            self.items.append(
                &line_row(
                    &name,
                    &subtitle,
                    tr("Delete Item"),
                    remove,
                    Some(Box::new(open)),
                )
                .0,
            );
        }
    }

    fn redraw_traps(self: &Rc<Self>) {
        clear(&self.traps);
        let traps = self.trap_list();
        for trap in traps {
            let remove = {
                let (editor, name) = (self.clone(), trap.name.clone());
                move || {
                    let mut traps = editor.trap_list();
                    match cr_store::encounter::remove_trap(&mut traps, &name) {
                        Ok(()) => editor.set_traps(traps),
                        Err(e) => editor.host.toast(&e.to_string()),
                    }
                }
            };
            self.traps
                .append(&line_row(&trap.name, &trap.damage, tr("Remove Trap"), remove, None).0);
        }
    }

    fn trap_list(&self) -> Vec<Trap> {
        self.record
            .borrow()
            .traps
            .iter()
            .filter_map(|o| match o {
                Object::Trap(t) => Some((**t).clone()),
                _ => None,
            })
            .collect()
    }

    fn set_traps(self: &Rc<Self>, traps: Vec<Trap>) {
        self.record.borrow_mut().traps = traps
            .into_iter()
            .map(|t| Object::Trap(Box::new(t)))
            .collect();
        self.redraw_traps();
    }

    /// Add… of IDD 303 (IDD 302): one monster, 1d1, always.
    fn add_monster(self: &Rc<Self>) {
        let editor = self.clone();
        let start = self.record.borrow().monster_type.clone();
        pick_monster(
            &self.dialog,
            &self.data,
            tr("Monsters for this Encounter Template"),
            topic::TEMPLATE_ADD_MONSTER,
            &start,
            false,
            move |picked| {
                if let Picked::Monster(m) = picked {
                    let line = tpl::new_monster_line(&m);
                    editor
                        .record
                        .borrow_mut()
                        .monsters
                        .push(Object::TemplateMonsterProperties(Box::new(line)));
                    editor.redraw_monsters();
                }
            },
        );
    }

    /// Add… of IDD 383: NPC Creation's "Add to Template".
    fn add_npc(self: &Rc<Self>) {
        let editor = self.clone();
        self.host.template_npc(
            self.dialog.upcast_ref(),
            Box::new(move |npc| {
                let line = tpl::new_npc_line(npc_of(npc));
                editor
                    .record
                    .borrow_mut()
                    .npcs
                    .push(Object::TemplateNpcProperties(Box::new(line)));
                editor.redraw_npcs();
            }),
        );
    }

    /// Add… of IDD 315 (IDD 314): one of the part, always.
    fn add_item(self: &Rc<Self>) {
        let editor = self.clone();
        pick_item(&self.dialog, &self.data, move |part| {
            let line = tpl::new_item_line(part);
            editor
                .record
                .borrow_mut()
                .items
                .push(Object::TemplateItemProperties(Box::new(line)));
            editor.redraw_items();
        });
    }

    /// Traps… (the encounters' Traps dialog): one copy of each trap.
    fn add_trap(self: &Rc<Self>) {
        let names: Vec<String> = self
            .data
            .traps
            .list(Filter::All)
            .iter()
            .map(|e| e.record.name.clone())
            .collect();
        let editor = self.clone();
        crate::lists::pick(&self.dialog, tr("Traps"), names, move |name| {
            let mut traps = editor.trap_list();
            match cr_store::encounter::add_trap(&mut traps, &editor.data.traps, &name) {
                Ok(()) => editor.set_traps(traps),
                Err(e) => super::message(&editor.dialog, tr("Unable to Add Trap"), &e.to_string()),
            }
        });
    }

    fn monster_line(&self, i: usize) -> Option<TemplateMonsterProperties> {
        match self.record.borrow().monsters.get(i) {
            Some(Object::TemplateMonsterProperties(p)) => Some((**p).clone()),
            _ => None,
        }
    }

    fn npc_line(&self, i: usize) -> Option<TemplateNpcProperties> {
        match self.record.borrow().npcs.get(i) {
            Some(Object::TemplateNpcProperties(p)) => Some((**p).clone()),
            _ => None,
        }
    }

    fn item_line(&self, i: usize) -> Option<TemplateItemProperties> {
        match self.record.borrow().items.get(i) {
            Some(Object::TemplateItemProperties(p)) => Some((**p).clone()),
            _ => None,
        }
    }

    /// IDD 304 on a copy of monster line `i`; OK settles and stores it.
    fn monster_properties(self: &Rc<Self>, i: usize) {
        let Some(mut line) = self.monster_line(i) else {
            return;
        };
        tpl::clamp_dice(&mut line.dice);
        let line = Rc::new(RefCell::new(line));
        let page = adw::PreferencesPage::new();
        let first = Lines::new(&line.borrow().dice, line.borrow().percent, 1000, 10000);
        page.add(&first.group);
        let dependent = Dependent::new(
            self,
            line.borrow().flags[0] != 0,
            line.borrow().per,
            line.borrow().beyond,
            tpl::reference_text(&line.borrow().reference_name, &line.borrow().monster_type),
            {
                let line = line.clone();
                move |picked| {
                    let mut p = line.borrow_mut();
                    match picked {
                        Picked::Monster(m) => {
                            p.reference_name = m.name.clone();
                            p.monster_type = tpl::empty_type();
                        }
                        Picked::Type(t) => {
                            p.monster_type = t;
                            p.reference_name.clear();
                        }
                    }
                    tpl::reference_text(&p.reference_name, &p.monster_type)
                }
            },
        );
        page.add(&dependent.group);

        // The random list.
        let add = flat_button("list-add-symbolic", tr("Add Monster"));
        let (group, list) = list_group(
            tr("Randomly selected from the following list"),
            tr("Each appearance picks one of these monsters"),
            &add,
        );
        let draw: Redraw = Rc::default();
        let render: Rc<dyn Fn()> = Rc::new({
            let (line, list, draw) = (line.clone(), list.clone(), draw.clone());
            move || {
                clear(&list);
                let names = monster_names(&line.borrow().monsters);
                let main = line.borrow().monster_name.clone();
                for (k, name) in names.iter().enumerate() {
                    let remove = {
                        let (line, draw) = (line.clone(), draw.clone());
                        move || {
                            line.borrow_mut().monsters.remove(k);
                            if let Some(draw) = draw.borrow().clone() {
                                draw();
                            }
                        }
                    };
                    let (row, button) = line_row(name, "", tr("Remove"), remove, None);
                    // The line's own monster stays while it is the only
                    // entry of that name (0x72dde2).
                    button.set_sensitive(
                        *name != main || names.iter().filter(|n| **n == main).count() > 1,
                    );
                    list.append(&row);
                }
            }
        });
        *draw.borrow_mut() = Some(render.clone());
        render();
        add.connect_clicked({
            let (editor, line, render) = (self.clone(), line.clone(), render.clone());
            move |_| {
                let (line, render) = (line.clone(), render.clone());
                pick_monster(
                    &editor.dialog,
                    &editor.data,
                    tr("More Monsters for the Encounter Template"),
                    topic::TEMPLATE_MORE_MONSTERS,
                    "~All",
                    false,
                    move |picked| {
                        if let Picked::Monster(m) = picked {
                            line.borrow_mut().monsters.push(Object::Monster(m));
                            render();
                        }
                    },
                );
            }
        });
        page.add(&group);

        let second_name = adw::ActionRow::builder()
            .title(tr("Monster"))
            .subtitle(escaped(&line.borrow().second_name).as_str())
            .build();
        let choose = label_button(tr("_Monster…"));
        choose.connect_clicked({
            let (editor, line, second_name) = (self.clone(), line.clone(), second_name.clone());
            move |_| {
                let (line, second_name) = (line.clone(), second_name.clone());
                pick_monster(
                    &editor.dialog,
                    &editor.data,
                    tr("More Monsters for the Encounter Template"),
                    topic::TEMPLATE_MORE_MONSTERS,
                    "~All",
                    false,
                    move |picked| {
                        if let Picked::Monster(m) = picked {
                            second_name.set_subtitle(escaped(&m.name).as_str());
                            line.borrow_mut().second_name = m.name.clone();
                        }
                    },
                );
            }
        });
        second_name.add_suffix(&choose);
        let second = Second::new(
            Combo::of_flags(&line.borrow().flags),
            &line.borrow().dice[4..],
            second_name.upcast(),
        );
        page.add(&second.group);

        let (dialog, ok) = sub_dialog(
            tr("Monster Properties for Encounter Templates"),
            &page,
            tr("_OK"),
            topic::TEMPLATE_MONSTER,
        );
        ok.connect_clicked({
            let (editor, dialog) = (self.clone(), dialog.clone());
            move |_| {
                let mut p = line.borrow().clone();
                p.dice[..4].copy_from_slice(&first.dice.values());
                p.dice[4..].copy_from_slice(&second.dice.values());
                p.percent = first.percent.value() as i32;
                p.per = dependent.per.value() as i32;
                p.beyond = dependent.beyond.value() as i32;
                p.flags = second.combo().flags(dependent.on.is_active());
                tpl::settle_monster_line(&mut p);
                if let Some(slot) = editor.record.borrow_mut().monsters.get_mut(i) {
                    *slot = Object::TemplateMonsterProperties(Box::new(p));
                }
                dialog.close();
                editor.redraw_monsters();
            }
        });
        dialog.present(Some(&self.dialog));
    }

    /// IDD 384 on a copy of NPC line `i`.
    fn npc_properties(self: &Rc<Self>, i: usize) {
        let Some(mut line) = self.npc_line(i) else {
            return;
        };
        tpl::clamp_dice(&mut line.dice);
        let line = Rc::new(RefCell::new(line));
        let page = adw::PreferencesPage::new();
        let first = Lines::new(&line.borrow().dice, line.borrow().percent, 1000, 10000);
        page.add(&first.group);
        let dependent = Dependent::new(
            self,
            line.borrow().flags[0] != 0,
            line.borrow().per,
            line.borrow().beyond,
            tpl::reference_text(&line.borrow().reference_name, &line.borrow().monster_type),
            {
                let line = line.clone();
                move |picked| {
                    let mut p = line.borrow_mut();
                    match picked {
                        Picked::Monster(m) => {
                            p.reference_name = m.name.clone();
                            p.monster_type = tpl::empty_type();
                        }
                        Picked::Type(t) => {
                            p.monster_type = t;
                            p.reference_name.clear();
                        }
                    }
                    tpl::reference_text(&p.reference_name, &p.monster_type)
                }
            },
        );
        page.add(&dependent.group);

        let add = flat_button("list-add-symbolic", tr("Add NPC"));
        let (group, list) = list_group(
            tr("Randomly selected from the following list"),
            tr("Each appearance picks one of these NPCs"),
            &add,
        );
        let draw: Redraw = Rc::default();
        let render: Rc<dyn Fn()> = Rc::new({
            let (line, list, draw) = (line.clone(), list.clone(), draw.clone());
            move || {
                clear(&list);
                let npcs = npc_list(&line.borrow().npcs);
                let main = line.borrow().npc.clone();
                for (k, npc) in npcs.iter().enumerate() {
                    let remove = {
                        let (line, draw) = (line.clone(), draw.clone());
                        move || {
                            line.borrow_mut().npcs.remove(k);
                            if let Some(draw) = draw.borrow().clone() {
                                draw();
                            }
                        }
                    };
                    let (row, button) = line_row(
                        &tpl::npc_description(npc),
                        &npc.name,
                        tr("Remove"),
                        remove,
                        None,
                    );
                    // 0x72f56f: the line's own NPC stays while it is there
                    // only once.
                    button.set_sensitive(
                        *npc != main || npcs.iter().filter(|n| **n == main).count() > 1,
                    );
                    list.append(&row);
                }
            }
        });
        *draw.borrow_mut() = Some(render.clone());
        render();
        add.connect_clicked({
            let (editor, line, render) = (self.clone(), line.clone(), render.clone());
            move |_| {
                let (line, render) = (line.clone(), render.clone());
                editor.host.template_npc(
                    editor.dialog.upcast_ref(),
                    Box::new(move |npc| {
                        line.borrow_mut()
                            .npcs
                            .push(Object::EncounterNpc(Box::new(npc_of(npc))));
                        render();
                    }),
                );
            }
        });
        page.add(&group);

        let second_npc = adw::ActionRow::builder()
            .title(tr("NPC"))
            .subtitle(escaped(&tpl::npc_description(&line.borrow().second_npc)).as_str())
            .build();
        let choose = label_button(tr("_NPC…"));
        choose.connect_clicked({
            let (editor, line, second_npc) = (self.clone(), line.clone(), second_npc.clone());
            move |_| {
                let (line, second_npc) = (line.clone(), second_npc.clone());
                editor.host.template_npc(
                    editor.dialog.upcast_ref(),
                    Box::new(move |npc| {
                        let npc = npc_of(npc);
                        second_npc.set_subtitle(escaped(&tpl::npc_description(&npc)).as_str());
                        line.borrow_mut().second_npc = npc;
                    }),
                );
            }
        });
        second_npc.add_suffix(&choose);
        let second = Second::new(
            Combo::of_flags(&line.borrow().flags),
            &line.borrow().dice[4..],
            second_npc.upcast(),
        );
        page.add(&second.group);

        let (dialog, ok) = sub_dialog(
            tr("Nonplayer Character Properties for Encounter Templates"),
            &page,
            tr("_OK"),
            topic::TEMPLATE_NPC,
        );
        ok.connect_clicked({
            let (editor, dialog) = (self.clone(), dialog.clone());
            move |_| {
                let mut p = line.borrow().clone();
                p.dice[..4].copy_from_slice(&first.dice.values());
                p.dice[4..].copy_from_slice(&second.dice.values());
                p.percent = first.percent.value() as i32;
                p.per = dependent.per.value() as i32;
                p.beyond = dependent.beyond.value() as i32;
                p.flags = second.combo().flags(dependent.on.is_active());
                tpl::settle_npc_line(&mut p);
                if let Some(slot) = editor.record.borrow_mut().npcs.get_mut(i) {
                    *slot = Object::TemplateNpcProperties(Box::new(p));
                }
                dialog.close();
                editor.redraw_npcs();
            }
        });
        dialog.present(Some(&self.dialog));
    }

    /// IDD 309 (with IDD 311's three lists) on a copy of item line `i`.
    fn item_properties(self: &Rc<Self>, i: usize) {
        let Some(mut line) = self.item_line(i) else {
            return;
        };
        tpl::clamp_item_dice(&mut line.dice);
        let line = Rc::new(RefCell::new(line));
        let page = adw::PreferencesPage::new();
        let first = Lines::new(
            &line.borrow().dice[..4],
            line.borrow().percent,
            1_000_000,
            1_000_000,
        );
        first.dice.title.set_label(tr("Number of Items"));
        first
            .group
            .set_title(&escaped(&line.borrow().part.head.name));
        page.add(&first.group);
        let reference = {
            let p = line.borrow();
            if p.monster.name.is_empty() {
                p.monster_type.name.clone()
            } else {
                p.monster.name.clone()
            }
        };
        let dependent = Dependent::new(
            self,
            line.borrow().dependency[0] != 0,
            line.borrow().dependency[1],
            line.borrow().dependency[2],
            reference,
            {
                let line = line.clone();
                move |picked| {
                    let mut p = line.borrow_mut();
                    match picked {
                        Picked::Monster(m) => {
                            p.monster = m;
                            p.monster_type = tpl::empty_type();
                        }
                        Picked::Type(t) => {
                            p.monster_type = t;
                            *p.monster = tpl::empty_monster();
                        }
                    }
                    if p.monster.name.is_empty() {
                        p.monster_type.name.clone()
                    } else {
                        p.monster.name.clone()
                    }
                }
            },
        );
        page.add(&dependent.group);

        let coins = DiceRow::new(
            tr("Containing"),
            &line.borrow().dice[5..],
            [(0, 1_000_000), (0, 1_000_000), (0, 1_000_000), (0, 100_000)],
        );
        let coin = adw::ComboRow::builder()
            .title(tr("Coins"))
            .model(&gtk::StringList::new(&["pp", "gp", "sp", "cp"]))
            .selected(line.borrow().dice[4].clamp(0, 3) as u32)
            .build();
        let coin_group = adw::PreferencesGroup::builder()
            .title(tr("Coins"))
            .description(tr("Coins each item holds"))
            .build();
        coin_group.add(&coins.row);
        coin_group.add(&coin);
        page.add(&coin_group);

        for per_item in [false, true] {
            let add = flat_button("list-add-symbolic", tr("Add Monster"));
            let (group, list) = list_group(
                if per_item {
                    tr("For each Item...")
                } else {
                    tr("These Monsters will appear if the previous Item(s) appear...")
                },
                "",
                &add,
            );
            let draw: Redraw = Rc::default();
            let render: Rc<dyn Fn()> = Rc::new({
                let (line, list, draw) = (line.clone(), list.clone(), draw.clone());
                move || {
                    clear(&list);
                    let texts: Vec<String> = {
                        let p = line.borrow();
                        let escorts = if per_item { &p.per_item } else { &p.escorts };
                        escorts
                            .iter()
                            .filter_map(|o| match o {
                                Object::TemplateNumAppear(n) => Some(tpl::escort_text(n)),
                                _ => None,
                            })
                            .collect()
                    };
                    for (k, text) in texts.iter().enumerate() {
                        let remove = {
                            let (line, draw) = (line.clone(), draw.clone());
                            move || {
                                {
                                    let mut p = line.borrow_mut();
                                    let escorts = if per_item {
                                        &mut p.per_item
                                    } else {
                                        &mut p.escorts
                                    };
                                    escorts.remove(k);
                                }
                                if let Some(draw) = draw.borrow().clone() {
                                    draw();
                                }
                            }
                        };
                        list.append(&line_row(text, "", tr("Delete"), remove, None).0);
                    }
                }
            });
            *draw.borrow_mut() = Some(render.clone());
            render();
            add.connect_clicked({
                let (editor, line, render) = (self.clone(), line.clone(), render.clone());
                move |_| {
                    let (line, render) = (line.clone(), render.clone());
                    editor.escort(move |escort| {
                        let mut p = line.borrow_mut();
                        let escorts = if per_item {
                            &mut p.per_item
                        } else {
                            &mut p.escorts
                        };
                        escorts.push(Object::TemplateNumAppear(Box::new(escort)));
                        drop(p);
                        render();
                    });
                }
            });
            page.add(&group);
        }

        // "Double the number of these Monsters".
        let add = flat_button("list-add-symbolic", tr("Add Monster"));
        let (group, list) = list_group(tr("Double the number of these Monsters..."), "", &add);
        let draw: Redraw = Rc::default();
        let render: Rc<dyn Fn()> = Rc::new({
            let (line, list, draw) = (line.clone(), list.clone(), draw.clone());
            move || {
                clear(&list);
                let names = line.borrow().monster_names.clone();
                for name in names {
                    let remove = {
                        let (line, draw, name) = (line.clone(), draw.clone(), name.clone());
                        move || {
                            // The first of that name (0x7333ba).
                            let mut p = line.borrow_mut();
                            if let Some(k) = p.monster_names.iter().position(|n| *n == name) {
                                p.monster_names.remove(k);
                            }
                            drop(p);
                            if let Some(draw) = draw.borrow().clone() {
                                draw();
                            }
                        }
                    };
                    list.append(&line_row(&name, "", tr("Delete"), remove, None).0);
                }
            }
        });
        *draw.borrow_mut() = Some(render.clone());
        render();
        add.connect_clicked({
            let (editor, line, render) = (self.clone(), line.clone(), render.clone());
            move |_| {
                let (line, render) = (line.clone(), render.clone());
                pick_monster(
                    &editor.dialog,
                    &editor.data,
                    tr("More Monsters for the Encounter Template"),
                    topic::TEMPLATE_MORE_MONSTERS,
                    "~All",
                    false,
                    move |picked| {
                        if let Picked::Monster(m) = picked {
                            line.borrow_mut().monster_names.push(m.name.clone());
                            render();
                        }
                    },
                );
            }
        });
        page.add(&group);

        let (dialog, ok) = sub_dialog(
            tr("Adding Items to an Encounter Template"),
            &page,
            tr("_OK"),
            topic::TEMPLATE_ITEM,
        );
        ok.connect_clicked({
            let (editor, dialog) = (self.clone(), dialog.clone());
            move |_| {
                let mut p = line.borrow().clone();
                p.dice[..4].copy_from_slice(&first.dice.values());
                p.dice[4] = coin.selected() as i32;
                p.dice[5..].copy_from_slice(&coins.values());
                p.percent = first.percent.value() as i32;
                p.dependency = [
                    i32::from(dependent.on.is_active()),
                    dependent.per.value() as i32,
                    dependent.beyond.value() as i32,
                ];
                tpl::settle_item_line(&mut p);
                if let Some(slot) = editor.record.borrow_mut().items.get_mut(i) {
                    *slot = Object::TemplateItemProperties(Box::new(p));
                }
                dialog.close();
                editor.redraw_items();
            }
        });
        dialog.present(Some(&self.dialog));
    }

    /// IDD 313: a monster and how many appear with an item.
    fn escort(self: &Rc<Self>, added: impl Fn(TemplateNumAppear) + 'static) {
        let dice = DiceRow::new(
            tr("Number Appearing"),
            &[1, 1, 1, 0],
            [
                (1, 1_000_000),
                (1, 1_000_000),
                (1, 1_000_000),
                (0, 1_000_000),
            ],
        );
        let picked: Rc<RefCell<Option<Monster>>> = Rc::default();
        let monster = adw::ActionRow::builder().title(tr("Monster")).build();
        let choose = label_button(tr("_Monster…"));
        monster.add_suffix(&choose);
        let group = adw::PreferencesGroup::new();
        group.add(&dice.row);
        group.add(&monster);
        let page = adw::PreferencesPage::new();
        page.add(&group);
        let (dialog, ok) = sub_dialog(
            tr("Select a Monster and the number appearing"),
            &page,
            tr("_Add"),
            topic::TEMPLATE_ESCORT,
        );
        ok.set_sensitive(false);
        choose.connect_clicked({
            let (editor, picked, monster, ok) =
                (self.clone(), picked.clone(), monster.clone(), ok.clone());
            move |_| {
                let (picked, monster, ok) = (picked.clone(), monster.clone(), ok.clone());
                pick_monster(
                    &editor.dialog,
                    &editor.data,
                    tr("More Monsters for the Encounter Template"),
                    topic::TEMPLATE_MORE_MONSTERS,
                    "~All",
                    false,
                    move |p| {
                        if let Picked::Monster(m) = p {
                            monster.set_subtitle(escaped(&m.name).as_str());
                            *picked.borrow_mut() = Some(*m);
                            ok.set_sensitive(true);
                        }
                    },
                );
            }
        });
        ok.connect_clicked({
            let dialog = dialog.clone();
            move |_| {
                if let Some(m) = picked.borrow().as_ref() {
                    added(tpl::new_escort(dice.values(), m));
                }
                dialog.close();
            }
        });
        dialog.present(Some(&self.dialog));
    }
}

/// The NPC as "Add to Template" stores it.
fn npc_of(npc: &cr_store::npc::Npc) -> EncounterNpc {
    tpl::npc_of(
        &npc.character.data.name,
        &npc.character.data.race,
        &npc.classes,
    )
}

/// The first box of a Properties dialog: how many appear and the chance.
struct Lines {
    group: adw::PreferencesGroup,
    dice: DiceRow,
    percent: adw::SpinRow,
}

impl Lines {
    fn new(dice: &[i32], percent: i32, sides: i32, multiplier: i32) -> Self {
        let dice = DiceRow::new(
            tr("Number Appearing"),
            dice,
            [(1, 1_000_000), (1, sides), (1, multiplier), (0, 1_000_000)],
        );
        let percent = spin_row(tr("Percent Chance"), percent, 1, 100);
        percent.set_subtitle(tr("There is a percent chance that these appear"));
        let group = adw::PreferencesGroup::new();
        group.add(&dice.row);
        group.add(&percent);
        Self {
            group,
            dice,
            percent,
        }
    }
}

/// "Dependent on the following, for every n beyond m": what the line
/// counts, picked as a monster or a monster type (IDD 306, mode 1).
struct Dependent {
    group: adw::PreferencesGroup,
    on: adw::SwitchRow,
    per: adw::SpinRow,
    beyond: adw::SpinRow,
}

impl Dependent {
    fn new<H: Host + 'static>(
        editor: &Rc<Editor<H>>,
        on: bool,
        per: i32,
        beyond: i32,
        reference: String,
        picked: impl Fn(Picked) -> String + 'static,
    ) -> Self {
        let on_row = adw::SwitchRow::builder()
            .title(tr("Dependent on the following"))
            .active(on)
            .build();
        let per = spin_row(tr("For every"), per, 0, 1_000_000);
        per.set_subtitle(tr("Enter 0 for a one time addition"));
        let beyond = spin_row(tr("Beyond"), beyond, 0, 1_000_000);
        let reference_row = adw::ActionRow::builder()
            .title(tr("Monster or Monster Type"))
            .subtitle(escaped(&reference).as_str())
            .build();
        let choose = label_button(tr("_Choose…"));
        reference_row.add_suffix(&choose);
        let picked = Rc::new(picked);
        choose.connect_clicked({
            let (editor, reference_row, picked) =
                (editor.clone(), reference_row.clone(), picked.clone());
            move |_| {
                let (reference_row, picked) = (reference_row.clone(), picked.clone());
                pick_monster(
                    &editor.dialog,
                    &editor.data,
                    tr("More Monsters for the Encounter Template"),
                    topic::TEMPLATE_MORE_MONSTERS,
                    "~All",
                    true,
                    move |p| {
                        let text = picked(p);
                        reference_row.set_subtitle(escaped(&text).as_str());
                    },
                );
            }
        });
        let group = adw::PreferencesGroup::new();
        group.add(&on_row);
        group.add(&per);
        group.add(&beyond);
        group.add(&reference_row);
        let show = {
            let (per, beyond, reference_row) = (per.clone(), beyond.clone(), reference_row.clone());
            move |on: bool| {
                per.set_visible(on);
                beyond.set_visible(on);
                reference_row.set_visible(on);
            }
        };
        show(on);
        on_row.connect_active_notify(move |row| show(row.is_active()));
        Self {
            group,
            on: on_row,
            per,
            beyond,
        }
    }
}

/// The combo ("appear.", "and", "or") and the second box it shows.
struct Second {
    group: adw::PreferencesGroup,
    combo: adw::ComboRow,
    dice: DiceRow,
}

impl Second {
    fn new(combo: Combo, dice: &[i32], who: gtk::Widget) -> Self {
        let combo_row = adw::ComboRow::builder()
            .title(tr("These then"))
            .model(&gtk::StringList::new(&Combo::NAMES.map(tr)))
            .selected(combo.index())
            .build();
        let dice = DiceRow::new(
            tr("Number Appearing"),
            dice,
            [
                (0, 1_000_000),
                (1, 1_000_000),
                (1, 1_000_000),
                (0, 1_000_000),
            ],
        );
        let group = adw::PreferencesGroup::new();
        group.add(&combo_row);
        group.add(&dice.row);
        group.add(&who);
        let show = {
            let (row, who) = (dice.row.clone(), who.clone());
            move |combo: Combo| {
                row.set_visible(combo != Combo::Appear);
                who.set_visible(combo != Combo::Appear);
            }
        };
        show(combo);
        combo_row.connect_selected_notify(move |row| show(Combo::of(row.selected())));
        Self {
            group,
            combo: combo_row,
            dice,
        }
    }

    fn combo(&self) -> Combo {
        Combo::of(self.combo.selected())
    }
}
