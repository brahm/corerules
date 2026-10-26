//! The treasure table editor with no toolkit in it (IDD 258, with the
//! entry details of IDD 262 and their Scroll Details, IDD 365): a table's
//! name and entries, each an item or another table with its weight, the
//! roll range the weights give, and what an item's entry rolls with it.

use std::sync::Arc;

use cr_formats::inventory::InventoryItem;
use cr_formats::treasure::{TreasureTable, TreasureTableEntry};
use cr_i18n::{tr, trn};
use cr_store::Installation;
use cr_store::help::topic;
use cr_store::userdb::tables::{self, TableDb};
use cr_store::userdb::{self as udb, Origin};

use super::form::Saved;
use crate::prompt::{self, Answer};

/// A question or message the table editor waits on.
pub type Prompt = prompt::Prompt<TableForm>;

/// The section of the treasure tables.
pub const SECTION: &str = "treasure-tables";

/// The most an entry's weight or probability takes.
pub const MOST_WEIGHT: u32 = 100_000;
pub const MOST_PROBABILITY: u32 = 5000;
/// The most each of an entry's dice takes, as the original's fields
/// allow.
pub const MOST_DICE: u32 = 1000;
/// The most spells a scroll entry rolls.
pub const MOST_SPELLS: u32 = 100;

/// What the editor offers to add, read when it opens: the table names a
/// row's Random Contents may roll, and each source's items.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableSources {
    pub tables: Vec<String>,
    /// Each source ("Weapons, Custom", "Tables") with its items and
    /// whether they are tables.
    pub items: Vec<(String, Vec<String>, bool)>,
}

impl TableSources {
    pub fn load(install: &Installation, db: &TableDb) -> cr_store::Result<Self> {
        Ok(Self {
            tables: tables::table_names(db),
            items: tables::all_source_items(install, db)?,
        })
    }

    /// The sources Add Entry offers (every one), or Always Contains (all
    /// but "Tables").
    pub fn sources(&self, contents: bool) -> Vec<String> {
        self.items
            .iter()
            .filter(|(_, _, tables)| !(contents && *tables))
            .map(|(s, ..)| s.clone())
            .collect()
    }

    fn source(&self, contents: bool, index: usize) -> Option<&(String, Vec<String>, bool)> {
        self.items
            .iter()
            .filter(|(_, _, tables)| !(contents && *tables))
            .nth(index)
    }
}

/// A source and one of its items, as they are chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pick {
    pub source: Option<usize>,
    pub item: Option<usize>,
}

/// Treasure Table Entry Details for one row.
#[derive(Debug, Clone, PartialEq)]
pub struct Details {
    /// The row, by index among the entries.
    pub index: usize,
    pub entry: TreasureTableEntry,
    /// What the item always contains.
    pub contained: Vec<InventoryItem>,
    /// Always Contains' Add.
    pub pick: Pick,
    /// Remove's "How many to remove?" for the contained item with this
    /// index: the most, and the number chosen.
    pub removing: Option<(usize, u32, u32)>,
}

/// A change in the table editor, or one of its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum TableInput {
    Name(String),
    /// An entry's weight, by its index.
    Weight(usize, u32),
    /// Remove the entry with this index.
    Remove(usize),
    /// Add Entry: its source and item, by index.
    AddSource(usize),
    AddItem(usize),
    /// Add the item chosen.
    Add,
    /// Open the details of the entry with this index.
    Details(usize),
    Detail(DetailInput),
    /// OK (true) or Cancel on the details.
    CloseDetails(bool),
    Save,
}

/// A change in an entry's details.
#[derive(Debug, Clone, PartialEq)]
pub enum DetailInput {
    Probability(u32),
    /// One of the number found's dice: count (0), sides (1), bonus (2).
    Found(usize, u32),
    Intelligent(u32),
    /// The first (0) or second (1) Random Contents table, by index among
    /// [`TableForm::sub_table_choices`].
    SubTable(usize, usize),
    /// One of a Random Contents table's dice.
    SubDice(usize, usize, u32),
    /// Always Contains' Add: its source and item, by index.
    Source(usize),
    Item(usize),
    /// Add the item chosen.
    Add,
    /// Remove the contained item with this index (asking how many when
    /// there are several).
    Remove(usize),
    /// How many to remove, and Remove (true) or Cancel on that.
    RemoveCount(u32),
    ConfirmRemove(bool),
    /// Scroll Details: wizard (true) or priest spells, the lowest and
    /// highest level, and how many.
    Wizard(bool),
    Lowest(u32),
    Highest(u32),
    Spells(u32),
}

