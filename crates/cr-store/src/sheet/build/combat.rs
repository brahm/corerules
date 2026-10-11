//! Saving throws, the Combat and Armor boxes, the weapon tables, turning
//! undead and movement.

use cr_formats::classes::Class;
use cr_formats::parts::Part;

use super::{AIM, BALANCE, Ctx, MUSCLE, STAMINA, STR, single, table};
use crate::sheet::tables::{self, at};
use crate::sheet::{Cell, Flavor, Section, SheetBox, ids};

/// The saving throws in the order the section prints them, with the
/// category of their base value (0 paralyzation/poison/death, 1
/// rod/staff/wand, 2 petrification/polymorph, 3 breath weapon, 4 spell)
/// and their bonus slot: the index into a part's `head.unknown_0x2d8`
/// (p2d8..p2fc: paralyzation, poison, death, rod, staff, wand,
/// petrification, polymorph, spell, breath).
const SAVES: [(&str, usize, usize); 10] = [
    ("Paralyzation", 0, 0),
    ("Poison", 0, 1),
    ("Death Magic", 0, 2),
    ("Petrification", 2, 6),
    ("Polymorph", 2, 7),
    ("Rod", 1, 3),
    ("Staff", 1, 4),
    ("Wand", 1, 5),
    ("Breath Weapon", 3, 9),
    ("Spell", 4, 8),
];

/// Bonus slots (as in [`SAVES`]) of the racial ability save flags
/// `data.unknown_0x530[6..16]` (+0x548..+0x56c): breath, death,
/// paralyzation, petrification, poison, polymorph, rod, spell, staff,
/// wand.
const RACIAL_SAVE_SLOTS: [usize; 10] = [9, 2, 0, 6, 1, 7, 3, 8, 4, 5];

/// The slots the Constitution bonus of a dwarf-like item covers: poison,
/// rod, staff, wand and spell.
const CONSTITUTION_SLOTS: [usize; 5] = [1, 3, 4, 5, 8];

/// Base saving throws per category (0x433744): the lowest of the classes'
/// tables at their levels, from 20.
fn base_saves(ctx: &Ctx) -> [i32; 5] {
    let mut base = [20; 5];
    for c in &ctx.classes {
        let row = match (c.class, ctx.flavor) {
            (Some(Class::Expansion { data, .. }), Flavor::Expansion) => {
                match &data.unknown_0x27a8 {
                    Some(t) => t[c.level.min(100) as usize],
                    None => continue,
                }
            }
            (_, Flavor::Core20) => tables::core20_save(c.group, c.level),
            _ => continue,
        };
        for (b, v) in base.iter_mut().zip(row) {
            *b = (*b).min(v);
        }
    }
    base
}

/// The saving throw bonuses by slot (cache 0x433985): items readied and
/// worn, class abilities, racial abilities and Constitution. Core Rules
/// 2.0 has none.
fn save_bonuses(ctx: &Ctx) -> [i32; 10] {
    let mut bonus = [0; 10];
    if !ctx.expansion() {
        return bonus;
    }
    let health = ctx.scores.sub(super::CON, 1);
    let con_bonus = at(&tables::CONSTITUTION_SAVE_BONUS, health);
    let mut dwarf_like = false;
    for part in ctx.equipped() {
        for (b, v) in bonus.iter_mut().zip(part.head.unknown_0x2d8) {
            *b += v;
        }
        dwarf_like |= part.head.unknown_0x80c[2] != 0;
    }
    if dwarf_like && !ctx.ch.data.race.contains("Dwarf") {
        for s in CONSTITUTION_SLOTS {
            bonus[s] += con_bonus;
        }
    }
    for g in ctx.counted_classes() {
        for part in ctx.class_abilities(g) {
            for (b, v) in bonus.iter_mut().zip(part.head.unknown_0x2d8) {
                *b += v;
            }
        }
    }
    let mut poison_touched = false;
    for part in ctx.racial_abilities() {
        let value = part.data.unknown_0x530[1];
        let b = if value != 0 { -value } else { con_bonus };
        for (flag, slot) in part.data.unknown_0x530[6..16].iter().zip(RACIAL_SAVE_SLOTS) {
            if *flag != 0 {
                bonus[slot] += b;
                poison_touched |= slot == 1;
            }
        }
    }
    if !poison_touched {
        bonus[1] += at(&tables::POISON_SAVE, health);
    }
    bonus
}

/// Saving Throws (0x5a403e): two rows of five.
pub(in crate::sheet) fn saves(ctx: &Ctx) -> Section {
    let base = base_saves(ctx);
    let bonus = save_bonuses(ctx);
    let rows = SAVES
        .chunks(5)
        .map(|row| {
            row.iter()
                .map(|&(name, cat, slot)| Cell::new(format!("{name}: {}", base[cat] - bonus[slot])))
                .collect()
        })
        .collect();
    single(ids::SAVES, "Saving Throws", vec![table(5, rows)])
}

