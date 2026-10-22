//! Update Character's Name page against the original.
//!
//! `tests/fixtures/characters/update` holds Borin as the Expansion saved
//! him after Update Character: `name-page.dat` after Neutral Good, Female,
//! the event Exiled, a history and the portrait the Background… dialog
//! picked; `level-3.dat` after Level… took him to level 3; `kit.dat` after
//! that character's kit became Outlaw Fighter (CRE), which rolled the
//! social class again.

use std::fs;
use std::path::{Path, PathBuf};

use cr_formats::character::Character;
use cr_store::Installation;
use cr_store::advance::Advancement;
use cr_store::create::Dice;
use cr_store::edit::CharacterDetails;

const BORIN: &str = "90688519.dat";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/characters")
}

fn read(path: &Path) -> Character {
    Character::parse(&fs::read(path).unwrap()).unwrap()
}

struct Scratch {
    root: PathBuf,
    install: Installation,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// An installation whose Borin is `file` of the update fixtures (or the
/// fixture itself).
fn scratch(test: &str, borin: Option<&str>) -> Scratch {
    let root = std::env::temp_dir().join(format!("cr-update-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Database")).unwrap();
    fs::create_dir_all(root.join("Chars")).unwrap();
    for file in ["index.chr", BORIN, "90988519.dat"] {
        fs::copy(fixtures().join(file), root.join("Chars").join(file)).unwrap();
    }
    if let Some(file) = borin {
        fs::copy(
            fixtures().join("update").join(file),
            root.join("Chars").join(BORIN),
        )
        .unwrap();
    }
    let install = Installation::open(&root).unwrap();
    Scratch { root, install }
}

/// Saves the Name page of `theirs` over `before` and compares. The
/// original also refreshes the money totals the equipment page caches
/// (`unknown_0x1778`), which saving details alone leaves as they were.
fn check(s: &Scratch, theirs: &str) {
    let theirs = read(&fixtures().join("update").join(theirs));
    let before = s.install.character(BORIN).unwrap();
    let mut details = CharacterDetails::of(&before);
    let wanted = CharacterDetails::of(&theirs);
    details.alignment = wanted.alignment;
    details.male = wanted.male;
    details.kit = wanted.kit;
    details.social_class = wanted.social_class;
    details.background = wanted.background;
    details.history = wanted.history;
    details.portrait = wanted.portrait;
    s.install.save_character_details(BORIN, &details).unwrap();
    let mut ours = s.install.character(BORIN).unwrap();
    ours.data.unknown_0x1778 = theirs.data.unknown_0x1778;
    assert_eq!(ours, theirs);
}

#[test]
fn the_name_page_writes_what_the_original_writes() {
    check(&scratch("name", None), "name-page.dat");
}

#[test]
fn a_new_kit_writes_the_kit_and_its_social_class() {
    let s = scratch("kit", Some("level-3.dat"));
    check(&s, "kit.dat");
    let index = s.install.characters().unwrap();
    assert_eq!(index.entries[0].details[6], "Kit: Outlaw Fighter (CRE)");
}

#[test]
fn a_male_keeps_the_value_the_original_stored() {
    let s = scratch("male", None);
    let mut details = CharacterDetails::of(&s.install.character(BORIN).unwrap());
    details.history = "Forjado nas minas.".into();
    s.install.save_character_details(BORIN, &details).unwrap();
    assert_eq!(s.install.character(BORIN).unwrap().data.unknown_0x163c, 3);
    details.male = false;
    s.install.save_character_details(BORIN, &details).unwrap();
    details.male = true;
    s.install.save_character_details(BORIN, &details).unwrap();
    assert_eq!(s.install.character(BORIN).unwrap().data.unknown_0x163c, 1);
}

/// The Hit Points arrows, with `CORERULES_DIR` (an Expansion installation)
/// for the class and Constitution tables. In the original, +11 took
/// Borin's level 1 roll from 2 to 13 (4 to 15 hit points), and one click
/// down took the level 3 Borin from [13, 8, 9] (36) to [13, 8, 8].
#[test]
fn the_hit_point_arrows_change_the_rolls_as_the_original_does() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.advance_rules().unwrap();
    let rolls = |c: &Character| c.data.unknown_0xc60[0][..3].to_vec();

    let mut borin = read(&fixtures().join(BORIN));
    assert_eq!(rules.hit_points(&borin).unwrap(), 4);
    rules.adjust_hit_points(&mut borin, 11).unwrap();
    assert_eq!(rolls(&borin), [13, 0, 0]);
    assert_eq!(rules.hit_points(&borin).unwrap(), 15);

    let mut level3 = read(&fixtures().join("update/level-3.dat"));
    level3.data.unknown_0xc60[0][2] = 9;
    assert_eq!(rules.hit_points(&level3).unwrap(), 36);
    rules.adjust_hit_points(&mut level3, -1).unwrap();
    assert_eq!(rolls(&level3), [13, 8, 8]);
    assert_eq!(rules.hit_points(&level3).unwrap(), 35);
    // Down as far as it goes: every roll stops at 1.
    rules.adjust_hit_points(&mut level3, -100).unwrap();
    assert_eq!(rolls(&level3), [1, 1, 1]);
    // Up goes to the highest level's roll.
    rules.adjust_hit_points(&mut level3, 5).unwrap();
    assert_eq!(rolls(&level3), [1, 1, 6]);
}

/// The Levels dialog Update opens from its Name page: the level arrows put
/// Borin at level 3 with the experience the original's Level... gave him,
/// rolling the new levels' hit points, and the character points typed in
/// reach the save with them.
#[test]
fn the_levels_dialog_saves_the_level_and_the_points() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    if install.is_core20() {
        return;
    }
    let rules = install.advance_rules().unwrap();
    let borin = read(&fixtures().join(BORIN));
    let theirs = read(&fixtures().join("update/level-3.dat"));
    let mut advanced = borin.clone();
    rules
        .set_level(&mut advanced, 0, 3, 3, &mut Dice::seeded(628))
        .unwrap();
    advanced.data.unknown_0x1778[0] += 7;

    let mut saved = borin.clone();
    Advancement::between(&borin, &advanced).apply(&mut saved);
    assert_eq!(saved.data.unknown_0x1618, theirs.data.unknown_0x1618);
    let levels: Vec<u32> = rules
        .levels(&saved)
        .unwrap()
        .iter()
        .map(|c| c.level)
        .collect();
    assert_eq!(levels, [3]);
    assert_eq!(
        saved.data.unknown_0xc60[0][0],
        borin.data.unknown_0xc60[0][0]
    );
    assert!(
        saved.data.unknown_0xc60[0][1..3]
            .iter()
            .all(|&roll| roll > 0)
    );
    assert_eq!(
        saved.data.unknown_0x1778[0],
        borin.data.unknown_0x1778[0] + 7
    );
}
