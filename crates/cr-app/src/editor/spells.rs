//! The editor's Spells page: the Expansion's Spells and Memorize pages of
//! Update Character. First what the kit asks about the schools and
//! spheres (see [`cr_store::kit_magic`]); then the wizard and priest
//! spells known, each learned as the original's Learn checks it; the
//! spells of each spellbook with the pages they take; and the spells each
//! class that casts has memorized, within the slots of its levels and
//! after the kit's warning.

use std::sync::Arc;

use cr_i18n::tr;
use cr_store::choices::{Caster, caster};
use cr_store::kit_magic::KitMagicRules;
use cr_store::learn::{ALREADY_KNOWN, Learner, Learning, WARNING};
use cr_store::lists::BUNDLE_NAMES;

use super::{Check, Checks, EditInput, Editor, Prompt};
use crate::create::{Choice, List, ListKind, item_at, with_none};

/// A group of the Spells page to tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MagicGroup {
    /// The schools the kit has the wizard eliminate.
    Eliminated,
    /// The minor or major spheres a priest class keeps, by its slot.
    Spheres(usize, bool),
}

/// The kit's schools and spheres, as the Spells page asks about them.
#[derive(Debug, Clone, PartialEq)]
pub struct KitMagicView {
    /// The kit's warning about the schools it bars, and whether they stay.
    pub barred: Option<(String, bool)>,
    /// The schools to eliminate and the spheres to keep.
    pub groups: Vec<Checks<MagicGroup>>,
    pub free_sphere: Choice,
}

/// A spell level as an ordinal ("1st").
pub fn ordinal(level: u32) -> String {
    match (level % 10, level % 100) {
        (1, n) if n != 11 => tr!("{level}st", level),
        (2, n) if n != 12 => tr!("{level}nd", level),
        (3, n) if n != 13 => tr!("{level}rd", level),
        _ => tr!("{level}th", level),
    }
}

/// How many of `levels` are spells of `level`.
fn used(levels: &[u32], level: u32) -> u32 {
    levels.iter().filter(|l| **l == level).count() as u32
}

/// Whether spells memorized of these levels fit the slots; with no slots
/// known, they do.
pub fn slots_fit(slots: Option<[u32; 9]>, levels: &[u32]) -> bool {
    slots.is_none_or(|slots| {
        (1..=9u32).all(|level| used(levels, level) <= slots[level as usize - 1])
    })
}

/// "1st: 2 of 3 · 2nd: 0 of 1" for spells memorized of these levels
/// against the slots, marking levels over the limit.
pub fn slots_text(slots: Option<[u32; 9]>, levels: &[u32]) -> Option<String> {
    let slots = slots?;
    let parts: Vec<String> = (1..=9u32)
        .filter_map(|level| {
            let used = used(levels, level);
            let have = slots[level as usize - 1];
            if have == 0 && used == 0 {
                return None;
            }
            let level = ordinal(level);
            Some(if used > have {
                tr!("{level}: {used} of {have} (too many)", level, used, have)
            } else {
                tr!("{level}: {used} of {have}", level, used, have)
            })
        })
        .collect();
    Some(if parts.is_empty() {
        tr("No spell slots at this level").into()
    } else {
        parts.join(" · ")
    })
}

/// `names` to tick, those in `chosen` ticked, at most `most` of them.
fn checks(names: &[String], chosen: &[String], most: usize) -> Vec<Check> {
    let full = names.iter().filter(|n| chosen.contains(n)).count() >= most;
    names
        .iter()
        .map(|name| {
            let on = chosen.contains(name);
            Check {
                title: name.clone(),
                subtitle: String::new(),
                on,
                enabled: on || !full,
            }
        })
        .collect()
}

