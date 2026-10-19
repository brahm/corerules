//! What a right-click on a record shows in place of help, for the kinds
//! the original summarizes (the list-box handler 0x4B3B22): a box of
//! text for a character, an NPC or a trap, and a printable page for an
//! encounter, an encounter template, a saved treasure bundle or a monster
//! of the user database.

use cr_formats::Object;
use cr_formats::database::ObjectList;
use cr_i18n::tr;

use crate::Installation;
use crate::catalog::Entry;
use crate::encounter::{Encounter, EncounterRules, hit_point_text};
use crate::help::{comparable, topic};
use crate::sheet::{self, Block, Cell, Flavor, ListItem, Sheet, SheetBox, Table};
use crate::treasure::{Bundle, TreasureRules};
use crate::userdb::Origin;

/// A record's summary.
#[derive(Debug, Clone, PartialEq)]
pub enum Summary {
    /// The original's read-only text box (dialog 245) with only OK.
    Text { title: String, text: String },
    /// A page, which the original shows in a print preview.
    Page(Sheet),
}

impl Installation {
    /// The summary of the record named `name` that screen `screen` (a
    /// [`topic`] number) lists, if the original summarizes that screen's
    /// records.
    pub fn summary(&self, screen: u32, name: &str) -> Option<Summary> {
        match screen {
            topic::CHARACTER_MANAGEMENT => self.character_summary(name),
            topic::TRAPS => self.trap_summary(name),
            topic::ENCOUNTERS => {
                let rules = self.encounter_rules().ok()?;
                let encounter = self
                    .saved_encounters()
                    .ok()?
                    .into_iter()
                    .find(|e| comparable(&e.name) == comparable(name))?;
                Some(Summary::Page(rules.sheet(&encounter)))
            }
            topic::ENCOUNTER_TEMPLATES => {
                let rules = self.encounter_rules().ok()?;
                let entry = crate::catalog::template_entries(&rules)
                    .into_iter()
                    .find(|e| comparable(&e.title) == comparable(name))?;
                Some(Summary::Page(entry_sheet(&entry, rules.flavor())))
            }
            topic::TREASURE_BUNDLES => {
                let rules = self.treasure_rules().ok()?;
                let saved = self
                    .saved_treasures()
                    .ok()?
                    .into_iter()
                    .find(|t| comparable(&t.name) == comparable(name))?;
                Some(Summary::Page(rules.sheet(&saved.name, &saved.bundle)))
            }
            _ => None,
        }
    }

    /// A monster of the user database as the original prints it (0x69DB64):
    /// its statistics, its type's picture and its texts.
    pub fn custom_monster_sheet(&self, name: &str) -> Option<Sheet> {
        let key = crate::userdb::key("monsters", Origin::Custom, name);
        let entry = self
            .udb_entries("Monster", "monsters", crate::catalog::monster_entry)
            .ok()?
            .into_iter()
            .find(|e| e.key == key)?;
        Some(entry_sheet(&entry, crate::encounter::flavor_of(self)))
    }

    /// Summary of a character or NPC (0x4B6572): its name and the lines
    /// its index entry shows.
    fn character_summary(&self, name: &str) -> Option<Summary> {
        let index = self.characters().ok()?;
        let entry = index
            .entries
            .iter()
            .find(|e| comparable(&e.name) == comparable(name))?;
        let title = if entry.unknown_0x24 != 0 {
            tr("Summary of NPC")
        } else {
            tr("Summary of Player Character")
        };
        let mut lines = vec![cr_i18n::tr!("Name: {name}", name = entry.name)];
        lines.extend(entry.details.iter().cloned());
        Some(Summary::Text {
            title: title.into(),
            text: lines.join("\n"),
        })
    }

    /// Summary of a trap (0x4B68DC).
    fn trap_summary(&self, name: &str) -> Option<Summary> {
        for file in ["Database/Traps.dat", "UserDbas/TrapsU.dat"] {
            let Ok(data) = self.read(file) else {
                continue;
            };
            let Ok(list) = ObjectList::parse(&data) else {
                continue;
            };
            for object in list.objects {
                let Object::Trap(t) = object else {
                    continue;
                };
                if comparable(&t.name) != comparable(name) {
                    continue;
                }
                return Some(Summary::Text {
                    title: tr("Trap").into(),
                    text: [
                        cr_i18n::tr!("Name: {name}", name = t.name),
                        cr_i18n::tr!("Damage: {damage}", damage = t.damage),
                        cr_i18n::tr!("Detection Modifier: {value}", value = t.unknown_0x14),
                        cr_i18n::tr!(
                            "Number of Characters Trapped: {value}",
                            value = t.unknown_0x10
                        ),
                        cr_i18n::tr!("Saving Throw: {value}", value = t.unknown_0xc),
                        cr_i18n::tr!("Special Effects: {value}", value = t.unknown_0x8),
                        cr_i18n::tr!("Location: {value}", value = t.unknown_0x4),
                    ]
                    .join("\n"),
                });
            }
        }
        None
    }
}

/// A record's page: its picture, its groups as tables and its text.
fn entry_sheet(entry: &Entry, flavor: Flavor) -> Sheet {
    // The name heads the page, over what the list says of it and the
    // picture.
    let mut head = vec![Block::Paragraph(entry.subtitle.clone())];
    if let Some(picture) = &entry.picture {
        head.push(Block::Image {
            windows: String::new(),
            file: Some(picture.clone()),
        });
    }
    let mut boxes = vec![SheetBox {
        title: entry.title.clone(),
        body: head,
    }];
    for group in &entry.groups {
        boxes.push(SheetBox {
            title: group.title.clone(),
            body: vec![Block::Table(Table {
                columns: 2,
                rows: group
                    .rows
                    .iter()
                    .map(|(label, value)| vec![Cell::new(label), Cell::new(value)])
                    .collect(),
            })],
        });
    }
    if !entry.text.trim().is_empty() {
        boxes.push(SheetBox {
            title: tr("Description").into(),
            body: vec![Block::Paragraph(entry.text.clone())],
        });
    }
    page(&entry.title, flavor, boxes)
}

