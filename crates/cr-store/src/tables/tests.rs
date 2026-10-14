use super::*;

/// A table file from (lo, hi, name, v3, v4) rows.
fn table(title: &str, rows: &[(i32, i32, &str, i32, i32)]) -> TableFile {
    let mut text = format!("{title}\r\nItem1\r\nItem2\r\n0 0 0 0 0 0\r\n");
    for (lo, hi, name, v3, v4) in rows {
        text += &format!("{name}\r\nItem1\r\nItem2\r\n{lo} {hi} {v3} {v4} 0 0\r\n");
    }
    TableFile::parse(text.as_bytes(), None)
}

fn rolls(tables: Vec<(i32, TableFile)>) -> TableRolls {
    let mut captions = HashMap::new();
    captions.insert(
        TableBook::Dmg,
        HashMap::from([(88, "Magic".to_owned()), (94, "Sticks".to_owned())]),
    );
    TableRolls {
        flavor: Flavor::Expansion,
        tables: tables.into_iter().collect(),
        missing: Vec::new(),
        captions,
        wizard_spells: vec![(1, "Spark".into()), (1, "Glow".into()), (2, "Bang".into())],
        priest_spells: Vec::new(),
    }
}

fn dmg(number: u16) -> &'static ListEntry {
    LIST.iter()
        .find(|e| e.book == TableBook::Dmg && e.number == Some(number))
        .unwrap()
}

#[test]
fn range_is_rand_mod_n_plus_lo() {
    let mut a = Rand::seeded(7);
    let mut b = a.clone();
    let expected = b.rand() % 6 + 3;
    assert_eq!(range(&mut a, 3, 8), expected);
    // An empty range draws nothing.
    let before = a.calls();
    assert_eq!(range(&mut a, 5, 4), 0);
    assert_eq!(a.calls(), before);
}

#[test]
fn a_table_roll_avoids_the_previous_result_and_its_neighbours() {
    let mut session = Session::default();
    let mut rand = Rand::seeded(1);
    let mut previous = session.roll(&mut rand, d(1, 6));
    for _ in 0..200 {
        let r = session.roll(&mut rand, d(1, 6));
        assert!((r - previous).abs() >= 2, "{previous} then {r}");
        previous = r;
    }
    // The comparison is with the sum before the modifier, as in 0x538d84.
    let mut session = Session {
        previous: Some(105),
    };
    let mut rand = Rand::seeded(2);
    let r = session.roll(&mut rand, d(1, 100).plus(100));
    assert!((101..=200).contains(&r));
}

#[test]
fn a_roll_draws_like_the_original() {
    // The first roll of a session is free: count draws of 1 + rand() % sides.
    let mut session = Session::default();
    let mut rand = Rand::seeded(42);
    let mut copy = rand.clone();
    let expected = (1 + copy.rand() % 10) + (1 + copy.rand() % 10);
    assert_eq!(session.roll(&mut rand, d(2, 10)), expected);
}

#[test]
fn a_hopeless_reroll_gives_up() {
    let mut session = Session { previous: Some(1) };
    let r = session.roll(&mut Rand::seeded(3), d(1, 2));
    assert!((1..=2).contains(&r));
}

#[test]
fn magic_items_chain_to_the_next_table() {
    let t = rolls(vec![
        (
            20088,
            table("Table 88 internal", &[(1, 100, "Wands", 20094, 0)]),
        ),
        (
            20094,
            table(
                "Table 94:Wands",
                &[
                    (1, 10, "Wand of Light", 400, 0),
                    (11, 20, "Wand /n1(Wizard)/n2", 0, 90001),
                ],
            ),
        ),
        (90001, table("Charges", &[(1, 20, "Lots", 0, 0)])),
    ]);
    let mut rand = Rand::seeded(5);
    let out = t.roll(dmg(88), &[], &mut Session::default(), &mut rand);
    assert!(out.blocks.len() >= 2, "{out:?}");
    assert_eq!(out.blocks[0].title.as_deref(), Some("Table 88: Magic"));
    assert_eq!(out.blocks[0].rows[0][1], "Wands");
    assert_eq!(out.blocks[1].title.as_deref(), Some("Table 94: Sticks"));
    assert_eq!(out.blocks[1].header, ["Roll", "Item", "XP Value"]);
    let row = &out.blocks[1].rows[0];
    let roll: i32 = row[0].parse().unwrap();
    if roll <= 10 {
        assert_eq!(row[1..], ["Wand of Light", "400"]);
        assert_eq!(out.blocks.len(), 2);
    } else {
        assert_eq!(row[1..], ["Wand\n(Wizard)", ""]);
        // v4 names the sub-subtable, rolled on its own die.
        assert_eq!(out.blocks[2].title.as_deref(), Some("Charges"));
        assert_eq!(out.blocks[2].rows[0][1], "Lots");
    }
    let text = out.to_text();
    assert!(
        text.starts_with("Table 88: Magic\n\nRoll  Result\n----  ------\n"),
        "{text}"
    );
}

