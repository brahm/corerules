//! Custom Races (`UserDbas/RaceU.dat`): IDD 141 "Races", the IDD 142
//! race editor and its sub-dialogs, used for subraces too.
//!
//! Races and subraces share one name space. A subrace has
//! `unknown_0x184` = 1 and its parent race in `unknown_0x188`; its editor
//! hides Dual-Class, Character Points, Ability Scores and Special and shows
//! Movement instead. Core Rules 2.0 keeps the allowed classes, multi-classes
//! and level limits in the race itself (Class Info and Level Limits); the
//! Expansion keeps them in `AllowedU.dat` ([`super::allowed`]).

use cr_formats::race::{MultiClass, Race};
use cr_formats::{ArchiveReader, Field, Object};
use cr_i18n::{tr, tr_noop};

use super::classes::ClassEntry;
use super::{
    FieldDesc, FieldKind, Flavor, NewContext, Options, Sources, UserKind, Value, udb_int, udb_list,
    udb_multiline, udb_text,
};
use crate::{Installation, Result};

pub struct Races;

/// The folder a new race's portraits are looked for in.
pub const PORTRAIT_FOLDER: &str = "C:\\Program Files\\AD&D Core Rules 2.0\\Bitmaps\\Portrait\\";

/// The Alignment page's checks, by index of `unknown_0x190`.
pub const ALIGNMENTS: [&str; 10] = [
    tr_noop!("Any"),
    tr_noop!("Chaotic Good"),
    tr_noop!("Chaotic Evil"),
    tr_noop!("Chaotic Neutral"),
    tr_noop!("Lawful Evil"),
    tr_noop!("Lawful Good"),
    tr_noop!("Lawful Neutral"),
    tr_noop!("Neutral Evil"),
    tr_noop!("Neutral Good"),
    tr_noop!("True Neutral"),
];

pub const ABILITIES: [&str; 6] = [
    tr_noop!("Strength"),
    tr_noop!("Dexterity"),
    tr_noop!("Constitution"),
    tr_noop!("Intelligence"),
    tr_noop!("Wisdom"),
    tr_noop!("Charisma"),
];

/// The thieving skills of the Special page: `thief_adjustments`, then
/// `unknown_0x158`.
pub const THIEF_SKILLS: [&str; 13] = [
    tr_noop!("Pick Pockets"),
    tr_noop!("Open Locks"),
    tr_noop!("Find/Remove Traps"),
    tr_noop!("Move Silently"),
    tr_noop!("Hide in Shadows"),
    tr_noop!("Detect Noise"),
    tr_noop!("Climb Walls"),
    tr_noop!("Read Languages"),
    tr_noop!("Detect Magic"),
    tr_noop!("Detect Illusion"),
    tr_noop!("Bribe"),
    tr_noop!("Tunneling"),
    tr_noop!("Escape Bonds"),
];

/// `level_limits` slots, in file (group) order.
pub const LIMIT_CLASSES: [&str; 9] = [
    "Fighter",
    "Ranger",
    "Paladin",
    "Thief",
    "Bard",
    "Cleric",
    "Druid",
    "Mage",
    "Specialist",
];

/// The rows of the Level Limits dialog (Evermore IDD 186), top to bottom,
/// as `level_limits` slots: Bard, Cleric, Druid, Fighter, Specialist,
/// Mage, Paladin, Ranger, Thief. A row's "Unlimited" check (1467 + row)
/// keeps its flag in `unknown_0x84[row]`.
pub const LIMIT_ROWS: [usize; 9] = [4, 5, 6, 0, 8, 7, 2, 1, 3];

/// The `unknown_0x84` flag of the `level_limits` slot `slot`.
pub fn unlimited_flag(slot: usize) -> usize {
    LIMIT_ROWS.iter().position(|&s| s == slot).unwrap_or(0)
}

