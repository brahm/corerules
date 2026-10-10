//! Builds the catalog of a real installation (`CORERULES_DIR`, see
//! `cr-formats/tests/installation.rs`); without it the test passes without
//! checking.

#[test]
fn every_section_loads() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        eprintln!("CORERULES_DIR not set; skipping");
        return;
    };
    let install = cr_store::Installation::open(dir).unwrap();
    for section in install.catalog() {
        assert_eq!(section.error, None, "{}", section.title);
        if section.id != "characters" {
            assert!(!section.entries.is_empty(), "{} is empty", section.title);
        }
    }
}
