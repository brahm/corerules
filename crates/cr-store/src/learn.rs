//! Learning a spell, as the original's spell picker of New Character and
//! Update Character checks it (`CCharSpellsDlg`, 0x62dc71): a spell
//! already known is refused; otherwise each limit the spell passes over
//! asks whether to learn it anyway, in turn, and last the character rolls
//! d100 against the chance to learn it (Knowledge, Table 4) with the
//! class's adjustments. Enter Character learns without any of it.

use std::collections::{HashMap, HashSet};

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::classes::Class;
use cr_formats::parts::PartsFile;
use cr_formats::spells::SpellsFile;
use cr_i18n::{tr, tr_noop};

use crate::create::{Dice, Question};
use crate::npc::data::Kit;
use crate::sheet::tables::LEARN_SPELL;
use crate::{Error, Installation, Result};

/// The original's title of every question but the roll's.
pub const WARNING: &str = tr_noop!("Warning");
/// The message for a spell the character knows.
pub const ALREADY_KNOWN: &str = tr_noop!("You have already learned that spell.");

/// 0x421f1f: the highest spell level by Intelligence (Reason).
const MAX_LEVEL: [i32; 26] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9,
];
/// 0x42205e: the most spells of one level by Intelligence (Reason); none
/// from 19 on.
const MAX_PER_LEVEL: [i32; 26] = [
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    6,
    7,
    7,
    7,
    9,
    9,
    11,
    11,
    14,
    18,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
];
/// What a specialist's class ability adds to or takes from the chance
/// (0x62e1e6).
const SPECIALIST_ADJUSTMENT: i32 = 15;

#[derive(Debug, Clone)]
struct SpellLearn {
    level: i32,
    schools: Vec<String>,
    /// +0x60: never of a barred school (0x630ed9).
    unbarred: bool,
}

#[derive(Debug, Clone, Default)]
struct ClassLearn {
    /// Schools the class may not learn (+0x4898).
    barred: Vec<String>,
    /// Specialist (+0x279d) and the school (+0x2788).
    specialist: bool,
    school: String,
    /// The chance's adjustments by school (+0x4884), which replace the
    /// specialist's.
    chances: Vec<(String, i32)>,
}

/// What the databases say about learning spells, loaded once per
/// installation.
#[derive(Debug, Default)]
pub struct LearnRules {
    wizard: HashMap<String, SpellLearn>,
    priest: HashMap<String, SpellLearn>,
    classes: HashMap<String, ClassLearn>,
    /// Per school part: its opposition schools (+0x1a0) and the spells it
    /// lists (+0x748).
    schools: HashMap<String, (Vec<String>, Vec<String>)>,
    /// Class abilities that add 15% for the specialist's school (+0x7a8)
    /// or take 15% for the others (+0x7a4).
    bonus: HashSet<String>,
    penalty: HashSet<String>,
    /// The kits' change to Intelligence for spells (+0x1f8).
    kit_reason: HashMap<String, i32>,
}

/// The character learning, as the picker sees it.
#[derive(Debug, Clone, Default)]
pub struct Learner {
    /// The class the spell is for, and its abilities.
    pub class: String,
    pub abilities: Vec<String>,
    /// [`crate::advance::AdvanceRules::castable`] for that class.
    pub castable: [bool; 10],
    pub kit: String,
    /// Intelligence, or Reason and Knowledge with Skills & Powers.
    pub reason: u32,
    pub knowledge: u32,
    pub wizard_spells: Vec<String>,
    pub priest_spells: Vec<String>,
    /// Enter Character, which learns without checks (+0x60 == 0x379).
    pub entering: bool,
}

impl Learner {
    /// Reason and Knowledge of `character`, its Intelligence twice
    /// without Skills & Powers.
    pub fn intelligence(character: &Character) -> (u32, u32) {
        let int = character.data.abilities[2];
        if character.skills_and_powers() {
            (int[1], int[2])
        } else {
            (int[0], int[0])
        }
    }
}

/// What learning a spell takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Learning {
    /// The character knows it already ([`ALREADY_KNOWN`]).
    Known,
    /// Learned once each question is answered Yes; none for no question.
    Ask(Vec<Question>),
}