/// What the app should do for the table editor.
#[derive(Clone)]
pub enum TableEffect {
    /// Save `table` in place of the one with index `editing`, or as a new
    /// one, and pass what it gives to [`TableForm::saved`].
    Save {
        editing: Option<usize>,
        table: Box<TreasureTable>,
    },
    /// Saved: leave the editor for the entry with `key`, saying `said`.
    Done { key: String, said: String },
}

/// The treasure table editor.
#[derive(Clone)]
pub struct TableForm {
    pub table: TreasureTable,
    kept: TreasureTable,
    /// The table's index in the database, when one is changed rather than
    /// added.
    editing: Option<usize>,
    title: String,
    sources: Arc<TableSources>,
    /// Add Entry's choice.
    pub pick: Pick,
    /// The details being edited.
    pub details: Option<Details>,
    prompt: Option<Prompt>,
    serial: u64,
    effects: Vec<TableEffect>,
    pub error: Option<String>,
    pub saving: bool,
}

impl TableForm {
    /// The editor of `table`, the database's table with index `editing`
    /// when it is changed rather than added.
    pub fn new(table: TreasureTable, editing: Option<usize>, sources: Arc<TableSources>) -> Self {
        let title = match editing {
            Some(_) => table.name.clone(),
            None => tr("New Treasure Table").to_owned(),
        };
        Self {
            kept: table.clone(),
            table,
            editing,
            title,
            sources,
            pick: Pick::default(),
            details: None,
            prompt: None,
            serial: 0,
            effects: Vec::new(),
            error: None,
            saving: false,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    /// The help topic of the page in view.
    pub fn help(&self) -> u32 {
        if self.details.is_some() {
            topic::TREASURE_ENTRY
        } else {
            topic::TREASURE_TABLE
        }
    }

    pub fn sources(&self) -> &TableSources {
        &self.sources
    }

    /// The items of the source chosen in `pick`, for Add Entry (or, with
    /// `contents`, Always Contains).
    pub fn items(&self, pick: Pick, contents: bool) -> &[String] {
        pick.source
            .and_then(|s| self.sources.source(contents, s))
            .map_or(&[], |(_, items, _)| items.as_slice())
    }

    /// The rows: each entry's line (a table is named so), its roll range
    /// and weight, and the line its details give (none for a table).
    pub fn rows(&self) -> Vec<Row> {
        let ranges = tables::ranges(&self.table);
        tables::entries(&self.table)
            .into_iter()
            .zip(ranges)
            .map(|(entry, range)| Row {
                title: if entry.is_table != 0 {
                    tr!("Table: {name}", name = entry.item.name.clone())
                } else {
                    entry.item.name.clone()
                },
                range: tr!("Roll {range}", range),
                weight: entry.weight,
                details: (entry.is_table == 0).then(|| details_summary(entry)),
            })
            .collect()
    }

    /// The tables a Random Contents box offers, "None" first, and the one
    /// chosen; a table that is gone stays listed, so that it is not lost.
    pub fn sub_table_choices(&self, k: usize) -> (Vec<String>, usize) {
        let mut choices = vec![tr("None").to_owned()];
        choices.extend(self.sources.tables.iter().cloned());
        let Some(details) = &self.details else {
            return (choices, 0);
        };
        let name = sub_table(&details.entry, k).0;
        if name.is_empty() {
            return (choices, 0);
        }
        match self
            .sources
            .tables
            .iter()
            .position(|n| n.eq_ignore_ascii_case(name))
        {
            Some(p) => (choices, p + 1),
            None => {
                choices.push(name.to_owned());
                let last = choices.len() - 1;
                (choices, last)
            }
        }
    }

    /// Whether anything was changed.
    pub fn changed(&self) -> bool {
        self.table != self.kept
    }

    pub fn can_save(&self) -> bool {
        !self.saving && self.prompt.is_none() && self.details.is_none()
    }

    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    pub fn prompt_serial(&self) -> u64 {
        self.serial
    }

    pub fn take_effects(&mut self) -> Vec<TableEffect> {
        std::mem::take(&mut self.effects)
    }

    pub fn answer(&mut self, answer: Answer) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        if let Some(then) = prompt.then(answer) {
            then(self);
        }
    }

    fn show(&mut self, prompt: Prompt) {
        self.prompt = Some(prompt);
        self.serial += 1;
    }

    pub fn input(&mut self, input: TableInput) {
        if self.prompt.is_some() || self.saving {
            return;
        }
        match input {
            TableInput::Name(name) => self.table.name = name,
            TableInput::Weight(i, weight) => {
                if let Some(e) = tables::entries_mut(&mut self.table).into_iter().nth(i) {
                    e.weight = weight.min(MOST_WEIGHT);
                }
            }
            TableInput::Remove(i) => {
                let mut seen = 0;
                self.table.entries.retain(|o| {
                    !matches!(o, cr_formats::Object::TreasureTableEntry(_)) || {
                        seen += 1;
                        seen - 1 != i
                    }
                });
            }
            TableInput::AddSource(s) => {
                self.pick = Pick {
                    source: Some(s),
                    item: None,
                }
            }
            TableInput::AddItem(i) => self.pick.item = Some(i),
            TableInput::Add => {
                let Some((_, items, is_table)) =
                    self.pick.source.and_then(|s| self.sources.source(false, s))
                else {
                    return;
                };
                if let Some(item) = self.pick.item.and_then(|i| items.get(i)) {
                    tables::push_entry(&mut self.table, tables::new_entry(item, *is_table, 1));
                    self.pick.item = None;
                }
            }
            TableInput::Details(i) => {
                if let Some(entry) = tables::entries(&self.table).get(i)
                    && entry.is_table == 0
                {
                    self.details = Some(Details {
                        index: i,
                        contained: tables::contained(entry).to_vec(),
                        entry: (*entry).clone(),
                        pick: Pick::default(),
                        removing: None,
                    });
                }
            }
            TableInput::Detail(change) => self.detail(change),
            TableInput::CloseDetails(ok) => {
                let Some(details) = self.details.take() else {
                    return;
                };
                if ok {
                    let mut entry = details.entry;
                    tables::set_contained(&mut entry, details.contained);
                    if let Some(row) = tables::entries_mut(&mut self.table)
                        .into_iter()
                        .nth(details.index)
                    {
                        *row = entry;
                    }
                }
            }
            TableInput::Save => {
                if self.details.is_none() {
                    self.saving = true;
                    self.error = None;
                    self.effects.push(TableEffect::Save {
                        editing: self.editing,
                        table: Box::new(self.table.clone()),
                    });
                }
            }
        }
    }

    fn detail(&mut self, change: DetailInput) {
        let sources = self.sources.clone();
        let Some(d) = &mut self.details else {
            return;
        };
        let e = &mut d.entry;
        match change {
            DetailInput::Probability(v) => e.weight = v.min(MOST_PROBABILITY),
            DetailInput::Found(k, v) => {
                if let Some(slot) = e.quantity_dice.get_mut(k) {
                    *slot = v.min(MOST_DICE);
                }
            }
            DetailInput::Intelligent(v) => e.intelligent_chance = v.min(100) as u8,
            DetailInput::SubTable(k, choice) => {
                let name = if choice == 0 {
                    String::new()
                } else {
                    match sources.tables.get(choice - 1) {
                        Some(n) => n.clone(),
                        // The table that is gone, listed last.
                        None => sub_table(e, k).0.to_owned(),
                    }
                };
                if k == 0 {
                    e.sub_table_1 = name;
                } else {
                    e.sub_table_2 = name;
                }
            }
            DetailInput::SubDice(k, n, v) => {
                let dice = if k == 0 {
                    &mut e.sub_table_1_dice
                } else {
                    &mut e.sub_table_2_dice
                };
                if let Some(slot) = dice.get_mut(n) {
                    *slot = v.min(MOST_DICE);
                }
            }
            DetailInput::Source(s) => {
                d.pick = Pick {
                    source: Some(s),
                    item: None,
                }
            }
            DetailInput::Item(i) => d.pick.item = Some(i),
            DetailInput::Add => {
                let item = d
                    .pick
                    .source
                    .and_then(|s| sources.source(true, s))
                    .and_then(|(_, items, _)| d.pick.item.and_then(|i| items.get(i)));
                if let Some(item) = item {
                    tables::add_contained(&mut d.contained, item);
                }
            }
            DetailInput::Remove(i) => match d.contained.get(i).map(|c| c.quantity) {
                Some(quantity) if quantity > 1 => d.removing = Some((i, quantity, quantity)),
                Some(_) => tables::remove_contained(&mut d.contained, i, 1),
                None => {}
            },
            DetailInput::RemoveCount(n) => {
                if let Some((_, most, count)) = &mut d.removing {
                    *count = n.min(*most);
                }
            }
            DetailInput::ConfirmRemove(yes) => {
                if let Some((i, _, count)) = d.removing.take()
                    && yes
                    && count > 0
                {
                    tables::remove_contained(&mut d.contained, i, count);
                }
            }
            DetailInput::Wizard(wizard) => tables::set_spell_wizard(e, wizard),
            DetailInput::Lowest(v) => {
                e.spell_min_level = v.min(tables::max_spell_level(e.spell_wizard != 0))
            }
            DetailInput::Highest(v) => {
                e.spell_max_level = v.min(tables::max_spell_level(e.spell_wizard != 0))
            }
            DetailInput::Spells(v) => e.spell_count = v.min(MOST_SPELLS),
        }
    }

    /// What saving did: done, the editor leaves for the table; a refused
    /// name is said and the editor stays.
    pub fn saved(&mut self, result: Result<Saved, String>) {
        self.saving = false;
        match result {
            Ok(Saved::Done { name, key }) => self.effects.push(TableEffect::Done {
                key,
                said: tr!("“{name}” saved", name),
            }),
            Ok(Saved::Refused { title, text }) => self.show(Prompt::message(&title, text)),
            Err(why) => self.error = Some(why),
        }
    }
}

/// An entry as the editor lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub title: String,
    /// "Roll 4-6".
    pub range: String,
    pub weight: u32,
    /// What its details set, in a line; none for a table.
    pub details: Option<String>,
}

