//! The book reader: a window per book with the contents on the side, the
//! text in a `TextView` with the book's formatting, find, zoom, and the
//! reading position remembered between sessions.
//!
//! A book is read and parsed on a worker thread; the text then goes into
//! the buffer a slice at a time from idle callbacks, so the window stays
//! responsive while the Monstrous Manual (19 000 paragraphs) loads.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

use adw::prelude::*;
use cr_app::action;
use cr_formats::rtf::{Align, Document, Format, Inline, Paragraph, Picture, Rgb, Script, TabKind};
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::books::{BOOK_KEY, Book, Heading, fold_char, outline};
use gtk::{gdk, gio, glib, pango};

/// The zoom levels Ctrl+plus and Ctrl+minus step through.
const ZOOM_STEPS: [f64; 13] = [
    0.5, 0.67, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0,
];

/// The widest the text gets at zoom 1, in pixels; book pages are about
/// 6.5 inches of text.
const TEXT_WIDTH: f64 = 760.0;

/// The space left and right of the text, in pixels.
const MARGIN: i32 = 24;

/// The detail-page group of a book in the Books section: a button that
/// opens it.
pub fn detail_group(key: &str) -> Option<adw::PreferencesGroup> {
    let name = key.strip_prefix(BOOK_KEY)?;
    let button = gtk::Button::builder()
        .label(tr("_Read Book"))
        .use_underline(true)
        .halign(gtk::Align::Start)
        .action_name(action::OPEN_BOOK.detailed)
        .action_target(&name.to_variant())
        .css_classes(["pill", "suggested-action"])
        .build();
    let group = adw::PreferencesGroup::new();
    group.add(&button);
    Some(group)
}

/// Opens the book `name` of `install` in a reader window, or raises the
/// window already showing it. With `find`, the find bar starts with that
/// text.
pub fn open(
    parent: &adw::ApplicationWindow,
    install: &Installation,
    name: &str,
    find: Option<&str>,
) -> Result<(), String> {
    let book = install
        .book(name)
        .ok_or_else(|| tr!("{name} is not in this installation", name))?;
    let key = format!("{}|{}", install.root().display(), book.info.short_name());
    let existing = OPEN.with_borrow(|open| open.get(&key).and_then(Weak::upgrade));
    let reader = match existing {
        Some(reader) => reader,
        None => {
            let app = parent
                .application()
                .ok_or(tr("The application is shutting down"))?;
            let reader = Reader::new(&app, book, key.clone());
            OPEN.with_borrow_mut(|open| {
                open.retain(|_, r| r.strong_count() > 0);
                open.insert(key, Rc::downgrade(&reader));
            });
            reader
        }
    };
    if let Some(find) = find {
        // A table caption: the book's List of Tables has the first match.
        reader.search_bar.set_search_mode(true);
        reader.skip_first_match.set(true);
        if reader.search_entry.text() == find {
            reader.search();
        } else {
            // Searched when the entry reports the change.
            reader.search_entry.set_text(find);
        }
    }
    reader.window.present();
    Ok(())
}

thread_local! {
    /// The readers open, by state key, so a book opens once.
    static OPEN: RefCell<HashMap<String, Weak<Reader>>> = RefCell::default();
}

/// Paragraph formatting that a tag carries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ParaKey {
    align: Align,
    left: i32,
    first: i32,
    right: i32,
    before: i32,
    after: i32,
    tabs: Vec<(i32, TabKind)>,
}

impl ParaKey {
    fn of(p: &Paragraph) -> Self {
        Self {
            align: p.align,
            left: p.left_indent,
            first: p.first_indent,
            right: p.right_indent,
            before: p.space_before,
            after: p.space_after,
            tabs: p.tabs.iter().map(|t| (t.pos, t.kind)).collect(),
        }
    }
}

/// A picture in the text: its widget and its size at zoom 1.
struct Inset {
    widget: gtk::Picture,
    anchor: gtk::TextChildAnchor,
    width: f64,
    height: f64,
}

/// A window action of the reader.
type Action = fn(&Rc<Reader>);

struct Reader {
    window: adw::ApplicationWindow,
    view: gtk::TextView,
    clamp: adw::ClampScrollable,
    /// The book's buffer; attached to the view once filled.
    buffer: gtk::TextBuffer,
    stack: gtk::Stack,
    progress: gtk::ProgressBar,
    split: adw::OverlaySplitView,
    outline_list: gtk::ListView,
    outline_selection: gtk::SingleSelection,
    headings: Rc<RefCell<Vec<Heading>>>,
    search_bar: gtk::SearchBar,
    search_entry: gtk::SearchEntry,
    match_label: gtk::Label,
    /// Font family of each font of the book.
    families: RefCell<Vec<String>>,
    run_tags: RefCell<HashMap<Format, gtk::TextTag>>,
    para_tags: RefCell<HashMap<ParaKey, gtk::TextTag>>,
    match_tag: gtk::TextTag,
    current_tag: gtk::TextTag,
    jump: gtk::TextMark,
    insets: RefCell<Vec<Inset>>,
    /// Character offset of each paragraph in the buffer.
    starts: RefCell<Vec<i32>>,
    /// The buffer's text folded for searching, one char per buffer char.
    folded: RefCell<Vec<char>>,
    /// Offsets of the matches of the find text, and its length.
    matches: RefCell<Vec<i32>>,
    needle_len: Cell<i32>,
    current: Cell<Option<usize>>,
    /// The next search goes to the second match.
    skip_first_match: Cell<bool>,
    zoom: Cell<f64>,
    loaded: Cell<bool>,
    save_pending: Cell<bool>,
    /// The key of the book in the state file.
    key: String,
}

