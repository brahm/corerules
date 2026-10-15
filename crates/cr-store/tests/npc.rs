//! The NPC generator against the original's saved NPCs.
//!
//! With `CORERULES_GOLDEN` pointing at the `npc` golden folder (holding
//! `t3/trace_session1.log`) and `CORERULES_DIR` at an installation, each
//! recorded generation is replayed from the `rand()` state its trace shows
//! and compared field by field with what the program saved; the stream
//! must also end where the next traced call starts. An Expansion
//! installation replays `t3/`, a Core Rules 2.0 one `ev/`. Without them
//! (or with another golden folder) the tests pass without checking.

use std::fs;
use std::path::{Path, PathBuf};

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::inventory::InventoryBundle;
use cr_store::Installation;
use cr_store::encounter::{Encounter, parse_encounters};
use cr_store::npc::{Flavor, GroupOptions, Npc, NpcOptions, NpcRules, Rand};

fn golden() -> Option<(PathBuf, Installation)> {
    let dir = PathBuf::from(std::env::var_os("CORERULES_GOLDEN")?);
    if !dir.join("t3/trace_session1.log").exists() {
        return None;
    }
    let install = Installation::open(PathBuf::from(std::env::var_os("CORERULES_DIR")?)).ok()?;
    Some((dir, install))
}

/// The states before each traced `rand()` call, 1-based like the README.
fn trace(path: &Path) -> Vec<u32> {
    let mut out = vec![0];
    for line in fs::read_to_string(path).unwrap().lines() {
        if let Some(rest) = line.strip_prefix("R ") {
            let state = rest.split_whitespace().nth(1).unwrap();
            out.push(u32::from_str_radix(state, 16).unwrap());
        }
    }
    out
}

/// Seeded where line `line` of the trace starts.
fn at(trace: &[u32], line: usize) -> Rand {
    Rand::seeded(trace[line])
}

/// The stream ended where the trace goes on.
fn check_stream(label: &str, trace: &[u32], line: usize, rand: &Rand) {
    let next = line + rand.calls() as usize;
    assert_eq!(
        trace.get(next).copied(),
        Some(rand.state()),
        "{label}: {} draws do not end where trace line {next} starts",
        rand.calls()
    );
}

fn bundle_items(b: &InventoryBundle) -> Vec<String> {
    b.items
        .iter()
        .map(|i| {
            let inner = match i.contents.as_deref() {
                Some(Object::InventoryBundle(b)) => format!(" {:?}", bundle_items(b)),
                _ => String::new(),
            };
            format!("{} x{}{inner}", i.name, i.quantity)
        })
        .collect()
}

/// What the simulator compares, by name.
fn fields(c: &Character) -> Vec<(&'static str, String)> {
    let d = &c.data;
    let n = d.classes.names.len();
    let abilities: Vec<u32> = std::iter::once(d.strength[0])
        .chain(d.abilities.iter().map(|a| a[0]))
        .collect();
    let hp: Vec<Vec<u32>> = d
        .unknown_0xc60
        .iter()
        .filter(|r| r.iter().any(|&x| x != 0))
        .map(|r| r.iter().copied().filter(|&x| x != 0).collect())
        .collect();
    let nwp_langs: Vec<Vec<String>> = d
        .nonweapon_details
        .iter()
        .map(|o| match o {
            Object::StringArray(a) => a.clone(),
            _ => Vec::new(),
        })
        .collect();
    let book_pages: Vec<Vec<u32>> = d.inventory[4]
        .items
        .iter()
        .filter(|i| i.name == "Spellbook")
        .map(|i| {
            i.extra
                .as_ref()
                .map_or(Vec::new(), |e| e.unknown_0x24.clone())
        })
        .collect();
    let carried = f32::from_bits(d.carried_weight);
    vec![
        ("name", d.name.clone()),
        ("race", format!("{} / {}", d.race, d.subrace)),
        ("kit", d.kit.clone()),
        ("alignment", d.alignment.clone()),
        ("social", d.social_class.clone()),
        ("background", d.unknown_0x90.clone()),
        ("abilities", format!("{abilities:?}")),
        ("hwa", format!("{:?}", [d.height, d.weight, d.age])),
        ("nwp", format!("{:?}", d.nonweapon_proficiencies)),
        ("wp", format!("{:?}", d.weapon_proficiencies)),
        ("wp_spec", format!("{:?}", d.unknown_0xac[3])),
        ("hp", format!("{hp:?}")),
        ("languages", format!("{:?}", d.languages)),
        ("nwp_langs", format!("{nwp_langs:?}")),
        (
            "inventory",
            format!(
                "{:?}",
                d.inventory.iter().map(bundle_items).collect::<Vec<_>>()
            ),
        ),
        ("carried", format!("{:.3}", f64::from(carried))),
        ("spells", format!("{:?}", &d.unknown_0x3c8[..n])),
        ("spellbook", format!("{:?}", [&d.spells, &d.priest_spells])),
        ("book_pages", format!("{book_pages:?}")),
        ("psionics", format!("{:?}", d.unknown_0x196c)),
        ("psp", format!("{:?}", &d.unknown_0x1430[..10])),
        ("rogue", format!("{:?}", d.rogue_skills.entries)),
        (
            "rogue_kit",
            format!("{:?} {:?}", d.flag_0x1610, d.unknown_0x15e4),
        ),
    ]
}

