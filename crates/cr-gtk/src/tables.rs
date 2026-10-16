//! Roll on a Table: the DM Tool Kit's Table Rolls. Pick a book and one of
//! its tables, set the table's options, and roll; Roll Again keeps the
//! session, so the original's re-roll of results within 1 of the previous
//! one carries over as it did in its Roll Results dialog.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_store::Installation;
use cr_store::tables::{Block, ListEntry, RollResult, Session, TableBook, TableRolls};
use cr_store::treasure::Rand;
use gtk::{gio, glib};

thread_local! {
    /// The tables of the installation last opened, kept for the session.
    static CACHE: RefCell<Option<(PathBuf, Rc<TableRolls>)>> = const { RefCell::new(None) };
    /// The book and table chosen last.
    static LAST: Cell<Option<(TableBook, usize)>> = const { Cell::new(None) };
}

/// Opens the dialog over `parent`. `open_book` opens a book (by file name)
/// at a search; `open_treasure` opens the treasure generator, which DMG
/// Table 84 opens in the original.
pub fn open(
    parent: &adw::ApplicationWindow,
    root: PathBuf,
    rand: Rc<RefCell<Rand>>,
    open_book: impl Fn(&str, Option<&str>) + 'static,
    open_treasure: impl Fn() + 'static,
) {
    let dialog = adw::Dialog::builder()
        .title("Roll on a Table")
        .content_width(680)
        .content_height(680)
        .build();
    let stack = gtk::Stack::new();
    let spinner = gtk::Spinner::builder()
        .spinning(true)
        .width_request(32)
        .height_request(32)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .build();
    stack.add_named(&spinner, Some("loading"));
    let view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    view.add_top_bar(&header);
    view.set_content(Some(&stack));
    dialog.set_child(Some(&view));
    dialog.present(Some(parent));

    let cached = CACHE.with_borrow(|c| {
        c.as_ref()
            .filter(|(r, _)| *r == root)
            .map(|(_, rolls)| rolls.clone())
    });
    let ui = Ui {
        dialog,
        stack,
        view,
        header,
        rand,
        open_book: Rc::new(open_book),
        open_treasure: Rc::new(open_treasure),
    };
    match cached {
        Some(rolls) => ui.show(rolls),
        None => {
            glib::spawn_future_local(async move {
                let worker_root = root.clone();
                let loaded = gio::spawn_blocking(move || {
                    Installation::open(worker_root).and_then(|i| i.table_rolls())
                })
                .await;
                match loaded {
                    Ok(Ok(rolls)) => {
                        let rolls = Rc::new(rolls);
                        CACHE.with_borrow_mut(|c| *c = Some((root, rolls.clone())));
                        ui.show(rolls);
                    }
                    Ok(Err(e)) => ui.error(&e.to_string()),
                    Err(_) => ui.error("The tables could not be read"),
                }
            });
        }
    }
}

/// Opens a book by file name at a search.
type OpenBook = dyn Fn(&str, Option<&str>);

struct Ui {
    dialog: adw::Dialog,
    stack: gtk::Stack,
    view: adw::ToolbarView,
    header: adw::HeaderBar,
    rand: Rc<RefCell<Rand>>,
    open_book: Rc<OpenBook>,
    open_treasure: Rc<dyn Fn()>,
}

/// The dialog once the tables are loaded.
struct Roller {
    rolls: Rc<TableRolls>,
    rand: Rc<RefCell<Rand>>,
    books: Vec<TableBook>,
    entries: RefCell<Vec<&'static ListEntry>>,
    book_row: adw::ComboRow,
    table_row: adw::ComboRow,
    options: adw::PreferencesGroup,
    param_rows: RefCell<Vec<adw::ComboRow>>,
    /// Set while the rows are being filled, so their signals are ignored.
    filling: Cell<bool>,
    results: gtk::TextBuffer,
    result: RefCell<Option<RollResult>>,
    session: RefCell<Session>,
    again: gtk::Button,
    read: gtk::Button,
    treasure: gtk::Button,
    copy: gtk::Button,
}

impl Ui {
    fn error(&self, message: &str) {
        let page = adw::StatusPage::builder()
            .icon_name("dialog-error-symbolic")
            .title("Could Not Read the Tables")
            .description(glib::markup_escape_text(message).as_str())
            .build();
        self.stack.add_named(&page, Some("error"));
        self.stack.set_visible_child_name("error");
    }

