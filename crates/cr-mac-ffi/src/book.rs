//! A rule book of the Books section, as the Mac app reads it in a window
//! of its own: the book as one web page (`cr_app::web::book`) for a
//! WKWebView, its contents, Find, zoom, and the reading position kept
//! between sessions (`cr_app::books`), as the Windows app's Book page
//! keeps them. The session says which script the page runs next; the
//! Swift side runs it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use cr_app::books::{Folded, State};
use cr_app::web::book as web;
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::books::{Heading, outline};

use crate::CoreError;

/// A heading of the book's contents.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct BookHeading {
    /// Its place in the contents, which [`BookSession::go`] takes.
    pub index: u32,
    /// 0 for the biggest headings, at most 3.
    pub level: u32,
    pub title: String,
}

/// What the page posted, for the window to act on.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum BookPosted {
    /// The page loaded: these scripts show where the reader was.
    Ready {
        scripts: Vec<String>,
    },
    /// The text scrolled; the contents select this heading.
    Moved {
        heading: Option<u32>,
    },
    /// A key of the reader pressed in the page: `find`, `zoom-in`,
    /// `zoom-out`, `zoom-reset`, `contents`, `close` or `help`.
    Key {
        key: String,
    },
    Nothing,
}

/// Counts the books opened, for each page to have a file of its own.
static OPENED: AtomicU64 = AtomicU64::new(0);

/// How old the page of a book a Core Rules no longer running left behind
/// is before the next book opened deletes it.
const STALE: Duration = Duration::from_secs(24 * 60 * 60);

struct Reader {
    ready: bool,
    top: usize,
    heading: Option<usize>,
    zoom: f64,
    query: String,
    matches: Vec<usize>,
    current: Option<usize>,
    /// The next search shows the second match: a table caption, whose
    /// first is in the book's List of Tables.
    skip_first: bool,
}

/// A book open in its window.
#[derive(uniffi::Object)]
pub struct BookSession {
    title: String,
    category: String,
    /// The book's key in the reader's state file.
    key: String,
    /// The page, in a file: a book is more than a page given as a string
    /// should be.
    path: PathBuf,
    headings: Vec<Heading>,
    folded: Folded,
    state: State,
    reader: Mutex<Reader>,
}

impl BookSession {
    /// Reads the book `name` of `install` and writes its page; `state`
    /// is the reader's state file, and `find` starts Find with a text.
    pub(crate) fn open(
        install: &Installation,
        name: &str,
        state: &str,
        find: Option<String>,
    ) -> Result<Self, CoreError> {
        let book = install.book(name).ok_or_else(|| {
            CoreError::new(tr!(
                "{name} is not in this installation",
                name = name.to_owned()
            ))
        })?;
        let doc = book.read().map_err(|e| CoreError::new(e.to_string()))?;
        let folder = std::env::temp_dir().join("Core Rules");
        sweep(&folder);
        let path = folder.join(format!(
            "{}-{}-{}.html",
            book.info.short_name(),
            std::process::id(),
            OPENED.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&folder)
            .and_then(|()| std::fs::write(&path, web::page(&doc)))
            .map_err(|e| {
                CoreError::new(tr!(
                    "Could not save {file}: {error}",
                    file = path.display(),
                    error = e
                ))
            })?;
        let state = State::at(state);
        let key = cr_app::books::position_key(install, &book);
        let headings = outline(&doc, book.info.heading_colors);
        let top = state.position(&key).unwrap_or(0);
        let reader = Reader {
            ready: false,
            top,
            heading: cr_app::books::heading_at(&headings, top),
            zoom: state.zoom(),
            skip_first: find.is_some(),
            query: find.unwrap_or_default(),
            matches: Vec::new(),
            current: None,
        };
        Ok(Self {
            title: book.info.title.to_owned(),
            category: tr(book.info.category.title()).to_owned(),
            key,
            path,
            headings,
            folded: Folded::of(&doc),
            state,
            reader: Mutex::new(reader),
        })
    }

    fn reader(&self) -> MutexGuard<'_, Reader> {
        self.reader.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The scripts that highlight what `reader.query` finds and show the
    /// first match after the top.
    fn find(&self, reader: &mut Reader) -> Vec<String> {
        reader.current = None;
        let matches = self.folded.find(&reader.query);
        let len = reader.query.chars().count();
        let places: Vec<(usize, usize)> = matches
            .iter()
            .take(cr_app::books::MOST_HIGHLIGHTED)
            .map(|&m| self.place(m))
            .collect();
        let top = self.folded.start_of(reader.top).unwrap_or(0);
        let first =
            cr_app::books::first_match(&matches, top, std::mem::take(&mut reader.skip_first));
        reader.matches = matches;
        let mut scripts = vec![if reader.query.is_empty() {
            web::UNMARK.to_owned()
        } else {
            web::mark(&places, len)
        }];
        scripts.extend(first.and_then(|first| self.show_match(reader, first)));
        scripts
    }

    fn show_match(&self, reader: &mut Reader, index: usize) -> Option<String> {
        let &offset = reader.matches.get(index)?;
        let (paragraph, at) = self.place(offset);
        reader.current = Some(index);
        Some(web::show(paragraph, at, reader.query.chars().count()))
    }

    /// The paragraph a match at `offset` is in, and where in it.
    fn place(&self, offset: usize) -> (usize, usize) {
        let paragraph = self.folded.paragraph_of(offset);
        let start = self.folded.start_of(paragraph).unwrap_or(0);
        (paragraph, offset - start)
    }
}

#[uniffi::export]
impl BookSession {
    pub fn title(&self) -> String {
        self.title.clone()
    }

