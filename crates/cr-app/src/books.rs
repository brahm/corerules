//! The book reader's logic, with no toolkit in it: the zoom steps, the
//! reading position and zoom kept between sessions, find over the book's
//! text folded for searching, and the contents heading a paragraph is
//! under. The GNOME reader window and the Windows Book page both use it.

use std::path::{Path, PathBuf};

use cr_formats::rtf::{Document, Inline};
use cr_store::Installation;
use cr_store::books::{Book, Heading, fold_char};

/// The zoom levels Zoom In and Zoom Out step through.
pub const ZOOM_STEPS: [f64; 13] = [
    0.5, 0.67, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0,
];

/// The widest the text gets at zoom 1, in pixels; book pages are about
/// 6.5 inches of text.
pub const TEXT_WIDTH: f64 = 760.0;

/// The highlighted matches at most; highlighting tens of thousands
/// would stall the view.
pub const MOST_HIGHLIGHTED: usize = 5000;

/// The key of the zoom in the state file.
const ZOOM: &str = "zoom";

/// The zoom step after `zoom` (`direction` 1) or before it (-1), if any.
pub fn step_zoom(zoom: f64, direction: i32) -> Option<f64> {
    if direction > 0 {
        ZOOM_STEPS.iter().copied().find(|z| *z > zoom + 0.001)
    } else {
        ZOOM_STEPS.iter().rev().copied().find(|z| *z < zoom - 0.001)
    }
}

/// The key of a book's reading position in the state file: the
/// installation and the book, as the GNOME app has always kept it.
pub fn position_key(install: &Installation, book: &Book) -> String {
    format!("{}|{}", install.root().display(), book.info.short_name())
}

/// The reader's state file: one `key<TAB>value` line per book position,
/// and the zoom, under the user's data folder (never in the
/// installation).
#[derive(Debug, Clone)]
pub struct State {
    path: PathBuf,
}

impl State {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
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

    pub fn get(&self, key: &str) -> Option<String> {
        Self::load(&self.path)
            .into_iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Stores a value; failures are ignored (the state is a convenience).
    pub fn set(&self, key: &str, value: &str) {
        if key.contains(['\n', '\t']) {
            return;
        }
        let mut entries = Self::load(&self.path);
        match entries.iter_mut().find(|(k, _)| k == key) {
            Some(entry) if entry.1 == value => return,
            Some(entry) => entry.1 = value.to_owned(),
            None => entries.push((key.to_owned(), value.to_owned())),
        }
        let text: String = entries.iter().map(|(k, v)| format!("{k}\t{v}\n")).collect();
        let Some(dir) = self.path.parent() else {
            return;
        };
        let temp = self.path.with_extension("tmp");
        let _ = std::fs::create_dir_all(dir)
            .and_then(|()| std::fs::write(&temp, text))
            .and_then(|()| std::fs::rename(&temp, &self.path));
    }

    /// The zoom kept, if it is one of the steps' range; 1 otherwise.
    pub fn zoom(&self) -> f64 {
        self.get(ZOOM)
            .and_then(|z| z.parse::<f64>().ok())
            .filter(|z| (ZOOM_STEPS[0]..=ZOOM_STEPS[ZOOM_STEPS.len() - 1]).contains(z))
            .unwrap_or(1.0)
    }

    pub fn set_zoom(&self, zoom: f64) {
        self.set(ZOOM, &zoom.to_string());
    }

    /// The paragraph a book was left at.
    pub fn position(&self, key: &str) -> Option<usize> {
        self.get(key).and_then(|p| p.parse().ok())
    }

    pub fn set_position(&self, key: &str, paragraph: usize) {
        self.set(key, &paragraph.to_string());
    }
}

/// A book's text folded for searching ([`fold_char`]), one char for each
/// char of the text: a picture is U+FFFC and each paragraph ends in a
/// line end. With where each paragraph starts in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Folded {
    pub chars: Vec<char>,
    pub starts: Vec<usize>,
}

impl Folded {
    /// The whole of `doc`, every picture counted.
    pub fn of(doc: &Document) -> Self {
        let mut folded = Self::default();
        for p in &doc.paragraphs {
            folded.start_paragraph();
            for item in &p.items {
                match item {
                    Inline::Text(text, _) => folded.text(text),
                    Inline::Picture(_) => folded.picture(),
                }
            }
            folded.end_paragraph();
        }
        folded
    }

    pub fn start_paragraph(&mut self) {
        self.starts.push(self.chars.len());
    }

    pub fn text(&mut self, text: &str) {
        self.chars.extend(text.chars().map(fold_char));
    }

    pub fn picture(&mut self) {
        self.chars.push('\u{fffc}');
    }

    pub fn end_paragraph(&mut self) {
        self.chars.push('\n');
    }

    /// Where each match of `needle` starts, without overlaps.
    pub fn find(&self, needle: &str) -> Vec<usize> {
        let needle: Vec<char> = needle.chars().map(fold_char).collect();
        let hay = &self.chars;
        let mut found = Vec::new();
        if needle.is_empty() {
            return found;
        }
        let mut i = 0;
        while i + needle.len() <= hay.len() {
            if hay[i] == needle[0] && hay[i..i + needle.len()] == needle[..] {
                found.push(i);
                i += needle.len();
            } else {
                i += 1;
            }
        }
        found
    }

