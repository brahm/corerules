//! Encounters: generating one from a template or a 2-20 table, the
//! Encounter dialog (monsters, NPCs, treasure, items, traps and totals),
//! and the 2-20 table editor.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::{tr, tr_noop, trn};
use cr_store::encounter::{
    Encounter, EncounterRules, FrequencyTable, LairMode, NameError, PICK_ROLLS, Package,
    PendingNpc, Source, TableFilter, add_trap, check_name, hit_point_text, remove_trap,
    roll_2_to_20, roll_and_pick,
};
use cr_store::help::topic;
use cr_store::sheet::Sheet;
use cr_store::treasure::{Bundle, Rand};
use cr_store::userdb::traps::Traps;
use cr_store::userdb::{Filter, Origin, UserDb};
use cr_store::{Installation, Result};
use gtk::{gio, glib};

use crate::treasure;

/// What the encounter dialogs need from the window.
pub trait Host {
    fn with_install<T>(&self, run: impl FnOnce(&Installation) -> Result<T>) -> Option<Result<T>>;
    fn rand(&self) -> Rc<RefCell<Rand>>;
    fn toast(&self, message: &str);
    /// Reloads the sections and shows the entry `key` of section `section`.
    fn reload(&self, section: &str, key: Option<String>);
    /// How the treasure roller saves and deletes bundles.
    fn treasure_store(&self) -> treasure::Store;
    fn parent(&self) -> gtk::Widget;
    /// New ("new"), Copy, Edit or Delete on a record of the user
    /// database's `section` ("traps" or "monsters").
    fn user_record(&self, section: &str, action: &str, origin: Origin, name: &str);
    /// New on `section`; `saved` gets the name of the record once saved.
    fn new_user_record(&self, section: &str, saved: Box<dyn Fn(&str)>);
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

/// Generates an encounter from the template `name` and opens it.
pub fn create_from_template<H: Host + 'static>(host: &Shared<H>, name: &str) {
    let Some(rules) = errors(&**host, host.with_install(|i| i.encounter_rules())) else {
        return;
    };
    let generated = rules.generate(name, &mut host.rand().borrow_mut());
    match generated {
        Ok(g) => {
            for message in &g.messages {
                host.toast(message);
            }
            editor(host, Rc::new(rules), g.encounter, None, g.pending_npcs);
        }
        Err(e) => {
            // As the original: "Creating an Empty Encounter".
            host.toast(&tr!("{error}. Creating an empty encounter.", error = e));
            editor(host, Rc::new(rules), Encounter::default(), None, Vec::new());
        }
    }
}

/// Roll and Generate on a saved 2-20 table: a uniformly drawn filled roll.
pub fn roll_and_generate<H: Host + 'static>(host: &Shared<H>, table: &str) {
    let Some((rules, tables)) = errors(
        &**host,
        host.with_install(|i| Ok((i.encounter_rules()?, i.frequency_tables()?))),
    ) else {
        return;
    };
    let Some(table) = tables.table(table) else {
        return;
    };
    let pick = roll_and_pick(&rules, table, &mut host.rand().borrow_mut());
    match pick {
        Some(i) => {
            host.toast(&tr!(
                "Rolled {roll}: {template}",
                roll = i + 2,
                template = table.slots[i]
            ));
            create_from_template(host, &table.slots[i].clone());
        }
        None => host.toast(tr("The table has no templates; fill it first")),
    }
}

/// Opens the saved encounter `name`.
pub fn open_saved<H: Host + 'static>(host: &Shared<H>, name: &str) {
    open(host, name, false);
}

/// The original's Copy on the Encounters tab (0x694117): the saved
/// encounter `name` opened with no name, so that OK adds a new one once
/// it is named.
pub fn copy_saved<H: Host + 'static>(host: &Shared<H>, name: &str) {
    open(host, name, true);
}

fn open<H: Host + 'static>(host: &Shared<H>, name: &str, copy: bool) {
    let Some((rules, saved)) = errors(
        &**host,
        host.with_install(|i| Ok((i.encounter_rules()?, i.saved_encounters()?))),
    ) else {
        return;
    };
    let Some(mut encounter) = saved.into_iter().find(|e| e.name == name) else {
        return;
    };
    if copy {
        encounter.name.clear();
    }
    let saved = (!copy).then(|| name.to_owned());
    editor(host, Rc::new(rules), encounter, saved, Vec::new());
}

/// New Encounter: the original's "Select an Encounter Template", with an
/// empty encounter first.
pub fn choose_template<H: Host + 'static>(host: &Shared<H>) {
    let Some(rules) = errors(&**host, host.with_install(|i| i.encounter_rules())) else {
        return;
    };
    let mut names: Vec<(String, String)> = rules
        .templates()
        .iter()
        .map(|t| (t.name.clone(), t.monster_type.clone()))
        .collect();
    names.sort_by_cached_key(|(n, _)| cr_store::collate::sort_key(n));
    let search = gtk::SearchEntry::builder()
        .placeholder_text(tr("Search Templates"))
        .hexpand(true)
        .build();
    let list = gtk::ListBox::builder()
        .css_classes(["boxed-list"])
        .selection_mode(gtk::SelectionMode::None)
        .build();
    let empty = adw::ActionRow::builder()
        .title(tr("Empty Encounter"))
        .subtitle(tr("Add monsters and characters yourself"))
        .activatable(true)
        .build();
    list.append(&empty);
    let rows: Vec<(adw::ActionRow, String)> = names
        .iter()
        .map(|(name, kind)| {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(name).as_str())
                .subtitle(glib::markup_escape_text(kind).as_str())
                .activatable(true)
                .build();
            list.append(&row);
            (row, format!("{name}\n{kind}").to_lowercase())
        })
        .collect();
    search.connect_search_changed({
        let rows = rows.clone();
        let empty = empty.clone();
        move |s| {
            let needle = s.text().to_lowercase();
            empty.set_visible(needle.is_empty());
            for (row, text) in &rows {
                row.set_visible(text.contains(&needle));
            }
        }
    });
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&list);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&adw::Clamp::builder().child(&content).build())
        .build();
    let header = adw::HeaderBar::new();
    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, || topic::ENCOUNTERS);
    // The search holds the focus.
    crate::help::add_key(&search, || topic::ENCOUNTERS);
    view.add_top_bar(&header);
    let bar = gtk::SearchBar::builder()
        .child(&search)
        .search_mode_enabled(true)
        .build();
    view.add_top_bar(&bar);
    view.set_content(Some(&scroll));
    let dialog = adw::Dialog::builder()
        .title(tr("New Encounter"))
        .content_width(560)
        .content_height(640)
        .width_request(360)
        .height_request(294)
        .child(&view)
        .build();
    let rules = Rc::new(rules);
    list.connect_row_activated({
        let (host, dialog, rules) = (host.clone(), dialog.clone(), rules.clone());
        move |_, row| {
            dialog.close();
            if row.index() == 0 {
                editor(&host, rules.clone(), Encounter::default(), None, Vec::new());
            } else {
                let name = &names[row.index() as usize - 1].0;
                create_from_template(&host, name);
            }
        }
    });
    dialog.present(Some(&host.parent()));
    search.grab_focus();
}

