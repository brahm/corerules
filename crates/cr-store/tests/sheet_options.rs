//! Saving the Character Sheet Options on a copy of an installation.
//!
//! With `CORERULES_DIR` (an Expansion installation) its `Database` is
//! linked and its `Chars` copied to a scratch folder, and the first
//! character's options are saved there and read back. Without it, or on
//! Core Rules 2.0, the tests pass without checking.

use std::fs;
use std::path::{Path, PathBuf};

use cr_store::Installation;
use cr_store::sheet::{Border, SheetChoice, ids};

struct Scratch {
    root: PathBuf,
    install: Installation,
    file: String,
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
    let original = Installation::open(&dir).ok()?;
    let root = std::env::temp_dir().join(format!("cr-sheet-options-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    std::os::unix::fs::symlink(original.path("Database")?, root.join("Database")).unwrap();
    if let Some(user) = original.path("UserDbas") {
        copy_dir(&user, &root.join("UserDbas"));
    }
    copy_dir(&original.path("Chars")?, &root.join("Chars"));
    let install = Installation::open(&root).unwrap();
    let file = install.characters().ok()?.entries.first()?.file.clone();
    let scratch = Scratch {
        root,
        install,
        file,
    };
    // Core Rules 2.0 has no options to save.
    let character = scratch.install.character(&scratch.file).unwrap();
    scratch.install.sheet(&character).unwrap().border?;
    Some(scratch)
}

#[test]
fn saved_options_come_back_in_order() {
    let Some(s) = scratch("order") else {
        return;
    };
    let index = fs::read(s.root.join("Chars/index.chr")).unwrap();
    let choice = SheetChoice {
        sections: vec![ids::SAVES, ids::PERSONAL, ids::PAGE_BREAK, ids::SAVES],
        border: Border::Thin,
    };
    s.install.save_sheet_choice(&s.file, &choice).unwrap();

    let character = s.install.character(&s.file).unwrap();
    let rules = s.install.sheet_rules().unwrap();
    assert_eq!(rules.sheet_choice(&character).unwrap(), choice);
    let sheet = rules.sheet(&character).unwrap();
    assert_eq!(sheet.border, Some(Border::Thin));
    let built: Vec<u32> = sheet.sections.iter().map(|s| s.id).collect();
    assert_eq!(built, choice.sections);
    assert_eq!(fs::read(s.root.join("Chars/index.chr")).unwrap(), index);
}

#[test]
fn every_border_and_an_empty_list_are_kept() {
    let Some(s) = scratch("border") else {
        return;
    };
    let rules = s.install.sheet_rules().unwrap();
    for border in Border::ALL {
        let choice = SheetChoice {
            sections: Vec::new(),
            border,
        };
        s.install.save_sheet_choice(&s.file, &choice).unwrap();
        let character = s.install.character(&s.file).unwrap();
        assert_eq!(rules.sheet_choice(&character).unwrap(), choice);
        assert!(rules.sheet(&character).unwrap().sections.is_empty());
    }
}

#[test]
fn stored_borders_map_like_the_original() {
    for border in Border::ALL {
        assert_eq!(Border::from_stored(border.stored()), border);
    }
    assert_eq!(Border::from_stored(7), Border::Normal);
}
