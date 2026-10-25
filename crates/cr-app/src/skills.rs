//! The Expansion's Skills page with no toolkit in it: the points put into
//! the rogue skills at the current level, the kit's bonuses and chosen
//! skills, a psionicist's disciplines and powers or anyone else's wild
//! talents, and the psionic attack and defense modes. New Character's
//! Psionics step is this page for a psionicist; Update Character's Skills
//! page is it for any character.
//!
//! The rules are `cr-store`'s ([`SkillRules`]); the form keeps what is
//! edited and says what each group shows.

use std::sync::Arc;

use cr_formats::character::Character;
use cr_i18n::tr;
use cr_store::create::{Dice, Question};
use cr_store::skills::wild::{WildMode, WildTest};
use cr_store::skills::{CharacterSkills, SkillRules};

use crate::create::{Choice, List, ListKind};

/// The most points a rogue skill takes on the page.
pub const MOST_POINTS: i32 = 250;

/// "Telepathic devotion, from level 3", for a power.
pub fn power_text(rules: &SkillRules, name: &str) -> String {
    let Some(power) = rules.power(name) else {
        return String::new();
    };
    let discipline = if power.discipline.is_empty() {
        tr("Metapsionic")
    } else {
        tr(&power.discipline)
    };
    let kind = if power.devotion {
        tr!("{discipline} devotion", discipline)
    } else {
        tr!("{discipline} science", discipline)
    };
    if power.level > 0 {
        tr!("{kind}, from level {level}", kind, level = power.level)
    } else {
        kind
    }
}

/// A rogue skill's row.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillRow {
    pub label: String,
    pub points: i32,
    /// The fewest points it may be lowered to.
    pub least: i32,
    /// The score, and what the kit and the levels add to it.
    pub subtitle: String,
}

/// The rogue skills of the current level.
#[derive(Debug, Clone, PartialEq)]
pub struct RogueGroup {
    pub description: String,
    /// The points left; below 0, too many are spent.
    pub left: i64,
    pub rows: Vec<SkillRow>,
    pub warnings: Vec<String>,
}

/// One skill of a list to pick, or none.
#[derive(Debug, Clone, PartialEq)]
pub struct BonusGroup {
    pub title: String,
    pub description: String,
    pub names: Vec<String>,
    pub picked: Option<usize>,
}

/// Skills to tick, up to a count.
#[derive(Debug, Clone, PartialEq)]
pub struct ChosenGroup {
    pub title: String,
    pub description: String,
    pub names: Vec<String>,
    pub ticked: Vec<bool>,
    /// As many are ticked as may be: the others are off.
    pub full: bool,
}

/// The Wild Talents dialog's Test and Roll, for a character who is no
/// psionicist.
#[derive(Debug, Clone, PartialEq)]
pub struct WildGroup {
    /// The chance a test makes the character a wild talent.
    pub chance: String,
    /// A wild talent already is not tested, as in the original.
    pub can_test: bool,
}

/// The Skills page as edited.
#[derive(Clone)]
pub struct SkillsForm {
    rules: Arc<SkillRules>,
    /// The character as edited elsewhere: its classes, levels and
    /// abilities.
    character: Character,
    /// The choices as edited.
    skills: CharacterSkills,
    /// A wild talent's powers, stored as sciences and devotions.
    talents: Vec<String>,
    /// The kit's chosen skills ticked.
    chosen: Vec<String>,
}

impl SkillsForm {
    pub fn new(rules: Arc<SkillRules>, character: Character) -> Self {
        let skills = rules.skills_of(&character);
        let talents = if rules.psionic_level(&character).is_some() {
            Vec::new()
        } else {
            skills
                .sciences
                .iter()
                .chain(&skills.devotions)
                .cloned()
                .collect()
        };
        Self {
            rules,
            character,
            skills,
            talents,
            chosen: Vec::new(),
        }
    }

    pub fn rules(&self) -> &Arc<SkillRules> {
        &self.rules
    }

    pub fn character(&self) -> &Character {
        &self.character
    }

    /// The character as now edited elsewhere: the abilities a test or
    /// another page changed.
    pub fn set_character(&mut self, character: Character) {
        self.character = character;
    }

    /// The choices as edited.
    pub fn skills(&self) -> CharacterSkills {
        let mut skills = self.skills.clone();
        if self.psionic_level().is_none() {
            (skills.devotions, skills.sciences) = self
                .talents
                .iter()
                .cloned()
                .partition(|n| self.rules.power(n).is_some_and(|p| p.devotion));
        }
        skills
    }

