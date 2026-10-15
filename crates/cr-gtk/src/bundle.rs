//! Editing a treasure bundle as the original's Treasure Bundle Editor
//! does (IDD 290): adding parts, removing some or all of an item, moving
//! items into others, copying and removing scroll spells and Edit
//! Scrolls. The roller and the Encounter dialog's Items use it.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_store::treasure::{
    ADD_CATEGORIES, Bundle, Item, MoveError, SCROLL_SPELL_LIMIT, SpellCopy, TreasureRules, list_cmp,
};
use gtk::{gio, glib};

/// A bundle being edited: how to read it and how to store a changed copy
/// (which redraws).
pub struct Editing {
    pub rules: Rc<TreasureRules>,
    pub parent: gtk::Widget,
    pub get: Box<dyn Fn() -> Option<Bundle>>,
    pub set: Box<dyn Fn(Bundle)>,
}

pub type Edit = Rc<Editing>;

/// A menu entry: its label and what it does.
type Entry<'a> = (&'a str, Box<dyn Fn()>);
/// A redraw callback set once its widgets exist.
type Redraw = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

impl Editing {
    fn change(&self, f: impl FnOnce(&mut Bundle)) {
        if let Some(mut bundle) = (self.get)() {
            f(&mut bundle);
            (self.set)(bundle);
        }
    }

    fn bundle(&self) -> Bundle {
        (self.get)().unwrap_or_default()
    }
}

/// The "Add Item…" button of a group header.
pub fn add_button(edit: &Edit) -> gtk::Button {
    let button = gtk::Button::builder()
        .label("_Add Item…")
        .use_underline(true)
        .css_classes(["flat"])
        .build();
    button.connect_clicked({
        let edit = edit.clone();
        move |_| add_dialog(&edit)
    });
    button
}

/// The bundle's items as rows with their edits; an item holding others
/// or spells expands.
pub fn rows(edit: &Edit, bundle: &Bundle) -> Vec<gtk::Widget> {
    bundle
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| item_row(edit, item, &[i]))
        .collect()
}

fn item_row(edit: &Edit, item: &Item, path: &[usize]) -> gtk::Widget {
    let title = glib::markup_escape_text(&item.label()).to_string();
    let mut entries: Vec<Entry> = Vec::new();
    entries.push(("_Remove…", {
        let (edit, path) = (edit.clone(), path.to_vec());
        Box::new(move || remove(&edit, &path))
    }));
    entries.push(("_Move Into…", {
        let (edit, path) = (edit.clone(), path.to_vec());
        Box::new(move || move_dialog(&edit, &path))
    }));
    if item.extra.is_some() || item.name.contains("Scroll") {
        entries.push(("Edit _Spells…", {
            let (edit, path) = (edit.clone(), path.to_vec());
            Box::new(move || edit_spells(&edit, &path))
        }));
    }
    let menu = row_menu(entries);
    let spells = item.spells();
    if item.contents.is_none() && spells.is_empty() {
        let row = adw::ActionRow::builder().title(title).build();
        if item.flag {
            row.add_css_class("dim-label");
        }
        row.add_suffix(&menu);
        return row.upcast();
    }
    let pages = item.extra.as_ref().map(|e| e.unknown_0x4);
    let subtitle = match pages {
        Some([capacity, used, _]) if !spells.is_empty() || capacity > 0 => {
            format!("{used} of {capacity} pages used")
        }
        _ => String::new(),
    };
    let row = adw::ExpanderRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build();
    row.add_suffix(&menu);
    for (i, spell) in spells.iter().enumerate() {
        let spell_row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&spell.name).as_str())
            .subtitle(format!(
                "{} page{}",
                spell.pages,
                if spell.pages == 1 { "" } else { "s" }
            ))
            .build();
        spell_row.add_prefix(&gtk::Image::from_icon_name(
            "accessories-text-editor-symbolic",
        ));
        let entries: Vec<Entry> = vec![
            ("_Remove", {
                let (edit, path) = (edit.clone(), path.to_vec());
                Box::new(move || edit.change(|b| _ = b.remove_spell(&path, i)))
            }),
            ("_Copy To…", {
                let (edit, path) = (edit.clone(), path.to_vec());
                Box::new(move || copy_spell_dialog(&edit, &path, i))
            }),
        ];
        spell_row.add_suffix(&row_menu(entries));
        row.add_row(&spell_row);
    }
    if let Some(contents) = &item.contents {
        for (i, inner) in contents.items.iter().enumerate() {
            let mut inner_path = path.to_vec();
            inner_path.push(i);
            row.add_row(&item_row(edit, inner, &inner_path));
        }
    }
    row.upcast()
}

