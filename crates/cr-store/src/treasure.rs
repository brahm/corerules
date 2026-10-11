//! The treasure generator of the DM Toolkit (Treasure menu): treasure
//! types A to Z, rolls on the treasure tables of `UserDbas/Treasure.dat`,
//! intelligent weapons and spell scrolls, the Calculate summary, and the
//! saved bundles of `UserDbas/TreaBunU.dat`.
//!
//! The rolls reproduce the originals exactly for a given seed: they use the
//! MSVC `rand()` ([`Rand`]) in the same order, quirks included. The
//! Expansion (Tsrp3.exe) and Core Rules 2.0 (Evermore.exe) differ only in
//! how spell scrolls pick a spell level and in item dice with no sides;
//! [`TreasureRules`] follows the installation's program. Addresses are the
//! Expansion's unless marked "2.0".

mod intelligent;
mod letters;

use std::collections::HashMap;

use cr_formats::Object;
use cr_formats::archive::ansi_bytes;
use cr_formats::database::ObjectList;
use cr_formats::inventory::{InventoryBundle, InventoryExtra, InventoryItem};
use cr_formats::parts::{PART_TYPE_COUNT, PartsFile};
use cr_formats::spells::SpellsFile;
use cr_formats::treasure::{Treasure, TreasureTableEntry};

pub use crate::sheet::Flavor;
pub use letters::{MonsterTreasure, encounter_counts, parse_letters};

use crate::{Error, Installation, Result};

/// The treasure types, in the order the roller rolls them.
pub const LETTERS: [char; 26] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

/// How many times to roll each treasure type, A to Z.
pub type Counts = [u32; 26];

/// The name of the group the roller gathers gems and art objects in.
pub const GEMS_AND_ART: &str = "Gems and Art";

/// Deepest sub-table nesting before a roll gives up (0x5462c7).
pub const DEPTH_LIMIT: u32 = 127;

/// The part type of gems and art objects (`PART_TYPES`).
const GEMS_AND_ART_TYPE: usize = 13;

/// The MSVC C runtime `rand()` (Expansion 0x718526, 2.0 0x5e3b70): one
/// stream shared by every roll, as in the original.
#[derive(Debug, Clone)]
pub struct Rand {
    state: u32,
    calls: u64,
}

impl Default for Rand {
    fn default() -> Self {
        Self::from_clock()
    }
}

impl Rand {
    /// `srand(seed)`.
    pub fn seeded(seed: u32) -> Self {
        Self {
            state: seed,
            calls: 0,
        }
    }

    /// Seeded as the originals do it: `srand(_ftime().millitm)`, the
    /// milliseconds of the clock (0 to 999).
    pub fn from_clock() -> Self {
        Self::seeded(Self::clock_seed())
    }

    /// The seed [`from_clock`](Self::from_clock) uses now.
    pub fn clock_seed() -> u32 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_millis())
    }

    /// The next value, 0 to 32767.
    pub fn rand(&mut self) -> i32 {
        self.state = self.state.wrapping_mul(214_013).wrapping_add(2_531_011);
        self.calls += 1;
        ((self.state >> 16) & 0x7fff) as i32
    }

    /// How many values have been drawn since seeding.
    pub fn calls(&self) -> u64 {
        self.calls
    }

    /// `rand() % n` with C's sign rules; `None` where C would divide by
    /// zero.
    fn modulo(&mut self, n: i32) -> Option<i32> {
        (n != 0).then(|| self.rand() % n)
    }

    /// `d(n, s, m)` (0x54e96c): n dice of s sides, times m.
    pub fn d(&mut self, n: i32, s: i32, m: i32) -> i32 {
        let mut sum = 0i32;
        for _ in 0..n {
            sum = sum.wrapping_add(1 + self.rand() % s);
        }
        sum.wrapping_mul(m)
    }

    /// `fine(n, s, m)` (0x54f152): "n·s × m" with the low digits filled in,
    /// so `fine(1, 3, 1000)` gives 1001 to 3000.
    pub fn fine(&mut self, n: i32, s: i32, m: i32) -> i32 {
        if n == 1 {
            let a = self.d(1, s - 1, m);
            let b = self.d(1, m, 1);
            a.wrapping_add(b)
        } else {
            let a = self.d(n - 1, s, m);
            let b = self.d(1, s - 1, m);
            let c = self.d(1, m, 1);
            a.wrapping_add(b).wrapping_add(c)
        }
    }
}

/// The coin the roller's Platinum / Electrum switch picks (default
/// Platinum, 0x54cade).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Coin {
    #[default]
    Platinum,
    Electrum,
}

/// One entry of a treasure bundle (`CInventoryItem`).
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub name: String,
    pub quantity: u32,
    /// Set on lines that are not items ("Gems and Art", the abilities of
    /// an intelligent weapon, "!!" errors); Calculate skips them.
    pub flag: bool,
    /// What the item holds; items with contents never merge.
    pub contents: Option<Bundle>,
    /// The spells of a scroll.
    pub extra: Option<InventoryExtra>,
    /// Member +0x08, 999 from the constructor.
    pub unknown_0x8: u32,
}

/// A spell written on a scroll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollSpell {
    pub name: String,
    pub pages: u32,
    pub wizard: bool,
}

impl Item {
    /// `quantity` of `name`, as the constructor makes it (0x4ac7ce).
    pub fn new(name: impl Into<String>, quantity: u32) -> Self {
        Self {
            name: name.into(),
            quantity,
            flag: false,
            contents: None,
            extra: None,
            unknown_0x8: 999,
        }
    }

