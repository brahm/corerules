//! The user database editors: New, Copy, Edit and Delete for the records
//! of the catalog sections backed by a user file (spells, parts, races,
//! monsters...), the generic record editor, the treasure-table editor and
//! the Expansion's Allowed Classes dialog.
//!
//! Every save writes the one changed user file at once (keeping a `.bak`),
//! then reloads the catalog and shows the saved record.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use cr_store::help::topic;
use cr_store::userdb::allowed::{self, Allowed, AllowedDb};
use cr_store::userdb::classes::{self, Classes};
use cr_store::userdb::monsters::{MonsterTypes, Monsters};
use cr_store::userdb::parts::{self, Parts};
use cr_store::userdb::powers::Powers;
use cr_store::userdb::races::Races;
use cr_store::userdb::spells::Spells;
use cr_store::userdb::tables::{self, TableDb};
use cr_store::userdb::traps::Traps;
use cr_store::userdb::{
    self as udb, FieldDesc, FieldKind, Filter, Flavor, Options, Origin, Sources, Suggestion,
    UserDb, UserKind, Value,
};
use cr_store::{Installation, Result};
use gtk::{gio, glib};

use cr_i18n::tr;

use crate::lists::pick;

mod attacks;
mod levels;
mod templates;
mod widgets;

/// A redraw callback set once its widgets exist.
type Redraw = Rc<RefCell<Option<Rc<dyn Fn()>>>>;
/// Widgets shown when any of their fields is visible.
type Shown<W> = Rc<RefCell<Vec<(W, Vec<usize>)>>>;

pub trait Host {
    fn with_install<T>(&self, run: impl FnOnce(&Installation) -> Result<T>) -> Option<Result<T>>;
    fn toast(&self, message: &str);
    /// Reloads the sections and shows the entry `key` of section `section`.
    fn reload(&self, section: &str, key: Option<String>);
    fn parent(&self) -> gtk::Widget;
    /// The key of the entry on the detail page.
    fn current_key(&self) -> String;
    /// The id of the selected section.
    fn current_section(&self) -> String;
    /// NPC Creation in template mode over `parent`; `chosen` gets the NPC.
    fn template_npc(&self, parent: &gtk::Widget, chosen: crate::npc::Chosen);
}

type Shared<H> = Rc<H>;

fn errors<T>(host: &impl Host, result: Option<Result<T>>) -> Option<T> {
    match result? {
        Ok(value) => Some(value),
        Err(e) => {
            host.toast(&e.to_string());
            None
        }
    }
}

/// The record kind behind a catalog section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Wizard (0) or priest (1) spells.
    Spells(usize),
    Traps,
    Powers,
    MonsterTypes,
    Monsters,
    Races,
    Classes,
    /// Parts of one type.
    Parts(usize),
    /// The section of the other part types, where New asks for the type.
    OtherParts,
    Tables,
    Templates,
}

fn kind_of(section: &str) -> Option<Kind> {
    Some(match section {
        "wizard-spells" => Kind::Spells(0),
        "priest-spells" => Kind::Spells(1),
        "traps" => Kind::Traps,
        "psionics" => Kind::Powers,
        "monster-types" => Kind::MonsterTypes,
        "monsters" => Kind::Monsters,
        "races" => Kind::Races,
        "classes" => Kind::Classes,
        "weapons" => Kind::Parts(parts::WEAPONS),
        "armor" => Kind::Parts(parts::ARMOR),
        "equipment" => Kind::Parts(parts::EQUIPMENT),
        "magic-items" => Kind::Parts(parts::MAGICAL_ITEMS),
        "kits" => Kind::Parts(parts::KITS),
        "nonweapon" => Kind::Parts(4),
        "treasure-tables" => Kind::Tables,
        "templates" => Kind::Templates,
        parts::OTHER_SECTION => Kind::OtherParts,
        id => Kind::Parts(parts::other_type(id)?),
    })
}

/// The title of a new record's editor and the tooltip of New.
fn new_title(kind: Kind) -> &'static str {
    match kind {
        Kind::Spells(0) => tr("New Wizard Spell"),
        Kind::Spells(_) => tr("New Priest Spell"),
        Kind::Traps => tr("New Trap"),
        Kind::Powers => tr("New Psionic Power"),
        Kind::MonsterTypes => tr("New Monster Type"),
        Kind::Monsters => tr("New Monster"),
        Kind::Races => tr("New Race"),
        Kind::Classes => tr("New Class"),
        Kind::Parts(parts::WEAPONS) => tr("New Weapon"),
        Kind::Parts(parts::ARMOR) => tr("New Armor"),
        Kind::Parts(parts::EQUIPMENT) => tr("New Equipment"),
        Kind::Parts(parts::MAGICAL_ITEMS) => tr("New Magical Item"),
        Kind::Parts(parts::KITS) => tr("New Kit"),
        Kind::Parts(4) => tr("New Proficiency"),
        Kind::Parts(5) => tr("New Racial Ability"),
        Kind::Parts(6) => tr("New Language"),
        Kind::Parts(9) => tr("New School of Magic"),
        Kind::Parts(11) => tr("New Trait"),
        Kind::Parts(12) => tr("New Disadvantage"),
        Kind::Parts(13) => tr("New Gem or Art Object"),
        Kind::Parts(parts::CLASS_ABILITIES) => tr("New Class Ability"),
        Kind::Parts(16) => tr("New Currency Type"),
        Kind::Parts(17) => tr("New Count"),
        Kind::Parts(18) => tr("New Sphere"),
        Kind::Parts(_) | Kind::OtherParts => tr("New Part"),
        Kind::Tables => tr("New Treasure Table"),
        Kind::Templates => tr("New Encounter Template"),
    }
}

/// The New item of the detail page's menu.
fn new_menu_label(kind: Option<Kind>) -> &'static str {
    match kind {
        None => tr("_New Record…"),
        Some(Kind::Spells(0)) => tr("_New Wizard Spell…"),
        Some(Kind::Spells(_)) => tr("_New Priest Spell…"),
        Some(Kind::Traps) => tr("_New Trap…"),
        Some(Kind::Powers) => tr("_New Psionic Power…"),
        Some(Kind::MonsterTypes) => tr("_New Monster Type…"),
        Some(Kind::Monsters) => tr("_New Monster…"),
        Some(Kind::Races) => tr("_New Race…"),
        Some(Kind::Classes) => tr("_New Class…"),
        Some(Kind::Parts(parts::WEAPONS)) => tr("_New Weapon…"),
        Some(Kind::Parts(parts::ARMOR)) => tr("_New Armor…"),
        Some(Kind::Parts(parts::EQUIPMENT)) => tr("_New Equipment…"),
        Some(Kind::Parts(parts::MAGICAL_ITEMS)) => tr("_New Magical Item…"),
        Some(Kind::Parts(parts::KITS)) => tr("_New Kit…"),
        Some(Kind::Parts(4)) => tr("_New Proficiency…"),
        // New in the other parts' section asks for the type.
        Some(Kind::Parts(_) | Kind::OtherParts) => tr("_New Part…"),
        Some(Kind::Tables) => tr("_New Treasure Table…"),
        Some(Kind::Templates) => tr("_New Encounter Template…"),
    }
}

/// Whether the section's list has a New button.
pub fn can_create(section: &str) -> bool {
    kind_of(section).is_some()
}

/// The tooltip of the section's New button.
pub fn new_tooltip(section: &str) -> String {
    kind_of(section).map_or_else(|| tr("New").into(), |k| new_title(k).into())
}

