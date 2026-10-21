//! What a kit does on the Name page of New Character, with
//! `CORERULES_DIR` (an Expansion installation).

use std::path::Path;

use cr_store::Installation;
use cr_store::create::{Dice, Draft, PickKind, ROGUE_SKILLS, WeaponCheck};
use cr_store::lists::CharacterLists;
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
        // The kit's hit points go on the 1st-level roll.
        assert_eq!(
            d.unknown_0xc60[0][0] as i32,
            draft.hit_dice[0] as i32 + rules.kit_hit_points(&kit)
        );
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

#[test]
fn the_rogue_skills_page_follows_the_kit() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let (mut own_points, mut bonuses, mut restricted, mut chosen) = (0, 0, 0, 0);
    // Every kit, whatever its races.
    let base = thief();
    let usual = rules.rogue_points(&base);
    for kit in rules.all_kits() {
        let mut draft = base.clone();
        draft.kit = kit;
        // A kit's own points, of which a thief puts half in a skill.
        let (points, most) = rules.rogue_points(&draft);
        if (points, most) != usual {
            own_points += 1;
            assert_eq!(most, points / 2);
        }
        let skills = rules.rogue_skills(&draft);
        for (_, skill) in &skills {
            if let Some(warning) = rules.kit_rogue_restriction(&draft, *skill) {
                restricted += 1;
                assert!(!warning.is_empty());
            }
        }
        // Choosing every skill keeps as many as the kit allows.
        let mut most_chosen = 0;
        if let Some((count, _, names)) = rules.kit_chosen_skills(&draft) {
            chosen += 1;
            most_chosen = count.min(names.len());
            draft.rogue_chosen = names;
        }
        let check_chosen = |character: &cr_formats::character::Character| {
            if let Some(lists) = &character.data.unknown_0x15e4 {
                assert_eq!(lists[1].len(), most_chosen);
            }
        };
        let Some((amount, names)) = rules.kit_rogue_bonus(&draft) else {
            let character = rules.build(&draft).unwrap();
            check_chosen(&character);
            if let Some(lists) = &character.data.unknown_0x15e4 {
                assert!(lists[0].is_empty());
            }
            continue;
        };
        bonuses += 1;
        let name = names[0].clone();
        let skill = skills.iter().find(|s| s.0 == name).unwrap().1;
        let before = rules.rogue_skill_score(&draft, skill).0;
        draft.rogue_bonus = Some(name.clone());
        assert_eq!(rules.rogue_skill_score(&draft, skill).0, before + amount);
        let character = rules.build(&draft).unwrap();
        check_chosen(&character);
        let d = &character.data;
        let values = &d
            .rogue_skills
            .entries
            .iter()
            .find(|e| e.0 == name)
            .unwrap()
            .1;
        assert_eq!(
            values[0] as i32,
            draft.rogue_points[skill] as i32 + rules.kit_rogue_adjustment(&draft, skill) + amount
        );
        if let Some(lists) = &d.unknown_0x15e4 {
            assert_eq!(lists[0].len(), 1);
            let Some(cr_formats::Object::KitBonus(b)) = lists[0][0].as_deref() else {
                panic!("no kit bonus");
            };
            assert_eq!(
                (b.name.as_str(), b.unknown_0x8, b.unknown_0xc),
                (name.as_str(), 1, amount)
            );
        }
    }
    // Some kit gives its own points, a bonus, a list for the starting
    // points and skills to choose.
    assert!(
        own_points > 0 && bonuses > 0 && restricted > 0 && chosen > 0,
        "{own_points} {bonuses} {restricted} {chosen}"
    );
}

#[test]
fn the_kit_trims_the_schools_and_spheres() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let magic = install.kit_magic_rules().unwrap();
    let (mut barred, mut eliminated, mut spheres, mut free, mut never) = (0, 0, 0, 0, 0);
    for kit in rules.all_kits() {
        for class in ["Mage", "Cleric", "Druid"] {
            let mut base = thief();
            base.classes = vec![class.into()];
            base.kit.clone_from(&kit);

            // Creation removes the barred schools unless ignored, and
            // Update removes them too.
            if rules.kit_barred_schools_warning(&base).is_some() {
                barred += 1;
                let built = rules.build(&base).unwrap();
                assert!(magic.barred_schools_warning(&built, &kit).is_none());
                let mut draft = base.clone();
                draft.barred_schools_ignored = true;
                assert!(rules.kit_barred_schools_warning(&draft).is_none());
                let mut kept = rules.build(&draft).unwrap();
                assert!(magic.barred_schools_warning(&kept, &kit).is_some());
                let mut new = magic.magic_of(&kept);
                assert!(new.barred_ignored);
                new.barred_ignored = false;
                magic.apply(&mut kept, &new);
                assert!(magic.barred_schools_warning(&kept, &kit).is_none());
            }

            // The schools beyond five go, once.
            if let Some((count, schools)) = rules.kit_schools_to_eliminate(&base) {
                eliminated += 1;
                let mut draft = base.clone();
                draft.eliminated_schools = schools[..count].to_vec();
                let built = rules.build(&draft).unwrap();
                assert_eq!(built.data.class_slots[0].unknown_0x270.len(), 5);
                let new = magic.magic_of(&built);
                assert_eq!(new.eliminated.len(), count);
                assert!(magic.schools_to_eliminate(&built, &kit, &new).is_none());
                let offered = rules.draft_spells(&draft);
                assert!(offered.len() <= rules.draft_spells(&base).len());
            }

            let built = rules.build(&base).unwrap();
            let mut updated = built.clone();
            let mut new = magic.magic_of(&built);
            if let Some((count, schools)) = magic.schools_to_eliminate(&built, &kit, &new) {
                new.eliminated = schools[..count].to_vec();
            }
            // The spheres kept are at most the kit's numbers.
            for pick in magic.sphere_picks(&built, &kit) {
                spheres += 1;
                new.spheres
                    .push((pick.slot, pick.major, pick.names[..pick.most].to_vec()));
            }
            let choices = magic.free_sphere_choices(&built, &kit);
            if let Some(sphere) = choices.first() {
                free += 1;
                new.free_sphere.clone_from(sphere);
            }
            if magic.spell_use_warning(&kit, &new).is_some() {
                never += 1;
                new.spell_use_ignored = true;
                assert!(magic.spell_use_warning(&kit, &new).is_none());
            }
            magic.apply(&mut updated, &new);
            let after = magic.magic_of(&updated);
            assert!(magic.sphere_picks(&updated, &kit).is_empty());
            assert!(magic.schools_to_eliminate(&updated, &kit, &after).is_none());
            if !choices.is_empty() {
                assert_eq!(after.free_sphere, choices[0]);
                assert!(magic.free_sphere_choices(&updated, &kit).is_empty());
            }
            assert_eq!(after.spell_use_ignored, new.spell_use_ignored);
        }
    }
    // Some kit bars schools, limits a wizard to five, gives a free sphere
    // and never uses spells.
    assert!(
        barred > 0 && eliminated > 0 && free > 0 && never > 0,
        "{barred} {eliminated} {spheres} {free} {never}"
    );
}

