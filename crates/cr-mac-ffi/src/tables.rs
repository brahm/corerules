//! Roll on a Table, as the Mac app draws it in a window of its own: the
//! roller of `cr_app::tables`, a book and one of its tables, the table's
//! options, and the results of the roll, as the Windows app's page has
//! them. What the original keeps between rolls of different tables, and
//! the table last rolled on, are the app's, kept by [`Generators`].

use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::tables::{TableInput, TableRoller};
use cr_store::tables::{Leftovers, TableBook, TableRolls};
use cr_store::treasure::Rand;

/// What the generators keep while the app runs: the random numbers they
/// share, what tables leave for the next roll, and the table last
/// rolled on.
pub(crate) struct Generators {
    pub rand: Arc<Mutex<Rand>>,
    pub leftovers: Arc<Mutex<Leftovers>>,
    pub table_last: Arc<Mutex<Option<(TableBook, usize)>>>,
}

impl Default for Generators {
    fn default() -> Self {
        Self {
            rand: Arc::new(Mutex::new(Rand::from_clock())),
            leftovers: Arc::default(),
            table_last: Arc::default(),
        }
    }
}

/// A table's option, with the one chosen.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TableOption {
    pub label: String,
    pub options: Vec<String>,
    pub chosen: u32,
}

/// A block of the results: its title and subtitle, its columns, the
/// roll first, and its notes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TableBlock {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub notes: Vec<String>,
}

/// The table a roll comes from, for Read in Book: the book and what Find
/// looks for in it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TableReference {
    pub book: String,
    pub find: String,
}

/// What the last roll produced.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TableResult {
    pub blocks: Vec<TableBlock>,
    /// The results as Copy Results copies them.
    pub text: String,
    pub reference: Option<TableReference>,
    /// The original opens the treasure generator for this result.
    pub treasure: bool,
}

/// Roll on a Table, open in its window.
#[derive(uniffi::Object)]
pub struct TableSession {
    roller: Mutex<TableRoller>,
    rand: Arc<Mutex<Rand>>,
    leftovers: Arc<Mutex<Leftovers>>,
    last: Arc<Mutex<Option<(TableBook, usize)>>>,
}

impl TableSession {
    pub(crate) fn open(rolls: Arc<TableRolls>, generators: &Generators) -> Arc<Self> {
        let last = *generators
            .table_last
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        Arc::new(Self {
            roller: Mutex::new(TableRoller::new(rolls, last)),
            rand: generators.rand.clone(),
            leftovers: generators.leftovers.clone(),
            last: generators.table_last.clone(),
        })
    }

    fn roller(&self) -> MutexGuard<'_, TableRoller> {
        self.roller.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn input(&self, input: TableInput) {
        let mut roller = self.roller();
        let mut leftovers = self.leftovers.lock().unwrap_or_else(|e| e.into_inner());
        let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
        roller.input(input, &mut leftovers, &mut rand);
        *self.last.lock().unwrap_or_else(|e| e.into_inner()) = roller.last();
    }
}

#[uniffi::export]
impl TableSession {
    /// The program help's topic.
    pub fn help(&self) -> u32 {
        cr_store::help::topic::TABLES
    }

    pub fn books(&self) -> Vec<String> {
        self.roller().book_titles()
    }

    /// The book chosen, by its place in [`Self::books`].
    pub fn book(&self) -> u32 {
        self.roller().book as u32
    }

    pub fn set_book(&self, index: u32) {
        self.input(TableInput::Book(index as usize));
    }

    /// The book's tables.
    pub fn tables(&self) -> Vec<String> {
        self.roller().table_titles().to_vec()
    }

    pub fn table(&self) -> u32 {
        self.roller().table as u32
    }

    pub fn set_table(&self, index: u32) {
        self.input(TableInput::Table(index as usize));
    }

    /// The table's options.
    pub fn options(&self) -> Vec<TableOption> {
        let roller = self.roller();
        roller
            .params()
            .iter()
            .enumerate()
            .map(|(i, param)| TableOption {
                label: param.label.clone(),
                options: param.options.clone(),
                chosen: roller.chosen.get(i).copied().unwrap_or(0) as u32,
            })
            .collect()
    }

    /// Option `option` of the table's option `index` chosen.
    pub fn set_option(&self, index: u32, option: u32) {
        self.input(TableInput::Option(index as usize, option as usize));
    }

    pub fn roll(&self) {
        self.input(TableInput::Roll);
    }

    /// Roll Again: on the same table, in the same session.
    pub fn again(&self) {
        self.input(TableInput::Again);
    }

    pub fn again_enabled(&self) -> bool {
        self.roller().again_enabled()
    }

    pub fn result(&self) -> Option<TableResult> {
        let roller = self.roller();
        let result = roller.result.as_ref()?;
        Some(TableResult {
            blocks: result
                .blocks
                .iter()
                .map(|b| TableBlock {
                    title: b.title.clone(),
                    subtitle: b.subtitle.clone(),
                    header: b.header.clone(),
                    rows: b.rows.clone(),
                    notes: b.notes.clone(),
                })
                .collect(),
            text: result.to_text(),
            reference: result.reference.map(|(book, number)| TableReference {
                book: book.short_name().to_owned(),
                find: format!("Table {number}:"),
            }),
            treasure: result.treasure,
        })
    }

    /// The window closed, or the table is read in its book: what the
    /// session leaves for the next roll is kept.
    pub fn close(&self) {
        let mut leftovers = self.leftovers.lock().unwrap_or_else(|e| e.into_inner());
        self.roller().close(&mut leftovers);
    }
}
