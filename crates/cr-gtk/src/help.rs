//! The help window: the installation's WinHelp files (`Help/*.hlp`) as
//! WinHelp shows them, with the contents file's topics in the sidebar, a
//! search over the topics' text, links between topics and files, popups
//! and the topics' pictures.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

use adw::prelude::*;
use cr_formats::rtf::{Align, Format, Rgb, TabKind};
use cr_formats::winhelp::{HelpFile, Item, Link, Macro, Paragraph, Topic};
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::help::Contents;
use gtk::{gdk, gio, glib, pango};

/// Which topic to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// The file's contents topic.
    Contents,
    /// A context string, as the contents files and links name topics.
    Context(String),
    /// A `[MAP]` number, as the original's Help buttons pass them.
    Number(u32),
}

/// A topic of a help file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Place {
    /// The file's name in `Help/`, lower case.
    file: String,
    topic: usize,
}

/// A row of the sidebar.
#[derive(Debug, Clone)]
struct Row {
    level: u32,
    title: String,
    place: Option<Place>,
    /// A contents entry's file and context string, found when chosen:
    /// the Tome of Help lists every book's topics.
    context: Option<(String, String)>,
}

impl Row {
    fn is_topic(&self) -> bool {
        self.place.is_some() || self.context.is_some()
    }
}

struct Viewer {
    window: adw::ApplicationWindow,
    install: Installation,
    files: RefCell<HashMap<String, Option<Rc<HelpFile>>>>,
    view: gtk::TextView,
    title: adw::WindowTitle,
    toasts: adw::ToastOverlay,
    back: gtk::Button,
    split: adw::OverlaySplitView,
    list: gtk::ListBox,
    search: gtk::SearchEntry,
    rows: RefCell<Vec<Row>>,
    /// The file the sidebar lists.
    listed: RefCell<String>,
    history: RefCell<Vec<Place>>,
    current: RefCell<Option<Place>>,
    links: RefCell<Vec<(i32, i32, Link)>>,
    tags: RefCell<HashMap<(Format, bool), gtk::TextTag>>,
    hovering: Cell<bool>,
}

/// The space beside the text, in pixels.
const MARGIN: i32 = 24;

thread_local! {
    /// The help window of each installation.
    static OPEN: RefCell<HashMap<PathBuf, Weak<Viewer>>> = RefCell::default();
}

/// Adds the original's Help… button to `header`, and F1 to `area` (a
/// widget that holds the focus while it shows): both open the program help
/// at the topic `topic` gives when pressed (see [`cr_store::help::topic`]).
pub fn add_button(
    header: &adw::HeaderBar,
    area: &impl IsA<gtk::Widget>,
    topic: impl Fn() -> u32 + 'static,
) {
    let topic = Rc::new(topic);
    let button = gtk::Button::builder()
        .icon_name("corerules-help-symbolic")
        .tooltip_text(tr("Help"))
        .build();
    button.connect_clicked({
        let topic = topic.clone();
        move |button| show(button, topic())
    });
    header.pack_end(&button);
    add_key(area, move || topic());
}

/// F1 on `area` opens the program help at the topic `topic` gives.
pub fn add_key(area: &impl IsA<gtk::Widget>, topic: impl Fn() -> u32 + 'static) {
    let keys = gtk::ShortcutController::new();
    keys.add_shortcut(gtk::Shortcut::new(
        gtk::ShortcutTrigger::parse_string("F1"),
        Some(gtk::CallbackAction::new(move |widget, _| {
            show(widget, topic());
            glib::Propagation::Stop
        })),
    ));
    area.add_controller(keys);
}

/// Opens the program help (`Program.hlp`) at `topic`, through the main
/// window's `win.help`.
pub fn show(widget: &impl IsA<gtk::Widget>, topic: u32) {
    let _ = widget.activate_action(
        "win.help",
        Some(&format!("Program.hlp#{topic}").to_variant()),
    );
}

