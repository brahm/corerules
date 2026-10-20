//! What the kit does on the Weapon Proficiencies page of the original's
//! Create Character (IDD 308): the slots or points (59e778), the weapons
//! it offers and what they cost (0x444079), the warnings when adding one
//! (5d4aa7), the nonweapon slots a kit lets weapons use, the
//! requirements it lists (5d44df), the weapon to focus in and, for the
//! Core Rules, weapon specialization (IDD 356, 61e897).

use cr_i18n::tr;

use super::{CreationRules, Draft};
use crate::npc::data::Kit;

/// A weapon part: who may not use it and what kind of weapon it is.
#[derive(Debug, Clone)]
pub(super) struct WeaponRules {
    pub name: String,
    /// Classes that may not use it (+0x82c).
    pub barred_classes: Vec<String>,
    /// A melee weapon (+0x3b8).
    pub melee: bool,
    /// A missile or thrown weapon (+0x3bc).
    pub missile: bool,
    /// It fires ranged attacks, needing ammunition (+0x3c0).
    pub ranged: bool,
    /// In the bows' tight group (+0x1cc).
    pub bow: bool,
}

/// A warning the kit gives when a weapon proficiency is added, which the
/// user may ignore for good, as the original's IDD 245.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponCheck {
    /// No missile weapons at creation (kit +0x508; the file's +0x1de8).
    Ranged,
    /// Weapons the kit bars (+0x3ac; +0x1de3).
    Barred,
    /// Every odd-numbered one a missile or thrown weapon (+0x438;
    /// +0x1ddf).
    OddMissile,
    /// Some of the first ones from a list (+0x43c, +0x440, +0x424;
    /// +0x1dde).
    FirstFromList,
    /// Only those the kit requires or recommends (+0x76; +0x1de9).
    Recommended,
    /// Specializations only from a list (+0x524; +0x1de6).
    SpecializationList,
    /// A specialization from a list (+0x3e8; +0x1de1), checked on leaving
    /// the page.
    SpecializationRequired,
    /// A melee or missile specialization by a level (+0x51c, +0x520,
    /// +0x521; +0x1de7), checked on leaving the page.
    SpecializationByLevel,
}

/// A Yes/No question before specializing, which No answers by not
/// specializing: its title and text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub title: String,
    pub text: String,
}

/// A warning about a weapon to add: which one and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponWarning {
    pub check: WeaponCheck,
    pub text: String,
}

impl CreationRules {
    fn weapon_part(&self, name: &str) -> Option<&WeaponRules> {
        self.weapons.iter().find(|w| w.name == name)
    }

    fn draft_kit(&self, draft: &Draft) -> Option<&Kit> {
        self.kit(&draft.kit).filter(|_| self.expansion)
    }

    /// Whether one of the classes may use the weapon, or the kit gives it
    /// (+0x484).
    fn weapon_in_class(&self, draft: &Draft, weapon: &WeaponRules) -> bool {
        draft
            .classes
            .iter()
            .any(|c| !weapon.barred_classes.contains(c))
            || self
                .draft_kit(draft)
                .is_some_and(|k| k.sa(0x484).contains(&weapon.name))
    }

    /// The weapons the page offers: those the classes or the kit allow,
    /// any weapon when the kit lets the Core Rules take others at a price
    /// (+0x460), and at 1st level only the kit's list when it has one
    /// (+0x4b8).
    pub fn weapon_choices(&self, draft: &Draft) -> Vec<String> {
        let kit = self.draft_kit(draft);
        let any = !draft.skills_and_powers && kit.is_some_and(|k| k.i_or(0x460, -1) != -1);
        let only = kit.map_or(&[][..], |k| k.sa(0x4b8));
        self.weapons
            .iter()
            .filter(|w| any || self.weapon_in_class(draft, w))
            .filter(|w| only.is_empty() || only.contains(&w.name))
            .map(|w| w.name.clone())
            .collect()
    }

    /// Core Rules weapon proficiency slots: the kit's in place of the
    /// classes' (+0x454), or 1 (+0x44c); then the kit's extra ones
    /// (+0x448) and the nonweapon slots lent to weapons.
    pub fn weapon_slots_for(&self, draft: &Draft) -> u32 {
        let class = self.weapon_slots(&draft.classes);
        let Some(kit) = self.draft_kit(draft) else {
            return class;
        };
        let base = if kit.flag(0x44c) {
            1
        } else {
            match kit.i_or(0x454, -1) {
                slots if slots > -1 => slots as u32,
                _ => class,
            }
        };
        (base as i32 + kit.i(0x448)).max(0) as u32 + self.borrowed_nonweapon(draft)
    }

