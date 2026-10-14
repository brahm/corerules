//! The Table Rolls list and what each table does, from the originals'
//! code (Expansion addresses): the list per book (0x52e86d and the
//! launchers 0x52f620), the per-table code of the Roll Results dialog
//! (0x50c88e, 0x51d5fd) and the id to file map (0x534d15).
//!
//! Only ids, dice, ranges and column labels live here; every text a roll
//! prints comes from the installation's `Tables/` and `Books/`.

use super::{
    Branch, Col, Critical, Dice, Extra, ListEntry, Options, ParamSpec, RollSpec, Step, TableBook,
    Title, d,
};

/// Table id, file name (without `.tbl`) and the number of records the
/// program reads (header included), as 0x534d15 maps them. Core Rules 2.0
/// (0x5c1160) differs only for 55400 (see `id_map`).
pub const ID_MAP: &[(i32, &str, usize)] = &[
    (10011, "PHB11", 7),
    (10019, "PHB19", 26),
    (10031, "PHB31", 17),
    (10036, "PHB36", 26),
    (10043, "PHB43", 5),
    (11010, "PHB10HM", 7),
    (11016, "PHB16A", 6),
    (12010, "PHB10HF", 7),
    (12016, "PHB16B", 6),
    (13010, "PHB10WM", 7),
    (13016, "PHB16C", 8),
    (14010, "PHB10WF", 7),
    (20047, "DMG47", 14),
    (20054, "DMG54", 10),
    (20084, "DMG84", 27),
    (20085, "DMG85", 7),
    (20086, "DMG86", 7),
    (20087, "DMG87", 13),
    (20088, "DMG88", 19),
    (20092, "DMG92", 15),
    (20093, "DMG93", 15),
    (20094, "DMG94", 21),
    (20095, "DMG95", 19),
    (20097, "DMG97", 17),
    (20098, "DMG98", 20),
    (20099, "DMG99", 14),
    (20100, "DMG100", 18),
    (20101, "DMG101", 21),
    (20102, "DMG102", 20),
    (20103, "DMG103", 21),
    (20105, "DMG105", 14),
    (20106, "DMG106", 7),
    (20107, "DMG107", 11),
    (20111, "DMG111", 10),
    (20113, "DMG113", 7),
    (20114, "DMG114", 10),
    (20115, "DMG115", 13),
    (20116, "DMG116", 18),
    (21026, "DMG26A", 11),
    (21043, "DMG43A", 21),
    (21070, "DMG70A", 21),
    (21079, "DMG79A", 8),
    (21089, "DMG89_A", 18),
    (21090, "DMG90_A", 18),
    (21091, "DMG91_A", 19),
    (21096, "DMG96_A", 20),
    (21104, "DMG104_A", 18),
    (21108, "DMG108_A", 17),
    (21109, "DMG109S", 7),
    (21110, "DMG110_A", 18),
    (21117, "DMG117A", 11),
    (21999, "DMG999A", 9),
    (22026, "DMG26B", 11),
    (22043, "DMG43B", 21),
    (22070, "DMG70B", 101),
    (22079, "DMG79B", 8),
    (22089, "DMG89_B", 20),
    (22090, "DMG90_B", 19),
    (22091, "DMG91_B", 20),
    (22096, "DMG96_B", 21),
    (22104, "DMG104_B", 17),
    (22108, "DMG108_B", 10),
    (22109, "DMG109O", 7),
    (22110, "DMG110_B", 17),
    (22117, "DMG117B", 8),
    (22999, "DMG999B", 9),
    (23079, "DMG79C", 8),
    (23089, "DMG89_C", 19),
    (23090, "DMG90_C", 18),
    (23110, "DMG110_C", 12),
    (23999, "DMG999C", 9),
    (24110, "DMG110_D", 16),
    (30001, "POSP1", 21),
    (30025, "POSP25", 21),
    (30031, "POSP31", 21),
    (30040, "POSP40", 5),
    (30041, "POSP41", 31),
    (30042, "POSP42", 21),
    (31083, "POSP83A", 58),
    (32083, "POSP83B", 20),
    (40014, "POSM14", 21),
    (40015, "POSM15", 21),
    (40023, "POSM23", 10),
    (40024, "POSM24", 21),
    (40025, "POSM25", 11),
    (40044, "POSM44", 21),
    (40046, "POSM46J", 4),
    (40048, "POSM48", 21),
    (40049, "POSMHit", 21),
    (41046, "POSM46A", 8),
    (41100, "POSM100A", 12),
    (41101, "POSM101A", 12),
    (41102, "POSM102A", 12),
    (41103, "POSM103A", 12),
    (41104, "POSM104A", 12),
    (41105, "POSM105A", 12),
    (41106, "POSM106A", 12),
    (41107, "POSM107A", 12),
    (41108, "POSM108A", 12),
    (41109, "POSM109A", 12),
    (42046, "POSM46B", 9),
    (42100, "POSM100B", 12),
    (42101, "POSM101B", 12),
    (42102, "POSM102B", 12),
    (42103, "POSM103B", 12),
    (42104, "POSM104B", 12),
    (42105, "POSM105B", 12),
    (42106, "POSM106B", 12),
    (42107, "POSM107B", 12),
    (42108, "POSM108B", 12),
    (42109, "POSM109B", 12),
    (43046, "POSM46C", 5),
    (43100, "POSM100C", 12),
    (43101, "POSM101C", 12),
    (43102, "POSM102C", 12),
    (43103, "POSM103C", 12),
    (43104, "POSM104C", 12),
    (43105, "POSM105C", 12),
    (43106, "POSM106C", 12),
    (43107, "POSM107C", 12),
    (43108, "POSM108C", 12),
    (43109, "POSM109C", 12),
    (44046, "POSM46D", 2),
    (44100, "POSM100D", 12),
    (44101, "POSM101D", 12),
    (44102, "POSM102D", 12),
    (44103, "POSM103D", 12),
    (44104, "POSM104D", 12),
    (44105, "POSM105D", 12),
    (44106, "POSM106D", 12),
    (44107, "POSM107D", 12),
    (44108, "POSM108D", 12),
    (44109, "POSM109D", 12),
    (45046, "POSM46E", 4),
    (45100, "POSM100E", 12),
    (45101, "POSM101E", 12),
    (45102, "POSM102E", 12),
    (45103, "POSM103E", 12),
    (45104, "POSM104E", 12),
    (45105, "POSM105E", 12),
    (45106, "POSM106E", 12),
    (45107, "POSM107E", 12),
    (45108, "POSM108E", 12),
    (45109, "POSM109E", 12),
    (46046, "POSM46F", 5),
    (46100, "POSM100F", 8),
    (46101, "POSM101F", 8),
    (46102, "POSM102F", 8),
    (46103, "POSM103F", 8),
    (46104, "POSM104F", 8),
    (46105, "POSM105F", 8),
    (46106, "POSM106F", 8),
    (46107, "POSM107F", 8),
    (46108, "POSM108F", 8),
    (46109, "POSM109F", 8),
    (47046, "POSM46G", 2),
    (48046, "POSM46H", 3),
    (49046, "POSM46I", 3),
    (50018, "DMO18", 6),
    (50029, "DMO29", 11),
    (50900, "POCT1A", 8),
    (50901, "POCT1B", 9),
    (50902, "POCT1C", 9),
    (50903, "POCT1D", 5),
    (51200, "POCT200A", 12),
    (51201, "POCT201A", 12),
    (51202, "POCT202A", 12),
    (51300, "POCT203A", 12),
    (51301, "POCT204A", 12),
    (51302, "POCT205A", 12),
    (51400, "POCT206A", 12),
    (51401, "POCT207A", 12),
    (51402, "POCT208A", 12),
    (52200, "POCT200B", 12),
    (52201, "POCT201B", 8),
    (52202, "POCT202B", 8),
    (52300, "POCT203B", 12),
    (52301, "POCT204B", 8),
    (52302, "POCT205B", 8),
    (52400, "POCT206B", 12),
    (52401, "POCT207B", 8),
    (52402, "POCT208B", 8),
    (53200, "POCT200C", 12),
    (53201, "POCT201C", 12),
    (53202, "POCT202C", 12),
    (53300, "POCT203C", 12),
    (53301, "POCT204C", 12),
    (53302, "POCT205C", 12),
    (53400, "POCT206C", 12),
    (53401, "POCT207C", 12),
    (53402, "POCT208C", 12),
    (54200, "POCT200D", 12),
    (54201, "POCT201D", 12),
    (54202, "POCT202D", 12),
    (54300, "POCT203D", 12),
    (54301, "POCT204D", 12),
    (54302, "POCT205D", 12),
    (54400, "POCT206D", 12),
    (54401, "POCT207D", 12),
    (54402, "POCT208D", 12),
    (55200, "POCT200E", 12),
    (55201, "POCT201E", 12),
    (55202, "POCT202E", 12),
    (55300, "POCT203E", 12),
    (55301, "POCT204E", 12),
    (55302, "POCT205E", 12),
    (55400, "POCT206E", 12),
    (55401, "POCT208A", 12),
    (55402, "POCT208E", 12),
    (90001, "Sub89A", 11),
    (90002, "Sub89B", 7),
    (90003, "Sub89C", 8),
    (90004, "Sub89D", 9),
    (90005, "Sub89E", 5),
    (90006, "Sub89F", 13),
    (90007, "Sub90A", 6),
    (90008, "Sub90B", 8),
    (90009, "Sub91A", 8),
    (90010, "Sub91B", 7),
    (90011, "Sub91C", 8),
    (90012, "Sub91D", 3),
    (90013, "Sub91E", 5),
    (90015, "Sub93A", 7),
    (90016, "Sub96A", 6),
    (90017, "Sub100A", 5),
    (90018, "Sub95A", 4),
    (90020, "Sub110A", 6),
    (90021, "Sub110B", 5),
    (90022, "Sub110C", 5),
    (90023, "Sub110D", 6),
    (90024, "Sub110E", 11),
    (90025, "Sub98C", 5),
    (90026, "Sub98D", 8),
    (90027, "Sub99A", 7),
    (90028, "Sub100B", 13),
    (90029, "Sub91G", 9),
    (90030, "Sub100C", 4),
    (90031, "Sub95B", 5),
    (90032, "Sub96B", 5),
    (90033, "Sub96C", 6),
    (90034, "Sub96D", 7),
    (90035, "Sub96E", 11),
    (90036, "Sub96F", 5),
    (90037, "Sub96G", 7),
    (90038, "Sub97A", 6),
    (90039, "Sub97B", 14),
    (90040, "Sub101A", 5),
    (90041, "Sub102A", 5),
    (90042, "Sub103A", 5),
    (90043, "Sub104A", 8),
    (90044, "Sub104B", 7),
    (90045, "Sub107A", 6),
    (90047, "Sub110G", 6),
    (90048, "Sub100D", 4),
    (91019, "Sub98A", 9),
    (92019, "Sub98B", 9),
];

