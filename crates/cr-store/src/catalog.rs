//! A display-ready view of an installation: sections of named entries,
//! each with a few groups of label/value rows. The GTK app shows this
//! without knowing the file formats.

use cr_formats::Object;
use cr_formats::character::Character;
use cr_formats::classes::Class;
use cr_formats::database::ObjectList;
use cr_formats::inventory::InventoryBundle;
use cr_formats::parts::PartsFile;
use cr_formats::spells::SpellsFile;
use cr_i18n::{tr, tr_noop, trn};

use crate::encounter::{Encounter, EncounterRules, hit_point_text, npc_line};
use crate::treasure::thousands;
use crate::userdb::tables::{self, TableDb};
use crate::userdb::{Origin, origin_label};
use crate::{Error, Installation, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// Stable identifier, e.g. `monsters`.
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    pub entries: Vec<Entry>,
    /// Set when the section's files could not be read; `entries` is then
    /// empty.
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Entry {
    /// What the entry was read from, when it can be edited: the file name
    /// of a character in `Chars/`.
    pub key: String,
    pub title: String,
    pub subtitle: String,
    pub groups: Vec<Group>,
    /// Long rules text, shown below the groups.
    pub text: String,
    /// A character the NPC generator made (index entry +0x24).
    pub npc: bool,
    /// The character's party, empty for none.
    pub party: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Group {
    pub title: String,
    pub rows: Vec<(String, String)>,
}

impl Group {
    fn new(title: &str) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
        }
    }

    /// Adds a row unless the value is empty.
    fn row(mut self, label: &str, value: impl ToString) -> Self {
        let value = value.to_string();
        if !value.is_empty() {
            self.rows.push((label.into(), value));
        }
        self
    }

    /// Adds a row listing `items`, unless there are none.
    fn list(self, label: &str, items: &[String]) -> Self {
        self.row(label, items.join(", "))
    }
}

/// The groups that have rows.
fn groups(groups: impl IntoIterator<Item = Group>) -> Vec<Group> {
    groups.into_iter().filter(|g| !g.rows.is_empty()).collect()
}

