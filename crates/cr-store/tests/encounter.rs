//! The encounter generator.
//!
//! The synthetic tests build a small installation of made-up monsters and
//! templates in a temporary folder. With `CORERULES_GOLDEN` pointing at the
//! `encounter` golden folder (holding `exp/EncountU_*.dat`), every saved
//! file is read and written back byte for byte; with `CORERULES_DIR` too,
//! the generations recorded in the original programs are replayed from
//! their seeds and compared with what they saved. Without them (or with
//! another golden folder) those tests pass without checking.

use std::fs;
use std::path::{Path, PathBuf};

use cr_formats::character::{Character, EXPANSION_VERSION};
use cr_formats::classes::Class;
use cr_formats::database::ObjectList;
use cr_formats::encounter::{
    EncounterTemplate, TemplateItemProperties, TemplateMonsterProperties, TemplateNumAppear,
};
use cr_formats::monster::Monster;
use cr_formats::race::Race;
use cr_formats::{ArchiveReader, Field, Object};
use cr_store::Installation;
use cr_store::encounter::{
    Dice, Encounter, EncounterRules, Flavor, FrequencyTable, FrequencyTables, LairMode, NameError,
    NpcPackage, Package, Prices, Rand, TableFilter, check_name, dice_text, encounters_to_bytes,
    experience_for_hit_dice, hit_point_text, parse_encounters, roll_2_to_20, roll_and_pick,
    store_encounter,
};

// ---- synthetic installation ----

/// A record of zeros: empty strings and lists, every number 0.
fn blank<T: Field>() -> T {
    let zeros = vec![0u8; 1 << 14];
    T::read(&mut ArchiveReader::new(&zeros)).unwrap()
}

fn monster(name: &str, kind: &str, hd: (i32, i32, i32), na: (i32, i32)) -> Monster {
    let mut m: Monster = blank();
    m.name = name.into();
    m.monster_type = kind.into();
    m.hit_dice = hd.0;
    m.unknown_0x438 = hd.1;
    m.hit_die_size = hd.2;
    m.number_appearing_min = na.0;
    m.number_appearing_max = na.1;
    m.experience = 10;
    m
}

fn line(name: &str, dice: [i32; 4]) -> TemplateMonsterProperties {
    let mut p: TemplateMonsterProperties = blank();
    p.monster_name = name.into();
    p.dice[..4].copy_from_slice(&dice);
    p.percent = 100;
    p
}

fn template(name: &str, lines: Vec<TemplateMonsterProperties>) -> EncounterTemplate {
    let mut t: EncounterTemplate = blank();
    t.name = name.into();
    t.monster_type = "Goblinoid".into();
    t.monsters = lines
        .into_iter()
        .map(|l| Object::TemplateMonsterProperties(Box::new(l)))
        .collect();
    t
}

struct Temp(PathBuf);

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// An installation holding `monsters` and `templates`, the Expansion's
/// (with `AllowedU.dat`) or 2.0's.
fn install(
    name: &str,
    monsters: Vec<Monster>,
    templates: Vec<EncounterTemplate>,
    expansion: bool,
) -> (Temp, Installation) {
    let root = std::env::temp_dir().join(format!("cr-encounter-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Database")).unwrap();
    fs::create_dir_all(root.join("UserDbas")).unwrap();
    let write = |relative: &str, objects: Vec<Object>| {
        fs::write(
            root.join(relative),
            ObjectList { objects }.to_bytes().unwrap(),
        )
        .unwrap();
    };
    write(
        "Database/Monster.dat",
        monsters
            .into_iter()
            .map(|m| Object::Monster(Box::new(m)))
            .collect(),
    );
    write(
        "Database/EncPlat.dat",
        templates
            .into_iter()
            .map(|t| Object::EncounterTemplate(Box::new(t)))
            .collect(),
    );
    write("UserDbas/Treasure.dat", vec![]);
    if expansion {
        fs::write(root.join("UserDbas/AllowedU.dat"), [0, 0]).unwrap();
    }
    let install = Installation::open(&root).unwrap();
    (Temp(root), install)
}

fn goblins() -> Vec<Monster> {
    vec![
        monster("Goblin", "Goblinoid", (1, 0, 6), (4, 6)),
        monster("Goblin, Leader", "Goblinoid", (2, 1, 8), (1, 1)),
        monster("Wolf", "Canine", (3, 0, 8), (2, 5)),
        monster("Half HD", "Goblinoid", (0, 4, 8), (1, 1)),
    ]
}

fn rules(name: &str, templates: Vec<EncounterTemplate>, expansion: bool) -> (Temp, EncounterRules) {
    let (temp, install) = install(name, goblins(), templates, expansion);
    let rules = install.encounter_rules().unwrap();
    (temp, rules)
}

/// What `generate` draws for a package of `count` monsters with these
/// hit dice, merged in first-occurrence order.
fn hit_points(r: &mut Rand, count: i32, (dice, bonus, sides): (i32, i32, i32)) -> Vec<(i32, i32)> {
    let mut list: Vec<(i32, i32)> = Vec::new();
    for _ in 0..count {
        let hp = (0..dice).map(|_| r.rand() % sides + 1).sum::<i32>() + bonus;
        match list.iter_mut().find(|e| e.0 == hp) {
            Some(e) => e.1 += 1,
            None => list.push((hp, 1)),
        }
    }
    list
}

fn names(e: &Encounter) -> Vec<(&str, i32)> {
    e.monsters
        .iter()
        .map(|p| (p.monster.name.as_str(), p.count))
        .collect()
}

