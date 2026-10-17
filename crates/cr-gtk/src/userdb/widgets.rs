//! Editor controls beyond plain rows: the originals' Available ⇄ Selected
//! lists (with an optional number per selected name), checks gating a
//! number or a list, choose-N-of groups and per-level tables.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::tr;
use cr_store::userdb::Value;
use gtk::glib;

use super::Redraw;

/// Selected names with their numbers.
type Names = Vec<(String, i64)>;
type Groups = Rc<RefCell<Vec<(i64, Vec<String>)>>>;

/// What a two-list shows next to each selected name.
#[derive(Debug, Clone, Copy)]
pub struct Number {
    pub label: &'static str,
    pub min: i64,
    pub max: i64,
}

/// The Available ⇄ Selected lists of the originals: the names not yet
/// chosen on the left (with a search entry), the chosen ones on the right,
/// and arrow buttons between them; activating a row moves it too. With
/// `number`, each selected name has a number beside it.
pub struct TwoList {
    pub widget: gtk::Widget,
    /// The selected names and their numbers, in order.
    pub selected: Rc<RefCell<Vec<(String, i64)>>>,
    /// Shows `selected` again after it was changed from outside.
    pub redraw: Rc<dyn Fn()>,
}

fn list_box() -> gtk::ListBox {
    gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Multiple)
        .activate_on_single_click(false)
        .css_classes(["boxed-list"])
        .build()
}

fn column(title: &str, search: Option<&gtk::SearchEntry>, list: &gtk::ListBox) -> gtk::Box {
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .hexpand(true)
        .build();
    column.append(
        &gtk::Label::builder()
            .label(title)
            .xalign(0.0)
            .css_classes(["heading"])
            .build(),
    );
    if let Some(search) = search {
        column.append(search);
    }
    column.append(
        &gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_height(180)
            .max_content_height(260)
            .propagate_natural_height(true)
            .child(list)
            .build(),
    );
    column
}

fn name_row(name: &str) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_child(Some(
        &gtk::Label::builder()
            .label(name)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .margin_start(8)
            .margin_end(8)
            .margin_top(6)
            .margin_bottom(6)
            .build(),
    ));
    row.set_widget_name(name);
    row
}

