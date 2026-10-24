//! The Race Abilities and Class Abilities pages of the Expansion's Update
//! Character: an existing character's subrace, racial abilities, class
//! abilities, schools of magic and spheres, and the Skills & Powers
//! character points they cost.
//!
//! The pages share the character's unspent points (`unknown_0x1778[0]`),
//! and the original never lets them go below 0. What each page charges:
//!
//! * the race page charges a subrace's whole package at the subrace's
//!   package cost (`Race.unknown_0xe0`: High elf 40, Gray elf 45), not the
//!   sum of its abilities; the abilities bought on top of it cost what
//!   their parts say. A new subrace brings its package and drops the
//!   abilities bought, refunding them; package abilities cannot be removed.
//!   Only abilities of the character's race with a cost may be bought, and
//!   only by a Skills & Powers character. A Core Rules character is
//!   offered the race's standard subraces only (or the race itself) and
//!   is charged nothing but the package cost. Removing an ability that
//!   names a focus weapon clears the character's focus weapon;
//! * class abilities cost their parts' points; a restriction ("Wizard, No
//!   potions, oils, or scrolls (-5)") costs nothing and gives its points
//!   instead. The original charges them for Core Rules characters too.
//!   Taking a rogue skill ability ("Thief, Pick pockets* (10)") adds an
//!   empty rogue skill entry of that name, and removing it deletes it;
//! * schools cost 5 points each (a constant in the program) unless the
//!   school sets its own cost, and a class limited to one school pays 15
//!   for it. Spheres cost their parts' minor and major prices, or 15 for
//!   the single (major) sphere a mage may buy. Both are free for Core
//!   Rules characters.
//!
//! Every list is saved sorted, as the original's list boxes keep them. The
//! original also asks for confirmation before the first change of each
//! kind ("should only be modified after character creation in exceptional
//! situations"), which the app leaves to its editor.

use std::collections::HashMap;

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::classes::Class;
use cr_formats::parts::{Part, PartsFile};
use cr_i18n::tr;

use crate::collate::sort_key;
use crate::{Error, Installation, Result};

/// What a school costs when its part sets no cost of its own.
const SCHOOL_COST: u32 = 5;
/// What a single school or a mage's single sphere costs.
const SINGLE_COST: u32 = 15;
/// `PartData.unknown_0x778[1]` of the class abilities that are rogue
/// skills.
const ROGUE_SKILL: i32 = 6;

/// How a class buys schools of magic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchoolRule {
    /// Mages, specialists and bards: each school at its cost.
    Each,
    /// Clerics: one school for 15 points.
    Single,
}

/// How a class buys spheres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SphereRule {
    /// Priests: minor and major access at the spheres' prices.
    Full,
    /// Mages: major access to one sphere for 15 points.
    Single,
}

#[derive(Debug, Clone)]
struct RaceEntry {
    name: String,
    /// The race a subrace belongs to.
    parent: Option<String>,
    /// One of the subraces a Core Rules character may have.
    standard: bool,
    package_cost: i64,
    package: Vec<String>,
}

#[derive(Debug, Clone)]
struct RacialPart {
    race: String,
    cost: i64,
    /// The ability names a focus weapon (Attack/Damage bonus).
    focus: bool,
}

#[derive(Debug, Clone, Copy)]
struct ClassPart {
    cost: i64,
    /// The points a restriction gives.
    gain: i64,
    rogue_skill: bool,
}

#[derive(Debug, Clone)]
struct ClassEntry {
    abilities: Vec<String>,
    /// `unknown_0x2774`: full spheres, schools, single sphere, single
    /// school, limited schools.
    flags: [i32; 5],
    /// The schools of a class with limited schools.
    schools: Vec<String>,
}

/// The rules of the two pages, read from the databases.
#[derive(Debug, Clone)]
pub struct AbilityRules {
    races: Vec<RaceEntry>,
    racial: HashMap<String, RacialPart>,
    /// Racial ability names, sorted.
    racial_names: Vec<String>,
    class_parts: HashMap<String, ClassPart>,
    classes: HashMap<String, ClassEntry>,
    /// Every school with its cost, sorted.
    schools: Vec<(String, u32)>,
    /// Every sphere with its minor and major access cost, sorted.
    spheres: Vec<(String, u32, u32)>,
}