/// Opens `file` of `Help/` at `target` in the help window, which opens
/// once per installation.
pub fn open(
    parent: &adw::ApplicationWindow,
    install: &Installation,
    file: &str,
    target: Target,
) -> Result<(), String> {
    let key = install.root().to_owned();
    let existing = OPEN.with_borrow(|open| open.get(&key).and_then(Weak::upgrade));
    let viewer = match existing {
        Some(viewer) => viewer,
        None => {
            let app = parent
                .application()
                .ok_or(tr("The application is shutting down"))?;
            let install = Installation::open(install.root()).map_err(|e| e.to_string())?;
            let viewer = Viewer::new(&app, install);
            OPEN.with_borrow_mut(|open| {
                open.retain(|_, v| v.strong_count() > 0);
                open.insert(key, Rc::downgrade(&viewer));
            });
            viewer
        }
    };
    let place = viewer
        .resolve(file, &target)
        .ok_or_else(|| tr!("The help topic is not in {file}", file = file.to_owned()))?;
    viewer.go(place, true);
    viewer.window.present();
    Ok(())
}

impl Viewer {
    fn new(app: &gtk::Application, install: Installation) -> Rc<Self> {
        let view = gtk::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk::WrapMode::WordChar)
            .left_margin(MARGIN)
            .right_margin(MARGIN)
            .top_margin(18)
            .bottom_margin(24)
            .build();
        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&view)
            .vexpand(true)
            .build();
        let title = adw::WindowTitle::new(tr("Help"), "");
        let back = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text(tr("Back"))
            .sensitive(false)
            .build();
        let sidebar_button = gtk::ToggleButton::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text(tr("Contents"))
            .active(true)
            .build();
        let header = adw::HeaderBar::builder().title_widget(&title).build();
        header.pack_start(&sidebar_button);
        header.pack_start(&back);
        let content = adw::ToolbarView::new();
        content.add_top_bar(&header);
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&scroller));
        content.set_content(Some(&toasts));

        let search = gtk::SearchEntry::builder()
            .placeholder_text(tr("Search the topics"))
            .margin_start(6)
            .margin_end(6)
            .margin_top(6)
            .margin_bottom(6)
            .build();
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(["navigation-sidebar"])
            .build();
        let list_scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&list)
            .vexpand(true)
            .build();
        let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        sidebar_box.append(&search);
        sidebar_box.append(&list_scroller);
        let sidebar = adw::ToolbarView::new();
        sidebar.add_top_bar(&adw::HeaderBar::builder().show_title(false).build());
        sidebar.set_content(Some(&sidebar_box));

        let split = adw::OverlaySplitView::builder()
            .sidebar(&sidebar)
            .content(&content)
            .min_sidebar_width(220.0)
            .build();
        sidebar_button
            .bind_property("active", &split, "show-sidebar")
            .bidirectional()
            .build();
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(tr("Help"))
            .default_width(900)
            .default_height(700)
            .width_request(360)
            .height_request(294)
            .content(&split)
            .build();
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            640.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);

        let viewer = Rc::new(Self {
            window,
            install,
            files: RefCell::default(),
            view,
            title,
            toasts,
            back,
            split,
            list,
            search,
            rows: RefCell::default(),
            listed: RefCell::default(),
            history: RefCell::default(),
            current: RefCell::default(),
            links: RefCell::default(),
            tags: RefCell::default(),
            hovering: Cell::new(false),
        });
        viewer.connect();
        // The window keeps the viewer until it closes.
        let keep = RefCell::new(Some(viewer.clone()));
        viewer.window.connect_close_request(move |_| {
            keep.take();
            glib::Propagation::Proceed
        });
        viewer
    }

    fn connect(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.back.connect_clicked({
            let weak = weak.clone();
            move |_| {
                if let Some(v) = weak.upgrade() {
                    v.go_back();
                }
            }
        });
        let close = gtk::ShortcutController::new();
        close.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("<Control>w|Escape"),
            Some(gtk::CallbackAction::new(|widget, _| {
                if let Some(w) = widget.downcast_ref::<gtk::Window>() {
                    w.close();
                }
                glib::Propagation::Stop
            })),
        ));
        close.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("<Alt>Left"),
            Some(gtk::CallbackAction::new({
                let weak = weak.clone();
                move |_, _| {
                    if let Some(v) = weak.upgrade() {
                        v.go_back();
                    }
                    glib::Propagation::Stop
                }
            })),
        ));
        self.window.add_controller(close);
        self.list.connect_row_activated({
            let weak = weak.clone();
            move |_, row| {
                let Some(v) = weak.upgrade() else { return };
                let chosen = v.rows.borrow().get(row.index() as usize).cloned();
                match chosen.and_then(|r| v.row_place(&r)) {
                    Some(place) => {
                        v.go(place, true);
                        if v.split.is_collapsed() {
                            v.split.set_show_sidebar(false);
                        }
                    }
                    None => v.missing(),
                }
            }
        });
        self.search.connect_search_changed({
            let weak = weak.clone();
            move |_| {
                if let Some(v) = weak.upgrade() {
                    v.fill_sidebar();
                }
            }
        });
        let click = gtk::GestureClick::new();
        click.connect_released({
            let weak = weak.clone();
            move |gesture, _, x, y| {
                let Some(v) = weak.upgrade() else { return };
                if let Some(link) = v.link_at(x, y) {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    v.follow(&link);
                }
            }
        });
        self.view.add_controller(click);
        let motion = gtk::EventControllerMotion::new();
        motion.connect_motion({
            let weak = weak.clone();
            move |_, x, y| {
                let Some(v) = weak.upgrade() else { return };
                let over = v.link_at(x, y).is_some();
                if over != v.hovering.get() {
                    v.hovering.set(over);
                    v.view
                        .set_cursor_from_name(Some(if over { "pointer" } else { "text" }));
                }
            }
        });
        self.view.add_controller(motion);
    }

    /// The help file `name`, read once.
    fn file(&self, name: &str) -> Option<Rc<HelpFile>> {
        let key = name.to_ascii_lowercase();
        if let Some(file) = self.files.borrow().get(&key) {
            return file.clone();
        }
        let file = self.install.help_file(name).ok().map(Rc::new);
        self.files.borrow_mut().insert(key, file.clone());
        file
    }

    fn resolve(&self, file: &str, target: &Target) -> Option<Place> {
        let help = self.file(file)?;
        let topic = match target {
            Target::Contents => Some(help.contents),
            Target::Context(name) => help.topic_of_context(name),
            Target::Number(n) => help.topic_of_number(*n),
        }?;
        Some(Place {
            file: file.to_ascii_lowercase(),
            topic,
        })
    }

    fn go(&self, place: Place, remember: bool) {
        let Some(help) = self.file(&place.file) else {
            return;
        };
        let Some(topic) = help.topics.get(place.topic) else {
            return;
        };
        if remember
            && let Some(current) = self.current.borrow().clone()
            && current != place
        {
            self.history.borrow_mut().push(current);
        }
        self.back.set_sensitive(!self.history.borrow().is_empty());
        // A file without a contents topic opens on an empty one.
        let title = Some(topic.display_title())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| help.title.clone());
        self.title.set_title(&title);
        self.title
            .set_subtitle(if help.title == title { "" } else { &help.title });
        self.window.set_title(Some(&help.title));
        let buffer = gtk::TextBuffer::new(None);
        self.tags.borrow_mut().clear();
        let (links, pictures) = render(&buffer, &self.view, &help, topic, &self.tags);
        if topic.paragraphs.is_empty() {
            buffer.insert(&mut buffer.end_iter(), tr("Choose a topic from the list."));
            self.split.set_show_sidebar(true);
        }
        self.view.set_buffer(Some(&buffer));
        add_pictures(&self.view, &help, pictures);
        *self.links.borrow_mut() = links;
        *self.current.borrow_mut() = Some(place.clone());
        // A contents file that includes the file shown, as the Tome of
        // Help's does every book's, stays listed.
        let listed = self
            .rows
            .borrow()
            .iter()
            .any(|r| r.context.as_ref().is_some_and(|(f, _)| *f == place.file));
        if *self.listed.borrow() != place.file && !listed {
            *self.listed.borrow_mut() = place.file.clone();
            self.fill_sidebar();
        }
        self.select_row(&place);
        self.view.grab_focus();
    }

    fn go_back(&self) {
        let previous = self.history.borrow_mut().pop();
        if let Some(place) = previous {
            self.go(place, false);
        }
    }

    /// Lists the contents file of the file shown, or its titled topics,
    /// or the topics whose text has what the search entry holds.
    fn fill_sidebar(&self) {
        let file = self.listed.borrow().clone();
        let Some(help) = self.file(&file) else { return };
        let query = self.search.text().to_lowercase();
        let mut rows = Vec::new();
        if !query.is_empty() {
            for (i, topic) in help.topics.iter().enumerate() {
                let title = topic.display_title();
                let found = title.to_lowercase().contains(&query)
                    || topic
                        .paragraphs
                        .iter()
                        .any(|p| p.text().to_lowercase().contains(&query));
                if found && !title.is_empty() {
                    rows.push(Row {
                        level: 1,
                        title,
                        context: None,
                        place: Some(Place {
                            file: file.clone(),
                            topic: i,
                        }),
                    });
                }
            }
        } else if let Some(contents) = self.install.help_contents(&file) {
            rows = self.contents_rows(&file, &contents);
        } else {
            // Like WinHelp's Index tab, alphabetically.
            for (i, topic) in help.topics.iter().enumerate() {
                let title = topic.display_title();
                if !title.is_empty() {
                    rows.push(Row {
                        level: 1,
                        title,
                        context: None,
                        place: Some(Place {
                            file: file.clone(),
                            topic: i,
                        }),
                    });
                }
            }
            rows.sort_by_cached_key(|r| r.title.to_lowercase());
        }
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        for row in &rows {
            let label = gtk::Label::builder()
                .label(&row.title)
                .xalign(0.0)
                .wrap(true)
                .wrap_mode(pango::WrapMode::WordChar)
                .margin_start(12 * row.level.saturating_sub(1) as i32)
                .build();
            if !row.is_topic() {
                label.add_css_class("heading");
            }
            let list_row = gtk::ListBoxRow::builder()
                .child(&label)
                .activatable(row.is_topic())
                .selectable(row.is_topic())
                .build();
            self.list.append(&list_row);
        }
        *self.rows.borrow_mut() = rows;
        if let Some(place) = self.current.borrow().clone() {
            self.select_row(&place);
        }
    }

    fn contents_rows(&self, file: &str, contents: &Contents) -> Vec<Row> {
        contents
            .entries
            .iter()
            .map(|entry| Row {
                level: entry.level,
                title: entry.title.clone(),
                place: None,
                context: entry.context.as_ref().map(|context| {
                    let file = entry.file.as_deref().unwrap_or(file);
                    (file.to_ascii_lowercase(), context.clone())
                }),
            })
            .collect()
    }

    fn missing(&self) {
        self.toasts.add_toast(adw::Toast::new(tr(
            "That topic is not in this installation",
        )));
    }

    /// Where a row leads; a contents entry's file is read then.
    fn row_place(&self, row: &Row) -> Option<Place> {
        match &row.context {
            Some((file, context)) => self.resolve(file, &Target::Context(context.clone())),
            None => row.place.clone(),
        }
    }

    fn select_row(&self, place: &Place) {
        let at = self.rows.borrow().iter().position(|r| match &r.context {
            // Only rows of the file shown can be it.
            Some((file, _)) => *file == place.file && self.row_place(r).as_ref() == Some(place),
            None => r.place.as_ref() == Some(place),
        });
        match at.and_then(|i| self.list.row_at_index(i as i32)) {
            Some(row) => self.list.select_row(Some(&row)),
            None => self.list.unselect_all(),
        }
    }

    fn link_at(&self, x: f64, y: f64) -> Option<Link> {
        let (bx, by) =
            self.view
                .window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
        let iter = self.view.iter_at_location(bx, by)?;
        let offset = iter.offset();
        self.links
            .borrow()
            .iter()
            .find(|(start, end, _)| (*start..*end).contains(&offset))
            .map(|(_, _, link)| link.clone())
    }

    fn follow(&self, link: &Link) {
        let Some(current) = self.current.borrow().clone() else {
            return;
        };
        let (place, popup) = match link {
            Link::Topic { hash, file, popup } => {
                let name = file.clone().unwrap_or_else(|| current.file.clone());
                let place = self.file(&name).and_then(|h| {
                    Some(Place {
                        file: name.to_ascii_lowercase(),
                        topic: h.topic_of_hash(*hash)?,
                    })
                });
                (place, *popup)
            }
            Link::Offset { offset, popup } => {
                let place = self.file(&current.file).and_then(|h| {
                    Some(Place {
                        file: current.file.clone(),
                        topic: h.topic_at(*offset)?,
                    })
                });
                (place, *popup)
            }
            Link::Macro(text) => match Macro::parse(text) {
                Some(Macro::Web(uri)) => {
                    gtk::UriLauncher::new(&uri).launch(
                        Some(&self.window),
                        gio::Cancellable::NONE,
                        |_| {},
                    );
                    return;
                }
                Some(Macro::Jump {
                    file,
                    context,
                    popup,
                }) => (
                    self.resolve(
                        file.as_deref().unwrap_or(&current.file),
                        &Target::Context(context),
                    ),
                    popup,
                ),
                Some(Macro::Contents { file }) => (
                    self.resolve(file.as_deref().unwrap_or(&current.file), &Target::Contents),
                    false,
                ),
                None => (None, false),
            },
        };
        match place {
            Some(place) if popup => self.popup(place),
            Some(place) => self.go(place, true),
            None => self.missing(),
        }
    }

    /// A popup topic, shown in a dialog over the window.
    fn popup(&self, place: Place) {
        let Some(help) = self.file(&place.file) else {
            return;
        };
        let Some(topic) = help.topics.get(place.topic) else {
            return;
        };
        let view = gtk::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk::WrapMode::WordChar)
            .left_margin(18)
            .right_margin(18)
            .top_margin(12)
            .bottom_margin(18)
            .build();
        let buffer = gtk::TextBuffer::new(None);
        let tags = RefCell::default();
        let (_, pictures) = render(&buffer, &view, &help, topic, &tags);
        view.set_buffer(Some(&buffer));
        add_pictures(&view, &help, pictures);
        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(500)
            .child(&view)
            .build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.set_content(Some(&scroller));
        let dialog = adw::Dialog::builder()
            .title(topic.display_title())
            .content_width(480)
            .child(&toolbar)
            .build();
        dialog.present(Some(&self.window));
    }
}

