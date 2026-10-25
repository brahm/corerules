//! Encounters with no toolkit in them: New Encounter's templates,
//! generating from a template or a 2-20 table, the Encounter page
//! (monsters, NPCs, treasure, items, traps and totals) and the 2-20
//! table editor. What reads or writes the installation is left to the
//! app, as effects or as what it hands in.

use std::sync::Arc;

use cr_formats::character::Character;
use cr_i18n::{tr, tr_noop, trn};
use cr_store::encounter::{
    Encounter, EncounterRules, FrequencyTable, LairMode, PICK_ROLLS, Package, PendingNpc, Source,
    add_trap, hit_point_text, remove_trap, roll_2_to_20, roll_and_pick,
};
use cr_store::sheet::Sheet;
use cr_store::treasure::{Bundle, Counts, Rand, TreasureRules, thousands};
use cr_store::userdb::traps::Traps;
use cr_store::userdb::{Filter, Origin, UserDb};

use crate::bundle::{BundleEditor, BundleInput};

/// Climate flags 0-4 of templates and tables.
pub const CLIMATES: [&str; 5] = [
    tr_noop!("Arctic"),
    tr_noop!("Sub-Arctic"),
    tr_noop!("Temperate"),
    tr_noop!("Sub-Tropical"),
    tr_noop!("Tropical"),
];

/// Terrain flags 5-36 (table 0x67f3cc).
pub const TERRAINS: [&str; 32] = [
    tr_noop!("Aerial"),
    tr_noop!("Bush"),
    tr_noop!("Caves"),
    tr_noop!("Coastal"),
    tr_noop!("Deserts"),
    tr_noop!("Farmland"),
    tr_noop!("Forests"),
    tr_noop!("Glaciers"),
    tr_noop!("Grasslands"),
    tr_noop!("Jungles"),
    tr_noop!("Fresh-Water Lakes"),
    tr_noop!("Salt-Water Lakes"),
    tr_noop!("High Mountains"),
    tr_noop!("High Mountain Forests"),
    tr_noop!("High Mountain Lakes"),
    tr_noop!("Low Mountains"),
    tr_noop!("Low Mountain Forests"),
    tr_noop!("Low Mountain Lakes"),
    tr_noop!("Deep Oceans"),
    tr_noop!("Shallow Oceans"),
    tr_noop!("Parkland"),
    tr_noop!("Scrub Plains"),
    tr_noop!("Rain Forests"),
    tr_noop!("Fresh-Water Rivers"),
    tr_noop!("Salt-Water Rivers"),
    tr_noop!("Rough/Hills"),
    tr_noop!("Salt Marshes"),
    tr_noop!("Steppes"),
    tr_noop!("Subterranean"),
    tr_noop!("Fresh-Water Swamps"),
    tr_noop!("Salt-Water Swamps"),
    tr_noop!("Tundras"),
];

/// The frequency categories of a roll's list (0x6559bd).
pub const FREQUENCIES: [&str; 5] = [
    tr_noop!("Common"),
    tr_noop!("Uncommon"),
    tr_noop!("Rare"),
    tr_noop!("Very Rare"),
    tr_noop!("Never"),
];

/// The choices of a table's Database.
pub const SOURCES: [&str; 3] = [tr_noop!("All"), tr_noop!("Standard"), tr_noop!("Custom")];

/// The choices of a table's Lair and/or Wandering.
pub const LAIR_MODES: [&str; 4] = [
    tr_noop!("Lair and Wandering"),
    tr_noop!("Lair or Wandering"),
    tr_noop!("Wandering"),
    tr_noop!("Lair"),
];

/// The most of a package the Number takes (Edit…, 0x63dba3).
pub const MOST_COUNT: i32 = 100_000_000;

/// New Encounter's templates (the original's "Select an Encounter
/// Template"): each name with its monster type, sorted by name.
pub fn templates(rules: &EncounterRules) -> Vec<(String, String)> {
    let mut names: Vec<(String, String)> = rules
        .templates()
        .iter()
        .map(|t| (t.name.clone(), t.monster_type.clone()))
        .collect();
    names.sort_by_cached_key(|(n, _)| cr_store::collate::sort_key(n));
    names
}

/// Whether a template, by its name and monster type, has `search` in
/// either, ignoring case.
pub fn template_matches(name: &str, kind: &str, search: &str) -> bool {
    let needle = search.trim().to_lowercase();
    needle.is_empty() || format!("{name}\n{kind}").to_lowercase().contains(&needle)
}

