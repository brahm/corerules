//! The character sheet: every value the originals' character sheet shows,
//! in the sections and order their export writes them (emitter 0x5a2257 in
//! the Expansion, 0x432e30 in Core Rules 2.0), the TXT and HTML exports
//! made from it, and a Markdown export the app adds in place of the
//! Expansion's RTF.
//!
//! A [`Sheet`] is a list of sections; each section is a row of columns,
//! each column holds boxes, and each box holds tables, lists, paragraphs
//! and images, exactly as the originals' writers receive them. The GTK
//! view and both exports read the same model:
//!
//! ```no_run
//! # fn main() -> cr_store::Result<()> {
//! let install = cr_store::Installation::open("/path/to/AD&D Core Rules 2.0")?;
//! let character = install.character("90688519.dat")?;
//! let sheet = install.sheet(&character)?;
//! let date = cr_store::sheet::Timestamp::now_utc().format(sheet.flavor);
//! print!("{}", sheet.to_html(&date));
//! # Ok(())
//! # }
//! ```
//!
//! The Expansion lets the player pick the sections and their order; the
//! choice is stored in the character (`sheet_options`) and [`Sheet`]
//! follows it, or the original's defaults when the character has none.
//! Core Rules 2.0 always writes the same sections, skipping those that do
//! not apply.
//!
//! The values follow the originals, quirks included: the Expansion's base
//! THAC0 reads the Fighter table at the character's level whatever the
//! class, 2.0 gives a dual-classed character a THAC0 of 20 until the new
//! class passes the old one, and the HTML escape turns `&` into `and`.

mod build;
pub(crate) use build::ability_display_name;
mod markdown;
pub mod tables;
mod write;

use std::collections::HashMap;

use cr_formats::Object;
use cr_formats::character::{Character, SheetEntries};
use cr_formats::classes::Class;
use cr_formats::parts::{Part, PartsFile};
use cr_formats::race::Race;
use cr_formats::spells::{Spell, SpellsFile};

pub use write::{Timestamp, escape_html};

use crate::advance::AdvanceRules;
use crate::{Error, Installation, Result};

/// Which program's sheet: the Expansion's (TXT and HTML, chosen sections)
/// or Core Rules 2.0's (HTML only, fixed sections).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Expansion,
    Core20,
}

/// A character sheet, ready to show or export.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    /// The character's name, the document title.
    pub title: String,
    pub flavor: Flavor,
    pub sections: Vec<Section>,
}

/// One section of the sheet, as chosen in the options (see
/// [`SECTIONS`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// The section id, 0 to 25 (see [`SECTIONS`]).
    pub id: u32,
    pub content: SectionContent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SectionContent {
    /// A row of columns, each a stack of boxes. Most sections have one
    /// column with one box; Proficiencies has three columns (Combat and
    /// Armor, Weapon Proficiencies, Non-Weapon Proficiencies).
    Columns(Vec<Vec<SheetBox>>),
    /// A page break (section 19).
    PageBreak,
}

/// A titled box.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetBox {
    pub title: String,
    pub body: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Table(Table),
    /// A bulleted list, which may nest.
    List(Vec<ListItem>),
    /// Running text; `\n` breaks lines and `\t` indents. The TXT export
    /// leaves paragraphs out.
    Paragraph(String),
    /// Lines of text each followed by a line break, written without
    /// newlines between them (2.0's Weapon Proficiencies box). The TXT
    /// export leaves them out.
    Lines(Vec<String>),
    /// A picture file (the portrait): the Windows path the exports
    /// write, and the file in the installation when it is there. The TXT
    /// export leaves images out.
    Image {
        windows: String,
        file: Option<std::path::PathBuf>,
    },
}

/// A table: rows of cells.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    /// How many columns the table is laid out in.
    pub columns: usize,
    pub rows: Vec<Vec<Cell>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub text: String,
    pub kind: CellKind,
    pub align: Align,
    /// Columns the cell spans, 1 or more.
    pub colspan: u32,
}

