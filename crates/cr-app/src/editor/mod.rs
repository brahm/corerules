//! Edit Character with no toolkit in it: the Expansion's Update Character
//! as the pages of one form, saved at once, with the questions and
//! messages the original asks as the character changes. The Windows app
//! draws the pages; the GNOME app's editor takes from here the texts and
//! checks they share.
//!
//! The form keeps the details as edited, the lists as read and changed by
//! the pages, and a copy of the character that the levels, experience,
//! hit points and PSPs change; Save compares that copy with the character
//! as read.

mod details;

use std::sync::Arc;

use cr_formats::character::Character;
use cr_i18n::tr_noop;
use cr_store::Installation;
use cr_store::advance::{AdvanceRules, Advancement};
use cr_store::create::{CreationRules, Dice};
use cr_store::dual::ClassTable;
use cr_store::edit::CharacterDetails;
use cr_store::help::topic;
use cr_store::lists::CharacterLists;
use cr_store::npc::Rand;

use crate::levels::{ExperienceForm, LevelsForm, LevelsInput};
use crate::prompt::{Answer, Prompt as AnyPrompt};

pub use details::{ClassOffer, MOST_AGE, MOST_HEIGHT, MOST_POINTS, MOST_WEIGHT, class_offer};

/// A question, warning or message the editor waits on.
pub type Prompt = AnyPrompt<Editor>;

/// What the editor reads from the installation, once.
pub struct EditNeeds {
    pub creation: Arc<CreationRules>,
    pub advance: Arc<AdvanceRules>,
    /// The dual-class rules, for the classes the character may add.
    pub class_table: Arc<ClassTable>,
    /// The Background… dialog's events, "NONE" first.
    pub backgrounds: Vec<String>,
}

impl EditNeeds {
    pub fn load(install: &Installation) -> cr_store::Result<Self> {
        Ok(Self {
            creation: Arc::new(install.creation_rules()?),
            advance: Arc::new(install.advance_rules()?),
            class_table: Arc::new(install.class_table()?),
            backgrounds: cr_store::edit::backgrounds(install)?,
        })
    }
}

/// A page of the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Details,
}

impl Tab {
    /// The pages, in order.
    pub const ALL: &[Tab] = &[Tab::Details];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Details => tr_noop!("Details"),
        }
    }

    /// The original's Update Character page the page stands for.
    pub fn help(self) -> u32 {
        match self {
            Tab::Details => topic::CHARACTER_INFORMATION,
        }
    }
}

/// A change in the editor, or one of its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum EditInput {
    Tab(Tab),
    // Details.
    Name(String),
    Player(String),
    SocialClass(String),
    /// The portrait's folder and file name, empty for none.
    Portrait([String; 2]),
    Gender(usize),
    Alignment(usize),
    Kit(usize),
    Background(usize),
    RandomBackground,
    History(String),
    Age(u32),
    Height(u32),
    Weight(u32),
    /// An ability (0 to 5), which of its score and two subabilities, and
    /// the value.
    Ability(usize, usize, u32),
    Exceptional(u32),
    /// A class of those offered to add, by index.
    AddClass(usize),
    RemoveClass,
    // Experience.
    /// "Levels…".
    OpenLevels,
    Levels(LevelsInput),
    CloseLevels,
    /// "Add…": give or take away experience.
    OpenExperience,
    /// A change in Add Experience, or its Add or Cancel.
    Experience(LevelsInput),
    HitPoints(u32),
    /// The PSPs, all levels together.
    Psp(u32),
    Save,
}

/// What the app does for the editor.
#[derive(Debug, Clone, PartialEq)]
pub enum EditEffect {
    /// Save: the app writes the character, and says how it went with
    /// [`Editor::saved`] or [`Editor::failed`].
    Save(Box<Saving>),
}

/// What Save writes into the character stored in `Chars/<file>`.
#[derive(Debug, Clone, PartialEq)]
pub struct Saving {
    pub file: String,
    pub details: CharacterDetails,
    pub lists: CharacterLists,
}

/// The editor over one character.
#[derive(Clone)]
pub struct Editor {
    needs: Arc<EditNeeds>,
    file: String,
    original: Box<Character>,
    original_details: CharacterDetails,
    pub details: CharacterDetails,
    lists: CharacterLists,
    /// The character at the levels, experience, hit points and PSPs the
    /// form has now.
    advanced: Box<Character>,
    pub tab: Tab,
    /// The alignments the classes allow, the character's own among them.
    pub alignments: Vec<String>,
    /// Every kit, the character's own among them.
    pub kits: Vec<String>,
    /// The background events, the character's own among them.
    pub backgrounds: Vec<String>,
    /// The class Add Class chose, which starts at level 1.
    pub new_class: Option<String>,
    /// The Levels dialog, while it is open.
    pub levels: Option<Box<LevelsForm>>,
    /// Add Experience, while it is open.
    pub experience: Option<ExperienceForm>,
    /// Counts Add Experience's openings and failed tries, for an app
    /// whose dialog closes on Add to open it again.
    pub experience_serial: u64,
    prompt: Option<Prompt>,
    serial: u64,
    effects: Vec<EditEffect>,
    /// Why the character could not be saved.
    pub error: Option<String>,
    /// Save is under way.
    pub saving: bool,
    dice: Dice,
    rand: Rand,
}