/// Base THAC0 (0x426e11). Every class's own table at its level counts,
/// the lowest wins, from 20. With `warrior_base` (the sheet's Base THAC0
/// passes it), while the warrior value is still 20 the Fighter table is
/// read at each class's level: so the Expansion shows the Fighter THAC0
/// for the highest level whatever the class. A dual-classed character
/// whose new class is still below the old one gets the new class's value.
/// Core Rules 2.0 (0x4134a0) uses built-in tables; for a dual-classed
/// character whose levels do not rise from class to class, no class
/// counts and the value stays 20 (golden Corin, fighter 7 then mage).
pub(in crate::sheet) fn thac0(ctx: &Ctx, warrior_base: bool) -> i32 {
    let n = ctx.classes.len();
    if ctx.flavor == Flavor::Core20 {
        let mut previous = 0;
        let falling = ctx.dual()
            && ctx.classes.iter().any(|c| {
                let fall = c.level <= previous;
                previous = c.level;
                fall
            });
        if falling {
            return 20;
        }
        return ctx
            .classes
            .iter()
            .map(|c| tables::core20_thac0(c.group, c.level))
            .fold(20, i32::min);
    }
    let table = |class: Option<&Class>, level: u32| -> Option<i32> {
        match class {
            Some(Class::Expansion { data, .. }) => data
                .unknown_0x27c
                .as_ref()
                .map(|t| t[level.min(100) as usize]),
            _ => None,
        }
    };
    let fighter = ctx.rules.classes.get("Fighter");
    let mut groups = [20; 5];
    for c in &ctx.classes {
        if let (Some(v), 0..=4) = (table(c.class, c.level), c.group) {
            groups[c.group as usize] = v;
        }
        if warrior_base
            && groups[0] == 20
            && let Some(v) = table(fighter, c.level)
        {
            groups[0] = v;
        }
    }
    let mut result = 20;
    for (g, v) in groups.iter().enumerate() {
        if ctx.has_group(g as i32) || (g == 0 && warrior_base) {
            result = result.min(*v);
        }
    }
    if ctx.dual() && n > 1 {
        let (new, old) = (&ctx.classes[n - 1], &ctx.classes[n - 2]);
        if new.level < old.level
            && let Some(v) = table(new.class, new.level)
        {
            result = v;
        }
    }
    result
}

/// Strength attack adjustment (0x4206e9), from Muscle.
fn attack_adjustment(ctx: &Ctx) -> i32 {
    let m = ctx.scores.sub(STR, MUSCLE);
    tables::strength(&tables::ATTACK_ADJUSTMENT, m, ctx.scores.exceptional_for(m))
}

/// Strength damage adjustment (0x42098f), from Muscle.
fn damage_adjustment(ctx: &Ctx) -> i32 {
    let m = ctx.scores.sub(STR, MUSCLE);
    tables::strength(&tables::DAMAGE_ADJUSTMENT, m, ctx.scores.exceptional_for(m))
}

/// Missile adjustment (0x421025), from Aim.
fn missile_adjustment(ctx: &Ctx) -> i32 {
    at(&tables::MISSILE_ADJUSTMENT, ctx.scores.sub(super::DEX, AIM))
}

/// The Combat box (0x5a4323).
pub(in crate::sheet) fn combat_box(ctx: &Ctx) -> SheetBox {
    let hp = ctx.rules.advance.hit_points(ctx.ch).unwrap_or(0);
    let base = thac0(ctx, ctx.expansion());
    let rows = [
        ("Hit Points:", hp),
        ("Base THAC0:", base),
        ("Melee THAC0:", base - attack_adjustment(ctx)),
        ("Missile THAC0:", base - missile_adjustment(ctx)),
    ]
    .into_iter()
    .map(|(label, v)| vec![Cell::new(label), ctx.value(v.to_string())])
    .collect();
    SheetBox {
        title: "Combat".into(),
        body: vec![table(2, rows)],
    }
}

/// Encumbrance categories, as the current state prints them.
const ENCUMBRANCE: [&str; 6] = ["None", "Light", "Moderate", "Heavy", "Severe", "Excessive"];

/// The carrying limits (0x4290f5) per category, indexed by Stamina 0 to 18,
/// then 18/01-50 to 18/00 (19 to 23) and Strength 19 to 25 (24 to 30).
const LIMITS: [[i32; 31]; 5] = [
    [
        0, 1, 2, 6, 11, 11, 21, 21, 36, 41, 41, 41, 46, 46, 56, 56, 71, 86, 111, 136, 161, 186,
        236, 336, 486, 536, 636, 786, 936, 1236, 1536,
    ],
    [
        0, 1, 3, 7, 14, 14, 30, 30, 51, 51, 59, 59, 70, 70, 86, 86, 101, 122, 150, 175, 200, 225,
        275, 375, 525, 575, 675, 825, 975, 1275, 1575,
    ],
    [
        0, 1, 4, 8, 17, 17, 39, 39, 66, 66, 77, 77, 94, 94, 116, 116, 131, 158, 189, 214, 239, 264,
        314, 414, 564, 614, 714, 864, 1014, 1314, 1614,
    ],
    [
        0, 1, 5, 10, 20, 20, 47, 47, 81, 81, 97, 97, 118, 118, 146, 146, 161, 194, 228, 253, 278,
        303, 353, 453, 603, 653, 753, 903, 1053, 1353, 1653,
    ],
    [
        0, 3, 6, 10, 25, 25, 55, 55, 90, 90, 110, 110, 140, 140, 170, 170, 195, 220, 255, 280, 305,
        330, 380, 480, 640, 700, 810, 970, 1130, 1440, 1750,
    ],
];

