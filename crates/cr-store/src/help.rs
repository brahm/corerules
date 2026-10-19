//! The installation's help: the WinHelp files in `Help/` and their
//! contents files (`.cnt`), as the Help… buttons, the Help menu and the
//! books' Help versions open them.

use std::collections::{HashMap, HashSet};

use cr_formats::database::ObjectList;
use cr_formats::object::Object;
use cr_formats::winhelp::HelpFile;

use crate::{Error, Installation, Result};

/// The program help's topics (`Program.hlp`): numbers of its `[MAP]`,
/// which the original passes to WinHelp (`HELP_CONTEXT`) from each screen's
/// Help… button and F1. A screen's number is its dialog's resource id;
/// the Monsters screen's encounter and template parts use the help ids of
/// their controls.
pub mod topic {
    pub const MAIN_MENU: u32 = 101;
    pub const BOOKS: u32 = 138;
    pub const PARTS: u32 = 139;
    pub const RACES: u32 = 141;
    pub const RACE: u32 = 142;
    pub const PARTY_MANAGEMENT: u32 = 145;
    pub const LIST_OF_BOOKS: u32 = 148;
    pub const NPC_GROUP: u32 = 149;
    pub const WEAPON: u32 = 164;
    pub const NONWEAPON_PROFICIENCY: u32 = 176;
    pub const MAGIC_ITEM: u32 = 182;
    pub const CLASSES_ALLOWED: u32 = 189;
    pub const CLASSES: u32 = 191;
    pub const CLASS: u32 = 192;
    pub const KIT: u32 = 194;
    pub const ROLL_METHOD: u32 = 199;
    pub const CHARACTER_MANAGEMENT: u32 = 202;
    pub const RACE_AND_CLASS: u32 = 205;
    pub const SPELLS: u32 = 208;
    pub const ABILITY_SCORES: u32 = 211;
    pub const CHARACTER_INFORMATION: u32 = 213;
    pub const SUBRACE: u32 = 214;
    pub const CLASS_ABILITIES: u32 = 215;
    pub const EQUIPMENT: u32 = 229;
    pub const TREASURE_TABLES: u32 = 257;
    pub const TREASURE_TABLE: u32 = 258;
    pub const ROLL_TREASURE: u32 = 260;
    pub const MONSTERS: u32 = 261;
    pub const TABLES: u32 = 265;
    pub const ENCOUNTER: u32 = 288;
    pub const TREASURE_BUNDLE: u32 = 290;
    pub const MONSTER_TYPE: u32 = 294;
    pub const TRAPS: u32 = 295;
    pub const TRAP: u32 = 296;
    pub const TREASURE_BUNDLES: u32 = 298;
    pub const NONWEAPON_PROFICIENCIES: u32 = 307;
    pub const WEAPON_PROFICIENCIES: u32 = 308;
    pub const ENCOUNTER_TABLES: u32 = 323;
    pub const ENCOUNTER_TABLE: u32 = 324;
    pub const EXPORT: u32 = 334;
    pub const IMPORT: u32 = 337;
    pub const ROGUE_SKILLS: u32 = 342;
    pub const PSIONIC_POWERS: u32 = 344;
    pub const PSIONIC_POWER: u32 = 346;
    pub const CHARACTER_SPELLS: u32 = 348;
    pub const NPC_CREATION: u32 = 349;
    pub const SKILLS: u32 = 353;
    pub const LEVELS: u32 = 368;
    pub const SELECT_IMAGE: u32 = 400;
    pub const CHARACTER_SHEET: u32 = 414;
    pub const EQUIPMENT_PICTURE: u32 = 415;
    pub const ENCOUNTERS: u32 = 0x8105_09C5;
    pub const ENCOUNTER_TEMPLATES: u32 = 0x8105_09BE;

    /// The topic of the original's screen for a section of the main
    /// window (a `catalog` section id).
    pub fn of_section(section: &str) -> u32 {
        match section {
            "characters" => CHARACTER_MANAGEMENT,
            "races" => RACES,
            "classes" => CLASSES,
            "kits" | "weapons" | "armor" | "equipment" | "magic-items" | "nonweapon" => PARTS,
            "wizard-spells" | "priest-spells" => SPELLS,
            "psionics" => PSIONIC_POWERS,
            "monsters" | "monster-types" => MONSTERS,
            "traps" => TRAPS,
            "encounters" => ENCOUNTERS,
            "templates" => ENCOUNTER_TEMPLATES,
            "tables" => ENCOUNTER_TABLES,
            "treasure" => TREASURE_BUNDLES,
            "treasure-tables" => TREASURE_TABLES,
            "books" => BOOKS,
            _ => MAIN_MENU,
        }
    }

