//! Roll on a Table (the DM Tool Kit's Table Rolls) with no toolkit in
//! it: a book and one of its tables, the table's options, and the roll.
//! Roll Again keeps the session, so the original's re-roll of results
//! within 1 of the previous one carries over as it did in its Roll
//! Results dialog; what a closed session leaves is the app's
//! [`Leftovers`], which outlive the roller as the original's stack does.

use std::sync::Arc;

use cr_store::tables::{
    Block, Leftovers, ListEntry, Param, RollResult, Session, TableBook, TableRolls,
};
use cr_store::treasure::Rand;

/// A change in the roller, or one of its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum TableInput {
    /// The book with this index.
    Book(usize),
    /// The book's table with this index.
    Table(usize),
    /// The option with this index of the parameter with that one.
    Option(usize, usize),
    Roll,
    /// Roll Again: on the same table, in the same session.
    Again,
}

/// The roller.
#[derive(Clone)]
pub struct TableRoller {
    rolls: Arc<TableRolls>,
    books: Vec<TableBook>,
    /// The book chosen, by index.
    pub book: usize,
    entries: Vec<&'static ListEntry>,
    titles: Vec<String>,
    /// The book's table chosen, by index.
    pub table: usize,
    params: Vec<Param>,
    /// The option chosen of each parameter.
    pub chosen: Vec<usize>,
    /// The last roll.
    pub result: Option<RollResult>,
    session: Session,
    /// Whether the session is the table's, which Roll Again goes on.
    open: bool,
}

impl TableRoller {
    /// The roller on `last`, the book and table chosen last, or on the
    /// DMG's first table.
    pub fn new(rolls: Arc<TableRolls>, last: Option<(TableBook, usize)>) -> Self {
        let books = rolls.books();
        let (last_book, last_table) = last.unwrap_or((TableBook::Dmg, 0));
        let book = books.iter().position(|b| *b == last_book).unwrap_or(0);
        let mut roller = Self {
            rolls,
            books,
            book,
            entries: Vec::new(),
            titles: Vec::new(),
            table: 0,
            params: Vec::new(),
            chosen: Vec::new(),
            result: None,
            session: Session::default(),
            open: false,
        };
        roller.fill_tables(last_table);
        roller
    }

    pub fn rolls(&self) -> &Arc<TableRolls> {
        &self.rolls
    }

    pub fn book_titles(&self) -> Vec<String> {
        self.books.iter().map(|b| b.title().to_owned()).collect()
    }

    /// The titles of the book's tables.
    pub fn table_titles(&self) -> &[String] {
        &self.titles
    }

    /// The parameters of the table, with their options.
    pub fn params(&self) -> &[Param] {
        &self.params
    }

    fn entry(&self) -> Option<&'static ListEntry> {
        self.entries.get(self.table).copied()
    }

    /// The book and table chosen, for the next roller to start on.
    pub fn last(&self) -> Option<(TableBook, usize)> {
        Some((*self.books.get(self.book)?, self.table))
    }

    /// Whether Roll Again goes on: not on a table the book has instead,
    /// nor on one that opens the treasure generator.
    pub fn again_enabled(&self) -> bool {
        self.open
            && self
                .result
                .as_ref()
                .is_some_and(|r| r.book_table.is_none() && !r.treasure)
    }

    /// Lists the tables of the chosen book and selects `selected`.
    fn fill_tables(&mut self, selected: usize) {
        let book = self.books.get(self.book).copied().unwrap_or(TableBook::Dmg);
        self.entries = self.rolls.entries(book);
        self.titles = self
            .entries
            .iter()
            .map(|e| self.rolls.entry_title(e))
            .collect();
        self.table = selected.min(self.titles.len().saturating_sub(1));
        self.fill_params();
    }

    /// The parameters of the chosen table, each on its first option.
    fn fill_params(&mut self) {
        self.params = self
            .entry()
            .map(|entry| self.rolls.params(entry, &[]))
            .unwrap_or_default();
        self.chosen = vec![0; self.params.len()];
    }

    /// Ends the session, as closing the original's dialog does, leaving
    /// its last result in `leftovers`.
    pub fn close(&mut self, leftovers: &mut Leftovers) {
        let session = std::mem::take(&mut self.session);
        leftovers.close(&session);
        self.open = false;
    }

    pub fn input(&mut self, input: TableInput, leftovers: &mut Leftovers, rand: &mut Rand) {
        match input {
            // A new table is a new Roll Results session.
            TableInput::Book(book) => {
                if book != self.book && book < self.books.len() {
                    self.close(leftovers);
                    self.book = book;
                    self.fill_tables(0);
                }
            }
            TableInput::Table(table) => {
                if table != self.table && table < self.entries.len() {
                    self.close(leftovers);
                    self.table = table;
                    self.fill_params();
                }
            }
            // Options can depend on earlier choices (the called shots
            // depend on the target): the later ones are worked out again,
            // keeping their choice where they still have it.
            TableInput::Option(param, option) => {
                let Some(entry) = self.entry() else {
                    return;
                };
                let Some(chosen) = self.chosen.get_mut(param) else {
                    return;
                };
                *chosen = option;
                let params = self.rolls.params(entry, &self.chosen);
                for (i, new) in params.into_iter().enumerate().skip(param + 1) {
                    if let (Some(old), Some(chosen)) =
                        (self.params.get_mut(i), self.chosen.get_mut(i))
                    {
                        if *chosen >= new.options.len() {
                            *chosen = 0;
                        }
                        *old = new;
                    }
                }
            }
            TableInput::Roll | TableInput::Again => {
                let Some(entry) = self.entry() else {
                    return;
                };
                if input == TableInput::Roll || !self.open {
                    self.close(leftovers);
                    self.session = leftovers.open(entry);
                    self.open = true;
                }
                self.result = Some(
                    self.rolls
                        .roll(entry, &self.chosen, &mut self.session, rand),
                );
            }
        }
    }
}

/// A block's columns, without its title, subtitle and notes, for a
/// fixed-width font.
pub fn columns(block: &Block) -> String {
    Block {
        title: None,
        subtitle: None,
        notes: Vec::new(),
        ..block.clone()
    }
    .to_text()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rolls() -> Option<Arc<TableRolls>> {
        let dir = std::env::var_os("CORERULES_DIR")?;
        let install = cr_store::Installation::open(std::path::Path::new(&dir)).ok()?;
        install.table_rolls().ok().map(Arc::new)
    }

    #[test]
    fn a_new_table_ends_the_session() {
        let Some(rolls) = rolls() else {
            return;
        };
        let mut leftovers = Leftovers::default();
        let mut rand = Rand::seeded(3);
        let mut roller = TableRoller::new(rolls, None);
        assert!(!roller.table_titles().is_empty());
        assert!(!roller.again_enabled());
        let rolled = (0..roller.table_titles().len()).find(|&t| {
            roller.input(TableInput::Table(t), &mut leftovers, &mut rand);
            roller.input(TableInput::Roll, &mut leftovers, &mut rand);
            roller.again_enabled()
        });
        let Some(table) = rolled else {
            panic!("no table of the DMG rolls here");
        };
        assert!(!roller.result.as_ref().unwrap().blocks.is_empty());
        roller.input(TableInput::Again, &mut leftovers, &mut rand);
        assert!(roller.again_enabled());
        let other = if table == 0 { 1 } else { 0 };
        roller.input(TableInput::Table(other), &mut leftovers, &mut rand);
        assert!(!roller.again_enabled());
        assert_eq!(roller.last().map(|(_, t)| t), Some(other));
    }
}