impl Editor {
    /// The kit's choices as its groups have them, once the kit or the
    /// barred schools changed: the schools eliminated among those it
    /// offers, the spheres kept of each class it has choose (the first
    /// ones until chosen), and the free sphere among those it offers.
    /// What the file has stays where the kit asks no more.
    pub(super) fn settle_kit_magic(&mut self) {
        let rules = self.needs.kit_magic.clone();
        let (character, kit) = (&*self.original, self.details.kit.as_str());
        let read = rules.magic_of(character);
        let magic = &mut self.kit_magic;
        if let Some((_, schools)) = rules.schools_to_eliminate(character, kit, magic) {
            magic.eliminated.retain(|s| schools.contains(s));
        } else if read.eliminated.is_empty() {
            magic.eliminated.clear();
        }
        magic.spheres = rules
            .sphere_picks(character, kit)
            .into_iter()
            .map(|pick| {
                let chosen = magic
                    .spheres
                    .iter()
                    .find(|(s, m, _)| *s == pick.slot && *m == pick.major)
                    .map_or_else(
                        || pick.names.iter().take(pick.most).cloned().collect(),
                        |p| p.2.clone(),
                    );
                (pick.slot, pick.major, chosen)
            })
            .collect();
        let free = rules.free_sphere_choices(character, kit);
        if free.is_empty() {
            if read.free_sphere.is_empty() {
                magic.free_sphere.clear();
            }
        } else if !free.contains(&magic.free_sphere) {
            magic.free_sphere.clear();
        }
    }

    /// The kit's groups of the page.
    pub fn kit_magic_view(&self) -> KitMagicView {
        let rules = &self.needs.kit_magic;
        let (character, kit) = (&*self.original, self.details.kit.as_str());
        let magic = &self.kit_magic;
        // The barred schools go unless ignored (63056f).
        let barred = rules
            .barred_schools_warning(character, kit)
            .map(|warning| (warning, magic.barred_ignored));
        let mut groups = Vec::new();
        // The schools beyond five, eliminated once (6309df).
        if let Some((count, schools)) = rules.schools_to_eliminate(character, kit, magic) {
            groups.push(Checks {
                group: MagicGroup::Eliminated,
                title: tr("Eliminated Schools").into(),
                description: KitMagicRules::elimination_text(kit, count),
                checks: checks(&schools, &magic.eliminated, count),
            });
        }
        // The spheres to keep where there are more than the kit allows
        // (63286d).
        let classes = &character.data.classes.names;
        for pick in rules.sphere_picks(character, kit) {
            let chosen = magic
                .spheres
                .iter()
                .find(|(s, m, _)| *s == pick.slot && *m == pick.major)
                .map_or(&[][..], |p| &p.2[..]);
            let class = classes.get(pick.slot).map_or("", String::as_str);
            groups.push(Checks {
                group: MagicGroup::Spheres(pick.slot, pick.major),
                title: if pick.major {
                    tr!("Major Spheres to Keep ({class})", class)
                } else {
                    tr!("Minor Spheres to Keep ({class})", class)
                },
                description: KitMagicRules::sphere_pick_text(kit, pick.major, pick.most),
                checks: checks(&pick.names, chosen, pick.most),
            });
        }
        // The free major sphere, while the character has none.
        let free = rules.free_sphere_choices(character, kit);
        let free_sphere = Choice {
            title: tr("Free Sphere").into(),
            description: KitMagicRules::free_sphere_text(kit),
            selected: free
                .iter()
                .position(|s| *s == magic.free_sphere)
                .map_or(0, |i| i + 1),
            shown: !free.is_empty(),
            items: with_none(free),
        };
        KitMagicView {
            barred,
            groups,
            free_sphere,
        }
    }