/// Group Encounter (0x44e30b): asks whether to make an encounter of the
/// NPCs of `party` (Chars/ files `members`, in order) or to print them as
/// one, then opens the Encounter dialog on it, or prints it.
pub fn group_encounter<H: Host + 'static>(
    host: &Shared<H>,
    parent: &gtk::Widget,
    party: &str,
    members: Vec<String>,
) {
    let create = gtk::CheckButton::builder().active(true).build();
    let print = gtk::CheckButton::builder().group(&create).build();
    let choice = |button: &gtk::CheckButton, title: &str| {
        let row = adw::ActionRow::builder()
            .title(title)
            .use_underline(true)
            .activatable_widget(button)
            .build();
        row.add_prefix(button);
        row
    };
    let condensed = adw::SwitchRow::builder()
        .title(tr("Co_ndensed Format"))
        .subtitle(tr("One stat line for each NPC"))
        .use_underline(true)
        .sensitive(false)
        .build();
    let preview = adw::SwitchRow::builder()
        .title(tr("Print P_review"))
        .use_underline(true)
        .sensitive(false)
        .build();
    print.connect_toggled({
        let (condensed, preview) = (condensed.clone(), preview.clone());
        move |b| {
            condensed.set_sensitive(b.is_active());
            preview.set_sensitive(b.is_active());
        }
    });
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    list.append(&choice(&create, tr("_Create and Save NPC Group Encounter")));
    list.append(&choice(&print, tr("_Print Group Encounter")));
    list.append(&condensed);
    list.append(&preview);
    let alert = adw::AlertDialog::builder()
        .heading(tr("Group Encounter"))
        .body(tr!(
            "Create or print an encounter of the NPCs of “{party}”.",
            party
        ))
        .extra_child(&list)
        .default_response("ok")
        .close_response("cancel")
        .build();
    alert.add_responses(&[("cancel", tr("_Cancel")), ("ok", tr("_OK"))]);
    alert.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
    let (host, party, under) = (host.clone(), party.to_owned(), parent.clone());
    alert.connect_response(Some("ok"), move |_, _| {
        let loaded = host.with_install(|install| {
            let members = members
                .iter()
                .map(|file| install.character(file))
                .collect::<Result<Vec<_>>>()?;
            Ok((install.encounter_rules()?, members))
        });
        let Some((rules, members)) = errors(&*host, loaded) else {
            return;
        };
        let mut encounter = Encounter::of_group(members);
        if create.is_active() {
            // The name is left for the user to give (0x44e759).
            editor(&host, Rc::new(rules), encounter, None, Vec::new());
            return;
        }
        encounter.name = party.clone();
        let sheet = rules.group_sheet(&encounter, condensed.is_active());
        let toast = {
            let host = host.clone();
            move |m: &str| host.toast(m)
        };
        if preview.is_active() {
            crate::sheet::preview(&under, sheet, toast);
        } else {
            crate::sheet::print_sheet(&under, &Rc::new(sheet), None, &toast);
        }
    });
    alert.present(Some(parent));
}

struct State<H: Host> {
    host: Shared<H>,
    rules: Rc<EncounterRules>,
    /// The databases Add Monster offers, read again after New Monster.
    monster_rules: RefCell<Rc<EncounterRules>>,
    /// New Monster was used since: Add Monster reads the databases again.
    monster_rules_stale: Cell<bool>,
    encounter: RefCell<Encounter>,
    /// The name the encounter is saved under.
    saved: RefCell<Option<String>>,
    pending: Vec<PendingNpc>,
    page: adw::PreferencesPage,
    /// The groups rebuilt when the encounter changes.
    groups: RefCell<Vec<adw::PreferencesGroup>>,
    notes: adw::PreferencesGroup,
    /// Gold Value, Monster XP and Treasure XP.
    totals: Vec<gtk::Label>,
    dialog: adw::Dialog,
    delete: std::cell::OnceCell<gio::SimpleAction>,
    /// Editing the items.
    items: std::cell::OnceCell<crate::bundle::Edit>,
}

