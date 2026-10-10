//! Round trips against a real installation, which cannot ship with the
//! repository. Set `CORERULES_DIR` to the installation folder (the one that
//! holds `Database/`) to run them; without it they pass without checking.

use std::path::PathBuf;

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