#[test]
fn between_follows_each_program() {
    let (_t, mut rules) = rules("between", vec![], true);
    assert_eq!(rules.flavor(), Flavor::Expansion);
    let mut r = Rand::seeded(1);
    // An empty range gives 0 without a draw in the Expansion.
    assert_eq!(rules.between(&mut r, 5, 4), 0);
    assert_eq!(rules.between(&mut r, 5, 3), 0);
    assert_eq!(r.calls(), 0);
    // A range of one value still draws.
    assert_eq!(rules.between(&mut r, 7, 7), 7);
    assert_eq!(r.calls(), 1);
    // 2.0 has no check: rand() % -1 is 0, so lo comes back after a draw.
    rules.set_flavor(Flavor::Core20);
    let mut r = Rand::seeded(1);
    assert_eq!(rules.between(&mut r, 5, 3), 5);
    assert_eq!(r.calls(), 1);
    // dice: (sum of 1 + rand() % sides) * m + b.
    let mut a = Rand::seeded(3);
    let mut b = Rand::seeded(3);
    let expected = ((b.rand() % 6 + 1) + (b.rand() % 6 + 1)) * 10 + 2;
    assert_eq!(rules.roll(&mut a, Dice::new(2, 6, 10, 2)), expected);
}

#[test]
fn hit_points_merge_in_first_order() {
    let (_t, rules) = rules("hp", vec![], true);
    let mut r = Rand::seeded(42);
    let mut check = Rand::seeded(42);
    let p = rules
        .new_monster_package("Goblin", Some(5), &mut r)
        .unwrap();
    assert_eq!(p.count, 5);
    assert_eq!(p.hit_points, hit_points(&mut check, 5, (1, 0, 6)));
    assert_eq!(p.hit_points.iter().map(|h| h.1).sum::<i32>(), 5);
    // Outside the No. Appearing range the count is its low end.
    let p = rules
        .new_monster_package("Goblin", Some(9), &mut r)
        .unwrap();
    assert_eq!(p.count, 4);
    // HD 1/2 is (0, 4, 8): a fixed 4, no draws.
    let before = r.calls();
    let p = rules.new_monster_package("Half HD", None, &mut r).unwrap();
    assert_eq!(p.hit_points, [(4, 1)]);
    assert_eq!(r.calls(), before);
    assert!(rules.new_monster_package("Nobody", None, &mut r).is_none());

    assert_eq!(
        hit_point_text(&[(40, 3), (31, 3), (42, 1)]),
        "3x40, 3x31, 42"
    );
}

#[test]
fn hit_points_given_by_hand() {
    let (_t, rules) = rules("hp-hand", vec![], true);
    let mut r = Rand::seeded(7);
    let mut p = rules
        .new_monster_package("Goblin, Leader", Some(1), &mut r)
        .unwrap();
    // HD 2+1 of d8: 3 to 17.
    assert_eq!(p.hit_point_range(), (3, 17));
    p.count = 4;
    rules.roll_hit_points(&mut p, &mut r);
    assert_eq!(p.unassigned(), 0);
    let (first, n) = p.hit_points[0];
    // Taking monsters off a value leaves them unassigned.
    p.set_count_with(first, 0);
    assert_eq!(p.count_with(first), 0);
    assert_eq!(p.unassigned(), n);
    // A value gets at most the unassigned ones, and goes at the end.
    let new = if first == 17 { 3 } else { 17 };
    let had = p.count_with(new);
    p.set_count_with(new, 100);
    assert_eq!(p.count_with(new), had + n);
    assert_eq!(p.unassigned(), 0);
    if had == 0 {
        assert_eq!(p.hit_points.last(), Some(&(new, n)));
    }
    assert_eq!(p.most_with(first), 0);
    // Re-roll gives every monster hit points again.
    p.set_count_with(new, 0);
    rules.roll_hit_points(&mut p, &mut r);
    assert_eq!(p.hit_points.iter().map(|h| h.1).sum::<i32>(), 4);
}

/// A package as name, count and hit points.
type Row<'a> = (&'a str, i32, Vec<(i32, i32)>);

#[test]
fn list_and_and_order() {
    // "Goblin" from a list of two goblin kinds and wolves, 1d1*10 of
    // them; "and" 1d1 leader.
    let mut l = line("Goblin", [1, 1, 10, 0]);
    l.monsters = vec![
        Object::Monster(Box::new(monster("Goblin", "Goblinoid", (1, 0, 6), (4, 6)))),
        Object::Monster(Box::new(monster("Wolf", "Canine", (3, 0, 8), (2, 5)))),
        Object::Monster(Box::new(monster("Goblin", "Goblinoid", (1, 0, 6), (4, 6)))),
    ];
    l.flags[1] = 1;
    l.second_name = "Goblin, Leader".into();
    l.dice[4..].copy_from_slice(&[1, 1, 1, 0]);
    let (_t, rules) = rules("list", vec![template("Band", vec![l])], true);

    let mut r = Rand::seeded(11);
    let g = rules.generate("Band", &mut r).unwrap();
    let e = &g.encounter;

    // Replay: 1d1, ten draws over three entries, then hit points entry by
    // entry, then the leader.
    let mut c = Rand::seeded(11);
    c.rand(); // 1d1
    let mut tally = [0; 3];
    for _ in 0..10 {
        tally[(c.rand() % 3) as usize] += 1;
    }
    let hd = [(1, 0, 6), (3, 0, 8), (1, 0, 6)];
    let kinds = ["Goblin", "Wolf", "Goblin"];
    let mut packages = Vec::new();
    for i in 0..3 {
        if tally[i] > 0 {
            packages.push((kinds[i], tally[i], hit_points(&mut c, tally[i], hd[i])));
        }
    }
    c.rand(); // the leader's 1d1
    let leader = hit_points(&mut c, 1, (2, 1, 8));
    // Extras first, then the "and" package, then the first list package.
    let mut expected: Vec<Row> = packages[1..].to_vec();
    expected.push(("Goblin, Leader", 1, leader));
    expected.push(packages[0].clone());
    let got: Vec<Row> = e
        .monsters
        .iter()
        .map(|p| (p.monster.name.as_str(), p.count, p.hit_points.clone()))
        .collect();
    assert_eq!(got, expected);
    // An empty treasure table set rolls nothing; the stream ends here.
    assert_eq!(r.calls(), c.calls());
    assert_eq!(e.template, "Band");
    assert_eq!(e.monster_type, "Goblinoid");
    assert!(e.name.is_empty());
}

