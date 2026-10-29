//! Help, as the Mac app shows it in a window of its own: the
//! installation's WinHelp files, the list beside the topic (the contents,
//! the titled topics, or what a search finds), the topic as a web page
//! for a WKWebView with its links and popups, and Back. What each of
//! these shows comes from `cr_app::help`; the session keeps where the
//! reader is.

use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::help::{self as shared, Files, Followed, Place, Row, Target};
use cr_app::web::help::{self as web, POPUP};
use cr_formats::winhelp::Link;
use cr_i18n::tr;

use crate::CoreError;

/// A help file the book selector opens.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HelpBook {
    pub file: String,
    pub label: String,
}

/// The books of the book selector: the program help, the Online Manual
/// and the Tome of Help.
#[uniffi::export]
pub fn help_books() -> Vec<HelpBook> {
    [
        (shared::PROGRAM, "_Help"),
        (shared::MANUAL, "_Online Manual"),
        (shared::TOME, "_Tome of Help"),
    ]
    .into_iter()
    .map(|(file, label)| HelpBook {
        file: file.to_owned(),
        label: tr(label).to_owned(),
    })
    .collect()
}

/// The help of the original's screen `number` (a `[MAP]` number of the
/// program help), as Help on This Screen opens it.
#[uniffi::export]
pub fn program_help(number: u32) -> String {
    shared::program(number)
}

/// A row of the list beside the topic.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HelpRow {
    /// Its place in the list, which [`HelpSession::choose_row`] takes.
    pub index: u32,
    /// 1 for the top level.
    pub level: u32,
    pub title: String,
    /// Whether it opens a topic, rather than heading the rows under it.
    pub topic: bool,
}

/// The topic shown.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HelpTopic {
    pub title: String,
    /// The file's title, when it is not the topic's.
    pub file_title: String,
    pub html: String,
}

/// What a link of the topic leads to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum HelpLinked {
    /// Another topic, now shown.
    Shown,
    /// A popup over the topic: the page shows this with `showPopup(html)`.
    Popup { html: String },
    /// A web page, for the browser.
    Web { uri: String },
    /// A topic that is not in this installation, as the app says it.
    Missing { message: String },
    Nothing,
}

#[derive(Default)]
struct State {
    /// The file the rows list, lower case, and its title.
    listed: String,
    listed_title: String,
    query: String,
    rows: Vec<Row>,
    /// The row of the topic shown, when the rows are not a search's.
    row: Option<usize>,
    shown: Option<(Place, web::Page, HelpTopic)>,
    history: Vec<Place>,
    /// The file of the popup shown and its links.
    popup: Option<(String, Vec<Link>)>,
}

/// The help, open in its window.
#[derive(uniffi::Object)]
pub struct HelpSession {
    files: Arc<Files>,
    state: Mutex<State>,
}

impl HelpSession {
    pub(crate) fn new(files: Arc<Files>) -> Arc<Self> {
        Arc::new(Self {
            files,
            state: Mutex::default(),
        })
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn missing() -> CoreError {
        CoreError::new(tr("That topic is not in this installation"))
    }

    /// Shows topic `place`, with the rows of its file beside it unless
    /// those listed now are or list it; `remember` keeps the topic it
    /// replaces for Back.
    fn show(&self, state: &mut State, place: Place, remember: bool) -> Result<(), CoreError> {
        let help = self.files.file(&place.file).ok_or_else(Self::missing)?;
        let topic = help.topics.get(place.topic).ok_or_else(Self::missing)?;
        let title = shared::title(&help, topic);
        let file_title = if help.title == title {
            String::new()
        } else {
            help.title.clone()
        };
        if state.listed != place.file && !shared::lists(&state.rows, &place) {
            state.rows = self.files.rows(&place.file, "");
            state.listed = place.file.clone();
            state.listed_title = help.title.clone();
            state.query.clear();
        }
        if remember
            && let Some((current, ..)) = &state.shown
            && *current != place
        {
            state.history.push(current.clone());
        }
        // A search's rows are not the ones it was found in.
        state.row = if state.query.is_empty() {
            self.files.row_of(&state.rows, &place)
        } else {
            None
        };
        let page = web::topic(&help, topic);
        let shown = HelpTopic {
            title,
            file_title,
            html: page.html.clone(),
        };
        state.shown = Some((place, page, shown));
        state.popup = None;
        Ok(())
    }
}

#[uniffi::export]
impl HelpSession {
    /// Opens the help at `spec`: `Program.hlp#1234` (a topic's number),
    /// `ADandD.hlp#ctx` (a context string) or `Manual.hlp` (its
    /// contents).
    pub fn open(&self, spec: String) -> Result<(), CoreError> {
        let (file, target) = Target::parse(&spec);
        let place = self.files.resolve(file, &target).ok_or_else(|| {
            CoreError::new(cr_i18n::tr!(
                "The help topic is not in {file}",
                file = file.to_owned()
            ))
        })?;
        let mut state = self.state();
        self.show(&mut state, place, true)
    }