    /// The paragraph `offset` is in.
    pub fn paragraph_of(&self, offset: usize) -> usize {
        self.starts
            .partition_point(|&s| s <= offset)
            .saturating_sub(1)
    }

    /// Where paragraph `paragraph` starts.
    pub fn start_of(&self, paragraph: usize) -> Option<usize> {
        self.starts.get(paragraph).copied()
    }
}

/// The match a new search shows: the first at or after `top` (where the
/// text in view starts), else the first; with `skip_first`, the second,
/// for a table caption whose first match is the book's List of Tables.
pub fn first_match(matches: &[usize], top: usize, skip_first: bool) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    if skip_first {
        return Some(1.min(matches.len() - 1));
    }
    let first = matches.partition_point(|&m| m < top);
    Some(if first < matches.len() { first } else { 0 })
}

/// The match after (`direction` 1) or before (-1) `current`, going round.
pub fn step_match(current: Option<usize>, count: usize, direction: i32) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some(match current {
        Some(i) if direction > 0 => (i + 1) % count,
        Some(i) => (i + count - 1) % count,
        None => 0,
    })
}

/// The heading of the section `paragraph` is in.
pub fn heading_at(headings: &[Heading], paragraph: usize) -> Option<usize> {
    headings
        .partition_point(|h| h.paragraph <= paragraph)
        .checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_steps_up_and_down_and_stops_at_the_ends() {
        assert_eq!(step_zoom(1.0, 1), Some(1.1));
        assert_eq!(step_zoom(1.0, -1), Some(0.9));
        assert_eq!(step_zoom(4.0, 1), None);
        assert_eq!(step_zoom(0.5, -1), None);
        // Between steps, the next one either way.
        assert_eq!(step_zoom(1.2, 1), Some(1.25));
        assert_eq!(step_zoom(1.2, -1), Some(1.1));
    }

    #[test]
    fn state_keeps_positions_and_the_zoom() {
        let dir = std::env::temp_dir().join(format!("cr-app-books-{}", std::process::id()));
        let state = State::at(dir.join("reader-state"));
        assert_eq!(state.zoom(), 1.0);
        state.set_zoom(1.5);
        state.set_position("C:\\Core|PhbBk", 120);
        state.set_position("C:\\Core|DmgBk", 7);
        state.set_position("C:\\Core|PhbBk", 121);
        let again = State::at(dir.join("reader-state"));
        assert_eq!(again.zoom(), 1.5);
        assert_eq!(again.position("C:\\Core|PhbBk"), Some(121));
        assert_eq!(again.position("C:\\Core|DmgBk"), Some(7));
        assert_eq!(again.position("C:\\Core|MonBk"), None);
        // Keys that would break the file are not kept.
        again.set("a\tb", "1");
        assert_eq!(again.get("a\tb"), None);
        state.set("zoom", "9");
        assert_eq!(again.zoom(), 1.0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn find_folds_case_and_quotes_and_skips_overlaps() {
        let doc = Document::parse(
            br"{\rtf1\ansi{\fonttbl{\f0 Times;}}Rolling Dice\par The \ldblquote dice\rdblquote  roll: ddd\par}",
        );
        let folded = Folded::of(&doc);
        assert_eq!(folded.starts, [0, 13]);
        assert_eq!(folded.find("DICE"), [8, 18]);
        assert_eq!(folded.find("\"dice\""), [17]);
        assert_eq!(folded.find("dd"), [30]);
        assert!(folded.find("").is_empty());
        assert_eq!(folded.paragraph_of(8), 0);
        assert_eq!(folded.paragraph_of(18), 1);
        assert_eq!(folded.start_of(1), Some(13));
    }

    #[test]
    fn matches_start_in_view_and_go_round() {
        let matches = [10, 50, 90];
        assert_eq!(first_match(&matches, 40, false), Some(1));
        assert_eq!(first_match(&matches, 95, false), Some(0));
        assert_eq!(first_match(&matches, 0, true), Some(1));
        assert_eq!(first_match(&[10], 0, true), Some(0));
        assert_eq!(first_match(&[], 0, false), None);
        assert_eq!(step_match(Some(2), 3, 1), Some(0));
        assert_eq!(step_match(Some(0), 3, -1), Some(2));
        assert_eq!(step_match(None, 3, -1), Some(0));
        assert_eq!(step_match(None, 0, 1), None);
    }

    #[test]
    fn a_paragraph_is_under_the_heading_before_it() {
        let heading = |paragraph| Heading {
            level: 0,
            paragraph,
            title: String::new(),
        };
        let headings = [heading(3), heading(10)];
        assert_eq!(heading_at(&headings, 0), None);
        assert_eq!(heading_at(&headings, 3), Some(0));
        assert_eq!(heading_at(&headings, 9), Some(0));
        assert_eq!(heading_at(&headings, 400), Some(1));
    }
}
