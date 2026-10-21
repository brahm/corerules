//! The class editor's Level Details lists: free spells (IDD 144), NWPs
//! (IDD 172) and class abilities by level, one row per level, and the
//! Auto Fill button (IDD 224).

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::tr;
use cr_store::userdb::Value;
use cr_store::userdb::classes::levels::{
    self, FREQUENCIES, FREQUENCY_MAX, FreeSpell, FreeSpellLevel, LevelNwps, MODIFICATION_MAX,
};
use gtk::glib;

use super::Redraw;
use super::widgets::{Number, two_list};
use crate::lists::pick_titled;

pub type Parent = Rc<dyn Fn() -> gtk::Widget>;

/// A spell the free spells may give: its name, whether it is a priest
/// spell, and its level.
pub type SpellChoice = (String, bool, i32);

const MAX_LEVEL: i32 = 100;

fn boxed_list() -> gtk::ListBox {
    gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
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

fn spin_row(title: &str, value: i32, min: i32, max: i32) -> adw::SpinRow {
    adw::SpinRow::builder()
        .title(glib::markup_escape_text(title).as_str())
        .adjustment(&gtk::Adjustment::new(
            f64::from(value),
            f64::from(min),
            f64::from(max),
            1.0,
            5.0,
            0.0,
        ))
        .numeric(true)
        .build()
}

fn level_row(level: i32) -> adw::SpinRow {
    spin_row(tr("Level"), level, 1, MAX_LEVEL)
}

/// The lowest level `used` lacks.
fn free_level(used: impl Iterator<Item = i32>) -> i32 {
    let used: Vec<i32> = used.collect();
    (1..=MAX_LEVEL).find(|l| !used.contains(l)).unwrap_or(1)
}

/// A Cancel/OK dialog around `content`; `ok` runs on OK, which closes it.
fn present(
    title: &str,
    content: &gtk::Box,
    height: i32,
    parent: &gtk::Widget,
    ok: impl Fn() + 'static,
) {
    let cancel = gtk::Button::builder()
        .label(tr("_Cancel"))
        .use_underline(true)
        .build();
    let ok_button = gtk::Button::builder()
        .label(tr("_OK"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let header = adw::HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .build();
    header.pack_start(&cancel);
    header.pack_end(&ok_button);
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    view.set_content(Some(
        &gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(content)
            .build(),
    ));
    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(560)
        .content_height(height)
        .child(&view)
        .build();
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    ok_button.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            ok();
            dialog.close();
        }
    });
    dialog.present(Some(parent));
}

fn vbox() -> gtk::Box {
    gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build()
}

fn heading(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["heading"])
        .build()
}

/// Fills `list` with one row per entry (its title and subtitle), each
/// with Edit and Remove, then an Add row.
fn draw_entries(
    list: &gtk::ListBox,
    entries: Vec<(String, String)>,
    add_label: &str,
    edit: &Rc<dyn Fn(Option<usize>)>,
    remove: &Rc<dyn Fn(usize)>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for (i, (title, subtitle)) in entries.into_iter().enumerate() {
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&title).as_str())
            .subtitle(glib::markup_escape_text(&subtitle).as_str())
            .build();
        let edit_button = flat_button("document-edit-symbolic", tr("Edit"));
        let edit = edit.clone();
        edit_button.connect_clicked(move |_| edit(Some(i)));
        let remove_button = flat_button("list-remove-symbolic", tr("Remove"));
        let remove = remove.clone();
        remove_button.connect_clicked(move |_| remove(i));
        row.add_suffix(&edit_button);
        row.add_suffix(&remove_button);
        list.append(&row);
    }
    let add = adw::ActionRow::builder()
        .title(glib::markup_escape_text(add_label).as_str())
        .activatable(true)
        .build();
    add.add_prefix(&gtk::Image::from_icon_name("list-add-symbolic"));
    let edit = edit.clone();
    add.connect_activated(move |_| edit(None));
    list.append(&add);
}

/// Puts `entry` in `entries` in place of the one at `at` and of any other
/// at its level, keeping them by level.
fn put<T>(entries: &mut Vec<T>, at: Option<usize>, entry: T, level: impl Fn(&T) -> i32) {
    if let Some(i) = at
        && i < entries.len()
    {
        entries.remove(i);
    }
    let l = level(&entry);
    entries.retain(|e| level(e) != l);
    let at = entries.partition_point(|e| level(e) < l);
    entries.insert(at, entry);
}