/// An encounter made from a template.
pub struct Made {
    pub encounter: Encounter,
    /// The NPCs the NPC generator could not make.
    pub pending: Vec<PendingNpc>,
    /// What the original's message boxes say.
    pub messages: Vec<String>,
}

/// An encounter generated from the template `name`, or, as the original
/// does when it cannot, an empty one with why.
pub fn from_template(rules: &EncounterRules, name: &str, rand: &mut Rand) -> Made {
    match rules.generate(name, rand) {
        Ok(g) => Made {
            encounter: g.encounter,
            pending: g.pending_npcs,
            messages: g.messages,
        },
        // As the original: "Creating an Empty Encounter".
        Err(e) => Made {
            encounter: Encounter::default(),
            pending: Vec::new(),
            messages: vec![tr!("{error}. Creating an empty encounter.", error = e)],
        },
    }
}

/// Roll and Generate on a saved 2-20 table: the template of a uniformly
/// drawn filled roll, with what to say of the roll, or why there is none.
pub fn roll_and_generate(
    rules: &EncounterRules,
    table: &FrequencyTable,
    rand: &mut Rand,
) -> Result<(String, String), String> {
    match roll_and_pick(rules, table, rand) {
        Some(i) => {
            let template = table.slots[i].clone();
            let said = tr!(
                "Rolled {roll}: {template}",
                roll = i + 2,
                template = template
            );
            Ok((template, said))
        }
        None => Err(tr("The table has no templates; fill it first").to_owned()),
    }
}

/// The Hit Points group of a monster package (the original's "Monsters
/// for the Encounter", IDD 292): a value, how many monsters have it, and
/// how many have none yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitPoints {
    pub low: i32,
    pub high: i32,
    pub value: i32,
    /// How many monsters have the value, and the most that may.
    pub with: i32,
    pub most_with: i32,
    pub unassigned: i32,
}

impl HitPoints {
    /// What the Monsters box says under its title.
    pub fn note(&self) -> String {
        tr!(
            "With these hit points; {count} unassigned",
            count = self.unassigned.max(0)
        )
    }
}

/// A package as the page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRow {
    pub package: Package,
    pub title: String,
    pub subtitle: String,
    /// The stat line.
    pub stats: String,
    pub count: i32,
    /// A monster package's hit points.
    pub hit_points: Option<HitPoints>,
}

/// What a picker on the Encounter page adds.
#[derive(Clone)]
pub enum Pick {
    Monster,
    /// A saved character; the files of the names, in their order.
    Npc(Vec<String>),
    /// A trap of the databases, shown by origin.
    Trap(Arc<UserDb<Traps>>, Filter),
}

/// Add Monster, Add NPC or Add Trap: a searchable list of names.
#[derive(Clone)]
pub struct Picker {
    pub pick: Pick,
    /// The names with what is shown under them.
    pub items: Vec<(String, String)>,
    pub search: String,
    /// The name chosen.
    pub chosen: Option<String>,
}

impl Picker {
    fn new(pick: Pick, items: Vec<(String, String)>) -> Self {
        Self {
            pick,
            items,
            search: String::new(),
            chosen: None,
        }
    }

    pub fn title(&self) -> &'static str {
        match self.pick {
            Pick::Monster => tr("Add Monster"),
            Pick::Npc(_) => tr("Add NPC"),
            Pick::Trap(..) => tr("Add Trap"),
        }
    }

    pub fn placeholder(&self) -> &'static str {
        match self.pick {
            Pick::Monster => tr("Search Monsters"),
            Pick::Npc(_) => tr("Search Characters"),
            Pick::Trap(..) => tr("Search Traps"),
        }
    }

    /// The trap list's origin filter, by its place in [`Filter::NAMES`].
    pub fn filter(&self) -> Option<usize> {
        match &self.pick {
            Pick::Trap(_, filter) => Some(match filter {
                Filter::All => 0,
                Filter::Standard => 1,
                Filter::Custom => 2,
            }),
            _ => None,
        }
    }

    /// The items the search lets through, by index.
    pub fn shown(&self) -> Vec<usize> {
        let needle = self.search.trim().to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, (name, _))| name.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect()
    }
}