/// Whether a section lists standard and custom records.
pub fn has_origins(section: &str) -> bool {
    kind_of(section).is_some()
}

/// Whether the entry with `key` can be edited: custom records, and every
/// treasure table (standard ones are edited in the user copy).
pub fn editable(key: &str) -> bool {
    match udb::parse_key(key) {
        Some((section, origin, _)) => origin == Origin::Custom || section == "treasure-tables",
        None => false,
    }
}

/// The origin of a catalog entry, for the All/Standard/Custom filter.
pub fn origin_of(key: &str) -> Option<Origin> {
    udb::parse_key(key).map(|(_, origin, _)| origin)
}

// ---- actions ----

/// What an action does to a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    New,
    Copy,
    Edit,
    Delete,
    NewSubrace,
}

/// Adds the `win.udb-*` actions; they act on the entry on the detail page
/// (New on the selected section).
pub fn add_actions<H: Host + 'static>(window: &adw::ApplicationWindow, host: Shared<H>) {
    let entry = |name: &str, run: fn(&Shared<H>)| {
        let host = host.clone();
        gio::ActionEntry::builder(name)
            .activate(move |_, _, _| run(&host))
            .build()
    };
    window.add_action_entries([
        entry("udb-new", |h| {
            let section = h.current_section();
            run(h, &section, Origin::Standard, "", Action::New);
        }),
        entry("udb-copy", |h| on_key(h, Action::Copy)),
        entry("udb-edit", |h| on_key(h, Action::Edit)),
        entry("udb-delete", |h| on_key(h, Action::Delete)),
        entry("udb-subrace", |h| on_key(h, Action::NewSubrace)),
        entry("udb-allowed", |h| {
            let key = h.current_key();
            if let Some((_, _, name)) = udb::parse_key(&key) {
                allowed_classes(h, name);
            }
        }),
        entry("udb-restore", |h| {
            let key = h.current_key();
            if let Some((_, _, name)) = udb::parse_key(&key) {
                restore_table(h, name);
            }
        }),
    ]);
}

fn on_key<H: Host + 'static>(host: &Shared<H>, action: Action) {
    let key = host.current_key();
    if let Some((section, origin, name)) = udb::parse_key(&key) {
        run(host, section, origin, name, action);
    }
}

/// Edit… on the detail page of a record.
pub fn edit<H: Host + 'static>(host: &Shared<H>, key: &str) {
    if let Some((section, origin, name)) = udb::parse_key(key) {
        run(host, section, origin, name, Action::Edit);
    }
}

/// New, Copy, Edit or Delete on a record of `section` from another
/// dialog: the Encounter dialog's Traps (IDD 295 has these buttons too)
/// and Create New Monsters… (0x64f291); `action` is "new", "copy", "edit"
/// or "delete".
pub fn record_action<H: Host + 'static>(
    host: &Shared<H>,
    section: &str,
    action: &str,
    origin: Origin,
    name: &str,
) {
    let action = match action {
        "new" => Action::New,
        "copy" => Action::Copy,
        "edit" => Action::Edit,
        "delete" => Action::Delete,
        _ => return,
    };
    run(host, section, origin, name, action);
}

fn run<H: Host + 'static>(
    host: &Shared<H>,
    section: &str,
    origin: Origin,
    name: &str,
    action: Action,
) {
    let Some(kind) = kind_of(section) else {
        return;
    };
    let section: &'static str = SECTIONS
        .iter()
        .copied()
        .chain(parts::OTHER_TYPES.iter().map(|(id, ..)| *id))
        .find(|s| *s == section)
        .unwrap_or("");
    let target = Target {
        section,
        kind,
        origin,
        name: name.to_owned(),
    };
    match kind {
        Kind::Spells(scope) => act::<Spells, H>(host, &target, scope, action),
        Kind::Traps => act::<Traps, H>(host, &target, 0, action),
        Kind::Powers => act::<Powers, H>(host, &target, 0, action),
        Kind::MonsterTypes => act::<MonsterTypes, H>(host, &target, 0, action),
        Kind::Monsters => act::<Monsters, H>(host, &target, 0, action),
        Kind::Races => act::<Races, H>(host, &target, 0, action),
        Kind::Classes => act::<Classes, H>(host, &target, 0, action),
        Kind::Parts(t) => act::<Parts, H>(host, &target, t, action),
        Kind::OtherParts if action == Action::New => new_other_part(host),
        Kind::OtherParts => {}
        Kind::Tables => table_action(host, &target, action),
        Kind::Templates => templates::action(host, &target, action),
    }
}

/// The sections, for a `'static` id.
const SECTIONS: [&str; 16] = [
    "wizard-spells",
    "priest-spells",
    "traps",
    "psionics",
    "monster-types",
    "monsters",
    "races",
    "classes",
    "weapons",
    "armor",
    "equipment",
    "magic-items",
    "kits",
    "nonweapon",
    "treasure-tables",
    "templates",
];

/// New in the other parts' section: asks for the type ("Select the Type"
/// of IDD 139), starting on the shown part's, then opens the editor.
fn new_other_part<H: Host + 'static>(host: &Shared<H>) {
    let Some(flavor) = errors(
        &**host,
        host.with_install(|i| Ok(cr_store::encounter::flavor_of(i))),
    ) else {
        return;
    };
    let offered = parts::editable_types(flavor);
    let types: Vec<(&'static str, &'static str)> = parts::OTHER_TYPES
        .iter()
        .filter(|(_, t, _)| offered.iter().any(|(o, _)| o == t))
        .map(|&(id, _, name)| (id, tr(name)))
        .collect();
    let shown = udb::parse_key(&host.current_key()).map(|(section, ..)| section.to_owned());
    let choice = gtk::DropDown::from_strings(&types.iter().map(|(_, n)| *n).collect::<Vec<_>>());
    choice.set_selected(
        types
            .iter()
            .position(|(id, _)| shown.as_deref() == Some(*id))
            .unwrap_or(0) as u32,
    );
    let alert = adw::AlertDialog::builder()
        .heading(tr("New Part"))
        .body(tr("Select the type of the new part."))
        .extra_child(&choice)
        .default_response("create")
        .close_response("cancel")
        .build();
    alert.add_responses(&[("cancel", tr("_Cancel")), ("create", tr("C_reate"))]);
    alert.set_response_appearance("create", adw::ResponseAppearance::Suggested);
    alert.connect_response(Some("create"), {
        let host = host.clone();
        move |_, _| {
            if let Some((id, _)) = types.get(choice.selected() as usize) {
                run(&host, id, Origin::Standard, "", Action::New);
            }
        }
    });
    alert.present(Some(&host.parent()));
}

/// The record an action is about.
struct Target {
    section: &'static str,
    kind: Kind,
    origin: Origin,
    name: String,
}

/// The index of the record `name` of `scope` in its own list.
fn position<K: UserKind>(
    db: &UserDb<K>,
    origin: Origin,
    scope: usize,
    name: &str,
) -> Option<usize> {
    let list = match origin {
        Origin::Standard => &db.standard,
        Origin::Custom => &db.user,
    };
    list.iter()
        .position(|r| K::scope(r) == scope && K::name(r) == name)
}

