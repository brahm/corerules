//! A character's lists: proficiencies, languages, spells known and the
//! items carried, with what the original keeps alongside them.
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
//!   an item of the same name that holds nothing, as when buying.
//!
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
use cr_formats::inventory::{InventoryBundle, InventoryItem};
use cr_formats::parts::PartsFile;

use crate::{Error, Installation, Result};

/// `inventory` index of the items carried.
pub const CARRIED: usize = 2;

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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CharacterLists {
    pub weapon_proficiencies: Vec<String>,
    pub nonweapon_proficiencies: Vec<String>,
    pub languages: Vec<String>,
    pub wizard_spells: Vec<String>,
    pub priest_spells: Vec<String>,
    /// Names of the items carried, one per item (an item may be several,
    /// such as 3 torches).
    pub carried: Vec<String>,
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
            carried: d.inventory[CARRIED]
                .items
                .iter()
                .map(|i| i.name.clone())
                .collect(),
        }
    }

    pub fn apply(&self, character: &mut Character, parts: &PartTable) -> Result<()> {
        let skills_and_powers = character.skills_and_powers();
        let keeps_weight = character.version.starts_with("TSRP3");
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

        // Items carried: keep each item whose name is still listed, drop
        // the rest and add the new names.
        let mut wanted = self.carried.clone();
        let mut weight_change = 0.0f64;
        let bundle = &mut d.inventory[CARRIED];
        bundle
            .items
            .retain(|item| match wanted.iter().position(|n| *n == item.name) {
                Some(i) => {
                    wanted.remove(i);
                    true
                }
                None => {
                    weight_change -= parts.item_weight(item, skills_and_powers);
                    false
                }
            });
        for name in wanted {
            let item = InventoryItem {
                name,
                quantity: 1,
                contents: None,
                unknown_0x8: 999,
                flag_0x4: 0,
                extra: None,
            };
            weight_change += parts.item_weight(&item, skills_and_powers);
            insert_item(bundle, item);
        }
        if weight_change != 0.0 && keeps_weight {
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
    let skills_and_powers = character.skills_and_powers();
    WEIGHED
        .iter()
        .flat_map(|&b| &character.data.inventory[b].items)
        .map(|item| parts.item_weight(item, skills_and_powers))
        .sum()
}

/// What the edits need to know about parts, by name.
#[derive(Debug, Default)]
pub struct PartTable {
    /// Core Rules slots and Skills & Powers character points.
    nonweapon_costs: HashMap<String, (u32, u32)>,
    /// Core Rules weight (0 when the part has none) and base weight.
    weights: HashMap<String, (f32, f32)>,
}

impl PartTable {
    fn nonweapon_cost(&self, name: &str, skills_and_powers: bool) -> u32 {
        self.nonweapon_costs.get(name).map_or(
            1,
            |&(core, points)| if skills_and_powers { points } else { core },
        )
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
            for part in ITEM_TYPES.iter().flat_map(|t| parts(t)) {
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
        lists.carried.push("Torch".into());
        lists.carried.push("Torch".into());
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
        lists.carried.retain(|n| n != "Backpack");
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
}
