//! Monsters and monster types (`UserDbas/MonsterU.dat`,
//! `UserDbas/MonTypeU.dat`): the Monsters book (IDD 261), the type editor
//! (IDD 294) and the monster editor (IDD 263) with its sub-dialogs.
//!
//! Attack types (`AttackU.dat`) are only read: a monster's Attacks page
//! picks from `Attack.dat` and `AttackU.dat`, and the chosen attacks are
//! stored in the monster as copies of those records. Its New and Edit make
//! or change the monster's own copies with the Monster Attack dialog
//! (IDD 269), never the attack types; the original has the same buttons
//! but hides them. Its Spells page
//! (IDD 208) keeps copies of spells the same way, its Inventory page is
//! the Treasure Bundle Editor (IDD 290), and Suggest XP Value works the
//! experience out from the other pages.

use cr_formats::Object;
use cr_formats::inventory::InventoryBundle;
use cr_formats::monster::{Arsenal, Monster, MonsterAttack, MonsterType};
use cr_formats::spells::Spell;
use cr_i18n::{tr, tr_noop};

use super::{
    FieldDesc, FieldKind, Flavor, NameError, NewContext, Options, UserKind, Value, default_reason,
    udb_combo, udb_int, udb_multiline, udb_text,
};
use crate::pictures::{Kind, split_stored};
use crate::treasure::{Bundle, LETTERS};
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
            )
            .suggest(
                tr_noop!("Suggest XP Value"),
                suggested_experience,
                experience_question,
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
                FieldKind::Attacks,
                |r, _| Value::Attacks(attacks(r)),
                |r, _, v, _| set_attacks(r, v.attacks()),
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
            FieldDesc::new(
                tr_noop!("Spells"),
                tr_noop!("Spells available to this Monster"),
                FieldKind::Dual(Options::Source("spells")),
                |r, _| Value::List(spell_names(r)),
                |r, _, v, sources| set_spells(r, &v.list(), &sources.spells),
            ),
            FieldDesc::new(
                tr_noop!("Inventory"),
                tr_noop!("Inventory"),
                FieldKind::Items,
                |r: &Rec, _| Value::Items(Bundle::from_inventory(&r.inventory).unwrap_or_default()),
                |r, _, v, _| r.inventory = v.items().to_inventory(),
            ),
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

/// A monster's own attacks (the Attacks list of IDD 271).
pub fn attacks(m: &Monster) -> Vec<MonsterAttack> {
    m.arsenal
        .attack_list
        .iter()
        .filter_map(|o| match o {
            Object::MonsterAttack(a) => Some((**a).clone()),
            _ => None,
        })
        .collect()
}

/// Sets a monster's own attacks.
pub fn set_attacks(m: &mut Monster, attacks: Vec<MonsterAttack>) {
    m.arsenal.attack_list = attacks
        .into_iter()
        .map(|a| Object::MonsterAttack(Box::new(a)))
        .collect();
}

/// The Monster Attack dialog's (IDD 269) "Must make a to hit roll
/// against" choices, with the value each stores.
pub const TO_HIT: [(i32, &str); 4] = [
    (0, tr_noop!("Normal AC")),
    (1, tr_noop!("Automatically Hits")),
    (2, tr_noop!("AC Excluding Armor")),
    (10, tr_noop!("Armor Class 10")),
];

/// Its Area of Effect choices, by value.
pub const AREAS: [&str; 5] = [
    tr_noop!("Single Target"),
    tr_noop!("Cone"),
    tr_noop!("Beam"),
    tr_noop!("Rectangle"),
    tr_noop!("Circle"),
];

/// Its "Save Against" choices, by value.
pub const SAVES: [&str; 11] = [
    tr_noop!("None"),
    tr_noop!("Paralyzation"),
    tr_noop!("Poison"),
    tr_noop!("Death Magic"),
    tr_noop!("Rod"),
    tr_noop!("Staff"),
    tr_noop!("Wand"),
    tr_noop!("Petrification"),
    tr_noop!("Polymorph"),
    tr_noop!("Breath Weapon"),
    tr_noop!("Spell"),
];

/// Its "to gain this" choices, by value.
pub const SAVE_RESULTS: [&str; 3] = [
    tr_noop!("Not Applicable"),
    tr_noop!("Half Damage"),
    tr_noop!("Negate Effect"),
];

/// The labels of an area's range, width, end width and length, `None` for
/// those the area hides (0x6bb7a0).
pub fn area_labels(area: i32) -> [Option<&'static str>; 4] {
    let range = Some(tr_noop!("Range:"));
    match area {
        1 => [
            range,
            Some(tr_noop!("Start Width:")),
            Some(tr_noop!("End Width:")),
            Some(tr_noop!("Length:")),
        ],
        2 | 3 => [
            range,
            Some(tr_noop!("Width:")),
            None,
            Some(tr_noop!("Length:")),
        ],
        4 => [range, Some(tr_noop!("Diameter:")), None, None],
        _ => [range, None, None, None],
    }
}

/// The heading of the message [`add_problem`] gives.
pub const ADD_HEADING: &str = tr_noop!("Unable to add attack");

/// Why the attack type `name` cannot be added to `attacks` (0x6fbe40): the
/// monster has one by that name, in any case.
pub fn add_problem(attacks: &[MonsterAttack], name: &str) -> Option<&'static str> {
    let name = name.to_lowercase();
    attacks
        .iter()
        .any(|a| a.name.to_lowercase() == name)
        .then(|| tr("The monster already has an attack by that name."))
}

