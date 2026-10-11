//! The ability tables the sheet prints, as the originals have them built
//! in (Player's Handbook, Tables 1 to 6). Every table has 26 entries, one
//! per score 0 to 25; the Strength tables have five more for 18/01-50,
//! 18/51-75, 18/76-90, 18/91-99 and 18/00.

/// A Strength table: scores 0 to 25, then the exceptional brackets.
type StrengthTable = ([i32; 26], [i32; 5]);
/// Open doors and locked doors per Strength, exceptional brackets apart.
type OpenDoorsTable = ([(i32, i32); 26], [(i32, i32); 5]);

pub const WEIGHT_ALLOWANCE: StrengthTable = (
    [
        0, 1, 1, 5, 10, 10, 20, 20, 35, 35, 40, 40, 45, 45, 55, 55, 70, 85, 110, 485, 535, 635,
        785, 935, 1235, 1535,
    ],
    [135, 160, 185, 235, 335],
);
pub const ATTACK_ADJUSTMENT: StrengthTable = (
    [
        0, -5, -3, -3, -2, -2, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 3, 3, 4, 4, 5, 6, 7,
    ],
    [1, 2, 2, 2, 3],
);
pub const DAMAGE_ADJUSTMENT: StrengthTable = (
    [
        0, -4, -2, -1, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 7, 8, 9, 10, 11, 12, 14,
    ],
    [3, 3, 4, 5, 6],
);
pub const MAXIMUM_PRESS: StrengthTable = (
    [
        0, 3, 5, 10, 25, 25, 55, 55, 90, 90, 115, 115, 140, 140, 170, 170, 195, 220, 255, 640, 700,
        810, 970, 1130, 1440, 1535,
    ],
    [280, 305, 330, 380, 480],
);
/// Open doors; a second number (locked doors) in parentheses, 0 for none.
pub const OPEN_DOORS: OpenDoorsTable = (
    [
        (0, 0),
        (1, 0),
        (1, 0),
        (2, 0),
        (3, 0),
        (3, 0),
        (4, 0),
        (4, 0),
        (5, 0),
        (5, 0),
        (6, 0),
        (6, 0),
        (7, 0),
        (7, 0),
        (8, 0),
        (8, 0),
        (9, 0),
        (10, 0),
        (11, 0),
        (16, 8),
        (17, 10),
        (17, 12),
        (18, 14),
        (18, 16),
        (19, 17),
        (19, 18),
    ],
    [(12, 0), (13, 0), (14, 0), (15, 3), (16, 6)],
);
pub const BEND_BARS: StrengthTable = (
    [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 2, 4, 4, 7, 7, 10, 13, 16, 50, 60, 70, 80, 90, 95, 99,
    ],
    [20, 25, 30, 35, 40],
);

pub const MISSILE_ADJUSTMENT: [i32; 26] = [
    0, -6, -4, -3, -2, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 2, 3, 3, 4, 4, 4, 5, 5,
];
pub const PICK_POCKETS: [i32; 26] = [
    0, -35, -30, -30, -25, -25, -20, -20, -15, -15, -10, -5, 0, 0, 0, 0, 0, 5, 10, 15, 20, 20, 25,
    25, 30, 30,
];
pub const OPEN_LOCKS: [i32; 26] = [
    0, -35, -30, -30, -25, -20, -20, -15, -15, -10, -5, 0, 0, 0, 0, 0, 5, 10, 15, 20, 20, 25, 25,
    30, 30, 35,
];
pub const REACTION_ADJUSTMENT: [i32; 26] = [
    0, -6, -4, -3, -2, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 2, 3, 3, 4, 4, 5, 5, 5,
];
pub const DEFENSIVE_ADJUSTMENT: [i32; 26] = [
    0, 5, 5, 4, 3, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, -1, -2, -3, -4, -4, -4, -5, -5, -6, -6, -6,
];
pub const MOVE_SILENTLY: [i32; 26] = [
    0, -35, -35, -30, -30, -30, -25, -25, -20, -20, -15, -10, -5, 0, 0, 0, 0, 5, 10, 15, 15, 20,
    20, 25, 25, 30,
];
pub const CLIMB_WALLS: [i32; 26] = [
    0, -35, -30, -30, -25, -20, -20, -15, -15, -10, -5, 0, 0, 0, 0, 0, 0, 5, 10, 15, 20, 20, 25,
    25, 30, 30,
];
pub const FIND_TRAPS: [i32; 26] = [
    0, -50, -45, -40, -35, -30, -25, -20, -15, -10, -10, -5, 0, 0, 0, 0, 0, 0, 5, 10, 15, 20, 20,
    30, 35, 40,
];
pub const HIDE_IN_SHADOWS: [i32; 26] = [
    0, -50, -45, -40, -35, -30, -25, -20, -15, -10, -5, 0, 0, 0, 0, 0, 0, 5, 10, 15, 20, 25, 30,
    35, 40, 45,
];
pub const TUNNELING: [i32; 26] = HIDE_IN_SHADOWS;
pub const ESCAPING_BONDS: [i32; 26] = [
    0, -55, -50, -45, -40, -35, -30, -25, -20, -15, -10, -5, 0, 0, 0, 0, 0, 5, 10, 15, 20, 25, 30,
    35, 40, 45,
];