    /// Skills & Powers points for weapon proficiencies, with the kit's in
    /// place of the classes' (+0x454) and its extra ones (+0x448).
    pub fn weapon_points_for(&self, draft: &Draft) -> u32 {
        let class = self.weapon_points(&draft.classes);
        let Some(kit) = self.draft_kit(draft) else {
            return class;
        };
        let base = match kit.i_or(0x454, -1) {
            points if points > -1 => points as u32,
            _ => class,
        };
        (base as i32 + kit.i(0x448)).max(0) as u32
    }

    /// What a weapon proficiency costs: with the Core Rules a slot, or
    /// the kit's price for a weapon the classes may not use (+0x460);
    /// with Skills & Powers the classes' points, nothing for the kit's
    /// free weapons (+0x470) and a point less for those it favors (+0x444,
    /// +0x384).
    pub fn weapon_cost_for(&self, draft: &Draft, name: &str) -> u32 {
        let kit = self.draft_kit(draft);
        if !draft.skills_and_powers {
            let out_of_class = self
                .weapon_part(name)
                .is_some_and(|w| !self.weapon_in_class(draft, w));
            return match kit.map(|k| k.i_or(0x460, -1)) {
                Some(price) if out_of_class && price != -1 => (price + 1).max(1) as u32,
                _ => 1,
            };
        }
        let cost = self.weapon_cost(&draft.classes);
        let Some(kit) = kit else {
            return cost;
        };
        if kit.free_weapons.iter().any(|(w, l)| w == name && *l <= 1) {
            return 0;
        }
        let favored = kit.flag(0x444) || kit.sa(0x384).iter().any(|n| n == name);
        cost.saturating_sub(u32::from(favored))
    }

    /// The slots or points the weapon proficiencies take, with the Core
    /// Rules the specializations' too.
    pub fn weapon_spent(&self, draft: &Draft) -> u32 {
        draft
            .weapon_proficiencies
            .iter()
            .map(|w| self.weapon_cost_for(draft, w))
            .sum::<u32>()
            + self.specialization_spent(draft)
    }

    /// Whether adding the weapon asks first, as the kit limits ranged
    /// attacks (+0x79).
    pub fn weapon_needs_confirmation(&self, draft: &Draft, name: &str) -> bool {
        self.draft_kit(draft).is_some_and(|k| k.flag(0x79))
            && self.weapon_part(name).is_some_and(|w| w.ranged)
    }

    /// The warnings adding a weapon to those `taken` gives, in the
    /// original's order, leaving out those ignored.
    pub fn weapon_warnings(
        &self,
        draft: &Draft,
        taken: &[String],
        name: &str,
    ) -> Vec<WeaponWarning> {
        let (Some(kit), Some(weapon)) = (self.draft_kit(draft), self.weapon_part(name)) else {
            return Vec::new();
        };
        let kit_name = draft.kit.as_str();
        let listed = |text: String, list: &[String]| format!("{text}\n{}", list.join(", "));
        let mut warnings = Vec::new();
        let mut warn = |check, text| {
            if !draft.ignored_weapon_checks.contains(&check) {
                warnings.push(WeaponWarning { check, text });
            }
        };
        if kit.flag(0x508) && weapon.missile {
            warn(
                WeaponCheck::Ranged,
                tr!(
                    "The kit \"{kit}\" does not allow you to select ranged weapon proficiencies \
                     during character creation.",
                    kit = kit_name
                ),
            );
        }
        let barred = kit.sa(0x3ac);
        if barred.contains(&weapon.name) {
            warn(
                WeaponCheck::Barred,
                listed(
                    tr!(
                        "The kit \"{kit}\" does not allow you to select weapon proficiencies \
                         from the following list.",
                        kit = kit_name
                    ),
                    barred,
                ),
            );
        }
        if kit.flag(0x438) && taken.len().is_multiple_of(2) && !weapon.missile {
            warn(
                WeaponCheck::OddMissile,
                tr!(
                    "The kit \"{kit}\" requires that every odd numbered weapon proficiency \
                     taken is a missile or thrown weapon.",
                    kit = kit_name
                ),
            );
        }
        let (fill, list) = (kit.i(0x43c), kit.sa(0x424));
        if fill > 0 && !list.is_empty() && !list.contains(&weapon.name) {
            let first = kit.i(0x440).max(fill);
            let filled = taken.iter().filter(|t| list.contains(t)).count();
            if filled < fill as usize && filled < list.len() && taken.len() as i32 >= first - fill {
                warn(
                    WeaponCheck::FirstFromList,
                    listed(
                        tr!(
                            "The kit \"{kit}\" requires that you fill {count} of the first \
                             {first} weapon proficiencies from the following list.",
                            kit = kit_name,
                            count = fill,
                            first
                        ),
                        list,
                    ),
                );
            }
        }
        if kit.flag(0x76) {
            let mut allowed: Vec<String> = kit
                .wp_options
                .iter()
                .flat_map(|o| o.names.iter().cloned())
                .collect();
            allowed.extend(kit.sa(0x384).iter().cloned());
            let mut seen = std::collections::HashSet::new();
            allowed.retain(|n| seen.insert(n.clone()));
            if !allowed.contains(&weapon.name) {
                warn(
                    WeaponCheck::Recommended,
                    format!(
                        "{}\n\n{}\n{}",
                        tr!(
                            "The kit \"{kit}\" does not allow you to select weapon \
                             proficiencies which are not required or recommended by your kit.",
                            kit = kit_name
                        ),
                        tr("Here is a list of the allowed weapon proficiencies:"),
                        allowed.join(", ")
                    ),
                );
            }
        }
        warnings
    }

