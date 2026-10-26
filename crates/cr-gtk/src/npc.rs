//! NPC Generation: the New NPC dialog (Create NPC, with Ability Scores…)
//! and the New NPC Group dialog (Create Multiple NPCs), drawn over the
//! forms of `cr_app::npc`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use adw::prelude::*;
use cr_app::npc::{GROUPS, GroupForm, GroupInput, MOST_LEVEL, MOST_NUMBER, NpcForm, NpcInput};
use cr_i18n::tr;
use cr_store::create::ABILITIES;
use cr_store::help::topic;
use cr_store::npc::{Npc, NpcRules};
use cr_store::treasure::Rand;
use cr_store::{Installation, Result};
use gtk::glib;

/// What the NPC dialogs need from the window.
pub trait Host {
    fn with_install<T>(&self, run: impl FnOnce(&Installation) -> Result<T>) -> Option<Result<T>>;
    /// The generator of the open installation, kept for the session as
    /// the original keeps its kit edits.
    fn npc_rules(&self) -> Option<Arc<NpcRules>>;
    fn rand(&self) -> Rc<RefCell<Rand>>;
    fn toast(&self, message: &str);
    /// Reloads the sections and shows the entry `key` of section `section`.
    fn reload(&self, section: &str, key: Option<String>);
    fn parent(&self) -> gtk::Widget;
}

fn spin_row(title: &str, value: i32, min: i32, max: i32) -> adw::SpinRow {
    adw::SpinRow::builder()
        .title(title)
        .adjustment(&gtk::Adjustment::new(
            value as f64,
            min as f64,
            max as f64,
            1.0,
            5.0,
            0.0,
        ))
        .build()
}

fn spin_button(value: i32, min: i32, max: i32) -> gtk::SpinButton {
    gtk::SpinButton::builder()
        .adjustment(&gtk::Adjustment::new(
            value as f64,
            min as f64,
            max as f64,
            1.0,
            5.0,
            0.0,
        ))
        .valign(gtk::Align::Center)
        .build()
}

fn combo_row(title: &str) -> adw::ComboRow {
    adw::ComboRow::builder()
        .title(title)
        .enable_search(true)
        .expression(gtk::PropertyExpression::new(
            gtk::StringObject::static_type(),
            None::<gtk::Expression>,
            "string",
        ))
        .build()
}

/// Puts `items` in `row` with `selected` chosen; the model is made again
/// only when the items changed.
fn set_items(row: &adw::ComboRow, items: &[String], selected: usize) {
    let same = row.model().is_some_and(|m| {
        m.n_items() as usize == items.len()
            && items.iter().enumerate().all(|(i, s)| {
                m.item(i as u32)
                    .and_downcast::<gtk::StringObject>()
                    .is_some_and(|o| o.string() == s.as_str())
            })
    });
    if !same {
        let texts: Vec<&str> = items.iter().map(String::as_str).collect();
        row.set_model(Some(&gtk::StringList::new(&texts)));
    }
    if row.selected() as usize != selected {
        row.set_selected(selected as u32);
    }
}

fn rules_row() -> adw::ComboRow {
    adw::ComboRow::builder()
        .title(tr("Rules"))
        .model(&gtk::StringList::new(&[
            tr("Core Rules"),
            "Player's Option",
        ]))
        .build()
}

fn switch_row(title: &str, subtitle: &str) -> adw::SwitchRow {
    adw::SwitchRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build()
}

fn dialog(
    title: &str,
    page: &adw::PreferencesPage,
    create: &gtk::Button,
    help: u32,
) -> adw::Dialog {
    let header = adw::HeaderBar::new();
    header.pack_end(create);
    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, move || help);
    view.add_top_bar(&header);
    view.set_content(Some(page));
    adw::Dialog::builder()
        .title(title)
        .content_width(520)
        .content_height(720)
        .width_request(360)
        .height_request(294)
        .child(&view)
        .build()
}

fn create_button(label: &str) -> gtk::Button {
    gtk::Button::builder()
        .label(label)
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build()
}

