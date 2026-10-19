//! The Weapon Proficiencies page of the Expansion's Update Character, with
//! its Specialization, Shield and Armor and Fighting Styles dialogs, and
//! the traits and disadvantages of its Nonweapon Proficiencies page: an
//! existing character's weapon proficiencies, weapon of choice, expertise,
//! specializations, mastery, unarmed combat, shields, armor, fighting
//! styles, traits and disadvantages, and the Skills & Powers character
//! points they cost.
//!
//! Update charges the character's unspent points (`unknown_0x1778[0]`) as
//! each choice is made. What the original charges and refunds (most costs
//! are constants in the program, by class group, the cheapest group of the
//! character's classes winning):
//!
//! * a weapon proficiency costs the cheapest of the classes' prices
//!   (`Class.unknown_0xcc[1]`; Fighter 2), 1 more when none of the classes
//!   may use the weapon, 2 more with a wizard class (3 for a weapon that
//!   charges wizards extra, `flag_0x4d0`). Removing one refunds nothing,
//!   and a proficiency used by a weapon of choice, expertise, mastery or
//!   specialization cannot be removed;
//! * weapon of choice costs warriors 2, wizards 4, the others 3; expertise
//!   warriors 2, wizards 5, the others 4, in full even in the weapon of
//!   choice (the 1 point less the dialog shows is the creation price);
//!   another weapon of choice replaces the old one for nothing;
//! * a specialization costs 2, or 4 multi-classed, for a class that
//!   specializes early (`flag_0x2791`), otherwise warriors 4, priests 6,
//!   rogues and psionicists 8, wizards 10; each unarmed style costs as
//!   much. Mastery needs the weapon specialized and costs 2, or 8
//!   multi-classed, for a class with `flags_0x2793[0]`, otherwise warriors
//!   8 and the others nothing. Unspecializing a weapon drops its mastery;
//! * shields cost warriors 2 and the others 4, armor warriors 2 and the
//!   others 3; whatever is dropped is refunded;
//! * a fighting style costs warriors 2, wizards 4, the others 3; Two
//!   Weapon 1 more unless a class fights with two weapons
//!   (`flags_0x2795[0]`), Two Weapon (+2) 3 more (2 for those classes),
//!   One-handed Weapon (+2) 2 more. A non-warrior may have one style;
//! * traits cost their parts' points and disadvantages give theirs (1
//!   less and 1 more for the race or subrace a part names), with no
//!   15-point limit after creation; removing either one undoes it.
//!
//! Everything is refunded in full when removed except weapon proficiencies.
//! Where the original offers a choice for free or lets the points go below
//! 0 after asking, the app refuses, as on the abilities pages. Kits, weapon
//! groups, racial weapon abilities and the program's level checks are not
//! modelled. Core Rules characters pay slots, which the original only
//! counts; the app charges them nothing and offers only the proficiencies,
//! specializations and unarmed styles their pages show.
//!
//! The lists keep their order with new choices appended, as the original
//! saves them; a fighting style code the app does not know is kept.

use std::collections::HashMap;

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::classes::Class;
use cr_formats::parts::{Part, PartsFile};
use cr_i18n::tr;

use crate::collate::sort_key;
use crate::create::MOST_FIGHTING_STYLES;
use crate::{Error, Installation, Result};

/// The fighting styles of the Fighting Styles dialog and the codes the
/// file keeps them by (`classes.unknown_0x50`).
pub const FIGHTING_STYLES: [(&str, u32); 9] = [
    ("One-handed Weapon", 1993),
    ("Weapon and Shield", 1994),
    ("Two-handed Weapon", 1995),
    ("Two Weapon", 1996),
    ("Missile", 1997),
    ("Horse Archer", 1998),
    ("Thrown Weapon/Sling", 1999),
    ("Two Weapon (+2)", 2001),
    ("One-handed Weapon (+2)", 2002),
];

/// The class group a price list is indexed by: warrior, rogue, priest,
/// wizard, psionicist.
const WARRIOR: usize = 0;
const WIZARD: usize = 3;

