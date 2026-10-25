//! A book of the Books section on a page of the window, as the GNOME
//! app's reader window shows it: the contents beside the text, find,
//! zoom, and the reading position kept between sessions
//! ([`cr_app::books`]). The text is one web page ([`cr_win::book`]) in a
//! WebView2, which says where the reader is and which keys were pressed
//! in it.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use cr_app::books::Folded;
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::books::{Heading, outline};
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::native::webview::Web;
use crate::{prefs, ui};

/// A book read and made a page, in the background.
pub struct Opened {
    pub title: String,
    pub category: String,
    /// The book's key in the reader's state file.
    pub key: String,
    /// The page, written where the WebView2 opens it from: a book is
    /// more than a page given as a string may be.
    pub path: PathBuf,
    pub headings: Vec<Heading>,
    pub folded: Folded,
}

/// Reads the book `name` of `install` and writes it as a page.
pub fn open(install: &Installation, name: &str) -> Result<Opened, String> {
    let book = install
        .book(name)
        .ok_or_else(|| tr!("{name} is not in this installation", name = name.to_owned()))?;
    let doc = book.read().map_err(|e| e.to_string())?;
    let path = std::env::temp_dir()
        .join("Core Rules")
        .join(format!("{}.html", book.info.short_name()));
    path.parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&path, cr_win::book::page(&doc)))
        .map_err(|e| {
            tr!(
                "Could not save {file}: {error}",
                file = path.display(),
                error = e
            )
        })?;
    Ok(Opened {
        title: book.info.title.to_owned(),
        category: tr(book.info.category.title()).to_owned(),
        key: cr_app::books::position_key(install, &book),
        path,
        headings: outline(&doc, book.info.heading_colors),
        folded: Folded::of(&doc),
    })
}

#[derive(Clone)]
pub struct BookPage {
    /// The book as it was asked for.
    pub name: String,
    pub book: Option<Arc<Opened>>,
    pub error: Option<String>,
    /// The page has loaded and takes scripts.
    pub ready: bool,
    pub contents: bool,
    /// The paragraph at the top of the text, and the heading it is under.
    pub top: usize,
    pub heading: Option<usize>,
    pub zoom: f64,
    pub finding: bool,
    /// Counts the times Find was asked for, to put the focus in its box.
    pub find_asked: u64,
    pub query: String,
    /// Where each match starts in the folded text, and the one shown.
    pub matches: Arc<Vec<usize>>,
    pub current: Option<usize>,
    /// The next search shows the second match: a table caption, whose
    /// first is in the book's List of Tables.
    pub skip_first: bool,
}

impl BookPage {
    /// The page while `name` opens; with `find`, Find starts with it.
    pub fn new(name: String, find: Option<String>) -> Self {
        Self {
            name,
            book: None,
            error: None,
            ready: false,
            contents: true,
            top: 0,
            heading: None,
            zoom: prefs::reader_state().zoom(),
            finding: find.is_some(),
            find_asked: u64::from(find.is_some()),
            skip_first: find.is_some(),
            query: find.unwrap_or_default(),
            matches: Arc::default(),
            current: None,
        }
    }

    /// The book has been read: it shows where it was left.
    pub fn opened(&mut self, book: Arc<Opened>) {
        self.top = prefs::reader_state().position(&book.key).unwrap_or(0);
        self.heading = cr_app::books::heading_at(&book.headings, self.top);
        self.book = Some(book);
    }

    /// The paragraph at the top is now `top`.
    pub fn moved(&mut self, top: usize) {
        let Some(book) = &self.book else {
            return;
        };
        self.top = top;
        self.heading = cr_app::books::heading_at(&book.headings, top);
        prefs::reader_state().set_position(&book.key, top);
    }

    /// The zoom step after the current one (`direction` 1), before it
    /// (-1), or 1 (0); the script that shows it, if it changed.
    pub fn zoom(&mut self, direction: i32) -> Option<String> {
        let zoom = match direction {
            0 => 1.0,
            _ => cr_app::books::step_zoom(self.zoom, direction)?,
        };
        if (zoom - self.zoom).abs() < 0.001 {
            return None;
        }
        self.zoom = zoom;
        prefs::reader_state().set_zoom(zoom);
        Some(cr_win::book::zoom(zoom))
    }

