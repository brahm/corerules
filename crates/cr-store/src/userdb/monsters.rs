//! Monsters and monster types (`UserDbas/MonsterU.dat`,
//! `UserDbas/MonTypeU.dat`): the Monsters book (IDD 261), the type editor
//! (IDD 294) and the monster editor (IDD 263) with its sub-dialogs.
//!
//! Attack types (`AttackU.dat`) are only read: a monster's Attacks page
//! picks from `Attack.dat` and `AttackU.dat`, and the chosen attacks are
//! stored in the monster as copies of those records.

use cr_formats::Object;
use cr_formats::inventory::InventoryBundle;
use cr_formats::monster::{Arsenal, Monster, MonsterAttack, MonsterType};
use cr_i18n::{tr, tr_noop};

use super::{
    FieldDesc, FieldKind, Flavor, NameError, NewContext, Options, UserKind, Value, default_reason,
    udb_combo, udb_int, udb_multiline, udb_text,
};
use crate::pictures::{Kind, split_stored};
use crate::treasure::LETTERS;
use crate::{Installation, Result};

pub struct MonsterTypes;

impl UserKind for MonsterTypes {
    type Rec = MonsterType;
    const NOUN: &'static str = tr_noop!("Monster Type");
    const FILE: &'static str = "UserDbas/MonTypeU.dat";

    fn name(r: &MonsterType) -> &str {
        &r.name
    }

    fn set_name(r: &mut MonsterType, name: &str) {
        name.clone_into(&mut r.name);
    }

    fn standard(install: &Installation) -> Result<Vec<MonsterType>> {
        super::standard_list(install, "MonType", pick_type)
    }

    fn parse_user(data: &[u8]) -> std::result::Result<Vec<MonsterType>, cr_formats::Error> {
        super::parse_list(data).map(|l| l.into_iter().filter_map(pick_type).collect())
    }

    fn user_bytes(records: &[MonsterType]) -> std::result::Result<Vec<u8>, cr_formats::Error> {
        super::list_bytes(
            records
                .iter()
                .map(|t| Object::MonsterType(Box::new(t.clone())))
                .collect(),
        )
    }

    fn new_record(_context: &NewContext) -> MonsterType {
        MonsterType {
            name: String::new(),
            page_picture: String::new(),
            unknown_0x14: String::new(),
            unknown_0x10: String::new(),
            unknown_0x18: 0,
            unknown_0x1c: 0,
            picture: String::new(),
        }
    }

    fn fields(_flavor: Flavor) -> Vec<FieldDesc<MonsterType>> {
        type Rec = MonsterType;
        const PAGE: &str = tr_noop!("Monster Type");
        vec![
            udb_text!(PAGE, tr_noop!("Name of Type"), 50, name),
            // The original's editor keeps the path whole and split into its
            // file and folder, and no page strip.
            FieldDesc::new(
                PAGE,
                tr_noop!("Bitmap"),
                FieldKind::Picture(Kind::Monster),
                |r: &Rec, _| Value::Text(r.picture.clone()),
                |r, _, v, _| {
                    r.picture = v.text();
                    let (folder, file) = split_stored(&r.picture);
                    r.unknown_0x14 = file.to_owned();
                    r.unknown_0x10 = folder.to_owned();
                    r.page_picture.clear();
                },
            ),
        ]
    }

    fn check_name(name: &str) -> Option<NameError> {
        name.starts_with('~').then_some(NameError::Reserved)
    }

    fn name_message(error: &NameError, _r: &MonsterType, _adding: bool) -> String {
        match error {
            NameError::Reserved => default_reason(error),
            e => tr!(
                "Monster Type not added: Error. {reason}",
                reason = default_reason(e)
            ),
        }
    }
}

fn pick_type(o: Object) -> Option<MonsterType> {
    match o {
        Object::MonsterType(t) => Some(*t),
        _ => None,
    }
}

