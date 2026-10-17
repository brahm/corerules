//! The generated NPC as a character record, the way the generator copies
//! its members into the character document.

use cr_formats::Object;
use cr_formats::character::{CORE20_VERSION, Character, SheetEntries, SheetOptions};
use cr_formats::inventory::{InventoryBundle, InventoryExtra, InventoryItem};

use super::Gen;
use super::rogue::KitBonus;
use super::state::{It, STR};
use crate::lists::PartTable;
use crate::{Error, Result};

fn u(v: i32) -> u32 {
    v.max(0) as u32
}

/// A generator item as an inventory item.
fn item(parts: &PartTable, it: &It) -> InventoryItem {
    let mut out = parts.new_item(&it.name, u(it.n));
    if let Some(b) = &it.bundle {
        out.contents = Some(Box::new(Object::InventoryBundle(InventoryBundle {
            items: b.iter().map(|x| item(parts, x)).collect(),
        })));
    }
    if let Some(book) = &it.book {
        out.extra = Some(InventoryExtra {
            unknown_0x4: [u(book.capacity), u(book.used), book.spells.len() as u32],
            unknown_0x10: book.spells.clone(),
            unknown_0x24: book.pages.iter().map(|&p| u(p)).collect(),
            unknown_0x38: book.wizard.iter().map(|&w| u8::from(w)).collect(),
        });
    }
    out
}

impl Gen<'_> {
    /// The character record, in the layout `version` names.
    pub fn character(&mut self, parts: &PartTable, version: &str) -> Result<Character> {
        let format = |source| Error::Format {
            path: "generated NPC".into(),
            source,
        };
        let expansion = version != CORE20_VERSION;
        let mut character = Character::blank(version).map_err(format)?;
        let d = &mut character.data;
        d.name.clone_from(&self.name);
        d.class_title.clone_from(&self.title);
        d.unknown_0x90.clone_from(&self.background);
        d.social_class.clone_from(&self.social);
        for (slot, xp) in d.unknown_0x1618.iter_mut().zip(&self.xp) {
            *slot = u(*xp);
        }
        d.height = u(self.height);
        d.weight = u(self.weight);
        d.age = u(self.age);
        d.unknown_0x163c = u32::from(self.male());
        let s = &self.subs;
        d.strength = [u(self.ab[STR]), u(s[STR][0]), u(s[STR][1])];
        d.exceptional_strength = u(self.exc);
        for (i, a) in d.abilities.iter_mut().enumerate() {
            let k = i + 1;
            *a = [u(self.ab[k]), u(s[k][0]), u(s[k][1])];
        }
        d.race.clone_from(&self.race.name);
        d.subrace.clone_from(&self.subrace.name);
        d.kit = if self.kit_name.is_empty() {
            "None".into()
        } else {
            self.kit_name.clone()
        };
        d.alignment.clone_from(&self.alignment);
        d.classes.names = self.classes.iter().map(|k| k.name.clone()).collect();
        d.weapon_proficiencies.clone_from(&self.weapon_list);
        d.unknown_0xac[3].clone_from(&self.weapon_spec);
        d.racial_abilities.clone_from(&self.racial_abilities);
        d.spells.clone_from(&self.book_w);
        d.priest_spells.clone_from(&self.book_p);
        for (g, per) in self.spells.iter().enumerate().take(5) {
            for (sl, (w, p)) in per.iter().enumerate().take(11) {
                d.unknown_0x3c8[g][sl] = (w.clone(), p.clone());
            }
        }
        for (g, rolls) in self.hp.iter().enumerate().take(5) {
            for (l, h) in rolls.iter().enumerate().take(100) {
                d.unknown_0xc60[g][l] = u(*h);
            }
        }
        for l in 1..self.psp.len().min(101) {
            d.unknown_0x1430[l - 1] = u(self.psp[l]);
        }
        d.nonweapon_proficiencies = self.nwp.iter().map(|n| n.0.clone()).collect();
        d.nonweapon_costs = self.nwp.iter().map(|n| u(n.1)).collect();
        if d.nonweapon_bonuses.is_some() {
            d.nonweapon_bonuses = Some(self.nwp.iter().map(|n| u(n.2)).collect());
        }
        d.nonweapon_details = (0..self.nwp.len())
            .map(|i| Object::StringArray(self.nwp_langs.get(&i).cloned().unwrap_or_default()))
            .collect();
        d.languages.clone_from(&self.languages);
        for (b, list) in d.inventory.iter_mut().zip(&self.inventory) {
            b.items = list.iter().map(|it| item(parts, it)).collect();
        }
        let (armor, magic) = parts.worn_lists(&d.inventory[5]);
        d.armor = armor;
        d.unknown_0x17a4 = magic;
        let (gp, sp, cp) = self.coins;
        d.unknown_0x1778 = [0, u(cp), u(sp), 0, u(gp), 0];
        let movement = match self.subrace.movement {
            0 => self.race.movement,
            m => m,
        };
        d.unknown_0x17bc[0] = u(movement);
        d.unknown_0x196c = [
            self.psi.disciplines.clone(),
            self.psi.sciences.clone(),
            self.psi.devotions.clone(),
        ];
        d.unknown_0x1980 = [self.psi.primary.clone(), self.psi.secondary.clone()];
        // 1 marks an NPC.
        d.unknown_0x1da4 = 1;
        if expansion {
            d.carried_weight = self.carried.to_bits();
            d.flags_0x1ddc = Some([1, 0]);
            d.unknown_0x1df0 = Some((u32::MAX, 0));
        }
        d.sheet_options = Some(SheetOptions {
            version: 2,
            unknown_0x260: 0,
            unknown_0x25c: if expansion { 0 } else { 1822 },
            entries: SheetEntries::Pairs(Vec::new()),
        });
        for (g, k) in self.classes.iter().enumerate().take(5) {
            let slot = &mut d.class_slots[g];
            slot.abilities = k.sa(0x2724).to_vec();
            slot.unknown_0x270 = self.schools.get(g).cloned().unwrap_or_default();
            slot.unknown_0x2d4 = self.minor.get(g).cloned().unwrap_or_default();
            slot.unknown_0x338 = self.major.get(g).cloned().unwrap_or_default();
        }
        // Rogue skills (0x4c751c), saved as they stand: name and 100 levels.
        d.rogue_skills.entries = self
            .rogue
            .iter()
            .map(|(n, v)| (n.clone(), v.map(|x| x as u32)))
            .collect();
        if d.flag_0x1610.is_some() {
            d.flag_0x1610 = Some(u8::from(self.m1610));
        }
        if d.unknown_0x15e4.is_some() {
            let objects = |list: &[Option<KitBonus>]| -> Vec<Option<Box<Object>>> {
                list.iter()
                    .map(|b| {
                        b.as_ref().map(|(name, level, amount)| {
                            Box::new(Object::KitBonus(cr_formats::classes::KitBonus {
                                version: 1,
                                name: name.clone(),
                                unknown_0x8: *level,
                                unknown_0xc: *amount,
                            }))
                        })
                    })
                    .collect()
            };
            d.unknown_0x15e4 = Some([objects(&self.rogue_random), objects(&self.rogue_fixed)]);
        }
        let bytes = character.to_bytes().map_err(format)?;
        Character::parse(&bytes).map_err(format)?;
        Ok(character)
    }
}
