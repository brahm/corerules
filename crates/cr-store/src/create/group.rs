//! The Player's Option weapon groups on the Weapon Proficiencies page
//! (IDD 308): who may buy them, what they cost and ask first (5cde66,
//! 5cdf08), the kit's warnings (5d3d63, 5d4121), when they may go, and
//! the weapons they make the character proficient in (5d1ac7, 0x431db1).

use cr_i18n::tr;

use super::{CreationRules, Draft, Question, WeaponCheck, WeaponWarning};

/// The points a broad and a tight group cost.
const BROAD_COST: u32 = 6;
const TIGHT_COST: u32 = 4;

/// What buying a weapon group costs and asks first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupOffer {
    pub cost: u32,
    pub questions: Vec<Question>,
}

impl CreationRules {
    /// The broad (else tight) groups to buy, with Skills & Powers only.
    pub fn group_choices(&self, draft: &Draft, broad: bool) -> Vec<String> {
        if !draft.skills_and_powers {
            return Vec::new();
        }
        let (all, taken) = if broad {
            (self.groups.broad(), &draft.broad_groups)
        } else {
            (self.groups.tight(), &draft.tight_groups)
        };
        all.into_iter().filter(|g| !taken.contains(g)).collect()
    }

    /// The weapons a group covers, for its row.
    pub fn group_weapons(&self, broad: bool, name: &str) -> Vec<String> {
        let group = [name.to_owned()];
        let (broad, tight): (&[String], &[String]) =
            if broad { (&group, &[]) } else { (&[], &group) };
        self.weapons
            .iter()
            .filter(|w| self.groups.covers(broad, tight, &w.tight))
            .map(|w| w.name.clone())
            .collect()
    }

    /// What buying a broad (else tight) group costs and asks first, or why
    /// the character may not (5cde66, 5cdf08): a priest may, asked first
    /// when a class says so (+0x27a1), a warrior may, others not.
    pub fn group_offer(&self, draft: &Draft, broad: bool) -> Result<GroupOffer, String> {
        let groups = self.class_groups(&draft.classes);
        let mut questions = Vec::new();
        if groups[2] {
            if self.draft_classes(draft).any(|c| c.group_question) {
                questions.push(Question::new(
                    if broad {
                        tr("Allow Cleric to buy Weapon Group?")
                    } else {
                        tr("Allow Cleric to buy Weapon Group")
                    },
                    tr(
                        "Purchasing weapon groups for Clerics might allow proficiencies in \
                        weapons which are restricted to clerics. Purchase anyway?",
                    )
                    .into(),
                ));
            }
        } else if !groups[0] {
            return Err(tr("Only Warriors are allowed to buy weapon Groups").into());
        }
        Ok(GroupOffer {
            cost: if broad { BROAD_COST } else { TIGHT_COST },
            questions,
        })
    }

    /// The points the groups bought take.
    pub fn group_spent(&self, draft: &Draft) -> u32 {
        if !draft.skills_and_powers {
            return 0;
        }
        draft.broad_groups.len() as u32 * BROAD_COST + draft.tight_groups.len() as u32 * TIGHT_COST
    }

    /// The warning buying a broad (else tight) group gives when the kit
    /// bars it (+0x3c0, +0x3d4), unless ignored.
    pub fn group_warnings(&self, draft: &Draft, broad: bool, name: &str) -> Vec<WeaponWarning> {
        let Some(kit) = self.draft_kit(draft) else {
            return Vec::new();
        };
        let (check, list) = if broad {
            (WeaponCheck::BroadGroup, kit.sa(0x3c0))
        } else {
            (WeaponCheck::TightGroup, kit.sa(0x3d4))
        };
        if !list.iter().any(|g| g == name) || draft.ignored_weapon_checks.contains(&check) {
            return Vec::new();
        }
        let text = if broad {
            tr!(
                "The kit \"{kit}\" does not allow you to select broad groups from the \
                 following list.",
                kit = draft.kit
            )
        } else {
            tr!(
                "The kit \"{kit}\" does not allow you to select tight groups from the \
                 following list.",
                kit = draft.kit
            )
        };
        vec![WeaponWarning {
            check,
            text: format!("{text}\n{}", list.join(", ")),
        }]
    }

    /// Why a broad (else tight) group may not go: while the character has
    /// a weapon of choice, expertise or specializations.
    pub fn group_removal(&self, draft: &Draft, broad: bool) -> Option<String> {
        let specialized = draft.weapon_of_choice.is_some()
            || draft.expertise.is_some()
            || !draft.specializations.is_empty();
        specialized.then(|| {
            if broad {
                tr(
                    "You must remove all specialization information before you can remove \
                    broad groups.",
                )
            } else {
                tr(
                    "You must remove all specialization information before you can remove \
                    tight groups.",
                )
            }
            .into()
        })
    }

    /// The weapons the character is proficient in: those `taken` and,
    /// with Skills & Powers, every other one the groups bought cover.
    pub(super) fn with_groups(&self, draft: &Draft, taken: &[String]) -> Vec<String> {
        let mut all = taken.to_vec();
        if draft.skills_and_powers {
            for w in &self.weapons {
                if !all.contains(&w.name)
                    && self
                        .groups
                        .covers(&draft.broad_groups, &draft.tight_groups, &w.tight)
                {
                    all.push(w.name.clone());
                }
            }
        }
        all
    }

    /// The weapons the character is proficient in, the groups' too: those
    /// a weapon of choice, expertise or specialization may be.
    pub fn covered_weapons(&self, draft: &Draft) -> Vec<String> {
        self.with_groups(draft, &draft.weapon_proficiencies)
    }
}