/// The Encounter dialog.
pub fn editor<H: Host + 'static>(
    host: &Shared<H>,
    rules: Rc<EncounterRules>,
    encounter: Encounter,
    saved: Option<String>,
    pending: Vec<PendingNpc>,
) {
    let name = adw::EntryRow::builder()
        .title(tr("Name"))
        .text(&encounter.name)
        .build();
    let template = adw::ActionRow::builder()
        .title(tr("Template"))
        .subtitle(if encounter.template.is_empty() {
            tr("None").to_owned()
        } else {
            glib::markup_escape_text(&encounter.template).to_string()
        })
        .css_classes(["property"])
        .build();
    let kind = adw::EntryRow::builder()
        .title(tr("Monster Type"))
        .text(&encounter.monster_type)
        .build();
    let about = adw::PreferencesGroup::builder()
        .title(tr("Encounter"))
        .build();
    about.add(&name);
    about.add(&template);
    about.add(&kind);
    let description = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(12)
        .bottom_margin(12)
        .left_margin(12)
        .right_margin(12)
        .height_request(100)
        .css_classes(["card"])
        .build();
    description.buffer().set_text(&encounter.description);
    let notes = adw::PreferencesGroup::builder()
        .title(tr("Description"))
        .build();
    notes.add(&description);

    let totals_group = adw::PreferencesGroup::builder()
        .title(tr("Encounter Totals"))
        .build();
    let totals: Vec<gtk::Label> = [tr("Gold Value"), tr("Monster XP"), tr("Treasure XP")]
        .iter()
        .map(|title| {
            let row = adw::ActionRow::builder().title(*title).build();
            let label = gtk::Label::builder()
                .css_classes(["dim-label", "numeric"])
                .build();
            row.add_suffix(&label);
            totals_group.add(&row);
            label
        })
        .collect();
    let page = adw::PreferencesPage::new();
    page.add(&about);
    page.add(&totals_group);

    let save = gtk::Button::builder()
        .label(tr("_Save"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let menu = gio::Menu::new();
    let print_section = gio::Menu::new();
    print_section.append(Some(tr("_Print…")), Some("encounter.print"));
    print_section.append(Some(tr("Save as P_DF…")), Some("encounter.pdf"));
    menu.append_section(None, &print_section);
    let delete_section = gio::Menu::new();
    delete_section.append(Some(tr("_Delete Encounter")), Some("encounter.delete"));
    menu.append_section(None, &delete_section);
    let header = adw::HeaderBar::new();
    header.pack_end(
        &gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .tooltip_text(tr("Encounter Menu"))
            .menu_model(&menu)
            .build(),
    );
    header.pack_end(&save);
    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, || topic::ENCOUNTER);
    view.add_top_bar(&header);
    view.set_content(Some(&page));
    let dialog = adw::Dialog::builder()
        .title(saved.as_deref().unwrap_or(tr("New Encounter")))
        .content_width(760)
        .content_height(760)
        .width_request(360)
        .height_request(294)
        .child(&view)
        .build();

    let state = Rc::new(State {
        host: host.clone(),
        monster_rules: RefCell::new(rules.clone()),
        rules,
        monster_rules_stale: Cell::new(false),
        encounter: RefCell::new(encounter),
        saved: RefCell::new(saved),
        pending,
        page,
        groups: RefCell::default(),
        notes,
        totals,
        dialog: dialog.clone(),
        delete: std::cell::OnceCell::new(),
        items: std::cell::OnceCell::new(),
    });
    // Items… (0x63e1a2) edits the items in place with the Treasure Bundle
    // Editor, then the totals are counted again.
    let _ = state.items.set(Rc::new(crate::bundle::Editing {
        rules: Rc::new(state.rules.treasure_rules().clone()),
        parent: dialog.clone().upcast(),
        get: Box::new({
            let state = Rc::downgrade(&state);
            move || Some(state.upgrade()?.encounter.borrow().items.clone())
        }),
        set: Box::new({
            let state = Rc::downgrade(&state);
            move |bundle| {
                if let Some(state) = state.upgrade() {
                    state.encounter.borrow_mut().items = bundle;
                    state.refresh();
                }
            }
        }),
    }));
    state.refresh();

    name.connect_changed({
        let state = state.clone();
        move |e| state.encounter.borrow_mut().name = e.text().to_string()
    });
    kind.connect_changed({
        let state = state.clone();
        move |e| state.encounter.borrow_mut().monster_type = e.text().to_string()
    });
    description.buffer().connect_changed({
        let state = state.clone();
        move |b| {
            state.encounter.borrow_mut().description =
                b.text(&b.start_iter(), &b.end_iter(), false).to_string()
        }
    });
    save.connect_clicked({
        let state = state.clone();
        move |_| state.save()
    });

    let actions = gio::SimpleActionGroup::new();
    let add = |name: &str, run: Box<dyn Fn()>| -> gio::SimpleAction {
        let a = gio::SimpleAction::new(name, None);
        a.connect_activate(move |_, _| run());
        actions.add_action(&a);
        a
    };
    let toast = {
        let host = host.clone();
        move |m: &str| host.toast(m)
    };
    add("print", {
        let (state, toast) = (state.clone(), toast.clone());
        Box::new(move || {
            let sheet = Rc::new(state.sheet());
            crate::sheet::print_sheet(&state.dialog, &sheet, None, &toast)
        })
    });
    add("pdf", {
        let state = state.clone();
        Box::new(move || {
            let sheet = Rc::new(state.sheet());
            let (dialog, toast) = (state.dialog.clone(), toast.clone());
            let title = sheet.title.clone();
            crate::sheet::save_file(
                &state.dialog,
                &title,
                "pdf",
                tr("PDF documents"),
                move |path| crate::sheet::print_sheet(&dialog, &sheet, Some(path), &toast),
            );
        })
    });
    let delete = add("delete", {
        let state = state.clone();
        Box::new(move || state.confirm_delete())
    });
    delete.set_enabled(state.saved.borrow().is_some());
    let _ = state.delete.set(delete);
    dialog.insert_action_group("encounter", Some(&actions));
    dialog.present(Some(&host.parent()));
}

impl<H: Host + 'static> State<H> {
    /// Rebuilds the groups that show the encounter's contents, keeping
    /// the description last.
    fn refresh(self: &Rc<Self>) {
        // Keep the place in the page: rebuilding would move the focus to
        // the first entry and scroll there.
        let scroll = self
            .page
            .first_child()
            .and_downcast::<gtk::ScrolledWindow>()
            .map(|s| s.vadjustment());
        let at = scroll.as_ref().map(|a| a.value());
        self.dialog.set_focus(None::<&gtk::Widget>);
        for group in self.groups.borrow_mut().drain(..) {
            self.page.remove(&group);
        }
        if self.notes.parent().is_some() {
            self.page.remove(&self.notes);
        }
        self.update_totals();
        let groups: Vec<adw::PreferencesGroup> = [
            self.packages_group(),
            self.pending_group(),
            self.treasure_group(),
            self.items_group(),
            self.traps_group(),
        ]
        .into_iter()
        .flatten()
        .collect();
        for group in &groups {
            self.page.add(group);
        }
        self.page.add(&self.notes);
        *self.groups.borrow_mut() = groups;
        if let (Some(adjustment), Some(at)) = (scroll, at) {
            glib::idle_add_local_once(move || adjustment.set_value(at));
        }
    }

    fn update_totals(&self) {
        let t = self.rules.totals(&self.encounter.borrow());
        for (label, value) in self
            .totals
            .iter()
            .zip([t.gold_value, t.monster_xp, t.treasure_xp])
        {
            label.set_label(&cr_store::treasure::thousands(value));
        }
    }

    fn packages_group(self: &Rc<Self>) -> Option<adw::PreferencesGroup> {
        let add_monster = gtk::Button::builder()
            .label(tr("Add _Monster…"))
            .use_underline(true)
            .css_classes(["flat"])
            .build();
        let add_npc = gtk::Button::builder()
            .label(tr("Add _NPC…"))
            .use_underline(true)
            .css_classes(["flat"])
            .build();
        let buttons = gtk::Box::builder().spacing(6).build();
        buttons.append(&add_monster);
        buttons.append(&add_npc);
        add_monster.connect_clicked({
            let state = self.clone();
            move |_| state.choose_monster()
        });
        add_npc.connect_clicked({
            let state = self.clone();
            move |_| state.choose_npc()
        });
        let group = adw::PreferencesGroup::builder()
            .title(tr("Monsters and Nonplayer Characters"))
            .header_suffix(&buttons)
            .build();
        let encounter = self.encounter.borrow();
        if encounter.monsters.is_empty() && encounter.npcs.is_empty() {
            group.add(
                &adw::ActionRow::builder()
                    .title(tr("No monsters or characters"))
                    .css_classes(["dim-label"])
                    .build(),
            );
        }
        for package in encounter.packages() {
            let (title, subtitle, stats, count) = match package {
                Package::Monster(i) => {
                    let p = &encounter.monsters[i];
                    (
                        format!("{} × {}", p.count, p.monster.name),
                        tr!(
                            "Hit points {hit_points}",
                            hit_points = hit_point_text(&p.hit_points)
                        ),
                        self.rules.stat_line(p),
                        p.count,
                    )
                }
                Package::Npc(i) => {
                    let p = &encounter.npcs[i];
                    (
                        format!("{} × {}", p.count, self.rules.npc_name(p).trim()),
                        tr("Nonplayer character").to_owned(),
                        self.rules.npc_stat_line(p),
                        p.count,
                    )
                }
            };
            let row = adw::ExpanderRow::builder()
                .title(glib::markup_escape_text(&title).as_str())
                .subtitle(glib::markup_escape_text(&subtitle).as_str())
                .build();
            let remove = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .tooltip_text(tr("Remove"))
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .build();
            remove.connect_clicked({
                let state = self.clone();
                move |_| {
                    state.encounter.borrow_mut().remove(package);
                    state.refresh();
                }
            });
            row.add_suffix(&remove);
            if let Package::Npc(i) = package {
                // View NPC… and Print NPC… (0x63EA49, 0x63EB3A): the
                // NPC's character sheet. Each suffix goes before the last.
                for (icon, tip, print) in [
                    ("printer-symbolic", tr("Print NPC"), true),
                    ("document-print-preview-symbolic", tr("View NPC"), false),
                ] {
                    let button = gtk::Button::builder()
                        .icon_name(icon)
                        .tooltip_text(tip)
                        .valign(gtk::Align::Center)
                        .css_classes(["flat"])
                        .build();
                    button.connect_clicked({
                        let state = self.clone();
                        move |_| state.npc_sheet(i, print)
                    });
                    row.add_suffix(&button);
                }
            }
            let number = adw::SpinRow::builder()
                .title(tr("Number"))
                .subtitle(if matches!(package, Package::Monster(_)) {
                    tr("Changing it rolls the hit points again")
                } else {
                    ""
                })
                .adjustment(&gtk::Adjustment::new(
                    count as f64,
                    1.0,
                    100_000_000.0,
                    1.0,
                    10.0,
                    0.0,
                ))
                .build();
            let stat = gtk::Label::builder()
                .label(&stats)
                .wrap(true)
                .wrap_mode(gtk::pango::WrapMode::WordChar)
                .xalign(0.0)
                .selectable(true)
                .margin_top(12)
                .margin_bottom(12)
                .margin_start(12)
                .margin_end(12)
                .build();
            // The Hit Points group of "Monsters for the Encounter" (IDD
            // 292): a value, how many monsters have it, and Re-roll.
            let hit_points = adw::SpinRow::builder()
                .title(tr("Hit Points"))
                .adjustment(&gtk::Adjustment::new(0.0, 0.0, 0.0, 1.0, 10.0, 0.0))
                .build();
            let with = adw::SpinRow::builder()
                .title(tr("Monsters"))
                .adjustment(&gtk::Adjustment::new(0.0, 0.0, 0.0, 1.0, 10.0, 0.0))
                .build();
            let reroll = gtk::Button::builder()
                .label(tr("_Re-roll"))
                .use_underline(true)
                .tooltip_text(tr("Roll the hit points of every monster again"))
                .valign(gtk::Align::Center)
                .build();
            let filling = Rc::new(Cell::new(false));
            // Shows the package as it now is (0x64f8c9).
            let show = {
                let (state, row, stat) = (self.clone(), row.clone(), stat.clone());
                let (hit_points, with, filling) =
                    (hit_points.clone(), with.clone(), filling.clone());
                move || {
                    let Package::Monster(i) = package else {
                        return;
                    };
                    let encounter = state.encounter.borrow();
                    let Some(p) = encounter.monsters.get(i) else {
                        return;
                    };
                    row.set_title(&glib::markup_escape_text(&format!(
                        "{} × {}",
                        p.count, p.monster.name
                    )));
                    row.set_subtitle(&glib::markup_escape_text(&tr!(
                        "Hit points {hit_points}",
                        hit_points = hit_point_text(&p.hit_points)
                    )));
                    stat.set_label(&state.rules.stat_line(p));
                    filling.set(true);
                    let (low, high) = p.hit_point_range();
                    let value = hit_points.value() as i32;
                    let value = if (low..=high).contains(&value) {
                        value
                    } else {
                        low
                    };
                    hit_points.adjustment().set_lower(low.into());
                    hit_points.adjustment().set_upper(high.into());
                    hit_points.set_value(value.into());
                    with.adjustment().set_upper(p.most_with(value).into());
                    with.set_value(p.count_with(value).into());
                    with.set_subtitle(&tr!(
                        "With these hit points; {count} unassigned",
                        count = p.unassigned().max(0)
                    ));
                    filling.set(false);
                }
            };
            show();
            number.connect_value_notify({
                let (state, show) = (self.clone(), show.clone());
                move |spin| {
                    let count = spin.value() as i32;
                    let rand = state.host.rand();
                    state.encounter.borrow_mut().set_count(
                        package,
                        count,
                        &state.rules,
                        &mut rand.borrow_mut(),
                    );
                    show();
                    state.update_totals();
                }
            });
            hit_points.connect_value_notify({
                let (show, filling) = (show.clone(), filling.clone());
                move |_| {
                    if !filling.get() {
                        show();
                    }
                }
            });
            with.connect_value_notify({
                let (state, show, filling) = (self.clone(), show.clone(), filling.clone());
                let hit_points = hit_points.clone();
                move |spin| {
                    if filling.get() {
                        return;
                    }
                    if let Package::Monster(i) = package
                        && let Some(p) = state.encounter.borrow_mut().monsters.get_mut(i)
                    {
                        p.set_count_with(hit_points.value() as i32, spin.value() as i32);
                    }
                    show();
                }
            });
            reroll.connect_clicked({
                let (state, show) = (self.clone(), show.clone());
                move |_| {
                    if let Package::Monster(i) = package {
                        let rand = state.host.rand();
                        if let Some(p) = state.encounter.borrow_mut().monsters.get_mut(i) {
                            state.rules.roll_hit_points(p, &mut rand.borrow_mut());
                        }
                    }
                    show();
                }
            });
            hit_points.add_suffix(&reroll);
            row.add_row(&number);
            if matches!(package, Package::Monster(_)) {
                row.add_row(&hit_points);
                row.add_row(&with);
            }
            row.add_row(&stat);
            group.add(&row);
        }
        Some(group)
    }

    fn pending_group(&self) -> Option<adw::PreferencesGroup> {
        if self.pending.is_empty() {
            return None;
        }
        let group = adw::PreferencesGroup::builder()
            .title(tr("Characters Not Generated"))
            .description(tr(
                "The template also calls for these nonplayer characters, but the NPC \
                 generator could not make them, for the reason under each one. Add \
                 saved characters with Add NPC instead.",
            ))
            .build();
        for p in &self.pending {
            let classes: Vec<String> = p
                .npc
                .classes
                .iter()
                .map(|(c, l)| format!("{c} {l}"))
                .collect();
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&format!("{} × {}", p.count, p.npc.race)).as_str())
                .subtitle(
                    glib::markup_escape_text(&format!("{}\n{}", classes.join(" / "), p.reason))
                        .as_str(),
                )
                .build();
            group.add(&row);
        }
        Some(group)
    }

    fn treasure_group(self: &Rc<Self>) -> Option<adw::PreferencesGroup> {
        let roll = gtk::Button::builder()
            .label(tr("_Roll Treasure…"))
            .use_underline(true)
            .css_classes(["flat"])
            .build();
        roll.connect_clicked({
            let state = self.clone();
            move |_| state.open_roller()
        });
        let encounter = self.encounter.borrow();
        let summary = self
            .rules
            .treasure_rules()
            .summary_with(&encounter.treasure, encounter.prices());
        let group = adw::PreferencesGroup::builder()
            .title(tr("Treasure"))
            .header_suffix(&roll)
            .build();
        if encounter.treasure.is_empty() {
            group.set_description(Some(tr(
                "Roll the treasure of the monsters' treasure types and the lair",
            )));
        } else {
            group.set_description(Some(&tr!(
                "{value} gp",
                value = cr_store::treasure::thousands(summary.value_cp / 100)
            )));
        }
        bundle_rows(&group, &encounter.treasure);
        Some(group)
    }

    fn items_group(&self) -> Option<adw::PreferencesGroup> {
        let edit = self.items.get()?;
        let encounter = self.encounter.borrow();
        let group = adw::PreferencesGroup::builder()
            .title(tr("Items"))
            .header_suffix(&crate::bundle::add_button(edit))
            .build();
        if encounter.items.is_empty() {
            group.set_description(Some(tr("Items found with the encounter")));
        }
        for row in crate::bundle::rows(edit, &encounter.items) {
            group.add(&row);
        }
        Some(group)
    }

    /// Traps (IDD 295): the encounter's own copies of traps from
    /// `Database` and `UserDbas/TrapsU.dat`.
    fn traps_group(self: &Rc<Self>) -> Option<adw::PreferencesGroup> {
        let add = gtk::Button::builder()
            .label(tr("_Add Trap…"))
            .use_underline(true)
            .css_classes(["flat"])
            .build();
        add.connect_clicked({
            let state = self.clone();
            move |_| state.choose_trap()
        });
        let encounter = self.encounter.borrow();
        let group = adw::PreferencesGroup::builder()
            .title(tr("Traps"))
            .header_suffix(&add)
            .build();
        if encounter.traps.is_empty() {
            group.set_description(Some(tr("No traps")));
        }
        for trap in &encounter.traps {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&trap.name).as_str())
                .subtitle(glib::markup_escape_text(&trap.damage).as_str())
                .build();
            let remove = gtk::Button::builder()
                .icon_name("list-remove-symbolic")
                .tooltip_text(tr("Remove Trap"))
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .build();
            remove.connect_clicked({
                let (state, name) = (self.clone(), trap.name.clone());
                move |_| {
                    let removed = remove_trap(&mut state.encounter.borrow_mut().traps, &name);
                    match removed {
                        Ok(()) => state.refresh(),
                        Err(e) => state.host.toast(&e.to_string()),
                    }
                }
            });
            row.add_suffix(&remove);
            group.add(&row);
        }
        Some(group)
    }

    /// The Traps dialog's lists: every trap, filtered by origin, to add to
    /// the encounter; New, Edit, Copy and Delete change the user's traps.
    fn choose_trap(self: &Rc<Self>) {
        let Some(db) = errors(&*self.host, self.host.with_install(UserDb::<Traps>::load)) else {
            return;
        };
        let db = Rc::new(db);
        let filter = gtk::DropDown::builder()
            .model(&gtk::StringList::new(&Filter::NAMES.map(tr)))
            .tooltip_text(tr("Show Standard or Custom Traps"))
            .build();
        let new = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text(tr("New Trap"))
            .build();
        let list = gtk::ListBox::builder()
            .css_classes(["boxed-list"])
            .selection_mode(gtk::SelectionMode::None)
            .build();
        let header = adw::HeaderBar::new();
        header.pack_start(&filter);
        header.pack_end(&new);
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(12)
            .margin_bottom(24)
            .margin_start(12)
            .margin_end(12)
            .build();
        content.append(&list);
        let view = adw::ToolbarView::new();
        crate::help::add_button(&header, &view, || topic::TRAPS);
        view.add_top_bar(&header);
        view.set_content(Some(
            &gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .vexpand(true)
                .child(&adw::Clamp::builder().child(&content).build())
                .build(),
        ));
        let dialog = adw::Dialog::builder()
            .title(tr("Add Trap"))
            .content_width(480)
            .content_height(600)
            .width_request(360)
            .height_request(294)
            .child(&view)
            .build();
        let record = {
            let (state, dialog) = (self.clone(), dialog.clone());
            move |action: &'static str, origin: Origin, name: String| {
                let (state, dialog) = (state.clone(), dialog.clone());
                move || {
                    dialog.close();
                    state.host.user_record("traps", action, origin, &name);
                }
            }
        };
        new.connect_clicked({
            let (state, dialog) = (self.clone(), dialog.clone());
            move |_| {
                dialog.close();
                // The new trap goes in the encounter, as the original's.
                let state2 = state.clone();
                state.host.new_user_record(
                    "traps",
                    Box::new(move |name| {
                        let Some(db) = errors(
                            &*state2.host,
                            state2.host.with_install(UserDb::<Traps>::load),
                        ) else {
                            return;
                        };
                        let added = add_trap(&mut state2.encounter.borrow_mut().traps, &db, name);
                        match added {
                            Ok(()) => state2.refresh(),
                            Err(e) => state2.host.toast(&e.to_string()),
                        }
                    }),
                );
            }
        });
        let fill = {
            let (state, list, dialog, db) =
                (self.clone(), list.clone(), dialog.clone(), db.clone());
            move |filter: Filter| {
                while let Some(child) = list.first_child() {
                    list.remove(&child);
                }
                for entry in db.list(filter) {
                    let trap = entry.record;
                    let origin = entry.origin;
                    let row = adw::ActionRow::builder()
                        .title(glib::markup_escape_text(&trap.name).as_str())
                        .subtitle(glib::markup_escape_text(&trap.damage).as_str())
                        .activatable(true)
                        .build();
                    if origin == Origin::Custom {
                        row.add_prefix(&gtk::Image::from_icon_name("document-edit-symbolic"));
                    }
                    let menu = gio::Menu::new();
                    let actions = gio::SimpleActionGroup::new();
                    let mut entries = vec![("copy", tr("_Copy…"))];
                    if origin == Origin::Custom {
                        entries.insert(0, ("edit", tr("_Edit…")));
                        entries.push(("delete", tr("_Delete")));
                    }
                    for (action, label) in entries {
                        let a = gio::SimpleAction::new(action, None);
                        let run = record(action, origin, trap.name.clone());
                        a.connect_activate(move |_, _| run());
                        actions.add_action(&a);
                        menu.append(Some(label), Some(&format!("trap.{action}")));
                    }
                    let button = gtk::MenuButton::builder()
                        .icon_name("view-more-symbolic")
                        .valign(gtk::Align::Center)
                        .menu_model(&menu)
                        .css_classes(["flat"])
                        .build();
                    button.update_property(&[gtk::accessible::Property::Label(tr("Trap Menu"))]);
                    button.insert_action_group("trap", Some(&actions));
                    row.add_suffix(&button);
                    row.connect_activated({
                        let (state, dialog, db, name) =
                            (state.clone(), dialog.clone(), db.clone(), trap.name.clone());
                        move |_| {
                            let added =
                                add_trap(&mut state.encounter.borrow_mut().traps, &db, &name);
                            match added {
                                Ok(()) => {
                                    dialog.close();
                                    state.refresh();
                                }
                                Err(e) => {
                                    let alert = adw::AlertDialog::builder()
                                        .heading(tr("Unable to Add Trap"))
                                        .body(e.to_string())
                                        .default_response("ok")
                                        .close_response("ok")
                                        .build();
                                    alert.add_response("ok", tr("_OK"));
                                    alert.present(Some(&dialog));
                                }
                            }
                        }
                    });
                    list.append(&row);
                }
            }
        };
        fill(Filter::All);
        filter.connect_selected_notify(move |d| fill(Filter::of(d.selected())));
        dialog.present(Some(&self.dialog));
    }

    /// Treasure…: the roller with the encounter's treasure types; Use
    /// Treasure puts the bundle in the encounter.
    fn open_roller(self: &Rc<Self>) {
        let encounter = self.encounter.borrow();
        let mut counts = [0u32; 26];
        for (c, n) in counts.iter_mut().zip(encounter.treasure_counts()) {
            *c = n.max(0) as u32;
        }
        let state = self.clone();
        let host = self.host.clone();
        treasure::roller(
            &self.dialog,
            Rc::new(self.rules.treasure_rules().clone()),
            self.host.rand(),
            treasure::Start {
                counts,
                bundle: encounter.treasure.clone(),
            },
            treasure::Mode::Pick {
                use_bundle: Box::new(move |bundle: Bundle| {
                    state.encounter.borrow_mut().treasure = bundle;
                    state.refresh();
                }),
            },
            self.host.treasure_store(),
            move |m| host.toast(m),
        );
    }

    /// Add…: a monster from the database, with its number. After New
    /// Monster the databases are read again, so the new monster is there.
    fn choose_monster(self: &Rc<Self>) {
        if self.monster_rules_stale.get() {
            let Some(rules) = errors(&*self.host, self.host.with_install(|i| i.encounter_rules()))
            else {
                return;
            };
            *self.monster_rules.borrow_mut() = Rc::new(rules);
            self.monster_rules_stale.set(false);
        }
        let rules = self.monster_rules.borrow().clone();
        let mut names: Vec<String> = rules.monsters().iter().map(|m| m.name.clone()).collect();
        names.sort_by_cached_key(|n| cr_store::collate::sort_key(n));
        names.dedup();
        let state = self.clone();
        // Create New Monsters… (0x64f291): the custom monster editor; the
        // new monster goes in the encounter.
        let new = {
            let state = self.clone();
            move || {
                state.monster_rules_stale.set(true);
                let state2 = state.clone();
                state.host.new_user_record(
                    "monsters",
                    Box::new(move |name| state2.add_new_monster(name)),
                );
            }
        };
        pick_from(
            &self.dialog,
            tr("Add Monster"),
            tr("Search Monsters"),
            names,
            Some((tr("New Monster"), Box::new(new))),
            move |name| {
                let rand = state.host.rand();
                let package = rules.new_monster_package(&name, None, &mut rand.borrow_mut());
                if let Some(package) = package {
                    state.encounter.borrow_mut().monsters.push(package);
                    state.refresh();
                }
            },
        );
    }

    /// A monster just made with New Monster, read from the databases
    /// again, into the encounter.
    fn add_new_monster(self: &Rc<Self>, name: &str) {
        let Some(rules) = errors(&*self.host, self.host.with_install(|i| i.encounter_rules()))
        else {
            return;
        };
        let rules = Rc::new(rules);
        *self.monster_rules.borrow_mut() = rules.clone();
        self.monster_rules_stale.set(false);
        let rand = self.host.rand();
        let package = rules.new_monster_package(name, None, &mut rand.borrow_mut());
        if let Some(package) = package {
            self.encounter.borrow_mut().monsters.push(package);
            self.refresh();
        }
    }

    /// Add NPC…: a saved character.
    fn choose_npc(self: &Rc<Self>) {
        let Some(index) = errors(&*self.host, self.host.with_install(|i| i.characters())) else {
            return;
        };
        let files: Vec<(String, String)> = index
            .entries
            .iter()
            .map(|e| (e.name.clone(), e.file.clone()))
            .collect();
        let state = self.clone();
        pick_from(
            &self.dialog,
            tr("Add NPC"),
            tr("Search Characters"),
            files.iter().map(|(n, _)| n.clone()).collect(),
            None,
            move |name| {
                let Some((_, file)) = files.iter().find(|(n, _)| *n == name) else {
                    return;
                };
                let flavor = state.rules.flavor();
                let added = state.host.with_install(|i| {
                    let character = i.character(file)?;
                    state.encounter.borrow_mut().add_npc(character, flavor)
                });
                if errors(&*state.host, added).is_some() {
                    state.refresh();
                }
            },
        );
    }

    /// OK: checks the name, then adds or replaces the saved encounter.
    fn save(self: &Rc<Self>) {
        let encounter = self.encounter.borrow().clone();
        let old = self.saved.borrow().clone();
        let result = self.host.with_install(|install| {
            let mut list = install.saved_encounters()?;
            let except = old
                .as_ref()
                .and_then(|o| list.iter().position(|e| &e.name == o));
            check_name(&list, &encounter.name, except)
                .map_err(|e: NameError| cr_store::Error::Invalid(e.to_string()))?;
            match except {
                Some(i) => list[i] = encounter.clone(),
                None => list.push(encounter.clone()),
            }
            install.write_saved_encounters(&list)
        });
        if errors(&*self.host, result).is_some() {
            let name = encounter.name.clone();
            self.dialog.set_title(&name);
            *self.saved.borrow_mut() = Some(name.clone());
            if let Some(delete) = self.delete.get() {
                delete.set_enabled(true);
            }
            self.host.toast(tr("Encounter saved"));
            self.host.reload(
                "encounters",
                Some(format!("{}{name}", cr_store::catalog::ENCOUNTER_KEY)),
            );
        }
    }

    fn confirm_delete(self: &Rc<Self>) {
        let Some(name) = self.saved.borrow().clone() else {
            return;
        };
        let alert = adw::AlertDialog::builder()
            .heading(tr("Delete Encounter?"))
            .body(tr!(
                "“{name}” will be removed from the saved encounters",
                name
            ))
            .default_response("cancel")
            .close_response("cancel")
            .build();
        alert.add_responses(&[("cancel", tr("_Cancel")), ("delete", tr("_Delete"))]);
        alert.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        let state = self.clone();
        alert.connect_response(Some("delete"), move |_, _| {
            let result = state.host.with_install(|install| {
                let mut list = install.saved_encounters()?;
                list.retain(|e| e.name != name);
                install.write_saved_encounters(&list)
            });
            if errors(&*state.host, result).is_some() {
                state.host.toast(tr("Encounter deleted"));
                state.host.reload("encounters", None);
                state.dialog.close();
            }
        });
        alert.present(Some(&self.dialog));
    }

    /// Shows or prints the sheet of the encounter's NPC `i`.
    fn npc_sheet(&self, i: usize, print: bool) {
        let Some(character) = self
            .encounter
            .borrow()
            .npcs
            .get(i)
            .map(|p| p.character.clone())
        else {
            return;
        };
        let result = self
            .host
            .with_install(|install| install.npc_sheet(&character));
        let Some(sheet) = errors(&*self.host, result) else {
            return;
        };
        let toast = {
            let host = self.host.clone();
            move |m: &str| host.toast(m)
        };
        if print {
            crate::sheet::print_sheet(&self.dialog, &Rc::new(sheet), None, &toast);
        } else {
            crate::sheet::preview(&self.dialog, sheet, toast);
        }
    }

    /// The encounter as a printable page (the original's View/Print).
    fn sheet(&self) -> Sheet {
        self.rules.sheet(&self.encounter.borrow())
    }
}

