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
//! Where the original lets the points go below 0 after asking, the app
//! refuses, as on the abilities pages; a weapon proficiency or unarmed
//! style it offers for free when the points run short is taken for free.
//! Core Rules characters pay slots, which the original only counts and
//! asks about when none is left; the app charges them nothing.
//!
//! The kit's and classes' weapon rules are New Character's, applied as
//! Update Character applies them (see [`crate::create`]): an existing
//! character is seen as a [`Draft`] at its highest class level, so the
//! kit's free weapons and the specialization level come with the levels.
//! The kit's warnings are asked again unless ignored, which the file
//! keeps; the creation-only ones are left out. The weapon the kit has the
//! character focus in may be changed, and the Cavalier picks its sword at
//! 3rd level and its horseman's weapon at 5th, which are fixed once saved.
//!
//! The lists keep their order with new choices appended, as the original
//! saves them; a fighting style code the app does not know is kept.

use std::collections::HashMap;
use std::sync::Arc;

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::classes::Class;
use cr_formats::parts::{Part, PartsFile};
use cr_i18n::tr;

use crate::advance::AdvanceRules;
use crate::collate::sort_key;
use crate::create::{
    CreationRules, Draft, KitPick, MOST_FIGHTING_STYLES, Question, UNARMED, WeaponCheck,
    WeaponWarning, ignored_checks, write_checks,
};
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
    /// New Character's rules, for the kit's and classes' weapon rules.
    creation: Option<Arc<CreationRules>>,
    /// For the class levels.
    advance: Option<Arc<AdvanceRules>>,
}

/// What taking a weapon proficiency or unarmed style asks and costs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Purchase {
    /// Asked in turn; No to any of them does not take it.
    pub questions: Vec<Question>,
    /// Asked last when the points run short: Yes takes it for nothing.
    pub free: Option<Question>,
    /// The character points it costs.
    pub cost: i64,
}

impl Installation {
    pub fn combat_rules(&self) -> Result<CombatRules> {
        self.combat_rules_with(
            Arc::new(self.creation_rules()?),
            Arc::new(self.advance_rules()?),
        )
    }

