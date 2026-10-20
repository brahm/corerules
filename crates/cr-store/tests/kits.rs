//! What a kit does on the Name page of New Character, with
//! `CORERULES_DIR` (an Expansion installation).

use std::path::Path;

use cr_store::Installation;
use cr_store::create::{Draft, PickKind, ROGUE_SKILLS, WeaponCheck};
use cr_store::settings::SETTINGS;

fn thief() -> Draft {
    let mut draft = Draft::new(&SETTINGS[0]);
    draft.scores = [16; 6];
    draft.race = "Human".into();
    draft.classes = vec!["Thief".into()];
    draft.alignment = "True Neutral".into();
    draft.name = "Test".into();
    draft.hit_dice = vec![1];
    draft
}

#[test]
fn create_offers_the_kits_of_the_gender_and_alignment() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    for male in [true, false] {
        let mut draft = thief();
        draft.classes = vec!["Fighter".into()];
        draft.male = male;
        let offered = rules.kits_for(&draft);
        assert_eq!(offered.first().map(String::as_str), Some("None"));
        let all = rules.kits(&draft.race, &draft.subrace, &draft.classes);
        for kit in &offered {
            assert!(all.contains(kit));
            assert!(rules.kit_allows_gender(kit, male));
            assert!(rules.kit_allows_alignment(kit, &draft.alignment));
        }
        // Some kit is for women only.
        if male {
            assert!(all.iter().any(|k| !rules.kit_allows_gender(k, true)));
        }
    }
}

#[test]
fn leaving_the_name_page_applies_the_kit() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let mut adjusted = 0;
    for kit in rules.kits_for(&thief()) {
        let mut draft = thief();
        draft.kit = kit.clone();
        let character = rules.build(&draft).unwrap();
        let d = &character.data;
        let barred = rules.kit_barred_abilities(&kit);
        assert!(
            d.class_slots[0]
                .abilities
                .iter()
                .all(|a| !barred.contains(a))
        );
        for (name, values) in &d.rogue_skills.entries {
            let skill = ROGUE_SKILLS
                .iter()
                .position(|(s, ..)| name.contains(s))
                .unwrap();
            let adjustment = rules.kit_rogue_adjustment(&draft, skill);
            assert_eq!(
                values[0] as i32,
                draft.rogue_points[skill] as i32 + adjustment
            );
            if adjustment != 0 {
                adjusted += 1;
                assert_eq!(d.flag_0x1610, Some(1));
            }
        }
    }
    // Some thief kit adjusts a skill.
    assert!(adjusted > 0);
}

#[test]
fn the_proficiencies_page_follows_the_class_and_kit() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let known = rules.nonweapon_proficiencies();
    let (mut picked, mut whole, mut free, mut based) = (0, 0, 0, 0);
    for class in [
        "Fighter", "Paladin", "Ranger", "Thief", "Bard", "Cleric", "Druid", "Mage",
    ] {
        let mut base = thief();
        base.classes = vec![class.into()];
        for kit in rules.kits(&base.race, &base.subrace, &base.classes) {
            let mut draft = base.clone();
            draft.kit = kit;
            for pick in rules.nonweapon_picks(&draft) {
                assert!(pick.count > 0 && !pick.names.is_empty());
                if let PickKind::Bonus(_) = pick.kind {
                    assert!(pick.count < pick.names.len());
                    picked += 1;
                }
                let names = pick.names.iter().take(pick.count).cloned().collect();
                draft.set_picked(pick.kind, names);
            }
            let required = rules.required_nonweapon(&draft);
            assert!(required.iter().all(|n| known.contains(n)));
            let bonus = rules.bonus_nonweapon(&draft);
            whole += usize::from(bonus.len() > draft.picked(PickKind::Bonus(0)).len());
            draft.nonweapon_proficiencies = required.clone();
            for name in bonus.iter().filter(|n| known.contains(n)) {
                if !draft.nonweapon_proficiencies.contains(name) {
                    draft.nonweapon_proficiencies.push(name.clone());
                }
            }
            draft.kit_nonweapon_base = Some(true);
            if let Some(slots) = rules.kit_nonweapon_base(&draft) {
                based += 1;
                assert!(rules.nonweapon_slots_for(&draft) >= slots);
            }
            let character = rules.build(&draft).unwrap();
            let d = &character.data;
            assert_eq!(d.unknown_0x1930.as_ref().unwrap()[1], bonus);
            assert_eq!(d.unknown_0xac[5], draft.picked(PickKind::PlusOne));
            let chosen = draft.picked(PickKind::Free);
            assert_eq!(
                d.unknown_0x1e20,
                chosen.first().cloned().unwrap_or_default()
            );
            free += chosen.len();
            for (name, cost) in d.nonweapon_proficiencies.iter().zip(&d.nonweapon_costs) {
                if bonus.contains(name) {
                    assert_eq!(*cost, 0);
                }
            }
            let expected = if rules.kit_nonweapon_base(&draft).is_some() {
                1
            } else {
                u32::MAX
            };
            assert_eq!(d.unknown_0x1df0.unwrap().0, expected);
        }
    }
    // Kits with bonus lists to pick from and lists taken whole, a free
    // proficiency to pick, and their own starting slots.
    assert!(picked > 0 && whole > 0 && free > 0 && based > 0);
}

