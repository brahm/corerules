//! `CEncTemplateDoc::CreateEncounter` (0x6421ee, 2.0 0x4c60b0): an
//! encounter from a template.

use cr_formats::monster::Monster;

use super::{
    Dice, Encounter, EncounterRules, ItemLine, MonsterLine, MonsterPackage, NpcDescription,
    NpcLine, Reference, Template, coin_name,
};
use crate::treasure::{Bundle, Coin, Item, Rand};
use crate::{Error, Result};

/// A generated encounter.
#[derive(Debug, Clone, PartialEq)]
pub struct Generated {
    pub encounter: Encounter,
    /// NPC packages the NPC generator would have made, in the order they
    /// would be in the encounter. Their counts are exact up to the first
    /// one; after it the original draws for the character and its hit
    /// points, which are not made here.
    pub pending_npcs: Vec<PendingNpc>,
    /// The original's message boxes ("Unable to find the Monster in the
    /// Database: …").
    pub messages: Vec<String>,
}

/// An NPC package still to be made by the NPC generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingNpc {
    /// Index of the template's NPC line.
    pub line: usize,
    pub npc: NpcDescription,
    pub count: i32,
}

/// The state of one generation.
struct Run<'a> {
    rules: &'a EncounterRules,
    rand: &'a mut Rand,
    encounter: Encounter,
    /// The packages dependent lines count (`this+0x68`): those of the
    /// first pass, as indices into `encounter.monsters`.
    work: Vec<usize>,
    pending: Vec<PendingNpc>,
    messages: Vec<String>,
}

impl EncounterRules {
    /// Makes an encounter from the template `name` (standard templates
    /// first), drawing from `rand` and leaving it where the original
    /// would (minus the NPC generator's draws, see [`Generated`]). The
    /// encounter has no name yet and the template's monster type; the
    /// original shows "~All" when it was made from the "~All" list.
    pub fn generate(&self, name: &str, rand: &mut Rand) -> Result<Generated> {
        let template = self
            .template(name)
            .ok_or_else(|| Error::Invalid(format!("no encounter template named {name}")))?;
        Ok(self.generate_from(template, rand))
    }

    /// [`generate`](Self::generate) for a template at hand.
    pub fn generate_from(&self, template: &Template, rand: &mut Rand) -> Generated {
        let mut run = Run {
            rules: self,
            rand,
            encounter: Encounter {
                template: template.name.clone(),
                monster_type: template.monster_type.clone(),
                ..Encounter::default()
            },
            work: Vec::new(),
            pending: Vec::new(),
            messages: Vec::new(),
        };
        for dependent in [false, true] {
            for line in template
                .monsters
                .iter()
                .filter(|l| l.dependent == dependent)
            {
                run.monster_line(line);
            }
        }
        let mut extra = Vec::new();
        for item in &template.items {
            run.item(item, &mut extra);
        }
        run.encounter.monsters.extend(extra);
        run.encounter.lair_treasure = template.lair_treasure;
        run.treasure();
        run.encounter.traps = template.traps.clone();
        for dependent in [false, true] {
            for (i, line) in template.npcs.iter().enumerate() {
                if line.dependent == dependent {
                    run.npc_line(i, line);
                }
            }
        }
        run.encounter.description = template.description.clone();
        Generated {
            encounter: run.encounter,
            pending_npcs: run.pending,
            messages: run.messages,
        }
    }
}