    /// A flagged line (errors, weapon abilities, the gems group).
    fn line(name: impl Into<String>) -> Self {
        Self {
            flag: true,
            ..Self::new(name, 1)
        }
    }

    /// As the roller's tree shows it (0x4ad31e): the name, then
    /// " x3,335" when there is more than one.
    pub fn label(&self) -> String {
        if self.quantity == 1 {
            self.name.clone()
        } else {
            format!("{} x{}", self.name, thousands(self.quantity as i32))
        }
    }

    /// The spells of a scroll, in the order they were written.
    pub fn spells(&self) -> Vec<ScrollSpell> {
        let Some(extra) = &self.extra else {
            return Vec::new();
        };
        extra
            .unknown_0x10
            .iter()
            .enumerate()
            .map(|(i, name)| ScrollSpell {
                name: name.clone(),
                pages: extra.unknown_0x24.get(i).copied().unwrap_or(0),
                wizard: extra.unknown_0x38.get(i).is_some_and(|&w| w != 0),
            })
            .collect()
    }

    pub fn from_inventory(item: &InventoryItem) -> Result<Self> {
        let contents = match item.contents.as_deref() {
            None => None,
            Some(Object::InventoryBundle(bundle)) => Some(Bundle::from_inventory(bundle)?),
            Some(other) => {
                return Err(Error::Invalid(format!(
                    "{}: contents are a {other:?}, not a bundle",
                    item.name
                )));
            }
        };
        Ok(Self {
            name: item.name.clone(),
            quantity: item.quantity,
            flag: item.flag_0x4 != 0,
            contents,
            extra: item.extra.clone(),
            unknown_0x8: item.unknown_0x8,
        })
    }

    pub fn to_inventory(&self) -> InventoryItem {
        InventoryItem {
            name: self.name.clone(),
            quantity: self.quantity,
            contents: self
                .contents
                .as_ref()
                .map(|b| Box::new(Object::InventoryBundle(b.to_inventory()))),
            unknown_0x8: self.unknown_0x8,
            flag_0x4: u8::from(self.flag),
            extra: self.extra.clone(),
        }
    }
}

/// A treasure bundle (`CInventoryBundle`): items kept sorted by name.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bundle {
    pub items: Vec<Item>,
}

impl Bundle {
    /// Adds `item` as the original does (0x4aa8e3). With `merge`, an item
    /// with no contents adds its quantity to an item of the same name that
    /// has none either (flag and extra are not compared). Otherwise it goes
    /// before the first item whose name is greater under `strcmp`.
    pub fn insert(&mut self, item: Item, merge: bool) {
        if merge
            && item.contents.is_none()
            && let Some(same) = self
                .items
                .iter_mut()
                .find(|e| e.contents.is_none() && e.name == item.name)
        {
            same.quantity = same.quantity.wrapping_add(item.quantity);
            return;
        }
        let key = ansi_bytes(&item.name);
        let at = self
            .items
            .iter()
            .position(|e| ansi_bytes(&e.name) > key)
            .unwrap_or(self.items.len());
        self.items.insert(at, item);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn from_inventory(bundle: &InventoryBundle) -> Result<Self> {
        Ok(Self {
            items: bundle
                .items
                .iter()
                .map(Item::from_inventory)
                .collect::<Result<_>>()?,
        })
    }

    pub fn to_inventory(&self) -> InventoryBundle {
        InventoryBundle {
            items: self.items.iter().map(Item::to_inventory).collect(),
        }
    }

    /// The printed listing (CTreasurePrinter 0x5403fb): one line per item,
    /// two spaces of indent per level, scroll spells as "name (pages)".
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        self.write_text(0, &mut out);
        out
    }

    fn write_text(&self, depth: usize, out: &mut String) {
        for item in &self.items {
            out.push_str(&"  ".repeat(depth));
            out.push_str(&item.label());
            out.push('\n');
            for spell in item.spells() {
                out.push_str(&"  ".repeat(depth + 1));
                out.push_str(&format!("{} ({})\n", spell.name, spell.pages));
            }
            if let Some(contents) = &item.contents {
                contents.write_text(depth + 1, out);
            }
        }
    }
}

/// A number with thousands separators (0x554d8a): "3,335", "-1,000".
pub fn thousands(n: i32) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    if n < 0 {
        out.push('-');
    }
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// One row of a treasure table, as the roller reads it.
#[derive(Debug, Clone)]
struct Entry {
    weight: u32,
    is_table: bool,
    item: Item,
    quantity: [u32; 3],
    sub_tables: [([u32; 3], String); 2],
    intelligent: u8,
    spell_levels: (u32, u32),
    spell_count: u32,
    wizard: bool,
}

impl Entry {
    fn of(e: &TreasureTableEntry) -> Result<Self> {
        Ok(Self {
            weight: e.weight,
            is_table: e.is_table != 0,
            item: Item::from_inventory(&e.item)?,
            quantity: e.quantity_dice,
            sub_tables: [
                (e.sub_table_1_dice, e.sub_table_1.clone()),
                (e.sub_table_2_dice, e.sub_table_2.clone()),
            ],
            intelligent: e.unknown_0x54,
            spell_levels: (e.spell_min_level, e.spell_max_level),
            spell_count: e.spell_count,
            wizard: e.spell_wizard != 0,
        })
    }
}

