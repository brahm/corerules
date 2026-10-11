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
        let mut sections = vec![
            section(
                "characters",
                "Characters",
                "avatar-default-symbolic",
                self.character_entries(),
            ),
            section(
                "races",
                "Races",
                "system-users-symbolic",
                self.list_entries("Race", race_entry),
            ),
            section(
                "classes",
                "Classes",
                "emblem-system-symbolic",
                self.list_entries("Class", class_entry),
            ),
        ];
        for (id, title, icon, part_type) in [
            (
                "kits",
                "Kits",
                "emblem-documents-symbolic",
                "Character Kits",
            ),
            (
                "weapons",
                "Weapons",
                "applications-games-symbolic",
                "Weapons",
            ),
            ("armor", "Armor", "security-high-symbolic", "Armor"),
            (
                "equipment",
                "Equipment",
                "package-x-generic-symbolic",
                "Equipment",
            ),
            (
                "magic-items",
                "Magical Items",
                "starred-symbolic",
                "Magical Items",
            ),
            (
                "nonweapon",
                "Proficiencies",
                "view-list-symbolic",
                "Non-Weapon Proficiencies",
            ),
        ] {
            let entries = match &parts {
                Ok(parts) => Ok(parts
                    .of_type(part_type)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(part_entry)
                    .collect()),
                Err(e) => Err(e.clone()),
            };
            sections.push(section(id, title, icon, entries));
        }
        for (id, title, wizard) in [
            ("wizard-spells", "Wizard Spells", true),
            ("priest-spells", "Priest Spells", false),
        ] {
            let entries = match &spells {
                Ok(spells) => Ok(sorted(
                    if wizard {
                        &spells.wizard
                    } else {
                        &spells.priest
                    }
                    .iter()
                    .filter_map(spell_entry)
                    .collect(),
                )),
                Err(e) => Err(e.clone()),
            };
            sections.push(section(id, title, "weather-clear-night-symbolic", entries));
        }
        sections.push(section(
            "psionics",
            "Psionic Powers",
            "display-brightness-symbolic",
            self.list_entries("Psionic", psionic_entry),
        ));
        sections.push(section(
            "monsters",
            "Monsters",
            "face-devilish-symbolic",
            self.list_entries("Monster", monster_entry),
        ));
        sections.push(section(
            "treasure",
            "Saved Treasure",
            "user-bookmarks-symbolic",
            self.treasure_entries(),
        ));
        sections
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
                        key: format!("{TREASURE_KEY}{}", t.name),
                        title: t.name.clone(),
                        subtitle: summary
                            .as_ref()
                            .map(|s| format!("{} gp", crate::treasure::thousands(s.value_cp / 100)))
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
                    Ok(c) => Entry {
                        key: e.file.clone(),
                        ..character_entry(&c)
                    },
                    Err(err) => Entry {
                        title: e.name.clone(),
                        subtitle: e.race.clone(),
                        text: format!("This character file could not be read: {err}"),
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

    fn list_entries(
        &self,
        name: &str,
        entry: fn(&Object) -> Option<Entry>,
    ) -> std::result::Result<Vec<Entry>, String> {
        self.list_objects(name)
            .map(|objects| sorted(objects.iter().filter_map(entry).collect()))
            .map_err(|e| e.to_string())
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
        title: "Treasure".into(),
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
            title: "Summary".into(),
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
    entries.sort_by_key(|e| e.title.to_lowercase());
    entries
}

const ABILITIES: [&str; 6] = [
    "Strength",
    "Dexterity",
    "Constitution",
    "Intelligence",
    "Wisdom",
    "Charisma",
];

fn character_entry(c: &Character) -> Entry {
    let d = &c.data;
    let mut abilities = Group::new("Abilities");
    let strength = match d.exceptional_strength {
        0 => d.strength[0].to_string(),
        e => format!("{}/{:02}", d.strength[0], e % 100),
    };
    abilities = abilities.row(ABILITIES[0], strength);
    for (name, scores) in ABILITIES[1..].iter().zip(&d.abilities) {
        abilities = abilities.row(name, scores[0]);
    }
    let race = match d.subrace.as_str() {
        "" => d.race.clone(),
        sub => format!("{} ({sub})", d.race),
    };
    Entry {
        key: String::new(),
        title: d.name.clone(),
        subtitle: format!("{race} {}", d.class_title),
        groups: groups([
            Group::new("Character")
                .row("Player", &d.player)
                .row("Race", race)
                .row("Class", &d.class_title)
                .row("Kit", if d.kit == "None" { "" } else { &d.kit })
                .row("Alignment", &d.alignment)
                .row("Social Class", &d.social_class)
                .row(
                    "Rules",
                    if c.skills_and_powers() {
                        "Skills & Powers"
                    } else {
                        "Core Rules"
                    },
                ),
            abilities,
            Group::new("Appearance")
                .row("Age", format!("{} years", d.age))
                .row("Height", feet_and_inches(d.height))
                .row("Weight", format!("{} lb", d.weight)),
            Group::new("Proficiencies")
                .list("Weapons", &d.weapon_proficiencies)
                .list("Nonweapon", &d.nonweapon_proficiencies)
                .list("Languages", &d.languages),
            Group::new("Abilities and Spells")
                .list("Racial", &d.racial_abilities)
                .list("Class", &d.class_slots[0].abilities)
                .list("Spells", &d.spells),
            Group::new("Equipment")
                .list("Armor", &d.armor)
                .list("Carried", &inventory_names(&d.inventory[2])),
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

const ABILITY_SHORT: [&str; 6] = ["Str", "Dex", "Con", "Int", "Wis", "Cha"];

fn race_entry(object: &Object) -> Option<Entry> {
    let Object::Race(r) = object else { return None };
    let mut limits = Group::new("Ability Limits");
    for (name, [min, max]) in ABILITY_SHORT.iter().zip(r.ability_limits) {
        limits = limits.row(name, format!("{min}–{max}"));
    }
    let adjustments: Vec<String> = ABILITY_SHORT
        .iter()
        .zip(r.ability_adjustments)
        .filter(|(_, a)| *a != 0)
        .map(|(name, a)| format!("{name} {a:+}"))
        .collect();
    Some(Entry {
        key: String::new(),
        title: r.name.clone(),
        subtitle: adjustments.join(", "),
        groups: groups([
            Group::new("Race")
                .list("Adjustments", &adjustments)
                .row("Movement", r.movement)
                .list("Languages", &r.languages),
            limits,
            Group::new("Racial Abilities").list("Abilities", &r.abilities),
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
    let (requirements, id, experience, alignments, abilities): (_, _, _, &[String], &[String]) =
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
    let mut minimums = Group::new("Ability Requirements");
    for (name, min) in ABILITY_SHORT.iter().zip(requirements) {
        if min > 0 {
            minimums = minimums.row(name, min);
        }
    }
    entry.subtitle = class_group(id).to_owned();
    entry.groups = groups([
        Group::new("Class")
            .row("Group", class_group(id))
            .list("Alignments", alignments)
            .row("Level 2", format!("{} XP", experience[2]))
            .row("Level 10", format!("{} XP", experience[10])),
        minimums,
        Group::new("Class Abilities").list("Abilities", abilities),
    ]);
    Some(entry)
}

/// The class group a class identifier falls in (Fighter 1000, Mage 2000).
fn class_group(id: i32) -> &'static str {
    match id / 1000 {
        1 => "Warrior",
        2 => "Rogue",
        3 => "Priest",
        4 => "Wizard",
        5 => "Psionicist",
        _ => "Other",
    }
}

fn part_entry(object: &Object) -> Option<Entry> {
    let Object::Part(p) = object else { return None };
    if p.head.name.is_empty() {
        return None;
    }
    Some(Entry {
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
    let kind = if s.priest != 0 { "Priest" } else { "Wizard" };
    Some(Entry {
        key: String::new(),
        title: s.name.clone(),
        subtitle: format!("{kind} level {}", s.level),
        groups: groups([Group::new("Spell")
            .row("Level", s.level)
            .list("Schools", &s.schools)
            .list("Spheres", &s.spheres)
            .row("Range", &s.range)
            .row("Components", &s.components)
            .row("Duration", &s.duration)
            .row("Casting Time", &s.casting_time)
            .row("Area of Effect", &s.area_of_effect)
            .row("Saving Throw", &s.saving_throw)]),
        text: String::new(),
    })
}

fn psionic_entry(object: &Object) -> Option<Entry> {
    let Object::PsionicPower(p) = object else {
        return None;
    };
    Some(Entry {
        key: String::new(),
        title: p.name.clone(),
        subtitle: p.cost.clone(),
        groups: groups([Group::new("Power")
            .row("Cost", &p.cost)
            .row("Range", &p.range)
            .row("Area", &p.area)
            .list("Prerequisites", &p.prerequisites)]),
        text: p.description.clone(),
    })
}

fn monster_entry(object: &Object) -> Option<Entry> {
    let Object::Monster(m) = object else {
        return None;
    };
    let movement = [
        (m.movement, ""),
        (m.fly_rate, "fly"),
        (m.swim_rate, "swim"),
        (m.burrow_rate, "burrow"),
    ]
    .iter()
    .filter(|(rate, _)| *rate > 0)
    .map(|(rate, how)| {
        if how.is_empty() {
            rate.to_string()
        } else {
            format!("{how} {rate}")
        }
    })
    .collect::<Vec<_>>()
    .join(", ");
    Some(Entry {
        key: String::new(),
        title: m.name.clone(),
        subtitle: m.climate_terrain.clone(),
        groups: groups([
            Group::new("Ecology")
                .row("Climate/Terrain", &m.climate_terrain)
                .row("Organization", &m.organization)
                .row("Activity Cycle", &m.activity_cycle)
                .row("Diet", &m.diet)
                .row("Intelligence", &m.intelligence)
                .row("Alignment", &m.alignment),
            Group::new("Combat")
                .row("Armor Class", m.armor_class)
                .row("Movement", movement)
                .row("Hit Dice", m.hit_dice)
                .row("THAC0", m.thac0)
                .row("Attacks", &m.arsenal.attacks)
                .row("Damage", &m.arsenal.damage)
                .row("Special Attacks", &m.arsenal.special_attacks)
                .row("Special Defenses", &m.arsenal.special_defenses)
                .row(
                    "Magic Resistance",
                    if m.magic_resistance > 0 {
                        format!("{}%", m.magic_resistance)
                    } else {
                        String::new()
                    },
                )
                .row("Morale", m.morale)
                .row("XP Value", m.experience),
        ]),
        text: m.description.clone(),
    })
}
