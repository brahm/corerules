//! Intelligent weapons (Expansion 0x546b4b, 2.0 0x5ce120): the lines the
//! roller adds to a weapon's contents. Every roll is `rand() % 100 + 1`
//! compared with `<` against a chain of thresholds; the strings are the
//! program's own (0x831128..0x831c48).

use super::{Bundle, Item, Rand};

const DETECT: [(i32, &str); 10] = [
    (
        12,
        "Ability: Detect \"elevator\"/shifting rooms/walls in a 10-foot radius",
    ),
    (23, "Ability: Detect sloping passages in a 10-foot radius"),
    (
        34,
        "Ability: Detect traps of large size in a 10-foot radius",
    ),
    (45, "Ability: Detect evil/good in a 10-foot radius"),
    (
        56,
        "Ability: Detect precious metals, kind, and amount in a 20-foot radius",
    ),
    (
        67,
        "Ability: Detect gems, kind, and number in a 5-foot radius",
    ),
    (78, "Ability: Detect magic in a 10-foot radius"),
    (83, "Ability: Detect secret doors in a 5-foot radius"),
    (88, "Ability: Detect invisible objects in a 10-foot radius"),
    (93, "Ability: Locate object in a 120-foot radius"),
];

const EXTRAORDINARY: [(i32, &str); 14] = [
    (8, "Ability: Charm person on contact - 3 times/day"),
    (
        16,
        "Ability: Clairaudience, 30 yards range - 3 times/day, 1 round per use",
    ),
    (
        23,
        "Ability: Clairvoyance, 30 yards range - 3 times/day, 1 round per use",
    ),
    (29, "Ability: Determine direction and depth - 2 times/day"),
    (
        35,
        "Ability: ESP, 30 yards range - 3 times/day, 1 round per use",
    ),
    (42, "Ability: Fly, 120 feet/turn - 1 hour/day"),
    (48, "Ability: Heal - 1 time/day"),
    (
        55,
        "Ability: Illusion, 120 yard range - 2 times/day, as the wand",
    ),
    (
        62,
        "Ability: Levitation, 1 turn duration - 3 times/day, at 6th level of magic use ability",
    ),
    (68, "Ability: Strength - 1 time/day (upon wielder only)"),
    (
        76,
        "Ability: Telekinesis, 250 pounds maximum - 2 times/day, 1 round each use",
    ),
    (82, "Ability: Telepathy, 60 yard range - 2 times/day"),
    (
        89,
        "Ability: Teleport - 1 time/day, 600 pounds maximum, casting time 2",
    ),
    (
        95,
        "Ability: X-ray vision, 40 yards range - 2 times/day, 1 turn per use",
    ),
];

const CHOICE: &str = "Ability: Character's choice of extraordinary powers";

/// The last entry of each chain is the "else" case.
const INTELLIGENCE: [(i32, i32); 6] = [(35, 12), (60, 13), (80, 14), (92, 15), (97, 16), (0, 17)];

const ALIGNMENT: [(i32, &str); 9] = [
    (6, "Chaotic good"),
    (16, "Chaotic neutral"),
    (21, "Chaotic evil"),
    (26, "Neutral evil"),
    (31, "Lawful evil"),
    (56, "Lawful good"),
    (61, "Lawful neutral"),
    (91, "True neutral"),
    (0, "Neutral good"),
];

const PURPOSE: [(i32, &str); 10] = [
    (
        11,
        "Special Purpose: Defeat/slay diametrically opposed alignment",
    ),
    (21, "Special Purpose: Defeat priests of a particular type"),
    (31, "Special Purpose: Defeat fighters"),
    (41, "Special Purpose: Defeat wizards"),
    (51, "Special Purpose: Defeat thieves"),
    (56, "Special Purpose: Defeat bards"),
    (66, "Special Purpose: Overthrow law and/or chaos"),
    (76, "Special Purpose: Defeat good and/or evil"),
    (96, "Special Purpose: Defeat nonhuman monsters"),
    (0, "Special Purpose: Other"),
];

/// "feat" is the program's typo for fear.
const POWER: [(i32, &str); 7] = [
    (11, "Special Purpose Power: blindness for 2d6 rounds"),
    (21, "Special Purpose Power: confusion for 2d6 rounds"),
    (26, "Special Purpose Power: disintegrate"),
    (56, "Special Purpose Power: feat for 1d4 rounds"),
    (66, "Special Purpose Power: insanity for 1d4 rounds"),
    (81, "Special Purpose Power: paralysis for 1d4 rounds"),
    (
        0,
        "Special Purpose Power: +2 to all saving throws, -1 to each die of damage sustained",
    ),
];

const LANGUAGES: [(i32, i32); 6] = [(41, 1), (71, 2), (86, 3), (96, 4), (100, 5), (0, 6)];

fn percent(rand: &mut Rand) -> i32 {
    rand.rand() % 100 + 1
}

/// The first value whose threshold is above the roll, else the last.
fn pick<T: Copy>(rand: &mut Rand, chain: &[(i32, T)]) -> T {
    let v = percent(rand);
    let (last, rest) = chain.split_last().expect("non-empty chain");
    rest.iter()
        .find(|(limit, _)| v < *limit)
        .map_or(last.1, |(_, x)| *x)
}

/// Adds the abilities of an intelligent weapon to `weapon`'s contents,
/// each a flagged line inserted with merging.
pub(super) fn make_intelligent(rand: &mut Rand, weapon: &mut Item) {
    let contents = weapon.contents.get_or_insert_with(Bundle::default);
    let mut add = |line: String| contents.insert(Item::line(line), true);

    let intelligence = pick(rand, &INTELLIGENCE);
    let (communication, mut detect, mut extraordinary) = match intelligence {
        12 => ("Semi-empathy", 1, 0),
        13 => ("Empathy", 2, 0),
        14 => ("Speech", 2, 0),
        15 | 16 => ("Speech", 3, 0),
        _ => ("Speech and Telepathy", 3, 1),
    };
    // Only Intelligence 14 rolls languages; that is what the code does.
    let languages = intelligence == 14;
    add(format!("Intelligence: {intelligence}"));
    add(format!("Communication: {communication}"));
    add(format!("Alignment: {}", pick(rand, &ALIGNMENT)));

    let mut twice = 0;
    let mut i = 0;
    while i < detect {
        let v = percent(rand);
        if let Some((_, line)) = DETECT.iter().find(|(limit, _)| v < *limit) {
            add((*line).into());
        } else if v < 99 && twice == 0 {
            detect += 2;
            twice = 2;
        } else {
            extraordinary += 1;
        }
        if twice != 0 {
            twice -= 1;
        }
        i += 1;
    }

    let mut twice = 0;
    let mut special = false;
    let mut i = 0;
    while i < extraordinary {
        let v = percent(rand);
        if let Some((_, line)) = EXTRAORDINARY.iter().find(|(limit, _)| v < *limit) {
            add((*line).into());
        } else if v < 98 && twice == 0 {
            extraordinary += 2;
            twice = 2;
        } else {
            // A second choice line merges with the first.
            add(CHOICE.into());
            special |= v >= 100;
        }
        if twice != 0 {
            twice -= 1;
        }
        i += 1;
    }

    if special {
        add(pick(rand, &PURPOSE).into());
        add(pick(rand, &POWER).into());
    }
    if languages {
        add(format!("Languages: {}", pick(rand, &LANGUAGES)));
    }
}
