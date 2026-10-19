//! The treasure roller (the DM Toolkit's Roll Treasure): how many of each
//! treasure type to roll, the bundle the rolls make with its Calculate
//! summary, and saving, printing and deleting bundles.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::tr;
use cr_store::help::topic;
use cr_store::sheet::Sheet;
use cr_store::treasure::{Bundle, Coin, Counts, LETTERS, Rand, TreasureRules};
use gtk::{gio, glib};

/// Where the roller was opened from.
pub enum Mode {
    /// Roll Treasure, or a saved bundle (by name) to change.
    Free { saved: Option<String> },
    /// An encounter's treasure: the "Use Treasure" button hands the bundle
    /// back, as the original's Ok does in that mode.
    Pick { use_bundle: Box<dyn Fn(Bundle)> },
}

/// Saves a bundle under a name, in place of the saved bundle being
/// changed, if any.
pub type SaveFn = Box<dyn Fn(Option<&str>, &str, &Bundle) -> Result<(), String>>;
pub type DeleteFn = Box<dyn Fn(&str) -> Result<(), String>>;

/// What the roller writes to the installation.
pub struct Store {
    pub save: SaveFn,
    pub delete: DeleteFn,
}

/// What the roller starts with: the treasure type counts and a bundle.
#[derive(Default)]
pub struct Start {
    pub counts: Counts,
    pub bundle: Bundle,
}

struct State {
    rules: Rc<TreasureRules>,
    rand: Rc<RefCell<Rand>>,
    bundle: RefCell<Bundle>,
    /// The saved bundle the roller is changing.
    saved: RefCell<Option<String>>,
    spins: Vec<gtk::SpinButton>,
    accumulate: adw::SwitchRow,
    coin: adw::ComboRow,
    items: adw::PreferencesGroup,
    summary: adw::PreferencesGroup,
    rows: RefCell<Vec<gtk::Widget>>,
    summary_rows: RefCell<Vec<adw::ActionRow>>,
    delete: std::cell::OnceCell<gio::SimpleAction>,
    empty: adw::StatusPage,
    results: gtk::Box,
    edit: std::cell::OnceCell<crate::bundle::Edit>,
    scroll: gtk::ScrolledWindow,
}