pub const SYSTEM_SHOCK: [i32; 26] = [
    0, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 88, 90, 95, 97, 99, 99, 99, 99, 99, 99,
    99, 100,
];
pub const POISON_SAVE: [i32; 26] = [
    0, -2, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4,
];
/// The dwarves' (and halflings', gnomes') saving throw bonus for
/// Constitution, 1 per 3.5 points.
pub const CONSTITUTION_SAVE_BONUS: [i32; 26] = [
    0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5,
];
pub const HIT_POINT_ADJUSTMENT: [i32; 26] = [
    0, -3, -2, -2, -1, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
];
pub const WARRIOR_HIT_POINT_ADJUSTMENT: [i32; 26] = [
    0, -3, -2, -2, -1, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 5, 6, 6, 6, 7, 7,
];
pub const MINIMUM_HIT_DIE_ROLL: [i32; 26] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 3, 4, 4, 4,
];
pub const RESURRECTION: [i32; 26] = [
    0, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90, 92, 94, 96, 98, 100, 100, 100, 100, 100,
    100, 100, 100,
];

pub const MAXIMUM_SPELL_LEVEL: [i32; 26] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9,
];
/// Maximum wizard spells per level; `i32::MAX` prints "All".
pub const MAXIMUM_SPELLS: [i32; 26] = [
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    6,
    7,
    7,
    7,
    9,
    9,
    11,
    11,
    14,
    18,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
];
pub const BONUS_PROFICIENCIES: [i32; 26] = [
    0, 0, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 3, 3, 4, 4, 5, 6, 7, 8, 9, 10, 11, 12, 15, 20,
];
pub const LEARN_SPELL: [i32; 26] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 35, 40, 45, 50, 55, 60, 65, 70, 75, 85, 95, 96, 97, 98, 99, 100, 100,
];
pub const SPELL_FAILURE: [i32; 26] = [
    0, 80, 60, 50, 45, 40, 35, 30, 25, 20, 15, 10, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
pub const MAGIC_DEFENSE: [i32; 26] = [
    0, -6, -4, -3, -2, -1, -1, -1, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 4, 4, 4, 4, 4, 4, 4,
];
pub const LOYALTY_BASE: [i32; 26] = [
    0, -8, -7, -6, -5, -4, -3, -2, -1, 0, 0, 0, 0, 0, 1, 3, 4, 6, 8, 10, 12, 14, 16, 18, 20, 20,
];
pub const HENCHMEN: [i32; 26] = [
    0, 0, 1, 1, 1, 2, 2, 3, 3, 4, 4, 4, 5, 5, 6, 7, 8, 10, 15, 20, 25, 30, 35, 40, 45, 50,
];
pub const INITIAL_REACTION: [i32; 26] = [
    0, -7, -6, -5, -4, -3, -2, -1, 0, 0, 0, 0, 0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
];

/// Bonus clerical spells for Wisdom 13 to 25, spell levels 1 to 7 (rows
/// below 13 are all 0).
pub const BONUS_CLERICAL_SPELLS: [[i32; 7]; 13] = [
    [1, 0, 0, 0, 0, 0, 0],
    [2, 0, 0, 0, 0, 0, 0],
    [2, 1, 0, 0, 0, 0, 0],
    [2, 2, 0, 0, 0, 0, 0],
    [2, 2, 1, 0, 0, 0, 0],
    [2, 2, 1, 1, 0, 0, 0],
    [3, 2, 2, 1, 0, 0, 0],
    [3, 3, 2, 2, 0, 0, 0],
    [3, 3, 3, 2, 1, 0, 0],
    [3, 3, 3, 3, 2, 0, 0],
    [4, 3, 3, 3, 2, 1, 0],
    [4, 3, 3, 3, 3, 2, 0],
    [4, 3, 3, 3, 3, 3, 1],
];

/// Looks a score up, clamped to the table.
pub fn at(table: &[i32; 26], score: i32) -> i32 {
    table[score.clamp(0, 25) as usize]
}

/// The bracket of an exceptional Strength (1 to 100): 0 for 18/01-50 up
/// to 4 for 18/00.
pub fn exceptional_bracket(exceptional: i32) -> Option<usize> {
    match exceptional {
        1..=50 => Some(0),
        51..=75 => Some(1),
        76..=90 => Some(2),
        91..=99 => Some(3),
        100 => Some(4),
        _ => None,
    }
}

/// Looks Strength up: the exceptional brackets apply only to a score of
/// 18 that can be exceptional, and 18/00 with no percentage reads as 18.
pub fn strength<T: Copy>(table: &([T; 26], [T; 5]), score: i32, exceptional: Option<i32>) -> T {
    if score == 18
        && let Some(bracket) = exceptional.and_then(exceptional_bracket)
    {
        return table.1[bracket];
    }
    table.0[score.clamp(0, 25) as usize]
}

/// Saving throws per class group and level, for Core Rules 2.0, which has
/// them built in (Player's Handbook, Table 60): paralyzation/poison/death,
/// rod/staff/wand, petrification/polymorph, breath weapon, spell. The
/// tables are built on the stack in 0x41a3f0 (psionicists have their own,
/// from The Complete Psionics Handbook); level 0 reads 16/18/17/20/19.
pub fn core20_save(group: i32, level: u32) -> [i32; 5] {
    if level == 0 {
        return [16, 18, 17, 20, 19];
    }
    let row = |rows: &[[i32; 5]], steps: &[u32]| {
        let i = steps.iter().filter(|&&s| level >= s).count();
        rows[i.min(rows.len() - 1)]
    };
    match group {
        0 => row(
            &[
                [16, 18, 17, 20, 19],
                [14, 16, 15, 17, 17],
                [13, 15, 14, 16, 16],
                [11, 13, 12, 13, 14],
                [10, 12, 11, 12, 13],
                [8, 10, 9, 9, 11],
                [7, 9, 8, 8, 10],
                [5, 7, 6, 5, 8],
                [4, 6, 5, 4, 7],
                [3, 5, 4, 4, 6],
            ],
            &[1, 3, 5, 7, 9, 11, 13, 15, 17],
        ),
        4 => row(
            &[
                [13, 15, 12, 16, 15],
                [12, 13, 10, 15, 14],
                [11, 11, 8, 13, 12],
                [10, 9, 7, 12, 11],
                [9, 7, 6, 11, 9],
                [8, 5, 5, 9, 7],
            ],
            &[5, 9, 13, 17, 21],
        ),
        1 => row(
            &[
                [13, 14, 12, 16, 15],
                [12, 12, 11, 15, 13],
                [11, 10, 10, 14, 11],
                [10, 8, 9, 13, 9],
                [9, 6, 8, 12, 7],
                [8, 4, 7, 11, 5],
            ],
            &[5, 9, 13, 17, 21],
        ),
        2 => row(
            &[
                [10, 14, 13, 16, 15],
                [9, 13, 12, 15, 14],
                [7, 11, 10, 13, 12],
                [6, 10, 9, 12, 11],
                [5, 9, 8, 11, 10],
                [4, 8, 7, 10, 9],
                [2, 6, 5, 8, 7],
            ],
            &[4, 7, 10, 13, 16, 19],
        ),
        _ => row(
            &[
                [14, 11, 13, 15, 12],
                [13, 9, 11, 13, 10],
                [11, 7, 9, 11, 8],
                [10, 5, 7, 9, 6],
                [8, 3, 5, 7, 4],
            ],
            &[6, 11, 16, 21],
        ),
    }
}

/// THAC0 per class group and level (capped at 20), for Core Rules 2.0
/// (0x4134a0): warriors 21 - level, priests 2 per 3 levels, rogues and
/// psionicists 1 per 2, wizards 1 per 3.
pub fn core20_thac0(group: i32, level: u32) -> i32 {
    let l = level.min(20) as i32;
    if l == 0 {
        return 20;
    }
    match group {
        0 => (21 - l).min(20),
        2 => 20 - 2 * ((l - 1) / 3),
        1 | 4 => 20 - (l - 1) / 2,
        _ => 20 - (l - 1) / 3,
    }
}

/// An ordinal: 1st, 2nd, 3rd, 4th ... 11th, 12th, 13th, 21st.
pub fn ordinal(n: i32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// A number with thousands separators (`64,000`).
pub fn thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_as_the_sheet_prints_them() {
        assert_eq!(thousands(64_000), "64,000");
        assert_eq!(thousands(1535), "1,535");
        assert_eq!(thousands(999), "999");
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(12), "12th");
        assert_eq!(ordinal(22), "22nd");
    }

    #[test]
    fn strength_brackets() {
        assert_eq!(strength(&WEIGHT_ALLOWANCE, 18, Some(76)), 185);
        assert_eq!(strength(&WEIGHT_ALLOWANCE, 18, Some(100)), 335);
        assert_eq!(strength(&WEIGHT_ALLOWANCE, 18, Some(0)), 110);
        assert_eq!(strength(&WEIGHT_ALLOWANCE, 18, None), 110);
        assert_eq!(strength(&WEIGHT_ALLOWANCE, 17, Some(76)), 85);
    }

    #[test]
    fn core20_tables() {
        assert_eq!(core20_thac0(3, 10), 17);
        assert_eq!(core20_thac0(2, 8), 16);
        assert_eq!(core20_thac0(1, 5), 18);
        assert_eq!(core20_thac0(0, 1), 20);
        assert_eq!(core20_save(2, 8), [7, 11, 10, 13, 12]);
        assert_eq!(core20_save(0, 6), [11, 13, 12, 13, 14]);
    }
}