/// The Books combo of the Table Rolls dialog, in its (sorted) order. The
/// dialog opens on the Dungeon Master Guide.
pub const TABLE_BOOKS: [TableBook; 6] = [
    TableBook::HighLevel,
    TableBook::Dmg,
    TableBook::Phb,
    TableBook::CombatTactics,
    TableBook::SkillsPowers,
    TableBook::SpellsMagic,
];

const RESULT: &[Col] = &[Col::Name("Result")];
const ITEM_XP: &[Col] = &[Col::Name("Item"), Col::Xp];

/// A roll titled with the book's table `number`.
const fn on(id: i32, dice: Dice, number: u16, cols: &'static [Col]) -> Step {
    Step::Roll(RollSpec::new(id, dice, Title::Table(number), cols))
}

/// A roll that continues the block above (no new title).
const fn then(id: i32, dice: Dice, cols: &'static [Col]) -> Step {
    Step::Roll(RollSpec::new(id, dice, Title::None, cols))
}

/// A magic item table: d20, item and experience value, sub-subtables in
/// v4 or v5.
const fn items(id: i32, number: u16) -> Step {
    on(id, d(1, 20), number, ITEM_XP)
}

const fn branch(hi: i32, label: &'static str, steps: &'static [Step]) -> Branch {
    Branch { hi, label, steps }
}

