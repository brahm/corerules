//! Spells, spheres and psionics.

use cr_formats::spells::Spell;

use super::{ClassInfo, Ctx, INT, WIS, single, table};
use crate::sheet::tables::{self, at, ordinal};
use crate::sheet::{Block, Cell, ListItem, Section, ids};

/// Whether a class casts priest spells and wizard spells.
pub(in crate::sheet) fn casts(ctx: &Ctx, c: &ClassInfo) -> (bool, bool) {
    ctx.rules.advance.casts(c.name)
}

/// The spell slots of class `c` (index into the classes), when it casts.
pub(in crate::sheet) fn slots(ctx: &Ctx, g: usize) -> Option<[u32; 9]> {
    ctx.rules.advance.spell_slots(ctx.ch, g).ok().flatten()
}

/// The highest spell level with a slot.
fn max_level(ctx: &Ctx, g: usize) -> usize {
    slots(ctx, g).map_or(0, |s| s.iter().rposition(|&n| n > 0).map_or(0, |i| i + 1))
}

/// Spells Summary (0x5a753c).
pub(in crate::sheet) fn summary(ctx: &Ctx) -> Section {
    let mut header = vec![Cell::header("Class")];
    header.extend((1..=9).map(|l| Cell::header(ordinal(l)).center()));
    let mut rows = vec![header];
    for (g, c) in ctx.classes.iter().enumerate() {
        if let Some(s) = slots(ctx, g) {
            let mut row = vec![Cell::new(c.name)];
            row.extend(
                s.iter().map(|&n| {
                    Cell::new(if n == 0 { String::new() } else { n.to_string() }).center()
                }),
            );
            rows.push(row);
        }
    }
    // 2.0 has no failure, maximum and learning rows.
    if !ctx.expansion() {
        return single(ids::SPELLS, "Spells", vec![table(10, rows)]);
    }
    let s = &ctx.scores;
    let reason = s.sub(INT, 1);
    let max_spells = match at(&tables::MAXIMUM_SPELLS, reason) {
        i32::MAX => "All".to_string(),
        n => n.to_string(),
    };
    // Unlike the Ability Scores box, this prints 0 as "0th".
    let max_level = ordinal(at(&tables::MAXIMUM_SPELL_LEVEL, reason));
    rows.push(vec![
        Cell::new(format!(
            "Cleric Spell Failure Rate: {}%",
            at(&tables::SPELL_FAILURE, s.sub(WIS, 1))
        ))
        .span(5),
        Cell::new(format!("Maximum Wizard Spells Per Level: {max_spells}")).span(5),
    ]);
    rows.push(vec![
        Cell::new(format!(
            "Wizard Chance to Learn New Spell: {}%",
            at(&tables::LEARN_SPELL, s.sub(INT, 2))
        ))
        .span(5),
        Cell::new(format!("Maximum Wizard Spell Level: {max_level}")).span(5),
    ]);
    single(ids::SPELLS, "Spells", vec![table(10, rows)])
}

/// The memorized spells of class `g` at slot level `l` (1 to 9): its
/// wizard and priest lists.
fn memorized_at<'a>(ctx: &Ctx<'a>, g: usize, l: usize) -> Vec<&'a String> {
    let (wizard, priest) = &ctx.ch.data.unknown_0x3c8[g][l];
    wizard.iter().chain(priest).collect()
}

/// How many spells the character has memorized.
pub(in crate::sheet) fn memorized_count(ctx: &Ctx) -> usize {
    (0..ctx.classes.len())
        .flat_map(|g| (0..11).map(move |l| (g, l)))
        .map(|(g, l)| memorized_at(ctx, g, l).len())
        .sum()
}