/// The traps of `db` that `filter` lets through, each with its damage.
fn trap_items(db: &UserDb<Traps>, filter: Filter) -> Vec<(String, String)> {
    db.list(filter)
        .into_iter()
        .map(|e| {
            let custom = if e.origin == Origin::Custom {
                tr("Custom")
            } else {
                ""
            };
            let damage = e.record.damage.trim();
            let under = match (custom.is_empty(), damage.is_empty()) {
                (true, _) => damage.to_owned(),
                (false, true) => custom.to_owned(),
                (false, false) => format!("{custom} · {damage}"),
            };
            (e.record.name.clone(), under)
        })
        .collect()
}

/// A change on the Encounter page, or one of its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum EncounterInput {
    Name(String),
    MonsterType(String),
    Description(String),
    Remove(Package),
    /// A new number for a package; a monster package rolls its hit
    /// points again.
    Count(Package, i32),
    /// The value the Hit Points box of the monster package with this
    /// index shows.
    HitPoints(usize, i32),
    /// How many monsters of the package have the value shown.
    With(usize, i32),
    /// Re-roll: the hit points again, except those set by hand.
    Reroll(usize),
    Items(BundleInput),
    /// The answer to the items' dialog with this number.
    ItemsAnswer(u64, bool),
    /// Use Treasure on the roller.
    Treasure(Bundle),
    AddMonster,
    PickSearch(String),
    /// The trap list's origin filter, by its place in [`Filter::NAMES`].
    PickFilter(usize),
    PickChoose(String),
    /// The picker answered: add, or cancel.
    PickAnswer(bool),
    RemoveTrap(String),
    Save,
}

/// What the page asks of the app.
#[derive(Debug, Clone, PartialEq)]
pub enum EncounterEffect {
    /// Read the saved character in this file, for [`EncounterForm::add_npc`].
    AddNpc(String),
    /// Save `encounter` in place of the saved encounter `old`.
    Save {
        old: Option<String>,
        encounter: Box<Encounter>,
    },
}

/// The Encounter page.
#[derive(Clone)]
pub struct EncounterForm {
    rules: Arc<EncounterRules>,
    treasure_rules: Arc<TreasureRules>,
    pub encounter: Encounter,
    /// The name the encounter is saved under.
    pub saved: Option<String>,
    pub pending: Vec<PendingNpc>,
    /// The value each monster package's Hit Points box shows.
    shown: Vec<i32>,
    /// Editing the items.
    pub items: BundleEditor,
    pub picker: Option<Picker>,
    /// What the page says, as the original's message boxes: a title and
    /// a text.
    pub notice: Option<(String, String)>,
    /// A save under way.
    pub saving: bool,
    effects: Vec<EncounterEffect>,
}

impl EncounterForm {
    pub fn new(
        rules: Arc<EncounterRules>,
        encounter: Encounter,
        saved: Option<String>,
        pending: Vec<PendingNpc>,
    ) -> Self {
        let treasure_rules = Arc::new(rules.treasure_rules().clone());
        Self {
            items: BundleEditor::new(treasure_rules.clone()),
            treasure_rules,
            rules,
            encounter,
            saved,
            pending,
            shown: Vec::new(),
            picker: None,
            notice: None,
            saving: false,
            effects: Vec::new(),
        }
    }

    pub fn rules(&self) -> &Arc<EncounterRules> {
        &self.rules
    }

    pub fn treasure_rules(&self) -> &Arc<TreasureRules> {
        &self.treasure_rules
    }

    /// The saved encounter's name, or New Encounter.
    pub fn title(&self) -> String {
        self.saved
            .clone()
            .unwrap_or_else(|| tr("New Encounter").to_owned())
    }