    /// Ticks a school to eliminate or a sphere to keep, while the group
    /// has room for it.
    fn tick_magic(&mut self, group: MagicGroup, index: usize, on: bool) {
        let view = self.kit_magic_view();
        let Some(checks) = view.groups.into_iter().find(|g| g.group == group) else {
            return;
        };
        if checks.checks.get(index).is_none_or(|c| on && !c.enabled) {
            return;
        }
        let names: Vec<String> = checks
            .checks
            .into_iter()
            .enumerate()
            .filter(|(i, c)| if *i == index { on } else { c.on })
            .map(|(_, c)| c.title)
            .collect();
        match group {
            MagicGroup::Eliminated => self.kit_magic.eliminated = names,
            MagicGroup::Spheres(slot, major) => {
                if let Some(kept) = self
                    .kit_magic
                    .spheres
                    .iter_mut()
                    .find(|(s, m, _)| *s == slot && *m == major)
                {
                    kept.2 = names;
                }
            }
        }
    }

    /// The classes that cast spells, by slot, and whether they cast
    /// wizard spells.
    fn casters(&self) -> Vec<(usize, bool)> {
        let ids = &self.needs.class_ids;
        self.original
            .data
            .classes
            .names
            .iter()
            .enumerate()
            .take(5)
            .filter_map(|(slot, class)| match ids.get(class).copied().map(caster) {
                Some(Caster::Wizard) => Some((slot, true)),
                Some(Caster::Priest) => Some((slot, false)),
                _ => None,
            })
            .collect()
    }

    /// The character learning wizard or priest spells, as the first of
    /// its classes that casts them at the level the form has now, with
    /// the abilities as edited.
    fn learner(&self, wizard: bool) -> Learner {
        let kind = if wizard {
            Caster::Wizard
        } else {
            Caster::Priest
        };
        let names = &self.advanced.data.classes.names;
        let slot = names
            .iter()
            .position(|n| self.needs.class_ids.get(n).copied().map(caster) == Some(kind));
        let (class, level, abilities) = match slot {
            Some(g) => (
                names[g].clone(),
                self.levels().get(g).copied().unwrap_or(1),
                self.abilities
                    .classes
                    .get(g)
                    .map(|c| c.abilities.clone())
                    .unwrap_or_default(),
            ),
            None => (String::new(), 0, Vec::new()),
        };
        let (reason, knowledge) =
            Learner::intelligence(&self.with_abilities((*self.advanced).clone()));
        Learner {
            castable: self.needs.advance.castable(&class, level, &abilities),
            class,
            abilities,
            kit: self.details.kit.clone(),
            reason,
            knowledge,
            wizard_spells: self.lists.wizard_spells.clone(),
            priest_spells: self.lists.priest_spells.clone(),
            entering: false,
        }
    }

    /// Every wizard or priest spell, and those the character knows.
    fn spells_of(&self, wizard: bool) -> (&[String], &[String]) {
        if wizard {
            (&self.needs.wizard_spells, &self.lists.wizard_spells)
        } else {
            (&self.needs.priest_spells, &self.lists.priest_spells)
        }
    }

    /// The wizard or priest spells known, and the chance to learn each
    /// spell Add offers, by its index (0x630052).
    pub fn spells_known(&self, wizard: bool) -> (List, Vec<String>) {
        let (all, known) = self.spells_of(wizard);
        let choices: Vec<String> = all.iter().filter(|n| !known.contains(n)).cloned().collect();
        let learner = self.learner(wizard);
        let chances = choices
            .iter()
            .map(|n| self.needs.learn.chance_text(&learner, n, wizard))
            .collect();
        let list = List {
            title: if wizard {
                tr("Wizard Spells Known")
            } else {
                tr("Priest Spells Known")
            }
            .into(),
            description: String::new(),
            rows: known
                .iter()
                .map(|n| (n.clone(), String::new(), None))
                .collect(),
            choices,
            shown: true,
            fixed: false,
        };
        (list, chances)
    }