fn act<K: UserKind, H: Host + 'static>(
    host: &Shared<H>,
    target: &Target,
    scope: usize,
    action: Action,
) {
    let Some(db) = errors(&**host, host.with_install(UserDb::<K>::load)) else {
        return;
    };
    if K::NOUN == "Class" && db.flavor == Flavor::Core20 {
        return host.toast(tr("Core Rules 2.0 has no custom classes"));
    }
    let found = position(&db, target.origin, scope, &target.name);
    let record = found.and_then(|i| db.get(target.origin, i)).cloned();
    match action {
        Action::New => {
            let record = db.new_record(scope, "");
            editor::<K, H>(
                host,
                target.section,
                db.flavor,
                None,
                new_title(target.kind).to_owned(),
                record,
            );
        }
        Action::NewSubrace => {
            // A subrace's subrace belongs to its race.
            let summary = record.as_ref().map(K::summary).unwrap_or_default();
            let parent = summary.strip_prefix("Subrace of ").unwrap_or(&target.name);
            let record = db.new_record(scope, parent);
            editor::<K, H>(
                host,
                target.section,
                db.flavor,
                None,
                tr!("New Subrace of {race}", race = parent),
                record,
            );
        }
        Action::Copy => {
            let Some(record) = record else { return };
            if let Some(refusal) = K::copy_refusal(&record) {
                return host.toast(&refusal);
            }
            let copy = db.copy_of(&record);
            editor::<K, H>(
                host,
                target.section,
                db.flavor,
                None,
                tr!("Copy of {name}", name = target.name),
                copy,
            );
        }
        Action::Edit => {
            let (Some(record), Some(index)) = (record, found) else {
                return;
            };
            if target.origin == Origin::Standard {
                return host.toast(tr("Standard records cannot be edited; copy them instead"));
            }
            editor::<K, H>(
                host,
                target.section,
                db.flavor,
                Some(index),
                target.name.clone(),
                record,
            );
        }
        Action::Delete => {
            let Some(record) = record else {
                return;
            };
            if target.origin == Origin::Standard {
                return host.toast(tr("Standard records cannot be deleted"));
            }
            let section = target.section;
            let name = target.name.clone();
            let delete = {
                let host = host.clone();
                move || {
                    let result = host.with_install(|install| {
                        let mut db = UserDb::<K>::load(install)?;
                        if let Some(i) = position(&db, Origin::Custom, scope, &name) {
                            db.delete(i);
                        }
                        db.save(install)
                    });
                    if errors(&*host, result).is_some() {
                        host.toast(&tr!("“{name}” deleted", name));
                        host.reload(section, None);
                    }
                }
            };
            match K::delete_warning(&record) {
                Some(warning) => {
                    let alert = adw::AlertDialog::builder()
                        .heading(tr!("Delete {name}?", name = target.name))
                        .body(warning)
                        .default_response("cancel")
                        .close_response("cancel")
                        .build();
                    alert.add_responses(&[("cancel", tr("_Cancel")), ("delete", tr("_Delete"))]);
                    alert.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                    alert.connect_response(Some("delete"), move |_, _| delete());
                    alert.present(Some(&host.parent()));
                }
                // The originals delete these without asking.
                None => delete(),
            }
        }
    }
}

// ---- the generic editor ----

/// One control of the editor.
struct Bound {
    /// Index into the fields.
    field: usize,
    widget: gtk::Widget,
    /// Shows the record's value (unless it already does).
    load: Box<dyn Fn()>,
}

/// Opens the editor on `record`; `editing` is its index in the user list
/// when it is changed rather than added.
fn editor<K: UserKind, H: Host + 'static>(
    host: &Shared<H>,
    section: &'static str,
    flavor: Flavor,
    editing: Option<usize>,
    title: String,
    record: K::Rec,
) {
    let fields = Rc::new(K::fields(flavor));
    let Some(sources) = errors(&**host, host.with_install(|i| Sources::load(i, &fields))) else {
        return;
    };
    let sources = Rc::new(sources);
    let record = Rc::new(RefCell::new(record));
    let loading = Rc::new(Cell::new(false));
    let bound: Rc<RefCell<Vec<Bound>>> = Rc::default();
    let groups: Shown<gtk::Widget> = Rc::default();
    let pages: Shown<gtk::ListBoxRow> = Rc::default();

    // Shows visibility, sensitivity and values after a change of field
    // `skip` (which keeps what the user typed).
    let refresh: Rc<dyn Fn(Option<usize>)> = Rc::new({
        let (fields, record, loading, bound, groups, pages) = (
            fields.clone(),
            record.clone(),
            loading.clone(),
            bound.clone(),
            groups.clone(),
            pages.clone(),
        );
        move |skip| {
            loading.set(true);
            let r = record.borrow().clone();
            let bound = bound.borrow();
            let mut visible = vec![false; bound.len()];
            for (i, b) in bound.iter().enumerate() {
                let field = &fields[b.field];
                visible[i] = field.is_visible(&r);
                b.widget.set_visible(visible[i]);
                b.widget.set_sensitive(field.is_enabled(&r));
                if Some(b.field) != skip {
                    (b.load)();
                }
            }
            for (group, members) in groups.borrow().iter() {
                group.set_visible(members.iter().any(|m| visible[*m]));
            }
            for (row, members) in pages.borrow().iter() {
                row.set_visible(members.iter().any(|m| visible[*m]));
            }
            loading.set(false);
        }
    });
    let changed: Rc<dyn Fn(usize, Value)> = Rc::new({
        let (fields, record, loading, sources, refresh) = (
            fields.clone(),
            record.clone(),
            loading.clone(),
            sources.clone(),
            refresh.clone(),
        );
        move |field, value| {
            if loading.get() {
                return;
            }
            fields[field].set(&mut record.borrow_mut(), &value, &sources);
            refresh(Some(field));
        }
    });

    // Pages, in the order of their first field, each with its groups.
    let stack = adw::ViewStack::new();
    let sidebar = gtk::ListBox::builder()
        .css_classes(["navigation-sidebar"])
        .build();
    let mut page_names: Vec<&'static str> = Vec::new();
    for f in fields.iter() {
        if !page_names.contains(&f.page) {
            page_names.push(f.page);
        }
    }
    for page_name in &page_names {
        let page = adw::PreferencesPage::new();
        let mut page_members = Vec::new();
        let mut page_groups: Vec<(&'static str, adw::PreferencesGroup, Vec<usize>)> = Vec::new();
        for (index, field) in fields
            .iter()
            .enumerate()
            .filter(|(_, f)| f.page == *page_name)
        {
            let (widget, load) = control(field, index, &record, &sources, &changed, host);
            // Lists and long text get a group of their own, titled with
            // the label.
            let own = matches!(
                field.kind,
                FieldKind::Dual(_)
                    | FieldKind::MultiLine
                    | FieldKind::Checked(_)
                    | FieldKind::Pairs { .. }
                    | FieldKind::Groups(_)
                    | FieldKind::Grid { .. }
                    | FieldKind::Items
                    | FieldKind::Attacks
                    | FieldKind::FreeSpells
                    | FieldKind::LevelNwps(_)
                    | FieldKind::LevelAbilities(_)
                    | FieldKind::AutoFill
            );
            let group_title = if own && field.group.is_empty() {
                field.label
            } else {
                field.group
            };
            let slot = page_groups
                .iter()
                .position(|(t, _, _)| *t == group_title && !own);
            let slot = match slot {
                Some(slot) => slot,
                None => {
                    let group = adw::PreferencesGroup::new();
                    if !group_title.is_empty() && !(group_title == *page_name && own) {
                        group.set_title(&glib::markup_escape_text(tr(group_title)));
                    }
                    page.add(&group);
                    page_groups.push((group_title, group, Vec::new()));
                    page_groups.len() - 1
                }
            };
            page_groups[slot].1.add(&widget);
            let b = bound.borrow().len();
            page_groups[slot].2.push(b);
            page_members.push(b);
            bound.borrow_mut().push(Bound {
                field: index,
                widget,
                load,
            });
        }
        for (_, group, members) in page_groups {
            groups.borrow_mut().push((group.upcast(), members));
        }
        stack.add_titled(&page, Some(page_name), tr(page_name));
        let row = gtk::ListBoxRow::builder()
            .child(
                &gtk::Label::builder()
                    .label(tr(page_name))
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
        pages.borrow_mut().push((row, page_members));
    }
    refresh(None);

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
    crate::help::add_button(&header, &view, move || topic::of_editor(section));
    view.add_top_bar(&header);
    let many = page_names.len() > 1;
    if many {
        let split = adw::OverlaySplitView::builder()
            .sidebar(
                &gtk::ScrolledWindow::builder()
                    .hscrollbar_policy(gtk::PolicyType::Never)
                    .child(&sidebar)
                    .build(),
            )
            .content(&stack)
            .min_sidebar_width(150.0)
            .max_sidebar_width(200.0)
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
        let dialog_breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            560.0,
            adw::LengthUnit::Sp,
        ));
        dialog_breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
        let dialog = dialog_for(&title, &view, many);
        dialog.add_breakpoint(dialog_breakpoint);
        finish::<K, H>(host, dialog, section, editing, record, cancel, save);
    } else {
        view.set_content(Some(&stack));
        let dialog = dialog_for(&title, &view, many);
        finish::<K, H>(host, dialog, section, editing, record, cancel, save);
    }
}