#[derive(Debug, Clone, Default)]
struct ClassEntry {
    group: usize,
    /// What a weapon proficiency costs (`unknown_0xcc[1]`).
    weapon_cost: i64,
    /// Specializes from 1st level at a lower price (`flag_0x2791`).
    early_specialization: bool,
    /// Masters weapons at a lower price (`flags_0x2793[0]`).
    early_mastery: bool,
    /// Fights with two weapons without the extra point (`flags_0x2795[0]`).
    two_weapons: bool,
    /// Weapons only from its own list (`unknown_0x3000`), and the list.
    weapon_list: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
struct WeaponEntry {
    /// The classes that may not use the weapon.
    banned: Vec<String>,
    /// A wizard pays 3 points more instead of 2 (`flag_0x4d0`).
    wizard_extra: bool,
}

/// The rules of the page and its dialogs, read from the databases.
#[derive(Debug, Clone, Default)]
pub struct CombatRules {
    classes: HashMap<String, ClassEntry>,
    /// Every weapon, sorted.
    weapons: Vec<(String, WeaponEntry)>,
    /// Every armor, sorted.
    armor: Vec<String>,
    /// Traits and disadvantages with their points and the race they name,
    /// sorted.
    traits: Vec<(String, i64, String)>,
    disadvantages: Vec<(String, i64, String)>,
}

impl Installation {
    pub fn combat_rules(&self) -> Result<CombatRules> {
        let mut rules = CombatRules::default();
        for object in self.list_objects("Class")? {
            if let Object::Class(class) = object
                && let Class::Expansion { data, .. } = *class
            {
                let only_listed = data.unknown_0x3000.as_deref() == Some("Only from list below");
                rules
                    .classes
                    .entry(data.name.clone())
                    .or_insert(ClassEntry {
                        group: data.unknown_0x14[0].clamp(0, 4) as usize,
                        weapon_cost: data.unknown_0xcc[1].into(),
                        early_specialization: data.flag_0x2791.unwrap_or(0) != 0,
                        early_mastery: data.flags_0x2793.is_some_and(|f| f[0] != 0),
                        two_weapons: data.flags_0x2795.is_some_and(|f| f[0] != 0),
                        weapon_list: only_listed.then(|| data.unknown_0x3004.unwrap_or_default()),
                    });
            }
        }
        for relative in ["Database/Parts.dat", "UserDbas/PartsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file = PartsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                path: relative.into(),
                source,
            })?;
            let parts = |part_type: &str| {
                file.of_type(part_type)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|o| match o {
                        Object::Part(p) if !p.head.name.is_empty() => Some(&**p),
                        _ => None,
                    })
                    .collect::<Vec<&Part>>()
            };
            for part in parts("Weapons") {
                // Magical weapons are items, not proficiencies.
                if part.head.unknown_0x224 == 0
                    && !rules.weapons.iter().any(|w| w.0 == part.head.name)
                {
                    rules.weapons.push((
                        part.head.name.clone(),
                        WeaponEntry {
                            banned: part.data.allowed_classes.clone().unwrap_or_default(),
                            wizard_extra: part.data.flag_0x4d0.unwrap_or(0) != 0,
                        },
                    ));
                }
            }
            for part in parts("Armor") {
                let name = part.head.name.to_lowercase();
                if part.head.unknown_0x224 == 0
                    && !name.contains("shield")
                    && !name.contains("buckler")
                    && !rules.armor.contains(&part.head.name)
                {
                    rules.armor.push(part.head.name.clone());
                }
            }
            for part in parts("Traits") {
                if !rules.traits.iter().any(|t| t.0 == part.head.name) {
                    rules.traits.push((
                        part.head.name.clone(),
                        part.head.unknown_0x308.into(),
                        part.data.unknown_0x21c.clone(),
                    ));
                }
            }
            for part in parts("Disadvantages") {
                if !rules.disadvantages.iter().any(|t| t.0 == part.head.name) {
                    rules.disadvantages.push((
                        part.head.name.clone(),
                        part.data.unknown_0x220.into(),
                        part.data.unknown_0x21c.clone(),
                    ));
                }
            }
        }
        rules.weapons.sort_by_cached_key(|w| sort_key(&w.0));
        rules.armor.sort_by_cached_key(|a| sort_key(a));
        rules.traits.sort_by_cached_key(|t| sort_key(&t.0));
        rules.disadvantages.sort_by_cached_key(|t| sort_key(&t.0));
        Ok(rules)
    }
}

/// A character's choices on the page and its dialogs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterCombat {
    pub weapons: Vec<String>,
    pub weapon_of_choice: Option<String>,
    pub expertise: Option<String>,
    pub mastery: Option<String>,
    pub specializations: Vec<String>,
    /// Punching, wrestling and martial arts, as [`crate::create::UNARMED`].
    pub unarmed: [bool; 3],
    /// Buckler, small, medium and body shield, as [`crate::create::SHIELDS`].
    pub shields: [bool; 4],
    pub armor: Vec<String>,
    /// Fighting styles; a code the app does not know is its number.
    pub styles: Vec<String>,
    pub traits: Vec<String>,
    pub disadvantages: Vec<String>,
}

