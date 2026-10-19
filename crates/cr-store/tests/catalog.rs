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
        let choices = install
            .picture_choices(cr_store::pictures::Kind::Portrait, &race.name)
            .unwrap();
        assert_eq!(choices.len(), bitmaps, "{}", race.name);
        let offered = choices.iter().take_while(|c| c.first).count();
        assert!(choices[offered..].iter().all(|c| !c.first), "{}", race.name);
        offered_any |= offered > 0;
        for choice in &choices[..offered] {
            let image = cr_formats::bmp::decode(&std::fs::read(&choice.path).unwrap());
            assert!(image.is_some(), "{}", choice.file);
        }
    }
    assert!(offered_any);
    assert_eq!(install.program_palette().map(|p| p.len()), Some(256));
}

/// Every shipped monster type's picture is among those a monster type may
/// have, and none of those is a strip the books' pages are cut from.
#[test]
fn monster_pictures_include_the_shipped_ones() {
    use cr_store::pictures::Kind;
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = cr_store::Installation::open(dir).unwrap();
    let choices = install.picture_choices(Kind::Monster, "").unwrap();
    assert!(choices.len() > 300, "{}", choices.len());
    for choice in &choices {
        let header = std::fs::read(&choice.path).unwrap();
        let image = cr_formats::bmp::decode(&header).unwrap();
        assert!(image.width <= 2 * image.height, "{}", choice.file);
    }
    for object in install.list_objects("MonType").unwrap() {
        let cr_formats::Object::MonsterType(t) = object else {
            continue;
        };
        if t.picture.is_empty() {
            continue;
        }
        let path = install.picture_path(&t.picture, Kind::Monster);
        let path = path.unwrap_or_else(|| panic!("{}", t.picture));
        assert!(choices.iter().any(|c| c.path == path), "{}", t.picture);
    }
}

/// Every shipped weapon, armor, piece of equipment, magical item, gem and
/// coin has an icon there, and the catalog shows it.
#[test]
fn every_shipped_item_has_its_icon() {
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = cr_store::Installation::open(dir).unwrap();
    let icons = install.item_icons().unwrap();
    let mut items = 0;
    for part_type in cr_store::pictures::ICON_PART_TYPES {
        for name in install.part_names(part_type).unwrap() {
            let icon = icons.get(&name).unwrap_or_else(|| panic!("{name}"));
            let data = std::fs::read(&icon.path).unwrap();
            let image = cr_formats::bmp::decode(&data).unwrap();
            assert_eq!(image.height, 64, "{name}");
            assert!(icon.left.unwrap() + 64 <= image.width, "{name}");
            items += 1;
        }
    }
    assert!(items > 3000, "{items}");
    let catalog = install.catalog();
    let weapons = catalog.iter().find(|s| s.id == "weapons").unwrap();
    assert!(weapons.entries.iter().all(|e| e.icon.is_some()));
    let kits = catalog.iter().find(|s| s.id == "kits").unwrap();
    assert!(kits.entries.iter().all(|e| e.icon.is_none()));
}

/// The icons an item may be given are the 678 of the strips, grouped by
/// the items that have them, after the user's own; one made from a
/// picture is a 64 × 64 bitmap in the original's palette in `UserBMP`.
#[test]
fn icons_to_choose_include_the_shipped_and_added_ones() {
    use cr_store::pictures::IconGroup;
    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = cr_store::Installation::open(&dir).unwrap();
    // A copy holding what icons are made from and offered with, so adding
    // one leaves the installation as it is.
    let root = std::env::temp_dir().join(format!("cr-icons-{}", std::process::id()));
    for relative in ["Database/Parts.dat", "Bitmaps/Equip", "Bitmaps/Required"] {
        let from = install.path(relative).unwrap();
        let to = root.join(relative);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        if from.is_dir() {
            std::fs::create_dir_all(&to).unwrap();
            for entry in std::fs::read_dir(&from).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
            }
        } else {
            std::fs::copy(&from, &to).unwrap();
        }
    }
    let copy = cr_store::Installation::open(&root).unwrap();

    let choices = copy.icon_choices().unwrap();
    assert_eq!(choices.len(), 678);
    let bag = choices.iter().find(|c| c.stored == "12").unwrap();
    assert_eq!(bag.group, IconGroup::PartType("Equipment"));
    assert!(bag.names.iter().any(|n| n == "Bag"), "{:?}", bag.names);
    assert_eq!(choices[0].group, IconGroup::PartType("Weapons"));
    let icons = copy.item_icons().unwrap();
    for part_type in cr_store::pictures::ICON_PART_TYPES {
        for name in copy.part_names(part_type).unwrap() {
            let icon = icons.get(&name).unwrap();
            assert!(choices.iter().any(|c| &c.icon == icon), "{name}");
        }
    }

    let mut png = Vec::new();
    image::RgbaImage::from_pixel(200, 100, image::Rgba([200, 20, 20, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    std::fs::write(root.join("Espada longa.png"), &png).unwrap();
    let stored = copy.add_icon(&root.join("Espada longa.png")).unwrap();
    assert_eq!(stored, "CRE_Espada longa.bmp");
    let icon = copy.icon(&stored).unwrap();
    assert_eq!(icon.left, None);
    let data = std::fs::read(&icon.path).unwrap();
    let image = cr_formats::bmp::decode(&data).unwrap();
    assert_eq!((image.width, image.height), (64, 64));
    assert_eq!(cr_formats::bmp::palette(&data), copy.program_palette());
    let choices = copy.icon_choices().unwrap();
    assert_eq!(choices.len(), 679);
    assert_eq!(choices[0].stored, stored);
    assert_eq!(choices[0].group, IconGroup::User);
    std::fs::remove_dir_all(&root).unwrap();
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