    fn show(&self, rolls: Rc<TableRolls>) {
        let books = rolls.books();
        let titles: Vec<&str> = books.iter().map(|b| b.title()).collect();
        let book_row = adw::ComboRow::builder()
            .title("Book")
            .model(&gtk::StringList::new(&titles))
            .build();
        let table_row = adw::ComboRow::builder()
            .title("Table")
            .enable_search(true)
            .expression(gtk::PropertyExpression::new(
                gtk::StringObject::static_type(),
                None::<gtk::Expression>,
                "string",
            ))
            .build();
        let choose = adw::PreferencesGroup::new();
        choose.add(&book_row);
        choose.add(&table_row);
        let options = adw::PreferencesGroup::builder()
            .title("Options")
            .visible(false)
            .build();

        let results_view = gtk::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::None)
            .left_margin(12)
            .right_margin(12)
            .top_margin(12)
            .bottom_margin(12)
            .build();
        let results = results_view.buffer();
        results.create_tag(Some("title"), &[("weight", &700i32)]);
        results.create_tag(Some("subtitle"), &[("style", &gtk::pango::Style::Italic)]);
        // The columns never wrap; the notes under them do.
        results.create_tag(Some("note"), &[("wrap-mode", &gtk::WrapMode::Word)]);
        results.set_text("Choose a table and roll.");
        let frame = gtk::Frame::builder()
            .child(
                &gtk::ScrolledWindow::builder()
                    .vexpand(true)
                    .min_content_height(160)
                    .child(&results_view)
                    .build(),
            )
            .build();
        let results_group = adw::PreferencesGroup::builder().title("Results").build();
        results_group.add(&frame);

        // The choices scroll when they are many; the results stay in
        // view below them.
        let choices = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(24)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        choices.append(&choose);
        choices.append(&options);
        let choices_scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(300)
            .child(
                &adw::Clamp::builder()
                    .maximum_size(720)
                    .child(&choices)
                    .build(),
            )
            .build();
        results_group.set_margin_start(12);
        results_group.set_margin_end(12);
        results_group.set_margin_bottom(12);
        results_group.set_vexpand(true);
        let scroller = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        scroller.append(&choices_scroller);
        scroller.append(
            &adw::Clamp::builder()
                .maximum_size(720)
                .vexpand(true)
                .child(&results_group)
                .build(),
        );
        self.stack.add_named(&scroller, Some("roller"));
        self.stack.set_visible_child_name("roller");

        let copy = gtk::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copy Results")
            .sensitive(false)
            .build();
        self.header.pack_end(&copy);
        let read = gtk::Button::builder()
            .label("Read in _Book")
            .use_underline(true)
            .visible(false)
            .build();
        let treasure = gtk::Button::builder()
            .label("Open _Treasure Generator")
            .use_underline(true)
            .visible(false)
            .build();
        let again = gtk::Button::builder()
            .label("Roll _Again")
            .use_underline(true)
            .sensitive(false)
            .tooltip_text("Roll on the same table, as the original's Roll Again did")
            .build();
        let roll = gtk::Button::builder()
            .label("_Roll")
            .use_underline(true)
            .css_classes(["suggested-action"])
            .build();
        let bar = gtk::ActionBar::new();
        bar.pack_start(&read);
        bar.pack_start(&treasure);
        bar.pack_end(&roll);
        bar.pack_end(&again);
        self.view.add_bottom_bar(&bar);
        self.dialog.set_default_widget(Some(&roll));
        roll.grab_focus();

        let roller = Rc::new(Roller {
            rolls,
            rand: self.rand.clone(),
            books,
            entries: RefCell::default(),
            book_row,
            table_row,
            options,
            param_rows: RefCell::default(),
            filling: Cell::new(false),
            results,
            result: RefCell::default(),
            session: RefCell::default(),
            again: again.clone(),
            read: read.clone(),
            treasure: treasure.clone(),
            copy: copy.clone(),
        });

        let (last_book, last_table) = LAST.get().unwrap_or((TableBook::Dmg, 0));
        let book = roller
            .books
            .iter()
            .position(|b| *b == last_book)
            .unwrap_or(0);
        roller.book_row.set_selected(book as u32);
        roller.fill_tables(last_table);

        let weak = Rc::downgrade(&roller);
        roller.book_row.connect_selected_notify(move |_| {
            if let Some(r) = weak.upgrade() {
                r.fill_tables(0);
            }
        });
        let weak = Rc::downgrade(&roller);
        roller.table_row.connect_selected_notify(move |_| {
            if let Some(r) = weak.upgrade()
                && !r.filling.get()
            {
                r.fill_params();
            }
        });
        let weak = Rc::downgrade(&roller);
        roll.connect_clicked(move |_| {
            if let Some(r) = weak.upgrade() {
                r.roll(false);
            }
        });
        let weak = Rc::downgrade(&roller);
        again.connect_clicked(move |_| {
            if let Some(r) = weak.upgrade() {
                r.roll(true);
            }
        });
        let weak = Rc::downgrade(&roller);
        copy.connect_clicked(move |button| {
            if let Some(r) = weak.upgrade()
                && let Some(result) = &*r.result.borrow()
            {
                button.clipboard().set_text(&result.to_text());
            }
        });
        let weak = Rc::downgrade(&roller);
        let open_book = self.open_book.clone();
        let dialog = self.dialog.clone();
        read.connect_clicked(move |_| {
            let Some(r) = weak.upgrade() else { return };
            let found = r.result.borrow().as_ref().and_then(|x| x.book_table);
            if let Some((book, number)) = found {
                dialog.close();
                open_book(book.short_name(), Some(&format!("Table {number}:")));
            }
        });
        let open_treasure = self.open_treasure.clone();
        let dialog = self.dialog.clone();
        treasure.connect_clicked(move |_| {
            dialog.close();
            open_treasure();
        });
        // The dialog keeps its roller alive.
        let keep = RefCell::new(Some(roller));
        self.dialog.connect_closed(move |_| {
            keep.borrow_mut().take();
        });
    }
}

