//! The library: the catalog's sections, the entries of the chosen one as
//! the user filters them, and the commands each entry has. Both apps show
//! the library from here, so they list and offer the same things.

use cr_i18n::tr;
use cr_store::books::BOOK_KEY;
use cr_store::catalog::{ENCOUNTER_KEY, Entry, Section, TABLE_KEY, TEMPLATES, TREASURE_KEY};
use cr_store::userdb::parts::{OTHER_SECTION, OTHER_TYPES};
use cr_store::userdb::{self, Filter as Origins};

use crate::action::{self, Action};
use crate::records;

/// The id of the characters' section.
pub const CHARACTERS: &str = "characters";

/// Whether a list row whose title and subtitle are these matches what was
/// typed in the search box, case ignored: the title or the subtitle holds
/// it.
pub fn matches(search: &str, title: &str, subtitle: &str) -> bool {
    let needle = search.to_lowercase();
    needle.is_empty()
        || title.to_lowercase().contains(&needle)
        || subtitle.to_lowercase().contains(&needle)
}

/// What the entry list shows of a section.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Filter {
    pub search: String,
    /// All, standard or custom records, in the sections of the user
    /// databases.
    pub origins: Origins,
    /// The characters shown: every one (`None`), those in no party
    /// (empty) or those of a party.
    pub party: Option<String>,
    /// The type of the other parts shown (an id of [`OTHER_TYPES`]), or
    /// every type.
    pub part_type: Option<&'static str>,
    /// Magic Only, in the sections of parts (0x68fe59).
    pub magic_only: bool,
}

impl Filter {
    /// Whether the list of section `section` shows `entry`. Each filter
    /// applies only in the sections that offer it.
    pub fn shows(&self, section: &str, entry: &Entry) -> bool {
        let origin = records::origin_of(&entry.key).is_none_or(|o| self.origins.accepts(o));
        let party = section != CHARACTERS
            || self
                .party
                .as_ref()
                .is_none_or(|wanted| entry.party.eq_ignore_ascii_case(wanted));
        let of_type = section != OTHER_SECTION
            || self
                .part_type
                .is_none_or(|wanted| part_type(&entry.key) == Some(wanted));
        let magic = !self.magic_only || !is_parts(section) || entry.magic;
        origin && party && of_type && magic && matches(&self.search, &entry.title, &entry.subtitle)
    }

    /// The positions of the entries of `section` that the list shows.
    pub fn visible(&self, section: &Section) -> Vec<usize> {
        section
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| self.shows(section.id, e))
            .map(|(i, _)| i)
            .collect()
    }
}

/// Whether section `id` lists parts, as the original's Parts list does,
/// and so offers Magic Only.
pub fn is_parts(id: &str) -> bool {
    id == OTHER_SECTION
        || [
            "kits",
            "weapons",
            "armor",
            "equipment",
            "magic-items",
            "nonweapon",
        ]
        .contains(&id)
}

/// The type of one of the other parts (an id of [`OTHER_TYPES`]), by
/// its entry's key.
pub fn part_type(key: &str) -> Option<&'static str> {
    let (id, ..) = userdb::parse_key(key)?;
    OTHER_TYPES.iter().map(|(i, ..)| *i).find(|i| *i == id)
}

/// The choices of the party filter: every character, then each party
/// by name, then no party when some character has none. Without parties
/// there is only the first.
pub fn parties(sections: &[Section]) -> Vec<Option<String>> {
    let mut parties: Vec<String> = Vec::new();
    let mut unassigned = false;
    for entry in sections
        .iter()
        .filter(|s| s.id == CHARACTERS)
        .flat_map(|s| &s.entries)
    {
        if entry.party.is_empty() {
            unassigned = true;
        } else if !parties.iter().any(|p| p.eq_ignore_ascii_case(&entry.party)) {
            parties.push(entry.party.clone());
        }
    }
    parties.sort_by_cached_key(|p| cr_store::collate::sort_key(p));
    let mut choices = vec![None];
    if !parties.is_empty() {
        choices.extend(parties.into_iter().map(Some));
        if unassigned {
            choices.push(Some(String::new()));
        }
    }
    choices
}

/// A choice of the party filter as it is listed.
pub fn party_label(choice: Option<&str>) -> &str {
    match choice {
        None => tr("All Parties"),
        Some("") => tr("No Party"),
        Some(party) => party,
    }
}

