//! Update Character's Race Abilities and Class Abilities pages against the
//! original, with `CORERULES_DIR` (an Expansion installation) for the
//! races, classes and parts.
//!
//! `tests/fixtures/characters/update` holds what the Expansion saved:
//! `race-page.dat`, Lirael after her subrace became Gray elf, her mage
//! took "Mage, Read magic (5)" and "Wizard, No potions, oils, or scrolls
//! (-5)", and Alchemy replaced Necromancy among her schools;
//! `sphere.dat`, Lirael after her mage took two restrictions and bought
//! the Animal sphere; `class-core.dat`, Borin (a Core Rules character)
//! after his fighter took "Fighter, Building (5)".

use std::fs;
use std::path::{Path, PathBuf};

use cr_formats::character::Character;
use cr_store::Installation;
use cr_store::abilities::AbilityRules;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/characters")
}

fn read(path: &Path) -> Character {
    Character::parse(&fs::read(path).unwrap()).unwrap()
}

fn rules() -> Option<AbilityRules> {
    let install = Installation::open(Path::new(&std::env::var_os("CORERULES_DIR")?)).unwrap();
    if install.is_core20() {
        return None;
    }
    Some(install.ability_rules().unwrap())
}

fn lirael() -> Character {
    read(&fixtures().join("90988519.dat"))
}

fn strings(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

#[test]
fn a_new_subrace_class_abilities_and_schools_write_what_the_original_writes() {
    let Some(rules) = rules() else { return };
    let mut ours = lirael();
    assert_eq!(rules.package("Elf", "High elf").len(), 6);
    assert!(rules.subraces(&ours).contains(&"Gray elf".to_string()));
    let mut new = rules.abilities_of(&ours);
    assert!(new.racial.is_empty());
    new.subrace = "Gray elf".into();
    new.classes[1].abilities = strings(&[
        "Wizard, No potions, oils, or scrolls (-5)",
        "Mage, Read magic (5)",
    ]);
    let schools = &mut new.classes[1].schools;
    schools.retain(|s| s != "Necromancy");
    schools.push("Alchemy".into());
    assert_eq!(rules.points_left(&ours, &new), 3);
    rules.apply(&mut ours, &new).unwrap();
    assert_eq!(ours, read(&fixtures().join("update/race-page.dat")));
}

#[test]
fn a_mage_buys_a_single_major_sphere_for_15_points() {
    let Some(rules) = rules() else { return };
    let mut ours = lirael();
    let mut new = rules.abilities_of(&ours);
    new.classes[1].abilities = strings(&[
        "Wizard, No potions, oils, or scrolls (-5)",
        "Wizard, No miscellaneous magical items (-5)",
    ]);
    new.classes[1].major_spheres = strings(&["Animal"]);
    assert_eq!(rules.points_left(&ours, &new), 3);
    rules.apply(&mut ours, &new).unwrap();
    assert_eq!(ours, read(&fixtures().join("update/sphere.dat")));

    // A second sphere is refused, and so is spending more than is left.
    let mut more = rules.abilities_of(&ours);
    more.classes[1].major_spheres.push("Astral".into());
    assert!(rules.problem(&ours, &more).is_some());
    let mut costly = rules.abilities_of(&ours);
    costly.classes[1]
        .abilities
        .push("Mage, Armored wizard (15)".into());
    assert_eq!(rules.points_left(&ours, &costly), -12);
    assert!(rules.apply(&mut ours.clone(), &costly).is_err());
}

#[test]
fn a_core_rules_character_still_pays_for_class_abilities() {
    let Some(rules) = rules() else { return };
    let mut borin = read(&fixtures().join("90688519.dat"));
    assert_eq!(rules.subraces(&borin), ["Hill dwarf"]);
    assert!(rules.racial_choices(&borin).is_empty());
    let mut new = rules.abilities_of(&borin);
    new.classes[0]
        .abilities
        .push("Fighter, Building (5)".into());
    rules.apply(&mut borin, &new).unwrap();
    assert_eq!(borin, read(&fixtures().join("update/class-core.dat")));
}

#[test]
fn package_abilities_stay_and_bought_ones_go_with_the_subrace() {
    let Some(rules) = rules() else { return };
    let mut ours = lirael();
    let mut new = rules.abilities_of(&ours);
    new.racial = strings(&["Elf, Less sleep (5)"]);
    assert_eq!(rules.points_left(&ours, &new), 3);
    rules.apply(&mut ours, &new).unwrap();
    assert!(
        ours.data
            .racial_abilities
            .contains(&"Elf, Less sleep (5)".to_string())
    );
    assert_eq!(rules.abilities_of(&ours).racial, new.racial);
    // Gray elf costs 45 of the 48 points, so the bought ability is refunded.
    let mut gray = rules.abilities_of(&ours);
    gray.subrace = "Gray elf".into();
    gray.racial.clear();
    assert_eq!(rules.points_left(&ours, &gray), 3);
}