impl Reader {
    fn new(app: &gtk::Application, book: Book, key: String) -> Rc<Self> {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(book.info.title)
            .default_width(1000)
            .default_height(760)
            .width_request(360)
            .height_request(294)
            .build();

        // Text.
        let view = gtk::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk::WrapMode::WordChar)
            .left_margin(MARGIN)
            .right_margin(MARGIN)
            .top_margin(24)
            .bottom_margin(48)
            .build();
        let clamp = adw::ClampScrollable::builder()
            .maximum_size(TEXT_WIDTH as i32)
            .tightening_threshold(TEXT_WIDTH as i32)
            .child(&view)
            .build();
        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&clamp)
            .build();
        let buffer = gtk::TextBuffer::new(None);
        let match_tag = buffer
            .create_tag(
                Some("match"),
                &[("background", &"rgba(246, 211, 45, 0.45)")],
            )
            .unwrap();
        let current_tag = buffer
            .create_tag(
                Some("current"),
                &[("background", &"rgba(255, 120, 0, 0.8)")],
            )
            .unwrap();
        let jump = buffer.create_mark(Some("jump"), &buffer.start_iter(), true);

        // Loading and error pages.
        let progress = gtk::ProgressBar::builder().width_request(240).build();
        let loading = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(18)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .build();
        loading.append(
            &gtk::Spinner::builder()
                .spinning(true)
                .width_request(32)
                .height_request(32)
                .build(),
        );
        loading.append(
            &gtk::Label::builder()
                .label(tr("Opening Book…"))
                .css_classes(["title-3"])
                .build(),
        );
        loading.append(&progress);
        let stack = gtk::Stack::new();
        stack.add_named(&loading, Some("loading"));
        stack.add_named(&scroller, Some("text"));

        // Contents.
        let headings: Rc<RefCell<Vec<Heading>>> = Rc::default();
        let titles = gtk::StringList::new(&[]);
        let outline_selection = gtk::SingleSelection::builder()
            .model(&titles)
            .autoselect(false)
            .can_unselect(true)
            .build();
        let outline_list = gtk::ListView::builder()
            .model(&outline_selection)
            .factory(&heading_factory(headings.clone()))
            .single_click_activate(true)
            .css_classes(["navigation-sidebar"])
            .build();
        let sidebar = adw::ToolbarView::new();
        sidebar.add_top_bar(
            &adw::HeaderBar::builder()
                .show_end_title_buttons(false)
                .title_widget(&adw::WindowTitle::new(tr("Contents"), ""))
                .build(),
        );
        sidebar.set_content(Some(
            &gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .child(&outline_list)
                .build(),
        ));

        // Find bar.
        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text(tr("Find in Book"))
            .hexpand(true)
            .build();
        let match_label = gtk::Label::builder()
            .css_classes(["dim-label", "numeric"])
            .width_chars(10)
            .build();
        let previous = gtk::Button::builder()
            .icon_name("go-up-symbolic")
            .tooltip_text(tr("Previous Match"))
            .build();
        let next = gtk::Button::builder()
            .icon_name("go-down-symbolic")
            .tooltip_text(tr("Next Match"))
            .build();
        let find_box = gtk::Box::builder().spacing(6).build();
        find_box.append(&search_entry);
        find_box.append(&match_label);
        let arrows = gtk::Box::builder().css_classes(["linked"]).build();
        arrows.append(&previous);
        arrows.append(&next);
        find_box.append(&arrows);
        let search_bar = gtk::SearchBar::builder()
            .child(
                &adw::Clamp::builder()
                    .maximum_size(600)
                    .child(&find_box)
                    .build(),
            )
            .show_close_button(true)
            .build();
        search_bar.connect_entry(&search_entry);

        // Header.
        let header = adw::HeaderBar::builder()
            .title_widget(&adw::WindowTitle::new(
                book.info.title,
                tr(book.info.category.title()),
            ))
            .build();
        let sidebar_button = gtk::ToggleButton::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text(tr("Contents"))
            .active(true)
            .build();
        header.pack_start(&sidebar_button);
        let menu = gio::Menu::new();
        let find_section = gio::Menu::new();
        find_section.append(Some(tr("_Find…")), Some(action::BOOK_FIND.detailed));
        menu.append_section(None, &find_section);
        let zoom_section = gio::Menu::new();
        zoom_section.append(Some(tr("Zoom _In")), Some(action::BOOK_ZOOM_IN.detailed));
        zoom_section.append(Some(tr("Zoom _Out")), Some(action::BOOK_ZOOM_OUT.detailed));
        zoom_section.append(
            Some(tr("_Reset Zoom")),
            Some(action::BOOK_ZOOM_RESET.detailed),
        );
        menu.append_section(None, &zoom_section);
        header.pack_end(
            &gtk::MenuButton::builder()
                .icon_name("open-menu-symbolic")
                .tooltip_text(tr("Main Menu"))
                .menu_model(&menu)
                .primary(true)
                .build(),
        );
        let find_button = gtk::ToggleButton::builder()
            .icon_name("edit-find-symbolic")
            .tooltip_text(tr("Find"))
            .build();
        find_button
            .bind_property("active", &search_bar, "search-mode-enabled")
            .bidirectional()
            .sync_create()
            .build();
        header.pack_end(&find_button);

        let content = adw::ToolbarView::new();
        content.add_top_bar(&header);
        content.add_top_bar(&search_bar);
        content.set_content(Some(&stack));
        let split = adw::OverlaySplitView::builder()
            .sidebar(&sidebar)
            .content(&content)
            .min_sidebar_width(220.0)
            .max_sidebar_width(320.0)
            .build();
        sidebar_button
            .bind_property("active", &split, "show-sidebar")
            .bidirectional()
            .sync_create()
            .build();
        window.set_content(Some(&split));
        let narrow = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            720.0,
            adw::LengthUnit::Sp,
        ));
        narrow.add_setter(&split, "collapsed", Some(&true.to_value()));
        narrow.add_setter(&sidebar_button, "active", Some(&false.to_value()));
        window.add_breakpoint(narrow);

        let zoom = state::get("zoom")
            .and_then(|z| z.parse::<f64>().ok())
            .filter(|z| (ZOOM_STEPS[0]..=ZOOM_STEPS[ZOOM_STEPS.len() - 1]).contains(z))
            .unwrap_or(1.0);
        let reader = Rc::new(Self {
            window,
            view,
            clamp,
            buffer,
            stack,
            progress,
            split,
            outline_list,
            outline_selection,
            headings,
            search_bar,
            search_entry,
            match_label,
            families: RefCell::default(),
            run_tags: RefCell::default(),
            para_tags: RefCell::default(),
            match_tag,
            current_tag,
            jump,
            insets: RefCell::default(),
            starts: RefCell::default(),
            folded: RefCell::default(),
            matches: RefCell::default(),
            needle_len: Cell::new(0),
            current: Cell::new(None),
            skip_first_match: Cell::new(false),
            zoom: Cell::new(zoom),
            loaded: Cell::new(false),
            save_pending: Cell::new(false),
            key,
        });
        reader.clamp.set_maximum_size((TEXT_WIDTH * zoom) as i32);
        reader.connect(&previous, &next, &scroller);
        reader.load(book);
        reader
    }

    fn connect(
        self: &Rc<Self>,
        previous: &gtk::Button,
        next: &gtk::Button,
        scroller: &gtk::ScrolledWindow,
    ) {
        let weak = Rc::downgrade(self);
        let with = move |f: Action| {
            let weak = weak.clone();
            move || {
                if let Some(reader) = weak.upgrade() {
                    f(&reader)
                }
            }
        };
        let actions: [(&str, Action); 6] = [
            (action::BOOK_FIND.name, |r| {
                r.search_bar.set_search_mode(true);
                r.search_entry.grab_focus();
                r.search_entry.select_region(0, -1);
            }),
            (action::BOOK_ZOOM_IN.name, |r| r.step_zoom(1)),
            (action::BOOK_ZOOM_OUT.name, |r| r.step_zoom(-1)),
            (action::BOOK_ZOOM_RESET.name, |r| r.set_zoom(1.0)),
            (action::BOOK_CONTENTS.name, |r| {
                r.split.set_show_sidebar(!r.split.shows_sidebar())
            }),
            (action::BOOK_CLOSE.name, |r| r.window.close()),
        ];
        let entries: Vec<_> = actions
            .into_iter()
            .map(|(name, f)| {
                let run = with(f);
                gio::ActionEntry::builder(name)
                    .activate(move |_: &adw::ApplicationWindow, _, _| run())
                    .build()
            })
            .collect();
        self.window.add_action_entries(entries);
        if let Some(app) = self.window.application() {
            app.set_accels_for_action(action::BOOK_FIND.detailed, &["<Control>f"]);
            app.set_accels_for_action(
                action::BOOK_ZOOM_IN.detailed,
                &["<Control>plus", "<Control>equal", "<Control>KP_Add"],
            );
            app.set_accels_for_action(
                action::BOOK_ZOOM_OUT.detailed,
                &["<Control>minus", "<Control>KP_Subtract"],
            );
            app.set_accels_for_action(
                action::BOOK_ZOOM_RESET.detailed,
                &["<Control>0", "<Control>KP_0"],
            );
            app.set_accels_for_action(action::BOOK_CONTENTS.detailed, &["F9"]);
            app.set_accels_for_action(action::BOOK_CLOSE.detailed, &["<Control>w"]);
        }

        let go = with(|r| r.step_match(1));
        next.connect_clicked(move |_| go());
        let go = with(|r| r.step_match(-1));
        previous.connect_clicked(move |_| go());
        let go = with(|r| r.step_match(1));
        self.search_entry.connect_activate(move |_| go());
        let go = with(|r| r.step_match(1));
        self.search_entry.connect_next_match(move |_| go());
        let go = with(|r| r.step_match(-1));
        self.search_entry.connect_previous_match(move |_| go());
        let go = with(|r| r.search());
        self.search_entry.connect_search_changed(move |_| go());
        let go = with(|r| {
            if !r.search_bar.is_search_mode() {
                r.clear_matches();
            } else if !r.search_entry.text().is_empty() {
                r.search();
            }
        });
        self.search_bar
            .connect_search_mode_enabled_notify(move |_| go());

        let weak = Rc::downgrade(self);
        self.outline_list.connect_activate(move |_, position| {
            let Some(reader) = weak.upgrade() else { return };
            let paragraph = reader
                .headings
                .borrow()
                .get(position as usize)
                .map(|h| h.paragraph);
            if let Some(paragraph) = paragraph {
                reader.go_to_paragraph(paragraph);
                if reader.split.is_collapsed() {
                    reader.split.set_show_sidebar(false);
                }
                reader.view.grab_focus();
            }
        });

        // The text never scrolls sideways; scrolling to a mark would
        // otherwise shift it when a tab-aligned table is wider than the
        // view.
        scroller.hadjustment().connect_value_changed(|a| {
            if a.value() != 0.0 {
                a.set_value(0.0);
            }
        });
        // The position is saved a moment after scrolling stops, and the
        // contents follow it.
        let weak = Rc::downgrade(self);
        scroller.vadjustment().connect_value_changed(move |_| {
            let Some(reader) = weak.upgrade() else { return };
            if !reader.loaded.get() || reader.save_pending.replace(true) {
                return;
            }
            let weak = Rc::downgrade(&reader);
            glib::timeout_add_local_once(Duration::from_millis(600), move || {
                if let Some(reader) = weak.upgrade() {
                    reader.save_pending.set(false);
                    reader.position_changed();
                }
            });
        });
        // The book's colours are adjusted for dark backgrounds.
        let weak = Rc::downgrade(self);
        let style = adw::StyleManager::default();
        let dark = style.connect_dark_notify(move |_| {
            if let Some(reader) = weak.upgrade() {
                reader.restyle();
            }
        });
        // The window keeps its reader alive until it closes.
        let keep = RefCell::new(Some((self.clone(), dark)));
        self.window.connect_close_request(move |_| {
            if let Some((reader, dark)) = keep.borrow_mut().take() {
                reader.save_position();
                style.disconnect(dark);
                OPEN.with_borrow_mut(|open| open.remove(&reader.key));
            }
            glib::Propagation::Proceed
        });
    }

    /// Parses the book on a worker thread, then fills the buffer.
    fn load(self: &Rc<Self>, book: Book) {
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let colors = book.info.heading_colors;
            let parsed = gio::spawn_blocking(move || {
                book.read().map(|doc| {
                    let headings = outline(&doc, colors);
                    (doc, headings)
                })
            })
            .await;
            let Some(reader) = weak.upgrade() else { return };
            match parsed {
                Ok(Ok((doc, headings))) => reader.fill(doc, headings),
                Ok(Err(e)) => reader.show_error(&e.to_string()),
                Err(_) => reader.show_error(tr("The book could not be read")),
            }
        });
    }

    fn show_error(&self, message: &str) {
        let page = adw::StatusPage::builder()
            .icon_name("dialog-error-symbolic")
            .title(tr("Could Not Open Book"))
            .description(glib::markup_escape_text(message).as_str())
            .build();
        self.stack.add_named(&page, Some("error"));
        self.stack.set_visible_child_name("error");
    }

    /// Inserts the paragraphs a few milliseconds' worth at a time.
    fn fill(self: &Rc<Self>, doc: Document, headings: Vec<Heading>) {
        *self.families.borrow_mut() = doc.fonts.iter().map(family).collect();
        let doc = Rc::new(doc);
        let next = Cell::new(0usize);
        let mut end = self.buffer.end_iter();
        let weak = Rc::downgrade(self);
        let headings = RefCell::new(Some(headings));
        glib::idle_add_local(move || {
            let Some(reader) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let started = Instant::now();
            let total = doc.paragraphs.len();
            while next.get() < total && started.elapsed() < Duration::from_millis(12) {
                reader.insert_paragraph(&mut end, &doc.paragraphs[next.get()]);
                next.set(next.get() + 1);
            }
            reader
                .progress
                .set_fraction(next.get() as f64 / total.max(1) as f64);
            if next.get() < total {
                return glib::ControlFlow::Continue;
            }
            reader.finish(headings.take().unwrap_or_default());
            glib::ControlFlow::Break
        });
    }

    fn insert_paragraph(&self, end: &mut gtk::TextIter, p: &Paragraph) {
        let para = self.para_tag(p);
        let mut folded = self.folded.borrow_mut();
        self.starts.borrow_mut().push(folded.len() as i32);
        for item in &p.items {
            match item {
                Inline::Text(text, format) => {
                    let run = self.run_tag(format);
                    self.buffer.insert_with_tags(end, text, &[&run, &para]);
                    folded.extend(text.chars().map(fold_char));
                }
                Inline::Picture(picture) => {
                    let Some(inset) = self.inset(picture, end) else {
                        continue;
                    };
                    let mut start = *end;
                    start.backward_char();
                    self.buffer.apply_tag(&para, &start, end);
                    folded.push('\u{fffc}');
                    self.insets.borrow_mut().push(inset);
                }
            }
        }
        self.buffer.insert_with_tags(end, "\n", &[&para]);
        folded.push('\n');
    }

    fn inset(&self, picture: &Picture, end: &mut gtk::TextIter) -> Option<Inset> {
        let texture = gdk::Texture::from_bytes(&glib::Bytes::from(&picture.data)).ok()?;
        let (mut width, mut height) = (picture.width as f64 / 15.0, picture.height as f64 / 15.0);
        if width <= 0.0 || height <= 0.0 {
            (width, height) = (texture.width() as f64, texture.height() as f64);
        }
        let widget = gtk::Picture::builder()
            .paintable(&texture)
            .can_shrink(true)
            .content_fit(gtk::ContentFit::Contain)
            .build();
        let anchor = self.buffer.create_child_anchor(end);
        Some(Inset {
            widget,
            anchor,
            width,
            height,
        })
    }

    /// Attaches the filled buffer, restores the reading position and
    /// shows the text.
    fn finish(self: &Rc<Self>, headings: Vec<Heading>) {
        self.view.set_buffer(Some(&self.buffer));
        for inset in self.insets.borrow().iter() {
            self.view.add_child_at_anchor(&inset.widget, &inset.anchor);
        }
        self.size_insets();
        let titles: Vec<&str> = headings.iter().map(|h| h.title.as_str()).collect();
        let model = gtk::StringList::new(&titles);
        let empty = headings.is_empty();
        *self.headings.borrow_mut() = headings;
        self.outline_selection.set_model(Some(&model));
        if empty {
            self.split.set_show_sidebar(false);
        }
        self.stack.set_visible_child_name("text");
        self.view.grab_focus();
        // Scrolling waits until the view has its size.
        let weak = Rc::downgrade(self);
        glib::idle_add_local_once(move || {
            let Some(reader) = weak.upgrade() else { return };
            reader.loaded.set(true);
            let saved = state::get(&reader.key).and_then(|p| p.parse::<usize>().ok());
            if reader.search_bar.is_search_mode() && !reader.search_entry.text().is_empty() {
                reader.search();
            } else if let Some(paragraph) = saved {
                reader.go_to_paragraph(paragraph);
            }
            reader.select_heading(saved.unwrap_or(0));
        });
    }

    fn twips(&self, twips: i32) -> i32 {
        (twips as f64 / 15.0 * self.zoom.get()).round() as i32
    }

    fn run_tag(&self, format: &Format) -> gtk::TextTag {
        if let Some(tag) = self.run_tags.borrow().get(format) {
            return tag.clone();
        }
        let tag = gtk::TextTag::new(None);
        self.buffer.tag_table().add(&tag);
        self.style_run(&tag, format);
        self.run_tags.borrow_mut().insert(*format, tag.clone());
        tag
    }

    fn style_run(&self, tag: &gtk::TextTag, f: &Format) {
        let zoom = self.zoom.get();
        let points = f.size as f64 / 2.0;
        let families = self.families.borrow();
        let family = families
            .get(f.font as usize)
            .map_or("Serif", String::as_str);
        tag.set_family(Some(family));
        let (size, rise) = match f.script {
            Script::Normal => (points, 0.0),
            Script::Super => (points * 0.7, points * 0.35),
            Script::Sub => (points * 0.7, -points * 0.15),
        };
        tag.set_size_points(size * zoom);
        tag.set_rise((rise * zoom * pango::SCALE as f64) as i32);
        tag.set_weight(if f.bold { 700 } else { 400 });
        tag.set_style(if f.italic {
            pango::Style::Italic
        } else {
            pango::Style::Normal
        });
        tag.set_underline(if f.underline {
            pango::Underline::Single
        } else {
            pango::Underline::None
        });
        tag.set_variant(if f.small_caps {
            pango::Variant::SmallCaps
        } else {
            pango::Variant::Normal
        });
        match f
            .color
            .and_then(|c| shown_color(c, adw::StyleManager::default().is_dark()))
        {
            Some(rgba) => tag.set_foreground_rgba(Some(&rgba)),
            None => tag.set_property("foreground-set", false),
        }
    }

    fn para_tag(&self, p: &Paragraph) -> gtk::TextTag {
        let key = ParaKey::of(p);
        if let Some(tag) = self.para_tags.borrow().get(&key) {
            return tag.clone();
        }
        let tag = gtk::TextTag::new(None);
        self.buffer.tag_table().add(&tag);
        self.style_para(&tag, &key);
        self.para_tags.borrow_mut().insert(key, tag.clone());
        tag
    }

    fn style_para(&self, tag: &gtk::TextTag, key: &ParaKey) {
        tag.set_justification(match key.align {
            Align::Left => gtk::Justification::Left,
            Align::Center => gtk::Justification::Center,
            Align::Right => gtk::Justification::Right,
            Align::Justify => gtk::Justification::Fill,
        });
        // The tags' margins replace the view's, so they include it.
        let left = self.twips(key.left).max(0);
        tag.set_left_margin(MARGIN + left);
        // A hanging indent cannot reach left of the margin.
        tag.set_indent(self.twips(key.first).max(-left));
        tag.set_right_margin(MARGIN + self.twips(key.right).max(0));
        tag.set_pixels_above_lines(self.twips(key.before).max(0));
        tag.set_pixels_below_lines(self.twips(key.after).max(0));
        tag.set_tabs(Some(&self.tab_array(key)));
    }

    /// The paragraph's tab stops, measured from its left indent as Pango
    /// measures them, followed by Word's default stops every half inch.
    fn tab_array(&self, key: &ParaKey) -> pango::TabArray {
        let mut stops: Vec<(i32, pango::TabAlign)> = key
            .tabs
            .iter()
            .filter(|(pos, _)| *pos > key.left)
            .map(|&(pos, kind)| {
                let align = match kind {
                    TabKind::Left => pango::TabAlign::Left,
                    TabKind::Center => pango::TabAlign::Center,
                    TabKind::Right => pango::TabAlign::Right,
                    TabKind::Decimal => pango::TabAlign::Decimal,
                };
                (pos - key.left, align)
            })
            .collect();
        stops.sort_by_key(|s| s.0);
        let last = stops.last().map_or(0, |s| s.0 + key.left);
        let mut default = (last / 720 + 1) * 720;
        while default <= 14400 {
            if default > key.left {
                stops.push((default - key.left, pango::TabAlign::Left));
            }
            default += 720;
        }
        let mut tabs = pango::TabArray::new(stops.len() as i32, true);
        for (i, (pos, align)) in stops.into_iter().enumerate() {
            tabs.set_tab(i as i32, align, self.twips(pos));
        }
        tabs
    }

    fn size_insets(&self) {
        let zoom = self.zoom.get();
        for inset in self.insets.borrow().iter() {
            inset.widget.set_size_request(
                (inset.width * zoom).round() as i32,
                (inset.height * zoom).round() as i32,
            );
        }
    }

    /// Applies the zoom and colour scheme to every tag.
    fn restyle(&self) {
        for (format, tag) in self.run_tags.borrow().iter() {
            self.style_run(tag, format);
        }
        for (key, tag) in self.para_tags.borrow().iter() {
            self.style_para(tag, key);
        }
        self.size_insets();
        self.clamp
            .set_maximum_size((TEXT_WIDTH * self.zoom.get()) as i32);
        self.clamp
            .set_tightening_threshold((TEXT_WIDTH * self.zoom.get()) as i32);
    }

    fn step_zoom(&self, direction: i32) {
        let zoom = self.zoom.get();
        let next = if direction > 0 {
            ZOOM_STEPS.iter().copied().find(|z| *z > zoom + 0.001)
        } else {
            ZOOM_STEPS.iter().rev().copied().find(|z| *z < zoom - 0.001)
        };
        if let Some(next) = next {
            self.set_zoom(next);
        }
    }

    fn set_zoom(&self, zoom: f64) {
        if (zoom - self.zoom.get()).abs() < 0.001 {
            return;
        }
        let paragraph = self.paragraph_at_top();
        self.zoom.set(zoom);
        self.restyle();
        state::set("zoom", &zoom.to_string());
        if let Some(paragraph) = paragraph {
            self.go_to_paragraph(paragraph);
            self.keep_at_jump();
        }
    }

    /// Scrolls to the jump mark again while the view lays out the restyled
    /// text: lines it has not measured yet are only estimated, so the
    /// first scroll lands elsewhere (another chapter, after a zoom). It
    /// stops once the text's height has settled.
    fn keep_at_jump(&self) {
        let jump = self.jump.clone();
        // The height last seen and the frames it has stayed so.
        let state = Cell::new((-1.0, 0u32));
        let started = std::time::Instant::now();
        self.view.add_tick_callback(move |view, _| {
            let upper = view.vadjustment().map_or(0.0, |a| a.upper());
            let (last_upper, still) = state.get();
            let still = if upper == last_upper { still + 1 } else { 0 };
            state.set((upper, still));
            view.scroll_to_mark(&jump, 0.0, true, 0.0, 0.0);
            if still < 20 && started.elapsed() < Duration::from_secs(10) {
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
    }

    fn go_to_paragraph(&self, paragraph: usize) {
        let Some(&offset) = self.starts.borrow().get(paragraph) else {
            return;
        };
        let iter = self.buffer.iter_at_offset(offset);
        self.buffer.move_mark(&self.jump, &iter);
        self.view.scroll_to_mark(&self.jump, 0.0, true, 0.0, 0.0);
    }

    /// The paragraph at the top of the view.
    fn paragraph_at_top(&self) -> Option<usize> {
        if !self.loaded.get() {
            return None;
        }
        let (_, y) = self
            .view
            .window_to_buffer_coords(gtk::TextWindowType::Widget, 0, 0);
        let (iter, _) = self.view.line_at_y(y.max(0));
        let starts = self.starts.borrow();
        Some(
            starts
                .partition_point(|&s| s <= iter.offset())
                .saturating_sub(1),
        )
    }

    fn position_changed(&self) {
        if let Some(paragraph) = self.paragraph_at_top() {
            self.select_heading(paragraph);
            state::set(&self.key, &paragraph.to_string());
        }
    }

    fn save_position(&self) {
        if let Some(paragraph) = self.paragraph_at_top() {
            state::set(&self.key, &paragraph.to_string());
        }
    }

    /// Selects the heading of the section a paragraph is in.
    fn select_heading(&self, paragraph: usize) {
        let index = self
            .headings
            .borrow()
            .partition_point(|h| h.paragraph <= paragraph);
        match index.checked_sub(1) {
            Some(i) => {
                if self.outline_selection.selected() != i as u32 {
                    self.outline_selection.set_selected(i as u32);
                    self.outline_list
                        .scroll_to(i as u32, gtk::ListScrollFlags::NONE, None);
                }
            }
            None => {
                self.outline_selection.unselect_all();
            }
        }
    }

    fn clear_matches(&self) {
        let (start, end) = self.buffer.bounds();
        self.buffer.remove_tag(&self.match_tag, &start, &end);
        self.buffer.remove_tag(&self.current_tag, &start, &end);
        self.matches.borrow_mut().clear();
        self.current.set(None);
        self.match_label.set_label("");
    }

    /// Finds every match of the find text and goes to the first one after
    /// the top of the view.
    fn search(&self) {
        self.clear_matches();
        if !self.loaded.get() {
            return;
        }
        let needle: Vec<char> = self.search_entry.text().chars().map(fold_char).collect();
        if needle.is_empty() {
            return;
        }
        let matches: Vec<i32> = {
            let hay = self.folded.borrow();
            let mut found = Vec::new();
            let mut i = 0;
            while i + needle.len() <= hay.len() {
                if hay[i] == needle[0] && hay[i..i + needle.len()] == needle[..] {
                    found.push(i as i32);
                    i += needle.len();
                } else {
                    i += 1;
                }
            }
            found
        };
        let len = needle.len() as i32;
        self.needle_len.set(len);
        // Highlighting tens of thousands of matches would stall the view.
        for &offset in matches.iter().take(5000) {
            let start = self.buffer.iter_at_offset(offset);
            let end = self.buffer.iter_at_offset(offset + len);
            self.buffer.apply_tag(&self.match_tag, &start, &end);
        }
        let top = self
            .paragraph_at_top()
            .and_then(|p| self.starts.borrow().get(p).copied())
            .unwrap_or(0);
        let first = matches.partition_point(|&m| m < top);
        let count = matches.len();
        *self.matches.borrow_mut() = matches;
        if count == 0 {
            self.match_label.set_label(tr("No Results"));
            return;
        }
        if self.skip_first_match.take() {
            return self.show_match(1.min(count - 1));
        }
        self.show_match(if first < count { first } else { 0 });
    }

    fn step_match(&self, direction: i32) {
        let count = self.matches.borrow().len();
        if count == 0 {
            if !self.search_entry.text().is_empty() && self.matches.borrow().is_empty() {
                self.search();
            }
            return;
        }
        let next = match self.current.get() {
            Some(i) if direction > 0 => (i + 1) % count,
            Some(i) => (i + count - 1) % count,
            None => 0,
        };
        self.show_match(next);
    }

    fn show_match(&self, index: usize) {
        let Some(&offset) = self.matches.borrow().get(index) else {
            return;
        };
        let (start, end) = self.buffer.bounds();
        self.buffer.remove_tag(&self.current_tag, &start, &end);
        let start = self.buffer.iter_at_offset(offset);
        let end = self.buffer.iter_at_offset(offset + self.needle_len.get());
        self.buffer.apply_tag(&self.current_tag, &start, &end);
        self.buffer.select_range(&start, &end);
        self.buffer.move_mark(&self.jump, &start);
        self.view.scroll_to_mark(&self.jump, 0.0, true, 0.0, 0.3);
        self.current.set(Some(index));
        self.match_label.set_label(&tr!(
            "{current} of {total}",
            current = index + 1,
            total = self.matches.borrow().len()
        ));
    }
}

fn heading_factory(headings: Rc<RefCell<Vec<Heading>>>) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let label = gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(pango::EllipsizeMode::End)
            .margin_top(3)
            .margin_bottom(3)
            .build();
        item.set_child(Some(&label));
    });
    factory.connect_bind(move |_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let label = item.child().and_downcast::<gtk::Label>().unwrap();
        let headings = headings.borrow();
        let Some(heading) = headings.get(item.position() as usize) else {
            return;
        };
        label.set_label(&heading.title);
        label.set_tooltip_text(Some(&heading.title));
        label.set_margin_start(12 * heading.level as i32);
        if heading.level == 0 {
            label.add_css_class("heading");
        } else {
            label.remove_css_class("heading");
        }
    });
    factory
}