    /// Why the choices cannot be kept, if they cannot.
    pub fn problem(&self) -> Option<String> {
        self.rules.problem(&self.character, &self.skills())
    }

    fn psionic_level(&self) -> Option<u32> {
        self.rules.psionic_level(&self.character)
    }

    /// The rogue skills, for a character with any.
    pub fn rogue(&self) -> Option<RogueGroup> {
        let (_, level) = self.rules.rogue_level(&self.character)?;
        let character = &self.character;
        let new = self.skills();
        let mut description = tr!(
            "The points put in at level {level}; lowering a skill gives its points back.",
            level
        );
        if let Some(limit) = self.rules.rogue_limit(character, &new) {
            description.push(' ');
            description.push_str(&tr!("One skill should get at most {limit} of them.", limit));
        }
        let minimums = self.rules.rogue_minimums(character, &new);
        let kit_points = self.rules.rogue_kit_points(character, &new);
        let rows = self
            .rules
            .rogue_skills(character)
            .into_iter()
            .zip(&new.rogue)
            .zip(minimums.iter().chain(std::iter::repeat(&0)))
            .zip(kit_points.iter().map(Some).chain(std::iter::repeat(None)))
            .map(|(((skill, &points), &least), kit)| {
                let mut subtitle = tr!("Score {score}%", score = skill.base_score + points);
                if let Some((adjustment, levels)) = kit {
                    if *adjustment != 0 {
                        subtitle.push_str(" · ");
                        subtitle.push_str(&tr!(
                            "kit modifier {points}",
                            points = format!("{adjustment:+}")
                        ));
                    }
                    for (level, bonus) in levels {
                        subtitle.push_str(" · ");
                        subtitle.push_str(&tr!(
                            "level {level}: {points}",
                            level = *level,
                            points = format!("{bonus:+}")
                        ));
                    }
                }
                SkillRow {
                    label: skill.label,
                    points,
                    least: least.min(points),
                    subtitle,
                }
            })
            .collect();
        Some(RogueGroup {
            description,
            left: self.rules.rogue_points_left(character, &new),
            rows,
            warnings: self.rules.rogue_warnings(character, &new),
        })
    }

    /// A skill's label, for a kit's name of it.
    fn label(&self, name: &str) -> String {
        self.rules
            .rogue_skills(&self.character)
            .into_iter()
            .find(|s| s.name == name)
            .map_or_else(|| name.to_owned(), |s| s.label)
    }

    /// The kit's bonus of each level still to go to a skill (62cc45).
    pub fn kit_bonuses(&self) -> Vec<BonusGroup> {
        if self.rules.rogue_level(&self.character).is_none() {
            return Vec::new();
        }
        self.rules
            .rogue_bonus_picks(&self.character)
            .into_iter()
            .map(|pick| {
                let picked = self
                    .skills
                    .kit_bonuses
                    .get(pick.level as usize - 1)
                    .and_then(Option::as_ref)
                    .and_then(|b| pick.names.iter().position(|n| *n == b.name));
                BonusGroup {
                    title: tr!("Kit Bonus at Level {level}", level = pick.level),
                    description: tr!(
                        "The selected kit allows you to add {points} point(s) to one rogue \
                         skill from the following list at level {level}.",
                        points = pick.amount,
                        level = pick.level
                    ),
                    names: pick.names.iter().map(|n| self.label(n)).collect(),
                    picked,
                }
            })
            .collect()
    }

    /// The skills the kit has the character choose (62c8e6).
    pub fn chosen(&self) -> Option<ChosenGroup> {
        self.rules.rogue_level(&self.character)?;
        let pick = self.rules.chosen_pick(&self.character)?;
        let ticked: Vec<bool> = pick.names.iter().map(|n| self.chosen.contains(n)).collect();
        Some(ChosenGroup {
            title: tr("Chosen Rogue Skills").to_owned(),
            description: tr!(
                "The selected kit allows you to select {count} skills as 'chosen' for a \
                 {bonus}% bonus.",
                count = pick.count,
                bonus = pick.bonus
            ),
            names: pick.names.iter().map(|n| self.label(n)).collect(),
            full: ticked.iter().filter(|t| **t).count() >= pick.count,
            ticked,
        })
    }

    /// What the psionicist level allows, for a psionicist.
    pub fn psionics(&self) -> Option<String> {
        let level = self.psionic_level()?;
        let (disciplines, sciences, devotions) = self.rules.allowed(&self.character);
        Some(tr!(
            "At psionicist level {level}: up to {disciplines} disciplines, {sciences} sciences \
             and {devotions} devotions.",
            level,
            disciplines,
            sciences,
            devotions
        ))
    }