/// Spells Memorized (0x5a78f2): per class with memorized spells, each
/// level up to the highest one used, then its spells (`name xN`).
pub(in crate::sheet) fn memorized(ctx: &Ctx) -> Section {
    let mut classes = Vec::new();
    for (g, c) in ctx.classes.iter().enumerate() {
        let highest = (0..11).rev().find(|&l| !memorized_at(ctx, g, l).is_empty());
        let Some(highest) = highest else {
            continue;
        };
        let mut levels = Vec::new();
        for l in 1..=highest {
            let mut spells: Vec<(String, usize)> = Vec::new();
            let mut names = memorized_at(ctx, g, l);
            names.sort();
            for name in names {
                match spells.iter_mut().find(|(n, _)| n == name) {
                    Some((_, count)) => *count += 1,
                    None => spells.push((name.clone(), 1)),
                }
            }
            levels.push(ListItem {
                text: format!("{} Level", ordinal(l as i32)),
                children: Some(
                    spells
                        .into_iter()
                        .map(|(name, count)| ListItem {
                            text: if count > 1 {
                                format!("{name} x{count}")
                            } else {
                                name
                            },
                            children: None,
                        })
                        .collect(),
                ),
            });
        }
        classes.push(ListItem {
            text: c.name.to_string(),
            children: Some(levels),
        });
    }
    single(
        ids::MEMORIZED,
        "Spells Memorized",
        vec![Block::List(classes)],
    )
}

/// A table of spells by level: a heading per level, then a row per spell.
fn spell_table(spells: &[&Spell]) -> Block {
    let mut rows = vec![
        ["Spell", "Time", "Range", "Area of Effect", "Components"]
            .iter()
            .map(|h| Cell::header(*h))
            .collect::<Vec<_>>(),
    ];
    let mut level = None;
    for spell in spells {
        if level != Some(spell.level) {
            level = Some(spell.level);
            rows.push(vec![
                Cell::header(format!("{} Level", ordinal(spell.level))).span(5),
            ]);
        }
        rows.push(vec![
            Cell::new(spell.name.clone()),
            Cell::new(spell.casting_time.clone()),
            Cell::new(spell.range.clone()),
            Cell::new(spell.area_of_effect.clone()),
            Cell::new(spell.components.clone()),
        ]);
    }
    table(5, rows)
}

/// Spells Known (0x5a940c): the wizard spells the character knows, absent
/// without a wizard class or spells.
pub(in crate::sheet) fn known(ctx: &Ctx) -> Option<Section> {
    if !ctx.classes.iter().any(|c| casts(ctx, c).1) || ctx.ch.data.spells.is_empty() {
        return None;
    }
    let mut spells: Vec<&Spell> = ctx
        .ch
        .data
        .spells
        .iter()
        .filter_map(|n| ctx.rules.wizard_spells.iter().find(|s| &s.name == n))
        .collect();
    spells.sort_by(|a, b| a.level.cmp(&b.level).then_with(|| a.name.cmp(&b.name)));
    Some(single(
        ids::SPELLS_KNOWN,
        "Spells Known",
        vec![spell_table(&spells)],
    ))
}

/// Whether a sphere the character has (`"Elemental, All"` covers every
/// elemental sphere) grants a spell sphere.
fn sphere_matches(have: &str, spell: &str) -> bool {
    if have == spell {
        return true;
    }
    match have.split_once(", ") {
        Some((family, "All")) => spell.starts_with(family),
        _ => false,
    }
}

/// Spells Granted (0x5a9874): the priest spells of the first priest
/// class's spheres (minor spheres up to 3rd level), up to its highest
/// level, absent without a priest class.
pub(in crate::sheet) fn granted(ctx: &Ctx) -> Option<Section> {
    let g = ctx.classes.iter().position(|c| casts(ctx, c).0)?;
    let slot = &ctx.ch.data.class_slots[g];
    let max = max_level(ctx, g) as i32;
    let mut spells: Vec<&Spell> = ctx
        .rules
        .priest_spells
        .iter()
        .filter(|s| (1..=max).contains(&s.level))
        .filter(|s| {
            s.spheres.iter().any(|sphere| {
                slot.unknown_0x338.iter().any(|h| sphere_matches(h, sphere))
                    || (s.level <= 3
                        && slot.unknown_0x2d4.iter().any(|h| sphere_matches(h, sphere)))
            })
        })
        .collect();
    spells.sort_by(|a, b| a.level.cmp(&b.level).then_with(|| a.name.cmp(&b.name)));
    let title = format!("{} Spells Granted", ctx.classes[g].name);
    Some(single(
        ids::SPELLS_GRANTED,
        &title,
        vec![spell_table(&spells)],
    ))
}