impl Cell {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: CellKind::Data,
            align: Align::Left,
            colspan: 1,
        }
    }

    pub fn header(text: impl Into<String>) -> Self {
        Self {
            kind: CellKind::Header,
            ..Self::new(text)
        }
    }

    pub fn large(text: impl Into<String>) -> Self {
        Self {
            kind: CellKind::Large,
            ..Self::new(text)
        }
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn span(mut self, colspan: u32) -> Self {
        self.colspan = colspan;
        self
    }

    pub fn center(self) -> Self {
        self.align(Align::Center)
    }

    pub fn right(self) -> Self {
        self.align(Align::Right)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellKind {
    Data,
    /// A column heading (HTML `TH`).
    Header,
    /// Large text: an ability's label and score.
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// An entry of a bulleted list, with the list nested under it, if any (an
/// empty nested list still shows in HTML).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    pub text: String,
    pub children: Option<Vec<ListItem>>,
}

/// Section ids.
pub mod ids {
    pub const PERSONAL: u32 = 0;
    pub const ABILITIES: u32 = 1;
    pub const SAVES: u32 = 2;
    pub const PROFICIENCIES: u32 = 3;
    pub const WEAPONS: u32 = 4;
    pub const ROGUE_SKILLS: u32 = 5;
    pub const TURNING: u32 = 6;
    pub const SPELLS: u32 = 7;
    pub const MEMORIZED: u32 = 8;
    pub const SPELLS_KNOWN: u32 = 11;
    pub const SPELLS_GRANTED: u32 = 12;
    pub const RACIAL: u32 = 13;
    pub const CLASS_ABILITIES: u32 = 14;
    pub const DISADVANTAGES: u32 = 15;
    pub const HISTORY: u32 = 16;
    pub const INVENTORY: u32 = 17;
    pub const PSIONICS: u32 = 18;
    pub const PAGE_BREAK: u32 = 19;
    pub const TRAITS: u32 = 20;
    pub const PORTRAIT: u32 = 21;
    pub const INVENTORY_SPELLS: u32 = 22;
    pub const MOVEMENT: u32 = 23;
    pub const KIT: u32 = 24;
    pub const PRIMARY_OFFHAND: u32 = 25;
}

/// The sections the Expansion offers, with the names it stores in
/// `sheet_options` (ids 9 and 10 exist but are never offered and write
/// nothing).
pub const SECTIONS: [(u32, &str); 24] = [
    (ids::PERSONAL, "Personal Information"),
    (ids::ABILITIES, "Ability Scores"),
    (ids::SAVES, "Saving Throws"),
    (ids::PROFICIENCIES, "Proficiencies"),
    (ids::WEAPONS, "Weapons"),
    (ids::ROGUE_SKILLS, "Rogue Skills"),
    (ids::TURNING, "Turning Undead"),
    (ids::SPELLS, "Spells Summary"),
    (ids::MEMORIZED, "Spells Memorized"),
    (ids::SPELLS_KNOWN, "Spells Known (Mage)"),
    (ids::SPELLS_GRANTED, "Spells Granted"),
    (ids::RACIAL, "Racial Abilities"),
    (ids::CLASS_ABILITIES, "Class Abilities"),
    (ids::DISADVANTAGES, "Disadvantages"),
    (ids::HISTORY, "Character History"),
    (ids::INVENTORY, "Inventory, No Spells"),
    (ids::PSIONICS, "Psionics"),
    (ids::PAGE_BREAK, "------  Page Break  ------ "),
    (ids::TRAITS, "Traits"),
    (ids::PORTRAIT, "Character Portrait"),
    (ids::INVENTORY_SPELLS, "Inventory with Spells"),
    (ids::MOVEMENT, "Movement and Encumbrance"),
    (ids::KIT, "Kit Description"),
    (ids::PRIMARY_OFFHAND, "Weapons - Primary and Offhand"),
];

/// The stored name of a section id.
pub fn section_name(id: u32) -> Option<&'static str> {
    SECTIONS.iter().find(|s| s.0 == id).map(|s| s.1)
}

/// The databases' view of the sheet, loaded once per installation.
#[derive(Debug)]
pub struct SheetRules {
    flavor: Flavor,
    advance: AdvanceRules,
    classes: HashMap<String, Class>,
    races: HashMap<String, Race>,
    /// Every part by its name in capitals (the original's map key, so
    /// lookups ignore case), the databases' before the user's.
    parts: HashMap<String, Part>,
    /// Part type (index into `PART_TYPES`) of each part, by the same key.
    part_types: HashMap<String, usize>,
    wizard_spells: Vec<Spell>,
    priest_spells: Vec<Spell>,
    /// Psionic powers, the database's before the user's.
    powers: Vec<cr_formats::psionic::PsionicPower>,
    /// The installation folder as the original sees it, for the default
    /// portrait path.
    windows_root: String,
    /// The installation folder here, where the portraits are looked for.
    root: std::path::PathBuf,
    /// The Turning Undead labels and legend, read from the program.
    turning: Vec<String>,
}

impl SheetRules {
    /// A part by name, ignoring case as the original's map does.
    pub(crate) fn part(&self, name: &str) -> Option<&Part> {
        self.parts.get(&name.to_uppercase())
    }

    /// A psionic power by name (0x60015f), the first that has it.
    pub(crate) fn power(&self, name: &str) -> Option<&cr_formats::psionic::PsionicPower> {
        self.powers.iter().find(|p| p.name == name)
    }

    /// The type of the part a name looks up.
    pub(crate) fn part_type(&self, name: &str) -> Option<usize> {
        self.part_types.get(&name.to_uppercase()).copied()
    }
}

impl Installation {
    /// Loads what the sheet needs from the databases.
    pub fn sheet_rules(&self) -> Result<SheetRules> {
        let expansion = self.path("UserDbas/AllowedU.dat").is_some();
        let mut rules = SheetRules {
            flavor: if expansion {
                Flavor::Expansion
            } else {
                Flavor::Core20
            },
            advance: self.advance_rules()?,
            classes: HashMap::new(),
            races: HashMap::new(),
            parts: HashMap::new(),
            part_types: HashMap::new(),
            wizard_spells: Vec::new(),
            priest_spells: Vec::new(),
            powers: Vec::new(),
            windows_root: "C:\\Program Files\\AD&D Core Rules 2.0".into(),
            root: self.root().to_owned(),
            turning: Vec::new(),
        };
        rules.turning = build::combat::turning_texts(self, rules.flavor);
        for object in self.list_objects("Class")? {
            if let Object::Class(class) = object {
                rules
                    .classes
                    .entry(class.name().to_owned())
                    .or_insert(*class);
            }
        }
        for object in self.list_objects("Race")? {
            if let Object::Race(race) = object {
                rules.races.entry(race.name.clone()).or_insert(*race);
            }
        }
        for relative in ["Database/Parts.dat", "UserDbas/PartsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file = PartsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                path: relative.into(),
                source,
            })?;
            for (t, array) in file.arrays.into_iter().enumerate() {
                for object in array {
                    if let Object::Part(part) = object {
                        let key = part.head.name.to_uppercase();
                        rules.part_types.entry(key.clone()).or_insert(t);
                        rules.parts.entry(key).or_insert(*part);
                    }
                }
            }
        }
        for relative in ["Database/Spells.dat", "UserDbas/SpellsU.dat"] {
            if self.path(relative).is_none() {
                continue;
            }
            let file =
                SpellsFile::parse(&self.read(relative)?).map_err(|source| Error::Format {
                    path: relative.into(),
                    source,
                })?;
            for (list, objects) in [
                (&mut rules.wizard_spells, file.wizard),
                (&mut rules.priest_spells, file.priest),
            ] {
                for object in objects {
                    if let Object::Spell(spell) = object {
                        list.push(*spell);
                    }
                }
            }
        }
        if self.path("Database/Psionic.dat").is_some() {
            for object in self.list_objects("Psionic")? {
                if let Object::PsionicPower(p) = object {
                    rules.powers.push(*p);
                }
            }
        }
        Ok(rules)
    }

    /// The sheet of `character`, loading the rules for it.
    pub fn sheet(&self, character: &Character) -> Result<Sheet> {
        self.sheet_rules()?.sheet(character)
    }
}