/// A menu button with `entries`.
fn row_menu(entries: Vec<Entry>) -> gtk::MenuButton {
    let menu = gio::Menu::new();
    let actions = gio::SimpleActionGroup::new();
    for (i, (label, run)) in entries.into_iter().enumerate() {
        let name = format!("a{i}");
        let action = gio::SimpleAction::new(&name, None);
        action.connect_activate(move |_, _| run());
        actions.add_action(&action);
        menu.append(Some(label), Some(&format!("row.{name}")));
    }
    let button = gtk::MenuButton::builder()
        .icon_name("view-more-symbolic")
        .tooltip_text("Edit")
        .valign(gtk::Align::Center)
        .menu_model(&menu)
        .css_classes(["flat"])
        .build();
    button.insert_action_group("row", Some(&actions));
    button
}

fn message(parent: &impl IsA<gtk::Widget>, heading: &str, body: &str) {
    let alert = adw::AlertDialog::builder()
        .heading(heading)
        .body(body)
        .default_response("ok")
        .close_response("ok")
        .build();
    alert.add_response("ok", "_OK");
    alert.present(Some(parent));
}

/// A Yes/No question, No the default as in the original.
fn ask(parent: &impl IsA<gtk::Widget>, heading: &str, body: &str, yes: impl Fn() + 'static) {
    let alert = adw::AlertDialog::builder()
        .heading(heading)
        .body(body)
        .default_response("no")
        .close_response("no")
        .build();
    alert.add_responses(&[("no", "_No"), ("yes", "_Yes")]);
    alert.connect_response(Some("yes"), move |_, _| yes());
    alert.present(Some(parent));
}

/// The "How many?" dialog (IDD 291): a number from `min` to `max`.
fn how_many(
    parent: &impl IsA<gtk::Widget>,
    heading: &str,
    action: &str,
    (min, max, default): (u32, u32, u32),
    then: impl Fn(u32) + 'static,
) {
    let spin = adw::SpinRow::builder()
        .title("How many?")
        .adjustment(&gtk::Adjustment::new(
            default as f64,
            min as f64,
            max as f64,
            1.0,
            10.0,
            0.0,
        ))
        .numeric(true)
        .build();
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    list.append(&spin);
    let alert = adw::AlertDialog::builder()
        .heading(heading)
        .extra_child(&list)
        .default_response("ok")
        .close_response("cancel")
        .build();
    alert.add_responses(&[("cancel", "_Cancel"), ("ok", action)]);
    alert.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
    alert.connect_response(Some("ok"), move |_, _| {
        then(spin.value().round().max(0.0) as u32)
    });
    alert.present(Some(parent));
}