fn compare(label: &str, ours: &Character, theirs: &Character) {
    let bad: Vec<String> = fields(theirs)
        .into_iter()
        .zip(fields(ours))
        .filter(|(t, o)| t.1 != o.1)
        .map(|(t, o)| format!("  {}:\n    golden {}\n    ours   {}", t.0, t.1, o.1))
        .collect();
    assert!(bad.is_empty(), "{label} differs:\n{}", bad.join("\n"));
}

fn saved(dir: &Path, file: &str) -> Character {
    Character::parse(&fs::read(dir.join(file)).unwrap()).unwrap()
}

fn classes(list: &[(&str, i32)]) -> Vec<(String, i32)> {
    list.iter().map(|(c, l)| (c.to_string(), *l)).collect()
}

fn rules(install: &Installation) -> NpcRules {
    install.npc_rules().unwrap()
}

/// File, first trace line, race, classes, "Ability Scores…" first.
type Case = (
    &'static str,
    usize,
    &'static str,
    &'static [(&'static str, i32)],
    bool,
);

#[test]
fn golden_single_npcs() {
    let Some((dir, install)) = golden() else {
        return;
    };
    let rules = rules(&install);
    if rules.flavor() != Flavor::Expansion {
        return;
    }
    let t = trace(&dir.join("t3/trace_session1.log"));
    let cases: [Case; 6] = [
        ("01_dwarf_cleric1.dat", 5, "Dwarf", &[("Cleric", 1)], true),
        (
            "02_human_fighter5.dat",
            156,
            "Human",
            &[("Fighter", 5)],
            false,
        ),
        (
            "03_elf_ftm_3_4_2.dat",
            364,
            "Elf",
            &[("Fighter", 3), ("Thief", 4), ("Mage", 2)],
            false,
        ),
        ("04_human_mage7.dat", 679, "Human", &[("Mage", 7)], false),
        (
            "05_halfling_tp_4_3.dat",
            885,
            "Halfling",
            &[("Thief", 4), ("Psionicist", 3)],
            false,
        ),
        (
            "06_human_paladin9.dat",
            1104,
            "Human",
            &[("Paladin", 9)],
            false,
        ),
    ];
    for (file, line, race, cl, prepass) in cases {
        let cl = classes(cl);
        let mut rand = at(&t, line);
        let mut options = NpcOptions::default();
        if prepass {
            // "Ability Scores…" first, then Create NPC keeps them.
            options.abilities = Some(
                rules
                    .roll_abilities_only(&mut rand, race, &cl, &options)
                    .unwrap(),
            );
        }
        let npc = rules.generate(&mut rand, race, &cl, &options).unwrap();
        check_stream(file, &t, line, &rand);
        compare(file, &npc.character, &saved(&dir, &format!("t3/{file}")));
        assert!(npc.messages.is_empty(), "{file}: {:?}", npc.messages);
    }
}

#[test]
fn golden_players_option_falls_back() {
    let Some((dir, install)) = golden() else {
        return;
    };
    let rules = rules(&install);
    if rules.flavor() != Flavor::Expansion {
        return;
    }
    let t = trace(&dir.join("t3/trace_session1.log"));
    let mut rand = at(&t, 1300);
    let options = NpcOptions {
        players_option: true,
        abilities: None,
    };
    let npc = rules
        .generate(&mut rand, "Dwarf", &classes(&[("Fighter", 4)]), &options)
        .unwrap();
    let golden = saved(&dir, "t3/07_dwarf_fighter4_PO.dat");
    // The constructor, name and abilities (with subabilities) are decoded.
    assert_eq!(npc.character.data.name, golden.data.name);
    assert_eq!(npc.character.data.strength, golden.data.strength);
    assert_eq!(npc.character.data.abilities, golden.data.abilities);
    assert!(npc.messages.iter().any(|m| m.contains("Player's Option")));
}

