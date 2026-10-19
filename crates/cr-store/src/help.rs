//! The installation's help: the WinHelp files in `Help/` and their
//! contents files (`.cnt`), as the Help… buttons, the Help menu and the
//! books' Help versions open them.

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
