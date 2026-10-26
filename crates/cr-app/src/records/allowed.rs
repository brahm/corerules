//! Allowed Classes… on a race, with no toolkit in it (the Expansion's
//! IDD 263): the classes and multi-classes the race may take, the shipped
//! ones kept as they are and the user's with a level limit, added from
//! the classes and the combinations of their groups.

use cr_i18n::tr;
use cr_store::Installation;
use cr_store::help::topic;
use cr_store::userdb::allowed::{self, Allowed, AllowedDb};
use cr_store::userdb::classes::{self, ClassEntry};
use cr_store::userdb::races::Races;
use cr_store::userdb::{Filter, UserDb};

use crate::prompt::{self, Answer};

/// A question or message the dialog waits on.
pub type Prompt = prompt::Prompt<AllowedForm>;

/// The highest level limit.
pub const MOST_LIMIT: i32 = 100;

/// A change in Allowed Classes, or one of its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum AllowedInput {
    /// The level limit of the entry with this index (0: none).
    Limit(usize, i32),
    /// Remove the user's entry with this index.
    Remove(usize),
    /// What Add Class's box (false) or Add Multi-Class's (true) chose.
    Adding(bool, String),
    /// Add the class or multi-class chosen.
    Add(bool),
    Save,
}

/// What the app should do for Allowed Classes.
#[derive(Clone)]
pub enum AllowedEffect {
    /// Save the user's `entries` for `race`, and pass what it gives to
    /// [`AllowedForm::saved`].
    Save { race: String, entries: Vec<Allowed> },
    /// Saved: leave, saying `said`.
    Done { said: String },
}

/// Allowed Classes for one race.
#[derive(Clone)]
pub struct AllowedForm {
    pub race: String,
    /// Its entries, in file order.
    entries: Vec<Allowed>,
    kept: Vec<Allowed>,
    classes: Vec<ClassEntry>,
    /// The race may dual-class, so it may not multi-class.
    dual: bool,
    adding: [Option<String>; 2],
    prompt: Option<Prompt>,
    serial: u64,
    effects: Vec<AllowedEffect>,
    pub error: Option<String>,
    pub saving: bool,
}

/// An entry as the dialog lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Its index, which the inputs name.
    pub index: usize,
    pub name: String,
    /// Shipped, kept as it is.
    pub standard: bool,
    pub limit: i32,
}

impl AllowedForm {
    /// Opens Allowed Classes for `race`, creating its record and, as the
    /// original does, writing the file at once when there was none.
    pub fn open(install: &Installation, race: &str) -> cr_store::Result<Self> {
        let mut db = AllowedDb::load(install)?;
        let (_, created) = db.ensure(race);
        if created {
            db.save(install)?;
        }
        let classes = classes::class_list(install)?;
        let dual = UserDb::<Races>::load(install)?
            .list(Filter::All)
            .iter()
            .find(|e| e.record.name == race)
            .is_some_and(|e| e.record.unknown_0x1e0[2] != 0);
        Ok(Self::new(race, db.entries(race), classes, dual))
    }

    pub fn new(race: &str, entries: Vec<Allowed>, classes: Vec<ClassEntry>, dual: bool) -> Self {
        Self {
            race: race.to_owned(),
            kept: entries.clone(),
            entries,
            classes,
            dual,
            adding: [None, None],
            prompt: None,
            serial: 0,
            effects: Vec::new(),
            error: None,
            saving: false,
        }
    }

    pub fn title(&self) -> String {
        tr!("Allowed Classes for {race}", race = self.race.clone())
    }

    pub fn help(&self) -> u32 {
        topic::CLASSES_ALLOWED
    }

