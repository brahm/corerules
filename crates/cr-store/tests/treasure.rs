//! The treasure generator.
//!
//! The synthetic tests need no data. With `CORERULES_DIR` (an installation)
//! and `CORERULES_GOLDEN` (the `treasure` golden folder, holding
//! `expansion/` and `core20/`), the sessions recorded in the original
//! program are replayed from their seeds and compared with the bundles it
//! saved, byte for byte; without them those tests pass without checking.

use std::fs;
use std::path::{Path, PathBuf};

use cr_formats::Object;
use cr_formats::database::ObjectList;
use cr_formats::inventory::InventoryItem;
use cr_formats::treasure::{TreasureTable, TreasureTableEntry};
use cr_store::Installation;
use cr_store::treasure::{
    Bundle, Coin, Counts, Flavor, Item, MonsterTreasure, Rand, SaveError, SavedTreasure, Summary,
    TreasureRules, TypeTotals, add_saved, encounter_counts, parse_letters, parse_saved,
    saved_to_bytes, thousands,
};

// ---- synthetic ----

#[test]
fn rand_matches_msvc() {
    // srand(1), the CRT's default state.
    let mut r = Rand::seeded(1);
    let first: Vec<i32> = (0..10).map(|_| r.rand()).collect();
    assert_eq!(
        first,
        [
            41, 18467, 6334, 26500, 19169, 15724, 11478, 29358, 26962, 24464
        ]
    );
    assert_eq!(r.calls(), 10);
    let mut r = Rand::seeded(857);
    assert!((0..1000).all(|_| (0..=0x7fff).contains(&r.rand())));
}

#[test]
fn fine_amount_fills_the_low_digits() {
    // n == 1: (1 + rand() % (s-1)) * m, then 1 + rand() % m.
    let mut r = Rand::seeded(1);
    // 41 % 2 = 1 gives 2000; 18467 % 1000 = 467 gives 468.
    assert_eq!(r.fine(1, 3, 1000), 2468);
    assert_eq!(r.calls(), 2);
    // fine(1, 3, 1000) is 1001..3000, never 1000.
    let mut r = Rand::seeded(5);
    let values: Vec<i32> = (0..5000).map(|_| r.fine(1, 3, 1000)).collect();
    assert!(values.iter().all(|v| (1001..=3000).contains(v)));
    // n > 1: d(n-1, s, m) + d(1, s-1, m) + d(1, m, 1), three draws for
    // n == 2.
    let mut a = Rand::seeded(9);
    let mut b = Rand::seeded(9);
    let expected = b.d(1, 10, 100) + b.d(1, 9, 100) + b.d(1, 100, 1);
    assert_eq!(a.fine(2, 10, 100), expected);
    assert_eq!(a.calls(), 3);
    // d(n, s, m) = m * sum of n dice.
    let mut r = Rand::seeded(1);
    assert_eq!(r.d(2, 6, 10), 10 * ((1 + 41 % 6) + (1 + 18467 % 6)));
}

fn names(bundle: &Bundle) -> Vec<(&str, u32)> {
    bundle
        .items
        .iter()
        .map(|i| (i.name.as_str(), i.quantity))
        .collect()
}

#[test]
fn insert_merges_and_sorts_like_strcmp() {
    let mut b = Bundle::default();
    b.insert(Item::new("apple", 1), true);
    b.insert(Item::new("Zebra", 1), true);
    b.insert(Item::new("* Gold Pieces", 10), true);
    b.insert(Item::new("* Gold Pieces", 5), true);
    // Without merge the same name is a second entry, after the first.
    b.insert(Item::new("Zebra", 2), false);
    assert_eq!(
        names(&b),
        [
            ("* Gold Pieces", 15),
            ("Zebra", 1),
            ("Zebra", 2),
            ("apple", 1)
        ]
    );

    // Items with contents (even empty) never merge, either way.
    let mut b = Bundle::default();
    let mut sword = Item::new("Sword", 1);
    sword.contents = Some(Bundle::default());
    b.insert(sword.clone(), true);
    b.insert(Item::new("Sword", 1), true);
    b.insert(Item::new("Sword", 1), true);
    b.insert(sword, true);
    assert_eq!(names(&b), [("Sword", 1), ("Sword", 2), ("Sword", 1)]);
    assert!(b.items[0].contents.is_some() && b.items[1].contents.is_none());

    // Flag is not compared when merging.
    let mut b = Bundle::default();
    let mut line = Item::new("Choice", 1);
    line.flag = true;
    b.insert(line, true);
    b.insert(Item::new("Choice", 1), true);
    assert_eq!(names(&b), [("Choice", 2)]);
    assert!(b.items[0].flag);
}