impl Installation {
    /// Every section the app shows, in sidebar order. A section whose files
    /// fail to load carries the error instead of entries, so one damaged
    /// file does not hide the rest.
    pub fn catalog(&self) -> Vec<Section> {
        let parts = self.load_parts();
        let spells = self.load_spells();
        let user_parts = self.load_user_parts();
        let user_spells = self.load_user_spells();
        let mut sections = vec![
            section(
                "characters",
                tr("Characters"),
                "avatar-default-symbolic",
                self.character_entries(),
            ),
            section(
                "races",
                tr("Races"),
                "system-users-symbolic",
                self.udb_entries("Race", "races", race_entry),
            ),
            section(
                "classes",
                tr("Classes"),
                "emblem-system-symbolic",
                self.udb_entries("Class", "classes", class_entry),
            ),
        ];
        for (id, title, icon, part_type) in [
            (
                "kits",
                tr("Kits"),
                "emblem-documents-symbolic",
                "Character Kits",
            ),
            (
                "weapons",
                tr("Weapons"),
                "applications-games-symbolic",
                "Weapons",
            ),
            ("armor", tr("Armor"), "security-high-symbolic", "Armor"),
            (
                "equipment",
                tr("Equipment"),
                "package-x-generic-symbolic",
                "Equipment",
            ),
            (
                "magic-items",
                tr("Magical Items"),
                "starred-symbolic",
                "Magical Items",
            ),
            (
                "nonweapon",
                tr("Proficiencies"),
                "view-list-symbolic",
                "Non-Weapon Proficiencies",
            ),
        ] {
            let entries = match (&parts, &user_parts) {
                (Ok(parts), Ok(user)) => Ok(sorted(
                    [
                        (Origin::Standard, Some(parts)),
                        (Origin::Custom, user.as_ref()),
                    ]
                    .into_iter()
                    .filter_map(|(origin, file)| Some((origin, file?)))
                    .flat_map(|(origin, file)| {
                        file.of_type(part_type)
                            .unwrap_or_default()
                            .iter()
                            .filter_map(part_entry)
                            .map(move |e| udb_mark(e, id, origin))
                    })
                    .collect(),
                )),
                (Err(e), _) | (_, Err(e)) => Err(e.clone()),
            };
            sections.push(section(id, title, icon, entries));
        }
        for (id, title, wizard) in [
            ("wizard-spells", tr("Wizard Spells"), true),
            ("priest-spells", tr("Priest Spells"), false),
        ] {
            let entries = match (&spells, &user_spells) {
                (Ok(spells), Ok(user)) => Ok(sorted(
                    [
                        (Origin::Standard, Some(spells)),
                        (Origin::Custom, user.as_ref()),
                    ]
                    .into_iter()
                    .filter_map(|(origin, file)| Some((origin, file?)))
                    .flat_map(|(origin, file)| {
                        if wizard { &file.wizard } else { &file.priest }
                            .iter()
                            .filter_map(spell_entry)
                            .map(move |e| udb_mark(e, id, origin))
                    })
                    .collect(),
                )),
                (Err(e), _) | (_, Err(e)) => Err(e.clone()),
            };
            sections.push(section(id, title, "weather-clear-night-symbolic", entries));
        }
        sections.push(section(
            "psionics",
            tr("Psionic Powers"),
            "display-brightness-symbolic",
            self.udb_entries("Psionic", "psionics", psionic_entry),
        ));
        sections.push(section(
            "monsters",
            tr("Monsters"),
            "face-devilish-symbolic",
            self.udb_entries("Monster", "monsters", monster_entry),
        ));
        sections.push(section(
            "monster-types",
            tr("Monster Types"),
            "view-list-symbolic",
            self.udb_entries("MonType", "monster-types", monster_type_entry),
        ));
        sections.push(section(
            "traps",
            tr("Traps"),
            "dialog-warning-symbolic",
            self.udb_entries("Traps", "traps", trap_entry),
        ));
        let rules = self.encounter_rules().map_err(|e| e.to_string());
        sections.push(section(
            "encounters",
            tr("Encounters"),
            "mark-location-symbolic",
            self.encounter_entries(rules.as_ref().ok()),
        ));
        sections.push(section(
            "templates",
            tr("Encounter Templates"),
            "view-list-bullet-symbolic",
            rules.as_ref().map(template_entries).map_err(Clone::clone),
        ));
        sections.push(section(
            "tables",
            tr("Encounter Tables"),
            "view-grid-symbolic",
            self.table_entries(),
        ));
        sections.push(section(
            "treasure",
            tr("Saved Treasure"),
            "user-bookmarks-symbolic",
            self.treasure_entries(),
        ));
        sections.push(section(
            "treasure-tables",
            tr("Treasure Tables"),
            "view-grid-symbolic",
            self.treasure_table_entries(),
        ));
        sections.push(section(
            "books",
            tr("Books"),
            "accessories-dictionary-symbolic",
            Ok(self.book_entries()),
        ));
        sections
    }

    /// The records of `Database/<name>.dat` then `UserDbas/<name>U.dat`,
    /// keyed for the user database editors of section `id`.
    fn udb_entries(
        &self,
        name: &str,
        id: &str,
        entry: fn(&Object) -> Option<Entry>,
    ) -> std::result::Result<Vec<Entry>, String> {
        let mut entries = Vec::new();
        for (origin, relative) in [
            (Origin::Standard, format!("Database/{name}.dat")),
            (Origin::Custom, format!("UserDbas/{name}U.dat")),
        ] {
            let Some(path) = self.path(&relative) else {
                continue;
            };
            let data = self.read(&relative).map_err(|e| e.to_string())?;
            let list = ObjectList::parse(&data)
                .map_err(|source| Error::Format { path, source }.to_string())?;
            entries.extend(
                list.objects
                    .iter()
                    .filter_map(entry)
                    .map(|e| udb_mark(e, id, origin)),
            );
        }
        Ok(sorted(entries))
    }

    /// The treasure tables (`UserDbas/Treasure.dat`, else the shipped
    /// ones).
    fn treasure_table_entries(&self) -> std::result::Result<Vec<Entry>, String> {
        let db = TableDb::load(self).map_err(|e| e.to_string())?;
        Ok(sorted(
            db.tables
                .iter()
                .map(|t| {
                    let rows = tables::entries(t);
                    let ranges = tables::ranges(t);
                    let entry = Entry {
                        title: t.name.clone(),
                        subtitle: trn!("{n} entry", "{n} entries", rows.len()),
                        groups: groups([Group {
                            title: tr("Entries").into(),
                            rows: rows
                                .iter()
                                .zip(&ranges)
                                .map(|(e, range)| (range.clone(), table_entry_text(e)))
                                .collect(),
                        }]),
                        ..Default::default()
                    };
                    udb_mark(entry, "treasure-tables", tables::origin(t))
                })
                .collect(),
        ))
    }

