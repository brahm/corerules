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
use cr_store::create::WeaponCheck;

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
    let (held, levels) = (rules.combat_of(&ours), rules.levels(&ours));
    assert_eq!(rules.weapon_price(&ours, &held, &levels, "Adze"), 2);
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

#[test]
fn update_applies_the_kit_and_class_weapon_rules_by_level() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let (creation, rules) = (
        install.creation_rules().unwrap(),
        install.combat_rules().unwrap(),
    );
    let fighter = |kit: &str, skills_and_powers: bool| {
        let mut draft = cr_store::create::Draft::new(&cr_store::settings::SETTINGS[0]);
        draft.scores = [16; 6];
        draft.race = "Human".into();
        draft.classes = vec!["Fighter".into()];
        draft.alignment = "Lawful Good".into();
        draft.name = "Test".into();
        draft.hit_dice = vec![1];
        draft.pack = false;
        draft.skills_and_powers = skills_and_powers;
        draft.kit = kit.into();
        creation.build(&draft).unwrap()
    };
    let weapons = creation.weapon_choices(&cr_store::create::Draft::new(
        &cr_store::settings::SETTINGS[0],
    ));

    // The Cavalier picks a sword from 3rd level and a horseman's weapon
    // from 5th, once.
    let mut picking = 0;
    for kit in creation.all_kits() {
        let mut character = fighter(&kit, false);
        character.data.weapon_proficiencies.clone_from(&weapons);
        let held = rules.combat_of(&character);
        let picks = |levels: &[u32], character: &Character| {
            rules.kit_picks(character, &rules.combat_of(character), levels)
        };
        if picks(&[20], &character).is_empty() {
            continue;
        }
        picking += 1;
        assert!(picks(&[1], &character).is_empty());
        assert_eq!(picks(&[3], &character).len(), 1);
        assert_eq!(picks(&[5], &character).len(), 2);
        let sword = picks(&[3], &character)[0].choices[0].clone();
        let mut new = held.clone();
        new.kit_picks[0] = Some(sword.clone());
        rules.apply(&mut character, &new).unwrap();
        assert_eq!(character.data.unknown_0x1e68.as_ref().unwrap()[4], sword);
        assert!(picks(&[3], &character).is_empty());
        // A pick kept is never changed.
        let mut again = rules.combat_of(&character);
        again.kit_picks[0] = Some(weapons[0].clone());
        rules.apply(&mut character, &again).unwrap();
        assert_eq!(character.data.unknown_0x1e68.as_ref().unwrap()[4], sword);
    }
    assert!(picking > 0);

    // The warnings ignored are kept in the file's flags.
    let mut character = fighter("None", false);
    let mut new = rules.combat_of(&character);
    new.ignored = vec![WeaponCheck::Sword, WeaponCheck::EveryOther];
    rules.apply(&mut character, &new).unwrap();
    let ignored = rules.combat_of(&character).ignored;
    assert!(ignored.contains(&WeaponCheck::Sword) && ignored.contains(&WeaponCheck::EveryOther));

    // The Core Rules ask before a weapon with no slot left.
    let levels = [1];
    let held = rules.combat_of(&character);
    let none_held = rules.weapon_purchase(&character, &held, &levels, &weapons[0]);
    let mut all = held.clone();
    all.weapons = weapons[1..].to_vec();
    assert!(rules.core_slots_left(&character, &all, &levels).unwrap() <= 0);
    let none_left = rules.weapon_purchase(&character, &all, &levels, &weapons[0]);
    assert_eq!(none_left.questions.len(), none_held.questions.len() + 1);

    // Skills & Powers offers a weapon for free when the points run short.
    let mut character = fighter("None", true);
    let held = rules.combat_of(&character);
    let bought = rules.weapon_purchase(&character, &held, &levels, &weapons[0]);
    assert!(bought.cost > 0);
    character.data.unknown_0x1778[0] = 0;
    let short = rules.weapon_purchase(&character, &held, &levels, &weapons[0]);
    assert!(bought.free.is_none() && short.free.is_some());
    let mut new = held.clone();
    rules.bought(&character, &mut new, &levels, &weapons[0], 0);
    new.weapons.push(weapons[0].clone());
    assert_eq!(rules.points_left(&character, &new), 0);
}

