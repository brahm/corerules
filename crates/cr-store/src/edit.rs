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
        }
    }

    /// Why the original would not accept these details, if it would not.
    pub fn problem(&self) -> Option<String> {
        if self.name.trim().is_empty() {
            return Some("A character needs a name".into());
        }
        if self
            .abilities
            .iter()
            .flatten()
            .any(|v| !ABILITY_RANGE.contains(v))
        {
            return Some("Ability scores go from 1 to 25".into());
        }
        if self.exceptional_strength > 100 {
            return Some("Exceptional strength goes up to 100".into());
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
        if let Some(problem) = details.problem() {
            return Err(Error::Invalid(problem));
        }
        let mut character = self.character(file)?;
        details.apply(&mut character);
        let format = |source| Error::Format {
            path: file.into(),
            source,
        };
        let bytes = character.to_bytes().map_err(format)?;

        let index_path = self
            .path("Chars/index.chr")
            .ok_or_else(|| Error::Invalid("The character list is missing".into()))?;
        let mut index = self.characters()?;
        let entry = index
            .entries
            .iter_mut()
            .find(|e| e.file.eq_ignore_ascii_case(file))
            .ok_or_else(|| Error::Invalid(format!("{file} is not in the character list")))?;
        entry.name.clone_from(&details.name);
        for line in &mut entry.details {
            if line.starts_with(PLAYER_LINE) {
                *line = format!("{PLAYER_LINE}{}", details.player);
            }
        }
        let index_bytes = index.to_bytes().map_err(format)?;
        // Both files are checked before either is replaced.
        Character::parse(&bytes).map_err(format)?;
        CharacterIndex::parse(&index_bytes).map_err(format)?;

        let path = self
            .path(&character_path(file)?)
            .ok_or_else(|| Error::Invalid(format!("{file} not found")))?;
        replace(&path, &bytes)?;
        replace(&index_path, &index_bytes)
    }
}

/// The detail line of an Expansion index entry that names the player.
const PLAYER_LINE: &str = "Player's Name: ";

/// `Chars/<file>`, refusing names that would leave the folder.
fn character_path(file: &str) -> Result<String> {
    if file.is_empty() || file.contains(['/', '\\']) || file.starts_with('.') {
        return Err(Error::Invalid(format!(
            "{file} is not a character file name"
        )));
    }
    Ok(format!("Chars/{file}"))
}

/// Keeps the current contents of `path` as `<path>.bak`, then replaces it
/// through a temporary file in the same folder.
fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
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

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
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
