//! What the classes and the kit do on the Nonweapon Proficiencies page of
//! the original's Create Character (IDD 307): the bonus proficiencies to
//! pick (5c7997), the one to get +2, the required ones it adds (0x4414e1),
//! the free and +1 picks (5c6ca8, 5c6ad8), the kit's starting slots
//! (5c458c), the proficiencies the kit bars (5c857b) and which ones cost
//! nothing (0x43776b).

use super::{CreationRules, Draft, bonus_languages};

/// A list the page has the user pick some proficiencies from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonweaponPick {
    pub kind: PickKind,
    /// The class or kit offering it.
    pub source: String,
    pub from_kit: bool,
    /// How many to pick, at most.
    pub count: usize,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickKind {
    /// Bonus proficiencies, which cost nothing: the classes' lists
    /// (+0x2fb4) and then the kit's (+0x330), by index among them.
    Bonus(usize),
    /// One to get +2 (kit +0x2a8).
    PlusTwo,
    /// One that costs nothing, from the kit's recommended ones (+0x78,
    /// +0x258).
    Free,
    /// Some to get +1, from the kit's recommended ones (+0x94).
    PlusOne,
}

impl Draft {
    /// What was picked from a list.
    pub fn picked(&self, kind: PickKind) -> &[String] {
        self.nonweapon_picks
            .iter()
            .find(|(k, _)| *k == kind)
            .map_or(&[], |(_, names)| names.as_slice())
    }

    pub fn set_picked(&mut self, kind: PickKind, names: Vec<String>) {
        self.nonweapon_picks.retain(|(k, _)| *k != kind);
        if !names.is_empty() {
            self.nonweapon_picks.push((kind, names));
        }
    }
}

impl CreationRules {
    /// Every bonus list with how many to pick: each class's, then each of
    /// the kit's groups.
    fn bonus_lists(&self, draft: &Draft) -> Vec<(String, usize, Vec<String>)> {
        let mut lists: Vec<_> = draft
            .classes
            .iter()
            .filter_map(|c| self.class(c))
            .filter(|c| !c.bonus_nonweapon.is_empty())
            .map(|c| {
                let count = c.bonus_nonweapon_count.max(0) as usize;
                (c.name.clone(), count, c.bonus_nonweapon.clone())
            })
            .collect();
        if let Some(kit) = self.kit(&draft.kit) {
            lists.extend(
                kit.nwp_options
                    .iter()
                    .filter(|o| !o.names.is_empty())
                    .map(|o| (draft.kit.clone(), o.count.max(0) as usize, o.names.clone())),
            );
        }
        lists
    }

    /// The lists the page offers to pick from, in the original's order. A
    /// bonus list with no more names than picks is taken whole instead.
    pub fn nonweapon_picks(&self, draft: &Draft) -> Vec<NonweaponPick> {
        if !self.expansion {
            return Vec::new();
        }
        let mut picks: Vec<NonweaponPick> = self
            .bonus_lists(draft)
            .into_iter()
            .enumerate()
            .filter(|(_, (_, count, names))| *count < names.len())
            .map(|(i, (source, count, names))| NonweaponPick {
                kind: PickKind::Bonus(i),
                from_kit: source == draft.kit && self.class(&source).is_none(),
                source,
                count,
                names,
            })
            .collect();
        let Some(kit) = self.kit(&draft.kit) else {
            return picks;
        };
        let source = draft.kit.clone();
        let mut pick = |kind, count: usize, names: &[String]| {
            if count > 0 && !names.is_empty() {
                picks.push(NonweaponPick {
                    kind,
                    source: source.clone(),
                    from_kit: true,
                    count,
                    names: names.to_vec(),
                });
            }
        };
        pick(PickKind::PlusTwo, 1, kit.sa(0x2a8));
        let recommended = kit.sa(0x258);
        pick(PickKind::Free, usize::from(kit.flag(0x78)), recommended);
        pick(PickKind::PlusOne, kit.i(0x94).max(0) as usize, recommended);
        picks
    }