const fn entry(book: TableBook, number: u16, steps: &'static [Step]) -> ListEntry {
    ListEntry {
        book,
        number: Some(number),
        label: None,
        params: &[],
        steps,
    }
}

const fn with_params(
    book: TableBook,
    number: Option<u16>,
    label: Option<&'static str>,
    params: &'static [ParamSpec],
    steps: &'static [Step],
) -> ListEntry {
    ListEntry {
        book,
        number,
        label,
        params,
        steps,
    }
}

/// Worked out in the original's code; not rolled here.
const CODE_ONLY: &[Step] = &[];

// DMG Table 26: nags and average horses, then high-spirited ones when the
// first table says so (v3).
const HORSE_A: &[Step] = &[on(21026, d(1, 10), 26, &[Col::Name("Trait")])];
const HORSE_B: &[Step] = &[Step::Roll(
    RollSpec::new(22026, d(1, 10), Title::None, &[Col::Name("Trait")])
        .subtitle("High-spirited horses and chargers"),
)];

// Punching and Wrestling (DMG 43, PHB 58).
const PUNCH_COLS: &[Col] = &[Col::Name("Punch"), Col::Item1("Damage"), Col::Item2("%KO")];
const WRESTLE_COLS: &[Col] = &[
    Col::Name("Wrestle"),
    Col::Item1("Damage"),
    Col::Item2("%KO"),
];
const PUNCHING: &[Step] = &[
    Step::Roll(RollSpec::new(21043, d(1, 20), Title::Entry, PUNCH_COLS)),
    then(22043, d(1, 20), WRESTLE_COLS),
];