#[test]
fn totals_are_floored_separately() {
    let mut s = Summary {
        value_cp: 84_099 + 33_399,
        xp: 1234,
        magical_items: 0,
        ..Default::default()
    };
    s.by_type[13] = TypeTotals {
        value_cp: 84_099,
        xp: 0,
        count: 12,
    };
    s.by_type[16] = TypeTotals {
        value_cp: 33_399,
        xp: 0,
        count: 3_335,
    };
    assert_eq!(
        s.lines(),
        [
            "GP Value: 1,174",
            "XP Value: 1,234",
            "Magical Items: 0",
            "Number of Gems and Art: 12",
            "XP Value of Gems and Art: 0",
            "GP Value of Gems and Art: 840",
            "Number of Money: 3,335",
            "XP Value of Money: 0",
            "GP Value of Money: 333",
        ]
    );
    assert_eq!(thousands(0), "0");
    assert_eq!(thousands(999), "999");
    assert_eq!(thousands(1000), "1,000");
    assert_eq!(thousands(-1_234_567), "-1,234,567");
    assert_eq!(
        Item::new("* Silver Pieces", 3335).label(),
        "* Silver Pieces x3,335"
    );
}

#[test]
fn monster_letters() {
    let t = MonsterTreasure::parse("S, T (H)").unwrap();
    assert_eq!(t.outside[18], 1);
    assert_eq!(t.outside[19], 1);
    assert_eq!(t.inside[7], 1);
    assert_eq!(t.to_string(), "S, T (H)");
    let t = MonsterTreasure::parse("Ax2, Q").unwrap();
    assert_eq!(t.to_string(), "Ax2, Q ");
    // Only the outside letters count in an encounter, times the number
    // appearing.
    let w = MonsterTreasure::parse("W (G)").unwrap();
    let mut template: Counts = [0; 26];
    template[0] = 1;
    let counts = encounter_counts(&template, &[(w, 3), (t, 2)]);
    assert_eq!(counts[0], 1 + 4);
    assert_eq!(counts[16], 2);
    assert_eq!(counts[22], 3);
    assert_eq!(counts[6], 0);
    assert_eq!(parse_letters("A B B q").unwrap()[1], 2);
    assert_eq!(parse_letters("Bx3, A x2").unwrap()[..2], [2, 3]);
    assert!(parse_letters("A 7").is_err());
    assert_eq!(
        MonsterTreasure::parse("None").unwrap(),
        MonsterTreasure::default()
    );
}

#[test]
fn saving_refuses_a_used_name() {
    let mut list = Vec::new();
    let mut bundle = Bundle::default();
    bundle.insert(Item::new("* Gold Pieces", 7), true);
    assert_eq!(add_saved(&mut list, "Hoard", &bundle), Ok(()));
    assert_eq!(
        add_saved(&mut list, " hoard ", &bundle),
        Err(SaveError::NameUsed)
    );
    assert_eq!(add_saved(&mut list, "  ", &bundle), Err(SaveError::NoName));
    assert_eq!(add_saved(&mut list, "Another", &bundle), Ok(()));
    let bytes = saved_to_bytes(&list).unwrap();
    assert_eq!(parse_saved(&bytes).unwrap(), list);
}

/// A throw-away installation with made-up treasure tables.
struct Fake(PathBuf);

