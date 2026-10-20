//! The character editor's nonweapon proficiencies: the Expansion's
//! Nonweapon Proficiencies page of Update Character. Taking a proficiency
//! again adds a rank, removing takes one off, and both ask and warn as
//! the original does (see [`cr_store::proficiencies`]).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::character::Character;
use cr_i18n::{tr, trn};
use cr_store::proficiencies::{CharacterNonweapon, ProficiencyRules};

use crate::create::{ask, ask_each, ignorable, message, points, slots};
use crate::lists::{RowList, RowSpec};

#[derive(Clone)]
pub struct NonweaponEditor {
    pub group: adw::PreferencesGroup,
    state: State,
}

/// The page's state, cheap to clone into callbacks.
#[derive(Clone)]
struct State {
    rules: Rc<ProficiencyRules>,
    /// The character at the levels the editor has now.
    character: Rc<RefCell<Character>>,
    parent: gtk::Widget,
    new: Rc<RefCell<CharacterNonweapon>>,
    /// Each row's id, in the order of the entries.
    ids: Rc<RefCell<Vec<u64>>>,
    next_id: Rc<Cell<u64>>,
    list: Rc<RefCell<Option<RowList<u64>>>>,
}

impl State {
    fn character(&self) -> Character {
        self.character.borrow().clone()
    }

    fn list(&self) -> Option<RowList<u64>> {
        self.list.borrow().clone()
    }

    /// What an entry took, with its ranks.
    fn subtitle(&self, index: usize) -> String {
        let character = self.character();
        let new = self.new.borrow();
        let Some(entry) = new.entries.get(index) else {
            return String::new();
        };
        let cost = if character.skills_and_powers() {
            points(entry.spent.into())
        } else {
            slots(entry.spent)
        };
        let ranks = self.rules.ranks(&character, &new, index);
        if ranks > 1 {
            trn!("{cost}, {n} rank", "{cost}, {n} ranks", ranks, cost)
        } else {
            cost
        }
    }

    /// Shows what each entry took and what is left.
    fn refresh(&self) {
        let Some(list) = self.list() else { return };
        let ids = self.ids.borrow().clone();
        list.set_subtitles(|id, _| {
            ids.iter()
                .position(|i| i == id)
                .map(|i| self.subtitle(i))
                .unwrap_or_default()
        });
        let character = self.character();
        let Some(left) = self.rules.left(&character, &self.new.borrow()) else {
            return;
        };
        let text = if character.skills_and_powers() {
            tr!("{points} left", points = points(left))
        } else if left < 0 {
            trn!(
                "The character has {n} slot too many non-weapon proficiencies.",
                "The character has {n} slots too many non-weapon proficiencies.",
                left.unsigned_abs()
            )
        } else {
            trn!("{n} slot left", "{n} slots left", left)
        };
        list.group.set_description(Some(&text));
    }

    /// Takes a proficiency once the kit's warning is ignored and the
    /// question answered Yes, or says why not.
    fn add(&self, name: String) {
        let character = self.character();
        let warning = self
            .rules
            .barred_warning(&character, &self.new.borrow(), &name);
        if let Some(warning) = warning {
            let this = self.clone();
            ignorable(
                &self.parent,
                tr("Kit Non-Weapon Proficiency Requirements"),
                &warning,
                move || {
                    this.new.borrow_mut().barred_ignored = true;
                    this.add(name.clone());
                },
            );
            return;
        }
        let purchase = match self.rules.purchase(&character, &self.new.borrow(), &name) {
            Ok(purchase) => purchase,
            Err(text) => {
                message(&self.parent, tr("Warning"), &text);
                return;
            }
        };
        let this = self.clone();
        let questions = purchase.question.clone().into_iter().collect();
        ask_each(
            &self.parent,
            questions,
            Rc::new(move || {
                this.rules
                    .take(&mut this.new.borrow_mut(), &name, &purchase);
                if purchase.rank_of.is_none() {
                    let id = this.next_id.get();
                    this.next_id.set(id + 1);
                    this.ids.borrow_mut().push(id);
                    if let Some(list) = this.list() {
                        list.add(row(id, name.clone()));
                    }
                }
                this.refresh();
            }),
        );
    }