impl SheetRules {
    pub fn flavor(&self) -> Flavor {
        self.flavor
    }

    /// The sections `character` would get, in order: its stored choice, or
    /// the Expansion's defaults when it has none (Core Rules 2.0 always
    /// uses its fixed list).
    pub fn section_ids(&self, character: &Character) -> Result<Vec<u32>> {
        if self.flavor == Flavor::Expansion
            && let Some(options) = &character.data.sheet_options
            && options.unknown_0x260 != 0
            && let SheetEntries::Pairs(pairs) = &options.entries
        {
            return Ok(pairs.iter().map(|p| p.1).collect());
        }
        build::default_sections(self, character)
    }

    /// Computes the sheet.
    pub fn sheet(&self, character: &Character) -> Result<Sheet> {
        build::build(self, character)
    }

    /// Each rogue skill's score without armor, by its entry's name.
    pub fn rogue_scores(&self, character: &Character) -> Result<Vec<(String, i32)>> {
        build::rogue_scores(self, character)
    }

    /// Each class's abbreviation and level, joined with "/": "F1",
    /// "F4/M4" (Expansion 0x43ec93, 2.0 0x421bf0). A class missing from
    /// the databases counts as level 0 with no abbreviation.
    pub fn class_levels_text(&self, character: &Character) -> String {
        character
            .data
            .classes
            .names
            .iter()
            .enumerate()
            .map(|(g, name)| {
                let abbreviation = match self.classes.get(name) {
                    Some(Class::Expansion { data, .. }) => data.abbreviation.as_str(),
                    Some(Class::Core20(data)) => data.unknown_0x2610[0].as_str(),
                    None => "",
                };
                let level = self.advance.level_of(character, g).unwrap_or(0);
                format!("{abbreviation}{level}")
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    /// What `character` is worth as a monster in an encounter: the
    /// experience of the hit dice it counts as (Expansion 0x43b133, 2.0
    /// 0x41ff10), shown as the NPC stat line's XP and added to the
    /// encounter's Monster XP once per individual. A Fighter 1 with an
    /// armor class above 0 and Intelligence 12 or less is worth 15.
    pub fn npc_experience(&self, character: &Character) -> Result<i32> {
        let hit_dice = build::npc_hit_dice(self, character)?;
        Ok(crate::encounter::experience_for_hit_dice(hit_dice, 0))
    }
}
