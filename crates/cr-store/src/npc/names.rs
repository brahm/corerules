//! Names, 0x4b9da3: the generator picked by the subrace name, each
//! concatenating syllables from the program's tables ([`super::program`]).
//! `pick` is 0x4b8eda: `table[mod(n)]` (call site 0x4b8f11).

use super::Gen;
use super::program::*;

impl Gen<'_> {
    fn pick(&mut self, table: &'static [&'static str]) -> &'static str {
        if table.is_empty() {
            return "";
        }
        table[self.modn(table.len() as i32) as usize]
    }

    /// 0x4bb283
    fn nm_human(&mut self, male: bool) -> String {
        if male {
            self.pick(HUMAN_MALE_194).into()
        } else {
            self.pick(HUMAN_FEMALE_164).into()
        }
    }

    /// 0x4b928b
    fn nm_elf(&mut self, male: bool, drow: bool) -> String {
        let mut s = String::from(self.pick(ELF_94));
        let mut i = 0;
        while i <= self.modn(5) {
            i += 1;
            let low = self.modn(10) < 4;
            let syl = match (male, low) {
                (false, true) | (true, false) => self.pick(ELF_C4),
                _ => self.pick(ELF_70),
            };
            s += syl;
            s += self.pick(ELF_58);
            if drow && self.modn(2) == 0 {
                s.push('\'');
                if self.modn(1) == 0 {
                    s += self.pick(ELF_58);
                }
            }
        }
        if self.modn(4) != 0 {
            s += self.pick(ELF_A8);
            s += self.pick(ELF_38);
        }
        s
    }

    /// 0x4b97dd
    fn nm_dwarf(&mut self, male: bool) -> String {
        let mut s = String::from(self.pick(DWARF_74));
        let mut i = 0;
        while self.modn(2) >= i {
            i += 1;
            s += self.pick(DWARF_5C);
            s += self.pick(DWARF_44);
        }
        if !male {
            s += self.pick(DWARF_C8);
        }
        s.push(' ');
        if self.modn(2) == 1 {
            s += self.pick(DWARF_130);
            s += self.pick(DWARF_B8);
        } else {
            s += self.pick(DWARF_8C);
            s += self.pick(DWARF_F8);
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
        match (male, one) {
            (false, true) => "Norma",
            (false, false) => "Louise",
            (true, true) => GNOME_MALE,
            (true, false) => "David",
        }
        .into()
    }

    /// 0x4ba82f
    fn nm_halfling(&mut self, male: bool) -> String {
        let mut s = String::from(if male {
            self.pick(HALFLING_8C)
        } else {
            self.pick(HALFLING_A8)
        });
        s.push(' ');
        if self.modn(2) == 1 {
            s += self.pick(HALFLING_68);
            s += self.pick(HALFLING_4C);
        } else {
            s += self.pick(HALFLING_30);
            s += self.pick(HALFLING_1C);
        }
        s
    }

    /// 0x4ba740
    fn nm_ogre(&mut self) -> String {
        self.pick(OGRE_70).into()
    }

    /// 0x4ba6ea: fixed, no draw.
    fn nm_halforc(male: bool) -> String {
        if male { "Mugwort" } else { "Grinka" }.into()
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
        let mut s = String::from(self.pick(ORC_98));
        for _ in 0..n {
            s += if self.modn(2) == 1 {
                self.pick(ORC_114)
            } else {
                self.pick(ORC_44)
            };
        }
        if !male {
            s += self.pick(ORC_C0);
        }
        s += " the ";
        s += self.pick(ORC_B4);
        s += self.pick(ORC_74);
        s
    }

    /// 0x4bbd92 (400 alaghi, 500 lizard man)
    fn nm_lizard(&mut self, kind: i32) -> String {
        let mut s = String::from(self.pick(LIZARD_34));
        if kind == 400 {
            s += if self.modn(9) < 8 {
                self.pick(LIZARD_44)
            } else {
                self.pick(LIZARD_6C)
            };
        } else if kind == 500 {
            s += if self.modn(9) < 9 {
                self.pick(LIZARD_6C)
            } else {
                self.pick(LIZARD_44)
            };
        }
        s
    }

    /// 0x4bbbda
    fn nm_thrikreen(&mut self) -> String {
        let n = self.modn(2) + 1;
        let mut flag = true;
        let mut s = String::from(self.pick(THRIKREEN_34));
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
            s += self.pick(THRIKREEN_50);
        }
        s
    }

    /// 0x4bba12
    fn nm_wemic(&mut self, male: bool) -> String {
        let mut s = String::from(self.pick(WEMIC_40));
        s += self.pick(WEMIC_5C);
        if !male {
            s += self.pick(WEMIC_20);
        }
        s
    }

    /// 0x4bb80a
    fn nm_bullywug(&mut self, male: bool) -> String {
        let n = self.modn(2) + 1;
        let mut s = String::from(self.pick(BULLYWUG_44));
        for _ in 0..=n {
            s += self.pick(BULLYWUG_64);
        }
        if !male {
            s += self.pick(BULLYWUG_24);
        } else if self.modn(5) == 0 {
            s += BULLYWUG_SUFFIX;
        }
        s
    }

    /// 0x4bae7e
    fn nm_aarakocra(&mut self, male: bool) -> String {
        let n = self.modn(2);
        let mut flag = true;
        let mut s = String::from(self.pick(AARAKOCRA_2C));
        s += self.pick(AARAKOCRA_90);
        for _ in 0..=n {
            if flag && self.modn(30) == 0 {
                s.push('\'');
                flag = false;
            }
            s += self.pick(AARAKOCRA_78);
            s += self.pick(AARAKOCRA_90);
        }
        s += self.pick(AARAKOCRA_54);
        if !male {
            s += self.pick(AARAKOCRA_B0);
        }
        s
    }

    /// 0x4bb2d7
    fn nm_giff(&mut self, male: bool) -> String {
        let n = self.modn(1) + 1;
        let mut s = String::from(self.pick(GIFF_44));
        for _ in 0..=n {
            s += self.pick(GIFF_60);
        }
        if !male {
            s += self.pick(GIFF_28);
        }
        s
    }

    /// 0x4bb4b5 (the kind is not used).
    fn nm_satyr(&mut self, male: bool) -> String {
        let b = self.modn(2);
        let mut s = String::from(self.pick(SATYR_9C));
        let mut i = 0;
        while (b >> i) != 0 {
            s += self.pick(SATYR_4C);
            i += 1;
        }
        if male {
            if self.modn(8) < 8 {
                s += self.pick(SATYR_B0);
            }
        } else if self.modn(10) < 10 {
            s += self.pick(SATYR_5C);
        }
        s
    }

    /// 0x4babc0
    fn nm_githzerai(&mut self) -> String {
        let n = self.modn(3) + 1;
        let mut flag = true;
        let mut s = String::from(self.pick(GITHZERAI_7C));
        s += self.pick(GITHZERAI_54);
        for _ in 0..=n {
            if flag && self.modn(4) == 0 {
                s.push('\'');
                flag = false;
            }
            s += self.pick(GITHZERAI_3C);
            s += self.pick(GITHZERAI_54);
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
            8 => Self::nm_halforc(male),
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
        const KEYS: [&str; 31] = [
            "HALF-ELF",
            "DROW",
            "ELF",
            "DWARF",
            "HUMAN",
            "GNOME",
            "HALFLING",
            "HALF-OGRE",
            "HALF-ORC",
            "ORC",
            "KOBOLD",
            "HOBGOBLIN",
            "GOBLIN",
            "GNOLL",
            "FLIND",
            "BUGBEAR",
            "LIZARD MAN",
            "ALAGHI",
            "THRI-KREEN",
            "MONGRELMAN",
            "WEMIC",
            "BULLYWUG",
            "AARAKOCRA",
            "GIFF",
            "SATYR",
            "CENTAUR",
            "MINOTAUR",
            "GITHZERAI",
            "SWANMAY",
            "OGRE",
            "HOBBIT",
        ];
        let Some(key) = KEYS.iter().find(|k| u.contains(*k)) else {
            return self.nm_human(male);
        };
        match *key {
            "HALF-ELF" => self.nm_halfelf(male),
            "DROW" => self.nm_elf(male, true),
            "ELF" => self.nm_elf(male, false),
            "DWARF" => self.nm_dwarf(male),
            "HUMAN" => self.nm_human(male),
            "GNOME" => self.nm_gnome(male),
            "HALFLING" | "HOBBIT" => self.nm_halfling(male),
            "HALF-OGRE" | "OGRE" => self.nm_ogre(),
            "HALF-ORC" => Self::nm_halforc(male),
            "ORC" => self.nm_orc(male, 4),
            "KOBOLD" => self.nm_orc(male, 1),
            "HOBGOBLIN" => self.nm_orc(male, 6),
            "GOBLIN" => self.nm_orc(male, 2),
            "GNOLL" | "FLIND" => self.nm_orc(male, 9),
            "BUGBEAR" => self.nm_orc(male, 8),
            "LIZARD MAN" => self.nm_lizard(500),
            "ALAGHI" => self.nm_lizard(400),
            "THRI-KREEN" => self.nm_thrikreen(),
            "MONGRELMAN" => self.nm_mongrel(male),
            "WEMIC" => self.nm_wemic(male),
            "BULLYWUG" => self.nm_bullywug(male),
            "AARAKOCRA" => self.nm_aarakocra(male),
            "GIFF" => self.nm_giff(male),
            "SATYR" | "CENTAUR" | "MINOTAUR" => self.nm_satyr(male),
            "GITHZERAI" => self.nm_githzerai(),
            "SWANMAY" => self.nm_human(false),
            _ => self.nm_human(male),
        }
    }
}