    fn load_user_parts(&self) -> std::result::Result<Option<PartsFile>, String> {
        let relative = "UserDbas/PartsU.dat";
        if self.path(relative).is_none() {
            return Ok(None);
        }
        let data = self.read(relative).map_err(|e| e.to_string())?;
        PartsFile::parse(&data)
            .map(Some)
            .map_err(|e| format!("{relative}: {e}"))
    }

    fn load_user_spells(&self) -> std::result::Result<Option<SpellsFile>, String> {
        let relative = "UserDbas/SpellsU.dat";
        if self.path(relative).is_none() {
            return Ok(None);
        }
        let data = self.read(relative).map_err(|e| e.to_string())?;
        SpellsFile::parse(&data)
            .map(Some)
            .map_err(|e| format!("{relative}: {e}"))
    }

    /// The saved encounters, standard ones (shipped empty) first. The key
    /// is `encounter:` and the encounter's name.
    fn encounter_entries(
        &self,
        rules: Option<&EncounterRules>,
    ) -> std::result::Result<Vec<Entry>, String> {
        let saved = self.saved_encounters().map_err(|e| e.to_string())?;
        Ok(sorted(
            saved.iter().map(|e| encounter_entry(e, rules)).collect(),
        ))
    }

    /// The user's 2-20 encounter tables (`UserDbas/EncTabU.dat`).
    fn table_entries(&self) -> std::result::Result<Vec<Entry>, String> {
        let tables = self.frequency_tables().map_err(|e| e.to_string())?;
        Ok(sorted(
            tables
                .tables
                .iter()
                .map(|t| Entry {
                    npc: false,
                    party: String::new(),
                    key: format!("{TABLE_KEY}{}", t.name),
                    title: t.name.clone(),
                    subtitle: tr!(
                        "{filled} of 19 rolls filled",
                        filled = t.slots.iter().filter(|s| !s.is_empty()).count()
                    ),
                    groups: vec![Group {
                        title: tr("Rolls").into(),
                        rows: (2..=20)
                            .map(|roll| (roll.to_string(), t.slot(roll).unwrap_or("").to_owned()))
                            .collect(),
                    }],
                    text: String::new(),
                })
                .collect(),
        ))
    }

    /// The saved treasure bundles, sorted by name as the Saved Treasure
    /// list shows them. The key is `treasure:` and the bundle's name.
    fn treasure_entries(&self) -> std::result::Result<Vec<Entry>, String> {
        let saved = self.saved_treasures().map_err(|e| e.to_string())?;
        let rules = self.treasure_rules().ok();
        Ok(sorted(
            saved
                .iter()
                .map(|t| {
                    let summary = rules.as_ref().map(|r| r.summary(&t.bundle));
                    Entry {
                        npc: false,
                        party: String::new(),
                        key: format!("{TREASURE_KEY}{}", t.name),
                        title: t.name.clone(),
                        subtitle: summary
                            .as_ref()
                            .map(|s| {
                                tr!(
                                    "{value} gp",
                                    value = crate::treasure::thousands(s.value_cp / 100)
                                )
                            })
                            .unwrap_or_default(),
                        groups: treasure_groups(&t.bundle, summary.as_ref()),
                        text: String::new(),
                    }
                })
                .collect(),
        ))
    }