/// The free spells by level ([`cr_store::userdb::FieldKind::FreeSpells`]).
pub fn free_spells_control(
    spells: Rc<Vec<SpellChoice>>,
    value: impl Fn() -> Value + 'static,
    changed: Rc<dyn Fn(Value)>,
    parent: Parent,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let list = boxed_list();
    let state: Rc<RefCell<(String, Vec<FreeSpellLevel>)>> = Rc::default();
    let value = Rc::new(value);
    let draw_slot: Redraw = Rc::default();
    let load: Rc<dyn Fn()> = Rc::new({
        let (state, value, draw_slot) = (state.clone(), value.clone(), draw_slot.clone());
        move || {
            if let Value::FreeSpells(class, levels) = value() {
                *state.borrow_mut() = (class, levels);
            }
            if let Some(draw) = draw_slot.borrow().clone() {
                draw();
            }
        }
    });
    let commit: Rc<dyn Fn()> = Rc::new({
        let (state, load) = (state.clone(), load.clone());
        move || {
            let (class, levels) = state.borrow().clone();
            changed(Value::FreeSpells(class, levels));
            load();
        }
    });
    let edit: Rc<dyn Fn(Option<usize>)> = Rc::new({
        let (state, commit) = (state.clone(), commit.clone());
        move |at: Option<usize>| {
            let start = at
                .and_then(|i| state.borrow().1.get(i).cloned())
                .unwrap_or_else(|| {
                    FreeSpellLevel::new(free_level(state.borrow().1.iter().map(|l| l.level)))
                });
            let (state, commit) = (state.clone(), commit.clone());
            free_spells_dialog(&spells, start, &parent(), move |level| {
                put(&mut state.borrow_mut().1, at, level, |l| l.level);
                commit();
            });
        }
    });
    let remove: Rc<dyn Fn(usize)> = Rc::new({
        let (state, commit) = (state.clone(), commit.clone());
        move |i| {
            state.borrow_mut().1.remove(i);
            commit();
        }
    });
    *draw_slot.borrow_mut() = Some(Rc::new({
        let (list, state) = (list.clone(), state.clone());
        move || {
            let entries = {
                let (class, levels) = &*state.borrow();
                levels
                    .iter()
                    .map(|l| {
                        (
                            tr!("Level {level}", level = l.level),
                            levels::free_spell_lines(class, l).join("\n"),
                        )
                    })
                    .collect()
            };
            draw_entries(&list, entries, tr("Add Free Spells…"), &edit, &remove);
        }
    }));
    (list.upcast(), Box::new(move || load()))
}