fn dialog_for(title: &str, view: &adw::ToolbarView, many: bool) -> adw::Dialog {
    adw::Dialog::builder()
        .title(title)
        .content_width(if many { 820 } else { 600 })
        .content_height(680)
        .width_request(360)
        .height_request(294)
        .child(view)
        .build()
}

/// Wires Cancel and Save and shows the dialog.
fn finish<K: UserKind, H: Host + 'static>(
    host: &Shared<H>,
    dialog: adw::Dialog,
    section: &'static str,
    editing: Option<usize>,
    record: Rc<RefCell<K::Rec>>,
    cancel: gtk::Button,
    save: gtk::Button,
) {
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    save.connect_clicked({
        let (host, dialog) = (host.clone(), dialog.clone());
        move |_| {
            let r = record.borrow().clone();
            let result = host.with_install(|install| {
                let mut db = UserDb::<K>::load(install)?;
                let stored = match editing {
                    Some(i) if i < db.user.len() => db.replace(i, r.clone()),
                    _ => db.add(r.clone()),
                };
                match stored {
                    Ok(i) => {
                        db.save(install)?;
                        Ok(Ok(K::name(&db.user[i]).to_owned()))
                    }
                    Err(error) => Ok(Err((error, r.clone()))),
                }
            });
            match errors(&*host, result) {
                Some(Ok(name)) => {
                    dialog.close();
                    host.reload(section, Some(udb::key(section, Origin::Custom, &name)));
                    host.toast(&tr!("“{name}” saved", name));
                    if editing.is_none() && K::NOUN == "Class" {
                        message(
                            &host.parent(),
                            tr("Class Added"),
                            &classes::added_message(&name),
                        );
                    }
                }
                Some(Err((error, r))) => {
                    let heading = if error == udb::NameError::Slash {
                        tr(classes::SLASH_TITLE)
                    } else {
                        tr("Not Saved")
                    };
                    message(
                        &dialog,
                        heading,
                        &K::name_message(&error, &r, editing.is_none()),
                    );
                }
                None => {}
            }
        }
    });
    dialog.present(Some(&host.parent()));
}

/// A message with an OK button.
fn message(parent: &impl IsA<gtk::Widget>, heading: &str, body: &str) {
    let alert = adw::AlertDialog::builder()
        .heading(heading)
        .body(body)
        .default_response("ok")
        .close_response("ok")
        .build();
    alert.add_response("ok", tr("_OK"));
    alert.present(Some(parent));
}