impl Installation {
    pub fn ability_rules(&self) -> Result<AbilityRules> {
        let mut races: Vec<RaceEntry> = Vec::new();
        for object in self.list_objects("Race")? {
            if let Object::Race(race) = object
                && !races.iter().any(|r| r.name == race.name)
            {
                races.push(RaceEntry {
                    name: race.name.clone(),
                    parent: (race.unknown_0x184 != 0).then(|| race.unknown_0x188.clone()),
                    standard: race.unknown_0x1e0[0] != 0,
                    package_cost: race.unknown_0xe0.into(),
                    package: race.abilities.clone(),
                });
            }
        }
        let mut classes = HashMap::new();
        for object in self.list_objects("Class")? {
            if let Object::Class(class) = object
                && let Class::Expansion { data, .. } = *class
            {
                classes.entry(data.name.clone()).or_insert(ClassEntry {
                    abilities: data.abilities,
                    flags: data.unknown_0x2774,
                    schools: data.unknown_0x2760,
                });
            }
        }
        let mut rules = AbilityRules {
            races,
            racial: HashMap::new(),
            racial_names: Vec::new(),
            class_parts: HashMap::new(),
            classes,
            schools: Vec::new(),
            spheres: Vec::new(),
        };
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
            for part in parts("Racial Abilities") {
                if !rules.racial.contains_key(&part.head.name) {
                    rules.racial_names.push(part.head.name.clone());
                    rules.racial.insert(
                        part.head.name.clone(),
                        RacialPart {
                            race: part.data.unknown_0x580.clone(),
                            cost: part.head.unknown_0x308.into(),
                            focus: part.data.unknown_0x510[1] != 0
                                || part.data.unknown_0x510[2] != 0,
                        },
                    );
                }
            }
            for part in parts("Class Abilities") {
                rules
                    .class_parts
                    .entry(part.head.name.clone())
                    .or_insert(ClassPart {
                        cost: part.head.unknown_0x308.into(),
                        gain: part.data.unknown_0x220.into(),
                        rogue_skill: part.data.unknown_0x778[1] == ROGUE_SKILL,
                    });
            }
            for part in parts("Schools Of Magic") {
                if !rules.schools.iter().any(|s| s.0 == part.head.name) {
                    let cost = if part.data.unknown_0x218 != 0 {
                        part.head.unknown_0x308.max(0) as u32
                    } else {
                        SCHOOL_COST
                    };
                    rules.schools.push((part.head.name.clone(), cost));
                }
            }
            for part in parts("Spheres") {
                if !rules.spheres.iter().any(|s| s.0 == part.head.name) {
                    rules.spheres.push((
                        part.head.name.clone(),
                        part.data.unknown_0x218.max(0) as u32,
                        part.head.unknown_0x308.max(0) as u32,
                    ));
                }
            }
        }
        rules.racial_names.sort_by_cached_key(|n| sort_key(n));
        rules.schools.sort_by_cached_key(|s| sort_key(&s.0));
        rules.spheres.sort_by_cached_key(|s| sort_key(&s.0));
        Ok(rules)
    }
}

/// A character's choices on the two pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterAbilities {
    pub subrace: String,
    /// Racial abilities bought on top of the subrace's package.
    pub racial: Vec<String>,
    /// One per class.
    pub classes: Vec<ClassAbilities>,
}

/// One class's abilities, schools and spheres.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassAbilities {
    pub abilities: Vec<String>,
    pub schools: Vec<String>,
    pub minor_spheres: Vec<String>,
    pub major_spheres: Vec<String>,
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort_by_cached_key(|n| sort_key(n));
    names
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

impl AbilityRules {
    fn race(&self, name: &str) -> Option<&RaceEntry> {
        self.races.iter().find(|r| r.name == name)
    }

    /// The subraces the race page offers, sorted: a Skills & Powers
    /// character's race and all its subraces, a Core Rules character's
    /// standard subraces (or the race when it has none). The character's
    /// own subrace is always offered, so saving keeps it.
    pub fn subraces(&self, character: &Character) -> Vec<String> {
        let d = &character.data;
        let skills_and_powers = d.skills_and_powers != 0;
        let mut names: Vec<String> = self
            .races
            .iter()
            .filter(|r| r.parent.as_deref() == Some(d.race.as_str()))
            .filter(|r| skills_and_powers || r.standard)
            .map(|r| r.name.clone())
            .collect();
        if skills_and_powers || names.is_empty() {
            names.push(d.race.clone());
        }
        if !d.subrace.is_empty() && !names.contains(&d.subrace) {
            names.push(d.subrace.clone());
        }
        sorted(names)
    }

    /// The package a subrace gives a character of `race`: its abilities
    /// that are abilities of the race.
    pub fn package(&self, race: &str, subrace: &str) -> Vec<String> {
        self.race(subrace).map_or_else(Vec::new, |s| {
            s.package
                .iter()
                .filter(|a| self.racial.get(*a).is_some_and(|p| p.race == race))
                .cloned()
                .collect()
        })
    }

    fn package_cost(&self, subrace: &str) -> i64 {
        self.race(subrace).map_or(0, |s| s.package_cost)
    }

