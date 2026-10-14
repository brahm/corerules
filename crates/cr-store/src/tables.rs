//! Table Rolls (DM Tool Kit): rolls on the tables of `Tables/*.TBL` as
//! the originals' Roll Results dialog does.
//!
//! The originals keep what each table does in code (Expansion 0x50c88e
//! and 0x51d5fd, about 8,000 decompiled lines): which die to roll, the
//! columns to print, how "(D6)" tables pick a subtable, which numbers of
//! an entry name the next table to roll on, and parameter pages such as
//! the critical hit dialog. Here that is data ([`data`]): every list
//! entry is a short program of [`Step`]s, and table ids reached by
//! following an entry have handlers of their own, with a generic roll for
//! the rest.
//!
//! What matches the originals:
//!
//! * The table id to file map and the record count read from each file,
//!   including the Expansion's PO:C&T mix-up (id 55400 reads POCT206E,
//!   2.0 reads POCT207A).
//! * Rolls use the shared MSVC `rand()` ([`Rand`]). A roll on a table
//!   (0x538d84) sums the dice and rolls again while the sum is within 1 of
//!   the previous result of the dialog, then adds the modifier; that
//!   previous result carries over to Roll Again and to chained tables
//!   ([`Session`]). Subtable choices ("D6") use a plain `1 + rand() % n`
//!   (0x554d44) with no such rule.
//! * The entry chosen is the first record whose high roll reaches the
//!   roll; `/nK` lines print under the first.
//! * Dice, subtable ranges, parameter lists and follow-up rules (v3 table
//!   ids, v4/v5 sub-subtable ids, location codes of the critical hit
//!   tables) are those of the code.
//!
//! Where this differs (the code was not followed far enough, or prints
//! text this project cannot ship):
//!
//! * The first roll of a dialog: the original compares with an
//!   uninitialised previous result (stack memory); here the first roll is
//!   free.
//! * Tables the originals compute entirely in code with their own text
//!   (DMO 14, 16, 17; DMG 47, 84, 112, 118; PHB 43, 61; S&P 25, 31, 36;
//!   S&M 14, 15, 47, 48) are not rolled; the result refers to the book.
//! * Some handlers re-roll in special cases (Table 36 "roll twice", the
//!   Wild Talents 97-100 rows); those print the table's entry instead.
//! * Spell scrolls pick each spell with `rand() % n` among the database's
//!   spells of the level; the original (0x52a35e) draws from its own
//!   spell index, so the names differ for the same seed.
//! * Column layout: the original pads to fixed character columns in a
//!   proportional font, so its columns do not line up; here they do.

mod data;

use std::collections::HashMap;

use cr_formats::Object;
use cr_formats::spells::SpellsFile;
use cr_formats::tbl::{TableFile, TableRecord};

pub use data::{LIST, TABLE_BOOKS};

use crate::books::{BOOKS, table_captions};
use crate::encounter::flavor_of;
use crate::sheet::Flavor;
use crate::treasure::Rand;
use crate::{Installation, Result};

/// `count` dice of `sides` sides plus `plus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dice {
    pub count: i32,
    pub sides: i32,
    pub plus: i32,
}

pub const fn d(count: i32, sides: i32) -> Dice {
    Dice {
        count,
        sides,
        plus: 0,
    }
}

impl Dice {
    pub const fn plus(self, plus: i32) -> Dice {
        Dice { plus, ..self }
    }

    /// "2d6+4".
    pub fn text(&self) -> String {
        let mut s = format!("{}d{}", self.count, self.sides);
        if self.plus > 0 {
            s += &format!("+{}", self.plus);
        } else if self.plus < 0 {
            s += &self.plus.to_string();
        }
        s
    }
}

/// The books of the Table Rolls combo box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TableBook {
    HighLevel,
    Dmg,
    Phb,
    CombatTactics,
    SkillsPowers,
    SpellsMagic,
}

impl TableBook {
    /// The registry number of the book (see [`BOOKS`]).
    pub fn book_number(self) -> u8 {
        match self {
            TableBook::HighLevel => 9,
            TableBook::Dmg => 3,
            TableBook::Phb => 1,
            TableBook::CombatTactics => 8,
            TableBook::SkillsPowers => 6,
            TableBook::SpellsMagic => 7,
        }
    }

