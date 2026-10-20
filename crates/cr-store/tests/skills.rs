//! Update Character's Skills page (Rogue Skills, Psionics, Wild Talents and
//! Psionic Attacks and Defenses) against the original, with
//! `CORERULES_DIR` (an Expansion installation) for the classes and powers.
//!
//! `tests/fixtures/characters/update` holds what the Expansion saved:
//! `rogue-skills.dat`, after Tamsin (`thief.dat`, a 1st-level thief) put 2
//! points each into Climb walls and Pick pockets; `wild-talent.dat`, after
//! she then took Catfall as a wild talent, which rolled her PSPs;
//! `psionics.dat`, after Oren (`psionicist.dat`, a 1st-level psionicist)
//! took Telepathic as his discipline and primary discipline, Domination,
//! Attraction, Mind Thrust and Mind Blank.

use std::fs;
use std::path::{Path, PathBuf};

use cr_formats::character::Character;
use cr_store::Installation;
use cr_store::create::Dice;
use cr_store::skills::SkillRules;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/characters/update")
}

fn read(name: &str) -> Character {
    Character::parse(&fs::read(fixtures().join(name)).unwrap()).unwrap()
}

fn rules() -> Option<SkillRules> {
    let install = Installation::open(Path::new(&std::env::var_os("CORERULES_DIR")?)).unwrap();
    if install.is_core20() {
        return None;
    }
    Some(install.skill_rules().unwrap())
}

fn rogue_points(rules: &SkillRules, character: &Character, points: &[(&str, u32)]) -> Vec<u32> {
    let mut rogue = rules.skills_of(character).rogue;
    for (skill, amount) in points {
        let at = rules
            .rogue_skills(character)
            .iter()
            .position(|s| s.label.starts_with(skill))
            .unwrap();
        rogue[at] = *amount;
    }
    rogue
}

#[test]
fn rogue_points_write_what_the_original_writes() {
    let Some(rules) = rules() else { return };
    let mut ours = read("thief.dat");
    assert_eq!(rules.rogue_level(&ours), Some((0, 1)));
    let mut new = rules.skills_of(&ours);
    assert_eq!(rules.rogue_points_left(&ours, &new), 60);
    // A 1st-level thief may put half of her 60 points into one skill.
    assert_eq!(rules.rogue_limit(&ours, &new), Some(30));
    new.rogue = rogue_points(&rules, &ours, &[("Climb walls", 2), ("Pick pockets", 2)]);
    assert_eq!(rules.rogue_points_left(&ours, &new), 56);
    assert!(rules.rogue_warnings(&ours, &new).is_empty());
    rules.apply(&mut ours, &new, &mut Dice::seeded(1)).unwrap();
    assert_eq!(ours, read("rogue-skills.dat"));

    // Over the limit for one skill or past 95% only warns; more points
    // than there are is refused.
    let mut more = rules.skills_of(&ours);
    more.rogue = rogue_points(&rules, &ours, &[("Climb walls", 31)]);
    assert_eq!(rules.rogue_warnings(&ours, &more).len(), 2);
    assert!(rules.problem(&ours, &more).is_none());
    more.rogue = rogue_points(&rules, &ours, &[("Climb walls", 40), ("Hide", 30)]);
    assert!(rules.problem(&ours, &more).is_some());
}

#[test]
fn a_wild_talent_rolls_the_psps_the_original_rolls() {
    let Some(rules) = rules() else { return };
    let base = read("rogue-skills.dat");
    assert_eq!(rules.psionic_level(&base), None);
    assert!(rules.wild_talent_choices().iter().any(|p| p == "Catfall"));
    let mut new = rules.skills_of(&base);
    // Catfall is a devotion.
    new.sciences.push("Catfall".into());
    assert!(rules.problem(&base, &new).is_some());
    new.sciences.clear();
    new.devotions.push("Catfall".into());
    let saved = read("wild-talent.dat");
    // The PSPs are 10 + 1d4 + the bonuses + Catfall's cost, so one of the
    // four rolls gives the original's.
    let mut rolls = Vec::new();
    for seed in 0..64 {
        let mut ours = base.clone();
        rules
            .apply(&mut ours, &new, &mut Dice::seeded(seed))
            .unwrap();
        rolls.push(ours.data.unknown_0x1430[0]);
        if ours == saved {
            // Dropping it gives its cost back but keeps the roll.
            let mut less = rules.skills_of(&ours);
            less.devotions.clear();
            let psp = rules.power("Catfall").unwrap().psp;
            rules
                .apply(&mut ours, &less, &mut Dice::seeded(seed))
                .unwrap();
            assert_eq!(ours.data.unknown_0x1430[0], 19 - psp);
            return;
        }
    }
    panic!("no roll gives the original's PSPs: {rolls:?}");
}

