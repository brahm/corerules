//! Rogue skills, class and racial ability values, and nonweapon
//! proficiency scores.

use cr_formats::parts::Part;

use super::{AIM, BALANCE, Ctx, DEX, MUSCLE, STR, ability_display_name, single, table};
use crate::sheet::tables::{self, at};
use crate::sheet::{Cell, Section, ids};

/// The skill ids (0x59ad4d), by name.
const SKILLS: [&str; 13] = [
    "pick pockets",
    "open locks",
    "find/remove traps",
    "move silently",
    "hide in shadows",
    "detect noise",
    "climb walls",
    "read languages",
    "detect magic",
    "detect illusion",
    "bribe",
    "tunneling",
    "escaping bonds",
];

/// What no armor at all adds (0x59b282), by skill id - 1.
const NO_ARMOR: [i32; 13] = [5, 0, 0, 10, 5, 0, 10, 0, 0, 0, 0, 10, 5];

/// A rogue skill's name without the class, the cost and the `*`.
fn skill_name(part_name: &str) -> String {
    ability_display_name(part_name)
        .trim_end_matches('*')
        .to_string()
}

/// The skill id, 1 to 13, of a skill name.
fn skill_id(name: &str) -> Option<usize> {
    let lower = name.to_lowercase();
    SKILLS.iter().position(|s| *s == lower).map(|i| i + 1)
}

/// One row of the Rogue Skills table.
pub(in crate::sheet) struct Skill {
    pub name: String,
    pub part_name: String,
    pub base: i32,
    pub racial: i32,
    pub ability: i32,
    pub discretionary: i32,
    pub armor: i32,
    pub total: i32,
}

/// The ability score adjustment of skill `id` (0x59b19d).
fn ability_adjustment(ctx: &Ctx, id: usize) -> i32 {
    let s = &ctx.scores;
    match id {
        1 => at(&tables::PICK_POCKETS, s.sub(DEX, AIM)),
        2 => at(&tables::OPEN_LOCKS, s.sub(DEX, AIM)),
        3 => at(&tables::FIND_TRAPS, s.sub(DEX, AIM)),
        4 => at(&tables::MOVE_SILENTLY, s.sub(DEX, BALANCE)),
        5 => at(&tables::HIDE_IN_SHADOWS, s.sub(DEX, BALANCE)),
        7 => at(&tables::CLIMB_WALLS, s.sub(DEX, BALANCE)),
        12 => at(&tables::TUNNELING, s.sub(STR, MUSCLE)),
        13 => at(&tables::ESCAPING_BONDS, s.sub(STR, MUSCLE)),
        _ => 0,
    }
}

/// The armor adjustment of skill `id` (0x59b282): the sum of the worn
/// parts' thief skill adjustments (`data.unknown_0x59c`, when
/// `data.unknown_0x598` is set), and whether body armor (+0x248 1) is
/// worn, which takes away the no-armor bonus.
fn part_adjustment(parts: &[&Part], id: usize) -> (bool, i32) {
    let mut armored = false;
    let mut sum = 0;
    for p in parts {
        if p.data.unknown_0x598 != 0 {
            sum += p.data.unknown_0x59c[id - 1];
        }
        if p.head.unknown_0x248 == 1 {
            armored = true;
        }
    }
    (armored, sum)
}

