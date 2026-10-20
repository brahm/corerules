//! What the kit does on the Weapon Proficiencies page of the original's
//! Create Character (IDD 308): the slots or points (59e778), the weapons
//! it offers, what they cost and the questions before buying one
//! (0x444079), the warnings when adding one (5d4aa7), the nonweapon slots
//! a kit lets weapons use, the requirements it lists (5d44df), the weapon
//! to focus in, weapon specialization and unarmed combat (IDD 356,
//! 61e897).

use cr_formats::parts::Part;
use cr_i18n::tr;

use super::{BOW_GROUP, CreationRules, Draft};
use crate::dual::ClassRules;
use crate::npc::data::{Kit, PartExt};
use crate::npc::{BROAD_SWORDS, broad_has_tight};

/// The slots a Core Rules unarmed style takes (61e5c1).
const CORE_UNARMED_SLOTS: u32 = 2;

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
    /// Its tight groups' ids (+0x1cc).
    pub tight: Vec<i32>,
    /// Its damage types, and the Core Rules' own when set (+0x3b4,
    /// +0x3e4).
    pub damage: [String; 2],
    /// A thief may use it (+0x448).
    pub thief: bool,
    /// Not for the classes the lesser or major Weapon Use ability lets use
    /// weapons (+0x4c8, +0x4cc).
    pub ability_barred: [bool; 2],
    /// A wizard pays 3 points more for it out of class (+0x4d0).
    pub wizard_extra: bool,
    /// The Cavalier's horseman's bonus applies to it (+0x724).
    pub horseman: bool,
}

impl WeaponRules {
    pub fn bow(&self) -> bool {
        self.tight.contains(&BOW_GROUP)
    }

    /// In a tight group of the swords' broad group (4798c2).
    pub fn sword(&self) -> bool {
        self.tight.iter().any(|&t| broad_has_tight(t, BROAD_SWORDS))
    }

    /// Piercing or slashing (0x661802).
    pub fn edged(&self, core: bool) -> bool {
        let damage = match &self.damage {
            [_, own] if core && !own.is_empty() => own,
            [damage, _] => damage,
        };
        damage.contains('P') || damage.contains('S')
    }
}

/// What a class ability does to weapon proficiencies.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AbilityWeapons {
    /// Cleric Weapon Allowance: one edged weapon (+0x7a0).
    pub allowance: bool,
    /// Melee, cleric (not edged) or thief weapons only (+0x7c0, +0x7c4,
    /// +0x7c8).
    pub melee_only: bool,
    pub blunt_only: bool,
    pub thief_only: bool,
    /// Weapon Use Lesser and Major (+0x81c, +0x820).
    pub use_lesser: bool,
    pub use_major: bool,
    /// Skills & Powers: allows weapon specialization, and more than one
    /// (+0x782, +0x783).
    pub specialize: bool,
    pub multiple: bool,
}

impl AbilityWeapons {
    pub(super) fn of(part: &Part) -> Self {
        let on = |off| part.n(off) != 0;
        Self {
            allowance: on(0x7a0),
            melee_only: on(0x7c0),
            blunt_only: on(0x7c4),
            thief_only: on(0x7c8),
            use_lesser: on(0x81c),
            use_major: on(0x820),
            specialize: on(0x782),
            multiple: on(0x783),
        }
    }

    fn or(self, other: Self) -> Self {
        Self {
            allowance: self.allowance || other.allowance,
            melee_only: self.melee_only || other.melee_only,
            blunt_only: self.blunt_only || other.blunt_only,
            thief_only: self.thief_only || other.thief_only,
            use_lesser: self.use_lesser || other.use_lesser,
            use_major: self.use_major || other.use_major,
            specialize: self.specialize || other.specialize,
            multiple: self.multiple || other.multiple,
        }
    }
}

