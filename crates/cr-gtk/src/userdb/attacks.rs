//! A monster's Attacks page (IDD 271): its own attacks, added from the
//! attack types or made with New, changed with the Monster Attack dialog
//! (IDD 269) and removed without asking, as the original does.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::monster::MonsterAttack;
use cr_i18n::tr;
use cr_store::help::topic;
use cr_store::userdb::Value;
use cr_store::userdb::monsters::{
    ADD_HEADING, AREAS, SAVE_RESULTS, SAVES, TO_HIT, add_problem, area_labels, attack_problem,
    attack_summary,
};
use gtk::glib;

use super::Redraw;

type Attacks = Rc<RefCell<Vec<MonsterAttack>>>;

/// The attacks control: one row per attack with Edit and Remove, and Add
/// and New under them. `types` are the attacks Add offers.
pub fn attacks_control(
    types: Vec<MonsterAttack>,
    value: impl Fn() -> Value + 'static,
    changed: Rc<dyn Fn(Value)>,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    root.append(&list);
    let attacks: Attacks = Rc::default();
    let redraw: Redraw = Rc::default();
    // Stores the attacks and shows them again.
    let store: Rc<dyn Fn(Vec<MonsterAttack>)> = Rc::new({
        let (attacks, redraw) = (attacks.clone(), redraw.clone());
        move |now| {
            *attacks.borrow_mut() = now.clone();
            changed(Value::Attacks(now));
            if let Some(redraw) = &*redraw.borrow() {
                redraw();
            }
        }
    });
    let draw: Rc<dyn Fn()> = Rc::new({
        let (list, attacks, store) = (list.clone(), attacks.clone(), store.clone());
        move || {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let now = attacks.borrow().clone();
            if now.is_empty() {
                list.append(
                    &adw::ActionRow::builder()
                        .title(tr("None"))
                        .css_classes(["dim-label"])
                        .build(),
                );
            }
            for (index, attack) in now.iter().enumerate() {
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&attack.name).as_str())
                    .subtitle(glib::markup_escape_text(&attack_summary(attack)).as_str())
                    .build();
                let edit = gtk::Button::builder()
                    .icon_name("document-edit-symbolic")
                    .tooltip_text(tr("Edit"))
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                let remove = gtk::Button::builder()
                    .icon_name("list-remove-symbolic")
                    .tooltip_text(tr("Remove"))
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                row.add_suffix(&edit);
                row.add_suffix(&remove);
                row.set_activatable_widget(Some(&edit));
                edit.connect_clicked({
                    let (attacks, store) = (attacks.clone(), store.clone());
                    move |button| {
                        let Some(attack) = attacks.borrow().get(index).cloned() else {
                            return;
                        };
                        let (check, store) = (attacks.clone(), store.clone());
                        let attacks = attacks.clone();
                        editor(
                            button,
                            attack,
                            move |name| attack_problem(&check.borrow(), Some(index), name),
                            move |attack| {
                                let mut now = attacks.borrow().clone();
                                if let Some(slot) = now.get_mut(index) {
                                    *slot = attack;
                                }
                                store(now);
                            },
                        );
                    }
                });
                remove.connect_clicked({
                    let (attacks, store) = (attacks.clone(), store.clone());
                    move |_| {
                        let mut now = attacks.borrow().clone();
                        if index < now.len() {
                            now.remove(index);
                            store(now);
                        }
                    }
                });
                list.append(&row);
            }
        }
    });
    *redraw.borrow_mut() = Some(draw.clone());

    let buttons = gtk::Box::builder()
        .spacing(12)
        .halign(gtk::Align::Center)
        .build();
    let add = gtk::Button::builder()
        .label(tr("Add…"))
        .css_classes(["pill"])
        .build();
    let new = gtk::Button::builder()
        .label(tr("New…"))
        .css_classes(["pill"])
        .build();
    buttons.append(&add);
    buttons.append(&new);
    root.append(&buttons);
    let types = Rc::new(types);
    add.connect_clicked({
        let (attacks, store, types) = (attacks.clone(), store.clone(), types.clone());
        move |button| {
            let mut names: Vec<String> = types.iter().map(|a| a.name.clone()).collect();
            names.sort_by_cached_key(|n| cr_store::collate::sort_key(n));
            names.dedup();
            let (attacks, store, types) = (attacks.clone(), store.clone(), types.clone());
            let parent = button.clone();
            crate::lists::pick(button, tr("Attacks"), names, move |name| {
                let now = attacks.borrow().clone();
                if let Some(problem) = add_problem(&now, &name) {
                    alert(&parent, tr(ADD_HEADING), problem);
                    return;
                }
                if let Some(attack) = types.iter().find(|a| a.name == name) {
                    let mut now = now;
                    now.push(attack.clone());
                    store(now);
                }
            });
        }
    });
    new.connect_clicked({
        let (attacks, store) = (attacks.clone(), store.clone());
        move |button| {
            let (check, store) = (attacks.clone(), store.clone());
            let attacks = attacks.clone();
            editor(
                button,
                MonsterAttack::default(),
                move |name| attack_problem(&check.borrow(), None, name),
                move |attack| {
                    let mut now = attacks.borrow().clone();
                    now.push(attack);
                    store(now);
                },
            );
        }
    });

    let load = move || {
        let now = value().attacks();
        if *attacks.borrow() != now {
            *attacks.borrow_mut() = now;
            draw();
        } else if list.first_child().is_none() {
            draw();
        }
    };
    (root.upcast(), Box::new(load))
}