/// The rogue skills (0x59aa97), sorted by name.
pub(in crate::sheet) fn skills(ctx: &Ctx) -> Vec<Skill> {
    let race = ctx.race();
    let traits: Vec<&Part> = ctx.ch.data.unknown_0x1830[..2]
        .iter()
        .flatten()
        .filter_map(|n| ctx.part(n))
        .collect();
    let mut list: Vec<Skill> = ctx
        .ch
        .data
        .rogue_skills
        .entries
        .iter()
        .filter_map(|(part_name, values)| {
            let name = skill_name(part_name);
            let id = skill_id(&name)?;
            let base = ctx.part(part_name).map_or(0, |p| p.data.unknown_0x510[5]);
            let racial = race.map_or(0, |r| {
                if id <= 8 {
                    r.thief_adjustments[id - 1]
                } else {
                    r.unknown_0x158[id - 9]
                }
            });
            let ability = ability_adjustment(ctx, id);
            let discretionary = values.iter().map(|&v| v as i32).sum();
            let (armored, mut armor) = part_adjustment(&ctx.worn, id);
            if !armored {
                armor += NO_ARMOR[id - 1];
            }
            let extra = part_adjustment(&traits, id).1;
            Some(Skill {
                name,
                part_name: part_name.clone(),
                base,
                racial,
                ability,
                discretionary,
                armor,
                total: base + racial + ability + discretionary + armor + extra,
            })
        })
        .collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    list
}

/// Rogue Skills (0x5a7b10).
pub(in crate::sheet) fn section(ctx: &Ctx) -> Section {
    let mut rows = vec![
        [
            "Skill",
            "Base",
            "Racial Mods",
            "Ability Scores",
            "Discretionary Points",
            "Armor",
            "Final",
        ]
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let c = Cell::header(*h);
            if i == 0 { c } else { c.right() }
        })
        .collect::<Vec<_>>(),
    ];
    for s in skills(ctx) {
        rows.push(vec![
            Cell::new(s.name),
            Cell::new(s.base.to_string()).right(),
            Cell::new(format!("{:+}", s.racial)).right(),
            Cell::new(format!("{:+}", s.ability)).right(),
            Cell::new(format!("{:+}", s.discretionary)).right(),
            Cell::new(format!("{:+}", s.armor)).right(),
            Cell::new(format!("{}%", s.total)).right(),
        ]);
    }
    single(ids::ROGUE_SKILLS, "Rogue Skills", vec![table(7, rows)])
}

/// Rangers' hide in shadows and move silently by level 0 to 15 (0x5fd6f7);
/// 99 above.
const RANGER_HIDE: [i32; 16] = [
    0, 10, 15, 20, 25, 31, 37, 43, 49, 56, 63, 70, 77, 85, 93, 99,
];
const RANGER_MOVE: [i32; 16] = [
    0, 15, 21, 27, 33, 40, 47, 55, 62, 70, 78, 86, 94, 99, 99, 99,
];

/// Skills & Powers scroll use by level 0 to 11 (0x5fd6f7); 80 above.
const SP_SCROLL_USE: [i32; 12] = [0, 10, 10, 20, 20, 30, 30, 40, 50, 60, 70, 80];

/// The value an ability box shows after a class or racial ability's name
/// (0x5fd6f7), by the part's kind (`data.unknown_0x778[1]`): 1 Dexterity
/// plus level, 2 and 7 the rangers' hide in shadows table, 3 and 8 their
/// move silently table, 5 a percentage (`data.unknown_0x510[5]`, plus
/// `[6]` per level after the first, at most `data.unknown_0x52c`), 6 the
/// rogue skill's final score, 4 the chance to use scrolls. A racial ability (`class` `None`) shows its percentage.
/// `None` when there is nothing, or 0, to show.
pub(in crate::sheet) fn ability_value(
    ctx: &Ctx,
    part: &Part,
    class: Option<usize>,
) -> Option<String> {
    let level = class
        .and_then(|g| ctx.classes.get(g))
        .map_or(0, |c| c.level as i32);
    let table = |t: &[i32; 16]| {
        if level > 15 {
            99
        } else {
            t[level.max(0) as usize]
        }
    };
    // Racial abilities (0x5a67c2) show their percentage as it is.
    let value = match part.data.unknown_0x778[1] {
        _ if class.is_none() => part.data.unknown_0x510[5],
        1 => ctx.scores.score(DEX) + level,
        2 | 7 => table(&RANGER_HIDE),
        3 | 8 => table(&RANGER_MOVE),
        5 => {
            let d = &part.data;
            let v = d.unknown_0x510[5] + d.unknown_0x510[6] * (level - 1);
            v.min(d.unknown_0x52c.unwrap_or(i32::MAX))
        }
        // Scroll use: Skills & Powers by level; the Core Rules from 10th
        // level, 85 for a bard (2001), else 75 for a thief (2000).
        4 if ctx.sp => SP_SCROLL_USE.get(level.max(0) as usize).copied().unwrap_or(80),
        4 if level > 9 => {
            let ids: Vec<i32> = ctx
                .classes
                .iter()
                .filter_map(|c| match c.class {
                    Some(cr_formats::classes::Class::Expansion { data, .. }) => Some(data.id),
                    _ => None,
                })
                .collect();
            if ids.contains(&2001) {
                85
            } else if ids.contains(&2000) {
                75
            } else {
                0
            }
        }
        6 => skills(ctx)
            .into_iter()
            .find(|s| s.part_name == part.head.name)
            .map_or(0, |s| s.total),
        _ => 0,
    };
    (value != 0).then(|| format!("{value}%"))
}