    /// The topic of a record editor for a section of the main window.
    pub fn of_editor(section: &str) -> u32 {
        match section {
            "races" => RACE,
            "classes" => CLASS,
            "kits" => KIT,
            "weapons" => WEAPON,
            "magic-items" => MAGIC_ITEM,
            "nonweapon" => NONWEAPON_PROFICIENCY,
            "psionics" => PSIONIC_POWER,
            "monster-types" => MONSTER_TYPE,
            "traps" => TRAP,
            "treasure-tables" => TREASURE_TABLE,
            other => of_section(other),
        }
    }

    /// The roll methods' screens (Method I to X), and the scores typed in
    /// (Method X's other topic).
    pub const ROLL_METHODS: [u32; 11] = [
        203,
        204,
        216,
        217,
        220,
        222,
        223,
        226,
        225,
        227,
        0x80E3_098A,
    ];

    /// The topic of a roll method's screen.
    pub fn of_roll_method(method: crate::create::RollMethod) -> u32 {
        use crate::create::RollMethod as M;
        match method {
            M::InOrder => ROLL_METHODS[0],
            M::BestOfTwo => ROLL_METHODS[1],
            M::Arrange => ROLL_METHODS[2],
            M::BestSixOfTwelve => ROLL_METHODS[3],
            M::FourDice => ROLL_METHODS[4],
            M::SevenDice => ROLL_METHODS[5],
            M::Points => ROLL_METHODS[6],
            M::DicePool => ROLL_METHODS[7],
            M::RolledPoints => ROLL_METHODS[8],
            M::PointsAndStrength => ROLL_METHODS[9],
            M::Enter => ROLL_METHODS[10],
            // Method XI: race and class first.
            M::PlayersChoice => RACE_AND_CLASS,
        }
    }

    /// Every topic above, for the tests.
    pub const ALL: [u32; 53] = [
        MAIN_MENU,
        BOOKS,
        PARTS,
        RACES,
        RACE,
        PARTY_MANAGEMENT,
        LIST_OF_BOOKS,
        NPC_GROUP,
        WEAPON,
        NONWEAPON_PROFICIENCY,
        MAGIC_ITEM,
        CLASSES_ALLOWED,
        CLASSES,
        CLASS,
        KIT,
        ROLL_METHOD,
        CHARACTER_MANAGEMENT,
        RACE_AND_CLASS,
        SPELLS,
        ABILITY_SCORES,
        CHARACTER_INFORMATION,
        SUBRACE,
        CLASS_ABILITIES,
        EQUIPMENT,
        TREASURE_TABLES,
        TREASURE_TABLE,
        ROLL_TREASURE,
        MONSTERS,
        TABLES,
        ENCOUNTER,
        TREASURE_BUNDLE,
        MONSTER_TYPE,
        TRAPS,
        TRAP,
        TREASURE_BUNDLES,
        NONWEAPON_PROFICIENCIES,
        WEAPON_PROFICIENCIES,
        ENCOUNTER_TABLES,
        ENCOUNTER_TABLE,
        EXPORT,
        IMPORT,
        ROGUE_SKILLS,
        PSIONIC_POWERS,
        PSIONIC_POWER,
        CHARACTER_SPELLS,
        NPC_CREATION,
        SKILLS,
        LEVELS,
        SELECT_IMAGE,
        CHARACTER_SHEET,
        EQUIPMENT_PICTURE,
        ENCOUNTERS,
        ENCOUNTER_TEMPLATES,
    ];
}

/// One line of a contents file: a heading, or a topic to open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentsEntry {
    /// 1 for the top level.
    pub level: u32,
    pub title: String,
    /// The topic's context string; `None` for a heading.
    pub context: Option<String>,
    /// The help file the topic is in, when not the contents file's own.
    pub file: Option<String>,
}