/// A bundle's items as rows; an item holding others expands.
fn bundle_rows(group: &adw::PreferencesGroup, bundle: &Bundle) {
    for item in &bundle.items {
        let title = glib::markup_escape_text(&item.label()).to_string();
        let spells: Vec<String> = item
            .spells()
            .iter()
            .map(|s| format!("{} ({})", s.name, s.pages))
            .collect();
        let subtitle = glib::markup_escape_text(&spells.join(", ")).to_string();
        match &item.contents {
            Some(contents) => {
                let row = adw::ExpanderRow::builder()
                    .title(title)
                    .subtitle(subtitle)
                    .build();
                for inner in &contents.items {
                    row.add_row(
                        &adw::ActionRow::builder()
                            .title(glib::markup_escape_text(&inner.label()).as_str())
                            .build(),
                    );
                }
                group.add(&row);
            }
            None => group.add(
                &adw::ActionRow::builder()
                    .title(title)
                    .subtitle(subtitle)
                    .subtitle_lines(0)
                    .build(),
            ),
        }
    }
}

/// A searchable list of names; activating one closes it and calls `chosen`.
/// A picker's New button: its tooltip and what it opens.
type NewButton<'a> = (&'a str, Box<dyn Fn()>);

fn pick_from(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    placeholder: &str,
    names: Vec<String>,
    new: Option<NewButton>,
    chosen: impl Fn(String) + 'static,
) {
    let search = gtk::SearchEntry::builder()
        .placeholder_text(placeholder)
        .hexpand(true)
        .build();
    let list = gtk::ListBox::builder()
        .css_classes(["boxed-list"])
        .selection_mode(gtk::SelectionMode::None)
        .build();
    let rows: Vec<(adw::ActionRow, String)> = names
        .iter()
        .map(|name| {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(name).as_str())
                .activatable(true)
                .build();
            list.append(&row);
            (row, name.to_lowercase())
        })
        .collect();
    let empty = adw::StatusPage::builder()
        .title(tr("No Results"))
        .icon_name("edit-find-symbolic")
        .css_classes(["compact"])
        .visible(names.is_empty())
        .build();
    if names.is_empty() {
        empty.set_title(tr("Nothing to Add"));
    }
    search.connect_search_changed({
        let (rows, empty, list) = (rows.clone(), empty.clone(), list.clone());
        move |s| {
            let needle = s.text().to_lowercase();
            let mut shown = 0;
            for (row, text) in &rows {
                let show = text.contains(&needle);
                row.set_visible(show);
                shown += show as usize;
            }
            empty.set_visible(shown == 0);
            list.set_visible(shown > 0);
        }
    });
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&list);
    content.append(&empty);
    let view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let new_button = new.map(|(tooltip, run)| {
        let button = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text(tooltip)
            .build();
        header.pack_end(&button);
        (button, run)
    });
    view.add_top_bar(&header);
    view.add_top_bar(
        &gtk::SearchBar::builder()
            .child(&search)
            .search_mode_enabled(true)
            .build(),
    );
    view.set_content(Some(
        &gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&adw::Clamp::builder().child(&content).build())
            .build(),
    ));
    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(480)
        .content_height(600)
        .width_request(360)
        .height_request(294)
        .child(&view)
        .build();
    if let Some((button, run)) = new_button {
        let dialog = dialog.clone();
        button.connect_clicked(move |_| {
            dialog.close();
            run();
        });
    }
    list.connect_row_activated({
        let dialog = dialog.clone();
        move |_, row| {
            let name = names[row.index() as usize].clone();
            dialog.close();
            chosen(name);
        }
    });
    dialog.present(Some(parent));
    search.grab_focus();
}