    pub fn title(self) -> &'static str {
        BOOKS[self.book_number() as usize - 1].title
    }

    /// The book's file name without `.rtf`.
    pub fn short_name(self) -> &'static str {
        BOOKS[self.book_number() as usize - 1].short_name()
    }
}

/// How a block is titled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Title {
    /// "Table N: " and the book's caption for table N.
    Table(u16),
    /// The title of the list entry that was rolled.
    Entry,
    /// The table file's own header line.
    Header,
    /// A fixed label.
    Label(&'static str),
    /// No title: the rows continue the block above.
    None,
}

/// A column after the roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Col {
    /// The entry's text, with its continuation lines.
    Name(&'static str),
    Item1(&'static str),
    Item2(&'static str),
    /// v3 as an experience point value (blank when 0 or less).
    Xp,
    /// v3 as a number.
    V3(&'static str),
    /// v4 with a sign (+1, -1).
    SignedV4(&'static str),
    /// Objects of art: v3 dice of v4 sides times v5, rolled.
    Art(&'static str),
}

/// What else an entry produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extra {
    None,
    /// A spell scroll: the name starts with the number of spells, each of
    /// a level from v3 to v4; true for wizard spells.
    Scroll(bool),
}

/// A roll on one table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollSpec {
    pub id: i32,
    /// `None`: one die as large as the table's highest roll.
    pub dice: Option<Dice>,
    pub title: Title,
    pub subtitle: Option<&'static str>,
    pub cols: &'static [Col],
    /// How many times to roll (Ranger's and Thief's followers).
    pub rolls: Option<Dice>,
    /// Roll on the tables the entry's v3 to v5 name.
    pub follow: bool,
    pub extra: Extra,
}

impl RollSpec {
    pub const fn new(id: i32, dice: Dice, title: Title, cols: &'static [Col]) -> Self {
        Self {
            id,
            dice: Some(dice),
            title,
            subtitle: None,
            cols,
            rolls: None,
            follow: true,
            extra: Extra::None,
        }
    }

    pub const fn subtitle(mut self, subtitle: &'static str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    pub const fn rolls(mut self, rolls: Dice) -> Self {
        self.rolls = Some(rolls);
        self
    }

    pub const fn extra(mut self, extra: Extra) -> Self {
        self.extra = extra;
        self
    }
}

/// One branch of a subtable choice: taken when the die is at most `hi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Branch {
    pub hi: i32,
    pub label: &'static str,
    pub steps: &'static [Step],
}

/// The critical hit dialogs (PO: Combat & Tactics, PO: Spells & Magic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Critical {
    /// By target type (parameter 0): the location table and the offset
    /// the target type adds to the effect table id.
    pub targets: &'static [(i32, i32)],
    /// By weapon or spell type (parameter 1): added to the location code.
    pub kinds: &'static [i32],
    /// By size advantage (parameter 2): the location die.
    pub location_dice: &'static [Dice],
    /// By weapon size or damage (parameter 3): the severity dice.
    pub severity_dice: &'static [Dice],
    pub label: &'static str,
}

/// One step of a list entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Roll(RollSpec),
    /// `1 + rand() % die`, then the first branch whose `hi` reaches it.
    Choose {
        die: i32,
        branches: &'static [Branch],
    },
    /// The steps for the chosen option of a parameter.
    ByParam {
        param: usize,
        options: &'static [&'static [Step]],
    },
    /// A value looked up in a table plus dice: v3 of row `row` (a
    /// parameter) of the table `ids[sex]`, plus `dice[row]`.
    Lookup {
        ids: &'static [i32],
        id_param: usize,
        row_param: usize,
        dice: &'static [Dice],
        title: Title,
        label: &'static str,
        unit: &'static str,
    },
    /// Dice chosen by a parameter, their `plus` the base.
    Formula {
        param: usize,
        dice: &'static [Dice],
        label: &'static str,
        unit: &'static str,
    },
    /// PO: Spells & Magic Tables 23 and 24 (parameter 0 is the spell
    /// level). `check`: Table 23 first, Table 24 when insane.
    Insanity {
        check: bool,
    },
    Critical(&'static Critical),
}

/// Where the options of a parameter come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Options {
    Fixed(&'static [&'static str]),
    /// The names of a table's entries (races, spell levels).
    Rows(i32),
    /// The header of each table, up to `cut` (spell types).
    Headers(&'static [i32], &'static str),
    /// The first row of the location table of the target type chosen in
    /// parameter 0, then its entries: called shots.
    CalledShot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamSpec {
    pub label: &'static str,
    pub options: Options,
}