/// Free Spells (IDD 144) for one level.
fn free_spells_dialog(
    spells: &Rc<Vec<SpellChoice>>,
    start: FreeSpellLevel,
    parent: &gtk::Widget,
    done: impl Fn(FreeSpellLevel) + 'static,
) {
    let content = vbox();
    let top = boxed_list();
    let level = level_row(start.level);
    let uses = spin_row(tr("The spells may be cast this often"), start.uses, 0, 1000);
    let frequency = adw::EntryRow::builder()
        .title(tr("Frequency"))
        .text(start.frequency.as_str())
        .build();
    let choose = flat_button("view-list-symbolic", tr("Choose"));
    choose.connect_clicked({
        let (frequency, parent) = (frequency.clone(), parent.clone());
        move |_| {
            let frequency = frequency.clone();
            let shown: Vec<String> = FREQUENCIES.iter().map(|f| tr(f).to_owned()).collect();
            pick_titled(&parent, tr("Frequency"), shown, move |chosen| {
                // Stored as the original writes them.
                let f = FREQUENCIES
                    .iter()
                    .find(|f| tr(f) == chosen)
                    .map_or(chosen.as_str(), |f| *f);
                frequency.set_text(f);
            });
        }
    });
    frequency.add_suffix(&choose);
    let pick_one = adw::SwitchRow::builder()
        .title(tr("Must select one spell from the list"))
        .active(start.pick_one)
        .build();
    top.append(&level);
    top.append(&uses);
    top.append(&frequency);
    top.append(&pick_one);
    content.append(&top);

    content.append(&heading(tr("Spells")));
    let chosen = boxed_list();
    content.append(&chosen);
    let picked = Rc::new(RefCell::new(start.spells.clone()));
    let draw_slot: Redraw = Rc::default();
    let draw: Rc<dyn Fn()> = Rc::new({
        let (chosen, picked, draw_slot) = (chosen.clone(), picked.clone(), draw_slot.clone());
        move || {
            while let Some(child) = chosen.first_child() {
                chosen.remove(&child);
            }
            for (i, spell) in picked.borrow().iter().enumerate() {
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&spell.name).as_str())
                    .subtitle(if spell.wizard {
                        tr("As the Wizard Spell")
                    } else {
                        tr("Priest")
                    })
                    .build();
                let count = gtk::SpinButton::with_range(0.0, 1000.0, 1.0);
                count.set_value(f64::from(spell.uses));
                count.set_valign(gtk::Align::Center);
                count.set_tooltip_text(Some(tr("Uses")));
                count.connect_value_changed({
                    let picked = picked.clone();
                    move |s| {
                        if let Some(spell) = picked.borrow_mut().get_mut(i) {
                            spell.uses = s.value() as i32;
                        }
                    }
                });
                let remove = flat_button("list-remove-symbolic", tr("Remove"));
                remove.connect_clicked({
                    let (picked, draw_slot) = (picked.clone(), draw_slot.clone());
                    move |_| {
                        picked.borrow_mut().remove(i);
                        if let Some(draw) = draw_slot.borrow().clone() {
                            draw();
                        }
                    }
                });
                row.add_suffix(&count);
                row.add_suffix(&remove);
                chosen.append(&row);
            }
            chosen.set_visible(!picked.borrow().is_empty());
        }
    });
    *draw_slot.borrow_mut() = Some(draw.clone());
    draw();

    // What Add offers: one type, one level or all (IDD 144's radio
    // buttons, Priest first, and its level spin).
    let add_group = boxed_list();
    let kind = adw::ComboRow::builder()
        .title(tr("Spell type"))
        .model(&gtk::StringList::new(&[tr("Priest"), tr("Wizard")]))
        .build();
    let spell_level = spin_row(tr("Spell level"), 0, 0, 10);
    spell_level.set_subtitle(tr("Set to 0 for all"));
    let add = adw::ActionRow::builder()
        .title(tr("Add a Spell…"))
        .activatable(true)
        .build();
    add.add_prefix(&gtk::Image::from_icon_name("list-add-symbolic"));
    add.connect_activated({
        let (spells, parent, picked, kind, spell_level, uses, draw) = (
            spells.clone(),
            parent.clone(),
            picked.clone(),
            kind.clone(),
            spell_level.clone(),
            uses.clone(),
            draw.clone(),
        );
        move |_| {
            let wizard = kind.selected() == 1;
            let at = spell_level.value() as i32;
            let mut names: Vec<String> = spells
                .iter()
                .filter(|(_, priest, level)| *priest != wizard && (at == 0 || *level == at))
                .map(|(n, _, _)| n.clone())
                .collect();
            names.dedup();
            let (picked, uses, draw) = (picked.clone(), uses.clone(), draw.clone());
            pick_titled(&parent, tr("Add a Spell…"), names, move |name| {
                let taken = picked
                    .borrow()
                    .iter()
                    .any(|s| s.name == name && s.wizard == wizard);
                if !taken {
                    picked.borrow_mut().push(FreeSpell {
                        name,
                        uses: uses.value() as i32,
                        wizard,
                    });
                    draw();
                }
            });
        }
    });
    add_group.append(&kind);
    add_group.append(&spell_level);
    add_group.append(&add);
    content.append(&add_group);

    present(tr("Free Spells"), &content, 600, parent, move || {
        done(FreeSpellLevel {
            level: level.value() as i32,
            uses: uses.value() as i32,
            frequency: frequency.text().chars().take(FREQUENCY_MAX).collect(),
            pick_one: pick_one.is_active(),
            spells: picked.borrow().clone(),
            unknown_0x4: start.unknown_0x4.clone(),
        });
    });
}