    fn character_entries(&self) -> std::result::Result<Vec<Entry>, String> {
        let index = self.characters().map_err(|e| e.to_string())?;
        Ok(index
            .entries
            .iter()
            .map(|e| {
                let file = format!("Chars/{}", e.file);
                match self.read(&file).and_then(|data| {
                    Character::parse(&data).map_err(|source| Error::Format {
                        path: file.clone().into(),
                        source,
                    })
                }) {
                    Ok(c) => {
                        let mut entry = Entry {
                            key: e.file.clone(),
                            npc: e.unknown_0x24 != 0,
                            party: crate::party::party_of(e).to_owned(),
                            ..character_entry(&c)
                        };
                        if entry.npc {
                            entry.subtitle = tr!("NPC · {subtitle}", subtitle = entry.subtitle);
                        }
                        if !entry.party.is_empty() {
                            entry.subtitle = format!("{} · {}", entry.subtitle, entry.party);
                            if let Some(group) = entry.groups.first_mut() {
                                let at = usize::from(
                                    group.rows.first().is_some_and(|r| r.0 == tr("Player")),
                                );
                                group
                                    .rows
                                    .insert(at, (tr("Party").into(), entry.party.clone()));
                            }
                        }
                        entry
                    }
                    Err(err) => Entry {
                        title: e.name.clone(),
                        subtitle: e.race.clone(),
                        text: tr!("This character file could not be read: {err}", err),
                        ..Default::default()
                    },
                }
            })
            .collect())
    }

    /// Objects of `Database/<name>.dat` followed by `UserDbas/<name>U.dat`.
    pub fn list_objects(&self, name: &str) -> Result<Vec<Object>> {
        let mut objects = Vec::new();
        for relative in [
            format!("Database/{name}.dat"),
            format!("UserDbas/{name}U.dat"),
        ] {
            let Some(path) = self.path(&relative) else {
                continue;
            };
            let data = self.read(&relative)?;
            let list = ObjectList::parse(&data).map_err(|source| Error::Format { path, source })?;
            objects.extend(list.objects);
        }
        Ok(objects)
    }

    fn load_parts(&self) -> std::result::Result<PartsFile, String> {
        let relative = "Database/Parts.dat";
        let data = self.read(relative).map_err(|e| e.to_string())?;
        PartsFile::parse(&data).map_err(|e| format!("{relative}: {e}"))
    }

    fn load_spells(&self) -> std::result::Result<SpellsFile, String> {
        let relative = "Database/Spells.dat";
        let data = self.read(relative).map_err(|e| e.to_string())?;
        SpellsFile::parse(&data).map_err(|e| format!("{relative}: {e}"))
    }
}

/// Entry keys of saved encounters, templates and 2-20 tables start with
/// these.
pub const ENCOUNTER_KEY: &str = "encounter:";
pub const TEMPLATE_KEY: &str = "template:";
pub const TABLE_KEY: &str = "table:";

fn encounter_entry(e: &Encounter, rules: Option<&EncounterRules>) -> Entry {
    let mut groups = Vec::new();
    if let Some(rules) = rules {
        let t = rules.totals(e);
        groups.push(Group {
            title: tr("Encounter Totals").into(),
            rows: vec![
                (tr("Gold Value").into(), thousands(t.gold_value)),
                (tr("Monster XP").into(), thousands(t.monster_xp)),
                (tr("Treasure XP").into(), thousands(t.treasure_xp)),
            ],
        });
    }
    groups.push(Group {
        title: tr("Monsters and Nonplayer Characters").into(),
        rows: e
            .monsters
            .iter()
            .map(|p| {
                (
                    format!("{} - {}", p.count, p.monster.name),
                    tr!("hp {hp}", hp = hit_point_text(&p.hit_points)),
                )
            })
            .chain(e.npcs.iter().map(|p| match rules {
                Some(rules) => (
                    format!("{} - {}", p.count, rules.npc_name(p)),
                    rules.npc_line(p),
                ),
                None => (
                    format!("{} - {}", p.count, p.character.data.name),
                    npc_line(p),
                ),
            }))
            .collect(),
    });
    groups.extend(treasure_groups(&e.treasure, None));
    groups.push(Group {
        title: tr("Items").into(),
        rows: e
            .items
            .items
            .iter()
            .map(|i| (i.label(), String::new()))
            .collect(),
    });
    groups.push(Group {
        title: tr("Traps").into(),
        rows: e
            .traps
            .iter()
            .map(|t| (t.name.clone(), t.damage.clone()))
            .collect(),
    });
    groups.retain(|g| !g.rows.is_empty());
    Entry {
        npc: false,
        party: String::new(),
        key: format!("{ENCOUNTER_KEY}{}", e.name),
        title: e.name.clone(),
        subtitle: if e.template.is_empty() {
            e.monster_type.clone()
        } else {
            e.template.clone()
        },
        groups,
        text: e.description.clone(),
    }
}

