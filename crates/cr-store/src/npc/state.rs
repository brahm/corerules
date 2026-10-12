//! The generator's state and its identity stages (0x4bf418): document
//! constructor, race, name, abilities, alignment, kit, social class,
//! background, height/weight/age, starting money and hit points.

use std::collections::HashMap;

use cr_formats::parts::Part;
use cr_formats::race::Race;

use super::Session;
use super::data::{CLASS_ABILITIES, Data, KITS, Kit, Klass, PartExt, RACIAL_ABILITIES};
use super::equip::Equipment;
use super::prof::Weapons;
use super::program::BACKGROUNDS;
use crate::treasure::Rand;
use crate::{Error, Result};

/// Str, Dex, Con, Int, Wis, Cha.
pub const STR: usize = 0;
pub const CON: usize = 2;
pub const INT: usize = 3;
pub const WIS: usize = 4;

/// An inventory item under construction (`CInventoryItem`).
#[derive(Debug, Clone, PartialEq)]
pub struct It {
    pub name: String,
    pub n: i32,
    pub bundle: Option<Vec<It>>,
    pub book: Option<Book>,
}

impl It {
    pub fn new(name: impl Into<String>, n: i32) -> Self {
        Self {
            name: name.into(),
            n,
            bundle: None,
            book: None,
        }
    }
}

/// A spellbook's pages (the item's extra data).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Book {
    pub capacity: i32,
    pub used: i32,
    pub spells: Vec<String>,
    pub pages: Vec<i32>,
    pub wizard: Vec<bool>,
}

/// Bytes of a name as the program compares them (strcmp on ANSI).
fn ansi(s: &str) -> impl Iterator<Item = u32> + '_ {
    s.chars().map(|c| c as u32)
}

/// 0x4aa8e3: an unbundled item merges with an equal unbundled one;
/// otherwise items go before the first one whose name is greater.
pub fn inv_add(list: &mut Vec<It>, it: It, merge: bool) {
    if merge
        && it.bundle.is_none()
        && let Some(x) = list
            .iter_mut()
            .find(|x| x.name == it.name && x.bundle.is_none())
    {
        x.n = x.n.wrapping_add(it.n);
        return;
    }
    let at = list
        .iter()
        .position(|x| ansi(&x.name).gt(ansi(&it.name)))
        .unwrap_or(list.len());
    list.insert(at, it);
}

/// Psionic disciplines and powers.
#[derive(Debug, Clone, Default)]
pub struct Psionics {
    pub disciplines: Vec<String>,
    pub sciences: Vec<String>,
    pub devotions: Vec<String>,
    pub primary: String,
    pub secondary: String,
}

/// One generation: the character being built and the stream it draws
/// from.
pub struct Gen<'a> {
    pub d: &'a Data,
    pub r: &'a mut Rand,
    pub session: &'a Session,
    pub messages: Vec<String>,

    /// Player's Option was chosen (sub-abilities, kit flag +0x74).
    pub po: bool,
    /// The stages after the kit follow the Core Rules paths. Always true:
    /// the Player's Option paths are not decoded (see `messages`).
    pub core: bool,
    pub classes: Vec<&'a Klass>,
    pub levels: Vec<i32>,
    pub title: String,
    pub xp: Vec<i32>,
    pub sex: i32,
    pub race: &'a Race,
    pub subrace: &'a Race,
    pub racial_abilities: Vec<String>,
    /// +0x738: "the level" of many stages (the last class's level).
    pub m738: i32,
    pub name: String,
    pub ab: [i32; 6],
    pub exc: i32,
    /// Sub-abilities per ability: (value + d, value - d) = muscle/stamina,
    /// aim/balance, health/fitness, reason/knowledge, intuition/willpower,
    /// leadership/appearance.
    pub subs: [[i32; 2]; 6],
    pub alignment: String,
    pub kit_name: String,
    pub kit_idx: Option<usize>,
    pub class_abilities: Vec<Vec<&'a Part>>,
    /// +0x6f4..+0x6f9 and +0x70c.
    pub m6f: [bool; 6],
    pub m70c: bool,
    pub social: String,
    pub background: String,
    pub height: i32,
    pub weight: i32,
    pub age: i32,
    pub gold: i32,
    pub budget: i32,

    pub nwp: Vec<(String, i32)>,
    pub slots: i32,
    pub lang_pool: i32,
    pub m784: i32,
    pub wp: Weapons,
    pub weapon_list: Vec<String>,
    pub weapon_spec: Vec<String>,
    pub languages: Vec<String>,
    pub nwp_langs: HashMap<usize, Vec<String>>,
    pub hp: Vec<Vec<i32>>,
    pub eq: Equipment,
    pub inventory: Vec<Vec<It>>,
    pub carried: f32,
    pub coins: (i32, i32, i32),
    pub schools: Vec<Vec<String>>,
    pub minor: Vec<Vec<String>>,
    pub major: Vec<Vec<String>>,
    /// Per class and spell level 0..10: memorized wizard and priest spells.
    pub spells: Vec<Vec<(Vec<String>, Vec<String>)>>,
    pub book_w: Vec<String>,
    pub book_p: Vec<String>,
    pub psi: Psionics,
    pub psp: Vec<i32>,
}

