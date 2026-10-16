//! Equipment (0x4c7fa1, Core Rules): weapons, armor, the backpack, kit
//! items, basics, proficiency and class gear, clothes, racial and magic
//! items, coins and the mount.

use cr_formats::parts::Part;

use super::Gen;
use super::data::{ARMOR, Data, EQUIPMENT, MAGIC_ITEMS, PartExt, WEAPONS};
use super::prof::{name0, pierce_slash};
use super::program::CAPACITY;
use super::program::Tx;
use super::state::{It, STR, inv_add};
use super::texts::*;

/// The generator's item lists by member offset; the character's nine
/// bundles take them in [`INV_ORDER`].
pub const L6C: usize = 0;
pub const L380: usize = 1;
pub const L3EC: usize = 2;
pub const L458: usize = 3;
pub const L4C4: usize = 4;
pub const L530: usize = 5;
pub const L59C: usize = 6;
pub const L608: usize = 7;
pub const L674: usize = 8;

/// The lists in inventory order (0x498110..0x498210); `None` is the
/// coins.
pub const INV_ORDER: [Option<usize>; 9] = [
    Some(L458),
    Some(L608),
    Some(L6C),
    Some(L4C4),
    Some(L530),
    Some(L3EC),
    None,
    Some(L59C),
    Some(L674),
];

/// Lists whose purchases count toward the weight (+0x6f0).
const WEIGHED: [usize; 4] = [L380, L3EC, L6C, L4C4];

/// Magic item lists by category (+0x584 of the magic item part).
const M290: usize = 0;
const M2A4: usize = 1;
const M2B8: usize = 2;
const M2CC: usize = 3;
const M2E0: usize = 4;
const M2F4: usize = 5;
const M308: usize = 6;
const M31C: usize = 7;
const M344: usize = 8;
const M330: usize = 9;

/// Where a purchase goes.
enum Dest<'x> {
    List(usize),
    Bundle(&'x mut Vec<It>),
}

/// The equipment stage's state.
#[derive(Debug, Clone, Default)]
pub struct Equipment {
    pub lists: [Vec<It>; 9],
    /// Copper left to spend.
    pub budget: i32,
    /// Weight carried.
    pub w: f32,
    pub horse: String,
    pub m700: bool,
    pub m701: bool,
    pub m704: i32,
    pub m708: i32,
    pub race_eq: Vec<String>,
    pub magic: [Vec<String>; 10],
    pub l724: Vec<String>,
}

/// Proficiency gear of 0x4cbc6e: (proficiency, [(item, free)]).
const NWP_GEAR: [(Tx, &[(Tx, bool)]); 10] = [
    (HEAL, &[(HEALER_S_BAG, true)]),
    (TAILOR, &[(SEWING_NEEDLE, false)]),
    (NAVIGATION, &[(SEXTANT, false)]),
    (HERBALIST, &[(HERBS_PER_LB, true)]),
    (ROPE_USE, &[(ROPE_SILK_50_FT, true)]),
    (
        READING_WRITING,
        &[(WRITING_INK_PER_VIAL, false), (PAPER_PER_SHEET, false)],
    ),
    (BOAT_PILOTING, &[(SEXTANT, false)]),
    (FISHING, &[(FISHHOOK, true), (FISHING_NET_10_FT_SQ, false)]),
    (COBBLING, &[(BOOTS_SOFT, false)]),
    (MOUNTAINEERING, &[(CRAMPONS, false)]),
];

/// 0x4d6504: the container a weapon or its ammunition is carried in.
fn container_for<'a>(d: &'a Data, p: &Part) -> &'a str {
    if p.n(0x2c0) != 0 {
        d.t(KNIFE_SHEATH)
    } else if p.n(0x2c8) != 0 {
        d.t(QUIVER)
    } else if p.n(0x2cc) != 0 {
        d.t(SCABBARD_HANGER_BALDRIC)
    } else if p.n(0x2b8) != 0 {
        d.t(BOLT_CASE)
    } else {
        ""
    }
}

/// The +0x44c (magic bonus) test of 0x4ca7ab: too strong or too weak for
/// level `l`.
fn level_gate(l: i32, b: i32) -> bool {
    (l < 2 && b > 0)
        || (l < 5 && b > 1)
        || (l < 9 && b > 2)
        || (l < 12 && b > 3)
        || (l < 16 && b > 4)
        || (l > 15 && b < 2)
        || (l > 10 && b < 1)
}

/// 0x4d1d3d: the ammunition whose +0x434 names the weapon.
fn ammo_for<'d>(d: &'d Data, wname: &str) -> Vec<&'d Part> {
    if wname.is_empty() || d.find(wname).is_none() {
        return Vec::new();
    }
    d.parts[WEAPONS]
        .iter()
        .filter(|p| p.n(0x420) != 0 && p.sa(0x434).iter().any(|n| n == wname))
        .collect()
}

fn f32_of(x: f64) -> f32 {
    x as f32
}

