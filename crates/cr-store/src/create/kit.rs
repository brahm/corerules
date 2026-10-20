//! What a kit does on the Name page of the original's wizard (IDD 213),
//! in Create Character and Update Character alike: which kits Create
//! offers (0x5bd5ed), the warnings a gender or alignment the kit does not
//! allow brings, the kit's information text, and what leaving the page
//! applies to the character (0x442cac): the kit's fixed rogue skill
//! adjustments, the class abilities it bars, its schools of magic, the
//! spells it knows and the languages it adds by level.

use cr_formats::Object;
use cr_formats::character::Character;

use super::{CreationRules, Draft, rogue_skill_of};
use crate::npc::data::Kit;
use crate::npc::state::KIT_TESTS;

/// The kit flag of each alignment (0x670ed3: +6 to +0xe).
const ALIGNMENT_FLAGS: [(&str, u32); 9] = [
    ("Lawful Good", 6),
    ("Lawful Neutral", 7),
    ("Lawful Evil", 8),
    ("Neutral Good", 9),
    ("True Neutral", 10),
    ("Neutral Evil", 11),
    ("Chaotic Good", 12),
    ("Chaotic Neutral", 13),
    ("Chaotic Evil", 14),
];

/// The schools a specialist (class id) gets with a kit that has different
/// opposition schools (+0x1f0; 0x442cdf).
const KIT_SCHOOLS: [(i32, &[&str]); 6] = [
    (
        4001,
        &[
            "Abjuration",
            "Conjuration/Summoning",
            "Enchantment/Charm",
            "Invocation/Evocation",
            "Necromancy",
        ],
    ),
    (
        4002,
        &[
            "Abjuration",
            "Conjuration/Summoning",
            "Enchantment/Charm",
            "Illusion/Phantasm",
            "Necromancy",
        ],
    ),
    (
        4003,
        &[
            "Alteration",
            "Enchantment/Charm",
            "Greater Divination",
            "Illusion/Phantasm",
            "Invocation/Evocation",
            "Necromancy",
        ],
    ),
    (
        4006,
        &[
            "Abjuration",
            "Alteration",
            "Greater Divination",
            "Invocation/Evocation",
            "Necromancy",
        ],
    ),
    (
        4007,
        &[
            "Abjuration",
            "Conjuration/Summoning",
            "Greater Divination",
            "Invocation/Evocation",
            "Necromancy",
        ],
    ),
    (
        4008,
        &[
            "Alteration",
            "Enchantment/Charm",
            "Greater Divination",
            "Illusion/Phantasm",
            "Invocation/Evocation",
        ],
    ),
];

/// The proficiency a kit's languages come as (0x442a83).
const MODERN_LANGUAGES: &str = "Modern Languages";

impl CreationRules {
    fn kit(&self, name: &str) -> Option<&Kit> {
        self.kits.iter().find(|k| k.name == name).map(|k| &k.kit)
    }

    /// The kits Create offers the draft, "None" first (0x5bd5ed): those
    /// for its rules, scores, gender, alignment, prime requisites, race
    /// and classes. Update Character offers [`Self::all_kits`].
    pub fn kits_for(&self, draft: &Draft) -> Vec<String> {
        let allowed = self.kits(&draft.race, &draft.subrace, &draft.classes);
        let scores = self.subability_scores(draft);
        // The lowest prime requisite of the classes.
        let lowest = draft
            .classes
            .iter()
            .filter_map(|c| self.class(c))
            .flat_map(|c| (0..6).filter(|&a| c.prime[a]).map(|a| scores[a][0] as i32))
            .min();
        allowed
            .into_iter()
            .filter(|name| {
                let Some(rules) = self.kits.iter().find(|k| &k.name == name) else {
                    return true;
                };
                if name == "None" {
                    return true;
                }
                let kit = &rules.kit;
                if !kit.flag(if draft.skills_and_powers { 0x74 } else { 0x73 }) {
                    return false;
                }
                let out_of_range = KIT_TESTS.iter().any(|&(a, sub, off)| {
                    let v = scores[a][sub.map_or(0, |s| s + 1)] as i32;
                    v < kit.b(off) || kit.max(off) < v
                });
                if out_of_range
                    || !kit_allows_gender(kit, draft.male)
                    || !kit_allows_alignment(kit, &draft.alignment)
                    || lowest.is_some_and(|low| kit.b(0x70) > low)
                {
                    return false;
                }
                !(!draft.classes.is_empty()
                    && draft
                        .classes
                        .iter()
                        .all(|c| rules.barred_classes.contains(c)))
            })
            .collect()
    }

    /// Whether characters of the gender may take the kit (+4 male, +5
    /// female).
    pub fn kit_allows_gender(&self, kit: &str, male: bool) -> bool {
        self.kit(kit).is_none_or(|k| kit_allows_gender(k, male))
    }

    /// Whether the kit allows the alignment.
    pub fn kit_allows_alignment(&self, kit: &str, alignment: &str) -> bool {
        kit == "None"
            || self
                .kit(kit)
                .is_none_or(|k| kit_allows_alignment(k, alignment))
    }

    /// The text the original shows in a "Kit Information" box when the kit
    /// is chosen (+0x9c), if it has one.
    pub fn kit_information(&self, kit: &str) -> Option<&str> {
        self.kit(kit).map(|k| k.s(0x9c)).filter(|s| !s.is_empty())
    }