    /// The bonus proficiencies (+0x1944): the lists taken whole and what
    /// was picked from the others.
    pub fn bonus_nonweapon(&self, draft: &Draft) -> Vec<String> {
        if !self.expansion {
            return Vec::new();
        }
        let mut bonus = Vec::new();
        for (i, (_, count, names)) in self.bonus_lists(draft).into_iter().enumerate() {
            let names = if count < names.len() {
                let picked = draft.picked(PickKind::Bonus(i));
                names.into_iter().filter(|n| picked.contains(n)).collect()
            } else {
                names
            };
            for name in names {
                if !bonus.contains(&name) {
                    bonus.push(name);
                }
            }
        }
        bonus
    }

    /// The proficiencies the page adds when it opens (0x440b76): the
    /// classes' (+0x2fcc) and the kit's (+0x280).
    pub fn required_nonweapon(&self, draft: &Draft) -> Vec<String> {
        if !self.expansion {
            return Vec::new();
        }
        let mut required: Vec<String> = Vec::new();
        let classes = draft.classes.iter().filter_map(|c| self.class(c));
        let kit = self
            .kit(&draft.kit)
            .map(|k| k.sa(0x280))
            .unwrap_or_default();
        for name in classes.flat_map(|c| c.required_nonweapon.iter()).chain(kit) {
            if !required.contains(name) && self.nonweapon.iter().any(|n| &n.name == name) {
                required.push(name.clone());
            }
        }
        required
    }

    /// The proficiencies the kit does not allow (+0x26c).
    pub fn barred_nonweapon(&self, draft: &Draft) -> &[String] {
        self.kit(&draft.kit).map_or(&[], |k| k.sa(0x26c))
    }

    /// The starting slots the kit specifies in place of the class's
    /// (+0x318), Core Rules only.
    pub fn kit_nonweapon_base(&self, draft: &Draft) -> Option<u32> {
        if !self.expansion || draft.skills_and_powers {
            return None;
        }
        let base = self.kit(&draft.kit)?.i_or(0x318, -1);
        (base >= 1).then_some(base as u32)
    }

    /// Core Rules nonweapon proficiency slots (5c458c): the best of the
    /// classes', or the kit's when chosen, plus the languages Intelligence
    /// gives, one more with kit +0x30c and one a level with +0x30d.
    pub fn nonweapon_slots_for(&self, draft: &Draft) -> u32 {
        let intelligence = self.adjusted(&draft.race, &draft.scores)[3];
        let mut slots = match self.kit_nonweapon_base(draft) {
            Some(base) if draft.kit_nonweapon_base == Some(true) => {
                base + bonus_languages(intelligence)
            }
            _ => self.nonweapon_slots(&draft.classes, intelligence),
        };
        if let Some(kit) = self.kit(&draft.kit).filter(|_| self.expansion) {
            slots += u32::from(kit.flag(0x30c)) + self.kit_level_slots(draft);
        }
        slots
    }

    /// One slot or point a level, at 1st level one (kit +0x30d).
    pub(crate) fn kit_level_slots(&self, draft: &Draft) -> u32 {
        let extra = self.expansion && self.kit(&draft.kit).is_some_and(|k| k.flag(0x30d));
        u32::from(extra)
    }

    /// Whether a proficiency costs the new character nothing (0x43776b):
    /// the classes make it free (+0x2fa0), it is a bonus one, the kit's
    /// free one (+0xa4) or the one picked as free, or the kit gives it by
    /// 1st level (+0x2f8).
    pub(crate) fn free_nonweapon(&self, draft: &Draft, name: &str) -> bool {
        if !self.expansion {
            return false;
        }
        let classes = draft.classes.iter().filter_map(|c| self.class(c));
        if classes.flat_map(|c| &c.free_nonweapon).any(|n| n == name) {
            return true;
        }
        if let Some(kit) = self.kit(&draft.kit)
            && (kit.s(0xa4) == name
                || draft.picked(PickKind::Free).iter().any(|n| n == name)
                || kit
                    .free_nwp
                    .iter()
                    .any(|(n, level)| n == name && *level <= 1))
        {
            return true;
        }
        self.bonus_nonweapon(draft).iter().any(|n| n == name)
    }

    /// Skills & Powers: the kit's recommended proficiencies (+0x258) cost
    /// a point less.
    pub(crate) fn recommended_nonweapon(&self, draft: &Draft, name: &str) -> bool {
        self.expansion
            && self
                .kit(&draft.kit)
                .is_some_and(|k| k.sa(0x258).iter().any(|n| n == name))
    }
}