/// The widget for one field and how it shows the record's value.
fn control<R: Clone + 'static, H: Host + 'static>(
    field: &FieldDesc<R>,
    index: usize,
    record: &Rc<RefCell<R>>,
    sources: &Rc<Sources>,
    changed: &Rc<dyn Fn(usize, Value)>,
    host: &Shared<H>,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let label = tr(field.label);
    let value = {
        let record = record.clone();
        let getter = (
            field.getter,
            field.source_getter,
            field.arg,
            sources.clone(),
        );
        move || match getter.1 {
            Some(get) => get(&record.borrow(), &getter.3),
            None => (getter.0)(&record.borrow(), getter.2),
        }
    };
    match field.kind {
        FieldKind::Text { .. } => {
            let row = adw::EntryRow::builder()
                .title(glib::markup_escape_text(label).as_str())
                .build();
            let changed = changed.clone();
            row.connect_changed(move |row| changed(index, Value::Text(row.text().into())));
            let load = {
                let row = row.clone();
                move || {
                    let text = value().text();
                    if row.text() != text {
                        row.set_text(&text);
                    }
                }
            };
            (row.upcast(), Box::new(load))
        }
        FieldKind::Int { min, max } => {
            let row = adw::SpinRow::builder()
                .title(glib::markup_escape_text(label).as_str())
                .adjustment(&gtk::Adjustment::new(
                    0.0, min as f64, max as f64, 1.0, 10.0, 0.0,
                ))
                .numeric(true)
                .build();
            let changed = changed.clone();
            row.connect_value_notify(move |row| changed(index, Value::Int(row.value() as i64)));
            if let Some(suggestion) = &field.suggest {
                suggest_button(&row, suggestion, record);
            }
            let load = {
                let row = row.clone();
                move || {
                    let v = value().int() as f64;
                    if row.value() != v {
                        row.set_value(v);
                    }
                }
            };
            (row.upcast(), Box::new(load))
        }
        FieldKind::Check => {
            let row = adw::SwitchRow::builder()
                .title(glib::markup_escape_text(label).as_str())
                .build();
            let changed = changed.clone();
            row.connect_active_notify(move |row| changed(index, Value::Bool(row.is_active())));
            let load = {
                let row = row.clone();
                move || {
                    let v = value().bool();
                    if row.is_active() != v {
                        row.set_active(v);
                    }
                }
            };
            (row.upcast(), Box::new(load))
        }
        FieldKind::Choice(options)
        | FieldKind::Combo {
            options,
            free: false,
        } => {
            let mut choices = sources.options(options);
            // A text value missing from the list is kept as a choice.
            let as_text = matches!(value(), Value::Text(_));
            if as_text {
                let current = value().text();
                if !choices.contains(&current) {
                    choices.insert(0, current);
                }
            }
            let choices = Rc::new(choices);
            // Fixed choices are interface words, shown translated; the
            // value is still taken from `choices` by position.
            let translate = matches!(field.kind, FieldKind::Choice(Options::Fixed(_)));
            let shown: Vec<&str> = choices
                .iter()
                .map(|c| if translate { tr(c) } else { c.as_str() })
                .collect();
            let row = adw::ComboRow::builder()
                .title(glib::markup_escape_text(label).as_str())
                .model(&gtk::StringList::new(&shown))
                .enable_search(choices.len() > 12)
                .build();
            {
                let (changed, choices) = (changed.clone(), choices.clone());
                row.connect_selected_notify(move |row| {
                    let i = row.selected() as usize;
                    if i >= choices.len() {
                        return;
                    }
                    changed(
                        index,
                        if as_text {
                            Value::Text(choices[i].clone())
                        } else {
                            Value::Int(i as i64)
                        },
                    );
                });
            }
            let load = {
                let row = row.clone();
                move || {
                    let selected = match value() {
                        Value::Text(t) => choices.iter().position(|c| *c == t).unwrap_or(0),
                        v => v.int().max(0) as usize,
                    } as u32;
                    if row.selected() != selected {
                        row.set_selected(selected);
                    }
                }
            };
            (row.upcast(), Box::new(load))
        }
        FieldKind::Combo {
            options,
            free: true,
        } => {
            let row = adw::EntryRow::builder()
                .title(glib::markup_escape_text(label).as_str())
                .build();
            let choices = sources.options(options);
            if !choices.is_empty() {
                let button = gtk::Button::builder()
                    .icon_name("view-list-symbolic")
                    .tooltip_text(tr("Choose"))
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                let (row2, host) = (row.clone(), host.clone());
                let title = tr!("Choose {field}", field = label);
                button.connect_clicked(move |_| {
                    let row = row2.clone();
                    pick(&host.parent(), &title, choices.clone(), move |c| {
                        row.set_text(&c)
                    });
                });
                row.add_suffix(&button);
            }
            let changed = changed.clone();
            row.connect_changed(move |row| changed(index, Value::Text(row.text().into())));
            let load = {
                let row = row.clone();
                move || {
                    let text = value().text();
                    if row.text() != text {
                        row.set_text(&text);
                    }
                }
            };
            (row.upcast(), Box::new(load))
        }
        FieldKind::MultiLine => {
            let text = gtk::TextView::builder()
                .wrap_mode(gtk::WrapMode::WordChar)
                .top_margin(12)
                .bottom_margin(12)
                .left_margin(12)
                .right_margin(12)
                .build();
            let changed = changed.clone();
            text.buffer().connect_changed(move |b| {
                let (start, end) = b.bounds();
                changed(index, Value::Text(b.text(&start, &end, false).into()));
            });
            let scroll = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .min_content_height(220)
                .child(&text)
                .css_classes(["card"])
                .build();
            let load = move || {
                let v = value().text();
                let b = text.buffer();
                let (start, end) = b.bounds();
                if b.text(&start, &end, false) != v {
                    b.set_text(&v);
                }
            };
            (scroll.upcast(), Box::new(load))
        }
        FieldKind::Dual(options) => {
            widgets::names_control(sources.options(options), None, value, emit(changed, index))
        }
        FieldKind::Pairs {
            options,
            number,
            min,
            max,
        } => widgets::names_control(
            sources.options(options),
            Some(widgets::Number {
                label: tr(number),
                min,
                max,
            }),
            value,
            emit(changed, index),
        ),
        FieldKind::Checked(options) => {
            widgets::checked_control(label, sources.options(options), value, emit(changed, index))
        }
        FieldKind::OptInt {
            min,
            max,
            none,
            start,
            inverted,
        } => widgets::opt_int_control(
            label,
            (min, max, none, start, inverted),
            value,
            emit(changed, index),
        ),
        FieldKind::Real { min, max, none } => {
            widgets::real_control(label, (min, max), none, value, emit(changed, index))
        }
        FieldKind::Groups(options) => {
            let host = host.clone();
            widgets::groups_control(
                label,
                sources.options(options),
                value,
                emit(changed, index),
                move || host.parent(),
            )
        }
        FieldKind::Grid {
            columns,
            first,
            min,
            max,
        } => widgets::grid_control(columns, first, (min, max), value, emit(changed, index)),
        FieldKind::Items => items_control(index, value, changed, host),
        FieldKind::Attacks => {
            attacks::attacks_control(sources.attacks.clone(), value, emit(changed, index))
        }
        FieldKind::FreeSpells => {
            let spells = sources
                .spells
                .iter()
                .map(|s| (s.name.clone(), s.priest != 0, s.level))
                .collect::<Vec<_>>();
            let mut spells = spells;
            spells.sort_by_cached_key(|(n, _, _)| cr_store::collate::sort_key(n));
            levels::free_spells_control(
                Rc::new(spells),
                value,
                emit(changed, index),
                parent_of(host),
            )
        }
        FieldKind::LevelNwps(options) => levels::level_nwps_control(
            sources.options(options),
            value,
            emit(changed, index),
            parent_of(host),
        ),
        FieldKind::LevelAbilities(options) => levels::level_abilities_control(
            sources.options(options),
            value,
            emit(changed, index),
            parent_of(host),
        ),
        FieldKind::AutoFill => levels::auto_fill_control(
            sources.list("class-records").to_vec(),
            emit(changed, index),
            parent_of(host),
        ),
        FieldKind::Dice => {
            let spins: Vec<gtk::SpinButton> = [(0.0, 1000.0), (0.0, 1000.0), (-1000.0, 100_000.0)]
                .iter()
                .map(|(min, max)| {
                    gtk::SpinButton::builder()
                        .adjustment(&gtk::Adjustment::new(0.0, *min, *max, 1.0, 5.0, 0.0))
                        .numeric(true)
                        .valign(gtk::Align::Center)
                        .width_chars(4)
                        .build()
                })
                .collect();
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(label).as_str())
                .build();
            let parts = gtk::Box::builder().spacing(6).build();
            for (i, spin) in spins.iter().enumerate() {
                parts.append(spin);
                if i < 2 {
                    parts.append(&gtk::Label::new(Some(if i == 0 { "d" } else { "+" })));
                }
            }
            row.add_suffix(&parts);
            let spins = Rc::new(spins);
            for spin in spins.iter() {
                let (changed, spins) = (changed.clone(), spins.clone());
                spin.connect_value_changed(move |_| {
                    let v = [0, 1, 2].map(|i| spins[i].value() as i64);
                    changed(index, Value::Dice(v));
                });
            }
            let load = move || {
                let d = value().dice();
                for (spin, v) in spins.iter().zip(d) {
                    if spin.value() as i64 != v {
                        spin.set_value(v as f64);
                    }
                }
            };
            (row.upcast(), Box::new(load))
        }
        FieldKind::Picture(kind) => {
            let root = host
                .with_install(|i| Ok(i.root().to_owned()))
                .and_then(|r| r.ok())
                .unwrap_or_default();
            let set = {
                let (root, changed) = (root.clone(), changed.clone());
                move |path: Option<&std::path::Path>| {
                    let stored = path
                        .and_then(|p| cr_store::pictures::stored_path(&root, p))
                        .unwrap_or_default();
                    changed(index, Value::Text(stored.clone()));
                    stored
                }
            };
            let (row, show) = crate::picture::row(
                root,
                kind,
                String::new(),
                glib::markup_escape_text(label).as_str(),
                value().text(),
                set,
            );
            (row.upcast(), Box::new(move || show(value().text())))
        }
        FieldKind::Icon => {
            let root = host
                .with_install(|i| Ok(i.root().to_owned()))
                .and_then(|r| r.ok())
                .unwrap_or_default();
            let set = {
                let changed = changed.clone();
                move |stored: Option<String>| {
                    let stored = stored.unwrap_or_default();
                    changed(index, Value::Text(stored.clone()));
                    stored
                }
            };
            let (row, show) = crate::picture::icon_row(
                root,
                glib::markup_escape_text(label).as_str(),
                value().text(),
                set,
            );
            (row.upcast(), Box::new(move || show(value().text())))
        }
    }
}