/// The NWPs by level ([`cr_store::userdb::FieldKind::LevelNwps`]).
pub fn level_nwps_control(
    available: Vec<String>,
    value: impl Fn() -> Value + 'static,
    changed: Rc<dyn Fn(Value)>,
    parent: Parent,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let list = boxed_list();
    let state: Rc<RefCell<Vec<LevelNwps>>> = Rc::default();
    let available = Rc::new(available);
    let value = Rc::new(value);
    let draw_slot: Redraw = Rc::default();
    let load: Rc<dyn Fn()> = Rc::new({
        let (state, value, draw_slot) = (state.clone(), value.clone(), draw_slot.clone());
        move || {
            *state.borrow_mut() = value().level_nwps();
            if let Some(draw) = draw_slot.borrow().clone() {
                draw();
            }
        }
    });
    let commit: Rc<dyn Fn()> = Rc::new({
        let (state, load) = (state.clone(), load.clone());
        move || {
            changed(Value::LevelNwps(state.borrow().clone()));
            load();
        }
    });
    let edit: Rc<dyn Fn(Option<usize>)> = Rc::new({
        let (state, commit) = (state.clone(), commit.clone());
        move |at: Option<usize>| {
            let start = at
                .and_then(|i| state.borrow().get(i).cloned())
                .unwrap_or_else(|| LevelNwps {
                    level: free_level(state.borrow().iter().map(|l| l.level)),
                    ..LevelNwps::default()
                });
            let (state, commit) = (state.clone(), commit.clone());
            level_nwps_dialog(&available, start, &parent(), move |level| {
                put(&mut state.borrow_mut(), at, level, |l| l.level);
                commit();
            });
        }
    });
    let remove: Rc<dyn Fn(usize)> = Rc::new({
        let (state, commit) = (state.clone(), commit.clone());
        move |i| {
            state.borrow_mut().remove(i);
            commit();
        }
    });
    *draw_slot.borrow_mut() = Some(Rc::new({
        let (list, state) = (list.clone(), state.clone());
        move || {
            let entries = state
                .borrow()
                .iter()
                .map(|l| {
                    (
                        tr!("Level {level}", level = l.level),
                        levels::level_nwps_summary(l),
                    )
                })
                .collect();
            draw_entries(&list, entries, tr("Add NWPs…"), &edit, &remove);
        }
    }));
    (list.upcast(), Box::new(move || load()))
}

/// NWPs (IDD 172) for one level.
fn level_nwps_dialog(
    available: &Rc<Vec<String>>,
    start: LevelNwps,
    parent: &gtk::Widget,
    done: impl Fn(LevelNwps) + 'static,
) {
    let content = vbox();
    let top = boxed_list();
    let level = level_row(start.level);
    top.append(&level);
    content.append(&top);

    content.append(&heading(tr("Bonus NWPs granted at this level")));
    let granted = Rc::new(RefCell::new(start.granted.clone()));
    let lists = two_list(
        available.clone(),
        None,
        Rc::new({
            let granted = granted.clone();
            move |l| *granted.borrow_mut() = l.into_iter().map(|(n, _)| n).collect()
        }),
    );
    *lists.selected.borrow_mut() = start.granted.iter().map(|n| (n.clone(), 0)).collect();
    (lists.redraw)();
    content.append(&lists.widget);
    let pick_one = adw::SwitchRow::builder()
        .title(tr("May only pick one from the list above"))
        .active(start.pick_one)
        .build();
    let check = boxed_list();
    check.append(&pick_one);
    content.append(&check);

    content.append(&heading(tr("NWP score modifications")));
    let modified = Rc::new(RefCell::new(start.modifications.clone()));
    let lists = two_list(
        available.clone(),
        Some(Number {
            label: tr("Modification"),
            min: 0,
            max: i64::from(MODIFICATION_MAX),
        }),
        Rc::new({
            let modified = modified.clone();
            move |l| *modified.borrow_mut() = l.into_iter().map(|(n, m)| (n, m as i32)).collect()
        }),
    );
    *lists.selected.borrow_mut() = start
        .modifications
        .iter()
        .map(|(n, m)| (n.clone(), i64::from(*m)))
        .collect();
    (lists.redraw)();
    content.append(&lists.widget);

    present(tr("NWPs"), &content, 680, parent, move || {
        done(LevelNwps {
            level: level.value() as i32,
            granted: granted.borrow().clone(),
            pick_one: pick_one.is_active(),
            modifications: modified.borrow().clone(),
        });
    });
}

