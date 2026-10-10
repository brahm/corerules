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

#[test]
fn choices_load() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = cr_store::Installation::open(dir).unwrap();
    for part_type in ["Non-Weapon Proficiencies", "Languages", "Equipment"] {
        let names = install.part_names(part_type).unwrap();
        assert!(!names.is_empty(), "{part_type}");
    }
    let wizard = install.spell_names(true).unwrap();
    assert!(wizard.iter().any(|n| n == "Burning Hands"));
    assert!(!install.spell_names(false).unwrap().is_empty());
}

/// The weight the Expansion stored in each of its test characters equals
/// what `carried_weight` computes from its databases.
#[test]
fn carried_weight_matches_the_original() {
    use cr_formats::character::Character;
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = cr_store::Installation::open(dir).unwrap();
    let expansion = install.path("UserDbas/AllowedU.dat").is_some();
    let fixtures = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/characters"
    );
    let files: &[&str] = if expansion {
        &["90688519.dat", "90988519.dat"]
    } else {
        &["v20/88010619.dat", "v20/99610619.dat"]
    };
    let parts = install.part_table().unwrap();
    for file in files {
        let data = std::fs::read(format!("{fixtures}/{file}")).unwrap();
        let character = Character::parse(&data).unwrap();
        let stored = f32::from_bits(character.data.carried_weight);
        if !expansion {
            // 2.0 never stores the weight.
            assert_eq!(stored, 0.0, "{file}");
            continue;
        }
        let computed = cr_store::lists::carried_weight(&character, &parts);
        assert!(
            (f64::from(stored) - computed).abs() < 0.01,
            "{file}: stored {stored}, computed {computed}"
        );
    }
}