pub struct Monsters;

/// The Frequency radio of IDD 263: the percentage each button sets.
pub const FREQUENCIES: [(&str, i32); 4] = [
    (tr_noop!("Very Rare (4%)"), 4),
    (tr_noop!("Rare (11%)"), 11),
    (tr_noop!("Uncommon (20%)"), 20),
    (tr_noop!("Common (65%)"), 65),
];

const FREQUENCY_NAMES: &[&str] = &[
    tr_noop!("Very Rare (4%)"),
    tr_noop!("Rare (11%)"),
    tr_noop!("Uncommon (20%)"),
    tr_noop!("Common (65%)"),
    tr_noop!("Other"),
];

fn frequency_index(f: i32) -> usize {
    FREQUENCIES
        .iter()
        .position(|(_, p)| *p == f)
        .unwrap_or(FREQUENCIES.len())
}

/// The Size check boxes (+0x47c to +0x490).
const SIZES: [&str; 6] = [
    tr_noop!("Tiny"),
    tr_noop!("Small"),
    tr_noop!("Man-Sized"),
    tr_noop!("Large"),
    tr_noop!("Huge"),
    tr_noop!("Gargantuan"),
];

/// The Special Defenses check boxes (+0x44c to +0x460).
const DEFENSES: [&str; 6] = [
    tr_noop!("Hit Only by Magical/Silver Weapons"),
    tr_noop!("Immunity to Any Spell"),
    tr_noop!("Immunity to Any Weapon (Including ½ Damage)"),
    tr_noop!("Invisible at Will"),
    tr_noop!("Regeneration"),
    tr_noop!("Special Defense Form"),
];

/// The Alignment combo.
pub const ALIGNMENTS: &[&str] = &[
    "Lawful Good",
    "Lawful Neutral",
    "Lawful Evil",
    "Neutral Good",
    "True Neutral",
    "Neutral Evil",
    "Chaotic Good",
    "Chaotic Neutral",
    "Chaotic Evil",
];

/// Climates (+0x4 [0..5]) and terrains ([5..37]) of the Terrain and
/// Climate dialog (IDD 273), in flag order.
pub const CLIMATES: [&str; 5] = [
    tr_noop!("Arctic"),
    tr_noop!("Sub-Arctic"),
    tr_noop!("Temperate"),
    tr_noop!("Sub-Tropical"),
    tr_noop!("Tropical"),
];
pub const TERRAINS: [&str; 32] = [
    tr_noop!("Aerial"),
    tr_noop!("Bush"),
    tr_noop!("Caves"),
    tr_noop!("Coastal"),
    tr_noop!("Desert"),
    tr_noop!("Farmland"),
    tr_noop!("Forest"),
    tr_noop!("Glacier"),
    tr_noop!("Grasslands"),
    tr_noop!("Jungle"),
    tr_noop!("Lake - Fresh Water"),
    tr_noop!("Lake - Salt Water"),
    tr_noop!("Mountains, high"),
    tr_noop!("Mountains, high - Forest"),
    tr_noop!("Mountains, high - Lake"),
    tr_noop!("Mountains, low"),
    tr_noop!("Mountains, low - Forest"),
    tr_noop!("Mountains, low - Lake"),
    tr_noop!("Ocean Deep"),
    tr_noop!("Ocean Shallows"),
    tr_noop!("Parkland"),
    tr_noop!("Plain/Scrub"),
    tr_noop!("Rain Forest"),
    tr_noop!("River - Fresh"),
    tr_noop!("River - Salt"),
    tr_noop!("Rough/Hill"),
    tr_noop!("Salt Marsh"),
    tr_noop!("Steppe"),
    tr_noop!("Subterranean"),
    tr_noop!("Swamp - Fresh"),
    tr_noop!("Swamp - Salt"),
    tr_noop!("Tundra"),
];