#[test]
fn number_appearing_fallback_and_percent() {
    // 0 dice: the monster's own 4-6; then a line at 0% that never
    // happens (one d100).
    let mut never = line("Wolf", [1, 1, 1, 0]);
    never.percent = 0;
    let (_t, rules) = rules(
        "na",
        vec![template("Pack", vec![line("Goblin", [0, 0, 0, 0]), never])],
        true,
    );
    let mut r = Rand::seeded(5);
    let g = rules.generate("Pack", &mut r).unwrap();
    let mut c = Rand::seeded(5);
    let n = c.rand() % 3 + 4;
    hit_points(&mut c, n, (1, 0, 6));
    c.rand();
    assert_eq!(names(&g.encounter), [("Goblin", n)]);
    assert_eq!(r.calls(), c.calls());
    // An unknown monster fails the line with the original's message.
    let (_t, rules) = self::rules(
        "missing",
        vec![template("Lost", vec![line("Orc", [1, 1, 1, 0])])],
        true,
    );
    let g = rules.generate("Lost", &mut Rand::seeded(1)).unwrap();
    assert!(g.encounter.monsters.is_empty());
    assert_eq!(
        g.messages,
        ["Unable to find the Monster in the Database: Orc"]
    );
}

#[test]
fn dependents_count_the_first_pass_only() {
    // 13 goblins; for every 5 goblins beyond 2, 1 leader: (13-2)/5 = 2.
    let mut leaders = line("Goblin, Leader", [1, 1, 1, 0]);
    leaders.flags[0] = 1;
    leaders.per = 5;
    leaders.beyond = 2;
    leaders.reference_name = "Goblin".into();
    // Counting leaders (a pass-2 package) finds none.
    let mut wolves = line("Wolf", [1, 1, 1, 0]);
    wolves.flags[0] = 1;
    wolves.reference_name = "Goblin, Leader".into();
    // By type: 13 goblinoids, beyond 13 is not above: nothing.
    let mut none = line("Wolf", [1, 1, 1, 0]);
    none.flags[0] = 1;
    none.beyond = 13;
    none.monster_type.name = "Goblinoid".into();
    // Dependent lines come after the others whatever their place.
    let lines = vec![leaders, wolves, line("Goblin", [1, 1, 13, 0]), none];
    let (_t, rules) = rules("dep", vec![template("Camp", lines)], true);
    let g = rules.generate("Camp", &mut Rand::seeded(9)).unwrap();
    assert_eq!(names(&g.encounter), [("Goblin", 13), ("Goblin, Leader", 2)]);
}

#[test]
fn items_compound_and_double() {
    // 7 goblins (pass 1); an item for every 2 goblins beyond 1: 3 groups
    // of 1d1 * 2 carts, compounding: (0+1)*2 = 2, (2+1)*2 = 6,
    // (6+1)*2 = 14. Coins 0d0 * 2 + 1 per cart compound the same way, 14
    // times: 2^14 - 1. Wolves come with the carts; goblins double, their
    // hit points stay.
    let mut item: TemplateItemProperties = blank();
    item.percent = 100;
    item.dependency = [1, 2, 1];
    item.monster.name = "Goblin".into();
    item.part.head.name = "Cart".into();
    item.dice = [1, 1, 2, 0, 3, 0, 0, 2, 1];
    item.monster_names = vec!["Goblin".into()];
    let mut escort: TemplateNumAppear = blank();
    escort.dice = [1, 1, 1, 1];
    *escort.monster = monster("Wolf", "Canine", (3, 0, 8), (2, 5));
    item.escorts = vec![Object::TemplateNumAppear(Box::new(escort.clone()))];
    // For each cart: 14 × 0 = 0, then the wolves' own 2-5.
    escort.dice = [0, 0, 0, 0];
    item.per_item = vec![Object::TemplateNumAppear(Box::new(escort))];
    let mut t = template("Caravan", vec![line("Goblin", [1, 1, 7, 0])]);
    t.items = vec![Object::TemplateItemProperties(Box::new(item))];
    let (_t, rules) = rules("items", vec![t], true);
    let g = rules.generate("Caravan", &mut Rand::seeded(21)).unwrap();
    let e = &g.encounter;
    let items: Vec<(&str, u32)> = e
        .items
        .items
        .iter()
        .map(|i| (i.name.as_str(), i.quantity))
        .collect();
    assert_eq!(items, [("* Copper Pieces", 16383), ("Cart", 14)]);
    assert_eq!(e.monsters[0].monster.name, "Goblin");
    assert_eq!(e.monsters[0].count, 14);
    assert_eq!(e.monsters[0].hit_points.iter().map(|h| h.1).sum::<i32>(), 7);
    assert_eq!(e.monsters[1].monster.name, "Wolf");
    assert_eq!(e.monsters[1].count, 2);
    assert_eq!(e.monsters[2].monster.name, "Wolf");
    assert!((2..=5).contains(&e.monsters[2].count));
}

#[test]
fn treasure_letters_scale_with_numbers() {
    let mut t = template("Hoard", vec![line("Goblin", [1, 1, 3, 0])]);
    t.lair_treasure[2] = 1;
    let mut gob = goblins();
    gob[0].unknown_0x25c[9] = (5, 2); // J x2 outside, inside ignored
    let (_t, install) = install("treasure", gob, vec![t], true);
    let rules = install.encounter_rules().unwrap();
    let g = rules.generate("Hoard", &mut Rand::seeded(2)).unwrap();
    let counts = g.encounter.treasure_counts();
    assert_eq!(counts[2], 1);
    assert_eq!(counts[9], 6);
    assert_eq!(g.encounter.lair_treasure[2], 1);
    // J is 3d8 copper per roll; C rolls with tables that do not exist.
    let copper = g
        .encounter
        .treasure
        .items
        .iter()
        .find(|i| i.name == "* Copper Pieces")
        .unwrap();
    assert!(copper.quantity >= 18);
}