/// The families to show a book font in: the font itself (fontconfig
/// substitutes a metric-compatible font such as Liberation Serif for
/// Times New Roman, which keeps the tab-aligned tables in their columns),
/// then a generic family. Symbol fonts were mapped to Unicode by the
/// reader, so they show in the generic serif.
fn family(font: &cr_formats::rtf::Font) -> String {
    let name = font.name.to_ascii_lowercase();
    let generic = if ["courier", "mono", "consol"]
        .iter()
        .any(|m| name.contains(m))
    {
        "Monospace"
    } else if [
        "arial",
        "helvetica",
        "sans",
        "verdana",
        "tahoma",
        "univers",
        "futura",
    ]
    .iter()
    .any(|m| name.contains(m))
    {
        "Sans"
    } else {
        "Serif"
    };
    if font.charset == 2 || font.name.is_empty() || font.name.contains(',') {
        generic.to_owned()
    } else {
        format!("{}, {generic}", font.name)
    }
}

/// A book colour as shown: black is the theme's text colour, and dark
/// colours are lightened on a dark background.
fn shown_color(Rgb(r, g, b): Rgb, dark: bool) -> Option<gdk::RGBA> {
    if (r, g, b) == (0, 0, 0) {
        return None;
    }
    let channel = |c: u8| c as f32 / 255.0;
    let (mut r, mut g, mut b) = (channel(r), channel(g), channel(b));
    let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    if dark && luminance < 0.5 {
        let lift = |c: f32| c + (1.0 - c) * 0.55;
        (r, g, b) = (lift(r), lift(g), lift(b));
    }
    Some(gdk::RGBA::new(r, g, b, 1.0))
}

