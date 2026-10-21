//! What Level Details (IDD 335) edits for one level at a time beyond its
//! numbers: the free spells (IDD 144) with the Free Spell Summary, the
//! NWPs granted and modified at a level (IDD 172), the class ability a
//! level grants (the Add/Remove dialog, 0x6f26d1), and Auto Fill (IDD 224),
//! which copies Level Details or More from another class.

use cr_formats::Object;
use cr_formats::classes::{ClassData, ClassFreeSpells, KitBonus};
use cr_i18n::{fill, tr, tr_noop};

use super::super::slots;

/// One spell a level's free spells give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeSpell {
    pub name: String,
    /// How many times it may be cast.
    pub uses: i32,
    /// Cast "as the Wizard Spell" (a wizard spell, type 0).
    pub wizard: bool,
}

/// The free spells of one level (a `CClassFreeSpellsOb`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeSpellLevel {
    pub level: i32,
    /// The uses spells added from now on get ("The spells may be cast
    /// this often:").
    pub uses: i32,
    /// "per day", "per week", "at will" or any text of the user's.
    pub frequency: String,
    /// "Must select one spell from the list".
    pub pick_one: bool,
    pub spells: Vec<FreeSpell>,
    /// A member no dialog shows, kept as it was.
    pub unknown_0x4: Vec<String>,
}

impl FreeSpellLevel {
    /// A level with no spells yet, as the object's constructor (0x6b6e9e)
    /// makes it: 1 "per day".
    pub fn new(level: i32) -> Self {
        Self {
            level,
            uses: 1,
            frequency: FREQUENCIES[0].to_owned(),
            pick_one: false,
            spells: Vec::new(),
            unknown_0x4: Vec::new(),
        }
    }
}

/// The frequency combo's items (IDD 144's 1129, which takes any text of
/// up to [`FREQUENCY_MAX`] characters too). They are stored as written.
pub const FREQUENCIES: [&str; 3] = [
    tr_noop!("per day"),
    tr_noop!("per week"),
    tr_noop!("at will"),
];
pub const FREQUENCY_MAX: usize = 40;

/// The object version the Expansion writes.
const FREE_SPELLS_VERSION: i32 = 4;

/// A class's free spells, one entry per level object, in stored order.
pub fn free_spells(d: &ClassData) -> Vec<FreeSpellLevel> {
    d.free_spells
        .iter()
        .filter_map(|o| match o {
            Object::ClassFreeSpells(f) => Some(free_spell_level(f)),
            _ => None,
        })
        .collect()
}

fn free_spell_level(f: &ClassFreeSpells) -> FreeSpellLevel {
    let spells = f
        .spells
        .iter()
        .flatten()
        .filter_map(|o| match o {
            // Below version 4 the second member is the type and the uses
            // are the object's (0x6b7286).
            Object::KitBonus(b) if f.version < FREE_SPELLS_VERSION => Some(FreeSpell {
                name: b.name.clone(),
                uses: f.uses,
                wizard: b.unknown_0x8 == 0,
            }),
            Object::KitBonus(b) => Some(FreeSpell {
                name: b.name.clone(),
                uses: b.unknown_0x8,
                wizard: b.unknown_0xc == 0,
            }),
            _ => None,
        })
        .collect();
    FreeSpellLevel {
        level: f.level,
        uses: f.uses,
        frequency: f
            .frequency
            .clone()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| FREQUENCIES[0].to_owned()),
        pick_one: f.pick_one != 0,
        spells,
        unknown_0x4: f.unknown_0x4.clone(),
    }
}

