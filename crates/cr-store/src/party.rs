//! Parties and deleting characters: the Character dialog's Party
//! Management… and Delete.
//!
//! The Expansion has no list of parties. A character's party is a name kept
//! in the character (doc +0x1dc0) and in its list entry (+0x2c), and the
//! parties are the names the list entries hold, so a party without members
//! does not last (CCharPartyManagementDlg, 0x5ab040…0x5ac3f2 in TSRP3.EXE).
//! Assigning a character to a party, or taking it out of one (0x5ab684,
//! 0x5abb05), saves it as a new character, like an import does: a new file
//! in `Chars/` and a new entry at the end of the list, rebuilt with the
//! class and level lines, and the old file and entry go. Core Rules 2.0 has
//! no parties.
//!
//! Delete (0x44b7cf in TSRP3.EXE, the same in EVERMORE.EXE) removes the
//! character's file and its entry; the party's Delete (0x5abffd) does so for
//! every member. Here the file is kept as `<file>.bak` until
//! [`Installation::forget_deleted`], so a deletion can be undone.

use cr_i18n::tr;
use std::fs;
use std::path::PathBuf;

use cr_formats::character::{CharacterEntry, CharacterIndex};

use crate::create::{new_file_name, write_new};
use crate::edit::{replace, with_suffix};
use crate::export::expansion_entry;
use crate::userdb::parts::{KITS, all_names};
use crate::{Error, Installation, Result};

/// The name the original's party lists use for every character; no party
/// may have it.
pub const ALL: &str = "~All";

/// The longest party name the original takes (DDX_MaxChars, 0x5ab2a4).
pub const MAX_NAME: usize = 50;

/// The party of a list entry; empty for none (and in Core Rules 2.0).
pub fn party_of(entry: &CharacterEntry) -> &str {
    entry.extra.as_ref().map_or("", |e| e.unknown_0x2c.as_str())
}

/// A character [`Installation::delete_character`] removed, for
/// [`Installation::restore_characters`].
#[derive(Debug, Clone, PartialEq)]
pub struct Deleted {
    pub name: String,
    pub file: String,
    entry: CharacterEntry,
    /// Where the entry was in the list.
    position: usize,
    /// The character file, renamed.
    backup: PathBuf,
}

impl Installation {
    /// Whether characters can be in parties (the Expansion).
    pub fn has_parties(&self) -> bool {
        self.path("UserDbas/AllowedU.dat").is_some()
    }

    /// The parties the characters are in, sorted by name, case ignored.
    pub fn parties(&self) -> Result<Vec<String>> {
        let mut parties: Vec<String> = Vec::new();
        for entry in self.characters()?.entries {
            let party = party_of(&entry);
            if !party.is_empty() && !parties.iter().any(|p| p.eq_ignore_ascii_case(party)) {
                parties.push(party.to_owned());
            }
        }
        parties.sort_by_cached_key(|p| crate::collate::sort_key(p));
        Ok(parties)
    }

    /// Why `name` cannot be a new party next to the characters' parties and
    /// `others` (new parties without members yet), if it cannot (0x5ab40a).
    pub fn party_name_problem(&self, name: &str, others: &[String]) -> Result<Option<String>> {
        let name = name.trim();
        if name.is_empty() {
            return Ok(Some(tr("A name must be supplied for the new party").into()));
        }
        if name.eq_ignore_ascii_case(ALL) {
            return Ok(Some(tr!("{all} is not a party name", all = ALL)));
        }
        if name.chars().count() > MAX_NAME {
            return Ok(Some(tr!(
                "A party name has at most {max} characters",
                max = MAX_NAME
            )));
        }
        let parties = self.parties()?;
        if parties
            .iter()
            .chain(others)
            .any(|p| p.eq_ignore_ascii_case(name))
        {
            return Ok(Some(tr!("The party name {name} already exists", name)));
        }
        Ok(None)
    }

    /// Puts the character of `Chars/<file>` in `party` (none when empty);
    /// returns the character's new file name.
    pub fn set_party(&self, file: &str, party: &str) -> Result<String> {
        if !self.has_parties() {
            return Err(Error::Invalid(tr("Core Rules 2.0 has no parties").into()));
        }
        let party = party.trim();
        if party.eq_ignore_ascii_case(ALL) {
            return Err(Error::Invalid(tr!(
                "You may not add to the {category} category",
                category = ALL
            )));
        }
        let format = |path: &str| {
            let path = PathBuf::from(path);
            move |source| Error::Format { path, source }
        };
        let mut index = self.characters()?;
        let position = position(&index, file)?;
        let mut character = self.character(file)?.upgraded().map_err(format(file))?;
        if let Some(lines) = character.data.unknown_0x1e68.as_mut() {
            lines[3] = party.into();
        }
        let bytes = character.to_bytes().map_err(format(file))?;
        let levels = self
            .advance_rules()?
            .levels(&character)?
            .iter()
            .map(|c| c.level.to_string())
            .collect::<Vec<_>>()
            .join("/");
        let kits = all_names(self)?.get(KITS).cloned().unwrap_or_default();
        let chars = self.chars_folder()?;
        let new = new_file_name(&chars, &index);
        index.entries.remove(position);
        index
            .entries
            .push(expansion_entry(&character, &new, &kits, Some(&levels)));
        let index_bytes = index.to_bytes().map_err(format("Chars/index.chr"))?;
        CharacterIndex::parse(&index_bytes).map_err(format("Chars/index.chr"))?;
        write_new(&chars.join(&new), &bytes)?;
        self.write_index(&index_bytes)?;
        let old = chars.join(file);
        fs::remove_file(&old).map_err(|source| Error::Io { path: old, source })?;
        Ok(new)
    }

