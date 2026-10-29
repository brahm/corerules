//! Roll Treasure, as the Mac app draws it in a window of its own: the
//! roller of `cr_app::treasure` described as a form ([`crate::form`]),
//! as the Windows app's page has it. How many of each treasure type to
//! roll, the coin and Accumulate Rolls, then the treasure with its edits
//! and the Calculate summary; the toolbar rolls, resets, saves, prints
//! and deletes the saved treasure.

use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::bundle::BundleInput;
use cr_app::treasure::{INDIVIDUAL, LAIR, MOST_COUNT, Roller, TreasureEffect, TreasureInput};
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::treasure::{Bundle, Counts, LETTERS, Rand, TreasureRules};

use crate::CoreError;
use crate::form::{Acts, Builder, FormDialog, FormPage, FormValue};

/// What the last Save came to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TreasureSaved {
    Saved { name: String },
    Failed { error: String },
}

/// Roll Treasure, open in its window.
#[derive(uniffi::Object)]
pub struct TreasureSession {
    install: Arc<Installation>,
    roller: Mutex<Roller>,
    /// What the controls of the form last described do.
    acts: Mutex<Acts<TreasureInput>>,
    rand: Arc<Mutex<Rand>>,
    /// What a Save came to, until the app takes it.
    saved: Mutex<Option<TreasureSaved>>,
}

impl TreasureSession {
    /// The roller on the saved treasure `saved`, or with `copy` on its
    /// items under no name, for Save to add a new one; else on nothing.
    pub(crate) fn open(
        install: Arc<Installation>,
        rules: Arc<TreasureRules>,
        saved: Option<String>,
        copy: bool,
        rand: Arc<Mutex<Rand>>,
    ) -> Result<Arc<Self>, CoreError> {
        let roller = match saved {
            None => Roller::new(rules, [0; 26], Bundle::default()),
            Some(name) => {
                let bundle = install
                    .saved_treasures()
                    .map_err(|e| CoreError::new(e.to_string()))?
                    .into_iter()
                    .find(|t| t.name == name)
                    .map(|t| t.bundle)
                    .unwrap_or_default();
                if copy {
                    Roller::new(rules, [0; 26], bundle)
                } else {
                    Roller::saved(rules, &name, bundle)
                }
            }
        };
        Ok(Arc::new(Self {
            install,
            roller: Mutex::new(roller),
            acts: Mutex::default(),
            rand,
            saved: Mutex::new(None),
        }))
    }

    /// The roller as an encounter's Roll Treasure… opens it: on the
    /// monsters' treasure types and the encounter's treasure, with Use
    /// Treasure.
    pub(crate) fn pick(
        install: Arc<Installation>,
        rules: Arc<TreasureRules>,
        counts: Counts,
        bundle: Bundle,
        rand: Arc<Mutex<Rand>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            install,
            roller: Mutex::new(Roller::pick(rules, counts, bundle)),
            acts: Mutex::default(),
            rand,
            saved: Mutex::new(None),
        })
    }

    /// The treasure as it is now, for Use Treasure.
    pub(crate) fn bundle(&self) -> Bundle {
        self.roller().bundle.clone()
    }

    fn roller(&self) -> MutexGuard<'_, Roller> {
        self.roller.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn input(&self, input: TreasureInput) {
        let mut roller = self.roller();
        let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
        roller.input(input, &mut rand);
        drop(rand);
        for effect in roller.take_effects() {
            match effect {
                TreasureEffect::Save { old, name, bundle } => {
                    let saved = match &old {
                        Some(old) => self.install.replace_saved_treasure(old, &name, &bundle),
                        None => self.install.save_treasure(&name, &bundle),
                    };
                    let said = match saved {
                        Ok(()) => {
                            roller.saved_as(name.clone());
                            TreasureSaved::Saved { name }
                        }
                        Err(e) => {
                            roller.saving = false;
                            TreasureSaved::Failed {
                                error: e.to_string(),
                            }
                        }
                    };
                    *self.saved.lock().unwrap_or_else(|e| e.into_inner()) = Some(said);
                }
            }
        }
    }
}

#[uniffi::export]
impl TreasureSession {
    /// The saved treasure's name, or Roll Treasure.
    pub fn title(&self) -> String {
        self.roller().title()
    }

    /// The program help's topic.
    pub fn help(&self) -> u32 {
        cr_store::help::topic::ROLL_TREASURE
    }

    /// The saved treasure the roller changes, by name.
    pub fn saved_name(&self) -> Option<String> {
        self.roller().saved.clone()
    }

    pub fn can_save(&self) -> bool {
        !self.roller().saving
    }