/// The heaviest load of each category: one less than the table value for
/// Unencumbered to Heavy, the value itself for Severe.
fn limits(ctx: &Ctx) -> [i32; 5] {
    let stamina = ctx.scores.sub(STR, STAMINA);
    let index = match ctx.scores.exceptional_for(stamina) {
        Some(e) if e > 0 => 19 + tables::exceptional_bracket(e).unwrap_or(0) as i32,
        _ if stamina > 18 => stamina + 5,
        _ => stamina.max(0),
    }
    .min(30) as usize;
    std::array::from_fn(|c| LIMITS[c][index] - i32::from(c < 4))
}

/// The weight carried, in pounds.
fn carried(ctx: &Ctx) -> f32 {
    // A stored -0.0 prints as 0.00.
    f32::from_bits(ctx.ch.data.carried_weight) + 0.0
}

/// The encumbrance category (0x429906): 0 none to 5 excessive.
pub(in crate::sheet) fn encumbrance(ctx: &Ctx) -> usize {
    let weight = carried(ctx);
    limits(ctx)
        .iter()
        .position(|&l| weight < (l + 1) as f32)
        .unwrap_or(5)
}

/// The final armor class (0x429ac5), -10 to 10.
pub(in crate::sheet) fn armor_class(ctx: &Ctx) -> i32 {
    armor(ctx).1
}

/// The Armor box (0x429ac5 → 0x5fac14): `label:value` lines, then FINAL.
pub(in crate::sheet) fn armor_box(ctx: &Ctx) -> SheetBox {
    let (lines, ac) = armor(ctx);
    let mut rows: Vec<Vec<Cell>> = lines
        .iter()
        .map(|line| {
            let (label, value) = line.split_once(':').unwrap_or((line, ""));
            vec![Cell::new(label), Cell::new(value).right()]
        })
        .collect();
    rows.push(vec![Cell::new("FINAL:"), Cell::new(ac.to_string()).right()]);
    SheetBox {
        title: "Armor".into(),
        body: vec![table(2, rows)],
    }
}

/// The armor class worked out (0x429ac5): its `label:value` lines and
/// the final value, kept within -10 to 10.
fn armor(ctx: &Ctx) -> (Vec<String>, i32) {
    let mut lines: Vec<String> = Vec::new();
    let mut natural = ctx.race().map_or(10, |r| r.unknown_0x16c[0]);
    if natural == 0 {
        natural = 10;
    }
    for c in &ctx.classes {
        if let Some(Class::Expansion { data, .. }) = c.class
            && let Some(t) = &data.unknown_0x3380
        {
            let v = t[c.level.min(100) as usize];
            if v != 0 {
                natural = natural.min(v);
            }
        }
    }
    lines.push(format!("Natural armor class: {natural}"));
    let mut ac = natural;
    let worn: Vec<&Part> = ctx.worn.clone();
    let body = worn.iter().find(|p| p.head.unknown_0x248 == 1);
    let shield = worn.iter().find(|p| p.head.unknown_0x248 == 3);
    let mut magic_armor = false;
    if let Some(armor) = body {
        let bonus = (10 - armor.head.unknown_0x250[8]).max(if natural < 10 { 1 } else { 0 });
        lines.push(format!("{}:-{bonus}", armor.head.name));
        ac -= bonus;
        let magic = armor.head.unknown_0x44c[0];
        if magic != 0 {
            magic_armor = true;
            lines.push(format!("Magic Armor adj.: -{magic}"));
            ac -= magic;
        }
    }
    if let Some(shield) = shield {
        let magic = shield.head.unknown_0x44c[0];
        if magic != 0 {
            lines.push(format!("{} magic adj.:-{magic}", shield.head.name));
            ac -= magic;
        }
        let bonus = 10 - shield.head.unknown_0x250[8];
        lines.push(format!("{} adj.: -{bonus}", shield.head.name));
        ac -= bonus;
    }
    if body.is_none() {
        for part in &worn {
            if part.data.unknown_0x584[1] != 0 {
                let bonus = 10 - part.head.unknown_0x250[8];
                lines.push(format!("{}: -{bonus}", part.head.name));
                ac -= bonus;
            }
        }
    }
    if !magic_armor {
        let mut counted = false;
        for part in &worn {
            let protection = part.data.unknown_0x584[2] != 0 || part.data.unknown_0x584[3] != 0;
            if !protection || (shield.is_some() && counted) {
                continue;
            }
            let magic = part.head.unknown_0x44c[0];
            if magic != 0 {
                lines.push(format!("{} magic adj.:-{magic}", part.head.name));
                ac -= magic;
                counted = true;
            }
        }
    }
    let dex = at(
        &tables::DEFENSIVE_ADJUSTMENT,
        ctx.scores.sub(super::DEX, BALANCE),
    );
    if dex != 0 {
        if ctx.sp {
            lines.push(format!("Balance Defensive adj.: {dex}"));
        } else {
            lines.push(format!("DEX Defensive adj.: {dex}"));
        }
        ac += dex;
    }
    if encumbrance(ctx) >= 4 {
        lines.push("Severe encumbrance penalty: 3".into());
        ac += 3;
    }
    (lines, ac.clamp(-10, 10))
}