impl Installation {
    pub fn learn_rules(&self) -> Result<LearnRules> {
        let mut rules = LearnRules::default();
        for relative in ["Database/Spells.dat", "UserDbas/SpellsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file =
                SpellsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                    path: relative.into(),
                    source,
                })?;
            for (list, map) in [
                (&file.wizard, &mut rules.wizard),
                (&file.priest, &mut rules.priest),
            ] {
                for o in list {
                    if let Object::Spell(s) = o
                        && !s.name.is_empty()
                    {
                        map.entry(s.name.clone()).or_insert_with(|| SpellLearn {
                            level: s.level,
                            schools: s.schools.clone(),
                            unbarred: s.unknown_0x60 != 0,
                        });
                    }
                }
            }
        }
        for object in self.list_objects("Class")? {
            if let Object::Class(class) = object
                && let Class::Expansion { data, .. } = &*class
            {
                rules
                    .classes
                    .entry(class.name().to_owned())
                    .or_insert_with(|| ClassLearn {
                        barred: data.unknown_0x4898.clone(),
                        specialist: data.flag_0x279d.unwrap_or(0) != 0,
                        school: data.unknown_0x2788.clone(),
                        chances: data
                            .unknown_0x4884
                            .iter()
                            .filter_map(|o| match o {
                                Object::KitBonus(b) => Some((b.name.clone(), b.unknown_0xc)),
                                _ => None,
                            })
                            .collect(),
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
            for object in file.of_type("Schools Of Magic").unwrap_or_default() {
                if let Object::Part(p) = object {
                    let spells = p
                        .data
                        .unknown_0x748
                        .iter()
                        .filter_map(|o| match o {
                            Object::Spell(s) => Some(s.name.clone()),
                            _ => None,
                        })
                        .collect();
                    rules
                        .schools
                        .entry(p.head.name.clone())
                        .or_insert((p.data.unknown_0x1a0.clone(), spells));
                }
            }
            for object in file.of_type("Class Abilities").unwrap_or_default() {
                if let Object::Part(p) = object {
                    if p.data.unknown_0x7a4[1] != 0 {
                        rules.bonus.insert(p.head.name.clone());
                    }
                    if p.data.unknown_0x7a4[0] != 0 {
                        rules.penalty.insert(p.head.name.clone());
                    }
                }
            }
            for object in file.of_type("Character Kits").unwrap_or_default() {
                if let Object::Part(p) = object
                    && let Some(Some(kit)) = &p.data.kit
                    && let Object::PartKit(kit) = &**kit
                    && let Some(data) = &kit.data
                {
                    rules
                        .kit_reason
                        .entry(p.head.name.clone())
                        .or_insert(Kit::of(data).i(0x1f8));
                }
            }
        }
        Ok(rules)
    }
}

impl LearnRules {
    fn spell(&self, name: &str, wizard: bool) -> Option<&SpellLearn> {
        if wizard {
            self.wizard.get(name)
        } else {
            self.priest.get(name)
        }
    }

    /// The level of a spell; 0 for one the databases lack.
    pub fn level(&self, name: &str, wizard: bool) -> i32 {
        self.spell(name, wizard).map_or(0, |s| s.level)
    }

    /// Intelligence (Reason) with the kit's change, kept within 1 to 25
    /// when there is one (0x421f1f).
    fn reason(&self, learner: &Learner) -> usize {
        let mut v = learner.reason as i32;
        let k = self.kit_reason.get(&learner.kit).copied().unwrap_or(0);
        if k != 0 {
            v = (v + k).clamp(1, 25);
        }
        v.clamp(0, 25) as usize
    }

    /// Whether the spell is of a school the class may not learn, or of an
    /// opposition school of the specialist's (0x630ea5).
    fn barred(&self, class: &ClassLearn, spell: &SpellLearn) -> bool {
        if spell.unbarred {
            return false;
        }
        let of = |schools: &[String]| schools.iter().any(|s| spell.schools.contains(s));
        if of(&class.barred) {
            return true;
        }
        class.specialist
            && !class.school.is_empty()
            && self
                .schools
                .get(&class.school)
                .is_some_and(|(opposition, _)| of(opposition))
    }

    /// The chance to learn `name` (0x422498 with 0x630d9f, else the
    /// specialist's 15% from 0x630052).
    pub fn chance(&self, learner: &Learner, name: &str, wizard: bool) -> i32 {
        let base = LEARN_SPELL[learner.knowledge.min(25) as usize];
        let (Some(spell), Some(class)) =
            (self.spell(name, wizard), self.classes.get(&learner.class))
        else {
            return base;
        };
        // The largest adjustment of the spell's schools the class lists.
        let special = class
            .chances
            .iter()
            .filter(|(school, _)| spell.schools.contains(school))
            .map(|c| c.1)
            .max();
        if let Some(special) = special {
            return base + special;
        }
        if class.school.is_empty() {
            return base;
        }
        let has = |set: &HashSet<String>| learner.abilities.iter().any(|a| set.contains(a));
        // The spell's schools as 0x4690ab gathers them: its own and those
        // of the school parts that list it.
        let of_school = spell.schools.contains(&class.school)
            || self
                .schools
                .get(&class.school)
                .is_some_and(|(_, spells)| spells.iter().any(|s| s == name));
        if of_school && has(&self.bonus) {
            base + SPECIALIST_ADJUSTMENT
        } else if !of_school && has(&self.penalty) {
            base - SPECIALIST_ADJUSTMENT
        } else {
            base
        }
    }

    /// What learning `name` takes, with `roll` the d100 against the
    /// chance (0x62dc71).
    pub fn learn(&self, learner: &Learner, name: &str, wizard: bool, roll: u32) -> Learning {
        let known = if wizard {
            &learner.wizard_spells
        } else {
            &learner.priest_spells
        };
        if known.iter().any(|k| k == name) {
            return Learning::Known;
        }
        let mut questions = Vec::new();
        if learner.entering {
            return Learning::Ask(questions);
        }
        let level = self.level(name, wizard);
        let warning = |text: String| Question::new(tr(WARNING), text);
        if !learner
            .castable
            .get(level as usize)
            .copied()
            .unwrap_or(false)
        {
            questions.push(warning(
                tr("The character cannot yet cast spells of this level. Learn anyway?").into(),
            ));
        }
        let reason = self.reason(learner);
        let max = MAX_LEVEL[reason];
        if max < level {
            questions.push(warning(tr!(
                "This spell is level {level}, this character is limited by intelligence to level {max}.  Override limit?",
                level,
                max
            )));
        }
        // Both lists count (0x43147d).
        let count = learner
            .wizard_spells
            .iter()
            .filter(|s| self.level(s, true) == level)
            .chain(
                learner
                    .priest_spells
                    .iter()
                    .filter(|s| self.level(s, false) == level),
            )
            .count() as i32;
        let most = MAX_PER_LEVEL[reason];
        if most <= count {
            let number = count + 1;
            questions.push(warning(tr!(
                "This character can learn up to {most} spells per level.  This will be spell number {number}.  Override limit?",
                most,
                number
            )));
        }
        if let (Some(spell), Some(class)) =
            (self.spell(name, wizard), self.classes.get(&learner.class))
            && self.barred(class, spell)
        {
            questions.push(warning(
                tr("This spell belongs to a school from which this character is banned.  Take the spell anyway?")
                    .into(),
            ));
        }
        if roll as i32 > self.chance(learner, name, wizard) {
            questions.push(Question::new(
                tr("Failed"),
                tr("The character failed to learn the spell.  Learn anyway?").into(),
            ));
        }
        Learning::Ask(questions)
    }

    /// [`Self::learn`] with a d100 from `dice`.
    pub fn learn_rolled(
        &self,
        learner: &Learner,
        name: &str,
        wizard: bool,
        dice: &mut Dice,
    ) -> Learning {
        let roll = dice.roll(1, 100);
        self.learn(learner, name, wizard, roll)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> LearnRules {
        let spell = |level, schools: &[&str]| SpellLearn {
            level,
            schools: schools.iter().map(|s| (*s).to_owned()).collect(),
            unbarred: false,
        };
        let mut rules = LearnRules::default();
        rules
            .wizard
            .insert("Sleep".into(), spell(1, &["Enchantment/Charm"]));
        rules
            .wizard
            .insert("Magic Missile".into(), spell(1, &["Invocation/Evocation"]));
        rules
            .wizard
            .insert("Fireball".into(), spell(3, &["Invocation/Evocation"]));
        rules
            .wizard
            .insert("Shield".into(), spell(1, &["Evocation"]));
        rules.classes.insert(
            "Illusionist".into(),
            ClassLearn {
                specialist: true,
                school: "Illusion/Phantasm".into(),
                ..Default::default()
            },
        );
        rules.schools.insert(
            "Illusion/Phantasm".into(),
            (vec!["Invocation/Evocation".into()], Vec::new()),
        );
        rules.bonus.insert("Specialist".into());
        rules.penalty.insert("Specialist".into());
        rules
    }

    fn learner() -> Learner {
        let mut castable = [false; 10];
        castable[1] = true;
        Learner {
            class: "Mage".into(),
            castable,
            reason: 9,
            knowledge: 9,
            ..Default::default()
        }
    }

    fn asked(learning: Learning) -> usize {
        match learning {
            Learning::Ask(q) => q.len(),
            Learning::Known => usize::MAX,
        }
    }

    #[test]
    fn every_limit_passed_over_asks_in_turn() {
        let rules = rules();
        let mut l = learner();
        assert_eq!(asked(rules.learn(&l, "Sleep", true, 1)), 0);
        // The roll above the chance asks last.
        let chance = rules.chance(&l, "Sleep", true);
        assert_eq!(asked(rules.learn(&l, "Sleep", true, chance as u32 + 1)), 1);
        // A level not yet cast and above Intelligence's: two questions.
        assert_eq!(asked(rules.learn(&l, "Fireball", true, 1)), 1);
        l.reason = 3;
        assert_eq!(asked(rules.learn(&l, "Fireball", true, 1)), 3);
        // Known already: refused.
        l.wizard_spells = vec!["Sleep".into()];
        assert_eq!(rules.learn(&l, "Sleep", true, 1), Learning::Known);
        // Enter Character asks nothing.
        l.entering = true;
        assert_eq!(asked(rules.learn(&l, "Fireball", true, 100)), 0);
    }

    #[test]
    fn a_specialist_learns_its_school_better_and_is_barred_from_the_opposition() {
        let rules = rules();
        let mut l = learner();
        l.class = "Illusionist".into();
        let base = rules.chance(&l, "Sleep", true);
        l.abilities = vec!["Specialist".into()];
        assert!(rules.chance(&l, "Sleep", true) < base);
        assert!(
            asked(rules.learn(&l, "Magic Missile", true, 1))
                > asked(rules.learn(&l, "Shield", true, 1))
        );
    }
}