impl Drop for Fake {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn row(weight: u32, name: &str, is_table: bool, quantity: [u32; 3]) -> Object {
    Object::TreasureTableEntry(Box::new(TreasureTableEntry {
        weight,
        is_table: u8::from(is_table),
        item: InventoryItem {
            name: name.into(),
            quantity: 1,
            contents: None,
            unknown_0x8: 999,
            flag_0x4: 0,
            extra: None,
        },
        quantity_dice: quantity,
        sub_table_1_dice: [0; 3],
        sub_table_1: String::new(),
        sub_table_2_dice: [0; 3],
        sub_table_2: String::new(),
        intelligent_chance: 0,
        spell_min_level: 0,
        spell_max_level: 0,
        spell_count: 0,
        spell_wizard: 0,
    }))
}

fn fake(tag: &str) -> (Fake, Installation, TreasureRules) {
    let root = std::env::temp_dir().join(format!("cr-treasure-{tag}-{}", std::process::id()));
    fs::create_dir_all(root.join("Database")).unwrap();
    fs::create_dir_all(root.join("UserDbas")).unwrap();
    let table = |name: &str, entries: Vec<Object>| {
        Object::TreasureTable(TreasureTable {
            name: name.into(),
            flag_0x4: 1,
            entries,
        })
    };
    let list = ObjectList {
        objects: vec![
            table("Loop", vec![row(1, "Loop", true, [0, 0, 1])]),
            table("Empty", vec![]),
            table("Lost", vec![row(1, "Nowhere", true, [0, 0, 1])]),
            table(
                "Things",
                vec![
                    row(3, "Rope", false, [2, 6, 0]),
                    row(1, "Lamp", false, [0, 0, 1]),
                ],
            ),
        ],
    };
    fs::write(root.join("UserDbas/Treasure.dat"), list.to_bytes().unwrap()).unwrap();
    let install = Installation::open(&root).unwrap();
    let rules = install.treasure_rules().unwrap();
    (Fake(root), install, rules)
}

#[test]
fn table_rolls_and_errors() {
    let (_fake, install, rules) = fake("tables");
    assert_eq!(rules.flavor(), Flavor::Core20);
    let mut r = Rand::seeded(3);
    let loop_ = rules.roll_table(&mut r, "Loop").unwrap();
    assert_eq!(
        loop_.name,
        "!! Recursion depth limit of 127 exceeded on table 'Loop'."
    );
    assert!(loop_.flag);
    // One draw per level, 0 to 127.
    assert_eq!(r.calls(), 128);
    let empty = rules.roll_table(&mut r, "Empty").unwrap();
    assert_eq!(empty.name, "!! Table 'Empty' is empty.");
    assert_eq!(r.calls(), 128);
    let lost = rules.roll_table(&mut r, "Lost").unwrap();
    assert_eq!(lost.name, "!! Table 'Nowhere' doesn't exist.");
    assert!(rules.roll_table(&mut r, "Missing").is_none());

    // Weighted pick: rand() % 4 + 1 <= 3 is Rope, with 2d6 of them.
    let mut a = Rand::seeded(1);
    let item = rules.roll_table(&mut a, "Things").unwrap();
    let pick = 41 % 4 + 1;
    assert!(pick <= 3);
    assert_eq!(item.name, "Rope");
    assert_eq!(item.quantity as i32, (18467 % 6 + 1) + (6334 % 6 + 1));
    assert_eq!(a.calls(), 3);

    // Coin types need no tables: J is 3d8 copper.
    let mut r = Rand::seeded(1);
    let mut counts: Counts = [0; 26];
    counts[9] = 2;
    let mut b = Bundle::default();
    rules.roll(&mut r, &counts, Coin::Electrum, &mut b);
    let expected = [41, 18467, 6334, 26500, 19169, 15724]
        .iter()
        .map(|v| v % 8 + 1)
        .sum::<i32>() as u32;
    assert_eq!(names(&b), [("* Copper Pieces", expected)]);
    // L is electrum or platinum.
    counts = [0; 26];
    counts[11] = 1;
    rules.roll(&mut r, &counts, Coin::Electrum, &mut b);
    assert_eq!(b.items[1].name, "* Electrum Pieces");

    // Saving to the installation.
    assert!(install.saved_treasures().unwrap().is_empty());
    install.save_treasure("Coins", &b).unwrap();
    assert!(install.save_treasure("COINS", &b).is_err());
    let saved = install.saved_treasures().unwrap();
    assert_eq!(
        saved,
        [SavedTreasure {
            name: "Coins".into(),
            bundle: b
        }]
    );
}

// ---- goldens ----

fn env() -> Option<(Installation, PathBuf)> {
    let dir = std::env::var_os("CORERULES_DIR")?;
    let golden = std::env::var_os("CORERULES_GOLDEN")?;
    let install = Installation::open(Path::new(&dir)).expect("installation");
    Some((install, PathBuf::from(golden)))
}

fn counts(letters: &str) -> Counts {
    parse_letters(letters).unwrap()
}

/// One roll of a recorded session: name, letters, the rand() calls it
/// started and ended at.
struct Session<'a> {
    rules: &'a TreasureRules,
    rand: Rand,
    saved: Vec<SavedTreasure>,
}