fn template_entries(rules: &EncounterRules) -> Vec<Entry> {
    sorted(
        rules
            .templates()
            .iter()
            .map(|t| {
                let frequency = match t.frequency {
                    65.. => tr("Common"),
                    20.. => tr("Uncommon"),
                    11.. => tr("Rare"),
                    4.. => tr("Very Rare"),
                    _ => tr("Exempt from tables"),
                };
                let placement = match t.placement {
                    [_, true, _] => tr("Lair only"),
                    [_, _, true] => tr("Wandering only"),
                    _ => tr("Lair and wandering"),
                };
                let lair: Vec<String> = t
                    .lair_treasure
                    .iter()
                    .zip(crate::treasure::LETTERS)
                    .filter(|(n, _)| **n != 0)
                    .map(|(n, c)| format!("{n}x{c}"))
                    .collect();
                let rows = [
                    (tr("Monster Type"), t.monster_type.clone()),
                    (tr("Climate/Terrain"), t.climate_terrain.clone()),
                    (tr("Frequency"), frequency.to_owned()),
                    (tr("Hit Dice"), t.hit_dice.to_string()),
                    (tr("Encountered"), placement.to_owned()),
                    (tr("Lair Treasure"), lair.join(", ")),
                    (
                        tr("Database"),
                        tr(if t.user { "Custom" } else { "Standard" }).to_owned(),
                    ),
                ];
                let text = rules.template_text(t);
                // The printout repeats the name and type on its first lines.
                let text = text.splitn(3, '\n').nth(2).unwrap_or("").trim().to_owned();
                Entry {
                    npc: false,
                    party: String::new(),
                    key: format!("{TEMPLATE_KEY}{}", t.name),
                    title: t.name.clone(),
                    subtitle: t.monster_type.clone(),
                    groups: vec![Group {
                        title: tr("Template").into(),
                        rows: rows
                            .into_iter()
                            .filter(|(_, v)| !v.is_empty())
                            .map(|(k, v)| (k.to_owned(), v))
                            .collect(),
                    }],
                    text: [t.description.trim(), &text]
                        .iter()
                        .filter(|s| !s.is_empty())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                }
            })
            .collect(),
    )
}

/// Entry keys of saved treasure bundles start with this.
pub const TREASURE_KEY: &str = "treasure:";

/// A bundle as groups: the loose items, then one group per item that holds
/// others ("Gems and Art"), then the Calculate summary.
pub fn treasure_groups(
    bundle: &crate::treasure::Bundle,
    summary: Option<&crate::treasure::Summary>,
) -> Vec<Group> {
    let rows = |bundle: &crate::treasure::Bundle| -> Vec<(String, String)> {
        bundle
            .items
            .iter()
            .filter(|i| i.contents.is_none())
            .map(|i| {
                let spells: Vec<String> = i
                    .spells()
                    .iter()
                    .map(|s| format!("{} ({})", s.name, s.pages))
                    .collect();
                (i.label(), spells.join(", "))
            })
            .collect()
    };
    let mut groups = vec![Group {
        title: tr("Treasure").into(),
        rows: rows(bundle),
    }];
    for item in &bundle.items {
        if let Some(contents) = &item.contents {
            groups.push(Group {
                title: item.label(),
                rows: rows(contents),
            });
        }
    }
    if let Some(summary) = summary {
        groups.push(Group {
            title: tr("Summary").into(),
            rows: summary
                .lines()
                .iter()
                .map(|l| match l.rsplit_once(": ") {
                    Some((k, v)) => (k.to_owned(), v.to_owned()),
                    None => (l.clone(), String::new()),
                })
                .collect(),
        });
    }
    groups.retain(|g| !g.rows.is_empty());
    groups
}

fn section(
    id: &'static str,
    title: &'static str,
    icon: &'static str,
    entries: std::result::Result<Vec<Entry>, String>,
) -> Section {
    match entries {
        Ok(entries) => Section {
            id,
            title,
            icon,
            entries,
            error: None,
        },
        Err(error) => Section {
            id,
            title,
            icon,
            entries: Vec::new(),
            error: Some(error),
        },
    }
}