#[test]
fn d6_tables_pick_a_subtable_first() {
    let t = rolls(vec![
        (21089, table("A", &[(1, 20, "Alpha", 100, 0)])),
        (22089, table("B", &[(1, 20, "Beta", 200, 0)])),
        (23089, table("C", &[(1, 20, "Gamma", 300, 0)])),
    ]);
    for seed in 0..20 {
        let mut rand = Rand::seeded(seed);
        let die = 1 + rand.clone().rand() % 6;
        let out = t.roll(dmg(89), &[], &mut Session::default(), &mut rand);
        let (name, label) = match die {
            1 | 2 => ("Alpha", "Subtable A (1-2)"),
            3 | 4 => ("Beta", "Subtable B (3-4)"),
            _ => ("Gamma", "Subtable C (5-6)"),
        };
        assert_eq!(out.blocks[0].rows[0][1], name);
        assert_eq!(out.blocks[0].subtitle.as_deref(), Some(label));
        assert_eq!(out.blocks[0].title.as_deref(), Some("Table 89"));
    }
}

#[test]
fn scrolls_list_their_spells() {
    let t = rolls(vec![
        (21090, table("W", &[(1, 20, "2 Spells", 1, 2)])),
        (23090, table("P", &[(1, 20, "1 Spell", 1, 1)])),
        (22090, table("B", &[(1, 20, "Protection", 1500, 0)])),
    ]);
    let mut wizard = false;
    for seed in 0..40 {
        let out = t.roll(
            dmg(90),
            &[],
            &mut Session::default(),
            &mut Rand::seeded(seed),
        );
        let block = &out.blocks[0];
        if block.rows[0][1] == "2 Spells" {
            wizard = true;
            assert!(
                block
                    .subtitle
                    .as_deref()
                    .unwrap()
                    .starts_with("Subtable A (1-4), Wizard")
            );
            assert_eq!(block.notes[0], "2 wizard spells:");
            for note in &block.notes[1..] {
                assert!(
                    ["  Level 1  Spark", "  Level 1  Glow", "  Level 2  Bang"]
                        .contains(&note.as_str()),
                    "{note}"
                );
            }
        }
    }
    assert!(wizard);
}

#[test]
fn roll_twice_ignoring_the_top_rows() {
    let t = rolls(vec![(
        20115,
        table(
            "Abilities",
            &[
                (1, 50, "Low", 0, 0),
                (51, 92, "High", 0, 0),
                (93, 100, "Twice", 20115, 0),
            ],
        ),
    )]);
    for seed in 0..60 {
        let out = t.roll(
            dmg(115),
            &[],
            &mut Session::default(),
            &mut Rand::seeded(seed),
        );
        if out.blocks[0].rows[0][1] == "Twice" {
            assert_eq!(out.blocks.len(), 3);
            for b in &out.blocks[1..] {
                assert!(b.title.is_none());
                let r: i32 = b.rows[0][0].parse().unwrap();
                assert!(r <= 92);
            }
            return;
        }
    }
    panic!("no seed rolled 93 or more");
}

#[test]
fn art_values_are_rolled() {
    let mut text = String::from("Art\r\nItem1\r\nItem2\r\n0 0 0 0 0 0\r\n");
    text += "Item0\r\nItem1\r\nItem2\r\n1 100 2 6 100 0\r\n";
    let t = rolls(vec![(20087, TableFile::parse(text.as_bytes(), None))]);
    let out = t.roll(dmg(87), &[], &mut Session::default(), &mut Rand::seeded(9));
    let value = &out.blocks[0].rows[0][1];
    assert!(value.ends_with("gp (2d6 x 100)"), "{value}");
}

#[test]
fn lookups_add_dice_to_a_base() {
    let base = |v: i32| table("H", &[(0, 0, "Dwarf", v, 0), (0, 0, "Elf", v + 10, 0)]);
    let t = rolls(vec![
        (11010, base(40)),
        (12010, base(30)),
        (13010, base(100)),
        (14010, base(90)),
    ]);
    let phb10 = LIST
        .iter()
        .find(|e| e.book == TableBook::Phb && e.number == Some(10))
        .unwrap();
    let params = t.params(phb10, &[]);
    assert_eq!(params[0].options, ["Dwarf", "Elf"]);
    let out = t.roll(
        phb10,
        &[1, 1],
        &mut Session::default(),
        &mut Rand::seeded(4),
    );
    assert_eq!(out.blocks[0].header[1], "Height of Elf");
    let height = &out.blocks[0].rows[0][1];
    assert!(
        height.starts_with("50 + ") && height.contains("(1d10)"),
        "{height}"
    );
    assert!(out.blocks[1].rows[0][1].starts_with("110 + "));
}