impl Session<'_> {
    fn roll(&mut self, name: &str, letters: &str, calls: (u64, u64)) -> Bundle {
        let mut b = Bundle::default();
        self.roll_into(name, letters, calls, Coin::Platinum, &mut b);
        b
    }

    fn roll_into(
        &mut self,
        name: &str,
        letters: &str,
        calls: (u64, u64),
        coin: Coin,
        b: &mut Bundle,
    ) {
        assert_eq!(self.rand.calls(), calls.0, "{name}: start");
        self.rules.roll(&mut self.rand, &counts(letters), coin, b);
        assert_eq!(self.rand.calls(), calls.1, "{name}: end");
        add_saved(&mut self.saved, name, b).unwrap();
    }
}

fn compare_file(path: &Path, ours: &[SavedTreasure]) {
    let bytes = fs::read(path).unwrap();
    let theirs = parse_saved(&bytes).unwrap();
    assert_eq!(theirs.len(), ours.len(), "{}", path.display());
    for (t, o) in theirs.iter().zip(ours) {
        assert_eq!(t, o, "{}: bundle {}", path.display(), t.name);
    }
    assert_eq!(saved_to_bytes(ours).unwrap(), bytes, "{}", path.display());
}

#[test]
fn golden_sessions() {
    let Some((install, golden)) = env() else {
        return;
    };
    let rules = install.treasure_rules().unwrap();
    match rules.flavor() {
        Flavor::Expansion => {
            // tools/check_all.py: Tsrp3, seed 857.
            let mut s = Session {
                rules: &rules,
                rand: Rand::seeded(857),
                saved: Vec::new(),
            };
            s.roll("Golden A1", "A", (0, 107));
            s.roll("Golden AtoI ST", "A B C D E F G H I S T", (107, 443));
            let e3 = s.roll("Golden E3", "E E E", (443, 504));
            let mut b = s.roll("Golden Int A1", "A", (504, 550));
            s.roll_into(
                "Golden Accum EP",
                "L L Q",
                (550, 558),
                Coin::Electrum,
                &mut b,
            );
            compare_file(&golden.join("expansion/TreaBunU_2rolls.dat"), &s.saved[..2]);
            compare_file(&golden.join("expansion/TreaBunU_5rolls.dat"), &s.saved);

            // The summary of E3 as e3_sum.png shows it.
            let lines = rules.summary(&e3).lines();
            for line in [
                "GP Value: 1,173",
                "GP Value of Gems and Art: 840",
                "GP Value of Money: 333",
            ] {
                assert!(lines.iter().any(|l| l == line), "{line} in {lines:?}");
            }
        }
        Flavor::Core20 => {
            // tools/check_ev.py: Evermore, seed 764.
            let mut s = Session {
                rules: &rules,
                rand: Rand::seeded(764),
                saved: Vec::new(),
            };
            s.roll("EV Golden A1", "A", (0, 40));
            s.roll("EV Golden A1 T1", "A T", (40, 157));
            s.roll("EV Golden V10 T1", "Vx10 T", (157, 286));
            compare_file(&golden.join("core20/TreaBunU_3rolls.dat"), &s.saved);

            // The Expansion's level formula picks other spells.
            let mut other = rules.clone();
            other.set_flavor(Flavor::Expansion);
            let mut r = Rand::seeded(764);
            let mut b = Bundle::default();
            other.roll(&mut r, &counts("A"), Coin::Platinum, &mut b);
            b = Bundle::default();
            other.roll(&mut r, &counts("A T"), Coin::Platinum, &mut b);
            assert_ne!(b, s.saved[1].bundle);
        }
    }
}
