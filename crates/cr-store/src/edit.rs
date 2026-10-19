//! Changing characters in place, in files both original programs can open.
//!
//! The original trusts what a character file stores and recomputes only
//! what derives from it (saving throws, hit point adjustments), so these
//! fields can change on their own. The character list (`Chars/index.chr`)
//! repeats the name and, in the Expansion, the player; the program never
//! refreshes an entry when it saves a character in place, so both are
//! updated here. Before a file is replaced its previous bytes are kept next to
//! it as `<file>.bak`, and the new file is written to a temporary name
//! first so a failed write never leaves half a character behind.

use std::fs;
use std::path::{Path, PathBuf};

use cr_formats::character::{Character, CharacterIndex};

use cr_i18n::tr;

use crate::lists::CharacterLists;
use crate::{Error, Installation, Result};

/// The editable details of a character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDetails {
    pub name: String,
    pub player: String,
    pub social_class: String,
    /// Inches.
    pub height: u32,
    /// Pounds.
    pub weight: u32,
    /// Years.
    pub age: u32,
    /// Strength, Dexterity, Constitution, Intelligence, Wisdom and
    /// Charisma, each as the score and its two Skills & Powers
    /// subabilities (Stamina and Muscle for Strength). The racial
    /// adjustment is part of the score.
    pub abilities: [[u32; 3]; 6],
    /// 1 to 100 for a warrior with Strength 18, else 0.
    pub exceptional_strength: u32,
    /// Read only: with the Core Rules, the program keeps both subabilities
    /// equal to the score.
    pub skills_and_powers: bool,
    /// The portrait's folder and file name, both empty for none.
    pub portrait: [String; 2],
    pub alignment: String,
    pub male: bool,
    /// The kit's name, `None` for none.
    pub kit: String,
    /// The background event (Background…), `NONE` for none.
    pub background: String,
    /// The character history (Background…).
    pub history: String,
}

/// Ability scores the original accepts.
pub const ABILITY_RANGE: std::ops::RangeInclusive<u32> = 1..=25;

impl CharacterDetails {
    pub fn of(character: &Character) -> Self {
        let d = &character.data;
        Self {
            name: d.name.clone(),
            player: d.player.clone(),
            social_class: d.social_class.clone(),
            height: d.height,
            weight: d.weight,
            age: d.age,
            abilities: [
                d.strength,
                d.abilities[0],
                d.abilities[1],
                d.abilities[2],
                d.abilities[3],
                d.abilities[4],
            ],
            exceptional_strength: d.exceptional_strength,
            skills_and_powers: character.skills_and_powers(),
            portrait: d.unknown_0x1db8.clone(),
            alignment: d.alignment.clone(),
            male: d.unknown_0x163c != 0,
            kit: d.kit.clone(),
            background: d.unknown_0x90.clone(),
            history: d.unknown_0x94.clone(),
        }
    }

    /// Why the original would not accept these details, if it would not.
    pub fn problem(&self) -> Option<String> {
        if self.name.trim().is_empty() {
            return Some(tr("A character needs a name").into());
        }
        if self
            .abilities
            .iter()
            .flatten()
            .any(|v| !ABILITY_RANGE.contains(v))
        {
            return Some(tr("Ability scores go from 1 to 25").into());
        }
        if self.exceptional_strength > 100 {
            return Some(tr("Exceptional strength goes up to 100").into());
        }
        None
    }

    pub fn apply(&self, character: &mut Character) {
        let d = &mut character.data;
        d.name.clone_from(&self.name);
        d.player.clone_from(&self.player);
        d.social_class.clone_from(&self.social_class);
        d.height = self.height;
        d.weight = self.weight;
        d.age = self.age;
        d.strength = self.abilities[0];
        d.abilities.copy_from_slice(&self.abilities[1..]);
        d.exceptional_strength = self.exceptional_strength;
        d.unknown_0x1db8.clone_from(&self.portrait);
        d.alignment.clone_from(&self.alignment);
        // The original writes 1 to 3 for a male; a male stays as stored.
        if !self.male {
            d.unknown_0x163c = 0;
        } else if d.unknown_0x163c == 0 {
            d.unknown_0x163c = 1;
        }
        d.kit.clone_from(&self.kit);
        d.unknown_0x90.clone_from(&self.background);
        d.unknown_0x94.clone_from(&self.history);
    }
}