impl Roller {
    fn book(&self) -> TableBook {
        self.books
            .get(self.book_row.selected() as usize)
            .copied()
            .unwrap_or(TableBook::Dmg)
    }

    fn entry(&self) -> Option<&'static ListEntry> {
        self.entries
            .borrow()
            .get(self.table_row.selected() as usize)
            .copied()
    }

    /// Lists the tables of the chosen book and selects `selected`.
    fn fill_tables(self: &Rc<Self>, selected: usize) {
        let entries = self.rolls.entries(self.book());
        let titles: Vec<String> = entries.iter().map(|e| self.rolls.entry_title(e)).collect();
        let titles: Vec<&str> = titles.iter().map(String::as_str).collect();
        *self.entries.borrow_mut() = entries;
        self.filling.set(true);
        self.table_row
            .set_model(Some(&gtk::StringList::new(&titles)));
        self.table_row
            .set_selected(selected.min(titles.len().saturating_sub(1)) as u32);
        self.filling.set(false);
        self.fill_params();
    }

    fn chosen(&self) -> Vec<usize> {
        self.param_rows
            .borrow()
            .iter()
            .map(|row| row.selected() as usize)
            .collect()
    }

    /// Makes a row for each parameter of the chosen table. A new table is
    /// a new Roll Results session.
    fn fill_params(self: &Rc<Self>) {
        for row in self.param_rows.borrow_mut().drain(..) {
            self.options.remove(&row);
        }
        self.session.replace(Session::default());
        self.again.set_sensitive(false);
        let Some(entry) = self.entry() else { return };
        LAST.set(Some((self.book(), self.table_row.selected() as usize)));
        let params = self.rolls.params(entry, &[]);
        self.options.set_visible(!params.is_empty());
        for (i, param) in params.iter().enumerate() {
            let options: Vec<&str> = param.options.iter().map(String::as_str).collect();
            let row = adw::ComboRow::builder()
                .title(&param.label)
                .model(&gtk::StringList::new(&options))
                .build();
            let weak = Rc::downgrade(self);
            row.connect_selected_notify(move |_| {
                if let Some(r) = weak.upgrade()
                    && !r.filling.get()
                {
                    r.refresh_params(i);
                }
            });
            self.options.add(&row);
            self.param_rows.borrow_mut().push(row);
        }
    }

    /// Options can depend on earlier choices (the called shots depend on
    /// the target): refreshes the rows after `changed`.
    fn refresh_params(&self, changed: usize) {
        let Some(entry) = self.entry() else { return };
        let chosen = self.chosen();
        let params = self.rolls.params(entry, &chosen);
        let rows = self.param_rows.borrow();
        self.filling.set(true);
        for (row, param) in rows.iter().zip(&params).skip(changed + 1) {
            let options: Vec<&str> = param.options.iter().map(String::as_str).collect();
            let selected = row.selected();
            row.set_model(Some(&gtk::StringList::new(&options)));
            if (selected as usize) < options.len() {
                row.set_selected(selected);
            }
        }
        self.filling.set(false);
    }

    fn roll(&self, again: bool) {
        let Some(entry) = self.entry() else { return };
        if !again {
            self.session.replace(Session::default());
        }
        let result = self.rolls.roll(
            entry,
            &self.chosen(),
            &mut self.session.borrow_mut(),
            &mut self.rand.borrow_mut(),
        );
        self.show(&result);
        self.read.set_visible(result.book_table.is_some());
        self.treasure.set_visible(result.treasure);
        self.again
            .set_sensitive(result.book_table.is_none() && !result.treasure);
        self.copy.set_sensitive(true);
        self.result.replace(Some(result));
    }

    fn show(&self, result: &RollResult) {
        let buffer = &self.results;
        buffer.set_text("");
        let mut end = buffer.end_iter();
        for (i, block) in result.blocks.iter().enumerate() {
            if i > 0 {
                buffer.insert(&mut end, "\n");
            }
            if let Some(title) = &block.title {
                buffer.insert_with_tags_by_name(&mut end, &format!("{title}\n"), &["title"]);
            }
            if let Some(subtitle) = &block.subtitle {
                buffer.insert_with_tags_by_name(&mut end, &format!("{subtitle}\n"), &["subtitle"]);
            }
            let body = Block {
                title: None,
                subtitle: None,
                notes: Vec::new(),
                ..block.clone()
            }
            .to_text();
            let headed = block.title.is_some() || block.subtitle.is_some();
            if !body.is_empty() {
                if headed {
                    buffer.insert(&mut end, "\n");
                }
                buffer.insert(&mut end, &body);
            }
            if !block.notes.is_empty() {
                if headed || !body.is_empty() {
                    buffer.insert(&mut end, "\n");
                }
                for note in &block.notes {
                    buffer.insert_with_tags_by_name(&mut end, &format!("{note}\n"), &["note"]);
                }
            }
        }
    }
}
