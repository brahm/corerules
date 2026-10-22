//! Enter Character's own dialogs (the wizard's mode 0x379): "All
//! Classes..." on the race/class page (IDD 193), and on the Name page the
//! classes' levels with the Levels dialog (IDD 368) and the hit point and
//! PSP arrows that stand in for the rolls.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use adw::prelude::*;
use cr_formats::character::Character;
use cr_i18n::{tr, tr_noop};
use cr_store::advance::{AdvanceRules, adjust_psionic_points, psionic_total};
use cr_store::create::{Dice, Entered};
use cr_store::help::topic;
use gtk::glib;

/// The groups "All Classes..." picks one class from.
const GROUPS: [&str; 5] = [
    tr_noop!("Warriors"),
    tr_noop!("Rogues"),
    tr_noop!("Priests"),
    tr_noop!("Wizards"),
    tr_noop!("Psionicists"),
];

/// A dialog with `groups` below a header bar that has `done` at its end.
fn dialog(
    title: &str,
    groups: &[&adw::PreferencesGroup],
    done: &gtk::Button,
    help: u32,
) -> adw::Dialog {
    let page = adw::PreferencesPage::new();
    for group in groups {
        page.add(*group);
    }
    let header = adw::HeaderBar::new();
    header.pack_end(done);
    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, move || help);
    view.add_top_bar(&header);
    view.set_content(Some(&page));
    adw::Dialog::builder()
        .title(title)
        .content_width(440)
        .child(&view)
        .default_widget(done)
        .build()
}

/// "All Classes..." (0x5b52d9): one class of each group, or none, for a
/// combination the race's list does not have; `chosen` gets the classes
/// in the groups' order.
pub(crate) fn all_classes(
    parent: &impl IsA<gtk::Widget>,
    groups: [Vec<String>; 5],
    chosen: impl Fn(Vec<String>) + 'static,
) {
    let group = adw::PreferencesGroup::builder()
        .description(tr(
            "Select the Class or combination of Classes for your character and click on the \
             Close button",
        ))
        .build();
    let rows: Vec<(adw::ComboRow, Vec<String>)> = GROUPS
        .iter()
        .zip(groups)
        .map(|(title, classes)| {
            let mut items = vec![tr("None").to_owned()];
            items.extend(classes.iter().cloned());
            let row = crate::create::combo_row(tr(title), &items, &items[0]);
            row.set_visible(!classes.is_empty());
            group.add(&row);
            (row, classes)
        })
        .collect();
    let close = gtk::Button::builder()
        .label(tr("_Close"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let dialog = dialog(
        tr("Allowed Multiclasses"),
        &[&group],
        &close,
        topic::RACE_AND_CLASS,
    );
    close.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            let classes: Vec<String> = rows
                .iter()
                .filter_map(|(row, classes)| {
                    (row.selected() as usize)
                        .checked_sub(1)
                        .and_then(|i| classes.get(i).cloned())
                })
                .collect();
            dialog.close();
            if !classes.is_empty() {
                chosen(classes);
            }
        }
    });
    dialog.present(Some(parent));
}

/// What keeps the levels in the draft: the Levels dialog's results and
/// each class's level.
pub(crate) type Changed = dyn Fn(&Entered, Vec<u32>);

/// What the Name page's levels work on: the character as it would be
/// saved, and the Levels dialog's choices; `changed` gets what to keep in
/// the draft after every change, with each class's level.
pub(crate) struct Levels {
    pub advance: Arc<AdvanceRules>,
    pub character: Rc<RefCell<Character>>,
    pub entered: RefCell<Entered>,
    pub changed: Box<Changed>,
}

impl Levels {
    pub(crate) fn store(&self) {
        let character = self.character.borrow();
        let mut entered = self.entered.borrow_mut();
        entered.take(&character);
        let levels = self
            .advance
            .levels(&character)
            .map(|l| l.iter().map(|c| c.level).collect())
            .unwrap_or_default();
        (self.changed)(&entered, levels);
    }

    fn psionic(&self) -> bool {
        let character = self.character.borrow();
        character
            .data
            .classes
            .names
            .iter()
            .any(|c| self.advance.psionicist(c))
            || psionic_total(&character) > 0
    }
}

