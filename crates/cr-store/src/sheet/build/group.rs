//! What the group encounter printout (CEncounterGroupPrinter, body
//! 0x638c1c) prints of each NPC: a few lines of its sheet, or the NPC
//! stat line (0x43b59b) the condensed format and the encounter screens
//! use.

use cr_formats::character::Character;

use super::tables::thousands;
use super::{CHA, CON, Ctx, DEX, INT, STR, WIS, combat, kit_name};
use crate::Result;
use crate::sheet::{Cell, SheetRules, Table};

/// The encumbrance columns of AC and THAC0 (0x600fec, 0x600ee5), None to
/// Excessive.
const AC_PENALTY: [&str; 6] = ["", "", "", "+1", "+3", "+3"];
const THAC0_PENALTY: [&str; 6] = ["", "", "-1", "-2", "-4", "-4"];

/// The full format, under the NPC's name: Personal Information, Ability Scores,
/// Combat, Armor, Weapons and Movement and Encumbrance, each titled in the
/// first column (eight spaces in front, as the original writes them) with
/// its values in the four columns at 2880, 4680, 6480 and 8280 twips, and
/// a blank line after each.
pub(in crate::sheet) fn full(rules: &SheetRules, ch: &Character) -> Result<Table> {
    let ctx = Ctx::new(rules, ch)?;
    let d = &ch.data;
    let multi = ctx.classes.len() > 1;
    let mut rows = Vec::new();
    let blank = || vec![Cell::new("").span(5)];
    let row = |cells: [String; 5]| cells.into_iter().map(Cell::new).collect::<Vec<_>>();
    let none = String::new;

    let race = if d.subrace.is_empty() {
        &d.race
    } else {
        &d.subrace
    };
    let gender = if d.unknown_0x163c != 0 {
        "Male"
    } else {
        "Female"
    };
    let levels = ctx
        .classes
        .iter()
        .map(|c| c.level.to_string())
        .collect::<Vec<_>>()
        .join("/");
    let xp = (0..ctx.classes.len().max(1))
        .map(|g| thousands(i64::from(d.unknown_0x1618[g])))
        .collect::<Vec<_>>()
        .join("/");
    let next = rules
        .advance
        .levels(ch)
        .unwrap_or_default()
        .iter()
        .map(|l| match l.next {
            Some(n) => thousands(i64::from(n)),
            None => "NA".into(),
        })
        .collect::<Vec<_>>()
        .join("/");
    rows.push(row([
        "        Personal Information:".into(),
        format!("Race: {race}"),
        format!("Gender: {gender}"),
        format!("Height: {}'{}\"", d.height / 12, d.height % 12),
        format!("Age: {}", d.age),
    ]));
    rows.push(row([
        none(),
        format!(
            "{}{}",
            if multi { "Classes: " } else { "Class: " },
            d.class_title
        ),
        none(),
        format!("Weight: {} lbs", d.weight),
        format!("Alignment: {}", d.alignment),
    ]));
    rows.push(row([
        none(),
        format!("{}{levels}", if multi { "Levels: " } else { "Level: " }),
        none(),
        format!("XP: {xp}"),
        format!("Kit: {}", kit_name(&ctx).unwrap_or("None")),
    ]));
    rows.push(row([
        none(),
        format!("Next Level: {next}"),
        none(),
        none(),
        none(),
    ]));
    rows.push(blank());

    let s = &ctx.scores;
    rows.push(row([
        "        Ability Scores:".into(),
        format!("Str:{}", s.strength_text(s.score(STR))),
        format!("Con:{}", s.score(CON)),
        format!("Wis:{}", s.score(WIS)),
        none(),
    ]));
    rows.push(row([
        none(),
        format!("Dex:{}", s.score(DEX)),
        format!("Int:{}", s.score(INT)),
        format!("Cha:{}", s.score(CHA)),
        none(),
    ]));
    rows.push(blank());

    let [hp, base, melee, missile] =
        combat::combat_values(&ctx).map(|(label, v)| format!("{label}{v}"));
    rows.push(row(["        Combat:".into(), hp, base, melee, missile]));
    rows.push(blank());

    let (lines, ac) = combat::armor(&ctx);
    // Label and value joined with ": ", the value keeping its own space.
    let line = |l: &String| match l.split_once(':') {
        Some((label, value)) => format!("{label}: {value}"),
        None => l.clone(),
    };
    rows.push(row([
        "        Armor:".into(),
        format!("Armor Class: {ac}"),
        none(),
        lines.first().map(line).unwrap_or_default(),
        none(),
    ]));
    for l in lines.iter().skip(1) {
        rows.push(row([none(), none(), none(), line(l), none()]));
    }
    if !lines.is_empty() {
        rows.push(blank());
    }

    let weapons = combat::weapon_texts(&ctx);
    for (i, [name, melee, missile, small, large]) in weapons.iter().enumerate() {
        let title = if i == 0 { "        Weapons:" } else { "" };
        rows.push(row([
            title.into(),
            name.clone(),
            none(),
            format!("Melee THAC0: {melee}"),
            format!("Dmg Sm-Med: {small}"),
        ]));
        rows.push(row([
            none(),
            none(),
            none(),
            format!("Missile THAC0: {missile}"),
            format!("Dmg Large: {large}"),
        ]));
    }
    if !weapons.is_empty() {
        rows.push(blank());
    }

    let c = combat::encumbrance(&ctx);
    rows.push(row([
        "        Movement and Encumbrance:".into(),
        format!("Movement:{}", combat::current_movement(&ctx, 0)),
        format!("AC:{}", AC_PENALTY[c]),
        format!("THAC0:{}", THAC0_PENALTY[c]),
        none(),
    ]));
    rows.push(blank());
    Ok(Table { columns: 5, rows })
}