/// An entry of the Tables list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListEntry {
    pub book: TableBook,
    /// The table's number in the book.
    pub number: Option<u16>,
    /// The title when the table has no number.
    pub label: Option<&'static str>,
    pub params: &'static [ParamSpec],
    /// Empty: the original works the table out in code, with its own
    /// text; the result refers to the book.
    pub steps: &'static [Step],
}

/// A parameter with its options resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub label: String,
    pub options: Vec<String>,
}

/// One block of the results: a title, a column header and rows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Block {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    /// Column labels, the roll first.
    pub header: Vec<String>,
    /// Cells, the roll first; a cell may have several lines.
    pub rows: Vec<Vec<String>>,
    /// Lines after the rows.
    pub notes: Vec<String>,
}

/// What a roll produced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RollResult {
    pub blocks: Vec<Block>,
    /// The book and table number to read instead, for tables that are not
    /// rolled here.
    pub book_table: Option<(TableBook, u16)>,
}

impl RollResult {
    /// The result as the Roll Results list shows it, in columns for a
    /// fixed-width font.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (i, block) in self.blocks.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            block.write(&mut out);
        }
        out
    }
}

impl Block {
    /// The block in columns for a fixed-width font.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        let mut wrote = false;
        for line in self.title.iter().chain(&self.subtitle) {
            out.push_str(line);
            out.push('\n');
            wrote = true;
        }
        if !self.header.is_empty() {
            if wrote {
                out.push('\n');
            }
            let columns = self.header.len();
            let mut widths: Vec<usize> = self.header.iter().map(|h| h.chars().count()).collect();
            widths[0] = widths[0].max(4);
            for row in &self.rows {
                for (c, cell) in row.iter().enumerate().take(columns) {
                    let w = cell.lines().map(|l| l.chars().count()).max().unwrap_or(0);
                    widths[c] = widths[c].max(w);
                }
            }
            let line = |cells: &[String], out: &mut String| {
                let height = cells
                    .iter()
                    .map(|c| c.lines().count().max(1))
                    .max()
                    .unwrap_or(1);
                for k in 0..height {
                    let mut text = String::new();
                    for (c, cell) in cells.iter().enumerate() {
                        let part = cell.lines().nth(k).unwrap_or("");
                        text.push_str(part);
                        if c + 1 < cells.len() {
                            let pad = widths[c] + 2 - part.chars().count();
                            text.extend(std::iter::repeat_n(' ', pad));
                        }
                    }
                    out.push_str(text.trim_end());
                    out.push('\n');
                }
            };
            line(&self.header, out);
            let rules: Vec<String> = self
                .header
                .iter()
                .map(|h| "-".repeat(h.chars().count().max(4)))
                .collect();
            line(&rules, out);
            for row in &self.rows {
                line(row, out);
            }
            wrote = true;
        }
        if !self.notes.is_empty() {
            if wrote {
                out.push('\n');
            }
            for note in &self.notes {
                out.push_str(note);
                out.push('\n');
            }
        }
    }
}

/// The state a Roll Results dialog keeps between rolls: the previous
/// result of its table object.
#[derive(Debug, Clone, Default)]
pub struct Session {
    previous: Option<i32>,
}

impl Session {
    /// A roll on a table (0x538d84): rolled again while the sum is within
    /// 1 of the previous result. Gives up after 1000 tries, where the
    /// original would hang (a 1d2 table after a 1 or 2).
    pub fn roll(&mut self, rand: &mut Rand, dice: Dice) -> i32 {
        let mut sum;
        let mut tries = 0;
        loop {
            sum = 0i32;
            for _ in 0..dice.count.max(0) {
                sum = sum.wrapping_add(range(rand, 1, dice.sides));
            }
            tries += 1;
            match self.previous {
                Some(p) if (p.wrapping_sub(sum)).abs() < 2 && tries < 1000 => continue,
                _ => break,
            }
        }
        let result = sum.wrapping_add(dice.plus);
        self.previous = Some(result);
        result
    }
}

/// 0x554d44: a number from `lo` to `hi`, or 0 when the range is empty.
pub fn range(rand: &mut Rand, lo: i32, hi: i32) -> i32 {
    let n = hi.wrapping_add(1).wrapping_sub(lo);
    if n < 1 { 0 } else { rand.rand() % n + lo }
}