// DMG 89 to 110: the magical items, with the "(D6)" and "(D10)" subtable
// choices of 0x51d5fd.
const T89: &[Step] = &[Step::Choose {
    die: 6,
    branches: &[
        branch(2, "Subtable A (1-2)", &[items(21089, 89)]),
        branch(4, "Subtable B (3-4)", &[items(22089, 89)]),
        branch(6, "Subtable C (5-6)", &[items(23089, 89)]),
    ],
}];
const SCROLL_COLS: &[Col] = &[Col::Name("Scroll"), Col::Item1("Level Range")];
const T90: &[Step] = &[Step::Choose {
    die: 6,
    branches: &[
        branch(
            4,
            "Subtable A (1-4)",
            &[Step::Choose {
                die: 100,
                branches: &[
                    branch(
                        70,
                        "Wizard scroll",
                        &[Step::Roll(
                            RollSpec::new(21090, d(1, 20), Title::Table(90), SCROLL_COLS)
                                .extra(Extra::Scroll(true)),
                        )],
                    ),
                    branch(
                        100,
                        "Priest scroll",
                        &[Step::Roll(
                            RollSpec::new(23090, d(1, 20), Title::Table(90), SCROLL_COLS)
                                .extra(Extra::Scroll(false)),
                        )],
                    ),
                ],
            }],
        ),
        branch(6, "Subtable B (5-6)", &[items(22090, 90)]),
    ],
}];
const T91: &[Step] = &[Step::Choose {
    die: 6,
    branches: &[
        branch(4, "Subtable A (1-4)", &[items(21091, 91)]),
        branch(6, "Subtable B (5-6)", &[items(22091, 91)]),
    ],
}];
const T96: &[Step] = &[Step::Choose {
    die: 6,
    branches: &[
        branch(3, "Subtable A (1-3)", &[items(21096, 96)]),
        branch(6, "Subtable B (4-6)", &[items(22096, 96)]),
    ],
}];
const T104: &[Step] = &[Step::Choose {
    die: 6,
    branches: &[
        branch(3, "Subtable A (1-3)", &[items(21104, 104)]),
        branch(6, "Subtable B (4-6)", &[items(22104, 104)]),
    ],
}];
const T105: &[Step] = &[on(20105, d(1, 20), 105, &[Col::Name("Armor")])];
const T106: &[Step] = &[on(
    20106,
    d(1, 20),
    106,
    &[Col::SignedV4("AC Adjustment"), Col::Xp],
)];
const T107: &[Step] = &[on(
    20107,
    d(1, 20),
    107,
    &[Col::Name("Armor Type"), Col::Xp],
)];
const WEAPON: &[Col] = &[Col::Name("Weapon")];
const T108: &[Step] = &[Step::Choose {
    die: 6,
    branches: &[
        branch(2, "Subtable A (1-2)", &[on(21108, d(1, 20), 108, WEAPON)]),
        branch(6, "Subtable B (3-6)", &[on(22108, d(1, 20), 108, WEAPON)]),
    ],
}];
const T109_SWORD: &[Step] = &[on(
    21109,
    d(1, 20),
    109,
    &[Col::SignedV4("Sword Adjustment"), Col::Xp],
)];
const T109_OTHER: &[Step] = &[on(
    22109,
    d(1, 20),
    109,
    &[Col::SignedV4("Other Weapon Adjustment"), Col::Xp],
)];
const T110: &[Step] = &[Step::Choose {
    die: 10,
    branches: &[
        branch(3, "Subtable A (1-3)", &[items(21110, 110)]),
        branch(6, "Subtable B (4-6)", &[items(22110, 110)]),
        branch(9, "Subtable C (7-9)", &[items(23110, 110)]),
        branch(10, "Subtable D (10)", &[items(24110, 110)]),
    ],
}];
const T115: &[Step] = &[on(20115, d(1, 100), 115, &[Col::Name("Ability")])];
const T116: &[Step] = &[on(20116, d(1, 100), 116, &[Col::Name("Power")])];
const T117: &[Step] = &[
    on(21117, d(1, 100), 117, &[Col::Name("Purpose")]),
    then(22117, d(1, 100), &[Col::Name("Special Purpose Power")]),
];