/// Why the Monster Attack dialog cannot close with the name `name`
/// (0x6bbb25): it is blank, or names another of `attacks`. An edited
/// attack (`editing`, its index) may not take another's name in any case
/// (0x6fc00b); a new one only the same name exactly (0x6fc729).
pub fn attack_problem(
    attacks: &[MonsterAttack],
    editing: Option<usize>,
    name: &str,
) -> Option<&'static str> {
    let name = name.trim();
    if name.is_empty() {
        return Some(tr("You need to supply a name for the attack."));
    }
    match editing {
        Some(index) => attacks
            .iter()
            .enumerate()
            .any(|(i, a)| i != index && a.name.to_lowercase() == name.to_lowercase())
            .then(|| tr("The monster already has an attack by that name.")),
        None => attacks
            .iter()
            .any(|a| a.name == name)
            .then(|| tr("A monster attack by that name has already been added.")),
    }
}

/// What an attack does, in a line: "1d8+2, Cone, save vs. Breath Weapon
/// for Half Damage, Bonus Hit Dice: 1".
pub fn attack_summary(a: &MonsterAttack) -> String {
    let mut parts = Vec::new();
    if a.damage_dice != 0 || a.damage_bonus != 0 {
        parts.push(super::dice_text(
            a.damage_dice.into(),
            a.damage_sides.into(),
            a.damage_bonus.into(),
        ));
    }
    if let Some((_, to_hit)) = TO_HIT.iter().find(|(v, _)| *v == a.to_hit && *v != 0) {
        parts.push(tr(to_hit).to_owned());
    }
    if a.area > 0
        && let Some(area) = AREAS.get(a.area as usize)
    {
        parts.push(tr(area).to_owned());
    }
    if a.save_against > 0
        && let Some(save) = SAVES.get(a.save_against as usize)
    {
        let save = tr(save);
        parts.push(match SAVE_RESULTS.get(a.save_result as usize) {
            Some(result) if a.save_result > 0 => {
                tr!("save vs. {save} for {result}", save, result = tr(result))
            }
            _ => tr!("save vs. {save}", save),
        });
    }
    if a.bonus_hit_dice != 0 {
        parts.push(tr!("Bonus Hit Dice: {n}", n = a.bonus_hit_dice));
    }
    parts.join(", ")
}

/// The names of a monster's spells (the "Spells available to this
/// Monster" list of IDD 208).
pub fn spell_names(m: &Monster) -> Vec<String> {
    m.spells
        .iter()
        .filter_map(|o| match o {
            Object::Spell(s) => Some(s.name.clone()),
            _ => None,
        })
        .collect()
}

