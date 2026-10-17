//! The rule books (`Books/*.rtf`): which ones an installation has, their
//! titles and categories, and a contents outline for each.
//!
//! The originals have no reader of their own. The Expansion lists its
//! twenty books on five shelves (registry `Book1`..`Book20`, written by
//! its installer) and hands the chosen file to WinHelp, a browser or
//! WordPad; Core Rules 2.0 has a fixed stack of nine. The books carry no
//! usable outline either: headings are direct formatting (a short
//! paragraph in one bold or large, coloured format), and each book uses
//! its own colours for them. [`outline`] finds them that way.

use std::path::PathBuf;

use cr_formats::rtf::{Document, Paragraph};

use crate::catalog::{Entry, Group};
use crate::encounter::flavor_of;
use crate::sheet::Flavor;
use crate::{Error, Installation, Result};

/// The shelves of the Expansion's Books screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    Core,
    Supplements,
    Options,
    Classes,
    Races,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Category::Core,
        Category::Supplements,
        Category::Options,
        Category::Classes,
        Category::Races,
    ];

    /// The title of the List of Books dialog for this shelf.
    pub fn title(self) -> &'static str {
        match self {
            Category::Core => "Core Books",
            Category::Supplements => "Supplements",
            Category::Options => "Option Books",
            Category::Classes => "Books of Classes",
            Category::Races => "Books of Races",
        }
    }
}

/// A book the programs know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BookInfo {
    /// The Expansion's registry number (`Book<n>`), 1 to 20.
    pub number: u8,
    pub title: &'static str,
    pub category: Category,
    /// The file in `Books/`.
    pub file: &'static str,
    /// The `\cfN` colours the book's headings use (see [`outline`]).
    pub heading_colors: &'static [u16],
}

impl BookInfo {
    /// The file name without `.rtf`, which the command line uses as the
    /// book's short name (`PhbBk`).
    pub fn short_name(&self) -> &'static str {
        self.file.strip_suffix(".rtf").unwrap_or(self.file)
    }
}

const fn book(
    number: u8,
    title: &'static str,
    category: Category,
    file: &'static str,
    heading_colors: &'static [u16],
) -> BookInfo {
    BookInfo {
        number,
        title,
        category,
        file,
        heading_colors,
    }
}

/// The Expansion's twenty books in registry order, with the heading
/// colours measured over each file: red (6) and purple (12) in the core
/// books, blue (2) or maroon (13) in the Complete Handbooks, navy (9) in
/// the Monstrous Manual and the Arms & Equipment Guide.
pub const BOOKS: [BookInfo; 20] = [
    book(
        1,
        "Player's Handbook",
        Category::Core,
        "PhbBk.rtf",
        &[6, 12],
    ),
    book(2, "Monstrous Manual", Category::Core, "MonBk.rtf", &[9]),
    book(
        3,
        "Dungeon Master's Guide",
        Category::Core,
        "DmgBk.rtf",
        &[6, 12],
    ),
    book(
        4,
        "Tome of Magic",
        Category::Supplements,
        "TomeBk.rtf",
        &[2],
    ),
    book(
        5,
        "Arms & Equipment Guide",
        Category::Supplements,
        "ArmsBk.rtf",
        &[9],
    ),
    book(
        6,
        "Player's Option: Skills & Powers",
        Category::Options,
        "POSPBk.rtf",
        &[6, 12],
    ),
    book(
        7,
        "Player's Option: Spells & Magic",
        Category::Options,
        "POSMBk.rtf",
        &[6, 12],
    ),
    book(
        8,
        "Player's Option: Combat & Tactics",
        Category::Options,
        "POCTBk.rtf",
        &[6, 12],
    ),
    book(
        9,
        "DM Option: High Level Campaigns",
        Category::Options,
        "DOHLCBk.rtf",
        &[6, 12],
    ),
    book(
        10,
        "The Complete Book of Elves",
        Category::Races,
        "ElfBk.rtf",
        &[13],
    ),
    book(
        11,
        "Complete Book of Dwarves",
        Category::Races,
        "DwarfBk.rtf",
        &[13],
    ),
    book(
        12,
        "Complete Book of Gnomes & Halflings",
        Category::Races,
        "GnmHlfBk.rtf",
        &[13],
    ),
    book(
        13,
        "The Complete Wizard's Handbook",
        Category::Classes,
        "WizardBk.rtf",
        &[2],
    ),
    book(
        14,
        "The Complete Ranger's Handbook",
        Category::Classes,
        "RangerBk.rtf",
        &[13],
    ),
    book(
        15,
        "The Complete Bard's Handbook",
        Category::Classes,
        "BardBk.rtf",
        &[13],
    ),
    book(
        16,
        "The Complete Druid's Handbook",
        Category::Classes,
        "DruidBk.rtf",
        &[13],
    ),
    book(
        17,
        "The Complete Fighter's Handbook",
        Category::Classes,
        "FightrBk.rtf",
        &[2],
    ),
    book(
        18,
        "The Complete Paladin's Handbook",
        Category::Classes,
        "PaladnBk.rtf",
        &[13],
    ),
    book(
        19,
        "The Complete Priest's Handbook",
        Category::Classes,
        "PriestBk.rtf",
        &[2],
    ),
    book(
        20,
        "The Complete Thief's Handbook",
        Category::Classes,
        "ThiefBk.rtf",
        &[2],
    ),
];