impl Gen<'_> {
    /// 0x4290f5(char, 2, 0): carrying capacity by Strength.
    fn capacity(&self) -> i32 {
        let mut s = self.ab[STR];
        let e = self.exc;
        if s == 18 {
            s = match e {
                1..=50 => 19,
                51..=75 => 20,
                76..=90 => 21,
                91..=99 => 22,
                100 => 23,
                _ => 18,
            };
        } else if s > 18 {
            s += 5;
        }
        CAPACITY[s.clamp(0, 30) as usize] - 1
    }

    /// 0x6697e9(class, armor name)
    fn class_allows_armor(&self, k: &super::data::Klass, name: &str) -> bool {
        let Some(a) = self.d.find_in(ARMOR, name) else {
            return false;
        };
        let an = match a.s(0x430) {
            "" => a.name(),
            s => s,
        };
        if k.sa(0x302c).iter().any(|n| n == an) {
            return true;
        }
        if k.flag(0x2ffc) && a.n(0x248) == 3 {
            return false;
        }
        match k.i3040 {
            2 => false,
            0 => true,
            _ => a.n(0x270) >= k.i2ff8,
        }
    }

    /// 0x443326
    fn kit_armor_ok(&self, p: &Part) -> bool {
        let list = self.kit().map_or(&[][..], |k| k.sa(0x1a0));
        list.is_empty() || list.iter().any(|n| n == p.name())
    }

    /// 0x43d7bc(char, part, 0x377): 0 = the character may own the part.
    fn restricted(&mut self, p: &Part) -> i32 {
        if p.s(0x1e0) == self.race.name {
            return 0x1f;
        }
        let t = self.d.part_type(p.name());
        let kit = self.kit();
        if t == 0 {
            let n = name0(p);
            if self.kit_grants(n) {
                return 0;
            }
            if !self.any_class_allows(p.name()) {
                return 0x1e;
            }
            if kit.is_some_and(|k| k.sa(0x620).iter().any(|x| x == n)) {
                return 0x29;
            }
        } else if t == 1 {
            if kit.is_some_and(|k| k.flag(0x188)) {
                return 0;
            }
            if !self
                .classes
                .iter()
                .any(|k| self.class_allows_armor(k, p.name()))
            {
                let Some(kit) = kit else { return 0x1e };
                if kit.sa(0x1a0).is_empty() {
                    return 0x1e;
                }
                if !self.kit_armor_ok(p) {
                    return 0x29;
                }
            }
            if !self.kit_armor_ok(p) {
                return 0x29;
            }
        }
        if self.barred(p, t, true) {
            return 0x1e;
        }
        if self.cflag(0x27a0)
            && pierce_slash(p, self.core)
            && t == 0
            && !self.weapon_list.iter().any(|w| w == name0(p))
        {
            return 0x1e;
        }
        if t == 0 && kit.is_some_and(|k| k.sa(0xbc).iter().any(|x| x == name0(p))) {
            return 0x29;
        }
        0
    }

    /// 0x4c1201(name, list, qty, free, check): true when added.
    fn buy_to(&mut self, name: &str, dest: Dest<'_>, qty: i32, free: bool, check: bool) -> bool {
        let Some(p) = self.d.find(name) else {
            return false;
        };
        let kit = self.kit();
        if kit.is_some_and(|k| k.flag(0x75)) && p.n(0x228) != 0 {
            return false;
        }
        let mut free = free;
        if kit.is_some_and(|k| {
            k.sa(0x598).iter().any(|n| n == name) || k.sa(0x5ac).iter().any(|n| n == name)
        }) {
            free = true;
        }
        if check && self.restricted(p) != 0 {
            return false;
        }
        let list = match dest {
            Dest::List(l) => Some(l),
            Dest::Bundle(_) => None,
        };
        let mut w = None;
        if let Some(l) = list
            && WEIGHED.contains(&l)
        {
            let x = f32_of(f64::from(p.weight()) * f64::from(qty));
            if f64::from(self.capacity()) < f64::from(x) + f64::from(self.eq.w) {
                return false;
            }
            w = Some(x);
        }
        if !free {
            let cost = p.price(qty);
            if self.eq.budget <= cost {
                return false;
            }
            self.eq.budget -= cost;
        }
        let item = It::new(name, qty);
        match dest {
            Dest::List(l) => inv_add(&mut self.eq.lists[l], item, true),
            Dest::Bundle(b) => inv_add(b, item, true),
        }
        if let Some(mut x) = w {
            if list == Some(L3EC)
                && p.n(0x224) != 0
                && self.d.part_type(name) == 1
                && p.n(0x248) == 1
            {
                x = 0.0;
            }
            self.eq.w = f32_of(f64::from(x) + f64::from(self.eq.w));
        }
        true
    }

    fn buy(&mut self, name: &str, list: usize, qty: i32, free: bool) -> bool {
        self.buy_to(name, Dest::List(list), qty, free, true)
    }

    /// 0x4c8e54: racial equipment and the magic item lists.
    fn eq_categories(&mut self) {
        let d = self.d;
        let rn = self.race.name.as_str();
        self.eq.race_eq = d.parts[EQUIPMENT]
            .iter()
            .filter(|p| {
                [
                    (DWARVEN_EQUIPMENT, DWARF),
                    (HALFLING_EQUIPMENT, HALFLING),
                    (ELVEN_EQUIPMENT, ELF),
                    (GNOMISH_EQUIPMENT, GNOME),
                ]
                .iter()
                .any(|&(e, r)| p.s(0x244) == d.t(e) && rn == d.t(r))
            })
            .map(|p| p.name().to_owned())
            .collect();
        for p in &d.parts[MAGIC_ITEMS] {
            let k = match p.n(0x584) {
                1 => M290,
                2 => M2A4,
                7 => M2B8,
                4..=6 => M2CC,
                3 => M2E0,
                12 => M2F4,
                15 => M308,
                9 => M31C,
                10 => M344,
                11 => M330,
                _ => continue,
            };
            self.eq.magic[k].push(p.name().to_owned());
        }
    }

    /// 0x4d19c0: may an NPC of this level get this weapon?
    fn magic_ok(&self, w: &Part) -> bool {
        if w.n(0x420) != 0 || w.n(0x454) < 0 {
            return false;
        }
        let (i454, i450, mag) = (w.n(0x454), w.n(0x450), w.n(0x224) != 0);
        let mut magammo = false;
        if w.n(0x3c0) != 0 {
            for a in ammo_for(self.d, w.name()) {
                if self.d.find(a.name()).is_some_and(|ap| ap.n(0x224) != 0) {
                    magammo = true;
                }
            }
        }
        let l = self.m738;
        let mut ok = true;
        match l {
            0..=2 => {
                if mag {
                    ok = false;
                }
            }
            3..=7 => {
                if i454 > 1 || i450 > 1 {
                    ok = false;
                }
            }
            8..=12 => {
                if i454 > 2 || i450 > 2 {
                    ok = false;
                }
            }
            13..=16 => {
                if !mag || (i454 < 1 && i450 < 1) {
                    ok = false;
                }
                if w.n(0x3c0) != 0 && magammo {
                    ok = true;
                }
            }
            _ => {
                if !mag || (i454 < 2 && i450 < 2) {
                    ok = false;
                }
                if w.n(0x3c0) != 0 && magammo {
                    ok = true;
                }
                if self.has_group(2) && i454 > 0 {
                    ok = true;
                }
            }
        }
        // A kit with +0x4b2 refuses magical weapons other than its weapon
        // of choice.
        let choice = &self.wp.s788;
        if !choice.is_empty()
            && mag
            && self.kit().is_some_and(|k| k.flag(0x4b2))
            && name0(w) != choice
        {
            ok = false;
        }
        ok
    }

    /// 0x4c8929(weapon, list, first_of)
    fn eq_ammo(&mut self, wname: &str, first_of: Option<usize>) {
        let l = self.m738;
        let kit = self.kit();
        let d = self.d;
        let names: Vec<&str> = ammo_for(d, wname).iter().map(|p| p.name()).collect();
        for a in names {
            let Some(ap) = d.find(a) else { continue };
            if ap.n(0x420) == 0 {
                continue;
            }
            if ap.n(0x224) == 0 {
                if l >= 10 {
                    continue;
                }
            } else {
                let b = ap.n(0x454);
                if l < 3 || b < 0 || !(l < 15 || b > 1) || !(l < 11 || b > 0) {
                    continue;
                }
            }
            if ap.n(0x228) != 0 && l < 2 && kit.is_some_and(|k| k.flag(0x75)) {
                continue;
            }
            let cont = container_for(self.d, ap);
            if cont.is_empty() {
                let q = self.between(5, 20);
                self.buy(a, L380, q, true);
            } else {
                let mut b = Vec::new();
                let q = self.between(5, 20);
                if self.buy_to(a, Dest::Bundle(&mut b), q, true, true) {
                    let mut it = It::new(cont, 1);
                    it.bundle = Some(b);
                    inv_add(&mut self.eq.lists[L3EC], it, true);
                }
            }
            if let Some(list) = first_of {
                let mut b = Vec::new();
                self.buy_to(a, Dest::Bundle(&mut b), 1, true, true);
                if let Some(first) = self.eq.lists[list].first_mut() {
                    first.bundle = Some(b);
                }
            }
            return;
        }
    }

    fn lance_horse(&mut self, name: &str) {
        if !self.eq.horse.is_empty() {
            return;
        }
        if name.contains(self.t(LANCE)) {
            if name.contains(self.t(HEAVY)) {
                self.eq.horse = self.t(HORSE_HEAVY_WAR).into();
            } else if name.contains(self.t(MEDIUM)) {
                self.eq.horse = self.t(HORSE_MEDIUM_WAR).into();
            } else if name.contains(self.t(LIGHT)) {
                self.eq.horse = self.t(HORSE_LIGHT_WAR).into();
            }
        } else if name.contains(self.t(HORSEMAN)) {
            self.eq.horse = self.t(HORSE_LIGHT_WAR).into();
        }
    }

    /// 0x4d0dcf(names, +0x4c4, +0x6c): the first weapon bought is free,
    /// the second paid.
    fn eq_buy_weapon(&mut self, names: &[String]) {
        if self.eq.m700 && self.eq.m701 {
            return;
        }
        let n = names.len() as i32;
        if n < 1 {
            return;
        }
        let d = self.d;
        let mut last = String::from("None");
        for _ in 0..50 {
            let k = self.between(1, n) - 1;
            let Some(w) = d.find(&names[k as usize]) else {
                continue;
            };
            if self.excluded_weapon(w) || name0(w) == last {
                continue;
            }
            let cur = w.name();
            let dart = cur.contains(self.t(DART_SP)) || cur == self.t(DART);
            if !self.eq.m701 {
                if self.buy(cur, L4C4, 1, true) {
                    self.lance_horse(cur);
                    if dart {
                        let q = self.between(5, 12);
                        self.buy(cur, L380, q, true);
                    }
                    self.eq.m701 = true;
                    last = name0(w).to_owned();
                    if w.n(0x3b8) != 0 {
                        self.eq.m704 += 1;
                        let cont = container_for(d, w);
                        if !cont.is_empty() {
                            self.buy(cont, L3EC, 1, false);
                        }
                    } else {
                        self.eq.m708 += 1;
                        if w.n(0x3c0) != 0 {
                            self.eq_ammo(cur, Some(L4C4));
                        }
                    }
                }
            } else if !self.eq.m700
                && !(self.eq.m708 > 0 && w.n(0x3b8) == 0)
                && !(self.eq.m704 > 0 && w.n(0x3bc) == 0)
                && self.buy(cur, L6C, 1, false)
            {
                if self.eq.horse.is_empty() && cur.contains(self.t(LANCE)) {
                    let l = self.m738;
                    if l > 15 {
                        self.eq.horse = self.t(HORSE_HEAVY_WAR).into();
                    } else if l > 10 {
                        self.eq.horse = self.t(HORSE_MEDIUM_WAR).into();
                    } else if l > 6 {
                        self.eq.horse = self.t(HORSE_LIGHT_WAR).into();
                    }
                }
                if dart {
                    let q = self.between(5, 12);
                    self.buy(cur, L380, q, true);
                }
                self.eq.m700 = true;
                last = name0(w).to_owned();
                if w.n(0x3b8) != 0 {
                    self.eq.m704 += 1;
                    let cont = container_for(d, w);
                    if !cont.is_empty() {
                        // The weapon moves from +0x6c into a container
                        // worn on +0x3ec.
                        let l6c = &mut self.eq.lists[L6C];
                        if let Some(i) = l6c.iter().position(|x| x.name == cur) {
                            l6c.remove(i);
                        }
                        let mut b = Vec::new();
                        if self.buy_to(cur, Dest::Bundle(&mut b), 1, true, true) {
                            let mut it = It::new(cont, 1);
                            it.bundle = Some(b);
                            inv_add(&mut self.eq.lists[L3EC], it, true);
                        }
                    }
                } else {
                    self.eq.m708 += 1;
                    if w.n(0x3c0) != 0 {
                        self.eq_ammo(cur, None);
                    }
                }
            }
            if self.eq.m700 && self.eq.m701 {
                return;
            }
        }
    }

    /// Weapon candidates of 0x4cd6b4: type-0 parts (from the end down to
    /// index 1) whose base name is in `src`.
    fn weapon_candidates(&self, src: &[String], common: bool, ge0_450: bool) -> Vec<String> {
        let t0 = &self.d.parts[WEAPONS];
        let mut out = Vec::new();
        for s in src {
            for i in (1..t0.len()).rev() {
                let w = &t0[i];
                if w.n(0x420) != 0 || name0(w) != s {
                    continue;
                }
                if w.n(0x454) < 0 || (ge0_450 && w.n(0x450) < 0) {
                    continue;
                }
                if common && !self.common_weapon(w.name()) {
                    continue;
                }
                if self.magic_ok(w) || out.is_empty() {
                    out.push(w.name().to_owned());
                }
            }
        }
        out
    }

    /// 0x4cd6b4(+0x4c4, +0x6c)
    fn eq_weapons(&mut self) {
        self.eq.m700 = false;
        self.eq.m701 = false;
        self.eq.m704 = 0;
        self.eq.m708 = 0;
        let done = |g: &Self| g.eq.m700 && g.eq.m701;
        // The kit's weapon of choice first, as the specializations.
        let choice = self.wp.s788.clone();
        if !choice.is_empty() {
            let c = self.weapon_candidates(std::slice::from_ref(&choice), false, true);
            self.eq_buy_weapon(&c);
        }
        if !self.weapon_spec.is_empty() {
            let spec: Vec<String> = self
                .weapon_spec
                .iter()
                .filter(|s| **s != choice)
                .cloned()
                .collect();
            let c = self.weapon_candidates(&spec, false, true);
            self.eq_buy_weapon(&c);
        }
        if done(self) {
            return;
        }
        let l724 = self.eq.l724.clone();
        if !self.wp.plain.is_empty() {
            let c = self.weapon_candidates(&self.wp.plain, false, false);
            self.eq_buy_weapon(&c);
            if done(self) {
                return;
            }
            let c = self.weapon_candidates(&l724, true, false);
            self.eq_buy_weapon(&c);
            if done(self) {
                return;
            }
        }
        self.eq_buy_weapon(&l724);
    }

    /// End of 0x4c48a8: +0x724, the common weapons 0x4d19c0 accepts;
    /// skipped when the stale last weapon is in the kit's +0xd0 list.
    fn wp_list_724(&self) -> Vec<String> {
        let kit = self.kit();
        if let Some(last) = &self.wp.last_weapon
            && kit.is_some_and(|k| k.sa(0xd0).iter().any(|n| n == last))
        {
            return Vec::new();
        }
        let mut out = Vec::new();
        for w in &self.d.parts[WEAPONS] {
            let n0 = name0(w);
            for s in &self.wp.l30 {
                if s == n0 && self.magic_ok(w) {
                    out.push(w.name().to_owned());
                }
            }
        }
        out
    }

    /// 0x440b26: a class without +0x2ffc.
    fn some_class_shields(&self) -> bool {
        self.classes.iter().any(|k| !k.flag(0x2ffc))
    }

    /// 0x4ca7ab: body armor and shield.
    fn eq_armor(&mut self) {
        if self.race.name == self.t(AARAKOCRA) || !self.has_kit_part() {
            return;
        }
        let d = self.d;
        let kit = self.kit();
        let kflag = |o: u32| kit.is_some_and(|k| k.flag(o));
        let ksa = |o: u32| kit.map_or(&[][..], |k| k.sa(o));
        let rn = self.race.name.clone();
        let l = self.m738;
        let (mut body, mut shields) = (Vec::new(), Vec::new());
        let special = self.kit_name != "None" && kflag(0x188);
        for a in &d.parts[ARMOR] {
            let nm = a.name();
            let mut keep = true;
            if special {
                if a.n(0x44c) < 0
                    || nm.contains(self.t(MISSILE_ATTRACTION))
                    || nm.contains(self.t(MISSILE_ATT))
                    || (nm.contains(self.t(BLENDING)) && l < 2)
                {
                    continue;
                }
            } else {
                if self.restricted(a) != 0 {
                    continue;
                }
                if l == 1 && a.n(0x228) != 0 && kflag(0x75) {
                    continue;
                }
                let s1b4 = ksa(0x1b4);
                if l == 1 && !s1b4.is_empty() && !s1b4.iter().any(|n| n == nm) {
                    continue;
                }
                let s1a0 = ksa(0x1a0);
                if !s1a0.is_empty() && !s1a0.iter().any(|n| n == nm) {
                    continue;
                }
                if l == 1 && kit.map_or(0, |k| k.i(0x98)) > a.n(0x270) {
                    continue;
                }
                if nm.contains(self.t(DROW)) || nm.contains(self.t(SEA_ELF)) {
                    keep = false;
                }
                if (nm.contains(self.t(DWARVEN)) && rn != self.t(DWARF))
                    || (nm.contains(self.t(GNOMISH)) && rn != self.t(GNOME))
                {
                    continue;
                }
                if a.n(0x44c) < 0 {
                    keep = false;
                }
                if nm.contains(self.t(MISSILE_ATTRACTION)) || nm.contains(self.t(MISSILE_ATT)) {
                    continue;
                }
                let e = a.s(0x244);
                if (e == d.t(DWARVEN_EQUIPMENT) && rn != d.t(DWARF))
                    || (e == d.t(HALFLING_EQUIPMENT) && rn != d.t(HALFLING))
                    || (e == d.t(ELVEN_EQUIPMENT) && !d.is(&rn, &[ELF, HALF_ELF, HUMAN]))
                    || (e == d.t(GNOMISH_EQUIPMENT) && rn != d.t(GNOME))
                {
                    continue;
                }
            }
            if level_gate(l, a.n(0x44c)) {
                continue;
            }
            if !special {
                if !self.some_class_shields() && a.n(0x248) == 3 {
                    continue;
                }
                if !keep {
                    continue;
                }
            }
            if a.n(0x248) == 3 {
                shields.push(nm.to_owned());
            } else if a.n(0x248) != 2 {
                body.push(nm.to_owned());
            }
        }
        if !body.is_empty() {
            let (mut tries, mut ok) = (0, false);
            while !ok && tries < 20 {
                tries += 1;
                let nm = body[self.between(0, body.len() as i32 - 1) as usize].clone();
                if nm.contains(self.t(OF_BLENDING)) && tries < 4 {
                    continue;
                }
                ok = self.buy(&nm, L3EC, 1, false);
            }
        }
        if self.some_class_shields() && !shields.is_empty() {
            let nm = shields[self.between(0, shields.len() as i32 - 1) as usize].clone();
            self.buy(&nm, L3EC, 1, false);
        }
    }

    /// 0x4c167c("Backpack"): paid when affordable, else free.
    fn eq_backpack(&mut self) -> Option<It> {
        let p = self.d.find(self.t(BACKPACK))?;
        let cost = p.price(1);
        if cost < self.eq.budget {
            self.eq.budget -= cost;
        }
        self.eq.w = f32_of(f64::from(p.weight()) + f64::from(self.eq.w));
        Some(It::new(self.t(BACKPACK), 1))
    }

    /// 0x4d76b4: the kit's item lists, free, filed by category.
    fn eq_kit_items(&mut self) {
        let Some(kit) = self.kit() else { return };
        for off in [0x584, 0x598, 0x5ac, 0x5c0] {
            for nm in kit.sa(off) {
                match self.d.find(nm) {
                    Some(p) => self.eq_file(p, true),
                    None => {
                        // 0x4d79c6 is called with the null part 0x465f6c
                        // returns and reads its category (+0x240): the
                        // original faults there.
                        self.warn(format!(
                            "Kit item {nm} is not in the database; the original stops with a fault here (0x4d79c6 reads the missing part), so it was skipped."
                        ));
                    }
                }
            }
        }
    }

    /// 0x4d79c6(part, free)
    fn eq_file(&mut self, p: &Part, free: bool) {
        let nm = p.name();
        let mut cat = p.s(0x240);
        let t = self.d.part_type(nm);
        let mut chk = true;
        if t == 0 {
            cat = self.t(WEAPON);
            if free {
                chk = false;
            }
        }
        let file = |g: &mut Self, list| g.buy_to(nm, Dest::List(list), 1, free, chk);
        let d = self.d;
        if d.is(cat, &[CLOTHING, HOUSEHOLD, TACK, MISC]) {
            if !(p.weight() <= 20.0 && file(self, L380)) {
                file(self, L530);
            }
        } else if d.is(cat, &[ANIMAL, TRANSPORTATION_ANIMALS]) {
            file(self, L458);
        } else if cat == d.t(SERVICES) {
            file(self, L59C);
        } else if cat == d.t(FOOD) {
            file(self, L608);
        } else if d.is(
            cat,
            &[
                TRANSPORTATION,
                TRANSPORTATION_WHEELED,
                TRANSPORTATION_WATERCRAFT,
            ],
        ) {
            file(self, L674);
        } else if t == 0 {
            if !(p.weight() <= 5.0 && file(self, L380)) {
                file(self, L530);
            }
        } else {
            file(self, L530);
        }
    }

    /// 0x4c96c7
    fn eq_basics(&mut self) {
        let soc = self.social.clone();
        self.buy(self.t(WINESKIN), L380, 1, false);
        let rations = if soc == self.t(LOWER_CLASS) {
            RATIONS_STANDARD_1_WEEK
        } else if soc == self.t(MIDDLE_LOWER_CLASS) {
            RATIONS_IRON_1_WEEK
        } else {
            DRY_RATIONS_1_WEEK
        };
        self.buy(self.t(rations), L380, 1, true);
        self.buy(self.t(FLINT_AND_STEEL), L380, 1, false);
        if self.between(1, 10) < 2 && soc != self.t(LOWER_CLASS) {
            self.buy(self.t(LANTERN_HOODED), L380, 1, false);
            self.buy(self.t(OIL_LAMP_PER_FLASK), L380, 1, false);
        } else {
            let q = self.between(1, 6);
            self.buy(self.t(TORCH), L380, q, false);
        }
        if self.between(1, 10) < 4 {
            let q = self.between(1, 6);
            self.buy(self.t(CANDLE), L380, q, false);
        }
        for nm in [SIGNAL_WHISTLE, WINTER_BLANKET, OIL_GREEK_FIRE_FLASK] {
            if self.between(1, 7) < 2 {
                self.buy(self.t(nm), L380, 1, false);
            }
        }
    }

    /// 0x4cbc6e: gear for some proficiencies, then for three kits.
    fn eq_nwp_kit(&mut self) {
        let names: Vec<String> = self.nwp.iter().map(|n| n.0.clone()).collect();
        for nm in &names {
            if nm == self.t(DISGUISE) {
                self.buy(self.t(WEAPONBLACK), L380, 1, false);
                if self.between(1, 5) < 3 && self.social != self.t(UPPER_CLASS) {
                    self.buy(self.t(RUFF), L380, 1, false);
                }
            }
            for (k, list) in NWP_GEAR {
                if nm == self.t(k) {
                    for &(it, free) in list {
                        self.buy(self.t(it), L380, 1, free);
                    }
                }
            }
            if nm == self.t(ANIMAL_TRAINING) {
                let pet = match self.between(1, 6) {
                    2 => Some(DOG_GUARD),
                    3 => Some(DOG_WAR),
                    4 => Some(DOG_HUNTING),
                    5 => Some(PIGEON_HOMING),
                    _ => None,
                };
                if let Some(pet) = pet {
                    self.buy(self.t(pet), L458, 1, false);
                }
            }
            if nm == self.t(FALCONRY_CRE) {
                self.buy(self.t(FALCON_TRAINED), L458, 1, true);
            }
            if nm == self.t(RIDING_LAND) {
                let r = self.between(1, 5);
                if self.m738 > 1 && r < 3 {
                    self.eq.horse = self.t(HORSE_RIDING).into();
                }
            }
        }
        if self.has_kit_part() {
            let kit = self.kit_name.as_str();
            if kit == self.t(EXPLORER) {
                self.buy(self.t(SPYGLASS), L380, 1, false);
            } else if kit == self.t(JESTER) {
                self.buy(self.t(LIRIPIPE), L380, 1, false);
            } else if kit == self.t(SCHOLAR) {
                self.buy(self.t(WRITING_INK_PER_VIAL), L380, 1, false);
                self.buy(self.t(PAPER_PER_SHEET), L380, 1, false);
            }
        }
    }

    /// 0x4cb9ca (warriors), 0x4cc460 (wizards), 0x4cbb74 (priests).
    fn eq_group_gear(&mut self) {
        if self.has_group(0) {
            for nm in [BLOCK_AND_TACKLE, WHETSTONE, CASSOCK, GRAPPLING_HOOK] {
                if self.between(1, 5) < 3 {
                    self.buy(self.t(nm), L380, 1, false);
                }
            }
        }
        if self.has_group(3) && self.between(1, 5) < 3 {
            self.buy(self.t(GLASS_BOTTLE), L380, 1, false);
        }
        if self.has_group(2) {
            self.buy(self.t(HOLY_ITEM), L380, 1, false);
        }
    }

    /// 0x4ccec2: a magic cloak or robe with chance (highest level) in 40.
    fn eq_cloak(&mut self) -> String {
        let mx = self.char_level();
        let list = self.eq.magic[M31C].clone();
        let n = list.len() as i32;
        if n == 0 {
            return String::new();
        }
        let k = self.between(1, n) - 1;
        let mut s = String::new();
        if self.between(1, 40) < mx {
            s = list[k as usize].clone();
        }
        if s.contains(self.t(ROBE_OF_THE_ARCHMAGI)) {
            let a = self.alignment.as_str();
            let d = self.d;
            s = d
                .t(if d.is(a, &[LAWFUL_GOOD, NEUTRAL_GOOD, CHAOTIC_GOOD]) {
                    ROBE_OF_THE_ARCHMAGI_GOOD
                } else if d.is(a, &[TRUE_NEUTRAL, LAWFUL_NEUTRAL, CHAOTIC_NEUTRAL]) {
                    ROBE_OF_THE_ARCHMAGI_NEUTRAL
                } else if d.is(a, &[NEUTRAL_EVIL, LAWFUL_EVIL, CHAOTIC_EVIL]) {
                    ROBE_OF_THE_ARCHMAGI_EVIL
                } else {
                    CLOAK_OF_PROTECTION_1
                })
                .into();
        }
        s
    }

    /// The race (+0x200) wears clothes: +0x1e4 or +0x1e0.
    fn wears_clothes(&self) -> bool {
        self.race.unknown_0x1e0[1] != 0 || self.race.unknown_0x1e0[0] != 0
    }

    fn female(&self) -> bool {
        self.sex == 0
    }

    /// Clothes by social class (0x4c9a75, 0x4ca197, 0x4ca618).
    fn eq_social(&mut self) {
        let soc = self.social.clone();
        if soc == self.t(UPPER_CLASS) {
            {
                if !self.female() || self.between(1, 5) > 3 {
                    self.buy(self.t(SHIRT), L3EC, 1, true);
                    if self.between(1, 5) < 4 {
                        self.buy(self.t(BREECHES), L3EC, 1, true);
                    } else {
                        self.buy(self.t(CANNONS), L3EC, 1, true);
                    }
                } else {
                    self.buy(self.t(GARNACHE), L3EC, 1, true);
                }
                let s = self.eq_cloak();
                if s.is_empty() {
                    if self.between(1, 5) < 2 {
                        self.buy(self.t(FINE_FUR_CLOAK), L3EC, 1, false);
                    } else if self.between(1, 5) < 3 {
                        self.buy(self.t(EMBROIDERED_ROBE), L3EC, 1, false);
                    }
                } else {
                    self.buy(&s, L3EC, 1, false);
                }
                for (nm, die, list) in [
                    (SASH, 2, L380),
                    (GORGET, 3, L3EC),
                    (PIN, 3, L3EC),
                    (RUFF, 3, L3EC),
                    (FERRONNIERE, 3, L3EC),
                    (SIGNET_RING_PERSONAL_SEAL, 3, L380),
                    (PERFUME_PER_VIAL, 2, L380),
                ] {
                    if self.between(1, 5) < die {
                        self.buy(self.t(nm), list, 1, false);
                    }
                }
                self.buy(self.t(GIPSER), L3EC, 1, false);
            }
        } else if soc == self.t(MIDDLE_LOWER_CLASS) {
            {
                if !self.female() || self.between(1, 5) > 3 {
                    self.buy(self.t(SHIRT), L3EC, 1, true);
                    if self.between(1, 5) < 2 {
                        self.buy(self.t(BREECHES), L3EC, 1, true);
                    } else {
                        self.buy(self.t(BRAIES), L3EC, 1, true);
                    }
                } else {
                    self.buy(self.t(COMMON_GOWN), L3EC, 1, true);
                }
                let s = self.eq_cloak();
                if s.is_empty() {
                    if self.between(1, 5) < 2 {
                        self.buy(self.t(COMMON_ROBE), L3EC, 1, false);
                    }
                } else {
                    self.buy(&s, L3EC, 1, false);
                }
                if self.between(1, 5) < 4 {
                    self.buy(self.t(BELT_POUCH_SMALL), L3EC, 1, false);
                }
            }
        } else {
            {
                if self.between(1, 5) < 3 {
                    self.buy(self.t(SLOP), L3EC, 1, true);
                } else {
                    self.buy(self.t(CHAINSE_CHEMISE), L3EC, 1, true);
                }
                self.buy(self.t(BRAIES), L3EC, 1, true);
                let s = self.eq_cloak();
                if !s.is_empty() {
                    self.buy(&s, L3EC, 1, false);
                }
            }
        }
    }

    /// 0x4ca3f6 (every other social class).
    fn eq_social_default(&mut self) {
        if self.wears_clothes() {
            let s = self.eq_cloak();
            let cloak = if s.is_empty() {
                self.t(GOOD_CLOTH_CLOAK).to_owned()
            } else {
                s
            };
            self.buy(&cloak, L3EC, 1, false);
            if !self.female() || self.between(1, 5) > 3 {
                self.buy(self.t(SHIRT), L3EC, 1, true);
                self.buy(self.t(BREECHES), L3EC, 1, true);
            } else if self.between(1, 5) < 3 {
                self.buy(self.t(COMMON_ROBE), L3EC, 1, true);
            } else {
                self.buy(self.t(GOWN_COMMON), L3EC, 1, true);
            }
        }
        if self.between(1, 5) < 3 {
            self.buy(self.t(BELT_POUCH_SMALL), L3EC, 1, false);
        } else {
            self.buy(self.t(BELT_POUCH_LARGE), L3EC, 1, false);
        }
    }

    /// 0x4c953b: up to 5 picks of racial equipment until one is bought.
    fn eq_racial(&mut self) {
        let list = self.eq.race_eq.clone();
        let n = list.len() as i32;
        if n == 0 {
            return;
        }
        let (mut ok, mut tries) = (false, 0);
        while !ok && tries < 5 {
            tries += 1;
            let nm = &list[self.between(0, n - 1) as usize];
            if let Some(p) = self.d.find(nm) {
                let price = match p.n(0x20c) {
                    0 => p.n(0x1fc),
                    x => x,
                };
                if p.weight() < 15.0 && price < 250 {
                    ok = self.buy(nm, L380, 1, false);
                }
            }
        }
    }

    /// 0x4c61cf: eight magic item rolls.
    fn eq_magic(&mut self) {
        let mut lv = [0i32; 5];
        for (k, &l) in self.classes.iter().zip(&self.levels) {
            if let Some(x) = lv.get_mut(k.group as usize) {
                *x = l;
            }
        }
        let mx = self.char_level();
        let wiz = if lv[3] > 1 { lv[3] } else { 0 };
        let pw = if lv[2] > 1 { lv[2] } else { wiz };
        let fig = if lv[0] > 1 { lv[0] } else { 0 };
        let plan = [
            (M2A4, 20, pw),
            (M290, 30, pw),
            (M2B8, 40, pw),
            (M2CC, 40, wiz),
            (M2E0, 40, 0),
            (M2F4, 40, mx),
            (M330, 40, fig),
            (M344, 40, lv[1]),
        ];
        for (key, die, thr) in plan {
            let list = self.eq.magic[key].clone();
            let n = list.len() as i32;
            let mut k = 0;
            if n > 0 {
                k = self.between(1, n);
            }
            k -= 1;
            if n > 0 && self.between(1, die) < thr {
                let nm = &list[k as usize];
                let dest = if key == M330 || key == M344 {
                    if self.d.find(nm).is_some_and(|p| p.n(0x250) != 0) {
                        L3EC
                    } else {
                        L380
                    }
                } else {
                    L380
                };
                self.buy(nm, dest, 1, true);
            }
        }
    }

    /// 0x4d0bd6: thieves' picks, a bard's instrument, a paladin's mount.
    fn eq_class_items(&mut self) {
        let abilities: Vec<Vec<String>> = self
            .class_abilities
            .iter()
            .map(|g| g.iter().map(|p| p.name().to_owned()).collect())
            .collect();
        for abl in &abilities {
            for (j, nm) in abl.iter().enumerate() {
                if nm.contains(self.t(OPEN_LOCKS)) {
                    self.buy(self.t(THIEVES_PICKS), L380, 1, true);
                }
                if j == 0 && nm.contains(self.t(BARD)) {
                    let list = self.eq.magic[M308].clone();
                    let n = list.len() as i32;
                    let mut k = 0;
                    if n > 0 {
                        k = self.between(1, n);
                    }
                    if n > 0 {
                        if self.between(2, 40) < self.m738 {
                            self.buy(&list[(k - 1) as usize], L6C, 1, false);
                        } else {
                            self.buy(self.t(MUSICAL_INSTRUMENT_SMALL), L6C, 1, false);
                        }
                    }
                }
                if nm.contains(self.t(FAITHFUL_MOUNT)) {
                    let l0 = self.levels[0];
                    if l0 > 3 {
                        let lo = if l0 > 5 { 4 } else { 0 };
                        let r = self.between(lo, l0);
                        if r >= 11 {
                            self.eq.horse = self.t(HORSE_HEAVY_WAR).into();
                        } else if r >= 7 {
                            self.eq.horse = self.t(HORSE_MEDIUM_WAR).into();
                        } else if r >= 4 {
                            self.eq.horse = self.t(HORSE_LIGHT_WAR).into();
                        }
                    }
                }
            }
        }
    }

    /// 0x4cc4a8: the copper left becomes gp, sp and cp.
    fn eq_coins(&mut self) -> Vec<It> {
        let mut b = self.eq.budget;
        let mut gp = b.div_euclid(100);
        b -= gp * 100;
        let mut sp = b.div_euclid(10);
        let mut cp = b - sp * 10;
        for k in &self.classes {
            if k.i2798 != 0 && k.i2798 < gp {
                gp = k.i2798;
            }
        }
        if self.m738 < 2
            && let Some(kit) = self.kit()
            && kit.flag(0x5dc)
        {
            cp = 0;
            sp = 0;
            if kit.i(0x5e0) < gp {
                gp = kit.i(0x5e0);
            }
        }
        self.eq.budget = cp;
        self.coins = (gp, sp, cp);
        let mut out = Vec::new();
        for (name, n) in [(COPPER_PIECES, cp), (SILVER_PIECES, sp), (GOLD_PIECES, gp)] {
            if n != 0 {
                inv_add(&mut out, It::new(self.t(name), n), true);
            }
        }
        out
    }

    /// 0x4d663c: the horse (a pony for small races) with its tack.
    fn eq_mount(&mut self) {
        if self.eq.horse.is_empty() || self.d.is(&self.race.name, &[AARAKOCRA, CENTAUR, WEMIC]) {
            return;
        }
        if self
            .d
            .is(&self.race.name, &[HALFLING, GNOME, DWARF, KOBOLD])
        {
            self.eq.horse = self.t(PONY).into();
        }
        let mut b = Vec::new();
        for nm in [SADDLE_RIDING, BIT_AND_BRIDLE, SADDLE_BLANKET] {
            inv_add(&mut b, It::new(self.t(nm), 1), true);
        }
        let mut it = It::new(self.eq.horse.clone(), 1);
        it.bundle = Some(b);
        inv_add(&mut self.eq.lists[L458], it, true);
    }

    /// 0x4c7fa1
    pub fn st_equip(&mut self) {
        self.eq = Equipment {
            budget: self.budget,
            ..Equipment::default()
        };
        self.eq.l724 = self.wp_list_724();
        self.wp.l724 = self.eq.l724.clone();
        self.eq_categories();
        self.eq_weapons();
        self.eq_armor();
        let pack = self.eq_backpack();
        self.eq_kit_items();
        self.eq_basics();
        self.eq_nwp_kit();
        self.eq_group_gear();
        if self.wears_clothes() {
            if self.d.is(
                &self.social,
                &[UPPER_CLASS, MIDDLE_LOWER_CLASS, LOWER_CLASS],
            ) {
                self.eq_social();
            } else {
                self.eq_social_default();
            }
        }
        self.eq_racial();
        self.eq_magic();
        self.eq_class_items();
        if let Some(mut pack) = pack {
            pack.bundle = Some(self.eq.lists[L380].clone());
            inv_add(&mut self.eq.lists[L3EC], pack, true);
        }
        let coins = self.eq_coins();
        self.eq_mount();
        self.carried = self.eq.w;
        let mut coins = Some(coins);
        self.inventory = INV_ORDER
            .iter()
            .map(|l| match l {
                Some(l) => self.eq.lists[*l].clone(),
                None => coins.take().unwrap_or_default(),
            })
            .collect();
    }
}