    /// Finds `query` and shows the first match after the top; the
    /// scripts that highlight them.
    pub fn search(&mut self, query: String) -> Vec<String> {
        self.query = query;
        self.current = None;
        let Some(book) = self.book.clone() else {
            return Vec::new();
        };
        let matches = book.folded.find(&self.query);
        let len = self.query.chars().count();
        let places: Vec<(usize, usize)> = matches
            .iter()
            .take(cr_app::books::MOST_HIGHLIGHTED)
            .map(|&m| place(&book.folded, m))
            .collect();
        let top = book.folded.start_of(self.top).unwrap_or(0);
        let first = cr_app::books::first_match(&matches, top, std::mem::take(&mut self.skip_first));
        self.matches = Arc::new(matches);
        let mut scripts = vec![if self.query.is_empty() {
            cr_win::book::UNMARK.to_owned()
        } else {
            cr_win::book::mark(&places, len)
        }];
        scripts.extend(first.and_then(|first| self.show_match(first)));
        scripts
    }

    /// The match after (`direction` 1) or before (-1) the one shown.
    pub fn step_match(&mut self, direction: i32) -> Option<String> {
        let next = cr_app::books::step_match(self.current, self.matches.len(), direction)?;
        self.show_match(next)
    }

    fn show_match(&mut self, index: usize) -> Option<String> {
        let book = self.book.as_ref()?;
        let &offset = self.matches.get(index)?;
        let (paragraph, at) = place(&book.folded, offset);
        self.current = Some(index);
        Some(cr_win::book::show(
            paragraph,
            at,
            self.query.chars().count(),
        ))
    }

    /// What Find says of its matches.
    fn found(&self) -> String {
        match (self.query.is_empty(), self.current) {
            (true, _) => String::new(),
            (false, Some(current)) => tr!(
                "{current} of {total}",
                current = current + 1,
                total = self.matches.len()
            ),
            (false, None) if self.matches.is_empty() && self.ready => tr("No Results").to_owned(),
            (false, None) => String::new(),
        }
    }
}

/// The paragraph a match at `offset` is in, and where in it.
fn place(folded: &Folded, offset: usize) -> (usize, usize) {
    let paragraph = folded.paragraph_of(offset);
    let start = folded.start_of(paragraph).unwrap_or(0);
    (paragraph, offset - start)
}

/// The WebView2 of the book page while it shows, as the help page keeps
/// its own, and the box of Find.
#[derive(Clone, Default)]
pub struct BookView {
    pub control: ElementRef<WebView2>,
    pub web: Rc<RefCell<Option<Web>>>,
    pub find: ElementRef<TextBox>,
}

impl BookView {
    pub fn web(&self) -> Option<Web> {
        self.web.borrow().clone()
    }

    /// Runs `script` in the page, once it has loaded.
    pub fn run(&self, page: &BookPage, script: &str) {
        if page.ready
            && let Some(web) = self.web()
        {
            web.run(script);
        }
    }
}