/// What is in `new` and not in `old`, and the other way round, counting
/// repeated names.
fn changes<'a>(old: &'a [String], new: &'a [String]) -> (Vec<&'a String>, Vec<&'a String>) {
    let mut removed: Vec<&String> = old.iter().collect();
    let mut added = Vec::new();
    for name in new {
        match removed.iter().position(|r| *r == name) {
            Some(i) => {
                removed.remove(i);
            }
            None => added.push(name),
        }
    }
    (added, removed)
}

fn named(name: &str) -> Option<String> {
    (!name.is_empty()).then(|| name.to_owned())
}

fn style_name(code: u32) -> String {
    FIGHTING_STYLES
        .iter()
        .find(|s| s.1 == code)
        .map_or_else(|| code.to_string(), |s| s.0.to_owned())
}

fn style_code(name: &str) -> Option<u32> {
    FIGHTING_STYLES
        .iter()
        .find(|s| s.0 == name)
        .map(|s| s.1)
        .or_else(|| name.parse().ok())
}

impl CombatRules {
    fn classes<'a>(&'a self, character: &'a Character) -> impl Iterator<Item = &'a ClassEntry> {
        character
            .data
            .classes
            .names
            .iter()
            .filter_map(|c| self.classes.get(c))
    }

    fn has_group(&self, character: &Character, group: usize) -> bool {
        self.classes(character).any(|c| c.group == group)
    }

    /// The cheapest price of the character's class groups.
    fn cheapest(&self, character: &Character, prices: [i64; 5]) -> i64 {
        self.classes(character)
            .map(|c| prices[c.group])
            .min()
            .unwrap_or(prices[4])
    }

    fn multi_classed(character: &Character) -> bool {
        character.data.classes.names.len() > 1
    }

    /// Every weapon, sorted.
    pub fn weapon_names(&self) -> Vec<String> {
        self.weapons.iter().map(|w| w.0.clone()).collect()
    }

    /// Every armor, sorted.
    pub fn armor_names(&self) -> &[String] {
        &self.armor
    }

    /// The traits and the disadvantages, sorted.
    pub fn trait_names(&self) -> Vec<String> {
        self.traits.iter().map(|t| t.0.clone()).collect()
    }

    pub fn disadvantage_names(&self) -> Vec<String> {
        self.disadvantages.iter().map(|t| t.0.clone()).collect()
    }

    /// The fighting styles the dialog offers.
    pub fn style_names(&self) -> Vec<String> {
        FIGHTING_STYLES.iter().map(|s| s.0.to_owned()).collect()
    }

    /// Whether one of the character's classes may use a weapon.
    pub fn may_use(&self, character: &Character, weapon: &str) -> bool {
        let banned = self
            .weapons
            .iter()
            .find(|w| w.0 == weapon)
            .map_or(&[][..], |w| &w.1.banned[..]);
        character.data.classes.names.iter().any(|class| {
            !banned.contains(class)
                && self
                    .classes
                    .get(class)
                    .and_then(|c| c.weapon_list.as_ref())
                    .is_none_or(|list| list.iter().any(|w| w == weapon))
        })
    }

    /// What a weapon proficiency costs; nothing for a Core Rules
    /// character.
    pub fn weapon_cost(&self, character: &Character, weapon: &str) -> i64 {
        if !character.skills_and_powers() {
            return 0;
        }
        let base = self
            .classes(character)
            .map(|c| match c.weapon_cost {
                cost if cost > 0 => cost,
                _ if c.group == WARRIOR => 2,
                _ => 3,
            })
            .min()
            .unwrap_or(3);
        if self.may_use(character, weapon) {
            base
        } else if !self.has_group(character, WIZARD) {
            base + 1
        } else if self
            .weapons
            .iter()
            .any(|w| w.0 == weapon && w.1.wizard_extra)
        {
            base + 3
        } else {
            base + 2
        }
    }

    pub fn weapon_of_choice_cost(&self, character: &Character) -> i64 {
        self.cheapest(character, [2, 3, 3, 4, 3])
    }

    pub fn expertise_cost(&self, character: &Character) -> i64 {
        self.cheapest(character, [2, 4, 4, 5, 4])
    }

