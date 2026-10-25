//! New Character with the user's own races, and a new psionicist's
//! powers, with `CORERULES_DIR` (an Expansion installation) on a scratch
//! copy of its `UserDbas` and `Chars`.

mod common;

use std::fs;
use std::path::PathBuf;

use cr_store::Installation;
use cr_store::create::Draft;
use cr_store::settings::SETTINGS;
use cr_store::userdb::UserDb;
use cr_store::userdb::allowed::{Allowed, AllowedDb};
use cr_store::userdb::races::{Races, new_race};

struct Scratch {
    root: PathBuf,
    install: Installation,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn scratch(test: &str) -> Option<Scratch> {
    let original = Installation::open(PathBuf::from(std::env::var_os("CORERULES_DIR")?)).ok()?;
    original.path("UserDbas/AllowedU.dat")?;
    let root = std::env::temp_dir().join(format!("cr-user-races-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("UserDbas")).unwrap();
    common::symlink(original.path("Database")?, root.join("Database")).unwrap();
    for entry in fs::read_dir(original.path("UserDbas")?).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), root.join("UserDbas").join(entry.file_name())).unwrap();
        }
    }
    fs::create_dir_all(root.join("Chars")).unwrap();
    let install = Installation::open(&root).unwrap();
    Some(Scratch { root, install })
}

fn classes(names: &[&str]) -> [String; 5] {
    std::array::from_fn(|i| names.get(i).map_or_else(String::new, |n| n.to_string()))
}

#[test]
fn a_user_subrace_follows_its_parent_race_and_its_own_classes() {
    let Some(s) = scratch("subrace") else { return };
    let mut db = UserDb::<Races>::load(&s.install).unwrap();
    let mut race = new_race("");
    race.name = "Povo das Dunas".into();
    race.ability_limits = [[3, 18]; 6];
    db.add(race).unwrap();
    // Its name does not hold the race's.
    let mut sub = new_race("Povo das Dunas");
    sub.name = "Rubros".into();
    db.add(sub).unwrap();
    db.save(&s.install).unwrap();
    let mut allowed = AllowedDb::load(&s.install).unwrap();
    allowed.set_user(
        "Povo das Dunas",
        &[
            Allowed::new(classes(&["Fighter"]), 0),
            Allowed::new(classes(&["", "Thief"]), 0),
        ],
    );
    allowed.set_user("Rubros", &[Allowed::new(classes(&["", "Thief"]), 0)]);
    allowed.save(&s.install).unwrap();

    let rules = s.install.creation_rules().unwrap();
    let generic = &SETTINGS[0];
    assert_eq!(rules.subraces(generic, "Povo das Dunas"), ["Rubros"]);
    let scores = [15; 6];
    let race = rules.class_options(generic, "Povo das Dunas", "Povo das Dunas", &scores);
    assert!(race.contains(&vec!["Fighter".to_string()]), "{race:?}");
    let sub = rules.class_options(generic, "Povo das Dunas", "Rubros", &scores);
    assert_eq!(sub, [vec!["Thief".to_string()]]);
}

#[test]
fn a_new_psionicist_keeps_the_powers_chosen() {
    let Some(s) = scratch("psionicist") else {
        return;
    };
    let rules = s.install.creation_rules().unwrap();
    let skills = s.install.skill_rules().unwrap();
    let mut d = Draft::new(&SETTINGS[0]);
    d.scores = [10, 10, 16, 16, 17, 10];
    d.race = "Human".into();
    d.subrace = "Human".into();
    d.classes = vec!["Psionicist".into()];
    d.name = "Psion Teste".into();
    d.alignment = rules.alignments(&d.classes)[0].clone();
    d.social_class = "Upper class".into();
    d.age = 20;
    d.height = 70;
    d.weight = 160;
    d.hit_dice = vec![5];
    d.psionic_roll = 3;
    d.languages = rules.native_languages(&d);
    d.weapon_proficiencies = vec!["Dagger".into()];
    d.gold = 100;
    // The Psionics step's choices.
    let built = rules.build(&d).unwrap();
    let mut psionics = skills.skills_of(&built);
    psionics.primary[0] = "Telepathic".into();
    psionics.disciplines = vec!["Telepathic".into()];
    psionics.sciences = vec!["Domination".into()];
    psionics.devotions = vec!["Attraction".into()];
    psionics.attacks = vec!["Mind Thrust (MT)".into()];
    psionics.defenses = vec!["Mind Blank (MBk)".into()];
    assert_eq!(skills.problem(&built, &psionics), None);
    d.psionics = Some(psionics.clone());
    assert_eq!(rules.problem(&d), None);
    let file = s.install.create_character(&rules, &d).unwrap();
    let made = s.install.character(&file).unwrap();
    let kept = skills.skills_of(&made);
    assert_eq!(kept.disciplines, psionics.disciplines);
    assert_eq!(kept.sciences, psionics.sciences);
    assert_eq!(kept.attacks, psionics.attacks);
    assert_eq!(kept.defenses, psionics.defenses);
}
