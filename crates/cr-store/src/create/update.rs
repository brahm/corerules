//! New Character's weapon rules as Update Character applies them: an
//! existing character seen as a [`Draft`] at its highest class level, the
//! file's flags for the kit's warnings ignored, and the weapons the kit
//! has the character pick as it rises in level.

use cr_formats::Object;
use cr_formats::character::{Character, CharacterData};
use cr_i18n::tr;

use super::{CreationRules, Draft, PickKind, UNARMED, WeaponCheck};
use crate::settings::SETTINGS;

/// The kit's warnings the file says were ignored, as [`write_checks`]
/// keeps them.
pub(crate) fn ignored_checks(d: &CharacterData) -> Vec<WeaponCheck> {
    let on = |f: Option<u8>| f.is_some_and(|f| f != 0);
    let groups = d.flags_0x1ddf.unwrap_or_default();
    let every_other = d.flags_0x1ddc.unwrap_or([1, 0]);
    let specialization = d.flags_0x1de6.unwrap_or_default();
    let creation = d.flags_0x1de8.unwrap_or_default();
    [
        (creation[0] != 0, WeaponCheck::Ranged),
        (groups[3] != 0, WeaponCheck::Barred),
        (groups[2] != 0, WeaponCheck::Sword),
        (groups[1] != 0, WeaponCheck::Thrown),
        (groups[0] != 0, WeaponCheck::OddMissile),
        (groups[4] != 0, WeaponCheck::BroadGroup),
        (groups[5] != 0, WeaponCheck::TightGroup),
        (on(d.flag_0x1dde), WeaponCheck::FirstFromList),
        (every_other[1] != 0, WeaponCheck::EveryOther),
        (creation[1] != 0, WeaponCheck::Recommended),
        (specialization[0] != 0, WeaponCheck::SpecializationList),
        (on(d.flag_0x1de1), WeaponCheck::SpecializationRequired),
        (specialization[1] != 0, WeaponCheck::SpecializationByLevel),
    ]
    .into_iter()
    .filter(|c| c.0)
    .map(|c| c.1)
    .collect()
}

/// Writes which of the kit's warnings were ignored, and whether the last
/// weapon added was from the kit's list for every other one (+0x1ddc).
pub(crate) fn write_checks(d: &mut CharacterData, ignored: &[WeaponCheck], listed: bool) {
    let ignored = |check| u8::from(ignored.contains(&check));
    d.flag_0x1dde = Some(ignored(WeaponCheck::FirstFromList));
    if let Some(flags) = d.flags_0x1ddf.as_mut() {
        flags[0] = ignored(WeaponCheck::OddMissile);
        flags[1] = ignored(WeaponCheck::Thrown);
        flags[2] = ignored(WeaponCheck::Sword);
        flags[3] = ignored(WeaponCheck::Barred);
        flags[4] = ignored(WeaponCheck::BroadGroup);
        flags[5] = ignored(WeaponCheck::TightGroup);
    }
    d.flags_0x1ddc = Some([u8::from(listed), ignored(WeaponCheck::EveryOther)]);
    d.flags_0x1de8 = Some([
        ignored(WeaponCheck::Ranged),
        ignored(WeaponCheck::Recommended),
    ]);
    d.flag_0x1de1 = Some(ignored(WeaponCheck::SpecializationRequired));
    d.flags_0x1de6 = Some([
        ignored(WeaponCheck::SpecializationList),
        ignored(WeaponCheck::SpecializationByLevel),
    ]);
}

/// A weapon the kit has the character pick once (+0x380; 0x49777b): the
/// Cavalier's sword from 3rd level and its horseman's weapon from 5th,
/// kept in `unknown_0x1e68[4]` and `[5]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitPick {
    /// 0 for the sword, 1 for the horseman's weapon.
    pub slot: usize,
    pub title: String,
    pub text: String,
    /// The weapon proficiencies it may be.
    pub choices: Vec<String>,
}

