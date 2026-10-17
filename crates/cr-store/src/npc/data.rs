//! The databases as the NPC generator reads them: races, classes, parts
//! (with the kits), spells, psionic powers and the race/class combinations
//! of `UserDbas/AllowedU.dat`.
//!
//! Members are read by their offset in the original objects (`n(0x47c)` is
//! the part member at `this + 0x47c`), the way the specification names
//! them, so each stage reads like the code it ports.

use std::collections::HashMap;

use cr_formats::Object;
use cr_formats::classes::Class;
use cr_formats::parts::{Part, PartKitData, PartsFile};
use cr_formats::race::Race;
use cr_formats::spells::SpellsFile;

use super::program::{Program, Tx};
use crate::exe::Image;
use crate::treasure::Flavor;
use crate::{Error, Installation, Result};

/// Part types (`PART_TYPES`).
pub const WEAPONS: usize = 0;
pub const ARMOR: usize = 1;
pub const NONWEAPON: usize = 4;
pub const RACIAL_ABILITIES: usize = 5;
pub const LANGUAGES: usize = 6;
pub const EQUIPMENT: usize = 7;
pub const MAGIC_ITEMS: usize = 8;
pub const SCHOOLS: usize = 9;
pub const TRAITS: usize = 11;
pub const DISADVANTAGES: usize = 12;
pub const CLASS_ABILITIES: usize = 14;
pub const KITS: usize = 15;
pub const SPHERES: usize = 18;
const TYPES: usize = cr_formats::parts::PART_TYPE_COUNT;

/// The order 0x465f6c searches the part maps in when no type is given.
pub const ANY_TYPE_ORDER: [usize; 16] = [1, 15, 14, 12, 7, 13, 6, 8, 4, 5, 9, 11, 0, 17, 16, 18];

/// Index of a member inside an array that starts at `base`.
fn at<T: Copy + Default>(values: &[T], base: u32, off: u32) -> T {
    values
        .get(((off - base) / 4) as usize)
        .copied()
        .unwrap_or_default()
}

/// A part's members by offset.
pub trait PartExt {
    fn name(&self) -> &str;
    /// An integer or flag member; 0 when the part has none there.
    fn n(&self, off: u32) -> i32;
    fn s(&self, off: u32) -> &str;
    fn sa(&self, off: u32) -> &[String];
    /// Weight (0x66118a): +0x1f0 in Core Rules (the global 0x85f478) when
    /// it is above 0.0, else +0x1ec.
    fn weight(&self, core: bool) -> f32 {
        let a = f32::from_bits(self.n(0x1f0) as u32);
        if core && a > 0.0 {
            a
        } else {
            f32::from_bits(self.n(0x1ec) as u32)
        }
    }
    /// Price in copper (0x4c887d). Core Rules takes the alternate prices
    /// where they are set (0x6610a6: the mode 0x446a40 sets with the
    /// character's rule system), Player's Option the base ones.
    fn price(&self, qty: i32, core: bool) -> i32 {
        let mut v = 0i32;
        for (base, alt, mul) in [
            (0x1f8, 0x208, 500),
            (0x1fc, 0x20c, 100),
            (0x200, 0x210, 10),
            (0x204, 0x214, 1),
        ] {
            let x = match self.n(alt) {
                x if core && x != 0 => x,
                _ => self.n(base),
            };
            v = v.wrapping_add(x.wrapping_mul(mul));
        }
        v.wrapping_mul(qty)
    }
}

impl PartExt for Part {
    fn name(&self) -> &str {
        &self.head.name
    }

