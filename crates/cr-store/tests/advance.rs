//! Advancement checked against characters the original programs levelled
//! up. The files come from the programs' own Level... dialog and are not in
//! the repository: `CORERULES_GOLDEN` points at a folder with `exp/` and
//! `w20/` (see the advancement spec), `CORERULES_DIR` at the matching
//! installation. Without them the tests pass without checking.

use cr_formats::character::Character;
use cr_store::Installation;
use cr_store::advance::AdvanceRules;
use cr_store::create::Dice;

fn setup() -> Option<(Installation, AdvanceRules, std::path::PathBuf, bool)> {
    let dir = std::env::var_os("CORERULES_DIR")?;
    let golden = std::path::PathBuf::from(std::env::var_os("CORERULES_GOLDEN")?);
    let install = Installation::open(dir).unwrap();
    let expansion = install.path("UserDbas/AllowedU.dat").is_some();
    let rules = install.advance_rules().unwrap();
    let golden = golden.join(if expansion { "exp" } else { "w20" });
    Some((install, rules, golden, expansion))
}

fn load(golden: &std::path::Path, file: &str) -> Character {
    Character::parse(&std::fs::read(golden.join(file)).unwrap()).unwrap()
}

fn levels(rules: &AdvanceRules, c: &Character) -> Vec<u32> {
    rules.levels(c).unwrap().iter().map(|l| l.level).collect()
}

#[test]
fn levels_and_hit_points_match_the_original() {
    let Some((_, rules, golden, expansion)) = setup() else {
        return;
    };
    if expansion {
        // Level 15 reached with limits applied, then dropped back to 4.
        let borin = load(&golden, "borin_after_L4_then15_then4.dat");
        assert_eq!(levels(&rules, &borin), [4]);
        assert_eq!(rules.hit_points(&borin).unwrap(), 29);
        let lirael = load(&golden, "lirael_after_all5000_split3001.dat");
        assert_eq!(levels(&rules, &lirael), [3, 3]);
        assert_eq!(rules.hit_points(&lirael).unwrap(), 12);
        let kip = load(&golden, "kip_cleric_after_13000.dat");
        assert_eq!(levels(&rules, &kip), [5]);
        assert_eq!(rules.hit_points(&kip).unwrap(), 36);
        // Core Rules clerics get no Wisdom bonus spells in the Expansion.
        let slots = rules.spell_slots(&kip, 0).unwrap().unwrap();
        assert_eq!(slots[..4], [3, 3, 1, 0]);
        assert_eq!(rules.spell_slots(&borin, 0).unwrap(), None);
    } else {
        let tordek = load(&golden, "tordek_after_3000000.dat");
        assert_eq!(levels(&rules, &tordek), [21]);
        assert_eq!(rules.hit_points(&tordek).unwrap(), 104);
    }
}

#[test]
fn adding_experience_matches_the_original() {
    let Some((_, rules, golden, expansion)) = setup() else {
        return;
    };
    let mut dice = Dice::seeded(7);
    if expansion {
        let mut lirael = load(&golden, "lirael_before.dat");
        // All Classes 5000, then Split Up Points 3001, prime bonus on.
        for g in 0..2 {
            rules
                .add_experience(&mut lirael, g, 5000, true, 3, &mut dice)
                .unwrap();
        }
        for g in 0..2 {
            rules
                .add_experience(&mut lirael, g, 3001 / 2, true, 3, &mut dice)
                .unwrap();
        }
        let after = load(&golden, "lirael_after_all5000_split3001.dat");
        assert_eq!(lirael.data.unknown_0x1618, after.data.unknown_0x1618);
        assert_eq!(lirael.data.unknown_0x1778[0], after.data.unknown_0x1778[0]);
        assert_eq!(levels(&rules, &lirael), [3, 3]);
        // Rolls above level 1 are already divided between the two classes.
        for g in 0..2 {
            for l in 1..3 {
                assert!((1..=5).contains(&lirael.data.unknown_0xc60[g][l]));
            }
        }

        let mut kip = load(&golden, "kip_cleric_before.dat");
        rules
            .add_experience(&mut kip, 0, 13000, false, 3, &mut dice)
            .unwrap();
        assert_eq!(levels(&rules, &kip), [5]);
        let rolls = &kip.data.unknown_0xc60[0];
        assert_eq!(
            rolls[0],
            load(&golden, "kip_cleric_before.dat").data.unknown_0xc60[0][0]
        );
        assert!(rolls[1..5].iter().all(|r| (1..=8).contains(r)));
        assert_eq!(rolls[5], 0);

        // Losing levels keeps the rolls.
        let mut borin = load(&golden, "borin_before.dat");
        rules
            .add_experience(&mut borin, 0, 10000, true, 3, &mut dice)
            .unwrap();
        assert_eq!(borin.data.unknown_0x1618[0], 11000);
        let rolls = borin.data.unknown_0xc60;
        rules
            .add_experience(&mut borin, 0, -10000, false, 3, &mut dice)
            .unwrap();
        assert_eq!(levels(&rules, &borin), [1]);
        assert_eq!(borin.data.unknown_0xc60, rolls);
    } else {
        let mut tordek = load(&golden, "tordek_before.dat");
        rules
            .add_experience(&mut tordek, 0, 3_000_000, true, 3, &mut dice)
            .unwrap();
        let after = load(&golden, "tordek_after_3000000.dat");
        assert_eq!(tordek.data.unknown_0x1618, after.data.unknown_0x1618);
        assert_eq!(levels(&rules, &tordek), [21]);
        // Flat 3 hit points a level after 9th.
        assert!(tordek.data.unknown_0xc60[0][9..21].iter().all(|&r| r == 3));
    }
}
