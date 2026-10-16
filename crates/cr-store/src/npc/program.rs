//! What the NPC generator takes from the program (`Program/Tsrp3.exe` or
//! `Program/Evermore.exe`) at run time: the name syllable tables, the name
//! generators' race keywords, the backgrounds, the weapon-name tests, the
//! club kit's lists and the texts it compares with or copies
//! ([`super::texts`]). Only addresses and the shapes of the tests are kept
//! here; the texts are read from the installation's program.

use std::collections::HashMap;

use super::exe::Image;
use super::texts;
use crate::treasure::Flavor;
use crate::{Error, Result};

/// A program text: its address in Tsrp3.exe and in Evermore.exe (0 where
/// Evermore has none).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tx(pub u32, pub u32);

/// 0x81f280 (Gnome, male).
pub const NORM: Tx = Tx(0x81f280, 0x6d62ec);
/// 0x81f67c (Bullywug suffix).
pub const BUP: Tx = Tx(0x81f67c, 0x6d6590);

/// What a text the flavor's program lacks reads as: it equals and is
/// contained in no name.
const MISSING: &str = "\u{1}";

/// A level test inside a weapon-name rule list: `cmp +0x738, K`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// `jg`: the level is above K.
    Above(i32),
    /// `jl`: the level is below K.
    Below(i32),
}

/// One test of 0x4ce3d6 / 0x4d6dc6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameRule {
    /// The name equals the string: the test hits.
    Eq(String),
    /// The name contains the string.
    Find(String),
    /// A race test (the race record's parent name, or the character's
    /// race), then the name contains `find`.
    Race {
        parent: bool,
        equal: bool,
        race: String,
        find: String,
    },
    /// When the level test holds, go on at rule `to` (`None`: leave the
    /// list with the "no hit" result).
    Gate { when: Level, to: Option<usize> },
    /// 0x4d6dc6 goes on with the kit tests.
    Stop,
}

/// The shape of a rule; its texts are the function's pushes, in order (a
/// race test pushes the race, then the text it looks for).
#[derive(Debug, Clone, Copy)]
enum Shape {
    Eq,
    Find,
    Race { parent: bool, equal: bool },
    Gate { when: Level, to: Option<usize> },
    Stop,
}

/// 0x4ce3d6: the "common weapon" tests on the upper-cased name, as runs of
/// (count, shape).
const COMMON_SHAPES: &[(usize, Shape)] = &[
    (3, Shape::Eq),
    (8, Shape::Find),
    (
        1,
        Shape::Race {
            parent: true,
            equal: false,
        },
    ),
    (
        4,
        Shape::Race {
            parent: true,
            equal: true,
        },
    ),
    (
        1,
        Shape::Race {
            parent: false,
            equal: true,
        },
    ),
    (
        1,
        Shape::Gate {
            when: Level::Above(1),
            to: Some(19),
        },
    ),
    (1, Shape::Find),
    (
        1,
        Shape::Gate {
            when: Level::Below(4),
            to: None,
        },
    ),
    (102, Shape::Find),
];

/// 0x4d6dc6 up to its call of 0x43fac6: the excluded-weapon tests.
const EXCLUDED_SHAPES: &[(usize, Shape)] = &[
    (
        1,
        Shape::Race {
            parent: false,
            equal: true,
        },
    ),
    (
        4,
        Shape::Race {
            parent: true,
            equal: true,
        },
    ),
    (3, Shape::Find),
    (1, Shape::Stop),
];

/// The name generators of Tsrp3.exe (each builds its tables on the stack
/// and picks with 0x4b8eda).
const NAME_FUNCTIONS: [u32; 15] = [
    0x4b8b0b, 0x4b8f34, 0x4b928b, 0x4b97dd, 0x4ba82f, 0x4ba740, 0x4bc074, 0x4bbd92, 0x4bbbda,
    0x4bba12, 0x4bb80a, 0x4bae7e, 0x4bb2d7, 0x4bb4b5, 0x4babc0,
];