/// Core Rules 2.0's stack of books (BooksDlg), top to bottom, by registry
/// number. Its two help files are not books.
pub const CORE20_BOOKS: [u8; 9] = [1, 3, 2, 5, 4, 9, 8, 6, 7];

/// A book found in an installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Book {
    pub info: &'static BookInfo,
    pub path: PathBuf,
    /// File size in bytes.
    pub size: u64,
}

impl Installation {
    /// The books present in `Books/`: the Expansion's list sorted by shelf
    /// then title (as each List of Books sorts it), or Core Rules 2.0's
    /// stack in order.
    pub fn books(&self) -> Vec<Book> {
        let present = |info: &'static BookInfo| {
            let path = self.path(&format!("Books/{}", info.file))?;
            let size = std::fs::metadata(&path).ok()?.len();
            Some(Book { info, path, size })
        };
        match flavor_of(self) {
            Flavor::Expansion => {
                let mut books: Vec<Book> = BOOKS.iter().filter_map(present).collect();
                books.sort_by_key(|b| (b.info.category, b.info.title));
                books
            }
            Flavor::Core20 => CORE20_BOOKS
                .iter()
                .filter_map(|n| present(&BOOKS[*n as usize - 1]))
                .collect(),
        }
    }

    /// A book by file name (with or without `.rtf`), registry number or
    /// title, ignoring case.
    pub fn book(&self, name: &str) -> Option<Book> {
        let name = name.trim();
        let wanted = name.strip_suffix(".rtf").unwrap_or(name);
        self.books().into_iter().find(|b| {
            b.info.short_name().eq_ignore_ascii_case(wanted)
                || b.info.title.eq_ignore_ascii_case(name)
                || name.parse::<u8>() == Ok(b.info.number)
        })
    }

    /// Reads and parses a book.
    pub fn read_book(&self, book: &Book) -> Result<Document> {
        book.read()
    }

    /// The Books section of the catalog: every book present, keyed
    /// [`BOOK_KEY`] and the book's file name without `.rtf`.
    pub(crate) fn book_entries(&self) -> Vec<Entry> {
        self.books()
            .into_iter()
            .map(|b| Entry {
                key: format!("{BOOK_KEY}{}", b.info.short_name()),
                title: b.info.title.to_owned(),
                subtitle: b.info.category.title().to_owned(),
                groups: vec![Group {
                    title: "Book".into(),
                    rows: vec![
                        ("Shelf".into(), b.info.category.title().to_owned()),
                        ("File".into(), b.info.file.to_owned()),
                        ("Size".into(), format!("{} KB", b.size.div_ceil(1024))),
                    ],
                }],
                text: String::new(),
                npc: false,
                party: String::new(),
            })
            .collect()
    }
}

/// The key prefix of a book in the catalog.
pub const BOOK_KEY: &str = "book:";

impl Book {
    /// Reads and parses the book (no installation needed, so it can run
    /// on another thread).
    pub fn read(&self) -> Result<Document> {
        let data = std::fs::read(&self.path).map_err(|source| Error::Io {
            path: self.path.clone(),
            source,
        })?;
        Ok(Document::parse(&data))
    }
}

/// A heading of the outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    /// 0 for the biggest headings, at most 3.
    pub level: u8,
    /// The paragraph the heading starts at.
    pub paragraph: usize,
    pub title: String,
}

/// What a heading paragraph looks like: (bold, size, colour index).
type Signature = (bool, u16, u16);