/// Shows the generator's notes on what it could not do (an item of the
/// kit missing from the database, no spellbook part), as the original's
/// messages do.
fn show_messages<'a>(parent: &gtk::Widget, npcs: impl IntoIterator<Item = &'a Npc>) {
    if let Some(text) = cr_app::npc::messages(npcs) {
        crate::create::message(parent, tr("NPC Generator"), &text);
    }
}

/// Called with the NPC "Add to Template" generates.
pub type Chosen = Box<dyn Fn(&Npc)>;

/// Opens the New NPC dialog.
pub fn new_npc<H: Host + 'static>(host: &Rc<H>) {
    npc_dialog(host, None, &host.parent());
}

/// NPC Creation as an encounter template's Add opens it (template mode,
/// 0x4bd39a): no ability scores or party, and "Add to Template" hands the
/// generated NPC to `chosen` instead of saving it.
pub fn template_npc<H: Host + 'static>(host: &Rc<H>, parent: &gtk::Widget, chosen: Chosen) {
    npc_dialog(host, Some(chosen), parent);
}

/// The widgets of the New NPC dialog, which show its form.
struct NpcView<H: Host> {
    host: Rc<H>,
    form: RefCell<NpcForm>,
    /// Set while the widgets are being filled, so their signals are
    /// ignored.
    filling: Cell<bool>,
    race: adw::ComboRow,
    all_races: adw::SwitchRow,
    class: adw::ComboRow,
    all_classes: adw::SwitchRow,
    rules: adw::ComboRow,
    levels_group: adw::PreferencesGroup,
    level_rows: RefCell<Vec<adw::SpinRow>>,
    abilities_group: adw::PreferencesGroup,
    ability_rows: Vec<adw::SpinRow>,
    exceptional: adw::SpinRow,
    clear: gtk::Button,
}

impl<H: Host + 'static> NpcView<H> {
    /// Changes the form and shows it as it now is.
    fn input(self: &Rc<Self>, input: NpcInput) {
        if self.filling.get() {
            return;
        }
        self.form
            .borrow_mut()
            .input(input, &mut self.host.rand().borrow_mut());
        let error = self.form.borrow_mut().error.take();
        if let Some(error) = error {
            self.host.toast(&error);
        }
        self.show();
    }

    /// Calls `input` with what `make` makes of the widget, while the view
    /// lives.
    fn on<W>(self: &Rc<Self>, make: impl Fn(&W) -> NpcInput + 'static) -> impl Fn(&W) + 'static {
        let view = Rc::downgrade(self);
        move |widget| {
            if let Some(view) = view.upgrade() {
                view.input(make(widget));
            }
        }
    }

    fn show(self: &Rc<Self>) {
        self.filling.set(true);
        let form = self.form.borrow();
        set_items(&self.race, form.races(), form.race_index());
        self.all_races.set_active(form.all_races);
        set_items(&self.class, &form.class_names(), form.class_index());
        self.all_classes.set_active(form.all_classes);
        self.rules.set_selected(u32::from(form.players_option));
        // A row for each class of the combination, named for it.
        let names: Vec<String> = self
            .level_rows
            .borrow()
            .iter()
            .map(|r| r.title().to_string())
            .collect();
        if names != form.classes() {
            for row in self.level_rows.borrow_mut().drain(..) {
                self.levels_group.remove(&row);
            }
            for (i, name) in form.classes().iter().enumerate() {
                let row = spin_row(name, form.levels[i], 1, MOST_LEVEL);
                row.connect_value_notify(
                    self.on(move |row: &adw::SpinRow| NpcInput::Level(i, row.value() as i32)),
                );
                self.levels_group.add(&row);
                self.level_rows.borrow_mut().push(row);
            }
        }
        for (row, level) in self.level_rows.borrow().iter().zip(&form.levels) {
            row.set_value(f64::from(*level));
        }
        let rolled = form.abilities;
        self.abilities_group
            .set_description(Some(if rolled.is_some() {
                tr("The NPC keeps these scores")
            } else {
                tr("Rolled with the NPC unless you roll them now")
            }));
        for (i, (row, (lo, hi))) in self
            .ability_rows
            .iter()
            .zip(form.ability_ranges())
            .enumerate()
        {
            row.set_visible(rolled.is_some());
            row.set_range(f64::from(lo), f64::from(hi.max(lo)));
            if let Some(a) = rolled {
                row.set_value(f64::from(a.scores[i]));
            }
        }
        self.exceptional.set_visible(form.exceptional_shown());
        self.exceptional
            .set_value(f64::from(rolled.map_or(0, |a| a.exceptional)));
        self.clear.set_visible(rolled.is_some());
        drop(form);
        self.filling.set(false);
    }
}

