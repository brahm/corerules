//! Testing for and rolling wild talents against the installation
//! `CORERULES_DIR` points at; without it the test passes without checking.

use cr_store::Installation;
use cr_store::create::Dice;

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