    /// The primary discipline, and a psychic adept's second, among the
    /// disciplines taken; "None" first.
    pub fn primary(&self) -> Vec<Choice> {
        if self.psionic_level().is_none() {
            return Vec::new();
        }
        let titles = [tr("Primary Discipline"), tr("Second Primary Discipline")];
        let count = if self.rules.adept(&self.character) {
            2
        } else {
            1
        };
        titles
            .iter()
            .zip(&self.skills.primary)
            .take(count)
            .map(|(title, selected)| {
                let mut items = vec![tr("None").to_owned()];
                items.extend(self.skills.disciplines.iter().cloned());
                Choice {
                    title: (*title).to_owned(),
                    description: String::new(),
                    selected: self
                        .skills
                        .disciplines
                        .iter()
                        .position(|d| d == selected)
                        .map_or(0, |i| i + 1),
                    items,
                    shown: true,
                }
            })
            .collect()
    }

    /// The names a list holds.
    fn names(&self, kind: ListKind) -> &[String] {
        let s = &self.skills;
        match kind {
            ListKind::Disciplines => &s.disciplines,
            ListKind::Sciences => &s.sciences,
            ListKind::Devotions => &s.devotions,
            ListKind::WildTalents => &self.talents,
            ListKind::Attacks => &s.attacks,
            ListKind::Defenses => &s.defenses,
            _ => &[],
        }
    }

    fn names_mut(&mut self, kind: ListKind) -> Option<&mut Vec<String>> {
        let s = &mut self.skills;
        Some(match kind {
            ListKind::Disciplines => &mut s.disciplines,
            ListKind::Sciences => &mut s.sciences,
            ListKind::Devotions => &mut s.devotions,
            ListKind::WildTalents => &mut self.talents,
            ListKind::Attacks => &mut s.attacks,
            ListKind::Defenses => &mut s.defenses,
            _ => return None,
        })
    }

    /// Whether the page has the list.
    fn has(&self, kind: ListKind) -> bool {
        let psionicist = self.psionic_level().is_some();
        match kind {
            ListKind::Disciplines | ListKind::Sciences | ListKind::Devotions => psionicist,
            ListKind::WildTalents => !psionicist,
            // A non-psionicist has attack and defense modes with wild
            // talents.
            ListKind::Attacks | ListKind::Defenses => {
                psionicist
                    || !self.skills.attacks.is_empty()
                    || !self.skills.defenses.is_empty()
                    || !self.talents.is_empty()
            }
            _ => false,
        }
    }

    /// What Add offers on a list.
    fn offer(&self, kind: ListKind) -> Vec<String> {
        let (disciplines, sciences, devotions) = self.rules.allowed(&self.character);
        let room = |most: usize| self.names(kind).len() < most;
        let offer = match kind {
            ListKind::Disciplines if room(disciplines) => self.rules.discipline_names(),
            ListKind::Sciences if room(sciences) => {
                self.rules
                    .power_choices(&self.character, &self.skills.disciplines, false)
            }
            ListKind::Devotions if room(devotions) => {
                self.rules
                    .power_choices(&self.character, &self.skills.disciplines, true)
            }
            ListKind::WildTalents => self.rules.wild_talent_choices(),
            ListKind::Attacks => self.rules.attack_names().to_vec(),
            ListKind::Defenses => self.rules.defense_names().to_vec(),
            _ => Vec::new(),
        };
        let taken = self.names(kind);
        offer.into_iter().filter(|n| !taken.contains(n)).collect()
    }

    /// One of the page's lists: the disciplines, sciences, devotions, wild
    /// talents, attack modes or defense modes.
    pub fn list(&self, kind: ListKind) -> List {
        let (title, description) = match kind {
            ListKind::Disciplines => (tr("Disciplines"), ""),
            ListKind::Sciences => (tr("Sciences"), ""),
            ListKind::Devotions => (tr("Devotions"), ""),
            ListKind::WildTalents => (
                tr("Your Wild Talents"),
                tr(
                    "Any character but a psionicist may have wild talents. A character \
                     without PSPs rolls them when the first is taken.",
                ),
            ),
            ListKind::Attacks => (tr("Psionic Attack Modes"), ""),
            ListKind::Defenses => (tr("Psionic Defense Modes"), ""),
            _ => ("", ""),
        };
        let powers = matches!(
            kind,
            ListKind::Sciences | ListKind::Devotions | ListKind::WildTalents
        );
        List {
            title: title.to_owned(),
            description: description.to_owned(),
            rows: self
                .names(kind)
                .iter()
                .map(|n| {
                    let text = if powers {
                        power_text(&self.rules, n)
                    } else {
                        String::new()
                    };
                    (n.clone(), text, None)
                })
                .collect(),
            choices: self.offer(kind),
            shown: self.has(kind),
            fixed: false,
        }
    }