/// Add (0x543a6b): a part of a category, at the top of the bundle; Money
/// asks how many.
fn add_dialog(edit: &Edit) {
    let mut categories: Vec<&str> = ADD_CATEGORIES.iter().map(|c| c.0).collect();
    categories.sort_by(|a, b| list_cmp(a, b));
    let category = gtk::DropDown::builder()
        .model(&gtk::StringList::new(&categories))
        .tooltip_text("Category")
        .build();
    let model = gtk::StringList::new(&[]);
    let fill = {
        let (model, edit, categories) = (model.clone(), edit.clone(), categories.clone());
        move |i: u32| {
            let mut names = edit.rules.category_parts(categories[i as usize]);
            names.sort_by(|a, b| list_cmp(a, b));
            let n = model.n_items();
            model.splice(0, n, &names.iter().map(String::as_str).collect::<Vec<_>>());
        }
    };
    fill(0);
    category.connect_selected_notify(move |d| fill(d.selected()));

    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search")
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
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let label = gtk::Label::builder()
            .xalign(0.0)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(6)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        item.downcast_ref::<gtk::ListItem>()
            .unwrap()
            .set_child(Some(&label));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let text = item.item().and_downcast::<gtk::StringObject>().unwrap();
        let label = item.child().and_downcast::<gtk::Label>().unwrap();
        label.set_label(&text.string());
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
    let header = adw::HeaderBar::new();
    header.pack_start(&category);
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.add_top_bar(
        &gtk::SearchBar::builder()
            .child(&search)
            .search_mode_enabled(true)
            .build(),
    );
    view.set_content(Some(&scroll));
    let dialog = adw::Dialog::builder()
        .title("Add Item")
        .content_width(420)
        .content_height(560)
        .child(&view)
        .build();
    list.connect_activate({
        let (dialog, edit, category) = (dialog.clone(), edit.clone(), category.clone());
        move |list, position| {
            let Some(text) = list
                .model()
                .and_then(|m| m.item(position))
                .and_downcast::<gtk::StringObject>()
            else {
                return;
            };
            let name = text.string().to_string();
            let money = category
                .selected_item()
                .and_downcast::<gtk::StringObject>()
                .is_some_and(|s| s.string() == "Money");
            dialog.close();
            if money {
                let edit2 = edit.clone();
                how_many(
                    &edit.parent,
                    "How many to add?",
                    "_Add",
                    (0, 10000, 1),
                    move |n| edit2.change(|b| b.add_part(&name, n)),
                );
            } else {
                edit.change(|b| b.add_part(&name, 1));
            }
        }
    });
    search.connect_activate({
        let list = list.clone();
        move |_| {
            if list.model().is_some_and(|m| m.n_items() > 0) {
                list.emit_by_name::<()>("activate", &[&0u32]);
            }
        }
    });
    dialog.present(Some(&edit.parent));
    search.grab_focus();
}

/// Remove (0x543c0f): asks how many when there is more than one.
fn remove(edit: &Edit, path: &[usize]) {
    let bundle = edit.bundle();
    let Some(item) = bundle.item(path) else {
        return;
    };
    let path = path.to_vec();
    if item.quantity > 1 {
        let edit2 = edit.clone();
        how_many(
            &edit.parent,
            "How many to delete?",
            "_Remove",
            (0, item.quantity, item.quantity),
            move |n| edit2.change(|b| b.remove_count(&path, n)),
        );
    } else {
        edit.change(|b| b.remove_count(&path, 1));
    }
}

/// Every node a drop could go to: the top, then each item, indented.
fn destinations(bundle: &Bundle) -> Vec<(Vec<usize>, String)> {
    fn walk(
        bundle: &Bundle,
        path: &mut Vec<usize>,
        depth: usize,
        out: &mut Vec<(Vec<usize>, String)>,
    ) {
        for (i, item) in bundle.items.iter().enumerate() {
            path.push(i);
            out.push((
                path.clone(),
                format!("{}{}", "    ".repeat(depth), item.label()),
            ));
            if let Some(contents) = &item.contents {
                walk(contents, path, depth + 1, out);
            }
            path.pop();
        }
    }
    let mut out = vec![(Vec::new(), "Treasure".to_string())];
    walk(bundle, &mut Vec::new(), 1, &mut out);
    out
}

/// A list of nodes to choose one from.
fn choose_node(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    nodes: Vec<(Vec<usize>, String)>,
    chosen: impl Fn(Vec<usize>) + 'static,
) {
    let list = gtk::ListBox::builder()
        .css_classes(["boxed-list"])
        .selection_mode(gtk::SelectionMode::None)
        .build();
    for (_, label) in &nodes {
        list.append(
            &adw::ActionRow::builder()
                .title(glib::markup_escape_text(label).as_str())
                .activatable(true)
                .build(),
        );
    }
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&list);
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
        .title(title)
        .content_width(420)
        .content_height(560)
        .width_request(360)
        .height_request(294)
        .child(&view)
        .build();
    list.connect_row_activated({
        let dialog = dialog.clone();
        move |_, row| {
            let path = nodes[row.index() as usize].0.clone();
            dialog.close();
            chosen(path);
        }
    });
    dialog.present(Some(parent));
}