/// The climate/terrain category whose flags the user picks.
pub const CUSTOM_TERRAIN: &str = "~Custom";

fn custom_terrain(m: &Monster) -> bool {
    m.climate_terrain == CUSTOM_TERRAIN
}

impl UserKind for Monsters {
    type Rec = Monster;
    const NOUN: &'static str = tr_noop!("Monster");
    const FILE: &'static str = "UserDbas/MonsterU.dat";

    fn name(r: &Monster) -> &str {
        &r.name
    }

    fn set_name(r: &mut Monster, name: &str) {
        name.clone_into(&mut r.name);
    }

    fn standard(install: &Installation) -> Result<Vec<Monster>> {
        super::standard_list(install, "Monster", pick_monster)
    }

    fn parse_user(data: &[u8]) -> std::result::Result<Vec<Monster>, cr_formats::Error> {
        super::parse_list(data).map(|l| l.into_iter().filter_map(pick_monster).collect())
    }

    fn user_bytes(records: &[Monster]) -> std::result::Result<Vec<u8>, cr_formats::Error> {
        super::list_bytes(
            records
                .iter()
                .map(|m| Object::Monster(Box::new(m.clone())))
                .collect(),
        )
    }

    /// The monster editor's defaults (screen `t3_monster_editor_new`):
    /// Other 0%, hit dice 0d8+0, morale 2/2, AC 10, the other armor
    /// classes 100 (none); the type is the one selected in the book.
    fn new_record(context: &NewContext) -> Monster {
        Monster {
            name: String::new(),
            description: String::new(),
            picture: 0,
            unknown_0x3b8: 0,
            frequency: 0,
            organization: String::new(),
            activity_cycle: String::new(),
            diet: String::new(),
            intelligence: String::new(),
            number_appearing_min: 0,
            number_appearing_max: 0,
            armor_class: 10,
            movement: 0,
            fly_maneuverability: String::new(),
            fly_rate: 0,
            unknown_0x414: String::new(),
            swim_rate: 0,
            unknown_0x41c: String::new(),
            burrow_rate: 0,
            unknown_0x424: String::new(),
            unknown_0x428: 0,
            unknown_0x42c: String::new(),
            unknown_0x430: 0,
            hit_dice: 0,
            unknown_0x438: 0,
            hit_die_size: 8,
            unknown_0x440: String::new(),
            unknown_0x444: String::new(),
            defense_notes: String::new(),
            unknown_0x464: String::new(),
            magic_resistance: 0,
            morale: 2,
            experience: 0,
            unknown_0x478: 0,
            thac0: 0,
            unknown_0x470: 2,
            unknown_0x3f8: 100,
            unknown_0x3fc: 100,
            unknown_0x400: 100,
            unknown_0x404: 100,
            monster_type: context.parent.clone(),
            alignment: String::new(),
            climate_terrain: String::new(),
            unknown_0x33c: String::new(),
            unknown_0x340: String::new(),
            unknown_0x344: String::new(),
            notes: String::new(),
            unknown_0x47c: [0; 6],
            unknown_0x44c: [0; 6],
            unknown_0x4: [0; 150],
            unknown_0x25c: [(0, 0); 26],
            armor_class_notes: Vec::new(),
            unknown_0x4c0: Vec::new(),
            spells: Vec::new(),
            arsenal: Arsenal {
                damage: String::new(),
                attacks: String::new(),
                special_attacks: String::new(),
                special_defenses: String::new(),
                attack_list: Vec::new(),
            },
            inventory: InventoryBundle { items: Vec::new() },
        }
    }