/// Handlers of table ids reached by following an entry (the cases of
/// 0x51d5fd); other ids get a plain roll on their own die.
const HANDLERS: &[(i32, &[Step])] = &[
    (21026, HORSE_A),
    (22026, HORSE_B),
    (21043, PUNCHING),
    (20088, &[on(20088, d(1, 100), 88, RESULT)]),
    (21089, T89),
    (21090, T90),
    (21091, T91),
    (20092, &[items(20092, 92)]),
    (20093, &[items(20093, 93)]),
    (20094, &[items(20094, 94)]),
    (20095, &[items(20095, 95)]),
    (21096, T96),
    (20097, &[items(20097, 97)]),
    (20098, &[items(20098, 98)]),
    (20099, &[items(20099, 99)]),
    (20100, &[items(20100, 100)]),
    (20101, &[items(20101, 101)]),
    (20102, &[items(20102, 102)]),
    (20103, &[items(20103, 103)]),
    (21104, T104),
    (20105, T105),
    (20106, T106),
    (20107, T107),
    (21108, T108),
    (21109, T109_SWORD),
    (22109, T109_OTHER),
    (21110, T110),
    (20115, T115),
    (20116, T116),
    (21117, T117),
    (
        22117,
        &[on(
            22117,
            d(1, 100),
            117,
            &[Col::Name("Special Purpose Power")],
        )],
    ),
];

pub(super) fn handler(id: i32) -> Option<&'static [Step]> {
    HANDLERS.iter().find(|(i, _)| *i == id).map(|(_, s)| *s)
}

pub(super) fn is_table_id(id: i32) -> bool {
    ID_MAP.iter().any(|(i, _, _)| *i == id)
}

/// Columns of a plain roll on a table without a handler.
pub(super) fn cols_of(id: i32) -> &'static [Col] {
    let n = id % 1000;
    if (20000..30000).contains(&id) && (89..=110).contains(&n) {
        ITEM_XP
    } else {
        RESULT
    }
}

/// The book a table id belongs to.
pub(super) fn book_of(id: i32) -> TableBook {
    match id {
        10000..=19999 => TableBook::Phb,
        30000..=39999 => TableBook::SkillsPowers,
        40000..=49999 => TableBook::SpellsMagic,
        50000..=50899 => TableBook::HighLevel,
        50900..=59999 => TableBook::CombatTactics,
        _ => TableBook::Dmg,
    }
}