/// Evermore.exe's name generators load their tables from registers (an
/// optimized build), so their entries are listed by address, keyed by the
/// Tsrp3.exe (function, local) of the same table.
#[rustfmt::skip]
const EV_NAME_TABLES: &[((u32, u32), &[u32])] = &[
    ((0x4b8b0b, 0x194), &[0x6d5d64, 0x6d5d5c, 0x6d5d54, 0x6d5d48, 0x6d5d40, 0x6d5d38, 0x6d5d30, 0x6d5d28, 0x6d5d20, 0x6d5d18, 0x6d5d10, 0x6d5d08, 0x6d5d00, 0x6d5cf8, 0x6d5cec, 0x6d5ce4, 0x6d5cdc, 0x6d5cd4, 0x6d5cd0, 0x6d5cc8, 0x6d5cc0, 0x6d5cbc, 0x6d5cb4, 0x6d5cac, 0x6d5ca4, 0x6d5c98, 0x6d5c90, 0x6d5c88, 0x6d5c80, 0x6d5c78, 0x6d5c70, 0x6d5c6c, 0x6d5c64, 0x6d5c5c, 0x6d5c54, 0x6d5c48, 0x6d5c40, 0x6d5c38, 0x6d5c30, 0x6d5c28, 0x6d5c20, 0x6d5c18, 0x6d5c10, 0x6d5c08, 0x6d5c00, 0x6d5bf8, 0x6d5bf4, 0x6d5bec, 0x6d5be4, 0x6d5bdc, 0x6d5bd4, 0x6d5bcc, 0x6d5bc4, 0x6d5bc0, 0x6d5bb8, 0x6d5bb0, 0x6d5bac, 0x6d5ba4, 0x6d5b9c, 0x6d5b94, 0x6d5b90, 0x6d5b88, 0x6d5b80, 0x6d5b78, 0x6d5b70, 0x6d5b68, 0x6d5b60, 0x6d5b58, 0x6d5b50, 0x6d5b44, 0x6d5b3c, 0x6d5b34, 0x6d5b2c, 0x6d5b24, 0x6d5b1c, 0x6d5b14, 0x6d5b0c, 0x6d5b08, 0x6d5b04, 0x6d5afc, 0x6d5af4, 0x6d5aec, 0x6d5ae4, 0x6d5adc, 0x6d5ad4, 0x6d5acc, 0x6d5ac4, 0x6d5abc, 0x6d5ab4, 0x6d5aac, 0x6d5aa4, 0x6d5a9c, 0x6d5a94, 0x6d5a8c, 0x6d5a84, 0x6d5a7c, 0x6d5a74, 0x6d5a6c]),
    ((0x4b8f34, 0x164), &[0x6d6030, 0x6d6028, 0x6d6020, 0x6d6018, 0x6d6010, 0x6d6008, 0x6d6000, 0x6d5ff8, 0x6d5ff0, 0x6d5fe4, 0x6d5fdc, 0x6d5fd4, 0x6d5fcc, 0x6d5fc4, 0x6d5fbc, 0x6d5fb4, 0x6d5fac, 0x6d5fa4, 0x6d5f9c, 0x6d5f94, 0x6d5f8c, 0x6d5f88, 0x6d5f7c, 0x6d5f74, 0x6d5f6c, 0x6d5f64, 0x6d5f58, 0x6d5f50, 0x6d5f48, 0x6d5f40, 0x6d5f38, 0x6d5f30, 0x6d5f24, 0x6d5f1c, 0x6d5f18, 0x6d5f10, 0x6d5f0c, 0x6d5f04, 0x6d5efc, 0x6d5ef4, 0x6d5eec, 0x6d5ee8, 0x6d5edc, 0x6d5ed8, 0x6d5ed0, 0x6d5ec8, 0x6d5ec0, 0x6d5eb8, 0x6d5eb0, 0x6d5ea8, 0x6d5ea0, 0x6d5e98, 0x6d5e90, 0x6d5e88, 0x6d5e7c, 0x6d5e74, 0x6d5e6c, 0x6d5e60, 0x6d5e58, 0x6d5e50, 0x6d5e44, 0x6d5e3c, 0x6d5e34, 0x6d5e2c, 0x6d5e24, 0x6d5e1c, 0x6d5e14, 0x6d5e0c, 0x6d5e04, 0x6d5e00, 0x6d5df8, 0x6d5df0, 0x6d5de8, 0x6d5de0, 0x6d5dd8, 0x6d5dcc, 0x6d5dc4, 0x6d5dbc, 0x6d5db4, 0x6d5dac, 0x6d5da0, 0x6d5d98, 0x6d5d8c, 0x6d5d84, 0x6d5d7c, 0x6d5d74, 0x6d5d6c]),
    ((0x4b928b, 0x94), &[0x6d607c, 0x6c46c4, 0x6d6078, 0x6d6074, 0x6c46c0, 0x6d6070, 0x6cf9cc, 0x6cea90]),
    ((0x4b928b, 0xc4), &[0x6d606c, 0x6d6068, 0x6d6064, 0x6d6060]),
    ((0x4b928b, 0x70), &[0x6d605c, 0x6d6058, 0x6d6054, 0x6d6050, 0x6d604c]),
    ((0x4b928b, 0x58), &[0x6d6048, 0x6d6044, 0x6d6040, 0x6d6088, 0x6d6084, 0x6d608c, 0x6d6080]),
    ((0x4b928b, 0xa8), &[0x6d606c, 0x6d6068, 0x6d6064, 0x6d6060]),
    ((0x4b928b, 0x38), &[0x6d6080, 0x6d603c, 0x6d6038, 0x6d608c, 0x6d6088, 0x6d6084, 0x6d6048, 0x6d6044]),
    ((0x4b97dd, 0x74), &[0x6c2174, 0x6d6074, 0x6cea90, 0x6c2178, 0x6c46bc]),
    ((0x4b97dd, 0x5c), &[0x6d61b8, 0x6d6060, 0x6d606c, 0x6d6064, 0x6d6068]),
    ((0x4b97dd, 0x44), &[0x6d61b4, 0x6d61b0, 0x6d6088, 0x6d61ac, 0x6d61a8, 0x6d61a4, 0x6c5130, 0x6d61a0, 0x6d619c, 0x6d6198, 0x6d6048]),
    ((0x4b97dd, 0xc8), &[0x6d6050, 0x6d6068, 0x6d6194]),
    ((0x4b97dd, 0x130), &[0x6d6178, 0x6d6174, 0x6d6170, 0x6d6168, 0x6cbb04, 0x6d6160, 0x6d6158, 0x6d6150, 0x6d6148, 0x6d6140, 0x6d6138, 0x6d6130, 0x6d6128]),
    ((0x4b97dd, 0xb8), &[0x6d6188, 0x6d6110, 0x6d6108, 0x6d6180, 0x6d6100, 0x6d60f8, 0x6d60f0, 0x6d60e8, 0x6d60e0, 0x6d60d8]),
    ((0x4b97dd, 0x8c), &[0x6ccfe0, 0x6cd2e0, 0x6cd2c8, 0x6d6120, 0x6d6118]),
    ((0x4b97dd, 0xf8), &[0x6d6180, 0x6d60d0, 0x6d6188, 0x6d60c8, 0x6d60bc, 0x6d60b4, 0x6d60ac, 0x6d60a8, 0x6d60a0, 0x6d6098, 0x6d6090]),
    ((0x4ba82f, 0x8c), &[0x6d648c, 0x6d5b08, 0x6d6488, 0x6d6480, 0x6d6478, 0x6d6470, 0x6d6468]),
    ((0x4ba82f, 0xa8), &[0x6d5e58, 0x6d6460, 0x6d6454, 0x6d644c, 0x6d6444, 0x6d643c]),
    ((0x4ba82f, 0x68), &[0x6d6434, 0x6d642c, 0x6d6424, 0x6d641c, 0x6d6414, 0x6d640c]),
    ((0x4ba82f, 0x4c), &[0x6d63e4, 0x6d63dc, 0x6d63d4, 0x6d63cc, 0x6d63c4, 0x6d63bc]),
    ((0x4ba82f, 0x30), &[0x6d6404, 0x6d63fc, 0x6d63f4, 0x6d63ec]),
    ((0x4ba82f, 0x1c), &[0x6d63b4, 0x6d63ac]),
    ((0x4ba740, 0x70), &[0x6d63a0, 0x6d639c, 0x6d6394, 0x6d638c, 0x6d6384, 0x6d6380, 0x6d6378, 0x6d6370, 0x6d6368, 0x6d6364, 0x6d635c, 0x6d6358, 0x6d6354, 0x6d6350, 0x6d634c, 0x6d6348, 0x6d6340, 0x6d6338, 0x6d6330, 0x6d6328, 0x6d6320, 0x6d3d38, 0x6d6318, 0x6d6310, 0x6d6308, 0x6d6304]),
    ((0x4bc074, 0x98), &[0x6d6728, 0x6d6720, 0x6d671c, 0x6d6718, 0x6d6714, 0x6d6710, 0x6d670c, 0x6d6330]),
    ((0x4bc074, 0x114), &[0x6d66e4, 0x6d66dc, 0x6d66d4, 0x6d66d0, 0x6d66cc, 0x6d66c8, 0x6d66c0, 0x6d66bc, 0x6d66b8, 0x6d66b0]),
    ((0x4bc074, 0x44), &[0x6d6704, 0x6d6700, 0x6d66fc, 0x6d66f4, 0x6d66f0, 0x6d66e8]),
    ((0x4bc074, 0xc0), &[0x6d6068, 0x6d64b0]),
    ((0x4bc074, 0xb4), &[0x6ccfe0, 0x6c2350, 0x6c215c, 0x6ccfe4, 0x6ccff0, 0x6d66a0]),
    ((0x4bc074, 0x74), &[0x6d6180, 0x6d6698, 0x6d60d0, 0x6d6188, 0x6d60c8, 0x6d60bc, 0x6d60b4, 0x6d60ac, 0x6d60a8, 0x6d60a0, 0x6d6090]),
    ((0x4bbd92, 0x34), &[0x6d6690, 0x6d668c, 0x6d6688, 0x6d6684, 0x6d6680, 0x6d667c, 0x6d6674]),
    ((0x4bbd92, 0x44), &[0x6d6644, 0x6d663c, 0x6d6634]),
    ((0x4bbd92, 0x6c), &[0x6d666c, 0x6d6664, 0x6d65cc, 0x6d6660, 0x6d6658, 0x6d6650, 0x6d6648]),
    ((0x4bbbda, 0x34), &[0x6d6630, 0x6d662c, 0x6d6628, 0x6d6624, 0x6d65f0, 0x6d6620, 0x6d6618]),
    ((0x4bbbda, 0x50), &[0x6d6614, 0x6d6610, 0x6d660c, 0x6d6608, 0x6d6604, 0x6d64b4]),
    ((0x4bba12, 0x40), &[0x6d6600, 0x6d65fc, 0x6d65f8, 0x6d65f4, 0x6d65f0, 0x6d65e8, 0x6d65e4]),
    ((0x4bba12, 0x5c), &[0x6d65dc, 0x6d65d4, 0x6d65cc, 0x6d65c8, 0x6d65c4, 0x6d65c0]),
    ((0x4bba12, 0x20), &[0x6d6060, 0x6d6068]),
    ((0x4bb80a, 0x44), &[0x6d65bc, 0x6d65b8, 0x6d65b4, 0x6d64f0, 0x6d5b04, 0x6d65ac, 0x6d65a8]),
    ((0x4bb80a, 0x64), &[0x6d65a4, 0x6d64e4, 0x6d65a0, 0x6d64dc, 0x6d659c, 0x6d6598, 0x6d6594]),
    ((0x4bb80a, 0x24), &[0x6d6060, 0x6d6068, 0x6d64b0]),
    ((0x4bae7e, 0x2c), &[0x6d64a8, 0x6d6070, 0x6d64d0, 0x6d64a8, 0x6d6074, 0x6d64d0]),
    ((0x4bae7e, 0x90), &[0x6d6060, 0x6d6068, 0x6d6064, 0x6d6060, 0x6d6068]),
    ((0x4bae7e, 0x78), &[0x6d61ac, 0x6d61a0, 0x6d6068, 0x6d649c, 0x6d6048, 0x6d6048, 0x6d6198, 0x6d61ac]),
    ((0x4bae7e, 0x54), &[0x6d64cc, 0x6d64c8, 0x6d64c4, 0x6d64c0, 0x6d64bc, 0x6d64b8, 0x6d61ac, 0x6d64b4]),
    ((0x4bae7e, 0xb0), &[0x6d6060, 0x6d6068, 0x6d606c, 0x6d6060, 0x6d6068, 0x6d64b0, 0x6d64ac]),
    ((0x4bb2d7, 0x44), &[0x6d64fc, 0x6d64f8, 0x6d64f4, 0x6d64f0, 0x6d5b04, 0x6d64ec]),
    ((0x4bb2d7, 0x60), &[0x6d64e8, 0x6d64e4, 0x6d64e0, 0x6d64dc, 0x6d64d8, 0x6d64d4]),
    ((0x4bb2d7, 0x28), &[0x6d6060, 0x6d6060, 0x6d61b8, 0x6d6068]),
    ((0x4bb4b5, 0x9c), &[0x6d658c, 0x6d6588, 0x6d6580, 0x6d657c, 0x6d6578, 0x6d6574, 0x6d6570, 0x6d656c, 0x6d6568, 0x6d6564, 0x6d655c, 0x6d6554, 0x6d6550, 0x6d6548, 0x6d6544]),
    ((0x4bb4b5, 0x4c), &[0x6d6540, 0x6d6538, 0x6d6534, 0x6d652c, 0x6d6524, 0x6d6520, 0x6d651c, 0x6d6514, 0x6d650c, 0x6d6508, 0x6d12b0, 0x6d6504, 0x6d12ac]),
    ((0x4bb4b5, 0xb0), &[0x6d6584, 0x6d6584, 0x6d6500, 0x6d6584]),
    ((0x4bb4b5, 0x5c), &[0x6d6050, 0x6d6050, 0x6d6058]),
    ((0x4babc0, 0x7c), &[0x6d64a8, 0x6c46c0, 0x6c2178, 0x6cf9cc, 0x6cea94, 0x6c46bc, 0x6d64a4, 0x6d6074, 0x6d64a0]),
    ((0x4babc0, 0x54), &[0x6d606c, 0x6d6068, 0x6d6064, 0x6d6060, 0x6d61b8]),
    ((0x4babc0, 0x3c), &[0x6d61ac, 0x6d608c, 0x6d61a0, 0x6d6088, 0x6d649c, 0x6d61b0, 0x6d605c, 0x6d6048, 0x6d6498]),
];