#[test]
fn nonweapon_costs_follow_the_groups_the_classes_and_kit_open() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let known = rules.nonweapon_proficiencies();
    let costs = |draft: &Draft| -> Vec<u32> {
        known
            .iter()
            .map(|n| rules.nonweapon_cost(draft, n))
            .collect()
    };
    // A proficiency of no group of the class costs a slot more, 2 points
    // with Skills & Powers; a class opening more groups pays less.
    for skills_and_powers in [false, true] {
        let mut fighter = thief();
        fighter.classes = vec!["Fighter".into()];
        fighter.skills_and_powers = skills_and_powers;
        let mut ranger = fighter.clone();
        ranger.classes = vec!["Ranger".into()];
        let extra = if skills_and_powers { 2 } else { 1 };
        let (a, b) = (costs(&fighter), costs(&ranger));
        assert!(a.iter().zip(&b).any(|(a, b)| *a == b + extra));
        assert!(a.iter().zip(&b).all(|(a, b)| a >= b || a + extra == *b));
    }
    // Some kits open groups or name proficiencies of their own.
    let (mut opened, mut learn) = (0, 0);
    for class in ["Fighter", "Thief", "Cleric", "Mage", "Paladin"] {
        let mut base = thief();
        base.classes = vec![class.into()];
        let plain = costs(&base);
        for kit in rules.kits(&base.race, &base.subrace, &base.classes) {
            let mut draft = base.clone();
            draft.kit = kit;
            opened += usize::from(costs(&draft).iter().zip(&plain).any(|(k, p)| k < p));
            // The proficiency to learn by a level is asked about from it.
            draft.level = 20;
            if let Some(question) = rules.nonweapon_by_level(&draft) {
                learn += 1;
                draft.level = 1;
                assert!(rules.nonweapon_by_level(&draft).is_none());
                draft.level = 20;
                draft.by_level_declined = true;
                assert!(rules.nonweapon_by_level(&draft).is_none());
                draft.by_level_declined = false;
                let name = known
                    .iter()
                    .find(|n| question.text.contains(n.as_str()))
                    .unwrap();
                draft.nonweapon_proficiencies = vec![name.clone()];
                assert!(rules.nonweapon_by_level(&draft).is_none());
            }
        }
    }
    assert!(opened > 0 && learn > 0);

    // Taken again, a proficiency is one entry costing 1 more a rank; a
    // Core Rules language one is another entry.
    let mut draft = thief();
    let (plain, language) = (
        known
            .iter()
            .find(|n| !rules.language_nonweapon(n) && rules.nonweapon_cost(&draft, n) > 0)
            .unwrap()
            .clone(),
        known
            .iter()
            .find(|n| rules.language_nonweapon(n))
            .unwrap()
            .clone(),
    );
    let cost = rules.nonweapon_cost(&draft, &plain);
    draft.nonweapon_proficiencies = vec![plain.clone(), plain.clone(), plain.clone()];
    assert_eq!(
        rules.nonweapon_entries(&draft),
        [(plain.clone(), 3, cost + 2)]
    );
    draft.nonweapon_proficiencies = vec![language.clone(), language.clone()];
    assert_eq!(rules.nonweapon_entries(&draft).len(), 2);
    draft.skills_and_powers = true;
    assert_eq!(rules.nonweapon_entries(&draft).len(), 1);
}

