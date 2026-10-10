//! A character's lists: proficiencies, languages, spells (known, in
//! spellbooks, memorized), equipment and money, with what the original
//! keeps alongside them.
//!
//! Neither original program checks these names when it opens a character,
//! but only names from the rules databases behave like ones the program
//! chose, so the app offers those. Alongside the lists:
//!
//! * each nonweapon proficiency has a cost, a bonus and a list of details
//!   (languages learned through it) at the same position in three other
//!   arrays; a new one costs what its part says (slots with the Core Rules,
//!   character points with Skills & Powers), as when the program adds it;
//! * the Expansion stores the weight carried and trusts it, so it changes
//!   by the weight of what is added or removed (contents included); Core
//!   Rules 2.0 leaves it at 0 in every file it writes, and so does this;
//! * items stay sorted by name in byte order, and an added item merges into
//!   an item of the same name that holds nothing, as when buying; an item
//!   with pages (a spellbook) gets an empty page record of its part's size;
//! * the Expansion lists the worn armor and worn magical items again after
//!   the worn items change (2.0 leaves both lists empty);
//! * money is the coin items among the spending money, and the coin totals
//!   the equipment page caches follow them;
//! * a spellbook's pages used and spell count follow its spells, and a
//!   book cannot hold more pages than it has;
//! * with Skills & Powers, a nonweapon proficiency is bought with
//!   character points, so adding one spends its cost from the unspent
//!   points and removing one refunds what it cost; a character without
//!   enough points cannot take it;
//! * native languages are kept in alphabetical order, as the program
//!   saves them.
//!
//! Proficiency slots are not checked: the original computes them when it
//! shows the character and simply shows a negative count when there are
//! too many.

use std::collections::HashMap;

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::inventory::{InventoryBundle, InventoryExtra, InventoryItem};
use cr_formats::parts::PartsFile;

use crate::{Error, Installation, Result};

/// Bundles whose weight counts: carried, readied and worn.
const WEIGHED: [usize; 3] = [2, 3, 5];

/// Part types whose parts can be items.
pub const ITEM_TYPES: [&str; 6] = [
    "Equipment",
    "Weapons",
    "Armor",
    "Magical Items",
    "Gems and Art",
    "Currency Types",
];

/// Names of the nine equipment bundles, in `inventory` order.
pub const BUNDLE_NAMES: [&str; 9] = [
    "Animals",
    "Daily Food and Lodging",
    "Items Carried",
    "Items Readied",
    "Items Stored",
    "Items Worn",
    "Spending Money",
    "Services",
    "Transport",
];

/// `inventory` index of the items carried.
pub const CARRIED: usize = 2;

/// `inventory` index of the spending money.
pub const MONEY: usize = 6;

/// `inventory` index of the items worn.
const WORN: usize = 5;

/// Coin item names, copper to platinum, and their worth in copper.
pub const COINS: [(&str, u32); 5] = [
    ("* Copper Pieces", 1),
    ("* Silver Pieces", 10),
    ("* Electrum Pieces", 50),
    ("* Gold Pieces", 100),
    ("* Platinum Pieces", 500),
];

/// Memorized spells per class slot and spell level: wizard, priest.
pub type Memorized = [[(Vec<String>, Vec<String>); 11]; 5];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CharacterLists {
    pub weapon_proficiencies: Vec<String>,
    pub nonweapon_proficiencies: Vec<String>,
    pub languages: Vec<String>,
    pub wizard_spells: Vec<String>,
    pub priest_spells: Vec<String>,
    pub memorized: Memorized,
    /// The items of each bundle; the spending money's coins are in `coins`.
    pub equipment: [Vec<Item>; 9],
    /// Copper, silver, electrum, gold and platinum pieces to spend.
    pub coins: [u32; 5],
    /// The spellbooks among the top-level items.
    pub spellbooks: Vec<Spellbook>,
}

/// An item in a bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    pub quantity: u32,
    /// Its position in the bundle as read, for an item that was there; it
    /// keeps its contents and pages.
    pub original: Option<usize>,
    /// Names of what it holds, for display.
    pub contents: Vec<String>,
}

impl Item {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            quantity: 1,
            original: None,
            contents: Vec::new(),
        }
    }
}

/// A spellbook: an item whose part has pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spellbook {
    /// Where the item is: bundle and position as read.
    pub bundle: usize,
    pub position: usize,
    pub name: String,
    pub capacity: u32,
    /// Spell, pages it takes, whether it is a wizard spell.
    pub spells: Vec<(String, u32, bool)>,
}

