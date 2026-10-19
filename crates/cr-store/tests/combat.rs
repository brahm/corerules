//! Update Character's Weapon Proficiencies page, its dialogs, and the
//! traits and disadvantages of the Nonweapon Proficiencies page against the
//! original, with `CORERULES_DIR` (an Expansion installation) for the
//! classes and parts.
//!
//! `tests/fixtures/characters/update` holds what the Expansion saved from
//! Lirael (fighter/mage, 8 points): `specialization.dat`, after she took
//! the Adze proficiency (2 points), the Adze as weapon of choice (2) and
//! Punching (4); `shield-style-traits.dat`, after she took the Buckler
//! (2), "Composite armor, back-and-breast" (2), the One-handed Weapon
//! style (2), the Allure trait (3) and the Allergies disadvantage (gives
//! 3), leaving 2.

use std::fs;
use std::path::{Path, PathBuf};

use cr_formats::character::Character;
use cr_store::Installation;
use cr_store::combat::CombatRules;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/characters")
}

fn read(path: &Path) -> Character {
    Character::parse(&fs::read(path).unwrap()).unwrap()
}

fn rules() -> Option<CombatRules> {
    let install = Installation::open(Path::new(&std::env::var_os("CORERULES_DIR")?)).unwrap();
    if install.is_core20() {
        return None;
    }
    Some(install.combat_rules().unwrap())
}

fn lirael() -> Character {
    read(&fixtures().join("90988519.dat"))
}

#[test]
fn a_proficiency_weapon_of_choice_and_unarmed_style_write_what_the_original_writes() {
    let Some(rules) = rules() else { return };
    let mut ours = lirael();
    assert_eq!(rules.weapon_cost(&ours, "Adze"), 2);
    assert_eq!(rules.weapon_of_choice_cost(&ours), 2);
    assert_eq!(rules.expertise_cost(&ours), 2);
    assert_eq!(rules.specialization_cost(&ours), 4);
    assert_eq!(rules.mastery_cost(&ours), 8);
    let mut new = rules.combat_of(&ours);
    new.weapons.push("Adze".into());
    new.weapon_of_choice = Some("Adze".into());
    new.unarmed[0] = true;
    assert_eq!(rules.points_left(&ours, &new), 0);
    rules.apply(&mut ours, &new).unwrap();
    assert_eq!(ours, read(&fixtures().join("update/specialization.dat")));

    // The proficiency is the weapon of choice's, and nothing is left.
    let mut more = rules.combat_of(&ours);
    more.weapons.retain(|w| w != "Adze");
    assert!(rules.problem(&ours, &more).is_some());
    let mut more = rules.combat_of(&ours);
    more.expertise = Some("Adze".into());
    assert!(rules.problem(&ours, &more).is_some());

    // Dropping the weapon of choice and the style refunds them; dropping
    // the proficiency refunds nothing.
    let mut less = rules.combat_of(&ours);
    less.weapon_of_choice = None;
    less.unarmed[0] = false;
    less.weapons.retain(|w| w != "Adze");
    assert_eq!(rules.points_left(&ours, &less), 6);
}

#[test]
fn shields_armor_styles_traits_and_disadvantages_write_what_the_original_writes() {
    let Some(rules) = rules() else { return };
    let mut ours = lirael();
    let mut new = rules.combat_of(&ours);
    new.shields[0] = true;
    new.armor.push("Composite armor, back-and-breast".into());
    new.styles.push("One-handed Weapon".into());
    new.traits.push("Allure".into());
    new.disadvantages.push("Allergies".into());
    assert_eq!(rules.trait_cost(&ours, "Allure"), 3);
    assert_eq!(rules.disadvantage_points(&ours, "Allergies"), 3);
    assert_eq!(rules.points_left(&ours, &new), 2);
    rules.apply(&mut ours, &new).unwrap();
    assert_eq!(
        ours,
        read(&fixtures().join("update/shield-style-traits.dat"))
    );

    // Each of them is refunded when dropped.
    let less = cr_store::combat::CharacterCombat {
        shields: [false; 4],
        armor: Vec::new(),
        styles: Vec::new(),
        traits: Vec::new(),
        disadvantages: Vec::new(),
        ..rules.combat_of(&ours)
    };
    assert_eq!(rules.points_left(&ours, &less), 8);
}

#[test]
fn fighting_styles_cost_by_class_group_and_keep_their_codes() {
    let Some(rules) = rules() else { return };
    let ours = lirael();
    assert_eq!(rules.style_cost(&ours, "One-handed Weapon"), 2);
    assert_eq!(rules.style_cost(&ours, "Two Weapon"), 3);
    assert_eq!(rules.style_cost(&ours, "Two Weapon (+2)"), 5);
    assert_eq!(rules.style_cost(&ours, "One-handed Weapon (+2)"), 4);
    assert!(!rules.single_style(&ours));
    let mut new = rules.combat_of(&ours);
    new.styles = vec!["One-handed Weapon (+2)".into(), "Missile".into()];
    let mut saved = ours.clone();
    // Not enough points for both, so give her some.
    saved.data.unknown_0x1778[0] = 20;
    rules.apply(&mut saved, &new).unwrap();
    assert_eq!(saved.data.classes.unknown_0x50[..3], [2002, 1997, 0]);
    assert_eq!(saved.data.unknown_0x1778[0], 14);
    assert_eq!(rules.combat_of(&saved).styles, new.styles);
}