#[derive(Debug, Clone)]
struct Table {
    name: String,
    entries: Vec<Entry>,
}

/// What Calculate reads from a part.
#[derive(Debug, Clone, Copy)]
struct PartValue {
    /// pp, gp, sp, cp (+0x1f8..+0x204).
    price: [i32; 4],
    /// +0x230.
    xp: i32,
    /// +0x224.
    magical: bool,
}

/// Part type names as Calculate writes them (0x468c52).
pub const SUMMARY_TYPE_NAMES: [&str; PART_TYPE_COUNT] = [
    "Weapons",
    "Armor",
    "Wizard Spells",
    "Weapon Proficiencies",
    "Non-Weapon Proficiencies",
    "Racial Abilities",
    "Languages",
    "Equipment",
    "Magical Items",
    "Schools of Magic",
    "Priest Spells",
    "Traits",
    "Disadvantages",
    "Gems and Art",
    "Class Abilities",
    "Character Kits",
    "Money",
    "Count",
    "Spheres",
    "Unknown",
    "Unknown",
];

/// The order 0x468356 tries the part types in to find a name's type.
const TYPE_ORDER: [usize; 16] = [1, 15, 14, 12, 7, 13, 6, 8, 4, 5, 9, 11, 0, 17, 16, 18];

/// Totals of one part type in a [`Summary`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypeTotals {
    /// Value in copper pieces.
    pub value_cp: i32,
    pub xp: i32,
    pub count: i32,
}

/// What Calculate shows (0x54232d).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Summary {
    /// Value in copper pieces.
    pub value_cp: i32,
    pub xp: i32,
    pub magical_items: i32,
    /// By part type (see [`SUMMARY_TYPE_NAMES`]).
    pub by_type: [TypeTotals; PART_TYPE_COUNT],
}

impl Summary {
    /// The summary list: "GP Value: 1,173", "XP Value: ...", "Magical
    /// Items: ...", then three lines per part type with any total. Each
    /// gold value is floored on its own, so the parts need not add up.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("GP Value: {}", thousands(self.value_cp / 100)),
            format!("XP Value: {}", thousands(self.xp)),
            format!("Magical Items: {}", thousands(self.magical_items)),
        ];
        for (name, t) in SUMMARY_TYPE_NAMES.iter().zip(&self.by_type) {
            if t.value_cp == 0 && t.xp == 0 && t.count == 0 {
                continue;
            }
            lines.push(format!("Number of {name}: {}", thousands(t.count)));
            lines.push(format!("XP Value of {name}: {}", thousands(t.xp)));
            lines.push(format!(
                "GP Value of {name}: {}",
                thousands(t.value_cp / 100)
            ));
        }
        lines
    }
}

/// The databases the treasure roller reads.
#[derive(Debug, Clone)]
pub struct TreasureRules {
    flavor: Flavor,
    tables: Vec<Table>,
    /// First table of each name (0x54ccbd).
    by_name: HashMap<String, usize>,
    /// (wizard, level, name) of `Database/Spells.dat`, then of
    /// `UserDbas/SpellsU.dat`, in file order.
    spells: [Vec<(bool, i32, String)>; 2],
    /// Part maps by type, user parts first then standard ones, keyed by the
    /// trimmed upper-case name.
    parts: [[HashMap<String, PartValue>; PART_TYPE_COUNT]; 2],
}

impl Installation {
    /// Loads the treasure tables (`UserDbas/Treasure.dat`, the copy the
    /// roller uses; `Database/Treasure.dat` when there is none), spells and
    /// parts.
    pub fn treasure_rules(&self) -> Result<TreasureRules> {
        let flavor = if self.path("UserDbas/AllowedU.dat").is_some() {
            Flavor::Expansion
        } else {
            Flavor::Core20
        };
        let relative = ["UserDbas/Treasure.dat", "Database/Treasure.dat"]
            .into_iter()
            .find(|r| self.path(r).is_some())
            .ok_or_else(|| Error::Invalid("no Treasure.dat in the installation".into()))?;
        let list = ObjectList::parse(&self.read(relative)?).map_err(|source| Error::Format {
            path: relative.into(),
            source,
        })?;
        let mut tables = Vec::new();
        for object in &list.objects {
            if let Object::TreasureTable(t) = object {
                let entries = t
                    .entries
                    .iter()
                    .filter_map(|e| match e {
                        Object::TreasureTableEntry(e) => Some(Entry::of(e)),
                        _ => None,
                    })
                    .collect::<Result<_>>()?;
                tables.push(Table {
                    name: t.name.clone(),
                    entries,
                });
            }
        }
        let mut rules = TreasureRules {
            flavor,
            by_name: HashMap::new(),
            tables,
            spells: Default::default(),
            parts: Default::default(),
        };
        for (i, t) in rules.tables.iter().enumerate() {
            rules.by_name.entry(t.name.clone()).or_insert(i);
        }
        for (source, relative) in ["Database/Spells.dat", "UserDbas/SpellsU.dat"]
            .into_iter()
            .enumerate()
        {
            if self.path(relative).is_none() {
                continue;
            }
            let file =
                SpellsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                    path: relative.into(),
                    source,
                })?;
            for (wizard, objects) in [(true, &file.wizard), (false, &file.priest)] {
                for object in objects {
                    if let Object::Spell(s) = object {
                        rules.spells[source].push((wizard, s.level, s.name.clone()));
                    }
                }
            }
        }
        for (source, relative) in ["UserDbas/PartsU.dat", "Database/Parts.dat"]
            .into_iter()
            .enumerate()
        {
            if self.path(relative).is_none() {
                continue;
            }
            let file = PartsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                path: relative.into(),
                source,
            })?;
            for (t, array) in file.arrays.iter().enumerate() {
                for object in array {
                    if let Object::Part(part) = object {
                        let h = &part.head;
                        rules.parts[source][t].insert(
                            part_key(&h.name),
                            PartValue {
                                price: [
                                    h.unknown_0x1f8[0],
                                    h.unknown_0x1f8[1],
                                    h.unknown_0x1f8[2],
                                    h.unknown_0x1f8[3],
                                ],
                                xp: h.unknown_0x230,
                                magical: h.unknown_0x224 != 0,
                            },
                        );
                    }
                }
            }
        }
        Ok(rules)
    }
}