    /// The racial abilities a Skills & Powers character of `race` may buy,
    /// with their cost, sorted; none for a Core Rules character.
    pub fn racial_choices(&self, character: &Character) -> Vec<(String, i64)> {
        let d = &character.data;
        if d.skills_and_powers == 0 {
            return Vec::new();
        }
        self.racial_names
            .iter()
            .filter_map(|n| {
                let p = &self.racial[n];
                (p.race == d.race && p.cost != 0).then(|| (n.clone(), p.cost))
            })
            .collect()
    }

    fn racial_cost(&self, name: &str) -> i64 {
        self.racial.get(name).map_or(0, |p| p.cost)
    }

    /// The class abilities the class page offers a class.
    pub fn class_choices(&self, class: &str) -> Vec<String> {
        self.classes.get(class).map_or_else(Vec::new, |c| {
            c.abilities
                .iter()
                .filter(|a| self.class_parts.contains_key(*a))
                .cloned()
                .collect()
        })
    }

    /// What taking a class ability costs: its points, or minus the points
    /// a restriction gives.
    pub fn class_ability_cost(&self, name: &str) -> i64 {
        self.class_parts
            .get(name)
            .map_or(0, |p| if p.cost != 0 { p.cost } else { -p.gain })
    }

    pub fn school_rule(&self, class: &str) -> Option<SchoolRule> {
        let flags = self.classes.get(class)?.flags;
        if flags[3] != 0 {
            Some(SchoolRule::Single)
        } else if flags[1] != 0 {
            Some(SchoolRule::Each)
        } else {
            None
        }
    }

    /// The schools a class may take, sorted.
    pub fn school_choices(&self, class: &str) -> Vec<String> {
        match self.classes.get(class) {
            Some(c) if c.flags[4] != 0 => sorted(c.schools.clone()),
            _ => self.schools.iter().map(|s| s.0.clone()).collect(),
        }
    }

    pub fn school_cost(&self, class: &str, name: &str) -> u32 {
        match self.school_rule(class) {
            Some(SchoolRule::Single) => SINGLE_COST,
            _ => self
                .schools
                .iter()
                .find(|s| s.0 == name)
                .map_or(SCHOOL_COST, |s| s.1),
        }
    }

    pub fn sphere_rule(&self, class: &str) -> Option<SphereRule> {
        let flags = self.classes.get(class)?.flags;
        if flags[0] != 0 {
            Some(SphereRule::Full)
        } else if flags[2] != 0 {
            Some(SphereRule::Single)
        } else {
            None
        }
    }

    /// Every sphere, sorted.
    pub fn sphere_choices(&self) -> Vec<String> {
        self.spheres.iter().map(|s| s.0.clone()).collect()
    }

    /// What minor and major access to a sphere cost a class.
    pub fn sphere_costs(&self, class: &str, name: &str) -> (u32, u32) {
        match self.sphere_rule(class) {
            Some(SphereRule::Single) => (SINGLE_COST, SINGLE_COST),
            _ => self
                .spheres
                .iter()
                .find(|s| s.0 == name)
                .map_or((0, 0), |s| (s.1, s.2)),
        }
    }

    /// The racial abilities the character bought: those of its abilities
    /// the race page offers, other than the package's.
    fn bought(&self, character: &Character) -> Vec<String> {
        let d = &character.data;
        let package = self.package(&d.race, &d.subrace);
        let choices = self.racial_choices(character);
        d.racial_abilities
            .iter()
            .filter(|a| !package.contains(a) && choices.iter().any(|c| &c.0 == *a))
            .cloned()
            .collect()
    }

    /// The character's choices as stored.
    pub fn abilities_of(&self, character: &Character) -> CharacterAbilities {
        let d = &character.data;
        CharacterAbilities {
            subrace: d.subrace.clone(),
            racial: self.bought(character),
            classes: (0..d.classes.names.len().min(d.class_slots.len()))
                .map(|i| {
                    let slot = &d.class_slots[i];
                    ClassAbilities {
                        abilities: slot.abilities.clone(),
                        schools: slot.unknown_0x270.clone(),
                        minor_spheres: slot.unknown_0x2d4.clone(),
                        major_spheres: slot.unknown_0x338.clone(),
                    }
                })
                .collect(),
        }
    }