/// Sets a class's free spells; a level with no spells is dropped, as an
/// empty Selected list deletes its object (0x668ed0). Objects of other
/// kinds in the array stay.
pub fn set_free_spells(d: &mut ClassData, levels: Vec<FreeSpellLevel>) {
    d.free_spells
        .retain(|o| !matches!(o, Object::ClassFreeSpells(_)));
    d.free_spells.extend(
        levels
            .into_iter()
            .filter(|l| !l.spells.is_empty())
            .map(|l| {
                Object::ClassFreeSpells(ClassFreeSpells {
                    version: FREE_SPELLS_VERSION,
                    level: l.level,
                    uses: l.uses,
                    frequency: Some(l.frequency),
                    pick_one: l.pick_one.into(),
                    unknown_0x4: l.unknown_0x4,
                    spells: Some(
                        l.spells
                            .into_iter()
                            .map(|s| {
                                Object::KitBonus(KitBonus {
                                    version: 1,
                                    name: s.name,
                                    unknown_0x8: s.uses,
                                    unknown_0xc: (!s.wizard).into(),
                                })
                            })
                            .collect(),
                    ),
                })
            }),
    );
}

/// The Free Spell Summary lines of one level (0x667936): one line, or one
/// per number of uses when the spells differ in it. "At level 3, Clerics
/// are able to cast Bless or Light once a day".
pub fn free_spell_lines(class: &str, level: &FreeSpellLevel) -> Vec<String> {
    lines(class, level, cr_i18n::tr)
}

/// [`free_spell_lines`] in the original's words, as the character sheet
/// prints them.
pub fn free_spell_lines_untranslated(class: &str, level: &FreeSpellLevel) -> Vec<String> {
    lines(class, level, |t| t)
}

fn lines(class: &str, level: &FreeSpellLevel, t: fn(&'static str) -> &'static str) -> Vec<String> {
    let Some(first) = level.spells.first() else {
        return Vec::new();
    };
    let groups: Vec<(i32, Vec<&FreeSpell>)> = if level.spells.iter().all(|s| s.uses == first.uses) {
        vec![(first.uses, level.spells.iter().collect())]
    } else {
        let most = level.spells.iter().map(|s| s.uses).max().unwrap_or(0);
        (0..=most)
            .map(|c| (c, level.spells.iter().filter(|s| s.uses == c).collect()))
            .filter(|(_, spells): &(i32, Vec<&FreeSpell>)| !spells.is_empty())
            .collect()
    };
    // The class's name made plural as the original does.
    let classes = if class.ends_with(['s', 'S']) {
        format!("{class}es")
    } else {
        format!("{class}s")
    };
    groups
        .into_iter()
        .map(|(uses, spells)| {
            let names: Vec<String> = spells
                .iter()
                .map(|s| {
                    if s.wizard {
                        fill(
                            t(tr_noop!("{spell} as the Wizard Spell")),
                            &[("spell", &s.name)],
                        )
                    } else {
                        s.name.clone()
                    }
                })
                .collect();
            let spells = match names.split_last() {
                Some((last, rest)) if level.pick_one && !rest.is_empty() => fill(
                    t(tr_noop!("{list} or {last}")),
                    &[("list", &rest.join(", ")), ("last", last)],
                ),
                _ => names.join(", "),
            };
            let line = fill(
                t(tr_noop!(
                    "At level {level}, {classes} are able to cast {spells}"
                )),
                &[
                    ("level", &level.level),
                    ("classes", &classes),
                    ("spells", &spells),
                ],
            );
            let template = match (level.frequency.as_str(), uses) {
                ("per day", 1) => tr_noop!("{line} once a day"),
                ("per day", _) => tr_noop!("{line} {n} times a day"),
                ("per week", 1) => tr_noop!("{line} once a week"),
                ("per week", _) => tr_noop!("{line} {n} times a week"),
                ("at will", _) => tr_noop!("{line} at will"),
                (other, n) => return format!("{line} {n} {other}"),
            };
            fill(t(template), &[("line", &line), ("n", &uses)])
        })
        .collect()
}

/// The NWPs one level grants and modifies (IDD 172).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LevelNwps {
    pub level: i32,
    /// "Bonus NWPs granted at current level".
    pub granted: Vec<String>,
    /// "May only pick one form the list above".
    pub pick_one: bool,
    /// NWP score modifications, 0 to 100.
    pub modifications: Vec<(String, i32)>,
}