/// The spheres and schools lines of a class in the Class Abilities box.
pub(in crate::sheet) fn spheres_text(ctx: &Ctx, g: usize) -> String {
    let slot = &ctx.ch.data.class_slots[g];
    let mut text = String::new();
    for (label, list) in [
        ("Minor Spheres of Magic", &slot.unknown_0x2d4),
        ("Major Spheres of Magic", &slot.unknown_0x338),
        ("Schools of Magic", &slot.unknown_0x270),
    ] {
        if !list.is_empty() {
            text.push_str(&format!("\t{label} - {}\n", list.join(", ")));
        }
    }
    text
}

/// The MTHAC0 (0x43cdcb): a psionicist's level table, or a slower one
/// for the highest level of any other character, less an Intelligence
/// adjustment. 2.0 (0x420e20) has no Intelligence adjustment.
fn mthac0(ctx: &Ctx) -> i32 {
    let psionicist = ctx.classes.iter().find(|c| c.group == 4);
    let (table, level): (Vec<i32>, u32) = match psionicist {
        Some(c) => (
            (0..31)
                .map(|i: i32| if i < 2 { 20 } else { 21 - i })
                .collect(),
            c.level,
        ),
        None => (
            (0..31)
                .map(|i: i32| if i < 3 { 20 } else { 20 - (i - 1) / 2 })
                .collect(),
            ctx.classes.iter().map(|c| c.level).max().unwrap_or(0),
        ),
    };
    let adjustment = match ctx.scores.score(INT) {
        _ if !ctx.expansion() => 0,
        ..=15 => 0,
        16 | 17 => -1,
        18 | 19 => -2,
        20..=22 => -3,
        _ => -4,
    };
    table[(level as usize).min(30)] + adjustment
}

/// The MAC (0x43cd3e): 10, or 25 less Wisdom above 15, plus the
/// Intelligence adjustment; psionicists 2 better, except in 2.0
/// (0x420e00).
fn mac(ctx: &Ctx) -> i32 {
    let wisdom = ctx.scores.sub(WIS, 2);
    let base = if wisdom <= 15 { 10 } else { 25 - wisdom };
    let int = ctx.scores.sub(INT, 1);
    let adjustment = if int <= 15 {
        0
    } else {
        [-1, -1, -2, -2, -3, -3, -3, -4, -4, -4][(int - 16).min(9) as usize]
    };
    let mut mac = base + adjustment;
    if ctx.expansion() && ctx.has_group(4) {
        mac -= 2;
    }
    mac
}

/// Devotions, sciences and disciplines a psionicist of `level` knows
/// (0x42fa1a, 0x42f835, 0x42f6ea).
fn psionic_powers(level: u32) -> (u32, u32, u32) {
    const DEVOTIONS: [u32; 21] = [
        0, 3, 5, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    ];
    const SCIENCES: [u32; 21] = [
        0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    ];
    const DISCIPLINES: [u32; 16] = [0, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5];
    let l = level as usize;
    let devotions = if l > 20 {
        25 + (l as u32 - 20)
    } else {
        DEVOTIONS[l]
    };
    let sciences = if l > 20 {
        10 + (l as u32 - 19) / 2
    } else {
        SCIENCES[l]
    };
    let disciplines = if l > 15 { 5 } else { DISCIPLINES[l] };
    (devotions, sciences, disciplines)
}

