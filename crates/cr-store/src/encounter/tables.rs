//! The 2-20 encounter tables (`UserDbas/EncTabU.dat`): nineteen template
//! names for the rolls 2 to 20, the filter that picks the candidates
//! (0x64ff72), Fill (0x655487), Roll (0x6561c4) and Roll and Generate
//! (0x640103).

use cr_formats::Object;
use cr_formats::encounter as format;

use super::{EncounterRules, Flavor, Rand, Template, flavor_of, replace_or_create};
use crate::{Error, Installation, Result};

/// The version string the Expansion writes (it reads both).
pub const FREQUENCY_VERSION_EXPANSION: &str = "TSRP3/FreqDoc/0001";
/// The version string Core Rules 2.0 writes (and the shipped file has).
pub const FREQUENCY_VERSION_20: &str = "TSRP2/FreqDoc/0001";

/// The frequency category Fill draws each roll 2 to 20 from: 0 common, 1
/// uncommon, 2 rare, 3 very rare.
pub const CATEGORY_ROLLS: [usize; 19] = [3, 3, 2, 2, 2, 1, 1, 0, 0, 0, 0, 0, 1, 1, 2, 2, 2, 3, 3];

const FILE: &str = "UserDbas/EncTabU.dat";

/// Which templates a table draws from (filter +0x268).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    All,
    Standard,
    Custom,
}

impl Source {
    pub fn of(raw: i32) -> Self {
        match raw {
            1 => Self::Standard,
            2 => Self::Custom,
            _ => Self::All,
        }
    }

    pub fn raw(self) -> i32 {
        self as i32
    }
}

/// Lair or wandering (filter +0x26c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LairMode {
    /// Templates marked for both.
    LairAndWandering,
    /// Any template.
    LairOrWandering,
    Wandering,
    Lair,
}

impl LairMode {
    pub fn of(raw: i32) -> Self {
        match raw {
            1 => Self::LairOrWandering,
            2 => Self::Wandering,
            3 => Self::Lair,
            _ => Self::LairAndWandering,
        }
    }

    pub fn raw(self) -> i32 {
        self as i32
    }
}

/// The filter of a table (the object at +0x4). Values are kept as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableFilter {
    /// Climate (0-4) and terrain (5-36) flags.
    pub climates: [i32; 150],
    /// A template's climate/terrain category; "Any" takes every one.
    pub category: String,
    pub hit_dice: (i32, i32),
    /// [`Source`] as stored.
    pub source: i32,
    /// [`LairMode`] as stored.
    pub lair: i32,
}

impl Default for TableFilter {
    /// The constructor's values (0x64fd10).
    fn default() -> Self {
        Self {
            climates: [1; 150],
            category: String::new(),
            hit_dice: (0, 100),
            source: 0,
            lair: 0,
        }
    }
}

impl TableFilter {
    /// Whether `t` is a candidate (0x64ff72), source aside.
    pub fn accepts(&self, t: &Template) -> bool {
        if t.hit_dice > self.hit_dice.1 || t.hit_dice < self.hit_dice.0 {
            return false;
        }
        let shared = |i: usize| self.climates[i] != 0 && t.climates[i] != 0;
        let place = self.category == "Any"
            || self.category == t.climate_terrain
            || ((0..5).any(shared) && (5..150).find(|&i| shared(i)).is_some_and(|i| i < 37));
        if !place {
            return false;
        }
        match LairMode::of(self.lair) {
            LairMode::LairAndWandering => t.placement[0],
            LairMode::LairOrWandering => true,
            LairMode::Wandering => t.placement[2],
            LairMode::Lair => t.placement[1],
        }
    }
}

/// One table (`CEncounterFreqTableOb`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrequencyTable {
    pub name: String,
    /// Template names for the rolls 2 to 20 (index = roll - 2); empty
    /// slots are empty strings.
    pub slots: Vec<String>,
    pub filter: TableFilter,
}

impl FrequencyTable {
    /// A new table: nineteen empty slots and the default filter.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            slots: vec![String::new(); 19],
            filter: TableFilter::default(),
        }
    }

    fn of(t: &format::FrequencyTable) -> Self {
        let d = &t.table;
        Self {
            name: t.name.clone(),
            slots: t.slots.clone(),
            filter: TableFilter {
                climates: d.climates,
                category: d.category.clone(),
                hit_dice: (d.hit_dice_min, d.hit_dice_max),
                source: d.source,
                lair: d.lair,
            },
        }
    }

    fn to_record(&self) -> format::FrequencyTable {
        let f = &self.filter;
        format::FrequencyTable {
            name: self.name.clone(),
            slots: self.slots.clone(),
            table: format::FrequencyTableData {
                hit_dice_min: f.hit_dice.0,
                hit_dice_max: f.hit_dice.1,
                source: f.source,
                category: f.category.clone(),
                lair: f.lair,
                climates: f.climates,
            },
        }
    }

    /// Fill (0x655487): each slot, roll 2 first, gets a template drawn
    /// with `rand() % n` from its category's candidates, or nothing when
    /// there are none.
    pub fn fill(&mut self, candidates: &[Vec<&Template>; 5], rand: &mut Rand) {
        self.slots.resize(19, String::new());
        for (slot, &category) in self.slots.iter_mut().zip(&CATEGORY_ROLLS) {
            let bucket = &candidates[category];
            *slot = if bucket.is_empty() {
                String::new()
            } else {
                bucket[(rand.rand() as usize) % bucket.len()].name.clone()
            };
        }
    }

    /// The template of a roll 2 to 20, if the slot has one.
    pub fn slot(&self, roll: i32) -> Option<&str> {
        let i = usize::try_from(roll - 2).ok()?;
        self.slots
            .get(i)
            .map(String::as_str)
            .filter(|s| !s.is_empty())
    }
}