    fn fields(_flavor: Flavor) -> Vec<FieldDesc<Monster>> {
        type Rec = Monster;
        const MAIN: &str = tr_noop!("Monster");
        const ECOLOGY: &str = tr_noop!("Ecology");
        const COMBAT: &str = tr_noop!("Combat");
        const MOVEMENT: &str = tr_noop!("Movement");
        const AC: &str = tr_noop!("Armor Class");
        const ATTACKS: &str = tr_noop!("Attacks");
        const DEFENSES_PAGE: &str = tr_noop!("Special Defenses");
        const TERRAIN: &str = tr_noop!("Terrain");
        const TEXT: &str = tr_noop!("Text");
        let mut fields = vec![
            udb_text!(MAIN, tr_noop!("Name"), 50, name),
            udb_combo!(
                MAIN,
                tr_noop!("Type"),
                Options::Source("monster-types"),
                false,
                monster_type
            ),
            FieldDesc::new(
                MAIN,
                tr_noop!("Frequency"),
                FieldKind::Choice(Options::Fixed(FREQUENCY_NAMES)),
                |r: &Monster, _| Value::Int(frequency_index(r.frequency) as i64),
                |r, _, v, _| {
                    if let Some((_, p)) = FREQUENCIES.get(v.int() as usize) {
                        r.frequency = *p;
                    } else if frequency_index(r.frequency) < FREQUENCIES.len() {
                        r.frequency = 0;
                    }
                },
            )
            .group(tr_noop!("Frequency")),
            udb_int!(MAIN, tr_noop!("Percent"), 0, 100, frequency)
                .group(tr_noop!("Frequency"))
                .enabled(|r| frequency_index(r.frequency) == FREQUENCIES.len()),
            udb_int!(
                MAIN,
                tr_noop!("Experience Points"),
                0,
                i32::MAX as i64,
                experience
            ),
            udb_combo!(
                ECOLOGY,
                tr_noop!("Organization"),
                Options::Source("monster-organization"),
                true,
                organization
            ),
            udb_combo!(
                ECOLOGY,
                tr_noop!("Activity Cycle"),
                Options::Source("monster-activity"),
                true,
                activity_cycle
            ),
            udb_combo!(
                ECOLOGY,
                tr_noop!("Diet"),
                Options::Source("monster-diet"),
                true,
                diet
            ),
            udb_combo!(
                ECOLOGY,
                tr_noop!("Intelligence"),
                Options::Source("monster-intelligence"),
                true,
                intelligence
            ),
            udb_combo!(
                ECOLOGY,
                tr_noop!("Alignment"),
                Options::Fixed(ALIGNMENTS),
                true,
                alignment
            ),
            udb_int!(
                ECOLOGY,
                tr_noop!("Fewest"),
                0,
                10_000_000,
                number_appearing_min
            )
            .group(tr_noop!("Number Appearing")),
            udb_int!(
                ECOLOGY,
                tr_noop!("Greatest"),
                0,
                10_000_000,
                number_appearing_max
            )
            .group(tr_noop!("Number Appearing")),
            FieldDesc::new(
                ECOLOGY,
                tr_noop!("Lair"),
                FieldKind::Text { max: 0 },
                |r: &Monster, _| Value::Text(letters_text(&r.unknown_0x25c.map(|p| p.0))),
                |r, _, v, _| {
                    let counts = parse_letters(&v.text());
                    for (pair, n) in r.unknown_0x25c.iter_mut().zip(counts) {
                        pair.0 = n;
                    }
                },
            )
            .group(tr_noop!("Treasure Types")),
            FieldDesc::new(
                ECOLOGY,
                tr_noop!("Individual"),
                FieldKind::Text { max: 0 },
                |r: &Monster, _| Value::Text(letters_text(&r.unknown_0x25c.map(|p| p.1))),
                |r, _, v, _| {
                    let counts = parse_letters(&v.text());
                    for (pair, n) in r.unknown_0x25c.iter_mut().zip(counts) {
                        pair.1 = n;
                    }
                },
            )
            .group(tr_noop!("Treasure Types")),
            FieldDesc::new(
                COMBAT,
                tr_noop!("Hit Dice"),
                FieldKind::Dice,
                |r: &Monster, _| {
                    Value::Dice([
                        r.hit_dice.into(),
                        r.hit_die_size.into(),
                        r.unknown_0x438.into(),
                    ])
                },
                |r, _, v, _| {
                    let [n, s, m] = v.dice();
                    r.hit_dice = n.clamp(0, 100_000) as i32;
                    r.hit_die_size = s.clamp(1, 100) as i32;
                    r.unknown_0x438 = m.clamp(0, 100) as i32;
                },
            ),
            udb_int!(COMBAT, tr_noop!("THAC0"), 0, 20, thac0),
            udb_int!(
                COMBAT,
                tr_noop!("Magic Resistance"),
                0,
                100,
                magic_resistance
            ),
            udb_int!(COMBAT, tr_noop!("Highest"), 2, 20, morale).group(tr_noop!("Morale")),
            udb_int!(COMBAT, tr_noop!("Lowest"), 2, 20, unknown_0x470).group(tr_noop!("Morale")),
            udb_int!(MOVEMENT, tr_noop!("On Land"), 0, 1000, movement),
            udb_int!(MOVEMENT, tr_noop!("Flying"), 0, 1000, fly_rate),
            udb_combo!(
                MOVEMENT,
                tr_noop!("Maneuverability Class"),
                Options::Fixed(&["", "A", "B", "C", "D", "E"]),
                false,
                fly_maneuverability
            )
            .enabled(|r| r.fly_rate > 0),
            udb_int!(MOVEMENT, tr_noop!("Swimming"), 0, 1000, swim_rate),
            udb_int!(MOVEMENT, tr_noop!("Burrowing"), 0, 1000, burrow_rate),
            udb_int!(MOVEMENT, tr_noop!("Climbing"), 0, 1000, unknown_0x428),
            udb_int!(MOVEMENT, tr_noop!("Web"), 0, 1000, unknown_0x430),
            udb_int!(AC, tr_noop!("Armor Class"), -20, 100, armor_class),
            FieldDesc::new(
                ATTACKS,
                tr_noop!("Attacks"),
                FieldKind::Dual(Options::Source("attacks")),
                |r, _| Value::List(attack_names(r)),
                |r, _, v, sources| set_attacks(r, &v.list(), &sources.attacks),
            )
            .group(tr_noop!("Attacks")),
            udb_text!(ATTACKS, tr_noop!("No. of Attacks"), 0, arsenal.attacks)
                .group(tr_noop!("Combat Text")),
            udb_text!(ATTACKS, tr_noop!("Damage/Attack"), 0, arsenal.damage)
                .group(tr_noop!("Combat Text")),
            udb_text!(
                ATTACKS,
                tr_noop!("Special Attacks"),
                0,
                arsenal.special_attacks
            )
            .group(tr_noop!("Combat Text")),
            udb_text!(
                ATTACKS,
                tr_noop!("Special Defenses"),
                0,
                arsenal.special_defenses
            )
            .group(tr_noop!("Combat Text")),
            udb_multiline!(
                DEFENSES_PAGE,
                tr_noop!("Defense Description"),
                defense_notes
            ),
            udb_combo!(
                TERRAIN,
                tr_noop!("Category"),
                Options::Source("monster-terrain"),
                true,
                climate_terrain
            ),
            udb_multiline!(TEXT, tr_noop!("Appearance"), description),
            udb_multiline!(TEXT, tr_noop!("Habitat/Society"), unknown_0x33c),
            udb_multiline!(TEXT, tr_noop!("Ecology"), unknown_0x340),
            udb_multiline!(TEXT, tr_noop!("Variants"), unknown_0x344),
            udb_multiline!(TEXT, tr_noop!("Combat"), notes),
        ];
        for i in 0..4 {
            fields.push(other_ac(i));
        }
        for i in 0..5 {
            fields.push(ac_note(i));
        }
        for i in 0..6 {
            fields.push(size_check(i));
            fields.push(defense_check(i));
        }
        for i in 0..37 {
            fields.push(terrain_check(i));
        }
        fields
    }