/// Opens the roller over `parent` with `bundle` and the treasure type
/// counts `counts` already filled in.
pub fn roller(
    parent: &impl IsA<gtk::Widget>,
    rules: Rc<TreasureRules>,
    rand: Rc<RefCell<Rand>>,
    start: Start,
    mode: Mode,
    store: Store,
    toast: impl Fn(&str) + 'static,
) {
    let toast: Rc<dyn Fn(&str)> = Rc::new(toast);
    let store = Rc::new(store);

    // Treasure types.
    let spins: Vec<gtk::SpinButton> = LETTERS
        .iter()
        .zip(start.counts)
        .map(|(_, n)| {
            gtk::SpinButton::builder()
                .adjustment(&gtk::Adjustment::new(n as f64, 0.0, 999.0, 1.0, 10.0, 0.0))
                .numeric(true)
                .width_chars(3)
                .valign(gtk::Align::Center)
                .build()
        })
        .collect();
    let letters_group = |title: &str, description: &str, range: std::ops::Range<usize>| {
        let flow = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .min_children_per_line(2)
            .max_children_per_line(3)
            .column_spacing(12)
            .row_spacing(6)
            .homogeneous(true)
            .build();
        for i in range {
            let label = gtk::Label::builder()
                .label(LETTERS[i].to_string())
                .width_chars(2)
                .css_classes(["heading"])
                .build();
            let cell = gtk::Box::builder().spacing(6).build();
            cell.append(&label);
            cell.append(&spins[i]);
            spins[i].update_property(&[gtk::accessible::Property::Label(&tr!(
                "Treasure type {letter}",
                letter = LETTERS[i]
            ))]);
            flow.append(&cell);
        }
        let group = adw::PreferencesGroup::builder()
            .title(title)
            .description(description)
            .build();
        group.add(&flow);
        group
    };
    let lair = letters_group(tr("Lair"), tr("Treasure types A to I"), 0..9);
    let individual = letters_group(tr("Individual"), tr("Treasure types J to Z"), 9..26);

    let accumulate = adw::SwitchRow::builder()
        .title(tr("Accumulate Rolls"))
        .subtitle(tr("Add new rolls to the treasure instead of replacing it"))
        .build();
    let coin = adw::ComboRow::builder()
        .title(tr("Coin"))
        .subtitle(tr("The coin of the treasure types' platinum line"))
        .model(&gtk::StringList::new(&[tr("Platinum"), tr("Electrum")]))
        .build();
    let options = adw::PreferencesGroup::new();
    options.add(&accumulate);
    options.add(&coin);

    let left = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .hexpand(false)
        .build();
    left.append(&lair);
    left.append(&individual);
    left.append(&options);

    // Results.
    let items = adw::PreferencesGroup::builder()
        .title(tr("Treasure"))
        .build();
    let summary = adw::PreferencesGroup::builder()
        .title(tr("Summary"))
        .build();
    let empty = adw::StatusPage::builder()
        .icon_name("media-playlist-shuffle-symbolic")
        .title(tr("No Treasure"))
        .description(tr(
            "Choose how many of each treasure type to roll, then press Roll, or add items yourself",
        ))
        .css_classes(["compact"])
        .vexpand(true)
        .build();
    let results = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .build();
    results.append(&items);
    results.append(&summary);
    let right = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .hexpand(true)
        .build();
    right.append(&empty);
    right.append(&results);

    let columns = gtk::Box::builder()
        .spacing(24)
        .margin_top(18)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();
    columns.append(&left);
    columns.append(&right);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(
            &adw::Clamp::builder()
                .maximum_size(1000)
                .child(&columns)
                .build(),
        )
        .build();

    // Header.
    let roll = gtk::Button::builder()
        .label(tr("_Roll"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let reset = gtk::Button::builder()
        .label(tr("Re_set"))
        .use_underline(true)
        .tooltip_text(tr("Clear the treasure types and the treasure"))
        .build();
    let menu = gio::Menu::new();
    let save_section = gio::Menu::new();
    save_section.append(Some(tr("_Save…")), Some("treasure.save"));
    menu.append_section(None, &save_section);
    let print_section = gio::Menu::new();
    print_section.append(Some(tr("_Print…")), Some("treasure.print"));
    print_section.append(Some(tr("Save as P_DF…")), Some("treasure.pdf"));
    menu.append_section(None, &print_section);
    let delete_section = gio::Menu::new();
    delete_section.append(Some(tr("_Delete Saved Treasure")), Some("treasure.delete"));
    menu.append_section(None, &delete_section);
    let menu_button = gtk::MenuButton::builder()
        .icon_name("view-more-symbolic")
        .tooltip_text(tr("Treasure Menu"))
        .menu_model(&menu)
        .build();
    let header = adw::HeaderBar::new();
    header.pack_start(&reset);
    header.pack_end(&menu_button);
    header.pack_end(&roll);
    let use_button = gtk::Button::builder()
        .label(tr("_Use Treasure"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    if matches!(mode, Mode::Pick { .. }) {
        roll.remove_css_class("suggested-action");
        header.pack_end(&use_button);
    }

    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, || topic::ROLL_TREASURE);
    view.add_top_bar(&header);
    view.set_content(Some(&scroll));
    let saved_name = match &mode {
        Mode::Free { saved } => saved.clone(),
        Mode::Pick { .. } => None,
    };
    let dialog = adw::Dialog::builder()
        .title(saved_name.as_deref().unwrap_or(tr("Roll Treasure")))
        .content_width(960)
        .content_height(720)
        .width_request(360)
        .height_request(294)
        .child(&view)
        .build();
    let narrow = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        640.0,
        adw::LengthUnit::Sp,
    ));
    narrow.add_setter(
        &columns,
        "orientation",
        Some(&gtk::Orientation::Vertical.to_value()),
    );
    dialog.add_breakpoint(narrow);

    let state = Rc::new(State {
        rules,
        rand,
        bundle: RefCell::new(start.bundle),
        saved: RefCell::new(saved_name),
        spins,
        accumulate,
        coin,
        items,
        summary,
        rows: RefCell::default(),
        summary_rows: RefCell::default(),
        delete: std::cell::OnceCell::new(),
        empty,
        results,
        edit: std::cell::OnceCell::new(),
        scroll: scroll.clone(),
    });
    let edit = Rc::new(crate::bundle::Editing {
        rules: state.rules.clone(),
        parent: dialog.clone().upcast(),
        get: Box::new({
            let state = Rc::downgrade(&state);
            move || Some(state.upgrade()?.bundle.borrow().clone())
        }),
        set: Box::new({
            let state = Rc::downgrade(&state);
            move |bundle| {
                if let Some(state) = state.upgrade() {
                    *state.bundle.borrow_mut() = bundle;
                    state.refresh();
                }
            }
        }),
    });
    state
        .items
        .set_header_suffix(Some(&crate::bundle::add_button(&edit)));
    let add_first = crate::bundle::add_button(&edit);
    add_first.set_css_classes(&["pill"]);
    add_first.set_halign(gtk::Align::Center);
    state.empty.set_child(Some(&add_first));
    let _ = state.edit.set(edit);
    state.refresh();

    roll.connect_clicked({
        let state = state.clone();
        move |_| state.roll()
    });
    reset.connect_clicked({
        let state = state.clone();
        move |_| {
            for spin in &state.spins {
                spin.set_value(0.0);
            }
            *state.bundle.borrow_mut() = Bundle::default();
            state.refresh();
        }
    });
    if let Mode::Pick { use_bundle } = mode {
        let (state, dialog) = (state.clone(), dialog.clone());
        use_button.connect_clicked(move |_| {
            use_bundle(state.bundle.borrow().clone());
            dialog.close();
        });
    }

    let actions = gio::SimpleActionGroup::new();
    let add = |name: &str, run: Box<dyn Fn()>| -> gio::SimpleAction {
        let a = gio::SimpleAction::new(name, None);
        a.connect_activate(move |_, _| run());
        actions.add_action(&a);
        a
    };
    add("save", {
        let (state, dialog, store, toast) =
            (state.clone(), dialog.clone(), store.clone(), toast.clone());
        Box::new(move || ask_name(&dialog, &state, &store, &toast))
    });
    add("print", {
        let (state, dialog, toast) = (state.clone(), dialog.clone(), toast.clone());
        Box::new(move || {
            let sheet = Rc::new(state.sheet());
            crate::sheet::print_sheet(&dialog, &sheet, None, &*toast)
        })
    });
    add("pdf", {
        let (state, dialog, toast) = (state.clone(), dialog.clone(), toast.clone());
        Box::new(move || {
            let sheet = Rc::new(state.sheet());
            let (dialog2, toast) = (dialog.clone(), toast.clone());
            let title = sheet.title.clone();
            crate::sheet::save_file(&dialog, &title, "pdf", tr("PDF documents"), move |path| {
                crate::sheet::print_sheet(&dialog2, &sheet, Some(path), &*toast)
            });
        })
    });
    let delete = add("delete", {
        let (state, dialog, store, toast) =
            (state.clone(), dialog.clone(), store.clone(), toast.clone());
        Box::new(move || confirm_delete(&dialog, &state, &store, &toast))
    });
    delete.set_enabled(state.saved.borrow().is_some());
    let _ = state.delete.set(delete);
    dialog.insert_action_group("treasure", Some(&actions));
    dialog.present(Some(parent));
}

impl State {
    fn counts(&self) -> Counts {
        let mut counts = [0; 26];
        for (n, spin) in counts.iter_mut().zip(&self.spins) {
            *n = spin.value_as_int().max(0) as u32;
        }
        counts
    }

    /// Roll: without Accumulate the treasure is replaced (0x5416ec).
    fn roll(self: &Rc<Self>) {
        let counts = self.counts();
        let coin = if self.coin.selected() == 1 {
            Coin::Electrum
        } else {
            Coin::Platinum
        };
        {
            let mut bundle = self.bundle.borrow_mut();
            if !self.accumulate.is_active() {
                *bundle = Bundle::default();
            }
            self.rules
                .roll(&mut self.rand.borrow_mut(), &counts, coin, &mut bundle);
        }
        self.refresh();
    }

    /// Shows the bundle and its summary.
    fn refresh(self: &Rc<Self>) {
        // Keep the place: removing the focused row would scroll to the top.
        let adjustment = self.scroll.vadjustment();
        let at = adjustment.value();
        if let Some(root) = self.scroll.root() {
            root.set_focus(None::<&gtk::Widget>);
        }
        glib::idle_add_local_once(move || adjustment.set_value(at));
        for row in self.rows.borrow_mut().drain(..) {
            self.items.remove(&row);
        }
        for row in self.summary_rows.borrow_mut().drain(..) {
            self.summary.remove(&row);
        }
        let bundle = self.bundle.borrow();
        let has_items = !bundle.is_empty();
        self.empty.set_visible(!has_items);
        self.results.set_visible(has_items);
        if !has_items {
            return;
        }
        let rows = match self.edit.get() {
            Some(edit) => crate::bundle::rows(edit, &bundle),
            None => Vec::new(),
        };
        for row in &rows {
            self.items.add(row);
        }
        *self.rows.borrow_mut() = rows;
        for line in self.rules.summary(&bundle).lines() {
            let (title, value) = line.rsplit_once(": ").unwrap_or((&line, ""));
            let row = adw::ActionRow::builder().title(title).build();
            row.add_suffix(
                &gtk::Label::builder()
                    .label(value)
                    .css_classes(["dim-label", "numeric"])
                    .build(),
            );
            self.summary.add(&row);
            self.summary_rows.borrow_mut().push(row);
        }
    }

    /// The bundle as a printable page: the items, nested, then the summary.
    fn sheet(&self) -> Sheet {
        let title = self.saved.borrow().clone();
        self.rules.sheet(
            title.as_deref().unwrap_or("Treasure"),
            &self.bundle.borrow(),
        )
    }
}

/// Save…: asks for the bundle's name (the original's "Name of Treasure
/// Bundle").
fn ask_name(dialog: &adw::Dialog, state: &Rc<State>, store: &Rc<Store>, toast: &Rc<dyn Fn(&str)>) {
    let entry = adw::EntryRow::builder()
        .title(tr("Name"))
        .activates_default(true)
        .build();
    if let Some(name) = &*state.saved.borrow() {
        entry.set_text(name);
    }
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    list.append(&entry);
    let alert = adw::AlertDialog::builder()
        .heading(tr("Save Treasure"))
        .body(tr(
            "Saved treasure appears under Saved Treasure, also in the original programs",
        ))
        .extra_child(&list)
        .default_response("save")
        .close_response("cancel")
        .build();
    alert.add_responses(&[("cancel", tr("_Cancel")), ("save", tr("_Save"))]);
    alert.set_response_appearance("save", adw::ResponseAppearance::Suggested);
    let (state, store, toast, dialog2, field) = (
        state.clone(),
        store.clone(),
        toast.clone(),
        dialog.clone(),
        entry.clone(),
    );
    alert.connect_response(Some("save"), move |_, _| {
        let name = field.text().to_string();
        let old = state.saved.borrow().clone();
        match (store.save)(old.as_deref(), &name, &state.bundle.borrow()) {
            Ok(()) => {
                dialog2.set_title(&name);
                *state.saved.borrow_mut() = Some(name);
                if let Some(delete) = state.delete.get() {
                    delete.set_enabled(true);
                }
                toast(tr("Treasure saved"));
            }
            Err(e) => toast(&e),
        }
    });
    alert.present(Some(dialog));
    entry.grab_focus();
}

fn confirm_delete(
    dialog: &adw::Dialog,
    state: &Rc<State>,
    store: &Rc<Store>,
    toast: &Rc<dyn Fn(&str)>,
) {
    let Some(name) = state.saved.borrow().clone() else {
        return;
    };
    let alert = adw::AlertDialog::builder()
        .heading(tr("Delete Saved Treasure?"))
        .body(tr!("“{name}” will be removed from Saved Treasure", name))
        .default_response("cancel")
        .close_response("cancel")
        .build();
    alert.add_responses(&[("cancel", tr("_Cancel")), ("delete", tr("_Delete"))]);
    alert.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    let (store, toast, dialog2) = (store.clone(), toast.clone(), dialog.clone());
    alert.connect_response(Some("delete"), move |_, _| match (store.delete)(&name) {
        Ok(()) => {
            toast(tr("Saved treasure deleted"));
            dialog2.close();
        }
        Err(e) => toast(&e),
    });
    alert.present(Some(dialog));
}