fn npc_dialog<H: Host + 'static>(host: &Rc<H>, template: Option<Chosen>, parent: &gtk::Widget) {
    let Some(rules) = host.npc_rules() else {
        return;
    };
    let form = NpcForm::new(rules, template.is_some());

    // Race and class.
    let race = combo_row(tr("Race"));
    let all_races = switch_row(tr("All Races"), tr("Include races that are not standard"));
    let class = adw::ComboRow::builder().title(tr("Class")).build();
    let all_classes = switch_row(
        tr("All Classes"),
        tr("Include classes the race may not take"),
    );
    let rules_choice = rules_row();
    let character = adw::PreferencesGroup::builder()
        .title(tr("Character"))
        .build();
    character.add(&race);
    character.add(&all_races);
    character.add(&class);
    character.add(&all_classes);
    character.add(&rules_choice);

    // Levels, one per class of the chosen combination.
    let levels_group = adw::PreferencesGroup::builder().title(tr("Levels")).build();

    // Ability scores.
    let roll = gtk::Button::builder()
        .label(tr("_Roll"))
        .use_underline(true)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let clear = gtk::Button::builder()
        .label(tr("_Clear"))
        .use_underline(true)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .visible(false)
        .build();
    let ability_buttons = gtk::Box::builder().spacing(6).build();
    ability_buttons.append(&clear);
    ability_buttons.append(&roll);
    let abilities_group = adw::PreferencesGroup::builder()
        .title(tr("Ability Scores"))
        .header_suffix(&ability_buttons)
        .build();
    let ability_rows: Vec<adw::SpinRow> = ABILITIES
        .iter()
        .map(|name| {
            let row = spin_row(tr(name), 10, 1, 25);
            row.set_visible(false);
            abilities_group.add(&row);
            row
        })
        .collect();
    let exceptional = spin_row(tr("Exceptional Strength"), 0, 0, 100);
    exceptional.set_subtitle(tr("1 to 100 for 18/01 to 18/00"));
    exceptional.set_visible(false);
    abilities_group.add(&exceptional);

    let party = adw::EntryRow::builder().title(tr("Party")).build();
    let save_group = adw::PreferencesGroup::new();
    save_group.add(&party);
    save_group.set_visible(form.party_shown());
    abilities_group.set_visible(!form.template);

    let page = adw::PreferencesPage::new();
    page.add(&character);
    page.add(&levels_group);
    page.add(&abilities_group);
    page.add(&save_group);
    let create = create_button(if template.is_some() {
        tr("Add to _Template")
    } else {
        tr("_Create")
    });
    let dialog = dialog(tr("New NPC"), &page, &create, topic::NPC_CREATION);

    let view = Rc::new(NpcView {
        host: host.clone(),
        form: RefCell::new(form),
        filling: Cell::new(false),
        race,
        all_races,
        class,
        all_classes,
        rules: rules_choice,
        levels_group,
        level_rows: RefCell::default(),
        abilities_group,
        ability_rows,
        exceptional,
        clear,
    });
    view.show();
    view.race.connect_selected_notify(
        view.on(|row: &adw::ComboRow| NpcInput::Race(row.selected() as usize)),
    );
    view.all_races
        .connect_active_notify(view.on(|row: &adw::SwitchRow| NpcInput::AllRaces(row.is_active())));
    view.class.connect_selected_notify(
        view.on(|row: &adw::ComboRow| NpcInput::Class(row.selected() as usize)),
    );
    view.all_classes.connect_active_notify(
        view.on(|row: &adw::SwitchRow| NpcInput::AllClasses(row.is_active())),
    );
    view.rules.connect_selected_notify(
        view.on(|row: &adw::ComboRow| NpcInput::PlayersOption(row.selected() == 1)),
    );
    for (i, row) in view.ability_rows.iter().enumerate() {
        row.connect_value_notify(
            view.on(move |row: &adw::SpinRow| NpcInput::Ability(i, row.value() as i32)),
        );
    }
    view.exceptional.connect_value_notify(
        view.on(|row: &adw::SpinRow| NpcInput::Exceptional(row.value() as i32)),
    );
    roll.connect_clicked(view.on(|_: &gtk::Button| NpcInput::Roll));
    view.clear
        .connect_clicked(view.on(|_: &gtk::Button| NpcInput::Clear));
    party.connect_changed(view.on(|row: &adw::EntryRow| NpcInput::Party(row.text().to_string())));

    create.connect_clicked({
        let (host, dialog, view) = (host.clone(), dialog.clone(), view.clone());
        move |create| {
            let npc = view.form.borrow().generate(&mut host.rand().borrow_mut());
            let npc = match npc {
                Ok(npc) => npc,
                Err(e) => return host.toast(&e),
            };
            if let Some(chosen) = &template {
                // Clicks made meanwhile create nothing more.
                create.set_sensitive(false);
                dialog.close();
                show_messages(&host.parent(), [&npc]);
                return chosen(&npc);
            }
            let party = view.form.borrow().party.trim().to_owned();
            match host.with_install(|install| install.save_npc(&npc, &party)) {
                Some(Ok(file)) => {
                    create.set_sensitive(false);
                    dialog.close();
                    show_messages(&host.parent(), [&npc]);
                    host.reload("characters", Some(file));
                    host.toast(&tr!(
                        "{name} was created",
                        name = glib::markup_escape_text(&npc.character.data.name)
                    ));
                }
                Some(Err(e)) => host.toast(&e.to_string()),
                None => {}
            }
        }
    });
    dialog.present(Some(parent));
}