    /// The roller, as a form, with the dialog an edit or Save waits on.
    pub fn form(&self) -> FormPage {
        let roller = self.roller();
        let mut form = Builder::new();
        describe(&mut form, &roller);
        let dialog = crate::bundle::dialog(
            &mut form,
            &roller.editor,
            TreasureInput::Bundle,
            TreasureInput::BundleAnswer,
        )
        .or_else(|| naming(&mut form, &roller));
        let (groups, acts) = form.finish();
        *self.acts.lock().unwrap_or_else(|e| e.into_inner()) = acts;
        FormPage { groups, dialog }
    }

    /// Control `id` of the form last described changed to `value`.
    pub fn act(&self, id: u32, value: FormValue) {
        let input = self
            .acts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .input(id, &value);
        if let Some(input) = input {
            self.input(input);
        }
    }

    pub fn roll(&self) {
        self.input(TreasureInput::Roll);
    }

    /// Clears the treasure types and the treasure.
    pub fn reset(&self) {
        self.input(TreasureInput::Reset);
    }

    /// Save…: asks for the treasure's name.
    pub fn save(&self) {
        self.input(TreasureInput::Save);
    }

    /// What the last Save came to, once.
    pub fn take_saved(&self) -> Option<TreasureSaved> {
        self.saved.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    /// The treasure as a PDF, on Letter paper or A4: the items, nested,
    /// then the summary.
    pub fn pdf(&self, letter: bool) -> Vec<u8> {
        let paper = if letter {
            cr_app::pdf::Paper::Letter
        } else {
            cr_app::pdf::Paper::A4
        };
        cr_app::pdf::sheet(&self.roller().sheet(), paper)
    }

    /// Deletes the saved treasure the roller changes.
    pub fn delete_saved(&self) -> Result<(), CoreError> {
        let Some(name) = self.roller().saved.clone() else {
            return Ok(());
        };
        self.install
            .delete_saved_treasure(&name)
            .map_err(|e| CoreError::new(e.to_string()))
    }
}

/// The roller's page.
fn describe(form: &mut Builder<TreasureInput>, roller: &Roller) {
    for (title, caption, range) in [
        (tr("Lair"), tr("Treasure types A to I"), LAIR),
        (tr("Individual"), tr("Treasure types J to Z"), INDIVIDUAL),
    ] {
        form.group(title, caption);
        for i in range {
            let letter = LETTERS[i];
            form.number(
                (&tr!("Treasure type {letter}", letter), ""),
                roller.counts[i].into(),
                (0, MOST_COUNT.into()),
                Vec::new(),
                move |n| TreasureInput::Count(i, n as u32),
            );
        }
    }
    form.group(tr("Options"), "")
        .toggle(
            (
                tr("Accumulate Rolls"),
                tr("Add new rolls to the treasure instead of replacing it"),
            ),
            roller.accumulate,
            TreasureInput::Accumulate,
        )
        .choice(
            (
                tr("Coin"),
                tr("The coin of the treasure types' platinum line"),
            ),
            vec![tr("Platinum").to_owned(), tr("Electrum").to_owned()],
            Some(usize::from(roller.electrum)),
            Vec::new(),
            |i| TreasureInput::Electrum(i == 1),
        );
    let add = form.button(
        tr("_Add Item…"),
        "",
        true,
        TreasureInput::Bundle(BundleInput::Add),
    );
    if roller.bundle.is_empty() {
        form.group(
            tr("No Treasure"),
            tr(
                "Choose how many of each treasure type to roll, then press Roll, or add items \
                 yourself",
            ),
        )
        .heading_button(add);
        return;
    }
    form.group(tr("Treasure"), "").heading_button(add);
    crate::bundle::rows(
        form,
        &cr_app::bundle::rows(&roller.bundle),
        true,
        TreasureInput::Bundle,
    );
    form.group(tr("Summary"), "");
    for (title, value) in roller.summary() {
        form.value(&title, "", &value);
    }
}

/// Save Treasure, while it asks for the name.
fn naming(form: &mut Builder<TreasureInput>, roller: &Roller) -> Option<FormDialog> {
    let name = roller.naming.as_ref()?;
    let page = form.take_groups();
    form.group(
        "",
        tr("Saved treasure appears under Saved Treasure, also in the original programs"),
    )
    .text(tr("Name"), name, false, Vec::new(), TreasureInput::Name);
    let close = form.button(tr("_Cancel"), "", true, TreasureInput::SaveAnswer(false));
    let accept = form.button(
        tr("_Save"),
        "",
        !name.trim().is_empty(),
        TreasureInput::SaveAnswer(true),
    );
    let groups = form.take_groups();
    form.put_groups(page);
    Some(FormDialog {
        title: tr("Save Treasure").to_owned(),
        groups,
        close,
        accept: Some(accept),
    })
}