/// Moving an item into another, the original's drag and drop (0x4ab3c2).
fn move_dialog(edit: &Edit, from: &[usize]) {
    let bundle = edit.bundle();
    let from = from.to_vec();
    let nodes: Vec<_> = destinations(&bundle)
        .into_iter()
        .filter(|(p, _)| *p != from && *p != from[..from.len() - 1])
        .collect();
    let edit2 = edit.clone();
    choose_node(&edit.parent, "Move Into", nodes, move |to| {
        let mut bundle = edit2.bundle();
        match bundle.begin_move(&from, &to) {
            Err(e @ MoveError::IntoItself) | Err(e @ MoveError::IntoSpell) => {
                message(&edit2.parent, "Cannot Move", &e.to_string())
            }
            Ok(None) => {}
            Ok(Some(pending)) => {
                (edit2.set)(bundle);
                if pending.quantity > 1 {
                    let edit3 = edit2.clone();
                    how_many(
                        &edit2.parent,
                        "How many to move?",
                        "_Move",
                        (0, pending.quantity, pending.quantity),
                        move |n| edit3.change(|b| b.finish_move(&pending, n)),
                    );
                } else {
                    edit2.change(|b| b.finish_move(&pending, 1));
                }
            }
        }
    });
}

/// Copying a spell onto an item, the original's drag and drop of a spell.
fn copy_spell_dialog(edit: &Edit, from: &[usize], index: usize) {
    let bundle = edit.bundle();
    let from = from.to_vec();
    let nodes: Vec<_> = destinations(&bundle)
        .into_iter()
        .filter(|(p, _)| !p.is_empty() && *p != from)
        .collect();
    let edit2 = edit.clone();
    choose_node(&edit.parent, "Copy Spell To", nodes, move |to| {
        copy_spell(&edit2, from.clone(), index, to, false, false)
    });
}

fn copy_spell(
    edit: &Edit,
    from: Vec<usize>,
    index: usize,
    to: Vec<usize>,
    make_holding: bool,
    add_pages: bool,
) {
    let mut bundle = edit.bundle();
    let Ok(outcome) = bundle.copy_spell(&from, index, &to, make_holding, add_pages) else {
        return;
    };
    (edit.set)(bundle);
    let again = {
        let edit = edit.clone();
        move |holding: bool, pages: bool| {
            let (edit, from, to) = (edit.clone(), from.clone(), to.clone());
            move || copy_spell(&edit, from.clone(), index, to.clone(), holding, pages)
        }
    };
    match outcome {
        SpellCopy::NotHolding => ask(
            &edit.parent,
            "Cannot Hold Spells",
            "The destination is not marked as being able to hold spells. Override?",
            again(true, false),
        ),
        SpellCopy::NoPages => ask(
            &edit.parent,
            "No Pages Left",
            "The target doesn't have pages left to copy the spell to.  Add needed pages?",
            again(true, true),
        ),
        SpellCopy::Nothing | SpellCopy::Copied => {}
    }
}