/// The change callback of one field.
/// The button that offers a number's suggested value, which Yes puts in
/// `row` (Suggest XP Value, 0x6d60fa).
fn suggest_button<R: 'static>(
    row: &adw::SpinRow,
    suggestion: &Suggestion<R>,
    record: &Rc<RefCell<R>>,
) {
    let button = gtk::Button::builder()
        .label(tr(suggestion.label))
        .valign(gtk::Align::Center)
        .build();
    let (value, question) = (suggestion.value, suggestion.question);
    button.connect_clicked({
        let (row, record) = (row.clone(), record.clone());
        move |_| {
            let suggested = value(&record.borrow());
            let dialog = adw::AlertDialog::builder()
                .body(question(suggested))
                .build();
            dialog.add_responses(&[("no", tr("_No")), ("yes", tr("_Yes"))]);
            dialog.set_response_appearance("yes", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("yes"));
            dialog.set_close_response("no");
            let row2 = row.clone();
            dialog.connect_response(None, move |_, response| {
                if response == "yes" {
                    row2.set_value(suggested as f64);
                }
            });
            dialog.present(Some(&row));
        }
    });
    row.add_suffix(&button);
}

/// Items edited as the Treasure Bundle Editor edits a bundle
/// ([`FieldKind::Items`]): a monster's Inventory.
fn items_control<H: Host + 'static>(
    index: usize,
    value: impl Fn() -> Value + 'static,
    changed: &Rc<dyn Fn(usize, Value)>,
    host: &Shared<H>,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    let Some(rules) = errors(&**host, host.with_install(|i| i.treasure_rules())) else {
        return (root.upcast(), Box::new(|| ()));
    };
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    root.append(&list);
    let value = Rc::new(value);
    let edit: Rc<std::cell::OnceCell<crate::bundle::Edit>> = Rc::default();
    let redraw: Rc<dyn Fn()> = Rc::new({
        let (list, value, edit) = (list.clone(), value.clone(), Rc::downgrade(&edit));
        move || {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let bundle = value().items();
            if let Some(edit) = edit.upgrade()
                && let Some(edit) = edit.get()
            {
                for row in crate::bundle::rows(edit, &bundle) {
                    list.append(&row);
                }
            }
            list.set_visible(!bundle.is_empty());
        }
    });
    let editing = Rc::new(crate::bundle::Editing {
        rules: Rc::new(rules),
        parent: root.clone().upcast(),
        get: Box::new({
            let value = value.clone();
            move || Some(value().items())
        }),
        set: Box::new({
            let (changed, redraw) = (changed.clone(), redraw.clone());
            move |bundle| {
                changed(index, Value::Items(bundle));
                redraw();
            }
        }),
    });
    let add = crate::bundle::add_button(&editing);
    add.set_css_classes(&["pill"]);
    add.set_halign(gtk::Align::Center);
    root.append(&add);
    let _ = edit.set(editing);
    // The editor keeps the editing alive.
    let keep = edit.clone();
    (
        root.upcast(),
        Box::new(move || {
            let _ = &keep;
            redraw()
        }),
    )
}

fn parent_of<H: Host + 'static>(host: &Shared<H>) -> levels::Parent {
    let host = host.clone();
    Rc::new(move || host.parent())
}

fn emit(changed: &Rc<dyn Fn(usize, Value)>, index: usize) -> Rc<dyn Fn(Value)> {
    let changed = changed.clone();
    Rc::new(move |v| changed(index, v))
}

// ---- treasure tables ----

fn table_action<H: Host + 'static>(host: &Shared<H>, target: &Target, action: Action) {
    let Some(db) = errors(&**host, host.with_install(TableDb::load)) else {
        return;
    };
    let index = db.position(&target.name);
    match action {
        Action::New | Action::NewSubrace => table_editor(host, None, TableDb::new_table()),
        Action::Copy => {
            if let Some(i) = index {
                table_editor(host, None, TableDb::copy_of(&db.tables[i]));
            }
        }
        Action::Edit => {
            if let Some(i) = index {
                table_editor(host, Some(i), db.tables[i].clone());
            }
        }
        Action::Delete => {
            let Some(i) = index else { return };
            if tables::is_standard(&db.tables[i]) {
                return host.toast(tr("Standard tables can be restored, not deleted"));
            }
            let alert = adw::AlertDialog::builder()
                .heading(tr("Are you sure?"))
                .body(tr(tables::DELETE_WARNING))
                .default_response("cancel")
                .close_response("cancel")
                .build();
            alert.add_responses(&[("cancel", tr("_Cancel")), ("delete", tr("_Delete"))]);
            alert.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
            let (host2, name) = (host.clone(), target.name.clone());
            alert.connect_response(Some("delete"), move |_, _| {
                let result = host2.with_install(|install| {
                    let mut db = TableDb::load(install)?;
                    if let Some(i) = db.position(&name) {
                        db.delete(i).map_err(cr_store::Error::Invalid)?;
                    }
                    db.save(install)
                });
                if errors(&*host2, result).is_some() {
                    host2.toast(&format!("“{name}” deleted"));
                    host2.reload("treasure-tables", None);
                }
            });
            alert.present(Some(&host.parent()));
        }
    }
}

fn restore_table<H: Host + 'static>(host: &Shared<H>, name: &str) {
    let alert = adw::AlertDialog::builder()
        .heading(tr("Restore Table?"))
        .body(tr(tables::RESTORE_WARNING))
        .default_response("cancel")
        .close_response("cancel")
        .build();
    alert.add_responses(&[("cancel", tr("_Cancel")), ("restore", tr("_Restore"))]);
    alert.set_response_appearance("restore", adw::ResponseAppearance::Destructive);
    let (host2, name) = (host.clone(), name.to_owned());
    alert.connect_response(Some("restore"), move |_, _| {
        let result = host2.with_install(|install| {
            let mut db = TableDb::load(install)?;
            if let Some(i) = db.position(&name) {
                db.restore(i).map_err(cr_store::Error::Invalid)?;
            }
            db.save(install)
        });
        if errors(&*host2, result).is_some() {
            host2.toast(&tr!("“{name}” restored", name));
            host2.reload(
                "treasure-tables",
                Some(udb::key("treasure-tables", Origin::Standard, &name)),
            );
        }
    });
    alert.present(Some(&host.parent()));
}