    /// The file the rows list, lower case.
    pub fn listed(&self) -> String {
        self.state().listed.clone()
    }

    /// The title of the file listed, for the book selector when it is
    /// none of its books.
    pub fn listed_title(&self) -> String {
        self.state().listed_title.clone()
    }

    /// The rows beside the topic.
    pub fn rows(&self) -> Vec<HelpRow> {
        self.state()
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| HelpRow {
                index: i as u32,
                level: r.level,
                title: r.title.split_whitespace().collect::<Vec<_>>().join(" "),
                topic: r.is_topic(),
            })
            .collect()
    }

    /// The row of the topic shown.
    pub fn row(&self) -> Option<u32> {
        self.state().row.map(|r| r as u32)
    }

    /// The search over the topics of the file listed; empty lists its
    /// contents again.
    pub fn search(&self, query: String) {
        let mut state = self.state();
        if state.query == query {
            return;
        }
        state.rows = self.files.rows(&state.listed, &query);
        state.query = query;
        state.row = None;
    }

    pub fn query(&self) -> String {
        self.state().query.clone()
    }

    /// Shows the topic of row `index`; a heading opens nothing.
    pub fn choose_row(&self, index: u32) -> Result<(), CoreError> {
        let mut state = self.state();
        let Some(row) = state.rows.get(index as usize) else {
            return Ok(());
        };
        if !row.is_topic() {
            return Ok(());
        }
        let place = self.files.row_place(row).ok_or_else(Self::missing)?;
        self.show(&mut state, place, true)
    }

    /// The topic shown.
    pub fn topic(&self) -> Option<HelpTopic> {
        self.state().shown.as_ref().map(|(.., t)| t.clone())
    }

    /// What the page posted: a link's index, or a popup's link's after
    /// [`POPUP`].
    pub fn link(&self, id: String) -> HelpLinked {
        let mut state = self.state();
        let (from, links, index) = match id.strip_prefix(POPUP) {
            Some(index) => match &state.popup {
                Some((file, links)) => (file.clone(), links.clone(), index.to_owned()),
                None => return HelpLinked::Nothing,
            },
            None => match &state.shown {
                Some((place, page, _)) => (place.file.clone(), page.links.clone(), id),
                None => return HelpLinked::Nothing,
            },
        };
        let Some(link) = index.parse::<usize>().ok().and_then(|i| links.get(i)) else {
            return HelpLinked::Nothing;
        };
        let missing = || HelpLinked::Missing {
            message: tr("That topic is not in this installation").to_owned(),
        };
        match self.files.follow(&from, link) {
            Followed::Go(place) => match self.show(&mut state, place, true) {
                Ok(()) => HelpLinked::Shown,
                Err(_) => missing(),
            },
            Followed::Popup(place) => {
                let page = self.files.file(&place.file).and_then(|help| {
                    let topic = help.topics.get(place.topic)?;
                    Some(web::popup(&help, topic))
                });
                match page {
                    Some(page) => {
                        state.popup = Some((place.file, page.links));
                        HelpLinked::Popup { html: page.html }
                    }
                    None => missing(),
                }
            }
            Followed::Web(uri) => HelpLinked::Web { uri },
            Followed::Missing => missing(),
        }
    }

    /// Whether Back has a topic to return to.
    pub fn can_go_back(&self) -> bool {
        !self.state().history.is_empty()
    }

    /// Back: the topic shown before.
    pub fn back(&self) -> Result<(), CoreError> {
        let mut state = self.state();
        let Some(place) = state.history.pop() else {
            return Ok(());
        };
        self.show(&mut state, place, false)
    }
}

/// `html` as a JavaScript string literal, for `showPopup(…)`.
#[uniffi::export]
pub fn js_string(text: String) -> String {
    web::js_string(&text)
}

/// The text of a popup topic, for an alert: its title and its
/// paragraphs.
pub(crate) fn popup_text(files: &Files, file: &str, number: u32) -> Option<(String, String)> {
    let help = files.file(file)?;
    let topic = help.topics.get(help.topic_of_number(number)?)?;
    let text = topic
        .paragraphs
        .iter()
        .map(|p| p.text())
        .collect::<Vec<_>>()
        .join("\n");
    Some((shared::title(&help, topic), text.trim().to_owned()))
}
