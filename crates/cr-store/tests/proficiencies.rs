//! Update Character's Nonweapon Proficiencies page, with `CORERULES_DIR`
//! (an Expansion installation).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cr_formats::character::Character;
use cr_store::Installation;
use cr_store::lists::CharacterLists;
use cr_store::proficiencies::ProficiencyRules;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/characters")
}

fn read(name: &str) -> Character {
    Character::parse(&fs::read(fixtures().join(name)).unwrap()).unwrap()
}

fn install() -> Option<Installation> {
    let install = Installation::open(Path::new(&std::env::var_os("CORERULES_DIR")?)).unwrap();
    (!install.is_core20()).then_some(install)
}

fn rules(install: &Installation) -> ProficiencyRules {
    install.proficiency_rules_with(
        Arc::new(install.creation_rules().unwrap()),
        Arc::new(install.advance_rules().unwrap()),
    )
}

#[test]
fn taking_and_removing_ranks_spends_and_refunds_what_they_take() {
    let Some(install) = install() else {
        return;
    };
    let rules = rules(&install);
    // Lirael is a Skills & Powers character.
    let lirael = read("90988519.dat");
    let held = rules.nonweapon_of(&lirael);
    assert_eq!(
        held.entries.len(),
        lirael.data.nonweapon_proficiencies.len()
    );
    let points = rules.left(&lirael, &held).unwrap();
    assert_eq!(points, i64::from(lirael.data.unknown_0x1778[0]));

    // A new proficiency is a new entry at its cost.
    let name = rules
        .names()
        .into_iter()
        .find(|n| {
            !held.entries.iter().any(|e| &e.name == n)
                && rules.cost(&lirael, &held, n) > 0
                && i64::from(rules.cost(&lirael, &held, n)) <= points
                && rules.barred_warning(&lirael, &held, n).is_none()
        })
        .unwrap();
    let mut new = held.clone();
    let purchase = rules.purchase(&lirael, &new, &name).unwrap();
    assert_eq!(purchase.rank_of, None);
    assert!(purchase.question.is_none());
    rules.take(&mut new, &name, &purchase);
    assert_eq!(new.entries.len(), held.entries.len() + 1);
    let index = new.entries.len() - 1;
    assert_eq!(rules.ranks(&lirael, &new, index), 1);
    assert_eq!(
        rules.left(&lirael, &new).unwrap(),
        points - i64::from(purchase.cost)
    );

    // Taken again, it is a rank more of the same entry, for 1 point.
    let again = rules.purchase(&lirael, &new, &name).unwrap();
    assert_eq!((again.rank_of, again.cost), (Some(index), 1));
    rules.take(&mut new, &name, &again);
    assert_eq!(new.entries.len(), held.entries.len() + 1);
    assert_eq!(rules.ranks(&lirael, &new, index), 2);

    // Saving writes what each took and spends the points.
    let mut lists = CharacterLists::of(&lirael);
    lists.nonweapon = Some(new.clone());
    let mut saved = lirael.clone();
    lists
        .apply(&mut saved, &install.part_table().unwrap())
        .unwrap();
    assert_eq!(saved.data.nonweapon_proficiencies.last(), Some(&name));
    assert_eq!(
        saved.data.nonweapon_costs.last(),
        Some(&(purchase.cost + 1))
    );
    assert_eq!(
        i64::from(saved.data.unknown_0x1778[0]),
        rules.left(&lirael, &new).unwrap()
    );

    // Removing takes a rank off, then the entry, refunding each.
    rules.remove(&lirael, &mut new, index);
    assert_eq!(rules.ranks(&lirael, &new, index), 1);
    rules.remove(&lirael, &mut new, index);
    assert_eq!(new, held);

    // More than the points left is refused.
    let mut broke = lirael.clone();
    broke.data.unknown_0x1778[0] = 0;
    assert!(rules.purchase(&broke, &held, &name).is_err());
}

#[test]
fn core_rules_slots_are_counted_and_asked_about() {
    let Some(install) = install() else {
        return;
    };
    let rules = rules(&install);
    // Borin is a Core Rules character.
    let borin = read("90688519.dat");
    let held = rules.nonweapon_of(&borin);
    let left = rules.left(&borin, &held).unwrap();
    let name = rules
        .names()
        .into_iter()
        .find(|n| {
            !held.entries.iter().any(|e| &e.name == n)
                && rules.cost(&borin, &held, n) > 0
                && rules.barred_warning(&borin, &held, n).is_none()
        })
        .unwrap();
    // Taking more than is left asks first, and the slots go below 0.
    let mut new = held.clone();
    let mut asked = false;
    for _ in 0..=left.max(0) + 1 {
        let purchase = rules.purchase(&borin, &new, &name).unwrap();
        asked |= purchase.question.is_some();
        rules.take(&mut new, &name, &purchase);
    }
    assert!(asked);
    assert!(rules.left(&borin, &new).unwrap() < 0);
}