/// A contents file (`.cnt`): what the Contents tab of WinHelp lists.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Contents {
    /// `:Base`, the help file it describes.
    pub base: Option<String>,
    /// `:Title`.
    pub title: Option<String>,
    pub entries: Vec<ContentsEntry>,
    /// `:include` files, in order, with the index of the entry each goes
    /// before.
    pub includes: Vec<(usize, String)>,
}

impl Contents {
    /// Parses a contents file. Lines it does not know (`:Index`, `:Link`,
    /// comments) are skipped.
    pub fn parse(data: &[u8]) -> Self {
        let text: String = data.iter().map(|&b| cr_formats::rtf::cp1252(b)).collect();
        let mut contents = Contents::default();
        for line in text.lines() {
            let line = line.trim_end();
            if let Some(command) = line.strip_prefix(':') {
                let (key, value) = command.split_once(' ').unwrap_or((command, ""));
                match key.to_ascii_lowercase().as_str() {
                    "base" => contents.base = Some(value.trim().to_owned()),
                    "title" => contents.title = Some(value.trim().to_owned()),
                    "include" => contents
                        .includes
                        .push((contents.entries.len(), value.trim().to_owned())),
                    _ => {}
                }
                continue;
            }
            let Some((level, rest)) = line.split_once(' ') else {
                continue;
            };
            let Ok(level) = level.parse::<u32>() else {
                continue;
            };
            let (title, target) = match rest.split_once('=') {
                Some((title, target)) => (title, Some(target)),
                None => (rest, None),
            };
            let (context, file) = match target {
                None => (None, None),
                Some(target) => {
                    // context[@file][>window]
                    let target = target.split('>').next().unwrap_or_default();
                    match target.split_once('@') {
                        Some((context, file)) => (
                            Some(context.trim().to_owned()),
                            Some(file.trim().to_owned()),
                        ),
                        None => (Some(target.trim().to_owned()), None),
                    }
                }
            };
            contents.entries.push(ContentsEntry {
                level,
                title: title.trim().to_owned(),
                context,
                file,
            });
        }
        contents
    }
}

impl Installation {
    /// `Help/<name>`, whatever its case.
    pub fn help_file(&self, name: &str) -> Result<HelpFile> {
        let relative = format!("Help/{name}");
        let path = self.path(&relative).ok_or_else(|| {
            Error::Invalid(cr_i18n::tr!(
                "{file} is not installed",
                file = relative.as_str()
            ))
        })?;
        let data = std::fs::read(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        HelpFile::parse(&data).map_err(|source| Error::Format {
            path: relative.into(),
            source,
        })
    }

    /// The contents file of `Help/<name>` (the same name with `.cnt`), if
    /// there is one with entries, with the entries of the contents files
    /// it includes that are installed (the Tome of Help, `ADandD.cnt`,
    /// includes every book's).
    pub fn help_contents(&self, name: &str) -> Option<Contents> {
        let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
        let mut contents = self.contents_file(&format!("{stem}.cnt"), 0)?;
        contents.includes.clear();
        Some(contents).filter(|c| !c.entries.is_empty())
    }

    /// `Help/<file>` with its includes in place; included entries name
    /// their help file.
    fn contents_file(&self, file: &str, depth: u32) -> Option<Contents> {
        let data = std::fs::read(self.path(&format!("Help/{file}"))?).ok()?;
        let mut contents = Contents::parse(&data);
        if depth > 4 {
            return Some(contents);
        }
        for (at, include) in std::mem::take(&mut contents.includes).into_iter().rev() {
            let Some(mut included) = self.contents_file(&include, depth + 1) else {
                continue;
            };
            if let Some(base) = &included.base {
                for entry in &mut included.entries {
                    if entry.context.is_some() && entry.file.is_none() {
                        entry.file = Some(base.clone());
                    }
                }
            }
            contents.entries.splice(at..at, included.entries);
        }
        Some(contents)
    }
}

/// The help of the controls of one of the original's screens: the topics
/// of `Program.hlp` that a right-click on a control shows in a popup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScreenHelp {
    /// Each control's text, then the title of its topic, made comparable,
    /// with the topic's number.
    texts: Vec<(String, u32)>,
}