    /// Its category in the Books section.
    pub fn category(&self) -> String {
        self.category.clone()
    }

    /// The page's file, for the web view to open.
    pub fn path(&self) -> String {
        self.path.display().to_string()
    }

    pub fn headings(&self) -> Vec<BookHeading> {
        self.headings
            .iter()
            .enumerate()
            .map(|(i, h)| BookHeading {
                index: i as u32,
                level: u32::from(h.level),
                title: h.title.clone(),
            })
            .collect()
    }

    /// The heading the top of the text is under.
    pub fn heading(&self) -> Option<u32> {
        self.reader().heading.map(|h| h as u32)
    }

    /// What the page posted.
    pub fn posted(&self, message: String) -> BookPosted {
        let mut reader = self.reader();
        if message == web::READY {
            // The page shows where the reader was, as zoomed, with what
            // Find found.
            reader.ready = true;
            let mut scripts = vec![web::start(reader.zoom, reader.top)];
            if !reader.query.is_empty() {
                scripts.extend(self.find(&mut reader));
            }
            BookPosted::Ready { scripts }
        } else if let Some(top) = message.strip_prefix(web::TOP) {
            let Ok(top) = top.parse() else {
                return BookPosted::Nothing;
            };
            reader.top = top;
            reader.heading = cr_app::books::heading_at(&self.headings, top);
            self.state.set_position(&self.key, top);
            BookPosted::Moved {
                heading: reader.heading.map(|h| h as u32),
            }
        } else if let Some(key) = message.strip_prefix(web::KEY) {
            BookPosted::Key {
                key: key.to_owned(),
            }
        } else {
            BookPosted::Nothing
        }
    }

    /// The script that goes to heading `index` of the contents.
    pub fn go(&self, index: u32) -> Option<String> {
        let heading = self.headings.get(index as usize)?;
        let mut reader = self.reader();
        if reader.heading == Some(index as usize) {
            return None;
        }
        reader.heading = Some(index as usize);
        Some(web::go(heading.paragraph))
    }

    pub fn zoom(&self) -> f64 {
        self.reader().zoom
    }

    /// The zoom step after the current one (`direction` 1), before it
    /// (-1), or 1 (0); the script that shows it, if it changed.
    pub fn step_zoom(&self, direction: i32) -> Option<String> {
        let mut reader = self.reader();
        let zoom = match direction {
            0 => 1.0,
            _ => cr_app::books::step_zoom(reader.zoom, direction)?,
        };
        if (zoom - reader.zoom).abs() < 0.001 {
            return None;
        }
        reader.zoom = zoom;
        self.state.set_zoom(zoom);
        Some(web::zoom(zoom))
    }

    pub fn query(&self) -> String {
        self.reader().query.clone()
    }

    /// Finds `query` and shows the first match after the top; the
    /// scripts that highlight them, once the page has loaded.
    pub fn search(&self, query: String) -> Vec<String> {
        let mut reader = self.reader();
        reader.query = query;
        if !reader.ready {
            return Vec::new();
        }
        self.find(&mut reader)
    }

    /// The match after (`direction` 1) or before (-1) the one shown.
    pub fn step_match(&self, direction: i32) -> Option<String> {
        let mut reader = self.reader();
        let next = cr_app::books::step_match(reader.current, reader.matches.len(), direction)?;
        self.show_match(&mut reader, next)
    }

    pub fn has_matches(&self) -> bool {
        !self.reader().matches.is_empty()
    }

    /// What Find says of its matches.
    pub fn found(&self) -> String {
        let reader = self.reader();
        match (reader.query.is_empty(), reader.current) {
            (true, _) => String::new(),
            (false, Some(current)) => tr!(
                "{current} of {total}",
                current = current + 1,
                total = reader.matches.len()
            ),
            (false, None) if reader.matches.is_empty() && reader.ready => {
                tr("No Results").to_owned()
            }
            (false, None) => String::new(),
        }
    }
}

impl Drop for BookSession {
    /// The page goes with the book.
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Deletes the pages of books left in `folder` by a Core Rules that
/// ended with a book open.
fn sweep(folder: &Path) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|modified| modified.elapsed().is_ok_and(|age| age > STALE));
        if stale && path.extension().is_some_and(|e| e == "html") {
            let _ = std::fs::remove_file(path);
        }
    }
}