/// Everything read from the program.
#[derive(Debug, Clone)]
pub struct Program {
    flavor: Flavor,
    /// Texts by the flavor's address.
    texts: HashMap<u32, String>,
    /// Name tables by Tsrp3.exe (function, local).
    tables: HashMap<(u32, u32), Vec<String>>,
    /// 0x4b9da3 / Evermore 0x536600: the race keywords, in test order.
    pub name_keys: Vec<String>,
    /// 0x54f660 (app +0x1f84) / Evermore 0x5d4dc2: the backgrounds, "NONE"
    /// first.
    pub backgrounds: Vec<String>,
    pub common_rules: Vec<NameRule>,
    pub excluded_rules: Vec<NameRule>,
    /// 0x4d8040: the upper-cased words of a club-like weapon.
    pub club_words: Vec<String>,
    /// 0x4d81ad: the club kit's replacements.
    pub club_choices: Vec<String>,
    /// 0x4d7d99: the upper-cased words of a missile weapon a kit's
    /// +0x3fc list does not let in.
    pub missile_words: Vec<String>,
}

fn rules(shapes: &[(usize, Shape)], texts: &[String]) -> Option<Vec<NameRule>> {
    let mut t = texts.iter().cloned();
    let mut out = Vec::new();
    for &(n, shape) in shapes {
        for _ in 0..n {
            out.push(match shape {
                Shape::Eq => NameRule::Eq(t.next()?),
                Shape::Find => NameRule::Find(t.next()?),
                Shape::Race { parent, equal } => NameRule::Race {
                    parent,
                    equal,
                    race: t.next()?,
                    find: t.next()?,
                },
                Shape::Gate { when, to } => NameRule::Gate { when, to },
                Shape::Stop => NameRule::Stop,
            });
        }
    }
    Some(out)
}