    fn n(&self, off: u32) -> i32 {
        let h = &self.head;
        let d = &self.data;
        let b = |v: u8| i32::from(v);
        match off {
            0x4..=0x194 => d.unknown_0x4.map_or(0, |a| at(&a, 0x4, off)),
            0x19c => d.unknown_0x19c,
            0x1e4 => h.unknown_0x1e4,
            0x1ec => h.unknown_0x1ec,
            0x1f0 => h.unknown_0x1f0,
            0x1f4 => d.unknown_0x1f4,
            0x1f8..=0x214 => at(&h.unknown_0x1f8, 0x1f8, off),
            0x218 => d.unknown_0x218,
            0x220 => d.unknown_0x220,
            0x224 => h.unknown_0x224,
            0x228 => d.unknown_0x228,
            0x22c => d.unknown_0x22c,
            0x230 => h.unknown_0x230,
            0x238..=0x23c => at(&h.unknown_0x238, 0x238, off),
            0x248 => h.unknown_0x248,
            0x24c => h.unknown_0x24c,
            0x250..=0x274 => at(&h.unknown_0x250, 0x250, off),
            0x278 => h.unknown_0x278,
            0x27c..=0x2a4 => at(&h.unknown_0x27c, 0x27c, off),
            0x2a8 => h.unknown_0x2a8,
            0x2ac => h.unknown_0x2ac,
            0x2b0 => d.unknown_0x2b0,
            0x2b4..=0x2d4 => at(&h.unknown_0x2b4, 0x2b4, off),
            0x2d8..=0x2fc => at(&h.unknown_0x2d8, 0x2d8, off),
            0x308 => h.unknown_0x308,
            0x30c..=0x354 => at(&h.unknown_0x30c, 0x30c, off),
            0x394..=0x39c => at(&h.unknown_0x394, 0x394, off),
            0x3b8..=0x3c4 => at(&h.unknown_0x3b8, 0x3b8, off),
            0x3c8 => h.unknown_0x3c8,
            0x3cc => h.unknown_0x3cc,
            0x3d4 => h.unknown_0x3d4,
            0x3d8 => h.unknown_0x3d8,
            0x3dc => h.unknown_0x3dc,
            0x3e0 => h.unknown_0x3e0,
            0x3e8 => h.unknown_0x3e8,
            0x3ec => h.unknown_0x3ec,
            0x3f0 => h.unknown_0x3f0,
            0x3f8 => h.unknown_0x3f8,
            0x3fc => h.unknown_0x3fc,
            0x400 => h.unknown_0x400,
            0x404 => h.unknown_0x404,
            0x40c..=0x418 => at(&h.unknown_0x40c, 0x40c, off),
            0x420..=0x424 => at(&d.unknown_0x420, 0x420, off),
            0x42c => d.unknown_0x42c,
            0x448 => d.flag_0x448.map_or(0, b),
            0x44c..=0x454 => at(&h.unknown_0x44c, 0x44c, off),
            0x470..=0x47c => at(&h.unknown_0x470, 0x470, off),
            0x480..=0x488 => at(&d.unknown_0x480, 0x480, off),
            0x48c..=0x4a0 => at(&d.unknown_0x48c, 0x48c, off),
            0x4a4..=0x4c4 => at(&d.old_class_flags, 0x4a4, off),
            0x4c8 => d.unknown_0x4c8,
            0x4cc => h.unknown_0x4cc,
            0x4d0 => d.flag_0x4d0.map_or(0, b),
            0x4e4 => d.unknown_0x4e4,
            0x4f0 => d.unknown_0x4f0,
            0x510..=0x528 => at(&d.unknown_0x510, 0x510, off),
            0x52c => d.unknown_0x52c.unwrap_or(0),
            0x530..=0x56c => at(&d.unknown_0x530, 0x530, off),
            0x570 => d.unknown_0x570,
            0x584..=0x594 => at(&d.unknown_0x584, 0x584, off),
            0x598 => d.unknown_0x598,
            0x59c..=0x5cc => at(&d.unknown_0x59c, 0x59c, off),
            0x5e4..=0x640 => at(&d.unknown_0x5e4, 0x5e4, off),
            0x644 => d.unknown_0x644,
            0x648..=0x64c => at(&d.unknown_0x648, 0x648, off),
            0x650 => d.unknown_0x650,
            0x654 => d.unknown_0x654,
            0x658 => d.unknown_0x658,
            0x65c => d.unknown_0x65c,
            0x660 => d.unknown_0x660,
            0x664..=0x668 => at(&d.unknown_0x664, 0x664, off),
            0x66c..=0x674 => at(&d.unknown_0x66c, 0x66c, off),
            0x678 => d.unknown_0x678,
            0x67c => b(d.flag_0x67c),
            0x67d => b(d.flag_0x67d),
            0x67e => b(d.flag_0x67e),
            0x67f => b(d.flag_0x67f),
            0x680 | 0x681 => b(d.flags_0x680[(off - 0x680) as usize]),
            0x71c => d.unknown_0x71c,
            0x724 => d.flag_0x724.map_or(0, b),
            0x72c..=0x740 => at(&d.unknown_0x72c, 0x72c, off),
            0x744 => d.unknown_0x744,
            0x75c..=0x760 => at(&d.unknown_0x75c, 0x75c, off),
            0x778..=0x77c => at(&d.unknown_0x778, 0x778, off),
            0x780..=0x785 => b(d.flags_0x780[(off - 0x780) as usize]),
            0x788 => d.unknown_0x788,
            0x78c => d.unknown_0x78c,
            0x790 => d.unknown_0x790,
            0x794 => d.unknown_0x794,
            0x798 => d.unknown_0x798,
            0x79c => d.unknown_0x79c,
            0x7a0 => d.unknown_0x7a0,
            0x7a4..=0x7b0 => at(&d.unknown_0x7a4, 0x7a4, off),
            0x7b4 => d.unknown_0x7b4,
            0x7b8 => d.unknown_0x7b8,
            0x7bc => d.unknown_0x7bc,
            0x7c0..=0x7c8 => at(&d.unknown_0x7c0, 0x7c0, off),
            0x7cc..=0x7e4 => at(&d.unknown_0x7cc, 0x7cc, off),
            0x7e8..=0x808 => at(&d.unknown_0x7e8, 0x7e8, off),
            0x80c..=0x818 => at(&h.unknown_0x80c, 0x80c, off),
            0x81c..=0x828 => at(&d.unknown_0x81c, 0x81c, off),
            _ => 0,
        }
    }