/// Where `choice` is among the party filter's `choices` now, after they
/// were made again: the same party, or every character when it is gone.
pub fn party_position(choices: &[Option<String>], choice: Option<&str>) -> usize {
    choice
        .and_then(|c| {
            choices
                .iter()
                .position(|p| p.as_deref().is_some_and(|p| p.eq_ignore_ascii_case(c)))
        })
        .unwrap_or(0)
}

/// The command that adds an entry to section `id`, with its tooltip.
pub fn new_command(id: &str) -> Option<(Action, String)> {
    Some(match id {
        CHARACTERS => (action::NEW_CHARACTER, tr("New Character").into()),
        "encounters" => (action::NEW_ENCOUNTER, tr("New Encounter").into()),
        "tables" => (action::NEW_TABLE, tr("New Encounter Table").into()),
        "treasure" => (action::ROLL_TREASURE, tr("Roll Treasure").into()),
        id if records::can_create(id) => (action::UDB_NEW, records::new_tooltip(id)),
        _ => return None,
    })
}

/// Whether the entry with `key` is a character, whose key is its file.
pub fn is_character(key: &str) -> bool {
    !key.is_empty() && !key.contains(':')
}

/// The name of a monster, and whether it is a custom one, by its key.
pub fn monster_key(key: &str) -> Option<(&str, bool)> {
    userdb::parse_key(key)
        .filter(|(section, ..)| *section == "monsters")
        .map(|(_, origin, name)| (name, origin == userdb::Origin::Custom))
}

/// Whether the record with `key` shows a monster type's picture: a
/// monster type, monster, template or saved encounter, the records of the
/// original's Monsters screen.
pub fn is_monster(key: &str) -> bool {
    key.starts_with(ENCOUNTER_KEY)
        || userdb::parse_key(key).is_some_and(|(section, ..)| {
            ["monsters", "monster-types", TEMPLATES].contains(&section)
        })
}

/// Whether the record with `key` has a page to view and print
/// ([`cr_store::Installation::page`]).
pub fn has_page(key: &str) -> bool {
    [ENCOUNTER_KEY, TABLE_KEY, TREASURE_KEY]
        .iter()
        .any(|p| key.starts_with(p))
        || userdb::parse_key(key)
            .is_some_and(|(section, ..)| section == TEMPLATES || section == "treasure-tables")
}

/// The short name of a book of the Books section (`PhbBk`), by its key.
pub fn book_name(key: &str) -> Option<&str> {
    key.strip_prefix(BOOK_KEY)
}

/// The name of an encounter template, by its key.
pub fn template_name(key: &str) -> Option<&str> {
    userdb::parse_key(key)
        .filter(|(section, ..)| *section == TEMPLATES)
        .map(|(.., name)| name)
}

/// What can be done with an entry, as the commands of its details.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Commands {
    /// Edit: a character, a record of the user's, a treasure table, a
    /// saved treasure, encounter or encounter table.
    pub edit: bool,
    /// Copy a saved encounter or treasure.
    pub copy: bool,
    /// Copy, New and Delete of a user database record
    /// ([`records`]).
    pub records: bool,
    /// The character sheet.
    pub sheet: bool,
    /// Convert to Character, for a character the NPC generator made.
    pub to_character: bool,
    /// Convert to NPC, for a player character.
    pub to_npc: bool,
    /// Delete a character.
    pub delete: bool,
    /// Create an encounter from a template or an encounter table.
    pub generate: bool,
    /// View: a monster's help topic, or the page of an encounter, table,
    /// template or treasure.
    pub view: bool,
    /// Whether View shows a monster's help topic.
    pub monster: bool,
    /// Print the page that View shows.
    pub print: bool,
    /// Read Book, for a book of the Books section.
    pub read: bool,
}

impl Commands {
    /// The commands of `entry`; `writable` is false for an installation
    /// that is only read (Core Rules 2.0).
    pub fn of(entry: &Entry, writable: bool) -> Self {
        let key = entry.key.as_str();
        let character = is_character(key);
        let monster = monster_key(key).is_some();
        let page = monster || has_page(key);
        Self {
            edit: writable
                && (character
                    || records::editable(key)
                    || [TREASURE_KEY, ENCOUNTER_KEY, TABLE_KEY]
                        .iter()
                        .any(|p| key.starts_with(p))),
            copy: writable
                && [TREASURE_KEY, ENCOUNTER_KEY]
                    .iter()
                    .any(|p| key.starts_with(p)),
            records: writable && userdb::parse_key(key).is_some(),
            sheet: character,
            to_character: writable && character && entry.npc,
            to_npc: writable && character && !entry.npc,
            delete: writable && character,
            generate: writable && (template_name(key).is_some() || key.starts_with(TABLE_KEY)),
            view: page,
            monster,
            print: page,
            read: book_name(key).is_some(),
        }
    }
}