/// The ability a nonweapon proficiency is checked against: an ability
/// name ("Intelligence") or "Ability/Subability".
fn named_score(ctx: &Ctx, name: &str) -> Option<i32> {
    const ABILITIES: [(&str, [&str; 2]); 6] = [
        ("Strength", ["Stamina", "Muscle"]),
        ("Dexterity", ["Aim", "Balance"]),
        ("Constitution", ["Health", "Fitness"]),
        ("Intelligence", ["Reason", "Knowledge"]),
        ("Wisdom", ["Intuition", "Willpower"]),
        ("Charisma", ["Leadership", "Appearance"]),
    ];
    let (ability, sub) = match name.split_once('/') {
        Some((a, s)) => (a, Some(s)),
        None => (name, None),
    };
    let a = ABILITIES.iter().position(|(n, _)| *n == ability)?;
    let s = match sub {
        Some(sub) if ctx.sp => ABILITIES[a]
            .1
            .iter()
            .position(|n| *n == sub)
            .map_or(0, |i| i + 1),
        _ => 0,
    };
    Some(ctx.scores.sub(a, s))
}

/// The Skills & Powers proficiency bonus of a subability (0x42d7bc).
const SUBABILITY_BONUS: [i32; 26] = [
    -5, -5, -5, -5, -4, -3, -2, -1, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 5, 5, 5, 5, 5, 5, 5,
];

/// 0x42d7bc: the Skills & Powers proficiency bonus of a subability.
pub(super) fn subability_bonus(score: i32) -> i32 {
    at(&SUBABILITY_BONUS, score)
}

/// A nonweapon proficiency's score (0x438ca1). With the Core Rules: its
/// ability plus its modifier (`data.unknown_0x480[1]`). With Skills &
/// Powers: its initial rating (`data.unknown_0x480[0]`) plus the bonus of
/// the better of its two subabilities. Then one per extra slot spent on
/// it.
pub(in crate::sheet) fn nonweapon_score(ctx: &Ctx, index: usize, name: &str) -> i32 {
    let Some(part) = ctx.part(name) else {
        return 0;
    };
    let extra = ctx.ch.data.nonweapon_proficiencies[..index]
        .iter()
        .filter(|n| *n == name)
        .count() as i32;
    let h = &part.head;
    let mut total = if ctx.sp {
        let score = [&h.unknown_0x458, &h.unknown_0x45c]
            .iter()
            .filter_map(|n| named_score(ctx, n))
            .max()
            .unwrap_or(0);
        part.data.unknown_0x480[0] + at(&SUBABILITY_BONUS, score)
    } else {
        named_score(ctx, &h.unknown_0x460).unwrap_or(0) + part.data.unknown_0x480[1]
    };
    total += extra;
    if ctx.ch.data.unknown_0xac[5].iter().any(|n| n == name) {
        total += 1;
    }
    total
}
