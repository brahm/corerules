//! Names, 0x4b9da3: the generator picked by the subrace name, each
//! concatenating syllables from the program's tables, read from the
//! installation's program ([`super::program`]).

use super::program::{BUP, NORM, Program, Tx};
use super::texts::{DAVID, GRINKA, LOUISE, MUGWORT, NORMA, THE_SEP};
use super::{Gen, Rand};

/// The generators by race keyword (0x4b9da3 tests the keywords in the
/// program's order; Evermore's 0x536600 has the same order).
#[derive(Debug, Clone, Copy)]
enum By {
    HalfElf,
    Drow,
    Elf,
    Dwarf,
    Human,
    Gnome,
    Halfling,
    Ogre,
    HalfOrc,
    Orc(i32),
    Lizard(i32),
    ThriKreen,
    Mongrel,
    Wemic,
    Bullywug,
    Aarakocra,
    Giff,
    Satyr,
    Githzerai,
    Swanmay,
}

/// The generator of each of the program's 31 race keywords.
const KEYWORDS: [By; 31] = [
    By::HalfElf,
    By::Drow,
    By::Elf,
    By::Dwarf,
    By::Human,
    By::Gnome,
    By::Halfling,
    By::Ogre,
    By::HalfOrc,
    By::Orc(4),
    By::Orc(1),
    By::Orc(6),
    By::Orc(2),
    By::Orc(9),
    By::Orc(9),
    By::Orc(8),
    By::Lizard(500),
    By::Lizard(400),
    By::ThriKreen,
    By::Mongrel,
    By::Wemic,
    By::Bullywug,
    By::Aarakocra,
    By::Giff,
    By::Satyr,
    By::Satyr,
    By::Satyr,
    By::Githzerai,
    By::Swanmay,
    By::Ogre,
    By::Halfling,
];

/// The name generators' inputs: the program's tables and texts, and the
/// stream they draw from.
pub(super) struct Namer<'a, 'r> {
    pub program: &'a Program,
    pub r: &'r mut Rand,
}

impl Gen<'_> {
    /// 0x4b9da3 on the generation's stream.
    pub(super) fn make_name(&mut self, race: &str, male: bool) -> String {
        Namer {
            program: &self.d.program,
            r: &mut *self.r,
        }
        .make_name(race, male)
    }
}