    fn s(&self, off: u32) -> &str {
        let h = &self.head;
        let d = &self.data;
        match off {
            0x198 => &h.name,
            0x1b4 => &h.unknown_0x1b4,
            0x1e0 => &d.unknown_0x1e0,
            0x1e8 => &h.unknown_0x1e8,
            0x21c => &d.unknown_0x21c,
            0x234 => &h.unknown_0x234,
            0x240 => &h.unknown_0x240,
            0x244 => &h.unknown_0x244,
            0x300 => &h.unknown_0x300,
            0x304 => &h.description,
            0x3b4 => &h.unknown_0x3b4,
            0x3d0 => &h.unknown_0x3d0,
            0x3e4 => &h.unknown_0x3e4,
            0x3f4 => &h.unknown_0x3f4,
            0x408 => &h.unknown_0x408,
            0x41c => &h.unknown_0x41c,
            0x428 => &d.unknown_0x428,
            0x430 => &d.unknown_0x430,
            0x458 => &h.unknown_0x458,
            0x45c => &h.unknown_0x45c,
            0x460 => &h.unknown_0x460,
            0x464 => &h.unknown_0x464,
            0x468 => &d.unknown_0x468,
            0x46c => &d.unknown_0x46c,
            0x574 | 0x578 | 0x57c => &d.unknown_0x574[((off - 0x574) / 4) as usize],
            0x580 => &d.unknown_0x580,
            0x710 | 0x714 => &d.unknown_0x710[((off - 0x710) / 4) as usize],
            0x718 => &d.unknown_0x718,
            0x720 => &d.unknown_0x720,
            0x868..=0x88c if off.is_multiple_of(4) => {
                &d.unknown_0x868[((off - 0x868) / 4) as usize]
            }
            _ => "",
        }
    }

    fn sa(&self, off: u32) -> &[String] {
        let d = &self.data;
        match off {
            0x1a0 => &d.unknown_0x1a0,
            0x1b8 => &d.unknown_0x1b8,
            0x1cc => &d.unknown_0x1cc,
            0x358 => &d.unknown_0x358,
            0x36c => &d.unknown_0x36c,
            0x380 => &d.unknown_0x380,
            0x3a0 => &d.unknown_0x3a0,
            0x434 => &d.unknown_0x434,
            0x4fc => &d.unknown_0x4fc,
            0x5d0 => &d.unknown_0x5d0,
            0x684 => &d.unknown_0x684,
            0x698 => &d.unknown_0x698,
            0x6ac => &d.unknown_0x6ac,
            0x6c0 => &d.unknown_0x6c0,
            0x6d4 => &d.unknown_0x6d4,
            0x6e8 => &d.unknown_0x6e8,
            0x6fc => &d.unknown_0x6fc,
            0x764 => &d.unknown_0x764,
            0x82c => d.allowed_classes.as_deref().unwrap_or_default(),
            0x840 => &d.unknown_0x840,
            0x854 => &d.unknown_0x854,
            _ => &[],
        }
    }
}

/// A choose-N-of list of a kit (`CNumList`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub count: i32,
    pub names: Vec<String>,
}

/// A kit (`CPartKitOb`) by member offset.
#[derive(Debug, Clone, Default)]
pub struct Kit {
    bytes: HashMap<u32, i32>,
    ints: HashMap<u32, i32>,
    strings: HashMap<u32, String>,
    lists: HashMap<u32, Vec<String>>,
    /// Nonweapon proficiency option groups (+0x330).
    pub nwp_options: Vec<Options>,
    /// Weapon proficiency option groups (+0x568).
    pub wp_options: Vec<Options>,
    /// The class combinations the kit allows (+0x65c), as titles.
    pub combinations: Vec<String>,
    /// Weapons the kit makes free from a level (+0x470 `CPartIDOb`s:
    /// weapon, level).
    pub free_weapons: Vec<(String, i32)>,
    /// Proficiencies the kit makes free from a level (+0x2f8).
    pub free_nwp: Vec<(String, i32)>,
    /// The kit's spell table by level (+0x648 `CPartKitLevelOb`s; `None`
    /// for a null entry).
    pub spell_table: Vec<Option<[i32; 10]>>,
    /// Fixed rogue skill adjustments (+0x174 `CPartIDOb`s: skill, amount).
    pub rogue_adjust: Vec<(String, i32)>,
}

impl Kit {
    /// A byte member, 0 when absent.
    pub fn b(&self, off: u32) -> i32 {
        self.bytes.get(&off).copied().unwrap_or(0)
    }

    /// A byte member as a flag.
    pub fn flag(&self, off: u32) -> bool {
        self.b(off) != 0
    }

    /// A byte pair's maximum (`off + 1`; 0 means 25).
    pub fn max(&self, off: u32) -> i32 {
        match self.b(off + 1) {
            0 => 25,
            v => v,
        }
    }

    pub fn i(&self, off: u32) -> i32 {
        self.ints.get(&off).copied().unwrap_or(0)
    }

    /// An integer member, `default` when the kit's version has none.
    pub fn i_or(&self, off: u32, default: i32) -> i32 {
        self.ints.get(&off).copied().unwrap_or(default)
    }

    pub fn s(&self, off: u32) -> &str {
        self.strings.get(&off).map_or("", String::as_str)
    }

    /// A string list; +0x384 reads the second copy the file holds.
    pub fn sa(&self, off: u32) -> &[String] {
        self.lists.get(&off).map_or(&[], Vec::as_slice)
    }