/// Psionics (0x5a8b41).
pub(in crate::sheet) fn psionics(ctx: &Ctx) -> Section {
    let psps: u64 = ctx
        .ch
        .data
        .unknown_0x1430
        .iter()
        .map(|&p| u64::from(p))
        .sum();
    let mut rows = vec![vec![
        Cell::new(format!("PSPs: {psps}")),
        Cell::new(format!("MTHAC0: {}", mthac0(ctx))).span(2),
        Cell::new(format!("MAC: {}", mac(ctx))).span(2),
    ]];
    if let Some(c) = ctx.classes.iter().find(|c| c.group == 4) {
        let (devotions, sciences, disciplines) = psionic_powers(c.level);
        rows.push(vec![
            Cell::new(format!("Devotions: {devotions}")),
            Cell::new(format!("Sciences: {sciences}")).span(2),
            Cell::new(format!("Disciplines: {disciplines}")).span(2),
        ]);
    }
    // 2.0 has no attack and defense mode rows.
    if ctx.expansion() {
        rows.push(vec![Cell::new("Attacks: None").span(5)]);
        rows.push(vec![Cell::new("Defenses: None").span(5)]);
    }
    rows.push(
        ["Power", "PSPs", "Range", "Area of Effect", "MAC"]
            .iter()
            .map(|h| Cell::header(*h))
            .collect(),
    );
    rows.extend(power_rows(ctx));
    single(ids::PSIONICS, "Psionics", vec![table(5, rows)])
}

/// The discipline a power belongs to (0x4ed938), by `unknown_0x28`;
/// metapsionic powers (5) have none.
fn discipline(p: &cr_formats::psionic::PsionicPower) -> &'static str {
    match p.unknown_0x28 {
        0 => "Clairsentient",
        1 => "Psychokinetic",
        2 => "Psychometabolic",
        3 => "Psychoportive",
        4 => "Telepathic",
        _ => "",
    }
}

/// A range or area as the sheet shows it: the first letter in capitals.
fn capitalized(text: &str) -> String {
    let mut c = text.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// The discipline rows (0x5ffbd2): per discipline, primary ones marked,
/// its sciences then its devotions (0x5a92dd); without disciplines, one
/// "Wild Talent" with every power.
fn power_rows(ctx: &Ctx) -> Vec<Vec<Cell>> {
    let d = &ctx.ch.data;
    let [disciplines, sciences, devotions] = &d.unknown_0x196c;
    let primary = &d.unknown_0x1980;
    let known = |names: &[String], disc: Option<&str>| -> Vec<String> {
        names
            .iter()
            .filter(|n| {
                ctx.rules
                    .power(n)
                    .is_some_and(|p| disc.is_none_or(|d| discipline(p) == d))
            })
            .cloned()
            .collect()
    };
    let mut groups: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
    if disciplines.is_empty() {
        groups.push((
            "Wild Talent".into(),
            known(sciences, None),
            known(devotions, None),
        ));
    } else {
        for disc in disciplines {
            let title = if primary.iter().any(|p| p == disc) {
                format!("{disc} (Primary)")
            } else {
                disc.clone()
            };
            groups.push((
                title,
                known(sciences, Some(disc)),
                known(devotions, Some(disc)),
            ));
        }
    }
    let mut rows = Vec::new();
    for (title, sciences, devotions) in groups {
        rows.push(vec![Cell::header(title).span(5)]);
        for (label, list) in [("Sciences", sciences), ("Devotions", devotions)] {
            if list.is_empty() {
                continue;
            }
            rows.push(vec![Cell::header(label).span(5)]);
            for name in list {
                let p = ctx.rules.power(&name).expect("known power");
                let score = if p.unknown_0x38 != 0 {
                    format!("{} (base)", p.unknown_0x30)
                } else {
                    p.unknown_0x30.to_string()
                };
                rows.push(vec![
                    Cell::new(name.clone()),
                    Cell::new(p.cost.clone()),
                    Cell::new(capitalized(&p.range)),
                    Cell::new(capitalized(&p.area)),
                    Cell::new(score),
                ]);
            }
        }
    }
    rows
}