    fn summary(r: &Monster) -> String {
        r.monster_type.clone()
    }
}

fn pick_monster(o: Object) -> Option<Monster> {
    match o {
        Object::Monster(m) => Some(*m),
        _ => None,
    }
}

fn size_check(i: usize) -> FieldDesc<Monster> {
    FieldDesc::new(
        tr_noop!("Combat"),
        SIZES[i],
        FieldKind::Check,
        |r: &Monster, i| Value::Bool(r.unknown_0x47c[i] != 0),
        |r, i, v, _| r.unknown_0x47c[i] = v.bool().into(),
    )
    .arg(i)
    .group(tr_noop!("Size"))
}

fn defense_check(i: usize) -> FieldDesc<Monster> {
    FieldDesc::new(
        tr_noop!("Special Defenses"),
        DEFENSES[i],
        FieldKind::Check,
        |r: &Monster, i| Value::Bool(r.unknown_0x44c[i] != 0),
        |r, i, v, _| r.unknown_0x44c[i] = v.bool().into(),
    )
    .arg(i)
    .group(tr_noop!("Defenses"))
}

fn terrain_check(i: usize) -> FieldDesc<Monster> {
    let (label, group) = if i < 5 {
        (CLIMATES[i], tr_noop!("Climates"))
    } else {
        (TERRAINS[i - 5], tr_noop!("Terrain Types"))
    };
    FieldDesc::new(
        tr_noop!("Terrain"),
        label,
        FieldKind::Check,
        |r: &Monster, i| Value::Bool(r.unknown_0x4[i] != 0),
        |r, i, v, _| r.unknown_0x4[i] = v.bool().into(),
    )
    .arg(i)
    .group(group)
    .enabled(custom_terrain)
}