pub fn two_list(
    available: Rc<Vec<String>>,
    number: Option<Number>,
    on_change: Rc<dyn Fn(Names)>,
) -> TwoList {
    let selected: Rc<RefCell<Vec<(String, i64)>>> = Rc::default();
    let left = list_box();
    let right = list_box();
    let search = gtk::SearchEntry::builder()
        .placeholder_text(tr("Search"))
        .build();
    let draw: Rc<dyn Fn()> = Rc::new({
        let (available, selected, left, right, search, on_change) = (
            available.clone(),
            selected.clone(),
            left.clone(),
            right.clone(),
            search.clone(),
            on_change.clone(),
        );
        move || {
            while let Some(child) = left.first_child() {
                left.remove(&child);
            }
            while let Some(child) = right.first_child() {
                right.remove(&child);
            }
            let filter = search.text().to_lowercase();
            let chosen = selected.borrow().clone();
            for name in available.iter() {
                if chosen.iter().any(|(n, _)| n == name)
                    || (!filter.is_empty() && !name.to_lowercase().contains(&filter))
                {
                    continue;
                }
                left.append(&name_row(name));
            }
            for (i, (name, value)) in chosen.iter().enumerate() {
                let row = name_row(name);
                if let Some(number) = number {
                    let line = gtk::Box::builder().spacing(6).build();
                    let label = row.child().expect("a name row has a label");
                    row.set_child(None::<&gtk::Widget>);
                    label.set_hexpand(true);
                    line.append(&label);
                    let spin = gtk::SpinButton::builder()
                        .adjustment(&gtk::Adjustment::new(
                            *value as f64,
                            number.min as f64,
                            number.max as f64,
                            1.0,
                            5.0,
                            0.0,
                        ))
                        .numeric(true)
                        .valign(gtk::Align::Center)
                        .tooltip_text(number.label)
                        .width_chars(4)
                        .build();
                    let (selected, on_change) = (selected.clone(), on_change.clone());
                    spin.connect_value_changed(move |s| {
                        let list = {
                            let mut list = selected.borrow_mut();
                            if let Some(entry) = list.get_mut(i) {
                                entry.1 = s.value() as i64;
                            }
                            list.clone()
                        };
                        on_change(list);
                    });
                    line.append(&spin);
                    row.set_child(Some(&line));
                }
                right.append(&row);
            }
        }
    });

    let move_rows = {
        let (selected, on_change, draw) = (selected.clone(), on_change.clone(), draw.clone());
        Rc::new(move |from: &gtk::ListBox, adding: bool| {
            let names: Vec<String> = from
                .selected_rows()
                .iter()
                .map(|r| r.widget_name().to_string())
                .collect();
            if names.is_empty() {
                return;
            }
            let list = {
                let mut list = selected.borrow_mut();
                if adding {
                    let start = number.map_or(0, |n| n.min.max(0).min(n.max));
                    list.extend(names.into_iter().map(|n| (n, start)));
                } else {
                    list.retain(|(n, _)| !names.contains(n));
                }
                list.clone()
            };
            on_change(list);
            draw();
        })
    };
    left.connect_row_activated({
        let move_rows = move_rows.clone();
        move |list, row| {
            list.select_row(Some(row));
            move_rows(list, true);
        }
    });
    right.connect_row_activated({
        let move_rows = move_rows.clone();
        move |list, row| {
            list.select_row(Some(row));
            move_rows(list, false);
        }
    });
    search.connect_search_changed({
        let draw = draw.clone();
        move |_| draw()
    });

    let add = gtk::Button::builder()
        .icon_name("go-next-symbolic")
        .tooltip_text(tr("Add the highlighted names"))
        .build();
    let remove = gtk::Button::builder()
        .icon_name("go-previous-symbolic")
        .tooltip_text(tr("Remove the highlighted names"))
        .build();
    add.connect_clicked({
        let (move_rows, left) = (move_rows.clone(), left.clone());
        move |_| move_rows(&left, true)
    });
    remove.connect_clicked({
        let (move_rows, right) = (move_rows.clone(), right.clone());
        move |_| move_rows(&right, false)
    });
    let arrows = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .valign(gtk::Align::Center)
        .build();
    arrows.append(&add);
    arrows.append(&remove);

    let root = gtk::Box::builder().spacing(6).build();
    root.append(&column(tr("Available"), Some(&search), &left));
    root.append(&arrows);
    root.append(&column(tr("Selected"), None, &right));
    TwoList {
        widget: root.upcast(),
        selected,
        redraw: draw,
    }
}

/// A list control: the field's two lists in a group-sized box, loaded
/// from the record.
pub fn names_control(
    available: Vec<String>,
    number: Option<Number>,
    value: impl Fn() -> Value + 'static,
    changed: Rc<dyn Fn(Value)>,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let pairs = number.is_some();
    let lists = two_list(
        Rc::new(available),
        number,
        Rc::new(move |list| {
            changed(if pairs {
                Value::Pairs(list)
            } else {
                Value::List(list.into_iter().map(|(n, _)| n).collect())
            })
        }),
    );
    let (selected, redraw) = (lists.selected.clone(), lists.redraw.clone());
    let load = move || {
        let now = value().pairs();
        if *selected.borrow() != now {
            *selected.borrow_mut() = now;
            redraw();
        }
    };
    (lists.widget, Box::new(load))
}

/// A check gating a list ([`FieldKind::Checked`]): the lists are greyed
/// while it is off, and turning it off empties them.
pub fn checked_control(
    label: &str,
    available: Vec<String>,
    value: impl Fn() -> Value + 'static,
    changed: Rc<dyn Fn(Value)>,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let switch = adw::SwitchRow::builder()
        .title(glib::markup_escape_text(label).as_str())
        .build();
    let header = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    header.append(&switch);
    let lists = two_list(
        Rc::new(available),
        None,
        Rc::new({
            let changed = changed.clone();
            move |list| changed(Value::List(list.into_iter().map(|(n, _)| n).collect()))
        }),
    );
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    root.append(&header);
    root.append(&lists.widget);
    // On while the user turned it on, even before a name is chosen.
    let wanted = Rc::new(Cell::new(false));
    let loading = Rc::new(Cell::new(false));
    switch.connect_active_notify({
        let (widget, wanted, loading) = (lists.widget.clone(), wanted.clone(), loading.clone());
        move |s| {
            widget.set_sensitive(s.is_active());
            if loading.get() {
                return;
            }
            wanted.set(s.is_active());
            if !s.is_active() {
                changed(Value::List(Vec::new()));
            }
        }
    });
    let load = {
        let (switch, selected, redraw, widget) = (
            switch.clone(),
            lists.selected.clone(),
            lists.redraw.clone(),
            lists.widget.clone(),
        );
        move || {
            let now = value().pairs();
            if *selected.borrow() != now {
                *selected.borrow_mut() = now;
                redraw();
            }
            loading.set(true);
            switch.set_active(!selected.borrow().is_empty() || wanted.get());
            widget.set_sensitive(switch.is_active());
            loading.set(false);
        }
    };
    (root.upcast(), Box::new(load))
}