impl<'a> Namer<'a, '_> {
    fn t(&self, tx: Tx) -> &'a str {
        self.program.text(tx)
    }

    /// `mod(n)` (0x554d70).
    fn modn(&mut self, n: i32) -> i32 {
        if n == 0 { 0 } else { self.r.rand() % n }
    }

    /// 0x4b8eda: `table[mod(n)]` of the table `local` of the Tsrp3.exe
    /// name generator `f`.
    fn pick(&mut self, f: u32, local: u32) -> &'a str {
        let program = self.program;
        let table = program.table(f, local);
        if table.is_empty() {
            return "";
        }
        &table[self.modn(table.len() as i32) as usize]
    }

    /// 0x4bb283
    fn nm_human(&mut self, male: bool) -> String {
        if male {
            self.pick(0x4b8b0b, 0x194).into()
        } else {
            self.pick(0x4b8f34, 0x164).into()
        }
    }

    /// 0x4b928b
    fn nm_elf(&mut self, male: bool, drow: bool) -> String {
        let mut s = String::from(self.pick(0x4b928b, 0x94));
        let mut i = 0;
        while i <= self.modn(5) {
            i += 1;
            let low = self.modn(10) < 4;
            let syl = match (male, low) {
                (false, true) | (true, false) => self.pick(0x4b928b, 0xc4),
                _ => self.pick(0x4b928b, 0x70),
            };
            s += syl;
            s += self.pick(0x4b928b, 0x58);
            if drow && self.modn(2) == 0 {
                s.push('\'');
                if self.modn(1) == 0 {
                    s += self.pick(0x4b928b, 0x58);
                }
            }
        }
        if self.modn(4) != 0 {
            s += self.pick(0x4b928b, 0xa8);
            s += self.pick(0x4b928b, 0x38);
        }
        s
    }

    /// 0x4b97dd
    fn nm_dwarf(&mut self, male: bool) -> String {
        let mut s = String::from(self.pick(0x4b97dd, 0x74));
        let mut i = 0;
        while self.modn(2) >= i {
            i += 1;
            s += self.pick(0x4b97dd, 0x5c);
            s += self.pick(0x4b97dd, 0x44);
        }
        if !male {
            s += self.pick(0x4b97dd, 0xc8);
        }
        s.push(' ');
        if self.modn(2) == 1 {
            s += self.pick(0x4b97dd, 0x130);
            s += self.pick(0x4b97dd, 0xb8);
        } else {
            s += self.pick(0x4b97dd, 0x8c);
            s += self.pick(0x4b97dd, 0xf8);
        }
        s
    }

    /// 0x4ba689
    fn nm_halfelf(&mut self, male: bool) -> String {
        if self.modn(2) == 1 {
            self.nm_human(male)
        } else {
            self.nm_elf(male, false)
        }
    }

    /// 0x4ba5e0: two fixed names per sex.
    fn nm_gnome(&mut self, male: bool) -> String {
        let one = self.modn(2) == 1;
        self.t(match (male, one) {
            (false, true) => NORMA,
            (false, false) => LOUISE,
            (true, true) => NORM,
            (true, false) => DAVID,
        })
        .into()
    }

    /// 0x4ba82f
    fn nm_halfling(&mut self, male: bool) -> String {
        let mut s = String::from(if male {
            self.pick(0x4ba82f, 0x8c)
        } else {
            self.pick(0x4ba82f, 0xa8)
        });
        s.push(' ');
        if self.modn(2) == 1 {
            s += self.pick(0x4ba82f, 0x68);
            s += self.pick(0x4ba82f, 0x4c);
        } else {
            s += self.pick(0x4ba82f, 0x30);
            s += self.pick(0x4ba82f, 0x1c);
        }
        s
    }

    /// 0x4ba740
    fn nm_ogre(&mut self) -> String {
        self.pick(0x4ba740, 0x70).into()
    }

    /// 0x4ba6ea: fixed, no draw.
    fn nm_halforc(&self, male: bool) -> String {
        self.t(if male { MUGWORT } else { GRINKA }).into()
    }

    /// 0x4bc074
    fn nm_orc(&mut self, male: bool, kind: i32) -> String {
        let mut n = self.modn(1);
        if kind == 8 {
            n = 0;
        }
        if kind == 1 || kind == 2 {
            n = 1;
        }
        let mut s = String::from(self.pick(0x4bc074, 0x98));
        for _ in 0..n {
            s += if self.modn(2) == 1 {
                self.pick(0x4bc074, 0x114)
            } else {
                self.pick(0x4bc074, 0x44)
            };
        }
        if !male {
            s += self.pick(0x4bc074, 0xc0);
        }
        s += self.t(THE_SEP);
        s += self.pick(0x4bc074, 0xb4);
        s += self.pick(0x4bc074, 0x74);
        s
    }

    /// 0x4bbd92 (400 alaghi, 500 lizard man)
    fn nm_lizard(&mut self, kind: i32) -> String {
        let mut s = String::from(self.pick(0x4bbd92, 0x34));
        if kind == 400 {
            s += if self.modn(9) < 8 {
                self.pick(0x4bbd92, 0x44)
            } else {
                self.pick(0x4bbd92, 0x6c)
            };
        } else if kind == 500 {
            s += if self.modn(9) < 9 {
                self.pick(0x4bbd92, 0x6c)
            } else {
                self.pick(0x4bbd92, 0x44)
            };
        }
        s
    }

    /// 0x4bbbda
    fn nm_thrikreen(&mut self) -> String {
        let n = self.modn(2) + 1;
        let mut flag = true;
        let mut s = String::from(self.pick(0x4bbbda, 0x34));
        if self.modn(7) == 0 {
            s.push('\'');
        }
        for _ in 0..=n {
            if !flag {
                flag = true;
            } else if self.modn(3) == 0 {
                s.push('\'');
                flag = false;
            }
            s += self.pick(0x4bbbda, 0x50);
        }
        s
    }

    /// 0x4bba12
    fn nm_wemic(&mut self, male: bool) -> String {
        let mut s = String::from(self.pick(0x4bba12, 0x40));
        s += self.pick(0x4bba12, 0x5c);
        if !male {
            s += self.pick(0x4bba12, 0x20);
        }
        s
    }

    /// 0x4bb80a
    fn nm_bullywug(&mut self, male: bool) -> String {
        let n = self.modn(2) + 1;
        let mut s = String::from(self.pick(0x4bb80a, 0x44));
        for _ in 0..=n {
            s += self.pick(0x4bb80a, 0x64);
        }
        if !male {
            s += self.pick(0x4bb80a, 0x24);
        } else if self.modn(5) == 0 {
            s += self.t(BUP);
        }
        s
    }

    /// 0x4bae7e
    fn nm_aarakocra(&mut self, male: bool) -> String {
        let n = self.modn(2);
        let mut flag = true;
        let mut s = String::from(self.pick(0x4bae7e, 0x2c));
        s += self.pick(0x4bae7e, 0x90);
        for _ in 0..=n {
            if flag && self.modn(30) == 0 {
                s.push('\'');
                flag = false;
            }
            s += self.pick(0x4bae7e, 0x78);
            s += self.pick(0x4bae7e, 0x90);
        }
        s += self.pick(0x4bae7e, 0x54);
        if !male {
            s += self.pick(0x4bae7e, 0xb0);
        }
        s
    }

    /// 0x4bb2d7
    fn nm_giff(&mut self, male: bool) -> String {
        let n = self.modn(1) + 1;
        let mut s = String::from(self.pick(0x4bb2d7, 0x44));
        for _ in 0..=n {
            s += self.pick(0x4bb2d7, 0x60);
        }
        if !male {
            s += self.pick(0x4bb2d7, 0x28);
        }
        s
    }

    /// 0x4bb4b5 (the kind is not used).
    fn nm_satyr(&mut self, male: bool) -> String {
        let b = self.modn(2);
        let mut s = String::from(self.pick(0x4bb4b5, 0x9c));
        let mut i = 0;
        while (b >> i) != 0 {
            s += self.pick(0x4bb4b5, 0x4c);
            i += 1;
        }
        if male {
            if self.modn(8) < 8 {
                s += self.pick(0x4bb4b5, 0xb0);
            }
        } else if self.modn(10) < 10 {
            s += self.pick(0x4bb4b5, 0x5c);
        }
        s
    }

    /// 0x4babc0
    fn nm_githzerai(&mut self) -> String {
        let n = self.modn(3) + 1;
        let mut flag = true;
        let mut s = String::from(self.pick(0x4babc0, 0x7c));
        s += self.pick(0x4babc0, 0x54);
        for _ in 0..=n {
            if flag && self.modn(4) == 0 {
                s.push('\'');
                flag = false;
            }
            s += self.pick(0x4babc0, 0x3c);
            s += self.pick(0x4babc0, 0x54);
        }
        s
    }

    /// 0x4bc580: a random race's generator.
    fn nm_mongrel(&mut self, male: bool) -> String {
        match self.modn(30) {
            1 => self.nm_elf(male, true),
            2 => self.nm_elf(male, false),
            3 => self.nm_dwarf(male),
            4 => self.nm_human(male),
            5 => self.nm_gnome(male),
            6 | 28 => self.nm_halfling(male),
            7 => self.nm_ogre(),
            8 => self.nm_halforc(male),
            9 => self.nm_orc(male, 4),
            10 => self.nm_orc(male, 12),
            11 => self.nm_orc(male, 1),
            12 => self.nm_orc(male, 6),
            13 => self.nm_orc(male, 2),
            14 | 15 => self.nm_orc(male, 9),
            16 => self.nm_orc(male, 8),
            17 => self.nm_lizard(500),
            18 => self.nm_lizard(400),
            19 => self.nm_thrikreen(),
            20 => self.nm_wemic(male),
            21 => self.nm_bullywug(male),
            22 => self.nm_aarakocra(male),
            23 => self.nm_giff(male),
            24..=26 => self.nm_satyr(male),
            27 => self.nm_githzerai(),
            _ => self.nm_human(male),
        }
    }

    /// 0x4b9da3(subrace name, male): the first key the upper-cased name
    /// contains picks the generator.
    pub(super) fn make_name(&mut self, race: &str, male: bool) -> String {
        let u = race.to_uppercase();
        let found = self
            .program
            .name_keys
            .iter()
            .position(|k| u.contains(k.as_str()));
        let Some(by) = found.and_then(|i| KEYWORDS.get(i)) else {
            return self.nm_human(male);
        };
        match *by {
            By::HalfElf => self.nm_halfelf(male),
            By::Drow => self.nm_elf(male, true),
            By::Elf => self.nm_elf(male, false),
            By::Dwarf => self.nm_dwarf(male),
            By::Human => self.nm_human(male),
            By::Gnome => self.nm_gnome(male),
            By::Halfling => self.nm_halfling(male),
            By::Ogre => self.nm_ogre(),
            By::HalfOrc => self.nm_halforc(male),
            By::Orc(kind) => self.nm_orc(male, kind),
            By::Lizard(kind) => self.nm_lizard(kind),
            By::ThriKreen => self.nm_thrikreen(),
            By::Mongrel => self.nm_mongrel(male),
            By::Wemic => self.nm_wemic(male),
            By::Bullywug => self.nm_bullywug(male),
            By::Aarakocra => self.nm_aarakocra(male),
            By::Giff => self.nm_giff(male),
            By::Satyr => self.nm_satyr(male),
            By::Githzerai => self.nm_githzerai(),
            By::Swanmay => self.nm_human(false),
        }
    }
}