impl Installation {
    /// Reads the character stored in `Chars/<file>`.
    pub fn character(&self, file: &str) -> Result<Character> {
        let relative = character_path(file)?;
        let data = self.read(&relative)?;
        Character::parse(&data).map_err(|source| Error::Format {
            path: relative.into(),
            source,
        })
    }

    /// Writes `details` into the character stored in `Chars/<file>` and its
    /// entry in the character list.
    pub fn save_character_details(&self, file: &str, details: &CharacterDetails) -> Result<()> {
        self.save_character(file, details, None)
    }

    /// Writes `details` and, when given, `lists` into the character stored
    /// in `Chars/<file>` and its entry in the character list.
    pub fn save_character(
        &self,
        file: &str,
        details: &CharacterDetails,
        lists: Option<&CharacterLists>,
    ) -> Result<()> {
        if let Some(problem) = details.problem() {
            return Err(Error::Invalid(problem));
        }
        let mut character = self.character(file)?;
        let before_kit = character.data.kit.clone();
        details.apply(&mut character);
        if let Some(lists) = lists {
            lists.apply(&mut character, &self.part_table()?)?;
            if !lists.new_classes.is_empty() {
                let table = self.class_table()?;
                for class in &lists.new_classes {
                    table.add(&mut character, class)?;
                }
            }
            if let Some(advancement) = &lists.advancement {
                advancement.apply(&mut character);
            }
        }
        // The index entry lists the levels joined with "/".
        let levels = match lists {
            Some(l) if !l.new_classes.is_empty() || l.advancement.is_some() => Some(
                self.advance_rules()?
                    .levels(&character)?
                    .iter()
                    .map(|c| c.level.to_string())
                    .collect::<Vec<_>>()
                    .join("/"),
            ),
            _ => None,
        };
        let format = |source| Error::Format {
            path: file.into(),
            source,
        };
        let bytes = character.to_bytes().map_err(format)?;

        let index_path = self
            .path("Chars/index.chr")
            .ok_or_else(|| Error::Invalid(tr("The character list is missing").into()))?;
        let mut index = self.characters()?;
        let entry = index
            .entries
            .iter_mut()
            .find(|e| e.file.eq_ignore_ascii_case(file))
            .ok_or_else(|| Error::Invalid(tr!("{file} is not in the character list", file)))?;
        entry.name.clone_from(&details.name);
        let added = lists.is_some_and(|l| !l.new_classes.is_empty());
        for line in &mut entry.details {
            if line.starts_with(PLAYER_LINE) {
                *line = format!("{PLAYER_LINE}{}", details.player);
            } else if details.kit != before_kit && line.starts_with(KIT_LINE) {
                *line = format!("{KIT_LINE}{}", details.kit);
            } else if added && line.starts_with(CLASS_LINE) {
                *line = format!("{CLASS_LINE}{}", character.data.class_title);
            } else if let Some(levels) = levels.as_ref().filter(|_| line.starts_with(LEVEL_LINE)) {
                *line = format!("{LEVEL_LINE}{levels}");
            }
        }
        // Update Character writes a fresh entry, whose kit line names the
        // new kit; an entry without one gets it when the kit changes.
        let kit_line = entry.details.iter().any(|l| l.starts_with(KIT_LINE));
        if !kit_line && entry.version > 1 && details.kit != before_kit {
            entry.details.push(format!("{KIT_LINE}{}", details.kit));
        }
        let index_bytes = index.to_bytes().map_err(format)?;
        // Both files are checked before either is replaced.
        Character::parse(&bytes).map_err(format)?;
        CharacterIndex::parse(&index_bytes).map_err(format)?;

        let path = self
            .path(&character_path(file)?)
            .ok_or_else(|| Error::Invalid(tr!("{file} not found", file)))?;
        replace(&path, &bytes)?;
        replace(&index_path, &index_bytes)
    }
}

/// The detail line of an Expansion index entry that names the player.
pub(crate) const PLAYER_LINE: &str = "Player's Name: ";
/// The detail lines of an index entry with the class title and the levels
/// joined with "/".
pub(crate) const CLASS_LINE: &str = "Class: ";
pub(crate) const LEVEL_LINE: &str = "Level: ";
/// The detail line of an Expansion index entry that names the kit.
pub(crate) const KIT_LINE: &str = "Kit: ";