impl CreationRules {
    /// An existing character as Update Character's pages see it, at the
    /// highest of its class `levels`.
    pub fn update_draft(&self, character: &Character, levels: &[u32]) -> Draft {
        let d = &character.data;
        let mut draft = Draft::new(&SETTINGS[0]);
        draft.skills_and_powers = character.skills_and_powers();
        draft.race.clone_from(&d.race);
        draft.subrace.clone_from(&d.subrace);
        draft.classes.clone_from(&d.classes.names);
        draft.kit.clone_from(&d.kit);
        draft.racial_abilities.clone_from(&d.racial_abilities);
        draft.class_abilities = d
            .class_slots
            .iter()
            .take(draft.classes.len())
            .map(|s| s.abilities.clone())
            .collect();
        draft
            .weapon_proficiencies
            .clone_from(&d.weapon_proficiencies);
        draft.broad_groups.clone_from(&d.unknown_0xac[1]);
        draft.tight_groups.clone_from(&d.unknown_0xac[2]);
        draft.specializations.clone_from(&d.unknown_0xac[3]);
        let named = |s: &String| (!s.is_empty()).then(|| s.clone());
        draft.weapon_of_choice = named(&d.unknown_0x124[0]);
        draft.expertise = named(&d.unknown_0x124[1]);
        let unarmed = d.flags_0x1e54.unwrap_or_default();
        draft.unarmed = UNARMED
            .iter()
            .zip(unarmed)
            .filter(|(_, f)| *f != 0)
            .map(|(u, _)| (*u).to_owned())
            .collect();
        draft.ignored_weapon_checks = ignored_checks(d);
        draft.every_other_listed = d.flags_0x1ddc.is_none_or(|f| f[0] != 0);
        draft.lent_nonweapon = d.unknown_0x1df8.unwrap_or(0);
        draft.weapon_focus = d.unknown_0x1e68.as_ref().and_then(|t| named(&t[6]));
        draft.free_specialization = d
            .unknown_0x1dc8
            .as_ref()
            .and_then(|f| f.first())
            .and_then(named);
        draft.level = levels.iter().copied().max().unwrap_or(1).max(1);
        draft.class_levels = levels.to_vec();
        draft.held_scores = Some(std::array::from_fn(|a| match a {
            0 => d.strength,
            a => d.abilities[a - 1],
        }));
        self.update_nonweapon(&mut draft, d);
        draft.proficiencies_started = true;
        draft.updating = true;
        draft
    }

    /// What the file keeps of the nonweapon proficiencies page: the
    /// proficiencies, the picks from the classes' and the kit's lists, the
    /// ones chosen at the classes' levels and the kit's questions answered.
    fn update_nonweapon(&self, draft: &mut Draft, d: &CharacterData) {
        draft
            .nonweapon_proficiencies
            .clone_from(&d.nonweapon_proficiencies);
        let bonus = d.unknown_0x1930.as_ref().map_or(&[][..], |l| &l[1][..]);
        for pick in self.nonweapon_picks(draft) {
            let picked = match pick.kind {
                PickKind::Bonus(_) => pick
                    .names
                    .iter()
                    .filter(|n| bonus.contains(n))
                    .cloned()
                    .collect(),
                PickKind::PlusTwo => d.unknown_0x1958.clone().unwrap_or_default(),
                PickKind::Free => (!d.unknown_0x1e20.is_empty())
                    .then(|| d.unknown_0x1e20.clone())
                    .into_iter()
                    .collect(),
                PickKind::PlusOne => d.unknown_0xac[5].clone(),
            };
            draft.set_picked(pick.kind, picked);
        }
        draft.level_nonweapon = d
            .unknown_0x1e28
            .iter()
            .flatten()
            .filter_map(|o| match o {
                Object::KitBonus(b) => Some(b),
                _ => None,
            })
            .filter(|b| {
                let level = draft.class_levels.get(b.unknown_0xc as usize).copied();
                level.is_some_and(|l| b.unknown_0x8 >= 1 && b.unknown_0x8 as u32 <= l)
            })
            .map(|b| b.name.clone())
            .collect();
        draft.barred_nonweapon_ignored = d.flags_0x1dea.is_some_and(|f| f[0] != 0);
        draft.kit_nonweapon_base = d.unknown_0x1df0.and_then(|(choice, _)| match choice {
            1 => Some(true),
            0 => None,
            _ => Some(false),
        });
        draft.by_level_declined = d.unknown_0x1df0.is_some_and(|(_, declined)| declined != 0);
    }

    /// Whether buying the weapon takes a slot the Core Rules count in
    /// Update Character, which asks first when none is left: not a free
    /// one, nor one the kit prices (+0x460).
    pub fn weapon_takes_slot(&self, draft: &Draft, name: &str) -> bool {
        self.weapon_offer(draft, &draft.weapon_proficiencies, name)
            .charged
    }

    /// The weapons the kit has the character pick that are still to be
    /// picked, among those it is proficient in: `picked` are those
    /// already kept.
    pub fn kit_picks(&self, draft: &Draft, picked: &[Option<String>; 2]) -> Vec<KitPick> {
        if !self.draft_kit(draft).is_some_and(|k| k.flag(0x380)) {
            return Vec::new();
        }
        let covered = self.covered_weapons(draft);
        let held = |test: fn(&super::WeaponRules) -> bool| -> Vec<String> {
            covered
                .iter()
                .filter(|n| self.weapon_part(n).is_some_and(test))
                .cloned()
                .collect()
        };
        let mut picks = Vec::new();
        if draft.level > 2 && picked[0].is_none() {
            picks.push(KitPick {
                slot: 0,
                title: tr("Selected sword").into(),
                text: tr(
                    "You may select one sword which gets attack bonuses. Once the character \
                     is saved you will not be able to change the selection.",
                )
                .into(),
                choices: held(|w| w.sword()),
            });
        }
        if draft.level > 4 && picked[1].is_none() {
            picks.push(KitPick {
                slot: 1,
                title: tr("Selected weapon").into(),
                text: tr(
                    "You may select one weapon which gets attack bonuses. Once the character \
                     is saved you will not be able to change the selection.",
                )
                .into(),
                choices: held(|w| w.horseman),
            });
        }
        picks.retain(|p| !p.choices.is_empty());
        picks
    }
}