/// Clicking an "Unlimited" check sets its flag and greys the class's
/// limit. The handler (`586d00` and its siblings) also stores 1 in the
/// dialog's copy of the limit, but without `UpdateData(FALSE)`, so OK's
/// `UpdateData(TRUE)` reads the greyed edit back: the limit is saved as
/// shown.
pub fn set_unlimited(r: &mut Race, row: usize, on: bool) {
    r.unknown_0x84[row] = i32::from(on);
}

pub fn is_subrace(r: &Race) -> bool {
    r.unknown_0x184 != 0
}

fn is_race(r: &Race) -> bool {
    !is_subrace(r)
}

/// Zero bytes to read an empty race from.
const ZEROS: [u8; 1024] = [0; 1024];

/// A new race, or with `parent` a new subrace of it, as the editor's New
/// leaves it.
pub fn new_race(parent: &str) -> Race {
    let mut r = Race::read(&mut ArchiveReader::new(&ZEROS)).expect("zero bytes read as a race");
    r.unknown_0x16c = [10, 0];
    PORTRAIT_FOLDER.clone_into(&mut r.portrait_folder);
    if !parent.is_empty() {
        r.unknown_0x184 = 1;
        parent.clone_into(&mut r.unknown_0x188);
        r.unknown_0xa8 = 45;
        r.unknown_0xac = 5;
    }
    r
}