/// Weapon parts: the `head.unknown_0x3b8` flags are melee, missile,
/// launcher and damage; `data.unknown_0x420[0]` marks ammunition.
fn is_melee(p: &Part) -> bool {
    p.head.unknown_0x3b8[0] != 0
}

fn is_missile(p: &Part) -> bool {
    p.head.unknown_0x3b8[1] != 0
}

fn is_launcher(p: &Part) -> bool {
    p.head.unknown_0x3b8[2] != 0
}

fn is_ammo(p: &Part) -> bool {
    p.data.unknown_0x420[0] != 0
}

/// The weapon a part is a version of ("Sword, long" for "Sword, long
/// +1").
fn base_name(p: &Part) -> &str {
    if p.data.unknown_0x428.is_empty() {
        &p.head.name
    } else {
        &p.data.unknown_0x428
    }
}

/// Weapon statistics. A part has two sets: Skills & Powers' (`0x3b4` to
/// `0x3e0`) and the Core Rules' (`0x3e4` to `0x404`), which a Core
/// Rules character uses when it is filled in (0x6621c0).
struct Stats {
    /// Small/medium and large dice: count, sides, bonus.
    small: (i32, i32, i32),
    large: (i32, i32, i32),
    speed: i32,
    kind: String,
    size: String,
}

fn stats(p: &Part, core: bool) -> Stats {
    let h = &p.head;
    let small = if core && (h.unknown_0x3f8 != 0 || h.unknown_0x3fc != 0 || h.unknown_0x400 != 0) {
        (h.unknown_0x3fc, h.unknown_0x3f8, h.unknown_0x400)
    } else {
        (h.unknown_0x3d8, h.unknown_0x3d4, h.unknown_0x3dc)
    };
    let large = if core && (h.unknown_0x3e8 != 0 || h.unknown_0x3ec != 0 || h.unknown_0x3f0 != 0) {
        (h.unknown_0x3e8, h.unknown_0x3ec, h.unknown_0x3f0)
    } else {
        (h.unknown_0x3b8[3], h.unknown_0x3c8, h.unknown_0x3cc)
    };
    let speed = if core && h.unknown_0x404 != 0 {
        h.unknown_0x404
    } else {
        h.unknown_0x3e0
    };
    let kind = if !core || h.unknown_0x3e4.is_empty() {
        h.unknown_0x3b4.clone()
    } else {
        h.unknown_0x3e4.clone()
    };
    let size = if !core || h.unknown_0x3f4.is_empty() {
        h.unknown_0x3d0.clone()
    } else {
        h.unknown_0x3f4.clone()
    };
    Stats {
        small,
        large,
        speed,
        kind,
        size,
    }
}

/// Damage text (0x554ff2): `NdM`, `xK`, `+B` (a negative bonus prints
/// `+-1`); a plain number when the dice are missing, `N*K+B` for one-sided
/// dice. "0" prints as nothing.
fn damage_text((n, m, _): (i32, i32, i32), bonus: i32) -> String {
    let k = 1;
    let text = if n == 0 || m == 0 {
        bonus.to_string()
    } else if m == 1 {
        (n * k + bonus).to_string()
    } else {
        let mut t = format!("{n}d{m}");
        if bonus != 0 {
            t.push_str(&format!("+{bonus}"));
        }
        t
    };
    if text == "0" { String::new() } else { text }
}

/// Racial weapon bonuses (0x432e02): (attack, damage) from the racial
/// abilities whose weapon list names this weapon (or, for ammunition, its
/// launchers).
fn racial_weapon_bonus(ctx: &Ctx, weapon: &Part) -> (i32, i32) {
    let (mut attack, mut damage) = (0, 0);
    for ability in ctx.racial_abilities() {
        for name in &ability.data.unknown_0x4fc {
            if weapon.head.name.contains(name.as_str()) {
                attack += ability.head.unknown_0x44c[2];
                damage += ability.head.unknown_0x44c[1];
            } else if is_ammo(weapon) && weapon.data.unknown_0x434.iter().any(|l| l == name) {
                damage += ability.head.unknown_0x44c[1];
            }
        }
    }
    (attack, damage)
}

fn proficient(ctx: &Ctx, p: &Part) -> bool {
    let base = base_name(p);
    ctx.ch
        .data
        .weapon_proficiencies
        .iter()
        .any(|w| w == base || w == &p.head.name)
}

fn specialized(ctx: &Ctx, p: &Part) -> bool {
    let base = base_name(p);
    ctx.ch.data.unknown_0xac[3]
        .iter()
        .any(|w| w == base || w == &p.head.name)
}

fn mastery(ctx: &Ctx, p: &Part) -> bool {
    let m = &ctx.ch.data.unknown_0x124[2];
    !m.is_empty() && (m == base_name(p) || m == &p.head.name)
}

fn weapon_of_choice(ctx: &Ctx, p: &Part) -> bool {
    let w = &ctx.ch.data.unknown_0x124[0];
    !w.is_empty() && (w == base_name(p) || w == &p.head.name)
}

/// The non-proficiency penalty of the character's classes (the least of
/// them), a negative number.
fn nonproficiency_penalty(ctx: &Ctx) -> i32 {
    ctx.classes
        .iter()
        .filter_map(|c| match c.class {
            Some(Class::Expansion { data, .. }) => Some(data.unknown_0xcc[4]),
            Some(Class::Core20(data)) => Some(data.unknown_0x10[31]),
            None => None,
        })
        .max()
        .unwrap_or(0)
}

