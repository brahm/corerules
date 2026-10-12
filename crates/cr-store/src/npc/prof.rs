//! Proficiencies and languages: nonweapon proficiencies (0x4d4a42),
//! weapon proficiencies (0x4c48a8) and the languages (0x4c3f4d), Core
//! Rules paths.

use cr_formats::parts::Part;

use super::Gen;
use super::data::{LANGUAGES, NONWEAPON, PartExt, RACIAL_ABILITIES, WEAPONS};
use super::program::{COMMON_WEAPON_RULES, EXCLUDED_WEAPON_RULES, Level, NameRule, TIGHT_GROUPS_6};

/// Intelligence (Knowledge) to bonus languages, 0x422376.
const INT_LANGS: [i32; 26] = [
    0, 0, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 3, 3, 4, 4, 5, 6, 7, 8, 9, 10, 11, 12, 15, 20,
];

/// 0x4d534b's proficiencies by class group rank (warrior, rogue,
/// psionicist, priest, wizard), Core Rules.
const GROUP_NWP_CORE: [&[&str]; 5] = [
    &["Blind Fighting", "Running"],
    &["Tumbling", "Tightrope Walking"],
    &["Engineering"],
    &["Religion", "Spellcraft", "Herbalism"],
    &["Spellcraft", "Reading/Writing", "Ancient History"],
];

/// The weapon proficiency stage's state (members of `CNPCGeneration`).
#[derive(Debug, Clone, Default)]
pub struct Weapons {
    pub m7c8: bool,
    pub m7d4: i32,
    pub m78c: usize,
    pub m790: bool,
    pub m7c4: usize,
    pub m794: i32,
    pub m7c0: bool,
    pub m7cc: i32,
    pub m7d0: i32,
    pub chosen: Vec<String>,
    pub spec: Vec<String>,
    pub plain: Vec<String>,
    pub slots: i32,
    pub m2224: i32,
    /// +0x6fc: the lowest base cost of the classes.
    pub m6fc: i32,
    pub d8: Vec<String>,
    pub l4c: Vec<String>,
    pub l6c: Vec<String>,
    pub la8: Vec<String>,
    pub l30: Vec<String>,
    pub lec: Vec<String>,
    /// The last weapon the classification loop looked at (a stale
    /// pointer 0x4d9e36 reads later).
    pub last_weapon: Option<String>,
    pub l724: Vec<String>,
}

/// 0x660e0c: the weapon's name, or its base name (+0x428) for a variant.
pub fn name0(w: &Part) -> &str {
    if w.n(0x424) != 0 {
        w.name()
    } else {
        w.s(0x428)
    }
}

/// 0x6613ec: +0x3e4 in Core Rules when set, else +0x3b4.
fn damage_type(w: &Part, core: bool) -> &str {
    let s = w.s(0x3e4);
    if core && !s.is_empty() { s } else { w.s(0x3b4) }
}

/// 0x661802: the damage type contains P or S.
pub fn pierce_slash(w: &Part, core: bool) -> bool {
    let s = damage_type(w, core);
    s.contains('P') || s.contains('S')
}