#[test]
fn psionics_write_what_the_original_writes() {
    let Some(rules) = rules() else { return };
    let mut ours = read("psionicist.dat");
    assert_eq!(rules.psionic_level(&ours), Some(1));
    assert_eq!(rules.allowed(&ours), (1, 1, 3));
    let mut new = rules.skills_of(&ours);
    new.disciplines = vec!["Telepathic".into()];
    // Powers need a primary discipline.
    new.sciences = vec!["Domination".into()];
    assert!(rules.problem(&ours, &new).is_some());
    new.primary[0] = "Telepathic".into();
    assert!(rules.problem(&ours, &new).is_none());
    assert!(
        rules
            .power_choices(&ours, &new.disciplines, false)
            .iter()
            .any(|p| p == "Domination")
    );
    new.devotions = vec!["Attraction".into()];
    new.attacks = vec!["Mind Thrust (MT)".into()];
    new.defenses = vec!["Mind Blank (MBk)".into()];
    assert_eq!(rules.attack_names().len(), 5);
    assert!(rules.attack_names().contains(&new.attacks[0]));
    assert!(rules.defense_names().contains(&new.defenses[0]));
    rules.apply(&mut ours, &new, &mut Dice::seeded(1)).unwrap();
    assert_eq!(ours, read("psionics.dat"));

    // A second discipline or science is more than a 1st-level psionicist
    // may have.
    let mut more = rules.skills_of(&ours);
    more.disciplines.push("Clairsentient".into());
    assert!(rules.problem(&ours, &more).is_some());
    let mut more = rules.skills_of(&ours);
    more.sciences.push("Ejection".into());
    assert!(rules.problem(&ours, &more).is_some());
}

#[test]
fn the_kit_gives_rogue_points_that_are_not_spent() {
    let Some(rules) = rules() else { return };
    let install =
        Installation::open(Path::new(&std::env::var_os("CORERULES_DIR").unwrap())).unwrap();
    let thief = read("thief.dat");
    let usual = rules.rogue_points_left(&thief, &rules.skills_of(&thief));
    let (mut own_points, mut bonuses, mut chosen) = (0, 0, 0);
    for kit in install.creation_rules().unwrap().all_kits() {
        let mut character = thief.clone();
        character.data.kit = kit;
        let mut new = rules.skills_of(&character);
        let left = rules.rogue_points_left(&character, &new);
        if left != usual {
            own_points += 1;
        }
        // The 1st-level bonus goes to one skill of the kit's list, which
        // keeps it, and costs no points.
        let picks = rules.rogue_bonus_picks(&character);
        assert!(picks.iter().all(|p| p.level == 1 && !p.names.is_empty()));
        if let Some(pick) = picks.first() {
            bonuses += 1;
            let skills = rules.rogue_skills(&character);
            let at = skills.iter().position(|s| s.name == pick.names[0]).unwrap();
            let before = new.rogue[at];
            rules.set_rogue_bonus(&character, &mut new, 1, Some(&pick.names[0]));
            assert_eq!(new.rogue[at] as i32, before as i32 + pick.amount);
            assert_eq!(rules.rogue_points_left(&character, &new), left);
            assert!(rules.rogue_minimums(&character, &new)[at] as i32 >= pick.amount);
            rules.set_rogue_bonus(&character, &mut new, 1, None);
            assert_eq!(new.rogue[at], before);
            rules.set_rogue_bonus(&character, &mut new, 1, Some(&pick.names[0]));
        }
        if let Some(pick) = rules.chosen_pick(&character) {
            chosen += 1;
            rules.set_chosen(&character, &mut new, &pick.names);
            assert_eq!(new.kit_chosen.len(), pick.count.min(pick.names.len()));
        }
        let mut saved = character.clone();
        rules.apply(&mut saved, &new, &mut Dice::seeded(1)).unwrap();
        if saved.data.unknown_0x15e4.is_some() {
            // Once given, the bonus and the chosen skills are not asked for
            // again.
            assert_eq!(rules.skills_of(&saved).kit_bonuses, new.kit_bonuses);
            assert!(rules.rogue_bonus_picks(&saved).is_empty());
            assert!(rules.chosen_pick(&saved).is_none());
        }
    }
    assert!(own_points > 0 && bonuses > 0 && chosen > 0);
}