/// Sets a monster's spells to `names`: spells it already has are kept,
/// new ones are copied from `spells`, as Add does.
pub fn set_spells(m: &mut Monster, names: &[String], spells: &[Spell]) {
    let old = std::mem::take(&mut m.spells);
    for name in names {
        let kept = old
            .iter()
            .find(|o| matches!(o, Object::Spell(s) if &s.name == name))
            .cloned()
            .or_else(|| {
                spells
                    .iter()
                    .find(|s| &s.name == name)
                    .map(|s| Object::Spell(Box::new(s.clone())))
            });
        m.spells.extend(kept);
    }
}

/// The experience Suggest XP Value offers (0x67eac2): the hit dice, plus
/// one for an armor class under 1, flying, each special defense and magic
/// resistance (two above 50%), plus each attack's own value, looked up
/// with the hit dice bonus in the experience table (0x67ebe5).
pub fn suggested_experience(m: &Monster) -> i64 {
    let mut level = i64::from(m.hit_dice);
    level += i64::from(m.armor_class < 1);
    level += i64::from(m.fly_rate != 0);
    level += m.unknown_0x44c.iter().filter(|&&f| f != 0).count() as i64;
    let resistance = m.magic_resistance;
    level += i64::from(resistance > 0 && resistance < 51);
    level += 2 * i64::from(resistance > 50);
    level += m
        .arsenal
        .attack_list
        .iter()
        .map(|o| match o {
            Object::MonsterAttack(a) => i64::from(a.bonus_hit_dice),
            _ => 0,
        })
        .sum::<i64>();
    experience_for(level, m.unknown_0x438)
}

/// The experience table (0x67ebe5) by level, a positive hit dice bonus
/// counting one level more below 10.
fn experience_for(level: i64, bonus: i32) -> i64 {
    const TABLE: [i64; 11] = [7, 15, 35, 65, 120, 175, 270, 420, 650, 975, 1400];
    match level {
        ..1 => 7,
        1 if bonus < -1 => 7,
        1..10 => TABLE[(level + i64::from(bonus > 0)) as usize],
        10 => 1400,
        11 | 12 => 2000,
        _ => (level - 10) * 1000,
    }
}

