//! Spells (0x4cc683 → `CNPCGenerationSpells` 0x4dd930) and psionics
//! (0x4cce66 → `CNPCGenerationPsionics` 0x4dc5d0), Core Rules paths.

use std::collections::HashMap;

use super::Gen;
use super::data::{EQUIPMENT, PartExt, SCHOOLS, SPHERES, Spell};
use super::state::{Book, It, Psionics, inv_add};

/// 0x421f1f: the highest spell level by Intelligence.
const INT_MAX_SPELL_LEVEL: [i32; 26] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9,
];
/// 0x42205e: the most spells per level by Intelligence.
const INT_MAX_SPELLS: [i32; 26] = [
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    6,
    7,
    7,
    7,
    9,
    9,
    11,
    11,
    14,
    18,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
    i32::MAX,
];

pub const DISCIPLINES: [&str; 5] = [
    "Clairsentient",
    "Psychokinetic",
    "Psychometabolic",
    "Psychoportive",
    "Telepathic",
];
/// 0x42fa1a: devotions by level (without and with the bonus).
const PSI_DEV: [[i32; 21]; 2] = [
    [
        0, 3, 5, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    ],
    [
        0, 4, 6, 8, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    ],
];
/// 0x42f835: sciences by level.
const PSI_SCI: [[i32; 21]; 2] = [
    [
        0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    ],
    [
        0, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11,
    ],
];
/// 0x42f6ea: disciplines by level.
const PSI_DISC: [[i32; 16]; 2] = [
    [0, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5],
    [0, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5],
];

fn psi_dev(l: i32, b: bool) -> i32 {
    if l < 21 {
        PSI_DEV[usize::from(b)][l.max(0) as usize]
    } else if b {
        26
    } else {
        l + 5
    }
}

fn psi_sci(l: i32, b: bool) -> i32 {
    if l < 21 {
        PSI_SCI[usize::from(b)][l.max(0) as usize]
    } else if b {
        11
    } else {
        (l - 19) / 2 + 10
    }
}

fn psi_disc(l: i32, b: bool) -> i32 {
    if l < 16 {
        PSI_DISC[usize::from(b)][l.max(0) as usize]
    } else {
        5
    }
}

/// 0x422fe9 and friends: 16 gives 1, 25 gives 10.
fn bonus16(v: i32) -> i32 {
    (v - 15).max(0)
}

/// 0x4def99: the last index with that name.
fn rfind(list: &[&Spell], name: &str) -> Option<usize> {
    list.iter().rposition(|s| s.name == name)
}

/// 0x46dd2d / 0x46e061
fn find_spell<'d>(list: &'d [Spell], name: &str) -> Option<&'d Spell> {
    list.iter().find(|s| s.name == name)
}

fn has_any(names: &[String], list: &[String]) -> bool {
    names.iter().any(|x| list.contains(x))
}

impl<'a> Gen<'a> {
    /// Int (Reason) with the kit's +0x1f8, clamped 1..25.
    fn int_adj(&self) -> i32 {
        let mut v = self.reason();
        let k = self.kit().map_or(0, |k| k.i(0x1f8));
        if k != 0 {
            v = (v + k).clamp(1, 25);
        }
        v
    }

    /// 0x421f1f, then the kit's +0x1f4.
    fn max_spell_level(&self) -> i32 {
        let m = INT_MAX_SPELL_LEVEL[self.int_adj().clamp(0, 25) as usize];
        m.min(self.kit().map_or(99, |k| k.i_or(0x1f4, 99)))
    }

    /// 0x431639: the kit's spell table (+0x648) with kit +0x5da, else the
    /// class table +0x414 (+0x159c with a class ability +0x784).
    fn spell_slots(&mut self, g: usize, sl: i32) -> i32 {
        let k = self.classes[g];
        let lv = self.levels[g];
        // With +0x5da, the kit's table for the class level (no Wisdom
        // bonus).
        if let Some(kit) = self.kit()
            && kit.flag(0x5da)
            && lv > 0
            && let Some(Some(row)) = kit.spell_table.get(lv as usize - 1)
        {
            return row[(sl - 1).rem_euclid(10) as usize];
        }
        let alt = self.class_abilities[g].iter().any(|p| p.n(0x784) != 0);
        let table = if alt { &k.alt_slots } else { &k.slots };
        table
            .get(lv.max(0) as usize)
            .and_then(|row| row.get(sl as usize))
            .copied()
            .unwrap_or(0)
    }