impl Spellbook {
    pub fn pages_used(&self) -> u32 {
        self.spells.iter().map(|s| s.1).sum()
    }
}

impl CharacterLists {
    pub fn of(character: &Character) -> Self {
        let d = &character.data;
        Self {
            weapon_proficiencies: d.weapon_proficiencies.clone(),
            nonweapon_proficiencies: d.nonweapon_proficiencies.clone(),
            languages: d.languages.clone(),
            wizard_spells: d.spells.clone(),
            priest_spells: d.priest_spells.clone(),
            memorized: d.unknown_0x3c8.clone(),
            equipment: std::array::from_fn(|b| {
                d.inventory[b]
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| b != MONEY || coin(&item.name).is_none())
                    .map(|(i, item)| Item {
                        name: item.name.clone(),
                        quantity: item.quantity,
                        original: Some(i),
                        contents: contents(item).iter().map(|c| c.name.clone()).collect(),
                    })
                    .collect()
            }),
            coins: std::array::from_fn(|c| {
                d.inventory[MONEY]
                    .items
                    .iter()
                    .filter(|item| coin(&item.name) == Some(c))
                    .map(|item| item.quantity)
                    .sum()
            }),
            spellbooks: d
                .inventory
                .iter()
                .enumerate()
                .flat_map(|(b, bundle)| {
                    bundle
                        .items
                        .iter()
                        .enumerate()
                        .filter_map(move |(i, item)| {
                            let extra = item.extra.as_ref()?;
                            Some(Spellbook {
                                bundle: b,
                                position: i,
                                name: item.name.clone(),
                                capacity: extra.unknown_0x4[0],
                                spells: extra
                                    .unknown_0x10
                                    .iter()
                                    .enumerate()
                                    .map(|(s, name)| {
                                        (
                                            name.clone(),
                                            extra.unknown_0x24.get(s).copied().unwrap_or(0),
                                            extra.unknown_0x38.get(s).copied().unwrap_or(1) != 0,
                                        )
                                    })
                                    .collect(),
                            })
                        })
                })
                .collect(),
        }
    }

    pub fn apply(&self, character: &mut Character, parts: &PartTable) -> Result<()> {
        let skills_and_powers = character.skills_and_powers();
        // Core Rules 2.0 leaves the weight and the worn lists empty.
        let expansion = character.version.starts_with("TSRP3");
        let d = &mut character.data;
        d.weapon_proficiencies
            .clone_from(&self.weapon_proficiencies);
        if d.languages != self.languages {
            d.languages.clone_from(&self.languages);
            d.languages.sort();
        }
        d.spells.clone_from(&self.wizard_spells);
        d.priest_spells.clone_from(&self.priest_spells);

        // Nonweapon proficiencies keep what they had; new ones start as
        // the program starts them.
        let mut kept: Vec<Option<(u32, Option<u32>, Object)>> =
            (0..d.nonweapon_proficiencies.len())
                .map(|i| {
                    Some((
                        d.nonweapon_costs.get(i).copied().unwrap_or(0),
                        d.nonweapon_bonuses
                            .as_ref()
                            .map(|b| b.get(i).copied().unwrap_or(0)),
                        d.nonweapon_details
                            .get(i)
                            .cloned()
                            .unwrap_or(Object::StringArray(Vec::new())),
                    ))
                })
                .collect();
        let (mut names, mut costs, mut bonuses, mut details) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut spent = 0i64;
        for name in &self.nonweapon_proficiencies {
            let old = d
                .nonweapon_proficiencies
                .iter()
                .enumerate()
                .find(|(i, n)| *n == name && kept[*i].is_some())
                .and_then(|(i, _)| kept[i].take());
            let (cost, bonus, detail) = old.unwrap_or_else(|| {
                let cost = parts.nonweapon_cost(name, skills_and_powers);
                spent += i64::from(cost);
                (
                    cost,
                    d.nonweapon_bonuses.as_ref().map(|_| 0),
                    Object::StringArray(Vec::new()),
                )
            });
            names.push(name.clone());
            costs.push(cost);
            bonuses.push(bonus.unwrap_or(0));
            details.push(detail);
        }
        if skills_and_powers {
            let refunded: i64 = kept.iter().flatten().map(|k| i64::from(k.0)).sum();
            let points = i64::from(d.unknown_0x1778[0]) + refunded - spent;
            if points < 0 {
                return Err(Error::Invalid(format!(
                    "Not enough character points: the new proficiencies cost {spent}, \
                     and {} are left",
                    i64::from(d.unknown_0x1778[0]) + refunded
                )));
            }
            d.unknown_0x1778[0] = points as u32;
        }
        d.nonweapon_proficiencies = names;
        d.nonweapon_costs = costs;
        if d.nonweapon_bonuses.is_some() {
            d.nonweapon_bonuses = Some(bonuses);
        }
        d.nonweapon_details = details;

        if d.unknown_0x3c8 != self.memorized {
            d.unknown_0x3c8.clone_from(&self.memorized);
        }

        // Spellbooks first, while positions are as read.
        for book in &self.spellbooks {
            let used = book.pages_used();
            if used > book.capacity {
                return Err(Error::Invalid(format!(
                    "{} has {} pages; these spells take {used}",
                    book.name, book.capacity
                )));
            }
            let Some(extra) = d
                .inventory
                .get_mut(book.bundle)
                .and_then(|b| b.items.get_mut(book.position))
                .and_then(|item| item.extra.as_mut())
            else {
                continue;
            };
            let spells: Vec<String> = book.spells.iter().map(|s| s.0.clone()).collect();
            if extra.unknown_0x10 == spells
                && extra
                    .unknown_0x24
                    .iter()
                    .eq(book.spells.iter().map(|s| &s.1))
            {
                continue;
            }
            extra.unknown_0x4 = [book.capacity, used, spells.len() as u32];
            extra.unknown_0x10 = spells;
            extra.unknown_0x24 = book.spells.iter().map(|s| s.1).collect();
            extra.unknown_0x38 = book.spells.iter().map(|s| u8::from(s.2)).collect();
        }

        // Equipment: rebuild each changed bundle from the items as read and
        // the new ones.
        let weight_before = weigh(&d.inventory, parts, skills_and_powers);
        let worn_before = d.inventory[WORN].clone();
        for (b, wanted) in self.equipment.iter().enumerate() {
            let old = std::mem::take(&mut d.inventory[b].items);
            let mut bundle = InventoryBundle { items: Vec::new() };
            if b == MONEY {
                // Keep whatever is there that is not a coin.
                bundle.items = old
                    .iter()
                    .filter(|i| coin(&i.name).is_none())
                    .cloned()
                    .collect();
            }
            let mut fresh = Vec::new();
            for item in wanted {
                match item.original.and_then(|i| old.get(i)) {
                    Some(kept) if kept.name == item.name => {
                        let mut kept = kept.clone();
                        kept.quantity = item.quantity;
                        bundle.items.push(kept);
                    }
                    _ => fresh.push(parts.new_item(&item.name, item.quantity)),
                }
            }
            if b == MONEY {
                for (c, &count) in self.coins.iter().enumerate() {
                    if count > 0 {
                        fresh.push(parts.new_item(COINS[c].0, count));
                    }
                }
            }
            bundle
                .items
                .sort_by(|x, y| x.name.as_bytes().cmp(y.name.as_bytes()));
            for item in fresh {
                insert_item(&mut bundle, item);
            }
            if b == MONEY {
                let coins_before: Vec<_> = old.iter().filter(|i| coin(&i.name).is_some()).collect();
                let coins_after: Vec<_> = bundle
                    .items
                    .iter()
                    .filter(|i| coin(&i.name).is_some())
                    .collect();
                if coins_before == coins_after {
                    // Same coins: keep them exactly as they were.
                    bundle.items = old;
                }
            }
            d.inventory[b].items = bundle.items;
        }
        if d.inventory[WORN] != worn_before && expansion {
            let (armor, magic) = parts.worn_lists(&d.inventory[WORN]);
            d.armor = armor;
            d.unknown_0x17a4 = magic;
        }
        let weight_change = weigh(&d.inventory, parts, skills_and_powers) - weight_before;
        if weight_change.abs() > 1e-6 && expansion {
            let weight = f64::from(f32::from_bits(d.carried_weight)) + weight_change;
            d.carried_weight = (weight.max(0.0) as f32).to_bits();
        }
        Ok(())
    }
}

