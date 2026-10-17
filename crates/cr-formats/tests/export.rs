//! Export files the originals wrote. With `CORERULES_GOLDEN` pointing at the
//! `export` golden folder (`t3/*.xpx` from the Expansion, `ev/*.xpt` from
//! Core Rules 2.0), every file is read and written back byte for byte.
//! Without it the test passes without checking.

use std::path::PathBuf;

use cr_formats::export::ExportFile;

#[test]
fn golden_exports_round_trip() {
    let Some(dir) = std::env::var_os("CORERULES_GOLDEN").map(PathBuf::from) else {
        eprintln!("CORERULES_GOLDEN not set; skipping");
        return;
    };
    if !dir.join("t3").is_dir() {
        eprintln!("CORERULES_GOLDEN is not the export folder; skipping");
        return;
    }
    let mut checked = 0;
    for program in ["t3", "ev"] {
        for entry in std::fs::read_dir(dir.join(program)).unwrap() {
            let path = entry.unwrap().path();
            if !path.is_file() {
                continue;
            }
            let data = std::fs::read(&path).unwrap();
            let file =
                ExportFile::parse(&data).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(file.is_expansion(), program == "t3", "{}", path.display());
            assert_eq!(file.to_bytes().unwrap(), data, "{}", path.display());
            eprintln!(
                "{}: {} characters, {} drawings, {} races, {}/{} spells, {} encounters, {} monsters, {} Map Maker maps",
                path.display(),
                file.characters.len(),
                file.campaign_maps.len(),
                file.races.len(),
                file.wizard_spells.len(),
                file.priest_spells.len(),
                file.encounters.len(),
                file.monsters.len(),
                file.map_maker.count
            );
            checked += 1;
        }
    }
    assert!(checked > 0);
}