    /// Encounter Totals: Gold Value, Monster XP and Treasure XP.
    pub fn totals(&self) -> [(&'static str, String); 3] {
        let t = self.rules.totals(&self.encounter);
        [
            (tr("Gold Value"), thousands(t.gold_value)),
            (tr("Monster XP"), thousands(t.monster_xp)),
            (tr("Treasure XP"), thousands(t.treasure_xp)),
        ]
    }

    /// The value the Hit Points box of monster package `i` shows: the one
    /// chosen while it is in the package's range, else the range's low
    /// end.
    fn shown_value(&self, i: usize) -> i32 {
        let Some(p) = self.encounter.monsters.get(i) else {
            return 0;
        };
        let (low, high) = p.hit_point_range();
        match self.shown.get(i) {
            Some(&v) if (low..=high).contains(&v) => v,
            _ => low,
        }
    }

    /// The packages, monsters then NPCs.
    pub fn rows(&self) -> Vec<PackageRow> {
        let encounter = &self.encounter;
        encounter
            .packages()
            .into_iter()
            .filter_map(|package| {
                Some(match package {
                    Package::Monster(i) => {
                        let p = encounter.monsters.get(i)?;
                        let (low, high) = p.hit_point_range();
                        let value = self.shown_value(i);
                        PackageRow {
                            package,
                            title: format!("{} × {}", p.count, p.monster.name),
                            subtitle: tr!(
                                "Hit points {hit_points}",
                                hit_points = hit_point_text(&p.hit_points)
                            ),
                            stats: self.rules.stat_line(p),
                            count: p.count,
                            hit_points: Some(HitPoints {
                                low,
                                high,
                                value,
                                with: p.count_with(value),
                                most_with: p.most_with(value),
                                unassigned: p.unassigned(),
                            }),
                        }
                    }
                    Package::Npc(i) => {
                        let p = encounter.npcs.get(i)?;
                        PackageRow {
                            package,
                            title: format!("{} × {}", p.count, self.rules.npc_name(p).trim()),
                            subtitle: tr("Nonplayer character").to_owned(),
                            stats: self.rules.npc_stat_line(p),
                            count: p.count,
                            hit_points: None,
                        }
                    }
                })
            })
            .collect()
    }

    /// The NPCs the NPC generator could not make: how many of which race,
    /// then the classes and why.
    pub fn pending_rows(&self) -> Vec<(String, String)> {
        self.pending
            .iter()
            .map(|p| {
                let classes: Vec<String> = p
                    .npc
                    .classes
                    .iter()
                    .map(|(c, l)| format!("{c} {l}"))
                    .collect();
                (
                    format!("{} × {}", p.count, p.npc.race),
                    format!("{}\n{}", classes.join(" / "), p.reason),
                )
            })
            .collect()
    }

    /// What the Treasure heading says: what to do, or what it is worth.
    pub fn treasure_note(&self) -> String {
        if self.encounter.treasure.is_empty() {
            return tr("Roll the treasure of the monsters' treasure types and the lair").to_owned();
        }
        let summary = self
            .treasure_rules
            .summary_with(&self.encounter.treasure, self.encounter.prices());
        tr!("{value} gp", value = thousands(summary.value_cp / 100))
    }

    /// The treasure types the roller opens with (0x63de14).
    pub fn treasure_counts(&self) -> Counts {
        let mut counts = [0u32; 26];
        for (c, n) in counts.iter_mut().zip(self.encounter.treasure_counts()) {
            *c = n.max(0) as u32;
        }
        counts
    }

    /// The encounter as a printable page (the original's View/Print).
    pub fn sheet(&self) -> Sheet {
        self.rules.sheet(&self.encounter)
    }

    /// The encounter was saved under `name`.
    pub fn saved_as(&mut self, name: String) {
        self.saved = Some(name);
        self.saving = false;
    }

    pub fn take_effects(&mut self) -> Vec<EncounterEffect> {
        std::mem::take(&mut self.effects)
    }

    /// Add NPC… with the saved characters, each by name and file.
    pub fn open_npcs(&mut self, characters: Vec<(String, String)>) {
        let (names, files): (Vec<_>, Vec<_>) = characters.into_iter().unzip();
        let items = names.into_iter().map(|n| (n, String::new())).collect();
        self.picker = Some(Picker::new(Pick::Npc(files), items));
    }

    /// Add Trap… with the traps of the databases.
    pub fn open_traps(&mut self, db: Arc<UserDb<Traps>>) {
        let items = trap_items(&db, Filter::All);
        self.picker = Some(Picker::new(Pick::Trap(db, Filter::All), items));
    }

    /// The character Add NPC… chose, as read.
    pub fn add_npc(&mut self, character: Character) {
        let flavor = self.rules.flavor();
        if let Err(e) = self.encounter.add_npc(character, flavor) {
            self.notice = Some((tr("Add NPC").to_owned(), e.to_string()));
        }
    }

    pub fn input(&mut self, input: EncounterInput, rand: &mut Rand) {
        match input {
            EncounterInput::Name(name) => self.encounter.name = name,
            EncounterInput::MonsterType(kind) => self.encounter.monster_type = kind,
            EncounterInput::Description(text) => self.encounter.description = text,
            EncounterInput::Remove(package) => {
                if let Package::Monster(i) = package
                    && i < self.shown.len()
                {
                    self.shown.remove(i);
                }
                self.encounter.remove(package);
            }
            EncounterInput::Count(package, count) => {
                self.encounter.set_count(package, count, &self.rules, rand)
            }
            EncounterInput::HitPoints(i, value) => {
                if i < self.encounter.monsters.len() {
                    if self.shown.len() <= i {
                        self.shown.resize(i + 1, i32::MIN);
                    }
                    self.shown[i] = value;
                }
            }
            EncounterInput::With(i, count) => {
                let value = self.shown_value(i);
                if let Some(p) = self.encounter.monsters.get_mut(i) {
                    p.set_count_with(value, count);
                }
            }
            EncounterInput::Reroll(i) => {
                if let Some(p) = self.encounter.monsters.get_mut(i) {
                    self.rules.roll_hit_points(p, rand);
                }
            }
            EncounterInput::Items(input) => self.items.input(&mut self.encounter.items, input),
            EncounterInput::ItemsAnswer(serial, yes) => {
                self.items.answer(&mut self.encounter.items, serial, yes)
            }
            EncounterInput::Treasure(bundle) => self.encounter.treasure = bundle,
            EncounterInput::AddMonster => {
                let mut names: Vec<String> = self
                    .rules
                    .monsters()
                    .iter()
                    .map(|m| m.name.clone())
                    .collect();
                names.sort_by_cached_key(|n| cr_store::collate::sort_key(n));
                names.dedup();
                let items = names.into_iter().map(|n| (n, String::new())).collect();
                self.picker = Some(Picker::new(Pick::Monster, items));
            }
            EncounterInput::PickSearch(search) => {
                if let Some(picker) = &mut self.picker {
                    picker.search = search;
                }
            }
            EncounterInput::PickFilter(index) => {
                if let Some(picker) = &mut self.picker
                    && let Pick::Trap(db, filter) = &mut picker.pick
                {
                    *filter = Filter::of(index as u32);
                    picker.items = trap_items(db, *filter);
                    picker.chosen = None;
                }
            }
            EncounterInput::PickChoose(name) => {
                if let Some(picker) = &mut self.picker {
                    picker.chosen = Some(name);
                }
            }
            EncounterInput::PickAnswer(yes) => {
                let Some(picker) = self.picker.take() else {
                    return;
                };
                let Some(name) = picker.chosen.filter(|_| yes) else {
                    return;
                };
                match picker.pick {
                    Pick::Monster => {
                        if let Some(package) = self.rules.new_monster_package(&name, None, rand) {
                            self.encounter.monsters.push(package);
                        }
                    }
                    Pick::Npc(files) => {
                        if let Some(file) = picker
                            .items
                            .iter()
                            .position(|(n, _)| *n == name)
                            .and_then(|i| files.get(i))
                        {
                            self.effects.push(EncounterEffect::AddNpc(file.clone()));
                        }
                    }
                    Pick::Trap(db, _) => {
                        if let Err(e) = add_trap(&mut self.encounter.traps, &db, &name) {
                            self.notice =
                                Some((tr("Unable to Add Trap").to_owned(), e.to_string()));
                        }
                    }
                }
            }
            EncounterInput::RemoveTrap(name) => {
                if let Err(e) = remove_trap(&mut self.encounter.traps, &name) {
                    self.notice = Some((tr("Remove Trap").to_owned(), e.to_string()));
                }
            }
            EncounterInput::Save => {
                if !self.saving {
                    self.saving = true;
                    self.effects.push(EncounterEffect::Save {
                        old: self.saved.clone(),
                        encounter: Box::new(self.encounter.clone()),
                    });
                }
            }
        }
    }
}

/// Picking the template of a roll in the table editor (0x6556a9): the
/// templates of one frequency that pass the filter, the roll's own
/// first ([`PICK_ROLLS`]), or (None).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollPick {
    /// The roll, 2 to 20, less 2.
    pub index: usize,
    pub frequency: usize,
    pub names: Vec<String>,
    /// The template chosen; `Some("")` for (None).
    pub chosen: Option<String>,
}