/// The reader's state file: one `key<TAB>value` line per book position,
/// and the zoom, under the user's data folder (never in the
/// installation).
mod state {
    use super::*;

    fn path() -> PathBuf {
        glib::user_data_dir().join("corerules").join("reader-state")
    }

    fn load(path: &Path) -> Vec<(String, String)> {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                let (key, value) = line.rsplit_once('\t')?;
                Some((key.to_owned(), value.to_owned()))
            })
            .collect()
    }

    pub fn get(key: &str) -> Option<String> {
        load(&path())
            .into_iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Stores a value; failures are ignored (the state is a convenience).
    pub fn set(key: &str, value: &str) {
        if key.contains(['\n', '\t']) {
            return;
        }
        let path = path();
        let mut entries = load(&path);
        match entries.iter_mut().find(|(k, _)| k == key) {
            Some(entry) if entry.1 == value => return,
            Some(entry) => entry.1 = value.to_owned(),
            None => entries.push((key.to_owned(), value.to_owned())),
        }
        let text: String = entries.iter().map(|(k, v)| format!("{k}\t{v}\n")).collect();
        let Some(dir) = path.parent() else { return };
        let temp = path.with_extension("tmp");
        let _ = std::fs::create_dir_all(dir)
            .and_then(|()| std::fs::write(&temp, text))
            .and_then(|()| std::fs::rename(&temp, &path));
    }
}
