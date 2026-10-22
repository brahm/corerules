//! Testing for and rolling wild talents against the installation
//! `CORERULES_DIR` points at; without it the test passes without checking.

use cr_store::Installation;
use cr_store::create::Dice;
use cr_store::skills::wild::{WildDraft, WildMode};

#[test]
fn rolled_wild_talents_are_powers_and_the_chance_is_small() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(dir).unwrap();
    let rules = install.skill_rules().unwrap();
    let mut dice = Dice::seeded(11);
    let mut rolled = 0;
    for _ in 0..300 {
        let (names, notices) = rules.roll_wild_talents(&mut dice);
        assert!(!names.is_empty() || !notices.is_empty());
        for name in &names {
            assert!(rules.power(name).is_some(), "{name}");
        }
        rolled += names.len();
    }
    assert!(rolled >= 300);
    for entry in std::fs::read_dir(install.root().join("Chars")).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        if name.ends_with(".dat") {
            let character = install.character(&name).unwrap();
            let chance = rules.wild_chance(&character);
            assert!((1..=12).contains(&chance));
            let test = rules.test_wild_talent(&character, &mut dice);
            assert!(!test.notices.is_empty());
            // Seeds until a roll of 97 or more: a loss leaves every score
            // at 1 or more and changes one ability, a 100 all three.
            let a = &character.data.abilities;
            let before = [a[1], a[2], a[3]];
            let mut bad = 0;
            for seed in 0..5000 {
                let test = rules.test_wild_talent(&character, &mut Dice::seeded(seed));
                if test.notices.len() < 2 {
                    continue;
                }
                bad += 1;
                assert!(test.abilities.iter().flatten().all(|&v| v >= 1));
                let changed = before
                    .iter()
                    .zip(&test.abilities)
                    .filter(|(b, t)| b != t)
                    .count();
                assert!(changed <= 1 || test.abilities == [[3; 3]; 3]);
            }
            assert!(bad > 0);
        }
    }
}

#[test]
fn new_characters_roll_once_and_keep_what_they_rolled() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(dir).unwrap();
    let rules = install.skill_rules().unwrap();
    let taken = vec!["x".to_owned()];
    for mode in [WildMode::Create, WildMode::Enter, WildMode::Update] {
        assert_eq!(
            rules.roll_questions(&[], mode).ok().map(|q| q.len()),
            Some(0)
        );
    }
    assert!(rules.roll_questions(&taken, WildMode::Create).is_err());
    assert_eq!(
        rules
            .roll_questions(&taken, WildMode::Enter)
            .ok()
            .map(|q| q.len()),
        Some(0)
    );
    assert_eq!(
        rules
            .roll_questions(&taken, WildMode::Update)
            .ok()
            .map(|q| q.len()),
        Some(1)
    );

    let mut dice = Dice::seeded(5);
    let mut checked = 0;
    for entry in std::fs::read_dir(install.root().join("Chars")).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        if !name.ends_with(".dat") {
            continue;
        }
        let mut character = install.character(&name).unwrap();
        if character.data.unknown_0x1430.iter().sum::<u32>() > 0 {
            continue;
        }
        let (talents, _) = rules.roll_wild_talents(&mut dice);
        let a = &character.data.abilities;
        let wild = WildDraft {
            talents: talents.clone(),
            attacks: vec!["Mind Thrust (MT)".into()],
            defenses: Vec::new(),
            wild_talent: true,
            tested: true,
            abilities: Some([a[1], a[2], a[3]]),
        };
        rules.apply_wild(&mut character, &wild, &mut dice);
        let d = &character.data;
        assert_eq!((d.unknown_0x186c, d.unknown_0x1874), (1, 1));
        let [_, sciences, devotions] = &d.unknown_0x196c;
        assert_eq!(sciences.len() + devotions.len(), talents.len());
        assert!(devotions.iter().all(|n| rules.power(n).unwrap().devotion));
        assert!(sciences.iter().all(|n| !rules.power(n).unwrap().devotion));
        assert_eq!(d.unknown_0xac[6], wild.attacks);
        assert_eq!(d.unknown_0x1430[0] > 0, !talents.is_empty());
        checked += 1;
    }
    assert!(checked > 0);
}