/// The NPC stat line (0x43b59b) for a package of `count` NPCs with the
/// hit points `hit_points`: "Borin Dwarf male F3 (S 17, D 12, C 16, I 10,
/// W 9, Ch 8; AL Lawful Good; AC 4; MV 6; hp 27; THAC0 18; XP 175)". As
/// in the original, a package of more than one starts with the count
/// and leaves the name out, and lists each individual's hit points
/// followed by ", ".
pub(in crate::sheet) fn stat_line(
    rules: &SheetRules,
    ch: &Character,
    count: i32,
    hit_points: &[i32],
) -> Result<String> {
    let ctx = Ctx::new(rules, ch)?;
    let d = &ch.data;
    let s = &ctx.scores;
    let gender = if d.unknown_0x163c != 0 {
        "male"
    } else {
        "female"
    };
    let mut line = if count == 1 {
        format!("{} ", d.name)
    } else {
        format!("{count} ")
    };
    line.push_str(&format!(
        "{} {gender} {} (S {}, D {}, C {}, I {}, W {}, Ch {}; AL {}; AC {}; MV {}",
        d.race,
        rules.class_levels_text(ch),
        s.strength_text(s.score(STR)),
        s.score(DEX),
        s.score(CON),
        s.score(INT),
        s.score(WIS),
        s.score(CHA),
        d.alignment,
        combat::armor_class(&ctx),
        combat::current_movement(&ctx, 0),
    ));
    for (kind, label) in [(2, "Fl"), (1, "Sw")] {
        let rate = combat::current_movement(&ctx, kind);
        if rate != 0 {
            line.push_str(&format!(", {label} {rate}"));
        }
    }
    if count == 1 {
        let hp = rules.advance.hit_points(ch).unwrap_or(0);
        line.push_str(&format!("; hp {hp}"));
    } else {
        line.push_str("; hp ");
        for hp in hit_points {
            line.push_str(&format!("{hp}, "));
        }
    }
    let thac0 = combat::combat_values(&ctx)[1].1;
    let xp = rules.npc_experience(ch)?;
    line.push_str(&format!("; THAC0 {thac0}; XP {xp})"));
    Ok(line)
}