/// The tooltip of Generate for the entry with `key`.
pub fn generate_label(key: &str) -> &'static str {
    if key.starts_with(TABLE_KEY) {
        tr("Roll and Create Encounter")
    } else {
        tr("Create Encounter")
    }
}

/// Text from the original's Windows edit boxes, cleaned up to be shown:
/// its line ends made one, tabs made spaces.
pub fn clean_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "    ")
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, title: &str, party: &str) -> Entry {
        Entry {
            key: key.into(),
            title: title.into(),
            subtitle: "Elf Ranger".into(),
            party: party.into(),
            ..Entry::default()
        }
    }

    fn section(id: &'static str, entries: Vec<Entry>) -> Section {
        Section {
            id,
            title: "",
            icon: "",
            entries,
            error: None,
        }
    }

    #[test]
    fn the_title_or_the_subtitle_holds_the_search() {
        assert!(matches("", "Lirael", "Elf Ranger"));
        assert!(matches("elf", "Lirael", "Elf Ranger"));
        assert!(matches("LIR", "Lirael", "Elf Ranger"));
        assert!(!matches("lirael elf", "Lirael", "Elf Ranger"));
    }

    #[test]
    fn filters_apply_only_where_offered() {
        let filter = Filter {
            party: Some("Blades".into()),
            part_type: Some("languages"),
            magic_only: true,
            ..Filter::default()
        };
        // Spells have no party, type or magic: the filters let them be.
        let spell = entry("udb:wizard-spells:standard:Sleep", "Sleep", "");
        assert!(filter.shows("wizard-spells", &spell));
        let lirael = entry("LIRAEL.CHR", "Lirael", "blades");
        let borin = entry("BORIN.CHR", "Borin", "");
        assert!(filter.shows(CHARACTERS, &lirael));
        assert!(!filter.shows(CHARACTERS, &borin));
        let common = entry("udb:languages:standard:Common", "Common", "");
        assert!(!filter.shows(OTHER_SECTION, &common), "not magic");
        let filter = Filter {
            origins: Origins::Custom,
            ..Filter::default()
        };
        assert!(!filter.shows(OTHER_SECTION, &common));
        assert!(filter.shows(CHARACTERS, &borin));
    }

    #[test]
    fn parties_are_listed_with_every_character_first() {
        let sections = [section(
            CHARACTERS,
            vec![
                entry("A", "A", "zeta"),
                entry("B", "B", ""),
                entry("C", "C", "Alpha"),
                entry("D", "D", "ZETA"),
            ],
        )];
        let choices = parties(&sections);
        assert_eq!(
            choices,
            [
                None,
                Some("Alpha".into()),
                Some("zeta".into()),
                Some(String::new())
            ]
        );
        assert_eq!(party_position(&choices, Some("Zeta")), 2);
        assert_eq!(party_position(&choices, Some("gone")), 0);
        let alone = [section(CHARACTERS, vec![entry("B", "B", "")])];
        assert_eq!(parties(&alone), [None]);
    }

    #[test]
    fn commands_follow_the_entry_and_the_installation() {
        let mut npc = entry("NPC.CHR", "Grog", "");
        npc.npc = true;
        let commands = Commands::of(&npc, true);
        assert!(commands.sheet && commands.edit && commands.to_character && commands.delete);
        assert!(!commands.to_npc && !commands.view);
        let read_only = Commands::of(&npc, false);
        assert!(read_only.sheet && !read_only.edit && !read_only.delete);
        let orc = Commands::of(&entry("udb:monsters:standard:Orc", "Orc", ""), true);
        assert!(orc.view && orc.monster && orc.records && !orc.edit);
        let treasure = Commands::of(&entry("treasure:Hoard", "Hoard", ""), true);
        assert!(treasure.view && !treasure.monster && treasure.copy && treasure.edit);
        let book = Commands::of(&entry(&format!("{BOOK_KEY}PhbBk"), "Handbook", ""), true);
        assert!(book.read && !book.view && !book.edit && !book.sheet);
        assert!(!orc.read);
        assert_eq!(book_name(&format!("{BOOK_KEY}PhbBk")), Some("PhbBk"));
    }
}
