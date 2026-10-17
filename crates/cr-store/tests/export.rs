//! Export, Import and package editing against the originals' own results.
//!
//! With `CORERULES_DIR` pointing at an installation and `CORERULES_GOLDEN`
//! at the `export` golden folder, the program's folder (`t3/` with
//! `AllowedU.dat`, `ev/` without) holds the packages the original wrote
//! (`*_all` with a Campaign Mapper drawing, `*_nomap` without) and, in
//! `import/before` and `import/after`, the `UserDbas` and `Chars` folders
//! the packages were exported from and what importing `*_all` into them
//! left. Drawings are neither exported nor imported. The tests work on a
//! temporary copy of `before` next to a link to the installation's
//! `Database`. Without the variables they pass without checking.

use std::fs;
use std::path::{Path, PathBuf};

use cr_formats::character::CharacterIndex;
use cr_formats::export::ExportFile;
use cr_store::Installation;
use cr_store::export::{Category, Exportable, contents, remove};

struct Scratch {
    root: PathBuf,
    install: Installation,
    golden: PathBuf,
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
    if !golden.join("t3/import").is_dir() {
        return None;
    }
    let original = Installation::open(&dir).ok()?;
    let expansion = original.path("UserDbas/AllowedU.dat").is_some();
    let golden = golden.join(if expansion { "t3" } else { "ev" });
    let root = std::env::temp_dir().join(format!("cr-export-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    std::os::unix::fs::symlink(original.path("Database")?, root.join("Database")).unwrap();
    for folder in ["UserDbas", "Chars"] {
        copy_dir(
            &golden.join("import/before").join(folder),
            &root.join(folder),
        );
    }
    let install = Installation::open(&root).unwrap();
    Some(Scratch {
        root,
        install,
        golden,
    })
}

impl Scratch {
    fn package(&self, which: &str) -> (PathBuf, ExportFile) {
        let prefix = if self.golden.ends_with("t3") {
            "t3"
        } else {
            "ev"
        };
        let ext = if prefix == "t3" { "xpx" } else { "xpt" };
        let path = self.golden.join(format!("{prefix}_{which}.{ext}"));
        let file = ExportFile::parse(&fs::read(&path).unwrap()).unwrap();
        (path, file)
    }
}

#[test]
fn import_leaves_what_the_original_left() {
    let Some(s) = scratch("import") else {
        return;
    };
    let (_, file) = s.package("all");
    let report = s.install.import(&file, None).unwrap();
    assert_eq!(
        report.imported.len() + report.skipped.len(),
        contents(&file).len()
    );

    let after = s.golden.join("import/after");
    for entry in fs::read_dir(after.join("UserDbas")).unwrap() {
        let entry = entry.unwrap();
        let ours = fs::read(s.root.join("UserDbas").join(entry.file_name())).unwrap();
        assert!(
            ours == fs::read(entry.path()).unwrap(),
            "UserDbas/{} differs from the original's",
            entry.file_name().to_string_lossy()
        );
    }

    // The new characters, in order, under other file names.
    let read_index = |dir: &Path| {
        CharacterIndex::parse(&fs::read(dir.join("Chars/index.chr")).unwrap()).unwrap()
    };
    let theirs = read_index(&after);
    let mut ours = read_index(&s.root);
    assert_eq!(ours.entries.len(), theirs.entries.len());
    for (o, t) in ours.entries.iter_mut().zip(&theirs.entries) {
        let a = fs::read(s.root.join("Chars").join(&o.file)).unwrap();
        let b = fs::read(after.join("Chars").join(&t.file)).unwrap();
        assert!(a == b, "{} differs from the original's {}", o.file, t.file);
        o.file.clone_from(&t.file);
    }
    assert_eq!(ours, theirs);
    assert_eq!(report.characters.len(), file.characters.len());
}

/// The items of `file`, in its order, as this installation offers them.
fn items_of(install: &Installation, file: &ExportFile) -> Vec<Exportable> {
    let offered = install.exportable().unwrap();
    let find = |category: Category, name: &str| {
        offered
            .iter()
            .find(|e| e.category == category && e.name == name)
            .unwrap_or_else(|| panic!("{} {name} not offered", category.label()))
            .clone()
    };
    let mut items: Vec<Exportable> = file
        .characters
        .iter()
        .map(|c| find(Category::Character, &c.data.name))
        .collect();
    let mut records: Vec<_> = contents(file)
        .into_iter()
        .filter(|c| c.category != Category::Character)
        .collect();
    records.sort_by_key(|c| c.index);
    items.extend(records.iter().map(|c| find(c.category, &c.name)));
    items
}

#[test]
fn export_writes_what_the_original_wrote() {
    let Some(s) = scratch("export") else {
        return;
    };
    let (path, mut theirs) = s.package("nomap");
    let ours = s
        .install
        .export(&items_of(&s.install, &theirs))
        .unwrap()
        .file;
    // The original writes the character it holds in memory, whose sheet
    // options it never initialized; the rest must match.
    assert_eq!(ours.characters.len(), theirs.characters.len());
    for (o, t) in ours.characters.iter().zip(&mut theirs.characters) {
        t.data.sheet_options.clone_from(&o.data.sheet_options);
        assert_eq!(o, t);
    }
    assert!(
        ours.to_bytes().unwrap() == theirs.to_bytes().unwrap(),
        "the export differs from {}",
        path.display()
    );
}

/// Sorted (category, name) pairs and race/class combinations.
fn summary(file: &ExportFile) -> (Vec<(String, String)>, Vec<String>) {
    let mut items: Vec<_> = contents(file)
        .into_iter()
        .map(|c| (c.category.label().to_owned(), c.name))
        .collect();
    items.sort();
    let mut allowed: Vec<String> = file
        .allowed
        .iter()
        .flatten()
        .map(|o| format!("{o:?}"))
        .collect();
    allowed.sort();
    (items, allowed)
}

#[test]
fn removed_items_come_back_with_add() {
    let Some(s) = scratch("edit") else {
        return;
    };
    let (_, theirs) = s.package("nomap");
    let mut file = theirs.clone();
    let all = contents(&file);
    // Every race and class, a monster and a character.
    let mut taken: Vec<_> = all
        .iter()
        .filter(|c| matches!(c.category, Category::Race | Category::Class))
        .cloned()
        .collect();
    taken.extend(
        all.iter()
            .find(|c| c.category == Category::Monster)
            .cloned(),
    );
    taken.extend(
        all.iter()
            .find(|c| c.category == Category::Character)
            .cloned(),
    );
    assert!(!taken.is_empty());
    remove(&mut file, &taken);
    assert_eq!(contents(&file).len(), all.len() - taken.len());
    if let Some(allowed) = &file.allowed {
        assert!(allowed.is_empty(), "combinations left: {allowed:?}");
    }
    // Putting them back gives the same items and combinations.
    let offered = s.install.exportable().unwrap();
    let back: Vec<Exportable> = taken
        .iter()
        .map(|c| {
            offered
                .iter()
                .find(|e| e.category == c.category && e.name == c.name)
                .unwrap()
                .clone()
        })
        .collect();
    s.install.add_to(&mut file, &back).unwrap();
    assert_eq!(summary(&file), summary(&theirs));
    // Adding what the package holds replaces it in place.
    let again = file.clone();
    s.install.add_to(&mut file, &back).unwrap();
    assert!(file.to_bytes().unwrap() == again.to_bytes().unwrap());
}

#[test]
fn core20_installations_hand_over_everything() {
    let Some(s) = scratch("core20") else {
        return;
    };
    if !s.install.is_core20() {
        assert!(s.install.core20_package().is_err());
        return;
    }
    // Every item Export offers, the original's package among them.
    let (_, theirs) = s.package("nomap");
    let ours = s.install.core20_package().unwrap().file;
    assert_eq!(contents(&ours).len(), s.install.exportable().unwrap().len());
    let (ours, _) = summary(&ours);
    for item in summary(&theirs).0 {
        assert!(ours.contains(&item), "{item:?} missing");
    }
}
