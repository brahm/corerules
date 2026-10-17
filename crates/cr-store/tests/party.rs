//! Parties and deleting characters against the originals' own results.
//!
//! With `CORERULES_DIR` pointing at an installation and `CORERULES_GOLDEN`
//! at the `party` golden folder, `t3_*` (the Expansion) and `ev_*` (Core
//! Rules 2.0) hold the `Chars` folder before (`*_before`) and after each
//! step the original took, in order: `t3_s1_borin_heroes` put Borin in the
//! new party Heroes, and so on. The tests replay the steps on a temporary
//! copy of `*_before` next to links to the installation's `Database` and a
//! copy of its `UserDbas`. Without the variables they pass without checking.

use std::fs;
use std::path::{Path, PathBuf};

use cr_formats::character::CharacterIndex;
use cr_store::Installation;
use cr_store::party::party_of;

struct Scratch {
    root: PathBuf,
    install: Installation,
    golden: PathBuf,
    prefix: &'static str,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

fn scratch(test: &str) -> Option<Scratch> {
    let dir = PathBuf::from(std::env::var_os("CORERULES_DIR")?);
    let golden = PathBuf::from(std::env::var_os("CORERULES_GOLDEN")?);
    if !golden.join("t3_before").is_dir() {
        return None;
    }
    let original = Installation::open(&dir).ok()?;
    let prefix = if original.has_parties() { "t3" } else { "ev" };
    let root = std::env::temp_dir().join(format!("cr-party-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    std::os::unix::fs::symlink(original.path("Database")?, root.join("Database")).unwrap();
    copy_dir(&original.path("UserDbas")?, &root.join("UserDbas"));
    copy_dir(
        &golden.join(format!("{prefix}_before")),
        &root.join("Chars"),
    );
    let install = Installation::open(&root).unwrap();
    Some(Scratch {
        root,
        install,
        golden,
        prefix,
    })
}

fn read_index(chars: &Path) -> CharacterIndex {
    CharacterIndex::parse(&fs::read(chars.join("index.chr")).unwrap()).unwrap()
}

fn dat_files(chars: &Path) -> usize {
    fs::read_dir(chars)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".dat")
        })
        .count()
}

impl Scratch {
    fn chars(&self) -> PathBuf {
        self.root.join("Chars")
    }

    fn file_of(&self, name: &str) -> String {
        read_index(&self.chars())
            .entries
            .into_iter()
            .find(|e| e.name.starts_with(name))
            .unwrap_or_else(|| panic!("no {name}"))
            .file
    }

    /// The `Chars` folder matches the original's after `step`, but for
    /// file names and the flag the original never set.
    fn check(&self, step: &str) {
        let after = self.golden.join(format!("{}_{step}", self.prefix));
        let ours_bytes = fs::read(self.chars().join("index.chr")).unwrap();
        let theirs_bytes = fs::read(after.join("index.chr")).unwrap();
        let theirs = read_index(&after);
        let mut ours = read_index(&self.chars());
        if theirs.entries.is_empty() {
            assert!(ours_bytes == theirs_bytes, "{step}: the empty list differs");
        }
        assert_eq!(ours.entries.len(), theirs.entries.len(), "{step}");
        assert_eq!(dat_files(&self.chars()), dat_files(&after), "{step}");
        for (o, t) in ours.entries.iter_mut().zip(&theirs.entries) {
            let a = fs::read(self.chars().join(&o.file)).unwrap();
            let b = fs::read(after.join(&t.file)).unwrap();
            assert!(a == b, "{step}: {} differs from the original's", o.name);
            o.file.clone_from(&t.file);
            o.flag_0x28 = t.flag_0x28;
        }
        assert_eq!(ours, theirs, "{step}");
    }
}

#[test]
fn parties_and_deletes_leave_what_the_original_left() {
    let Some(s) = scratch("steps") else {
        return;
    };
    let install = &s.install;
    if s.prefix == "ev" {
        assert!(install.parties().unwrap().is_empty());
        assert!(install.set_party(&s.file_of("Mialee"), "Heroes").is_err());
        install.delete_character(&s.file_of("Mialee")).unwrap();
        s.check("s1_mialee_deleted");
        install.delete_character(&s.file_of("Tordek")).unwrap();
        s.check("s2_all_deleted");
        return;
    }
    assert!(install.party_name_problem("", &[]).unwrap().is_some());
    assert!(install.party_name_problem("~all", &[]).unwrap().is_some());
    assert!(
        install
            .party_name_problem(&"x".repeat(51), &[])
            .unwrap()
            .is_some()
    );
    assert!(install.party_name_problem("Heroes", &[]).unwrap().is_none());
    assert!(install.set_party(&s.file_of("Borin"), "~All").is_err());

    install.set_party(&s.file_of("Borin"), "Heroes").unwrap();
    s.check("s1_borin_heroes");
    assert_eq!(install.parties().unwrap(), ["Heroes"]);
    assert!(install.party_name_problem("heroes", &[]).unwrap().is_some());
    install.set_party(&s.file_of("Lirael"), "Heroes").unwrap();
    s.check("s2_lirael_heroes");
    install.set_party(&s.file_of("Lirael"), "").unwrap();
    s.check("s3_lirael_unassigned");
    install
        .set_party(&s.file_of("Lirael"), "HeroesDoomed")
        .unwrap();
    s.check("s4_lirael_heroesdoomed");
    assert_eq!(install.parties().unwrap(), ["Heroes", "HeroesDoomed"]);
    let deleted = install.delete_party("HeroesDoomed").unwrap();
    assert_eq!(deleted.len(), 1);
    s.check("s5_heroesdoomed_deleted");
    install.delete_character(&s.file_of("Borin")).unwrap();
    s.check("s6_borin_deleted");
}

#[test]
fn deletes_can_be_undone() {
    let Some(s) = scratch("undo") else {
        return;
    };
    let install = &s.install;
    let index = |s: &Scratch| fs::read(s.chars().join("index.chr")).unwrap();
    let before = index(&s);
    let files: Vec<String> = read_index(&s.chars())
        .entries
        .into_iter()
        .map(|e| e.file)
        .collect();
    let first = install.delete_character(&files[0]).unwrap();
    install.restore_characters(&[first]).unwrap();
    assert!(index(&s) == before);

    let party = if install.has_parties() {
        for file in &files {
            install.set_party(file, "Undo").unwrap();
        }
        "Undo"
    } else {
        ""
    };
    let before = index(&s);
    let deleted = if party.is_empty() {
        files
            .iter()
            .map(|f| install.delete_character(f).unwrap())
            .collect()
    } else {
        install.delete_party(party).unwrap()
    };
    assert_eq!(deleted.len(), files.len());
    assert!(read_index(&s.chars()).entries.is_empty());
    install.restore_characters(&deleted).unwrap();
    assert!(index(&s) == before);
    let index = read_index(&s.chars());
    assert!(index.entries.iter().all(|e| party_of(e) == party));
    assert_eq!(dat_files(&s.chars()), files.len());
}