// Player's Handbook Tables 10 and 11, by race in the order of the tables'
// rows (Dwarf, Elf, Gnome, Half-elf, Halfling, Human); the dice of the
// code (0x50c88e), including its 4d10 for a halfling's weight.
const RACE: ParamSpec = ParamSpec {
    label: "Race",
    options: Options::Rows(11010),
};
const HEIGHT_DICE: &[Dice] = &[d(1, 10), d(1, 10), d(1, 6), d(2, 6), d(2, 8), d(2, 10)];
const WEIGHT_DICE: &[Dice] = &[d(4, 10), d(3, 10), d(5, 4), d(3, 12), d(4, 10), d(6, 10)];
const START_AGE: &[Dice] = &[d(5, 6), d(5, 6), d(3, 12), d(1, 6), d(3, 4), d(1, 4)];
const MAX_AGE: &[Dice] = &[
    d(2, 100).plus(250),
    d(4, 100).plus(350),
    d(3, 100).plus(200),
    d(3, 20).plus(125),
    d(1, 100).plus(105),
    d(2, 20).plus(90),
];

// The critical hit dialogs: location, then severity on the effect table
// (location code + weapon or spell type + target offset).
const SIZE_ADVANTAGE: ParamSpec = ParamSpec {
    label: "Size Advantage",
    options: Options::Fixed(&["None", "Defender", "Attacker"]),
};
const CALLED_SHOT: ParamSpec = ParamSpec {
    label: "Location",
    options: Options::CalledShot,
};
const LOCATION_DICE: &[Dice] = &[d(1, 10), d(1, 6), d(1, 6).plus(4)];
const SEVERITY_DICE: &[Dice] = &[d(1, 6), d(2, 4), d(2, 6), d(2, 8)];
const WEAPON_CRITICAL: Critical = Critical {
    targets: &[(50900, 0), (50901, 1), (50902, 2), (50903, 2), (50903, 1)],
    kinds: &[200, 300, 400],
    location_dice: LOCATION_DICE,
    severity_dice: SEVERITY_DICE,
    label: "Critical Hit Location and Effect",
};
const SPELL_CRITICAL: Critical = Critical {
    targets: &[(41046, 0), (42046, 0), (43046, 0)],
    kinds: &[100, 101, 102, 103, 104, 105, 106, 107, 108, 109],
    location_dice: LOCATION_DICE,
    severity_dice: SEVERITY_DICE,
    label: "Spell Hit Location and Effect",
};
const SPELL_LEVEL: ParamSpec = ParamSpec {
    label: "Spell Level",
    options: Options::Rows(40023),
};

use TableBook::{CombatTactics, Dmg, HighLevel, Phb, SkillsPowers, SpellsMagic};