fn sorted(mut entries: Vec<Entry>) -> Vec<Entry> {
    entries.sort_by_cached_key(|e| crate::collate::sort_key(&e.title));
    entries
}

const ABILITIES: [&str; 6] = [
    tr_noop!("Strength"),
    tr_noop!("Dexterity"),
    tr_noop!("Constitution"),
    tr_noop!("Intelligence"),
    tr_noop!("Wisdom"),
    tr_noop!("Charisma"),
];

fn character_entry(c: &Character) -> Entry {
    let d = &c.data;
    let mut abilities = Group::new(tr("Abilities"));
    let strength = match d.exceptional_strength {
        0 => d.strength[0].to_string(),
        e => format!("{}/{:02}", d.strength[0], e % 100),
    };
    abilities = abilities.row(tr(ABILITIES[0]), strength);
    for (name, scores) in ABILITIES[1..].iter().zip(&d.abilities) {
        abilities = abilities.row(tr(name), scores[0]);
    }
    let race = match d.subrace.as_str() {
        "" => d.race.clone(),
        sub => format!("{} ({sub})", d.race),
    };
    Entry {
        npc: false,
        party: String::new(),
        key: String::new(),
        title: d.name.clone(),
        subtitle: format!("{race} {}", d.class_title),
        groups: groups([
            Group::new(tr("Character"))
                .row(tr("Player"), &d.player)
                .row(tr("Race"), race)
                .row(tr("Class"), &d.class_title)
                .row(tr("Kit"), if d.kit == "None" { "" } else { &d.kit })
                .row(tr("Alignment"), tr(&d.alignment))
                .row(tr("Social Class"), tr(&d.social_class))
                .row(
                    tr("Rules"),
                    if c.skills_and_powers() {
                        tr("Skills & Powers")
                    } else {
                        tr("Core Rules")
                    },
                ),
            abilities,
            Group::new(tr("Appearance"))
                .row(tr("Age"), tr!("{age} years", age = d.age))
                .row(tr("Height"), feet_and_inches(d.height))
                .row(tr("Weight"), tr!("{weight} lb", weight = d.weight)),
            Group::new(tr("Proficiencies"))
                .list(tr("Weapons"), &d.weapon_proficiencies)
                .list(tr("Nonweapon"), &d.nonweapon_proficiencies)
                .list(tr("Languages"), &d.languages),
            Group::new(tr("Abilities and Spells"))
                .list(tr("Racial"), &d.racial_abilities)
                .list(tr("Class"), &d.class_slots[0].abilities)
                .list(tr("Spells"), &d.spells),
            Group::new(tr("Equipment"))
                .list(tr("Armor"), &d.armor)
                .list(tr("Carried"), &inventory_names(&d.inventory[2])),
        ]),
        text: String::new(),
    }
}

fn feet_and_inches(inches: u32) -> String {
    format!("{}′ {}″", inches / 12, inches % 12)
}

/// Item names in a bundle, with quantities and contents.
fn inventory_names(bundle: &InventoryBundle) -> Vec<String> {
    bundle
        .items
        .iter()
        .map(|item| {
            let mut name = match item.quantity {
                0 | 1 => item.name.clone(),
                n => format!("{} ×{n}", item.name),
            };
            if let Some(contents) = &item.contents
                && let Object::InventoryBundle(inner) = &**contents
                && !inner.items.is_empty()
            {
                name = format!("{name} ({})", inventory_names(inner).join(", "));
            }
            name
        })
        .collect()
}

const ABILITY_SHORT: [&str; 6] = [
    tr_noop!("Str"),
    tr_noop!("Dex"),
    tr_noop!("Con"),
    tr_noop!("Int"),
    tr_noop!("Wis"),
    tr_noop!("Cha"),
];