/// The widgets of the New NPC Group dialog, which show its form.
struct GroupView<H: Host> {
    host: Rc<H>,
    form: RefCell<GroupForm>,
    filling: Cell<bool>,
    race: adw::ComboRow,
    all_races: adw::SwitchRow,
    fewest: adw::SpinRow,
    most: adw::SpinRow,
    rules: adw::ComboRow,
    allowed: adw::SwitchRow,
    class_rows: Vec<adw::ComboRow>,
    kit: adw::ComboRow,
    all_kits: adw::SwitchRow,
    alignment: adw::ComboRow,
    any_alignment: adw::SwitchRow,
    levels: Vec<(gtk::SpinButton, gtk::SpinButton)>,
    lines_group: adw::PreferencesGroup,
    line_rows: RefCell<Vec<adw::ActionRow>>,
}

impl<H: Host + 'static> GroupView<H> {
    fn input(&self, input: GroupInput) {
        if self.filling.get() {
            return;
        }
        self.form.borrow_mut().input(input);
        let error = self.form.borrow_mut().error.take();
        if let Some(error) = error {
            self.host.toast(&error);
        }
        self.show();
    }

    fn on<W>(self: &Rc<Self>, make: impl Fn(&W) -> GroupInput + 'static) -> impl Fn(&W) + 'static {
        let view = Rc::downgrade(self);
        move |widget| {
            if let Some(view) = view.upgrade() {
                view.input(make(widget));
            }
        }
    }

    fn show(&self) {
        self.filling.set(true);
        let form = self.form.borrow();
        set_items(&self.race, &form.race_items(), form.race_index());
        self.all_races.set_active(form.all_races);
        self.fewest.set_value(f64::from(form.number.0));
        self.most.set_value(f64::from(form.number.1));
        self.rules.set_selected(u32::from(form.players_option));
        self.allowed.set_active(form.allowed);
        for (g, row) in self.class_rows.iter().enumerate() {
            set_items(row, &form.class_items(g), form.class_index(g));
            row.set_sensitive(form.classes_enabled());
        }
        set_items(&self.kit, &form.kit_items(), form.kit_index());
        self.all_kits.set_active(form.all_kits);
        set_items(
            &self.alignment,
            &form.alignment_items(),
            form.alignment_index(),
        );
        self.alignment.set_sensitive(form.alignment_enabled());
        self.any_alignment.set_active(form.any_alignment);
        for ((low, high), (lo, hi)) in self.levels.iter().zip(form.levels) {
            low.set_value(f64::from(lo));
            high.set_value(f64::from(hi));
        }
        let lines = form.lines();
        if lines.len() != self.line_rows.borrow().len() {
            for row in self.line_rows.borrow_mut().drain(..) {
                self.lines_group.remove(&row);
            }
            for line in &lines {
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(line))
                    .build();
                self.lines_group.add(&row);
                self.line_rows.borrow_mut().push(row);
            }
        }
        drop(form);
        self.filling.set(false);
    }
}