fn ac_note(i: usize) -> FieldDesc<Monster> {
    const LABELS: [&str; 5] = [
        tr_noop!("Description"),
        tr_noop!("Second Description"),
        tr_noop!("Third Description"),
        tr_noop!("Fourth Description"),
        tr_noop!("Fifth Description"),
    ];
    FieldDesc::new(
        tr_noop!("Armor Class"),
        LABELS[i],
        FieldKind::Text { max: 25 },
        |r: &Monster, i| Value::Text(r.armor_class_notes.get(i).cloned().unwrap_or_default()),
        |r, i, v, _| {
            let text = v.text();
            if text.is_empty() && r.armor_class_notes.len() <= i {
                return;
            }
            // The dialog writes all five descriptions once any is set.
            r.armor_class_notes.resize(5, String::new());
            r.armor_class_notes[i] = text;
        },
    )
    .arg(i)
    .group(tr_noop!("Descriptions"))
}

fn other_ac(i: usize) -> FieldDesc<Monster> {
    const LABELS: [&str; 4] = [
        tr_noop!("Second"),
        tr_noop!("Third"),
        tr_noop!("Fourth"),
        tr_noop!("Fifth"),
    ];
    fn member(r: &mut Monster, i: usize) -> &mut i32 {
        match i {
            0 => &mut r.unknown_0x3f8,
            1 => &mut r.unknown_0x3fc,
            2 => &mut r.unknown_0x400,
            _ => &mut r.unknown_0x404,
        }
    }
    FieldDesc::new(
        tr_noop!("Armor Class"),
        LABELS[i],
        FieldKind::Int { min: -20, max: 100 },
        |r: &Monster, i| {
            Value::Int(
                match i {
                    0 => r.unknown_0x3f8,
                    1 => r.unknown_0x3fc,
                    2 => r.unknown_0x400,
                    _ => r.unknown_0x404,
                }
                .into(),
            )
        },
        |r, i, v, _| *member(r, i) = v.int() as i32,
    )
    .arg(i)
    .group(tr_noop!("Other Armor Classes (100 for None)"))
}