fn race_entry(object: &Object) -> Option<Entry> {
    let Object::Race(r) = object else { return None };
    let mut limits = Group::new(tr("Ability Limits"));
    for (name, [min, max]) in ABILITY_SHORT.iter().zip(r.ability_limits) {
        limits = limits.row(tr(name), format!("{min}–{max}"));
    }
    let adjustments: Vec<String> = ABILITY_SHORT
        .iter()
        .zip(r.ability_adjustments)
        .filter(|(_, a)| *a != 0)
        .map(|(name, a)| format!("{} {a:+}", tr(name)))
        .collect();
    Some(Entry {
        npc: false,
        party: String::new(),
        key: String::new(),
        title: r.name.clone(),
        subtitle: adjustments.join(", "),
        groups: groups([
            Group::new(tr("Race"))
                .list(tr("Adjustments"), &adjustments)
                .row(tr("Movement"), r.movement)
                .list(tr("Languages"), &r.languages),
            limits,
            Group::new(tr("Racial Abilities")).list(tr("Abilities"), &r.abilities),
        ]),
        text: r.description.clone(),
    })
}

fn class_entry(object: &Object) -> Option<Entry> {
    let Object::Class(class) = object else {
        return None;
    };
    let mut entry = Entry {
        title: class.name().to_owned(),
        ..Default::default()
    };
    // The 2.0 layout keeps the same leading values without names: the
    // requirements are values 2 to 7, the identifier value 22, and the
    // first string list holds the class abilities.
    let (requirements, _id, experience, alignments, abilities): (_, _, _, &[String], &[String]) =
        match &**class {
            Class::Expansion { data, .. } => (
                data.ability_requirements,
                data.id,
                &data.experience,
                &data.alignments,
                &data.abilities,
            ),
            Class::Core20(data) => {
                let mut requirements = [0; 6];
                requirements.copy_from_slice(&data.unknown_0x10[2..8]);
                (
                    requirements,
                    data.unknown_0x10[22],
                    &data.experience,
                    &[],
                    &data.unknown_0x58[0],
                )
            }
        };
    let mut minimums = Group::new(tr("Ability Requirements"));
    for (name, min) in ABILITY_SHORT.iter().zip(requirements) {
        if min > 0 {
            minimums = minimums.row(tr(name), min);
        }
    }
    let group = crate::userdb::classes::class_group(class)
        .map_or(tr("Other"), |g| tr(crate::userdb::classes::GROUPS[g]));
    entry.subtitle = group.to_owned();
    entry.groups = groups([
        Group::new(tr("Class"))
            .row(tr("Group"), group)
            .list(tr("Alignments"), alignments)
            .row(tr("Level 2"), tr!("{xp} XP", xp = experience[2]))
            .row(tr("Level 10"), tr!("{xp} XP", xp = experience[10])),
        minimums,
        Group::new(tr("Class Abilities")).list(tr("Abilities"), abilities),
    ]);
    Some(entry)
}

fn part_entry(object: &Object) -> Option<Entry> {
    let Object::Part(p) = object else { return None };
    if p.head.name.is_empty() {
        return None;
    }
    Some(Entry {
        npc: false,
        party: String::new(),
        key: String::new(),
        title: p.head.name.clone(),
        subtitle: String::new(),
        groups: Vec::new(),
        text: p.head.description.clone(),
    })
}

fn spell_entry(object: &Object) -> Option<Entry> {
    let Object::Spell(s) = object else {
        return None;
    };
    Some(Entry {
        npc: false,
        party: String::new(),
        key: String::new(),
        title: s.name.clone(),
        subtitle: if s.priest != 0 {
            tr!("Priest level {level}", level = s.level)
        } else {
            tr!("Wizard level {level}", level = s.level)
        },
        groups: groups([Group::new(tr("Spell"))
            .row(tr("Level"), s.level)
            .list(tr("Schools"), &s.schools)
            .list(tr("Spheres"), &s.spheres)
            .row(tr("Range"), &s.range)
            .row(tr("Components"), &s.components)
            .row(tr("Duration"), &s.duration)
            .row(tr("Casting Time"), &s.casting_time)
            .row(tr("Area of Effect"), &s.area_of_effect)
            .row(tr("Saving Throw"), &s.saving_throw)]),
        text: String::new(),
    })
}

fn psionic_entry(object: &Object) -> Option<Entry> {
    let Object::PsionicPower(p) = object else {
        return None;
    };
    Some(Entry {
        npc: false,
        party: String::new(),
        key: String::new(),
        title: p.name.clone(),
        subtitle: p.cost.clone(),
        groups: groups([Group::new(tr("Power"))
            .row(tr("Cost"), &p.cost)
            .row(tr("Range"), &p.range)
            .row(tr("Area"), &p.area)
            .list(tr("Prerequisites"), &p.prerequisites)]),
        text: p.description.clone(),
    })
}