#[test]
fn editing_an_encounter() {
    let (_t, rules) = rules("edit", vec![], true);
    let mut r = Rand::seeded(4);
    let mut e = Encounter::default();
    e.monsters
        .push(rules.new_monster_package("Wolf", Some(3), &mut r).unwrap());
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/characters/90688519.dat"
    );
    let character = Character::parse(&fs::read(fixture).unwrap()).unwrap();
    // 2.0 cannot hold an Expansion character.
    assert!(e.add_npc(character.clone(), Flavor::Core20).is_err());
    e.add_npc(character, Flavor::Expansion).unwrap();
    assert_eq!(e.packages(), [Package::Monster(0), Package::Npc(0)]);
    assert_eq!(
        e.package_line(Package::Monster(0), &rules).unwrap(),
        "3 - Wolf"
    );

    // Edit…: a new number rolls every hit point again.
    let before = r.calls();
    e.set_count(Package::Monster(0), 6, &rules, &mut r);
    assert_eq!(e.monsters[0].count, 6);
    assert_eq!(r.calls(), before + 18);
    assert_eq!(e.monsters[0].hit_points.iter().map(|h| h.1).sum::<i32>(), 6);
    // The same number or 0 changes nothing; NPCs only take the number.
    e.set_count(Package::Monster(0), 6, &rules, &mut r);
    e.set_count(Package::Npc(0), 4, &rules, &mut r);
    assert_eq!(r.calls(), before + 18);
    assert_eq!(e.npcs[0].count, 4);
    e.remove(Package::Monster(0));
    assert_eq!(e.packages(), [Package::Npc(0)]);

    let totals = rules.totals(&Encounter {
        monsters: vec![
            rules
                .new_monster_package("Goblin", Some(5), &mut r)
                .unwrap(),
        ],
        ..Encounter::default()
    });
    assert_eq!(totals.monster_xp, 50);

    // OK: names are trimmed and compared without regard to case.
    let mut list = vec![Encounter {
        name: "Goblin Camp".into(),
        ..Encounter::default()
    }];
    assert_eq!(check_name(&list, "  ", None), Err(NameError::NoName));
    assert_eq!(
        check_name(&list, " goblin camp", None),
        Err(NameError::NameUsed)
    );
    assert_eq!(check_name(&list, "goblin camp", Some(0)), Ok(()));
    let renamed = Encounter {
        name: "Wolves".into(),
        ..Encounter::default()
    };
    store_encounter(&mut list, renamed.clone(), Some(0)).unwrap();
    store_encounter(&mut list, e, None).unwrap_err();
    assert_eq!(list, [renamed]);
}

#[test]
fn experience_by_hit_dice() {
    let xp = |hd, m| experience_for_hit_dice(hd, m);
    assert_eq!([xp(-3, 0), xp(0, 5), xp(1, -2), xp(1, -1)], [7, 7, 7, 15]);
    let table: Vec<i32> = (1..=9).map(|hd| xp(hd, 0)).collect();
    assert_eq!(table, [15, 35, 65, 120, 175, 270, 420, 650, 975]);
    // A positive modifier reads the next entry below 10 hit dice only.
    assert_eq!([xp(1, 1), xp(9, 3), xp(10, 1)], [35, 1400, 1400]);
    assert_eq!(
        [xp(11, 0), xp(12, 0), xp(13, 0), xp(20, 0)],
        [2000, 2000, 3000, 10000]
    );
}

/// A 2.0-layout class: experience `(level - 1) * 2000`, the abbreviation,
/// and spell slots from `slots` (level, spell level) on.
fn class(name: &str, abbreviation: &str, group: i32, slots: &[(usize, usize)]) -> Object {
    let Class::Core20(mut c) = blank::<Class>() else {
        unreachable!()
    };
    c.name = name.into();
    c.unknown_0x10[0] = group;
    c.unknown_0x2610[0] = abbreviation.into();
    for (level, xp) in c.experience.iter_mut().enumerate().skip(2) {
        *xp = (level as i32 - 1) * 2000;
    }
    for &(from, spell_level) in slots {
        for row in &mut c.unknown_0x264[from..] {
            row[spell_level] = 1;
        }
    }
    Object::Class(Box::new(Class::Core20(c)))
}