/// A change in the table editor, or one of its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum TableEditInput {
    Name(String),
    /// Database, by its place in [`SOURCES`].
    Source(usize),
    /// Lair and/or Wandering, by its place in [`LAIR_MODES`].
    Lair(usize),
    Fewest(i32),
    Most(i32),
    /// A climate (0-4) or terrain (5-36) flag.
    Place(usize, bool),
    Fill,
    Roll,
    /// Pick the template of the roll with this index (roll - 2).
    PickRoll(usize),
    PickFrequency(usize),
    /// A template chosen for the roll; empty for (None).
    PickChoose(String),
    PickAnswer(bool),
    Save,
}

/// What the table editor asks of the app.
#[derive(Debug, Clone, PartialEq)]
pub enum TableEffect {
    /// Say this, as a toast.
    Notice(String),
    /// Create the encounter of this template, saying that.
    Generate { template: String, said: String },
    /// Save `table` in place of the saved table `old`.
    Save {
        old: Option<String>,
        table: Box<FrequencyTable>,
    },
}

/// The 2-20 table editor: name, filter, the 19 rolls, Fill, Roll.
#[derive(Clone)]
pub struct TableEditor {
    rules: Arc<EncounterRules>,
    pub table: FrequencyTable,
    /// The name the table is saved under.
    pub saved: Option<String>,
    pub picking: Option<RollPick>,
    pub error: Option<String>,
    /// A save under way.
    pub saving: bool,
    effects: Vec<TableEffect>,
}