pub fn view(shell: &Shell, page: &BookPage, context: &mut ViewContext<Shell>) -> View {
    let (title, category) = match &page.book {
        Some(book) => (book.title.clone(), book.category.clone()),
        None => (page.name.clone(), String::new()),
    };
    let loaded = page.book.is_some();
    let headings = page
        .book
        .as_ref()
        .map_or(&[][..], |b| b.headings.as_slice());
    let commands = [
        KeyedView::new(
            "find",
            ui::command(
                &plain(tr("_Find…")),
                Symbol::Find,
                loaded,
                context.message(Message::Act(cr_app::action::BOOK_FIND)),
            ),
        ),
        KeyedView::new(
            "contents",
            ui::command(
                tr("Contents"),
                if page.contents {
                    Symbol::ClosePane
                } else {
                    Symbol::OpenPane
                },
                !headings.is_empty(),
                context.message(Message::Act(cr_app::action::BOOK_CONTENTS)),
            ),
        ),
        KeyedView::new(
            "zoom-out",
            ui::command(
                &plain(tr("Zoom _Out")),
                Symbol::ZoomOut,
                loaded,
                context.message(Message::Act(cr_app::action::BOOK_ZOOM_OUT)),
            ),
        ),
        KeyedView::new(
            "zoom-in",
            ui::command(
                &plain(tr("Zoom _In")),
                Symbol::ZoomIn,
                loaded,
                context.message(Message::Act(cr_app::action::BOOK_ZOOM_IN)),
            ),
        ),
        KeyedView::new(
            "zoom-reset",
            ui::command(
                &plain(tr("_Reset Zoom")),
                Symbol::Zoom,
                loaded && (page.zoom - 1.0).abs() > 0.001,
                context.message(Message::Act(cr_app::action::BOOK_ZOOM_RESET)),
            ),
        ),
        KeyedView::new(
            "close",
            ui::command(
                &plain(tr("_Close")),
                Symbol::Cancel,
                true,
                context.message(Message::Act(cr_app::action::BOOK_CLOSE)),
            ),
        ),
    ];
    let header = Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .margin(Thickness::new(24.0, 24.0, 24.0, 8.0))
        .children((
            StackPanel::new()
                .spacing(2.0)
                .vertical_alignment(VerticalAlignment::Center)
                .children((ui::subtitle(title.clone()), ui::caption(category))),
            CommandBar::new()
                .grid_column(1)
                .horizontal_alignment(HorizontalAlignment::Right)
                .collection_slot(CommandBarSlot::PrimaryCommands, commands),
        ));
    let find: View = if page.finding {
        find_bar(shell, page, context)
    } else {
        View::empty()
    };
    let text: View = match (&page.book, &page.error) {
        (_, Some(error)) => StackPanel::new()
            .spacing(6.0)
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                ui::subtitle(tr("Could Not Open Book")),
                ui::body(error.clone()),
            )),
        (None, None) => StackPanel::new()
            .spacing(18.0)
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                ProgressRing::new().is_active(true).width(32.0).height(32.0),
                ui::subtitle(tr("Opening Book…")),
            )),
        (Some(book), None) => {
            web_effects(shell, book, context);
            Border::new()
                .margin(Thickness::new(16.0, 0.0, 24.0, 24.0))
                .corner_radius(8.0)
                .border_brush(ThemeBrush::CardStroke)
                .border_thickness(1.0)
                .content(
                    WebView2::new()
                        .element_ref(&shell.book_view.control)
                        .automation_name(title),
                )
        }
    };
    let main = Grid::new()
        .grid_column(1)
        .rows([GridLength::Auto, GridLength::Auto, GridLength::STAR])
        .children((
            header,
            Border::new().grid_row(1).content(find),
            Border::new().grid_row(2).content(text),
        ));
    if !page.contents || headings.is_empty() {
        return Grid::new()
            .columns([GridLength::Pixel(0.0), GridLength::STAR])
            .children((View::empty(), main));
    }
    let side = Grid::new()
        .rows([GridLength::Auto, GridLength::STAR])
        .row_spacing(8.0)
        .margin(Thickness::new(24.0, 24.0, 0.0, 24.0))
        .children((
            ui::strong(tr("Contents")),
            Border::new()
                .grid_row(1)
                .content(contents(headings, page, context)),
        ));
    Grid::new()
        .columns([GridLength::Pixel(300.0), GridLength::STAR])
        .children((side, main))
}