/// A message with an OK button.
fn alert(parent: &impl IsA<gtk::Widget>, heading: &str, body: &str) {
    let alert = adw::AlertDialog::builder()
        .heading(heading)
        .body(body)
        .default_response("ok")
        .close_response("ok")
        .build();
    alert.add_response("ok", tr("_OK"));
    alert.present(Some(parent));
}

fn spin(min: f64, max: f64, value: i32) -> gtk::SpinButton {
    gtk::SpinButton::builder()
        .adjustment(&gtk::Adjustment::new(
            f64::from(value),
            min,
            max,
            1.0,
            10.0,
            0.0,
        ))
        .numeric(true)
        .valign(gtk::Align::Center)
        .width_chars(4)
        .build()
}

fn spin_row(title: &str, min: f64, max: f64, value: i32) -> adw::SpinRow {
    adw::SpinRow::builder()
        .title(glib::markup_escape_text(title).as_str())
        .numeric(true)
        .adjustment(&gtk::Adjustment::new(
            f64::from(value),
            min,
            max,
            1.0,
            10.0,
            0.0,
        ))
        .build()
}

fn combo_row(title: &str, items: &[&str], selected: usize) -> adw::ComboRow {
    let items: Vec<&str> = items.iter().map(|i| tr(i)).collect();
    adw::ComboRow::builder()
        .title(glib::markup_escape_text(title).as_str())
        .model(&gtk::StringList::new(&items))
        .selected(selected as u32)
        .build()
}

