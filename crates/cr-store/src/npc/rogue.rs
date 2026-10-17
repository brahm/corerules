//! Rogue skills (0x4c751c): the skill list of the first class that has
//! them, the kit's fixed adjustments, the kit's random per-level bonus and
//! the distribution of the class's (or kit's) discretionary points.

use super::Gen;
use super::data::{CLASS_ABILITIES, DISADVANTAGES, PartExt, TRAITS};
use super::program::Tx;
use super::texts::*;
use crate::sheet::tables;

/// 0x59ad4d: the first keyword found in the upper-cased skill name gives
/// its id, 1 to 13.
const SKILL_IDS: [Tx; 13] = [
    PICK_POCKETS,
    OPEN_LOCKS_UC,
    FIND_REMOVE_TRAPS,
    MOVE_SILENTLY,
    HIDE_IN_SHADOWS,
    DETECT_NOISE,
    CLIMB_WALLS,
    READ_LANGUAGES,
    DETECT_MAGIC,
    DETECT_ILLUSION,
    BRIBE,
    TUNNELING,
    ESCAPING_BONDS,
];

/// A skill's highest value the distribution fills to.
const CAP: i32 = 95;

fn skill_id<'t>(name: &str, text: impl Fn(Tx) -> &'t str) -> usize {
    let upper = name.to_uppercase();
    SKILL_IDS
        .iter()
        .position(|&k| upper.contains(text(k)))
        .map_or(0, |i| i + 1)
}

/// Which Dexterity score a skill's table reads (0x59b19d: vtable +0xd0
/// aim, +0xd4 balance, +0xc8 Dexterity).
#[derive(Clone, Copy)]
enum Score {
    Aim,
    Balance,
    Dexterity,
}

fn dexterity_table(id: usize) -> Option<(&'static [i32; 26], Score)> {
    Some(match id {
        1 => (&tables::PICK_POCKETS, Score::Aim),
        2 => (&tables::OPEN_LOCKS, Score::Aim),
        3 => (&tables::FIND_TRAPS, Score::Aim),
        4 => (&tables::MOVE_SILENTLY, Score::Balance),
        5 => (&tables::HIDE_IN_SHADOWS, Score::Balance),
        7 => (&tables::CLIMB_WALLS, Score::Balance),
        12 => (&tables::TUNNELING, Score::Dexterity),
        13 => (&tables::ESCAPING_BONDS, Score::Dexterity),
        _ => return None,
    })
}

/// A kit bonus object (`CPartKitBonusOb`): skill, level, amount.
pub type KitBonus = (String, i32, i32);

const DEX: usize = 1;

