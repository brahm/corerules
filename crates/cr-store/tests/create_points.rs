//! Skills & Powers racial points in New Character, with `CORERULES_DIR`
//! (an Expansion installation). The original's race page showed 5 Points
//! Left for a new High elf: the package costs 40 of the elf's 45 points,
//! though its abilities' costs add up to 45.

use std::path::Path;

use cr_store::Installation;
use cr_store::create::Draft;
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
