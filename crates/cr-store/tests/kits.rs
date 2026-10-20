//! What a kit does on the Name page of New Character, with
//! `CORERULES_DIR` (an Expansion installation).

use std::path::Path;

use cr_store::Installation;
use cr_store::create::{Draft, PickKind, ROGUE_SKILLS};
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
