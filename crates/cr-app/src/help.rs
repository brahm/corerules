//! The help as both apps show it: the installation's WinHelp files
//! (`Help/*.hlp`), the topic a target names, the list beside the topic
//! (the contents file's entries, the file's titled topics, or the topics
//! a search finds), where a link leads, and what a right-click on a
//! control of one of the original's screens shows.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use cr_formats::winhelp::{HelpFile, Link, Macro, Topic};
use cr_store::Installation;
use cr_store::help::{Contents, ItemFound, ItemHelp, ItemKind, ItemTopic, ScreenHelp, topic};
use cr_store::sheet::Sheet;
use cr_store::summary::Summary;

/// The help file of the program, whose topics the screens' Help open.
pub const PROGRAM: &str = "Program.hlp";
/// The Online Manual and the Tome of Help, as the Help menu opens them.
pub const MANUAL: &str = "Manual.hlp";
pub const TOME: &str = "ADandD.hlp";

/// Which topic to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// The file's contents topic.
    Contents,
    /// A context string, as the contents files and links name topics.
    Context(String),
    /// A `[MAP]` number, as the original's Help buttons pass them.
    Number(u32),
}

impl Target {
    /// The file and the topic of the help action's parameter:
    /// `Program.hlp#1234` (a number), `ADandD.hlp#ctx` (a context string)
    /// or `Manual.hlp` (its contents).
    pub fn parse(spec: &str) -> (&str, Self) {
        let (file, topic) = spec.split_once('#').unwrap_or((spec, ""));
        let target = if topic.is_empty() {
            Self::Contents
        } else if let Ok(n) = topic.parse() {
            Self::Number(n)
        } else {
            Self::Context(topic.to_owned())
        };
        (file, target)
    }
}

/// The help action's parameter for topic `number` of the program help.
pub fn program(number: u32) -> String {
    format!("{PROGRAM}#{number}")
}

/// A topic of a help file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    /// The file's name in `Help/`, lower case.
    pub file: String,
    pub topic: usize,
}

/// A row of the list beside the topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// 1 for the top level.
    pub level: u32,
    pub title: String,
    pub place: Option<Place>,
    /// A contents entry's file and context string, found when chosen:
    /// the Tome of Help lists every book's topics.
    pub context: Option<(String, String)>,
}

impl Row {
    /// Whether it opens a topic, rather than heading the rows under it.
    pub fn is_topic(&self) -> bool {
        self.place.is_some() || self.context.is_some()
    }
}

/// Where a link leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Followed {
    /// A topic, shown in place of the one with the link.
    Go(Place),
    /// A topic shown in a popup over it, as WinHelp's glossary terms.
    Popup(Place),
    /// A web page, opened in the browser.
    Web(String),
    /// A topic that is not in this installation.
    Missing,
}

/// The title a topic shows, its file's own when it has none: a file
/// without a contents topic opens on an empty one.
pub fn title(help: &HelpFile, topic: &Topic) -> String {
    Some(topic.display_title())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| help.title.clone())
}

/// The help files of an installation, each read once.
pub struct Files {
    install: Installation,
    files: Mutex<HashMap<String, Option<Arc<HelpFile>>>>,
}

impl Files {
    pub fn new(install: Installation) -> Self {
        Self {
            install,
            files: Mutex::default(),
        }
    }

    pub fn install(&self) -> &Installation {
        &self.install
    }