/// The frequency categories of a roll's list (0x6559bd).
const FREQUENCIES: [&str; 5] = [
    tr_noop!("Common"),
    tr_noop!("Uncommon"),
    tr_noop!("Rare"),
    tr_noop!("Very Rare"),
    tr_noop!("Never"),
];

/// Picking roll `index` + 2 in the table dialog (0x6556a9): the templates of
/// one frequency, the roll's own first ([`PICK_ROLLS`]), that pass
/// `filter`. `chosen` gets the name, or "" for (None).
fn pick_for_roll(
    parent: &impl IsA<gtk::Widget>,
    index: usize,
    rules: &Rc<EncounterRules>,
    filter: TableFilter,
    chosen: impl Fn(String) + 'static,
) {
    let frequency = adw::ComboRow::builder()
        .title(tr("Frequency"))
        .model(&gtk::StringList::new(&FREQUENCIES.map(tr)))
        .selected(PICK_ROLLS[index] as u32)
        .build();
    let top = gtk::ListBox::builder()
        .css_classes(["boxed-list"])
        .selection_mode(gtk::SelectionMode::None)
        .build();
    top.append(&frequency);
    let none = adw::ActionRow::builder()
        .title(tr("(None)"))
        .activatable(true)
        .build();
    top.append(&none);
    let list = gtk::ListBox::builder()
        .css_classes(["boxed-list"])
        .selection_mode(gtk::SelectionMode::None)
        .build();
    let empty = adw::StatusPage::builder()
        .title(tr("No Templates"))
        .description(tr("None of this frequency pass the filter"))
        .icon_name("edit-find-symbolic")
        .css_classes(["compact"])
        .build();
    let names: Rc<RefCell<Vec<String>>> = Rc::default();
    let show = {
        let (rules, list, empty, names) =
            (rules.clone(), list.clone(), empty.clone(), names.clone());
        move |category: usize| {
            list.remove_all();
            let found = rules.candidate_names(&filter, category);
            for name in &found {
                list.append(
                    &adw::ActionRow::builder()
                        .title(glib::markup_escape_text(name).as_str())
                        .activatable(true)
                        .build(),
                );
            }
            list.set_visible(!found.is_empty());
            empty.set_visible(found.is_empty());
            *names.borrow_mut() = found;
        }
    };
    show(PICK_ROLLS[index]);
    frequency.connect_selected_notify(move |row| show(row.selected() as usize));
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&top);
    content.append(&list);
    content.append(&empty);
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(
        &gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&adw::Clamp::builder().child(&content).build())
            .build(),
    ));
    let dialog = adw::Dialog::builder()
        .title(tr!("Roll {roll}", roll = index + 2))
        .content_width(480)
        .content_height(600)
        .width_request(360)
        .height_request(294)
        .child(&view)
        .build();
    let chosen = Rc::new(chosen);
    none.connect_activated({
        let (dialog, chosen) = (dialog.clone(), chosen.clone());
        move |_| {
            dialog.close();
            chosen(String::new());
        }
    });
    list.connect_row_activated({
        let dialog = dialog.clone();
        move |_, row| {
            let name = names.borrow()[row.index() as usize].clone();
            dialog.close();
            chosen(name);
        }
    });
    dialog.present(Some(parent));
}