/// The Name page's "Level..." button 1613 with each class's level, and
/// the hit point (2372, 1 to 999) and PSP (2373, 0 to 999) arrows in
/// place of the roll buttons (0x5bc9dc, 0x5bcaa3). Returns the group and
/// what shows the character again.
pub(crate) fn levels_group(levels: Rc<Levels>) -> (adw::PreferencesGroup, Rc<dyn Fn()>) {
    let open = gtk::Button::builder()
        .label(tr("Levels…"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let group = adw::PreferencesGroup::builder()
        .title(tr("Levels"))
        .header_suffix(&open)
        .build();
    let class_rows: Vec<adw::ActionRow> = levels
        .character
        .borrow()
        .data
        .classes
        .names
        .iter()
        .map(|name| {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(name).as_str())
                .build();
            group.add(&row);
            row
        })
        .collect();
    let spin = |title: &str, low: f64| {
        adw::SpinRow::builder()
            .title(title)
            .numeric(true)
            .adjustment(&gtk::Adjustment::new(low, low, 999.0, 1.0, 10.0, 0.0))
            .build()
    };
    let hit_points = spin(tr("Hit Points"), 1.0);
    hit_points.set_subtitle(tr("Changes the rolls of the highest level"));
    let psionic = spin(tr("PSPs"), 0.0);
    psionic.set_subtitle(tr("Changes those of the highest level that has them"));
    group.add(&hit_points);
    group.add(&psionic);
    let showing = Rc::new(Cell::new(false));
    let refresh: Rc<dyn Fn()> = Rc::new({
        let (levels, hit_points, psionic, showing) = (
            levels.clone(),
            hit_points.clone(),
            psionic.clone(),
            showing.clone(),
        );
        move || {
            let character = levels.character.borrow();
            if let Ok(classes) = levels.advance.levels(&character) {
                for (row, class) in class_rows.iter().zip(classes) {
                    row.set_subtitle(&crate::level_text(&class));
                }
            }
            showing.set(true);
            match levels.advance.hit_points(&character) {
                Ok(hp) => {
                    hit_points.set_sensitive(true);
                    hit_points.set_value(f64::from(hp));
                }
                Err(_) => hit_points.set_sensitive(false),
            }
            psionic.set_value(f64::from(psionic_total(&character)));
            showing.set(false);
            drop(character);
            psionic.set_visible(levels.psionic());
        }
    });
    refresh();
    hit_points.connect_value_notify({
        let (levels, refresh, showing) = (levels.clone(), refresh.clone(), showing.clone());
        move |row| {
            if showing.get() {
                return;
            }
            let wanted = row.value() as i32;
            let changed = {
                let mut character = levels.character.borrow_mut();
                levels.advance.hit_points(&character).and_then(|hp| {
                    levels
                        .advance
                        .adjust_hit_points(&mut character, wanted - hp)
                })
            };
            if changed.is_ok() {
                levels.store();
            }
            refresh();
        }
    });
    psionic.connect_value_notify({
        let (levels, refresh) = (levels.clone(), refresh.clone());
        move |row| {
            if showing.get() {
                return;
            }
            {
                let mut character = levels.character.borrow_mut();
                let delta = row.value() as i64 - i64::from(psionic_total(&character));
                adjust_psionic_points(&mut character, delta as i32);
            }
            levels.store();
            refresh();
        }
    });
    open.connect_clicked({
        let refresh = refresh.clone();
        move |button| {
            let options = {
                let entered = levels.entered.borrow();
                (entered.prime_bonus, entered.points_per_level)
            };
            let closed = {
                let (levels, refresh) = (levels.clone(), refresh.clone());
                move |prime, per_level| {
                    {
                        let mut entered = levels.entered.borrow_mut();
                        entered.prime_bonus = prime;
                        entered.points_per_level = per_level;
                    }
                    levels.store();
                    refresh();
                }
            };
            levels_dialog(button, &levels.advance, &levels.character, options, closed);
        }
    });
    (group, refresh)
}

/// The Levels dialog (0x616c00), which Enter Character's and Update's
/// Name pages open alike: each class's level from 1 to 100 or its
/// experience, the prime requisite bonus, the level limits and, with
/// Skills & Powers, the character points. `options` are the prime
/// requisite bonus and the character points per level it opens with;
/// `closed` gets them once it closes, with the abilities the levels grant
/// given (0x61990a).
pub(crate) fn levels_dialog(
    parent: &impl IsA<gtk::Widget>,
    advance: &Arc<AdvanceRules>,
    character: &Rc<RefCell<Character>>,
    (prime, per_level): (bool, u32),
    closed: impl Fn(bool, u32) + 'static,
) {
    let (names, skills_and_powers, limits) = {
        let character = character.borrow();
        (
            character.data.classes.names.clone(),
            character.skills_and_powers(),
            character.data.unknown_0x1dc4.min(2),
        )
    };
    let add = gtk::Button::builder()
        .label(tr("Add Experience…"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let classes = adw::PreferencesGroup::builder()
        .title(tr("Classes"))
        .description(tr(
            "A level puts the class at the experience it takes; each level gained rolls its hit \
             points and PSPs",
        ))
        .header_suffix(&add)
        .build();
    let level_rows: Vec<adw::SpinRow> = names
        .iter()
        .map(|name| {
            let row = adw::SpinRow::builder()
                .title(glib::markup_escape_text(name).as_str())
                .numeric(true)
                .adjustment(&gtk::Adjustment::new(1.0, 1.0, 100.0, 1.0, 5.0, 0.0))
                .build();
            classes.add(&row);
            row
        })
        .collect();

    let options = adw::PreferencesGroup::new();
    let prime_row = adw::SwitchRow::builder()
        .title(tr("Apply Prime Requisite Experience Bonuses"))
        .subtitle(tr(
            "10% more for a class whose prime requisites are all 16 or more",
        ))
        .active(prime)
        .build();
    options.add(&prime_row);
    let limits_row = adw::ComboRow::builder()
        .title(tr("Level Limits"))
        .model(&gtk::StringList::new(&[
            tr("None"),
            tr("Racial Limits"),
            tr("Racial Limits and Prime Requisite Bonus"),
        ]))
        .selected(limits)
        .visible(advance.expansion())
        .build();
    options.add(&limits_row);
    let points_row = adw::SpinRow::builder()
        .title(tr("Character Points"))
        .subtitle(tr("Unspent"))
        .numeric(true)
        .adjustment(&gtk::Adjustment::new(0.0, 0.0, 9999.0, 1.0, 10.0, 0.0))
        .visible(skills_and_powers)
        .build();
    options.add(&points_row);
    let per_level_row = adw::SpinRow::builder()
        .title(tr("Character Points per Level"))
        .numeric(true)
        .adjustment(&gtk::Adjustment::new(
            per_level.into(),
            0.0,
            10.0,
            1.0,
            1.0,
            0.0,
        ))
        .visible(skills_and_powers)
        .build();
    options.add(&per_level_row);

    let showing = Rc::new(Cell::new(false));
    let refresh = Rc::new({
        let (advance, character, level_rows, points_row, showing) = (
            advance.clone(),
            character.clone(),
            level_rows.clone(),
            points_row.clone(),
            showing.clone(),
        );
        move || {
            let character = character.borrow();
            showing.set(true);
            if let Ok(classes) = advance.levels(&character) {
                for (row, class) in level_rows.iter().zip(classes) {
                    row.set_value(class.level.into());
                    row.set_subtitle(&crate::level_text(&class));
                }
            }
            points_row.set_value(character.data.unknown_0x1778[0].into());
            showing.set(false);
        }
    });
    refresh();

    let close = gtk::Button::builder()
        .label(tr("_Close"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let dialog = dialog(tr("Levels"), &[&classes, &options], &close, topic::LEVELS);
    for (g, row) in level_rows.iter().enumerate() {
        let (advance, character) = (advance.clone(), character.clone());
        let (refresh, showing) = (refresh.clone(), showing.clone());
        let (dialog, per_level_row) = (dialog.clone(), per_level_row.clone());
        row.connect_value_notify(move |row| {
            if showing.get() {
                return;
            }
            let result = advance.set_level(
                &mut character.borrow_mut(),
                g,
                row.value() as u32,
                per_level_row.value() as u32,
                &mut Dice::default(),
            );
            refresh();
            match result {
                Ok(Some(text)) => crate::create::message(&dialog, tr("Levels"), &text),
                Ok(None) => {}
                Err(e) => crate::create::message(&dialog, tr("Levels"), &e.to_string()),
            }
        });
    }
    limits_row.connect_selected_notify({
        let (character, refresh) = (character.clone(), refresh.clone());
        move |row| {
            character.borrow_mut().data.unknown_0x1dc4 = row.selected();
            refresh();
        }
    });
    points_row.connect_value_notify({
        let (character, showing) = (character.clone(), showing.clone());
        move |row| {
            if !showing.get() {
                character.borrow_mut().data.unknown_0x1778[0] = row.value() as u32;
            }
        }
    });
    add.connect_clicked({
        let (advance, character, refresh) = (advance.clone(), character.clone(), refresh.clone());
        let (prime_row, per_level_row) = (prime_row.clone(), per_level_row.clone());
        move |button| {
            let refresh = refresh.clone();
            crate::experience_dialog(
                button,
                &advance,
                &character,
                Some((prime_row.is_active(), per_level_row.value() as u32)),
                move || refresh(),
            );
        }
    });
    close.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    let advance = advance.clone();
    let character = character.clone();
    dialog.connect_closed(move |_| {
        advance.grant_level_abilities(&mut character.borrow_mut());
        closed(prime_row.is_active(), per_level_row.value() as u32);
    });
    dialog.present(Some(parent));
}