impl<'a> Gen<'a> {
    /// The setup of 0x4bf418 for `race` and `classes`; no draw.
    pub fn new(
        d: &'a Data,
        r: &'a mut Rand,
        session: &'a Session,
        race: &str,
        classes: &[(String, i32)],
        po: bool,
    ) -> Result<Self> {
        if classes.is_empty() {
            return Err(Error::Invalid(
                "You must select a class for your NPC".into(),
            ));
        }
        let mut ks = Vec::new();
        for (name, _) in classes {
            let k = d
                .class(name)
                .ok_or_else(|| Error::Invalid(format!("no class named {name}")))?;
            ks.push(&d.classes[k]);
        }
        let (race, subrace) = resolve_race(d, race)?;
        let levels: Vec<i32> = classes.iter().map(|c| c.1).collect();
        Ok(Self {
            d,
            r,
            session,
            messages: Vec::new(),
            po,
            core: true,
            xp: ks.iter().zip(&levels).map(|(k, &l)| k.xp_for(l)).collect(),
            title: classes
                .iter()
                .map(|c| c.0.as_str())
                .collect::<Vec<_>>()
                .join("/"),
            classes: ks,
            levels,
            sex: 0,
            race,
            subrace,
            racial_abilities: subrace.abilities.clone(),
            m738: 0,
            name: String::new(),
            ab: [0; 6],
            exc: 0,
            subs: [[0; 2]; 6],
            alignment: String::new(),
            kit_name: String::new(),
            kit_idx: None,
            class_abilities: Vec::new(),
            m6f: [false; 6],
            m70c: false,
            social: String::new(),
            background: String::new(),
            height: 0,
            weight: 0,
            age: 0,
            gold: 0,
            budget: 0,
            nwp: Vec::new(),
            slots: 0,
            lang_pool: 0,
            m784: 0,
            wp: Weapons::default(),
            weapon_list: Vec::new(),
            weapon_spec: Vec::new(),
            languages: Vec::new(),
            nwp_langs: HashMap::new(),
            hp: Vec::new(),
            eq: Equipment::default(),
            inventory: vec![Vec::new(); 9],
            carried: 0.0,
            coins: (0, 0, 0),
            schools: Vec::new(),
            minor: Vec::new(),
            major: Vec::new(),
            spells: Vec::new(),
            book_w: Vec::new(),
            book_p: Vec::new(),
            psi: Psionics::default(),
            psp: vec![0; 101],
        })
    }

    // ---- the stream ----

    /// `between(lo, hi)` (0x554d44): no draw for an empty range.
    pub fn between(&mut self, lo: i32, hi: i32) -> i32 {
        let n = hi.wrapping_add(1).wrapping_sub(lo);
        if n < 1 {
            return 0;
        }
        (self.r.rand() % n).wrapping_add(lo)
    }

    /// `mod(n)` (0x554d70).
    pub fn modn(&mut self, n: i32) -> i32 {
        if n == 0 { 0 } else { self.r.rand() % n }
    }