    /// The entries, sorted by name as the dialog's list is.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows: Vec<Row> = self
            .entries
            .iter()
            .enumerate()
            .map(|(index, a)| Row {
                index,
                name: a.name.clone(),
                standard: a.standard,
                limit: a.level_limit,
            })
            .collect();
        rows.sort_by_cached_key(|r| cr_store::collate::sort_key(&r.name));
        rows
    }

    /// What Add Class (false) or Add Multi-Class (true) offers: the
    /// classes or combinations not yet allowed.
    pub fn choices(&self, multi: bool) -> Vec<String> {
        let names = if multi {
            classes::multiclass_names(&self.classes)
        } else {
            self.classes.iter().map(|c| c.name.clone()).collect()
        };
        names
            .into_iter()
            .filter(|n| !self.entries.iter().any(|a| a.name == *n))
            .collect()
    }

    /// What the box of Add Class (false) or Add Multi-Class (true) chose.
    pub fn adding(&self, multi: bool) -> Option<&str> {
        self.adding[usize::from(multi)].as_deref()
    }

    pub fn changed(&self) -> bool {
        self.entries != self.kept
    }

    pub fn can_save(&self) -> bool {
        !self.saving && self.prompt.is_none()
    }

    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    pub fn prompt_serial(&self) -> u64 {
        self.serial
    }

    pub fn take_effects(&mut self) -> Vec<AllowedEffect> {
        std::mem::take(&mut self.effects)
    }

    pub fn answer(&mut self, answer: Answer) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        if let Some(then) = prompt.then(answer) {
            then(self);
        }
    }

    pub fn input(&mut self, input: AllowedInput) {
        if self.prompt.is_some() || self.saving {
            return;
        }
        match input {
            AllowedInput::Limit(i, limit) => {
                if let Some(a) = self.entries.get_mut(i).filter(|a| !a.standard) {
                    a.level_limit = limit.clamp(0, MOST_LIMIT);
                }
            }
            AllowedInput::Remove(i) => {
                if self.entries.get(i).is_some_and(|a| !a.standard) {
                    self.entries.remove(i);
                }
            }
            AllowedInput::Adding(multi, name) => self.adding[usize::from(multi)] = Some(name),
            AllowedInput::Add(multi) => {
                if multi && self.dual {
                    self.prompt = Some(Prompt::message(
                        tr("Multi-Classes"),
                        tr(allowed::DUAL_AND_MULTI),
                    ));
                    self.serial += 1;
                    return;
                }
                let Some(name) = self.adding[usize::from(multi)].take() else {
                    return;
                };
                let names: Vec<&str> = name.split('/').collect();
                if !self.entries.iter().any(|a| a.name == name)
                    && let Some(entry) = Allowed::from_names(&names, &self.classes, 0)
                {
                    self.entries.push(entry);
                }
            }
            AllowedInput::Save => {
                // The dialog's list is sorted by name.
                let mut entries: Vec<Allowed> = self
                    .entries
                    .iter()
                    .filter(|a| !a.standard)
                    .cloned()
                    .collect();
                entries.sort_by_cached_key(|a| cr_store::collate::sort_key(&a.name));
                self.saving = true;
                self.error = None;
                self.effects.push(AllowedEffect::Save {
                    race: self.race.clone(),
                    entries,
                });
            }
        }
    }

    /// What saving did.
    pub fn saved(&mut self, result: Result<(), String>) {
        self.saving = false;
        match result {
            Ok(()) => self.effects.push(AllowedEffect::Done {
                said: tr("Allowed classes saved").to_owned(),
            }),
            Err(why) => self.error = Some(why),
        }
    }
}

/// Saves the user's `entries` for `race`.
pub fn save(install: &Installation, race: &str, entries: &[Allowed]) -> Result<(), String> {
    let mut db = AllowedDb::load(install).map_err(|e| e.to_string())?;
    db.set_user(race, entries);
    db.save(install).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(name: &str, group: usize) -> ClassEntry {
        ClassEntry {
            name: name.into(),
            id: 0,
            group: Some(group),
        }
    }

    #[test]
    fn classes_are_added_once_and_shipped_ones_stay() {
        let mut shipped = Allowed::new(
            ["Fighter".into(), "".into(), "".into(), "".into(), "".into()],
            0,
        );
        shipped.standard = true;
        let classes = vec![class("Fighter", 0), class("Thief", 1)];
        let mut form = AllowedForm::new("Elf", vec![shipped], classes.clone(), false);
        assert_eq!(form.choices(false), vec!["Thief"]);
        form.input(AllowedInput::Adding(false, "Thief".into()));
        form.input(AllowedInput::Add(false));
        form.input(AllowedInput::Adding(true, "Fighter/Thief".into()));
        form.input(AllowedInput::Add(true));
        assert_eq!(form.rows().len(), 3);
        // The shipped entry is neither removed nor limited.
        form.input(AllowedInput::Remove(0));
        form.input(AllowedInput::Limit(0, 9));
        assert_eq!(form.rows()[0].limit, 0);
        assert!(form.changed());

        let mut dual = AllowedForm::new("Human", Vec::new(), classes, true);
        dual.input(AllowedInput::Adding(true, "Fighter/Thief".into()));
        dual.input(AllowedInput::Add(true));
        assert!(
            dual.prompt().is_some(),
            "a race that dual-classes may not multi-class"
        );
    }
}