    /// What the kit asks of the weapon proficiencies, as the box the
    /// original shows when the page opens (5d44df); none of it is
    /// enforced.
    pub fn weapon_requirements(&self, draft: &Draft) -> Vec<String> {
        let Some(kit) = self.draft_kit(draft) else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        let count = |off| kit.i(off).max(0);
        if count(0x514) > 0 {
            lines.push(tr!(
                "You must select at least {count} melee weapon proficiencies during character \
                 creation.",
                count = count(0x514)
            ));
        }
        if count(0x510) > 0 {
            lines.push(tr!(
                "You must select at least {count} missile weapon proficiencies during character \
                 creation.",
                count = count(0x510)
            ));
        }
        if count(0x50c) > 0 {
            lines.push(tr!(
                "You must spend at least {count} slots on missile weapon proficiencies during \
                 character creation.",
                count = count(0x50c)
            ));
        }
        if kit.flag(0x465) {
            lines.push(
                tr(
                    "You must select at least one weapon proficiency during character creation \
                    which represents a weapon made from wood.",
                )
                .into(),
            );
        }
        let options: Vec<String> = kit
            .wp_options
            .iter()
            .filter(|o| !o.names.is_empty())
            .map(|o| option_label(o.count, &o.names))
            .collect();
        if !options.is_empty() {
            lines.push(format!(
                "{}\n{}",
                tr("You are required to take the following:"),
                options.join("\n")
            ));
        }
        let list = |text: &str, names: &[String]| format!("{text}\n{}", names.join(", "));
        if !kit.sa(0x4f4).is_empty() {
            lines.push(list(
                tr(
                    "At least half of all weapon proficiencies taken must be from the following \
                    list:",
                ),
                kit.sa(0x4f4),
            ));
        }
        if !kit.sa(0x4e0).is_empty() {
            lines.push(list(
                tr("You must always select weapon proficiencies from the following list:"),
                kit.sa(0x4e0),
            ));
        }
        lines
    }

    /// Nonweapon slots the kit lets weapons use, at least and at most
    /// (+0x44d): with +0x44e at least half of them. Core Rules only.
    pub fn nonweapon_lending(&self, draft: &Draft) -> Option<(u32, u32)> {
        let kit = self.draft_kit(draft).filter(|_| !draft.skills_and_powers)?;
        if !kit.flag(0x44d) {
            return None;
        }
        let slots = self.nonweapon_slots_unlent(draft);
        let least = if kit.flag(0x44e) { slots / 2 } else { 0 };
        Some((least, slots))
    }

    /// The nonweapon slots lent to weapons (the file's +0x1df8).
    pub fn borrowed_nonweapon(&self, draft: &Draft) -> u32 {
        self.nonweapon_lending(draft)
            .map_or(0, |(least, most)| draft.lent_nonweapon.clamp(least, most))
    }