    /// Removes the character of `Chars/<file>` from the list, keeping its
    /// file as `<file>.bak`.
    pub fn delete_character(&self, file: &str) -> Result<Deleted> {
        Ok(self.delete_characters(&[file])?.remove(0))
    }

    /// Removes every member of `party` from the list (0x5abffd).
    pub fn delete_party(&self, party: &str) -> Result<Vec<Deleted>> {
        let index = self.characters()?;
        let files: Vec<&str> = index
            .entries
            .iter()
            .filter(|e| !party.is_empty() && party_of(e).eq_ignore_ascii_case(party))
            .map(|e| e.file.as_str())
            .collect();
        if files.is_empty() {
            return Ok(Vec::new());
        }
        self.delete_characters(&files)
    }

    fn delete_characters(&self, files: &[&str]) -> Result<Vec<Deleted>> {
        let mut index = self.characters()?;
        let chars = self.chars_folder()?;
        let mut deleted = Vec::new();
        for file in files {
            let position = position(&index, file)?;
            let entry = index.entries.remove(position);
            deleted.push(Deleted {
                name: entry.name.clone(),
                file: entry.file.clone(),
                backup: with_suffix(&chars.join(&entry.file), ".bak"),
                entry,
                position,
            });
        }
        let index_bytes = self.index_bytes(&index)?;
        self.write_index(&index_bytes)?;
        for d in &deleted {
            let path = chars.join(&d.file);
            fs::rename(&path, &d.backup).map_err(|source| Error::Io { path, source })?;
        }
        Ok(deleted)
    }

    /// Puts back what [`Self::delete_character`] or [`Self::delete_party`]
    /// removed, given in the order it was removed.
    pub fn restore_characters(&self, deleted: &[Deleted]) -> Result<()> {
        let mut index = self.characters()?;
        if index.schema == 0 {
            // An empty list keeps no schema.
            index.schema = crate::create::INDEX_SCHEMA;
        }
        let chars = self.chars_folder()?;
        for d in deleted.iter().rev() {
            if index
                .entries
                .iter()
                .any(|e| e.file.eq_ignore_ascii_case(&d.file))
            {
                return Err(Error::Invalid(tr!(
                    "{file} is in the list already",
                    file = d.file
                )));
            }
            let at = d.position.min(index.entries.len());
            index.entries.insert(at, d.entry.clone());
        }
        if let Some(d) = deleted.iter().find(|d| chars.join(&d.file).exists()) {
            return Err(Error::Invalid(tr!(
                "{file} exists already",
                file = format!("Chars/{}", d.file)
            )));
        }
        let index_bytes = self.index_bytes(&index)?;
        for d in deleted {
            let path = chars.join(&d.file);
            fs::rename(&d.backup, &path).map_err(|source| Error::Io {
                path: d.backup.clone(),
                source,
            })?;
        }
        self.write_index(&index_bytes)
    }

    /// Removes the files of characters that will not be put back.
    pub fn forget_deleted(&self, deleted: &[Deleted]) -> Result<()> {
        for d in deleted {
            fs::remove_file(&d.backup).map_err(|source| Error::Io {
                path: d.backup.clone(),
                source,
            })?;
        }
        Ok(())
    }

    pub(crate) fn chars_folder(&self) -> Result<PathBuf> {
        self.path("Chars")
            .ok_or_else(|| Error::Invalid(tr("The installation has no Chars folder").into()))
    }

    fn index_bytes(&self, index: &CharacterIndex) -> Result<Vec<u8>> {
        let format = |source| Error::Format {
            path: "Chars/index.chr".into(),
            source,
        };
        let bytes = index.to_bytes().map_err(format)?;
        CharacterIndex::parse(&bytes).map_err(format)?;
        Ok(bytes)
    }

    pub(crate) fn write_index(&self, bytes: &[u8]) -> Result<()> {
        match self.path("Chars/index.chr") {
            Some(path) => replace(&path, bytes),
            None => write_new(&self.chars_folder()?.join("index.chr"), bytes),
        }
    }
}

fn position(index: &CharacterIndex, file: &str) -> Result<usize> {
    index
        .entries
        .iter()
        .position(|e| e.file.eq_ignore_ascii_case(file))
        .ok_or_else(|| Error::Invalid(tr!("{file} is not in the character list", file)))
}