/// A part map key: trimmed, upper case, with the program's fix for a
/// misspelt potion (0x466e7b).
fn part_key(name: &str) -> String {
    let key = name
        .trim_matches([' ', '\t', '\n', '\x0b', '\x0c', '\r'])
        .to_ascii_uppercase();
    if key == "POTION OF INVISIBLITY" {
        "POTION OF INVISIBILITY".into()
    } else {
        key
    }
}

/// Coin names; the fourth is platinum or electrum.
const COPPER: &str = "* Copper Pieces";
const SILVER: &str = "* Silver Pieces";
const GOLD: &str = "* Gold Pieces";

const GEMS: &str = "DMG Treasure: Gems";
const ART: &str = "DMG 87: Objects of Art";
const MAGIC: &str = "DMG 88: Magical Items";
const POTIONS: &str = "DMG 89: Potions and Oils";
const SCROLLS: &str = "DMG 90: Scrolls";
const ARMOR_OR_WEAPON: &str = "DMG Treasure: Random Armor or Weapon";
const NON_WEAPON: &str = "DMG Treasure: Random Non-Weapon";

#[derive(Debug, Clone, Copy)]
enum Amount {
    /// `fine(n, s, m)`.
    Fine(i32, i32, i32),
    /// `d(n, s, 1)`.
    Dice(i32, i32),
}

#[derive(Debug, Clone, Copy)]
enum CoinName {
    Copper,
    Silver,
    Gold,
    /// Platinum or electrum, as the roller's switch says.
    Platinum,
}

#[derive(Debug, Clone, Copy)]
enum Step {
    Coins(CoinName, Amount),
    /// The amount is a number of rolls on the table.
    Rolls(Amount, &'static str),
    /// Fixed numbers of rolls on one or two tables.
    Tables(&'static [(i32, &'static str)]),
}

use Amount::{Dice as D, Fine as F};
use CoinName::{Copper as Cp, Gold as Gp, Platinum as Pp, Silver as Sp};
use Step::{Coins, Rolls, Tables};