/// The tables of an installation, loaded once.
#[derive(Debug, Clone)]
pub struct TableRolls {
    flavor: Flavor,
    tables: HashMap<i32, TableFile>,
    /// Ids whose file is mapped but missing.
    missing: Vec<i32>,
    captions: HashMap<TableBook, HashMap<u16, String>>,
    /// (level, name) of every wizard and priest spell.
    wizard_spells: Vec<(i32, String)>,
    priest_spells: Vec<(i32, String)>,
}

/// Table id to file and the number of records the program reads (0x534d15,
/// 2.0 0x5c1160).
pub fn id_map(flavor: Flavor) -> impl Iterator<Item = (i32, &'static str, usize)> {
    data::ID_MAP.iter().map(move |&(id, file, count)| {
        let file = match (flavor, id) {
            (Flavor::Core20, 55400) => "POCT207A",
            _ => file,
        };
        (id, file, count)
    })
}

impl Installation {
    /// Loads the tables, the table captions of the books present and the
    /// spell list for scrolls.
    pub fn table_rolls(&self) -> Result<TableRolls> {
        let flavor = flavor_of(self);
        let mut tables = HashMap::new();
        let mut missing = Vec::new();
        for (id, file, count) in id_map(flavor) {
            match self.path(&format!("Tables/{file}.tbl")) {
                Some(path) => {
                    let bytes =
                        std::fs::read(&path).map_err(|source| crate::Error::Io { path, source })?;
                    tables.insert(id, TableFile::parse(&bytes, Some(count)));
                }
                None => missing.push(id),
            }
        }
        let mut captions = HashMap::new();
        for book in TABLE_BOOKS {
            if let Some(found) = self.book(book.short_name())
                && let Ok(doc) = self.read_book(&found)
            {
                captions.insert(book, table_captions(&doc));
            }
        }
        let (mut wizard_spells, mut priest_spells) = (Vec::new(), Vec::new());
        if let Ok(data) = self.read("Database/Spells.dat")
            && let Ok(spells) = SpellsFile::parse(&data)
        {
            for (list, out) in [
                (&spells.wizard, &mut wizard_spells),
                (&spells.priest, &mut priest_spells),
            ] {
                for object in list {
                    if let Object::Spell(s) = object {
                        out.push((s.level, s.name.clone()));
                    }
                }
            }
        }
        Ok(TableRolls {
            flavor,
            tables,
            missing,
            captions,
            wizard_spells,
            priest_spells,
        })
    }
}

impl TableRolls {
    pub fn flavor(&self) -> Flavor {
        self.flavor
    }

    /// A table by id.
    pub fn table(&self, id: i32) -> Option<&TableFile> {
        self.tables.get(&id)
    }

    /// Mapped ids whose file is not in `Tables/`.
    pub fn missing(&self) -> &[i32] {
        &self.missing
    }

    /// The books of the combo box that have tables, in its (sorted) order.
    pub fn books(&self) -> Vec<TableBook> {
        TABLE_BOOKS.to_vec()
    }