/// The THAC0 penalty of the encumbrance category.
fn encumbrance_penalty(ctx: &Ctx) -> i32 {
    match encumbrance(ctx) {
        2 => 1,
        3 => 2,
        4 | 5 => 4,
        _ => 0,
    }
}

/// The magic attack bonus of a weapon (0x447700), less 3 for a weapon
/// that needs a class (0x447720) the character lacks.
fn magic_attack(ctx: &Ctx, p: &Part) -> i32 {
    let mut magic = p.head.unknown_0x44c[2];
    if p.head.unknown_0x80c[1] != 0
        && !ctx.classes.iter().any(|c| {
            matches!(c.class, Some(Class::Expansion { data, .. }) if data.flag_0x278d.unwrap_or(0) == 1)
        })
    {
        magic -= 3;
    }
    magic
}

/// Two-weapon fighting: `Some(true)` for the primary hand, `Some(false)`
/// for the offhand.
type Hand = Option<bool>;

/// Melee THAC0 of a weapon (0x423a3d), `None` when it is no melee weapon.
fn melee_thac0(ctx: &Ctx, p: &Part, hand: Hand) -> Option<i32> {
    if is_ammo(p) || !is_melee(p) {
        return None;
    }
    let mut t = thac0(ctx, false) - attack_adjustment(ctx);
    t -= common_thac0(ctx, p);
    if specialized(ctx, p) && !mastery(ctx, p) {
        t -= 1;
    }
    if mastery(ctx, p) {
        t -= 3;
    }
    if let Some(primary) = hand
        && !two_weapon_style(ctx)
    {
        t += if primary { 2 } else { 4 };
        let reaction = at(
            &tables::REACTION_ADJUSTMENT,
            ctx.scores.sub(super::DEX, BALANCE),
        );
        t -= reaction.min(if primary { 2 } else { 4 });
    }
    Some(clamp_thac0(ctx, t))
}

/// Whether a class ability gives the ranger's two-weapon style and the
/// armor allows it (0x43a60f): no armor, or leather, studded leather,
/// padded, hide or elven chain. Fighting with two weapons then costs
/// nothing.
fn two_weapon_style(ctx: &Ctx) -> bool {
    const LIGHT: [&str; 5] = [
        " leather",
        " studded leather",
        " padded",
        " hide",
        " elven chain",
    ];
    let style = (0..ctx.classes.len()).any(|g| {
        ctx.class_abilities(g)
            .iter()
            .any(|p| p.head.name.contains("Two-weapon style"))
    });
    style && {
        let armor: Vec<&Part> = ctx
            .worn
            .iter()
            .copied()
            .filter(|p| p.head.unknown_0x248 == 1)
            .collect();
        armor.is_empty()
            || armor
                .iter()
                .any(|p| LIGHT.iter().any(|l| p.head.name.contains(l)))
    }
}

/// What the melee and missile THAC0 share: magic, weapon of choice,
/// proficiency, racial bonus and encumbrance.
fn common_thac0(ctx: &Ctx, p: &Part) -> i32 {
    let mut bonus = magic_attack(ctx, p);
    if weapon_of_choice(ctx, p) {
        bonus += 1;
    }
    if !proficient(ctx, p) {
        bonus += nonproficiency_penalty(ctx);
    }
    bonus += racial_weapon_bonus(ctx, p).0;
    bonus -= encumbrance_penalty(ctx);
    bonus
}

/// Missile THAC0 of a weapon (0x425799), `None` when it cannot be shot or
/// thrown.
fn missile_thac0(ctx: &Ctx, p: &Part) -> Option<i32> {
    if is_ammo(p) || !is_missile(p) {
        return None;
    }
    let mut t = thac0(ctx, false) - missile_adjustment(ctx);
    t -= common_thac0(ctx, p);
    if mastery(ctx, p) {
        t -= 2;
    }
    Some(clamp_thac0(ctx, t))
}

/// C's `atoi`: the leading number, after blanks; 0 when there is none.
fn atoi(text: &str) -> i32 {
    let text = text.trim_start();
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, text.strip_prefix('+').unwrap_or(text)),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    sign * digits[..end].parse::<i32>().unwrap_or(0)
}

/// 2.0 keeps a weapon's THAC0 between 1 and 20 (0x411c10, 0x412980).
fn clamp_thac0(ctx: &Ctx, t: i32) -> i32 {
    if ctx.expansion() { t } else { t.clamp(1, 20) }
}

/// Attacks per round (0x43f0c0).
fn attacks(ctx: &Ctx, p: &Part, missile: bool, hand: Hand) -> String {
    if hand == Some(false) {
        return "1".into();
    }
    let warrior = ctx
        .classes
        .iter()
        .filter(|c| c.group == 0)
        .map(|c| c.level)
        .max()
        .unwrap_or(0);
    if missile {
        // The rate of fire "a/b": the Expansion shows `a`, 2.0 (0x421d40)
        // the whole fraction, with 0 read as 1 and "/1" left out.
        let rof = &p.head.unknown_0x41c;
        let mut parts = rof.split('/').map(|s| atoi(s).max(1));
        let n = parts.next().unwrap_or(1);
        let d = parts.next().unwrap_or(1);
        return if ctx.expansion() || d == 1 {
            n.to_string()
        } else {
            format!("{n}/{d}")
        };
    }
    let special = specialized(ctx, p) || mastery(ctx, p);
    let text = if special {
        match warrior {
            0..=6 => "3/2",
            7..=12 => "2",
            _ => "5/2",
        }
    } else {
        match warrior {
            0..=6 => "1",
            7..=12 => "3/2",
            _ => "2",
        }
    };
    text.into()
}