/// The highest NWP score modification.
pub const MODIFICATION_MAX: i32 = 100;

fn bonuses(objects: &[Object]) -> impl Iterator<Item = &KitBonus> {
    objects.iter().filter_map(|o| match o {
        Object::KitBonus(b) => Some(b),
        _ => None,
    })
}

/// A class's per-level NWPs, by level.
pub fn level_nwps(d: &ClassData) -> Vec<LevelNwps> {
    let mut levels: Vec<LevelNwps> = Vec::new();
    let mut at = |level: i32| -> usize {
        match levels.iter().position(|l| l.level == level) {
            Some(i) => i,
            None => {
                levels.push(LevelNwps {
                    level,
                    ..Default::default()
                });
                levels.len() - 1
            }
        }
    };
    let mut granted = Vec::new();
    for b in bonuses(&d.bonus_nwps) {
        granted.push((at(b.unknown_0x8), b));
    }
    let mut modified = Vec::new();
    for b in bonuses(&d.nwp_modifications) {
        modified.push((at(b.unknown_0x8), b));
    }
    for (i, b) in granted {
        levels[i].granted.push(b.name.clone());
        levels[i].pick_one |= b.unknown_0xc != 0;
    }
    for (i, b) in modified {
        levels[i]
            .modifications
            .push((b.name.clone(), b.unknown_0xc));
    }
    levels.sort_by_key(|l| l.level);
    levels
}

/// Sets a class's per-level NWPs; every granted NWP of a level carries
/// its pick-one flag, as IDD 172's OK sets them (0x710dc2).
pub fn set_level_nwps(d: &mut ClassData, levels: Vec<LevelNwps>) {
    let bonus = |name: String, level: i32, value: i32| {
        Object::KitBonus(KitBonus {
            version: 1,
            name,
            unknown_0x8: level,
            unknown_0xc: value,
        })
    };
    d.bonus_nwps.retain(|o| !matches!(o, Object::KitBonus(_)));
    d.nwp_modifications
        .retain(|o| !matches!(o, Object::KitBonus(_)));
    for l in levels {
        d.bonus_nwps.extend(
            l.granted
                .into_iter()
                .map(|n| bonus(n, l.level, l.pick_one.into())),
        );
        d.nwp_modifications.extend(
            l.modifications
                .into_iter()
                .map(|(n, v)| bonus(n, l.level, v.clamp(0, MODIFICATION_MAX))),
        );
    }
}

/// What a level's NWPs come to, in a line.
pub fn level_nwps_summary(l: &LevelNwps) -> String {
    let mut parts = Vec::new();
    if !l.granted.is_empty() {
        let list = l.granted.join(", ");
        parts.push(if l.pick_one {
            tr!("Pick one of: {list}", list)
        } else {
            tr!("Free: {list}", list)
        });
    }
    if !l.modifications.is_empty() {
        let list = l
            .modifications
            .iter()
            .map(|(n, v)| format!("{n} {v:+}"))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(tr!("Score: {list}", list));
    }
    parts.join(" · ")
}

/// How many levels the per-level class abilities cover (0 to 100).
const LEVELS: usize = 101;

/// The class abilities levels grant, as (ability, level) pairs.
pub fn level_abilities(d: &ClassData) -> Vec<(String, i64)> {
    d.level_abilities
        .iter()
        .flatten()
        .enumerate()
        .filter(|(level, name)| *level > 0 && !name.is_empty())
        .map(|(level, name)| (name.clone(), level as i64))
        .collect()
}

/// Sets the class abilities levels grant; the array gets its 101 names
/// once one is given (0x6f26d1). A class of a version without the array
/// keeps none.
pub fn set_level_abilities(d: &mut ClassData, pairs: &[(String, i64)]) {
    let Some(array) = d.level_abilities.as_mut() else {
        return;
    };
    if array.is_empty() && pairs.is_empty() {
        return;
    }
    array.clear();
    array.resize(LEVELS, String::new());
    for (name, level) in pairs {
        if let Some(slot) = usize::try_from(*level)
            .ok()
            .filter(|l| (1..LEVELS).contains(l))
            .and_then(|l| array.get_mut(l))
        {
            name.clone_into(slot);
        }
    }
}