#[test]
fn a_kit_may_scale_the_experience_each_level_takes() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let advance = install.advance_rules().unwrap();
    let base = rules.build(&thief()).unwrap();
    let plain = advance.levels(&base).unwrap()[0].next.unwrap();
    let mut scaled = 0;
    for kit in rules.all_kits() {
        let mut character = base.clone();
        character.data.kit = kit;
        let next = advance.levels(&character).unwrap()[0].next.unwrap();
        if next == plain {
            continue;
        }
        scaled += 1;
        // The 2nd level comes at the kit's threshold, not the class's.
        character.data.unknown_0x1618[0] = plain.min(next) - 1;
        assert_eq!(advance.levels(&character).unwrap()[0].level, 1);
        character.data.unknown_0x1618[0] = next;
        assert_eq!(advance.levels(&character).unwrap()[0].level, 2);
        character.data.unknown_0x1618[0] = next - 1;
        assert_eq!(advance.levels(&character).unwrap()[0].level, 1);
    }
    // Some kit changes the experience its levels take.
    assert!(scaled > 0);
}

#[test]
fn the_equipment_page_follows_the_kit() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let (mut texts, mut notices, mut funds) = (0, 0, 0);
    for kit in rules.all_kits() {
        let mut draft = thief();
        draft.kit.clone_from(&kit);
        if rules.kit_equipment_text(&kit).is_some() {
            texts += 1;
        }
        for notice in rules.kit_equipment_notices(&draft) {
            notices += 1;
            assert!(notice.notice && !notice.text.is_empty());
        }
        // Keeping no money is always allowed.
        assert!(rules.kit_retained_funds_warning(&draft, 0).is_none());
        if rules.kit_retained_funds_warning(&draft, u32::MAX).is_some() {
            funds += 1;
        }
    }
    assert!(
        texts > 0 && notices > 0 && funds > 0,
        "{texts} {notices} {funds}"
    );
}

#[test]
fn the_kit_sets_prices_money_and_free_items() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let mut dice = Dice::seeded(7);
    let item = rules
        .wares
        .iter()
        .find(|w| w.cost >= 100)
        .unwrap()
        .name
        .clone();
    let (mut priced, mut funded, mut equipped) = (0, 0, 0);
    for kit in rules.all_kits() {
        let mut draft = thief();
        draft.kit.clone_from(&kit);
        draft.pack = false;
        draft.purchases = vec![(item.clone(), 2)];
        draft.gold = 10_000;
        // The kit's percentage of the price, and the notice that says so.
        assert_eq!(rules.spent(&draft), rules.price(&kit, &item, 2));
        let percent = rules.kit_price_percent(&kit);
        if percent != 100 {
            priced += 1;
            assert_ne!(rules.price(&kit, &item, 2), rules.price("None", &item, 2));
            assert!(rules.kit_price_notice(&kit).is_some());
        } else {
            assert!(rules.kit_price_notice(&kit).is_none());
        }
        // A thief's own money is 2d6 x 10 gold pieces; some kits change it.
        if (0..20).any(|_| {
            let gold = rules.roll_gold(&draft, &mut dice);
            !(20..=120).contains(&gold) || !gold.is_multiple_of(10)
        }) {
            funded += 1;
        }
        // The free items are on the new character, and only those cost
        // nothing.
        let free = rules.kit_free_items(&kit);
        let character = rules.build(&draft).unwrap();
        let lists = CharacterLists::of(&character);
        let carried: Vec<&str> = lists
            .equipment
            .iter()
            .flatten()
            .map(|i| i.name.as_str())
            .collect();
        for name in &free {
            assert!(carried.contains(&name.as_str()), "{kit}: {name}");
        }
        if !free.is_empty() {
            equipped += 1;
        }
        let spent = rules.spent(&draft);
        assert_eq!(
            lists.coins[3] * 100 + lists.coins[1] * 10 + lists.coins[0],
            draft.gold * 100 - spent
        );
    }
    assert!(
        priced > 0 && funded > 0 && equipped > 0,
        "{priced} {funded} {equipped}"
    );
}