/// "Fighter/Thief": a multi-class combination by its class names, in group
/// order.
pub fn multiclass_name(m: &MultiClass, class_ids: &[ClassEntry]) -> String {
    m.classes
        .iter()
        .filter(|id| **id != 0)
        .map(|id| {
            class_ids
                .iter()
                .find(|c| c.id == *id)
                .map_or_else(|| id.to_string(), |c| c.name.clone())
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// The combination a name of [`multiclass_name`] stands for; `None` when a
/// class is unknown or two share a group.
pub fn multiclass_of(name: &str, class_ids: &[ClassEntry]) -> Option<MultiClass> {
    let mut classes = [0; 5];
    for part in name.split('/') {
        let class = class_ids.iter().find(|c| c.name == part)?;
        let group = class.group?;
        if classes[group] != 0 {
            return None;
        }
        classes[group] = class.id;
    }
    Some(MultiClass { classes })
}

fn multiclass_names(r: &Race, class_ids: &[ClassEntry]) -> Vec<String> {
    r.multi_classes
        .iter()
        .filter_map(|o| match o {
            Object::MultiClass(m) => Some(multiclass_name(m, class_ids)),
            _ => None,
        })
        .collect()
}

fn indexed(
    page: &'static str,
    label: &'static str,
    kind: FieldKind,
    get: fn(&Race, usize) -> Value,
    set: fn(&mut Race, usize, &Value, &Sources),
    i: usize,
) -> FieldDesc<Race> {
    FieldDesc::new(page, label, kind, get, set).arg(i)
}

const INT: FieldKind = FieldKind::Int {
    min: -1000,
    max: 100_000,
};
const CHECK: FieldKind = FieldKind::Check;

impl UserKind for Races {
    type Rec = Race;
    const NOUN: &'static str = tr_noop!("Race");
    const FILE: &'static str = "UserDbas/RaceU.dat";

    fn name(r: &Race) -> &str {
        &r.name
    }

    fn set_name(r: &mut Race, name: &str) {
        name.clone_into(&mut r.name);
    }

    fn standard(install: &Installation) -> Result<Vec<Race>> {
        super::standard_list(install, "Race", pick)
    }

    fn parse_user(data: &[u8]) -> std::result::Result<Vec<Race>, cr_formats::Error> {
        super::parse_list(data).map(|l| l.into_iter().filter_map(pick).collect())
    }

    fn user_bytes(records: &[Race]) -> std::result::Result<Vec<u8>, cr_formats::Error> {
        super::list_bytes(
            records
                .iter()
                .cloned()
                .map(|r| Object::Race(Box::new(r)))
                .collect(),
        )
    }

    fn new_record(context: &NewContext) -> Race {
        new_race(&context.parent)
    }

    fn copied(r: &mut Race) {
        r.id = 0;
    }

    fn fields(flavor: Flavor) -> Vec<FieldDesc<Race>> {
        type Rec = Race;
        const MAIN: &str = tr_noop!("Race");
        let mut f = vec![
            udb_text!(MAIN, tr_noop!("Name"), 50, name),
            FieldDesc::new(
                MAIN,
                tr_noop!("Parent Race"),
                FieldKind::Choice(Options::Source("main-races")),
                |r: &Race, _| Value::Text(r.unknown_0x188.clone()),
                |r, _, v, _| r.unknown_0x188 = v.text(),
            )
            .visible(is_subrace)
            .enabled(|_| false),
            indexed(
                MAIN,
                tr_noop!("Core Rules Race"),
                CHECK,
                |r: &Race, i| Value::Bool(r.unknown_0x1e0[i] != 0),
                |r, i, v, _| r.unknown_0x1e0[i] = i32::from(v.bool()),
                0,
            ),
        ];
        if flavor == Flavor::Expansion {
            f.push(
                indexed(
                    MAIN,
                    tr_noop!("Dual-Class"),
                    CHECK,
                    |r: &Race, i| Value::Bool(r.unknown_0x1e0[i] != 0),
                    |r, i, v, _| r.unknown_0x1e0[i] = i32::from(v.bool()),
                    2,
                )
                .visible(is_race),
            );
        }
        f.extend([
            udb_int!(MAIN, tr_noop!("Starting"), 0, 100_000, unknown_0xa8)
                .group(tr_noop!("Character Points"))
                .enabled(is_race),
            udb_int!(MAIN, tr_noop!("Retained"), 0, 100_000, unknown_0xac)
                .group(tr_noop!("Character Points"))
                .enabled(is_race),
        ]);
        for (i, label, group) in [
            (0, tr_noop!("Male"), tr_noop!("Height (inches)")),
            (1, tr_noop!("Female"), tr_noop!("Height (inches)")),
            (2, tr_noop!("Dice"), tr_noop!("Height (inches)")),
            (3, tr_noop!("Sides"), tr_noop!("Height (inches)")),
            (4, tr_noop!("Male"), tr_noop!("Weight (lbs.)")),
            (5, tr_noop!("Female"), tr_noop!("Weight (lbs.)")),
            (6, tr_noop!("Dice"), tr_noop!("Weight (lbs.)")),
            (7, tr_noop!("Sides"), tr_noop!("Weight (lbs.)")),
            (8, tr_noop!("Base"), tr_noop!("Starting Age (years)")),
            (9, tr_noop!("Dice"), tr_noop!("Starting Age (years)")),
            (10, tr_noop!("Sides"), tr_noop!("Starting Age (years)")),
        ] {
            f.push(
                indexed(
                    MAIN,
                    label,
                    INT,
                    |r: &Race, i| Value::Int(r.physical[i].into()),
                    |r, i, v, _| r.physical[i] = v.int() as i32,
                    i,
                )
                .group(group),
            );
        }
        f.push(udb_int!(
            MAIN,
            tr_noop!("Number of Hands"),
            0,
            100,
            unknown_0x21c[0]
        ));
        f.push(udb_text!(MAIN, tr_noop!("Race Image"), 0, portrait));

        // Evermore: Class Info and Level Limits.
        if flavor == Flavor::Core20 {
            const CLASSES: &str = tr_noop!("Class Info");
            f.push(udb_list!(
                CLASSES,
                tr_noop!("Allowed Classes"),
                Options::Source("classes"),
                unknown_0x1b8
            ));
            f.push(
                FieldDesc::new(
                    CLASSES,
                    tr_noop!("Multi-Classes"),
                    FieldKind::Dual(Options::Source("multiclasses")),
                    |r: &Race, _| Value::List(multiclass_names(r, &[])),
                    |r, _, v, sources| {
                        r.multi_classes = v
                            .list()
                            .iter()
                            .filter_map(|n| multiclass_of(n, &sources.classes))
                            .map(Object::MultiClass)
                            .collect();
                    },
                )
                .with_sources(|r: &Race, sources| Value::List(multiclasses(r, sources))),
            );
            f.push(indexed(
                CLASSES,
                tr_noop!("Dual-Class"),
                CHECK,
                |r: &Race, i| Value::Bool(r.unknown_0x1e0[i] != 0),
                |r, i, v, _| r.unknown_0x1e0[i] = i32::from(v.bool()),
                2,
            ));
            // The dialog's rows (IDD 186): its "Unlimited" checks keep
            // their flags in `unknown_0x84` in row order.
            for (row, &i) in LIMIT_ROWS.iter().enumerate() {
                const GROUP: &str = tr_noop!("Racial Level Limits by Class (0: cannot choose)");
                f.push(
                    indexed(
                        tr_noop!("Level Limits"),
                        LIMIT_CLASSES[i],
                        FieldKind::Int { min: 0, max: 100 },
                        |r: &Race, i| Value::Int(r.level_limits[i].into()),
                        |r, i, v, _| r.level_limits[i] = v.int() as i32,
                        i,
                    )
                    .group(GROUP)
                    .enabled_at(|r, i| r.unknown_0x84[unlimited_flag(i)] == 0),
                );
                f.push(
                    indexed(
                        tr_noop!("Level Limits"),
                        tr_noop!("Unlimited"),
                        CHECK,
                        |r: &Race, row| Value::Bool(r.unknown_0x84[row] != 0),
                        |r, row, v, _| set_unlimited(r, row, v.bool()),
                        row,
                    )
                    .group(GROUP),
                );
            }
        }

        // Ability Scores (races only).
        let strength_check = match flavor {
            Flavor::Expansion => tr_noop!("Allow Exceptional Strength"),
            Flavor::Core20 => tr_noop!("Allow Extra Strength"),
        };
        for (i, ability) in ABILITIES.iter().enumerate() {
            for (label, get, set) in [
                (
                    tr_noop!("Minimum"),
                    (|r: &Race, i: usize| Value::Int(r.ability_limits[i][0].into()))
                        as fn(&Race, usize) -> Value,
                    (|r: &mut Race, i: usize, v: &Value, _: &Sources| {
                        r.ability_limits[i][0] = v.int() as i32
                    }) as fn(&mut Race, usize, &Value, &Sources),
                ),
                (
                    tr_noop!("Maximum"),
                    |r: &Race, i| Value::Int(r.ability_limits[i][1].into()),
                    |r, i, v, _| r.ability_limits[i][1] = v.int() as i32,
                ),
                (
                    tr_noop!("Adjustment"),
                    |r: &Race, i| Value::Int(r.ability_adjustments[i].into()),
                    |r, i, v, _| r.ability_adjustments[i] = v.int() as i32,
                ),
            ] {
                f.push(
                    indexed(
                        tr_noop!("Ability Scores"),
                        label,
                        FieldKind::Int { min: -25, max: 25 },
                        get,
                        set,
                        i,
                    )
                    .group(ability)
                    .visible(is_race),
                );
            }
        }
        f.push(
            FieldDesc::new(
                tr_noop!("Ability Scores"),
                strength_check,
                CHECK,
                |r: &Race, _| Value::Bool(r.unknown_0x40 != 0),
                |r, _, v, _| r.unknown_0x40 = i32::from(v.bool()),
            )
            .group(tr_noop!("Options"))
            .visible(is_race),
        );
        f.push(
            FieldDesc::new(
                tr_noop!("Ability Scores"),
                tr_noop!("Plus one to any score (Example: Mongrelman)"),
                CHECK,
                |r: &Race, _| Value::Bool(r.unknown_0x5c != 0),
                |r, _, v, _| r.unknown_0x5c = i32::from(v.bool()),
            )
            .group(tr_noop!("Options"))
            .visible(is_race),
        );

        // Special (races only).
        const SPECIAL: &str = tr_noop!("Special");
        for (i, skill) in THIEF_SKILLS.iter().enumerate() {
            f.push(
                indexed(
                    SPECIAL,
                    skill,
                    FieldKind::Int {
                        min: -100,
                        max: 100,
                    },
                    |r: &Race, i| {
                        Value::Int(
                            match i {
                                0..8 => r.thief_adjustments[i],
                                _ => r.unknown_0x158[i - 8],
                            }
                            .into(),
                        )
                    },
                    |r, i, v, _| match i {
                        0..8 => r.thief_adjustments[i] = v.int() as i32,
                        _ => r.unknown_0x158[i - 8] = v.int() as i32,
                    },
                    i,
                )
                .group(tr_noop!("Thieving Skill Racial Adjustments"))
                .visible(is_race),
            );
        }
        f.extend([
            udb_int!(
                SPECIAL,
                tr_noop!("Natural Armor Class (10 is standard)"),
                -10,
                10,
                unknown_0x16c[0]
            )
            .group(tr_noop!("Combat"))
            .visible(is_race),
            udb_int!(
                SPECIAL,
                tr_noop!("Bonus Hit Points at First"),
                0,
                1000,
                unknown_0x16c[1]
            )
            .group(tr_noop!("Combat"))
            .visible(is_race),
            udb_text!(
                SPECIAL,
                tr_noop!("Natural Attacks (without a weapon)"),
                0,
                unknown_0x180
            )
            .group(tr_noop!("Combat"))
            .visible(is_race),
        ]);
        // Special's movement for a race, the Movement page for a subrace.
        for (page, visible) in [
            (SPECIAL, is_race as fn(&Race) -> bool),
            (tr_noop!("Movement"), is_subrace),
        ] {
            f.extend([
                udb_int!(page, tr_noop!("Unencumbered Movement"), 0, 1000, movement)
                    .group(tr_noop!("Movement Rate"))
                    .visible(visible),
                udb_int!(page, tr_noop!("Swimming"), 0, 1000, unknown_0x178[0])
                    .group(tr_noop!("Movement Rate"))
                    .visible(visible),
                udb_int!(page, tr_noop!("Flying"), 0, 1000, unknown_0x178[1])
                    .group(tr_noop!("Movement Rate"))
                    .visible(visible),
            ]);
        }

        // Skill Package.
        f.extend([
            udb_list!(
                tr_noop!("Skill Package"),
                tr_noop!("Selected Abilities"),
                Options::Source("Racial Abilities"),
                abilities
            ),
            udb_int!(
                tr_noop!("Skill Package"),
                tr_noop!("Character Point Cost"),
                0,
                100_000,
                unknown_0xe0
            ),
            udb_list!(
                tr_noop!("Languages"),
                tr_noop!("Languages"),
                Options::Source("Languages"),
                languages
            ),
            udb_multiline!(
                tr_noop!("Description"),
                tr_noop!("Description"),
                unknown_0x8
            ),
            udb_multiline!(tr_noop!("Penalties"), tr_noop!("Penalties"), description),
        ]);
        for (i, alignment) in ALIGNMENTS.iter().enumerate() {
            f.push(
                indexed(
                    tr_noop!("Alignment"),
                    alignment,
                    CHECK,
                    |r: &Race, i| Value::Bool(r.unknown_0x190[i] != 0),
                    |r, i, v, _| r.unknown_0x190[i] = i32::from(v.bool()),
                    i,
                )
                .group(tr_noop!("Alignments Allowed")),
            );
        }
        f
    }

    fn delete_warning(r: &Race) -> Option<String> {
        Some(if is_subrace(r) {
            tr("Any character or NPC created from the deleted Subrace will no longer function correctly")
                .into()
        } else {
            tr("Any character or NPC created from the deleted Race will no longer function correctly")
                .into()
        })
    }

    fn summary(r: &Race) -> String {
        if is_subrace(r) {
            tr!("Subrace of {race}", race = r.unknown_0x188)
        } else {
            tr("Race").into()
        }
    }
}

/// The multi-class names of `r`, with class names from `sources`.
pub fn multiclasses(r: &Race, sources: &Sources) -> Vec<String> {
    multiclass_names(r, &sources.classes)
}

fn pick(o: Object) -> Option<Race> {
    match o {
        Object::Race(r) => Some(*r),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::userdb::testing::TempInstall;
    use crate::userdb::{NameError, UserDb};

    #[test]
    fn subraces_share_the_race_names_and_warn_on_delete() {
        let mut dwarf = new_race("");
        dwarf.name = "Dwarf".into();
        dwarf.id = 106;
        let t = TempInstall::new(
            "races",
            &[("Database/Race.dat", Races::user_bytes(&[dwarf]).unwrap())],
        );
        let mut db = UserDb::<Races>::load(&t.install).unwrap();
        let mut sub = db.new_record(0, "Dwarf");
        assert_eq!(
            (
                sub.unknown_0x184,
                sub.unknown_0x188.as_str(),
                sub.unknown_0xa8
            ),
            (1, "Dwarf", 45)
        );
        sub.name = "dwarf".into();
        assert_eq!(db.add(sub.clone()), Err(NameError::UsedByStandard));
        sub.name = "Udb dwarf".into();
        assert!(
            Races::delete_warning(&sub)
                .unwrap()
                .contains("deleted Subrace")
        );
        let fields = Races::fields(Flavor::Expansion);
        let special = fields.iter().find(|f| f.page == "Special").unwrap();
        assert!(!special.is_visible(&sub));
        let movement = fields.iter().find(|f| f.page == "Movement").unwrap();
        movement.set(&mut sub, &Value::Int(6), &Sources::default());
        assert_eq!(sub.movement, 6);
        db.add(sub.clone()).unwrap();
        db.save(&t.install).unwrap();
        assert_eq!(UserDb::<Races>::load(&t.install).unwrap().user, [sub]);
    }

    #[test]
    fn multiclasses_round_trip_by_name() {
        let ids = [("Fighter", 1000), ("Thief", 2000)].map(|(n, id)| ClassEntry {
            name: n.into(),
            id,
            group: crate::userdb::classes::group_of(id),
        });
        let m = multiclass_of("Fighter/Thief", &ids).unwrap();
        assert_eq!(m.classes, [1000, 2000, 0, 0, 0]);
        assert_eq!(multiclass_name(&m, &ids), "Fighter/Thief");
        assert!(multiclass_of("Fighter/Fighter", &ids).is_none());
    }

    #[test]
    fn unlimited_checks_follow_the_dialog_rows() {
        let fields = Races::fields(Flavor::Core20);
        let page: Vec<_> = fields.iter().filter(|f| f.page == "Level Limits").collect();
        let labels: Vec<_> = page.iter().step_by(2).map(|f| f.label).collect();
        assert_eq!(
            labels,
            [
                "Bard",
                "Cleric",
                "Druid",
                "Fighter",
                "Specialist",
                "Mage",
                "Paladin",
                "Ranger",
                "Thief"
            ]
        );
        let mut race = new_race("");
        race.level_limits[0] = 12;
        // Fighter is the fourth row: its check is flag 3, and checking it
        // greys the limit but keeps it.
        let (limit, unlimited) = (page[6], page[7]);
        assert_eq!(limit.label, "Fighter");
        assert!(limit.is_enabled(&race));
        unlimited.set(&mut race, &Value::Bool(true), &Sources::default());
        assert_eq!(race.unknown_0x84, [0, 0, 0, 1, 0, 0, 0, 0, 0]);
        assert_eq!(race.level_limits[0], 12);
        assert!(!limit.is_enabled(&race));
        assert_eq!(unlimited_flag(8), 4);
    }
}