/// Adds `item` where the program would: merged into an item of the same
/// name that holds nothing, otherwise before the first item whose name
/// sorts after it byte by byte.
fn insert_item(bundle: &mut InventoryBundle, item: InventoryItem) {
    if item.contents.is_none()
        && let Some(same) = bundle
            .items
            .iter_mut()
            .find(|i| i.name == item.name && i.contents.is_none())
    {
        same.quantity += item.quantity;
        return;
    }
    let position = bundle
        .items
        .iter()
        .position(|i| i.name.as_bytes() > item.name.as_bytes())
        .unwrap_or(bundle.items.len());
    bundle.items.insert(position, item);
}

/// Weight carried as the program counts it, for checking the stored value.
pub fn carried_weight(character: &Character, parts: &PartTable) -> f64 {
    weigh(
        &character.data.inventory,
        parts,
        character.skills_and_powers(),
    )
}

fn weigh(inventory: &[InventoryBundle; 9], parts: &PartTable, skills_and_powers: bool) -> f64 {
    WEIGHED
        .iter()
        .flat_map(|&b| &inventory[b].items)
        .map(|item| parts.item_weight(item, skills_and_powers))
        .sum()
}

/// Which coin an item name is, as an index into [`COINS`].
fn coin(name: &str) -> Option<usize> {
    COINS.iter().position(|c| c.0 == name)
}