    /// 0x431862: one more spell of the specialist's school.
    fn spec_bonus(&mut self, g: usize, sl: i32) -> i32 {
        if self.classes[g].school.is_empty() {
            return 0;
        }
        if self.class_abilities[g].iter().any(|p| p.n(0x7ac) != 0) {
            return i32::from(self.spell_slots(g, sl) != 0);
        }
        0
    }

    /// Fallback of 0x4792d3: a school or sphere part's spell list +0x748.
    fn part_lists(&mut self, t: usize, name: &str) -> bool {
        let d = self.d;
        if let Some(i) = d.part_exact(t, name)
            && !d.parts[t][i].data.unknown_0x748.is_empty()
        {
            self.warn("School and sphere spell lists (+0x748, 0x4792d3) are not decoded and were ignored.");
        }
        false
    }

    fn any_listed(&mut self, t: usize, names: &[String], spell_list: &[String]) -> bool {
        has_any(names, spell_list) || names.iter().any(|x| self.part_lists(t, x))
    }

    /// 0x4792d3(out, W, P, schools, minor, major, lvl, 0); lvl 0 = any.
    fn spell_list(
        &mut self,
        schools: &[String],
        minor: &[String],
        major: &[String],
        lvl: i32,
    ) -> Vec<&'a Spell> {
        let d = self.d;
        let mut out: Vec<&'a Spell> = Vec::new();
        if !schools.is_empty() {
            for s in &d.wizard_spells {
                if lvl != 0 && s.level != lvl {
                    continue;
                }
                if self.any_listed(SCHOOLS, schools, &s.schools) {
                    out.push(s);
                }
            }
        }
        let mut rest: Vec<&'a Spell> = Vec::new();
        if lvl < 4 {
            if minor.is_empty() {
                rest.extend(d.priest_spells.iter().filter(|s| s.level < 4));
            } else {
                for s in &d.priest_spells {
                    if (lvl == 0 || s.level == lvl) && s.level < 4 {
                        if self.any_listed(SPHERES, minor, &s.spheres) {
                            out.push(s);
                        } else {
                            rest.push(s);
                        }
                    }
                }
            }
        }
        if lvl == 0 {
            rest.extend(d.priest_spells.iter().filter(|s| s.level > 3));
        }
        let src: Vec<&'a Spell> = if lvl < 4 {
            rest
        } else {
            d.priest_spells.iter().collect()
        };
        if !major.is_empty() {
            for s in src {
                if out.iter().any(|x| std::ptr::eq(*x, s)) {
                    continue;
                }
                if lvl != 0 && s.level != lvl {
                    continue;
                }
                if self.any_listed(SPHERES, major, &s.spheres) {
                    out.push(s);
                }
            }
        }
        out
    }

    /// 0x4df3ec(schools, minor, major): the kit's edits. Its lists +0x230
    /// and +0x208 are filtered in place in the shared kit, so the edits
    /// outlive this NPC: they are kept in the session.
    fn kit_spell_edits(&mut self, g: usize) {
        let (Some(kit), Some(idx)) = (self.kit(), self.kit_idx) else {
            return;
        };
        let mut s = std::mem::take(&mut self.schools[g]);
        let mut mi = std::mem::take(&mut self.minor[g]);
        let mut ma = std::mem::take(&mut self.major[g]);
        for n in kit.sa(0x1dc) {
            if !n.is_empty() {
                s.retain(|x| x != n);
            }
        }
        if kit.flag(0x1fc) {
            while s.len() > 5 {
                let r = self.between(0, s.len() as i32 - 1) as usize;
                s.remove(if r < s.len() { r } else { 0 });
            }
        }
        let allow = kit.sa(0x21c);
        let filt = |l: &mut Vec<String>| {
            if !allow.is_empty() {
                l.retain(|x| allow.contains(x));
            }
        };
        filt(&mut mi);
        filt(&mut ma);
        let (mut l230, mut l208) = self
            .session
            .kit_lists(idx)
            .unwrap_or_else(|| (kit.sa(0x230).to_vec(), kit.sa(0x208).to_vec()));
        filt(&mut l230);
        for n in &l230 {
            if !mi.contains(n) {
                mi.push(n.clone());
            }
        }
        filt(&mut l208);
        l208.retain(|x| !ma.contains(x));
        if !l208.is_empty() {
            let r = self.between(0, l208.len() as i32 - 1) as usize;
            ma.push(l208[r].clone());
        }
        self.session.set_kit_lists(idx, (l230, l208));
        for (list, off) in [(&mut mi, 0x200), (&mut ma, 0x204)] {
            let n = kit.i(off);
            while n > 0 && !list.is_empty() && (n as usize) < list.len() {
                let r = self.between(0, list.len() as i32 - 1) as usize;
                list.remove(if r < list.len() { r } else { 0 });
            }
        }
        self.schools[g] = s;
        self.minor[g] = mi;
        self.major[g] = ma;
    }

    /// 0x4ddb05: memorized priest spells, drawn without replacement.
    fn priest_spells(&mut self, g: usize) {
        self.kit_spell_edits(g);
        for sl in 1..=self.max_spell_level() {
            let (s, mi, ma) = (
                self.schools[g].clone(),
                self.minor[g].clone(),
                self.major[g].clone(),
            );
            let mut list = self.spell_list(&s, &mi, &ma, sl);
            let slot = sl as usize;
            if let Some(x) = self.spells[g].get_mut(slot) {
                *x = Default::default();
            }
            if list.is_empty() {
                continue;
            }
            for _ in 0..self.spell_slots(g, sl) {
                if list.is_empty() {
                    // The original reads a stale pointer here.
                    self.warn("The priest spell list ran out while memorizing (stale read in the original); the rest were skipped.");
                    break;
                }
                let k = self.between(0, list.len() as i32 - 1) as usize;
                let s = list.remove(k);
                if let Some(x) = self.spells[g].get_mut(slot) {
                    x.1.push(s.name.clone());
                }
            }
        }
    }

    /// 0x43129d (+0x3a0) / 0x4312be (+0x3b4).
    fn learn(&mut self, s: &Spell) {
        if s.wizard {
            self.book_w.push(s.name.clone());
        } else {
            self.book_p.push(s.name.clone());
        }
    }

    /// 0x4ddbfa: the spellbook, level by level.
    fn wizard_learn(&mut self, g: usize) {
        let spec = self.classes[g].school.clone();
        let maxl = self.max_spell_level();
        self.kit_spell_edits(g);
        for sl in 1..=maxl {
            if self.spell_slots(g, sl) == 0 {
                continue;
            }
            let (s, mi, ma) = (
                self.schools[g].clone(),
                self.minor[g].clone(),
                self.major[g].clone(),
            );
            let mut general = self.spell_list(&s, &mi, &ma, sl);
            let mut sp = if spec.is_empty() {
                Vec::new()
            } else {
                self.spell_list(std::slice::from_ref(&spec), &[], &[], sl)
            };
            let mut n = i64::from(INT_MAX_SPELLS[self.int_adj().clamp(0, 25) as usize]);
            let r = self.between(70, 100);
            let f = f64::from((f64::from(r) / 100.0) as f32);
            let x = (n as f64 * f) as i64;
            if x > 4 {
                n = x;
            }
            let mut n = n.min(20) as i32;
            if n < 1 {
                continue;
            }
            let f2 = f64::from((1.0 - f64::from(sl - 1) * 0.05) as f32);
            let y = (f64::from(n) * f2) as i32;
            if y > 2 {
                n = y;
            }
            if sl == 1 {
                n -= 1;
                self.book_w.push("Read Magic".into());
                for l in [&mut sp, &mut general] {
                    if let Some(i) = rfind(l, "Read Magic") {
                        l.remove(i);
                    }
                }
            }
            if n < 1 {
                continue;
            }
            let mut ns = 0;
            if !spec.is_empty() {
                let mut i = 0;
                while i < n {
                    if self.between(1, 100) < 25 {
                        ns += 1;
                        n -= 1;
                    }
                    i += 1;
                }
            }
            if (sp.len() as i32) < ns {
                n += sp.len() as i32 - ns;
                ns = sp.len() as i32;
            }
            while n > 0 || ns > 0 {
                if n > 0 {
                    if general.is_empty() {
                        return;
                    }
                    let k = (self.between(1, general.len() as i32) - 1) as usize;
                    let s = general.remove(k);
                    self.learn(s);
                    if !spec.is_empty() {
                        match rfind(&sp, &s.name) {
                            None => n -= 1,
                            Some(i) => {
                                if ns > 0 {
                                    ns -= 1;
                                } else {
                                    n -= 1;
                                }
                                sp.remove(i);
                            }
                        }
                    } else {
                        n -= 1;
                    }
                } else {
                    if sp.is_empty() {
                        return;
                    }
                    let k = (self.between(1, sp.len() as i32) - 1) as usize;
                    let s = sp.remove(k);
                    self.learn(s);
                    ns -= 1;
                }
            }
        }
    }

    /// 0x4de905: memorized wizard spells from the spellbook, drawn with
    /// replacement.
    fn wizard_memorize(&mut self, g: usize) {
        let d = self.d;
        let spec = self.classes[g].school.clone();
        for sl in 1..=self.max_spell_level() {
            let n = self.spell_slots(g, sl);
            let extra = self.spec_bonus(g, sl);
            if n == 0 && extra == 0 {
                continue;
            }
            let mut cand: Vec<&Spell> = Vec::new();
            for (book, db, times) in [
                (&self.book_w, &d.wizard_spells, 2),
                (&self.book_p, &d.priest_spells, 1),
            ] {
                for _ in 0..times {
                    for nm in book {
                        if let Some(s) = find_spell(db, nm)
                            && s.level == sl
                        {
                            cand.push(s);
                        }
                    }
                }
            }
            let spc: Vec<&Spell> = cand
                .iter()
                .copied()
                .filter(|s| !spec.is_empty() && s.schools.contains(&spec))
                .collect();
            let slot = sl as usize;
            if let Some(x) = self.spells[g].get_mut(slot) {
                *x = Default::default();
            }
            if cand.is_empty() {
                return;
            }
            let gi = g;
            let mem = |g: &mut Self, s: &Spell| {
                if let Some(x) = g.spells[gi].get_mut(slot) {
                    if s.wizard {
                        x.0.push(s.name.clone());
                    } else {
                        x.1.push(s.name.clone());
                    }
                }
            };
            for _ in 0..n {
                let k = (self.between(1, cand.len() as i32) - 1) as usize;
                mem(self, cand[k]);
            }
            if !spc.is_empty() {
                for _ in 0..extra {
                    let k = (self.between(1, spc.len() as i32) - 1) as usize;
                    mem(self, spc[k]);
                }
            }
        }
    }

    /// 0x4dede7: the wizard spells, then the priest spells, into new
    /// "Spellbook" items (list 4); a spell takes level + 0..5 pages.
    fn spellbooks(&mut self) {
        let d = self.d;
        let Some(i) = d.part_exact(EQUIPMENT, "Spellbook") else {
            self.warn("There is no Spellbook part; no spellbook was made.");
            return;
        };
        let cap = d.parts[EQUIPMENT][i].n(0x7e4);
        for wiz in [true, false] {
            let mut books = vec![Book {
                capacity: cap,
                ..Book::default()
            }];
            let names = if wiz {
                self.book_w.clone()
            } else {
                self.book_p.clone()
            };
            let db = if wiz {
                &d.wizard_spells
            } else {
                &d.priest_spells
            };
            for nm in names {
                let Some(s) = find_spell(db, &nm) else {
                    self.warn(format!("The spell {nm} is not in the spell database and was left out of the spellbook."));
                    continue;
                };
                let pages = self.between(0, 5) + s.level;
                let mut bk = books.last_mut().expect("one book");
                if pages + bk.used > bk.capacity {
                    books.push(Book {
                        capacity: cap,
                        ..Book::default()
                    });
                    bk = books.last_mut().expect("one book");
                    if pages + bk.used > bk.capacity {
                        continue;
                    }
                }
                bk.used += pages;
                bk.spells.push(nm);
                bk.pages.push(pages);
                bk.wizard.push(wiz);
            }
            for b in books {
                let mut it = It::new("Spellbook", 1);
                it.book = Some(b);
                inv_add(&mut self.inventory[4], it, false);
            }
        }
    }

    /// 0x4cc683
    pub fn st_spells(&mut self) {
        if self.schools.is_empty() {
            self.schools = self.classes.iter().map(|k| k.sa(0x2760).to_vec()).collect();
            self.minor = self.classes.iter().map(|k| k.sa(0x274c).to_vec()).collect();
            self.major = self.classes.iter().map(|k| k.sa(0x2738).to_vec()).collect();
        }
        self.spells = vec![vec![Default::default(); 11]; self.classes.len()];
        for g in 0..self.classes.len() {
            let k = self.classes[g];
            if k.priest_caster {
                self.priest_spells(g);
            } else if k.wizard_caster {
                self.wizard_learn(g);
                self.wizard_memorize(g);
                self.spellbooks();
            }
        }
    }

    // ---- psionics ----

    /// 0x42e916(level), Core path.
    fn psp_gain(&mut self, l: i32) {
        let d6 = self.between(1, 6);
        let wis = bonus16(self.intuition());
        let int = bonus16(self.reason());
        let con = bonus16(self.fitness());
        let v = if l == 1 {
            con + int + d6 + 15 + wis
        } else if l < 9 {
            con + int + d6 + wis
        } else {
            wis + 3
        };
        if let Some(x) = self.psp.get_mut(l as usize) {
            *x = v;
        }
    }

    /// 0x4dcf96: the powers that may be learned next.
    fn psi_candidates(&self, g: usize, dev: bool, cnt: &HashMap<String, i32>) -> Vec<usize> {
        let l = self.levels[g];
        let p = &self.psi;
        let mut out = Vec::new();
        for (i, pw) in self.d.powers.iter().enumerate() {
            if pw.devotion != dev || pw.level > l {
                continue;
            }
            let known = if dev { &p.devotions } else { &p.sciences };
            if known.contains(&pw.name) {
                continue;
            }
            let dn = usize::try_from(pw.discipline)
                .ok()
                .and_then(|i| DISCIPLINES.get(i))
                .copied()
                .unwrap_or("");
            if !p.disciplines.iter().any(|x| x == dn) {
                continue;
            }
            if !pw
                .prerequisites
                .iter()
                .all(|x| p.devotions.contains(x) || p.sciences.contains(x))
            {
                continue;
            }
            if dn != p.primary && dn != p.secondary {
                let mut lim = cnt.get(&p.primary).copied().unwrap_or(0);
                if !p.secondary.is_empty() {
                    lim = lim.min(cnt.get(&p.secondary).copied().unwrap_or(0));
                }
                if cnt.get(dn).copied().unwrap_or(0) + 1 >= lim {
                    continue;
                }
            }
            out.push(i);
        }
        out
    }

    /// 0x4dc7ec: true when a candidate list ran dry.
    fn psi_attempt(&mut self, g: usize) -> bool {
        let bonus = self.class_ability_any(|p| p.n(0x785) != 0);
        self.psi = Psionics {
            primary: std::mem::take(&mut self.psi.primary),
            secondary: std::mem::take(&mut self.psi.secondary),
            ..Psionics::default()
        };
        let mut pool: Vec<&str> = DISCIPLINES.to_vec();
        let mut cnt: HashMap<String, i32> = HashMap::new();
        let (mut nd, mut ns, mut nv) = (0, 0, 0);
        let mut fail = false;
        for l in 1..=self.levels[g] {
            self.psp_gain(l);
            for _ in 0..psi_disc(l, bonus) - nd {
                if pool.is_empty() {
                    break;
                }
                let k = (self.between(1, pool.len() as i32) - 1) as usize;
                let name = pool.remove(k).to_owned();
                self.psi.disciplines.push(name.clone());
                nd += 1;
                if nd == 1 {
                    self.psi.primary = name.clone();
                    self.psi.secondary.clear();
                }
                if (nd == 1 || nd == 2) && bonus {
                    self.psi.secondary = name;
                }
            }
            let need = [
                (true, psi_dev(l, bonus) - nv),
                (false, psi_sci(l, bonus) - ns),
            ];
            for (kind, need) in need {
                for _ in 0..need {
                    let cand = self.psi_candidates(g, kind, &cnt);
                    if cand.is_empty() {
                        fail = true;
                        break;
                    }
                    let k = (self.between(1, cand.len() as i32) - 1) as usize;
                    let pw = &self.d.powers[cand[k]];
                    let dn = DISCIPLINES
                        .get(pw.discipline.max(0) as usize)
                        .copied()
                        .unwrap_or("");
                    *cnt.entry(dn.to_owned()).or_insert(0) += 1;
                    if kind {
                        self.psi.devotions.push(pw.name.clone());
                        nv += 1;
                    } else {
                        self.psi.sciences.push(pw.name.clone());
                        ns += 1;
                    }
                }
            }
        }
        fail
    }

    /// 0x4dc65b: each psionicist class retries 0x4dc7ec until no list
    /// runs dry (10 tries).
    pub fn st_psionics(&mut self) {
        self.psp = vec![0; 101];
        for g in 0..self.classes.len() {
            if self.classes[g].group != 4 {
                continue;
            }
            let mut tries = 0;
            while self.psi_attempt(g) {
                tries += 1;
                if tries > 9 {
                    break;
                }
            }
        }
    }
}