/// The treasure types (A 0x54d5c0 .. Z 0x54e6fb; 2.0 0x5d30c0 ..
/// 0x5d4030, identical): each step with the percent chance that guards it.
const TYPES: [&[(Option<i32>, Step)]; 26] = [
    // A
    &[
        (Some(25), Coins(Cp, F(1, 3, 1000))),
        (Some(30), Coins(Sp, F(2, 10, 100))),
        (Some(40), Coins(Gp, F(1, 6, 1000))),
        (Some(35), Coins(Pp, F(3, 6, 100))),
        (Some(60), Rolls(F(1, 4, 10), GEMS)),
        (Some(50), Rolls(D(2, 6), ART)),
        (Some(30), Tables(&[(3, MAGIC)])),
    ],
    // B
    &[
        (Some(50), Coins(Cp, F(1, 6, 1000))),
        (Some(25), Coins(Sp, F(1, 3, 1000))),
        (Some(25), Coins(Gp, F(2, 10, 100))),
        (Some(25), Coins(Pp, F(1, 10, 100))),
        (Some(30), Rolls(D(1, 8), GEMS)),
        (Some(20), Rolls(D(1, 4), ART)),
        (Some(10), Tables(&[(1, ARMOR_OR_WEAPON)])),
    ],
    // C
    &[
        (Some(20), Coins(Cp, F(1, 10, 1000))),
        (Some(30), Coins(Sp, F(1, 6, 1000))),
        (Some(10), Coins(Pp, F(1, 6, 100))),
        (Some(25), Rolls(D(1, 6), GEMS)),
        (Some(20), Rolls(D(1, 3), ART)),
        (Some(10), Tables(&[(2, MAGIC)])),
    ],
    // D
    &[
        (Some(10), Coins(Cp, F(1, 6, 1000))),
        (Some(15), Coins(Sp, F(1, 10, 1000))),
        (Some(50), Coins(Gp, F(1, 3, 1000))),
        (Some(15), Coins(Pp, F(1, 6, 100))),
        (Some(30), Rolls(D(1, 10), GEMS)),
        (Some(25), Rolls(D(1, 6), ART)),
        (Some(15), Tables(&[(2, MAGIC), (1, POTIONS)])),
    ],
    // E
    &[
        (Some(5), Coins(Cp, F(1, 6, 1000))),
        (Some(25), Coins(Sp, F(1, 10, 1000))),
        (Some(25), Coins(Gp, F(1, 4, 1000))),
        (Some(25), Coins(Pp, F(3, 6, 100))),
        (Some(15), Rolls(D(1, 12), GEMS)),
        (Some(10), Rolls(D(1, 6), ART)),
        (Some(25), Tables(&[(3, MAGIC), (1, SCROLLS)])),
    ],
    // F
    &[
        (Some(10), Coins(Sp, F(3, 6, 1000))),
        (Some(40), Coins(Gp, F(1, 6, 1000))),
        (Some(15), Coins(Pp, F(1, 4, 1000))),
        (Some(20), Rolls(D(2, 10), GEMS)),
        (Some(10), Rolls(D(1, 8), ART)),
        (Some(30), Tables(&[(5, NON_WEAPON)])),
    ],
    // G
    &[
        (Some(50), Coins(Gp, F(2, 10, 1000))),
        (Some(50), Coins(Pp, F(1, 10, 1000))),
        (Some(30), Rolls(D(3, 6), GEMS)),
        (Some(25), Rolls(D(1, 6), ART)),
        (Some(35), Tables(&[(5, MAGIC)])),
    ],
    // H
    &[
        (Some(25), Coins(Cp, F(3, 6, 1000))),
        (Some(40), Coins(Sp, F(2, 10, 1000))),
        (Some(55), Coins(Gp, F(2, 10, 1000))),
        (Some(40), Coins(Pp, F(1, 8, 1000))),
        (Some(50), Rolls(D(3, 10), GEMS)),
        (Some(50), Rolls(D(2, 10), ART)),
        (Some(15), Tables(&[(6, MAGIC)])),
    ],
    // I
    &[
        (Some(30), Coins(Pp, F(1, 6, 100))),
        (Some(55), Rolls(D(2, 6), GEMS)),
        (Some(50), Rolls(D(2, 4), ART)),
        (Some(15), Tables(&[(1, MAGIC)])),
    ],
    // J
    &[(None, Coins(Cp, D(3, 8)))],
    // K
    &[(None, Coins(Sp, D(3, 6)))],
    // L
    &[(None, Coins(Pp, D(2, 6)))],
    // M
    &[(None, Coins(Gp, D(2, 4)))],
    // N
    &[(None, Coins(Pp, D(1, 6)))],
    // O
    &[
        (None, Coins(Cp, F(1, 4, 10))),
        (None, Coins(Sp, F(1, 3, 10))),
    ],
    // P
    &[(None, Coins(Sp, F(1, 6, 10))), (None, Coins(Pp, D(1, 20)))],
    // Q
    &[(None, Rolls(D(1, 4), GEMS))],
    // R
    &[
        (None, Coins(Gp, D(2, 10))),
        (None, Coins(Pp, F(1, 6, 10))),
        (None, Rolls(D(2, 4), GEMS)),
        (None, Rolls(D(1, 3), ART)),
    ],
    // S
    &[(None, Rolls(D(1, 8), POTIONS))],
    // T
    &[(None, Rolls(D(1, 4), SCROLLS))],
    // U
    &[
        (Some(90), Rolls(D(2, 8), GEMS)),
        (Some(80), Rolls(D(1, 6), ART)),
        (Some(70), Tables(&[(1, MAGIC)])),
    ],
    // V
    &[(None, Tables(&[(2, MAGIC)]))],
    // W
    &[
        (None, Coins(Gp, D(5, 6))),
        (None, Coins(Pp, D(1, 8))),
        (Some(60), Rolls(D(2, 8), GEMS)),
        (Some(50), Rolls(D(1, 8), ART)),
        (Some(60), Tables(&[(2, MAGIC)])),
    ],
    // X
    &[(None, Tables(&[(2, POTIONS)]))],
    // Y
    &[(None, Coins(Gp, F(2, 6, 100)))],
    // Z
    &[
        (None, Coins(Cp, F(1, 3, 100))),
        (None, Coins(Sp, F(1, 4, 100))),
        (None, Coins(Gp, F(1, 6, 100))),
        (None, Coins(Pp, F(1, 4, 100))),
        (Some(55), Rolls(D(1, 6), GEMS)),
        (Some(50), Rolls(D(2, 6), ART)),
        (Some(50), Tables(&[(3, MAGIC)])),
    ],
];

impl TreasureRules {
    /// The program whose rules apply: the Expansion when the installation
    /// has `UserDbas/AllowedU.dat`, else Core Rules 2.0.
    pub fn flavor(&self) -> Flavor {
        self.flavor
    }

    /// Follows another program's rules (for comparing the two).
    pub fn set_flavor(&mut self, flavor: Flavor) {
        self.flavor = flavor;
    }

    /// The table names, in file order.
    pub fn table_names(&self) -> impl Iterator<Item = &str> {
        self.tables.iter().map(|t| t.name.as_str())
    }

    /// Rolls the treasure types `counts` times each, A to Z, into `bundle`,
    /// then gathers gems and art objects into the "Gems and Art" group
    /// (0x54d199). Pass an empty bundle for a new roll; passing the last
    /// result is the roller's Accumulate Rolls.
    pub fn roll(&self, rand: &mut Rand, counts: &Counts, coin: Coin, bundle: &mut Bundle) {
        for (letter, &count) in counts.iter().enumerate() {
            for _ in 0..count {
                self.roll_type(rand, letter, coin, bundle);
            }
        }
        self.group_gems_and_art(bundle);
    }