impl TableEditor {
    pub fn new(rules: Arc<EncounterRules>, table: Option<FrequencyTable>) -> Self {
        let saved = table.as_ref().map(|t| t.name.clone());
        let mut table = table.unwrap_or_else(|| FrequencyTable::new(""));
        table.slots.resize(19, String::new());
        Self {
            rules,
            table,
            saved,
            picking: None,
            error: None,
            saving: false,
            effects: Vec::new(),
        }
    }

    pub fn rules(&self) -> &Arc<EncounterRules> {
        &self.rules
    }

    /// The saved table's name, or New Encounter Table.
    pub fn title(&self) -> String {
        self.saved
            .clone()
            .unwrap_or_else(|| tr("New Encounter Table").to_owned())
    }

    /// Database, by its place in [`SOURCES`].
    pub fn source(&self) -> usize {
        self.table.filter.source.clamp(0, 2) as usize
    }

    /// Lair and/or Wandering, by its place in [`LAIR_MODES`].
    pub fn lair(&self) -> usize {
        self.table.filter.lair.clamp(0, 3) as usize
    }

    /// The template of each roll, or (None).
    pub fn slots(&self) -> Vec<String> {
        (0..19)
            .map(|i| {
                self.table
                    .slots
                    .get(i)
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .unwrap_or_else(|| tr("(None)").to_owned())
            })
            .collect()
    }

    /// The table was saved under `name`.
    pub fn saved_as(&mut self, name: String) {
        self.saved = Some(name);
        self.saving = false;
    }

    pub fn take_effects(&mut self) -> Vec<TableEffect> {
        std::mem::take(&mut self.effects)
    }

    fn pick_names(&self, frequency: usize) -> Vec<String> {
        self.rules.candidate_names(&self.table.filter, frequency)
    }

