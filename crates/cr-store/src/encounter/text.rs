//! What the encounter screens show and print: the monster stat line
//! (0x640d23, 2.0 0x4bfc00), the dice and hit point texts, the encounter
//! printout (CEncounterPrinter 0x64adf4) and the template printout
//! (CEncTemplatePrinter 0x64d1f6). The printouts were read from the code
//! only (Wine has no printer), so their layout here is plain text.

use cr_formats::Object;
use cr_formats::monster::Monster;

use super::{
    Dice, Encounter, EncounterRules, Flavor, MonsterLine, MonsterPackage, NpcPackage, Template,
    coin_name, npc_inventory,
};
use crate::treasure::{Bundle, LETTERS, SUMMARY_TYPE_NAMES, thousands};

/// A dice text of the template printer (0x554ff2): "3d6x10+5"; the
/// bonus alone when a part is 0, "N·M+B" for one-sided dice.
pub fn dice_text(d: Dice) -> String {
    if d.count == 0 || d.sides == 0 || d.multiplier == 0 {
        return d.bonus.to_string();
    }
    if d.sides == 1 {
        return d
            .count
            .wrapping_mul(d.multiplier)
            .wrapping_add(d.bonus)
            .to_string();
    }
    let mut s = format!("{}d{}", d.count, d.sides);
    if d.multiplier != 1 {
        s.push_str(&format!("x{}", d.multiplier));
    }
    if d.bonus != 0 {
        s.push_str(&format!("+{}", d.bonus));
    }
    s
}

