use super::strings as st;
use super::*;

/// The made-up text the test program has at `t` ([`Program::fake`]).
fn f(t: Txt) -> String {
    format!("<{:x}>", t.0)
}

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
        program: Ok(Program::fake(&[])),
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
    assert_eq!(out.blocks[1].header.len(), 3);
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
    assert!(text.starts_with("Table 88: Magic\n\n"), "{text}");
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
            1 | 2 => ("Alpha", st::SUBTABLE_89[0]),
            3 | 4 => ("Beta", st::SUBTABLE_89[1]),
            _ => ("Gamma", st::SUBTABLE_89[2]),
        };
        assert_eq!(out.blocks[0].rows[0][1], name);
        assert_eq!(out.blocks[0].subtitle, Some(f(label)));
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
            assert_eq!(block.subtitle, Some(f(st::SUBTABLE_90[0])));
            // The spells follow in a block of their own: level, name.
            let spells = &out.blocks[1].rows;
            assert_eq!(spells.len(), 2);
            for row in spells {
                assert!(
                    [["1", "Spark"], ["1", "Glow"], ["2", "Bang"]]
                        .contains(&[&row[0][..], &row[1][..]]),
                    "{row:?}"
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
    assert_eq!(
        out.blocks[0].header[1],
        format!(
            "{}Elf {}{}",
            f(st::HEIGHT[0]),
            f(st::SEXES[1]),
            f(st::HEIGHT[1])
        )
    );
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
    assert_eq!(
        params[4].options,
        [f(st::RANDOM_LOCATION), "Right leg".into(), "Head".into()]
    );
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

fn entry_of(book: TableBook, number: u16) -> &'static ListEntry {
    LIST.iter()
        .find(|e| e.book == book && e.number == Some(number))
        .unwrap()
}

#[test]
fn dice_text_rolls_the_dice_in_a_text() {
    let mut rand = Rand::seeded(5);
    let mut copy = rand.clone();
    let out = code::dice_text(&mut rand, "Arrow (4d6), 1d4+1 bolts", DiceList::B);
    // "1d4+1" comes before "4d6" in the list.
    let bolts = 1 + copy.rand() % 4 + 1;
    let arrows: i32 = (0..4).map(|_| 1 + copy.rand() % 6).sum();
    assert_eq!(out, format!("Arrow ({arrows}), {bolts} bolts"));
    // "x 10" multiplies.
    let mut rand = Rand::seeded(6);
    let mut copy = rand.clone();
    let out = code::dice_text(&mut rand, "1d6+4 x 10 gp", DiceList::B);
    assert_eq!(out, format!("{} gp", (1 + copy.rand() % 6 + 4) * 10));
}

#[test]
fn dice_text_makes_a_unit_singular_after_a_one() {
    // 1d2 rolls a 1 for some seed; find one.
    for seed in 0..50 {
        let mut copy = Rand::seeded(seed);
        if copy.rand() % 2 != 0 {
            continue;
        }
        let out = code::dice_text(&mut Rand::seeded(seed), "for 1d2 rounds.", DiceList::B);
        assert_eq!(out, "for 1 round.");
        let out = code::dice_text(&mut Rand::seeded(seed), "1d2 wish spells", DiceList::B);
        assert_eq!(out, "1 wish spellls");
        // Only the parameter dialog's list knows turns.
        let out = code::dice_text(&mut Rand::seeded(seed), "1d2 turns", DiceList::B);
        assert_eq!(out, "1 turns");
        return;
    }
    panic!("no seed rolls a 1");
}

#[test]
fn rings_and_weapons_roll_the_dice_in_their_names() {
    let t = rolls(vec![(
        21108,
        table("Weapons", &[(1, 20, "Arrow (4d6)", 0, 0)]),
    )]);
    let spec = RollSpec::new(21108, d(1, 20), Title::None, st::WEAPON_HEAD, &[Col::Name])
        .dice_text(DiceList::B);
    let mut session = Session::default();
    let mut rand = Rand::seeded(3);
    let mut run = Run {
        rolls: &t,
        entry: None,
        book: TableBook::Dmg,
        params: &[],
        session: &mut session,
        rand: &mut rand,
        out: RollResult::default(),
        depth: 0,
        missing_text: false,
    };
    run.roll(&spec, None);
    let text = &run.out.blocks[0].rows[0][1];
    let n: i32 = text
        .trim_start_matches("Arrow (")
        .trim_end_matches(')')
        .parse()
        .unwrap();
    assert!((4..=24).contains(&n), "{text}");
}

#[test]
fn turning_undead_rolls_once_for_every_undead() {
    let t = rolls(Vec::new());
    let entry = entry_of(TableBook::Phb, 61);
    let params = t.params(entry, &[1]);
    assert_eq!(params[1].options[0], f(st::PALADIN_LEVELS[0]));
    assert_eq!(
        t.params(entry, &[0])[1].options[11],
        f(st::PRIEST_LEVELS[11])
    );
    let mut rand = Rand::seeded(9);
    let mut copy = rand.clone();
    // A 1st-level priest.
    let out = t.roll(entry, &[0, 0], &mut Session::default(), &mut rand);
    let roll = 1 + copy.rand() % 20;
    let rows = &out.blocks[0].rows;
    assert_eq!(rows.len(), 13);
    assert!(rows.iter().all(|r| r[0] == roll.to_string()));
    // Skeletons need a 10; ghasts cannot be turned.
    let skeletons: i32 = rows[0][2].parse().unwrap();
    if roll >= 10 {
        assert!((2..=12).contains(&skeletons));
    } else {
        assert_eq!(skeletons, 0);
    }
    assert_eq!(rows[5][2], "-");
    assert_eq!(rows[5][3], "-");
    // A 14th-level priest destroys skeletons (D*): some turned, some
    // dispelled.
    let out = t.roll(
        entry,
        &[0, 11],
        &mut Session::default(),
        &mut Rand::seeded(9),
    );
    let row = &out.blocks[0].rows[0];
    assert!((2..=8).contains(&row[2].parse::<i32>().unwrap()));
    assert!((2..=12).contains(&row[3].parse::<i32>().unwrap()));
}

#[test]
fn initial_funds_are_ten_times_the_dice() {
    let t = rolls(Vec::new());
    let entry = entry_of(TableBook::Phb, 43);
    let mut rand = Rand::seeded(4);
    let mut copy = rand.clone();
    let out = t.roll(entry, &[1], &mut Session::default(), &mut rand);
    let gold = (1 + copy.rand() % 4 + 1) * 10;
    assert_eq!(out.blocks[0].rows[0][0], gold.to_string());
    assert!(out.blocks[0].header[0].ends_with(&f(st::CLASS_GROUPS[1])));
    let out = t.roll(entry, &[3], &mut Session::default(), &mut rand);
    assert_eq!(out.blocks[0].notes.len(), 1);
}

#[test]
fn chance_tables_compare_a_d100() {
    let t = rolls(Vec::new());
    let mut rand = Rand::seeded(11);
    let mut copy = rand.clone();
    let out = t.roll(
        entry_of(TableBook::Dmg, 112),
        &[4],
        &mut Session::default(),
        &mut rand,
    );
    let roll = 1 + copy.rand() % 100;
    let result = f(st::SPELL_FAILURE[usize::from(roll > 50)]);
    assert_eq!(out.blocks[0].rows[0], vec![roll.to_string(), result]);
    let out = t.roll(
        entry_of(TableBook::SkillsPowers, 31),
        &[7],
        &mut Session::default(),
        &mut rand,
    );
    let roll = 1 + copy.rand() % 100;
    let read = f(st::SCROLL_USE[usize::from(roll > 80)]);
    assert_eq!(out.blocks[0].rows[0][1], read);
}

#[test]
fn languages_by_band() {
    let t = rolls(Vec::new());
    for seed in 0..40 {
        let mut copy = Rand::seeded(seed);
        let roll = 1 + copy.rand() % 100;
        let out = t.roll(
            entry_of(TableBook::Dmg, 118),
            &[],
            &mut Session::default(),
            &mut Rand::seeded(seed),
        );
        let expected = match roll {
            ..=40 => "1",
            41..=70 => "2",
            71..=85 => "3",
            86..=95 => "4",
            96..=99 => "5",
            _ => "6*",
        };
        assert_eq!(out.blocks[0].rows[0][1], expected);
    }
}

#[test]
fn field_searches_count_turns_like_the_original() {
    let t = rolls(Vec::new());
    let mut rand = Rand::seeded(21);
    let mut copy = rand.clone();
    let out = t.roll(
        entry_of(TableBook::SpellsMagic, 14),
        &[2],
        &mut Session::default(),
        &mut rand,
    );
    // Rare: 1d4+1 turns at 25%, then 27% a turn, the first counted twice.
    let turns = 1 + copy.rand() % 4 + 1;
    let mut turn = 1;
    let mut found = false;
    while turn <= turns {
        if copy.rand() % 100 < 25 {
            found = true;
            break;
        }
        turn += 1;
    }
    while !found {
        turn += 1;
        found = copy.rand() % 100 < 27;
    }
    let plural = if turn > 1 {
        f(st::FOUND_IN[2])
    } else {
        String::new()
    };
    assert_eq!(
        out.blocks[0].notes[0],
        format!(
            "{}{turn}{}{plural}{}",
            f(st::FOUND_IN[0]),
            f(st::FOUND_IN[1]),
            f(st::FOUND_IN[3])
        )
    );
}

#[test]
fn purchases_by_place() {
    let t = rolls(Vec::new());
    let mut rand = Rand::seeded(2);
    let mut copy = rand.clone();
    let out = t.roll(
        entry_of(TableBook::SpellsMagic, 15),
        &[1],
        &mut Session::default(),
        &mut rand,
    );
    let roll = 1 + copy.rand() % 100;
    let notes = &out.blocks[0].notes;
    assert_eq!(notes.len(), 4);
    assert_eq!(notes[0] == f(st::CAN[0]), roll <= 50);
    assert_eq!(notes[3] == f(st::CAN[3]), roll <= 80);
}

#[test]
fn gems_name_a_stone_of_the_class() {
    let t = rolls(vec![(
        20085,
        table(
            "Gems",
            &[
                (1, 25, "10 gp     Ornamental", 10, 0),
                (26, 100, "50 gp     Semi-precious", 50, 0),
            ],
        ),
    )]);
    let out = t.roll(
        entry_of(TableBook::Dmg, 85),
        &[],
        &mut Session::default(),
        &mut Rand::seeded(8),
    );
    let row = &out.blocks[0].rows[0];
    let roll: i32 = row[0].parse().unwrap();
    if roll <= 25 {
        assert_eq!(row[1..3], ["10 gp".to_owned(), "Ornamental".to_owned()]);
    } else {
        assert_eq!(row[2], "Semi-precious");
    }
    assert!(!row[3].is_empty());
}

#[test]
fn ioun_stones_list_each_kind_once() {
    let t = rolls(Vec::new());
    let out = t.roll_id(90040, &mut Session::default(), &mut Rand::seeded(13));
    let block = &out.blocks[0];
    let title = block.title.as_ref().unwrap();
    let count: usize = title.split('<').next().unwrap().parse().unwrap();
    assert!((1..=10).contains(&count));
    // Each kind once, each with at most two lines, and the dead stones.
    assert!(!block.rows.is_empty() && block.rows.len() <= 2 * count);
}

#[test]
fn a_robe_rolls_twice_more() {
    let t = rolls(vec![(
        90039,
        table(
            "Robe of Useful Items",
            &[(1, 96, "Ladder", 0, 0), (97, 100, "Roll twice more", 0, 2)],
        ),
    )]);
    for seed in 0..20 {
        let out = t.roll_id(90039, &mut Session::default(), &mut Rand::seeded(seed));
        let twice = out.blocks[0]
            .rows
            .iter()
            .filter(|r| r[1] == "Roll twice more")
            .count();
        if twice == 0 {
            assert_eq!(out.blocks.len(), 1);
        } else {
            assert_eq!(out.blocks[1].rows.len(), twice * 2);
            assert!(out.blocks[1].rows.iter().all(|r| r[1] == "Ladder"));
        }
    }
}

#[test]
fn blank_pages_of_the_book_of_infinite_spells_print_nothing() {
    let t = rolls(vec![(
        90018,
        table(
            "Book of Infinite Spells",
            &[(1, 30, "Blank page", 0, 0), (31, 100, "Wizard spell", 0, 0)],
        ),
    )]);
    let out = t.roll_id(90018, &mut Session::default(), &mut Rand::seeded(1));
    let block = &out.blocks[0];
    let pages: usize = block
        .subtitle
        .as_ref()
        .unwrap()
        .split('<')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!((23..=30).contains(&pages));
    assert!(block.rows.len() < pages);
    // The test spells exist for levels 1 and 2 only.
    for row in &block.rows {
        assert_eq!(row[1], "Wizard spell");
        assert!(row[2] == "1" || row[2] == "2");
    }
}

#[test]
fn table_84_opens_the_treasure_generator() {
    let t = rolls(Vec::new());
    let out = t.roll(dmg(84), &[], &mut Session::default(), &mut Rand::seeded(1));
    assert!(out.treasure);
}

#[test]
fn without_the_program_its_text_is_blank_and_noted() {
    let mut t = rolls(Vec::new());
    t.program = Err("Program/Tsrp3.exe is missing".into());
    let out = t.roll(
        dmg(112),
        &[0],
        &mut Session::default(),
        &mut Rand::seeded(1),
    );
    // The roll is still there, its text blank.
    assert_eq!(out.blocks[0].rows[0][1], "");
    let note = &out.blocks.last().unwrap().notes[0];
    assert!(note.contains("Program/Tsrp3.exe is missing"), "{note}");
    assert!(
        t.params(dmg(112), &[])[0]
            .options
            .iter()
            .all(String::is_empty)
    );
}

/// Every address of the text resolves in the real programs
/// (CORERULES_DIR; Core Rules 2.0 when the folder has Evermore.exe).
#[test]
fn the_text_is_in_the_real_programs() {
    let Ok(dir) = std::env::var("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(&dir).unwrap();
    let mut checked = 0;
    for flavor in [Flavor::Expansion, Flavor::Core20] {
        let Ok(program) = Program::load(&install, flavor) else {
            continue;
        };
        for t in st::all() {
            assert!(program.get(t).is_some(), "{flavor:?} {t:?}");
        }
        checked += 1;
    }
    assert!(checked > 0, "no known program in {dir}");
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
                Step::Formula { .. } | Step::Insanity { .. } | Step::Code(_) => {}
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
        treasure: false,
    };
    assert_eq!(
        r.to_text(),
        "T\n\nRoll  Item       XP Value\n----  ----       --------\n12    Long name  4000\n      second\n\nnote\n"
    );
}