/// The class ability each level grants
/// ([`cr_store::userdb::FieldKind::LevelAbilities`]).
pub fn level_abilities_control(
    available: Vec<String>,
    value: impl Fn() -> Value + 'static,
    changed: Rc<dyn Fn(Value)>,
    parent: Parent,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let list = boxed_list();
    let state: Rc<RefCell<Vec<(String, i64)>>> = Rc::default();
    let available = Rc::new(available);
    let value = Rc::new(value);
    let draw_slot: Redraw = Rc::default();
    let load: Rc<dyn Fn()> = Rc::new({
        let (state, value, draw_slot) = (state.clone(), value.clone(), draw_slot.clone());
        move || {
            *state.borrow_mut() = value().pairs();
            if let Some(draw) = draw_slot.borrow().clone() {
                draw();
            }
        }
    });
    let commit: Rc<dyn Fn()> = Rc::new({
        let (state, load) = (state.clone(), load.clone());
        move || {
            changed(Value::Pairs(state.borrow().clone()));
            load();
        }
    });
    let edit: Rc<dyn Fn(Option<usize>)> = Rc::new({
        let (state, commit) = (state.clone(), commit.clone());
        move |at: Option<usize>| {
            let (name, level) = at
                .and_then(|i| state.borrow().get(i).cloned())
                .unwrap_or_else(|| {
                    let level = free_level(state.borrow().iter().map(|(_, l)| *l as i32));
                    (String::new(), i64::from(level))
                });
            let (state, commit) = (state.clone(), commit.clone());
            ability_dialog(&available, &name, level, &parent(), move |entry| {
                put(&mut state.borrow_mut(), at, entry, |(_, l)| *l as i32);
                commit();
            });
        }
    });
    let remove: Rc<dyn Fn(usize)> = Rc::new({
        let (state, commit) = (state.clone(), commit.clone());
        move |i| {
            state.borrow_mut().remove(i);
            commit();
        }
    });
    *draw_slot.borrow_mut() = Some(Rc::new({
        let (list, state) = (list.clone(), state.clone());
        move || {
            let entries = state
                .borrow()
                .iter()
                .map(|(name, level)| (tr!("Level {level}", level), name.clone()))
                .collect();
            draw_entries(&list, entries, tr("Add a Class Ability…"), &edit, &remove);
        }
    }));
    (list.upcast(), Box::new(move || load()))
}

/// The class ability one level grants: the original's Add/Remove dialog
/// with one selection.
fn ability_dialog(
    available: &Rc<Vec<String>>,
    name: &str,
    level: i64,
    parent: &gtk::Widget,
    done: impl Fn((String, i64)) + 'static,
) {
    let content = vbox();
    let prompt = gtk::Label::builder()
        .label(levels::ability_prompt(level))
        .xalign(0.0)
        .wrap(true)
        .build();
    content.append(&prompt);
    let top = boxed_list();
    let level_spin = level_row(level as i32);
    level_spin.connect_value_notify({
        let prompt = prompt.clone();
        move |s| prompt.set_label(&levels::ability_prompt(s.value() as i64))
    });
    let names: Vec<&str> = available.iter().map(String::as_str).collect();
    let ability = adw::ComboRow::builder()
        .title(tr("Class Ability"))
        .model(&gtk::StringList::new(&names))
        .enable_search(names.len() > 12)
        .build();
    if let Some(i) = available.iter().position(|n| n == name) {
        ability.set_selected(i as u32);
    }
    top.append(&level_spin);
    top.append(&ability);
    content.append(&top);
    let available = available.clone();
    present(tr("Class Abilities"), &content, 320, parent, move || {
        if let Some(name) = available.get(ability.selected() as usize) {
            done((name.clone(), level_spin.value() as i64));
        }
    });
}

/// Auto Fill ([`cr_store::userdb::FieldKind::AutoFill`]): picks a class
/// (IDD 224), asks before overwriting the page, and gives the class's
/// name to the setter.
pub fn auto_fill_control(
    classes: Vec<String>,
    changed: Rc<dyn Fn(Value)>,
    parent: Parent,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let list = boxed_list();
    let row = adw::ActionRow::builder()
        .title(tr("Copy from another class…"))
        .subtitle(tr("Fills this page with what another class has on it"))
        .activatable(true)
        .build();
    row.add_prefix(&gtk::Image::from_icon_name("edit-copy-symbolic"));
    row.connect_activated(move |_| {
        let (changed, parent2) = (changed.clone(), parent.clone());
        pick_titled(
            &parent(),
            tr("Select a class to copy"),
            classes.clone(),
            move |class| {
                let alert = adw::AlertDialog::builder()
                    .heading(tr(levels::AUTO_FILL_HEADING))
                    // The original breaks the first line where its message
                    // box was narrow.
                    .body(levels::auto_fill_question(&class).replacen('\n', " ", 1))
                    .default_response("no")
                    .close_response("no")
                    .build();
                alert.add_responses(&[("no", tr("_No")), ("yes", tr("_Yes"))]);
                alert.set_response_appearance("yes", adw::ResponseAppearance::Destructive);
                let changed = changed.clone();
                alert.connect_response(None, move |_, response| {
                    if response == "yes" {
                        changed(Value::Text(class.clone()));
                    }
                });
                alert.present(Some(&parent2()));
            },
        );
    });
    list.append(&row);
    (list.upcast(), Box::new(|| ()))
}