/// The names of a monster's attacks (the Attacks list of IDD 271).
pub fn attack_names(m: &Monster) -> Vec<String> {
    m.arsenal
        .attack_list
        .iter()
        .filter_map(|o| match o {
            Object::MonsterAttack(a) => Some(a.name.clone()),
            _ => None,
        })
        .collect()
}

/// Sets a monster's attacks to `names`: attacks it already has are kept,
/// new ones are copied from the attack types, as Add does.
pub fn set_attacks(m: &mut Monster, names: &[String], types: &[MonsterAttack]) {
    let old = std::mem::take(&mut m.arsenal.attack_list);
    for name in names {
        let kept = old
            .iter()
            .find(|o| matches!(o, Object::MonsterAttack(a) if &a.name == name));
        let attack = match kept {
            Some(o) => Some(o.clone()),
            None => types
                .iter()
                .find(|a| &a.name == name)
                .map(|a| Object::MonsterAttack(Box::new(a.clone()))),
        };
        m.arsenal.attack_list.extend(attack);
    }
}

/// Treasure letters as the book writes them: "A, Bx2, Q".
pub fn letters_text(counts: &[i32; 26]) -> String {
    counts
        .iter()
        .zip(LETTERS)
        .filter(|(n, _)| **n != 0)
        .map(|(n, l)| {
            if *n == 1 {
                l.to_string()
            } else {
                format!("{l}x{n}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Reads "A, Bx2 Q" leniently: other characters are ignored.
pub fn parse_letters(text: &str) -> [i32; 26] {
    let mut counts = [0; 26];
    for token in text.split([',', ' ', '\t']).filter(|t| !t.is_empty()) {
        let mut chars = token.chars();
        let Some(letter) = chars.next().map(|c| c.to_ascii_uppercase()) else {
            continue;
        };
        if !letter.is_ascii_uppercase() {
            continue;
        }
        let rest: String = chars.collect();
        let n = rest
            .strip_prefix(['x', 'X'])
            .unwrap_or(&rest)
            .parse()
            .unwrap_or(1);
        counts[(letter as u8 - b'A') as usize] += n;
    }
    counts
}

/// Distinct values of a text member of the standard monsters, for the
/// editor's combos: `monster-organization`, `monster-activity`,
/// `monster-diet`, `monster-intelligence`, `monster-terrain`.
pub fn distinct(install: &Installation, source: &str) -> Result<Vec<String>> {
    let field: fn(&Monster) -> &String = match source {
        "monster-organization" => |m| &m.organization,
        "monster-activity" => |m| &m.activity_cycle,
        "monster-diet" => |m| &m.diet,
        "monster-intelligence" => |m| &m.intelligence,
        "monster-terrain" => |m| &m.climate_terrain,
        _ => return Ok(Vec::new()),
    };
    let mut values: Vec<String> = Monsters::standard(install)?
        .iter()
        .map(|m| field(m).clone())
        .filter(|v| !v.is_empty())
        .collect();
    if source == "monster-terrain" {
        values.push(CUSTOM_TERRAIN.into());
    }
    values.sort();
    values.dedup();
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userdb::testing::TempInstall;
    use crate::userdb::{Sources, UserDb};

    fn context(parent: &str) -> NewContext {
        NewContext {
            flavor: Flavor::Expansion,
            scope: 0,
            parent: parent.into(),
        }
    }

    #[test]
    fn type_names_with_a_tilde_are_reserved() {
        let t = TempInstall::new("montypes", &[]);
        let mut db = UserDb::<MonsterTypes>::load(&t.install).unwrap();
        let mut ty = db.new_record(0, "");
        ty.name = "~All".into();
        let error = db.add(ty.clone()).unwrap_err();
        assert_eq!(
            MonsterTypes::name_message(&error, &ty, true),
            "Monster Type names beginning with '~' are reserved.\n Please enter a different name"
        );
        ty.name = "Beasts".into();
        db.add(ty).unwrap();
        db.save(&t.install).unwrap();
        assert!(t.root.join("UserDbas/MonTypeU.dat").exists());
    }

    #[test]
    fn a_type_keeps_its_picture_as_the_original_writes_it() {
        let mut ty = MonsterTypes::new_record(&context(""));
        let fields = MonsterTypes::fields(Flavor::Expansion);
        let bitmap = fields.iter().find(|f| f.label == "Bitmap").unwrap();
        let stored = "C:\\Program Files\\AD&D Core Rules 2.0\\Userbmp\\teste.bmp";
        bitmap.set(&mut ty, &Value::Text(stored.into()), &Sources::default());
        assert_eq!(ty.picture, stored);
        assert_eq!(ty.page_picture, "");
        assert_eq!(ty.unknown_0x14, "teste.bmp");
        assert_eq!(
            ty.unknown_0x10,
            "C:\\Program Files\\AD&D Core Rules 2.0\\Userbmp\\"
        );
        assert_eq!(bitmap.get(&ty), Value::Text(stored.into()));
        bitmap.set(&mut ty, &Value::Text(String::new()), &Sources::default());
        assert_eq!(
            [
                ty.picture,
                ty.page_picture,
                ty.unknown_0x14,
                ty.unknown_0x10
            ],
            [""; 4].map(String::from)
        );
    }

    #[test]
    fn monster_fields_write_their_members() {
        let mut m = Monsters::new_record(&context("Udbtype"));
        assert_eq!(m.monster_type, "Udbtype");
        let fields = Monsters::fields(Flavor::Expansion);
        let field = |label: &str| fields.iter().find(|f| f.label == label).unwrap();
        let sources = Sources {
            attacks: vec![MonsterAttack {
                name: "Poison".into(),
                unknown_0xc: String::new(),
                unknown_0x8: 0,
                unknown_0x10: [0; 10],
                unknown_0x3c: 0,
                unknown_0x38: 0,
                unknown_0x40: 2,
                unknown_0x44: String::new(),
            }],
            ..Default::default()
        };
        field("Frequency").set(&mut m, &Value::Int(3), &sources);
        assert_eq!(m.frequency, 65);
        assert!(!field("Percent").is_enabled(&m));
        field("Frequency").set(&mut m, &Value::Int(4), &sources);
        assert_eq!(m.frequency, 0);
        field("Hit Dice").set(&mut m, &Value::Dice([3, 8, 0]), &sources);
        field("Man-Sized").set(&mut m, &Value::Bool(true), &sources);
        field("Attacks").set(&mut m, &Value::List(vec!["Poison".into()]), &sources);
        field("Lair").set(&mut m, &Value::Text("c, Hx3".into()), &sources);
        assert_eq!(field("Lair").get(&m), Value::Text("C, Hx3".into()));
        assert_eq!((m.hit_dice, m.hit_die_size), (3, 8));
        assert_eq!(m.unknown_0x47c, [0, 0, 1, 0, 0, 0]);
        assert_eq!(attack_names(&m), ["Poison"]);
        assert!(!field("Arctic").is_enabled(&m));
        m.climate_terrain = CUSTOM_TERRAIN.into();
        assert!(field("Arctic").is_enabled(&m));
        field("Tundra").set(&mut m, &Value::Bool(true), &sources);
        assert_eq!(m.unknown_0x4[36], 1);
        // One AC description fills all five.
        field("Second Description").set(&mut m, &Value::Text("Underside".into()), &sources);
        assert_eq!(m.armor_class_notes, ["", "Underside", "", "", ""]);
        field("Description").set(&mut m, &Value::Text(String::new()), &sources);
        assert_eq!(m.armor_class_notes.len(), 5);
    }
}