/// The treasure-table editor (IDD 258 with the IDD 262 entry details):
/// the table's name and its rows, each an item or another table with a
/// weight and how many are rolled.
fn table_editor<H: Host + 'static>(
    host: &Shared<H>,
    editing: Option<usize>,
    table: cr_formats::treasure::TreasureTable,
) {
    let title = match editing {
        Some(_) => table.name.clone(),
        None => tr("New Treasure Table").to_owned(),
    };
    let table = Rc::new(RefCell::new(table));
    let name = adw::EntryRow::builder()
        .title(tr("Name"))
        .text(&table.borrow().name)
        .build();
    let about = adw::PreferencesGroup::new();
    about.add(&name);
    let add = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text(tr("Add Entry"))
        .css_classes(["flat"])
        .build();
    let rows_group = adw::PreferencesGroup::builder()
        .title(tr("Entries"))
        .description(tr("Each entry is rolled in proportion to its weight"))
        .header_suffix(&add)
        .build();
    let rows_box = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    rows_group.add(&rows_box);
    let draw: Redraw = Rc::default();
    let render: Rc<dyn Fn()> = Rc::new({
        let (table, rows_box, draw) = (table.clone(), rows_box.clone(), draw.clone());
        move || {
            while let Some(child) = rows_box.first_child() {
                rows_box.remove(&child);
            }
            let t = table.borrow().clone();
            let ranges = tables::ranges(&t);
            if ranges.is_empty() {
                rows_box.append(
                    &adw::ActionRow::builder()
                        .title(tr("No entries"))
                        .css_classes(["dim-label"])
                        .build(),
                );
            }
            for (i, (entry, range)) in tables::entries(&t).into_iter().zip(ranges).enumerate() {
                let item = if entry.is_table != 0 {
                    tr!("Table: {name}", name = entry.item.name)
                } else {
                    entry.item.name.clone()
                };
                let row = adw::ExpanderRow::builder()
                    .title(glib::markup_escape_text(&item).as_str())
                    .subtitle(tr!("Roll {range}", range))
                    .build();
                let weight = adw::SpinRow::builder()
                    .title(tr("Weight"))
                    .adjustment(&gtk::Adjustment::new(
                        entry.weight.into(),
                        0.0,
                        100_000.0,
                        1.0,
                        10.0,
                        0.0,
                    ))
                    .build();
                let (table2, draw2) = (table.clone(), draw.clone());
                weight.connect_value_notify(move |w| {
                    if let Some(e) = tables::entries_mut(&mut table2.borrow_mut())
                        .into_iter()
                        .nth(i)
                    {
                        e.weight = w.value() as u32;
                    }
                    // Ranges follow the weights; redraw after the change.
                    let draw2 = draw2.clone();
                    glib::idle_add_local_once(move || {
                        if let Some(d) = draw2.borrow().clone() {
                            d();
                        }
                    });
                });
                row.add_row(&weight);
                let dice = adw::ActionRow::builder().title(tr("Quantity")).build();
                let quantity = gtk::Box::builder().spacing(6).build();
                let spins: Vec<gtk::SpinButton> = (0..3)
                    .map(|k| {
                        gtk::SpinButton::builder()
                            .adjustment(&gtk::Adjustment::new(
                                entry.quantity_dice[k].into(),
                                0.0,
                                100_000.0,
                                1.0,
                                5.0,
                                0.0,
                            ))
                            .numeric(true)
                            .width_chars(4)
                            .valign(gtk::Align::Center)
                            .build()
                    })
                    .collect();
                for (k, spin) in spins.iter().enumerate() {
                    quantity.append(spin);
                    if k < 2 {
                        quantity.append(&gtk::Label::new(Some(if k == 0 { "d" } else { "+" })));
                    }
                    let table2 = table.clone();
                    spin.connect_value_changed(move |s| {
                        if let Some(e) = tables::entries_mut(&mut table2.borrow_mut())
                            .into_iter()
                            .nth(i)
                        {
                            e.quantity_dice[k] = s.value() as u32;
                        }
                    });
                }
                dice.add_suffix(&quantity);
                row.add_row(&dice);
                let remove = gtk::Button::builder()
                    .icon_name("user-trash-symbolic")
                    .tooltip_text(tr("Remove Entry"))
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                let (table2, draw2) = (table.clone(), draw.clone());
                remove.connect_clicked(move |_| {
                    let mut t = table2.borrow_mut();
                    let mut seen = 0;
                    t.entries.retain(|o| {
                        !matches!(o, cr_formats::Object::TreasureTableEntry(_)) || {
                            seen += 1;
                            seen - 1 != i
                        }
                    });
                    drop(t);
                    if let Some(d) = draw2.borrow().clone() {
                        d();
                    }
                });
                row.add_suffix(&remove);
                rows_box.append(&row);
            }
        }
    });
    *draw.borrow_mut() = Some(render.clone());
    render();

    add.connect_clicked({
        let (host, table, render) = (host.clone(), table.clone(), render.clone());
        move |_| add_table_entry(&host, &table, &render)
    });

    let page = adw::PreferencesPage::new();
    page.add(&about);
    page.add(&rows_group);
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
    crate::help::add_button(&header, &view, || topic::TREASURE_TABLE);
    view.add_top_bar(&header);
    view.set_content(Some(&page));
    let dialog = dialog_for(&title, &view, false);
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    save.connect_clicked({
        let (host, dialog) = (host.clone(), dialog.clone());
        move |_| {
            let mut t = table.borrow().clone();
            t.name = name.text().into();
            let result = host.with_install(|install| {
                let mut db = TableDb::load(install)?;
                let stored = match editing {
                    Some(i) if i < db.tables.len() => db.replace(i, t.clone()).map(|()| i),
                    _ => db.add(t.clone()),
                };
                match stored {
                    Ok(i) => {
                        db.save(install)?;
                        let t = &db.tables[i];
                        Ok(Ok((t.name.clone(), tables::origin(t))))
                    }
                    Err(message) => Ok(Err(message)),
                }
            });
            match errors(&*host, result) {
                Some(Ok((name, origin))) => {
                    dialog.close();
                    host.reload(
                        "treasure-tables",
                        Some(udb::key("treasure-tables", origin, &name)),
                    );
                    host.toast(&tr!("“{name}” saved", name));
                }
                Some(Err(text)) => message(&dialog, tr("Not Saved"), &text),
                None => {}
            }
        }
    });
    dialog.present(Some(&host.parent()));
}

/// Add Entry: a category ("Weapons, Custom", "Tables") and one of its
/// items.
fn add_table_entry<H: Host + 'static>(
    host: &Shared<H>,
    table: &Rc<RefCell<cr_formats::treasure::TreasureTable>>,
    render: &Rc<dyn Fn()>,
) {
    let sources = tables::source_names();
    let (host2, table, render) = (host.clone(), table.clone(), render.clone());
    pick(&host.parent(), "Category", sources, move |source| {
        let items = host2.with_install(|install| {
            let db = TableDb::load(install)?;
            tables::source_items(install, &db, &source)
        });
        let Some((items, is_table)) = errors(&*host2, items) else {
            return;
        };
        if items.is_empty() {
            return host2.toast(tr("Nothing in this category"));
        }
        let (table, render) = (table.clone(), render.clone());
        pick(&host2.parent(), &source, items, move |item| {
            tables::push_entry(
                &mut table.borrow_mut(),
                tables::new_entry(&item, is_table, 1),
            );
            render();
        });
    });
}

// ---- Allowed Classes (Expansion) ----