    pub fn of(d: &PartKitData) -> Self {
        let mut k = Kit::default();
        let mut bytes = |base: u32, v: &[u8]| {
            for (i, x) in v.iter().enumerate() {
                k.bytes.insert(base + i as u32, i32::from(*x));
            }
        };
        bytes(0x4, &d.flags_0x4);
        bytes(0x71, &d.flags_0x71);
        bytes(0x444, &[d.flag_0x444]);
        if let Some(f) = d.flag_0x4b4 {
            bytes(0x4b4, &[f]);
        }
        bytes(0x75, &d.flags_0x75);
        bytes(0x84, &[d.flag_0x84]);
        bytes(0x4c, &d.unknown_0x4c);
        macro_rules! opt_bytes {
            ($($base:literal => $field:expr),* $(,)?) => {
                $( if let Some(v) = &$field { bytes($base, &v[..]); } )*
            };
        }
        opt_bytes!(
            0x73 => d.flags_0x73,
            0x188 => d.flags_0x188,
            0x30c => d.flags_0x30c,
            0x44c => d.flags_0x44c,
            0x464 => d.flags_0x464,
            0x4ac => d.flags_0x4ac,
            0x520 => d.flags_0x520,
            0x438 => d.flags_0x438,
            0x5d4 => d.flags_0x5d4,
            0x5dd => d.flags_0x5dd,
        );
        for (base, v) in [
            (0x1f0, d.flag_0x1f0),
            (0x1fc, d.flag_0x1fc),
            (0x348, d.flag_0x348),
            (0x380, d.flag_0x380),
            (0x508, d.flag_0x508),
            (0x518, d.flag_0x518),
            (0x538, d.flag_0x538),
            (0x5da, d.flag_0x5da),
            (0x580, d.flag_0x580),
            (0x5dc, d.flag_0x5dc),
            (0x618, d.flag_0x618),
            (0x5df, d.flag_0x5df),
        ] {
            if let Some(v) = v {
                bytes(base, &[v]);
            }
        }
        let mut ints = |base: u32, v: &[i32]| {
            for (i, x) in v.iter().enumerate() {
                k.ints.insert(base + 4 * i as u32, *x);
            }
        };
        ints(0x80, &[d.unknown_0x80]);
        ints(0x88, &d.unknown_0x88);
        macro_rules! opt_ints {
            ($($base:literal => $field:expr),* $(,)?) => {
                $( if let Some(v) = &$field { ints($base, &v[..]); } )*
            };
        }
        let one = |v: Option<i32>| v.map(|x| [x]);
        opt_ints!(
            0x670 => one(d.unknown_0x670),
            0x120 => d.unknown_0x120,
            0x13c => one(d.unknown_0x13c),
            0x154 => one(d.unknown_0x154),
            0x16c => d.unknown_0x16c,
            0x18c => d.unknown_0x18c,
            0x1f4 => d.unknown_0x1f4,
            0x200 => d.unknown_0x200,
            0x318 => one(d.unknown_0x318),
            0x320 => one(d.unknown_0x320),
            0x32c => one(d.unknown_0x32c),
            0x34c => d.unknown_0x34c,
            0x368 => one(d.unknown_0x368),
            0x448 => one(d.unknown_0x448),
            0x450 => d.unknown_0x450,
            0x468 => d.unknown_0x468,
            0x50c => d.unknown_0x50c,
            0x51c => one(d.unknown_0x51c),
            0x564 => one(d.unknown_0x564),
            0x43c => d.unknown_0x43c,
            0x5e0 => d.unknown_0x5e0,
            0x5fc => d.unknown_0x5fc,
            0x608 => d.unknown_0x608,
            0x604 => one(d.unknown_0x604),
        );
        for (i, s) in d.unknown_0x9c.iter().enumerate() {
            k.strings.insert(0x9c + 4 * i as u32, s.clone());
        }
        if let Some(s) = &d.unknown_0x674 {
            k.strings.insert(0x674, s.clone());
        }
        if let Some(s) = &d.unknown_0x31c {
            k.strings.insert(0x31c, s.clone());
        }
        if let Some(s) = &d.unknown_0x324 {
            k.strings.insert(0x324, s[0].clone());
            k.strings.insert(0x328, s[1].clone());
        }
        if let Some(s) = &d.unknown_0x61c {
            k.strings.insert(0x61c, s.clone());
        }
        let mut list = |off: u32, v: &Vec<String>| {
            k.lists.insert(off, v.clone());
        };
        for (off, v) in [
            (0x10, &d.unknown_0x10),
            (0x24, &d.unknown_0x24),
            (0x38, &d.unknown_0x38),
            (0xa8, &d.unknown_0xa8),
            (0x384, &d.unknown_0x384),
            (0xbc, &d.unknown_0xbc),
            (0xd0, &d.unknown_0xd0),
            (0x258, &d.unknown_0x258),
            (0xe4, &d.unknown_0xe4),
            (0xf8, &d.unknown_0xf8),
            (0x10c, &d.unknown_0x10c),
        ] {
            list(off, v);
        }
        for (off, v) in [
            (0x128, &d.unknown_0x128),
            (0x140, &d.unknown_0x140),
            (0x158, &d.unknown_0x158),
            (0x1c8, &d.unknown_0x1c8),
            (0x1dc, &d.unknown_0x1dc),
            (0x208, &d.unknown_0x208),
            (0x26c, &d.unknown_0x26c),
            (0x280, &d.unknown_0x280),
            (0x294, &d.unknown_0x294),
            (0x2a8, &d.unknown_0x2a8),
            (0x2bc, &d.unknown_0x2bc),
            (0x2d0, &d.unknown_0x2d0),
            (0x2e4, &d.unknown_0x2e4),
            (0x354, &d.unknown_0x354),
            (0x36c, &d.unknown_0x36c),
            // The second copy of +0x384 is the one the program keeps.
            (0x384, &d.unknown_0x384_again),
            (0x484, &d.unknown_0x484),
            (0x498, &d.unknown_0x498),
            (0x4b8, &d.unknown_0x4b8),
            (0x4cc, &d.unknown_0x4cc),
            (0x4e0, &d.unknown_0x4e0),
            (0x4f4, &d.unknown_0x4f4),
            (0x524, &d.unknown_0x524),
            (0x53c, &d.unknown_0x53c),
            (0x550, &d.unknown_0x550),
            (0x1a0, &d.unknown_0x1a0),
            (0x398, &d.unknown_0x398),
            (0x3ac, &d.unknown_0x3ac),
            (0x3c0, &d.unknown_0x3c0),
            (0x3d4, &d.unknown_0x3d4),
            (0x3e8, &d.unknown_0x3e8),
            (0x3fc, &d.unknown_0x3fc),
            (0x410, &d.unknown_0x410),
            (0x424, &d.unknown_0x424),
            (0x1b4, &d.unknown_0x1b4),
            (0x598, &d.unknown_0x598),
            (0x584, &d.starting_equipment),
            (0x620, &d.unknown_0x620),
            (0x21c, &d.unknown_0x21c),
            (0x230, &d.unknown_0x230),
            (0x5ac, &d.unknown_0x5ac),
            (0x5c0, &d.unknown_0x5c0),
        ] {
            if let Some(v) = v {
                list(off, v);
            }
        }
        let options = |l: &Option<cr_formats::parts::ListOfListsOfOptions>| -> Vec<Options> {
            l.iter()
                .flat_map(|l| &l.lists)
                .filter_map(|o| match o {
                    Object::NumList(n) => Some(Options {
                        count: n.count,
                        names: n.options.clone(),
                    }),
                    _ => None,
                })
                .collect()
        };
        k.nwp_options = options(&d.unknown_0x330);
        k.wp_options = options(&d.unknown_0x568);
        k.combinations = d
            .allowed_classes
            .iter()
            .flatten()
            .filter_map(|o| match o {
                Object::ClassesAllowedHelper(h) => {
                    let h = &h.data;
                    Some(
                        [&h.warrior, &h.rogue, &h.priest, &h.wizard, &h.psionicist]
                            .into_iter()
                            .filter(|n| !n.is_empty())
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("/"),
                    )
                }
                _ => None,
            })
            .collect();
        let ids = |l: &Option<Vec<Object>>| -> Vec<(String, i32)> {
            l.iter()
                .flatten()
                .filter_map(|o| match o {
                    Object::PartId(p) => Some((p.name.clone(), p.part_type)),
                    _ => None,
                })
                .collect()
        };
        k.free_weapons = ids(&d.unknown_0x470);
        k.free_nwp = ids(&d.unknown_0x2f8);
        k.spell_table = d
            .levels
            .iter()
            .flatten()
            .map(|o| match o {
                Object::PartKitLevel(l) => Some(l.unknown_0x4),
                _ => None,
            })
            .collect();
        k.rogue_adjust = d
            .unknown_0x174
            .iter()
            .flatten()
            .filter_map(|o| match o {
                Object::PartId(p) => Some((p.name.clone(), p.part_type)),
                _ => None,
            })
            .collect();
        k
    }
}