/// The 2-20 table editor: name, filter, the 19 rolls, Fill, Roll and
/// Generate. `saved` is the table's name when it is already saved.
pub fn table_editor<H: Host + 'static>(host: &Shared<H>, saved: Option<&str>) {
    let Some((rules, tables)) = errors(
        &**host,
        host.with_install(|i| Ok((i.encounter_rules()?, i.frequency_tables()?))),
    ) else {
        return;
    };
    let rules = Rc::new(rules);
    let table = saved
        .and_then(|n| tables.table(n).cloned())
        .unwrap_or_else(|| FrequencyTable::new(""));
    let table = Rc::new(RefCell::new(table));

    let name = adw::EntryRow::builder()
        .title(tr("Name"))
        .text(&table.borrow().name)
        .build();
    let about = adw::PreferencesGroup::new();
    about.add(&name);

    // Filter.
    let filter = table.borrow().filter.clone();
    let database = adw::ComboRow::builder()
        .title(tr("Database"))
        .model(&gtk::StringList::new(&[
            tr("All"),
            tr("Standard"),
            tr("Custom"),
        ]))
        .selected(filter.source.clamp(0, 2) as u32)
        .build();
    let lair = adw::ComboRow::builder()
        .title(tr("Lair and/or Wandering"))
        .model(&gtk::StringList::new(&[
            tr("Lair and Wandering"),
            tr("Lair or Wandering"),
            tr("Wandering"),
            tr("Lair"),
        ]))
        .selected(filter.lair.clamp(0, 3) as u32)
        .build();
    let min = adw::SpinRow::builder()
        .title(tr("Fewest Hit Dice"))
        .adjustment(&gtk::Adjustment::new(
            filter.hit_dice.0 as f64,
            0.0,
            100.0,
            1.0,
            5.0,
            0.0,
        ))
        .build();
    let max = adw::SpinRow::builder()
        .title(tr("Most Hit Dice"))
        .adjustment(&gtk::Adjustment::new(
            filter.hit_dice.1 as f64,
            0.0,
            100.0,
            1.0,
            5.0,
            0.0,
        ))
        .build();
    let climates = adw::ExpanderRow::builder()
        .title(tr("Climate and Terrain"))
        .subtitle(tr("Where the templates may be encountered"))
        .build();
    let mut checks = Vec::new();
    for (i, place) in CLIMATES.iter().chain(TERRAINS.iter()).enumerate() {
        let check = gtk::CheckButton::builder()
            .active(filter.climates[i] != 0)
            .valign(gtk::Align::Center)
            .build();
        let row = adw::ActionRow::builder()
            .title(tr(place))
            .activatable_widget(&check)
            .build();
        row.add_prefix(&check);
        climates.add_row(&row);
        checks.push(check);
    }
    let filter_group = adw::PreferencesGroup::builder()
        .title(tr("Filter"))
        .description(tr(
            "Fill picks templates that pass the filter, by frequency",
        ))
        .build();
    filter_group.add(&database);
    filter_group.add(&min);
    filter_group.add(&max);
    filter_group.add(&lair);
    filter_group.add(&climates);
    let read_filter = {
        let (database, lair, min, max, checks, table) = (
            database.clone(),
            lair.clone(),
            min.clone(),
            max.clone(),
            checks.clone(),
            table.clone(),
        );
        move || {
            let mut t = table.borrow_mut();
            t.filter.source = Source::of(database.selected() as i32).raw();
            t.filter.lair = LairMode::of(lair.selected() as i32).raw();
            t.filter.hit_dice = (min.value() as i32, max.value() as i32);
            for (i, check) in checks.iter().enumerate() {
                t.filter.climates[i] = check.is_active() as i32;
            }
        }
    };

    // Rolls.
    let fill = gtk::Button::builder()
        .label(tr("_Fill"))
        .use_underline(true)
        .tooltip_text(tr("Pick a template for every roll"))
        .css_classes(["flat"])
        .build();
    let roll = gtk::Button::builder()
        .label(tr("_Roll"))
        .use_underline(true)
        .tooltip_text(tr("Roll 1d8+1d12 and create its encounter"))
        .css_classes(["flat"])
        .build();
    let buttons = gtk::Box::builder().spacing(6).build();
    buttons.append(&fill);
    buttons.append(&roll);
    let slots_group = adw::PreferencesGroup::builder()
        .title(tr("Rolls"))
        .description(tr(
            "Pick a roll to choose its template from those that pass the filter",
        ))
        .header_suffix(&buttons)
        .build();
    let slot_rows: Vec<adw::ActionRow> = (0..19)
        .map(|i| {
            let row = adw::ActionRow::builder()
                .title((i + 2).to_string())
                .activatable(true)
                .build();
            row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
            slots_group.add(&row);
            row
        })
        .collect();
    let show_slots = {
        let (rows, table) = (slot_rows.clone(), table.clone());
        move || {
            let t = table.borrow();
            for (i, row) in rows.iter().enumerate() {
                let name = t.slots.get(i).filter(|s| !s.is_empty());
                row.set_subtitle(&glib::markup_escape_text(
                    name.map_or(tr("(None)"), String::as_str),
                ));
            }
        }
    };
    let show_slots = Rc::new(show_slots);
    show_slots();
    for (i, row) in slot_rows.iter().enumerate() {
        let (rules, table, read_filter, show_slots) = (
            rules.clone(),
            table.clone(),
            read_filter.clone(),
            show_slots.clone(),
        );
        row.connect_activated(move |row| {
            read_filter();
            let filter = table.borrow().filter.clone();
            let (table, show_slots) = (table.clone(), show_slots.clone());
            pick_for_roll(row, i, &rules, filter, move |name| {
                let mut t = table.borrow_mut();
                t.slots.resize(19, String::new());
                t.slots[i] = name;
                drop(t);
                show_slots();
            });
        });
    }
    fill.connect_clicked({
        let (rules, table, host, read_filter, show_slots) = (
            rules.clone(),
            table.clone(),
            host.clone(),
            read_filter.clone(),
            show_slots.clone(),
        );
        move |_| {
            read_filter();
            let filter = table.borrow().filter.clone();
            let candidates = rules.candidates(&filter);
            let count: usize = candidates[..4].iter().map(Vec::len).sum();
            table
                .borrow_mut()
                .fill(&candidates, &mut host.rand().borrow_mut());
            show_slots();
            host.toast(&trn!(
                "{n} template passes the filter",
                "{n} templates pass the filter",
                count
            ));
        }
    });

    let page = adw::PreferencesPage::new();
    page.add(&about);
    page.add(&filter_group);
    page.add(&slots_group);
    let save = gtk::Button::builder()
        .label(tr("_Save"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let delete = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text(tr("Delete Table"))
        .visible(saved.is_some())
        .build();
    let header = adw::HeaderBar::new();
    header.pack_start(&delete);
    header.pack_end(&save);
    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, || topic::ENCOUNTER_TABLE);
    view.add_top_bar(&header);
    view.set_content(Some(&page));
    let dialog = adw::Dialog::builder()
        .title(saved.unwrap_or(tr("New Encounter Table")))
        .content_width(640)
        .content_height(760)
        .width_request(360)
        .height_request(294)
        .child(&view)
        .build();

    roll.connect_clicked({
        let (rules, table, host, dialog) =
            (rules.clone(), table.clone(), host.clone(), dialog.clone());
        move |_| {
            if table.borrow().slots.iter().all(|s| s.is_empty()) {
                return host.toast(tr("The table has no templates; fill it first"));
            }
            let r = roll_2_to_20(&rules, &mut host.rand().borrow_mut());
            let slot = table.borrow().slot(r).map(str::to_owned);
            match slot {
                Some(name) if !name.is_empty() => {
                    host.toast(&tr!("Rolled {roll}: {template}", roll = r, template = name));
                    dialog.close();
                    create_from_template(&host, &name);
                }
                _ => host.toast(&tr!("Rolled {roll}, which has no template", roll = r)),
            }
        }
    });
    let old = saved.map(str::to_owned);
    save.connect_clicked({
        let (table, host, dialog, old) = (table.clone(), host.clone(), dialog.clone(), old.clone());
        move |_| {
            read_filter();
            let mut t = table.borrow().clone();
            t.name = name.text().trim().to_owned();
            if t.name.is_empty() {
                return host.toast(tr("A name must be supplied."));
            }
            let result = host.with_install(|install| {
                let mut tables = install.frequency_tables()?;
                let clash = tables
                    .tables
                    .iter()
                    .any(|x| x.name.eq_ignore_ascii_case(&t.name) && Some(&x.name) != old.as_ref());
                if clash {
                    return Err(cr_store::Error::Invalid(tr("Name already used.").into()));
                }
                match old
                    .as_ref()
                    .and_then(|o| tables.tables.iter().position(|x| &x.name == o))
                {
                    Some(i) => tables.tables[i] = t.clone(),
                    None => tables.tables.push(t.clone()),
                }
                install.write_frequency_tables(&tables)
            });
            if errors(&*host, result).is_some() {
                host.toast(tr("Table saved"));
                host.reload(
                    "tables",
                    Some(format!("{}{}", cr_store::catalog::TABLE_KEY, t.name)),
                );
                dialog.close();
            }
        }
    });
    delete.connect_clicked({
        let (host, dialog) = (host.clone(), dialog.clone());
        move |_| {
            let Some(old) = old.clone() else { return };
            let alert = adw::AlertDialog::builder()
                .heading(tr("Delete Encounter Table?"))
                .body(tr!("“{name}” will be removed", name = old))
                .default_response("cancel")
                .close_response("cancel")
                .build();
            alert.add_responses(&[("cancel", tr("_Cancel")), ("delete", tr("_Delete"))]);
            alert.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
            let (host, dialog2) = (host.clone(), dialog.clone());
            alert.connect_response(Some("delete"), move |_, _| {
                let result = host.with_install(|install| {
                    let mut tables = install.frequency_tables()?;
                    tables.tables.retain(|t| t.name != old);
                    install.write_frequency_tables(&tables)
                });
                if errors(&*host, result).is_some() {
                    host.toast(tr("Table deleted"));
                    host.reload("tables", None);
                    dialog2.close();
                }
            });
            alert.present(Some(&dialog));
        }
    });
    dialog.present(Some(&host.parent()));
}

