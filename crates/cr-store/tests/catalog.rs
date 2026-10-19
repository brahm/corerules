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
        if !["characters", "treasure", "encounters", "tables"].contains(&section.id) {
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

/// Every race's portraits come first among the portraits offered, and
/// every bitmap in `Bitmaps/Portrait` is offered once; the original's
/// palette, which added portraits are drawn in, is found.
#[test]
fn portraits_are_offered_race_first() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = cr_store::Installation::open(dir).unwrap();
    let folder = install.path("Bitmaps/Portrait").unwrap();
    let bitmaps = std::fs::read_dir(folder)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("bmp"))
        })
        .count();
    let mut offered_any = false;
    for race in install.list_objects("Race").unwrap() {
        let cr_formats::Object::Race(race) = race else {
            continue;
        };
        let choices = install.portrait_choices(&race.name).unwrap();
        assert_eq!(choices.len(), bitmaps, "{}", race.name);
        let offered = choices.iter().take_while(|c| c.race).count();
        assert!(choices[offered..].iter().all(|c| !c.race), "{}", race.name);
        offered_any |= offered > 0;
        for choice in &choices[..offered] {
            let image = cr_formats::bmp::decode(&std::fs::read(&choice.path).unwrap());
            assert!(image.is_some(), "{}", choice.file);
        }
    }
    assert!(offered_any);
    assert_eq!(install.program_palette().map(|p| p.len()), Some(256));
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
        let (armor, magic) = if expansion {
            parts.worn_lists(&character.data.inventory[5])
        } else {
            // 2.0 never stores the worn lists.
            Default::default()
        };
        assert_eq!(armor, character.data.armor, "{file}");
        assert_eq!(magic, character.data.unknown_0x17a4, "{file}");
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

/// The classes offered to a human made from Borin (the Expansion's own
/// dual-class dialog offered exactly these for the same abilities), and
/// none to the demihumans.
#[test]
fn dual_class_choices_match_the_original() {
    use cr_formats::character::Character;
    use cr_store::dual::ClassChoice;
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = cr_store::Installation::open(dir).unwrap();
    let fixtures = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/characters"
    );
    let table = install.class_table().unwrap();
    let borin =
        Character::parse(&std::fs::read(format!("{fixtures}/90688519.dat")).unwrap()).unwrap();
    assert!(matches!(table.choices(&borin), ClassChoice::Refused(_)));

    let mut human = borin.clone();
    human.data.race = "Human".into();
    human.data.strength = [17; 3];
    for (a, score) in human.data.abilities.iter_mut().zip([14, 16, 17, 13, 8]) {
        *a = [score; 3];
    }
    let ClassChoice::Allowed(mut names) = table.choices(&human) else {
        panic!("a human fighter with Str 17 may dual-class");
    };
    names.sort();
    if install.path("UserDbas/AllowedU.dat").is_some() {
        assert_eq!(
            names,
            [
                "Air Elementalist",
                "Alchemist",
                "Conjurer",
                "Earth Elementalist",
                "Fire Elementalist",
                "Geometer",
                "Invoker",
                "Mage",
                "Shadow Mage",
                "Song Wizard",
                "Water Elementalist",
                "Wild Mage",
            ]
        );
    } else {
        assert!(names.contains(&"Mage".to_owned()), "{names:?}");
    }
    table.add(&mut human, "Mage").unwrap();
    assert_eq!(human.data.class_title, "Fighter/Mage");
    assert_eq!(human.data.classes.names, ["Fighter", "Mage"]);
    assert_eq!(human.data.class_slots[1].unknown_0x270.len(), 8);
    assert!(table.add(&mut human, "Cleric").is_err());
    // Below 15 in the fighter's prime requisite, no class.
    human.data.classes.names.truncate(1);
    human.data.strength = [14; 3];
    assert!(matches!(table.choices(&human), ClassChoice::Refused(_)));
}