/// A class (`CClassOb`) as the generator reads it.
#[derive(Debug, Clone, Default)]
pub struct Klass {
    pub name: String,
    /// Class group, 0 warrior, 1 rogue, 2 priest, 3 wizard, 4 psionicist.
    pub group: i32,
    /// Members +0x10 to +0xe0 by offset (the 33 values both layouts share).
    values: HashMap<u32, i32>,
    flags: HashMap<u32, u8>,
    pub xp: Vec<i32>,
    pub slots: Vec<[i32; 11]>,
    pub alt_slots: Vec<[i32; 11]>,
    pub i2798: i32,
    pub i2ff8: i32,
    pub i3040: i32,
    pub i4860: i32,
    pub i90: i32,
    pub m94: [i32; 3],
    pub c8: i32,
    /// Specialist school (+0x2788).
    pub school: String,
    /// Casts priest spells, wizard spells (+0x2774, +0x2778).
    pub priest_caster: bool,
    pub wizard_caster: bool,
    /// +0x2774..+0x2784: the two above, then a priestly wizard (+0x277c), a
    /// wizardly priest (+0x2780) and no spheres (+0x2784).
    pub i2774: [i32; 5],
    pub s3000: Option<String>,
    lists: HashMap<u32, Vec<String>>,
    /// The proficiencies the class gains by level (+0x3514,
    /// `CPartKitBonusOb`): name, level (+8) and kind (+0xc, 0 for a
    /// nonweapon proficiency).
    pub level_profs: Vec<(String, i32, i32)>,
}

impl Klass {
    /// One of the 33 shared values (+0x10..+0xe0).
    pub fn v(&self, off: u32) -> i32 {
        self.values.get(&off).copied().unwrap_or(0)
    }

    pub fn flag(&self, off: u32) -> bool {
        self.flags.get(&off).is_some_and(|&f| f != 0)
    }

    pub fn sa(&self, off: u32) -> &[String] {
        self.lists.get(&off).map_or(&[], Vec::as_slice)
    }

    /// Experience for a level.
    pub fn xp_for(&self, level: i32) -> i32 {
        self.xp.get(level.max(0) as usize).copied().unwrap_or(0)
    }

    /// The order of the 33 values (`h33`): +0x14 (2), +0x1c (6), +0x34
    /// (6), +0x4c, +0x54 (2), +0x84 (3), +0xb4 (2), +0x10, +0x50, +0xbc
    /// (3), +0xcc (6).
    fn value_offsets() -> Vec<u32> {
        let mut out = Vec::new();
        for (base, n) in [
            (0x14, 2),
            (0x1c, 6),
            (0x34, 6),
            (0x4c, 1),
            (0x54, 2),
            (0x84, 3),
            (0xb4, 2),
            (0x10, 1),
            (0x50, 1),
            (0xbc, 3),
            (0xcc, 6),
        ] {
            for i in 0..n {
                out.push(base + 4 * i);
            }
        }
        out
    }