    /// The kit's fixed adjustment to a rogue skill (+0x174), which goes to
    /// the skill as it is on the draft's rogue skills.
    pub fn kit_rogue_adjustment(&self, draft: &Draft, skill: usize) -> i32 {
        let Some(kit) = self.kit(&draft.kit) else {
            return 0;
        };
        let skills = self.rogue_skills(draft);
        kit.rogue_adjust
            .iter()
            .filter(|(name, _)| {
                skills.iter().any(|(n, s)| n == name && *s == skill)
                    && rogue_skill_of(name) == Some(skill)
            })
            .map(|(_, amount)| amount)
            .sum()
    }

    /// The class abilities a Core Rules character of the kit loses from
    /// its first class (+0x1c8; 0x443662).
    pub fn kit_barred_abilities(&self, kit: &str) -> &[String] {
        self.kit(kit).map_or(&[], |k| k.sa(0x1c8))
    }

    /// What leaving the Name page applies to the character (0x442cac),
    /// with `levels` its class levels: Create when it writes the
    /// character, Update Character on every save.
    pub(crate) fn apply_kit(&self, character: &mut Character, levels: &[u32]) {
        let skills_and_powers = character.skills_and_powers();
        let d = &mut character.data;
        let Some(kit) = self.kit(&d.kit) else { return };
        let level = levels.iter().copied().max().unwrap_or(1).clamp(1, 100);

        // 0x442604: the fixed rogue adjustments go to 1st level, once,
        // from the level of the first class with rogue skills; without
        // one the character has no rogue skills.
        if d.flag_0x1610 != Some(1) {
            let rogue = d
                .classes
                .names
                .iter()
                .position(|c| self.class(c).is_some_and(|c| c.rogue_skills));
            match rogue {
                None => d.rogue_skills.entries.clear(),
                Some(_) if kit.rogue_adjust.is_empty() => {}
                Some(r) => {
                    let at = levels.get(r).copied().unwrap_or(1).clamp(1, 100) as usize;
                    for (name, amount) in &kit.rogue_adjust {
                        if let Some((_, values)) =
                            d.rogue_skills.entries.iter_mut().find(|(n, _)| n == name)
                        {
                            values[0] = values[at - 1].wrapping_add_signed(*amount);
                        }
                    }
                    if d.flag_0x1610.is_some() {
                        d.flag_0x1610 = Some(1);
                    }
                }
            }
        }

        // 0x443662: the barred class abilities, from the first class of
        // a Core Rules character.
        if !skills_and_powers {
            let barred = kit.sa(0x1c8);
            d.class_slots[0].abilities.retain(|a| !barred.contains(a));
        }

        // 0x442cdf: different opposition schools.
        if kit.flag(0x1f0) {
            for (slot, name) in d.classes.names.iter().enumerate().take(5) {
                let Some(class) = self.class(name) else {
                    continue;
                };
                if let Some((_, schools)) = KIT_SCHOOLS.iter().find(|(id, _)| *id == class.id) {
                    d.class_slots[slot].unknown_0x270 =
                        schools.iter().map(|s| (*s).to_owned()).collect();
                }
            }
        }

        // 0x4427ab: the spells the kit knows.
        for (spell, wizard) in &kit.bonus_spells {
            let list = if *wizard {
                &mut d.spells
            } else {
                &mut d.priest_spells
            };
            if !list.iter().any(|s| s.eq_ignore_ascii_case(spell)) {
                list.push(spell.clone());
            }
        }

        // 0x442a83: a language every `every` levels from `from`, each
        // level once, as Modern Languages or one more rank of it.
        let (every, from) = (kit.i(0x5fc), kit.i(0x600));
        if from == 0 {
            return;
        }
        let Some(granted) = d.unknown_0x2174.as_mut() else {
            return;
        };
        for l in 1..=level as i32 {
            let due = if l == from || (l > from && every == 0) {
                true
            } else if every == 0 {
                // The original divides by zero here.
                false
            } else {
                (l - from) % every == 0
            };
            let done = granted.iter().any(|o| {
                matches!(o, Object::KitBonus(b)
                    if b.name.eq_ignore_ascii_case(MODERN_LANGUAGES) && b.unknown_0x8 == l)
            });
            if !due || done {
                continue;
            }
            let known = d
                .nonweapon_proficiencies
                .iter()
                .position(|n| n == MODERN_LANGUAGES);
            match known {
                Some(i) if skills_and_powers => {
                    if let Some(cost) = d.nonweapon_costs.get_mut(i) {
                        *cost += 1;
                    }
                }
                _ => {
                    d.nonweapon_proficiencies.push(MODERN_LANGUAGES.into());
                    d.nonweapon_costs.push(
                        self.parts
                            .nonweapon_cost(MODERN_LANGUAGES, skills_and_powers),
                    );
                    if let Some(bonuses) = d.nonweapon_bonuses.as_mut() {
                        bonuses.push(0);
                    }
                    d.nonweapon_details.push(Object::StringArray(Vec::new()));
                }
            }
            granted.push(Object::KitBonus(cr_formats::classes::KitBonus {
                version: 1,
                name: MODERN_LANGUAGES.into(),
                unknown_0x8: l,
                unknown_0xc: 6,
            }));
        }
    }
}

fn kit_allows_gender(kit: &Kit, male: bool) -> bool {
    kit.flag(if male { 0x4 } else { 0x5 })
}

fn kit_allows_alignment(kit: &Kit, alignment: &str) -> bool {
    // Any other alignment reads +0x63.
    let flag = ALIGNMENT_FLAGS
        .iter()
        .find(|(a, _)| *a == alignment)
        .map_or(0x63, |(_, f)| *f);
    kit.flag(flag)
}
