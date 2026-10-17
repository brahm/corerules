//! The Expansion's Non-Weapon Proficiencies box: the list (0x5fae3a), its
//! order, and each proficiency's score (0x438ca1) with the slot cost it
//! is measured against (0x43776b).

use cr_formats::Object;
use cr_formats::classes::KitBonus;
use cr_formats::parts::{Part, PartKitData};

use super::{Ctx, rogue};
use crate::npc::data::{Kit, Klass, PartExt};

/// One row of the box: the name, the score and the languages learned
/// with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Row {
    pub name: String,
    pub score: String,
    pub extra: String,
}

/// What the cost and score need beyond the sheet context: the classes
/// and the kit as the rules see them.
struct Rules<'a, 'c> {
    ctx: &'c Ctx<'a>,
    klasses: Vec<Klass>,
    /// Per class: the per-level bonus objects (+0x3514, +0x3528).
    level_lists: Vec<[Vec<&'a KitBonus>; 2]>,
    kit_data: Option<&'a PartKitData>,
    kit: Option<Kit>,
}

fn bonuses(list: &[Object]) -> Vec<&KitBonus> {
    list.iter()
        .filter_map(|o| match o {
            Object::KitBonus(b) => Some(b),
            _ => None,
        })
        .collect()
}

/// `CPartIDOb` entries: name and number.
fn part_ids(list: Option<&Vec<Object>>) -> impl Iterator<Item = (&str, i32)> {
    list.into_iter().flatten().filter_map(|o| match o {
        Object::PartId(p) => Some((p.name.as_str(), p.part_type)),
        _ => None,
    })
}

impl<'a, 'c> Rules<'a, 'c> {
    fn new(ctx: &'c Ctx<'a>) -> Self {
        let mut klasses = Vec::new();
        let mut level_lists = Vec::new();
        for c in &ctx.classes {
            klasses.push(c.class.map(Klass::of).unwrap_or_default());
            level_lists.push(match c.class {
                Some(cr_formats::classes::Class::Expansion { data, .. }) => {
                    [bonuses(&data.unknown_0x3514), bonuses(&data.kit_bonuses)]
                }
                _ => [Vec::new(), Vec::new()],
            });
        }
        Self {
            ctx,
            klasses,
            level_lists,
            kit_data: ctx.kit_data,
            kit: ctx.kit.clone(),
        }
    }

    fn core(&self) -> bool {
        !self.ctx.sp
    }

    fn cflag(&self, off: u32) -> bool {
        self.klasses.iter().any(|k| k.flag(off))
    }

    fn level(&self, i: usize) -> i32 {
        self.ctx.classes[i].level as i32
    }

