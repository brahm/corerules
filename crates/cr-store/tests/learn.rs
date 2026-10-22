//! Learning spells against the installation `CORERULES_DIR` points at;
//! without it the test passes without checking.

use cr_store::Installation;
use cr_store::learn::{Learner, Learning};

fn questions(learning: Learning) -> usize {
    match learning {
        Learning::Ask(questions) => questions.len(),
        Learning::Known => panic!("not known"),
    }
}

#[test]
fn a_first_level_mage_rolls_and_may_learn_anyway() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(dir).unwrap();
    let castable = install.advance_rules().unwrap().castable("Mage", 1, &[]);
    assert!(castable[1] && !castable[2]);
    let rules = install.learn_rules().unwrap();
    let learner = Learner {
        class: "Mage".into(),
        castable,
        reason: 18,
        knowledge: 18,
        ..Default::default()
    };
    assert_eq!(rules.level("Magic Missile", true), 1);
    assert_eq!(
        questions(rules.learn(&learner, "Magic Missile", true, 1)),
        0
    );
    // A roll above the chance asks whether to learn it anyway.
    assert_eq!(
        questions(rules.learn(&learner, "Magic Missile", true, 100)),
        1
    );
}