/// A warning the kit gives when a weapon proficiency is added, which the
/// user may ignore for good, as the original's IDD 245.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponCheck {
    /// No missile weapons at creation (kit +0x508; the file's +0x1de8).
    Ranged,
    /// Weapons the kit bars (+0x3ac; +0x1de3).
    Barred,
    /// The first sword from a list (+0x398; +0x1de2).
    Sword,
    /// Thrown weapons or those from a list (+0x3fc; +0x1de0).
    Thrown,
    /// Every odd-numbered one a missile or thrown weapon (+0x438;
    /// +0x1ddf).
    OddMissile,
    /// Some of the first ones from a list (+0x43c, +0x440, +0x424;
    /// +0x1dde).
    FirstFromList,
    /// Every other one from a list (+0x410; +0x1ddd).
    EveryOther,
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
    /// Skills & Powers: broad and tight weapon groups the kit bars
    /// (+0x3c0, +0x3d4; +0x1de4, +0x1de5).
    BroadGroup,
    TightGroup,
}

/// A question before taking something, which No answers by not taking
/// it, or a notice to acknowledge: its title and text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub title: String,
    pub text: String,
    /// Only to acknowledge, with OK.
    pub notice: bool,
}

impl Question {
    pub fn new(title: &str, text: String) -> Self {
        Self {
            title: title.into(),
            text,
            notice: false,
        }
    }

    /// Only to acknowledge, with OK.
    pub fn notice(title: &str, text: String) -> Self {
        Self {
            notice: true,
            ..Self::new(title, text)
        }
    }
}

/// A warning about a weapon to add: which one and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponWarning {
    pub check: WeaponCheck,
    pub text: String,
}

/// What buying a weapon proficiency costs and asks first.
pub(super) struct Offer {
    cost: u32,
    questions: Vec<Question>,
    /// The original's price is above 0 (0x444079): not a free weapon nor
    /// one the kit prices.
    pub charged: bool,
}

impl CreationRules {
    pub(super) fn weapon_part(&self, name: &str) -> Option<&WeaponRules> {
        self.weapons.iter().find(|w| w.name == name)
    }

    pub(super) fn draft_kit(&self, draft: &Draft) -> Option<&Kit> {
        self.kit(&draft.kit).filter(|_| self.expansion)
    }