/// A check with a number ([`FieldKind::OptInt`]).
pub fn opt_int_control(
    label: &str,
    (min, max, none, start, inverted): (i64, i64, i64, i64, bool),
    value: impl Fn() -> Value + 'static,
    changed: Rc<dyn Fn(Value)>,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let check = gtk::CheckButton::builder()
        .valign(gtk::Align::Center)
        .build();
    let spin = gtk::SpinButton::builder()
        .adjustment(&gtk::Adjustment::new(
            0.0, min as f64, max as f64, 1.0, 10.0, 0.0,
        ))
        .numeric(true)
        .valign(gtk::Align::Center)
        .width_chars(6)
        .build();
    let row = adw::ActionRow::builder()
        .title(glib::markup_escape_text(label).as_str())
        .activatable_widget(&check)
        .build();
    row.add_prefix(&check);
    row.add_suffix(&spin);
    let loading = Rc::new(Cell::new(false));
    check.connect_toggled({
        let (changed, loading, spin) = (changed.clone(), loading.clone(), spin.clone());
        move |c| {
            let numbered = c.is_active() != inverted;
            spin.set_sensitive(numbered);
            if loading.get() {
                return;
            }
            changed(Value::Int(if numbered { start } else { none }));
        }
    });
    spin.connect_value_changed({
        let (changed, loading) = (changed.clone(), loading.clone());
        move |s| {
            if !loading.get() {
                changed(Value::Int(s.value() as i64));
            }
        }
    });
    let load = move || {
        let v = value().int();
        // A "not limited" check is on for any negative number when the
        // limit-free value is negative (the Container page tests `< 0`).
        let numbered = v != none && !(inverted && none < 0 && v < 0);
        loading.set(true);
        check.set_active(numbered != inverted);
        spin.set_sensitive(numbered);
        if numbered && spin.value() as i64 != v {
            spin.set_value(v as f64);
        }
        loading.set(false);
    };
    (row.upcast(), Box::new(load))
}