    pub fn of(class: &Class) -> Self {
        match class {
            Class::Expansion { data: d, .. } => {
                let mut values = HashMap::new();
                let mut seq = Vec::new();
                seq.extend_from_slice(&d.unknown_0x14);
                seq.extend_from_slice(&d.ability_requirements);
                seq.extend_from_slice(&d.unknown_0x34);
                seq.push(d.unknown_0x4c);
                seq.extend_from_slice(&d.unknown_0x54);
                seq.extend_from_slice(&d.unknown_0x84);
                seq.extend_from_slice(&d.unknown_0xb4);
                seq.push(d.id);
                seq.push(d.unknown_0x50);
                seq.extend_from_slice(&d.unknown_0xbc);
                seq.extend_from_slice(&d.unknown_0xcc);
                for (off, v) in Self::value_offsets().into_iter().zip(seq) {
                    values.insert(off, v);
                }
                let mut flags = HashMap::new();
                let mut set = |base: u32, v: &[u8]| {
                    for (i, x) in v.iter().enumerate() {
                        flags.insert(base + i as u32, *x);
                    }
                };
                // In file order; a member read twice keeps the later value.
                set(0x278c, &[d.flag_0x278c]);
                let one = |v: Option<u8>| v.map(|x| vec![x]);
                for (base, v) in [
                    (0x278d, one(d.flag_0x278d)),
                    (0x278e, d.flags_0x278e.map(|a| a.to_vec())),
                    (0x2790, one(d.flag_0x2790)),
                    (0x2791, one(d.flag_0x2791)),
                    (0x2792, one(d.flag_0x2792)),
                    (0x2793, d.flags_0x2793.map(|a| a.to_vec())),
                    (0x279d, one(d.flag_0x279d)),
                    (0x279e, d.flags_0x279e.map(|a| a.to_vec())),
                    (0x27a4, one(d.flag_0x27a4)),
                    (0x2795, d.flags_0x2795.map(|a| a.to_vec())),
                    (0x2797, one(d.flag_0x2797)),
                    (0x27a1, one(d.flag_0x27a1)),
                    (0x2ffc, one(d.flag_0x2ffc)),
                    (0x3554, one(d.flag_0x3554)),
                    (0x3044, d.flags_0x3044.map(|a| a.to_vec())),
                    (0x279c, one(d.flag_0x279c)),
                    (0x3049, d.flags_0x3049.map(|a| a.to_vec())),
                    (0x4864, d.flags_0x4864.map(|a| a.to_vec())),
                    (0x486c, one(d.flag_0x486c)),
                ] {
                    if let Some(v) = v {
                        set(base, &v);
                    }
                }
                // Groups whose members are not consecutive (0x6644af).
                for (offs, v) in [
                    (
                        &[0x486e, 0x48ac, 0x4a58, 0x27a5, 0x486d][..],
                        d.flags_0x486e.map(|a| a.to_vec()),
                    ),
                    (
                        &[0x27a0, 0x27a2, 0x27a3][..],
                        d.flags_0x27a0.map(|a| a.to_vec()),
                    ),
                ] {
                    for (&o, x) in offs.iter().zip(v.unwrap_or_default()) {
                        flags.insert(o, x);
                    }
                }
                let mut lists = HashMap::new();
                for (off, v) in [
                    (0x70, &d.unknown_0x70),
                    (0xa0, &d.alignments),
                    (0x2724, &d.default_abilities),
                    (0x2738, &d.unknown_0x2738),
                    (0x2760, &d.unknown_0x2760),
                    (0x274c, &d.unknown_0x274c),
                    (0x2fa0, &d.unknown_0x2fa0),
                    (0x2fcc, &d.unknown_0x2fcc),
                    (0x3018, &d.unknown_0x3018),
                    (0x302c, &d.unknown_0x302c),
                ] {
                    lists.insert(off, v.clone());
                }
                if let Some(v) = &d.unknown_0x3004 {
                    lists.insert(0x3004, v.clone());
                }
                Self {
                    name: d.name.clone(),
                    group: d.unknown_0x14[0],
                    values,
                    flags,
                    xp: d.experience.to_vec(),
                    slots: d.unknown_0x414.to_vec(),
                    alt_slots: d.unknown_0x159c.to_vec(),
                    i2798: d.unknown_0x2798,
                    i2ff8: d.unknown_0x2ff8,
                    i3040: d.unknown_0x3040,
                    i4860: d.unknown_0x485c[1],
                    i90: d.unknown_0x90,
                    m94: d.unknown_0x94.unwrap_or_default(),
                    c8: d.unknown_0xc8,
                    school: d.unknown_0x2788.clone(),
                    priest_caster: d.unknown_0x2774[0] != 0,
                    wizard_caster: d.unknown_0x2774[1] != 0,
                    i2774: d.unknown_0x2774,
                    s3000: d.unknown_0x3000.clone(),
                    lists,
                    level_profs: d
                        .unknown_0x3514
                        .iter()
                        .filter_map(|o| match o {
                            Object::KitBonus(b) => {
                                Some((b.name.clone(), b.unknown_0x8, b.unknown_0xc))
                            }
                            _ => None,
                        })
                        .collect(),
                }
            }
            // Core Rules 2.0: the 33 values and the tables in the same
            // order; no flags and none of the Expansion's lists.
            Class::Core20(d) => {
                let values = Self::value_offsets()
                    .into_iter()
                    .zip(d.unknown_0x10)
                    .collect();
                let group = d.unknown_0x10[0];
                let slots = d.unknown_0x264.to_vec();
                let any = |t: &[[i32; 11]]| t.iter().flatten().any(|&s| s > 0);
                let casts = any(&slots) || any(&d.unknown_0x13ec);
                // The twelve lists start with the abilities offered, then
                // the Expansion's eight lists in the same order.
                let lists = [0x70, 0xa0, 0x2724, 0x2738, 0x2760, 0x274c, 0x2fa0, 0x2fcc]
                    .into_iter()
                    .zip(d.unknown_0x58.iter().skip(1).cloned())
                    .collect();
                Self {
                    school: d.unknown_0x25d8.clone(),
                    lists,
                    name: d.name.clone(),
                    group,
                    values,
                    xp: d.experience.to_vec(),
                    alt_slots: d.unknown_0x13ec.to_vec(),
                    slots,
                    priest_caster: group == 2 || (casts && group == 0),
                    wizard_caster: group == 3 || (casts && group == 1),
                    s3000: Some("Any".into()),
                    ..Self::default()
                }
            }
        }
    }
}