    /// One roll of treasure type `letter` (0 for A), without grouping.
    fn roll_type(&self, rand: &mut Rand, letter: usize, coin: Coin, bundle: &mut Bundle) {
        for &(percent, step) in TYPES[letter] {
            // One d100 per guarded step; the step runs on a roll at or
            // under the chance.
            if percent.is_some_and(|p| rand.d(1, 100, 1) > p) {
                continue;
            }
            let amount = |rand: &mut Rand, a: Amount| match a {
                Amount::Fine(n, s, m) => rand.fine(n, s, m),
                Amount::Dice(n, s) => rand.d(n, s, 1),
            };
            match step {
                Coins(name, a) => {
                    let name = match (name, coin) {
                        (CoinName::Copper, _) => COPPER,
                        (CoinName::Silver, _) => SILVER,
                        (CoinName::Gold, _) => GOLD,
                        (CoinName::Platinum, Coin::Platinum) => "* Platinum Pieces",
                        (CoinName::Platinum, Coin::Electrum) => "* Electrum Pieces",
                    };
                    let n = amount(rand, a);
                    bundle.insert(Item::new(name, n as u32), true);
                }
                Rolls(a, table) => {
                    let n = amount(rand, a);
                    self.roll_on(rand, n, table, bundle);
                }
                Tables(list) => {
                    for &(n, table) in list {
                        self.roll_on(rand, n, table, bundle);
                    }
                }
            }
        }
    }

    /// `count` rolls on the table `name`, merged into `bundle`; nothing
    /// when there is no such table (0x54e8bd).
    fn roll_on(&self, rand: &mut Rand, count: i32, name: &str, bundle: &mut Bundle) {
        if let Some(&t) = self.by_name.get(name) {
            for _ in 0..count {
                bundle.insert(self.roll_table_at(rand, t, 0), true);
            }
        }
    }

    /// One roll on the table `name`, as the Edit Treasure Tables View
    /// does; `None` when there is no such table.
    pub fn roll_table(&self, rand: &mut Rand, name: &str) -> Option<Item> {
        let &t = self.by_name.get(name)?;
        Some(self.roll_table_at(rand, t, 0))
    }

    /// Rolls a table (0x5462c7, 2.0 0x5cdbb0).
    fn roll_table_at(&self, rand: &mut Rand, t: usize, depth: u32) -> Item {
        let table = &self.tables[t];
        if table.entries.is_empty() {
            return Item::line(format!("!! Table '{}' is empty.", table.name));
        }
        let total = table
            .entries
            .iter()
            .fold(0i32, |s, e| s.wrapping_add(e.weight as i32));
        let missing = || Item::new(format!("#{}", table.name), 1);
        // The original divides by zero on a table whose weights sum to 0.
        let Some(r) = rand.modulo(total) else {
            return missing();
        };
        let roll = r + 1;
        let mut sum = 0i32;
        let Some(entry) = table.entries.iter().find(|e| {
            sum = sum.wrapping_add(e.weight as i32);
            sum >= roll
        }) else {
            return missing();
        };
        if entry.is_table {
            if depth >= DEPTH_LIMIT {
                return Item::line(format!(
                    "!! Recursion depth limit of {DEPTH_LIMIT} exceeded on table '{}'.",
                    table.name
                ));
            }
            return match self.by_name.get(&entry.item.name) {
                Some(&sub) => self.roll_table_at(rand, sub, depth + 1),
                None => Item::line(format!("!! Table '{}' doesn't exist.", entry.item.name)),
            };
        }
        let mut item = entry.item.clone();
        item.quantity = self.dice(rand, entry.quantity) as u32;
        let n1 = self.dice(rand, entry.sub_tables[0].0);
        let n2 = self.dice(rand, entry.sub_tables[1].0);
        if n1 != 0 || n2 != 0 {
            let contents = item.contents.get_or_insert_with(Bundle::default);
            for (n, (_, name)) in [n1, n2].into_iter().zip(&entry.sub_tables) {
                for _ in 0..n {
                    if let Some(&sub) = self.by_name.get(name) {
                        contents.insert(self.roll_table_at(rand, sub, depth + 1), true);
                    }
                }
            }
        }
        // rand() % 100 + 1 <= chance.
        if entry.intelligent != 0 && rand.rand() % 100 < i32::from(entry.intelligent) {
            intelligent::make_intelligent(rand, &mut item);
        }
        if entry.spell_count != 0 {
            self.write_spells(rand, entry, &mut item);
        }
        item
    }

    /// `bonus + count × (1 + rand() % sides)` (0x54987d, 2.0 0x5d0760). The
    /// Expansion rolls nothing for dice with no sides (0x554d44); 2.0 would
    /// divide by zero, and rolls nothing here either.
    fn dice(&self, rand: &mut Rand, [count, sides, bonus]: [u32; 3]) -> i32 {
        let (count, sides) = (count as i32, sides as i32);
        let mut v = bonus as i32;
        for _ in 0..count {
            let die = match self.flavor {
                Flavor::Expansion if sides < 1 => 0,
                _ => rand.modulo(sides).map_or(0, |r| r + 1),
            };
            v = v.wrapping_add(die);
        }
        v
    }