fn contents(item: &InventoryItem) -> &[InventoryItem] {
    match item.contents.as_deref() {
        Some(Object::InventoryBundle(bundle)) => &bundle.items,
        _ => &[],
    }
}

/// What the edits need to know about parts, by name.
#[derive(Debug, Default)]
pub struct PartTable {
    /// Core Rules slots and Skills & Powers character points.
    nonweapon_costs: HashMap<String, (u32, u32)>,
    /// Core Rules weight (0 when the part has none) and base weight.
    weights: HashMap<String, (f32, f32)>,
    /// Pages of the parts that have them (spellbooks, scrolls).
    pages: HashMap<String, u32>,
    armor: std::collections::HashSet<String>,
    magical: std::collections::HashSet<String>,
}

impl PartTable {
    fn nonweapon_cost(&self, name: &str, skills_and_powers: bool) -> u32 {
        self.nonweapon_costs.get(name).map_or(
            1,
            |&(core, points)| if skills_and_powers { points } else { core },
        )
    }

    /// A new item as buying makes it: the constructor's values, and an
    /// empty page record for a part with pages.
    fn new_item(&self, name: &str, quantity: u32) -> InventoryItem {
        InventoryItem {
            name: name.into(),
            quantity,
            contents: None,
            unknown_0x8: 999,
            flag_0x4: 0,
            extra: self.pages.get(name).map(|&capacity| InventoryExtra {
                unknown_0x4: [capacity, 0, 0],
                unknown_0x10: Vec::new(),
                unknown_0x24: Vec::new(),
                unknown_0x38: Vec::new(),
            }),
        }
    }

    /// Pages of a part with pages.
    pub fn pages(&self, name: &str) -> Option<u32> {
        self.pages.get(name).copied()
    }

    /// The armor and the other magical items among the worn items, in
    /// order, as the program lists them.
    pub fn worn_lists(&self, worn: &InventoryBundle) -> (Vec<String>, Vec<String>) {
        let mut armor = Vec::new();
        let mut magic = Vec::new();
        for item in &worn.items {
            if self.armor.contains(&item.name) {
                armor.push(item.name.clone());
            } else if self.magical.contains(&item.name) {
                magic.push(item.name.clone());
            }
        }
        (armor, magic)
    }

    /// The weight of an item, its quantity and contents included; an item
    /// the databases lack weighs nothing, as in the original.
    fn item_weight(&self, item: &InventoryItem, skills_and_powers: bool) -> f64 {
        let own = self.weights.get(&item.name).map_or(0.0, |&(core, base)| {
            if !skills_and_powers && core > 0.0 {
                core
            } else {
                base
            }
        });
        let contents = match item.contents.as_deref() {
            Some(Object::InventoryBundle(bundle)) => bundle
                .items
                .iter()
                .map(|i| self.item_weight(i, skills_and_powers))
                .sum(),
            _ => 0.0,
        };
        f64::from(own) * f64::from(item.quantity) + contents
    }
}