#[cfg(test)]
impl Klass {
    /// A class with the given values (+0x10..+0xe0), flags and lists.
    pub fn synthetic(
        name: &str,
        group: i32,
        values: &[(u32, i32)],
        flags: &[u32],
        lists: &[(u32, &[&str])],
    ) -> Self {
        let mut values: HashMap<u32, i32> = values.iter().copied().collect();
        values.insert(0x14, group);
        Self {
            name: name.into(),
            group,
            values,
            flags: flags.iter().map(|&f| (f, 1)).collect(),
            lists: lists
                .iter()
                .map(|(o, l)| (*o, l.iter().map(|s| s.to_string()).collect()))
                .collect(),
            xp: (0..=20).map(|l| l * 1000).collect(),
            slots: vec![[0; 11]; 21],
            alt_slots: vec![[0; 11]; 21],
            s3000: Some("Any".into()),
            ..Self::default()
        }
    }
}

/// A spell record (`CSpellsOb`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spell {
    pub name: String,
    /// The "wizard spell" flag (+0x68).
    pub wizard: bool,
    pub level: i32,
    pub schools: Vec<String>,
    pub spheres: Vec<String>,
}

/// A psionic power (`CPsionicPowerOb`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Power {
    pub name: String,
    /// Discipline index (+0x28).
    pub discipline: i32,
    /// 0 science, else devotion (+0x2c).
    pub devotion: bool,
    /// Minimum level (+0x44).
    pub level: i32,
    pub prerequisites: Vec<String>,
}

/// A class combination of `AllowedU.dat`, one class name per group slot:
/// warrior, rogue, priest, wizard, psionicist.
pub type Combination = [String; 5];

/// Everything the generator reads, loaded once per installation.
#[derive(Debug, Clone, Default)]
pub struct Data {
    pub races: Vec<Race>,
    pub classes: Vec<Klass>,
    /// Parts by type, `Parts.dat` then `PartsU.dat`.
    pub parts: Vec<Vec<Part>>,
    /// How many parts of each type come from `Parts.dat`.
    pub db_count: Vec<usize>,
    /// Per type, the standard and the user map: upper-cased name to index
    /// (a later duplicate in one file replaces an earlier one).
    maps: Vec<[HashMap<String, usize>; 2]>,
    /// The kit of each Character Kits part.
    pub kits: Vec<Option<Kit>>,
    pub wizard_spells: Vec<Spell>,
    pub priest_spells: Vec<Spell>,
    pub powers: Vec<Power>,
    /// Combinations by race; a later record of a race replaces an earlier.
    pub allowed: HashMap<String, Vec<Combination>>,
    /// The tight weapon groups' ids (CPartDoc +0x248, from the program).
    pub tight_groups: HashMap<String, i32>,
    /// The texts and tables read from the program.
    pub program: Program,
}

fn upper(s: &str) -> String {
    s.to_uppercase()
}

impl Data {
    /// The program text `tx`.
    pub fn t(&self, tx: Tx) -> &str {
        self.program.text(tx)
    }

    /// `s` is one of the program texts `any`.
    pub fn is(&self, s: &str, any: &[Tx]) -> bool {
        any.iter().any(|&t| s == self.t(t))
    }

    /// No records, every part type present.
    pub fn empty() -> Self {
        Data {
            parts: vec![Vec::new(); TYPES],
            db_count: vec![0; TYPES],
            maps: (0..TYPES).map(|_| Default::default()).collect(),
            ..Default::default()
        }
    }