/// Edit Scrolls (0x5452aa, AddRemoveDlg IDD 408): the spells on a priest
/// or wizard scroll, chosen from every spell of its kind.
fn edit_spells(edit: &Edit, path: &[usize]) {
    let mut bundle = edit.bundle();
    let Some(item) = bundle.item_mut(path) else {
        return;
    };
    let prepared = item.prepare_scroll_edit();
    let current = item.spell_names();
    let title = item.name.clone();
    // The check may have made the scroll able to hold spells.
    (edit.set)(bundle);
    let wizard = match prepared {
        Ok(wizard) => wizard,
        Err(e) => return message(&edit.parent, e.title(), e.message()),
    };
    let mut choices = edit.rules.scroll_spell_choices(wizard);
    choices.sort_by(|a, b| list_cmp(a, b));
    let selected: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(current));
    selected.borrow_mut().sort_by(|a, b| list_cmp(a, b));

    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    let group = adw::PreferencesGroup::builder()
        .title("Selected Spells")
        .description("Select Spells that will be in the scroll.")
        .build();
    group.add(&list);
    let page = adw::PreferencesPage::new();
    page.add(&group);
    let draw: Redraw = Rc::default();
    let render: Rc<dyn Fn()> =
        Rc::new({
            let (list, selected, group, draw, choices) = (
                list.clone(),
                selected.clone(),
                group.clone(),
                draw.clone(),
                choices.clone(),
            );
            move || {
                while let Some(child) = list.first_child() {
                    list.remove(&child);
                }
                let left = SCROLL_SPELL_LIMIT as i64 - selected.borrow().len() as i64;
                group.set_title(&format!("Selected Spells (Spells Left: {left})"));
                for (i, name) in selected.borrow().iter().enumerate() {
                    let row = adw::ActionRow::builder()
                        .title(glib::markup_escape_text(name).as_str())
                        .build();
                    let remove = gtk::Button::builder()
                        .icon_name("list-remove-symbolic")
                        .tooltip_text("Remove")
                        .valign(gtk::Align::Center)
                        .css_classes(["flat"])
                        .build();
                    let (selected, draw) = (selected.clone(), draw.clone());
                    remove.connect_clicked(move |_| {
                        selected.borrow_mut().remove(i);
                        if let Some(draw) = draw.borrow().clone() {
                            draw();
                        }
                    });
                    row.add_suffix(&remove);
                    list.append(&row);
                }
                let add = adw::ActionRow::builder()
                    .title("Add Spell…")
                    .activatable(true)
                    .build();
                add.add_prefix(&gtk::Image::from_icon_name("list-add-symbolic"));
                let (selected, draw, choices, list2) = (
                    selected.clone(),
                    draw.clone(),
                    choices.clone(),
                    list.clone(),
                );
                add.connect_activated(move |_| {
                let pick = {
                    let (selected, draw, list2, choices) =
                        (selected.clone(), draw.clone(), list2.clone(), choices.clone());
                    move || {
                        let (selected, draw) = (selected.clone(), draw.clone());
                        crate::lists::pick(&list2, "Scroll", choices.clone(), move |name| {
                            let mut s = selected.borrow_mut();
                            s.push(name);
                            s.sort_by(|a, b| list_cmp(a, b));
                            drop(s);
                            if let Some(draw) = draw.borrow().clone() {
                                draw();
                            }
                        })
                    }
                };
                if selected.borrow().len() >= SCROLL_SPELL_LIMIT {
                    ask(
                        &list2,
                        "Warning",
                        "Scrolls should contain a maximum of 6 spells.  Do you want to ignore this \
                         limit?",
                        pick,
                    );
                } else {
                    pick();
                }
            });
                list.append(&add);
            }
        });
    *draw.borrow_mut() = Some(render.clone());
    render();

    let cancel = gtk::Button::builder()
        .label("_Cancel")
        .use_underline(true)
        .build();
    let ok = gtk::Button::builder()
        .label("_Done")
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
    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(440)
        .content_height(560)
        .width_request(360)
        .height_request(294)
        .child(&view)
        .build();
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| _ = dialog.close()
    });
    // Break the redraw cycle.
    dialog.connect_closed(move |_| _ = draw.borrow_mut().take());
    ok.connect_clicked({
        let (dialog, edit, path) = (dialog.clone(), edit.clone(), path.to_vec());
        move |_| {
            let names = selected.borrow().clone();
            edit.change(|b| {
                if let Some(item) = b.item_mut(&path) {
                    item.set_scroll_spells(&names, wizard);
                }
            });
            dialog.close();
        }
    });
    dialog.present(Some(&edit.parent));
}