impl Gen<'_> {
    /// 0x4311d9: the first class with +0x27a4.
    fn rogue_class(&self) -> Option<usize> {
        self.classes.iter().position(|k| k.flag(0x27a4))
    }

    /// 0x59a491: the 1-based index of a skill, 0 when absent.
    fn rogue_find(&self, name: &str) -> usize {
        self.rogue
            .iter()
            .position(|(n, _)| n == name)
            .map_or(0, |i| i + 1)
    }

    /// 0x59b019 / 0x446670: the per-level table (part +4 + 4 * level) of
    /// the class whose abilities hold the skill, or the part's default
    /// +0x524 when no such class exists or one of them lacks +0x27a5.
    fn rogue_base(&self, part: &cr_formats::parts::Part) -> i32 {
        let mut idx = None;
        for (g, k) in self.classes.iter().enumerate() {
            if !k.flag(0x27a4) {
                continue;
            }
            for p in &self.class_abilities[g] {
                if p.name() == part.name() && p.n(0x77c) == 6 {
                    if !k.flag(0x27a5) {
                        return part.n(0x524);
                    }
                    idx = Some(g);
                }
            }
        }
        match idx {
            Some(g) => part.n(0x4 + 4 * self.levels[g].clamp(0, 99) as u32),
            None => part.n(0x524),
        }
    }

    /// 0x59aa97(s, L, char, no armor): the points of levels 1..L, the
    /// base, the race (+0x138 by id), the Dexterity table, the traits and
    /// disadvantages (+0x598 / +0x59c by id) and the kit adjustment
    /// 0x442f93 (char +0x2160, empty for a new NPC).
    fn rogue_final(&self, s: usize, level: i32) -> i32 {
        let (name, values) = &self.rogue[s - 1];
        let mut total: i32 = values[..level.clamp(0, 100) as usize].iter().sum();
        let id = skill_id(name, |t| self.t(t));
        if id == 0 {
            return 0;
        }
        let Some(part) = self.d.find_in(CLASS_ABILITIES, name) else {
            return 0;
        };
        total += self.rogue_base(part);
        total += if id <= 8 {
            self.race.thief_adjustments[id - 1]
        } else {
            self.race.unknown_0x158[id - 9]
        };
        if let Some((table, score)) = dexterity_table(id) {
            let v = match (score, self.po) {
                (Score::Dexterity, _) | (_, false) => self.ab[DEX],
                (Score::Aim, true) => self.subs[DEX][0],
                (Score::Balance, true) => self.subs[DEX][1],
            };
            total += tables::at(table, v);
        }
        for (list, kind) in [(&self.traits, TRAITS), (&self.disadvantages, DISADVANTAGES)] {
            for n in list {
                if let Some(p) = self.d.find_in(kind, n)
                    && p.n(0x598) != 0
                {
                    total += p.n(0x59c + 4 * (id as u32 - 1));
                }
            }
        }
        total
    }

    /// 0x59ba5c: the skills (+0x77c == 6) of every class with +0x27a4.
    fn rogue_init(&mut self) {
        let mut out: Vec<(String, [i32; 100])> = Vec::new();
        for (g, k) in self.classes.iter().enumerate() {
            if !k.flag(0x27a4) {
                continue;
            }
            for p in &self.class_abilities[g] {
                if p.n(0x77c) == 6 && !out.iter().any(|(n, _)| n == p.name()) {
                    out.push((p.name().to_string(), [0; 100]));
                }
            }
        }
        self.rogue = out;
    }

    /// 0x442604: the kit's fixed adjustments (+0x174) go to level 1, once.
    fn rogue_kit_adjust(&mut self, g: usize) {
        if self.m1610 {
            return;
        }
        let Some(kit) = self.kit() else { return };
        if kit.rogue_adjust.is_empty() {
            return;
        }
        let level = self.levels[g].clamp(1, 100) as usize;
        for (name, amount) in &kit.rogue_adjust {
            let s = self.rogue_find(name);
            if s != 0 {
                let v = &mut self.rogue[s - 1].1;
                v[0] = v[level - 1].wrapping_add(*amount);
            }
        }
        self.m1610 = true;
    }

    /// 0x4c6c0b: +0x16c bonus objects of +0x170 each, one per skill.
    fn rogue_kit_objects(&mut self) {
        let Some(kit) = self.kit() else { return };
        for i in 0..kit.i(0x16c).max(0) as usize {
            let entry = self.rogue.get(i).map(|(n, _)| (n.clone(), 1, kit.i(0x170)));
            self.rogue_fixed.push(entry);
        }
    }

    /// 0x4c6efc: the kit's random bonus of a level (+0x13c from +0x140 at
    /// level 1, +0x154 from +0x158 later).
    fn rogue_kit_random(&mut self, level: i32) {
        let Some(kit) = self.kit() else { return };
        let (amount, list) = if level == 1 {
            (kit.i(0x13c), kit.sa(0x140))
        } else {
            (kit.i(0x154), kit.sa(0x158))
        };
        if amount == 0 {
            return;
        }
        let at = (level - 1) as usize;
        if self.rogue_random.get(at).is_some_and(Option::is_some) {
            return;
        }
        let list: Vec<String> = list
            .iter()
            .filter(|n| {
                let s = self.rogue_find(n);
                s != 0 && self.rogue_final(s, level) + amount <= CAP
            })
            .cloned()
            .collect();
        if self.rogue_random.len() <= at {
            self.rogue_random.resize(at + 1, None);
        }
        if list.is_empty() {
            self.rogue_random[at] = Some((String::new(), 0, 0));
            return;
        }
        let name = list[self.between(0, list.len() as i32 - 1) as usize].clone();
        let s = self.rogue_find(&name);
        self.rogue[s - 1].1[at] += amount;
        self.rogue_random[at] = Some((name, level, amount));
    }

    /// 0x4c7194: the discretionary points, level by level.
    fn rogue_distribute(&mut self, g: usize) {
        let Some(kit) = self.kit() else { return };
        let k = self.classes[g];
        let listed: Vec<String> = kit.sa(0x128).to_vec();
        let n = self.rogue.len() as i32;
        // 0x4c6e2a
        let kn = if listed.is_empty() {
            n
        } else {
            listed.iter().filter(|x| self.rogue_find(x) != 0).count() as i32
        };
        let (first, later) = (kit.i(0x120), kit.i(0x124));
        let (mut points, mut per, mut rem) = (0i32, 0i32, 0i32);
        for level in 1..=self.levels[g] {
            self.rogue_kit_random(level);
            if level == 1 {
                points = if first != -1 { first } else { k.v(0xb4) };
            } else {
                points += if later != -1 { later } else { k.v(0xb8) };
            }
            if level == 1 && kn != 0 {
                (per, rem) = (points / kn, points % kn);
            } else if n != 0 {
                (per, rem) = (points / n, points % n);
            }
            let at = (level - 1) as usize;
            for s in 1..=n as usize {
                if points == 0 {
                    continue;
                }
                // An empty kit list skips every skill at level 1, so the
                // first level's points wait for the second.
                if kn != 0 && level == 1 && !listed.contains(&self.rogue[s - 1].0) {
                    continue;
                }
                let current = self.rogue[s - 1].1[at];
                let fin = self.rogue_final(s, level);
                let mut give = per;
                if fin + give <= CAP {
                    if fin + rem + give <= CAP {
                        give += rem;
                        rem = 0;
                    } else {
                        rem = rem + fin + give - CAP;
                        give = CAP - fin;
                    }
                } else {
                    rem = rem + fin + give - CAP;
                    give = (CAP - fin).max(0);
                }
                points -= give;
                self.rogue[s - 1].1[at] = give + current;
            }
        }
    }

    /// 0x4c751c
    pub fn st_rogue(&mut self) {
        let Some(g) = self.rogue_class() else {
            return;
        };
        if self.rogue.is_empty() {
            self.rogue_init();
        }
        if !self.m1610 && self.rogue.is_empty() {
            self.rogue_init();
        }
        self.rogue_kit_adjust(g);
        self.rogue_kit_objects();
        self.rogue_distribute(g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_ids_follow_the_keyword_order() {
        let text = |t: Tx| match t {
            t if t == SKILL_IDS[0] => "PICK",
            t if t == SKILL_IDS[2] => "TRAPS",
            t if t == SKILL_IDS[6] => "CLIMB",
            _ => "\u{1}",
        };
        assert_eq!(skill_id("Thief, Pick pockets* (10)", text), 1);
        assert_eq!(skill_id("Thief, Find/remove traps* (10)", text), 3);
        assert_eq!(skill_id("Thief, Climb walls* (5)", text), 7);
        assert_eq!(skill_id("Thief, Backstab (10)", text), 0);
    }
}