/// The text tag of a run.
fn run_tag(
    buffer: &gtk::TextBuffer,
    help: &HelpFile,
    format: &Format,
    link: bool,
    tags: &RefCell<HashMap<(Format, bool), gtk::TextTag>>,
) -> gtk::TextTag {
    if let Some(tag) = tags.borrow().get(&(*format, link)) {
        return tag.clone();
    }
    let tag = gtk::TextTag::new(None);
    buffer.tag_table().add(&tag);
    let face = help
        .faces
        .get(usize::from(format.font))
        .map_or("", String::as_str);
    tag.set_family(Some(&family(face)));
    tag.set_size_points(f64::from(format.size) / 2.0);
    tag.set_weight(if format.bold { 700 } else { 400 });
    if format.italic {
        tag.set_style(pango::Style::Italic);
    }
    if format.small_caps {
        tag.set_variant(pango::Variant::SmallCaps);
    }
    if link {
        // WinHelp's hotspots: green and underlined.
        tag.set_underline(pango::Underline::Single);
        tag.set_foreground(Some(if adw::StyleManager::default().is_dark() {
            "#8ff0a4"
        } else {
            "#1c7c3c"
        }));
    } else {
        if format.underline {
            tag.set_underline(pango::Underline::Single);
        }
        if let Some(rgba) = format
            .color
            .and_then(|c| shown_color(c, adw::StyleManager::default().is_dark()))
        {
            tag.set_foreground_rgba(Some(&rgba));
        }
    }
    tags.borrow_mut().insert((*format, link), tag.clone());
    tag
}