fn monster_entry(object: &Object) -> Option<Entry> {
    let Object::Monster(m) = object else {
        return None;
    };
    let movement = [
        (m.movement, ""),
        (m.fly_rate, tr_noop!("fly {rate}")),
        (m.swim_rate, tr_noop!("swim {rate}")),
        (m.burrow_rate, tr_noop!("burrow {rate}")),
    ]
    .iter()
    .filter(|(rate, _)| *rate > 0)
    .map(|(rate, how)| {
        if how.is_empty() {
            rate.to_string()
        } else {
            cr_i18n::fill(tr(how), &[("rate", rate as &dyn std::fmt::Display)])
        }
    })
    .collect::<Vec<_>>()
    .join(", ");
    Some(Entry {
        npc: false,
        party: String::new(),
        key: String::new(),
        title: m.name.clone(),
        subtitle: m.climate_terrain.clone(),
        groups: groups([
            Group::new(tr("Ecology"))
                .row(tr("Climate/Terrain"), &m.climate_terrain)
                .row(tr("Organization"), &m.organization)
                .row(tr("Activity Cycle"), &m.activity_cycle)
                .row(tr("Diet"), &m.diet)
                .row(tr("Intelligence"), &m.intelligence)
                .row(tr("Alignment"), &m.alignment),
            Group::new(tr("Combat"))
                .row(tr("Armor Class"), m.armor_class)
                .row(tr("Movement"), movement)
                .row(tr("Hit Dice"), m.hit_dice)
                .row(tr("THAC0"), m.thac0)
                .row(tr("Attacks"), &m.arsenal.attacks)
                .row(tr("Damage"), &m.arsenal.damage)
                .row(tr("Special Attacks"), &m.arsenal.special_attacks)
                .row(tr("Special Defenses"), &m.arsenal.special_defenses)
                .row(
                    tr("Magic Resistance"),
                    if m.magic_resistance > 0 {
                        format!("{}%", m.magic_resistance)
                    } else {
                        String::new()
                    },
                )
                .row(tr("Morale"), m.morale)
                .row(tr("XP Value"), m.experience),
        ]),
        text: m.description.clone(),
    })
}

/// Keys `entry` for the user database editors and says where it comes
/// from in its subtitle.
fn udb_mark(mut entry: Entry, section: &str, origin: Origin) -> Entry {
    entry.key = crate::userdb::key(section, origin, &entry.title);
    let label = tr(origin_label(origin));
    entry.subtitle = if entry.subtitle.is_empty() {
        label.to_owned()
    } else {
        format!("{label} · {}", entry.subtitle)
    };
    entry
}

fn monster_type_entry(object: &Object) -> Option<Entry> {
    let Object::MonsterType(t) = object else {
        return None;
    };
    Some(Entry {
        title: t.name.clone(),
        groups: groups([Group::new(tr("Monster Type")).row(tr("Bitmap"), &t.picture)]),
        ..Default::default()
    })
}

fn trap_entry(object: &Object) -> Option<Entry> {
    let Object::Trap(t) = object else {
        return None;
    };
    Some(Entry {
        title: t.name.clone(),
        subtitle: t.unknown_0x4.clone(),
        groups: groups([Group::new(tr("Trap"))
            .row(tr("Location"), &t.unknown_0x4)
            .row(tr("Damage"), &t.damage)
            .row(tr("Detection Modifier"), &t.unknown_0x14)
            .row(tr("Characters Trapped"), &t.unknown_0x10)
            .row(tr("Saving Throw"), &t.unknown_0xc)
            .row(tr("Special Effects"), &t.unknown_0x8)]),
        ..Default::default()
    })
}

/// One treasure-table row as the editor's list shows it: "Udb sword",
/// "2d6 × Gold Pieces", "Table: Gems".
fn table_entry_text(e: &cr_formats::treasure::TreasureTableEntry) -> String {
    let [n, s, m] = e.quantity_dice;
    let count = crate::userdb::dice_text(n.into(), s.into(), m.into());
    let item = if e.is_table != 0 {
        tr!("Table: {name}", name = e.item.name)
    } else {
        e.item.name.clone()
    };
    if count == "1" {
        item
    } else {
        format!("{count} × {item}")
    }
}