/// The outline of a book, from its formatting.
///
/// A paragraph is a heading when its text is at most 90 characters with
/// no tab, starts with a capital or a digit, every non-blank run has the
/// same weight, size and colour, and it is either 14 pt or larger, or bold
/// in one of the book's heading colours (`heading_colors`; when empty, any
/// colour but automatic and black). Levels rank the (size, bold) pairs
/// found, biggest first. "Chapter 1:" followed by its title in the next
/// paragraph makes one heading. When nothing qualifies, any short bold
/// paragraph in a colour does, and failing that any short bold paragraph.
pub fn outline(doc: &Document, heading_colors: &[u16]) -> Vec<Heading> {
    let passes: [&dyn Fn(Signature) -> bool; 3] = [
        &|(bold, size, color)| {
            size >= 28
                || (bold
                    && if heading_colors.is_empty() {
                        color > 1
                    } else {
                        heading_colors.contains(&color)
                    })
        },
        &|(bold, size, color)| size >= 28 || (bold && color > 1),
        &|(bold, _, _)| bold,
    ];
    for accept in passes {
        let found = headings(doc, accept);
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

fn headings(doc: &Document, accept: &dyn Fn(Signature) -> bool) -> Vec<Heading> {
    let found: Vec<(usize, Signature, String)> = doc
        .paragraphs
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let (sig, text) = signature(p)?;
            accept(sig).then_some((i, sig, text))
        })
        .collect();
    let mut ranks: Vec<(u16, bool)> = found.iter().map(|(_, s, _)| (s.1, s.0)).collect();
    ranks.sort_unstable_by(|a, b| b.cmp(a));
    ranks.dedup();
    let level = |s: &Signature| {
        ranks
            .iter()
            .position(|r| *r == (s.1, s.0))
            .unwrap_or(0)
            .min(3) as u8
    };

    let mut out: Vec<Heading> = Vec::new();
    let mut i = 0;
    while i < found.len() {
        let (para, sig, text) = &found[i];
        let mut title = text.clone();
        let mut lvl = level(sig);
        // "Chapter 1:" then "Player Character Ability Scores".
        if title.ends_with(':')
            && let Some((next, nsig, ntext)) = found.get(i + 1)
            && next - para <= 2
            && doc.paragraphs[para + 1..*next]
                .iter()
                .all(|p| p.text().trim().is_empty())
        {
            title = format!("{title} {ntext}");
            lvl = lvl.min(level(nsig));
            i += 1;
        }
        let duplicate = out.last().is_some_and(|h| h.title == title);
        if !duplicate {
            out.push(Heading {
                level: lvl,
                paragraph: *para,
                title,
            });
        }
        i += 1;
    }
    drop_printed_contents(&mut out);
    out
}

/// Drops the entries of the book's printed contents ("Table of
/// Contents" and the headings under it) that a later heading repeats:
/// the outline already leads to the chapter itself.
fn drop_printed_contents(out: &mut Vec<Heading>) {
    let fold = |t: &str| -> String {
        t.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    };
    let Some(start) = out
        .iter()
        .position(|h| matches!(fold(&h.title).as_str(), "tableofcontents" | "contents"))
    else {
        return;
    };
    let level = out[start].level;
    let end = out[start + 1..]
        .iter()
        .position(|h| h.level <= level)
        .map_or(out.len(), |p| start + 1 + p);
    let later: std::collections::HashSet<String> =
        out[end..].iter().map(|h| fold(&h.title)).collect();
    let mut index = 0;
    out.retain(|h| {
        let keep = index <= start || index >= end || !later.contains(&fold(&h.title));
        index += 1;
        keep
    });
}

/// The format a heading candidate is in, and its text.
fn signature(p: &Paragraph) -> Option<(Signature, String)> {
    let text = p.text();
    let text = text.trim();
    if text.is_empty() || text.chars().count() > 90 || text.contains('\t') {
        return None;
    }
    let first = text.chars().next()?;
    if !(first.is_uppercase() || first.is_ascii_digit()) {
        return None;
    }
    let mut sig = None;
    for (t, f) in p.runs() {
        if t.trim().is_empty() {
            continue;
        }
        let s = (f.bold, f.size, f.color_index);
        match sig {
            None => sig = Some(s),
            Some(prev) if prev != s => return None,
            _ => {}
        }
    }
    Some((sig?, text.split_whitespace().collect::<Vec<_>>().join(" ")))
}

/// The captions `Table N: Title` a book gives its tables, first one per
/// number (the book's own List of Tables comes first).
pub fn table_captions(doc: &Document) -> std::collections::HashMap<u16, String> {
    let mut captions = std::collections::HashMap::new();
    for p in &doc.paragraphs {
        let text = p.text();
        let text = text.trim();
        let Some(rest) = text.strip_prefix("Table ") else {
            continue;
        };
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let Ok(number) = digits.parse::<u16>() else {
            continue;
        };
        let Some(title) = rest[digits.len()..].strip_prefix(':') else {
            continue;
        };
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        if !title.is_empty() && title.len() <= 80 {
            captions.entry(number).or_insert(title);
        }
    }
    captions
}

/// Text folded for searching: lower case, non-breaking spaces and hyphens
/// made plain, curly quotes straight. Every character folds to exactly
/// one, so character offsets in the folded text are offsets in the
/// original.
pub fn search_fold(text: &str) -> String {
    text.chars().map(fold_char).collect()
}