/// A real number with an optional "not limited" check
/// ([`FieldKind::Real`]).
pub fn real_control(
    label: &str,
    (min, max): (f64, f64),
    none: Option<(f64, &'static str)>,
    value: impl Fn() -> Value + 'static,
    changed: Rc<dyn Fn(Value)>,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let spin = gtk::SpinButton::builder()
        .adjustment(&gtk::Adjustment::new(0.0, min, max, 1.0, 10.0, 0.0))
        .digits(2)
        .numeric(true)
        .valign(gtk::Align::Center)
        .width_chars(8)
        .build();
    let row = adw::ActionRow::builder()
        .title(glib::markup_escape_text(label).as_str())
        .build();
    let loading = Rc::new(Cell::new(false));
    let check = none.map(|(none, check_label)| {
        let check = gtk::CheckButton::builder()
            .label(tr(check_label))
            .valign(gtk::Align::Center)
            .build();
        let (changed, loading, spin) = (changed.clone(), loading.clone(), spin.clone());
        check.connect_toggled(move |c| {
            spin.set_sensitive(!c.is_active());
            if !loading.get() {
                changed(Value::Real(if c.is_active() { none } else { 0.0 }));
            }
        });
        row.add_suffix(&check);
        check
    });
    row.add_suffix(&spin);
    spin.connect_value_changed({
        let (changed, loading) = (changed.clone(), loading.clone());
        move |s| {
            if !loading.get() {
                changed(Value::Real(s.value()));
            }
        }
    });
    let load = move || {
        let v = value().real();
        loading.set(true);
        let limited = match (&check, none) {
            (Some(check), Some((none, _))) => {
                let unlimited = if none < 0.0 { v < 0.0 } else { v == none };
                check.set_active(unlimited);
                !unlimited
            }
            _ => true,
        };
        spin.set_sensitive(limited);
        if limited && spin.value() != v {
            spin.set_value(v);
        }
        loading.set(false);
    };
    (row.upcast(), Box::new(load))
}

/// Choose-N-of groups ([`FieldKind::Groups`]): one row per group, with
/// its count, an edit button that opens the two lists, and remove.
pub fn groups_control(
    label: &'static str,
    available: Vec<String>,
    value: impl Fn() -> Value + 'static,
    changed: Rc<dyn Fn(Value)>,
    parent: impl Fn() -> gtk::Widget + 'static,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    let groups: Groups = Rc::default();
    let available = Rc::new(available);
    let parent: Rc<dyn Fn() -> gtk::Widget> = Rc::new(parent);
    let draw_slot: Redraw = Rc::default();
    let edit: Rc<dyn Fn(Option<usize>)> = Rc::new({
        let (groups, available, changed, parent, draw_slot) = (
            groups.clone(),
            available.clone(),
            changed.clone(),
            parent.clone(),
            draw_slot.clone(),
        );
        move |at: Option<usize>| {
            let (count, names) = at
                .and_then(|i| groups.borrow().get(i).cloned())
                .unwrap_or((1, Vec::new()));
            let picked = Rc::new(RefCell::new(names.clone()));
            let lists = two_list(
                available.clone(),
                None,
                Rc::new({
                    let picked = picked.clone();
                    move |l| *picked.borrow_mut() = l.into_iter().map(|(n, _)| n).collect()
                }),
            );
            *lists.selected.borrow_mut() = names.into_iter().map(|n| (n, 0)).collect();
            (lists.redraw)();
            let spin = adw::SpinRow::builder()
                .title(tr("Number to choose"))
                .adjustment(&gtk::Adjustment::new(
                    count as f64,
                    1.0,
                    100.0,
                    1.0,
                    5.0,
                    0.0,
                ))
                .numeric(true)
                .build();
            let top = gtk::ListBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .css_classes(["boxed-list"])
                .build();
            top.append(&spin);
            let content = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(12)
                .margin_top(12)
                .margin_bottom(12)
                .margin_start(12)
                .margin_end(12)
                .build();
            content.append(&top);
            content.append(&lists.widget);
            let cancel = gtk::Button::builder()
                .label(tr("_Cancel"))
                .use_underline(true)
                .build();
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
            view.set_content(Some(
                &gtk::ScrolledWindow::builder()
                    .hscrollbar_policy(gtk::PolicyType::Never)
                    .child(&content)
                    .build(),
            ));
            let dialog = adw::Dialog::builder()
                .title(label)
                .content_width(560)
                .content_height(480)
                .child(&view)
                .build();
            cancel.connect_clicked({
                let dialog = dialog.clone();
                move |_| {
                    dialog.close();
                }
            });
            ok.connect_clicked({
                let (dialog, groups, changed, draw_slot) = (
                    dialog.clone(),
                    groups.clone(),
                    changed.clone(),
                    draw_slot.clone(),
                );
                move |_| {
                    let group = (spin.value() as i64, picked.borrow().clone());
                    {
                        let mut g = groups.borrow_mut();
                        match at {
                            Some(i) if i < g.len() => g[i] = group,
                            _ => g.push(group),
                        }
                    }
                    changed(Value::Groups(groups.borrow().clone()));
                    if let Some(draw) = draw_slot.borrow().clone() {
                        draw();
                    }
                    dialog.close();
                }
            });
            dialog.present(Some(&parent()));
        }
    });
    let draw: Rc<dyn Fn()> = Rc::new({
        let (list, groups, changed, edit, draw_slot) = (
            list.clone(),
            groups.clone(),
            changed.clone(),
            edit.clone(),
            draw_slot.clone(),
        );
        move || {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            for (i, (count, names)) in groups.borrow().iter().enumerate() {
                let row = adw::ActionRow::builder()
                    .title(tr!("Choose {count} of", count))
                    .subtitle(glib::markup_escape_text(&names.join(", ")).as_str())
                    .build();
                let edit_button = gtk::Button::builder()
                    .icon_name("document-edit-symbolic")
                    .tooltip_text(tr("Edit"))
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                let edit = edit.clone();
                edit_button.connect_clicked(move |_| edit(Some(i)));
                let remove = gtk::Button::builder()
                    .icon_name("list-remove-symbolic")
                    .tooltip_text(tr("Remove"))
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                let (groups, changed, draw_slot) =
                    (groups.clone(), changed.clone(), draw_slot.clone());
                remove.connect_clicked(move |_| {
                    groups.borrow_mut().remove(i);
                    changed(Value::Groups(groups.borrow().clone()));
                    if let Some(draw) = draw_slot.borrow().clone() {
                        draw();
                    }
                });
                row.add_suffix(&edit_button);
                row.add_suffix(&remove);
                list.append(&row);
            }
            let add = adw::ActionRow::builder()
                .title(tr("Add a Group…"))
                .activatable(true)
                .build();
            add.add_prefix(&gtk::Image::from_icon_name("list-add-symbolic"));
            let edit = edit.clone();
            add.connect_activated(move |_| edit(None));
            list.append(&add);
        }
    });
    *draw_slot.borrow_mut() = Some(draw.clone());
    let load = move || {
        let now = value().groups();
        if *groups.borrow() != now {
            *groups.borrow_mut() = now;
        }
        draw();
    };
    (list.upcast(), Box::new(load))
}

/// A table of numbers, one row per level ([`FieldKind::Grid`]).
pub fn grid_control(
    columns: &'static [&'static str],
    first: i64,
    (min, max): (i64, i64),
    value: impl Fn() -> Value + 'static,
    changed: Rc<dyn Fn(Value)>,
) -> (gtk::Widget, Box<dyn Fn()>) {
    let grid = gtk::Grid::builder()
        .row_spacing(2)
        .column_spacing(4)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();
    grid.attach(
        &gtk::Label::builder()
            .label(tr("Level"))
            .css_classes(["heading"])
            .build(),
        0,
        0,
        1,
        1,
    );
    for (c, title) in columns.iter().enumerate() {
        grid.attach(
            &gtk::Label::builder()
                .label(tr(title))
                .css_classes(["heading"])
                .wrap(true)
                .max_width_chars(8)
                .justify(gtk::Justification::Center)
                .build(),
            c as i32 + 1,
            0,
            1,
            1,
        );
    }
    let cells: Rc<RefCell<Vec<Vec<gtk::Entry>>>> = Rc::default();
    let rows: Rc<RefCell<Vec<Vec<i64>>>> = Rc::default();
    let loading = Rc::new(Cell::new(false));
    let load = {
        let (grid, cells, rows) = (grid.clone(), cells.clone(), rows.clone());
        move || {
            let now = value().grid();
            if *rows.borrow() == now && !cells.borrow().is_empty() {
                return;
            }
            loading.set(true);
            // Build the cells once, for as many rows as the record has.
            if cells.borrow().len() != now.len() {
                for row in cells.borrow_mut().drain(..) {
                    for cell in row {
                        grid.remove(&cell);
                    }
                }
                for (r, values) in now.iter().enumerate() {
                    grid.attach(
                        &gtk::Label::new(Some(&(first + r as i64).to_string())),
                        0,
                        r as i32 + 1,
                        1,
                        1,
                    );
                    let mut line = Vec::new();
                    for c in 0..values.len() {
                        let entry = gtk::Entry::builder()
                            .width_chars(5)
                            .max_width_chars(6)
                            .input_purpose(gtk::InputPurpose::Number)
                            .build();
                        let (rows, changed, loading) =
                            (rows.clone(), changed.clone(), loading.clone());
                        entry.connect_changed(move |e| {
                            if loading.get() {
                                return;
                            }
                            let Ok(v) = e.text().trim().parse::<i64>() else {
                                return;
                            };
                            let v = v.clamp(min, max);
                            let grid = {
                                let mut rows = rows.borrow_mut();
                                if let Some(cell) = rows.get_mut(r).and_then(|l| l.get_mut(c)) {
                                    *cell = v;
                                }
                                rows.clone()
                            };
                            changed(Value::Grid(grid));
                        });
                        grid.attach(&entry, c as i32 + 1, r as i32 + 1, 1, 1);
                        line.push(entry);
                    }
                    cells.borrow_mut().push(line);
                }
            }
            for (line, values) in cells.borrow().iter().zip(&now) {
                for (cell, v) in line.iter().zip(values) {
                    let text = v.to_string();
                    if cell.text() != text {
                        cell.set_text(&text);
                    }
                }
            }
            *rows.borrow_mut() = now;
            loading.set(false);
        }
    };
    let scroll = gtk::ScrolledWindow::builder()
        .min_content_height(320)
        .child(&grid)
        .css_classes(["card"])
        .build();
    (scroll.upcast(), Box::new(load))
}
