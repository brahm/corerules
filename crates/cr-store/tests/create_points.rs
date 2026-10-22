//! What the race decides in New Character (Skills & Powers racial points,
//! a point to any ability, the random name), with `CORERULES_DIR` (an
//! Expansion installation). The original's race page showed 5 Points
//! Left for a new High elf: the package costs 40 of the elf's 45 points,
//! though its abilities' costs add up to 45.

use std::path::Path;

use cr_store::Installation;
use cr_store::create::Draft;
use cr_store::npc::Rand;
use cr_store::settings::SETTINGS;

#[test]
fn a_subrace_package_costs_its_price() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let mut draft = Draft::new(&SETTINGS[0]);
    draft.skills_and_powers = true;
    draft.race = "Elf".into();
    draft.subrace = "High elf".into();
    draft.racial_abilities = rules.racial_package("Elf", "High elf");
    assert_eq!(rules.racial_points_left(&draft), 5);
    draft.racial_abilities.push("Elf, Less sleep (5)".into());
    assert_eq!(rules.racial_points_left(&draft), 0);
}

#[test]
fn a_race_may_add_one_point_to_the_ability_chosen() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let mut with_point = 0;
    for race in rules.all_races(&SETTINGS[0], false) {
        let any_point = rules.race(&race).unwrap().any_point;
        with_point += usize::from(any_point);
        let mut draft = Draft::new(&SETTINGS[0]);
        draft.race = race;
        let before = rules.draft_scores(&draft);
        for a in 0..6 {
            draft.raised = Some(a);
            let after = rules.draft_scores(&draft);
            for b in 0..6 {
                let point = u32::from(any_point && a == b);
                assert_eq!(after[b], before[b] + point);
            }
        }
    }
    assert!(with_point > 0);
}

#[test]
fn a_new_character_gets_a_name_of_its_subrace() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.creation_rules().unwrap();
    let npc = install.npc_rules().unwrap();
    let mut names = std::collections::HashSet::new();
    for race in rules.all_races(&SETTINGS[0], false) {
        for subrace in rules.subraces(&SETTINGS[0], &race) {
            for male in [true, false] {
                // A half-orc's name is fixed by sex; it draws nothing.
                let name = npc.random_name(&mut Rand::seeded(628), &subrace, male);
                assert!(!name.trim().is_empty(), "{subrace}");
                let again = npc.random_name(&mut Rand::seeded(628), &subrace, male);
                assert_eq!(name, again);
                names.insert(name);
            }
        }
    }
    assert!(names.len() > 1);
}