    /// Some class ability of any class has `f`.
    fn class_abilities(&self) -> impl Iterator<Item = (usize, &'a Part)> + '_ {
        (0..self.ctx.classes.len())
            .flat_map(|i| self.ctx.class_abilities(i).into_iter().map(move |p| (i, p)))
    }

    /// 0x4388ba: a subability by its "Ability/Subability" name; 0 when
    /// empty, 111 when unknown.
    fn subability(&self, name: &str) -> i32 {
        if name.is_empty() {
            return 0;
        }
        const NAMES: [&str; 12] = [
            "Strength/Stamina",
            "Strength/Muscle",
            "Dexterity/Aim",
            "Dexterity/Balance",
            "Constitution/Health",
            "Constitution/Fitness",
            "Intelligence/Reason",
            "Intelligence/Knowledge",
            "Wisdom/Intuition",
            "Wisdom/Willpower",
            "Charisma/Leadership",
            "Charisma/Appearance",
        ];
        match NAMES.iter().position(|n| *n == name) {
            Some(i) => self.ctx.scores.sub(i / 2, i % 2 + 1),
            None => 111,
        }
    }

    /// 0x43776b: the slots a proficiency costs this character.
    fn cost(&self, p: &Part) -> i32 {
        let name = p.name();
        let core = self.core();
        let mut cost = if core { p.n(0x47c) } else { p.n(0x478) };
        let kit = self.kit.as_ref();
        let kf = |o: u32| kit.is_some_and(|k| k.flag(o));
        let kw = kf(0x30f) || self.cflag(0x3044);
        let kr = kf(0x312) || self.cflag(0x3045);
        let kp = kf(0x310) || self.cflag(0x3046);
        let kz = kf(0x311) || self.cflag(0x3047);
        let kpsi = kf(0x313) || self.cflag(0x3048);
        let [fs, fd, fc, fi, fw, fch] =
            [0x3049, 0x304a, 0x304b, 0x304c, 0x304d, 0x304e].map(|o| self.cflag(o));
        let g = |x: i32| self.ctx.has_group(x);
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
        if !core {
            if kit.is_some_and(|k| k.sa(0x258).iter().any(|n| n == name)) {
                cost -= 1;
            }
            for sub in [p.s(0x458), p.s(0x45c)] {
                let v = match self.subability(sub) {
                    0 => 9,
                    v => v,
                };
                if v < 9 {
                    cost += 9 - v;
                }
            }
        }
        if self.class_abilities().any(|(_, ca)| ca.s(0x718) == name) {
            cost = 0;
        }
        if let Some(kit) = kit {
            if cost > 0 && kit.s(0xa4) == name {
                cost = 0;
            }
            if cost > 0 && self.ctx.ch.data.unknown_0x1e20 == name {
                cost = 0;
            }
            if cost > 0 {
                // 0x4381c8: the kit's proficiencies up to the highest
                // class level.
                let top = (0..self.ctx.classes.len())
                    .map(|i| self.level(i))
                    .max()
                    .unwrap_or(0);
                if part_ids(self.kit_data.and_then(|d| d.unknown_0x2f8.as_ref()))
                    .any(|(n, level)| n == name && level <= top)
                {
                    cost = 0;
                }
            }
            if cost > 0 && self.granted(p) {
                cost = 0;
            }
        }
        cost.max(0)
    }

    /// 0x440ddf: proficiencies the classes, the character's per-level
    /// choices and the kit grant.
    fn granted(&self, p: &Part) -> bool {
        let name = p.name();
        let d = &self.ctx.ch.data;
        let chosen = d.unknown_0x1930.as_ref().map_or(&[][..], |a| &a[1][..]);
        for (i, k) in self.klasses.iter().enumerate() {
            if k.sa(0x2fa0).iter().any(|n| n == name) || chosen.iter().any(|n| n == name) {
                return true;
            }
            let level = self.level(i);
            // 0x6684f0: free entries (value 0) up to the class level.
            let free = self.level_lists[i][0]
                .iter()
                .any(|b| b.unknown_0xc == 0 && b.unknown_0x8 <= level && b.name == name);
            if free && (!self.core() || p.n(0x75c) == 0) {
                return true;
            }
            // 0x443af3: the proficiency chosen for this class at each
            // level (+0x1e28).
            for lvl in 1..=level {
                let first = d
                    .unknown_0x1e28
                    .iter()
                    .flatten()
                    .filter_map(|o| match o {
                        Object::KitBonus(b) => Some(b),
                        _ => None,
                    })
                    .find(|b| b.unknown_0x8 == lvl && b.unknown_0xc == i as i32);
                if first.is_some_and(|b| b.name == name) {
                    return true;
                }
            }
        }
        self.kit.as_ref().is_some_and(|k| k.s(0xa4) == name)
    }

    /// 0x438ca1: a proficiency's score, given the slots spent on it.
    fn score(&self, p: &Part, spent: i32) -> i32 {
        let ctx = self.ctx;
        let d = &ctx.ch.data;
        let name = p.name();
        let extra = spent - self.cost(p);
        let mut total = if self.core() {
            let ability = match p.s(0x460) {
                "" => 0,
                "Strength" => ctx.scores.score(super::STR),
                "Dexterity" => ctx.scores.score(super::DEX),
                "Constitution" => ctx.scores.score(super::CON),
                "Wisdom" => ctx.scores.score(super::WIS),
                "Intelligence" => ctx.scores.score(super::INT),
                "Charisma" => ctx.scores.score(super::CHA),
                _ => 101,
            };
            ability + p.n(0x484) + extra
        } else {
            let a = self.subability(p.s(0x458));
            let b = self.subability(p.s(0x45c));
            let best = if name == "Blind Fighting" {
                b
            } else {
                a.max(b)
            };
            let mut t = p.n(0x480) + rogue::subability_bonus(best) + extra;
            if p.n(0x474) != 0 {
                let n = d
                    .nonweapon_proficiencies
                    .iter()
                    .filter(|n| *n == p.s(0x464))
                    .count() as i32;
                t += n * p.n(0x474);
            }
            if p.n(0x488) != 0 {
                let n = d.unknown_0x1830[0]
                    .iter()
                    .filter(|n| *n == p.s(0x468))
                    .count() as i32;
                t += n * p.n(0x488);
            }
            let base = name.find('(').map_or(name, |i| &name[..i]);
            if let Some(ra) = ctx
                .racial_abilities()
                .into_iter()
                .find(|ra| ra.s(0x57c) == base)
            {
                t += ra.n(0x530);
            }
            t
        };
        if let Some(kd) = self.kit_data {
            // 0x671119: the kit's proficiency bonuses, by name ignoring
            // case and surrounding blanks.
            let key = name.trim().to_uppercase();
            if let Some((_, v)) =
                part_ids(kd.unknown_0x244.as_ref()).find(|(n, _)| n.trim().to_uppercase() == key)
            {
                total += v;
            }
            // 0x671237: a bonus when the character also has the kit's
            // companion proficiency.
            let kit = self.kit.as_ref().expect("kit data has a kit");
            if kit.s(0x328) == name {
                let other = kit.s(0x324);
                if d.nonweapon_proficiencies.iter().any(|n| n == other) {
                    total += kit.i(0x32c);
                }
            }
        }
        if d.unknown_0xac[5].iter().any(|n| n == name) {
            total += 1;
        }
        if d.unknown_0x1958.iter().flatten().any(|n| n == name) {
            total += 2;
        }
        for (i, ca) in self.class_abilities() {
            if ca.n(0x71c) != 0 && ca.s(0x718) == name {
                total += self.level(i) / 3;
            }
        }
        for (i, lists) in self.level_lists.iter().enumerate() {
            let level = self.level(i);
            for lvl in 1..=level {
                for b in &lists[1] {
                    if b.unknown_0x8 == lvl && b.name == name {
                        total += b.unknown_0xc;
                    }
                }
            }
        }
        total
    }
}

/// Languages as the box joins them: ", " between, each "," made " -".
fn join_languages(list: &[String]) -> String {
    list.iter()
        .map(|l| l.replace(',', " -"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The box's rows in the original's order, and whether they are the
/// secondary skills instead.
pub(super) fn rows(ctx: &Ctx) -> (Vec<Row>, bool) {
    let d = &ctx.ch.data;
    let mut rows = Vec::new();
    let secondary = d.unknown_0x1dac != 0;
    if secondary {
        for name in &d.unknown_0x1830[2] {
            rows.push(Row {
                name: name.clone(),
                score: String::new(),
                extra: String::new(),
            });
        }
    } else {
        let rules = Rules::new(ctx);
        for (i, name) in d.nonweapon_proficiencies.iter().enumerate() {
            let spent = d.nonweapon_costs.get(i).copied().unwrap_or(0) as i32;
            let bonus = d
                .nonweapon_bonuses
                .as_ref()
                .and_then(|b| b.get(i))
                .copied()
                .unwrap_or(0) as i32;
            let score = ctx.part(name).map_or(0, |p| rules.score(p, spent)) + bonus;
            let extra = match d.nonweapon_details.get(i) {
                Some(Object::StringArray(list)) => join_languages(list),
                _ => String::new(),
            };
            rows.push(Row {
                name: name.clone(),
                score: score.to_string(),
                extra,
            });
        }
    }
    if !d.languages.is_empty() {
        rows.push(Row {
            name: "Native Languages".into(),
            score: String::new(),
            extra: join_languages(&d.languages),
        });
    }
    sort(&mut rows);
    (rows, secondary)
}

/// The box's bubble sort, as written: rows without languages by name,
/// then rows with languages by name. Once a pass has swapped, it swaps
/// every later pair too.
fn sort(rows: &mut [Row]) {
    let n = rows.len();
    for _ in 0..100_000 {
        let mut swapped = false;
        for i in 0..n.saturating_sub(1) {
            let (a, b) = (&rows[i], &rows[i + 1]);
            if !swapped {
                swapped = match (a.extra.is_empty(), b.extra.is_empty()) {
                    (true, true) | (false, false) => a.name > b.name,
                    (false, true) => true,
                    (true, false) => false,
                };
            }
            if swapped {
                rows.swap(i, i + 1);
            }
        }
        if !swapped {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, extra: &str) -> Row {
        Row {
            name: name.into(),
            score: String::new(),
            extra: extra.into(),
        }
    }

    fn names(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r.name.as_str()).collect()
    }

    #[test]
    fn sorts_plain_rows_first() {
        let mut rows = vec![
            row("Native Languages", "Common"),
            row("Swimming", ""),
            row("Ancient History", ""),
            row("Modern Languages", "Elf"),
        ];
        sort(&mut rows);
        assert_eq!(
            names(&rows),
            [
                "Ancient History",
                "Swimming",
                "Modern Languages",
                "Native Languages"
            ]
        );
    }

    #[test]
    fn joins_languages() {
        let list = vec!["Elf, high dialect".to_string(), "Common".to_string()];
        assert_eq!(join_languages(&list), "Elf - high dialect, Common");
    }
}