/// The background events of the Background… dialog, "NONE" first, as the
/// program lists them (pushed at 0x54f66a, the list the NPC generator
/// draws from).
pub fn backgrounds(install: &Installation) -> Result<Vec<String>> {
    let image = crate::exe::Image::open(install, crate::treasure::Flavor::Expansion)?;
    let mut list = image.pushed_texts(0x54f66a, 0x800);
    if list.len() < BACKGROUNDS || list[0] != "NONE" {
        return Err(Error::Invalid(
            tr("The program's background events were not found").into(),
        ));
    }
    list.truncate(BACKGROUNDS);
    Ok(list)
}

/// "NONE" and the 20 events.
const BACKGROUNDS: usize = 21;

/// `Chars/<file>`, refusing names that would leave the folder.
fn character_path(file: &str) -> Result<String> {
    if file.is_empty() || file.contains(['/', '\\']) || file.starts_with('.') {
        return Err(Error::Invalid(tr!(
            "{file} is not a character file name",
            file
        )));
    }
    Ok(format!("Chars/{file}"))
}

/// Keeps the current contents of `path` as `<path>.bak`, then replaces it
/// through a temporary file in the same folder.
pub(crate) fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| Error::Io { path, source }
    };
    let backup = with_suffix(path, ".bak");
    fs::copy(path, &backup).map_err(io(&backup))?;
    let temporary = with_suffix(path, ".tmp");
    fs::write(&temporary, bytes).map_err(io(&temporary))?;
    fs::rename(&temporary, path).map_err(io(path))
}

pub(crate) fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/characters"
    );

    fn copy_of_fixtures(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("cr-edit-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Database")).unwrap();
        fs::create_dir_all(root.join("Chars")).unwrap();
        for file in ["index.chr", "90688519.dat", "90988519.dat"] {
            fs::copy(
                Path::new(FIXTURES).join(file),
                root.join("Chars").join(file),
            )
            .unwrap();
        }
        root
    }

    #[test]
    fn saving_changes_only_what_was_edited() {
        let root = copy_of_fixtures("save");
        let install = Installation::open(&root).unwrap();
        let before = install.character("90688519.dat").unwrap();
        let mut details = CharacterDetails::of(&before);
        details.name = "Borin Pedraforte II".into();
        details.player = "Someone Else".into();
        details.age = 70;
        details.abilities[2] = [15; 3];
        install
            .save_character_details("90688519.dat", &details)
            .unwrap();

        let after = install.character("90688519.dat").unwrap();
        assert_eq!(CharacterDetails::of(&after), details);
        let mut expected = before.clone();
        details.apply(&mut expected);
        assert_eq!(after, expected);

        let index = install.characters().unwrap();
        assert_eq!(index.entries[0].name, "Borin Pedraforte II");
        assert_eq!(index.entries[0].details[1], "Player's Name: Someone Else");
        assert_eq!(after.data.abilities[1], [15; 3]);
        assert_eq!(index.entries[1].name, "Lirael Folhaverde");
        let original = fs::read(Path::new(FIXTURES).join("90688519.dat")).unwrap();
        assert_eq!(
            fs::read(root.join("Chars/90688519.dat.bak")).unwrap(),
            original
        );
        assert!(!root.join("Chars/90688519.dat.tmp").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_bad_input() {
        let root = copy_of_fixtures("refuse");
        let install = Installation::open(&root).unwrap();
        let mut details = CharacterDetails::of(&install.character("90988519.dat").unwrap());
        details.name = " ".into();
        assert!(
            install
                .save_character_details("90988519.dat", &details)
                .is_err()
        );
        details.name = "Lirael \u{263a}".into();
        assert!(
            install
                .save_character_details("90988519.dat", &details)
                .is_err()
        );
        details.name = "Lirael".into();
        details.abilities[0][1] = 26;
        assert!(
            install
                .save_character_details("90988519.dat", &details)
                .is_err()
        );
        assert!(install.character("../index.chr").is_err());
        let original = fs::read(Path::new(FIXTURES).join("90988519.dat")).unwrap();
        assert_eq!(fs::read(root.join("Chars/90988519.dat")).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }
}
