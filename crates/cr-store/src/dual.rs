//! Adding a class to a character, as the Expansion's "Dual Class..." button
//! does (`CharDualClass.cpp`, OnOK 0x619f14). The rulebook lets only humans
//! dual-class; a race allows it when `Race.unknown_0x1e0[2]` is set (Human
//! in the shipped data). Multi-class combinations are chosen when the
//! character is created and are never extended afterwards, so demihumans
//! cannot gain a class. Changing race or class is not offered at all.
//!
//! The program writes only the class name, the title and the class slot;
//! experience stays 0 (level 1) and no hit points are rolled until the new
//! class gains levels.

use std::collections::HashSet;

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::classes::Class;

use cr_i18n::{tr, tr_noop};

use crate::{Error, Installation, Result};

/// What adding a class needs to know about one class of the databases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassRules {
    pub name: String,
    /// Database identifier (Fighter 1000, Mage 4000).
    pub id: i32,
    /// 0 warrior, 1 rogue, 2 priest, 3 wizard, 4 psionicist.
    pub group: i32,
    /// Sides of the hit die rolled at 1st level.
    pub hit_die: i32,
    /// Weapon and nonweapon proficiency slots at 1st level.
    pub weapon_slots: i32,
    pub nonweapon_slots: i32,
    pub alignments: Vec<String>,
    /// Minimum Str, Dex, Con, Int, Wis, Cha.
    pub requirements: [i32; 6],
    /// Prime requisites, in the same order.
    pub prime: [bool; 6],
    pub default_abilities: Vec<String>,
    /// Class abilities Skills & Powers offers, and the character points a
    /// new character has to buy them (none in Core Rules 2.0).
    pub abilities: Vec<String>,
    pub points: i32,
    /// Skills & Powers allotments: character points for nonweapon
    /// proficiencies and for weapon proficiencies, and what one weapon
    /// proficiency costs (none in Core Rules 2.0).
    pub nonweapon_points: i32,
    pub weapon_points: i32,
    pub weapon_cost: i32,
    pub major_spheres: Vec<String>,
    pub minor_spheres: Vec<String>,
    pub schools: Vec<String>,
}

impl ClassRules {
    pub(crate) fn of(class: &Class) -> Self {
        match class {
            Class::Expansion { data, .. } => Self {
                name: data.name.clone(),
                id: data.id,
                group: data.unknown_0x14[0],
                hit_die: data.unknown_0x84[0],
                weapon_slots: data.unknown_0xcc[2],
                nonweapon_slots: data.unknown_0xbc[1],
                alignments: data.alignments.clone(),
                requirements: data.ability_requirements,
                prime: data.unknown_0x34.map(|p| p != 0),
                default_abilities: data.default_abilities.clone(),
                abilities: data.abilities.clone(),
                points: data.unknown_0x54[0],
                nonweapon_points: data.unknown_0xbc[0],
                weapon_points: data.unknown_0xcc[0],
                weapon_cost: data.unknown_0xcc[1],
                major_spheres: data.unknown_0x2738.clone(),
                minor_spheres: data.unknown_0x274c.clone(),
                schools: data.unknown_0x2760.clone(),
            },
            // The 2.0 layout keeps the same values in one array, and the
            // same lists in a fixed array of lists.
            Class::Core20(data) => {
                let v = &data.unknown_0x10;
                Self {
                    name: data.name.clone(),
                    id: v[22],
                    group: v[0],
                    hit_die: v[17],
                    weapon_slots: v[29],
                    nonweapon_slots: v[25],
                    alignments: data.unknown_0x58[2].clone(),
                    requirements: v[2..8].try_into().unwrap(),
                    prime: std::array::from_fn(|i| v[8 + i] != 0),
                    default_abilities: data.unknown_0x58[3].clone(),
                    abilities: Vec::new(),
                    points: 0,
                    nonweapon_points: 0,
                    weapon_points: 0,
                    weapon_cost: 0,
                    major_spheres: data.unknown_0x58[4].clone(),
                    schools: data.unknown_0x58[5].clone(),
                    minor_spheres: data.unknown_0x58[6].clone(),
                }
            }
        }
    }
}

/// The highest number of classes a character file has room for.
pub const MAX_CLASSES: usize = 5;

/// Which classes a character may add, or why it may add none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassChoice {
    Allowed(Vec<String>),
    Refused(String),
}

/// The databases' view of classes and races, loaded once.
#[derive(Debug, Default)]
pub struct ClassTable {
    classes: Vec<ClassRules>,
    /// Races that may dual-class.
    dual_races: HashSet<String>,
    /// Single classes `UserDbas/AllowedU.dat` allows per race; races it
    /// does not list (and every race in Core Rules 2.0, which has no such
    /// file) may take any class.
    allowed: Vec<(String, HashSet<String>)>,
    /// Class Abilities parts.
    abilities: HashSet<String>,
}

impl Installation {
    pub fn class_table(&self) -> Result<ClassTable> {
        let mut table = ClassTable::default();
        let mut seen = HashSet::new();
        for object in self.list_objects("Class")? {
            if let Object::Class(class) = object
                && seen.insert(class.name().to_owned())
            {
                table.classes.push(ClassRules::of(&class));
            }
        }
        for object in self.list_objects("Race")? {
            if let Object::Race(race) = object
                && race.unknown_0x1e0[2] != 0
            {
                table.dual_races.insert(race.name.clone());
            }
        }
        for object in self.list_objects("Allowed")? {
            if let Object::ClassesAllowed(allowed) = object {
                let singles = allowed
                    .classes
                    .iter()
                    .filter_map(|o| match o {
                        Object::ClassesAllowedHelper(h) => {
                            let d = &h.data;
                            let names: Vec<_> =
                                [&d.warrior, &d.rogue, &d.priest, &d.wizard, &d.psionicist]
                                    .into_iter()
                                    .filter(|n| !n.is_empty())
                                    .collect();
                            (names.len() == 1).then(|| names[0].clone())
                        }
                        _ => None,
                    })
                    .collect();
                table.allowed.push((allowed.race.clone(), singles));
            }
        }
        table.abilities = self.part_names("Class Abilities")?.into_iter().collect();
        Ok(table)
    }
}