    /// [`Self::combat_rules`] with New Character's and the advancement
    /// rules already loaded.
    pub fn combat_rules_with(
        &self,
        creation: Arc<CreationRules>,
        advance: Arc<AdvanceRules>,
    ) -> Result<CombatRules> {
        let mut rules = CombatRules {
            creation: Some(creation),
            advance: Some(advance),
            ..CombatRules::default()
        };
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
    /// The kit's warnings ignored for good.
    pub ignored: Vec<WeaponCheck>,
    /// The last weapon added was from the kit's list for every other one,
    /// or none was yet (`flags_0x1ddc[0]`).
    pub every_other_listed: bool,
    /// The weapon the kit has the character focus in
    /// (`unknown_0x1e68[6]`).
    pub focus: Option<String>,
    /// The weapons the kit has the character pick, as [`KitPick::slot`]
    /// (`unknown_0x1e68[4]` and `[5]`).
    pub kit_picks: [Option<String>; 2],
    /// What each weapon proficiency and unarmed style taken here was
    /// charged, the last one counting.
    pub bought: Vec<(String, i64)>,
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

    /// The character's class levels as stored.
    pub fn levels(&self, character: &Character) -> Vec<u32> {
        self.advance
            .as_ref()
            .and_then(|a| a.levels(character).ok())
            .map_or_else(|| vec![1], |l| l.iter().map(|c| c.level).collect())
    }

    /// The character with `new` chosen, as New Character's rules see it in
    /// Update Character at its class `levels`.
    fn draft(
        &self,
        character: &Character,
        new: &CharacterCombat,
        levels: &[u32],
    ) -> Option<(&CreationRules, Draft)> {
        let rules = self.creation.as_deref()?;
        let mut draft = rules.update_draft(character, levels);
        draft.weapon_proficiencies.clone_from(&new.weapons);
        draft.specializations.clone_from(&new.specializations);
        draft.weapon_of_choice.clone_from(&new.weapon_of_choice);
        draft.expertise.clone_from(&new.expertise);
        draft.unarmed = UNARMED
            .iter()
            .zip(new.unarmed)
            .filter(|u| u.1)
            .map(|u| (*u.0).to_owned())
            .collect();
        draft.ignored_weapon_checks.clone_from(&new.ignored);
        draft.every_other_listed = new.every_other_listed;
        draft.weapon_focus.clone_from(&new.focus);
        Some((rules, draft))
    }

    /// The Core Rules weapon slots left, for a Core Rules character of the
    /// Expansion.
    pub fn core_slots_left(
        &self,
        character: &Character,
        new: &CharacterCombat,
        levels: &[u32],
    ) -> Option<i64> {
        let (rules, draft) = self.draft(character, new, levels)?;
        let dual = self
            .advance
            .as_ref()
            .is_some_and(|a| a.dual_classed(character));
        rules
            .core_specializes(&draft)
            .then(|| rules.core_weapon_slots_left(&draft, levels, dual))
    }

    /// What a weapon proficiency costs at `levels` with `new` chosen;
    /// nothing for a Core Rules character.
    pub fn weapon_price(
        &self,
        character: &Character,
        new: &CharacterCombat,
        levels: &[u32],
        weapon: &str,
    ) -> i64 {
        if !character.skills_and_powers() {
            return 0;
        }
        match self.draft(character, new, levels) {
            Some((rules, draft)) => rules.weapon_cost_for(&draft, weapon).into(),
            None => self.weapon_cost(character, weapon),
        }
    }

    /// What taking a weapon proficiency besides those of `new` asks and
    /// costs (0x444079, 5cc802): the class and kit questions, then with the
    /// Core Rules whether to take it with no slot left, with Skills &
    /// Powers whether to take it for free when the points run short.
    pub fn weapon_purchase(
        &self,
        character: &Character,
        new: &CharacterCombat,
        levels: &[u32],
        weapon: &str,
    ) -> Purchase {
        let cost = self.weapon_price(character, new, levels, weapon);
        let mut purchase = Purchase {
            questions: Vec::new(),
            free: None,
            cost,
        };
        let Some((rules, draft)) = self.draft(character, new, levels) else {
            return purchase;
        };
        purchase.questions = rules.weapon_questions(&draft, &new.weapons, weapon);
        if self
            .core_slots_left(character, new, levels)
            .is_some_and(|left| left <= 0)
            && rules.weapon_takes_slot(&draft, weapon)
        {
            purchase.questions.push(Question::new(
                tr("Warning"),
                tr("No slots are available. Select anyway?").into(),
            ));
        }
        if character.skills_and_powers()
            && cost > self.points_left(character, new) + i64::from(draft.lent_nonweapon)
        {
            purchase.free = Some(Question::new(
                tr("Warning"),
                tr(
                    "You do not have enough Character Points to purchase this weapon \
                     proficiency. Would you like to purchase it for free?",
                )
                .into(),
            ));
        }
        purchase
    }

    /// The kit's warnings taking a weapon proficiency besides those of
    /// `new` gives, leaving out those ignored and the creation-only ones.
    pub fn weapon_warnings(
        &self,
        character: &Character,
        new: &CharacterCombat,
        levels: &[u32],
        weapon: &str,
    ) -> Vec<WeaponWarning> {
        self.draft(character, new, levels)
            .map(|(rules, draft)| rules.weapon_warnings(&draft, &new.weapons, weapon))
            .unwrap_or_default()
    }

    /// Notes a weapon proficiency or unarmed style taken for `cost`, the
    /// kit that wants every other weapon from its list remembering whether
    /// this one was (5d1501). The caller adds it to `new`.
    pub fn bought(
        &self,
        character: &Character,
        new: &mut CharacterCombat,
        levels: &[u32],
        name: &str,
        cost: i64,
    ) {
        if !UNARMED.contains(&name)
            && let Some((rules, mut draft)) = self.draft(character, new, levels)
        {
            rules.weapon_added(&mut draft, &new.weapons, name);
            new.every_other_listed = draft.every_other_listed;
        }
        new.bought.push((name.to_owned(), cost));
    }

    /// The questions before specializing in a weapon (61edf1, 61e897):
    /// whether to with an unarmed style, then the classes' and level's.
    pub fn specialization_questions(
        &self,
        character: &Character,
        new: &CharacterCombat,
        levels: &[u32],
        weapon: &str,
    ) -> Vec<Question> {
        let Some((rules, draft)) = self.draft(character, new, levels) else {
            return Vec::new();
        };
        let mut questions: Vec<Question> =
            rules.specialization_conflict(&draft).into_iter().collect();
        questions.extend(rules.specialization_questions(&draft, weapon));
        questions
    }

    /// The kit's warnings specializing in a weapon gives (61ff56).
    pub fn specialization_warnings(
        &self,
        character: &Character,
        new: &CharacterCombat,
        levels: &[u32],
        weapon: &str,
    ) -> Vec<WeaponWarning> {
        self.draft(character, new, levels)
            .map(|(rules, draft)| rules.specialization_warnings(&draft, weapon))
            .unwrap_or_default()
    }

    /// What taking an unarmed style asks and costs (6208ac): whether the
    /// classes allow it, and whether to have it with a weapon
    /// specialization, which then asks about the slots or points.
    pub fn unarmed_purchase(
        &self,
        character: &Character,
        new: &CharacterCombat,
        levels: &[u32],
    ) -> Purchase {
        let cost = if character.skills_and_powers() {
            self.specialization_cost(character)
        } else {
            0
        };
        let mut purchase = Purchase {
            questions: Vec::new(),
            free: None,
            cost,
        };
        let Some((rules, draft)) = self.draft(character, new, levels) else {
            return purchase;
        };
        purchase.questions = rules.unarmed_questions(&draft);
        if rules.unarmed_conflict(&draft).is_some() {
            if character.skills_and_powers() {
                if cost > self.points_left(character, new) {
                    purchase.free = Some(Question::new(
                        tr("Warning"),
                        tr(
                            "The character does not have enough Character Points. Purchase for \
                            free?",
                        )
                        .into(),
                    ));
                }
            } else if self
                .core_slots_left(character, new, levels)
                .is_some_and(|left| left < 2)
            {
                purchase.questions.push(Question::new(
                    tr("Warning"),
                    tr("You do not have enough slots. Select anyway?").into(),
                ));
            }
        }
        purchase
    }

    /// The weapon the kit has the character focus in: the prompt and the
    /// proficiencies it may be.
    pub fn focus_choices(
        &self,
        character: &Character,
        new: &CharacterCombat,
        levels: &[u32],
    ) -> Option<(String, Vec<String>)> {
        let (rules, draft) = self.draft(character, new, levels)?;
        rules.focus_choices(&draft)
    }

    /// The weapons the kit has the character pick at `levels` that the
    /// file does not keep yet.
    pub fn kit_picks(
        &self,
        character: &Character,
        new: &CharacterCombat,
        levels: &[u32],
    ) -> Vec<KitPick> {
        let stored = self.combat_of(character).kit_picks;
        self.draft(character, new, levels)
            .map(|(rules, draft)| rules.kit_picks(&draft, &stored))
            .unwrap_or_default()
    }

    /// The kit's warnings about the specializations on leaving the page
    /// (5d2bef, 5d2f71).
    pub fn leaving_warnings(
        &self,
        character: &Character,
        new: &CharacterCombat,
        levels: &[u32],
    ) -> Vec<WeaponWarning> {
        self.draft(character, new, levels)
            .map(|(rules, draft)| rules.leaving_warnings(&draft))
            .unwrap_or_default()
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

    /// What a weapon proficiency costs without New Character's rules;
    /// nothing for a Core Rules character.
    fn weapon_cost(&self, character: &Character, weapon: &str) -> i64 {
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
            ignored: ignored_checks(d),
            every_other_listed: d.flags_0x1ddc.is_none_or(|f| f[0] != 0),
            focus: d.unknown_0x1e68.as_ref().and_then(|t| named(&t[6])),
            kit_picks: d
                .unknown_0x1e68
                .as_ref()
                .map(|t| [named(&t[4]), named(&t[5])])
                .unwrap_or_default(),
            bought: Vec::new(),
        }
    }

    /// The points `new` costs compared with what the character has.
    fn spent(&self, character: &Character, new: &CharacterCombat) -> i64 {
        let old = self.combat_of(character);
        let bought = |name: &str| new.bought.iter().rev().find(|b| b.0 == name).map(|b| b.1);
        let (added, _) = changes(&old.weapons, &new.weapons);
        let levels = self.levels(character);
        let mut spent: i64 = added
            .iter()
            .map(|w| bought(w).unwrap_or_else(|| self.weapon_price(character, &old, &levels, w)))
            .sum();
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
        spent +=
            (new.specializations.len() as i64 - old.specializations.len() as i64) * specialization;
        for (i, style) in UNARMED.iter().enumerate() {
            spent += match (old.unarmed[i], new.unarmed[i]) {
                (false, true) => bought(style).unwrap_or(specialization),
                (true, false) => -specialization,
                _ => 0,
            };
        }
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
        // The kit's flags, in the Expansion's files.
        if d.flags_0x1ddc.is_some()
            && (new.ignored != old.ignored || new.every_other_listed != old.every_other_listed)
        {
            write_checks(d, &new.ignored, new.every_other_listed);
        }
        if let Some(texts) = d.unknown_0x1e68.as_mut() {
            if new.focus != old.focus {
                texts[6] = new.focus.clone().unwrap_or_default();
            }
            // A pick kept is never changed.
            for (slot, pick) in new.kit_picks.iter().enumerate() {
                if old.kit_picks[slot].is_none()
                    && let Some(pick) = pick
                {
                    texts[4 + slot].clone_from(pick);
                }
            }
        }
        Ok(())
    }
}