impl Installation {
    /// Costs and weights of the parts in the databases and the user's own.
    pub fn part_table(&self) -> Result<PartTable> {
        let mut table = PartTable::default();
        for relative in ["Database/Parts.dat", "UserDbas/PartsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file = PartsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                path: relative.into(),
                source,
            })?;
            let parts = |part_type| {
                file.of_type(part_type)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|o| match o {
                        Object::Part(p) => Some(p),
                        _ => None,
                    })
            };
            for part in parts("Non-Weapon Proficiencies") {
                let cost = part.head.unknown_0x470;
                table
                    .nonweapon_costs
                    .entry(part.head.name.clone())
                    .or_insert((cost[3].max(0) as u32, cost[2].max(0) as u32));
            }
            for part in parts("Armor") {
                table.armor.insert(part.head.name.clone());
            }
            for part in ITEM_TYPES.iter().flat_map(|t| parts(t)) {
                if part.head.unknown_0x224 != 0 {
                    table.magical.insert(part.head.name.clone());
                }
                let pages = part.data.unknown_0x7cc[6];
                if pages > 0 {
                    table
                        .pages
                        .entry(part.head.name.clone())
                        .or_insert(pages as u32);
                }
                let weight = |bits: i32| f32::from_bits(bits as u32);
                table.weights.entry(part.head.name.clone()).or_insert((
                    weight(part.head.unknown_0x1f0),
                    weight(part.head.unknown_0x1ec),
                ));
            }
        }
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BORIN: &[u8] = include_bytes!("../../../tests/fixtures/characters/90688519.dat");

    fn table() -> PartTable {
        let mut table = PartTable::default();
        table.nonweapon_costs.insert("Swimming".into(), (1, 2));
        table.weights.insert("Torch".into(), (0.5, 0.5));
        table.weights.insert("Backpack".into(), (2.0, 2.0));
        table
    }

    fn weight(character: &Character) -> f32 {
        f32::from_bits(character.data.carried_weight)
    }

    #[test]
    fn lists_change_with_what_goes_with_them() {
        let before = Character::parse(BORIN).unwrap();
        let mut lists = CharacterLists::of(&before);
        assert_eq!(lists, CharacterLists::of(&before));

        // Unchanged lists leave the file as it was.
        let mut same = before.clone();
        lists.apply(&mut same, &table()).unwrap();
        assert_eq!(same, before);

        lists.nonweapon_proficiencies.remove(1);
        lists.nonweapon_proficiencies.push("Swimming".into());
        lists.weapon_proficiencies.push("Sword, long".into());
        lists.languages.push("Elf".into());
        lists.equipment[CARRIED].push(Item::new("Torch"));
        lists.equipment[CARRIED].push(Item::new("Torch"));
        let mut after = before.clone();
        lists.apply(&mut after, &table()).unwrap();
        let (b, a) = (&before.data, &after.data);

        assert_eq!(
            a.nonweapon_proficiencies.len(),
            b.nonweapon_proficiencies.len()
        );
        assert_eq!(a.nonweapon_proficiencies.last().unwrap(), "Swimming");
        assert_eq!(a.nonweapon_costs.len(), a.nonweapon_proficiencies.len());
        assert_eq!(a.nonweapon_details.len(), a.nonweapon_proficiencies.len());
        assert_eq!(
            a.nonweapon_bonuses.as_ref().map(Vec::len),
            Some(a.nonweapon_proficiencies.len())
        );
        assert_eq!(a.nonweapon_proficiencies[1], b.nonweapon_proficiencies[2]);
        assert_eq!(a.nonweapon_costs[1], b.nonweapon_costs[2]);
        assert_eq!(*a.nonweapon_costs.last().unwrap(), 1);
        assert_eq!(a.weapon_proficiencies.last().unwrap(), "Sword, long");
        assert_eq!(a.languages, ["Dwarf", "Elf"]);
        // Core Rules: no character points change hands.
        assert_eq!(a.unknown_0x1778, b.unknown_0x1778);

        let torches: Vec<_> = a.inventory[CARRIED]
            .items
            .iter()
            .filter(|i| i.name == "Torch")
            .collect();
        assert_eq!(torches.len(), 1);
        assert_eq!((torches[0].quantity, torches[0].unknown_0x8), (2, 999));
        let names: Vec<_> = a.inventory[CARRIED].items.iter().map(|i| &i.name).collect();
        assert!(names.is_sorted_by(|x, y| x.as_bytes() <= y.as_bytes()));
        assert!((weight(&after) - weight(&before) - 1.0).abs() < 1e-4);

        // Removing the backpack takes its contents' weight too (unknown
        // contents weigh nothing).
        let mut lists = CharacterLists::of(&after);
        lists.equipment[CARRIED].retain(|i| i.name != "Backpack");
        let mut lighter = after.clone();
        lists.apply(&mut lighter, &table()).unwrap();
        let backpack = after.data.inventory[CARRIED]
            .items
            .iter()
            .find(|i| i.name == "Backpack")
            .unwrap();
        let expected = table().item_weight(backpack, false);
        assert!(expected >= 2.0);
        assert!((f64::from(weight(&after) - weight(&lighter)) - expected).abs() < 1e-4);
    }

    const LIRAEL: &[u8] = include_bytes!("../../../tests/fixtures/characters/90988519.dat");

    #[test]
    fn skills_and_powers_proficiencies_cost_character_points() {
        let before = Character::parse(LIRAEL).unwrap();
        let points = before.data.unknown_0x1778[0];
        let mut lists = CharacterLists::of(&before);
        let removed = before.data.nonweapon_costs[0];
        lists.nonweapon_proficiencies.remove(0);
        lists.nonweapon_proficiencies.push("Swimming".into());
        let mut after = before.clone();
        lists.apply(&mut after, &table()).unwrap();
        assert_eq!(after.data.unknown_0x1778[0], points + removed - 2);

        // More than the points left is refused.
        let mut table = table();
        table
            .nonweapon_costs
            .insert("Swimming".into(), (1, points + removed + 1));
        let mut refused = before.clone();
        assert!(lists.apply(&mut refused, &table).is_err());
    }

    #[test]
    fn money_quantities_and_worn_items() {
        let before = Character::parse(BORIN).unwrap();
        let mut lists = CharacterLists::of(&before);
        assert!(lists.coins.iter().any(|&c| c > 0));
        assert!(lists.equipment[MONEY].is_empty());

        // Unchanged: identical file.
        let mut same = before.clone();
        lists.apply(&mut same, &table()).unwrap();
        assert_eq!(same, before);

        lists.coins = [0, 3, 0, 20, 1];
        lists.equipment[CARRIED][0].quantity = 2;
        let worn = lists.equipment[WORN].remove(0);
        lists.equipment[CARRIED].push(Item::new(&worn.name));
        let mut table = table();
        table.armor.extend(before.data.armor.iter().cloned());
        let mut after = before.clone();
        lists.apply(&mut after, &table).unwrap();
        let a = &after.data;

        let money: Vec<_> = a.inventory[MONEY]
            .items
            .iter()
            .map(|i| (i.name.as_str(), i.quantity))
            .collect();
        assert_eq!(
            money,
            [
                ("* Gold Pieces", 20),
                ("* Platinum Pieces", 1),
                ("* Silver Pieces", 3)
            ]
        );
        // The coin totals are left for the equipment page to recompute.
        assert_eq!(a.unknown_0x1778, before.data.unknown_0x1778);
        assert_eq!(a.inventory[CARRIED].items[0].quantity, 2);
        assert!(!a.armor.contains(&worn.name));
        assert_eq!(a.armor.len(), before.data.armor.len() - 1);
        assert_eq!(CharacterLists::of(&after).coins, [0, 3, 0, 20, 1]);
    }

    #[test]
    fn spellbooks_keep_their_page_count() {
        let lirael = Character::parse(LIRAEL).unwrap();
        let mut lists = CharacterLists::of(&lirael);
        assert_eq!(lists.spellbooks.len(), 1);
        let book = &mut lists.spellbooks[0];
        assert_eq!(book.spells, [("Armor".to_string(), 2, true)]);
        book.spells.push(("Burning Hands".into(), 1, true));
        let mut after = lirael.clone();
        lists.apply(&mut after, &table()).unwrap();
        let book =
            &after.data.inventory[lists.spellbooks[0].bundle].items[lists.spellbooks[0].position];
        let extra = book.extra.as_ref().unwrap();
        assert_eq!(extra.unknown_0x4, [100, 3, 2]);
        assert_eq!(extra.unknown_0x24, [2, 1]);
        assert_eq!(extra.unknown_0x38, [1, 1]);

        lists.spellbooks[0].spells.push(("Wish".into(), 98, true));
        assert!(lists.apply(&mut lirael.clone(), &table()).is_err());
    }
}