    /// The list entries of a book.
    pub fn entries(&self, book: TableBook) -> Vec<&'static ListEntry> {
        LIST.iter().filter(|e| e.book == book).collect()
    }

    /// The book's caption for a table number.
    pub fn caption(&self, book: TableBook, number: u16) -> Option<&str> {
        self.captions.get(&book)?.get(&number).map(String::as_str)
    }

    fn table_title(&self, book: TableBook, number: u16) -> String {
        match self.caption(book, number) {
            Some(c) => format!("Table {number}: {c}"),
            None => format!("Table {number}"),
        }
    }

    /// The title the list shows for an entry.
    pub fn entry_title(&self, entry: &ListEntry) -> String {
        match (entry.number, entry.label) {
            (Some(n), _) => self.table_title(entry.book, n),
            (None, Some(label)) => label.to_owned(),
            (None, None) => String::new(),
        }
    }

    /// The parameters of an entry, with options resolved; `chosen` holds
    /// the options chosen so far (the called shots depend on the target
    /// type).
    pub fn params(&self, entry: &ListEntry, chosen: &[usize]) -> Vec<Param> {
        entry
            .params
            .iter()
            .map(|p| Param {
                label: p.label.to_owned(),
                options: self.options(entry, p.options, chosen),
            })
            .collect()
    }

    fn options(&self, entry: &ListEntry, options: Options, chosen: &[usize]) -> Vec<String> {
        match options {
            Options::Fixed(list) => list.iter().map(|s| (*s).to_owned()).collect(),
            Options::Rows(id) => self
                .table(id)
                .map(|t| {
                    t.entries
                        .iter()
                        .map(|e| e.lines().first().copied().unwrap_or("").to_owned())
                        .collect()
                })
                .unwrap_or_default(),
            Options::Headers(ids, cut) => ids
                .iter()
                .map(|id| {
                    let title = self.table(*id).map_or("", |t| t.title.as_str());
                    title.split(cut).next().unwrap_or("").trim().to_owned()
                })
                .collect(),
            Options::CalledShot => {
                let mut out = vec!["Random location".to_owned()];
                if let Some(table) = self.location_table(entry, chosen) {
                    for e in &table.entries {
                        if let Some(name) = e.lines().first() {
                            let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
                            if !out.contains(&name) {
                                out.push(name);
                            }
                        }
                    }
                }
                out
            }
        }
    }

    fn location_table(&self, entry: &ListEntry, chosen: &[usize]) -> Option<&TableFile> {
        let crit = entry.steps.iter().find_map(|s| match s {
            Step::Critical(c) => Some(*c),
            _ => None,
        })?;
        let (id, _) = crit.targets.get(chosen.first().copied().unwrap_or(0))?;
        self.table(*id)
    }

    /// Rolls an entry of the list with the chosen parameter options.
    pub fn roll(
        &self,
        entry: &ListEntry,
        params: &[usize],
        session: &mut Session,
        rand: &mut Rand,
    ) -> RollResult {
        let mut run = Run {
            rolls: self,
            entry: Some(entry),
            book: entry.book,
            params,
            session,
            rand,
            out: RollResult::default(),
            depth: 0,
        };
        if entry.steps.is_empty() {
            let number = entry.number.unwrap_or(0);
            run.out.book_table = Some((entry.book, number));
            run.out.blocks.push(Block {
                title: Some(self.entry_title(entry)),
                notes: vec![format!(
                    "The original program works this table out in its own code, \
                     not from the Tables folder. See it in {}.",
                    entry.book.title()
                )],
                ..Default::default()
            });
            return run.out;
        }
        run.steps(entry.steps);
        run.out
    }

    /// Rolls on a table by id, as a follow-up roll would (`cr table`).
    pub fn roll_id(&self, id: i32, session: &mut Session, rand: &mut Rand) -> RollResult {
        let book = data::book_of(id);
        let mut run = Run {
            rolls: self,
            entry: None,
            book,
            params: &[],
            session,
            rand,
            out: RollResult::default(),
            depth: 0,
        };
        run.follow(id, None);
        run.out
    }
}

/// One roll in progress.
struct Run<'a> {
    rolls: &'a TableRolls,
    entry: Option<&'a ListEntry>,
    book: TableBook,
    params: &'a [usize],
    session: &'a mut Session,
    rand: &'a mut Rand,
    out: RollResult,
    depth: u32,
}

/// Deepest chain of follow-up tables.
const MAX_DEPTH: u32 = 8;