/// A sheet of `boxes`, one under the other.
fn page(title: &str, flavor: Flavor, boxes: Vec<SheetBox>) -> Sheet {
    Sheet {
        title: title.to_owned(),
        flavor,
        sections: boxes
            .into_iter()
            .map(|b| sheet::Section {
                id: 0,
                content: sheet::SectionContent::Columns(vec![vec![b]]),
            })
            .collect(),
    }
}

/// A bundle's items as a nested list: a spell book's spells and a
/// container's contents under it.
pub fn bundle_list(bundle: &Bundle) -> Vec<ListItem> {
    bundle
        .items
        .iter()
        .map(|item| {
            let mut children: Vec<ListItem> = item
                .spells()
                .iter()
                .map(|s| ListItem {
                    text: format!("{} ({})", s.name, s.pages),
                    children: None,
                })
                .collect();
            if let Some(contents) = &item.contents {
                children.extend(bundle_list(contents));
            }
            ListItem {
                text: item.label(),
                children: (!children.is_empty()).then_some(children),
            }
        })
        .collect()
}

impl EncounterRules {
    /// The encounter as a printable page (the original's View/Print and
    /// its summary, 0x64ADF4).
    pub fn sheet(&self, encounter: &Encounter) -> Sheet {
        let t = self.totals(encounter);
        let mut rows = vec![vec![
            Cell::header("Number"),
            Cell::header("Monster"),
            Cell::header("Hit Points"),
        ]];
        for p in &encounter.monsters {
            rows.push(vec![
                Cell::new(p.count.to_string()).right(),
                Cell::new(&p.monster.name),
                Cell::new(hit_point_text(&p.hit_points)),
            ]);
        }
        for p in &encounter.npcs {
            rows.push(vec![
                Cell::new(p.count.to_string()).right(),
                Cell::new(self.npc_name(p).trim()),
                Cell::new(""),
            ]);
        }
        let stats: Vec<String> = encounter
            .monsters
            .iter()
            .map(|p| self.stat_line(p))
            .chain(encounter.npcs.iter().map(|p| self.npc_line(p)))
            .collect();
        let mut boxes = vec![
            SheetBox {
                title: "Encounter Totals".into(),
                body: vec![Block::Table(Table {
                    columns: 2,
                    rows: [
                        ("Gold Value", t.gold_value),
                        ("Monster XP", t.monster_xp),
                        ("Treasure XP", t.treasure_xp),
                    ]
                    .iter()
                    .map(|(k, v)| {
                        vec![
                            Cell::new(*k),
                            Cell::new(crate::treasure::thousands(*v)).right(),
                        ]
                    })
                    .collect(),
                })],
            },
            SheetBox {
                title: "Monsters and Nonplayer Characters".into(),
                body: vec![Block::Table(Table { columns: 3, rows })],
            },
        ];
        if !stats.is_empty() {
            boxes.push(SheetBox {
                title: "Statistics".into(),
                body: vec![Block::Paragraph(stats.join("\n\n"))],
            });
        }
        // Each trap with the rows of the Traps summary (0x64ADF4).
        for trap in &encounter.traps {
            boxes.push(SheetBox {
                title: trap.name.clone(),
                body: vec![Block::Table(Table {
                    columns: 2,
                    rows: [
                        ("Damage:", &trap.damage),
                        ("Detection Modifier:", &trap.unknown_0x14),
                        ("Number of Characters Trapped:", &trap.unknown_0x10),
                        ("Saving Throw:", &trap.unknown_0xc),
                        ("Special Effects:", &trap.unknown_0x8),
                        ("Location:", &trap.unknown_0x4),
                    ]
                    .iter()
                    .map(|(k, v)| vec![Cell::new(*k), Cell::new(v.as_str())])
                    .collect(),
                })],
            });
        }
        for (title, bundle) in [
            ("Treasure", &encounter.treasure),
            ("Items", &encounter.items),
        ] {
            if !bundle.is_empty() {
                boxes.push(SheetBox {
                    title: title.into(),
                    body: vec![Block::List(bundle_list(bundle))],
                });
            }
        }
        if !encounter.description.trim().is_empty() {
            boxes.push(SheetBox {
                title: "Description".into(),
                body: vec![Block::Paragraph(encounter.description.clone())],
            });
        }
        let title = if encounter.name.is_empty() {
            "Encounter"
        } else {
            &encounter.name
        };
        page(title, self.flavor(), boxes)
    }
}

impl TreasureRules {
    /// The bundle `title` as a printable page: the items, nested, then
    /// the summary.
    pub fn sheet(&self, title: &str, bundle: &Bundle) -> Sheet {
        let rows = self
            .summary(bundle)
            .lines()
            .iter()
            .map(|line| {
                let (title, value) = line.rsplit_once(": ").unwrap_or((line, ""));
                vec![Cell::new(title), Cell::new(value).right()]
            })
            .collect();
        Sheet {
            title: title.to_owned(),
            flavor: self.flavor(),
            sections: vec![sheet::Section {
                id: 0,
                content: sheet::SectionContent::Columns(vec![
                    vec![SheetBox {
                        title: "Treasure".into(),
                        body: vec![Block::List(bundle_list(bundle))],
                    }],
                    vec![SheetBox {
                        title: "Summary".into(),
                        body: vec![Block::Table(Table { columns: 2, rows })],
                    }],
                ]),
            }],
        }
    }
}