/// The prompt of the class ability picker (0x6f26d1).
pub fn ability_prompt(level: i64) -> String {
    tr!(
        "Select one class ability which you will get for free at level {level}.",
        level
    )
}

/// What Auto Fill copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoFill {
    /// Level Details (0x669a3d).
    Levels,
    /// More (0x669da9).
    More,
}

/// The heading of Auto Fill's question.
pub const AUTO_FILL_HEADING: &str = tr_noop!("Warning Message");

/// Auto Fill's question before it copies `class` (0x711b3e).
pub fn auto_fill_question(class: &str) -> String {
    tr!(
        "If you continue, the next screen will be filled with the information\nstored in the class {class} and any information you already entered for that screen will be over-written.\nContinue Anyway?",
        class
    )
}

/// The members More's Auto Fill copies (0x669da9), by offset: hit points,
/// NWP and WP slots, magic, gender, rogue skill points, the
/// abbreviation, the default abilities and spheres lists and the
/// reminders.
const MORE_MEMBERS: [usize; 37] = [
    0x84, 0x88, 0x8c, 0x90, 0x94, 0x98, 0x9c, 0x4860, 0x4864, 0x4865, 0xbc, 0xc0, 0xc4, 0xcc, 0xd0,
    0xd4, 0xd8, 0xdc, 0xe0, 0x2774, 0x2778, 0x277c, 0x2780, 0x2784, 0x4868, 0x3550, 0xb4, 0xb8,
    0x354c, 0x2724, 0x2738, 0x2760, 0x274c, 0x3558, 0x353c, 0x3540, 0x3544,
];

fn copy_option<T: Clone>(to: &mut Option<T>, from: &Option<T>) {
    if let (Some(to), Some(from)) = (to.as_mut(), from) {
        to.clone_from(from);
    }
}