    /// The points `new` costs compared with what the character has.
    fn spent(&self, character: &Character, new: &CharacterAbilities) -> i64 {
        let d = &character.data;
        let skills_and_powers = d.skills_and_powers != 0;
        let old = self.abilities_of(character);
        let mut spent = 0;
        if new.subrace != old.subrace || new.racial != old.racial {
            let bought = |a: &CharacterAbilities| -> i64 {
                a.racial.iter().map(|r| self.racial_cost(r)).sum()
            };
            spent += self.package_cost(&new.subrace) - self.package_cost(&old.subrace);
            if skills_and_powers {
                spent += bought(new) - bought(&old);
            }
        }
        for (slot, (class, new)) in d.classes.names.iter().zip(&new.classes).enumerate() {
            let Some(old) = old.classes.get(slot) else {
                continue;
            };
            let (added, removed) = changes(&old.abilities, &new.abilities);
            spent += added
                .iter()
                .map(|a| self.class_ability_cost(a))
                .sum::<i64>()
                - removed
                    .iter()
                    .map(|a| self.class_ability_cost(a))
                    .sum::<i64>();
            if !skills_and_powers {
                continue;
            }
            let schools = |list: &[String]| -> i64 {
                list.iter()
                    .map(|s| i64::from(self.school_cost(class, s)))
                    .sum()
            };
            spent += schools(&new.schools) - schools(&old.schools);
            let spheres = |a: &ClassAbilities| -> i64 {
                a.minor_spheres
                    .iter()
                    .map(|s| i64::from(self.sphere_costs(class, s).0))
                    .chain(
                        a.major_spheres
                            .iter()
                            .map(|s| i64::from(self.sphere_costs(class, s).1)),
                    )
                    .sum()
            };
            spent += spheres(new) - spheres(old);
        }
        spent
    }

    /// The character points left after `new`; negative when it costs more
    /// than the character has.
    pub fn points_left(&self, character: &Character, new: &CharacterAbilities) -> i64 {
        i64::from(character.data.unknown_0x1778[0]) - self.spent(character, new)
    }

    /// Why `new` cannot be saved, if it cannot.
    pub fn problem(&self, character: &Character, new: &CharacterAbilities) -> Option<String> {
        let left = self.points_left(character, new);
        if left < 0 {
            return Some(tr!(
                "Not enough character points: the new abilities need {more} more",
                more = -left
            ));
        }
        for (slot, class) in character.data.classes.names.iter().enumerate() {
            let Some(c) = new.classes.get(slot) else {
                continue;
            };
            if self.school_rule(class) == Some(SchoolRule::Single) && c.schools.len() > 1 {
                return Some(tr!("A {class} may have only one school", class));
            }
            if self.sphere_rule(class) == Some(SphereRule::Single)
                && c.minor_spheres.len() + c.major_spheres.len() > 1
            {
                return Some(tr!("A {class} may have only one sphere", class));
            }
        }
        None
    }

    /// Writes `new` into the character, as the two pages do when they are
    /// left: only what changed is rewritten.
    pub fn apply(&self, character: &mut Character, new: &CharacterAbilities) -> Result<()> {
        if let Some(problem) = self.problem(character, new) {
            return Err(Error::Invalid(problem));
        }
        let left = self.points_left(character, new);
        let old = self.abilities_of(character);
        let d = &mut character.data;
        d.unknown_0x1778[0] = left as u32;
        if new.subrace != old.subrace || new.racial != old.racial {
            let mut racial = self.package(&d.race, &new.subrace);
            racial.extend(new.racial.iter().cloned());
            let lost_focus = d
                .racial_abilities
                .iter()
                .any(|a| !racial.contains(a) && self.racial.get(a).is_some_and(|p| p.focus));
            if lost_focus && let Some(strings) = &mut d.unknown_0x1e68 {
                strings[2].clear();
            }
            d.racial_abilities = sorted(racial);
            d.subrace.clone_from(&new.subrace);
        }
        for (slot, new) in new.classes.iter().enumerate() {
            let (Some(old), Some(stored)) = (old.classes.get(slot), d.class_slots.get_mut(slot))
            else {
                continue;
            };
            if new.abilities != old.abilities {
                let (added, removed) = changes(&old.abilities, &new.abilities);
                let rogue = |a: &String| self.class_parts.get(a).is_some_and(|p| p.rogue_skill);
                for a in removed.into_iter().filter(|a| rogue(a)) {
                    d.rogue_skills.entries.retain(|e| &e.0 != a);
                }
                for a in added.into_iter().filter(|a| rogue(a)) {
                    if !d.rogue_skills.entries.iter().any(|e| &e.0 == a) {
                        d.rogue_skills.entries.push((a.clone(), [0; 100]));
                    }
                }
                stored.abilities = sorted(new.abilities.clone());
            }
            if new.schools != old.schools {
                stored.unknown_0x270 = sorted(new.schools.clone());
            }
            if new.minor_spheres != old.minor_spheres || new.major_spheres != old.major_spheres {
                stored.unknown_0x2d4 = sorted(new.minor_spheres.clone());
                stored.unknown_0x338 = sorted(new.major_spheres.clone());
            }
        }
        Ok(())
    }
}