impl Editor {
    /// The editor over `character`, stored in `Chars/<file>`.
    pub fn new(needs: Arc<EditNeeds>, file: &str, character: Character) -> Self {
        let details = CharacterDetails::of(&character);
        let lists = CharacterLists::of(&character);
        let with = |mut items: Vec<String>, current: &str| {
            if !items.iter().any(|i| i == current) {
                items.push(current.to_owned());
            }
            items
        };
        let creation = &needs.creation;
        let alignments = with(
            creation.alignments(&character.data.classes.names),
            &details.alignment,
        );
        let kits = with(creation.all_kits(), &details.kit);
        let backgrounds = if needs.backgrounds.is_empty() {
            Vec::new()
        } else {
            with(needs.backgrounds.clone(), &details.background)
        };
        Self {
            file: file.to_owned(),
            original_details: details.clone(),
            details,
            lists,
            advanced: Box::new(character.clone()),
            original: Box::new(character),
            tab: Tab::Details,
            alignments,
            kits,
            backgrounds,
            new_class: None,
            levels: None,
            experience: None,
            experience_serial: 0,
            prompt: None,
            serial: 0,
            effects: Vec::new(),
            error: None,
            saving: false,
            dice: Dice::default(),
            rand: Rand::default(),
            needs,
        }
    }

    /// The character's file in `Chars`.
    pub fn file(&self) -> &str {
        &self.file
    }

    /// The character at the levels the form has now.
    pub fn character(&self) -> &Character {
        &self.advanced
    }

    pub fn needs(&self) -> &EditNeeds {
        &self.needs
    }

    /// Whether anything was changed.
    pub fn changed(&self) -> bool {
        self.details != self.original_details
            || self.new_class.is_some()
            || *self.advanced != *self.original
    }

    /// Why the character cannot be saved as it is, if it cannot.
    pub fn problem(&self) -> Option<String> {
        self.details.problem()
    }

    /// Whether Save may be pressed.
    pub fn can_save(&self) -> bool {
        !self.saving && self.prompt.is_none() && self.problem().is_none()
    }

    /// The question or message in view.
    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    /// Which prompt is in view, so that an answer to one that is gone is
    /// not taken for the next.
    pub fn prompt_serial(&self) -> u64 {
        self.serial
    }

    /// What the app should do now, once.
    pub fn take_effects(&mut self) -> Vec<EditEffect> {
        std::mem::take(&mut self.effects)
    }

    /// The character could not be written, for `why`.
    pub fn failed(&mut self, why: String) {
        self.saving = false;
        self.error = Some(why);
    }

    /// The help topic of the page in view.
    pub fn help(&self) -> u32 {
        if self.levels.is_some() || self.experience.is_some() {
            return topic::LEVELS;
        }
        self.tab.help()
    }

    fn show(&mut self, prompt: Prompt) {
        self.prompt = Some(prompt);
        self.serial += 1;
    }

    fn message(&mut self, title: &str, text: impl Into<String>) {
        self.show(Prompt::message(title, text));
    }

    /// Answers the prompt in view.
    pub fn answer(&mut self, answer: Answer) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        if let Some(then) = prompt.then(answer) {
            then(self);
        }
    }

    /// Makes a change, or presses a button.
    pub fn input(&mut self, input: EditInput) {
        if self.prompt.is_some() || self.saving {
            return;
        }
        match input {
            EditInput::Tab(tab) => self.tab = tab,
            EditInput::Save => self.save(),
            input => self.details_input(input),
        }
    }

    /// Asks the app to write the character.
    fn save(&mut self) {
        if !self.can_save() {
            return;
        }
        let mut details = self.details.clone();
        for text in [
            &mut details.name,
            &mut details.player,
            &mut details.social_class,
        ] {
            *text = text.trim().to_owned();
        }
        let mut lists = self.lists.clone();
        lists.new_classes = self.new_class.iter().cloned().collect();
        let advancement = Advancement::between(&self.original, &self.advanced);
        if advancement != Advancement::of(&self.original) {
            lists.advancement = Some(advancement);
        }
        self.error = None;
        self.saving = true;
        self.effects.push(EditEffect::Save(Box::new(Saving {
            file: self.file.clone(),
            details,
            lists,
        })));
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The editor over the first character of the installation
    /// `CORERULES_DIR` names, if it is set and has one.
    pub(crate) fn editor() -> Option<Editor> {
        let dir = std::env::var_os("CORERULES_DIR")?;
        let install = Installation::open(std::path::Path::new(&dir)).ok()?;
        let file = install.characters().ok()?.entries.first()?.file.clone();
        let character = install.character(&file).ok()?;
        let needs = Arc::new(EditNeeds::load(&install).ok()?);
        Some(Editor::new(needs, &file, character))
    }

    #[test]
    fn save_gives_the_details_and_the_lists() {
        let Some(mut e) = editor() else {
            return;
        };
        assert!(!e.changed());
        e.input(EditInput::Name("  Someone Else ".into()));
        assert!(e.changed());
        e.input(EditInput::Save);
        let effects = e.take_effects();
        let [EditEffect::Save(saving)] = effects.as_slice() else {
            panic!("{effects:?}");
        };
        assert_eq!(saving.details.name, "Someone Else");
        assert_eq!(saving.lists.advancement, None);
        assert!(saving.lists.new_classes.is_empty());
        // Nothing more while it saves.
        e.input(EditInput::Name("Other".into()));
        assert_eq!(e.details.name, "  Someone Else ");
        e.failed("No".into());
        assert_eq!(e.error.as_deref(), Some("No"));
        assert!(e.can_save());
    }

    #[test]
    fn an_empty_name_cannot_be_saved() {
        let Some(mut e) = editor() else {
            return;
        };
        e.input(EditInput::Name("  ".into()));
        assert!(!e.can_save());
        e.input(EditInput::Save);
        assert!(e.take_effects().is_empty());
    }
}