/// The hit points of a package (0x64093a): "3x40, 3x31, 42".
pub fn hit_point_text(hit_points: &[(i32, i32)]) -> String {
    hit_points
        .iter()
        .map(|&(hp, n)| {
            if n == 1 {
                hp.to_string()
            } else {
                format!("{n}x{hp}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

const ALIGNMENTS: [(&str, &str); 9] = [
    ("Lawful Good", "LG"),
    ("Lawful Neutral", "LN"),
    ("Lawful Evil", "LE"),
    ("Neutral Good", "NG"),
    ("True Neutral", "N"),
    ("Neutral Evil", "NE"),
    ("Chaotic Good", "CG"),
    ("Chaotic Neutral", "CN"),
    ("Chaotic Evil", "CE"),
];

/// Armor class with its qualifiers (0x67fea5): "4 (Overall), 2
/// (Underside)".
fn armor_class(m: &Monster) -> String {
    let q = &m.armor_class_notes;
    let note = |i: usize, s: &mut String| {
        if let Some(n) = q.get(i)
            && !n.is_empty()
        {
            s.push_str(&format!(" ({n})"));
        }
    };
    let other = |ac: i32, s: &mut String| {
        if ac != 100 {
            s.push_str(&format!(", {ac}"));
        }
    };
    let mut s = m.armor_class.to_string();
    note(0, &mut s);
    other(m.unknown_0x3f8, &mut s);
    note(1, &mut s);
    other(m.unknown_0x3fc, &mut s);
    note(2, &mut s);
    if q.len() > 3 {
        other(m.unknown_0x400, &mut s);
    }
    note(3, &mut s);
    other(m.unknown_0x404, &mut s);
    note(4, &mut s);
    s
}

/// Movement (0x67f5ca): "9, Fl 18(B), Sw 6".
fn movement(m: &Monster) -> String {
    let mut s = m.movement.to_string();
    if m.fly_rate != 0 {
        s.push_str(&format!(", Fl {}({})", m.fly_rate, m.fly_maneuverability));
    }
    for (rate, name) in [
        (m.swim_rate, "Sw"),
        (m.burrow_rate, "Br"),
        (m.unknown_0x428, "Cl"),
        (m.unknown_0x430, "Wb"),
    ] {
        if rate != 0 {
            s.push_str(&format!(", {name} {rate}"));
        }
    }
    s
}

/// Hit dice (0x67ee9c): "8", "4+1", "1d4", "2d10+2".
pub fn hit_dice_text(m: &Monster) -> String {
    let (dice, bonus, sides) = (m.hit_dice, m.unknown_0x438, m.hit_die_size);
    match (sides == 8, bonus == 0) {
        (true, true) => dice.to_string(),
        (true, false) => format!("{dice}+{bonus}"),
        (false, true) => format!("{dice}d{sides}"),
        (false, false) => format!("{dice}d{sides}+{bonus}"),
    }
}

/// Sizes (0x680548): T, S, M, L, H, G joined with ", ", where small
/// after tiny reads "Sm".
fn sizes(m: &Monster) -> String {
    const NAMES: [(&str, &str); 6] = [
        ("T", "T"),
        ("S", ", Sm"),
        ("M", ", M"),
        ("L", ", L"),
        ("H", ", H"),
        ("G", ", G"),
    ];
    let mut s = String::new();
    for (&flag, (first, later)) in m.unknown_0x47c.iter().zip(NAMES) {
        if flag != 0 {
            s.push_str(if s.is_empty() { first } else { later });
        }
    }
    s
}

fn morale_name(n: i32) -> &'static str {
    match n {
        ..5 => "Unreliable",
        5..8 => "Unsteady",
        8..11 => "Average",
        11..13 => "Steady",
        13..15 => "Elite",
        15..17 => "Champion",
        17..19 => "Fanatic",
        _ => "Fearless",
    }
}

/// Morale (0x67fd7b): "Elite (13)", "Steady (11-12)", "Average-Steady
/// (8-12)".
fn morale(m: &Monster) -> String {
    let (low, high) = (m.unknown_0x470, m.morale);
    if low == high {
        return format!("{} ({low})", morale_name(low));
    }
    let (a, b) = (morale_name(low), morale_name(high));
    if a == b {
        format!("{a} ({low}-{high})")
    } else {
        format!("{a}-{b} ({low}-{high})")
    }
}

fn trimmed(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
}

/// The stat line of a monster package (0x640d23). Core Rules 2.0 leaves
/// out the special attacks and defenses.
pub fn stat_line(package: &MonsterPackage, flavor: Flavor) -> String {
    let m = &package.monster;
    let alignment = ALIGNMENTS
        .iter()
        .find(|(name, _)| *name == m.alignment)
        .map_or("", |(_, abbr)| abbr);
    let mut s = format!(
        "{} (Int. {}; AL {alignment}; AC {}; MV {}; HD {}; hp {}; THAC0 {}; #AT {}; Dmg {}",
        m.name,
        m.intelligence,
        armor_class(m),
        movement(m),
        hit_dice_text(m),
        hit_point_text(&package.hit_points),
        thousands(m.thac0),
        m.arsenal.attacks,
        m.arsenal.damage,
    );
    if flavor == Flavor::Expansion {
        for (label, text) in [
            ("SA", &m.arsenal.special_attacks),
            ("SD", &m.arsenal.special_defenses),
        ] {
            let text = trimmed(text);
            if !text.is_empty() {
                s.push_str(&format!("; {label} {text}"));
            }
        }
    }
    s.push_str(&format!(
        "; SZ {}; ML {}; XP {}",
        sizes(m),
        morale(m),
        thousands(m.experience)
    ));
    let spells: Vec<&str> = m
        .spells
        .iter()
        .filter_map(|o| match o {
            Object::Spell(spell) => Some(spell.name.as_str()),
            _ => None,
        })
        .collect();
    if !spells.is_empty() {
        s.push_str("; Memorized: ");
        s.push_str(&spells.join(", "));
    }
    s.push(')');
    s
}

/// A short line for an NPC package: number, name, race, class and the hit
/// points of each individual. (The original's NPC stat line, 0x43b59b,
/// needs the character sheet's values and is not ported.)
pub fn npc_line(package: &NpcPackage) -> String {
    let d = &package.character.data;
    let mut s = format!("{} {} {} {}", package.count, d.name, d.race, d.class_title);
    let hp = package.individual_hit_points();
    if !hp.is_empty() {
        let hp: Vec<String> = hp.iter().map(i32::to_string).collect();
        s.push_str(&format!(" (hp {})", hp.join(", ")));
    }
    s
}

/// The rows of a printout table: two right-aligned numbers and a name.
fn table(rows: &[[String; 3]]) -> String {
    rows.iter()
        .map(|[a, b, c]| format!("{a:>10}  {b:>12}  {c}\n"))
        .collect()
}

impl EncounterRules {
    /// The stat line of a monster package as this installation's program
    /// shows it.
    pub fn stat_line(&self, package: &MonsterPackage) -> String {
        stat_line(package, self.flavor)
    }

    /// Everything the encounter holds, merged as the printer merges it:
    /// treasure, items, then what each monster and NPC carries, once per
    /// individual.
    pub fn merged_treasure(&self, encounter: &Encounter) -> Bundle {
        let mut all = encounter.items.clone();
        let mut add = |bundle: &Bundle, times: i32| {
            for _ in 0..times.max(0) {
                for item in &bundle.items {
                    all.insert(item.clone(), true);
                }
            }
        };
        add(&encounter.treasure, 1);
        for p in &encounter.monsters {
            if let Ok(b) = Bundle::from_inventory(&p.monster.inventory)
                && !b.is_empty()
            {
                add(&b, p.count);
            }
        }
        for p in &encounter.npcs {
            let b = npc_inventory(&p.character);
            if !b.is_empty() {
                add(&b, p.count);
            }
        }
        all
    }

    /// The printout of an encounter (View / Print, 0x64adf4): title,
    /// template and type, the monsters with their experience, the
    /// treasure by category, the description, the stat lines, the NPCs,
    /// the traps and the treasure itself.
    pub fn encounter_text(&self, encounter: &Encounter) -> String {
        let mut out = format!(
            "AD&D [{}]\nTemplate: {}\nType: {}\n\n",
            encounter.name, encounter.template, encounter.monster_type
        );
        let mut rows = vec![[
            "Number".to_string(),
            "Experience".to_string(),
            "Monster".to_string(),
        ]];
        let (mut number, mut xp) = (0i32, 0i32);
        for p in &encounter.monsters {
            let x = p.monster.experience.wrapping_mul(p.count);
            number = number.wrapping_add(p.count);
            xp = xp.wrapping_add(x);
            rows.push([thousands(p.count), thousands(x), p.monster.name.clone()]);
        }
        for p in &encounter.npcs {
            rows.push([
                thousands(p.count),
                "0".into(),
                p.character.data.name.clone(),
            ]);
        }
        rows.push([thousands(number), thousands(xp), "TOTAL".into()]);
        out.push_str(&table(&rows));
        out.push('\n');

        let merged = self.merged_treasure(encounter);
        let summary = self.treasure.summary(&merged);
        out.push_str(&format!(
            "{:>10}  {:>12}  {:>12}  Category\n",
            "Number", "Experience", "Gold Value"
        ));
        let mut count = 0i32;
        for (totals, name) in summary.by_type.iter().zip(SUMMARY_TYPE_NAMES) {
            if totals.value_cp == 0 && totals.xp == 0 && totals.count == 0 {
                continue;
            }
            count = count.wrapping_add(totals.count);
            out.push_str(&format!(
                "{:>10}  {:>12}  {:>12}  {}\n",
                thousands(totals.count),
                thousands(totals.xp),
                thousands(totals.value_cp / 100),
                name
            ));
        }
        out.push_str(&format!(
            "{:>10}  {:>12}  {:>12}  TOTAL\n",
            thousands(count),
            thousands(summary.xp),
            thousands(summary.value_cp / 100)
        ));

        if !encounter.description.is_empty() {
            out.push_str(&format!("\nDescription\n{}\n", encounter.description));
        }
        if !encounter.monsters.is_empty() {
            out.push_str("\nMonsters\n");
            let lines: Vec<String> = encounter
                .monsters
                .iter()
                .map(|p| self.stat_line(p))
                .collect();
            out.push_str(&lines.join("\n\n"));
            out.push('\n');
        }
        if !encounter.npcs.is_empty() {
            out.push_str("\nNon-Players Characters\n");
            for p in &encounter.npcs {
                out.push_str(&npc_line(p));
                out.push('\n');
            }
        }
        if !encounter.traps.is_empty() {
            out.push_str("\nTraps\n");
            for t in &encounter.traps {
                out.push_str(&format!("{}\n", t.name));
                for (label, value) in [
                    ("Damage", &t.damage),
                    ("Detection Modifier", &t.unknown_0x14),
                    ("Number of Characters Trapped", &t.unknown_0x10),
                    ("Saving Throw", &t.unknown_0xc),
                    ("Special Effects", &t.unknown_0x8),
                    ("Location", &t.unknown_0x4),
                ] {
                    out.push_str(&format!("  {label}: {value}\n"));
                }
            }
        }
        if !merged.is_empty() {
            out.push_str("\nTreasure\n");
            out.push_str(&merged.to_text());
        }
        out
    }

    /// The printout of a template (CEncTemplatePrinter 0x64d1f6).
    pub fn template_text(&self, template: &Template) -> String {
        let mut out = format!("{}\nType: {}\n\n", template.name, template.monster_type);
        for line in &template.monsters {
            out.push_str(&monster_line_text(line));
            out.push('\n');
        }
        for line in &template.npcs {
            let mut s = format!("There is a {}% chance that\n", line.percent);
            if line.dependent {
                s.push_str(&dependency_text(
                    line.per,
                    line.beyond,
                    line.reference.name(),
                ));
            }
            let npc = |n: &super::NpcDescription| {
                let classes: Vec<String> =
                    n.classes.iter().map(|(c, l)| format!("{c} {l}")).collect();
                format!("{} {}", n.race, classes.join("/"))
            };
            if line.list.len() < 2 {
                s.push_str(&format!(
                    "  {} {} appear.\n",
                    dice_text(line.dice),
                    npc(&line.npc)
                ));
            } else {
                s.push_str(&format!(
                    "  {} selected from the following:\n",
                    dice_text(line.dice)
                ));
                for n in &line.list {
                    s.push_str(&format!("    {}\n", npc(n)));
                }
            }
            if line.and || line.or {
                s.push_str(&format!(
                    "  {} {} {}\n",
                    if line.and { "and" } else { "or" },
                    dice_text(line.second_dice),
                    npc(&line.second_npc)
                ));
            }
            out.push_str(&s);
            out.push('\n');
        }
        let lair: Vec<String> = template
            .lair_treasure
            .iter()
            .zip(LETTERS)
            .filter(|(n, _)| **n != 0)
            .map(|(n, c)| format!("{n}x{c}"))
            .collect();
        out.push_str(&format!(
            "Lair Treasure: {}\n",
            if lair.is_empty() {
                "None".to_string()
            } else {
                lair.join(", ")
            }
        ));
        for item in &template.items {
            out.push_str(&format!("\nThere is a {}% chance that\n", item.percent));
            if !item.reference.name().is_empty() {
                out.push_str(&dependency_text(
                    item.per,
                    item.beyond,
                    item.reference.name(),
                ));
            }
            out.push_str(&format!(
                "  {} {} worth {} {}\n",
                dice_text(item.quantity),
                item.item,
                dice_text(item.coins),
                coin_name(item.coin)
            ));
            if !item.escorts.is_empty() {
                out.push_str("  With these items,\n");
                for e in &item.escorts {
                    out.push_str(&format!("    {} {}\n", dice_text(e.dice), e.monster.name));
                }
            }
            if !item.per_item.is_empty() {
                out.push_str("  For each of these items\n");
                for e in &item.per_item {
                    out.push_str(&format!("    {} {}\n", dice_text(e.dice), e.monster.name));
                }
            }
            if !item.doubled.is_empty() {
                out.push_str(&format!(
                    "  The presence of these items causes the {} appearing to double.\n",
                    item.doubled.join(", ")
                ));
            }
        }
        if !template.traps.is_empty() {
            out.push_str("\nTraps\n");
            for t in &template.traps {
                out.push_str(&format!("  {}\n", t.name));
            }
        }
        out
    }
}

fn dependency_text(per: i32, beyond: i32, reference: &str) -> String {
    if per != 0 {
        if beyond != 0 {
            format!("  For every {per} {reference} beyond {beyond}\n")
        } else {
            format!("  For every {per} {reference} present\n")
        }
    } else {
        format!("  If there are at least {beyond} {reference} then\n")
    }
}

/// One monster line of the template printout.
fn monster_line_text(line: &MonsterLine) -> String {
    let mut s = format!("There is a {}% chance that\n", line.percent);
    if line.dependent {
        s.push_str(&dependency_text(
            line.per,
            line.beyond,
            line.reference.name(),
        ));
    }
    if line.list.len() < 2 {
        s.push_str(&format!(
            "  {} {} appear.\n",
            dice_text(line.dice),
            line.monster
        ));
    } else {
        s.push_str(&format!(
            "  {} selected from the following:\n",
            dice_text(line.dice)
        ));
        for m in &line.list {
            s.push_str(&format!("    {}\n", m.name));
        }
    }
    if line.and || line.or {
        s.push_str(&format!(
            "  {} {} {}\n",
            if line.and { "and" } else { "or" },
            dice_text(line.second_dice),
            line.second_monster
        ));
    }
    s
}