    /// What one specialization, or one unarmed style, costs.
    pub fn specialization_cost(&self, character: &Character) -> i64 {
        if self.classes(character).any(|c| c.early_specialization) {
            if Self::multi_classed(character) { 4 } else { 2 }
        } else {
            self.cheapest(character, [4, 8, 6, 10, 8])
        }
    }

    pub fn mastery_cost(&self, character: &Character) -> i64 {
        if self.classes(character).any(|c| c.early_mastery) {
            if Self::multi_classed(character) { 8 } else { 2 }
        } else if self.has_group(character, WARRIOR) {
            8
        } else {
            0
        }
    }

    pub fn shield_cost(&self, character: &Character) -> i64 {
        if self.has_group(character, WARRIOR) {
            2
        } else {
            4
        }
    }

    pub fn armor_cost(&self, character: &Character) -> i64 {
        if self.has_group(character, WARRIOR) {
            2
        } else {
            3
        }
    }

    pub fn style_cost(&self, character: &Character, style: &str) -> i64 {
        let base = self.cheapest(character, [2, 3, 3, 4, 3]);
        let two_weapons = self.classes(character).any(|c| c.two_weapons);
        let extra = match style {
            "Two Weapon (+2)" if two_weapons => 2,
            "Two Weapon (+2)" => 3,
            "One-handed Weapon (+2)" => 2,
            "Two Weapon" if !two_weapons => 1,
            _ => 0,
        };
        base + extra
    }

    /// Whether the character may only have one fighting style.
    pub fn single_style(&self, character: &Character) -> bool {
        !self.has_group(character, WARRIOR)
    }

    fn race_matches(character: &Character, race: &str) -> bool {
        let d = &character.data;
        !race.is_empty() && (race == d.race || race == d.subrace)
    }

    pub fn trait_cost(&self, character: &Character, name: &str) -> i64 {
        self.traits
            .iter()
            .find(|t| t.0 == name)
            .map_or(0, |t| t.1 - i64::from(Self::race_matches(character, &t.2)))
    }

    pub fn disadvantage_points(&self, character: &Character, name: &str) -> i64 {
        self.disadvantages
            .iter()
            .find(|t| t.0 == name)
            .map_or(0, |t| t.1 + i64::from(Self::race_matches(character, &t.2)))
    }

    /// The character's choices as stored.
    pub fn combat_of(&self, character: &Character) -> CharacterCombat {
        let d = &character.data;
        let flags = d.flags_0x1e54.unwrap_or_default();
        CharacterCombat {
            weapons: d.weapon_proficiencies.clone(),
            weapon_of_choice: named(&d.unknown_0x124[0]),
            expertise: named(&d.unknown_0x124[1]),
            mastery: named(&d.unknown_0x124[2]),
            specializations: d.unknown_0xac[3].clone(),
            unarmed: std::array::from_fn(|i| flags[i] != 0),
            shields: d.unknown_0x134.map(|s| s != 0),
            armor: d.unknown_0xac[4].clone(),
            styles: d
                .classes
                .unknown_0x50
                .iter()
                .filter(|&&c| c != 0)
                .map(|&c| style_name(c))
                .collect(),
            traits: d.unknown_0x1830[0].clone(),
            disadvantages: d.unknown_0x1830[1].clone(),
        }
    }

    /// The points `new` costs compared with what the character has.
    fn spent(&self, character: &Character, new: &CharacterCombat) -> i64 {
        let old = self.combat_of(character);
        let (added, _) = changes(&old.weapons, &new.weapons);
        let mut spent: i64 = added.iter().map(|w| self.weapon_cost(character, w)).sum();
        if !character.skills_and_powers() {
            return spent;
        }
        // Each of the three is charged when set and refunded when cleared;
        // another weapon replaces the old one for nothing.
        let single = |old: &Option<String>, new: &Option<String>, cost: i64| match (old, new) {
            (None, Some(_)) => cost,
            (Some(_), None) => -cost,
            _ => 0,
        };
        spent += single(
            &old.weapon_of_choice,
            &new.weapon_of_choice,
            self.weapon_of_choice_cost(character),
        );
        spent += single(
            &old.expertise,
            &new.expertise,
            self.expertise_cost(character),
        );
        spent += single(&old.mastery, &new.mastery, self.mastery_cost(character));
        let specialization = self.specialization_cost(character);
        let count = |flags: &[bool]| flags.iter().filter(|&&f| f).count() as i64;
        spent += (new.specializations.len() as i64 - old.specializations.len() as i64
            + count(&new.unarmed)
            - count(&old.unarmed))
            * specialization;
        spent += (count(&new.shields) - count(&old.shields)) * self.shield_cost(character);
        spent += (new.armor.len() as i64 - old.armor.len() as i64) * self.armor_cost(character);
        let styles =
            |list: &[String]| -> i64 { list.iter().map(|s| self.style_cost(character, s)).sum() };
        spent += styles(&new.styles) - styles(&old.styles);
        let traits =
            |list: &[String]| -> i64 { list.iter().map(|t| self.trait_cost(character, t)).sum() };
        spent += traits(&new.traits) - traits(&old.traits);
        let disadvantages = |list: &[String]| -> i64 {
            list.iter()
                .map(|t| self.disadvantage_points(character, t))
                .sum()
        };
        spent -= disadvantages(&new.disadvantages) - disadvantages(&old.disadvantages);
        spent
    }

