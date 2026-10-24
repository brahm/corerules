//! NPC Generation: the New NPC dialog (Create NPC, with Ability Scores…)
//! and the New NPC Group dialog (Create Multiple NPCs).

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::{tr, tr_noop, trn};
use cr_store::help::topic;
use cr_store::npc::{Abilities, Choice, GroupOptions, Npc, NpcOptions, NpcRules};
use cr_store::treasure::{Flavor, Rand};
use cr_store::{Installation, Result};
use gtk::glib;

/// What the NPC dialogs need from the window.
pub trait Host {
    fn with_install<T>(&self, run: impl FnOnce(&Installation) -> Result<T>) -> Option<Result<T>>;
    /// The generator of the open installation, kept for the session as
    /// the original keeps its kit edits.
    fn npc_rules(&self) -> Option<Rc<NpcRules>>;
    fn rand(&self) -> Rc<RefCell<Rand>>;
    fn toast(&self, message: &str);
    /// Reloads the sections and shows the entry `key` of section `section`.
    fn reload(&self, section: &str, key: Option<String>);
    fn parent(&self) -> gtk::Widget;
}

const ABILITIES: [&str; 6] = [
    tr_noop!("Strength"),
    tr_noop!("Dexterity"),
    tr_noop!("Constitution"),
    tr_noop!("Intelligence"),
    tr_noop!("Wisdom"),
    tr_noop!("Charisma"),
];

/// The class groups of Create Multiple NPCs, in combination order.
const GROUPS: [&str; 5] = [
    tr_noop!("Warrior"),
    tr_noop!("Rogue"),
    tr_noop!("Priest"),
    tr_noop!("Wizard"),
    tr_noop!("Psionicist"),
];

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

