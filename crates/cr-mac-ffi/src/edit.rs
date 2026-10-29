//! Edit Character, as the Mac app draws it: the form and its checks come
//! from `cr_app::editor`. The prototype draws the Details tab's
//! description of the character; the other tabs come as the Mac app gets
//! them.

use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::create::Answer;
use cr_app::editor::{EditEffect, EditInput, EditNeeds, Editor};
use cr_store::Installation;

use crate::{CoreError, PromptInfo};

/// What the Details form shows.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EditDetails {
    pub name: String,
    pub player: String,
    pub male: bool,
    pub alignments: Vec<String>,
    /// The alignment, by its place in `alignments`.
    pub alignment: u32,
    /// Years.
    pub age: u32,
    /// Inches.
    pub height: u32,
    /// Pounds.
    pub weight: u32,
    pub history: String,
}

/// What Save came to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SaveState {
    /// The character was written.
    Saved { name: String },
    /// A question or message waits on an answer first.
    Asking,
    /// Nothing was saved: nothing changed, or the form has a problem.
    NotSaved,
}

/// Edit Character, open in a window of its own.
#[derive(uniffi::Object)]
pub struct EditSession {
    install: Arc<Installation>,
    editor: Mutex<Editor>,
}

impl EditSession {
    pub(crate) fn open(install: Arc<Installation>, file: &str) -> Result<Arc<Self>, CoreError> {
        let character = install.character(file)?;
        let needs = EditNeeds::load(&install)?;
        let editor = Editor::new(Arc::new(needs), file, character);
        Ok(Arc::new(Self {
            install,
            editor: Mutex::new(editor),
        }))
    }

    fn editor(&self) -> MutexGuard<'_, Editor> {
        self.editor.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn input(&self, input: EditInput) {
        self.editor().input(input);
    }

    /// Does what the editor asks of the app: writes the character.
    fn settle(&self, editor: &mut Editor) -> SaveState {
        let mut state = if editor.prompt().is_some() {
            SaveState::Asking
        } else {
            SaveState::NotSaved
        };
        for effect in editor.take_effects() {
            match effect {
                EditEffect::Save(saving) => {
                    match self.install.save_character(
                        &saving.file,
                        &saving.details,
                        Some(&saving.lists),
                    ) {
                        Ok(()) => {
                            state = SaveState::Saved {
                                name: saving.details.name.trim().to_owned(),
                            }
                        }
                        Err(e) => editor.failed(e.to_string()),
                    }
                }
            }
        }
        state
    }
}

#[uniffi::export]
impl EditSession {
    /// The program help's topic of the tab in view.
    pub fn help(&self) -> u32 {
        self.editor().help()
    }

    /// The character's name as it was read, for the window's title.
    pub fn title(&self) -> String {
        self.editor().character().data.name.trim().to_owned()
    }

    /// What the Details form shows now.
    pub fn details(&self) -> EditDetails {
        let editor = self.editor();
        let d = &editor.details;
        EditDetails {
            name: d.name.clone(),
            player: d.player.clone(),
            male: d.male,
            alignment: editor
                .alignments
                .iter()
                .position(|a| *a == d.alignment)
                .unwrap_or(0) as u32,
            alignments: editor.alignments.clone(),
            age: d.age,
            height: d.height,
            weight: d.weight,
            history: d.history.clone(),
        }
    }

    pub fn set_name(&self, name: String) {
        self.input(EditInput::Name(name));
    }

    pub fn set_player(&self, player: String) {
        self.input(EditInput::Player(player));
    }

    pub fn set_male(&self, male: bool) {
        self.input(EditInput::Gender(if male { 0 } else { 1 }));
    }

    /// The alignment, by its place in [`EditDetails::alignments`].
    pub fn set_alignment(&self, index: u32) {
        self.input(EditInput::Alignment(index as usize));
    }

    pub fn set_age(&self, years: u32) {
        self.input(EditInput::Age(years));
    }

    pub fn set_height(&self, inches: u32) {
        self.input(EditInput::Height(inches));
    }

    pub fn set_weight(&self, pounds: u32) {
        self.input(EditInput::Weight(pounds));
    }

    pub fn set_history(&self, history: String) {
        self.input(EditInput::History(history));
    }

    /// Whether anything was changed.
    pub fn changed(&self) -> bool {
        self.editor().changed()
    }

    /// Why the character cannot be saved as it is, if it cannot.
    pub fn problem(&self) -> Option<String> {
        self.editor().problem()
    }

    /// Whether Save may be pressed.
    pub fn can_save(&self) -> bool {
        self.editor().can_save()
    }

    /// Why the character could not be written, when it could not.
    pub fn error(&self) -> Option<String> {
        self.editor().error.clone()
    }

    /// The question or message in view.
    pub fn prompt(&self) -> Option<PromptInfo> {
        self.editor().prompt().map(PromptInfo::of)
    }

    /// Save: checks the character, asks what the original asks, and
    /// writes it.
    pub fn save(&self) -> SaveState {
        let mut editor = self.editor();
        editor.error = None;
        editor.input(EditInput::Save);
        self.settle(&mut editor)
    }

    /// Answers the prompt in view, which may go on to save.
    pub fn answer(&self, yes: bool) -> SaveState {
        let mut editor = self.editor();
        editor.answer(if yes { Answer::Yes } else { Answer::No });
        self.settle(&mut editor)
    }
}