#[test]
fn golden_group() {
    let Some((dir, install)) = golden() else {
        return;
    };
    let rules = rules(&install);
    if rules.flavor() != Flavor::Expansion {
        return;
    }
    let t = trace(&dir.join("t3/trace_session1.log"));
    let mut rand = at(&t, 1606);
    let options = GroupOptions {
        number: (3, 3),
        ..GroupOptions::default()
    };
    let npcs: Vec<Npc> = rules.generate_group(&mut rand, &options).unwrap();
    check_stream("08", &t, 1606, &rand);
    assert_eq!(npcs.len(), 3);
    for (npc, file) in npcs.iter().zip(["57905719", "67905719", "77905719"]) {
        let path = format!("t3/08_group_{file}.dat");
        compare(&path, &npc.character, &saved(&dir, &path));
    }
}

#[test]
fn golden_encounter_packages() {
    let Some((dir, install)) = golden() else {
        return;
    };
    let encounter_rules = install.encounter_rules().unwrap();
    if encounter_rules.flavor() != Flavor::Expansion {
        return;
    }
    let t = trace(&dir.join("t3/trace_session1.log"));
    let list: Vec<Encounter> =
        parse_encounters(&fs::read(dir.join("t3/09_EncountU_bandits_adventurers.dat")).unwrap())
            .unwrap();
    for (template, line, theirs) in [
        ("Human, Bandit/Brigand", 2121, &list[3]),
        ("Human, Adventurer Group, Levels 1-3", 2590, &list[4]),
    ] {
        let mut rand = at(&t, line);
        let g = encounter_rules.generate(template, &mut rand).unwrap();
        check_stream(template, &t, line, &rand);
        assert!(
            g.pending_npcs.is_empty(),
            "{template}: {:?}",
            g.pending_npcs
        );
        let ours = &g.encounter.npcs;
        assert_eq!(ours.len(), theirs.npcs.len(), "{template}: packages");
        for (i, (o, t)) in ours.iter().zip(&theirs.npcs).enumerate() {
            let label = format!("{template} [{i}]");
            assert_eq!(o.count, t.count, "{label}: count");
            assert_eq!(
                o.individual_hit_points(),
                t.individual_hit_points(),
                "{label}: hit points"
            );
            compare(&label, &o.character, &t.character);
        }
    }
}

#[test]
fn golden_saved_npc_round_trips() {
    let Some((_, install)) = golden() else {
        return;
    };
    let rules = rules(&install);
    let mut rand = Rand::seeded(7);
    let npc = rules
        .generate(
            &mut rand,
            "Elf",
            &classes(&[("Fighter", 3), ("Mage", 3)]),
            &NpcOptions::default(),
        )
        .unwrap();
    let bytes = npc.character.to_bytes().unwrap();
    assert_eq!(Character::parse(&bytes).unwrap(), npc.character);
}

#[test]
fn golden_evermore_prefix() {
    let Some((dir, install)) = golden() else {
        return;
    };
    let rules = rules(&install);
    if rules.flavor() != Flavor::Core20 {
        return;
    }
    let t = trace(&dir.join("ev/trace_session1.log"));
    for (file, line, race, cl, check_name) in [
        (
            "01_dwarf_ft_2_3.dat",
            5,
            "Dwarf",
            &[("Fighter", 2), ("Thief", 3)][..],
            true,
        ),
        // Evermore's human name table differs from the Expansion's.
        (
            "02_human_mage5.dat",
            105,
            "Human",
            &[("Mage", 5)][..],
            false,
        ),
    ] {
        let mut rand = at(&t, line);
        let npc = rules
            .generate(&mut rand, race, &classes(cl), &NpcOptions::default())
            .unwrap();
        let golden = saved(&dir, &format!("ev/{file}"));
        if check_name {
            assert_eq!(npc.character.data.name, golden.data.name, "{file}");
        }
        assert_eq!(npc.character.data.strength[0], golden.data.strength[0]);
        let ours: Vec<u32> = npc.character.data.abilities.iter().map(|a| a[0]).collect();
        let theirs: Vec<u32> = golden.data.abilities.iter().map(|a| a[0]).collect();
        assert_eq!(ours, theirs, "{file}");
        assert!(npc.messages.iter().any(|m| m.contains("Core Rules 2.0")));
    }
}