fn family(face: &str) -> String {
    let name = face.to_ascii_lowercase();
    if name.contains("courier") || name.contains("mono") {
        "Monospace".into()
    } else if ["arial", "helvetica", "sans", "verdana", "tahoma", "ms sans"]
        .iter()
        .any(|m| name.contains(m))
    {
        format!("{face}, Sans")
    } else if face.is_empty() {
        "Sans".into()
    } else {
        format!("{face}, Serif")
    }
}

/// A colour as shown: dark colours are lightened on a dark background.
fn shown_color(Rgb(r, g, b): Rgb, dark: bool) -> Option<gdk::RGBA> {
    let channel = |c: u8| f32::from(c) / 255.0;
    let (mut r, mut g, mut b) = (channel(r), channel(g), channel(b));
    let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    if dark && luminance < 0.5 {
        let lift = |c: f32| c + (1.0 - c) * 0.55;
        (r, g, b) = (lift(r), lift(g), lift(b));
    }
    Some(gdk::RGBA::new(r, g, b, 1.0))
}

fn paragraph_tag(
    buffer: &gtk::TextBuffer,
    p: &Paragraph,
    tabs: Option<pango::TabArray>,
) -> gtk::TextTag {
    let tag = gtk::TextTag::new(None);
    buffer.tag_table().add(&tag);
    let px = |twips: i32| twips / 15;
    // A tag's margins replace the view's.
    tag.set_left_margin(MARGIN + px(p.left_indent.max(0)));
    tag.set_indent(px(p.first_indent));
    tag.set_right_margin(MARGIN + px(p.right_indent.max(0)));
    tag.set_pixels_above_lines(px(p.space_before.max(0)));
    tag.set_pixels_below_lines(px(p.space_after.max(0)));
    tag.set_justification(match p.align {
        Align::Center => gtk::Justification::Center,
        Align::Right => gtk::Justification::Right,
        _ => gtk::Justification::Left,
    });
    let tabs = tabs.or_else(|| {
        (!p.tabs.is_empty()).then(|| {
            let mut array = pango::TabArray::new(p.tabs.len() as i32, true);
            for (i, stop) in p.tabs.iter().enumerate() {
                let align = match stop.kind {
                    TabKind::Right => pango::TabAlign::Right,
                    TabKind::Center => pango::TabAlign::Center,
                    _ => pango::TabAlign::Left,
                };
                array.set_tab(i as i32, align, px(stop.pos));
            }
            array
        })
    });
    if let Some(tabs) = tabs {
        tag.set_tabs(Some(&tabs));
    }
    tag
}