impl ScreenHelp {
    /// The topic of the control that reads `text`, its own text or its
    /// topic's title, ignoring case, mnemonics and a trailing `…` or `:`.
    pub fn topic_of(&self, text: &str) -> Option<u32> {
        let text = comparable(text);
        self.texts
            .iter()
            .find(|(t, _)| *t == text)
            .map(|&(_, topic)| topic)
    }

    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }
}

/// A database record's topic in its book's help file, as the original's
/// right-click on a list row opens it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTopic {
    /// The file in `Help/`.
    pub file: &'static str,
    /// The number of its `[MAP]`: the record's help id.
    pub number: u32,
    /// Shown in a popup (`HELP_CONTEXTPOPUP`), not in the help window.
    pub popup: bool,
}

/// What a record is: the original's list of it knows, and opens its
/// record's help by name among those.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// Races, subraces and classes.
    Character,
    Part,
    Spell,
    Psionic,
    /// Monsters and monster types.
    Monster,
}

impl ItemKind {
    /// The kinds of record a screen lists first, then the others: a part
    /// and a race can share a name.
    pub fn order(screen: u32) -> [ItemKind; 5] {
        use ItemKind::*;
        let first = match screen {
            topic::RACES
            | topic::RACE
            | topic::CLASSES
            | topic::CLASS
            | topic::CLASSES_ALLOWED
            | topic::RACE_AND_CLASS
            | topic::SUBRACE
            | topic::CLASS_ABILITIES
            | topic::CHARACTER_INFORMATION
            | topic::NPC_CREATION
            | topic::NPC_GROUP => Character,
            topic::SPELLS | topic::CHARACTER_SPELLS => Spell,
            // Update Character's psionics and their modes.
            topic::PSIONIC_POWERS | topic::PSIONIC_POWER | topic::SKILLS => Psionic,
            topic::MONSTERS | topic::MONSTER_TYPE | topic::ENCOUNTER => Monster,
            _ => Part,
        };
        let mut order = [first; 5];
        let rest = [Part, Spell, Character, Psionic, Monster];
        for (at, kind) in (1..).zip(rest.into_iter().filter(|k| *k != first)) {
            order[at] = kind;
        }
        order
    }
}

/// What a right-click on a record opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemFound {
    /// A shipped record's topic.
    Topic(ItemTopic),
    /// A record of the user database: the original's Custom Help box
    /// (dialog 394) with what the record says.
    Custom { title: String, text: String },
}

/// The help of the installation's records, by name: the shipped records'
/// topics, the user database's records' descriptions, and the rows the
/// original finds by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemHelp {
    /// By name, a kind's user record before its shipped one.
    names: HashMap<String, Vec<(ItemKind, ItemFound)>>,
    classes: HashSet<String>,
    /// Rows whose text has one of these words (0x4B40EA, 0x4B5235).
    within: Vec<(String, ItemKind, ItemTopic)>,
}

impl ItemHelp {
    /// What the row named `name` (what a list row shows, up to a tab)
    /// opens: the first kind of `order` that has a record of that name,
    /// a multi-class (class names joined by `/`), or a row the original
    /// knows by a word in it.
    pub fn find(&self, name: &str, order: &[ItemKind]) -> Option<ItemFound> {
        let name = comparable(name.split('\t').next().unwrap_or(name));
        let named = self.names.get(&name).and_then(|found| {
            order.iter().find_map(|kind| {
                found
                    .iter()
                    .find(|(k, _)| k == kind)
                    .map(|(_, found)| found.clone())
            })
        });
        if named.is_some() {
            return named;
        }
        let multi = name.contains('/')
            && name
                .split('/')
                .all(|class| self.classes.contains(class.trim()));
        if multi {
            return Some(ItemFound::Topic(ItemTopic {
                file: "Charactr.hlp",
                number: 100,
                popup: false,
            }));
        }
        order.iter().find_map(|kind| {
            self.within
                .iter()
                .find(|(word, k, _)| k == kind && name.contains(word.as_str()))
                .map(|(_, _, topic)| ItemFound::Topic(topic.clone()))
        })
    }

