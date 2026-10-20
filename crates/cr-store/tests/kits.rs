//! What a kit does on the Name page of New Character, with
//! `CORERULES_DIR` (an Expansion installation).

use std::path::Path;

use cr_store::Installation;
use cr_store::create::{Draft, ROGUE_SKILLS};
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