type Links = Vec<(i32, i32, Link)>;
type Pictures = Vec<(gtk::TextChildAnchor, u16)>;

/// Fills `buffer` with `topic`, and returns the links by character range
/// and where the pictures go.
fn render(
    buffer: &gtk::TextBuffer,
    view: &gtk::TextView,
    help: &HelpFile,
    topic: &Topic,
    tags: &RefCell<HashMap<(Format, bool), gtk::TextTag>>,
) -> (Links, Pictures) {
    let mut out = Out {
        buffer,
        help,
        tags,
        links: Vec::new(),
        pictures: Vec::new(),
    };
    let mut end = buffer.end_iter();
    let paragraphs = &topic.paragraphs;
    let mut i = 0;
    while i < paragraphs.len() {
        let p = &paragraphs[i];
        let Some(cell) = p.cell else {
            let tag = paragraph_tag(buffer, p, None);
            out.items(&mut end, &p.items, &tag);
            buffer.insert_with_tags(&mut end, "\n", &[&tag]);
            i += 1;
            continue;
        };
        // A table: its rows, each a line of tab-separated cells.
        let first = i;
        let mut last_row = cell.row;
        while i < paragraphs.len()
            && paragraphs[i]
                .cell
                .is_some_and(|c| c.row == last_row || c.row == last_row + 1)
        {
            last_row = paragraphs[i].cell.unwrap().row;
            i += 1;
        }
        let table = &paragraphs[first..i];
        let columns = table
            .iter()
            .filter_map(|p| p.cell)
            .map(|c| usize::from(c.column) + 1)
            .max()
            .unwrap_or(1);
        let mut widths = vec![0i32; columns];
        for p in table {
            let c = p.cell.unwrap();
            let layout = view.create_pango_layout(Some(p.text().trim()));
            let w = layout.pixel_size().0 + 18;
            let slot = &mut widths[usize::from(c.column)];
            *slot = (*slot).max(w.min(320));
        }
        let mut tabs = pango::TabArray::new(columns as i32, true);
        let mut x = 0;
        for (n, w) in widths.iter().enumerate() {
            tabs.set_tab(n as i32, pango::TabAlign::Left, x);
            x += w;
        }
        let mut row = table[0].cell.unwrap().row;
        let mut row_tag = paragraph_tag(buffer, &table[0], Some(tabs.clone()));
        let mut column = 0u16;
        for p in table {
            let c = p.cell.unwrap();
            if c.row != row {
                buffer.insert_with_tags(&mut end, "\n", &[&row_tag]);
                row = c.row;
                row_tag = paragraph_tag(buffer, p, Some(tabs.clone()));
                column = 0;
            }
            while column < c.column {
                buffer.insert_with_tags(&mut end, "\t", &[&row_tag]);
                column += 1;
            }
            let items: Vec<Item> = p
                .items
                .iter()
                .map(|item| match item {
                    Item::Text(run) => {
                        let mut run = run.clone();
                        run.text = run.text.replace(['\n', '\t'], " ");
                        Item::Text(run)
                    }
                    other => other.clone(),
                })
                .collect();
            out.items(&mut end, &items, &row_tag);
            buffer.insert_with_tags(&mut end, " ", &[&row_tag]);
        }
        buffer.insert_with_tags(&mut end, "\n", &[&row_tag]);
    }
    (out.links, out.pictures)
}