    /// The spells of a spell scroll row (end of 0x5462c7).
    fn write_spells(&self, rand: &mut Rand, entry: &Entry, item: &mut Item) {
        let (min, max) = (entry.spell_levels.0 as i32, entry.spell_levels.1 as i32);
        let wizard = entry.wizard;
        let extra = item.extra.get_or_insert_with(|| InventoryExtra {
            unknown_0x4: [0; 3],
            unknown_0x10: Vec::new(),
            unknown_0x24: Vec::new(),
            unknown_0x38: Vec::new(),
        });
        for _ in 0..entry.spell_count as i32 {
            let level = match self.flavor {
                Flavor::Expansion => {
                    let span = max.wrapping_sub(min).wrapping_add(1).max(1);
                    let level = (rand.rand() % span + min).max(1);
                    level.min(if wizard { 9 } else { 7 })
                }
                // min+1 to max, no clamps; min == max would divide by
                // zero, taken here as min (0x5cdbb0).
                Flavor::Core20 => match rand.modulo(max.wrapping_sub(min)) {
                    Some(r) => r + 1 + min,
                    None => min,
                },
            };
            let of = |source: usize| -> Vec<&str> {
                self.spells[source]
                    .iter()
                    .filter(|(w, l, _)| *w == wizard && *l == level)
                    .map(|(_, _, n)| n.as_str())
                    .collect()
            };
            let (standard, user) = (of(0), of(1));
            let Some(k) = rand.modulo((standard.len() + user.len()) as i32) else {
                continue;
            };
            let k = k as usize;
            let name = if k < standard.len() {
                standard[k]
            } else {
                // 2.0 indexes the user list with k itself and reads past
                // its end unless it is long enough; such a spell has no
                // name here.
                let i = match self.flavor {
                    Flavor::Expansion => k - standard.len(),
                    Flavor::Core20 => k,
                };
                user.get(i).copied().unwrap_or_default()
            };
            let pages = rand.rand() % 6 + level;
            let pages = pages as u32;
            extra.unknown_0x4[0] = extra.unknown_0x4[0].wrapping_add(pages);
            extra.unknown_0x4[1] = extra.unknown_0x4[1].wrapping_add(pages);
            extra.unknown_0x4[2] = extra.unknown_0x4[2].wrapping_add(1);
            extra.unknown_0x10.push(name.into());
            extra.unknown_0x24.push(pages);
            extra.unknown_0x38.push(u8::from(wizard));
        }
    }

    /// The part type of an item name (0x468356): the first type, in the
    /// program's order, with a part of that name.
    pub fn part_type(&self, name: &str) -> Option<usize> {
        let key = part_key(name);
        TYPE_ORDER
            .into_iter()
            .find(|&t| self.parts[0][t].contains_key(&key) || self.parts[1][t].contains_key(&key))
    }

    /// The part of type `t` named `name`, the user's first (0x465e59).
    fn part(&self, name: &str, t: usize) -> Option<&PartValue> {
        if name.is_empty() {
            return None;
        }
        let key = part_key(name);
        self.parts[0][t]
            .get(&key)
            .or_else(|| self.parts[1][t].get(&key))
    }

    /// Moves the gems and art objects at the top of `bundle` into its
    /// "Gems and Art" group, making the group when there is none
    /// (0x4ab1ad, 2.0 0x4e7250). [`roll`](Self::roll) does this.
    pub fn group_gems_and_art(&self, bundle: &mut Bundle) {
        // The last flagged "Gems and Art" is the group.
        let mut group = None;
        for (i, e) in bundle.items.iter_mut().enumerate() {
            if e.flag && e.name == GEMS_AND_ART {
                e.contents.get_or_insert_with(Bundle::default);
                group = Some(i);
            }
        }
        let items = std::mem::take(&mut bundle.items);
        let mut moved = Vec::new();
        let mut index = None;
        for (i, e) in items.into_iter().enumerate() {
            if Some(i) != group && self.part_type(&e.name) == Some(GEMS_AND_ART_TYPE) {
                moved.push(e);
            } else {
                if Some(i) == group {
                    index = Some(bundle.items.len());
                }
                bundle.items.push(e);
            }
        }
        match index {
            Some(i) => {
                let contents = bundle.items[i].contents.get_or_insert_with(Bundle::default);
                for e in moved {
                    contents.insert(e, true);
                }
            }
            None => {
                let mut group = Item::line(GEMS_AND_ART);
                let mut contents = Bundle::default();
                for e in moved {
                    contents.insert(e, true);
                }
                // A new group is added only when it holds something.
                if !contents.is_empty() {
                    group.contents = Some(contents);
                    bundle.insert(group, true);
                }
            }
        }
    }

    /// Calculate (0x54232d): value, experience and magical items of
    /// everything in `bundle` that is a part, by part type. Flagged lines
    /// count for nothing but their contents do.
    pub fn summary(&self, bundle: &Bundle) -> Summary {
        let mut summary = Summary::default();
        self.add_to_summary(bundle, &mut summary);
        summary
    }

    fn add_to_summary(&self, bundle: &Bundle, s: &mut Summary) {
        for item in &bundle.items {
            if !item.flag
                && let Some(t) = self.part_type(&item.name)
                && let Some(part) = self.part(&item.name, t)
            {
                let qty = item.quantity as i32;
                let [pp, gp, sp, cp] = part.price;
                let value = pp
                    .wrapping_mul(500)
                    .wrapping_add(gp.wrapping_mul(100))
                    .wrapping_add(sp.wrapping_mul(10))
                    .wrapping_add(cp)
                    .wrapping_mul(qty);
                let xp = part.xp.wrapping_mul(qty);
                let totals = &mut s.by_type[t];
                totals.value_cp = totals.value_cp.wrapping_add(value);
                totals.xp = totals.xp.wrapping_add(xp);
                totals.count = totals.count.wrapping_add(qty);
                s.value_cp = s.value_cp.wrapping_add(value);
                s.xp = s.xp.wrapping_add(xp);
                if part.magical {
                    s.magical_items = s.magical_items.wrapping_add(qty);
                }
            }
            if let Some(contents) = &item.contents {
                self.add_to_summary(contents, s);
            }
        }
    }
}