    /// Takes a rank off the entry with row `id` once the question, if
    /// any, is answered Yes; `remove` removes the row with its last rank.
    fn remove(&self, id: u64, remove: Box<dyn FnOnce()>) {
        let Some(index) = self.ids.borrow().iter().position(|i| *i == id) else {
            return;
        };
        let character = self.character();
        let question = self
            .rules
            .removal_question(&character, &self.new.borrow(), index);
        let remove = Cell::new(Some(remove));
        let this = self.clone();
        let take_off = move || {
            let character = this.character();
            let before = this.new.borrow().entries.len();
            this.rules
                .remove(&character, &mut this.new.borrow_mut(), index);
            if this.new.borrow().entries.len() < before {
                this.ids.borrow_mut().remove(index);
                if let Some(remove) = remove.take() {
                    remove();
                }
            }
            this.refresh();
        };
        match question {
            Some(q) => ask(&self.parent, &q.title, &q.text, take_off, || {}),
            None => take_off(),
        }
    }
}

/// A row for an entry.
fn row(id: u64, name: String) -> RowSpec<u64> {
    RowSpec {
        value: id,
        title: name,
        subtitle: String::new(),
        number: None,
    }
}

impl NonweaponEditor {
    /// `character` is the character at the levels the editor has now.
    pub fn new(rules: Rc<ProficiencyRules>, character: Rc<RefCell<Character>>) -> Self {
        let new = rules.nonweapon_of(&character.borrow());
        let count = new.entries.len() as u64;
        // Add asks first, which needs the state.
        let cell: Rc<RefCell<Option<State>>> = Rc::default();
        let rows = new
            .entries
            .iter()
            .zip(0..)
            .map(|(e, id)| row(id, e.name.clone()))
            .collect();
        let names = rules.names();
        let list = RowList::new(
            tr("Nonweapon Proficiencies"),
            rows,
            move || names.clone(),
            {
                let cell = cell.clone();
                move |name| {
                    let state = cell.borrow().clone();
                    if let Some(state) = state {
                        state.add(name);
                    }
                    None
                }
            },
        );
        let state = State {
            rules,
            character,
            parent: list.group.clone().upcast(),
            new: Rc::new(RefCell::new(new)),
            ids: Rc::new(RefCell::new((0..count).collect())),
            next_id: Rc::new(Cell::new(count)),
            list: Rc::new(RefCell::new(Some(list.clone()))),
        };
        *cell.borrow_mut() = Some(state.clone());
        list.connect_remove({
            let state = state.clone();
            move |id, remove| state.remove(*id, remove)
        });
        state.refresh();
        Self {
            group: list.group.clone(),
            state,
        }
    }

    /// The choices as edited.
    pub fn nonweapon(&self) -> CharacterNonweapon {
        self.state.new.borrow().clone()
    }

    /// Shows again what depends on the class levels, once they changed.
    pub fn levels_changed(&self) {
        self.state.refresh();
    }

    /// Asks about the proficiency the kit has the character learn by a
    /// level, as leaving the original's page does, then runs `then`
    /// unless the answer is to stay.
    pub fn check_leaving(&self, stay: Rc<dyn Fn()>, then: Rc<dyn Fn()>) {
        let state = &self.state;
        let question = state
            .rules
            .leaving_question(&state.character(), &state.new.borrow());
        let Some(question) = question else {
            then();
            return;
        };
        let this = self.clone();
        ask(
            &state.parent,
            &question.title,
            &question.text,
            move || stay(),
            move || {
                this.state.new.borrow_mut().by_level_declined = true;
                then();
            },
        );
    }
}