#[test]
fn weapon_groups_cover_their_weapons_and_cost_warriors_and_priests_points() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let (creation, rules) = (
        install.creation_rules().unwrap(),
        install.combat_rules().unwrap(),
    );
    let draft = |class: &str, skills_and_powers: bool| {
        let mut draft = cr_store::create::Draft::new(&cr_store::settings::SETTINGS[0]);
        draft.scores = [16; 6];
        draft.race = "Human".into();
        draft.classes = vec![class.into()];
        draft.alignment = "Lawful Good".into();
        draft.name = "Test".into();
        draft.hit_dice = vec![1];
        draft.pack = false;
        draft.skills_and_powers = skills_and_powers;
        draft
    };

    // Skills & Powers only, and warriors and priests only.
    let fighter = draft("Fighter", true);
    assert!(
        creation
            .group_choices(&draft("Fighter", false), true)
            .is_empty()
    );
    let covering = |draft: &cr_store::create::Draft, broad: bool| {
        creation
            .group_choices(draft, broad)
            .into_iter()
            .find(|g| !creation.group_weapons(broad, g).is_empty())
            .unwrap()
    };
    let (broad, tight) = (covering(&fighter, true), covering(&fighter, false));
    assert!(creation.group_offer(&fighter, true).is_ok());
    assert!(creation.group_offer(&draft("Cleric", true), false).is_ok());
    assert!(creation.group_offer(&draft("Thief", true), true).is_err());

    // A group costs points and covers its weapons, which a specialization
    // may then be in.
    let mut grouped = fighter.clone();
    grouped.broad_groups = vec![broad.clone()];
    grouped.tight_groups = vec![tight.clone()];
    let covered = creation.group_weapons(true, &broad);
    assert!(!covered.is_empty());
    assert!(
        covered
            .iter()
            .all(|w| creation.covered_weapons(&grouped).contains(w))
    );
    assert!(creation.points_left(&grouped) < creation.points_left(&fighter));
    assert!(!creation.group_choices(&grouped, true).contains(&broad));
    grouped.weapon_of_choice = Some(covered[0].clone());
    assert!(creation.combat_problem(&grouped).is_none());
    assert!(creation.group_removal(&grouped, true).is_some());
    grouped.weapon_of_choice = None;
    assert!(creation.group_removal(&grouped, true).is_none());

    // The file keeps them, and Update Character reads them back.
    let mut character = creation.build(&grouped).unwrap();
    assert_eq!(character.data.unknown_0xac[1], std::slice::from_ref(&broad));
    assert_eq!(character.data.unknown_0xac[2], std::slice::from_ref(&tight));
    let held = rules.combat_of(&character);
    assert_eq!(held.broad_groups, std::slice::from_ref(&broad));
    let levels = [1];
    assert!(covered.iter().all(|w| {
        rules
            .covered_weapons(&character, &held, &levels)
            .contains(w)
    }));

    // Update Character offers a group for free when the points run short,
    // and keeps it in the file.
    let next = covering(&grouped, true);
    character.data.unknown_0x1778[0] = 0;
    let purchase = rules
        .group_purchase(&character, &held, &levels, true)
        .unwrap();
    assert!(purchase.cost > 0 && purchase.free.is_some());
    let mut new = held.clone();
    rules.group_bought(&mut new, &next, 0);
    new.broad_groups.push(next.clone());
    assert_eq!(rules.points_left(&character, &new), 0);
    rules.apply(&mut character, &new).unwrap();
    assert_eq!(character.data.unknown_0xac[1], [broad, next]);

    // The kit's warning about the groups it bars is kept once ignored.
    let mut new = rules.combat_of(&character);
    new.ignored = vec![WeaponCheck::BroadGroup, WeaponCheck::TightGroup];
    rules.apply(&mut character, &new).unwrap();
    let ignored = rules.combat_of(&character).ignored;
    assert!(
        ignored.contains(&WeaponCheck::BroadGroup) && ignored.contains(&WeaponCheck::TightGroup)
    );
}