    pub fn load(install: &Installation, flavor: Flavor) -> Result<Self> {
        let mut d = Self::empty();
        let image = Image::open(install, flavor)?;
        if flavor == Flavor::Expansion {
            // The part document's constructor 0x677978 (adds: 0x572030).
            d.tight_groups = image
                .id_tables(0x677978, 0x3800, 0x572030)
                .remove(&0x248)
                .unwrap_or_default()
                .into_iter()
                .collect();
        }
        d.program = Program::read(&image, flavor)?;
        for object in install.list_objects("Race")? {
            if let Object::Race(r) = object {
                d.races.push(*r);
            }
        }
        for object in install.list_objects("Class")? {
            if let Object::Class(c) = object {
                d.classes.push(Klass::of(&c));
            }
        }
        for (user, relative) in ["Database/Parts.dat", "UserDbas/PartsU.dat"]
            .into_iter()
            .enumerate()
        {
            if install.path(relative).is_none() {
                continue;
            }
            let file =
                PartsFile::parse(&install.read(relative)?).map_err(|source| Error::Format {
                    path: relative.into(),
                    source,
                })?;
            for (t, array) in file.arrays.into_iter().enumerate() {
                for object in array {
                    if let Object::Part(p) = object {
                        let i = d.parts[t].len();
                        d.maps[t][user].insert(upper(&p.head.name), i);
                        d.parts[t].push(*p);
                    }
                }
                if user == 0 {
                    d.db_count[t] = d.parts[t].len();
                }
            }
        }
        d.kits = d.parts[KITS]
            .iter()
            .map(|p| match p.data.kit.as_ref().and_then(|k| k.as_deref()) {
                Some(Object::PartKit(k)) => k.data.as_ref().map(Kit::of),
                _ => None,
            })
            .collect();
        for relative in ["Database/Spells.dat", "UserDbas/SpellsU.dat"] {
            if install.path(relative).is_none() {
                continue;
            }
            let data = install.read(relative)?;
            if data.len() < 4 {
                continue;
            }
            let file = SpellsFile::parse(&data).map_err(|source| Error::Format {
                path: relative.into(),
                source,
            })?;
            let spells = |list: &[Object]| -> Vec<Spell> {
                list.iter()
                    .filter_map(|o| match o {
                        Object::Spell(s) => Some(Spell {
                            name: s.name.clone(),
                            wizard: s.wizard != 0,
                            level: s.level,
                            schools: s.schools.clone(),
                            spheres: s.spheres.clone(),
                        }),
                        _ => None,
                    })
                    .collect()
            };
            d.wizard_spells.extend(spells(&file.wizard));
            d.priest_spells.extend(spells(&file.priest));
        }
        for object in install.list_objects("Psionic")? {
            if let Object::PsionicPower(p) = object {
                d.powers.push(Power {
                    name: p.name.clone(),
                    discipline: p.unknown_0x28 as i32,
                    devotion: p.unknown_0x2c != 0,
                    level: p.unknown_0x44 as i32,
                    prerequisites: p.prerequisites.clone(),
                });
            }
        }
        if install.path("UserDbas/AllowedU.dat").is_some() {
            for object in install.list_objects("Allowed")? {
                if let Object::ClassesAllowed(a) = object {
                    let combos = a
                        .classes
                        .iter()
                        .filter_map(|o| match o {
                            Object::ClassesAllowedHelper(h) => {
                                let h = &h.data;
                                Some([
                                    h.warrior.clone(),
                                    h.rogue.clone(),
                                    h.priest.clone(),
                                    h.wizard.clone(),
                                    h.psionicist.clone(),
                                ])
                            }
                            _ => None,
                        })
                        .collect();
                    d.allowed.insert(a.race.clone(), combos);
                }
            }
        }
        Ok(d)
    }

    pub fn race(&self, name: &str) -> Option<usize> {
        self.races.iter().position(|r| r.name == name)
    }

    pub fn class(&self, name: &str) -> Option<usize> {
        self.classes.iter().position(|c| c.name == name)
    }

    /// The first part of a type with exactly that name (`D.part`).
    pub fn part_exact(&self, t: usize, name: &str) -> Option<usize> {
        self.parts[t].iter().position(|p| p.head.name == name)
    }

    /// 0x465e59 (one type): the standard map, then the user map, without
    /// regard to case.
    pub fn find_in(&self, t: usize, name: &str) -> Option<&Part> {
        self.find_index(t, &upper(name)).map(|i| &self.parts[t][i])
    }

    fn find_index(&self, t: usize, upper: &str) -> Option<usize> {
        self.maps[t][0]
            .get(upper)
            .or_else(|| self.maps[t][1].get(upper))
            .copied()
    }

    /// 0x465f6c (any type), searching the types in [`ANY_TYPE_ORDER`].
    pub fn find(&self, name: &str) -> Option<&Part> {
        let u = upper(name);
        ANY_TYPE_ORDER
            .iter()
            .find_map(|&t| self.find_index(t, &u).map(|i| &self.parts[t][i]))
    }

    /// The kit of the part 0x465f6c finds by that name (0x6618ce), if it
    /// is a kit.
    pub fn find_kit(&self, name: &str) -> Option<&Kit> {
        let u = upper(name);
        let t = *ANY_TYPE_ORDER
            .iter()
            .find(|&&t| self.find_index(t, &u).is_some())?;
        if t != KITS {
            return None;
        }
        self.kits[self.find_index(t, &u)?].as_ref()
    }

    /// 0x468356: the type of the first map holding the name, else -1.
    pub fn part_type(&self, name: &str) -> i32 {
        let u = upper(name);
        ANY_TYPE_ORDER
            .iter()
            .find(|&&t| self.find_index(t, &u).is_some())
            .map_or(-1, |&t| t as i32)
    }

    /// A weapon of `PartsU.dat` with that name, without regard to case
    /// (0x468d95).
    pub fn user_weapon(&self, name: &str) -> bool {
        self.maps[WEAPONS][1].contains_key(&upper(name))
    }
}