/// A saved treasure bundle (`CTreasureOb`).
#[derive(Debug, Clone, PartialEq)]
pub struct SavedTreasure {
    pub name: String,
    pub bundle: Bundle,
}

/// Why a bundle could not be saved, with the original's messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SaveError {
    #[error("A name must be supplied.")]
    NoName,
    #[error("Name already used.")]
    NameUsed,
}

const SAVED: &str = "UserDbas/TreaBunU.dat";

/// Adds a bundle to the saved list as the Save... button does (0x5427e5,
/// 0x53dcc7): names are compared trimmed and without regard to case, and
/// the new bundle goes at the end (the dialog sorts its list for display).
pub fn add_saved(
    list: &mut Vec<SavedTreasure>,
    name: &str,
    bundle: &Bundle,
) -> std::result::Result<(), SaveError> {
    let key = |n: &str| n.trim().to_ascii_uppercase();
    let wanted = key(name);
    if wanted.is_empty() {
        return Err(SaveError::NoName);
    }
    if list.iter().any(|t| key(&t.name) == wanted) {
        return Err(SaveError::NameUsed);
    }
    list.push(SavedTreasure {
        name: name.into(),
        bundle: bundle.clone(),
    });
    Ok(())
}

/// Reads a `TreaBunU.dat` (a `CObList` of `CTreasureOb`).
pub fn parse_saved(data: &[u8]) -> std::result::Result<Vec<SavedTreasure>, String> {
    let list = ObjectList::parse(data).map_err(|e| e.to_string())?;
    list.objects
        .iter()
        .map(|o| match o {
            Object::Treasure(t) => Ok(SavedTreasure {
                name: t.name.clone(),
                bundle: Bundle::from_inventory(&t.bundle).map_err(|e| e.to_string())?,
            }),
            other => Err(format!("not a saved treasure: {other:?}")),
        })
        .collect()
}

/// The bytes of a `TreaBunU.dat` holding `list`, as the original writes it.
pub fn saved_to_bytes(list: &[SavedTreasure]) -> Result<Vec<u8>> {
    ObjectList {
        objects: list
            .iter()
            .map(|t| {
                Object::Treasure(Treasure {
                    name: t.name.clone(),
                    bundle: t.bundle.to_inventory(),
                })
            })
            .collect(),
    }
    .to_bytes()
    .map_err(|source| Error::Format {
        path: SAVED.into(),
        source,
    })
}

impl Installation {
    /// The saved treasure bundles, in file order; none when there is no
    /// `UserDbas/TreaBunU.dat`.
    pub fn saved_treasures(&self) -> Result<Vec<SavedTreasure>> {
        if self.path(SAVED).is_none() {
            return Ok(Vec::new());
        }
        parse_saved(&self.read(SAVED)?).map_err(|e| Error::Invalid(format!("{SAVED}: {e}")))
    }

    /// Replaces `UserDbas/TreaBunU.dat` with `list` (the original writes it
    /// when the Saved Treasure Bundles dialog closes). The old file is kept
    /// as `TreaBunU.dat.bak`.
    pub fn write_saved_treasures(&self, list: &[SavedTreasure]) -> Result<()> {
        let bytes = saved_to_bytes(list)?;
        if let Some(path) = self.path(SAVED) {
            return crate::edit::replace(&path, &bytes);
        }
        let folder = self
            .path("UserDbas")
            .ok_or_else(|| Error::Invalid("the installation has no UserDbas folder".into()))?;
        crate::create::write_new(&folder.join("TreaBunU.dat"), &bytes)
    }

    /// Saves `bundle` under `name`, refusing a name already used.
    pub fn save_treasure(&self, name: &str, bundle: &Bundle) -> Result<()> {
        let mut list = self.saved_treasures()?;
        add_saved(&mut list, name, bundle).map_err(|e| Error::Invalid(e.to_string()))?;
        self.write_saved_treasures(&list)
    }

    /// Saves `bundle` under `name` in place of the saved bundle `old` (the
    /// one the roller was opened on), keeping its place in the file; a
    /// name used by another bundle is refused.
    pub fn replace_saved_treasure(&self, old: &str, name: &str, bundle: &Bundle) -> Result<()> {
        let mut list = self.saved_treasures()?;
        let Some(index) = list.iter().position(|t| t.name == old) else {
            return self.save_treasure(name, bundle);
        };
        let removed = list.remove(index);
        add_saved(&mut list, name, bundle).map_err(|e| {
            list.insert(index, removed.clone());
            Error::Invalid(e.to_string())
        })?;
        let new = list.pop().expect("just added");
        list.insert(index, new);
        self.write_saved_treasures(&list)
    }

    /// Deletes the saved bundle named `name`.
    pub fn delete_saved_treasure(&self, name: &str) -> Result<()> {
        let mut list = self.saved_treasures()?;
        list.retain(|t| t.name != name);
        self.write_saved_treasures(&list)
    }
}