/// The damage bonus of a melee weapon (0x428a99 small, 0x4281ab large):
/// its own bonus, Strength, specialization, magic and racial bonuses.
fn melee_damage_bonus(ctx: &Ctx, p: &Part, own: i32) -> i32 {
    let mut b = own + damage_adjustment(ctx);
    if !is_melee(p) {
        return 0;
    }
    if mastery(ctx, p) {
        b += 3;
    } else if specialized(ctx, p) {
        b += 2;
    }
    b += p.head.unknown_0x44c[1];
    b += racial_weapon_bonus(ctx, p).1;
    b
}

/// The damage bonus of ammunition shot from `launcher` (0x427a8f small,
/// 0x42754d large): its own bonus, Strength as far as the launcher allows,
/// the ammunition's magic and racial bonuses.
fn ammo_damage_bonus(ctx: &Ctx, launcher: &Part, ammo: &Part, own: i32) -> i32 {
    if !is_missile(launcher) {
        return 0;
    }
    let mut strength = damage_adjustment(ctx);
    if launcher.data.unknown_0x7b8 != 0 && strength < 0 {
        strength = 0;
    }
    strength = strength.min(launcher.data.unknown_0x7cc[0]);
    own + strength + ammo.head.unknown_0x44c[1] + racial_weapon_bonus(ctx, ammo).1
}

/// One row of the weapon tables: the weapon (or ammunition, indented, with
/// its launcher).
pub(in crate::sheet) struct WeaponRow<'a> {
    part: &'a Part,
    launcher: Option<&'a Part>,
}

/// The weapon list (0x5fc26d): every weapon readied, carried and worn,
/// each launcher followed by the ammunition it fires.
pub(in crate::sheet) fn weapon_rows<'a>(ctx: &Ctx<'a>) -> Vec<WeaponRow<'a>> {
    let mut names: Vec<&str> = Vec::new();
    for b in [super::READIED, super::CARRIED, super::WORN] {
        for item in &ctx.ch.data.inventory[b].items {
            names.push(&item.name);
        }
    }
    let weapons: Vec<&Part> = names
        .iter()
        .filter(|n| ctx.rules.part_types.get(**n) == Some(&0))
        .filter_map(|n| ctx.part(n))
        .collect();
    let mut rows = Vec::new();
    for w in &weapons {
        if is_ammo(w) {
            continue;
        }
        rows.push(WeaponRow {
            part: w,
            launcher: None,
        });
        if is_launcher(w) {
            for a in &weapons {
                if is_ammo(a) && a.data.unknown_0x434.contains(&w.head.name) {
                    rows.push(WeaponRow {
                        part: a,
                        launcher: Some(w),
                    });
                }
            }
        }
    }
    rows
}

fn weapon_cells(ctx: &Ctx, row: &WeaponRow, hand: Hand) -> Vec<Cell> {
    let p = row.part;
    let s = stats(p, !ctx.sp);
    let blank = |v: Option<i32>| match v {
        Some(v) if v != 99 => v.to_string(),
        _ => String::new(),
    };
    let (name, melee, missile, attacks, small, large) = match row.launcher {
        Some(launcher) => (
            format!("  {}", p.head.name),
            String::new(),
            String::new(),
            String::new(),
            damage_text(s.small, ammo_damage_bonus(ctx, launcher, p, s.small.2)),
            damage_text(s.large, ammo_damage_bonus(ctx, launcher, p, s.large.2)),
        ),
        None => {
            let melee = melee_thac0(ctx, p, hand);
            let missile = missile_thac0(ctx, p);
            let attacks = attacks(ctx, p, melee.is_none(), hand);
            let (small, large) = if is_launcher(p) {
                (String::new(), String::new())
            } else {
                (
                    damage_text(s.small, melee_damage_bonus(ctx, p, s.small.2)),
                    damage_text(s.large, melee_damage_bonus(ctx, p, s.large.2)),
                )
            };
            (
                p.head.name.clone(),
                blank(melee),
                blank(missile),
                attacks,
                small,
                large,
            )
        }
    };
    let speed = if row.launcher.is_some() {
        String::new()
    } else {
        s.speed.to_string()
    };
    let range = |v: i32| {
        if v == 0 { String::new() } else { v.to_string() }
    };
    let r = &p.head.unknown_0x40c;
    vec![
        Cell::new(name),
        Cell::new(melee).center(),
        Cell::new(missile).center(),
        Cell::new(attacks).center(),
        Cell::new(speed).center(),
        Cell::new(small).center(),
        Cell::new(large).center(),
        Cell::new(s.kind).center(),
        Cell::new(s.size).center(),
        Cell::new(range(r[2])).center(),
        Cell::new(range(r[3])).center(),
        Cell::new(range(r[1])).center(),
    ]
}