    /// Learns a spell once the original's Learn checks are answered Yes
    /// (0x62dc71), for the first class that casts spells of its kind.
    fn learn(&mut self, name: String, wizard: bool) {
        let (all, known) = self.spells_of(wizard);
        if !all.contains(&name) || known.contains(&name) {
            return;
        }
        let learner = self.learner(wizard);
        let rules = self.needs.learn.clone();
        match rules.learn_rolled(&learner, &name, wizard, &mut self.dice) {
            Learning::Known => self.message(tr(WARNING), tr(ALREADY_KNOWN)),
            Learning::Ask(questions) => self.ask_each(
                questions.into(),
                Arc::new(move |e: &mut Editor| {
                    let known = if wizard {
                        &mut e.lists.wizard_spells
                    } else {
                        &mut e.lists.priest_spells
                    };
                    if !known.contains(&name) {
                        known.push(name.clone());
                    }
                }),
            ),
        }
    }

    /// The spellbooks, each with the wizard spells known it may take and
    /// the most pages a spell takes in it.
    pub fn spellbooks(&self) -> Vec<(List, u32)> {
        self.lists
            .spellbooks
            .iter()
            .map(|book| {
                let bundle = BUNDLE_NAMES.get(book.bundle).map_or("", |b| tr(b));
                let list = List {
                    title: format!("{} ({bundle})", book.name),
                    description: tr!(
                        "{used} of {total} pages used",
                        used = book.pages_used(),
                        total = book.capacity
                    ),
                    rows: book
                        .spells
                        .iter()
                        .map(|(name, pages, _)| (name.clone(), String::new(), Some(*pages)))
                        .collect(),
                    choices: self
                        .lists
                        .wizard_spells
                        .iter()
                        .filter(|n| !book.spells.iter().any(|s| s.0 == **n))
                        .cloned()
                        .collect(),
                    shown: true,
                    fixed: false,
                };
                (list, book.capacity.max(1))
            })
            .collect()
    }

    /// The spells a class has memorized, by their levels, in order.
    fn memorized_rows(&self, slot: usize, wizard: bool) -> Vec<(u32, String)> {
        self.lists.memorized[slot]
            .iter()
            .enumerate()
            .flat_map(|(level, (w, p))| {
                (if wizard { w } else { p })
                    .iter()
                    .map(move |name| (level as u32, name.clone()))
            })
            .collect()
    }

    /// The spells each class that casts has memorized, by its slot, with
    /// the slots of the levels the form has now, as the original's
    /// Memorize dialog counts them.
    pub fn memorized(&self) -> Vec<(usize, List)> {
        self.casters()
            .into_iter()
            .map(|(slot, wizard)| {
                let rows = self.memorized_rows(slot, wizard);
                let levels: Vec<u32> = rows.iter().map(|r| r.0).collect();
                let slots = self
                    .needs
                    .advance
                    .spell_slots(&self.advanced, slot)
                    .ok()
                    .flatten();
                let class = self.original.data.classes.names[slot].as_str();
                let (offered, spell_levels) = if wizard {
                    (&self.lists.wizard_spells, &self.needs.wizard_levels)
                } else {
                    (&self.needs.priest_spells, &self.needs.priest_levels)
                };
                let list = List {
                    title: tr!("Memorized ({class})", class),
                    description: slots_text(slots, &levels).unwrap_or_default(),
                    rows: rows
                        .into_iter()
                        .map(|(level, name)| {
                            let level = tr!("{ordinal} level", ordinal = ordinal(level));
                            (name, level, None)
                        })
                        .collect(),
                    choices: offered
                        .iter()
                        .filter(|n| spell_levels.contains_key(*n))
                        .cloned()
                        .collect(),
                    shown: true,
                    fixed: false,
                };
                (slot, list)
            })
            .collect()
    }