    /// The Wild Talents dialog's buttons, for a character who is no
    /// psionicist.
    pub fn wild(&self) -> Option<WildGroup> {
        if self.psionic_level().is_some() {
            return None;
        }
        let chance = self.rules.wild_chance(&self.character);
        Some(WildGroup {
            chance: tr!("A {chance}% chance to become a wild talent", chance),
            can_test: !self
                .rules
                .is_wild_talent(&self.character, self.skills.wild_talent),
        })
    }

    /// Changes a rogue skill's points, by its row; false when refused:
    /// points past what is left, past the most one skill may get at this
    /// level, or past a 95% score.
    pub fn set_points(&mut self, row: usize, points: i32) -> bool {
        let Some(&before) = self.skills.rogue.get(row) else {
            return false;
        };
        let least = self
            .rules
            .rogue_minimums(&self.character, &self.skills)
            .get(row)
            .copied()
            .unwrap_or(0)
            .min(before);
        let points = points.clamp(least, MOST_POINTS);
        let mut new = self.skills.clone();
        new.rogue[row] = points;
        if points > before {
            let character = &self.character;
            if self.rules.rogue_points_left(character, &new) < 0
                || self.rules.rogue_warnings(character, &new).len()
                    > self.rules.rogue_warnings(character, &self.skills).len()
            {
                return false;
            }
        }
        self.skills = new;
        true
    }

    /// Gives the kit's bonus of a group to one of its skills.
    pub fn pick_bonus(&mut self, group: usize, name: usize) {
        let Some(pick) = self
            .rules
            .rogue_bonus_picks(&self.character)
            .into_iter()
            .nth(group)
        else {
            return;
        };
        if let Some(name) = pick.names.get(name) {
            self.rules
                .set_rogue_bonus(&self.character, &mut self.skills, pick.level, Some(name));
        }
    }

    /// Ticks one of the kit's chosen skills, or not; no more than its
    /// count.
    pub fn tick_chosen(&mut self, name: usize, on: bool) {
        let Some(pick) = self.rules.chosen_pick(&self.character) else {
            return;
        };
        let Some(name) = pick.names.get(name) else {
            return;
        };
        if on && !self.chosen.contains(name) && self.chosen.len() >= pick.count {
            return;
        }
        // In the list's order.
        self.chosen = pick
            .names
            .iter()
            .filter(|n| {
                if *n == name {
                    on
                } else {
                    self.chosen.contains(n)
                }
            })
            .cloned()
            .collect();
        let chosen = self.chosen.clone();
        self.rules
            .set_chosen(&self.character, &mut self.skills, &chosen);
    }

    /// Chooses a primary discipline, by its index in [`Self::primary`]'s
    /// items ("None" first).
    pub fn set_primary(&mut self, slot: usize, index: usize) {
        let value = match index.checked_sub(1) {
            Some(i) => match self.skills.disciplines.get(i) {
                Some(d) => d.clone(),
                None => return,
            },
            None => String::new(),
        };
        if let Some(primary) = self.skills.primary.get_mut(slot) {
            *primary = value;
        }
    }

    /// What Add asks first on a list: a wild talent should be rolled
    /// (0x6363c7).
    pub fn add_question(&self, kind: ListKind) -> Option<Question> {
        (kind == ListKind::WildTalents).then(|| self.rules.add_question())
    }

    /// Adds `name` to a list, if Add offers it.
    pub fn add(&mut self, kind: ListKind, name: &str) {
        if !self.offer(kind).iter().any(|n| n == name) {
            return;
        }
        if let Some(names) = self.names_mut(kind) {
            names.push(name.to_owned());
        }
    }

    /// Removes a list's row; the primary disciplines are among the
    /// disciplines taken.
    pub fn remove(&mut self, kind: ListKind, row: usize) {
        let Some(names) = self.names_mut(kind) else {
            return;
        };
        if row >= names.len() {
            return;
        }
        let gone = names.remove(row);
        if kind == ListKind::Disciplines {
            for primary in &mut self.skills.primary {
                if *primary == gone {
                    primary.clear();
                }
            }
        }
    }

    /// What Test asks before it rolls.
    pub fn test_questions(&self) -> Vec<Question> {
        self.rules
            .test_questions(&self.character, self.skills.tested)
    }