    pub fn input(&mut self, input: TableEditInput, rand: &mut Rand) {
        let filter = &mut self.table.filter;
        match input {
            TableEditInput::Name(name) => {
                self.table.name = name;
                self.error = None;
            }
            TableEditInput::Source(i) => filter.source = Source::of(i as i32).raw(),
            TableEditInput::Lair(i) => filter.lair = LairMode::of(i as i32).raw(),
            TableEditInput::Fewest(n) => filter.hit_dice.0 = n.clamp(0, 100),
            TableEditInput::Most(n) => filter.hit_dice.1 = n.clamp(0, 100),
            TableEditInput::Place(i, on) => {
                if let Some(flag) = filter.climates.get_mut(i) {
                    *flag = i32::from(on);
                }
            }
            TableEditInput::Fill => {
                let candidates = self.rules.candidates(&self.table.filter);
                let count: usize = candidates[..4].iter().map(Vec::len).sum();
                self.table.fill(&candidates, rand);
                self.effects.push(TableEffect::Notice(trn!(
                    "{n} template passes the filter",
                    "{n} templates pass the filter",
                    count
                )));
            }
            TableEditInput::Roll => {
                if self.table.slots.iter().all(|s| s.is_empty()) {
                    self.effects.push(TableEffect::Notice(
                        tr("The table has no templates; fill it first").to_owned(),
                    ));
                    return;
                }
                let roll = roll_2_to_20(&self.rules, rand);
                self.effects.push(match self.table.slot(roll) {
                    Some(name) => TableEffect::Generate {
                        template: name.to_owned(),
                        said: tr!("Rolled {roll}: {template}", roll, template = name),
                    },
                    None => TableEffect::Notice(tr!("Rolled {roll}, which has no template", roll)),
                });
            }
            TableEditInput::PickRoll(index) => {
                if let Some(&frequency) = PICK_ROLLS.get(index) {
                    self.picking = Some(RollPick {
                        index,
                        frequency,
                        names: self.pick_names(frequency),
                        chosen: None,
                    });
                }
            }
            TableEditInput::PickFrequency(frequency) => {
                let names = self.pick_names(frequency);
                if let Some(pick) = &mut self.picking {
                    pick.frequency = frequency;
                    pick.names = names;
                    pick.chosen = None;
                }
            }
            TableEditInput::PickChoose(name) => {
                if let Some(pick) = &mut self.picking {
                    pick.chosen = Some(name);
                }
            }
            TableEditInput::PickAnswer(yes) => {
                if let Some(pick) = self.picking.take()
                    && yes
                    && let Some(name) = pick.chosen
                {
                    self.table.slots.resize(19, String::new());
                    self.table.slots[pick.index] = name;
                }
            }
            TableEditInput::Save => {
                let mut table = self.table.clone();
                table.name = table.name.trim().to_owned();
                if table.name.is_empty() {
                    self.error = Some(tr("A name must be supplied.").to_owned());
                } else if !self.saving {
                    self.saving = true;
                    self.effects.push(TableEffect::Save {
                        old: self.saved.clone(),
                        table: Box::new(table),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Option<Arc<EncounterRules>> {
        let dir = std::env::var_os("CORERULES_DIR")?;
        let install = cr_store::Installation::open(std::path::Path::new(&dir)).ok()?;
        install.encounter_rules().ok().map(Arc::new)
    }

    #[test]
    fn a_monster_added_shows_its_hit_points() {
        let Some(rules) = rules() else {
            return;
        };
        let mut rand = Rand::seeded(11);
        let mut form = EncounterForm::new(rules, Encounter::default(), None, Vec::new());
        form.input(EncounterInput::AddMonster, &mut rand);
        let picker = form.picker.as_ref().unwrap();
        let name = picker.items[picker.shown()[0]].0.clone();
        form.input(EncounterInput::PickChoose(name.clone()), &mut rand);
        form.input(EncounterInput::PickAnswer(true), &mut rand);
        assert!(form.picker.is_none());
        let rows = form.rows();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].title.ends_with(&name));
        let hp = rows[0].hit_points.clone().unwrap();
        assert_eq!(hp.value, hp.low);
        form.input(EncounterInput::Count(rows[0].package, 3), &mut rand);
        assert_eq!(form.rows()[0].count, 3);
        form.input(EncounterInput::Remove(rows[0].package), &mut rand);
        assert!(form.rows().is_empty());
    }

    #[test]
    fn a_table_needs_a_name_and_picks_by_frequency() {
        let Some(rules) = rules() else {
            return;
        };
        let mut rand = Rand::seeded(11);
        let mut editor = TableEditor::new(rules, None);
        editor.input(TableEditInput::Save, &mut rand);
        assert!(editor.error.is_some() && editor.take_effects().is_empty());
        editor.input(TableEditInput::PickRoll(7), &mut rand);
        assert_eq!(editor.picking.as_ref().unwrap().frequency, PICK_ROLLS[7]);
        editor.input(TableEditInput::PickChoose(String::new()), &mut rand);
        editor.input(TableEditInput::PickAnswer(true), &mut rand);
        assert_eq!(editor.slots()[7], tr("(None)"));
        editor.input(TableEditInput::Fill, &mut rand);
        assert!(matches!(
            editor.take_effects()[..],
            [TableEffect::Notice(_)]
        ));
        editor.input(TableEditInput::Name(" Wilds ".into()), &mut rand);
        editor.input(TableEditInput::Save, &mut rand);
        assert!(matches!(
            &editor.take_effects()[..],
            [TableEffect::Save { old: None, table }] if table.name == "Wilds"
        ));
    }
}