    /// Memorizes a spell for the class in `slot` once the kit's warning
    /// is ignored (5d7732), if a slot of its level is left.
    fn memorize(&mut self, slot: usize, wizard: bool, name: String) {
        let spell_levels = if wizard {
            &self.needs.wizard_levels
        } else {
            &self.needs.priest_levels
        };
        let Some(&level) = spell_levels.get(&name) else {
            return;
        };
        let warning = self
            .needs
            .kit_magic
            .spell_use_warning(&self.details.kit, &self.kit_magic);
        if let Some(warning) = warning {
            self.show(Prompt::ignorable(
                tr("Kit Spell Use"),
                &warning,
                Arc::new(move |e: &mut Editor| {
                    e.kit_magic.spell_use_ignored = true;
                    e.memorize(slot, wizard, name.clone());
                }),
            ));
            return;
        }
        let mut levels: Vec<u32> = self
            .memorized_rows(slot, wizard)
            .iter()
            .map(|r| r.0)
            .collect();
        levels.push(level);
        let slots = self
            .needs
            .advance
            .spell_slots(&self.advanced, slot)
            .ok()
            .flatten();
        if !slots_fit(slots, &levels) {
            self.message(
                tr("Not Allowed"),
                tr("There are no more spell slots of this level to memorize it in."),
            );
            return;
        }
        if let Some((w, p)) = self.lists.memorized[slot].get_mut(level as usize) {
            (if wizard { w } else { p }).push(name);
        }
    }

    /// Takes row `row` out of the spells a class has memorized.
    fn forget(&mut self, slot: usize, wizard: bool, row: usize) {
        let mut left = row;
        for (w, p) in &mut self.lists.memorized[slot] {
            let names = if wizard { w } else { p };
            if left < names.len() {
                names.remove(left);
                return;
            }
            left -= names.len();
        }
    }

    pub(super) fn spells_input(&mut self, input: EditInput) {
        match input {
            EditInput::IgnoreBarred(on) => {
                self.kit_magic.barred_ignored = on;
                // The schools to eliminate depend on those kept.
                self.settle_kit_magic();
            }
            EditInput::MagicTick(group, index, on) => self.tick_magic(group, index, on),
            EditInput::FreeSphere(index) => {
                let free = self.kit_magic_view().free_sphere;
                if free.shown && index < free.items.len() {
                    self.kit_magic.free_sphere = item_at(&free.items, index).unwrap_or_default();
                }
            }
            EditInput::Add(ListKind::Spells, name) => self.learn(name, true),
            EditInput::Add(ListKind::PriestSpells, name) => self.learn(name, false),
            EditInput::Remove(ListKind::Spells, row) if row < self.lists.wizard_spells.len() => {
                self.lists.wizard_spells.remove(row);
            }
            EditInput::Remove(ListKind::PriestSpells, row)
                if row < self.lists.priest_spells.len() =>
            {
                self.lists.priest_spells.remove(row);
            }
            EditInput::Add(ListKind::Spellbook(b), name) => {
                let Some((list, most)) = self.spellbooks().into_iter().nth(b) else {
                    return;
                };
                if !list.choices.contains(&name) {
                    return;
                }
                // One page per spell level, as the rules have it.
                let pages = self.needs.wizard_levels.get(&name).copied().unwrap_or(1);
                self.lists.spellbooks[b]
                    .spells
                    .push((name, pages.clamp(1, most), true));
            }
            EditInput::Remove(ListKind::Spellbook(b), row) => {
                if let Some(book) = self.lists.spellbooks.get_mut(b)
                    && row < book.spells.len()
                {
                    book.spells.remove(row);
                }
            }
            EditInput::Times(ListKind::Spellbook(b), row, pages) => {
                if let Some(book) = self.lists.spellbooks.get_mut(b) {
                    let most = book.capacity.max(1);
                    if let Some(spell) = book.spells.get_mut(row) {
                        spell.1 = pages.clamp(1, most);
                    }
                }
            }
            EditInput::Add(ListKind::Memorized(slot), name) => {
                let Some((_, list)) = self.memorized().into_iter().find(|(s, _)| *s == slot) else {
                    return;
                };
                let wizard = self.casters().contains(&(slot, true));
                if list.choices.contains(&name) {
                    self.memorize(slot, wizard, name);
                }
            }
            EditInput::Remove(ListKind::Memorized(slot), row) => {
                if let Some(&(_, wizard)) = self.casters().iter().find(|(s, _)| *s == slot) {
                    self.forget(slot, wizard, row);
                }
            }
            _ => {}
        }
    }