/// Weapons (0x5a4b5b) or Weapons - Primary and Offhand (0x5a5515).
pub(in crate::sheet) fn weapons(ctx: &Ctx, primary_offhand: bool) -> Section {
    let mut rows = vec![
        vec![
            Cell::header(""),
            Cell::header("THAC0").span(2).center(),
            Cell::header("Attacks/").center(),
            Cell::header("Speed").center(),
            Cell::header("Damage").span(2).center(),
            Cell::header("").center(),
            Cell::header("").center(),
            Cell::header("Range").center(),
            Cell::header("(-2)").center(),
            Cell::header("(-5)").center(),
        ],
        [
            "Weapon", "Melee", "Missile", "Round", "Factor", "Sm-Med", "Large", "Type", "Size",
            "Short", "Medium", "Long",
        ]
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let c = Cell::header(*h);
            if i == 0 { c } else { c.center() }
        })
        .collect(),
    ];
    let list = weapon_rows(ctx);
    let (id, title) = if primary_offhand {
        for (label, primary) in [("Primary:", true), ("Offhand:", false)] {
            let mut row = vec![Cell::header(label)];
            row.extend((0..11).map(|_| Cell::header("").center()));
            rows.push(row);
            for w in &list {
                rows.push(weapon_cells(ctx, w, Some(primary)));
            }
        }
        (ids::PRIMARY_OFFHAND, "Weapons - Primary and Offhand")
    } else {
        for w in &list {
            rows.push(weapon_cells(ctx, w, None));
        }
        (ids::WEAPONS, "Weapons")
    };
    single(id, title, vec![table(12, rows)])
}

/// The turn level (0x4346df, 0x433fe6): per class the lowest "turns at
/// level" of its abilities; the best level less that, plus one; 0 when
/// the character cannot turn. A class's own turning bonus
/// (`unknown_0x485c[0]`) can raise it.
pub(in crate::sheet) fn turn_level(ctx: &Ctx) -> i32 {
    let mut best: Option<i32> = None;
    for (g, c) in ctx.classes.iter().enumerate() {
        let t = ctx
            .class_abilities(g)
            .iter()
            .map(|p| p.data.unknown_0x778[0])
            .filter(|&t| t > 0)
            .min();
        if let Some(t) = t
            && c.level as i32 >= t
        {
            let candidate = c.level as i32 - t;
            best = Some(best.map_or(candidate, |b| b.max(candidate)));
        }
    }
    let mut level = best.map_or(0, |b| b + 1);
    for c in &ctx.classes {
        if let Some(Class::Expansion { data, .. }) = c.class {
            let v = data.unknown_0x485c[0];
            if v != 0 {
                level = level.max(c.level as i32 + v);
            }
        }
    }
    level
}

/// Turning results per undead type and column (0x433fe6).
const TURNING: [[&str; 12]; 13] = [
    [
        "10", "7", "4", "T", "T", "D", "D", "D*", "D*", "D*", "D*", "D*",
    ],
    [
        "13", "10", "7", "4", "T", "T", "D", "D", "D*", "D*", "D*", "D*",
    ],
    [
        "16", "13", "10", "7", "4", "T", "T", "D", "D", "D*", "D*", "D*",
    ],
    [
        "19", "16", "13", "10", "7", "4", "T", "T", "D", "D", "D*", "D*",
    ],
    [
        "20", "19", "16", "13", "10", "7", "4", "T", "T", "D", "D", "D*",
    ],
    [
        "-", "20", "19", "16", "13", "10", "7", "4", "T", "T", "D", "D",
    ],
    [
        "-", "-", "20", "19", "16", "13", "10", "7", "4", "T", "T", "D",
    ],
    [
        "-", "-", "-", "20", "19", "16", "13", "10", "7", "4", "T", "T",
    ],
    [
        "-", "-", "-", "-", "20", "19", "16", "13", "10", "7", "4", "T",
    ],
    [
        "-", "-", "-", "-", "-", "20", "19", "16", "13", "10", "7", "4",
    ],
    [
        "-", "-", "-", "-", "-", "-", "20", "19", "16", "13", "10", "7",
    ],
    [
        "-", "-", "-", "-", "-", "-", "-", "20", "19", "16", "13", "10",
    ],
    [
        "-", "-", "-", "-", "-", "-", "-", "-", "20", "19", "16", "13",
    ],
];

const UNDEAD: [&str; 13] = [
    "Skeleton or 1 HD:",
    "Zombie:",
    "Ghoul or 2 HD:",
    "Shadow or 3-4 HD:",
    "Wight or 5 HD:",
    "Ghast:",
    "Wraith or 6 HD:",
    "Mummy or 7 HD:",
    "Spectre or 8 HD:",
    "Vampire or 9 HD:",
    "Ghost or 10 HD:",
    "Lich or 11+ HD:",
    "Special:",
];