/// Copies what `fill` covers from `from` into `to`. Level Details takes
/// every level, not the current one: the maximum level, experience,
/// THAC0, AC, attacks, spell progressions, saving throws, ability score
/// bonuses, free spells and NWPs; not the class abilities by level.
pub fn auto_fill(to: &mut ClassData, from: &ClassData, fill: AutoFill) {
    match fill {
        AutoFill::Levels => {
            to.unknown_0x54[1] = from.unknown_0x54[1];
            to.experience = from.experience;
            copy_option(&mut to.unknown_0x27c, &from.unknown_0x27c);
            copy_option(&mut to.unknown_0x3380, &from.unknown_0x3380);
            copy_option(&mut to.unknown_0x356c, &from.unknown_0x356c);
            copy_option(&mut to.unknown_0x3050, &from.unknown_0x3050);
            copy_option(&mut to.unknown_0x31e8, &from.unknown_0x31e8);
            to.unknown_0x414 = from.unknown_0x414;
            to.unknown_0x159c = from.unknown_0x159c;
            copy_option(&mut to.unknown_0x27a8, &from.unknown_0x27a8);
            to.free_spells.clone_from(&from.free_spells);
            to.bonus_nwps.clone_from(&from.bonus_nwps);
            to.nwp_modifications.clone_from(&from.nwp_modifications);
        }
        AutoFill::More => {
            for off in MORE_MEMBERS {
                if let (Some(value), Some(slot)) = (
                    slots::class(from, off).map(slots::Slot::value),
                    slots::class_mut(to, off),
                ) {
                    slot.set(&value);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class() -> ClassData {
        match super::super::new_class() {
            cr_formats::classes::Class::Expansion { data, .. } => data,
            cr_formats::classes::Class::Core20(_) => unreachable!(),
        }
    }

    fn spell(name: &str, uses: i32, wizard: bool) -> FreeSpell {
        FreeSpell {
            name: name.into(),
            uses,
            wizard,
        }
    }

    #[test]
    fn free_spells_round_trip_and_empty_levels_go() {
        let mut d = class();
        let mut three = FreeSpellLevel::new(3);
        three.spells = vec![spell("Bless", 1, false), spell("Light", 1, true)];
        set_free_spells(&mut d, vec![three.clone(), FreeSpellLevel::new(5)]);
        assert_eq!(d.free_spells.len(), 1);
        assert_eq!(free_spells(&d), [three]);
    }

    #[test]
    fn old_free_spells_take_the_object_uses() {
        let old = ClassFreeSpells {
            version: 2,
            level: 1,
            uses: 2,
            frequency: None,
            pick_one: 0,
            unknown_0x4: vec![],
            spells: Some(vec![Object::KitBonus(KitBonus {
                version: 1,
                name: "Bless".into(),
                unknown_0x8: 1,
                unknown_0xc: 0,
            })]),
        };
        let level = free_spell_level(&old);
        assert_eq!(level.spells, [spell("Bless", 2, false)]);
        assert_eq!(level.frequency, "per day");
    }

    #[test]
    fn the_summary_follows_the_uses_and_frequency() {
        let mut l = FreeSpellLevel::new(2);
        l.spells = vec![spell("A", 1, false), spell("B", 1, false)];
        let lines = free_spell_lines("Druid", &l);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Druids"));
        assert!(lines[0].ends_with("once a day"));
        l.pick_one = true;
        assert!(free_spell_lines("Druid", &l)[0].contains(" or "));
        l.spells[1].uses = 3;
        l.frequency = "per week".into();
        let lines = free_spell_lines("Priests", &l);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("Priestses"));
        assert!(lines[1].ends_with("3 times a week"));
        l.frequency = "per moon".into();
        assert!(free_spell_lines("Druid", &l)[0].ends_with("1 per moon"));
    }

    #[test]
    fn level_nwps_round_trip() {
        let mut d = class();
        let levels = vec![
            LevelNwps {
                level: 1,
                granted: vec!["Tracking".into()],
                pick_one: false,
                modifications: vec![("Tracking".into(), 5)],
            },
            LevelNwps {
                level: 4,
                granted: vec!["Riding".into(), "Swimming".into()],
                pick_one: true,
                modifications: vec![],
            },
        ];
        set_level_nwps(&mut d, levels.clone());
        assert_eq!(d.bonus_nwps.len(), 3);
        assert_eq!(d.nwp_modifications.len(), 1);
        assert_eq!(level_nwps(&d), levels);
    }

    #[test]
    fn level_abilities_fill_the_array_once_given() {
        let mut d = class();
        set_level_abilities(&mut d, &[]);
        assert!(d.level_abilities.as_ref().is_some_and(Vec::is_empty));
        set_level_abilities(&mut d, &[("Turn undead".into(), 3)]);
        assert_eq!(d.level_abilities.as_ref().map(Vec::len), Some(LEVELS));
        assert_eq!(level_abilities(&d), [("Turn undead".to_owned(), 3)]);
    }

    #[test]
    fn auto_fill_copies_its_screen() {
        let mut from = class();
        from.unknown_0x54[1] = 20;
        from.experience[2] = 2000;
        from.unknown_0x84 = [0, 10, 0];
        let mut to = class();
        auto_fill(&mut to, &from, AutoFill::Levels);
        assert_eq!((to.unknown_0x54[1], to.experience[2]), (20, 2000));
        assert_ne!(to.unknown_0x84, from.unknown_0x84);
        auto_fill(&mut to, &from, AutoFill::More);
        assert_eq!(to.unknown_0x84, from.unknown_0x84);
        for off in MORE_MEMBERS {
            assert!(slots::class(&from, off).is_some(), "{off:#x}");
        }
    }
}