    pub(super) fn draft_classes<'a>(
        &'a self,
        draft: &'a Draft,
    ) -> impl Iterator<Item = &'a ClassRules> {
        draft.classes.iter().filter_map(|c| self.class(c))
    }

    /// What the character's class abilities do to weapons.
    fn ability_weapons(&self, draft: &Draft) -> AbilityWeapons {
        (0..draft.classes.len())
            .flat_map(|slot| self.chosen_class_abilities(draft, slot))
            .filter_map(|a| self.ability_weapons.get(&a).copied())
            .fold(AbilityWeapons::default(), AbilityWeapons::or)
    }

    /// Whether a class's own rule lets it use the weapon (0x669550).
    fn class_allows(class: &ClassRules, weapon: &WeaponRules, core: bool) -> bool {
        if class.weapon_rule == "Any" || class.weapon_list.contains(&weapon.name) {
            return true;
        }
        if (class.no_edged && weapon.edged(core)) || class.weapon_rule == "Only from list below" {
            return false;
        }
        let damage = match &weapon.damage {
            [_, own] if core && !own.is_empty() => own,
            [damage, _] => damage,
        };
        damage == "none" || class.weapon_rule.contains(damage.as_str())
    }

    /// Whether one of the classes may use the weapon, or the kit gives it
    /// (+0x484), or a Weapon Use ability allows it (0x444079).
    fn weapon_in_class(&self, draft: &Draft, weapon: &WeaponRules) -> bool {
        if !self.expansion {
            return draft
                .classes
                .iter()
                .any(|c| !weapon.barred_classes.contains(c));
        }
        let core = !draft.skills_and_powers;
        let kit_gives = self
            .draft_kit(draft)
            .is_some_and(|k| k.sa(0x484).contains(&weapon.name));
        let some = (draft
            .classes
            .iter()
            .any(|c| !weapon.barred_classes.contains(c))
            || kit_gives)
            && self
                .draft_classes(draft)
                .any(|c| Self::class_allows(c, weapon, core));
        let abilities = self.ability_weapons(draft);
        some || (abilities.use_lesser && !weapon.ability_barred[0])
            || (abilities.use_major && !weapon.ability_barred[1])
    }

    /// An edged weapon a class may not use, as clerics, unless the kit
    /// gives it (+0x484).
    fn cleric_restricted(&self, draft: &Draft, weapon: &WeaponRules) -> bool {
        self.draft_classes(draft).any(|c| c.no_edged)
            && weapon.edged(!draft.skills_and_powers)
            && !self
                .draft_kit(draft)
                .is_some_and(|k| k.sa(0x484).contains(&weapon.name))
    }

    /// A weapon the kit (+0x470, up to the level) or a racial ability
    /// (+0x574, +0x578) makes free.
    fn weapon_free(&self, draft: &Draft, name: &str) -> bool {
        self.draft_kit(draft).is_some_and(|k| {
            k.free_weapons
                .iter()
                .any(|(w, l)| w == name && *l <= draft.level.max(1) as i32)
        }) || draft
            .racial_abilities
            .iter()
            .filter_map(|a| self.racial_weapons.get(a))
            .any(|weapons| weapons.iter().any(|w| w == name))
    }

    /// The weapons the page offers: every weapon, the classes' price and
    /// questions deciding (the Core Rules 2.0 only those the classes may
    /// use), and at 1st level only the kit's list when it has one
    /// (+0x4b8).
    pub fn weapon_choices(&self, draft: &Draft) -> Vec<String> {
        let only = match self.draft_kit(draft) {
            Some(kit) if draft.level <= 1 => kit.sa(0x4b8),
            _ => &[],
        };
        self.weapons
            .iter()
            .filter(|w| self.expansion || self.weapon_in_class(draft, w))
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

    /// What buying a weapon besides those `taken` costs and asks first, in
    /// the original's order (0x444079, 5cc802). The Core Rules charge a
    /// slot, or the kit's price where the original refuses the weapon
    /// (+0x460): an edged one for a cleric, or one out of class. Skills &
    /// Powers charge the classes' points, nothing for a free weapon, a
    /// point less for one the kit favors (+0x444, +0x384), and 1 more for
    /// one out of class (a wizard 2, or 3 with +0x4d0).
    pub(super) fn weapon_offer(&self, draft: &Draft, taken: &[String], name: &str) -> Offer {
        let core = !draft.skills_and_powers;
        let base = if core {
            1
        } else {
            self.weapon_cost(&draft.classes)
        };
        let offer = |cost, questions| Offer {
            cost,
            questions,
            charged: true,
        };
        let Some(weapon) = self.weapon_part(name).filter(|_| self.expansion) else {
            return offer(base, Vec::new());
        };
        if self.weapon_free(draft, name) {
            return Offer {
                charged: false,
                ..offer(u32::from(core), Vec::new())
            };
        }
        let kit = self.draft_kit(draft);
        let price = kit
            .map(|k| k.i_or(0x460, -1))
            .filter(|&p| p != -1)
            .map(|p| (p + 1).max(0) as u32);
        let abilities = self.ability_weapons(draft);
        let class_restriction = tr("Verify ignore Class Restriction");
        let mut questions = Vec::new();
        if self.cleric_restricted(draft, weapon) {
            if abilities.allowance {
                let restricted = taken
                    .iter()
                    .filter_map(|t| self.weapon_part(t))
                    .any(|w| self.cleric_restricted(draft, w));
                if restricted {
                    questions.push(Question::new(
                        class_restriction,
                        tr(
                            "Your Cleric character already has at least one restricted Weapon \
                            Proficiency. Would you like to purchase another anyway?",
                        )
                        .into(),
                    ));
                }
                questions.push(Question {
                    notice: true,
                    ..Question::new(
                        class_restriction,
                        tr(
                            "This Weapon Proficiency is restricted to Clerics. Since you have \
                            the Class Ability 'Weapon Allowance' this will be your favored edged \
                            weapon.",
                        )
                        .into(),
                    )
                });
            } else if core && let Some(price) = price {
                return Offer {
                    charged: false,
                    ..offer(price, questions)
                };
            } else {
                questions.push(Question::new(
                    class_restriction,
                    tr(
                        "This Weapon Proficiency is not allowed for Clerics. Would you like to \
                        purchase it anyway?",
                    )
                    .into(),
                ));
            }
        }
        let mut cost = base;
        if !core && kit.is_some_and(|k| k.flag(0x444) || k.sa(0x384).iter().any(|n| n == name)) {
            cost = cost.saturating_sub(1);
        }
        if kit.is_some_and(|k| k.flag(0x79)) && weapon.ranged {
            questions.push(Question::new(
                tr("Verify ignore Kit Restriction"),
                tr(
                    "The selected kit limits the character from weapons which fire ranged \
                    attacks. This weapon fires ranged attacks. Would you like to purchase it \
                    anyway?",
                )
                .into(),
            ));
        }
        if !self.weapon_in_class(draft, weapon) {
            if !core {
                let wizard = self.draft_classes(draft).any(|c| c.group == 3);
                cost += match (wizard, weapon.wizard_extra) {
                    (false, _) => 1,
                    (true, false) => 2,
                    (true, true) => 3,
                };
            } else if let Some(price) = price {
                return Offer {
                    charged: false,
                    ..offer(price, questions)
                };
            } else {
                questions.push(Question::new(
                    class_restriction,
                    tr(
                        "This Weapon Proficiency is not allowed to your class. Would you like \
                        to purchase it anyway?",
                    )
                    .into(),
                ));
            }
        }
        let restrictions = [
            (
                abilities.melee_only && !weapon.melee,
                tr(
                    "This character has a class restriction limiting it to melee weapons. The \
                    selected weapon proficiency is not a melee weapon. Would you like to \
                    purchase this proficiency anyway?",
                ),
            ),
            (
                abilities.blunt_only && weapon.edged(core),
                tr(
                    "This character has a class restriction limiting it to non-edged weapons. \
                    Would you like to purchase this proficiency anyway?",
                ),
            ),
            (
                abilities.thief_only && !weapon.thief,
                tr(
                    "This character has a class restriction limiting it to thief weapons. This \
                    weapon is not available to thieves. Would you like to purchase this \
                    proficiency anyway?",
                ),
            ),
        ];
        for (_, text) in restrictions.into_iter().filter(|r| r.0) {
            questions.push(Question::new(class_restriction, text.into()));
        }
        offer(if core { 1 } else { cost }, questions)
    }

    /// What a weapon proficiency costs (see [`Self::weapon_questions`]
    /// for the questions buying it asks).
    pub fn weapon_cost_for(&self, draft: &Draft, name: &str) -> u32 {
        self.weapon_offer(draft, &draft.weapon_proficiencies, name)
            .cost
    }

    /// The slots or points the weapon proficiencies take, with the Core
    /// Rules the specializations' and unarmed styles' too.
    pub fn weapon_spent(&self, draft: &Draft) -> u32 {
        let core_unarmed = if self.core_specializes(draft) {
            draft.unarmed.len() as u32 * CORE_UNARMED_SLOTS
        } else {
            0
        };
        draft
            .weapon_proficiencies
            .iter()
            .map(|w| self.weapon_cost_for(draft, w))
            .sum::<u32>()
            + self.group_spent(draft)
            + self.specialization_spent(draft)
            + core_unarmed
    }

    /// The questions buying a weapon besides those `taken` asks first, in
    /// the original's order; No to any of them does not buy it.
    pub fn weapon_questions(&self, draft: &Draft, taken: &[String], name: &str) -> Vec<Question> {
        self.weapon_offer(draft, taken, name).questions
    }

    /// Notes a weapon added to those `taken`: the kit that wants every
    /// other one from its list remembers whether this one was (5d1501).
    pub fn weapon_added(&self, draft: &mut Draft, taken: &[String], name: &str) {
        if let Some(list) = self.every_other_list(draft, taken) {
            draft.every_other_listed = list.iter().any(|n| n == name);
        }
    }

    /// The kit's list for every other weapon, while it still applies.
    fn every_other_list<'a>(&'a self, draft: &Draft, taken: &[String]) -> Option<&'a [String]> {
        let list = self.draft_kit(draft)?.sa(0x410);
        let taken = self.with_groups(draft, taken);
        let applies = !list.is_empty()
            && !draft
                .ignored_weapon_checks
                .contains(&WeaponCheck::EveryOther)
            && !list.iter().all(|n| taken.contains(n));
        applies.then_some(list)
    }

    /// The warnings adding a weapon to those `taken` gives, in the
    /// original's order, leaving out those ignored; the weapons the
    /// groups cover count as taken.
    pub fn weapon_warnings(
        &self,
        draft: &Draft,
        taken: &[String],
        name: &str,
    ) -> Vec<WeaponWarning> {
        let (Some(kit), Some(weapon)) = (self.draft_kit(draft), self.weapon_part(name)) else {
            return Vec::new();
        };
        let taken = &self.with_groups(draft, taken);
        let kit_name = draft.kit.as_str();
        let listed = |text: String, list: &[String]| format!("{text}\n{}", list.join(", "));
        let mut warnings = Vec::new();
        let mut warn = |check, text| {
            if !draft.ignored_weapon_checks.contains(&check) {
                warnings.push(WeaponWarning { check, text });
            }
        };
        // Creation only: no missile weapons (+0x508).
        if kit.flag(0x508) && weapon.missile && !draft.updating {
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
        // The first sword from the list; then any (5d3345).
        let swords = kit.sa(0x398);
        if !swords.is_empty()
            && weapon.sword()
            && !swords.contains(&weapon.name)
            && !taken.iter().any(|t| swords.contains(t))
        {
            warn(
                WeaponCheck::Sword,
                listed(
                    tr!(
                        "The kit \"{kit}\" requires that you select a sword weapon proficiency \
                         from the following list.",
                        kit = kit_name
                    ),
                    swords,
                ),
            );
        }
        let thrown = kit.sa(0x3fc);
        if !thrown.is_empty() && !weapon.missile && !thrown.contains(&weapon.name) {
            warn(
                WeaponCheck::Thrown,
                listed(
                    tr!(
                        "The kit \"{kit}\" requires that you select a thrown weapon or one from \
                         the following list.",
                        kit = kit_name
                    ),
                    thrown,
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
        // After one off the list, the next from it (5d1501).
        if let Some(list) = self.every_other_list(draft, taken)
            && !draft.every_other_listed
            && !list.contains(&weapon.name)
        {
            warn(
                WeaponCheck::EveryOther,
                listed(
                    tr!(
                        "The kit \"{kit}\" requires that you select every other weapon \
                         proficiency from the following list.",
                        kit = kit_name
                    ),
                    list,
                ),
            );
        }
        if kit.flag(0x76) && !draft.updating {
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
        if self.weapon_part(name).is_some_and(|w| w.bow()) {
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

    /// Skills & Powers: whether a class ability allows specialization
    /// (+0x782).
    pub(super) fn abilities_allow_specialization(&self, draft: &Draft) -> bool {
        self.ability_weapons(draft).specialize
    }

    /// The questions before specializing in a weapon, in the original's
    /// order (61e897, 61fdab): with the Core Rules whether others than a
    /// single-class fighter may (the kit may let them, +0x518, or the
    /// focus weapon with +0x4b3) and whether one more may; then whether a
    /// melee weapon may at creation (+0x538).
    pub fn specialization_questions(&self, draft: &Draft, name: &str) -> Vec<Question> {
        let kit = self.draft_kit(draft);
        let mut questions = Vec::new();
        if self.core_specializes(draft) {
            let kit_allows = kit.is_some_and(|k| {
                k.flag(0x518)
                    || (k.flag(0x4ac)
                        && k.flag(0x4b3)
                        && draft.weapon_focus.as_deref() == Some(name))
            });
            let fighter = self.draft_classes(draft).any(|c| c.core_specialization);
            let not_allowed = |text: &str| Question::new(tr("Not Allowed"), text.into());
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
        }
        if !self.core_specializes(draft) && draft.updating {
            // Update Character asks where New Character refuses.
            let abilities = self.ability_weapons(draft);
            if !abilities.specialize && !kit.is_some_and(|k| k.flag(0x518)) {
                questions.push(Question::new(
                    tr("Not Allowed"),
                    tr("This character does not have any Class Abilities allowing \
                        specialization. Add specialization anyway?")
                    .into(),
                ));
            }
            if !self.level_allows_specialization(draft) {
                questions.push(Question::new(
                    tr("Not Allowed"),
                    tr(
                        "This character is not of high enough level to specialize. Add \
                        specialization anyway?",
                    )
                    .into(),
                ));
            }
            if !draft.specializations.is_empty() && !abilities.multiple {
                questions.push(Question::new(
                    tr("Warning"),
                    tr(
                        "This character does not have any Class Abilities allowing multiple \
                        specialization. Add specialization anyway?",
                    )
                    .into(),
                ));
            }
        }
        if kit.is_some_and(|k| k.flag(0x538))
            && !draft.updating
            && self.weapon_part(name).is_some_and(|w| w.melee)
        {
            questions.push(Question::new(
                tr("Kit Restriction"),
                tr!(
                    "The kit \"{kit}\" does not allow specialization of melee weapons during \
                     character creation. Do you wish to ignore this?",
                    kit = draft.kit
                ),
            ));
        }
        questions
    }

    /// Skills & Powers: whether the highest class level lets the character
    /// specialize (0x43a17e): always with the kit's +0x518; a class that
    /// specializes early when single-classed or past 1st level, warriors
    /// past 2nd, priests past 4th, rogues and psionicists past 5th, wizards
    /// past 6th.
    pub(super) fn level_allows_specialization(&self, draft: &Draft) -> bool {
        if self.draft_kit(draft).is_some_and(|k| k.flag(0x518)) {
            return true;
        }
        let top = draft.level.max(1);
        if self.draft_classes(draft).any(|c| c.early_specialization) {
            return draft.classes.len() == 1 || top > 1;
        }
        let groups = self.class_groups(&draft.classes);
        [(0, 2), (2, 4), (1, 5), (4, 5), (3, 6)]
            .iter()
            .find(|&&(g, _)| groups[g])
            .is_some_and(|&(_, past)| top > past)
    }

    /// Whether the character may take an unarmed style.
    pub fn unarmed_allowed(&self, draft: &Draft) -> bool {
        self.unarmed_cost(draft).is_some()
    }

    /// Update Character: the questions before taking an unarmed style
    /// (6208ac), whether the classes allow it while none is held and
    /// whether to have it with a weapon specialization; Yes to both keeps
    /// everything.
    pub fn unarmed_questions(&self, draft: &Draft) -> Vec<Question> {
        let mut questions = Vec::new();
        if !self.unarmed_allowed(draft) && draft.unarmed.is_empty() {
            questions.push(Question::new(
                tr("Warning Message"),
                tr("This class is not allowed Unarmed Combat. Take it anyway?").into(),
            ));
        }
        questions.extend(self.unarmed_conflict(draft));
        questions
    }

    /// Core Rules weapon slots left in Update Character (0x4397d1): the
    /// classes' first slots and one more every so many levels (each
    /// class's for a dual-class character, or combined with +0x486d), or
    /// the kit's (+0x454, +0x44c, +0x458, +0x45c, +0x448, +0x468), less
    /// what the proficiencies, specializations and unarmed styles take,
    /// plus the nonweapon slots lent. `levels` are the classes' levels.
    pub fn core_weapon_slots_left(&self, draft: &Draft, levels: &[u32], dual: bool) -> i64 {
        let kit = self.draft_kit(draft);
        let ki = |off, default| kit.map_or(default, |k| k.i_or(off, default));
        let every_slot = kit.is_some_and(|k| k.flag(0x44c));
        let (per_level, from_level) = (ki(0x458, -1), ki(0x45c, -1));
        let combine = self.draft_classes(draft).any(|c| c.combine_slots);
        let (mut first, mut per, mut top, mut count, mut more) = (0, i32::MAX, 0, 0, 0);
        for (class, &level) in self.draft_classes(draft).zip(levels) {
            let level = level as i32;
            let (initial, step) = (class.weapon_slots, class.weapon_slot_levels);
            if dual && step != 0 {
                first += initial + level.div_euclid(step);
            } else if combine {
                first += initial;
                per = per.wrapping_add(step);
                top = top.max(level);
            } else {
                first = first.max(initial);
                per = per.min(step);
                top = top.max(level);
            }
            if every_slot {
                count += i32::from(level > 0) + i32::from(level > 2) + i32::from(level > 5);
            }
            if per_level > 0 && level > per_level {
                more = level / per_level;
            }
        }
        if ki(0x454, -1) > -1 {
            first = ki(0x454, -1);
        }
        let classes = draft.classes.len() as i32;
        if !dual && classes > 1 {
            count = count.div_euclid(classes);
            more = more.div_euclid(classes);
        }
        let mut slots = if every_slot {
            count
        } else if per_level < 1 {
            match per {
                0 | i32::MAX => first,
                per => first + top.div_euclid(per),
            }
        } else {
            first + more
        };
        let level = draft.level.max(1) as i32;
        if from_level != -1 && level < from_level {
            slots = first;
        }
        slots += ki(0x448, 0);
        if ki(0x468, 0) != 0 && ki(0x46c, 0) <= level {
            slots += ki(0x468, 0);
        }
        let price = kit
            .map(|k| k.i_or(0x460, -1))
            .filter(|&p| p != -1)
            .map(|p| i64::from(p) + 1);
        let taken: i64 = draft
            .weapon_proficiencies
            .iter()
            .map(|name| {
                let grants = kit.is_some_and(|k| {
                    k.free_weapons.iter().any(|(w, l)| w == name && *l <= level)
                        || k.sa(0x484).contains(name)
                });
                let out_of_class = self
                    .weapon_part(name)
                    .is_some_and(|w| !self.weapon_in_class(draft, w));
                match price {
                    _ if grants => 0,
                    Some(price) if out_of_class => price,
                    _ => 1,
                }
            })
            .sum();
        let specializations: u32 = self
            .all_specializations(draft)
            .iter()
            .map(|s| self.core_specialization_cost(draft, s))
            .sum();
        i64::from(slots) - taken - i64::from(specializations) - 2 * draft.unarmed.len() as i64
            + i64::from(draft.lent_nonweapon)
    }

    /// Core Rules: whether one more unarmed style fits the weapon slots.
    pub fn unarmed_fits(&self, draft: &Draft) -> bool {
        self.weapon_spent(draft) + CORE_UNARMED_SLOTS <= self.weapon_slots_for(draft)
    }

    /// The question taking an unarmed style asks when the character
    /// specializes in a weapon (6208ac); in New Character Yes drops the
    /// specializations, in Update Character it keeps both.
    pub fn unarmed_conflict(&self, draft: &Draft) -> Option<Question> {
        let specialized = if self.core_specializes(draft) {
            !self.all_specializations(draft).is_empty()
        } else {
            !draft.specializations.is_empty()
        };
        specialized.then(|| {
            Question::new(
                tr("Warning Message"),
                if draft.updating {
                    tr(
                        "You may not specialize in both a weapon and unarmed combat. Specialize \
                        in both anyway?",
                    )
                } else {
                    tr(
                        "You may not specialize in both a weapon and unarmed combat. Remove the \
                        weapon specialization?",
                    )
                }
                .into(),
            )
        })
    }

    /// The question specializing in a weapon asks when the character has
    /// an unarmed style (61edf1); in New Character Yes drops the unarmed
    /// styles, in Update Character it keeps both.
    pub fn specialization_conflict(&self, draft: &Draft) -> Option<Question> {
        (!draft.unarmed.is_empty()).then(|| {
            Question::new(
                tr("Warning"),
                if draft.updating {
                    tr(
                        "You may not specialize in both a weapon and unarmed combat. Specialize \
                        in both anyway?",
                    )
                } else {
                    tr(
                        "You may not specialize in both a weapon and unarmed combat. Remove \
                        unarmed combat?",
                    )
                }
                .into(),
            )
        })
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
        if level <= draft.level.max(1) as i32 {
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