/// A Random Contents table of an entry: its name and dice.
pub fn sub_table(entry: &TreasureTableEntry, k: usize) -> (&str, [u32; 3]) {
    if k == 0 {
        (&entry.sub_table_1, entry.sub_table_1_dice)
    } else {
        (&entry.sub_table_2, entry.sub_table_2_dice)
    }
}

/// Saves `table` in place of the database's table with index `editing`,
/// or as a new one.
pub fn save(
    install: &Installation,
    editing: Option<usize>,
    table: TreasureTable,
) -> Result<Saved, String> {
    let mut db = TableDb::load(install).map_err(|e| e.to_string())?;
    let stored = match editing {
        Some(i) if i < db.tables.len() => db.replace(i, table).map(|()| i),
        _ => db.add(table),
    };
    match stored {
        Ok(i) => {
            db.save(install).map_err(|e| e.to_string())?;
            let t = &db.tables[i];
            Ok(Saved::Done {
                key: udb::key(SECTION, tables::origin(t), &t.name),
                name: t.name.clone(),
            })
        }
        Err(text) => Ok(Saved::Refused {
            title: tr("Not Saved").to_owned(),
            text: tr(&text).to_owned(),
        }),
    }
}

/// What the details of a row set, in a line: "1d4+2 found; 25%
/// intelligent; …".
pub fn details_summary(entry: &TreasureTableEntry) -> String {
    let mut parts = vec![tr!("{dice} found", dice = dice_text(entry.quantity_dice))];
    if entry.intelligent_chance != 0 {
        parts.push(tr!(
            "{percent}% intelligent",
            percent = entry.intelligent_chance
        ));
    }
    for k in 0..2 {
        let (name, dice) = sub_table(entry, k);
        if !name.is_empty() {
            parts.push(tr!(
                "{dice} on {table}",
                dice = dice_text(dice),
                table = name
            ));
        }
    }
    let contained = tables::contained(entry);
    if !contained.is_empty() {
        let names: Vec<String> = contained.iter().map(tables::contained_line).collect();
        parts.push(tr!("contains {items}", items = names.join(", ")));
    }
    if entry.spell_count != 0 {
        parts.push(trn!("{n} spell", "{n} spells", entry.spell_count as usize));
    }
    parts.join("; ")
}

