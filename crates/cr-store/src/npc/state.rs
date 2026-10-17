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
use super::program::Tx;
use super::texts::*;
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

/// A kit or alignment choice of the options object (+0x14c): empty from
/// Create NPC, "~Random" or a name from Create Multiple NPCs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Opt {
    #[default]
    Empty,
    Random,
    Name(String),
}

/// The kit tests of 0x4c2a75 and 0x4d9eb0 in their order: the ability
/// (0..6) and sub-ability (`None`: the score itself, else 0 or 1), and the
/// offset of the kit's minimum byte.
const KIT_TESTS: [(usize, Option<usize>, u32); 18] = [
    (0, None, 0x4c),
    (0, Some(1), 0x50),
    (0, Some(0), 0x4e),
    (1, None, 0x52),
    (1, Some(0), 0x54),
    (1, Some(1), 0x56),
    (2, None, 0x58),
    (2, Some(0), 0x5a),
    (2, Some(1), 0x5c),
    (3, None, 0x5e),
    (3, Some(0), 0x62),
    (3, Some(1), 0x60),
    (4, None, 0x64),
    (4, Some(0), 0x66),
    (4, Some(1), 0x68),
    (5, None, 0x6a),
    (5, Some(0), 0x6e),
    (5, Some(1), 0x6c),
];

/// The alignments in 0x4c209c's order and the kit flag of each
/// (0x670ed3: +6 .. +0xe).
pub const ALIGNMENTS: [(Tx, u32); 9] = [
    (LAWFUL_GOOD, 6),
    (LAWFUL_NEUTRAL, 7),
    (LAWFUL_EVIL, 8),
    (NEUTRAL_GOOD, 9),
    (TRUE_NEUTRAL, 10),
    (NEUTRAL_EVIL, 11),
    (CHAOTIC_GOOD, 12),
    (CHAOTIC_NEUTRAL, 13),
    (CHAOTIC_EVIL, 14),
];

/// The order 0x4c209c lists a kit's alignments in (indexes into
/// [`ALIGNMENTS`]; Chaotic Evil twice).
const KIT_ALIGN_ORDER: [usize; 10] = [8, 1, 2, 3, 5, 6, 7, 8, 4, 0];