/// Suggest XP Value's question (0x6d60fa).
pub fn experience_question(xp: i64) -> String {
    tr!(
        "This monster is worth approximately {xp} experience points. Would you like to set the experience point value to {xp}?",
        xp
    )
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
    fn attack_names_are_checked_as_the_original_does() {
        let attack = |name: &str| MonsterAttack {
            name: name.into(),
            ..Default::default()
        };
        let list = [attack("Bite"), attack("Claw")];
        assert!(add_problem(&list, "bite").is_some());
        assert!(add_problem(&list, "Gaze").is_none());
        // A blank name never closes the dialog.
        assert!(attack_problem(&list, None, "  ").is_some());
        assert!(attack_problem(&list, Some(0), "").is_some());
        // New compares exactly, after trimming.
        assert!(attack_problem(&list, None, " Bite ").is_some());
        assert!(attack_problem(&list, None, "bite").is_none());
        // Edit may keep its own name but not take another's.
        assert!(attack_problem(&list, Some(0), "Bite").is_none());
        assert!(attack_problem(&list, Some(0), "claw").is_some());
    }

    #[test]
    fn the_area_decides_which_measures_show() {
        let shown = |area| area_labels(area).map(|l| l.is_some());
        assert_eq!(shown(0), [true, false, false, false]);
        assert_eq!(shown(1), [true, true, true, true]);
        assert_eq!(shown(2), shown(3));
        assert_eq!(shown(3), [true, true, false, true]);
        assert_eq!(shown(4), [true, true, false, false]);
        assert_eq!(TO_HIT.map(|(v, _)| v), [0, 1, 2, 10]);
        assert_eq!((AREAS.len(), SAVES.len(), SAVE_RESULTS.len()), (5, 11, 3));
    }

    #[test]
    fn an_attack_summary_names_what_is_set() {
        assert!(attack_summary(&MonsterAttack::default()).is_empty());
        let a = MonsterAttack {
            damage_dice: 2,
            damage_sides: 6,
            damage_bonus: 1,
            area: 1,
            save_against: 9,
            save_result: 1,
            bonus_hit_dice: 2,
            ..Default::default()
        };
        let summary = attack_summary(&a);
        assert!(summary.starts_with("2d6+1"));
        assert_eq!(summary.matches(", ").count(), 3);
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
                bonus_hit_dice: 2,
                ..Default::default()
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
        field("Attacks").set(&mut m, &Value::Attacks(sources.attacks.clone()), &sources);
        field("Lair").set(&mut m, &Value::Text("c, Hx3".into()), &sources);
        assert_eq!(field("Lair").get(&m), Value::Text("C, Hx3".into()));
        assert_eq!((m.hit_dice, m.hit_die_size), (3, 8));
        assert_eq!(m.unknown_0x47c, [0, 0, 1, 0, 0, 0]);
        assert_eq!(attacks(&m), sources.attacks);
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

    #[test]
    fn the_suggested_experience_counts_what_the_monster_has() {
        let mut m = Monsters::new_record(&context("Udbtype"));
        let at = |m: &Monster| suggested_experience(m);
        assert_eq!(at(&m), 7, "no hit dice");
        m.hit_dice = 3;
        let three = at(&m);
        m.armor_class = 0;
        let four = at(&m);
        assert!(four > three, "an armor class under 1 counts");
        m.armor_class = 10;
        m.fly_rate = 12;
        assert_eq!(at(&m), four, "flying counts as much");
        m.fly_rate = 0;
        m.unknown_0x44c[2] = 1;
        assert_eq!(at(&m), four, "so does a special defense");
        m.unknown_0x44c[2] = 0;
        m.unknown_0x438 = 2;
        assert_eq!(at(&m), four, "a hit dice bonus counts one level below 10");
        m.unknown_0x438 = 0;
        m.magic_resistance = 60;
        let five = at(&m);
        m.hit_dice = 4;
        m.magic_resistance = 20;
        assert_eq!(at(&m), five, "over 50% resistance counts two");
        m.magic_resistance = 0;
        m.hit_dice = 20;
        m.unknown_0x438 = 3;
        let twenty = at(&m);
        m.unknown_0x438 = 0;
        assert_eq!(at(&m), twenty, "the bonus no longer counts from 10 on");
        m.hit_dice = 21;
        assert_eq!(at(&m) - twenty, 1000, "a thousand a level from 13 on");
        assert!(experience_question(five).contains(&five.to_string()));
    }

    #[test]
    fn spells_and_inventory_are_edited_as_copies() {
        let mut m = Monsters::new_record(&context("Udbtype"));
        let fields = Monsters::fields(Flavor::Expansion);
        let field = |label: &str| fields.iter().find(|f| f.label == label).unwrap();
        let mut sources = Sources::default();
        let mut spell = crate::userdb::spells::Spells::new_record(&context(""));
        spell.name = "Udb bolt".into();
        sources.spells = vec![spell];
        let spells = field("Spells available to this Monster");
        spells.set(&mut m, &Value::List(vec!["Udb bolt".into()]), &sources);
        assert_eq!(spells.get(&m), Value::List(vec!["Udb bolt".into()]));
        // A spell the monster has is kept when the list is set again.
        sources.spells.clear();
        let names = spells.get(&m);
        spells.set(&mut m, &names, &sources);
        assert_eq!(m.spells.len(), 1);

        let inventory = field("Inventory");
        let mut bundle = Bundle::default();
        bundle.insert(crate::treasure::Item::new("Udb sword", 2), true);
        inventory.set(&mut m, &Value::Items(bundle.clone()), &sources);
        assert_eq!(m.inventory.items.len(), 1);
        assert_eq!(inventory.get(&m), Value::Items(bundle));
    }
}