    /// The character points left after `new`; negative when it costs more
    /// than the character has.
    pub fn points_left(&self, character: &Character, new: &CharacterCombat) -> i64 {
        i64::from(character.data.unknown_0x1778[0]) - self.spent(character, new)
    }

    /// Why `new` cannot be saved, if it cannot.
    pub fn problem(&self, character: &Character, new: &CharacterCombat) -> Option<String> {
        let left = self.points_left(character, new);
        if left < 0 {
            return Some(tr!(
                "Not enough character points: the new choices need {more} more",
                more = -left
            ));
        }
        let used = new
            .weapon_of_choice
            .iter()
            .chain(&new.expertise)
            .chain(&new.mastery)
            .chain(&new.specializations);
        if let Some(weapon) = used.into_iter().find(|w| !new.weapons.contains(w)) {
            return Some(tr!(
                "{weapon} needs a weapon proficiency in it",
                weapon = weapon.as_str()
            ));
        }
        if let Some(mastery) = &new.mastery
            && !new.specializations.contains(mastery)
        {
            return Some(tr!(
                "Mastery in {weapon} needs a specialization in it",
                weapon = mastery.as_str()
            ));
        }
        if new.styles.len() > MOST_FIGHTING_STYLES {
            return Some(tr!(
                "A character may know at most {count} fighting styles",
                count = MOST_FIGHTING_STYLES
            ));
        }
        if let Some(style) = new.styles.iter().find(|s| style_code(s).is_none()) {
            return Some(tr!("{name} is not a fighting style", name = style.as_str()));
        }
        None
    }

    /// Writes `new` into the character, as the page and its dialogs do
    /// when they are closed: only what changed is rewritten.
    pub fn apply(&self, character: &mut Character, new: &CharacterCombat) -> Result<()> {
        if let Some(problem) = self.problem(character, new) {
            return Err(Error::Invalid(problem));
        }
        let left = self.points_left(character, new);
        let old = self.combat_of(character);
        let d = &mut character.data;
        d.unknown_0x1778[0] = left as u32;
        if new.weapons != old.weapons {
            d.weapon_proficiencies.clone_from(&new.weapons);
        }
        let dialog = |c: &CharacterCombat| {
            (
                c.weapon_of_choice.clone(),
                c.expertise.clone(),
                c.mastery.clone(),
                c.specializations.clone(),
                c.unarmed,
            )
        };
        if dialog(new) != dialog(&old) {
            d.unknown_0x124 = [
                new.weapon_of_choice.clone().unwrap_or_default(),
                new.expertise.clone().unwrap_or_default(),
                new.mastery.clone().unwrap_or_default(),
            ];
            d.unknown_0xac[3].clone_from(&new.specializations);
            if let Some(flags) = d.flags_0x1e54.as_mut() {
                flags[..3].copy_from_slice(&new.unarmed.map(u8::from));
            }
            // The old single unarmed style; the dialog always clears it.
            d.unknown_0x130 = 0;
        }
        if new.shields != old.shields || new.armor != old.armor {
            d.unknown_0x134 = new.shields.map(u32::from);
            d.unknown_0xac[4].clone_from(&new.armor);
        }
        if new.styles != old.styles {
            d.classes.unknown_0x50 = [0; 10];
            for (slot, style) in d.classes.unknown_0x50.iter_mut().zip(&new.styles) {
                *slot = style_code(style).unwrap_or(0);
            }
        }
        if new.traits != old.traits {
            d.unknown_0x1830[0].clone_from(&new.traits);
        }
        if new.disadvantages != old.disadvantages {
            d.unknown_0x1830[1].clone_from(&new.disadvantages);
        }
        Ok(())
    }
}