/// One character of [`search_fold`].
pub fn fold_char(c: char) -> char {
    match c {
        '\u{a0}' => ' ',
        '\u{2011}' | '\u{2010}' => '-',
        '\u{2018}' | '\u{2019}' => '\'',
        '\u{201c}' | '\u{201d}' => '"',
        c => c.to_lowercase().next().unwrap_or(c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(body: &str) -> Document {
        Document::parse(
            format!(
                r"{{\rtf1\ansi{{\fonttbl{{\f0 Times;}}}}{{\colortbl;\red0\green0\blue0;\red0\green0\blue255;\red255\green0\blue0;}}{body}}}"
            )
            .as_bytes(),
        )
    }

    #[test]
    fn headings_by_format() {
        let d = doc(concat!(
            r"{\fs48\cf3 Chapter 1:\par}\par{\fs48\cf3 Getting Started\par}",
            r"Body text in plain format that goes on.\par",
            r"{\b\cf2 Rolling Dice\par}",
            r"{\b\cf3 Not a heading colour\par}",
            r"{\b\cf2 lowercase cell\par}",
            r"{\b\cf2 Mixed }{\cf2 runs\par}",
            r"{\b\cf2 Tabbed\tab row\par}",
            r"{\b\fs36\cf2 Chapter 2: Next\par}",
            r"{\b\fs36\cf2 Chapter 2: Next\par}",
        ));
        let o = outline(&d, &[2]);
        let titles: Vec<_> = o.iter().map(|h| (h.level, h.title.as_str())).collect();
        assert_eq!(
            titles,
            [
                (0, "Chapter 1: Getting Started"),
                (2, "Rolling Dice"),
                (1, "Chapter 2: Next"),
            ]
        );
        assert_eq!(o[0].paragraph, 0);
    }

    #[test]
    fn the_printed_contents_give_way_to_the_chapters() {
        let d = doc(concat!(
            r"{\fs36\cf3 Table of Contents\par}",
            r"{\b\cf2 Getting Started\par}",
            r"{\b\cf2 Only In Contents\par}",
            r"{\fs36\cf3 Getting Started\par}",
        ));
        let titles: Vec<_> = outline(&d, &[2]).into_iter().map(|h| h.title).collect();
        assert_eq!(
            titles,
            ["Table of Contents", "Only In Contents", "Getting Started"]
        );
    }

    #[test]
    fn falls_back_to_bold_paragraphs() {
        let d = doc(r"{\b Plain Bold Title\par}Text.\par");
        assert_eq!(outline(&d, &[6])[0].title, "Plain Bold Title");
        assert!(outline(&doc(r"Nothing here.\par"), &[]).is_empty());
    }

    #[test]
    fn captions() {
        let d = doc(
            r"Table 61:\tab Turning Undead\par Table 61: Later copy\par Table 7:\par Tables are\par",
        );
        let c = table_captions(&d);
        assert_eq!(c.get(&61).map(String::as_str), Some("Turning Undead"));
        assert!(!c.contains_key(&7));
    }

    #[test]
    fn folding() {
        assert_eq!(
            search_fold("Non\u{2011}Stop\u{a0}\u{2019}Ok"),
            "non-stop 'ok"
        );
    }

    #[test]
    fn books_in_an_installation() {
        let root = std::env::temp_dir().join(format!("cr-books-test-{}", std::process::id()));
        std::fs::create_dir_all(root.join("Database")).unwrap();
        std::fs::create_dir_all(root.join("UserDbas")).unwrap();
        std::fs::create_dir_all(root.join("BOOKS")).unwrap();
        std::fs::write(root.join("UserDbas/AllowedU.dat"), []).unwrap();
        std::fs::write(root.join("BOOKS/phbbk.RTF"), br"{\rtf1 Hi\par}").unwrap();
        std::fs::write(root.join("BOOKS/ElfBk.rtf"), b"").unwrap();
        let install = Installation::open(&root).unwrap();
        let books = install.books();
        let names: Vec<_> = books.iter().map(|b| b.info.short_name()).collect();
        assert_eq!(names, ["PhbBk", "ElfBk"]);
        let phb = install.book("phbbk").unwrap();
        assert_eq!(install.book("1").unwrap().info.number, 1);
        assert_eq!(install.book("player's handbook").unwrap().info.number, 1);
        assert_eq!(install.read_book(&phb).unwrap().paragraphs[0].text(), "Hi");
        // Without the Expansion's AllowedU.dat, the 2.0 stack: only the
        // nine 2.0 books.
        std::fs::remove_file(root.join("UserDbas/AllowedU.dat")).unwrap();
        let names: Vec<_> = install.books().iter().map(|b| b.info.number).collect();
        assert_eq!(names, [1]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
