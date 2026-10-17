//! Intelligent weapons (Expansion 0x546b4b, 2.0 0x5ce120): the lines the
//! roller adds to a weapon's contents. Every roll is `rand() % 100 + 1`
//! compared with `<` against a chain of thresholds. The ability lines are
//! the program's own strings (0x831128..0x831c48), read from it at run
//! time; only their addresses are kept here.

use super::{Bundle, Flavor, Item, Rand};
use crate::Installation;
use crate::exe::Image;

/// A string's address in Tsrp3.exe and in Evermore.exe.
type Addr = [u32; 2];

const DETECT: [(i32, Addr); 10] = [
    (12, [0x831320, 0x6e1854]),
    (23, [0x831364, 0x6e181c]),
    (34, [0x83139c, 0x6e17e4]),
    (45, [0x8313d4, 0x6e17b4]),
    (56, [0x831404, 0x6e176c]),
    (67, [0x83144c, 0x6e1730]),
    (78, [0x831488, 0x6e1704]),
    (83, [0x8314b4, 0x6e16d4]),
    (88, [0x8314e4, 0x6e169c]),
    (93, [0x83151c, 0x6e1670]),
];

const EXTRAORDINARY: [(i32, Addr); 14] = [
    (8, [0x831548, 0x6e1640]),
    (16, [0x831578, 0x6e15f8]),
    (23, [0x8315c0, 0x6e15b0]),
    (29, [0x831608, 0x6e1578]),
    (35, [0x831640, 0x6e153c]),
    (42, [0x83167c, 0x6e1510]),
    (48, [0x8316a8, 0x6e14f4]),
    (55, [0x8316c4, 0x6e14b4]),
    (62, [0x831704, 0x6e145c]),
    (68, [0x83175c, 0x6e1428]),
    (76, [0x831790, 0x6e13dc]),
    (82, [0x8317dc, 0x6e13ac]),
    (89, [0x83180c, 0x6e1368]),
    (95, [0x831850, 0x6e1324]),
];

const CHOICE: Addr = [0x831894, 0x6e12f0];

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

const PURPOSE: [(i32, Addr); 10] = [
    (11, [0x8318fc, 0x6e12b0]),
    (21, [0x83193c, 0x6e1278]),
    (31, [0x831974, 0x6e1254]),
    (41, [0x831998, 0x6e1234]),
    (51, [0x8319b8, 0x6e1214]),
    (56, [0x8319d8, 0x6e11f4]),
    (66, [0x8319f8, 0x6e11c8]),
    (76, [0x831a24, 0x6e119c]),
    (96, [0x831a50, 0x6e1170]),
    (0, [0x831a7c, 0x6e1158]),
];

/// The fourth line has the program's typo "feat" for fear.
const POWER: [(i32, Addr); 7] = [
    (11, [0x831a94, 0x6e1128]),
    (21, [0x831ac4, 0x6e10f8]),
    (26, [0x831af4, 0x6e10d4]),
    (56, [0x831b18, 0x6e10a8]),
    (66, [0x831b44, 0x6e1078]),
    (81, [0x831b74, 0x6e1048]),
    (0, [0x831ba4, 0x6e0ff4]),
];

const LANGUAGES: [(i32, i32); 6] = [(41, 1), (71, 2), (86, 3), (96, 4), (100, 5), (0, 6)];

/// The ability lines, read from the installation's program; without the
/// program each line says where its text is.
#[derive(Debug, Clone)]
pub(super) struct Texts {
    detect: Vec<String>,
    extraordinary: Vec<String>,
    choice: String,
    purpose: Vec<String>,
    power: Vec<String>,
}

impl Texts {
    pub(super) fn read(install: &Installation, flavor: Flavor) -> Texts {
        let image = Image::open(install, flavor).ok();
        let column = usize::from(flavor == Flavor::Core20);
        let text = |a: Addr| {
            image
                .as_ref()
                .and_then(|i| i.text(a[column]))
                .unwrap_or_else(|| {
                    let program = match flavor {
                        Flavor::Expansion => "Tsrp3.exe",
                        Flavor::Core20 => "Evermore.exe",
                    };
                    format!("(an ability line of Program/{program}, which is missing)")
                })
        };
        let all = |chain: &[(i32, Addr)]| chain.iter().map(|&(_, a)| text(a)).collect();
        Texts {
            detect: all(&DETECT),
            extraordinary: all(&EXTRAORDINARY),
            choice: text(CHOICE),
            purpose: all(&PURPOSE),
            power: all(&POWER),
        }
    }
}

/// The index of the first threshold above the roll, else the last.
fn pick_index(rand: &mut Rand, chain: &[(i32, Addr)]) -> usize {
    let v = percent(rand);
    chain[..chain.len() - 1]
        .iter()
        .position(|(limit, _)| v < *limit)
        .unwrap_or(chain.len() - 1)
}

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
pub(super) fn make_intelligent(texts: &Texts, rand: &mut Rand, weapon: &mut Item) {
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
        if let Some(k) = DETECT.iter().position(|(limit, _)| v < *limit) {
            add(texts.detect[k].clone());
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
        if let Some(k) = EXTRAORDINARY.iter().position(|(limit, _)| v < *limit) {
            add(texts.extraordinary[k].clone());
        } else if v < 98 && twice == 0 {
            extraordinary += 2;
            twice = 2;
        } else {
            // A second choice line merges with the first.
            add(texts.choice.clone());
            special |= v >= 100;
        }
        if twice != 0 {
            twice -= 1;
        }
        i += 1;
    }

    if special {
        add(texts.purpose[pick_index(rand, &PURPOSE)].clone());
        add(texts.power[pick_index(rand, &POWER)].clone());
    }
    if languages {
        add(format!("Languages: {}", pick(rand, &LANGUAGES)));
    }
}