/// The character's Str, Dex, Con, Int, Wis and Cha scores.
fn scores(character: &Character) -> [i32; 6] {
    let d = &character.data;
    let mut scores = [d.strength[0] as i32; 6];
    for (s, a) in scores[1..].iter_mut().zip(&d.abilities) {
        *s = a[0] as i32;
    }
    scores
}

const ABILITIES: [&str; 6] = [
    tr_noop!("Strength"),
    tr_noop!("Dexterity"),
    tr_noop!("Constitution"),
    tr_noop!("Intelligence"),
    tr_noop!("Wisdom"),
    tr_noop!("Charisma"),
];

impl ClassTable {
    fn class(&self, name: &str) -> Option<&ClassRules> {
        self.classes.iter().find(|c| c.name == name)
    }

    /// The classes `character` may add, by the dual-class rules of the
    /// Player's Handbook as the original applies them: a race that may
    /// dual-class, every prime requisite of the first class at 15 or more,
    /// and for the new class a group the character does not have yet, its
    /// ability minimums and every prime requisite at 17 or more. The class
    /// must also be one the race may take.
    pub fn choices(&self, character: &Character) -> ClassChoice {
        let d = &character.data;
        if !self.dual_races.contains(&d.race) {
            return ClassChoice::Refused(tr!(
                "{race} characters cannot take another class: only races that may \
                 dual-class can, and multi-class combinations are chosen at \
                 creation",
                race = d.race
            ));
        }
        if d.classes.names.len() >= MAX_CLASSES {
            return ClassChoice::Refused(tr("A character has room for five classes").into());
        }
        let current: Vec<_> = d
            .classes
            .names
            .iter()
            .filter_map(|n| self.class(n))
            .collect();
        if current.len() != d.classes.names.len() {
            return ClassChoice::Refused(
                tr("One of the character's classes is not in the databases").into(),
            );
        }
        let scores = scores(character);
        if let Some(first) = current.first()
            && let Some(a) = (0..6).find(|&a| first.prime[a] && scores[a] < 15)
        {
            return ClassChoice::Refused(tr!(
                "Dual-classing needs {ability} 15 or more for the first class, {class}",
                ability = tr(ABILITIES[a]),
                class = first.name
            ));
        }
        let allowed = self
            .allowed
            .iter()
            .find(|(race, _)| *race == d.race)
            .map(|(_, classes)| classes);
        let names = self
            .classes
            .iter()
            .filter(|c| {
                !current.iter().any(|k| k.group == c.group)
                    && (0..6).all(|a| scores[a] >= c.requirements[a])
                    && (0..6).all(|a| !c.prime[a] || scores[a] >= 17)
                    && allowed.is_none_or(|allowed| allowed.contains(&c.name))
            })
            .map(|c| c.name.clone())
            .collect::<Vec<_>>();
        ClassChoice::Allowed(sorted(names))
    }

    /// Why `name` is not among the classes `character` may add.
    fn why_not(&self, character: &Character, name: &str) -> String {
        let who = &character.data.name;
        let Some(class) = self.class(name) else {
            return tr!("{name} is not a class in the databases", name);
        };
        let scores = scores(character);
        if let Some(held) = character
            .data
            .classes
            .names
            .iter()
            .find(|n| self.class(n).is_some_and(|c| c.group == class.group))
        {
            return tr!(
                "{who} already has {held}, a class of the same group as {name}",
                who,
                held,
                name
            );
        }
        if let Some(a) = (0..6).find(|&a| scores[a] < class.requirements[a]) {
            return tr!(
                "{name} needs {ability} {score} or more",
                name,
                ability = tr(ABILITIES[a]),
                score = class.requirements[a]
            );
        }
        if let Some(a) = (0..6).find(|&a| class.prime[a] && scores[a] < 17) {
            return tr!(
                "Dual-classing into {name} needs {ability} 17 or more",
                name,
                ability = tr(ABILITIES[a])
            );
        }
        tr!(
            "{race} characters cannot be {name}",
            race = character.data.race,
            name
        )
    }

    /// Adds `name` to `character` as the dual-class dialog does, after
    /// checking it against [`choices`](Self::choices).
    pub fn add(&self, character: &mut Character, name: &str) -> Result<()> {
        match self.choices(character) {
            ClassChoice::Refused(why) => return Err(Error::Invalid(why)),
            ClassChoice::Allowed(names) if !names.iter().any(|n| n == name) => {
                return Err(Error::Invalid(self.why_not(character, name)));
            }
            ClassChoice::Allowed(_) => {}
        }
        let class = self.class(name).expect("allowed classes exist");
        let d = &mut character.data;
        let g = d.classes.names.len();
        d.classes.names.push(name.to_owned());
        d.class_title = format!("{}/{name}", d.class_title);
        let slot = &mut d.class_slots[g];
        slot.abilities = class
            .default_abilities
            .iter()
            .filter(|a| self.abilities.contains(*a))
            .cloned()
            .collect();
        slot.unknown_0x270.extend(class.schools.iter().cloned());
        slot.unknown_0x2d4
            .extend(class.minor_spheres.iter().cloned());
        slot.unknown_0x338
            .extend(class.major_spheres.iter().cloned());
        Ok(())
    }
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort_by_cached_key(|n| crate::collate::sort_key(n));
    names
}