/// Climate flags 0-4 of templates and tables.
const CLIMATES: [&str; 5] = [
    tr_noop!("Arctic"),
    tr_noop!("Sub-Arctic"),
    tr_noop!("Temperate"),
    tr_noop!("Sub-Tropical"),
    tr_noop!("Tropical"),
];

/// Terrain flags 5-36 (table 0x67f3cc).
const TERRAINS: [&str; 32] = [
    tr_noop!("Aerial"),
    tr_noop!("Bush"),
    tr_noop!("Caves"),
    tr_noop!("Coastal"),
    tr_noop!("Deserts"),
    tr_noop!("Farmland"),
    tr_noop!("Forests"),
    tr_noop!("Glaciers"),
    tr_noop!("Grasslands"),
    tr_noop!("Jungles"),
    tr_noop!("Fresh-Water Lakes"),
    tr_noop!("Salt-Water Lakes"),
    tr_noop!("High Mountains"),
    tr_noop!("High Mountain Forests"),
    tr_noop!("High Mountain Lakes"),
    tr_noop!("Low Mountains"),
    tr_noop!("Low Mountain Forests"),
    tr_noop!("Low Mountain Lakes"),
    tr_noop!("Deep Oceans"),
    tr_noop!("Shallow Oceans"),
    tr_noop!("Parkland"),
    tr_noop!("Scrub Plains"),
    tr_noop!("Rain Forests"),
    tr_noop!("Fresh-Water Rivers"),
    tr_noop!("Salt-Water Rivers"),
    tr_noop!("Rough/Hills"),
    tr_noop!("Salt Marshes"),
    tr_noop!("Steppes"),
    tr_noop!("Subterranean"),
    tr_noop!("Fresh-Water Swamps"),
    tr_noop!("Salt-Water Swamps"),
    tr_noop!("Tundras"),
];