fn take(mut list: Vec<String>, n: usize, what: &str) -> Result<Vec<String>> {
    if list.len() < n {
        return Err(Error::Invalid(format!(
            "the program's {what} were not found where the NPC generator reads them"
        )));
    }
    list.truncate(n);
    Ok(list)
}

impl Default for Program {
    fn default() -> Self {
        Self::empty(Flavor::Expansion)
    }
}

impl Program {
    /// No program: every text reads as missing, every table is empty.
    pub fn empty(flavor: Flavor) -> Self {
        Program {
            flavor,
            texts: HashMap::new(),
            tables: HashMap::new(),
            name_keys: Vec::new(),
            backgrounds: Vec::new(),
            common_rules: Vec::new(),
            excluded_rules: Vec::new(),
            club_words: Vec::new(),
            club_choices: Vec::new(),
            missile_words: Vec::new(),
        }
    }

    /// Reads the generator's texts from the flavor's program.
    pub fn read(image: &Image, flavor: Flavor) -> Result<Self> {
        let mut p = Self::empty(flavor);
        for &tx in texts::ALL.iter().chain(&[NORM, BUP]) {
            let va = p.va(tx);
            if va == 0 {
                continue;
            }
            let text = image
                .text(va)
                .ok_or_else(|| Error::Invalid(format!("the program has no text at {va:#x}")))?;
            p.texts.insert(va, text);
        }
        match flavor {
            Flavor::Expansion => {
                for f in NAME_FUNCTIONS {
                    for (disp, list) in image.stack_tables(f, 0x3000, 0x4b8eda) {
                        p.tables.insert((f, (4 - disp) as u32), list);
                    }
                }
                p.name_keys = take(image.pushed_texts(0x4b9da3, 0x1000), 31, "race keywords")?;
                p.backgrounds = take(image.pushed_texts(0x54f66a, 0x800), 21, "backgrounds")?;
                let common = take(image.pushed_texts(0x4ce3d6, 0x4000), 126, "weapon tests")?;
                let excluded = take(image.pushed_texts(0x4d6dc6, 0x800), 13, "weapon tests")?;
                p.common_rules = rules(COMMON_SHAPES, &common).unwrap_or_default();
                p.excluded_rules = rules(EXCLUDED_SHAPES, &excluded).unwrap_or_default();
                p.club_words = take(image.pushed_texts(0x4d8040, 0x400), 6, "club words")?;
                p.club_choices = image
                    .stack_tables(0x4d81ad, 0x400, 0x4c2042)
                    .remove(&-36)
                    .unwrap_or_default();
                p.missile_words = take(image.pushed_texts(0x4d7d99, 0x2a0), 11, "missile words")?;
            }
            Flavor::Core20 => {
                for (key, vas) in EV_NAME_TABLES {
                    let list = vas.iter().map(|&va| image.text(va).unwrap_or_default());
                    p.tables.insert(*key, list.collect());
                }
                p.name_keys = take(image.pushed_texts(0x536600, 0x1000), 31, "race keywords")?;
                p.backgrounds = take(image.pushed_texts(0x5d4dc2, 0x800), 21, "backgrounds")?;
            }
        }
        Ok(p)
    }