/// What a topic's runs and pictures go into.
struct Out<'a> {
    buffer: &'a gtk::TextBuffer,
    help: &'a HelpFile,
    tags: &'a RefCell<HashMap<(Format, bool), gtk::TextTag>>,
    links: Links,
    pictures: Pictures,
}

impl Out<'_> {
    fn items(&mut self, end: &mut gtk::TextIter, items: &[Item], paragraph: &gtk::TextTag) {
        for item in items {
            match item {
                Item::Text(run) => {
                    let tag = run_tag(
                        self.buffer,
                        self.help,
                        &run.format,
                        run.link.is_some(),
                        self.tags,
                    );
                    let start = end.offset();
                    self.buffer
                        .insert_with_tags(end, &run.text, &[&tag, paragraph]);
                    if let Some(link) = &run.link {
                        self.links.push((start, end.offset(), link.clone()));
                    }
                }
                Item::Picture(n) => {
                    // The widget goes in once the buffer is shown.
                    let anchor = self.buffer.create_child_anchor(end);
                    let mut start = *end;
                    start.backward_char();
                    self.buffer.apply_tag(paragraph, &start, end);
                    self.pictures.push((anchor, *n));
                }
            }
        }
    }
}

/// Puts the pictures in their anchors, no wider than a screen of the
/// original.
fn add_pictures(view: &gtk::TextView, help: &HelpFile, pictures: Pictures) {
    for (anchor, n) in pictures {
        let Some(bitmap) = help.picture(n) else {
            continue;
        };
        let Ok(texture) = gdk::Texture::from_bytes(&glib::Bytes::from(&bitmap.bmp)) else {
            continue;
        };
        let width = bitmap.width.min(560);
        let height = u64::from(bitmap.height) * u64::from(width) / u64::from(bitmap.width.max(1));
        let picture = gtk::Picture::builder()
            .paintable(&texture)
            .can_shrink(true)
            .content_fit(gtk::ContentFit::Contain)
            .width_request(width as i32)
            .height_request(height as i32)
            .build();
        view.add_child_at_anchor(&picture, &anchor);
    }
}