/// The Monster Attack dialog (IDD 269) on a copy of `attack`. OK asks
/// `problem` about the name first and stays open with its message;
/// otherwise `done` gets the attack, its name trimmed (0x6bbb25).
fn editor(
    parent: &impl IsA<gtk::Widget>,
    attack: MonsterAttack,
    problem: impl Fn(&str) -> Option<&'static str> + 'static,
    done: impl Fn(MonsterAttack) + 'static,
) {
    // The digits-only edits take no minus sign; Bonus Hit Dice does.
    const MAX: f64 = i32::MAX as f64;
    let name = adw::EntryRow::builder()
        .title(tr("Name"))
        .text(attack.name.as_str())
        .build();
    let help_id = spin_row(tr("Help ID"), 0.0, MAX, attack.help_id);
    let dice = spin(0.0, MAX, attack.damage_dice);
    let sides = spin(0.0, MAX, attack.damage_sides);
    let bonus = spin(0.0, MAX, attack.damage_bonus);
    let damage = adw::ActionRow::builder().title(tr("Damage")).build();
    for (i, widget) in [&dice, &sides, &bonus].into_iter().enumerate() {
        if i > 0 {
            damage.add_suffix(&gtk::Label::new(Some(if i == 1 { "d" } else { "+" })));
        }
        damage.add_suffix(widget);
    }
    let to_hit_labels: Vec<&str> = TO_HIT.iter().map(|(_, l)| *l).collect();
    let to_hit = combo_row(
        tr("Must make a to hit roll against"),
        &to_hit_labels,
        TO_HIT
            .iter()
            .position(|(v, _)| *v == attack.to_hit)
            .unwrap_or(0),
    );
    let effect = adw::EntryRow::builder()
        .title(tr("Special Effect"))
        .text(attack.special_effect.as_str())
        .build();
    let hit_dice = spin_row(tr("Bonus Hit Dice"), -MAX, MAX, attack.bonus_hit_dice);
    hit_dice.set_subtitle(tr("Used to calculate modified HD for experience"));
    let main = adw::PreferencesGroup::new();
    for row in [
        name.upcast_ref::<gtk::Widget>(),
        help_id.upcast_ref(),
        damage.upcast_ref(),
        to_hit.upcast_ref(),
        effect.upcast_ref(),
        hit_dice.upcast_ref(),
    ] {
        main.add(row);
    }

    let area = combo_row(
        tr("Type"),
        &AREAS,
        usize::try_from(attack.area)
            .ok()
            .filter(|&a| a < AREAS.len())
            .unwrap_or(0),
    );
    let measures: Vec<adw::SpinRow> = [attack.range, attack.width, attack.end_width, attack.length]
        .into_iter()
        .map(|v| {
            let row = spin_row("", 0.0, MAX, v);
            row.set_subtitle(tr("feet"));
            row
        })
        .collect();
    let shape = adw::PreferencesGroup::builder()
        .title(tr("Area of Effect"))
        .build();
    shape.add(&area);
    for row in &measures {
        shape.add(row);
    }
    // The labels and the rows shown follow the area (0x6bb7a0); hidden
    // rows keep their numbers.
    let relabel = {
        let measures = measures.clone();
        move |area: u32| {
            for (row, label) in measures.iter().zip(area_labels(area as i32)) {
                row.set_visible(label.is_some());
                if let Some(label) = label {
                    row.set_title(&glib::markup_escape_text(tr(label).trim_end_matches(':')));
                }
            }
        }
    };
    relabel(area.selected());
    area.connect_selected_notify(move |row| relabel(row.selected()));

    let save_against = combo_row(
        tr("Save Against"),
        &SAVES,
        usize::try_from(attack.save_against)
            .ok()
            .filter(|&s| s < SAVES.len())
            .unwrap_or(0),
    );
    let modifier = spin_row(tr("Modified by"), 0.0, MAX, attack.save_modifier);
    let result = combo_row(
        tr("To gain this"),
        &SAVE_RESULTS,
        usize::try_from(attack.save_result)
            .ok()
            .filter(|&r| r < SAVE_RESULTS.len())
            .unwrap_or(0),
    );
    let saves = adw::PreferencesGroup::builder()
        .title(tr("Saving Throws"))
        .build();
    saves.add(&save_against);
    saves.add(&modifier);
    saves.add(&result);

    let page = adw::PreferencesPage::new();
    page.add(&main);
    page.add(&shape);
    page.add(&saves);
    let cancel = gtk::Button::with_label(tr("_Cancel"));
    cancel.set_use_underline(true);
    let ok = gtk::Button::builder()
        .label(tr("_OK"))
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
    view.add_top_bar(&header);
    view.set_content(Some(&page));
    crate::help::add_button(&header, &view, || topic::MONSTER_ATTACK);
    let dialog = adw::Dialog::builder()
        .title(tr("Monster Attack"))
        .content_width(520)
        .content_height(640)
        .child(&view)
        .build();
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    let attack = RefCell::new(Some(attack));
    ok.connect_clicked({
        let (dialog, name) = (dialog.clone(), name.clone());
        move |_| {
            let text = name.text();
            if let Some(problem) = problem(&text) {
                alert(&dialog, tr("Monster Attack"), problem);
                return;
            }
            let Some(mut a) = attack.borrow_mut().take() else {
                return;
            };
            let value = |s: &gtk::SpinButton| s.value() as i32;
            text.trim().clone_into(&mut a.name);
            a.help_id = help_id.value() as i32;
            a.damage_dice = value(&dice);
            a.damage_sides = value(&sides);
            a.damage_bonus = value(&bonus);
            a.to_hit = TO_HIT
                .get(to_hit.selected() as usize)
                .map_or(0, |(v, _)| *v);
            effect.text().as_str().clone_into(&mut a.special_effect);
            a.bonus_hit_dice = hit_dice.value() as i32;
            a.area = area.selected() as i32;
            a.range = measures[0].value() as i32;
            a.width = measures[1].value() as i32;
            a.end_width = measures[2].value() as i32;
            a.length = measures[3].value() as i32;
            a.save_against = save_against.selected() as i32;
            a.save_modifier = modifier.value() as i32;
            a.save_result = result.selected() as i32;
            dialog.close();
            done(a);
        }
    });
    dialog.present(Some(parent));
    name.grab_focus();
}