    /// The topic [`ItemHelp::find`] gives, for a shipped record.
    pub fn topic_of(&self, name: &str, order: &[ItemKind]) -> Option<ItemTopic> {
        match self.find(name, order)? {
            ItemFound::Topic(topic) => Some(topic),
            ItemFound::Custom { .. } => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    fn add(&mut self, kind: ItemKind, name: &str, found: ItemFound) {
        let entries = self.names.entry(comparable(name)).or_default();
        if !entries.iter().any(|(k, _)| *k == kind) {
            entries.push((kind, found));
        }
    }
}

/// The rows of psionic attack and defense modes (the program's table at
/// 0x47BE3A) and of fighting styles (0x4B5C29, by the styles' numbers in
/// [`crate::create::FIGHTING_STYLES`]): popups of `Psionics.hlp` and
/// `Prof.hlp`.
const PSIONIC_MODES: [(&str, u32); 10] = [
    ("Ego Whip", 8008),
    ("Id Insinuation", 8009),
    ("Mind Thrust", 8010),
    ("Psionic Blast", 8011),
    ("Psychic Crush", 8012),
    ("Intellect Fortress", 9008),
    ("Mental Barrier", 9009),
    ("Mind Blank", 9010),
    ("Thought Shield", 9011),
    ("Tower of Iron Will", 9012),
];
const FIGHTING_STYLES: [(u32, u32); 9] = [
    (1998, 8001),
    (1997, 8002),
    (1993, 8003),
    (1999, 8004),
    (1996, 8005),
    (1995, 8006),
    (1994, 8007),
    (2001, 8008),
    (2002, 8009),
];

/// The psionic disciplines a row may name, and their popups of
/// `Psionics.hlp` (0x4B40EA).
const DISCIPLINES: [(&str, u32); 5] = [
    ("Clairsentient", 210),
    ("Psychokinetic", 211),
    ("Psychometabolic", 212),
    ("Psychoportive", 213),
    ("Telepathic", 214),
];

/// The help file of a part type (an index of `PART_TYPES`) and whether its
/// topics are popups (0x4B3B22).
fn part_help(part_type: usize) -> Option<(&'static str, bool)> {
    Some(match part_type {
        // Weapons, armor, weapon proficiencies, currency.
        0 | 1 | 3 | 16 => ("Equip.hlp", false),
        // Equipment, gems and art.
        7 | 13 => ("Equip.hlp", true),
        // Nonweapon proficiencies.
        4 => ("Prof.hlp", false),
        // Racial abilities, languages, traits, disadvantages, class
        // abilities.
        5 | 6 | 11 | 12 | 14 => ("Prof.hlp", true),
        8 => ("Magic.hlp", false),
        // Count.
        17 => ("Magic.hlp", true),
        // Schools, spheres.
        9 | 18 => ("Spells.hlp", true),
        15 => ("Kits.hlp", false),
        _ => return None,
    })
}

/// A control's text made comparable with the app's labels.
fn comparable(text: &str) -> String {
    let text: String = text
        .chars()
        .filter(|&c| c != '&' && c != '_')
        .collect::<String>()
        .to_lowercase();
    let mut text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    while let Some(t) = text
        .strip_suffix("...")
        .or_else(|| text.strip_suffix('…'))
        .or_else(|| text.strip_suffix(':'))
    {
        text = t.trim_end().to_owned();
    }
    text
}

/// A monster's help id, with the original's fixes for three monsters
/// (0x4B56FA) and its type's when it has none.
fn monster_number(name: &str, id: i32, types: &HashMap<String, i64>) -> i64 {
    let id = if name == "Drider" {
        0x70
    } else if name.contains("Earth Kin") && id == 0x65 {
        0x66
    } else if id == 0 && name.contains("Kuo-Toa") {
        0xD7
    } else {
        id
    };
    match id {
        0 => types.get(&comparable(name)).copied().unwrap_or(0),
        n => n.into(),
    }
}

/// What the Custom Help box of a user record says: its rows, then its
/// rules text, as the main window shows them.
fn custom_text(entry: &crate::catalog::Entry) -> String {
    let mut lines = Vec::new();
    for group in &entry.groups {
        for (label, value) in &group.rows {
            lines.push(format!("{label}: {value}"));
        }
    }
    if !entry.text.is_empty() {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(entry.text.clone());
    }
    lines.join("\n")
}

/// The number a file name starts with (`atoi`): a monster type's help id
/// is its picture's ("6.bmp").
fn leading_number(text: &str) -> i64 {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

/// What the original's About box shows (Help > About…, dialog 100): its
/// lines, the installation's `License.txt` and the lines of its Credits
/// window (dialog 412). Read from the Expansion's program at run time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OriginalAbout {
    pub lines: Vec<String>,
    pub license: Option<String>,
    pub credits: Vec<String>,
}

impl Installation {
    /// The original's About box, as far as the installation has it.
    /// The help of the controls of the original's screen `screen` (a
    /// [`topic`] number): the hotspots of its dialog template whose help
    /// id is a topic of `Program.hlp`.
    pub fn screen_help(&self, screen: u32) -> ScreenHelp {
        // A dialog's own id, or a control's help id (0x8000_0000 | dialog
        // << 16 | control).
        let dialog = if screen < 0x1_0000 {
            screen
        } else {
            (screen >> 16) & 0x7FFF
        };
        let controls = u16::try_from(dialog).ok().and_then(|dialog| {
            crate::exe::Image::open(self, crate::treasure::Flavor::Expansion)
                .ok()?
                .dialog_controls(dialog)
        });
        let Some(program) = self.help_file("Program.hlp").ok() else {
            return ScreenHelp::default();
        };
        let mut texts = Vec::new();
        let mut titles = Vec::new();
        for control in controls.unwrap_or_default() {
            // The original draws its screens as pictures, with a hotspot
            // for each control of these styles (0x40F036).
            if control.help == 0 || control.style & 0x0C == 0 || control.style & 0xC0 != 0 {
                continue;
            }
            let Some(n) = program.topic_of_number(control.help) else {
                continue;
            };
            if let Some(text) = &control.text {
                texts.push((comparable(text), control.help));
            }
            titles.push((comparable(&program.topics[n].display_title()), control.help));
        }
        texts.extend(titles);
        texts.retain(|(t, _)| !t.is_empty());
        ScreenHelp { texts }
    }

    /// What a right-click on a record's row opens, as the original's
    /// list rows do (0x4B3B22): the shipped records' topics (`Database/`)
    /// whose help id is a topic of their file, the user database's
    /// records' descriptions, and the rows the original knows by name.
    pub fn item_help(&self) -> ItemHelp {
        let mut files: HashMap<&str, Option<HelpFile>> = HashMap::new();
        let mut help = ItemHelp::default();
        for (kind, entry) in self.custom_records() {
            let text = custom_text(&entry);
            help.add(
                kind,
                &entry.title,
                ItemFound::Custom {
                    title: entry.title.clone(),
                    text,
                },
            );
        }
        let mut add = |help: &mut ItemHelp,
                       kind,
                       name: &str,
                       file: &'static str,
                       number: i64,
                       popup: bool| {
            let Ok(number) = u32::try_from(number) else {
                return;
            };
            let known = files
                .entry(file)
                .or_insert_with(|| self.help_file(file).ok())
                .as_ref()
                .is_some_and(|h| number > 0 && h.topic_of_number(number).is_some());
            if known {
                let topic = ItemTopic {
                    file,
                    number,
                    popup,
                };
                help.add(kind, name, ItemFound::Topic(topic));
            }
        };
        if let Ok(parts) = self.load_parts() {
            for (part_type, objects) in parts.arrays.iter().enumerate() {
                let Some((file, popup)) = part_help(part_type) else {
                    continue;
                };
                for object in objects {
                    if let Object::Part(p) = object {
                        let number = p.head.unknown_0x1e4.into();
                        add(&mut help, ItemKind::Part, &p.head.name, file, number, popup);
                    }
                }
            }
        }
        if let Ok(spells) = self.load_spells() {
            for object in spells.wizard.iter().chain(&spells.priest) {
                if let Object::Spell(s) = object {
                    let number = s.id.into();
                    add(
                        &mut help,
                        ItemKind::Spell,
                        &s.name,
                        "Spells.hlp",
                        number,
                        false,
                    );
                }
            }
        }
        let types: HashMap<String, i64> = self
            .shipped("MonType")
            .iter()
            .filter_map(|o| match o {
                Object::MonsterType(t) => Some((comparable(&t.name), leading_number(&t.picture))),
                _ => None,
            })
            .collect();
        for name in ["Race", "Class", "Psionic", "Monster", "MonType"] {
            for object in self.shipped(name) {
                let (kind, name, file, number) = match &object {
                    Object::Race(r) => (ItemKind::Character, &r.name, "Charactr.hlp", r.id.into()),
                    Object::Class(c) => {
                        help.classes.insert(comparable(c.name()));
                        let cr_formats::classes::Class::Expansion { data, .. } = &**c else {
                            continue;
                        };
                        let number = data.unknown_0xc.into();
                        (ItemKind::Character, &data.name, "Charactr.hlp", number)
                    }
                    Object::PsionicPower(p) => {
                        let number = p.unknown_0x24.into();
                        (ItemKind::Psionic, &p.name, "Psionics.hlp", number)
                    }
                    Object::Monster(m) => {
                        let number = monster_number(&m.name, m.unknown_0x3b8, &types);
                        (ItemKind::Monster, &m.name, "Monsters.hlp", number)
                    }
                    Object::MonsterType(t) => {
                        let number = leading_number(&t.picture);
                        (ItemKind::Monster, &t.name, "Monsters.hlp", number)
                    }
                    _ => continue,
                };
                add(&mut help, kind, name, file, number, false);
            }
        }
        // The rows the original knows by name.
        let fixed = |file, number, popup| ItemTopic {
            file,
            number,
            popup,
        };
        let mut known = |help: &mut ItemHelp, kind, name: &str, topic: ItemTopic| {
            let file = topic.file;
            add(help, kind, name, file, topic.number.into(), topic.popup);
        };
        // A language row "Regional" (0x4B4C45).
        known(
            &mut help,
            ItemKind::Part,
            "Regional",
            fixed("Prof.hlp", 1046, true),
        );
        for (name, number) in PSIONIC_MODES {
            known(
                &mut help,
                ItemKind::Psionic,
                name,
                fixed("Psionics.hlp", number, true),
            );
        }
        for (style, number) in FIGHTING_STYLES {
            if let Some((name, ..)) = crate::create::FIGHTING_STYLES
                .iter()
                .find(|(_, n, _)| *n == style)
            {
                known(
                    &mut help,
                    ItemKind::Part,
                    name,
                    fixed("Prof.hlp", number, true),
                );
            }
        }
        // The weapon groups' rows (0x4B42BD, 0x4B42F1): the group topics.
        if let Ok(image) = crate::exe::Image::open(self, crate::treasure::Flavor::Expansion) {
            let mut tables = image.id_tables(0x67_7978, 0x3800, 0x57_2030);
            for (table, number) in [(0x248, 3017), (0x234, 3018)] {
                for (name, _) in tables.remove(&table).unwrap_or_default() {
                    if name == "None" {
                        continue;
                    }
                    known(
                        &mut help,
                        ItemKind::Part,
                        &name,
                        fixed("Equip.hlp", number, false),
                    );
                }
            }
        }
        for (word, number) in DISCIPLINES {
            help.within.push((
                comparable(word),
                ItemKind::Psionic,
                fixed("Psionics.hlp", number, true),
            ));
        }
        // A spell list's spell book (0x4B5235).
        help.within.push((
            comparable("Book of Infinite Spells"),
            ItemKind::Spell,
            fixed("Magic.hlp", 149, false),
        ));
        help
    }

    /// The records of `Database/<name>.dat`.
    fn shipped(&self, name: &str) -> Vec<Object> {
        self.read(&format!("Database/{name}.dat"))
            .ok()
            .and_then(|data| ObjectList::parse(&data).ok())
            .map(|list| list.objects)
            .unwrap_or_default()
    }

    pub fn original_about(&self) -> OriginalAbout {
        let image = crate::exe::Image::open(self, crate::treasure::Flavor::Expansion).ok();
        let texts = |id| {
            image
                .as_ref()
                .and_then(|i| i.dialog_texts(id))
                .map(|(_, texts)| texts)
                .unwrap_or_default()
        };
        let tidy = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
        let lines = texts(100).iter().map(|t| tidy(&t.text)).collect();
        // The Credits window draws its lines sunken (SS_SUNKEN); its title
        // and OK button are static texts too. A line starting with spaces
        // goes on with the one before.
        let mut credits: Vec<String> = Vec::new();
        for line in texts(412).iter().filter(|t| t.style & 0x1000 != 0) {
            match credits.last_mut() {
                Some(last) if line.text.starts_with(' ') => {
                    *last = tidy(&format!("{last} {}", line.text));
                }
                _ => credits.push(tidy(&line.text)),
            }
        }
        let license = self
            .path("License.txt")
            .and_then(|p| std::fs::read(p).ok())
            .map(|d| {
                d.iter()
                    .map(|&b| cr_formats::rtf::cp1252(b))
                    .collect::<String>()
            })
            .map(|t| t.replace("\r\n", "\n").trim().to_owned())
            .filter(|t| !t.is_empty());
        OriginalAbout {
            lines,
            license,
            credits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_screen_lists_its_own_kind_of_record_first() {
        use ItemKind::*;
        assert_eq!(
            ItemKind::order(topic::RACES),
            [Character, Part, Spell, Psionic, Monster]
        );
        assert_eq!(
            ItemKind::order(topic::EQUIPMENT),
            [Part, Spell, Character, Psionic, Monster]
        );
        assert_eq!(
            ItemKind::order(topic::MONSTERS),
            [Monster, Part, Spell, Character, Psionic]
        );
    }

    /// With `CORERULES_DIR` (an Expansion installation): every row the
    /// original knows by name opens its fixed topic.
    #[test]
    fn the_rows_the_original_knows_by_name_open_their_topics() {
        let Some(dir) = std::env::var_os("CORERULES_DIR") else {
            return;
        };
        let install = Installation::open(std::path::Path::new(&dir)).unwrap();
        if install.path("Help").is_none() {
            return;
        }
        let help = install.item_help();
        let found = |name: &str, screen| match help.find(name, &ItemKind::order(screen)) {
            Some(ItemFound::Topic(topic)) => Some((topic.file, topic.number, topic.popup)),
            _ => None,
        };
        let number = |name: &str| found(name, topic::WEAPON_PROFICIENCIES);
        for (name, n) in PSIONIC_MODES {
            let mode = found(name, topic::SKILLS);
            assert_eq!(mode, Some(("Psionics.hlp", n, true)), "{name}");
        }
        for (style, n) in FIGHTING_STYLES {
            let (name, ..) = crate::create::FIGHTING_STYLES
                .iter()
                .find(|(_, s, _)| *s == style)
                .unwrap();
            assert_eq!(number(name), Some(("Prof.hlp", n, true)), "{name}");
        }
        for (word, n) in DISCIPLINES {
            let row = format!("{word} Devotions");
            assert_eq!(number(&row), Some(("Psionics.hlp", n, true)), "{row}");
        }
        for group in [3017, 3018] {
            let rows = help
                .names
                .values()
                .flatten()
                .filter(|(_, found)| {
                    matches!(found, ItemFound::Topic(t) if t.file == "Equip.hlp" && t.number == group)
                })
                .count();
            assert!(rows > 1, "{group}: {rows}");
        }
        let book = number("Book of Infinite Spells (2 pages)");
        assert_eq!(book, Some(("Magic.hlp", 149, false)));
    }

    #[test]
    fn control_texts_compare_without_case_mnemonics_or_ellipses() {
        assert_eq!(comparable("&Help..."), "help");
        assert_eq!(comparable("_Points  Left:"), "points left");
        assert_eq!(comparable("Next …"), "next");
    }

    #[test]
    fn contents_lines_are_headings_or_topics() {
        let contents = Contents::parse(
            b":Base Manual.HLP\r\n:Title Online Manual\r\n1 Online Manual \r\n\
              2 Introduction=Online_Manual\r\n:include Players.cnt\r\n\
              3 Table=Table_PH@players.hlp>main\r\n",
        );
        assert_eq!(contents.base.as_deref(), Some("Manual.HLP"));
        assert_eq!(contents.title.as_deref(), Some("Online Manual"));
        assert_eq!(contents.includes, [(2, "Players.cnt".to_owned())]);
        assert_eq!(
            contents.entries,
            [
                ContentsEntry {
                    level: 1,
                    title: "Online Manual".into(),
                    context: None,
                    file: None,
                },
                ContentsEntry {
                    level: 2,
                    title: "Introduction".into(),
                    context: Some("Online_Manual".into()),
                    file: None,
                },
                ContentsEntry {
                    level: 3,
                    title: "Table".into(),
                    context: Some("Table_PH".into()),
                    file: Some("players.hlp".into()),
                },
            ]
        );
    }
}