fn combo_row(title: &str, items: &[String]) -> adw::ComboRow {
    let items: Vec<&str> = items.iter().map(String::as_str).collect();
    adw::ComboRow::builder()
        .title(title)
        .model(&gtk::StringList::new(&items))
        .enable_search(true)
        .expression(gtk::PropertyExpression::new(
            gtk::StringObject::static_type(),
            None::<gtk::Expression>,
            "string",
        ))
        .build()
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
    let lines: Vec<String> = npcs
        .into_iter()
        .flat_map(|npc| {
            npc.messages
                .iter()
                .map(move |m| format!("{}: {m}", npc.character.data.name))
        })
        .collect();
    if !lines.is_empty() {
        crate::create::message(parent, tr("NPC Generator"), &lines.join("\n"));
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

fn npc_dialog<H: Host + 'static>(host: &Rc<H>, template: Option<Chosen>, parent: &gtk::Widget) {
    let Some(rules) = host.npc_rules() else {
        return;
    };
    let standard = rules.standard_races();
    let all = rules.races();
    let expansion = rules.flavor() == Flavor::Expansion;

    // Race and class.
    let all_races = adw::SwitchRow::builder()
        .title(tr("All Races"))
        .subtitle(tr("Include races that are not standard"))
        .build();
    let race = combo_row(
        tr("Race"),
        if standard.is_empty() { &all } else { &standard },
    );
    let all_classes = adw::SwitchRow::builder()
        .title(tr("All Classes"))
        .subtitle(tr("Include classes the race may not take"))
        .build();
    let class = adw::ComboRow::builder().title(tr("Class")).build();
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
    let level_rows: Rc<RefCell<Vec<adw::SpinRow>>> = Rc::default();
    let options: Rc<RefCell<Vec<Vec<String>>>> = Rc::default();

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
        .description(tr("Rolled with the NPC unless you roll them now"))
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
    let rolled: Rc<RefCell<Option<Abilities>>> = Rc::default();

    let party = adw::EntryRow::builder().title(tr("Party")).build();
    let save_group = adw::PreferencesGroup::new();
    save_group.add(&party);
    save_group.set_visible(expansion && template.is_none());
    abilities_group.set_visible(template.is_none());

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

    let selected_race = {
        let race = race.clone();
        move || {
            race.selected_item()
                .and_downcast::<gtk::StringObject>()
                .map(|s| s.string().to_string())
                .unwrap_or_default()
        }
    };
    let selected_classes = {
        let (class, options) = (class.clone(), options.clone());
        move || {
            options
                .borrow()
                .get(class.selected() as usize)
                .cloned()
                .unwrap_or_default()
        }
    };
    let show_levels = {
        let (levels_group, level_rows) = (levels_group.clone(), level_rows.clone());
        let selected_classes = selected_classes.clone();
        move || {
            let old: Vec<i32> = level_rows
                .borrow()
                .iter()
                .map(|r| r.value() as i32)
                .collect();
            for row in level_rows.borrow_mut().drain(..) {
                levels_group.remove(&row);
            }
            for (i, name) in selected_classes().iter().enumerate() {
                let row = spin_row(name, old.get(i).copied().unwrap_or(1), 1, 30);
                levels_group.add(&row);
                level_rows.borrow_mut().push(row);
            }
        }
    };
    let show_classes = {
        let (rules, class, all_classes, options) = (
            rules.clone(),
            class.clone(),
            all_classes.clone(),
            options.clone(),
        );
        let (selected_race, show_levels) = (selected_race.clone(), show_levels.clone());
        move || {
            let race = selected_race();
            let mut list = if all_classes.is_active() {
                Vec::new()
            } else {
                rules.class_options(&race)
            };
            if list.is_empty() {
                list = rules.classes().into_iter().map(|c| vec![c]).collect();
            }
            // Single classes before multi-class combinations.
            list.sort_by_key(Vec::len);
            let names: Vec<String> = list.iter().map(|c| c.join("/")).collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            *options.borrow_mut() = list;
            class.set_model(Some(&gtk::StringList::new(&names)));
            class.set_selected(0);
            show_levels();
        }
    };
    show_classes();
    race.connect_selected_notify({
        let show_classes = show_classes.clone();
        move |_| show_classes()
    });
    all_classes.connect_active_notify({
        let show_classes = show_classes.clone();
        move |_| show_classes()
    });
    class.connect_selected_notify({
        let show_levels = show_levels.clone();
        move |_| show_levels()
    });
    all_races.connect_active_notify({
        let race = race.clone();
        move |row| {
            let current = race
                .selected_item()
                .and_downcast::<gtk::StringObject>()
                .map(|s| s.string().to_string());
            let list = if row.is_active() || standard.is_empty() {
                &all
            } else {
                &standard
            };
            let names: Vec<&str> = list.iter().map(String::as_str).collect();
            race.set_model(Some(&gtk::StringList::new(&names)));
            let keep = current.and_then(|c| list.iter().position(|r| *r == c));
            race.set_selected(keep.unwrap_or(0) as u32);
        }
    });

    let chosen = {
        let level_rows = level_rows.clone();
        let selected_classes = selected_classes.clone();
        move || -> Vec<(String, i32)> {
            selected_classes()
                .into_iter()
                .zip(level_rows.borrow().iter())
                .map(|(c, row)| (c, row.value() as i32))
                .collect()
        }
    };
    let show_abilities = {
        let (ability_rows, exceptional, clear, abilities_group) = (
            ability_rows.clone(),
            exceptional.clone(),
            clear.clone(),
            abilities_group.clone(),
        );
        move |a: Option<&Abilities>| {
            abilities_group.set_description(Some(if a.is_some() {
                tr("The NPC keeps these scores")
            } else {
                tr("Rolled with the NPC unless you roll them now")
            }));
            for (i, row) in ability_rows.iter().enumerate() {
                row.set_visible(a.is_some());
                if let Some(a) = a {
                    row.set_value(a.scores[i] as f64);
                }
            }
            exceptional.set_visible(a.is_some_and(|a| a.exceptional > 0));
            exceptional.set_value(a.map_or(0, |a| a.exceptional) as f64);
            clear.set_visible(a.is_some());
        }
    };
    // The scores typed stay within the race's limits, and 18 Strength
    // brings exceptional Strength for a race that rolls it.
    let fit_abilities = Rc::new({
        let (rules, ability_rows, exceptional, rolled, selected_race) = (
            rules.clone(),
            ability_rows.clone(),
            exceptional.clone(),
            rolled.clone(),
            selected_race.clone(),
        );
        move || {
            let race = selected_race();
            for (row, (lo, hi)) in ability_rows.iter().zip(rules.ability_range(&race)) {
                row.set_range(f64::from(lo), f64::from(hi));
            }
            let strong = rolled.borrow().is_some()
                && ability_rows[0].value() as i32 == 18
                && rules.exceptional_strength(&race);
            if strong && exceptional.value() == 0.0 {
                exceptional.set_value(1.0);
            }
            exceptional.set_visible(strong);
        }
    });
    ability_rows[0].connect_value_notify({
        let fit_abilities = fit_abilities.clone();
        move |_| fit_abilities()
    });
    race.connect_selected_notify({
        let fit_abilities = fit_abilities.clone();
        move |_| fit_abilities()
    });
    roll.connect_clicked({
        let (rules, host, rules_choice, rolled) = (
            rules.clone(),
            host.clone(),
            rules_choice.clone(),
            rolled.clone(),
        );
        let (selected_race, chosen, show_abilities) = (
            selected_race.clone(),
            chosen.clone(),
            show_abilities.clone(),
        );
        move |_| {
            let options = NpcOptions {
                players_option: rules_choice.selected() == 1,
                abilities: None,
            };
            let result = rules.roll_abilities_only(
                &mut host.rand().borrow_mut(),
                &selected_race(),
                &chosen(),
                &options,
            );
            match result {
                Ok(a) => {
                    show_abilities(Some(&a));
                    *rolled.borrow_mut() = Some(a);
                }
                Err(e) => host.toast(&e.to_string()),
            }
        }
    });
    clear.connect_clicked({
        let (rolled, show_abilities) = (rolled.clone(), show_abilities.clone());
        move |_| {
            *rolled.borrow_mut() = None;
            show_abilities(None);
        }
    });

    create.connect_clicked({
        let (host, dialog) = (host.clone(), dialog.clone());
        move |create| {
            let classes = chosen();
            if classes.is_empty() {
                return host.toast(&rules.no_class_message());
            }
            // The spinners edit what Ability Scores… rolled.
            let abilities = rolled.borrow().map(|mut a| {
                for (i, row) in ability_rows.iter().enumerate() {
                    a.scores[i] = row.value() as i32;
                }
                a.exceptional = if a.scores[0] == 18 {
                    exceptional.value() as i32
                } else {
                    0
                };
                a
            });
            let options = NpcOptions {
                players_option: rules_choice.selected() == 1,
                abilities,
            };
            let npc = rules.generate(
                &mut host.rand().borrow_mut(),
                &selected_race(),
                &classes,
                &options,
            );
            let npc = match npc {
                Ok(npc) => npc,
                Err(e) => return host.toast(&e.to_string()),
            };
            if let Some(chosen) = &template {
                // Clicks made meanwhile create nothing more.
                create.set_sensitive(false);
                dialog.close();
                show_messages(&host.parent(), [&npc]);
                return chosen(&npc);
            }
            let party = party.text().trim().to_owned();
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

/// "~Random" in the dialog's choices.
const RANDOM: &str = tr_noop!("~Random");
/// The class choice that takes no class of a group.
const NO_CLASS: &str = tr_noop!("None");
/// The kit choice that takes no kit.
const NO_KIT: &str = "NONE";

/// The texts a choice row shows for `items`: the first `fixed` are the
/// dialog's own choices ("~Random", "None"), translated; the rest are
/// names from the data. The row's selection is read back from `items`.
fn shown(items: &[String], fixed: usize) -> Vec<String> {
    items
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if i < fixed {
                tr(s).to_owned()
            } else {
                s.clone()
            }
        })
        .collect()
}

/// A choice row's selection: `items[0]` is "~Random".
fn choice(row: &adw::ComboRow, items: &[String]) -> Choice {
    match row.selected() {
        0 => Choice::Random,
        i => items
            .get(i as usize)
            .map_or(Choice::Random, |n| Choice::Name(n.clone())),
    }
}

/// Puts `items` in a choice row as [`shown`] does, keeping the chosen
/// name when it is still there and "~Random" (the first) otherwise.
fn refill(row: &adw::ComboRow, list: &RefCell<Vec<String>>, items: Vec<String>, fixed: usize) {
    let old = list.borrow().get(row.selected() as usize).cloned();
    let at = old
        .and_then(|o| items.iter().position(|i| *i == o))
        .unwrap_or(0);
    let texts = shown(&items, fixed);
    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
    *list.borrow_mut() = items;
    row.set_model(Some(&gtk::StringList::new(&texts)));
    row.set_selected(at as u32);
}

/// Opens the New NPC Group dialog: one or more lines (number, race,
/// classes, kit, alignment and levels), made in the order of the sorted
/// list as Create Group does.
pub fn new_group<H: Host + 'static>(host: &Rc<H>) {
    let Some(rules) = host.npc_rules() else {
        return;
    };
    // Race, kit and alignment take the original's Standard/All,
    // Standard/All and Allowed/Any choices; their lists follow the rest.
    let races: Rc<RefCell<Vec<String>>> = Rc::default();
    let race = combo_row(tr("Race"), &[]);
    let all_races = adw::SwitchRow::builder()
        .title(tr("All Races"))
        .subtitle(tr("Include races that are not standard"))
        .build();
    let fewest = spin_row(tr("Fewest"), 1, 1, 100);
    let most = spin_row(tr("Most"), 1, 1, 100);
    let rules_choice = rules_row();
    let party = adw::EntryRow::builder().title(tr("Group Name")).build();
    let group = adw::PreferencesGroup::builder().title(tr("Group")).build();
    group.add(&party);
    group.add(&race);
    group.add(&all_races);
    group.add(&fewest);
    group.add(&most);
    group.add(&rules_choice);

    let allowed = adw::SwitchRow::builder()
        .title(tr("Allowed Combination"))
        .subtitle(tr("Each NPC takes a class combination its race allows"))
        .active(true)
        .build();
    let classes_group = adw::PreferencesGroup::builder()
        .title(tr("Classes"))
        .description(tr("A class of each group, or none"))
        .build();
    classes_group.add(&allowed);
    let class_rows: Vec<(adw::ComboRow, Vec<String>)> = GROUPS
        .iter()
        .enumerate()
        .map(|(g, name)| {
            let mut items = vec![RANDOM.to_owned(), NO_CLASS.to_owned()];
            items.extend(rules.group_classes(g));
            let row = combo_row(tr(name), &shown(&items, 2));
            // Without a choice only the warrior group takes a class.
            row.set_selected(if g == 0 { 0 } else { 1 });
            row.set_sensitive(false);
            classes_group.add(&row);
            (row, items)
        })
        .collect();
    allowed.connect_active_notify({
        let rows: Vec<adw::ComboRow> = class_rows.iter().map(|r| r.0.clone()).collect();
        move |switch| {
            for row in &rows {
                row.set_sensitive(!switch.is_active());
            }
        }
    });
    let kits: Rc<RefCell<Vec<String>>> = Rc::default();
    let kit = combo_row(tr("Kit"), &[]);
    let all_kits = adw::SwitchRow::builder()
        .title(tr("All Kits"))
        .subtitle(tr(
            "Include kits the race, classes or alignment may not take",
        ))
        .build();
    let alignments: Rc<RefCell<Vec<String>>> = Rc::default();
    let alignment = combo_row(tr("Alignment"), &[]);
    let any_alignment = adw::SwitchRow::builder()
        .title(tr("Any Alignment"))
        .subtitle(tr(
            "Include alignments the classes, race or kit do not allow",
        ))
        .build();
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
            let low = spin_button(1, 1, 30);
            let high = spin_button(1, 1, 30);
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
    let lines: Rc<RefCell<Vec<GroupOptions>>> = Rc::default();
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
    let line_rows: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::default();

    let class_rows_view: Vec<adw::ComboRow> = class_rows.iter().map(|r| r.0.clone()).collect();
    let current = {
        let (fewest, most) = (fewest.clone(), most.clone());
        let (race, races) = (race.clone(), races.clone());
        let (rules_choice, allowed) = (rules_choice.clone(), allowed.clone());
        let (kit, kits) = (kit.clone(), kits.clone());
        let (alignment, alignments) = (alignment.clone(), alignments.clone());
        move || {
            let mut options = GroupOptions {
                number: (fewest.value() as i32, most.value() as i32),
                race: match race.selected() {
                    0 => None,
                    i => races.borrow().get(i as usize).cloned(),
                },
                allowed_combination: allowed.is_active(),
                kit: choice(&kit, &kits.borrow()),
                alignment: choice(&alignment, &alignments.borrow()),
                players_option: rules_choice.selected() == 1,
                ..Default::default()
            };
            for (slot, (row, items)) in options.classes.iter_mut().zip(&class_rows) {
                *slot = match row.selected() {
                    1 => None,
                    _ => Some(choice(row, items)),
                };
            }
            for (slot, (low, high)) in options.levels.iter_mut().zip(&levels) {
                let (low, high) = (low.value() as i32, high.value() as i32);
                *slot = (low.min(high), low.max(high));
            }
            options
        }
    };
    let current = Rc::new(current);

    // 0x4e1cc3: the alignments, which a "~Random" kit holds at "~Random"
    // under Allowed (0x4e3721).
    let fill_alignments = {
        let (rules, current) = (rules.clone(), current.clone());
        let (alignment, alignments) = (alignment.clone(), alignments.clone());
        let any_alignment = any_alignment.clone();
        move || {
            let options = current();
            let mut items = vec![RANDOM.to_owned()];
            items.extend(if any_alignment.is_active() {
                rules.alignments()
            } else {
                rules.group_alignments(&options)
            });
            let locked = !any_alignment.is_active() && matches!(options.kit, Choice::Random);
            refill(&alignment, &alignments, items, 1);
            if locked {
                alignment.set_selected(0);
            }
            alignment.set_sensitive(!locked && alignments.borrow().len() > 1);
        }
    };
    let fill_alignments = Rc::new(fill_alignments);
    // 0x4e15d3, then the alignments.
    let fill_kits = {
        let (rules, current) = (rules.clone(), current.clone());
        let (kit, kits, all_kits) = (kit.clone(), kits.clone(), all_kits.clone());
        let fill_alignments = fill_alignments.clone();
        move || {
            let mut items = vec![RANDOM.to_owned(), NO_KIT.to_owned()];
            items.extend(rules.group_kits(!all_kits.is_active(), &current()));
            refill(&kit, &kits, items, 1);
            fill_alignments();
        }
    };
    let fill_kits = Rc::new(fill_kits);
    // 0x4e095e and 0x4e09ad, then the kits.
    let fill_races = {
        let rules = rules.clone();
        let (race, races, all_races) = (race.clone(), races.clone(), all_races.clone());
        let rules_choice = rules_choice.clone();
        let fill_kits = fill_kits.clone();
        move || {
            let mut items = vec![RANDOM.to_owned()];
            items.extend(if all_races.is_active() {
                rules.races()
            } else {
                rules.group_standard_races(rules_choice.selected() == 1)
            });
            refill(&race, &races, items, 1);
            fill_kits();
        }
    };
    let fill_races = Rc::new(fill_races);
    // Refilling a list notifies its row: the rows follow the person only.
    let filling = Rc::new(std::cell::Cell::new(false));
    let guarded = |fill: Rc<dyn Fn()>| {
        let filling = filling.clone();
        move || {
            if !filling.replace(true) {
                fill();
                filling.set(false);
            }
        }
    };
    let races_changed = guarded(fill_races.clone());
    let kits_changed = guarded(fill_kits.clone());
    let alignments_changed = guarded(fill_alignments.clone());
    races_changed();
    all_races.connect_active_notify({
        let f = races_changed.clone();
        move |_| f()
    });
    rules_choice.connect_selected_notify({
        let f = races_changed.clone();
        move |_| f()
    });
    race.connect_selected_notify({
        let f = kits_changed.clone();
        move |_| f()
    });
    all_kits.connect_active_notify({
        let f = kits_changed.clone();
        move |_| f()
    });
    allowed.connect_active_notify({
        let f = kits_changed.clone();
        move |_| f()
    });
    for row in &class_rows_view {
        row.connect_selected_notify({
            let f = kits_changed.clone();
            move |_| f()
        });
    }
    kit.connect_selected_notify({
        let f = alignments_changed.clone();
        move |_| f()
    });
    any_alignment.connect_active_notify({
        let f = alignments_changed.clone();
        move |_| f()
    });
    add.connect_clicked({
        let (lines, line_rows, lines_group) =
            (lines.clone(), line_rows.clone(), lines_group.clone());
        let current = current.clone();
        move |_| {
            let mut lines = lines.borrow_mut();
            lines.push(current());
            GroupOptions::sort(&mut lines);
            for row in line_rows.borrow_mut().drain(..) {
                lines_group.remove(&row);
            }
            for line in lines.iter() {
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&line.display_label()))
                    .build();
                lines_group.add(&row);
                line_rows.borrow_mut().push(row);
            }
        }
    });

    let page = adw::PreferencesPage::new();
    page.add(&group);
    page.add(&classes_group);
    page.add(&levels_group);
    page.add(&lines_group);
    let create = create_button(tr("_Create"));
    let dialog = dialog(tr("New NPC Group"), &page, &create, topic::NPC_GROUP);
    fewest.connect_value_notify({
        let most = most.clone();
        move |row| {
            if most.value() < row.value() {
                most.set_value(row.value());
            }
        }
    });
    most.connect_value_notify({
        let fewest = fewest.clone();
        move |row| {
            if fewest.value() > row.value() {
                fewest.set_value(row.value());
            }
        }
    });

    create.connect_clicked({
        let (host, dialog) = (host.clone(), dialog.clone());
        move |create| {
            let name = party.text().trim().to_owned();
            if name.is_empty() {
                return host.toast(tr("A group name must be supplied."));
            }
            let mut lines = lines.borrow().clone();
            if lines.is_empty() {
                lines.push(current());
            }
            let npcs = rules.generate_groups(&mut host.rand().borrow_mut(), &lines);
            let npcs = match npcs {
                Ok(npcs) => npcs,
                Err(e) => return host.toast(&e.to_string()),
            };
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
                    host.toast(&trn!(
                        "{n} NPC was created",
                        "{n} NPCs were created",
                        npcs.len()
                    ));
                }
                Some(Err(e)) => host.toast(&e.to_string()),
                None => {}
            }
        }
    });
    dialog.present(Some(&host.parent()));
}