/// The book's headings, the one the top of the text is under selected.
fn contents(headings: &[Heading], page: &BookPage, context: &mut ViewContext<Shell>) -> View {
    ListView::new()
        .automation_name(tr("Contents"))
        .selected_index(page.heading)
        .on_selection_changed(context.callback(Message::BookHeading))
        .collection_slot(
            ListViewSlot::Items,
            headings.iter().enumerate().map(|(i, heading)| {
                let text = ui::body(heading.title.clone())
                    .text_wrapping(TextWrapping::NoWrap)
                    .margin(Thickness::new(
                        12.0 * f64::from(heading.level),
                        4.0,
                        0.0,
                        4.0,
                    ));
                let text = if heading.level == 0 {
                    text.font_weight(FontWeight::SEMI_BOLD)
                } else {
                    text
                };
                KeyedView::new(
                    i,
                    ListViewItem::new()
                        .tag(i.to_string())
                        .content(text.tooltip(heading.title.clone())),
                )
            }),
        )
}

/// Find: its box, what it found, and the matches before and after.
fn find_bar(shell: &Shell, page: &BookPage, context: &mut ViewContext<Shell>) -> View {
    let find = shell.book_view.find.clone();
    context.use_effect("book-find", page.find_asked, move || {
        let _ = find.request_focus();
        None
    });
    let has_matches = !page.matches.is_empty();
    Grid::new()
        .columns([
            GridLength::STAR,
            GridLength::Auto,
            GridLength::Auto,
            GridLength::Auto,
            GridLength::Auto,
        ])
        .column_spacing(8.0)
        .margin(Thickness::new(16.0, 0.0, 24.0, 12.0))
        // Enter in the box goes to the next match.
        .key_accelerators(KeyAccelerators::new([KeyAccelerator::new(
            AcceleratorKey::Enter,
            AcceleratorModifiers::None,
            context.message(Message::BookMatch(1)),
        )]))
        .children((
            TextBox::new()
                .element_ref(&shell.book_view.find)
                .text(page.query.clone())
                .placeholder_text(tr("Find in Book"))
                .automation_name(tr("Find in Book"))
                .max_width(600.0)
                .on_text_changed(context.callback(Message::BookFind)),
            ui::caption(page.found())
                .grid_column(1)
                .min_width(80.0)
                .vertical_alignment(VerticalAlignment::Center),
            Border::new().grid_column(2).content(glyph_button(
                tr("Previous Match"),
                "\u{e70e}",
                has_matches,
                Message::BookMatch(-1),
                context,
            )),
            Border::new().grid_column(3).content(glyph_button(
                tr("Next Match"),
                "\u{e70d}",
                has_matches,
                Message::BookMatch(1),
                context,
            )),
            Border::new().grid_column(4).content(ui::icon_button(
                &plain(tr("_Close")),
                Symbol::Cancel,
                Message::BookFindClosed,
                context,
            )),
        ))
}

/// A button that is only a glyph of Segoe Fluent Icons, which has some
/// that Symbol lacks (the chevrons).
fn glyph_button(
    name: &str,
    glyph: &str,
    enabled: bool,
    message: Message,
    context: &mut ViewContext<Shell>,
) -> View {
    Button::new()
        .automation_name(name)
        .is_enabled(enabled)
        .on_click(context.message(message))
        .content(FontIcon::new().glyph(glyph))
        .tooltip(name)
}

/// The page opens in the WebView2 once the control has its CoreWebView2,
/// and again when it is shown anew.
fn web_effects(shell: &Shell, book: &Opened, context: &mut ViewContext<Shell>) {
    let book_view = shell.book_view.clone();
    context.use_effect("book-view", (), move || {
        let web = book_view.web.clone();
        Some(Box::new(move || {
            web.borrow_mut().take();
        }))
    });
    let book_view = shell.book_view.clone();
    let sender = context.sender();
    let path = book.path.clone();
    context.use_effect("book-text", path.clone(), move || {
        let slot = book_view.web.clone();
        let _ = book_view.control.request_core_web_view2(move |core| {
            let Some(web) = core.ok().as_ref().and_then(Web::of) else {
                return;
            };
            let mut current = slot.borrow_mut();
            if current.as_ref().is_none_or(|w| !w.same(&web)) {
                web.keep_keys();
                let sender = sender.clone();
                web.on_message(move |message| {
                    sender.send(Message::BookPosted(message));
                });
                *current = Some(web.clone());
            }
            web.open(&path);
        });
        None
    });
}