    /// Whether an input is the Spells page's.
    pub(super) fn is_spells_input(input: &EditInput) -> bool {
        match input {
            EditInput::Add(kind, _) | EditInput::Remove(kind, _) | EditInput::Times(kind, ..) => {
                matches!(
                    kind,
                    ListKind::Spells
                        | ListKind::PriestSpells
                        | ListKind::Spellbook(_)
                        | ListKind::Memorized(_)
                )
            }
            EditInput::IgnoreBarred(_) | EditInput::MagicTick(..) | EditInput::FreeSphere(_) => {
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{editor, editor_where};
    use super::*;
    use crate::prompt::Answer;

    #[test]
    fn slots_count_the_spells_of_each_level() {
        let slots = Some([2, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert!(slots_fit(slots, &[1, 1, 2]));
        assert!(!slots_fit(slots, &[1, 1, 1]));
        assert!(!slots_fit(slots, &[3]));
        assert!(slots_fit(None, &[9, 9, 9]));
        assert_eq!(
            slots_text(slots, &[1, 1, 1]).as_deref(),
            Some("1st: 3 of 2 (too many) · 2nd: 0 of 1")
        );
        assert_eq!(
            slots_text(Some([0; 9]), &[]).as_deref(),
            Some("No spell slots at this level")
        );
        assert_eq!(slots_text(None, &[1]), None);
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(22), "22nd");
    }

    #[test]
    fn a_spell_learned_is_known_and_its_chance_was_offered() {
        let Some(mut e) = editor() else {
            return;
        };
        for wizard in [true, false] {
            let (list, chances) = e.spells_known(wizard);
            assert_eq!(list.choices.len(), chances.len());
            let Some(name) = list.choices.first().cloned() else {
                continue;
            };
            let kind = if wizard {
                ListKind::Spells
            } else {
                ListKind::PriestSpells
            };
            let count = list.rows.len();
            e.input(EditInput::Add(kind, name.clone()));
            while e.prompt().is_some() {
                e.answer(Answer::Yes);
            }
            let (list, _) = e.spells_known(wizard);
            if list.rows.len() == count {
                // Its kind of spell is not for the character.
                continue;
            }
            assert!(list.rows.iter().any(|r| r.0 == name));
            assert!(!list.choices.contains(&name));
            assert!(e.changed());
            e.input(EditInput::Remove(kind, count));
            assert_eq!(e.spells_known(wizard).0.rows.len(), count);
        }
    }

    #[test]
    fn a_spell_memorized_takes_a_slot_of_its_level() {
        let Some(mut e) = editor_where(|_| true) else {
            return;
        };
        for (slot, list) in e.memorized() {
            let Some(name) = list.choices.first().cloned() else {
                continue;
            };
            let count = list.rows.len();
            e.input(EditInput::Add(ListKind::Memorized(slot), name.clone()));
            while e.prompt().is_some() {
                e.answer(Answer::Yes);
            }
            let (_, after) = e.memorized().into_iter().find(|(s, _)| *s == slot).unwrap();
            if after.rows.len() == count {
                // No slot of its level is left.
                continue;
            }
            assert!(after.rows.iter().any(|r| r.0 == name));
            let row = after.rows.iter().position(|r| r.0 == name).unwrap();
            e.input(EditInput::Remove(ListKind::Memorized(slot), row));
            let (_, back) = e.memorized().into_iter().find(|(s, _)| *s == slot).unwrap();
            assert_eq!(back.rows, list.rows);
        }
    }
}