#[test]
fn npc_experience_and_names() {
    let (temp, install) = install("npcs", goblins(), vec![], true);
    let mut dwarf: Race = blank();
    dwarf.name = "Dwarf".into();
    // Natural armor class 1.
    dwarf.unknown_0x16c[0] = 1;
    fs::write(
        temp.0.join("Database/Race.dat"),
        ObjectList {
            objects: vec![Object::Race(Box::new(dwarf))],
        }
        .to_bytes()
        .unwrap(),
    )
    .unwrap();
    let classes = vec![
        class("Fighter", "F", 0, &[]),
        class("Mage", "M", 3, &[(1, 1), (5, 3)]),
    ];
    fs::write(
        temp.0.join("Database/Class.dat"),
        ObjectList { objects: classes }.to_bytes().unwrap(),
    )
    .unwrap();
    let rules = install.encounter_rules().unwrap();

    let mut c = Character::blank(EXPANSION_VERSION).unwrap();
    c.data.name = "Borin".into();
    c.data.race = "Elf".into();
    c.data.classes.names = vec!["Fighter".into()];
    // Dex 10, Int 12.
    c.data.abilities[0] = [10; 3];
    c.data.abilities[2] = [12; 3];
    let xp = |c: &Character| rules.npc_experience(c);
    // A Fighter 1 with armor class 10: 1 hit die.
    assert_eq!(xp(&c), 15);
    assert_eq!(rules.npc_title(&c), "Elf F1");
    // Intelligence above 12: +1.
    c.data.abilities[2] = [13; 3];
    assert_eq!(xp(&c), 35);
    c.data.abilities[2] = [12; 3];
    // Armor class 0 or better (natural 1, Dex 17 -3): +1.
    c.data.race = "Dwarf".into();
    assert_eq!(xp(&c), 15);
    c.data.abilities[0] = [17; 3];
    assert_eq!(xp(&c), 35);
    c.data.abilities[0] = [10; 3];
    // The second psionic list +2, the third +1.
    c.data.unknown_0x196c[1] = vec!["x".into()];
    assert_eq!(xp(&c), 65);
    c.data.unknown_0x196c[2] = vec!["y".into()];
    assert_eq!(xp(&c), 120);
    c.data.unknown_0x196c = Default::default();
    // Fighter 3/Mage 5: the highest level, +1 for 1st-level slots, +1 for
    // 3rd-level ones.
    c.data.classes.names.push("Mage".into());
    c.data.unknown_0x1618[..2].copy_from_slice(&[4000, 8000]);
    assert_eq!(rules.npc_title(&c), "Dwarf F3/M5");
    assert_eq!(xp(&c), experience_for_hit_dice(7, 0));
    c.data.unknown_0x1618[1] = 6000;
    assert_eq!(xp(&c), experience_for_hit_dice(5, 0));
    // A class the databases lack: worth nothing, titled with level 0.
    c.data.classes.names[1] = "Psionicist".into();
    assert_eq!(xp(&c), 0);
    assert_eq!(rules.npc_title(&c), "Dwarf F3/0");
    c.data.classes.names.truncate(1);
    c.data.unknown_0x1618[1] = 0;

    // The list names a lone NPC with its name, a group without.
    let mut e = Encounter::default();
    e.add_npc(c, Flavor::Expansion).unwrap();
    assert_eq!(
        e.package_line(Package::Npc(0), &rules).unwrap(),
        "1 - Borin Dwarf F3"
    );
    let mut r = Rand::seeded(1);
    e.set_count(Package::Npc(0), 4, &rules, &mut r);
    assert_eq!(rules.npc_name(&e.npcs[0]), "Dwarf F3");
    assert_eq!(
        e.package_line(Package::Npc(0), &rules).unwrap(),
        "4 - Dwarf F3"
    );
    // Totals and the printout count each individual.
    e.monsters.push(
        rules
            .new_monster_package("Goblin", Some(5), &mut r)
            .unwrap(),
    );
    assert_eq!(rules.totals(&e).monster_xp, 5 * 10 + 4 * 65);
    let text = rules.encounter_text(&e);
    assert!(text.contains("         4           260  Borin\n"), "{text}");
    assert!(text.contains("         5           310  TOTAL\n"), "{text}");
    assert!(text.contains("\n4 Borin Dwarf F3 (XP 65)\n"), "{text}");
}

#[test]
fn saving_keeps_a_backup_and_the_programs_apart() {
    let (temp, install) = install("save", goblins(), vec![], false);
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/characters/90688519.dat"
    );
    let character = Character::parse(&fs::read(fixture).unwrap()).unwrap();
    let rules = install.encounter_rules().unwrap();
    let mut r = Rand::seeded(8);
    let mut e = Encounter {
        name: "Goblins".into(),
        ..Encounter::default()
    };
    e.monsters
        .push(rules.new_monster_package("Goblin", None, &mut r).unwrap());
    install.write_saved_encounters(&[e.clone()]).unwrap();
    assert_eq!(install.saved_encounters().unwrap(), [e.clone()]);
    install.write_saved_encounters(&[]).unwrap();
    assert!(temp.0.join("UserDbas/EncountU.dat.bak").exists());
    // An Expansion NPC is never written into a 2.0 installation.
    let mut npc = e.clone();
    npc.npcs.push(NpcPackage::of_character(character));
    assert!(install.write_saved_encounters(&[npc.clone()]).is_err());
    fs::write(temp.0.join("UserDbas/AllowedU.dat"), [0, 0]).unwrap();
    install.write_saved_encounters(&[npc.clone()]).unwrap();
    let bytes = fs::read(temp.0.join("UserDbas/EncountU.dat")).unwrap();
    assert_eq!(parse_encounters(&bytes).unwrap(), [npc.clone()]);
    assert_eq!(encounters_to_bytes(&[npc]).unwrap(), bytes);
}

#[test]
fn texts() {
    assert_eq!(dice_text(Dice::new(3, 6, 10, 5)), "3d6x10+5");
    assert_eq!(dice_text(Dice::new(1, 1, 1, 9)), "10");
    assert_eq!(dice_text(Dice::new(0, 6, 1, 4)), "4");
    assert_eq!(dice_text(Dice::new(2, 4, 1, 0)), "2d4");

    let (_t, mut rules) = rules("text", vec![], true);
    let mut m = monster("Goblin", "Goblinoid", (1, 0, 8), (4, 6));
    m.intelligence = "Low".into();
    m.alignment = "Lawful Evil".into();
    m.armor_class = 6;
    m.unknown_0x3f8 = 100;
    m.unknown_0x3fc = 4;
    m.unknown_0x400 = 100;
    m.unknown_0x404 = 100;
    m.armor_class_notes = vec!["Overall".into(), String::new(), "Back".into()];
    m.movement = 6;
    m.fly_rate = 12;
    m.fly_maneuverability = "B".into();
    m.thac0 = 19;
    m.arsenal.attacks = "1".into();
    m.arsenal.damage = "1-6".into();
    m.arsenal.special_attacks = "  Nil ".into();
    m.unknown_0x47c = [0, 1, 1, 0, 0, 0];
    m.morale = 12;
    m.unknown_0x470 = 10;
    m.experience = 1500;
    let package = cr_store::encounter::MonsterPackage {
        count: 3,
        monster: m,
        hit_points: vec![(5, 2), (3, 1)],
    };
    assert_eq!(
        rules.stat_line(&package),
        "Goblin (Int. Low; AL LE; AC 6 (Overall), 4 (Back); MV 6, Fl 12(B); HD 1; \
         hp 2x5, 3; THAC0 19; #AT 1; Dmg 1-6; SA Nil; SZ S, M; ML Average-Steady (10-12); \
         XP 1,500)"
    );
    rules.set_flavor(Flavor::Core20);
    assert!(!rules.stat_line(&package).contains("SA"));
}