fn align_flag(d: &Data, name: &str) -> Option<u32> {
    ALIGNMENTS.iter().find(|a| d.t(a.0) == name).map(|a| a.1)
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
    /// The Core Rules paths (0x446a70: char +0x7c == 0), not Player's
    /// Option.
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
    /// Sub-abilities per ability: (value + d, value - d) = stamina/muscle,
    /// aim/balance, health/fitness, reason/knowledge, intuition/willpower,
    /// leadership/appearance.
    pub subs: [[i32; 2]; 6],
    pub alignment: String,
    /// The options object's kit (+0xc) and alignment (+0x10), and the kit
    /// of the kit option (0x465f6c, 0x6618ce).
    pub opt_kit: Opt,
    pub opt_align: Opt,
    pub in_kit: Option<&'a Kit>,
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

    /// Name, cost and the third member of 0x4300b5.
    pub nwp: Vec<(String, i32, i32)>,
    /// char +0x1e3c: (class, proficiency, copies) 0x4d9527 recorded.
    pub lang_counts: Vec<(usize, String, i32)>,
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
    /// Traits (+0x1830) and disadvantages (+0x1844), Player's Option.
    pub traits: Vec<String>,
    pub disadvantages: Vec<String>,
    /// Rogue skills (+0x15c8): name and the points of each level.
    pub rogue: Vec<(String, [i32; 100])>,
    /// +0x1610: the kit's fixed rogue adjustments were applied.
    pub m1610: bool,
    /// +0x15e4 (random, by level) and +0x15f8 (fixed) kit bonus objects.
    pub rogue_random: Vec<Option<super::rogue::KitBonus>>,
    pub rogue_fixed: Vec<Option<super::rogue::KitBonus>>,
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
            return Err(Error::Invalid(d.t(NO_CLASS).into()));
        }
        let mut ks = Vec::new();
        for (name, _) in classes {
            let k = d
                .class(name)
                .ok_or_else(|| Error::Invalid(format!("no class named {name}")))?;
            ks.push(&d.classes[k]);
        }
        let (race, subrace) = resolve_race(d, race, po)?;
        let levels: Vec<i32> = classes.iter().map(|c| c.1).collect();
        Ok(Self {
            d,
            r,
            session,
            messages: Vec::new(),
            po,
            core: !po,
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
            opt_kit: Opt::Empty,
            opt_align: Opt::Empty,
            in_kit: None,
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
            lang_counts: Vec::new(),
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
            traits: Vec::new(),
            disadvantages: Vec::new(),
            rogue: Vec::new(),
            m1610: false,
            rogue_random: Vec::new(),
            rogue_fixed: Vec::new(),
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

    /// The program text `tx` (read from the installation's program).
    pub fn t(&self, tx: Tx) -> &'a str {
        self.d.program.text(tx)
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
        self.abil_get(CON, Some(1))
    }

    pub fn knowledge(&self) -> i32 {
        self.abil_get(INT, Some(1))
    }

    pub fn reason(&self) -> i32 {
        self.abil_get(INT, Some(0))
    }

    pub fn intuition(&self) -> i32 {
        self.abil_get(WIS, Some(0))
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
        if self.race.name.to_uppercase().contains(self.t(SWANMAY)) {
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
        if let Some(kit) = self.in_kit {
            for (i, a) in v.iter_mut().enumerate() {
                let off = 0x4c + 6 * i as u32;
                *a = (*a).max(kit.b(off)).min(kit.max(off));
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

    /// Sets the kit and alignment options of Create Multiple NPCs.
    pub fn set_options(&mut self, kit: Opt, alignment: Opt) {
        let norm = |o: Opt| match o {
            Opt::Name(s) if s.is_empty() => Opt::Empty,
            o => o,
        };
        let (kit, alignment) = (norm(kit), norm(alignment));
        self.in_kit = match &kit {
            Opt::Name(s) if s != "NONE" => self.d.find_kit(s),
            _ => None,
        };
        self.opt_kit = kit;
        self.opt_align = alignment;
    }

    /// 0x4c209c: an alignment option is kept ("~Random" presets Lawful
    /// Good). Without a kit option one of class 0's alignments (+0xa0);
    /// with one, the class alignments the kit allows, else every
    /// alignment the kit allows, else any (0x4c1966).
    pub fn st_alignment(&mut self) {
        match &self.opt_align {
            Opt::Name(a) => {
                self.alignment = a.clone();
                return;
            }
            Opt::Random => self.alignment = self.t(ALIGNMENTS[0].0).into(),
            _ => {}
        }
        let list = self.classes[0].sa(0xa0);
        let kit = self.in_kit;
        let has = |off: u32| kit.is_some_and(|k| k.flag(off));
        let mut filtered: Vec<&str> = Vec::new();
        for x in list {
            if matches!(&self.opt_kit, Opt::Empty | Opt::Random)
                || self.opt_kit == Opt::Name("NONE".into())
            {
                let i = self.between(0, list.len() as i32 - 1);
                self.alignment = list[i as usize].clone();
                return;
            }
            if align_flag(self.d, x).is_some_and(has) {
                filtered.push(x);
            }
        }
        let mut pick = String::new();
        if filtered.is_empty() {
            let all: Vec<&str> = KIT_ALIGN_ORDER
                .iter()
                .map(|&i| ALIGNMENTS[i])
                .filter(|a| has(a.1))
                .map(|a| self.t(a.0))
                .collect();
            if !all.is_empty() {
                pick = all[self.between(0, all.len() as i32 - 1) as usize].into();
            }
        } else {
            pick = filtered[self.between(0, filtered.len() as i32 - 1) as usize].into();
        }
        if pick.is_empty() {
            let i = self.modn(9) as usize;
            pick = self.t(ALIGNMENTS[i].0).into();
        }
        self.alignment = pick;
    }

    /// An ability or sub-ability as the vtable getters read it: a
    /// sub-ability reads its score outside Player's Option, and is held
    /// within 1..25 in it (0x43c870 -> 0x4b00ba).
    pub(super) fn abil_get(&self, i: usize, sub: Option<usize>) -> i32 {
        match sub {
            Some(j) if self.po => clamp125(self.subs[i][j]),
            _ => self.ab[i],
        }
    }

    fn abil_set(&mut self, i: usize, sub: Option<usize>, v: i32) {
        match sub {
            Some(j) => self.subs[i][j] = v,
            None => self.ab[i] = v,
        }
    }

    /// 0x4d9eb0 (a kit option): every class's minima (+0x34..) raise the
    /// scores, then each is held within the kit's minimum and maximum.
    fn fit_to_kit(&mut self) {
        for k in self.classes.clone() {
            for i in 0..6 {
                let min = k.v(0x34 + 4 * i as u32);
                if self.abil_get(i, None) < min {
                    self.ab[i] = min;
                }
            }
        }
        let Some(kit) = self.kit() else { return };
        for (i, sub, off) in KIT_TESTS {
            if self.abil_get(i, sub) < kit.b(off) {
                self.abil_set(i, sub, kit.b(off));
            }
            if kit.max(off) < self.abil_get(i, sub) {
                self.abil_set(i, sub, kit.max(off));
            }
        }
    }

    /// 0x4c2a75: a kit option is kept (and the scores fitted to it);
    /// "~Random" presets "None" and draws like Create NPC: one of the kits
    /// the character qualifies for, then the kit's experience (0x4c3d1d).
    /// Then the class abilities (0x442cac) and their flags (0x4c7cb2).
    pub fn st_kit(&mut self) {
        let explicit = match &self.opt_kit {
            Opt::Name(ks) => Some(ks.clone()),
            Opt::Random => {
                self.kit_name = "None".into();
                None
            }
            _ => None,
        };
        if let Some(ks) = explicit {
            // An explicit kit (also "None"): set, the scores fitted to it,
            // and no kit experience.
            self.kit_idx = self.d.part_exact(KITS, &ks);
            self.kit_name = ks;
            self.fit_to_kit();
        } else {
            self.pick_kit();
        }
        self.class_abilities_of();
    }

    fn pick_kit(&mut self) {
        let mut low = 100_000;
        for k in &self.classes {
            for i in 0..6 {
                if k.v(0x34 + 4 * i as u32) != 0 && self.ab[i] < low {
                    low = self.ab[i];
                }
            }
        }
        // (offset of the minimum byte, value tested), in 0x4c2a75's order.
        let tests = KIT_TESTS.map(|(i, sub, off)| (off, self.abil_get(i, sub)));
        // 0x670ed3: b6..be; any other alignment reads +0x63.
        let align_flag = align_flag(self.d, &self.alignment).unwrap_or(99);
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
    }

    /// The class abilities (0x442cac; 0x4d5e28 leaves out the kit's
    /// +0x1c8 names) and their flags (0x4c7cb2).
    fn class_abilities_of(&mut self) {
        let skip = self.kit().map_or(&[][..], |k| k.sa(0x1c8));
        self.class_abilities = self
            .classes
            .iter()
            .map(|k| {
                k.sa(0x2724)
                    .iter()
                    .filter(|n| !skip.contains(n))
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
            self.t(MIDDLE_LOWER_CLASS).into()
        } else {
            s
        };
    }

    /// 0x4c5c34: one of the program's 21 backgrounds. The code skips
    /// index 0, yet the saved character then reads "NONE" (the first
    /// entry): the field is taken as the drawn entry.
    pub fn st_background(&mut self) {
        let d = self.d;
        let list = &d.program.backgrounds;
        let i = self.between(1, list.len() as i32) - 1;
        self.background = list.get(i as usize).cloned().unwrap_or_default();
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
        let d = self.d;
        let f32of = |x: f64| f64::from(x as f32);
        let factor = |kit: &Kit, social: &str, name: &str| -> (f64, f64) {
            let pct = match kit.i(0x8c) {
                0 => 100,
                p => p,
            };
            let mut f = f32of(f64::from(pct) * f64::from(0.01f32));
            if name == d.t(CAVALIER) && social == d.t(LOWER_CLASS) {
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
        let add = if self.social == self.t(LOWER_CLASS) {
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

    /// 0x43ba4a(g): the warriors' table when a class is a warrior, class g
    /// has +0x279c or a class ability has +0x7b0.
    /// Dual-class: class g alone (a warrior, +0x279c, or one of its own
    /// class abilities with +0x7b0).
    fn con_hp(&self, g: usize) -> i32 {
        let war = if self.dual() {
            self.classes[g].group == 0
                || self.classes[g].flag(0x279c)
                || self.class_abilities[g].iter().any(|p| p.n(0x7b0) != 0)
        } else {
            self.has_group(0)
                || self.classes[g].flag(0x279c)
                || self.class_ability_any(|p| p.n(0x7b0) != 0)
        };
        let table = if war { &CON_HP_WARRIOR } else { &CON_HP };
        table[self.fitness().clamp(0, 25) as usize]
    }

    /// 0x42e0ab: the maximum hit points of the rolls in `hp`.
    pub fn max_hp(&mut self) -> i32 {
        if self.dual() {
            // The level counter runs on across the classes: each class adds
            // the rolls of its levels above the earlier classes', each with
            // its own Con bonus while within +0x88; no clamp.
            let mut total = 0i32;
            let mut level = 1;
            for g in 0..self.classes.len() {
                let last = self.classes[g].v(0x88);
                let bonus = self.con_hp(g);
                while level <= self.levels[g] {
                    total = total.wrapping_add(self.hp[g][(level - 1) as usize]);
                    if level <= last {
                        total = total.wrapping_add(bonus);
                    }
                    level += 1;
                }
            }
            return total;
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
/// its first subrace with +0x1e0; a subrace takes its parent as race. In
/// Player's Option "Half-ogre" and "Half-orc" are their "Standard ..."
/// subraces.
fn resolve_race<'a>(d: &'a Data, name: &str, po: bool) -> Result<(&'a Race, &'a Race)> {
    let t = |tx| d.program.text(tx);
    let name = match name {
        n if po && n == t(HALF_OGRE) => t(STANDARD_HALF_OGRE),
        n if po && n == t(HALF_ORC) => t(STANDARD_HALF_ORC),
        n => n,
    };
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