    /// `between0(lo, hi)` (0x4db6c0): only an empty range of 0 is guarded.
    pub fn between0(&mut self, lo: i32, hi: i32) -> i32 {
        let n = hi.wrapping_add(1).wrapping_sub(lo);
        if n == 0 {
            return 0;
        }
        (self.r.rand() % n).wrapping_add(lo)
    }

    /// Records a fallback once.
    pub fn warn(&mut self, message: impl Into<String>) {
        let m = message.into();
        if !self.messages.contains(&m) {
            self.messages.push(m);
        }
    }

    // ---- shorthands ----

    pub fn kit(&self) -> Option<&'a Kit> {
        self.kit_idx.and_then(|i| self.d.kits[i].as_ref())
    }

    /// The kit part exists (`c['_kit']`), with or without its kit object.
    pub fn has_kit_part(&self) -> bool {
        self.kit_idx.is_some()
    }

    pub fn has_group(&self, g: i32) -> bool {
        self.classes.iter().any(|k| k.group == g)
    }

    /// Some class has the flag.
    pub fn cflag(&self, off: u32) -> bool {
        self.classes.iter().any(|k| k.flag(off))
    }

    pub fn fitness(&self) -> i32 {
        self.subs[CON][1]
    }

    pub fn knowledge(&self) -> i32 {
        self.subs[INT][1]
    }

    pub fn reason(&self) -> i32 {
        self.subs[INT][0]
    }

    pub fn intuition(&self) -> i32 {
        self.subs[WIS][0]
    }

    pub fn male(&self) -> bool {
        self.sex != 0
    }

    /// 0x43a8a6: two or more classes on a race that dual-classes.
    pub fn dual(&self) -> bool {
        self.classes.len() >= 2 && self.race.unknown_0x1e0[2] != 0
    }

    /// 0x43fac6: the highest class level.
    pub fn char_level(&self) -> i32 {
        self.levels.iter().copied().max().unwrap_or(0)
    }

    pub fn class_ability_any(&self, f: impl Fn(&Part) -> bool) -> bool {
        self.class_abilities.iter().flatten().any(|p| f(p))
    }

    // ---- stages ----

    /// CCharacterDocNew 0x59c290: the base constructor (0x419607, call
    /// site 0x419f40) draws a sex, the derived one (0x59c466) draws it
    /// again; 0 is female.
    pub fn char_ctor(&mut self) -> i32 {
        self.between(0, 3);
        self.between(0, 3)
    }

    /// Constructor, +0x738, sex fix-up and name (0x4bf418 up to 0x4bfac5).
    pub fn identity(&mut self) {
        self.sex = self.char_ctor();
        // 0x4bf6b7: meant to be the highest level, but the running maximum
        // is never updated: the last class's level (or that of the first
        // class with +0x279d).
        for (k, &lv) in self.classes.iter().zip(&self.levels) {
            if lv > 0 {
                self.m738 = lv;
            }
            if k.flag(0x279d) {
                break;
            }
        }
        if self.race.name.to_uppercase().contains("SWANMAY") {
            self.sex = 0;
        }
        let sub = self.subrace.name.clone();
        self.name = self.make_name(&sub, self.male());
    }

    /// 0x4bfd74.
    pub fn roll_abilities(&mut self) {
        let mut v = [0i32; 6];
        for a in v.iter_mut() {
            // Two 3d6 decide a branch; both branches roll a fresh 3d6.
            for _ in 0..6 {
                self.between(1, 6);
            }
            *a = self.between(1, 6) + self.between(1, 6) + self.between(1, 6);
        }
        let classes = self.classes.clone();
        for k in &classes {
            let b = self.between(1, 3);
            for (i, a) in v.iter_mut().enumerate() {
                if k.v(0x1c + 4 * i as u32) > 3 {
                    *a += b;
                }
            }
        }
        let race = self.race;
        for (i, a) in v.iter_mut().enumerate() {
            let [lo, hi] = race.ability_limits[i];
            *a = (*a).max(lo).min(hi);
        }
        for (i, a) in v.iter_mut().enumerate() {
            *a = clamp125(*a + race.ability_adjustments[i]);
        }
        // The loop never advances its class index: class 0's level.
        let bonus = (self.levels[0].max(0) / 5).min(5);
        for k in &classes {
            for (i, a) in v.iter_mut().enumerate() {
                *a = (*a).max(k.v(0x1c + 4 * i as u32));
            }
            for (i, a) in v.iter_mut().enumerate() {
                if k.v(0x1c + 4 * i as u32) > 3 {
                    *a += bonus;
                }
            }
        }
        for a in v.iter_mut() {
            *a = clamp125(*a);
        }
        self.exc = if race.unknown_0x40 != 0 && v[STR] == 18 {
            self.between(1, 100)
        } else {
            0
        };
        self.ab = v;
        for (i, &a) in v.iter().enumerate() {
            let d = if self.po { self.between(-2, 2) } else { 0 };
            self.subs[i] = [a + d, a - d];
        }
    }

    /// EV 0x53ae70: 3d6 straight, race limits and adjustment, class
    /// minima; no class or level bonus.
    pub fn ev_roll_abilities(&mut self) {
        let mut v = [0i32; 6];
        for a in v.iter_mut() {
            *a = (0..3).map(|_| self.r.rand() % 6).sum::<i32>() + 3;
        }
        let race = self.race;
        for (i, a) in v.iter_mut().enumerate() {
            let [lo, hi] = race.ability_limits[i];
            *a = (*a).max(lo).min(hi);
            *a = clamp125(*a + race.ability_adjustments[i]);
        }
        for k in &self.classes {
            for (i, a) in v.iter_mut().enumerate() {
                *a = (*a).max(k.v(0x1c + 4 * i as u32));
            }
        }
        self.exc = if race.unknown_0x40 != 0 && v[STR] == 18 {
            self.r.rand() % 100 + 1
        } else {
            0
        };
        self.ab = v;
        for (i, &a) in v.iter().enumerate() {
            let d = if self.po { self.r.rand() % 5 - 2 } else { 0 };
            self.subs[i] = [a + d, a - d];
        }
    }

    /// 0x4c209c (no input alignment): one of class 0's alignments.
    pub fn st_alignment(&mut self) {
        let list = self.classes[0].sa(0xa0);
        if !list.is_empty() {
            let i = self.between(0, list.len() as i32 - 1);
            self.alignment = list[i as usize].clone();
        }
    }

    /// 0x4c2a75 (no input kit), then the kit's experience (0x4c3d1d), the
    /// class abilities (0x442cac) and their flags (0x4c7cb2).
    pub fn st_kit(&mut self) {
        let mut low = 100_000;
        for k in &self.classes {
            for i in 0..6 {
                if k.v(0x34 + 4 * i as u32) != 0 && self.ab[i] < low {
                    low = self.ab[i];
                }
            }
        }
        // (offset of the minimum byte, value tested), in 0x4c2a75's order.
        let s = &self.subs;
        let a = &self.ab;
        let tests: [(u32, i32); 18] = [
            (0x4c, a[0]),
            (0x50, s[0][1]),
            (0x4e, s[0][0]),
            (0x52, a[1]),
            (0x54, s[1][0]),
            (0x56, s[1][1]),
            (0x58, a[2]),
            (0x5a, s[2][0]),
            (0x5c, s[2][1]),
            (0x5e, a[3]),
            (0x62, s[3][0]),
            (0x60, s[3][1]),
            (0x64, a[4]),
            (0x66, s[4][0]),
            (0x68, s[4][1]),
            (0x6a, a[5]),
            (0x6e, s[5][0]),
            (0x6c, s[5][1]),
        ];
        // 0x670ed3: b6..be; any other alignment reads +0x63.
        let align_flag = match self.alignment.as_str() {
            "Lawful Good" => 6,
            "Lawful Neutral" => 7,
            "Lawful Evil" => 8,
            "Neutral Good" => 9,
            "True Neutral" => 10,
            "Neutral Evil" => 11,
            "Chaotic Good" => 12,
            "Chaotic Neutral" => 13,
            "Chaotic Evil" => 14,
            _ => 99,
        };
        let mut cand = Vec::new();
        for (p, kit) in self.d.parts[KITS].iter().zip(&self.d.kits) {
            let Some(kit) = kit else { continue };
            let usable = p.name() != "None";
            if !kit.flag(if self.po { 0x74 } else { 0x73 }) {
                continue;
            }
            if tests.iter().any(|&(o, v)| v < kit.b(o) || kit.max(o) < v) {
                continue;
            }
            let excluded = kit.sa(0xa8);
            if excluded.contains(&self.race.name) || excluded.contains(&self.subrace.name) {
                continue;
            }
            let allowed = kit.sa(0x10);
            if !allowed.is_empty() && !allowed.contains(&self.subrace.name) {
                continue;
            }
            let barred = p.sa(0x82c);
            if self.classes.iter().all(|k| barred.contains(&k.name)) {
                continue;
            }
            if !kit.combinations.is_empty() && !kit.combinations.contains(&self.title) {
                continue;
            }
            if !kit.flag(0x4) && self.sex != 0 {
                continue;
            }
            if !kit.flag(align_flag) {
                continue;
            }
            if kit.b(0x70) > low {
                continue;
            }
            if usable {
                cand.push(p.name().to_owned());
            }
        }
        if !cand.is_empty() {
            let i = self.between(1, cand.len() as i32) - 1;
            self.kit_name = cand[i as usize].clone();
        }
        self.kit_idx = self.d.part_exact(KITS, &self.kit_name);
        // 0x4c3d1d
        if let Some(kit) = self.kit()
            && kit.flag(0x5de)
        {
            let f = i64::from(kit.i(0x5ec)) + 100;
            for x in &mut self.xp {
                *x = (i64::from(*x) * f / 100) as i32;
            }
        }
        self.class_abilities = self
            .classes
            .iter()
            .map(|k| {
                k.sa(0x2724)
                    .iter()
                    .filter_map(|n| self.d.find_in(CLASS_ABILITIES, n))
                    .collect()
            })
            .collect();
        let any = |off: u32| self.class_ability_any(|p| p.n(off) != 0);
        let m6f = [
            any(0x7a0),
            any(0x7c0),
            any(0x7c4),
            any(0x7c8),
            any(0x81c),
            any(0x820),
        ];
        let m70c = any(0x782);
        self.m6f = m6f;
        self.m70c = m70c;
    }

    /// 0x4c2935: 2d6 indexes the kit's social classes (+0x10c).
    pub fn st_social(&mut self) {
        let mut n = self.between(1, 6) + self.between(1, 6);
        let list = self.kit().map_or(&[][..], |k| k.sa(0x10c));
        let top = list.len() as i32 - 1;
        if top < n {
            n = top;
        }
        let s = if top > 0 {
            list[n as usize].clone()
        } else {
            String::new()
        };
        self.social = if s.is_empty() {
            "Middle lower class".into()
        } else {
            s
        };
    }

    /// 0x4c5c34: one of the program's 21 backgrounds; "NONE" leaves it
    /// empty.
    pub fn st_background(&mut self) {
        let i = self.between(1, BACKGROUNDS.len() as i32) - 1;
        if i != 0 {
            self.background = BACKGROUNDS[i as usize].into();
        }
    }

    fn dice(&mut self, base: i32, n: i32, s: i32) -> i32 {
        let mut v = base;
        for _ in 0..n {
            v = v.wrapping_add(self.between0(1, s));
        }
        v
    }

    /// Height, weight, age (0x4c3daf, 0x4c3e4e, 0x4c3eed): the subrace's
    /// dice, the race's when that gives 0.
    pub fn st_physical(&mut self) {
        let male = self.male();
        let roll = |g: &mut Self, race: &Race, i: usize| -> i32 {
            let p = &race.physical;
            match i {
                0 => g.dice(if male { p[0] } else { p[1] }, p[2], p[3]),
                1 => g.dice(if male { p[4] } else { p[5] }, p[6], p[7]),
                _ => g.dice(p[8], p[9], p[10]),
            }
        };
        let mut out = [0; 3];
        for (i, o) in out.iter_mut().enumerate() {
            let sub = self.subrace;
            let v = roll(self, sub, i);
            *o = if v != 0 {
                v
            } else {
                let race = self.race;
                roll(self, race, i)
            };
        }
        [self.height, self.weight, self.age] = out;
    }

    /// 0x4c1086 → 0x4330ab: starting gold, and the shopping budget in
    /// copper.
    pub fn st_money(&mut self) {
        let kit = self.kit();
        let f32of = |x: f64| f64::from(x as f32);
        let factor = |kit: &Kit, social: &str, name: &str| -> (f64, f64) {
            let pct = match kit.i(0x8c) {
                0 => 100,
                p => p,
            };
            let mut f = f32of(f64::from(pct) * f64::from(0.01f32));
            if name == "Cavalier" && social == "Lower class" {
                f = 1.0;
            }
            let b = f32of(f64::from(kit.i(0x88)) * f64::from(0.01f32));
            (f, b)
        };
        let mut gp;
        if let Some(kit) = kit
            && kit.flag(0x618)
        {
            let (n, s, mul, add) = (kit.i(0x608), kit.i(0x60c), kit.i(0x610), kit.i(0x614));
            let mut m = if !kit.flag(0x84) {
                let mut sum = 0i32;
                for _ in 0..n {
                    sum = sum.wrapping_add(self.between(1, s));
                }
                sum
            } else {
                n.wrapping_mul(s)
            };
            m = m.wrapping_mul(mul).wrapping_add(add);
            let (f, b) = factor(kit, &self.social, &self.kit_name);
            let m = f64::from(m);
            gp = (m * f + m * b) as i32;
        } else {
            let (mut m, alt) = if self.has_group(0) {
                (self.sum_dice(5, 4) * 10, 200)
            } else if self.has_group(2) {
                (self.sum_dice(3, 6) * 10, 180)
            } else if self.has_group(4) {
                (self.sum_dice(3, 4) * 10, 120)
            } else if self.has_group(1) {
                (self.sum_dice(2, 6) * 10, 120)
            } else if self.has_group(3) {
                ((self.between(1, 4) + 1) * 10, 50)
            } else {
                (0, 0)
            };
            let (mut f, mut b) = (1.0, 0.0);
            if let Some(kit) = kit {
                (f, b) = factor(kit, &self.social, &self.kit_name);
                if kit.flag(0x84) {
                    m = alt;
                }
            }
            let m = f64::from(m);
            gp = (m * f + m * b) as i32;
            if self.cflag(0x4865) {
                gp = gp.wrapping_shl(1);
            }
        }
        let add = if self.social == "Lower class" {
            25
        } else {
            100
        };
        if self.m738 - 1 != 0 {
            gp = gp.wrapping_add(add * self.m738 * 10);
        }
        self.gold = gp;
        self.budget = gp.wrapping_mul(100);
    }

    fn sum_dice(&mut self, n: i32, s: i32) -> i32 {
        (0..n).map(|_| self.between(1, s)).sum()
    }

    // ---- hit points ----

    /// 0x42d8f7(g, L).
    pub fn hp_roll(&mut self, g: usize, level: i32) -> i32 {
        let n = self.classes.len() as i32;
        let k = self.classes[g];
        let mut h;
        if level > k.v(0x88) {
            h = k.v(0x8c).div_euclid(n).max(1);
        } else {
            let extra = self.class_abilities[g]
                .iter()
                .filter(|p| p.n(0x781) != 0)
                .count() as i32;
            h = self.between(1, k.v(0x84) + 2 * extra);
            h = h.max(fit_min(self.fitness()));
            if level > 1 {
                for name in &self.racial_abilities {
                    if let Some(p) = self.d.find_in(RACIAL_ABILITIES, name) {
                        h += p.n(0x520);
                    }
                }
            }
            if level == 1 && k.m94[1] > 0 {
                h += k.m94[2] * self.between(1, k.m94[1]);
            }
            if level > 1 && !self.dual() {
                h /= n;
            }
            if h < 1 {
                h = 1;
            }
            if level == 1 {
                let rb = self.race.unknown_0x16c[1];
                h += rb;
                if rb < 1 {
                    h += self.classes.iter().map(|k| k.i90).sum::<i32>();
                }
            }
            h += k.i4860;
        }
        if let Some(kit) = self.kit() {
            h += kit.i(0x604);
        }
        h
    }

    /// Every level's roll of every class (0x4c6149, and 0x42d888 for each
    /// individual of an encounter package).
    pub fn st_hp(&mut self) {
        let mut hp = Vec::new();
        for g in 0..self.classes.len() {
            let mut rolls = Vec::new();
            for level in 1..=self.levels[g] {
                rolls.push(self.hp_roll(g, level));
            }
            hp.push(rolls);
        }
        self.hp = hp;
    }

    /// 0x43ba4a(g), non-dual: the warriors' table when a class is a
    /// warrior, class g has +0x279c or a class ability has +0x7b0.
    fn con_hp(&self, g: usize) -> i32 {
        let war = self.has_group(0)
            || self.classes[g].flag(0x279c)
            || self.class_ability_any(|p| p.n(0x7b0) != 0);
        let table = if war { &CON_HP_WARRIOR } else { &CON_HP };
        table[self.fitness().clamp(0, 25) as usize]
    }

    /// 0x42e0ab: the maximum hit points of the rolls in `hp`.
    pub fn max_hp(&mut self) -> i32 {
        if self.dual() {
            // 0x42e0ab / 0x43ba4a, dual-class paths: not decoded; the
            // multi-class formula is used.
            self.warn("Dual-class maximum hit points (0x42e0ab) are not decoded; the multi-class formula was used.");
        }
        let n = self.classes.len() as i32;
        let mut s: i32 = (0..self.classes.len())
            .map(|g| self.hp[g].first().copied().unwrap_or(0))
            .sum();
        if !self.cflag(0x4864) {
            s /= n;
        }
        let cb = self.con_hp(0);
        s += cb;
        if s <= 0 {
            s = 1;
        }
        let mut total = f64::from(s);
        let per = f64::from(cb) / f64::from(n);
        for g in 0..self.classes.len() {
            let last = self.classes[g].v(0x88);
            for level in 2..=self.levels[g] {
                let mut v = f64::from(self.hp[g][(level - 1) as usize]);
                if level <= last {
                    v += per;
                }
                if v < 1.0 {
                    v = 1.0;
                }
                total += v;
            }
        }
        total as i32
    }
}