#[test]
fn frequency_tables() {
    let mut common = template("Common", vec![]);
    common.frequency = 65;
    common.hit_dice = 3;
    common.placement = [1, 0, 0];
    common.climates[2] = 1;
    common.climates[10] = 1;
    let mut rare = template("Rare", vec![]);
    rare.frequency = 11;
    rare.hit_dice = 3;
    rare.placement = [0, 0, 1];
    rare.climate_terrain = "Hills".into();
    let mut big = template("Big", vec![]);
    big.frequency = 65;
    big.hit_dice = 30;
    big.placement = [1, 0, 0];
    big.climate_terrain = "Hills".into();
    let (_t, rules) = rules("tables", vec![common, rare, big], true);

    let mut filter = TableFilter {
        hit_dice: (1, 10),
        lair: LairMode::LairOrWandering.raw(),
        climates: [0; 150],
        ..TableFilter::default()
    };
    filter.climates[2] = 1;
    filter.climates[10] = 1;
    let names = |f: &TableFilter| -> Vec<Vec<String>> {
        rules
            .candidates(f)
            .iter()
            .map(|b| b.iter().map(|t| t.name.clone()).collect())
            .collect()
    };
    // Climate and terrain shared: Common; Big is outside the hit dice.
    assert_eq!(names(&filter)[0], ["Common"]);
    assert!(names(&filter)[2].is_empty());
    // The category matches Rare's exactly.
    filter.category = "Hills".into();
    assert_eq!(names(&filter)[2], ["Rare"]);
    filter.lair = LairMode::Wandering.raw();
    assert!(names(&filter)[0].is_empty());

    let mut table = FrequencyTable::new("Hills");
    filter.lair = LairMode::LairOrWandering.raw();
    table.filter = filter;
    let mut r = Rand::seeded(3);
    table.fill(&rules.candidates(&table.filter), &mut r);
    // Every slot of a non-empty category draws once: five common, six
    // rare.
    assert_eq!(r.calls(), 5 + 6);
    assert_eq!(table.slot(2), None);
    assert_eq!(table.slot(5), Some("Rare"));
    assert_eq!(table.slot(11), Some("Common"));
    let roll = roll_2_to_20(&rules, &mut r);
    assert!((2..=20).contains(&roll));
    let i = roll_and_pick(&rules, &table, &mut r).unwrap();
    assert!(!table.slots[i].is_empty());
    let before = r.calls();
    assert_eq!(
        roll_and_pick(&rules, &FrequencyTable::new("x"), &mut r),
        None
    );
    assert_eq!(r.calls(), before);

    let doc = FrequencyTables {
        version: "TSRP3/FreqDoc/0001".into(),
        tables: vec![table],
    };
    let bytes = doc.to_bytes().unwrap();
    assert_eq!(FrequencyTables::parse(&bytes).unwrap(), doc);
}

// ---- goldens ----

/// The encounter golden folder, if `CORERULES_GOLDEN` is it.
fn golden() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("CORERULES_GOLDEN")?);
    dir.join("exp/EncountU_03_t3_orc_bandits.dat")
        .exists()
        .then_some(dir)
}

fn saved(path: &Path, name: &str) -> Encounter {
    parse_encounters(&fs::read(path).unwrap())
        .unwrap()
        .into_iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("{name} in {}", path.display()))
}

