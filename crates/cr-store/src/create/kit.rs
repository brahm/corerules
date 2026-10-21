//! What a kit does on the Name page of the original's wizard (IDD 213),
//! in Create Character and Update Character alike: which kits Create
//! offers (0x5bd5ed), the warnings a gender or alignment the kit does not
//! allow brings, the kit's information text, and what leaving the page
//! applies to the character (0x442cac): the kit's fixed rogue skill
//! adjustments, the class abilities it bars, its schools of magic, the
//! spells it knows and the languages it adds by level, and the hit points
//! it adds to each roll (0x42d8f7). Also what the kit
//! does on the Rogue Skills page of Create (IDD 342): its bonus to one
//! skill at 1st level, the skills it has the character choose and the
//! list its starting points go to; and on the Spells page (62f1c8), the
//! schools it bars and those a kit limited to five has the wizard
//! eliminate (see [`crate::kit_magic`]); and on the Equipment page, the
//! kit's equipment text (0x494875), its restriction notices (0x5ea499)
//! and the gold it lets the character keep (0x5e33cb).

use cr_formats::Object;
use cr_formats::character::{Character, CharacterData};
use cr_formats::classes::KitBonus;

use cr_i18n::tr;

use super::{CreationRules, Draft, Question, rogue_skill_of};
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
    pub(super) fn kit(&self, name: &str) -> Option<&Kit> {
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

    /// The kit's free equipment (+0x598), weapons (+0x5ac) and magic items
    /// (+0x5c0), in that order, those the databases have.
    pub fn kit_free_items(&self, kit: &str) -> Vec<String> {
        self.kit(kit)
            .filter(|_| self.expansion)
            .map(|k| {
                [0x598, 0x5ac, 0x5c0]
                    .iter()
                    .flat_map(|&o| k.sa(o))
                    .filter(|name| self.parts.has(name))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The percentage of an item's price the kit's characters pay (+0x90;
    /// 5dda98); 100 without a kit.
    pub fn kit_price_percent(&self, kit: &str) -> i32 {
        self.kit(kit)
            .filter(|_| self.expansion)
            .map_or(100, |k| k.i_or(0x90, 100))
    }

    /// The notice New Character's Equipment page opens with when the kit
    /// changes what items cost (5e2ba2), which says "higher" either way.
    pub fn kit_price_notice(&self, kit: &str) -> Option<&'static str> {
        let percent = self.kit_price_percent(kit);
        (percent != 0 && percent != 100)
            .then(|| tr("This character pays a higher price for items."))
    }

    /// The hit points the kit adds to every level's roll (+0x604, at the
    /// end of 0x42d8f7), which may be negative.
    pub fn kit_hit_points(&self, kit: &str) -> i32 {
        self.kit(kit)
            .filter(|_| self.expansion)
            .map_or(0, |k| k.i(0x604))
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

    /// The kit's bonus to one rogue skill at 1st level (+0x13c, from the
    /// skills in +0x140 the character has; 62cc45): the points and the
    /// skills it may go to.
    pub fn kit_rogue_bonus(&self, draft: &Draft) -> Option<(i32, Vec<String>)> {
        let kit = self.kit(&draft.kit)?;
        let amount = kit.i(0x13c);
        let skills = self.rogue_skills(draft);
        let names: Vec<String> = kit
            .sa(0x140)
            .iter()
            .filter(|n| skills.iter().any(|(s, _)| s == *n))
            .cloned()
            .collect();
        (amount != 0 && !names.is_empty()).then_some((amount, names))
    }

    /// The rogue skill the kit's 1st-level bonus went to, its index in
    /// [`super::ROGUE_SKILLS`] and the points.
    pub(super) fn kit_rogue_bonus_taken(&self, draft: &Draft) -> Option<(String, usize, i32)> {
        let name = draft.rogue_bonus.as_ref()?;
        let (amount, names) = self.kit_rogue_bonus(draft)?;
        let skill = self
            .rogue_skills(draft)
            .into_iter()
            .find(|(n, _)| n == name)?
            .1;
        names.contains(name).then(|| (name.clone(), skill, amount))
    }

    /// The rogue skills the kit has the character choose (+0x16c of them
    /// for a +0x170% bonus; 62c8e6): the count, the bonus and the skills.
    pub fn kit_chosen_skills(&self, draft: &Draft) -> Option<(usize, i32, Vec<String>)> {
        let kit = self.kit(&draft.kit)?;
        let count = kit.i(0x16c);
        let names: Vec<String> = self.rogue_skills(draft).into_iter().map(|s| s.0).collect();
        (count > 0 && !names.is_empty()).then_some((count as usize, kit.i(0x170), names))
    }

    /// The schools of the draft's wizard class as the kit has them: the
    /// different opposition schools (+0x1f0; 0x442cdf), or the class's.
    fn kit_class_schools(&self, draft: &Draft) -> Option<Vec<String>> {
        let class = draft
            .classes
            .iter()
            .filter_map(|c| self.class(c))
            .find(|c| c.group == 3)?;
        let opposition = self
            .draft_kit(draft)
            .filter(|k| k.flag(0x1f0))
            .and_then(|_| KIT_SCHOOLS.iter().find(|(id, _)| *id == class.id));
        Some(match opposition {
            Some((_, schools)) => schools.iter().map(|s| (*s).to_owned()).collect(),
            None => class.schools.clone(),
        })
    }

    /// The kit's warning about the schools it bars that the wizard class
    /// has, unless ignored (+0x1dc; 63056f).
    pub fn kit_barred_schools_warning(&self, draft: &Draft) -> Option<String> {
        let barred = self.draft_kit(draft)?.sa(0x1dc);
        let schools = self.kit_class_schools(draft)?;
        (!draft.barred_schools_ignored && schools.iter().any(|s| barred.contains(s)))
            .then(|| crate::kit_magic::barred_text(&draft.kit, barred))
    }

    /// How many schools beyond five a kit limited to five (+0x1fc;
    /// 6309df) has the draft eliminate, and the schools to eliminate them
    /// from.
    pub fn kit_schools_to_eliminate(&self, draft: &Draft) -> Option<(usize, Vec<String>)> {
        let kit = self.draft_kit(draft)?;
        if !kit.flag(0x1fc) {
            return None;
        }
        let schools = crate::kit_magic::kept(
            &self.kit_class_schools(draft)?,
            kit.sa(0x1dc),
            draft.barred_schools_ignored,
            &[],
        );
        let count = schools.len().checked_sub(5).filter(|&n| n > 0)?;
        Some((count, schools))
    }

    /// The wizard class's schools once the kit's barred ones go, unless
    /// ignored, and the eliminated ones; none without a wizard class.
    pub fn kit_wizard_schools(&self, draft: &Draft) -> Option<Vec<String>> {
        let schools = self.kit_class_schools(draft)?;
        let barred = self.draft_kit(draft).map_or(&[][..], |k| k.sa(0x1dc));
        Some(crate::kit_magic::kept(
            &schools,
            barred,
            draft.barred_schools_ignored,
            &draft.eliminated_schools,
        ))
    }

    /// Writes the draft's schools into the wizard class, with the kit's
    /// warning ignored and the schools eliminated.
    pub(super) fn write_kit_schools(&self, draft: &Draft, character: &mut Character) {
        let Some(kit) = self.draft_kit(draft) else {
            return;
        };
        let d = &mut character.data;
        for (slot, name) in draft.classes.iter().enumerate().take(5) {
            if self.class(name).is_some_and(|c| c.group == 3) {
                let s = &mut d.class_slots[slot];
                s.unknown_0x270 = crate::kit_magic::kept(
                    &s.unknown_0x270,
                    kit.sa(0x1dc),
                    draft.barred_schools_ignored,
                    &draft.eliminated_schools,
                );
            }
        }
        if d.flag_0x1ea8.is_some() {
            d.flag_0x1ea8 = Some(u8::from(draft.barred_schools_ignored));
        }
        if d.unknown_0x1e00.is_some() && !draft.eliminated_schools.is_empty() {
            d.unknown_0x1e00 = Some(draft.eliminated_schools.clone());
        }
    }

    /// The kit's text shown before a character of the kit is equipped
    /// (+0xa0; 494875), in New Character and Update Character alike.
    pub fn kit_equipment_text(&self, kit: &str) -> Option<&str> {
        self.kit(kit)
            .filter(|_| self.expansion)
            .map(|k| k.s(0xa0))
            .filter(|s| !s.trim().is_empty())
    }

    /// What New Character's Equipment page tells about the kit as it opens
    /// (5ea499): the items it requires and its chosen weapon, neither
    /// enforced.
    pub fn kit_equipment_notices(&self, draft: &Draft) -> Vec<Question> {
        let Some(kit) = self.draft_kit(draft) else {
            return Vec::new();
        };
        let mut notices = Vec::new();
        let items = kit.sa(0x584);
        if !items.is_empty() {
            let mut text = tr!(
                "The kit {kit} requires that the following list of items be purchased during \
                 character generation:",
                kit = draft.kit.as_str()
            );
            for item in items {
                text.push('\n');
                text.push_str(item);
            }
            notices.push(Question::notice(tr("Kit Restriction"), text));
        }
        if kit.flag(0x580) {
            notices.push(Question::notice(
                tr("Kit Restriction"),
                tr!(
                    "The kit {kit} requires that the chosen weapon be purchased during \
                     character generation.",
                    kit = draft.kit.as_str()
                ),
            ));
        }
        notices
    }

    /// The kit's warning when the character keeps more than the money it
    /// allows, `kept` in copper pieces (+0x5dc, +0x5e0; 5e33cb), which may
    /// be ignored.
    pub fn kit_retained_funds_warning(&self, draft: &Draft, kept: u32) -> Option<String> {
        let kit = self.draft_kit(draft)?;
        let most = kit.i(0x5e0);
        (kit.flag(0x5dc) && i64::from(kept) > i64::from(most) * 100).then(|| {
            tr!(
                "The kit {kit} allows maximum retained funds of {most} gold piece(s).",
                kit = draft.kit.as_str(),
                most
            )
        })
    }

    /// The kit's warning when starting rogue points go to a skill outside
    /// its list (+0x128; 62c144, Create only), which may be ignored.
    pub fn kit_rogue_restriction(&self, draft: &Draft, skill: usize) -> Option<String> {
        let list = self.kit(&draft.kit)?.sa(0x128);
        if list.is_empty() || list.iter().any(|n| rogue_skill_of(n) == Some(skill)) {
            return None;
        }
        let mut text = cr_i18n::tr(
            "The selected kit requires that starting rogue points be spent only on skills \
             from the following list.",
        )
        .to_owned();
        for name in list {
            text.push('\n');
            text.push_str(name);
        }
        Some(text)
    }

    /// Writes the kit's 1st-level rogue bonus and chosen skills as the
    /// original keeps them (+0x15e4: `CPartKitBonusOb`s of name, level
    /// and bonus), adding the bonus to its skill.
    pub(super) fn write_kit_rogue_bonuses(&self, draft: &Draft, d: &mut CharacterData) {
        let bonus = |name: &str, level: i32, amount: i32| {
            Box::new(Object::KitBonus(KitBonus {
                version: 1,
                name: name.to_owned(),
                unknown_0x8: level,
                unknown_0xc: amount,
            }))
        };
        let taken = self.kit_rogue_bonus_taken(draft);
        if let Some((name, _, amount)) = &taken
            && let Some((_, values)) = d.rogue_skills.entries.iter_mut().find(|(n, _)| n == name)
        {
            values[0] = values[0].wrapping_add_signed(*amount);
        }
        let Some(lists) = &mut d.unknown_0x15e4 else {
            return;
        };
        lists[0] = taken
            .iter()
            .map(|(name, _, amount)| Some(bonus(name, 1, *amount)))
            .collect();
        lists[1] = match self.kit_chosen_skills(draft) {
            Some((count, amount, names)) => draft
                .rogue_chosen
                .iter()
                .filter(|n| names.contains(n))
                .take(count)
                .map(|n| Some(bonus(n, 1, amount)))
                .collect(),
            None => Vec::new(),
        };
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