    fn va(&self, tx: Tx) -> u32 {
        match self.flavor {
            Flavor::Expansion => tx.0,
            Flavor::Core20 => tx.1,
        }
    }

    /// The text at `tx` in the flavor's program.
    pub fn text(&self, tx: Tx) -> &str {
        self.texts.get(&self.va(tx)).map_or(MISSING, String::as_str)
    }

    /// The name table `local` of the Tsrp3.exe name generator `f`.
    pub fn table(&self, f: u32, local: u32) -> &[String] {
        self.tables.get(&(f, local)).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
impl Program {
    /// For tests: sets the text at `tx`.
    pub fn set_text(&mut self, tx: Tx, text: &str) {
        let va = self.va(tx);
        self.texts.insert(va, text.into());
    }

    /// For tests: sets a name table.
    pub fn set_names(&mut self, key: (u32, u32), list: &[&str]) {
        self.tables
            .insert(key, list.iter().map(|s| s.to_string()).collect());
    }

    /// For tests: the race keyword `index` of 0x4b9da3 (the others match
    /// nothing).
    pub fn set_name_key(&mut self, index: usize, key: &str) {
        self.name_keys.resize(31, MISSING.into());
        self.name_keys[index] = key.into();
    }
}

/// 0x4290f5, column 2: weight allowance by Strength (19..23 are 18/01..18/00).
pub const CAPACITY: [i32; 31] = [
    0, 1, 4, 8, 17, 17, 39, 39, 66, 66, 77, 77, 94, 94, 116, 116, 131, 158, 189, 214, 239, 264,
    314, 414, 564, 614, 714, 864, 1014, 1314, 1614,
];