/// The About window (Help > About…): the app's, with what the original's
/// About box and Credits window show, read from the installation when it
/// has them.
pub fn about(parent: &adw::ApplicationWindow, install: Option<&Installation>) {
    let repository = env!("CARGO_PKG_REPOSITORY");
    let dialog = adw::AboutDialog::builder()
        .application_name("Core Rules")
        .application_icon(crate::APP_ID)
        .developer_name("Brahm")
        .version(env!("CARGO_PKG_VERSION"))
        .website(repository)
        .issue_url(format!("{repository}/issues"))
        .license_type(gtk::License::MitX11)
        // The comments, a section's license and a credit section's name
        // are markup.
        .comments(glib::markup_escape_text(tr(
            "A Linux replica of AD&D Core Rules 2.0. It reads the rules data from a Core \
             Rules installation you own and contains no TSR or Wizards of the Coast content.",
        )))
        .build();
    if let Some(install) = install {
        let original = install.original_about();
        // The product's name, then its version, copyright and developer.
        let split = original.lines.len().min(2);
        let (name, rest) = original.lines.split_at(split);
        let name = name.join(" ");
        if !name.is_empty() || original.license.is_some() {
            dialog.add_legal_section(
                &name,
                Some(&rest.join("\n"))
                    .filter(|r| !r.is_empty())
                    .map(String::as_str),
                gtk::License::Custom,
                original
                    .license
                    .as_deref()
                    .map(glib::markup_escape_text)
                    .as_deref(),
            );
        }
        if !original.credits.is_empty() {
            let credits: Vec<&str> = original.credits.iter().map(String::as_str).collect();
            // The product's name without the game's, which does not fit.
            let product = original.lines.get(1).unwrap_or(&name);
            dialog.add_credit_section(Some(&glib::markup_escape_text(product)), &credits);
        }
    }
    dialog.present(Some(parent));
}