impl Run<'_> {
    fn between(&mut self, lo: i32, hi: i32) -> i32 {
        self.rules.between(self.rand, lo, hi)
    }

    fn dice(&mut self, dice: Dice) -> i32 {
        self.rules.roll(self.rand, dice)
    }

    /// `percent != 100 && d100 > percent`: the line does not happen.
    fn fails(&mut self, percent: i32) -> bool {
        percent != 100 && self.between(1, 100) > percent
    }

    /// The "or" roll: a d100 when the line has "or", else 0.
    fn or_roll(&mut self, or: bool) -> i32 {
        if or { self.between(1, 100) } else { 0 }
    }

    fn package(&mut self, monster: &Monster, count: i32) -> MonsterPackage {
        let mut p = MonsterPackage {
            count,
            monster: monster.clone(),
            hit_points: Vec::new(),
        };
        self.rules.roll_hit_points(&mut p, self.rand);
        p
    }

    fn push(&mut self, package: MonsterPackage, work: bool) {
        if work {
            self.work.push(self.encounter.monsters.len());
        }
        self.encounter.monsters.push(package);
    }

    fn lookup(&mut self, name: &str) -> Option<Monster> {
        let found = self.rules.monsters.standard_first(name).cloned();
        if found.is_none() {
            self.messages.push(format!(
                "Unable to find the Monster in the Database: {name}"
            ));
        }
        found
    }

    /// The monsters counted by a reference (0x64430b by type, 0x644458 by
    /// name); `None` when it has neither.
    fn reference_count(&self, reference: &Reference) -> Option<i32> {
        let by_type = !reference.monster_type.is_empty();
        if !by_type && reference.monster.is_empty() {
            return None;
        }
        Some(self.work.iter().fold(0i32, |sum, &i| {
            let p = &self.encounter.monsters[i];
            let name = if by_type {
                &p.monster.monster_type
            } else {
                &p.monster.name
            };
            let wanted = if by_type {
                &reference.monster_type
            } else {
                &reference.monster
            };
            if name == wanted {
                sum.wrapping_add(p.count)
            } else {
                sum
            }
        }))
    }

    /// How many times a dependent line rolls: `(c - beyond) / per`, or 1
    /// without "for every"; `None` when `c` is not above `beyond`.
    fn groups(&self, reference: &Reference, per: i32, beyond: i32) -> Option<i32> {
        let c = self.reference_count(reference).unwrap_or(0);
        if c < 1 || c <= beyond {
            return None;
        }
        Some(if per >= 1 {
            c.wrapping_sub(beyond) / per
        } else {
            1
        })
    }

    /// Splits `total` monsters over a random list (one draw each), then
    /// makes a package per entry drawn, in list order. The first is
    /// returned; the others go into the encounter at once, and into the
    /// working list in the first pass.
    fn distribute(&mut self, list: &[Monster], total: i32, work: bool) -> MonsterPackage {
        let mut tally = vec![0i32; list.len()];
        let last = list.len() as i32 - 1;
        for _ in 0..total {
            let i = self.between(0, last);
            if let Some(t) = tally.get_mut(i as usize) {
                *t += 1;
            }
        }
        let mut first = None;
        for (monster, &n) in list.iter().zip(&tally) {
            if n > 0 {
                let p = self.package(monster, n);
                if first.is_none() {
                    first = Some(p);
                } else {
                    self.push(p, work);
                }
            }
        }
        first.expect("total is at least 1")
    }

    fn monster_line(&mut self, line: &MonsterLine) {
        let work = !line.dependent;
        let r = self.or_roll(line.or);
        let main = match (line.dependent, r < 51) {
            (false, true) => self.primary(line),
            (false, false) => self.alternative(line),
            (true, true) => self.dependent_primary(line),
            (true, false) => self.dependent_alternative(line),
        };
        if line.and && main.is_some() {
            let second = if line.dependent {
                self.dependent_and(line)
            } else {
                self.and(line)
            };
            if let Some(p) = second {
                self.push(p, work);
            }
        }
        if let Some(p) = main {
            self.push(p, work);
        }
    }

    /// Primary (0x643482).
    fn primary(&mut self, line: &MonsterLine) -> Option<MonsterPackage> {
        if self.fails(line.percent) {
            return None;
        }
        let monster = self.lookup(&line.monster)?;
        let mut n = self.dice(line.dice);
        if n < 1 {
            n = self.rules.number_appearing(self.rand, &monster);
        }
        let n = n.max(1);
        Some(if line.list.len() < 2 {
            self.package(&monster, n)
        } else {
            self.distribute(&line.list, n, true)
        })
    }

    /// The second box, alone ("or", 0x6437e7) or with the first ("and",
    /// 0x6438a6, no percent check).
    fn second(&mut self, line: &MonsterLine) -> Option<MonsterPackage> {
        let monster = self.lookup(&line.second_monster)?;
        let mut n = self.dice(line.second_dice);
        if n < 1 {
            n = self.rules.number_appearing(self.rand, &monster);
        }
        Some(self.package(&monster, n.max(1)))
    }

    fn alternative(&mut self, line: &MonsterLine) -> Option<MonsterPackage> {
        if self.fails(line.percent) {
            return None;
        }
        self.second(line)
    }

    fn and(&mut self, line: &MonsterLine) -> Option<MonsterPackage> {
        self.second(line)
    }

    /// Dependent primary (0x643936): no No. Appearing fallback.
    fn dependent_primary(&mut self, line: &MonsterLine) -> Option<MonsterPackage> {
        if self.fails(line.percent) {
            return None;
        }
        let k = self.groups(&line.reference, line.per, line.beyond)?.max(0);
        let mut n = 0i32;
        for _ in 0..k {
            n = n.wrapping_add(self.dice(line.dice));
        }
        if n < 1 {
            return None;
        }
        if line.list.len() < 2 {
            let monster = self.lookup(&line.monster)?;
            Some(self.package(&monster, n))
        } else {
            Some(self.distribute(&line.list, n, false))
        }
    }

    /// Dependent "or" (0x643e03) and "and" (0x6440af): at least one group;
    /// the No. Appearing fallback rolls once per group for "or", once for
    /// "and".
    fn dependent_second(&mut self, line: &MonsterLine, or: bool) -> Option<MonsterPackage> {
        let k = self.groups(&line.reference, line.per, line.beyond)?.max(1);
        let monster = self.lookup(&line.second_monster)?;
        let mut n = 0i32;
        for _ in 0..k {
            n = n.wrapping_add(self.dice(line.second_dice));
        }
        if n < 1 {
            n = 0;
            for _ in 0..if or { k } else { 1 } {
                n = n.wrapping_add(self.rules.number_appearing(self.rand, &monster));
            }
        }
        Some(self.package(&monster, n.max(1)))
    }

    fn dependent_alternative(&mut self, line: &MonsterLine) -> Option<MonsterPackage> {
        if self.fails(line.percent) {
            return None;
        }
        self.dependent_second(line, true)
    }

    fn dependent_and(&mut self, line: &MonsterLine) -> Option<MonsterPackage> {
        self.dependent_second(line, false)
    }

    /// One item line (inside 0x6421ee); the monsters it brings go to
    /// `extra`, added to the encounter after every item.
    fn item(&mut self, item: &ItemLine, extra: &mut Vec<MonsterPackage>) {
        // 0x6445a5: the percent, then the reference if there is one.
        if self.fails(item.percent) {
            return;
        }
        if self.reference_count(&item.reference).is_some() {
            match self.groups(&item.reference, item.per, item.beyond) {
                Some(k) => item.groups.set(k.max(1)),
                None => return,
            }
        }
        for escort in &item.escorts {
            let mut n = self.dice(escort.dice);
            if n < 1 {
                n = self.rules.number_appearing(self.rand, &escort.monster);
            }
            extra.push(self.package(&escort.monster, n.max(1)));
        }
        // 0x651341: the multiplier applies to the running total.
        if item.groups.get() < 1 {
            item.groups.set(1);
        }
        let mut quantity = 0i32;
        for _ in 0..item.groups.get() {
            quantity = self.add_dice(quantity, item.quantity);
        }
        if quantity > 0 {
            self.encounter
                .items
                .insert(Item::new(item.item.clone(), quantity as u32), true);
        }
        // 0x6513fc: coins for at least one item, compounding the same way.
        let mut coins = 0i32;
        for _ in 0..quantity.max(1) {
            coins = self.add_dice(coins, item.coins);
        }
        if coins > 0 {
            self.encounter
                .items
                .insert(Item::new(coin_name(item.coin), coins as u32), true);
        }
        for escort in &item.per_item {
            let mut n = 0i32;
            for _ in 0..quantity {
                n = n.wrapping_add(self.dice(escort.dice));
            }
            if n < 1 {
                n = self.rules.number_appearing(self.rand, &escort.monster);
            }
            extra.push(self.package(&escort.monster, n.max(1)));
        }
        // 0x64481b: only the number doubles, the hit points stay.
        for name in &item.doubled {
            for p in &mut self.encounter.monsters {
                if &p.monster.name == name {
                    p.count = p.count.wrapping_shl(1);
                }
            }
        }
    }

    /// `(acc + Σ dice) × multiplier + bonus`.
    fn add_dice(&mut self, acc: i32, dice: Dice) -> i32 {
        let mut sum = acc;
        for _ in 0..dice.count {
            sum = sum.wrapping_add(self.between(1, dice.sides));
        }
        sum.wrapping_mul(dice.multiplier).wrapping_add(dice.bonus)
    }

    /// The treasure (§6.9): the lair letters plus each monster's letters
    /// times its number, rolled into a new bundle with platinum coins.
    fn treasure(&mut self) {
        let counts = self.encounter.treasure_counts().map(|n| n.max(0) as u32);
        let mut bundle = Bundle::default();
        self.rules
            .treasure
            .roll(self.rand, &counts, Coin::Platinum, &mut bundle);
        self.encounter.treasure = bundle;
    }

    /// One NPC line (0x644e7d). The packages the NPC generator would make
    /// are recorded as pending; the draws it makes are not.
    fn npc_line(&mut self, index: usize, line: &NpcLine) {
        let r = self.or_roll(line.or);
        let main = if r < 51 {
            self.npc_primary(index, line)
        } else {
            self.npc_second(index, line, true)
        };
        if line.and
            && main.is_some()
            && let Some(p) = self.npc_second(index, line, false)
        {
            self.pending.push(p);
        }
        if let Some(p) = main {
            self.pending.push(p);
        }
    }

    /// The number of groups of a dependent NPC line, at least 1.
    fn npc_groups(&self, line: &NpcLine) -> Option<i32> {
        if !line.dependent {
            return Some(1);
        }
        Some(self.groups(&line.reference, line.per, line.beyond)?.max(1))
    }

    /// Primary (0x645760) and dependent primary (0x646530): at least one
    /// NPC, no No. Appearing fallback.
    fn npc_primary(&mut self, index: usize, line: &NpcLine) -> Option<PendingNpc> {
        if self.fails(line.percent) {
            return None;
        }
        let k = self.npc_groups(line)?;
        let mut n = 0i32;
        for _ in 0..k {
            n = n.wrapping_add(self.dice(line.dice));
        }
        let n = n.max(1);
        if line.list.len() < 2 {
            return Some(PendingNpc {
                line: index,
                npc: line.npc.clone(),
                count: n,
            });
        }
        let mut tally = vec![0i32; line.list.len()];
        let last = line.list.len() as i32 - 1;
        for _ in 0..n {
            let i = self.between(0, last);
            if let Some(t) = tally.get_mut(i as usize) {
                *t += 1;
            }
        }
        let mut first = None;
        for (npc, &count) in line.list.iter().zip(&tally) {
            if count > 0 {
                let p = PendingNpc {
                    line: index,
                    npc: npc.clone(),
                    count,
                };
                if first.is_none() {
                    first = Some(p);
                } else {
                    self.pending.push(p);
                }
            }
        }
        first
    }

    /// "or" (0x6455a7, 0x645d68) and "and" (0x64542c, 0x64616b): the
    /// second NPC; the original makes the character before rolling the
    /// number.
    fn npc_second(&mut self, index: usize, line: &NpcLine, or: bool) -> Option<PendingNpc> {
        if or && self.fails(line.percent) {
            return None;
        }
        let k = self.npc_groups(line)?;
        let mut n = 0i32;
        for _ in 0..k {
            n = n.wrapping_add(self.dice(line.second_dice));
        }
        Some(PendingNpc {
            line: index,
            npc: line.second_npc.clone(),
            count: n.max(1),
        })
    }
}