/// Turning Undead (0x5a819e), absent when the character cannot turn.
pub(in crate::sheet) fn turning(ctx: &Ctx) -> Option<Section> {
    let level = turn_level(ctx);
    if level == 0 {
        return None;
    }
    let column = match level {
        ..=0 => None,
        1..=10 => Some(level - 1),
        11 | 12 => Some(level - 2),
        13 => Some(10),
        _ => Some(11),
    };
    // 2.0 right-aligns the results and words the legend differently,
    // beside the last undead.
    let result = |t: usize| {
        let cell = Cell::new(column.map_or("-", |c| TURNING[t][c as usize]));
        if ctx.expansion() { cell } else { cell.right() }
    };
    let mut rows: Vec<Vec<Cell>> = [[0, 4, 7, 10], [1, 5, 8, 11], [2, 6, 9, 12]]
        .iter()
        .map(|types| {
            types
                .iter()
                .flat_map(|&t| [Cell::new(UNDEAD[t]), result(t)])
                .collect()
        })
        .collect();
    if !ctx.expansion() {
        rows.push(vec![
            Cell::new(UNDEAD[3]),
            result(3),
            Cell::new("# = Roll less than # on a 1d20 to turn 2d6 undead.").span(3),
            Cell::new("T = Automatically turns 2d6 undead.").span(3),
        ]);
        rows.push(vec![
            Cell::new("D = Automatically destroys 2d6 undead.").span(4),
            Cell::new("D* = Automatically destroys 2d6+2d4 undead.").span(4),
        ]);
        return Some(single(ids::TURNING, "Turning Undead", vec![table(8, rows)]));
    }
    rows.push(vec![Cell::new(UNDEAD[3]), result(3).span(7)]);
    rows.push(vec![
        Cell::new("# = Roll # or greater on a 1d20 to turn 2d6 undead.").span(4),
        Cell::new("D = Automatically destroys 2d6 undead.").span(4),
    ]);
    rows.push(vec![
        Cell::new("T = Automatically turns 2d6 undead.").span(4),
        Cell::new("D* = Automatically destroys 2d6+2d4 undead.").span(4),
    ]);
    Some(single(ids::TURNING, "Turning Undead", vec![table(8, rows)]))
}

/// The Skills & Powers movement bonus of Stamina (0x42083c).
const STAMINA_MOVEMENT: [i32; 26] = [
    0, -5, -3, -3, -2, -2, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 3, 3, 4, 4, 5, 6, 7,
];

/// The base land movement rate (0x42b801): the race's; with Skills &
/// Powers, plus the Stamina bonus and the Balance reaction adjustment
/// when positive.
fn base_movement(ctx: &Ctx) -> i32 {
    let mut rate = ctx.race().map_or(12, |r| r.movement);
    if ctx.sp {
        let stamina = ctx.scores.sub(STR, STAMINA);
        let bonus = match ctx.scores.exceptional_for(stamina) {
            Some(1..=50) => 1,
            Some(51..=99) => 2,
            Some(100) => 3,
            _ => at(&STAMINA_MOVEMENT, stamina),
        };
        rate += bonus.max(0);
        rate += at(
            &tables::REACTION_ADJUSTMENT,
            ctx.scores.sub(super::DEX, BALANCE),
        )
        .max(0);
    }
    rate
}

/// The movement rate of each category, Unencumbered to Severe.
fn rates(ctx: &Ctx) -> [i32; 5] {
    let b = base_movement(ctx) as f32;
    let (light, heavy) = if ctx.sp {
        (0.75, 0.25)
    } else {
        (2.0 / 3.0, 1.0 / 3.0)
    };
    [
        b as i32,
        (b * light) as i32,
        (b / 2.0) as i32,
        (b * heavy) as i32,
        1,
    ]
}

/// Movement and Encumbrance (0x5a9cb4).
pub(in crate::sheet) fn movement(ctx: &Ctx) -> Section {
    let limits = limits(ctx);
    let rates = rates(ctx);
    let mut rows = vec![{
        let mut r = vec![Cell::new("Encumbrance:")];
        r.extend(
            ["Unencumbered", "Light", "Moderate", "Heavy", "Severe"]
                .iter()
                .map(|t| Cell::new(*t).center()),
        );
        r
    }];
    let mut weights = vec![Cell::new("Weight (lbs):")];
    let mut lo = 0;
    for hi in limits {
        weights.push(
            Cell::new(if lo == hi {
                hi.to_string()
            } else {
                format!("{lo}-{hi}")
            })
            .center(),
        );
        lo = hi + 1;
    }
    rows.push(weights);
    let mut movement = vec![Cell::new("Movement:")];
    movement.extend(rates.iter().map(|r| Cell::new(r.to_string()).center()));
    rows.push(movement);
    let mut thac0 = vec![Cell::new("THAC0:")];
    thac0.extend(
        ["", "", "-1", "-2", "-4"]
            .iter()
            .map(|t| Cell::new(*t).center()),
    );
    rows.push(thac0);
    let mut ac = vec![Cell::new("AC:")];
    ac.extend(
        ["", "", "", "+1", "+3"]
            .iter()
            .map(|t| Cell::new(*t).center()),
    );
    rows.push(ac);
    let category = encumbrance(ctx);
    let current = rates.get(category).copied().unwrap_or(0);
    rows.push(vec![
        Cell::new(format!(
            "Currently carrying {:.2} pounds ({} Encumbrance, {current} Movement)",
            carried(ctx),
            ENCUMBRANCE[category]
        ))
        .span(6),
    ]);
    single(
        ids::MOVEMENT,
        "Movement and Encumbrance",
        vec![table(6, rows)],
    )
}