#[test]
fn golden_files_round_trip() {
    let Some(dir) = golden() else {
        return;
    };
    let mut files: Vec<PathBuf> = fs::read_dir(dir.join("exp"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "dat"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 4);
    for path in files {
        let bytes = fs::read(&path).unwrap();
        let list = parse_encounters(&bytes).unwrap();
        assert_eq!(
            encounters_to_bytes(&list).unwrap(),
            bytes,
            "{}",
            path.display()
        );
    }
}

#[test]
fn golden_generations() {
    let (Some(dir), Some(install)) = (golden(), std::env::var_os("CORERULES_DIR")) else {
        return;
    };
    let install = Installation::open(Path::new(&install)).unwrap();
    let rules = install.encounter_rules().unwrap();
    let exp = dir.join("exp");
    let (template, seed, file, name) = match rules.flavor() {
        Flavor::Expansion => (
            "Orc, Wandering",
            628,
            "EncountU_03_t3_orc_bandits.dat",
            "T3 Orc Wandering",
        ),
        Flavor::Core20 => (
            "Aboleth, Lair",
            647,
            "EncountU_02_ev_aboleth_lair.dat",
            "EV Aboleth Lair",
        ),
    };
    let mut rand = Rand::seeded(seed);
    let g = rules.generate(template, &mut rand).unwrap();
    assert!(g.pending_npcs.is_empty() && g.messages.is_empty());
    let mut ours = g.encounter;
    ours.name = name.into();
    ours.monster_type = "~All".into();
    let theirs = saved(&exp.join(file), name);
    assert_eq!(names(&ours), names(&theirs));
    for (o, t) in ours.monsters.iter().zip(&theirs.monsters) {
        assert_eq!(o.hit_points, t.hit_points, "{}", o.monster.name);
    }
    assert_eq!(ours.items, theirs.items);
    assert_eq!(ours.treasure, theirs.treasure);
    assert_eq!(ours, theirs);
    assert_eq!(
        encounters_to_bytes(std::slice::from_ref(&ours)).unwrap(),
        encounters_to_bytes(&[theirs]).unwrap()
    );

    if rules.flavor() == Flavor::Expansion {
        // The treasure engine starts after 483 draws with L x223, O x36.
        let counts = ours.treasure_counts();
        assert_eq!((counts[11], counts[14]), (223, 36));
        assert_eq!(counts.iter().sum::<i32>(), 259);
        let mut r = Rand::seeded(seed);
        (0..483).for_each(|_| {
            r.rand();
        });
        let mut bundle = cr_store::treasure::Bundle::default();
        let letters = counts.map(|n| n as u32);
        rules.treasure_rules().roll(
            &mut r,
            &letters,
            cr_store::treasure::Coin::Platinum,
            &mut bundle,
        );
        assert_eq!(bundle, ours.treasure);
        assert_eq!(r.calls(), rand.calls());
        let text = rules.encounter_text(&ours);
        assert!(text.starts_with("AD&D [T3 Orc Wandering]\nTemplate: Orc, Wandering\n"));
    }
}

/// "T3 Bandits" was made in the session of "T3 Orc Wandering" (seed 628)
/// after 1073 draws: the NPC generator replays its packages.
#[test]
fn golden_npc_generation() {
    let (Some(dir), Some(install)) = (golden(), std::env::var_os("CORERULES_DIR")) else {
        return;
    };
    let install = Installation::open(Path::new(&install)).unwrap();
    let rules = install.encounter_rules().unwrap();
    if rules.flavor() != Flavor::Expansion {
        return;
    }
    let theirs = saved(
        &dir.join("exp/EncountU_03_t3_orc_bandits.dat"),
        "T3 Bandits",
    );
    let mut rand = Rand::seeded(628);
    (0..1073).for_each(|_| {
        rand.rand();
    });
    let g = rules.generate("Human, Bandit/Brigand", &mut rand).unwrap();
    assert!(g.pending_npcs.is_empty());
    // Only the note that rogue skill points are not simulated.
    assert!(
        g.messages
            .iter()
            .all(|m| m.starts_with("Rogue skill points")),
        "{:?}",
        g.messages
    );
    let ours = &g.encounter.npcs;
    assert_eq!(ours.len(), theirs.npcs.len());
    for (o, t) in ours.iter().zip(&theirs.npcs) {
        assert_eq!(o.count, t.count);
        assert_eq!(o.individual_hit_points(), t.individual_hit_points());
        let (o, t) = (&o.character.data, &t.character.data);
        assert_eq!(
            (&o.name, &o.class_title, &o.kit, &o.alignment),
            (&t.name, &t.class_title, &t.kit, &t.alignment)
        );
        assert_eq!((o.strength, o.abilities), (t.strength, t.abilities));
        assert_eq!(o.weapon_proficiencies, t.weapon_proficiencies);
        assert_eq!(o.nonweapon_proficiencies, t.nonweapon_proficiencies);
        assert_eq!(o.inventory, t.inventory);
    }
}

/// The NPC packages of "T3 Bandits" as the Expansion's Encounter dialog
/// showed them (`screens/enc_dialog_bandit_npcs.png`): the list's names
/// and Monster XP 1410, a Thief 1, 3 and 5 being worth 15, 65 and 175.
#[test]
fn golden_npc_packages() {
    let (Some(dir), Some(install)) = (golden(), std::env::var_os("CORERULES_DIR")) else {
        return;
    };
    let install = Installation::open(Path::new(&install)).unwrap();
    let rules = install.encounter_rules().unwrap();
    if rules.flavor() != Flavor::Expansion {
        return;
    }
    let e = saved(
        &dir.join("exp/EncountU_03_t3_orc_bandits.dat"),
        "T3 Bandits",
    );
    let lines: Vec<String> = e
        .packages()
        .into_iter()
        .map(|p| e.package_line(p, &rules).unwrap())
        .collect();
    assert_eq!(
        lines,
        ["13 - Human T1", "16 - Human T3", "1 - Erik Human T5"]
    );
    let xp: Vec<i32> = e
        .npcs
        .iter()
        .map(|p| rules.npc_experience(&p.character))
        .collect();
    assert_eq!(xp, [15, 65, 175]);
    assert_eq!(rules.totals(&e).monster_xp, 1410);
}

/// The Gold Value of "T3 Bandits" as the dialog showed it
/// (`screens/enc_dialog_bandit_npcs.png`): 52,870. Its NPCs do not use
/// Skills & Powers, so the Player's Handbook prices count; with the Skills
/// & Powers ones it would be 52,871 (sling bullets cost 5 cp, not 1).
#[test]
fn golden_bandits_gold_value() {
    let (Some(dir), Some(install)) = (golden(), std::env::var_os("CORERULES_DIR")) else {
        return;
    };
    let install = Installation::open(Path::new(&install)).unwrap();
    let rules = install.encounter_rules().unwrap();
    if rules.flavor() != Flavor::Expansion {
        return;
    }
    let e = saved(
        &dir.join("exp/EncountU_03_t3_orc_bandits.dat"),
        "T3 Bandits",
    );
    assert_eq!(e.prices(), Prices::Handbook);
    assert_eq!(rules.totals(&e).gold_value, 52870);
    assert_eq!(
        rules.totals_with(&e, Prices::SkillsAndPowers).gold_value,
        52871
    );
}

/// The Traps dialog's Add and Remove (0x53bb2e, 0x53bded).
#[test]
fn traps_add_and_remove() {
    use cr_formats::traps::Trap;
    use cr_store::encounter::{TrapError, add_trap, remove_trap};
    use cr_store::userdb::UserDb;
    let trap = |name: &str, damage: &str| Trap {
        name: name.into(),
        damage: damage.into(),
        unknown_0x14: String::new(),
        unknown_0x10: String::new(),
        unknown_0xc: String::new(),
        unknown_0x8: String::new(),
        unknown_0x4: String::new(),
    };
    let db = UserDb::new(
        Flavor::Expansion,
        vec![trap("Pit", "1d6"), trap("Dart", "1d3")],
        vec![trap("Pit", "2d6")],
    );
    let mut traps = Vec::new();
    // The user's first.
    add_trap(&mut traps, &db, "Pit").unwrap();
    assert_eq!(traps[0].damage, "2d6");
    assert_eq!(add_trap(&mut traps, &db, "Pit"), Err(TrapError::AlreadyIn));
    assert_eq!(add_trap(&mut traps, &db, "pit"), Err(TrapError::NotFound));
    add_trap(&mut traps, &db, "Dart").unwrap();
    assert_eq!(traps.len(), 2);
    assert_eq!(remove_trap(&mut traps, "dart"), Err(TrapError::NotIn));
    remove_trap(&mut traps, "Pit").unwrap();
    assert_eq!(traps[0].name, "Dart");
}

/// Group Encounter: the NPCs of a party as an encounter, and its printout
/// in both formats, with a real Expansion installation (`CORERULES_DIR`).
#[test]
fn group_encounter_printout() {
    use cr_store::encounter::group_job_title;
    use cr_store::sheet::{Block, SectionContent, SheetBox};

    let Some(dir) = std::env::var_os("CORERULES_DIR") else {
        return;
    };
    let install = Installation::open(Path::new(&dir)).unwrap();
    let rules = install.encounter_rules().unwrap();
    if rules.flavor() != Flavor::Expansion {
        return;
    }
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/characters/90688519.dat"
    );
    let character = Character::parse(&fs::read(fixture).unwrap()).unwrap();
    let name = character.data.name.clone();

    // One package of one per member, no hit points, no name.
    let mut e = Encounter::of_group(vec![character.clone(), character.clone()]);
    assert_eq!(e.npcs.len(), 2);
    assert!(
        e.npcs
            .iter()
            .all(|p| p.count == 1 && p.hit_points.is_empty())
    );
    assert!(e.name.is_empty() && e.monsters.is_empty() && e.treasure.is_empty());
    e.name = "The Company of the Long Road".into();

    let title = group_job_title(&e.name);
    assert!(title.starts_with("Group Encounter ["));
    assert_eq!(title.chars().count(), 31);
    assert_eq!(group_job_title("Few"), "Group Encounter [Few]");

    let boxes = |condensed: bool| -> Vec<SheetBox> {
        let sheet = rules.group_sheet(&e, condensed);
        assert_eq!(sheet.title, title);
        sheet
            .sections
            .into_iter()
            .flat_map(|s| match s.content {
                SectionContent::Columns(c) => c.into_iter().flatten().collect(),
                SectionContent::PageBreak => Vec::new(),
            })
            .collect()
    };
    let full = boxes(false);
    let titles: Vec<&str> = full.iter().map(|b| b.title.as_str()).collect();
    assert_eq!(
        &titles[..4],
        [e.name.as_str(), "Group", "Treasure", "NPC's"]
    );
    // The Group table: a row per NPC, then TOTAL counting the rows.
    let Block::Table(group) = &full[1].body[0] else {
        panic!("Group is a table")
    };
    assert_eq!(group.rows.len(), 4);
    assert_eq!(group.rows[1][2].text, name);
    assert_eq!(group.rows[3][0].text, "2");
    assert_eq!(group.rows[3][2].text, "TOTAL");
    // Each NPC in full, under its name: the titled lines in five columns.
    assert!(full[3].body.is_empty());
    assert_eq!(full.len(), 7, "the treasure list comes last");
    for npc in &full[4..6] {
        assert_eq!(npc.title, name);
        let [Block::Table(t)] = &npc.body[..] else {
            panic!("an NPC is a table")
        };
        assert_eq!(t.columns, 5);
        let heads: Vec<&str> = t
            .rows
            .iter()
            .map(|r| r[0].text.as_str())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(
            heads,
            [
                "        Personal Information:",
                "        Ability Scores:",
                "        Combat:",
                "        Armor:",
                "        Weapons:",
                "        Movement and Encumbrance:",
            ][..heads.len()]
        );
        assert!(heads.len() >= 5);
        assert!(t.rows.iter().all(|r| r.len() == 5 || r[0].colspan == 5));
        let cells: Vec<&str> = t.rows.iter().flatten().map(|c| c.text.as_str()).collect();
        for prefix in [
            "Race: ",
            "Str:",
            "Hit Points:",
            "Armor Class: ",
            "Movement:",
        ] {
            assert!(cells.iter().any(|c| c.starts_with(prefix)), "{prefix}");
        }
    }

    // Condensed: one stat line per NPC, a blank line between them.
    let condensed = boxes(true);
    let Block::Paragraph(text) = &condensed[3].body[0] else {
        panic!("the condensed NPCs are a paragraph")
    };
    let lines: Vec<&str> = text.split("\n\n").collect();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        assert!(line.starts_with(&format!("{name} ")), "{line}");
        assert!(line.ends_with(')'), "{line}");
        let at = |s: &str| line.find(s).unwrap_or_else(|| panic!("{s} in {line}"));
        let order = [
            "(S ", ", D ", ", Ch ", "; AL ", "; AC ", "; MV ", "; hp ", "; THAC0 ", "; XP ",
        ]
        .map(at);
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{line}");
    }
    // A package of several starts with the count, without the name, and
    // lists each individual's hit points.
    let mut package = NpcPackage::of_character(character);
    package.count = 3;
    package.hit_points = e.npcs[0].hit_points.clone();
    for hp in [5, 6, 7] {
        let mut id: cr_formats::parts::PartId = blank();
        id.part_type = hp;
        package.hit_points.push(id);
    }
    let line = rules.npc_stat_line(&package);
    assert!(line.starts_with("3 ") && !line.contains(&name), "{line}");
    assert!(line.contains("; hp 5, 6, 7, ; THAC0 "), "{line}");
}