impl Run<'_> {
    fn param(&self, index: usize) -> usize {
        self.params.get(index).copied().unwrap_or(0)
    }

    fn steps(&mut self, steps: &[Step]) {
        for step in steps {
            self.step(step);
        }
    }

    fn step(&mut self, step: &Step) {
        match *step {
            Step::Roll(spec) => self.roll(&spec, None),
            Step::Choose { die, branches } => {
                let r = range(self.rand, 1, die);
                if let Some(b) = branches.iter().find(|b| b.hi >= r) {
                    let first = self.out.blocks.len();
                    self.steps(b.steps);
                    if let Some(block) = self.out.blocks.get_mut(first) {
                        block.subtitle = Some(match block.subtitle.take() {
                            Some(inner) => format!("{}, {inner}", b.label),
                            None => b.label.to_owned(),
                        });
                    }
                }
            }
            Step::ByParam { param, options } => {
                if let Some(steps) = options.get(self.param(param)) {
                    self.steps(steps);
                }
            }
            Step::Lookup {
                ids,
                id_param,
                row_param,
                dice,
                title,
                label,
                unit,
            } => {
                let row = self.param(row_param);
                let id = ids.get(self.param(id_param)).copied().unwrap_or(ids[0]);
                let Some(table) = self.rolls.table(id) else {
                    return self.missing(id, title);
                };
                let Some(record) = table.entries.get(row) else {
                    return;
                };
                let dice = dice.get(row).copied().unwrap_or(d(1, 6));
                let roll = self.session.roll(self.rand, dice);
                let name = record.lines().first().copied().unwrap_or("").to_owned();
                let title = self.title(title, Some(table));
                self.out.blocks.push(Block {
                    title,
                    header: vec!["Roll".into(), format!("{label} {name}")],
                    rows: vec![vec![
                        roll.to_string(),
                        format!(
                            "{} + {} ({}) = {} {unit}",
                            record.v3,
                            roll,
                            dice.text(),
                            record.v3 + roll
                        ),
                    ]],
                    ..Default::default()
                });
            }
            Step::Formula {
                param,
                dice,
                label,
                unit,
            } => {
                let dice = dice.get(self.param(param)).copied().unwrap_or(d(1, 6));
                let base = Dice { plus: 0, ..dice };
                let roll = self.session.roll(self.rand, base);
                let row = vec![
                    roll.to_string(),
                    format!(
                        "{} + {} ({}) = {} {unit}",
                        dice.plus,
                        roll,
                        base.text(),
                        dice.plus + roll
                    ),
                ];
                self.out.blocks.push(Block {
                    header: vec!["Roll".into(), label.into()],
                    rows: vec![row],
                    ..Default::default()
                });
            }
            Step::Insanity { check } => self.insanity(check),
            Step::Critical(crit) => self.critical(crit),
        }
    }

    fn title(&self, title: Title, table: Option<&TableFile>) -> Option<String> {
        match title {
            Title::Table(n) => Some(self.rolls.table_title(self.book, n)),
            Title::Entry => self.entry.map(|e| self.rolls.entry_title(e)),
            Title::Header => table
                .map(|t| t.title.trim().to_owned())
                .filter(|t| !t.is_empty() && t != "Item0"),
            Title::Label(l) => Some(l.to_owned()),
            Title::None => None,
        }
    }

    fn missing(&mut self, id: i32, title: Title) {
        let title = self.title(title, None);
        self.out.blocks.push(Block {
            title,
            notes: vec![format!(
                "The table for id {id} is not in the Tables folder."
            )],
            ..Default::default()
        });
    }

    /// Rolls on a table and prints the entry, then follows it.
    fn roll(&mut self, spec: &RollSpec, fixed: Option<i32>) {
        let Some(table) = self.rolls.table(spec.id) else {
            return self.missing(spec.id, spec.title);
        };
        let dice = spec.dice.unwrap_or_else(|| d(1, table.max_roll().max(1)));
        let times = match spec.rolls {
            Some(n) => {
                let mut sum = 0;
                for _ in 0..n.count {
                    sum += range(self.rand, 1, n.sides);
                }
                (sum + n.plus).max(0)
            }
            None => 1,
        };
        let mut header = vec!["Roll".to_owned()];
        header.extend(spec.cols.iter().map(|c| match c {
            Col::Name(l)
            | Col::Item1(l)
            | Col::Item2(l)
            | Col::V3(l)
            | Col::SignedV4(l)
            | Col::Art(l) => (*l).to_owned(),
            Col::Xp => "XP Value".to_owned(),
        }));
        let block = Block {
            title: self.title(spec.title, Some(table)),
            subtitle: spec.subtitle.map(str::to_owned),
            header,
            ..Default::default()
        };
        self.out.blocks.push(block);
        let index = self.out.blocks.len() - 1;
        if spec.rolls.is_some() {
            self.out.blocks[index].notes.push(format!("{times} rolls"));
        }
        let mut follows = Vec::new();
        for _ in 0..times {
            let roll = fixed.unwrap_or_else(|| self.session.roll(self.rand, dice));
            let Some(record) = table.lookup(roll) else {
                self.out.blocks[index]
                    .rows
                    .push(vec![roll.to_string(), String::new()]);
                continue;
            };
            let mut row = vec![roll.to_string()];
            for col in spec.cols {
                row.push(self.cell(col, record));
            }
            self.out.blocks[index].rows.push(row);
            if let Extra::Scroll(wizard) = spec.extra {
                self.scroll(index, record, wizard);
            }
            if spec.follow {
                follows.push((record.clone(), spec.id));
            }
        }
        for (record, from) in follows {
            self.follow_record(&record, from);
        }
    }

    fn cell(&mut self, col: &Col, r: &TableRecord) -> String {
        match col {
            Col::Name(_) => r.lines().join("\n"),
            Col::Item1(_) => r.item1.clone().unwrap_or_default(),
            Col::Item2(_) => r.item2.clone().unwrap_or_default(),
            Col::Xp => {
                if r.v3 > 0 {
                    r.v3.to_string()
                } else {
                    String::new()
                }
            }
            Col::V3(_) => r.v3.to_string(),
            Col::SignedV4(_) => format!("{:+}", r.v4),
            Col::Art(_) => {
                let mut value = 0i32;
                for _ in 0..r.v3.max(0) {
                    value = value.wrapping_add(range(self.rand, 1, r.v4));
                }
                format!(
                    "{} gp ({}d{} x {})",
                    thousands(value.wrapping_mul(r.v5)),
                    r.v3,
                    r.v4,
                    r.v5
                )
            }
        }
    }

    /// The spells of a scroll: the count from the name, each level from v3
    /// to v4.
    fn scroll(&mut self, index: usize, record: &TableRecord, wizard: bool) {
        let count = record
            .name
            .as_deref()
            .map_or(0, |n| {
                n.trim()
                    .split(' ')
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0)
            })
            .min(10);
        if count == 0 {
            return;
        }
        let spells = if wizard {
            &self.rolls.wizard_spells
        } else {
            &self.rolls.priest_spells
        };
        let block = &mut self.out.blocks[index];
        block.notes.push(format!(
            "{count} {} spell{}:",
            if wizard { "wizard" } else { "priest" },
            if count == 1 { "" } else { "s" }
        ));
        for _ in 0..count {
            let level = range(self.rand, record.v3, record.v4);
            let of_level: Vec<&str> = spells
                .iter()
                .filter(|(l, _)| *l == level)
                .map(|(_, n)| n.as_str())
                .collect();
            let name = if of_level.is_empty() {
                String::new()
            } else {
                of_level[(self.rand.rand() as usize) % of_level.len()].to_owned()
            };
            self.out.blocks[index]
                .notes
                .push(format!("  Level {level}  {name}"));
        }
    }

    /// Follows the table ids an entry names in v3, v4 and v5.
    fn follow_record(&mut self, record: &TableRecord, from: i32) {
        for value in record.values() {
            if value < 10000 || !data::is_table_id(value) {
                continue;
            }
            if value == from {
                // "Roll twice on this table ignoring scores of N and up".
                let Some(table) = self.rolls.table(from) else {
                    continue;
                };
                let die = record.lo - 1;
                if die < 1 || self.depth >= MAX_DEPTH {
                    continue;
                }
                self.depth += 1;
                let spec = RollSpec::new(from, d(1, die), Title::None, data::cols_of(from));
                for _ in 0..2 {
                    self.roll(
                        &RollSpec {
                            follow: false,
                            ..spec
                        },
                        None,
                    );
                }
                self.depth -= 1;
                let _ = table;
                continue;
            }
            self.follow(value, Some(from));
        }
    }

    /// Rolls on table `id` with its handler, or a plain roll.
    fn follow(&mut self, id: i32, _from: Option<i32>) {
        if self.depth >= MAX_DEPTH {
            return;
        }
        self.depth += 1;
        match data::handler(id) {
            Some(steps) => self.steps(steps),
            None => self.roll(
                &RollSpec {
                    dice: None,
                    ..RollSpec::new(id, d(1, 1), Title::Header, data::cols_of(id))
                },
                None,
            ),
        }
        self.depth -= 1;
    }

    fn insanity(&mut self, check: bool) {
        let level = self.param(0);
        let Some(chances) = self.rolls.table(40023) else {
            return self.missing(40023, Title::Table(23));
        };
        let Some(row) = chances.entries.get(level).cloned() else {
            return;
        };
        if check {
            let roll = self.session.roll(self.rand, d(1, 100));
            let insane = roll <= row.v3;
            self.out.blocks.push(Block {
                title: Some(self.rolls.table_title(TableBook::SpellsMagic, 23)),
                header: vec!["Roll".into(), "Chance of Insanity".into(), "Result".into()],
                rows: vec![vec![
                    roll.to_string(),
                    format!("{}%", row.v3),
                    if insane { "Insane" } else { "Sane" }.into(),
                ]],
                ..Default::default()
            });
            if !insane {
                return;
            }
        }
        let Some(chart) = self.rolls.table(40024) else {
            return self.missing(40024, Title::Table(24));
        };
        let raw = self.session.roll(self.rand, d(1, 100));
        let modified = raw + row.v4;
        let name = chart
            .lookup(modified)
            .or(chart.entries.last())
            .map(|r| r.lines().join("\n"))
            .unwrap_or_default();
        self.out.blocks.push(Block {
            title: Some(self.rolls.table_title(TableBook::SpellsMagic, 24)),
            header: vec!["Roll".into(), "Modified".into(), "Result".into()],
            rows: vec![vec![raw.to_string(), modified.to_string(), name]],
            ..Default::default()
        });
    }

    fn critical(&mut self, crit: &Critical) {
        let (target, kind, size, severity, called) = (
            self.param(0),
            self.param(1),
            self.param(2),
            self.param(3),
            self.param(4),
        );
        let Some(&(location_id, offset)) = crit.targets.get(target) else {
            return;
        };
        let Some(locations) = self.rolls.table(location_id) else {
            return self.missing(location_id, Title::Label(crit.label));
        };
        let kind = crit.kinds.get(kind).copied().unwrap_or(crit.kinds[0]);
        let mut block = Block {
            title: Some(crit.label.to_owned()),
            ..Default::default()
        };
        let location = if called == 0 {
            let dice = crit.location_dice.get(size).copied().unwrap_or(d(1, 10));
            let roll = self.session.roll(self.rand, dice);
            let record = locations.lookup(roll).or(locations.entries.last());
            block.header = vec!["Roll".into(), "Strike Location".into()];
            let name = record.map(|r| r.lines().join("\n")).unwrap_or_default();
            block.rows.push(vec![roll.to_string(), name]);
            record.cloned()
        } else {
            // A called shot: the location named in the list.
            let names = self.rolls.options(
                self.entry.expect("critical hits come from the list"),
                Options::CalledShot,
                self.params,
            );
            let wanted = names.get(called).cloned().unwrap_or_default();
            let record = locations.entries.iter().find(|e| {
                e.lines()
                    .first()
                    .is_some_and(|n| n.split_whitespace().collect::<Vec<_>>().join(" ") == wanted)
            });
            block.header = vec!["Strike Location".into()];
            block.rows.push(vec![wanted]);
            record.cloned()
        };
        self.out.blocks.push(block);
        let Some(location) = location else {
            return;
        };
        let dice = crit.severity_dice.get(severity).copied().unwrap_or(d(1, 6));
        self.effect(location.v3 + kind + offset, dice, kind, offset, 0);
    }

    /// The effect of a critical hit on a location, and an extra location
    /// when the effect names one.
    fn effect(&mut self, id: i32, dice: Dice, kind: i32, offset: i32, depth: u32) {
        let Some(table) = self.rolls.table(id) else {
            return self.missing(id, Title::None);
        };
        let roll = self.session.roll(self.rand, dice);
        let record = table.lookup(roll).or(table.entries.last()).cloned();
        let text = record
            .as_ref()
            .map(|r| r.lines().join("\n"))
            .unwrap_or_default();
        self.out.blocks.push(Block {
            subtitle: Some(table.title.trim().to_owned()),
            header: vec!["Severity".into(), "Effect".into()],
            rows: vec![vec![roll.to_string(), text]],
            ..Default::default()
        });
        // "Additional abdomen hit": a location table (d100) whose entry
        // carries a location code.
        if let Some(r) = record
            && depth < 2
            && data::is_table_id(r.v3)
            && let Some(extra) = self.rolls.table(r.v3)
        {
            let roll = self.session.roll(self.rand, d(1, extra.max_roll().max(1)));
            if let Some(loc) = extra.lookup(roll).cloned() {
                self.out.blocks.push(Block {
                    header: vec!["Roll".into(), "Additional Location".into()],
                    rows: vec![vec![roll.to_string(), loc.lines().join("\n")]],
                    ..Default::default()
                });
                if loc.v3 >= 10000 {
                    self.effect(loc.v3 + kind + offset, dice, kind, offset, depth + 1);
                }
            }
        }
    }
}

fn thousands(n: i32) -> String {
    crate::treasure::thousands(n)
}

#[cfg(test)]
mod tests;