/// 0x4c105c
pub fn clamp125(v: i32) -> i32 {
    v.clamp(1, 25)
}

/// 0x423590: the lowest hit point roll by Con (fitness).
fn fit_min(fitness: i32) -> i32 {
    match fitness {
        20 => 2,
        21 | 22 => 3,
        23..=25 => 4,
        _ => 0,
    }
}

/// 0x421bcb: hit point adjustment by Con.
const CON_HP: [i32; 26] = [
    0, -3, -2, -2, -1, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
];
/// 0x421ca0: the warriors' table.
const CON_HP_WARRIOR: [i32; 26] = [
    0, -3, -2, -2, -1, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 5, 6, 6, 6, 7, 7,
];

/// 0x4c76b8: the race (+0x200) and subrace (+0x204). A base race takes
/// its first subrace with +0x1e0; a subrace takes its parent as race.
fn resolve_race<'a>(d: &'a Data, name: &str) -> Result<(&'a Race, &'a Race)> {
    let r = d
        .race(name)
        .map(|i| &d.races[i])
        .ok_or_else(|| Error::Invalid(format!("no race named {name}")))?;
    if r.unknown_0x184 == 0 {
        let sub = d
            .races
            .iter()
            .find(|x| x.unknown_0x184 != 0 && x.unknown_0x188 == r.name && x.unknown_0x1e0[0] != 0)
            .unwrap_or(r);
        Ok((r, sub))
    } else {
        let parent = d.race(&r.unknown_0x188).map_or(r, |i| &d.races[i]);
        Ok((parent, r))
    }
}
