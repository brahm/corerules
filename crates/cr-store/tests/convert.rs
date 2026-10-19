//! Convert to NPC and Convert to Character against the original.
//!
//! `tests/fixtures/characters` holds Borin and Lirael as the Expansion saved
//! them, and `to-npc` the `Chars` folder after its Convert to NPC on Lirael:
//! the original saved her as `49612429.dat` and deleted `90988519.dat`.

use std::fs;
use std::path::{Path, PathBuf};

use cr_formats::character::CharacterIndex;
use cr_store::Installation;

const LIRAEL: &str = "90988519.dat";
const CONVERTED: &str = "49612429.dat";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/characters")
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

/// An installation with only the fixture characters (and an empty
/// `Database`, which converting never reads).
fn scratch(test: &str) -> Scratch {
    let root = std::env::temp_dir().join(format!("cr-convert-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Database")).unwrap();
    fs::create_dir_all(root.join("Chars")).unwrap();
    for file in ["index.chr", "90688519.dat", LIRAEL] {
        fs::copy(fixtures().join(file), root.join("Chars").join(file)).unwrap();
    }
    let install = Installation::open(&root).unwrap();
    Scratch { root, install }
}

fn read_index(path: &Path) -> CharacterIndex {
    CharacterIndex::parse(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn convert_to_npc_writes_what_the_original_writes() {
    let s = scratch("to-npc");
    let chars = s.root.join("Chars");
    let new = s.install.convert_to_npc(LIRAEL).unwrap();
    assert_ne!(new, LIRAEL);
    assert!(!chars.join(LIRAEL).exists());
    let theirs = fixtures().join("to-npc");
    assert_eq!(
        fs::read(chars.join(&new)).unwrap(),
        fs::read(theirs.join(CONVERTED)).unwrap()
    );
    // The new name is random, as in the original; everything else matches.
    let mut ours = read_index(&chars.join("index.chr"));
    ours.entries[1].file = CONVERTED.into();
    assert_eq!(ours, read_index(&theirs.join("index.chr")));
    assert_eq!(ours.entries[1].unknown_0x24, 1);
    assert_eq!(ours.entries[1].flag_0x28, 1);
    assert_eq!(s.install.character(&new).unwrap().data.unknown_0x1da4, 1);
}

#[test]
fn convert_back_to_character_clears_the_marks() {
    let s = scratch("back");
    let new = s.install.convert_to_npc(LIRAEL).unwrap();
    s.install.convert_to_character(&new).unwrap();
    let chars = s.root.join("Chars");
    assert_eq!(
        fs::read(chars.join(&new)).unwrap(),
        fs::read(fixtures().join(LIRAEL)).unwrap()
    );
    let index = read_index(&chars.join("index.chr"));
    assert_eq!(index.entries[1].file, new);
    assert_eq!(index.entries[1].unknown_0x24, 0);
}

#[test]
fn unknown_files_are_refused() {
    let s = scratch("unknown");
    assert!(s.install.convert_to_npc("12345678.dat").is_err());
    let index = fs::read(s.root.join("Chars/index.chr")).unwrap();
    assert_eq!(index, fs::read(fixtures().join("index.chr")).unwrap());
}