/// Allowed Classes… for race `race`: the shipped entries as they are, the
/// user's with a level limit; multi-classes from Add Multi-Class….
fn allowed_classes<H: Host + 'static>(host: &Shared<H>, race: &str) {
    let loaded = host.with_install(|install| {
        let mut db = AllowedDb::load(install)?;
        // Opening the dialog creates the race's record and, as the
        // original does, writes the file at once.
        let (_, created) = db.ensure(race);
        if created {
            db.save(install)?;
        }
        let classes = classes::class_list(install)?;
        let dual = UserDb::<Races>::load(install)?
            .list(Filter::All)
            .iter()
            .find(|e| e.record.name == race)
            .is_some_and(|e| e.record.unknown_0x1e0[2] != 0);
        Ok((db, classes, dual))
    });
    let Some((db, class_list, dual)) = errors(&**host, loaded) else {
        return;
    };
    let entries = Rc::new(RefCell::new(db.entries(race)));
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    let draw: Redraw = Rc::default();
    let render: Rc<dyn Fn()> = Rc::new({
        let (entries, list, draw) = (entries.clone(), list.clone(), draw.clone());
        move || {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let mut shown: Vec<(usize, Allowed)> =
                entries.borrow().iter().cloned().enumerate().collect();
            shown.sort_by_cached_key(|(_, a)| cr_store::collate::sort_key(&a.name));
            for (i, a) in shown {
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&a.name).as_str())
                    .subtitle(if a.standard {
                        tr("Standard")
                    } else {
                        tr("Custom")
                    })
                    .build();
                let limit = gtk::SpinButton::builder()
                    .adjustment(&gtk::Adjustment::new(
                        a.level_limit.into(),
                        0.0,
                        100.0,
                        1.0,
                        5.0,
                        0.0,
                    ))
                    .numeric(true)
                    .valign(gtk::Align::Center)
                    .tooltip_text(tr("Level Limit (0: none)"))
                    .sensitive(!a.standard)
                    .build();
                let entries2 = entries.clone();
                limit.connect_value_changed(move |s| {
                    entries2.borrow_mut()[i].level_limit = s.value() as i32;
                });
                row.add_suffix(&limit);
                if !a.standard {
                    let remove = gtk::Button::builder()
                        .icon_name("list-remove-symbolic")
                        .tooltip_text("Remove")
                        .valign(gtk::Align::Center)
                        .css_classes(["flat"])
                        .build();
                    let (entries2, draw2) = (entries.clone(), draw.clone());
                    remove.connect_clicked(move |_| {
                        entries2.borrow_mut().remove(i);
                        if let Some(d) = draw2.borrow().clone() {
                            d();
                        }
                    });
                    row.add_suffix(&remove);
                }
                list.append(&row);
            }
        }
    });
    *draw.borrow_mut() = Some(render.clone());
    render();

    let add_class = gtk::Button::builder()
        .label(tr("Add _Class…"))
        .use_underline(true)
        .build();
    let add_multi = gtk::Button::builder()
        .label(tr("Add _Multi-Class…"))
        .use_underline(true)
        .build();
    let buttons = gtk::Box::builder().spacing(6).build();
    buttons.append(&add_class);
    buttons.append(&add_multi);
    let group = adw::PreferencesGroup::builder()
        .title(tr("Classes Allowed"))
        .description(tr("Standard entries are kept as shipped"))
        .header_suffix(&buttons)
        .build();
    group.add(&list);
    let class_list = Rc::new(class_list);
    let groups_of = {
        let class_list = class_list.clone();
        move |name: &str| -> Option<[String; 5]> {
            let names: Vec<&str> = name.split('/').collect();
            Allowed::from_names(&names, &class_list, 0).map(|a| a.classes)
        }
    };
    let groups_of = Rc::new(groups_of);
    add_class.connect_clicked({
        let (host, entries, render, class_list, groups_of) = (
            host.clone(),
            entries.clone(),
            render.clone(),
            class_list.clone(),
            groups_of.clone(),
        );
        move |_| {
            let taken: Vec<String> = entries.borrow().iter().map(|a| a.name.clone()).collect();
            let choices: Vec<String> = class_list
                .iter()
                .map(|c| c.name.clone())
                .filter(|n| !taken.contains(n))
                .collect();
            let (entries, render, groups_of) = (entries.clone(), render.clone(), groups_of.clone());
            pick(
                &host.parent(),
                tr("Available Classes"),
                choices,
                move |name| {
                    if let Some(slots) = groups_of(&name) {
                        entries.borrow_mut().push(Allowed::new(slots, 0));
                        render();
                    }
                },
            );
        }
    });
    add_multi.connect_clicked({
        let (host, entries, render, class_list, groups_of) = (
            host.clone(),
            entries.clone(),
            render.clone(),
            class_list.clone(),
            groups_of.clone(),
        );
        move |_| {
            if dual {
                return message(
                    &host.parent(),
                    tr("Multi-Classes"),
                    tr(allowed::DUAL_AND_MULTI),
                );
            }
            let taken: Vec<String> = entries.borrow().iter().map(|a| a.name.clone()).collect();
            let choices: Vec<String> = classes::multiclass_names(&class_list)
                .into_iter()
                .filter(|n| !taken.contains(n))
                .collect();
            let (entries, render, groups_of) = (entries.clone(), render.clone(), groups_of.clone());
            pick(&host.parent(), tr("Multi-Classes"), choices, move |name| {
                if let Some(slots) = groups_of(&name) {
                    entries.borrow_mut().push(Allowed::new(slots, 0));
                    render();
                }
            });
        }
    });

    let page = adw::PreferencesPage::new();
    page.add(&group);
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
    crate::help::add_button(&header, &view, || topic::CLASSES_ALLOWED);
    view.add_top_bar(&header);
    view.set_content(Some(&page));
    let dialog = dialog_for(&tr!("Allowed Classes for {race}", race), &view, false);
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    let race = race.to_owned();
    save.connect_clicked({
        let (host, dialog) = (host.clone(), dialog.clone());
        move |_| {
            // The dialog's list is sorted by name.
            let mut user: Vec<Allowed> = entries
                .borrow()
                .iter()
                .filter(|a| !a.standard)
                .cloned()
                .collect();
            user.sort_by_cached_key(|a| cr_store::collate::sort_key(&a.name));
            let result = host.with_install(|install| {
                let mut db = AllowedDb::load(install)?;
                db.set_user(&race, &user);
                db.save(install)
            });
            if errors(&*host, result).is_some() {
                host.toast(tr("Allowed classes saved"));
                dialog.close();
            }
        }
    });
    dialog.present(Some(&host.parent()));
}

// ---- detail page and list ----

/// The buttons the detail page shows for standard and custom records:
/// Copy, and a menu with New, Delete and the race and table extras.
#[derive(Clone)]
pub struct DetailActions {
    copy: gtk::Button,
    menu: gtk::MenuButton,
}

impl DetailActions {
    pub fn new() -> Self {
        let copy = gtk::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text(tr("Copy"))
            .action_name("win.udb-copy")
            .visible(false)
            .build();
        let menu = gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .tooltip_text(tr("More Actions"))
            .visible(false)
            .build();
        Self { copy, menu }
    }

    pub fn pack(&self, header: &adw::HeaderBar) {
        header.pack_end(&self.menu);
        header.pack_end(&self.copy);
    }

    /// Shows the actions for the entry with `key`.
    pub fn update(&self, key: &str) {
        let Some((section, origin, _)) = udb::parse_key(key) else {
            self.copy.set_visible(false);
            self.menu.set_visible(false);
            return;
        };
        let kind = kind_of(section);
        self.copy.set_visible(true);
        let menu = gio::Menu::new();
        menu.append(Some(new_menu_label(kind)), Some("win.udb-new"));
        if kind == Some(Kind::Races) {
            menu.append(Some(tr("New _Subrace…")), Some("win.udb-subrace"));
            let item = gio::MenuItem::new(Some(tr("_Allowed Classes…")), Some("win.udb-allowed"));
            item.set_attribute_value("hidden-when", Some(&"action-disabled".to_variant()));
            menu.append_item(&item);
        }
        let tables = kind == Some(Kind::Tables);
        if tables && origin == Origin::Standard {
            menu.append(Some(tr("_Restore")), Some("win.udb-restore"));
        }
        if origin == Origin::Custom {
            let section = gio::Menu::new();
            section.append(Some(tr("_Delete")), Some("win.udb-delete"));
            menu.append_section(None, &section);
        }
        self.menu.set_menu_model(Some(&menu));
        self.menu.set_visible(true);
    }
}

/// The All / Standard / Custom filter of the entry list.
pub fn origin_filter() -> gtk::DropDown {
    gtk::DropDown::builder()
        .model(&gtk::StringList::new(&Filter::NAMES.map(tr)))
        .tooltip_text(tr("Show Standard or Custom Records"))
        .visible(false)
        .build()
}

/// Whether the filter shows an entry from `origin`.
pub fn passes(filter: &gtk::DropDown, origin: Option<Origin>) -> bool {
    match origin {
        Some(origin) => Filter::of(filter.selected()).accepts(origin),
        None => true,
    }
}
