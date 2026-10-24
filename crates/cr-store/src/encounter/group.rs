//! Group Encounter (the NPC screen's button, handler 0x44e30b): an
//! encounter made of the NPCs of a party, and its printout
//! (CEncounterGroupPrinter 0x6384f1, body 0x638c1c) in the full or the
//! condensed format.

use cr_formats::character::Character;

use super::{Encounter, EncounterRules, Flavor, NpcPackage};
use crate::sheet::{Block, Cell, Section, SectionContent, Sheet, SheetBox, Table};
use crate::summary::bundle_list;
use crate::treasure::{SUMMARY_TYPE_NAMES, thousands};

/// The print job's title (0x63878e): "Group Encounter [<party>]". The
/// original cuts it to 31 characters, which loses most party names.
pub fn group_job_title(party: &str) -> String {
    format!("Group Encounter [{party}]")
}

impl Encounter {
    /// The encounter Group Encounter builds (0x44e759): one NPC package
    /// per member, each of one individual with no hit points of its own,
    /// in the order given, and nothing else, not even a name.
    pub fn of_group(members: Vec<Character>) -> Self {
        Self {
            npcs: members.into_iter().map(NpcPackage::of_character).collect(),
            ..Self::default()
        }
    }
}

impl EncounterRules {
    /// The NPC stat line (0x43b59b) of a package: "Borin Dwarf male F3 (S
    /// 17, D 12, …; hp 27; THAC0 18; XP 175)". Core Rules 2.0's (0x420300)
    /// is not ported; it gets [`npc_line`](Self::npc_line).
    pub fn npc_stat_line(&self, package: &NpcPackage) -> String {
        if self.flavor != Flavor::Expansion {
            return self.npc_line(package);
        }
        self.sheet
            .npc_stat_line(
                &package.character,
                package.count,
                &package.individual_hit_points(),
            )
            .unwrap_or_else(|_| self.npc_line(package))
    }

    /// The group encounter printout of `encounter`, whose name is the
    /// party's: the title, the Group table (NPCs first, then monsters,
    /// the TOTAL number counting rows), the treasure by category, the
    /// description, the NPCs (a few lines of each one's sheet, or one stat
    /// line each when `condensed`), the monsters' stat lines, the traps
    /// and the treasure.
    pub fn group_sheet(&self, encounter: &Encounter, condensed: bool) -> Sheet {
        let mut boxes = vec![SheetBox {
            title: encounter.name.clone(),
            body: Vec::new(),
        }];

        let mut rows = vec![vec![
            Cell::header("Number").right(),
            Cell::header("Experience").right(),
            Cell::header("Individual"),
        ]];
        let mut xp = 0i32;
        let mut row = |count: i32, x: i32, name: &str| {
            let x = x.wrapping_mul(count);
            xp = xp.wrapping_add(x);
            rows.push(vec![
                Cell::new(thousands(count)).right(),
                Cell::new(thousands(x)).right(),
                Cell::new(name),
            ]);
        };
        for p in &encounter.npcs {
            row(
                p.count,
                self.npc_experience(&p.character),
                &p.character.data.name,
            );
        }
        for p in &encounter.monsters {
            row(p.count, p.monster.experience, &p.monster.name);
        }
        let number = (encounter.npcs.len() + encounter.monsters.len()) as i32;
        rows.push(vec![
            Cell::new(thousands(number)).right(),
            Cell::new(thousands(xp)).right(),
            Cell::new("TOTAL"),
        ]);
        boxes.push(table_box("Group", 3, rows));

        let merged = self.merged_treasure(encounter);
        let summary = self.treasure.summary_with(&merged, encounter.prices());
        let mut rows = vec![vec![
            Cell::header("Number").right(),
            Cell::header("Experience").right(),
            Cell::header("Gold Value").right(),
            Cell::header("Category"),
        ]];
        // The total gold is that of the rows, each floored to gold, so
        // that it adds up as printed.
        let (mut count, mut gold) = (0i32, 0i32);
        for (totals, name) in summary.by_type.iter().zip(SUMMARY_TYPE_NAMES) {
            if totals.value_cp == 0 && totals.xp == 0 && totals.count == 0 {
                continue;
            }
            count = count.wrapping_add(totals.count);
            gold = gold.wrapping_add(totals.value_cp / 100);
            rows.push(vec![
                Cell::new(thousands(totals.count)).right(),
                Cell::new(thousands(totals.xp)).right(),
                Cell::new(thousands(totals.value_cp / 100)).right(),
                Cell::new(name),
            ]);
        }
        rows.push(vec![
            Cell::new(thousands(count)).right(),
            Cell::new(thousands(summary.xp)).right(),
            Cell::new(thousands(gold)).right(),
            Cell::new("TOTAL"),
        ]);
        boxes.push(table_box("Treasure", 4, rows));

        if !encounter.description.is_empty() {
            boxes.push(paragraph_box("Description", &encounter.description));
        }
        if condensed {
            let lines: Vec<String> = encounter
                .npcs
                .iter()
                .map(|p| self.npc_stat_line(p))
                .collect();
            if !lines.is_empty() {
                boxes.push(paragraph_box("NPC's", &lines.join("\n\n")));
            }
        } else {
            // The heading comes even when there are no NPCs; each NPC is
            // headed by its name in bold.
            boxes.push(SheetBox {
                title: "NPC's".into(),
                body: Vec::new(),
            });
            for p in &encounter.npcs {
                if let Ok(table) = self.sheet.group_npc_table(&p.character) {
                    boxes.push(SheetBox {
                        title: p.character.data.name.clone(),
                        body: vec![Block::Table(table)],
                    });
                }
            }
        }
        if !encounter.monsters.is_empty() {
            let lines: Vec<String> = encounter
                .monsters
                .iter()
                .map(|p| self.stat_line(p))
                .collect();
            boxes.push(paragraph_box("Monsters", &lines.join("\n\n")));
        }
        if !encounter.traps.is_empty() {
            boxes.push(SheetBox {
                title: "Traps".into(),
                body: Vec::new(),
            });
            for t in &encounter.traps {
                let rows = [
                    ("Damage:", &t.damage),
                    ("Detection Modifier:", &t.unknown_0x14),
                    ("Number of Characters Trapped:", &t.unknown_0x10),
                    ("Saving Throw:", &t.unknown_0xc),
                    ("Special Effects:", &t.unknown_0x8),
                    ("Location:", &t.unknown_0x4),
                ]
                .iter()
                .map(|(k, v)| vec![Cell::new(*k), Cell::new(v.as_str())])
                .collect();
                boxes.push(table_box(&t.name, 2, rows));
            }
        }
        // Unlike the encounter printout, no page break comes first.
        if !merged.is_empty() {
            boxes.push(SheetBox {
                title: "Treasure".into(),
                body: vec![Block::List(bundle_list(&merged))],
            });
        }
        Sheet {
            title: group_job_title(&encounter.name),
            flavor: self.flavor(),
            sections: boxes
                .into_iter()
                .map(|b| Section {
                    id: 0,
                    content: SectionContent::Columns(vec![vec![b]]),
                })
                .collect(),
            border: None,
        }
    }
}

fn table_box(title: &str, columns: usize, rows: Vec<Vec<Cell>>) -> SheetBox {
    SheetBox {
        title: title.to_owned(),
        body: vec![Block::Table(Table { columns, rows })],
    }
}

fn paragraph_box(title: &str, text: &str) -> SheetBox {
    SheetBox {
        title: title.to_owned(),
        body: vec![Block::Paragraph(text.to_owned())],
    }
}