#[test]
fn the_weapon_proficiencies_follow_the_kit() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let (mut warned, mut narrowed, mut dearer, mut lent, mut focused, mut listed) =
        (0, 0, 0, 0, 0, 0);
    let (mut freed, mut asked) = (0, 0);
    for class in [
        "Fighter", "Paladin", "Ranger", "Thief", "Bard", "Cleric", "Druid", "Mage",
    ] {
        let mut base = thief();
        base.classes = vec![class.into()];
        let plain = rules.weapon_choices(&base);
        for kit in rules.kits(&base.race, &base.subrace, &base.classes) {
            let mut draft = base.clone();
            draft.kit = kit;
            let choices = rules.weapon_choices(&draft);
            assert!(!choices.is_empty());
            narrowed += usize::from(choices.len() < plain.len());
            listed += usize::from(!rules.weapon_requirements(&draft).is_empty());
            // Take every weapon the page offers, ignoring each warning.
            for name in &choices {
                assert!(rules.weapon_cost_for(&draft, name) >= 1);
                dearer += usize::from(rules.weapon_cost_for(&draft, name) > 1);
                let taken = draft.weapon_proficiencies.clone();
                for warning in rules.weapon_warnings(&draft, &taken, name) {
                    assert!(warning.text.contains(&draft.kit));
                    assert!(!draft.ignored_weapon_checks.contains(&warning.check));
                    draft.ignored_weapon_checks.push(warning.check);
                    warned += 1;
                }
                assert!(rules.weapon_warnings(&draft, &taken, name).is_empty());
                draft.weapon_proficiencies.push(name.clone());
            }
            if let Some((least, most)) = rules.nonweapon_lending(&draft) {
                assert!(least <= most);
                let slots = rules.nonweapon_slots_for(&draft) + rules.borrowed_nonweapon(&draft);
                draft.lent_nonweapon = most + 1;
                assert_eq!(rules.borrowed_nonweapon(&draft), most);
                assert_eq!(rules.nonweapon_slots_for(&draft) + most, slots);
                lent += 1;
            }
            // Specialize in the first weapon, or take the free one.
            assert!(rules.core_specializes(&draft));
            if let Some(free) = rules.free_specialization_choices(&draft).first() {
                draft.free_specialization = Some(free.clone());
                assert_eq!(rules.core_specialization_cost(&draft, free), 0);
                freed += 1;
            } else if let Some(first) = draft.weapon_proficiencies.first().cloned() {
                let questions = rules.specialization_questions(&draft, &first);
                if class == "Fighter" {
                    // Only a kit barring melee specialization asks.
                    assert!(questions.len() <= 1);
                } else {
                    // Unless the kit lets them specialize.
                    asked += usize::from(!questions.is_empty());
                }
                assert!((1..=2).contains(&rules.core_specialization_cost(&draft, &first)));
                draft.specializations.push(first.clone());
                assert!(!rules.specialization_questions(&draft, &first).is_empty());
            }
            for warning in rules.leaving_warnings(&draft) {
                draft.ignored_weapon_checks.push(warning.check);
                warned += 1;
            }
            assert!(rules.leaving_warnings(&draft).is_empty());
            if let Some((_, weapons)) = rules.focus_choices(&draft) {
                assert!(
                    weapons
                        .iter()
                        .all(|w| draft.weapon_proficiencies.contains(w))
                );
                draft.weapon_focus = weapons.first().cloned();
                focused += usize::from(draft.weapon_focus.is_some());
            }
            let specializations = rules.all_specializations(&draft);
            assert_eq!(
                rules.weapon_spent(&draft),
                draft
                    .weapon_proficiencies
                    .iter()
                    .map(|w| rules.weapon_cost_for(&draft, w))
                    .chain(
                        specializations
                            .iter()
                            .map(|w| rules.core_specialization_cost(&draft, w))
                    )
                    .sum::<u32>()
            );
            let character = rules.build(&draft).unwrap();
            let d = &character.data;
            let ignored = |check| u8::from(draft.ignored_weapon_checks.contains(&check));
            assert_eq!(d.flag_0x1dde, Some(ignored(WeaponCheck::FirstFromList)));
            let flags = d.flags_0x1ddf.unwrap();
            assert_eq!(flags[0], ignored(WeaponCheck::OddMissile));
            assert_eq!(flags[3], ignored(WeaponCheck::Barred));
            assert_eq!(
                d.flags_0x1de8,
                Some([
                    ignored(WeaponCheck::Ranged),
                    ignored(WeaponCheck::Recommended)
                ])
            );
            assert_eq!(
                d.unknown_0x1e68.as_ref().unwrap()[6],
                draft.weapon_focus.clone().unwrap_or_default()
            );
            if rules.nonweapon_lending(&draft).is_some() {
                assert_eq!(d.unknown_0x1df8, Some(rules.borrowed_nonweapon(&draft)));
            }
            let mut sorted = specializations.clone();
            sorted.sort();
            let mut written = d.unknown_0xac[3].clone();
            written.sort();
            assert_eq!(written, sorted);
            let free: Vec<String> = rules.free_specialization(&draft).into_iter().collect();
            assert_eq!(d.unknown_0x1dc8.as_ref(), Some(&free));
        }
    }
    // Kits that warn, offer fewer weapons or some at more slots, lend
    // nonweapon slots, have a weapon to focus in, list requirements and
    // give a free specialization.
    assert!(warned > 0 && narrowed > 0 && dearer > 0);
    assert!(lent > 0 && focused > 0 && listed > 0 && freed > 0 && asked > 0);
}