#[test]
fn critical_hits_find_the_effect_table_from_the_location() {
    let t = rolls(vec![
        (
            50900,
            table(
                "Loc",
                &[(1, 5, "Right leg", 51000, 0), (6, 10, "Head", 55000, 0)],
            ),
        ),
        (51300, table("Slashing legs", &[(1, 20, "Leg cut", 0, 0)])),
        (55300, table("Slashing head", &[(1, 20, "Head cut", 0, 0)])),
    ]);
    let crit = LIST
        .iter()
        .find(|e| e.book == TableBook::CombatTactics)
        .unwrap();
    let params = t.params(crit, &[0]);
    assert_eq!(params[4].options, ["Random location", "Right leg", "Head"]);
    // Called shot to the head with a slashing weapon.
    let out = t.roll(
        crit,
        &[0, 1, 0, 0, 2],
        &mut Session::default(),
        &mut Rand::seeded(1),
    );
    assert_eq!(out.blocks[0].rows[0], ["Head"]);
    assert_eq!(out.blocks[1].rows[0][1], "Head cut");
    // A random location.
    let out = t.roll(
        crit,
        &[0, 1, 0, 3, 0],
        &mut Session::default(),
        &mut Rand::seeded(1),
    );
    let roll: i32 = out.blocks[0].rows[0][0].parse().unwrap();
    let effect = if roll <= 5 { "Leg cut" } else { "Head cut" };
    assert_eq!(out.blocks[1].rows[0][1], effect);
    let severity: i32 = out.blocks[1].rows[0][0].parse().unwrap();
    assert!((2..=16).contains(&severity));
}

#[test]
fn code_only_tables_refer_to_the_book() {
    let t = rolls(Vec::new());
    let out = t.roll(dmg(47), &[], &mut Session::default(), &mut Rand::seeded(1));
    assert_eq!(out.book_table, Some((TableBook::Dmg, 47)));
    assert!(out.blocks[0].notes[0].contains("Dungeon Master's Guide"));
}

#[test]
fn missing_tables_say_so() {
    let t = rolls(Vec::new());
    let out = t.roll(dmg(88), &[], &mut Session::default(), &mut Rand::seeded(1));
    assert!(out.to_text().contains("not in the Tables folder"));
}

#[test]
fn the_id_map() {
    assert_eq!(id_map(Flavor::Expansion).count(), 256);
    let file = |flavor, id| id_map(flavor).find(|e| e.0 == id).unwrap().1;
    assert_eq!(file(Flavor::Expansion, 55400), "POCT206E");
    assert_eq!(file(Flavor::Core20, 55400), "POCT207A");
    assert_eq!(
        id_map(Flavor::Expansion).find(|e| e.0 == 20093).unwrap().2,
        15
    );
    // Every step names mapped tables.
    fn check(steps: &[Step]) {
        for s in steps {
            match s {
                Step::Roll(r) => assert!(data::is_table_id(r.id), "{}", r.id),
                Step::Choose { branches, .. } => branches.iter().for_each(|b| check(b.steps)),
                Step::ByParam { options, .. } => options.iter().for_each(|o| check(o)),
                Step::Lookup { ids, .. } => ids.iter().for_each(|i| assert!(data::is_table_id(*i))),
                Step::Critical(c) => c
                    .targets
                    .iter()
                    .for_each(|t| assert!(data::is_table_id(t.0))),
                Step::Formula { .. } | Step::Insanity { .. } => {}
            }
        }
    }
    for e in LIST {
        check(e.steps);
    }
}

#[test]
fn text_layout_aligns_columns() {
    let block = Block {
        title: Some("T".into()),
        subtitle: None,
        header: vec!["Roll".into(), "Item".into(), "XP Value".into()],
        rows: vec![vec!["12".into(), "Long name\nsecond".into(), "4000".into()]],
        notes: vec!["note".into()],
    };
    let r = RollResult {
        blocks: vec![block],
        book_table: None,
    };
    assert_eq!(
        r.to_text(),
        "T\n\nRoll  Item       XP Value\n----  ----       --------\n12    Long name  4000\n      second\n\nnote\n"
    );
}
