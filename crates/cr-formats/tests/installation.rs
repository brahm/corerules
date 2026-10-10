//! Round trips against a real installation, which cannot ship with the
//! repository. Set `CORERULES_DIR` to the installation folder (the one that
//! holds `Database/`) to run them; without it they pass without checking.

use std::path::PathBuf;

use cr_formats::convert::ConversionMaps;
use cr_formats::database::ObjectList;

fn installation() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("CORERULES_DIR")?);
    assert!(
        dir.join("Database").is_dir(),
        "CORERULES_DIR has no Database folder"
    );
    Some(dir)
}

/// Reads `relative` (matched case-insensitively), checks that it parses to
/// the end and writes back identical bytes.
fn round_trip_list(relative: &str) {
    let Some(dir) = installation() else {
        eprintln!("CORERULES_DIR not set; skipping {relative}");
        return;
    };
    let path = find(&dir, relative).unwrap_or_else(|| panic!("{relative} not found"));
    let data = std::fs::read(&path).unwrap();
    let list = ObjectList::parse(&data).unwrap_or_else(|e| panic!("{relative}: {e}"));
    assert_eq!(
        list.to_bytes().unwrap(),
        data,
        "{relative} changed on write"
    );
}

fn find(dir: &std::path::Path, relative: &str) -> Option<PathBuf> {
    let mut path = dir.to_path_buf();
    for part in relative.split('/') {
        let entry = std::fs::read_dir(&path)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        path = entry.path();
    }
    Some(path)
}

#[test]
fn races() {
    round_trip_list("Database/Race.dat");
    round_trip_list("UserDbas/RaceU.dat");
}

#[test]
fn classes() {
    round_trip_list("Database/Class.dat");
    round_trip_list("UserDbas/ClassU.dat");
}

#[test]
fn treasure_tables() {
    round_trip_list("Database/Treasure.dat");
    round_trip_list("UserDbas/Treasure.dat");
}

#[test]
fn saved_treasures() {
    round_trip_list("Database/TreaBun.dat");
    round_trip_list("UserDbas/TreaBunU.dat");
}

#[test]
fn psionic_powers() {
    round_trip_list("Database/Psionic.dat");
    round_trip_list("UserDbas/PsionicU.dat");
}

#[test]
fn traps() {
    round_trip_list("Database/Traps.dat");
    round_trip_list("UserDbas/TrapsU.dat");
}

#[test]
fn phrase_spacing() {
    round_trip_list("Database/Phrase.DAT");
}

#[test]
fn conversion_maps() {
    let Some(dir) = installation() else {
        eprintln!("CORERULES_DIR not set; skipping Convert.dat");
        return;
    };
    let path = find(&dir, "Database/Convert.dat").expect("Convert.dat not found");
    let data = std::fs::read(&path).unwrap();
    let doc = ConversionMaps::parse(&data).unwrap_or_else(|e| panic!("Convert.dat: {e}"));
    let sizes: Vec<usize> = doc.maps.iter().map(Vec::len).collect();
    assert_eq!(sizes, [1454, 1348, 6, 407, 312, 1329, 65, 13]);
    assert_eq!(
        doc.to_bytes().unwrap(),
        data,
        "Convert.dat changed on write"
    );
}

/// The bytes of `relative`, or `None` when no installation is configured.
fn installation_file(relative: &str) -> Option<Vec<u8>> {
    let Some(dir) = installation() else {
        eprintln!("CORERULES_DIR not set; skipping {relative}");
        return None;
    };
    let path = find(&dir, relative).unwrap_or_else(|| panic!("{relative} not found"));
    Some(std::fs::read(&path).unwrap())
}

#[test]
fn parts() {
    use cr_formats::Object;
    use cr_formats::parts::PartsFile;
    for relative in ["Database/Parts.dat", "UserDbas/PartsU.dat"] {
        let Some(data) = installation_file(relative) else {
            return;
        };
        let file = PartsFile::parse(&data).unwrap_or_else(|e| panic!("{relative}: {e}"));
        assert_eq!(
            file.to_bytes().unwrap(),
            data,
            "{relative} changed on write"
        );
    }
    let data = installation_file("Database/Parts.dat").unwrap();
    let file = PartsFile::parse(&data).unwrap();
    let kits = file.of_type("Character Kits").unwrap();
    assert_eq!(kits.len(), 210);
    let Object::Part(kit) = &kits[0] else {
        panic!("not a part")
    };
    assert_eq!(kit.layout_version(), 7);
    assert!(matches!(kit.data.kit, Some(Some(ref k)) if matches!(**k, Object::PartKit(_))));
}

#[test]
fn spells() {
    use cr_formats::spells::SpellsFile;
    for relative in ["Database/Spells.dat", "UserDbas/SpellsU.dat"] {
        let Some(data) = installation_file(relative) else {
            return;
        };
        let file = SpellsFile::parse(&data).unwrap_or_else(|e| panic!("{relative}: {e}"));
        assert_eq!(
            file.to_bytes().unwrap(),
            data,
            "{relative} changed on write"
        );
    }
}

#[test]
fn classes_allowed() {
    round_trip_list("UserDbas/AllowedU.dat");
}