#[test]
fn buying_a_weapon_asks_what_the_original_asks() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let (mut sword_lists, mut unarmed_kits, mut melee_barred) = (0, 0, 0);
    for skills_and_powers in [false, true] {
        for class in [
            "Fighter", "Paladin", "Ranger", "Thief", "Bard", "Cleric", "Druid", "Mage",
        ] {
            let mut base = thief();
            base.classes = vec![class.into()];
            base.skills_and_powers = skills_and_powers;
            let class_cost = rules.weapon_cost_for(&base, "");
            let unarmed_alone = rules.unarmed_cost(&base).is_some();
            assert_eq!(unarmed_alone, class == "Fighter");
            // Every weapon is offered; those out of class ask first (the
            // Core Rules) or cost more (Skills & Powers).
            let choices = rules.weapon_choices(&base);
            let asked = choices
                .iter()
                .filter(|w| !rules.weapon_questions(&base, &[], w).is_empty())
                .count();
            let dearer = choices
                .iter()
                .filter(|w| rules.weapon_cost_for(&base, w) > class_cost)
                .count();
            if class == "Mage" {
                assert!(if skills_and_powers { dearer } else { asked } > 0);
            }
            if !skills_and_powers {
                assert_eq!(dearer, 0);
            }
            for kit in rules.kits(&base.race, &base.subrace, &base.classes) {
                let mut draft = base.clone();
                draft.kit = kit;
                // A kit's sword list: a sword off it warns until one on it
                // is taken.
                for name in rules.weapon_choices(&draft) {
                    let warns = |taken: &[String]| {
                        rules
                            .weapon_warnings(&draft, taken, &name)
                            .iter()
                            .any(|w| w.check == WeaponCheck::Sword)
                    };
                    if warns(&[]) {
                        let cleared = rules
                            .weapon_choices(&draft)
                            .into_iter()
                            .any(|listed| !warns(std::slice::from_ref(&listed)));
                        assert!(cleared);
                        sword_lists += 1;
                    }
                }
                if rules.unarmed_cost(&draft).is_some() && !unarmed_alone {
                    unarmed_kits += 1;
                }
            }
        }
    }
    // A Skills & Powers kit may bar specializing in melee weapons at
    // creation, after asking.
    for kit in rules.all_kits() {
        let mut draft = thief();
        draft.classes = vec!["Fighter".into()];
        draft.skills_and_powers = true;
        draft.kit = kit;
        let asked = rules
            .weapon_choices(&draft)
            .iter()
            .filter(|w| !rules.specialization_questions(&draft, w).is_empty())
            .count();
        melee_barred += usize::from(asked > 0);
    }
    assert!(sword_lists > 0 && melee_barred > 0 && unarmed_kits > 0);

    // Unarmed combat and weapon specialization exclude each other.
    let mut fighter = thief();
    fighter.classes = vec!["Fighter".into()];
    fighter.pack = false;
    let weapon = rules.weapon_choices(&fighter)[0].clone();
    fighter.weapon_proficiencies = vec![weapon.clone()];
    assert_eq!(rules.unarmed_cost(&fighter), Some(2));
    assert!(rules.unarmed_conflict(&fighter).is_none());
    fighter.unarmed = vec!["Punching".into()];
    assert!(rules.specialization_conflict(&fighter).is_some());
    assert_eq!(rules.weapon_spent(&fighter), 1 + 2);
    assert_eq!(rules.problem(&fighter), None);
    let character = rules.build(&fighter).unwrap();
    assert_eq!(character.data.flags_0x1e54.unwrap()[..3], [1, 0, 0]);
    fighter.specializations = vec![weapon];
    assert!(rules.unarmed_conflict(&fighter).is_some());
    assert!(rules.problem(&fighter).is_some());
    // The kit's every-other list starts open, and is written with its
    // flag.
    assert!(fighter.every_other_listed);
    assert_eq!(character.data.flags_0x1ddc, Some([1, 0]));
}