    /// The kit's focus (+0x4ac): the prompt and the weapons taken that
    /// may be the one, melee (+0x4ad), missile (+0x4ae), hurled (+0x4af)
    /// or any.
    pub fn focus_choices(&self, draft: &Draft) -> Option<(String, Vec<String>)> {
        let kit = self.draft_kit(draft)?;
        if !kit.flag(0x4ac) {
            return None;
        }
        let (prompt, test): (&str, fn(&WeaponRules) -> bool) = if kit.flag(0x4ad) {
            (tr("Select a melee weapon in which to focus."), |w| w.melee)
        } else if kit.flag(0x4ae) {
            (tr("Select a missile weapon in which to focus."), |w| {
                w.missile
            })
        } else if kit.flag(0x4af) {
            (tr("Select a hurled weapon in which to focus."), |w| {
                w.melee && w.missile
            })
        } else {
            (tr("Select a weapon in which to focus."), |_| true)
        };
        let weapons = draft
            .weapon_proficiencies
            .iter()
            .filter(|n| self.weapon_part(n).is_some_and(test))
            .cloned()
            .collect();
        Some((prompt.to_owned(), weapons))
    }

    /// Core Rules: whether specializations are chosen on this page, as
    /// the original's Specialization button.
    pub fn core_specializes(&self, draft: &Draft) -> bool {
        self.expansion && !draft.skills_and_powers
    }

    /// The free specialization: the one picked from the kit's list
    /// (+0x550), or the first taken from the list it makes free (+0x53c).
    pub fn free_specialization(&self, draft: &Draft) -> Option<String> {
        let kit = self.draft_kit(draft)?;
        if let Some(free) = &draft.free_specialization
            && kit.sa(0x550).contains(free)
        {
            return Some(free.clone());
        }
        let free = kit.sa(0x53c);
        draft
            .specializations
            .iter()
            .find(|s| free.contains(s))
            .cloned()
    }

    /// The weapons the kit lets the character pick a free specialization
    /// from (+0x550), Core Rules only.
    pub fn free_specialization_choices(&self, draft: &Draft) -> &[String] {
        match self.draft_kit(draft) {
            Some(kit) if self.core_specializes(draft) => kit.sa(0x550),
            _ => &[],
        }
    }

    /// Every specialization, the free one too.
    pub fn all_specializations(&self, draft: &Draft) -> Vec<String> {
        let mut all = draft.specializations.clone();
        if let Some(free) = self.free_specialization(draft)
            && !all.contains(&free)
        {
            all.push(free);
        }
        all
    }

    /// Core Rules: the slots specializing in a weapon takes on top of its
    /// proficiency (61de85): 2 for a bow unless the kit says 1 (+0x5d4),
    /// 1 for the others, nothing for the free one.
    pub fn core_specialization_cost(&self, draft: &Draft, name: &str) -> u32 {
        if self.free_specialization(draft).as_deref() == Some(name) {
            return 0;
        }
        if self.weapon_part(name).is_some_and(|w| w.bow) {
            self.bow_specialization_cost(draft)
        } else {
            1
        }
    }

    /// Core Rules: the slots specializing in a bow takes.
    pub fn bow_specialization_cost(&self, draft: &Draft) -> u32 {
        if self.draft_kit(draft).is_some_and(|k| k.flag(0x5d4)) {
            1
        } else {
            2
        }
    }

    /// Core Rules: the slots the specializations take.
    pub fn specialization_spent(&self, draft: &Draft) -> u32 {
        if !self.core_specializes(draft) {
            return 0;
        }
        self.all_specializations(draft)
            .iter()
            .map(|s| self.core_specialization_cost(draft, s))
            .sum()
    }

    /// Core Rules: the questions before specializing in a weapon, in the
    /// original's order (61e897); the kit may let others than single-class
    /// fighters specialize (+0x518, or the focus weapon with +0x4b3).
    pub fn specialization_questions(&self, draft: &Draft, name: &str) -> Vec<Question> {
        let kit = self.draft_kit(draft);
        let kit_allows = kit.is_some_and(|k| {
            k.flag(0x518)
                || (k.flag(0x4ac) && k.flag(0x4b3) && draft.weapon_focus.as_deref() == Some(name))
        });
        let fighter = draft
            .classes
            .iter()
            .filter_map(|c| self.class(c))
            .any(|c| c.core_specialization);
        let not_allowed = |text: &str| Question {
            title: tr("Not Allowed").into(),
            text: text.into(),
        };
        let mut questions = Vec::new();
        if !fighter && !kit_allows {
            questions.push(not_allowed(tr(
                "Only fighters may specialize. Specialize anyway?",
            )));
        }
        if draft.classes.len() > 1 && !kit_allows {
            questions.push(not_allowed(tr(
                "Only single-class fighters may specialize. Specialize anyway?",
            )));
        }
        if !self.all_specializations(draft).is_empty() {
            questions.push(not_allowed(tr(
                "Only one specialization is allowed. Specialize anyway?",
            )));
        }
        if kit.is_some_and(|k| k.flag(0x538)) && self.weapon_part(name).is_some_and(|w| w.melee) {
            questions.push(Question {
                title: tr("Kit Restriction").into(),
                text: tr!(
                    "The kit \"{kit}\" does not allow specialization of melee weapons during \
                     character creation. Do you wish to ignore this?",
                    kit = draft.kit
                ),
            });
        }
        questions
    }