/// Opens the New NPC Group dialog: one or more lines (number, race,
/// classes, kit, alignment and levels), made in the order of the sorted
/// list as Create Group does.
pub fn new_group<H: Host + 'static>(host: &Rc<H>) {
    let Some(rules) = host.npc_rules() else {
        return;
    };
    let race = combo_row(tr("Race"));
    let all_races = switch_row(tr("All Races"), tr("Include races that are not standard"));
    let fewest = spin_row(tr("Fewest"), 1, 1, MOST_NUMBER);
    let most = spin_row(tr("Most"), 1, 1, MOST_NUMBER);
    let rules_choice = rules_row();
    let party = adw::EntryRow::builder().title(tr("Group Name")).build();
    let group = adw::PreferencesGroup::builder().title(tr("Group")).build();
    group.add(&party);
    group.add(&race);
    group.add(&all_races);
    group.add(&fewest);
    group.add(&most);
    group.add(&rules_choice);

    let allowed = switch_row(
        tr("Allowed Combination"),
        tr("Each NPC takes a class combination its race allows"),
    );
    let classes_group = adw::PreferencesGroup::builder()
        .title(tr("Classes"))
        .description(tr("A class of each group, or none"))
        .build();
    classes_group.add(&allowed);
    let class_rows: Vec<adw::ComboRow> = GROUPS
        .iter()
        .map(|name| {
            let row = combo_row(tr(name));
            classes_group.add(&row);
            row
        })
        .collect();
    let kit = combo_row(tr("Kit"));
    let all_kits = switch_row(
        tr("All Kits"),
        tr("Include kits the race, classes or alignment may not take"),
    );
    let alignment = combo_row(tr("Alignment"));
    let any_alignment = switch_row(
        tr("Any Alignment"),
        tr("Include alignments the classes, race or kit do not allow"),
    );
    classes_group.add(&kit);
    classes_group.add(&all_kits);
    classes_group.add(&alignment);
    classes_group.add(&any_alignment);

    let levels_group = adw::PreferencesGroup::builder()
        .title(tr("Levels"))
        .description(tr("Lowest and highest level for each class group"))
        .build();
    let levels: Vec<(gtk::SpinButton, gtk::SpinButton)> = GROUPS
        .iter()
        .map(|name| {
            let low = spin_button(1, 1, MOST_LEVEL);
            let high = spin_button(1, 1, MOST_LEVEL);
            let name = tr(name);
            low.update_property(&[gtk::accessible::Property::Label(&tr!(
                "Lowest {name} Level",
                name
            ))]);
            high.update_property(&[gtk::accessible::Property::Label(&tr!(
                "Highest {name} Level",
                name
            ))]);
            let to = gtk::Label::new(Some(tr("to")));
            to.add_css_class("dim-label");
            let row = adw::ActionRow::builder().title(name).build();
            row.add_suffix(&low);
            row.add_suffix(&to);
            row.add_suffix(&high);
            levels_group.add(&row);
            (low, high)
        })
        .collect();

    // The lines added so far, shown sorted as the original's list.
    let lines_group = adw::PreferencesGroup::builder()
        .title(tr("Lines"))
        .description(tr("Without lines, Create makes the line above"))
        .build();
    let add = gtk::Button::builder()
        .label(tr("_Add Line"))
        .use_underline(true)
        .valign(gtk::Align::Center)
        .build();
    lines_group.set_header_suffix(Some(&add));

    let page = adw::PreferencesPage::new();
    page.add(&group);
    page.add(&classes_group);
    page.add(&levels_group);
    page.add(&lines_group);
    let create = create_button(tr("_Create"));
    let dialog = dialog(tr("New NPC Group"), &page, &create, topic::NPC_GROUP);

    let view = Rc::new(GroupView {
        host: host.clone(),
        form: RefCell::new(GroupForm::new(rules)),
        filling: Cell::new(false),
        race,
        all_races,
        fewest,
        most,
        rules: rules_choice,
        allowed,
        class_rows,
        kit,
        all_kits,
        alignment,
        any_alignment,
        levels,
        lines_group,
        line_rows: RefCell::default(),
    });
    view.show();
    party.connect_changed(view.on(|row: &adw::EntryRow| GroupInput::Name(row.text().to_string())));
    view.race.connect_selected_notify(
        view.on(|row: &adw::ComboRow| GroupInput::Race(row.selected() as usize)),
    );
    view.all_races.connect_active_notify(
        view.on(|row: &adw::SwitchRow| GroupInput::AllRaces(row.is_active())),
    );
    view.fewest
        .connect_value_notify(view.on(|row: &adw::SpinRow| GroupInput::Fewest(row.value() as i32)));
    view.most
        .connect_value_notify(view.on(|row: &adw::SpinRow| GroupInput::Most(row.value() as i32)));
    view.rules.connect_selected_notify(
        view.on(|row: &adw::ComboRow| GroupInput::PlayersOption(row.selected() == 1)),
    );
    view.allowed.connect_active_notify(
        view.on(|row: &adw::SwitchRow| GroupInput::Allowed(row.is_active())),
    );
    for (g, row) in view.class_rows.iter().enumerate() {
        row.connect_selected_notify(
            view.on(move |row: &adw::ComboRow| GroupInput::Class(g, row.selected() as usize)),
        );
    }
    view.kit.connect_selected_notify(
        view.on(|row: &adw::ComboRow| GroupInput::Kit(row.selected() as usize)),
    );
    view.all_kits.connect_active_notify(
        view.on(|row: &adw::SwitchRow| GroupInput::AllKits(row.is_active())),
    );
    view.alignment.connect_selected_notify(
        view.on(|row: &adw::ComboRow| GroupInput::Alignment(row.selected() as usize)),
    );
    view.any_alignment.connect_active_notify(
        view.on(|row: &adw::SwitchRow| GroupInput::AnyAlignment(row.is_active())),
    );
    for (g, (low, high)) in view.levels.iter().enumerate() {
        low.connect_value_changed(
            view.on(move |spin: &gtk::SpinButton| GroupInput::Low(g, spin.value_as_int())),
        );
        high.connect_value_changed(
            view.on(move |spin: &gtk::SpinButton| GroupInput::High(g, spin.value_as_int())),
        );
    }
    add.connect_clicked(view.on(|_: &gtk::Button| GroupInput::AddLine));

    create.connect_clicked({
        let (host, dialog, view) = (host.clone(), dialog.clone(), view.clone());
        move |create| {
            let npcs = view.form.borrow().generate(&mut host.rand().borrow_mut());
            let npcs = match npcs {
                Ok(npcs) => npcs,
                Err(e) => return host.toast(&e),
            };
            let name = view.form.borrow().name.trim().to_owned();
            let saved = host.with_install(|install| {
                let mut last = None;
                for npc in &npcs {
                    last = Some(install.save_npc(npc, &name)?);
                }
                Ok(last)
            });
            match saved {
                Some(Ok(last)) => {
                    create.set_sensitive(false);
                    dialog.close();
                    show_messages(&host.parent(), &npcs);
                    host.reload("characters", last);
                    host.toast(&cr_app::npc::created_text(npcs.len()));
                }
                Some(Err(e)) => host.toast(&e.to_string()),
                None => {}
            }
        }
    });
    dialog.present(Some(&host.parent()));
}