/// Roll in the table dialog (0x6561c4): 1d8 + 1d12, 2 to 20.
pub fn roll_2_to_20(rules: &EncounterRules, rand: &mut Rand) -> i32 {
    let a = rules.between(rand, 1, 8);
    let b = rules.between(rand, 1, 12);
    a + b
}

/// Roll and Generate on the 2-20 screen (0x640103, 2.0 0x4bc1a0): a slot
/// drawn uniformly with `rand_between(0, 18)` until one holds a template,
/// at most 10,000 times. `None`, without drawing, unless the table has 19
/// slots and one of them is filled. Returns the slot index (roll - 2).
pub fn roll_and_pick(
    rules: &EncounterRules,
    table: &FrequencyTable,
    rand: &mut Rand,
) -> Option<usize> {
    if table.slots.len() != 19 || table.slots.iter().all(|s| s.is_empty()) {
        return None;
    }
    for _ in 0..10_000 {
        let i = rules.between(rand, 0, 18) as usize;
        if table.slots.get(i).is_some_and(|s| !s.is_empty()) {
            return Some(i);
        }
    }
    None
}

/// `UserDbas/EncTabU.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrequencyTables {
    pub version: String,
    pub tables: Vec<FrequencyTable>,
}

impl FrequencyTables {
    pub fn parse(data: &[u8]) -> std::result::Result<Self, String> {
        let doc = format::FrequencyTables::parse(data).map_err(|e| e.to_string())?;
        let tables = doc
            .tables
            .iter()
            .map(|o| match o {
                Object::FrequencyTable(t) => Ok(FrequencyTable::of(t)),
                other => Err(format!("not a frequency table: {}", other.class().0)),
            })
            .collect::<std::result::Result<_, _>>()?;
        Ok(Self {
            version: doc.version,
            tables,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        format::FrequencyTables {
            version: self.version.clone(),
            tables: self
                .tables
                .iter()
                .map(|t| Object::FrequencyTable(Box::new(t.to_record())))
                .collect(),
        }
        .to_bytes()
        .map_err(|source| Error::Format {
            path: FILE.into(),
            source,
        })
    }

    pub fn table(&self, name: &str) -> Option<&FrequencyTable> {
        self.tables.iter().find(|t| t.name == name)
    }
}

/// The version string `flavor`'s program writes.
pub fn frequency_version(flavor: Flavor) -> &'static str {
    match flavor {
        Flavor::Expansion => FREQUENCY_VERSION_EXPANSION,
        Flavor::Core20 => FREQUENCY_VERSION_20,
    }
}

impl EncounterRules {
    /// The candidates of a filter (0x64ff72) by frequency: common (65 and
    /// up), uncommon (20), rare (7), very rare (1), exempt (never drawn).
    pub fn candidates(&self, filter: &TableFilter) -> [Vec<&Template>; 5] {
        let mut buckets: [Vec<&Template>; 5] = Default::default();
        let source = Source::of(filter.source);
        for t in self.templates() {
            let from = match source {
                Source::All => true,
                Source::Standard => !t.user,
                Source::Custom => t.user,
            };
            if !from || !filter.accepts(t) {
                continue;
            }
            let bucket = match t.frequency {
                65.. => 0,
                20.. => 1,
                7.. => 2,
                1.. => 3,
                _ => 4,
            };
            buckets[bucket].push(t);
        }
        buckets
    }
}

impl Installation {
    /// The 2-20 tables; none (in the program's version) without a file.
    pub fn frequency_tables(&self) -> Result<FrequencyTables> {
        if self.path(FILE).is_none() {
            return Ok(FrequencyTables {
                version: frequency_version(flavor_of(self)).into(),
                tables: Vec::new(),
            });
        }
        FrequencyTables::parse(&self.read(FILE)?)
            .map_err(|e| Error::Invalid(format!("{FILE}: {e}")))
    }

    /// Replaces `UserDbas/EncTabU.dat`, keeping the old file as
    /// `EncTabU.dat.bak`. The version string is the one the installation's
    /// program writes.
    pub fn write_frequency_tables(&self, tables: &FrequencyTables) -> Result<()> {
        let doc = FrequencyTables {
            version: frequency_version(flavor_of(self)).into(),
            tables: tables.tables.clone(),
        };
        replace_or_create(self, FILE, "EncTabU.dat", &doc.to_bytes()?)
    }
}