    /// The warning specializing in a weapon gives, when the kit allows
    /// specializations only from a list (+0x524).
    pub fn specialization_warnings(&self, draft: &Draft, name: &str) -> Vec<WeaponWarning> {
        let Some(kit) = self.draft_kit(draft) else {
            return Vec::new();
        };
        let list = kit.sa(0x524);
        if list.is_empty()
            || list.iter().any(|n| n == name)
            || draft
                .ignored_weapon_checks
                .contains(&WeaponCheck::SpecializationList)
        {
            return Vec::new();
        }
        vec![WeaponWarning {
            check: WeaponCheck::SpecializationList,
            text: format!(
                "{}\n{}",
                tr!(
                    "The kit \"{kit}\" only allows you to select specializations from the \
                     following list.",
                    kit = draft.kit
                ),
                list.join(", ")
            ),
        }]
    }

    /// Whether specializing in the weapon fits the weapon slots left.
    pub fn specialization_fits(&self, draft: &Draft, name: &str) -> bool {
        self.weapon_spent(draft) + self.core_specialization_cost(draft, name)
            <= self.weapon_slots_for(draft)
    }

    /// The warnings on leaving the page about the specializations the kit
    /// requires (5d2bef, 5d2f71), leaving out those ignored.
    pub fn leaving_warnings(&self, draft: &Draft) -> Vec<WeaponWarning> {
        let Some(kit) = self.draft_kit(draft) else {
            return Vec::new();
        };
        let taken = self.all_specializations(draft);
        let mut warnings = Vec::new();
        let mut warn = |check, text| {
            if !draft.ignored_weapon_checks.contains(&check) {
                warnings.push(WeaponWarning { check, text });
            }
        };
        let required = kit.sa(0x3e8);
        if !required.is_empty() && !taken.iter().any(|t| required.contains(t)) {
            warn(
                WeaponCheck::SpecializationRequired,
                format!(
                    "{}\n{}",
                    tr!(
                        "The kit \"{kit}\" requires that you select a specialization from the \
                         following list.",
                        kit = draft.kit
                    ),
                    required.join(", ")
                ),
            );
        }
        let level = kit.i_or(0x51c, 101);
        if level <= 1 {
            let (melee, missile) = (kit.flag(0x520), kit.flag(0x521));
            let fits = |w: &WeaponRules| {
                if melee {
                    w.melee
                } else if missile {
                    w.missile
                } else {
                    w.melee || w.missile
                }
            };
            if !taken.iter().any(|t| self.weapon_part(t).is_some_and(fits)) {
                let text = if melee {
                    tr!(
                        "The kit \"{kit}\" requires that you select a melee weapon \
                         specialization by level {level}.",
                        kit = draft.kit,
                        level
                    )
                } else if missile {
                    tr!(
                        "The kit \"{kit}\" requires that you select a missile weapon \
                         specialization by level {level}.",
                        kit = draft.kit,
                        level
                    )
                } else {
                    tr!(
                        "The kit \"{kit}\" requires that you select either a melee or a \
                         missile weapon specialization by level {level}.",
                        kit = draft.kit,
                        level
                    )
                };
                warn(WeaponCheck::SpecializationByLevel, text);
            }
        }
        warnings
    }
}

/// A group of weapons to take: the one weapon, or how many of which
/// (0x4b2aab).
fn option_label(count: i32, names: &[String]) -> String {
    if count == 1 && names.len() == 1 {
        return names[0].clone();
    }
    tr!(
        "{count} of {names}",
        count = count.max(0),
        names = names.join(", ")
    )
}