impl Gen<'_> {
    // ---- nonweapon proficiencies ----

    fn nwp_known(&self, name: &str) -> Option<usize> {
        self.nwp.iter().position(|n| n.0 == name)
    }

    /// 0x442516 (Skills & Powers only).
    fn nwp_forbidden(&self, p: Option<&Part>) -> bool {
        !self.core
            && p.is_some_and(|p| p.n(0x478) == 0 && p.n(0x480) == 0 && !p.s(0x458).is_empty())
    }

    /// 0x43776b
    pub(super) fn nwp_cost(&mut self, part: Option<&Part>) -> i32 {
        let Some(p) = part else { return 0 };
        let name = p.name();
        let core = self.core;
        let mut cost = if core { p.n(0x47c) } else { p.n(0x478) };
        let kit = self.kit();
        let kf = |o: u32| kit.is_some_and(|k| k.flag(o));
        let kw = kf(0x30f) || self.cflag(0x3044);
        let kr = kf(0x312) || self.cflag(0x3045);
        let kp = kf(0x310) || self.cflag(0x3046);
        let kz = kf(0x311) || self.cflag(0x3047);
        let kpsi = kf(0x313) || self.cflag(0x3048);
        let [fs, fd, fc, fi, fw, fch] =
            [0x3049, 0x304a, 0x304b, 0x304c, 0x304d, 0x304e].map(|o| self.cflag(o));
        let g = |x: i32| self.has_group(x);
        let mut ing = p.n(0x48c) != 0
            || (p.n(0x490) != 0 && (g(2) || kp))
            || (p.n(0x498) != 0 && (g(0) || kw))
            || (p.n(0x49c) != 0 && (g(3) || kz))
            || (p.n(0x494) != 0 && (g(1) || kr))
            || (p.n(0x4a0) != 0 && (g(4) || kpsi));
        if !ing {
            ing = match p.s(0x460) {
                "Strength" => fs,
                "Dexterity" => fd,
                "Constitution" => fc,
                "Wisdom" => fw,
                "Intelligence" => fi,
                "Charisma" => fch,
                _ => false,
            };
        }
        if !ing && kit.is_some_and(|k| k.sa(0x2bc).iter().any(|n| n == name)) {
            ing = true;
        }
        if !ing {
            cost += if core { 1 } else { 2 };
        }
        if self.class_ability_any(|ca| ca.s(0x718) == name) {
            cost = 0;
        }
        if let Some(kit) = kit {
            if cost > 0 && kit.s(0xa4) == name {
                cost = 0;
            }
            if cost > 0 && kit.has_0x2f8 {
                // 0x43776b: the kit's proficiency objects (+0x2f8) are not
                // decoded; ignored.
                self.warn(
                    "Kit proficiency objects (+0x2f8, 0x43776b) are not decoded and were ignored.",
                );
            }
            if cost > 0 && self.nwp_bonus(p) {
                cost = 0;
            }
        }
        cost.max(0)
    }

    /// 0x440ddf: proficiencies the classes and kit grant.
    fn nwp_bonus(&mut self, p: &Part) -> bool {
        let name = p.name();
        for k in self.classes.clone() {
            if k.sa(0x2fa0).iter().any(|n| n == name) {
                return true;
            }
            if k.has_0x3514 || k.has_0x3528 {
                // 0x440ddf: the per-level proficiency lists are not decoded.
                self.warn("Class per-level proficiency lists (+0x3514/+0x3528, 0x440ddf) are not decoded and were ignored.");
            }
        }
        self.kit().is_some_and(|k| k.s(0xa4) == name)
    }

    fn nwp_add(&mut self, name: &str, cost: i32) {
        self.nwp.push((name.to_owned(), cost));
    }

    /// 0x4d4e81
    fn nwp_take_free(&mut self, name: &str, free: bool) -> i32 {
        let Some(p) = self.d.find(name) else {
            return -1;
        };
        if self.nwp_forbidden(Some(p)) {
            return -1;
        }
        let known = self.nwp_known(name).is_some();
        if free && known && p.n(0x75c) != 0 {
            return -2;
        }
        if !known {
            let cost = if free { 0 } else { self.nwp_cost(Some(p)) };
            self.nwp_add(name, cost);
            return cost;
        }
        -1
    }

    /// 0x4d5c98: the language pool (0 in Core Rules).
    fn nwp_from_int_pool(&mut self, name: &str) -> bool {
        if self.lang_pool < 1 {
            return false;
        }
        if let Some(p) = self.d.find_in(NONWEAPON, name)
            && (p.s(0x458).contains("Intelligence") || p.s(0x45c).contains("Intelligence"))
            && p.n(0x478) <= self.lang_pool
        {
            self.lang_pool -= p.n(0x478);
            return true;
        }
        false
    }

    /// 0x4d5036: pays from the slots.
    fn nwp_learn(&mut self, name: &str) -> i32 {
        if self.nwp_known(name).is_some() {
            return -1;
        }
        let Some(p) = self.d.find(name) else {
            return -1;
        };
        if self.nwp_forbidden(Some(p)) {
            return -1;
        }
        let cost = self.nwp_cost(Some(p));
        if self.slots < cost {
            return -1;
        }
        self.nwp_add(name, cost);
        if !self.nwp_from_int_pool(name) {
            self.slots -= cost;
        }
        cost
    }

    pub(super) fn int_languages(&self) -> i32 {
        INT_LANGS[self.knowledge().clamp(0, 25) as usize]
    }

    /// 0x4382d2, Core Rules branch.
    fn nwp_slots_core(&self) -> i32 {
        let kit = self.kit();
        let k318 = kit.map_or(0, |k| k.i(0x318));
        let from_kit = kit.is_some() && k318 >= 1;
        let (mut init, mut per, mut lvl, mut inc, mut extra) = (0, 100_000, 0, 0, 0);
        let dual = self.dual();
        for (k, &level) in self.classes.iter().zip(&self.levels) {
            if !dual {
                init = init.max(k.v(0xc0));
                per = per.min(k.v(0xc4));
                lvl = lvl.max(level);
                inc = inc.max(k.c8);
            } else {
                init += k.v(0xc0);
                extra += level.checked_div(k.v(0xc4)).unwrap_or(0) * inc;
            }
        }
        if from_kit {
            init = k318;
        }
        if kit.is_some_and(|k| k.flag(0x30c)) {
            init += 1;
            per = 3;
            extra = 0;
            if lvl > 6 {
                lvl = 0;
            }
        }
        if per < 1 {
            per = 1;
        }
        extra + init + lvl.div_euclid(per) * inc + self.int_languages()
    }

    /// 0x4d4a42 (Core Rules).
    pub fn st_nwp(&mut self) {
        self.m784 = 0;
        let s34 = self.nwp_slots_core();
        if self.kit().is_some_and(|k| k.flag(0x44e)) {
            self.m784 = s34 / 2;
        }
        self.slots = s34 - self.m784;
        self.lang_pool = 0;
        self.nwp_class_free();
        self.nwp_auto_zero();
        self.nwp_class_lists();
        self.nwp_kit_lists();
        let mut spent = self.nwp_bonus_lists();
        if !self.languages.iter().any(|l| l == "Common") {
            let r = self.nwp_learn("Modern Languages");
            if r > -1 {
                spent += r;
            }
        }
        spent += self.nwp_kit_random();
        if spent < s34 {
            self.nwp_group_defaults();
        }
        self.nwp_random();
    }

    /// 0x4420f1
    fn nwp_class_free(&mut self) {
        let d = self.d;
        for k in self.classes.clone() {
            for n in k.sa(0x2fa0) {
                let p = d.find_in(NONWEAPON, n);
                if p.is_some() && !self.nwp_forbidden(p) && self.nwp_known(n).is_none() {
                    let cost = self.nwp_cost(p);
                    if cost < 1 {
                        self.nwp_add(n, cost);
                    }
                }
            }
            if k.has_0x3514 {
                self.warn("Class per-level proficiency lists (+0x3514, 0x4420f1) are not decoded and were ignored.");
            }
        }
        if let Some(kit) = self.kit() {
            let s = kit.s(0xa4);
            if !s.is_empty() {
                let p = d.find_in(NONWEAPON, s);
                let cost = self.nwp_cost(p);
                if !self.nwp_forbidden(p) {
                    self.nwp_add(s, cost);
                }
            }
        }
    }

    /// 0x4d9262: every proficiency that costs nothing.
    fn nwp_auto_zero(&mut self) {
        let d = self.d;
        for p in &d.parts[NONWEAPON] {
            if self.core && p.n(0x75c) != 0 && self.classes.iter().any(|k| k.has_0x3514) {
                // 0x4d9527
                self.warn("Language proficiency copies from class level lists (0x4d9527) are not decoded.");
            }
            if self.nwp_cost(Some(p)) == 0
                && self.nwp_known(p.name()).is_none()
                && !(!self.core && self.nwp_forbidden(Some(p)))
            {
                self.nwp_add(p.name(), 0);
            }
        }
    }

    /// 0x4d3c02
    fn nwp_class_lists(&mut self) {
        for k in self.classes.clone() {
            for n in k.sa(0x2fa0) {
                self.nwp_take_free(n, true);
            }
        }
        if self.classes.iter().any(|k| k.has_0x3514) {
            self.warn("Class per-level proficiency lists (+0x3514, 0x4d3c02) are not decoded and were ignored.");
        }
    }

    /// 0x4d3e6c: the kit's +0xa4, then each option group of +0x330.
    fn nwp_kit_lists(&mut self) {
        let Some(kit) = self.kit() else { return };
        let s = kit.s(0xa4);
        if !s.is_empty() {
            self.nwp_take_free(s, true);
        }
        for o in &kit.nwp_options {
            let opts = &o.names;
            let n = opts.len() as i32;
            if n == 1 {
                self.nwp_take_free(&opts[0], true);
            } else if n > 1 {
                let want = o.count.min(n);
                if want == n {
                    for name in opts {
                        if self.nwp_take_free(name, true) == -2 {
                            // 0x4d3e6c: the language choice is not decoded.
                            self.warn("A kit's language proficiency choice (0x4d3e6c) is not decoded and was skipped.");
                        }
                    }
                } else {
                    let (mut got, mut tries) = (0, 0);
                    while got < want && tries < n + 25 {
                        let name = &opts[self.between(0, n - 1) as usize];
                        if self.nwp_take_free(name, true) > -1 {
                            got += 1;
                        }
                        tries += 1;
                    }
                }
            }
        }
    }

    /// 0x4d45df: class +0x2fcc, kit +0x280 and the kit's level
    /// proficiency +0x31c.
    fn nwp_bonus_lists(&mut self) -> i32 {
        let mut spent = 0;
        for k in self.classes.clone() {
            for n in k.sa(0x2fcc) {
                let r = self.nwp_learn(n);
                if r > -1 {
                    spent += r;
                }
            }
        }
        let Some(kit) = self.kit() else { return spent };
        for n in kit.sa(0x280) {
            let r = self.nwp_learn(n);
            if r > -1 {
                spent += r;
            }
        }
        let s = kit.s(0x31c);
        if kit.i(0x320) <= self.m738 && !s.is_empty() {
            let r = self.nwp_take_free(s, false);
            if r > -1 {
                if !self.nwp_from_int_pool(s) {
                    self.slots -= r;
                }
                spent += r;
            }
        }
        spent
    }

    /// 0x4d43cc: the kit's recommended proficiencies (+0x258).
    fn nwp_kit_random(&mut self) -> i32 {
        if self.slots < 1 {
            return 0;
        }
        let list = self.kit().map_or(&[][..], |k| k.sa(0x258));
        // Rolled and only used as a cap.
        self.between(1, self.slots);
        let n = list.len() as i32;
        let (mut tries, mut total) = (0, 0);
        if n > 0 {
            while self.slots > 0 && tries < n + 50 {
                tries += 1;
                let name = &list[self.between(0, n - 1) as usize];
                let p = self.d.find(name);
                if p.is_some() && !self.nwp_forbidden(p) && self.nwp_known(name).is_none() {
                    let r = self.nwp_learn(name);
                    if r != -1 {
                        total += r;
                    }
                }
            }
        }
        total
    }

    /// 0x4d534b: the proficiencies of the highest-ranked class group
    /// (warrior < rogue < psionicist < priest < wizard).
    fn nwp_group_defaults(&mut self) {
        let mut k = 0;
        for c in &self.classes {
            let rank = match c.group {
                0 => 0,
                1 => 1,
                2 => 3,
                3 => 4,
                4 => 2,
                _ => -1,
            };
            if rank > k {
                k = rank;
            }
        }
        for n in GROUP_NWP_CORE[k as usize] {
            self.nwp_learn(n);
        }
    }

    /// 0x4d6110: any proficiency of `Parts.dat` the kit does not bar.
    fn nwp_random(&mut self) {
        let d = self.d;
        let barred = self.kit().map_or(&[][..], |k| k.sa(0x26c));
        let list = &d.parts[NONWEAPON][..d.db_count[NONWEAPON]];
        let n = list.len() as i32;
        let mut tries = 0;
        if n > 0 {
            while self.slots > 0 && tries < 50 {
                tries += 1;
                let p = &list[self.between(0, n - 1) as usize];
                let name = p.name();
                if barred.iter().any(|b| b == name) {
                    continue;
                }
                if !self.nwp_forbidden(Some(p)) && self.nwp_known(name).is_none() {
                    self.nwp_learn(name);
                }
            }
        }
    }

    // ---- weapon proficiencies ----

    /// Runs the straight-line name tests of 0x4ce3d6 / 0x4d6dc6.
    fn run_name_rules(&self, rules: &[NameRule], name: &str, hit: i32, end: i32) -> Option<i32> {
        let mut i = 0;
        while i < rules.len() {
            match rules[i] {
                NameRule::Stop => return None,
                NameRule::Eq(s) => {
                    if name == s {
                        return Some(hit);
                    }
                }
                NameRule::Find(s) => {
                    if name.contains(s) {
                        return Some(hit);
                    }
                }
                NameRule::Race {
                    parent,
                    equal,
                    race,
                    find,
                } => {
                    let v = if parent {
                        self.race.unknown_0x188.as_str()
                    } else {
                        self.race.name.as_str()
                    };
                    if (v == race) == equal && name.contains(find) {
                        return Some(hit);
                    }
                }
                NameRule::Gate { when, to } => {
                    let ok = match when {
                        Level::Above(k) => self.m738 > k,
                        Level::Below(k) => self.m738 < k,
                    };
                    if ok {
                        match to {
                            Some(t) => {
                                i = t;
                                continue;
                            }
                            None => return Some(end),
                        }
                    }
                }
            }
            i += 1;
        }
        Some(end)
    }

    /// 0x4ce3d6: true unless the upper-cased name hits one of the
    /// program's exclusions; user weapons always pass.
    pub(super) fn common_weapon(&self, name: &str) -> bool {
        if self.d.user_weapon(name) {
            return true;
        }
        self.run_name_rules(COMMON_WEAPON_RULES, &name.to_uppercase(), 0, 1) != Some(0)
    }

    /// 0x4d6dc6: the weapon is excluded.
    pub(super) fn excluded_weapon(&self, w: &Part) -> bool {
        let name = w.name();
        if let Some(r) = self.run_name_rules(EXCLUDED_WEAPON_RULES, name, 1, 0) {
            return r != 0;
        }
        let Some(kit) = self.kit() else { return true };
        if !kit.sa(0x384).iter().any(|n| n == name) && self.m738 <= 1 && kit.flag(0x76) {
            return true;
        }
        if kit.flag(0x79) && w.n(0x3c0) != 0 {
            return true;
        }
        if self.m6f[1] && w.n(0x3b8) == 0 {
            return true;
        }
        if self.m6f[3] && w.n(0x448) == 0 {
            return true;
        }
        false
    }

    /// 0x669550(class, name)
    fn class_allows(&mut self, k: &super::data::Klass, name: &str) -> bool {
        if k.s3000.as_deref() == Some("Any") {
            return true;
        }
        let Some(w) = self.d.find_in(WEAPONS, name) else {
            return false;
        };
        if k.sa(0x3004).iter().any(|n| n == name0(w)) {
            return true;
        }
        if k.flag(0x27a0) && pierce_slash(w, self.core) {
            return false;
        }
        if k.s3000.as_deref() != Some("Only from list below") {
            // 0x669550: the damage-type list is not decoded; refused.
            self.warn("A class's weapon damage-type list (0x669550) is not decoded; its weapons were refused.");
        }
        false
    }

    /// 0x440996
    pub(super) fn any_class_allows(&mut self, name: &str) -> bool {
        for k in self.classes.clone() {
            if self.class_allows(k, name) {
                return true;
            }
        }
        false
    }

    /// Shared core of 0x4400fa and 0x43d7bc for a part of type `t`: no
    /// class may use it. `armor` enables the +0x224 armor test.
    pub(super) fn barred(&self, p: &Part, t: i32, armor: bool) -> bool {
        let barred = p.sa(0x82c);
        let a279d = self.cflag(0x279d);
        let (mut ok_some, mut f48ac) = (false, false);
        for k in &self.classes {
            let mut found = false;
            if armor {
                if !f48ac {
                    f48ac = k.flag(0x48ac);
                }
                if p.n(0x224) != 0
                    && !f48ac
                    && k.sa(0x3018)
                        .iter()
                        .any(|s| !s.is_empty() && barred.contains(s))
                {
                    found = true;
                }
            }
            if barred.contains(&k.name) {
                found = true;
            }
            if !found {
                ok_some = true;
                break;
            }
        }
        if armor && f48ac {
            ok_some = true;
        }
        let a38 = a279d && p.n(0x4c0) == 0;
        let (mut f20, mut f44) = (false, false);
        if t == 0 || t == 1 {
            let (o20, o44) = if t == 0 {
                (0x81c, 0x820)
            } else {
                (0x824, 0x828)
            };
            for ca in self.class_abilities.iter().flatten() {
                if ca.n(o20) != 0 && p.n(0x4c8) == 0 {
                    f20 = true;
                }
                if ca.n(o44) != 0 && p.n(0x4cc) == 0 {
                    f44 = true;
                }
            }
        }
        if a279d && self.classes.len() == 1 && p.n(0x4c0) != 0 {
            ok_some = false;
        }
        if self.has_group(4) {
            if self.core && p.n(0x4c8) == 0 {
                f20 = true;
            }
            if self.classes.len() == 1 && !f44 && !f20 {
                ok_some = false;
            }
        }
        !(f44 || f20 || a38 || ok_some)
    }

    /// 0x4400fa(char, w): 0 = usable.
    fn usable_0x4400fa(&self, w: &Part) -> i32 {
        if self.barred(w, 0, false) {
            return 0x1e;
        }
        if self.cflag(0x27a0) && pierce_slash(w, self.core) {
            return 0x1e;
        }
        if w.s(0x1e0) == self.race.name {
            return 0x1f;
        }
        if self
            .kit()
            .is_some_and(|k| k.sa(0xbc).iter().any(|n| n == name0(w)))
        {
            return 0x29;
        }
        0
    }

    /// 0x44681a: the kit grants the weapon (+0x484).
    pub(super) fn kit_grants(&mut self, n: &str) -> bool {
        let Some(kit) = self.kit() else { return false };
        if kit.has_0x470 {
            // 0x44681a: the kit's bonus objects (+0x470) are not decoded.
            self.warn("Kit weapon bonus objects (+0x470) are not decoded and were ignored.");
        }
        kit.sa(0x484).iter().any(|x| x == n)
    }

    /// 0x43d7bc(char, w, 0x377) for a weapon: 0 = usable.
    fn usable_0x43d7bc(&mut self, w: &Part) -> i32 {
        if w.s(0x1e0) == self.race.name {
            return 0x1f;
        }
        let n = name0(w);
        // (0x468356 would test a name shared with another part type; every
        // weapon examined here has +0x424.)
        if self.kit_grants(n) {
            return 0;
        }
        if !self.any_class_allows(w.name()) {
            return 0x1e;
        }
        let kit = self.kit();
        if kit.is_some_and(|k| k.sa(0x620).iter().any(|x| x == n)) {
            return 0x29;
        }
        // (0x4321ac, the weapon list, is still empty here.)
        if self.barred(w, 0, true) || (self.cflag(0x27a0) && pierce_slash(w, self.core)) {
            return 0x1e;
        }
        if kit.is_some_and(|k| k.sa(0xbc).iter().any(|x| x == n)) {
            return 0x29;
        }
        0
    }

    /// 0x4d0217 (Core Rules: 0, 1 or 999).
    pub(super) fn wp_cost(&mut self, name: &str) -> i32 {
        let d = self.d;
        for n in &self.racial_abilities {
            let Some(p) = d.find_in(RACIAL_ABILITIES, n) else {
                continue;
            };
            if name == p.s(0x574) && self.m738 >= p.n(0x53c) {
                return 0;
            }
            if name == p.s(0x578) && self.m738 >= p.n(0x540) {
                return 0;
            }
        }
        let mut cost = self.wp.m6fc;
        let Some(w) = d.find_in(WEAPONS, name) else {
            return 999;
        };
        let Some(kit) = self.kit() else { return 999 };
        if kit.has_0x470 {
            self.warn("Kit weapon bonus objects (+0x470) are not decoded and were ignored.");
        }
        if kit.flag(0x444) || kit.sa(0x384).iter().any(|n| n == name) {
            cost -= 1;
        }
        let barred = w.sa(0x82c);
        let some = self.classes.iter().any(|k| !barred.contains(&k.name));
        let f24 = self.m6f[4] && w.n(0x4c8) == 0;
        let f4c = self.m6f[4] && w.n(0x4cc) == 0;
        if !(some || f24 || f4c) {
            return 999;
        }
        if self.core && cost > 0 {
            cost = 1;
        }
        cost.max(0)
    }

    /// 0x43a17e
    fn can_specialize(&self) -> bool {
        if self.kit().is_some_and(|k| k.flag(0x518)) {
            return true;
        }
        let top = self.char_level();
        if self.cflag(0x2792) {
            return self.classes.len() == 1 || top > 1;
        }
        if self.has_group(0) {
            return top > 2;
        }
        if self.has_group(2) {
            return top > 4;
        }
        if self.has_group(1) || self.has_group(4) {
            return top > 5;
        }
        if self.has_group(3) {
            return top > 6;
        }
        false
    }

    /// 0x4d98fb: specialization cost, Core Rules.
    fn spec_cost_core(&self, name: &str) -> i32 {
        if name.is_empty() {
            return 1;
        }
        if let Some(kit) = self.kit() {
            if kit.sa(0x53c).iter().any(|n| n == name) {
                return 0;
            }
            if kit.flag(0x5d4) {
                return 1;
            }
        }
        if let Some(w) = self.d.find_in(WEAPONS, name)
            && w.sa(0x1cc)
                .iter()
                .any(|g| TIGHT_GROUPS_6.contains(&g.as_str()))
        {
            return 2;
        }
        1
    }

    /// 0x4397d1 with an empty weapon list.
    fn wp_slots_core(&self) -> i32 {
        let kit = self.kit();
        let ki = |o: u32, dflt: i32| kit.map_or(dflt, |k| k.i_or(o, dflt));
        let b44c = kit.is_some_and(|k| k.flag(0x44c));
        let i458 = ki(0x458, -1);
        let i45c = ki(0x45c, -1);
        let (mut init, mut per, mut lvl, mut cnt, mut div) = (0i32, i32::MAX, 0i32, 0i32, 0i32);
        let sumd = self.cflag(0x486d);
        let dual = self.dual();
        for (k, &l) in self.classes.iter().zip(&self.levels) {
            if dual && k.v(0xd8) != 0 {
                init += k.v(0xd4) + l.div_euclid(k.v(0xd8));
            } else if !sumd {
                init = init.max(k.v(0xd4));
                per = per.min(k.v(0xd8));
                lvl = lvl.max(l);
            } else {
                init += k.v(0xd4);
                per = per.wrapping_add(k.v(0xd8));
                lvl = lvl.max(l);
            }
            if b44c {
                cnt += i32::from(l > 0) + i32::from(l > 2) + i32::from(l > 5);
            }
            if i458 > 0 && l > i458 {
                div = l / i458;
            }
        }
        let i454 = ki(0x454, -1);
        if i454 > -1 {
            init = i454;
        }
        let n = self.classes.len() as i32;
        if !dual && n > 1 {
            cnt = cnt.div_euclid(n);
            div = div.div_euclid(n);
        }
        let mut s = if b44c {
            cnt
        } else if i458 < 1 {
            if per != 0 {
                init + lvl.div_euclid(per)
            } else {
                0
            }
        } else {
            init + div
        };
        if i45c != -1 && self.char_level() < i45c {
            s = init;
        }
        s += ki(0x448, 0);
        if ki(0x468, 0) != 0 && ki(0x46c, 0) <= self.char_level() {
            s += ki(0x468, 0);
        }
        s
    }

    /// 0x4c48a8 (Core Rules).
    pub fn st_wp(&mut self) {
        let d = self.d;
        let kit = self.kit();
        let ksa = |o: u32| kit.map_or(&[][..], |k| k.sa(o));
        let kflag = |o: u32| kit.is_some_and(|k| k.flag(o));
        self.wp = super::prof::Weapons {
            m7c8: !ksa(0x410).is_empty() || !ksa(0x4f4).is_empty(),
            m78c: ksa(0x398).len(),
            m790: kflag(0x465),
            m7c4: ksa(0x53c).len(),
            ..Default::default()
        };
        self.wp.slots = self.wp_slots_core() + self.m784;
        self.wp.m6fc = self.classes.iter().map(|k| k.v(0xd0)).min().unwrap_or(0);
        let (sa4b8, sa3fc, sa4e0, sa3e8) = (ksa(0x4b8), ksa(0x3fc), ksa(0x4e0), ksa(0x3e8));
        let has = |l: &[String], n: &str| l.iter().any(|x| x == n);
        // The kit's option groups (+0x568).
        let mut l4c: Vec<String> = Vec::new();
        for o in kit.map_or(&[][..], |k| &k.wp_options[..]) {
            let names = &o.names;
            let n = names.len() as i32;
            if n < 2 {
                l4c.push(kit_option_label(o.count, names));
                continue;
            }
            let want = n.min(o.count);
            let (mut got, mut tries) = (0, 0);
            while got < want && tries < (n + 40).max(n) {
                let nm = &names[self.between(0, n - 1) as usize];
                if !l4c.contains(nm) {
                    got += 1;
                    l4c.push(nm.clone());
                }
                tries += 1;
            }
        }
        let (mut ld8, mut l6c, mut la8, mut l30, mut lec) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let m738 = self.m738;
        // The weapons (0x4725b4): type-0 parts with +0x424.
        let weapons: Vec<&Part> = d.parts[WEAPONS]
            .iter()
            .filter(|p| p.n(0x424) != 0)
            .collect();
        for &w in &weapons {
            if w.n(0x420) != 0 {
                continue;
            }
            let name = w.name();
            if self.wp.chosen.iter().any(|c| c == name) {
                continue;
            }
            let cost = self.wp_cost(name);
            if cost == 0 {
                ld8.push(name.to_owned());
            }
            let mut flag = has(ksa(0x384), name);
            if has(sa3e8, name) {
                flag = true;
            }
            if !sa4b8.is_empty() {
                if m738 <= 1 {
                    if !has(sa4b8, name) {
                        continue;
                    }
                    flag = true;
                } else if has(sa4b8, name) {
                    flag = true;
                }
            }
            if !sa3fc.is_empty() {
                if !has(sa3fc, name) {
                    // 0x4d7d99 is not decoded: the weapon is skipped.
                    self.warn("A kit's weapon list (+0x3fc, 0x4d7d99) is not decoded; weapons outside it were skipped.");
                    continue;
                }
                flag = true;
            }
            if !sa4e0.is_empty() {
                if !has(sa4e0, name) {
                    continue;
                }
                flag = true;
            }
            if m738 <= 1
                && kflag(0x508)
                && (w.n(0x3bc) != 0 || w.n(0x410) != 0 || w.n(0x414) != 0 || w.n(0x418) != 0)
            {
                continue;
            }
            if !self.any_class_allows(name) {
                continue;
            }
            if self.excluded_weapon(w) {
                continue;
            }
            if self.usable_0x43d7bc(w) != 0 {
                continue;
            }
            if self.usable_0x4400fa(w) != 0 {
                continue;
            }
            if m738 == 1 && has(ksa(0xbc), name) {
                continue;
            }
            if cost >= 999 {
                continue;
            }
            if kflag(0x444) && self.common_weapon(name) {
                flag = true;
            }
            if l4c.iter().any(|x| x == name) {
                lec.push(name.to_owned());
            } else if cost == 0 {
                l6c.push(name.to_owned());
            } else if flag {
                la8.push(name.to_owned());
            } else {
                if !self.common_weapon(name) {
                    continue;
                }
                if has(ksa(0xd0), name) {
                    continue;
                }
                l30.push(name.to_owned());
            }
        }
        self.wp.last_weapon = weapons.last().map(|w| w.name().to_owned());
        self.wp_pick(&ld8, true);
        self.wp_pick(&l4c, true);
        self.wp_take_all(&l4c);
        self.wp_take_all(&ld8);
        if self.wp.m7c4 > 0 {
            let list = ksa(0x53c);
            let nm = list[self.between(0, list.len() as i32 - 1) as usize].clone();
            self.wp_pick(&[nm], true);
        }
        self.wp_pick(&la8, false);
        if !kflag(0x76) || m738 > 1 {
            self.wp_pick(&l30, false);
        }
        if self.wp.m790 {
            self.wp.chosen.push("Club".into());
        }
        self.wp.d8 = ld8;
        self.wp.l4c = l4c;
        self.wp.l6c = l6c;
        self.wp.la8 = la8;
        self.wp.l30 = l30;
        self.wp.lec = lec;
        self.weapon_list = self.wp.chosen.clone();
        self.weapon_spec = self.wp.spec.clone();
    }

    /// 0x4d9c23: every name not chosen yet, while its cost fits.
    fn wp_take_all(&mut self, list: &[String]) {
        if list.is_empty() {
            return;
        }
        self.wp.m7c8 = false;
        for name in list {
            if self.wp.chosen.contains(name) {
                continue;
            }
            let cost = self.wp_cost(name);
            let Some(w) = self.d.find_in(WEAPONS, name) else {
                continue;
            };
            if cost <= self.wp.slots {
                self.wp.chosen.push(name.clone());
                self.wp.slots -= cost;
                if w.n(0x3bc) != 0 {
                    self.wp.m7cc += 1;
                }
                if w.n(0x3b8) != 0 {
                    self.wp.m7d0 += 1;
                }
                if self.wp.m790 {
                    self.warn("The club kit path (0x4d8040) is not decoded and was skipped.");
                }
            }
        }
    }

    /// 0x4d8278(list, p1): up to 40 tries while slots remain.
    fn wp_pick(&mut self, list: &[String], p1: bool) {
        let n = list.len() as i32;
        if n == 0 {
            return;
        }
        let kit = self.kit();
        let ksa = |o: u32| kit.map_or(&[][..], |k| k.sa(o));
        let kflag = |o: u32| kit.is_some_and(|k| k.flag(o));
        let has = |l: &[String], n: &str| l.iter().any(|x| x == n);
        let l50 = ksa(0x410).len();
        let lac = ksa(0x4f4).len();
        let b438 = kflag(0x438);
        let sa424 = ksa(0x424);
        let i43c = (kit.map_or(0, |k| k.i(0x43c))).min(sa424.len() as i32);
        let la4 = ksa(0x3e8).len();
        let (b538, b521, b520) = (kflag(0x538), kflag(0x521), kflag(0x520));
        let (mut cnt3bc, mut cnt3b8) = (0, 0);
        self.wp.m2224 = 0;
        let spec_ok = self.can_specialize()
            && self.classes[0].flag(0x2797)
            && (self.classes.len() == 1 || kit.is_some_and(|k| k.i(0x51c) != 0));
        let mut tries = 0;
        while self.wp.slots >= 1 && tries < 40 {
            tries += 1;
            let mut name = list[(self.between(1, n) - 1) as usize].clone();
            if self.wp.m790 {
                self.warn("The club kit path (0x4d8040) is not decoded and was skipped.");
            }
            if self.wp.m7c8 {
                if lac > 0 {
                    let l = ksa(0x4f4);
                    name = l[self.between(0, l.len() as i32 - 1) as usize].clone();
                } else if l50 > 0 {
                    let l = ksa(0x410);
                    name = l[self.between(0, l.len() as i32 - 1) as usize].clone();
                }
            }
            if self.wp.chosen.contains(&name) {
                continue;
            }
            if self.wp.m78c > 0 {
                self.warn("A kit's sword list (0x4d6c6c) is not decoded and was ignored.");
            }
            let Some(w) = self.d.find_in(WEAPONS, &name) else {
                if n < 2 {
                    tries = 40;
                }
                continue;
            };
            if b438 && w.n(0x3bc) == 0 && cnt3b8 > cnt3bc {
                continue;
            }
            let cost = self.wp_cost(&name);
            if cost < 0 {
                continue;
            }
            if cost > self.wp.slots {
                if n < 2 {
                    tries = 40;
                }
                continue;
            }
            if i43c != 0 {
                if !has(sa424, &name) {
                    if self.wp.m7d4 < i43c {
                        continue;
                    }
                } else {
                    self.wp.m7d4 += 1;
                }
            }
            if lac > 0 || l50 > 0 {
                if (lac > 0 && has(ksa(0x4f4), &name)) || (l50 > 0 && has(ksa(0x410), &name)) {
                    self.wp.m794 += 1;
                    self.wp.m7c8 = false;
                } else {
                    self.wp.m7c8 = !((lac > 0 && lac as i32 <= self.wp.m794)
                        || (l50 > 0 && l50 as i32 <= self.wp.m794));
                }
            }
            self.wp.chosen.push(name.clone());
            if kflag(0x4ac) {
                self.warn(
                    "A kit's weapon of choice (+0x4ac, +0x788) is not decoded and was ignored.",
                );
            }
            if w.n(0x3bc) != 0 {
                cnt3bc += 1;
            }
            if w.n(0x3b8) != 0 {
                cnt3b8 += 1;
            }
            self.wp.slots -= cost;
            self.wp.m2224 += cost;
            if spec_ok && !self.wp.m7c0 {
                let mut ok = true;
                if la4 > 0 {
                    if !has(ksa(0x3e8), &name) {
                        ok = false;
                    }
                    if p1 && !self.common_weapon(&name) {
                        ok = false;
                    }
                }
                if b538 && w.n(0x3b8) != 0 {
                    ok = false;
                }
                if b521 && w.n(0x3bc) == 0 {
                    ok = false;
                }
                if b520 && w.n(0x3b8) == 0 {
                    ok = false;
                }
                let sc = self.spec_cost_core(&name);
                if !ok || self.wp.slots < sc || sc == -1 {
                    self.wp.plain.push(name);
                } else {
                    self.wp.m7c0 = true;
                    self.wp.spec.push(name);
                    if self.wp.m7c4 > 0 && sc == 0 {
                        self.wp.m7c4 = 0;
                    }
                    self.wp.slots -= sc;
                    self.wp.m2224 += sc;
                }
            } else {
                self.wp.plain.push(name);
            }
            if n < 2 {
                tries = 40;
            }
        }
    }

    // ---- languages ----

    /// 0x4c3f4d: the racial languages, then 0x4c44ba.
    pub fn st_lang2(&mut self) {
        let sub = self.subrace;
        let sn = sub.name.as_str();
        self.languages.clear();
        // The +0x768 / +0x754 lists are filled only by 0x4d5a07, which
        // returns at once in Core Rules: a Stout or Hairfoot halfling gets
        // no racial language here.
        if !(sn.contains("Stout halfling") || sn.contains("Hairfoot halfling")) {
            self.languages.extend(sub.languages.iter().cloned());
        }
        if matches!(
            sn,
            "Human" | "Half-orc" | "Half-ogre" | "Half-elf" | "Standard half-elf"
        ) && !self
            .languages
            .iter()
            .any(|l| l == "Human, Regional dialect")
        {
            self.languages.push("Human, Regional dialect".into());
        }
        self.lang_nwp();
    }

    /// 0x4c44ba: the languages of each language-teaching proficiency.
    fn lang_nwp(&mut self) {
        let d = self.d;
        let per = if self.kit().is_some_and(|k| k.flag(0x5d6)) {
            2
        } else {
            1
        };
        let pool: Vec<&str> = d.parts[LANGUAGES].iter().map(|p| p.name()).collect();
        if pool.is_empty() {
            return;
        }
        let mut common_done = false;
        for i in 0..self.nwp.len() {
            let name = self.nwp[i].0.clone();
            let Some(p) = d.find_in(NONWEAPON, &name) else {
                continue;
            };
            if p.n(0x75c) == 0 {
                continue;
            }
            let mut list = self.nwp_langs.remove(&i).unwrap_or_default();
            for _ in 0..per {
                let mut got = None;
                if !common_done && name == "Modern Languages" && !list.iter().any(|l| l == "Common")
                {
                    got = Some("Common".to_owned());
                    common_done = true;
                }
                while got.is_none() {
                    if pool.iter().all(|p| list.iter().any(|l| l == p)) {
                        break;
                    }
                    let k = self.between(0, pool.len() as i32 - 1) as usize;
                    if !list.iter().any(|l| l == pool[k]) {
                        got = Some(pool[k].to_owned());
                    }
                }
                if let Some(g) = got {
                    list.push(g);
                }
            }
            self.nwp_langs.insert(i, list);
        }
    }
}

/// 0x4b2aab: the single name, or "Select N from a, b and c".
fn kit_option_label(count: i32, names: &[String]) -> String {
    if count == 1 && names.len() == 1 {
        return names[0].clone();
    }
    let mut s = format!("Select {count} from ");
    for (i, n) in names.iter().enumerate() {
        if i > 0 {
            s += ", ";
        }
        if i + 1 == names.len() && i > 0 {
            s.truncate(s.len() - 2);
            s += " and ";
        }
        s += n;
    }
    s
}