/// Dice as the details write them: "1d4+2", "1" for 0d0+1.
pub fn dice_text([count, sides, bonus]: [u32; 3]) -> String {
    if count == 0 || sides == 0 {
        return bonus.to_string();
    }
    if bonus == 0 {
        format!("{count}d{sides}")
    } else {
        format!("{count}d{sides}+{bonus}")
    }
}

/// Delete's question on a custom table: its title and text.
pub fn delete_question() -> (String, String) {
    (
        tr("Are you sure?").to_owned(),
        tr(tables::DELETE_WARNING).to_owned(),
    )
}

/// Restore's question on a standard table: its title and text.
pub fn restore_question() -> (String, String) {
    (
        tr("Restore Table?").to_owned(),
        tr(tables::RESTORE_WARNING).to_owned(),
    )
}

/// Restores the standard table `name` as shipped; what to say.
pub fn restore(install: &Installation, name: &str) -> Result<String, String> {
    let mut db = TableDb::load(install).map_err(|e| e.to_string())?;
    if let Some(i) = db.position(name) {
        db.restore(i).map_err(|e| tr(&e).to_owned())?;
    }
    db.save(install).map_err(|e| e.to_string())?;
    Ok(tr!("“{name}” restored", name = name.to_owned()))
}

/// The key of the standard table `name`, which Restore shows.
pub fn restored_key(name: &str) -> String {
    udb::key(SECTION, Origin::Standard, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> TableForm {
        let mut table = TableDb::new_table();
        table.name = "Udb table".into();
        tables::push_entry(&mut table, tables::new_entry("Bag", false, 3));
        tables::push_entry(&mut table, tables::new_entry("DMG 88", true, 1));
        let sources = TableSources {
            tables: vec!["DMG 88".into()],
            items: vec![
                (
                    "Equipment, All".into(),
                    vec!["Rope".into(), "Torch".into()],
                    false,
                ),
                ("Tables".into(), vec!["DMG 88".into()], true),
            ],
        };
        TableForm::new(table, None, Arc::new(sources))
    }

    #[test]
    fn rows_show_ranges_and_details_only_for_items() {
        let form = form();
        let rows = form.rows();
        assert_eq!(rows[0].range, "Roll 1-3");
        assert_eq!(rows[1].title, "Table: DMG 88");
        assert!(rows[0].details.is_some() && rows[1].details.is_none());
        assert_eq!(dice_text([0, 0, 1]), "1");
        assert_eq!(dice_text([1, 4, 2]), "1d4+2");
    }

    #[test]
    fn details_add_and_remove_contents_and_ok_keeps_them() {
        let mut form = form();
        form.input(TableInput::Details(0));
        for _ in 0..3 {
            form.input(TableInput::Detail(DetailInput::Source(0)));
            form.input(TableInput::Detail(DetailInput::Item(1)));
            form.input(TableInput::Detail(DetailInput::Add));
        }
        form.input(TableInput::Detail(DetailInput::Remove(0)));
        form.input(TableInput::Detail(DetailInput::RemoveCount(2)));
        form.input(TableInput::Detail(DetailInput::ConfirmRemove(true)));
        form.input(TableInput::CloseDetails(true));
        let entry = tables::entries(&form.table)[0].clone();
        assert_eq!(tables::contained(&entry)[0].name, "Torch");
        assert_eq!(tables::contained(&entry)[0].quantity, 1);
        assert!(form.changed());
    }

    #[test]
    fn add_entry_takes_a_table_from_the_tables_source() {
        let mut form = form();
        form.input(TableInput::AddSource(1));
        form.input(TableInput::AddItem(0));
        form.input(TableInput::Add);
        let entries = tables::entries(&form.table);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[2].is_table, 1);
        // Always Contains offers no tables.
        assert_eq!(form.sources().sources(true), vec!["Equipment, All"]);
    }
}