    /// Tests for wild talents (0x6366b7): the character's Constitution,
    /// Intelligence and Wisdom take the test's losses, which the caller
    /// keeps on the character too, with the notices to show.
    pub fn test(&mut self, dice: &mut Dice) -> WildTest {
        let test = self.rules.test_wild_talent(&self.character, dice);
        self.skills.tested = true;
        self.skills.wild_talent |= test.wild_talent;
        self.character.data.abilities[1..4].copy_from_slice(&test.abilities);
        test
    }

    /// What Roll asks first: a character with wild talents may roll only
    /// once.
    pub fn roll_questions(&self) -> Vec<Question> {
        self.rules
            .roll_questions(&self.talents, WildMode::Update)
            .unwrap_or_default()
    }

    /// Rolls the wild talents on Table 83 in place of those taken, and
    /// gives the notices to show.
    pub fn roll(&mut self, dice: &mut Dice) -> Vec<Question> {
        let (names, notices) = self.rules.roll_wild_talents(dice);
        self.talents = names;
        notices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create::tests::wizard;

    /// A character of `classes`, as New Character builds it.
    fn character(classes: &[&str]) -> Option<(Arc<SkillRules>, Character)> {
        let dir = std::env::var_os("CORERULES_DIR")?;
        let install = cr_store::Installation::open(std::path::Path::new(&dir)).ok()?;
        let rules = Arc::new(install.skill_rules().ok()?);
        let w = wizard(false)?;
        let mut draft = w.draft().clone();
        draft.race = "Human".into();
        draft.classes = classes.iter().map(|c| (*c).to_owned()).collect();
        draft.class_abilities = vec![Vec::new(); classes.len()];
        draft.scores = [16; 6];
        let character = w.rules().build(&draft).ok()?;
        Some((rules, character))
    }

    #[test]
    fn a_psionicist_takes_disciplines_then_their_powers() {
        let Some((rules, character)) = character(&["Psionicist"]) else {
            return;
        };
        let mut form = SkillsForm::new(rules, character);
        assert!(form.psionics().is_some());
        assert!(form.wild().is_none());
        assert!(!form.list(ListKind::WildTalents).shown);
        let disciplines = form.list(ListKind::Disciplines);
        assert!(disciplines.shown && !disciplines.choices.is_empty());
        // No discipline, no powers.
        assert!(form.list(ListKind::Sciences).choices.is_empty());
        let first = disciplines.choices[0].clone();
        form.add(ListKind::Disciplines, &first);
        form.set_primary(0, 1);
        assert_eq!(form.skills().primary[0], first);
        let devotions = form.list(ListKind::Devotions).choices;
        assert!(!devotions.is_empty(), "{first} has devotions");
        form.add(ListKind::Devotions, &devotions[0]);
        assert_eq!(form.list(ListKind::Devotions).rows.len(), 1);
        assert!(!form.list(ListKind::Devotions).rows[0].1.is_empty());
        // Removing the discipline leaves no primary.
        form.remove(ListKind::Disciplines, 0);
        assert!(form.skills().primary[0].is_empty());
        assert_eq!(form.primary()[0].selected, 0);
    }

    #[test]
    fn rogue_points_past_those_left_are_refused() {
        let Some((rules, character)) = character(&["Thief"]) else {
            return;
        };
        let mut form = SkillsForm::new(rules, character);
        let rogue = form.rogue().expect("a thief has rogue skills");
        let left = rogue.left;
        assert!(left > 0, "{rogue:?}");
        assert!(form.set_points(0, 5));
        assert_eq!(form.rogue().unwrap().left, left - 5);
        assert!(!form.set_points(1, left as i32 + 10));
        assert_eq!(form.rogue().unwrap().rows[1].points, 0);
        // A thief has wild talents to test for.
        assert!(form.wild().is_some_and(|w| w.can_test));
    }

    #[test]
    fn wild_talents_rolled_replace_those_taken() {
        let Some((rules, character)) = character(&["Fighter"]) else {
            return;
        };
        let mut form = SkillsForm::new(rules, character);
        assert!(form.roll_questions().is_empty());
        assert!(form.add_question(ListKind::WildTalents).is_some());
        let mut dice = Dice::seeded(3);
        let _ = form.roll(&mut dice);
        let rolled = form.list(ListKind::WildTalents).rows.len();
        if rolled > 0 {
            // A second roll asks first, and the modes show.
            assert_eq!(form.roll_questions().len(), 1);
            assert!(form.list(ListKind::Attacks).shown);
            let skills = form.skills();
            assert_eq!(skills.sciences.len() + skills.devotions.len(), rolled);
        }
    }
}