    /// The help file `name`, if it reads.
    pub fn file(&self, name: &str) -> Option<Arc<HelpFile>> {
        let key = name.to_ascii_lowercase();
        let read = |files: &mut HashMap<String, Option<Arc<HelpFile>>>| {
            files
                .entry(key.clone())
                .or_insert_with(|| self.install.help_file(name).ok().map(Arc::new))
                .clone()
        };
        read(&mut self.files.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// The topic of `file` that `target` names.
    pub fn resolve(&self, file: &str, target: &Target) -> Option<Place> {
        let help = self.file(file)?;
        let topic = match target {
            Target::Contents => Some(help.contents),
            Target::Context(name) => help.topic_of_context(name),
            Target::Number(n) => help.topic_of_number(*n),
        }?;
        Some(Place {
            file: file.to_ascii_lowercase(),
            topic,
        })
    }

    /// The rows beside a topic of `file` (lower case): the topics whose
    /// title or text holds `query`, or else the contents file's entries,
    /// or else the file's titled topics in alphabetical order, as
    /// WinHelp's Index tab lists them.
    pub fn rows(&self, file: &str, query: &str) -> Vec<Row> {
        let Some(help) = self.file(file) else {
            return Vec::new();
        };
        let query = query.trim().to_lowercase();
        let topic_row = |file: &str, i: usize, title: String| Row {
            level: 1,
            title,
            context: None,
            place: Some(Place {
                file: file.to_owned(),
                topic: i,
            }),
        };
        let mut rows = Vec::new();
        if !query.is_empty() {
            // The files the contents include are searched too: the Tome
            // of Help's search covers every book.
            let mut files = vec![(file.to_owned(), help)];
            if let Some(contents) = self.install.help_contents(file) {
                for name in contents.entries.iter().filter_map(|e| e.file.as_deref()) {
                    let name = name.to_ascii_lowercase();
                    if files.iter().all(|(f, _)| *f != name)
                        && let Some(other) = self.file(&name)
                    {
                        files.push((name, other));
                    }
                }
            }
            for (file, help) in files {
                for (i, topic) in help.topics.iter().enumerate() {
                    let title = topic.display_title();
                    let found = title.to_lowercase().contains(&query)
                        || topic
                            .paragraphs
                            .iter()
                            .any(|p| p.text().to_lowercase().contains(&query));
                    if found && !title.is_empty() {
                        rows.push(topic_row(&file, i, title));
                    }
                }
            }
        } else if let Some(contents) = self.install.help_contents(file) {
            rows = contents_rows(file, &contents);
        } else {
            for (i, topic) in help.topics.iter().enumerate() {
                let title = topic.display_title();
                if !title.is_empty() {
                    rows.push(topic_row(file, i, title));
                }
            }
            rows.sort_by_cached_key(|r| r.title.to_lowercase());
        }
        rows
    }

    /// Where a row leads; a contents entry's file is read then.
    pub fn row_place(&self, row: &Row) -> Option<Place> {
        match &row.context {
            Some((file, context)) => self.resolve(file, &Target::Context(context.clone())),
            None => row.place.clone(),
        }
    }

    /// The row of `rows` that is `place`, to be marked.
    pub fn row_of(&self, rows: &[Row], place: &Place) -> Option<usize> {
        rows.iter().position(|r| match &r.context {
            // Only rows of the file shown can be it.
            Some((file, _)) => *file == place.file && self.row_place(r).as_ref() == Some(place),
            None => r.place.as_ref() == Some(place),
        })
    }

    /// Where `link`, in a topic of `from` (lower case), leads.
    pub fn follow(&self, from: &str, link: &Link) -> Followed {
        let (place, popup) = match link {
            Link::Topic { hash, file, popup } => {
                let name = file.clone().unwrap_or_else(|| from.to_owned());
                let place = self.file(&name).and_then(|h| {
                    Some(Place {
                        file: name.to_ascii_lowercase(),
                        topic: h.topic_of_hash(*hash)?,
                    })
                });
                (place, *popup)
            }
            Link::Offset { offset, popup } => {
                let place = self.file(from).and_then(|h| {
                    Some(Place {
                        file: from.to_owned(),
                        topic: h.topic_at(*offset)?,
                    })
                });
                (place, *popup)
            }
            Link::Macro(text) => match Macro::parse(text) {
                Some(Macro::Web(uri)) => return Followed::Web(uri),
                Some(Macro::Jump {
                    file,
                    context,
                    popup,
                }) => (
                    self.resolve(file.as_deref().unwrap_or(from), &Target::Context(context)),
                    popup,
                ),
                Some(Macro::Contents { file }) => (
                    self.resolve(file.as_deref().unwrap_or(from), &Target::Contents),
                    false,
                ),
                None => (None, false),
            },
        };
        match place {
            Some(place) if popup => Followed::Popup(place),
            Some(place) => Followed::Go(place),
            None => Followed::Missing,
        }
    }
}

/// Whether `rows` already list the file of `place`: a contents file that
/// includes it, as the Tome of Help's does every book's, stays listed.
pub fn lists(rows: &[Row], place: &Place) -> bool {
    rows.iter()
        .any(|r| r.context.as_ref().is_some_and(|(f, _)| *f == place.file))
}

fn contents_rows(file: &str, contents: &Contents) -> Vec<Row> {
    contents
        .entries
        .iter()
        .map(|entry| Row {
            level: entry.level,
            title: entry.title.clone(),
            place: None,
            context: entry.context.as_ref().map(|context| {
                let file = entry.file.as_deref().unwrap_or(file);
                (file.to_ascii_lowercase(), context.clone())
            }),
        })
        .collect()
}

/// What a right-click on a control or a row shows.
#[derive(Debug, Clone, PartialEq)]
pub enum Found {
    /// A topic of a help file: in a popup over the screen, or in the help.
    Topic(ItemTopic),
    /// The original's Custom Help box (dialog 394): what a record of the
    /// user database says. It prints.
    Custom { title: String, text: String },
    /// The original's summary of a character or trap, as text.
    Text { title: String, text: String },
    /// A page shown as it prints: a monster of the user database, or the
    /// summary of an encounter or a treasure.
    Page(Sheet),
}

/// The screens whose rows the original summarizes rather than open their
/// help.
pub fn summarized(screen: u32) -> bool {
    [
        topic::ENCOUNTERS,
        topic::ENCOUNTER_TEMPLATES,
        topic::TRAPS,
        topic::TREASURE_BUNDLES,
        topic::CHARACTER_MANAGEMENT,
    ]
    .contains(&screen)
}

/// What a right-click on the original's screen `screen` (a [`topic`]
/// number) at a control that reads one of `texts` (as shown, or in
/// English), nearest first, shows: the first that is one of the screen's
/// hotspots (`controls`, a popup of the program help), a record the screen
/// summarizes (its summary) or a record (`items`: its topic in its book's
/// help, or what a record of the user database says). `items` is only
/// read for the screens that are not [`summarized`].
pub fn context_help(
    install: &Installation,
    screen: u32,
    texts: &[&str],
    controls: &ScreenHelp,
    items: impl FnOnce() -> Arc<ItemHelp>,
) -> Option<Found> {
    let items = (!summarized(screen)).then(items);
    texts.iter().find_map(|&text| {
        let english = std::iter::once(text).chain(cr_i18n::sources(text));
        let control = english
            .clone()
            .find_map(|t| controls.topic_of(t))
            .map(|number| {
                Found::Topic(ItemTopic {
                    file: PROGRAM,
                    number,
                    popup: true,
                })
            });
        control.or_else(|| match &items {
            Some(items) => match items.find(text, &ItemKind::order(screen))? {
                ItemFound::Topic(found) => Some(Found::Topic(found)),
                ItemFound::Custom { title, text } => Some(Found::Custom { title, text }),
                ItemFound::CustomMonster(name) => {
                    install.custom_monster_sheet(&name).map(Found::Page)
                }
            },
            None => match install.summary(screen, text)? {
                Summary::Text { title, text } => Some(Found::Text { title, text }),
                Summary::Page(sheet) => Some(Found::Page(sheet)),
            },
        })
    })
}

/// What the Monsters screen's View and Print (0x6D3BFA, 0x6D2B4F) open
/// for the monster `name`.
#[derive(Debug, Clone, PartialEq)]
pub enum MonsterHelp {
    /// A monster of the user database, as it prints.
    Page(Sheet),
    /// A shipped monster's topic of `Monsters.hlp`.
    Topic(ItemTopic),
    /// A shipped monster with no topic.
    None,
}

/// What View and Print show for the monster `name`: a monster of the user
/// database (`custom`) as it prints, a shipped one's help topic.
pub fn monster(
    install: &Installation,
    name: &str,
    custom: bool,
    items: impl FnOnce() -> Arc<ItemHelp>,
) -> MonsterHelp {
    if custom {
        return install
            .custom_monster_sheet(name)
            .map_or(MonsterHelp::None, MonsterHelp::Page);
    }
    items()
        .topic_of(name, &[ItemKind::Monster])
        .map_or(MonsterHelp::None, MonsterHelp::Topic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spec_names_a_number_a_context_or_the_contents() {
        assert_eq!(
            Target::parse("Program.hlp#1234"),
            ("Program.hlp", Target::Number(1234))
        );
        assert_eq!(
            Target::parse("ADandD.hlp#idh_spells"),
            ("ADandD.hlp", Target::Context("idh_spells".into()))
        );
        assert_eq!(
            Target::parse("Manual.hlp"),
            ("Manual.hlp", Target::Contents)
        );
        assert_eq!(program(7), "Program.hlp#7");
    }

    #[test]
    fn headings_open_nothing() {
        let heading = Row {
            level: 1,
            title: "Spells".into(),
            place: None,
            context: None,
        };
        assert!(!heading.is_topic());
        let entry = Row {
            context: Some(("phb.hlp".into(), "idh_1".into())),
            ..heading.clone()
        };
        assert!(entry.is_topic());
        let place = Place {
            file: "phb.hlp".into(),
            topic: 3,
        };
        assert!(lists(std::slice::from_ref(&entry), &place));
        assert!(!lists(&[heading], &place));
    }

    #[test]
    fn summaries_are_for_the_screens_the_original_summarizes() {
        assert!(summarized(topic::TRAPS));
        assert!(!summarized(topic::SPELLS));
    }
}