/// The Tables list of each book, in the original's order.
pub const LIST: &[ListEntry] = &[
    // DM Option: High-Level Campaigns.
    entry(HighLevel, 14, CODE_ONLY),
    entry(HighLevel, 16, CODE_ONLY),
    entry(HighLevel, 17, CODE_ONLY),
    // Dungeon Master Guide.
    entry(Dmg, 26, HORSE_A),
    entry(Dmg, 43, PUNCHING),
    entry(Dmg, 47, CODE_ONLY),
    entry(
        Dmg,
        54,
        &[on(20054, d(2, 10), 54, &[Col::Name("Frequency")])],
    ),
    entry(
        Dmg,
        70,
        &[on(21070, d(1, 20), 70, &[Col::Name("General Trait")])],
    ),
    with_params(
        Dmg,
        Some(79),
        None,
        &[ParamSpec {
            label: "Season",
            options: Options::Fixed(&["Spring or fall", "Summer", "Winter"]),
        }],
        &[Step::ByParam {
            param: 0,
            options: &[
                &[on(
                    21079,
                    d(2, 6),
                    79,
                    &[Col::Name("Spring and Fall Conditions")],
                )],
                &[on(22079, d(2, 6), 79, &[Col::Name("Summer Conditions")])],
                &[on(23079, d(2, 6), 79, &[Col::Name("Winter Conditions")])],
            ],
        }],
    ),
    entry(Dmg, 84, CODE_ONLY),
    entry(Dmg, 85, &[on(20085, d(1, 100), 85, &[Col::Name("Gem")])]),
    entry(
        Dmg,
        86,
        &[on(20086, d(1, 6), 86, &[Col::Name("Variation")])],
    ),
    entry(Dmg, 87, &[on(20087, d(1, 100), 87, &[Col::Art("Value")])]),
    entry(Dmg, 88, &[on(20088, d(1, 100), 88, RESULT)]),
    entry(Dmg, 89, T89),
    entry(Dmg, 90, T90),
    entry(Dmg, 91, T91),
    entry(Dmg, 92, &[items(20092, 92)]),
    entry(Dmg, 93, &[items(20093, 93)]),
    entry(Dmg, 94, &[items(20094, 94)]),
    entry(Dmg, 95, &[items(20095, 95)]),
    entry(Dmg, 96, T96),
    entry(Dmg, 97, &[items(20097, 97)]),
    entry(Dmg, 98, &[items(20098, 98)]),
    entry(Dmg, 99, &[items(20099, 99)]),
    entry(Dmg, 100, &[items(20100, 100)]),
    entry(Dmg, 101, &[items(20101, 101)]),
    entry(Dmg, 102, &[items(20102, 102)]),
    entry(Dmg, 103, &[items(20103, 103)]),
    entry(Dmg, 104, T104),
    entry(Dmg, 105, T105),
    entry(Dmg, 106, T106),
    entry(Dmg, 107, T107),
    entry(Dmg, 108, T108),
    entry(Dmg, 109, T109_SWORD),
    entry(Dmg, 110, T110),
    entry(Dmg, 111, &[on(20111, d(1, 100), 111, RESULT)]),
    entry(Dmg, 112, CODE_ONLY),
    entry(
        Dmg,
        113,
        &[on(
            20113,
            d(1, 100),
            113,
            &[
                Col::V3("INT"),
                Col::Name("Communication"),
                Col::Item1("Capabilities"),
            ],
        )],
    ),
    entry(
        Dmg,
        114,
        &[on(
            20114,
            d(1, 100),
            114,
            &[Col::Name("Alignment of Weapon")],
        )],
    ),
    entry(Dmg, 115, T115),
    entry(Dmg, 116, T116),
    entry(Dmg, 117, T117),
    entry(Dmg, 118, CODE_ONLY),
    // Player's Handbook.
    with_params(
        Phb,
        Some(10),
        None,
        &[
            RACE,
            ParamSpec {
                label: "Sex",
                options: Options::Fixed(&["Female", "Male"]),
            },
        ],
        &[
            Step::Lookup {
                ids: &[12010, 11010],
                id_param: 1,
                row_param: 0,
                dice: HEIGHT_DICE,
                title: Title::Table(10),
                label: "Height of",
                unit: "inches",
            },
            Step::Lookup {
                ids: &[14010, 13010],
                id_param: 1,
                row_param: 0,
                dice: WEIGHT_DICE,
                title: Title::None,
                label: "Weight of",
                unit: "pounds",
            },
        ],
    ),
    with_params(
        Phb,
        Some(11),
        None,
        &[ParamSpec {
            label: "Race",
            options: Options::Rows(10011),
        }],
        &[
            Step::Lookup {
                ids: &[10011],
                id_param: usize::MAX,
                row_param: 0,
                dice: START_AGE,
                title: Title::Table(11),
                label: "Starting Age of",
                unit: "years",
            },
            Step::Formula {
                param: 0,
                dice: MAX_AGE,
                label: "Maximum Age",
                unit: "years",
            },
        ],
    ),
    entry(
        Phb,
        16,
        &[
            on(11016, d(1, 100), 16, &[Col::Name("Leader")]),
            then(12016, d(1, 100), &[Col::Name("Troops")]),
            then(13016, d(1, 100), &[Col::Name("Elite Unit")]),
        ],
    ),
    entry(
        Phb,
        19,
        &[Step::Roll(
            RollSpec::new(
                10019,
                d(1, 100),
                Title::Table(19),
                &[Col::Name("Followers")],
            )
            .rolls(d(2, 6)),
        )],
    ),
    entry(
        Phb,
        31,
        &[Step::Roll(
            RollSpec::new(
                10031,
                d(1, 100),
                Title::Table(31),
                &[Col::Name("Follower"), Col::Item1("Level Range")],
            )
            .rolls(d(4, 6)),
        )],
    ),
    entry(
        Phb,
        36,
        &[on(10036, d(1, 100), 36, &[Col::Name("Secondary Skills")])],
    ),
    entry(Phb, 43, CODE_ONLY),
    entry(Phb, 58, PUNCHING),
    entry(Phb, 61, CODE_ONLY),
    // Player's Option: Combat & Tactics.
    with_params(
        CombatTactics,
        None,
        Some("Critical Hits"),
        &[
            ParamSpec {
                label: "Target Type",
                options: Options::Fixed(&[
                    "Humanoid",
                    "Animal",
                    "Monster",
                    "Fish or snake-like monster",
                    "Fish or snake",
                ]),
            },
            ParamSpec {
                label: "Weapon Type",
                options: Options::Fixed(&["Bludgeoning", "Slashing", "Piercing"]),
            },
            SIZE_ADVANTAGE,
            ParamSpec {
                label: "Weapon Size",
                options: Options::Fixed(&[
                    "Smaller than the target",
                    "Same size as the target",
                    "Larger than the target",
                    "Two sizes larger than the target",
                ]),
            },
            CALLED_SHOT,
        ],
        &[Step::Critical(&WEAPON_CRITICAL)],
    ),
    // Player's Option: Skills & Powers.
    entry(
        SkillsPowers,
        1,
        &[on(30001, d(1, 20), 1, &[Col::Name("Event")])],
    ),
    entry(SkillsPowers, 25, CODE_ONLY),
    entry(SkillsPowers, 31, CODE_ONLY),
    entry(SkillsPowers, 36, CODE_ONLY),
    entry(
        SkillsPowers,
        40,
        &[on(30040, d(2, 6), 40, &[Col::Name("Social Rank")])],
    ),
    entry(
        SkillsPowers,
        41,
        &[on(30041, d(1, 100), 41, &[Col::Name("Kit")])],
    ),
    entry(
        SkillsPowers,
        42,
        &[on(30042, d(1, 20), 42, &[Col::Name("Companion")])],
    ),
    entry(
        SkillsPowers,
        83,
        &[on(31083, d(1, 100), 83, &[Col::Name("Wild Devotion")])],
    ),
    // Player's Option: Spells & Magic.
    entry(SpellsMagic, 14, CODE_ONLY),
    entry(SpellsMagic, 15, CODE_ONLY),
    with_params(
        SpellsMagic,
        Some(23),
        None,
        &[SPELL_LEVEL],
        &[Step::Insanity { check: true }],
    ),
    with_params(
        SpellsMagic,
        Some(24),
        None,
        &[SPELL_LEVEL],
        &[Step::Insanity { check: false }],
    ),
    entry(
        SpellsMagic,
        25,
        &[on(40025, d(1, 10), 25, &[Col::Name("Phobia")])],
    ),
    with_params(
        SpellsMagic,
        Some(46),
        None,
        &[ParamSpec {
            label: "Target Type",
            options: Options::Fixed(&["Humanoid", "Monster or animal", "Snake or fish"]),
        }],
        &[Step::ByParam {
            param: 0,
            options: &[&[location(41046)], &[location(42046)], &[location(43046)]],
        }],
    ),
    entry(SpellsMagic, 47, CODE_ONLY),
    entry(SpellsMagic, 48, CODE_ONLY),
    with_params(
        SpellsMagic,
        None,
        Some("Spell Hit Location and Effect"),
        &[
            ParamSpec {
                label: "Target Type",
                options: Options::Fixed(&["Humanoid", "Monster or animal", "Snake or fish"]),
            },
            ParamSpec {
                label: "Spell Type",
                options: Options::Headers(
                    &[
                        41100, 41101, 41102, 41103, 41104, 41105, 41106, 41107, 41108, 41109,
                    ],
                    " Spell",
                ),
            },
            SIZE_ADVANTAGE,
            ParamSpec {
                label: "Maximum Damage",
                options: Options::Fixed(&[
                    "Less than half the target's hit points",
                    "Less than the target's hit points",
                    "Less than twice the target's hit points",
                    "Twice the target's hit points or more",
                ]),
            },
            CALLED_SHOT,
        ],
        &[Step::Critical(&SPELL_CRITICAL)],
    ),
];

/// A roll on a location table of S&M Table 46 (its v3 location codes are
/// not tables).
const fn location(id: i32) -> Step {
    let mut spec = RollSpec::new(id, d(1, 10), Title::Table(46), &[Col::Name("Location")]);
    spec.follow = false;
    Step::Roll(spec)
}
