//! The Spells page: the wizard spells known at 1st level, each learned
//! as the original's Learn checks it, and the schools a kit has the
//! wizard eliminate.

use std::sync::Arc;

use cr_i18n::tr;
use cr_store::create::{Draft, max_spells_per_level};
use cr_store::kit_magic::KitMagicRules;
use cr_store::learn::{ALREADY_KNOWN, Learner, Learning, WARNING};

use super::{Input, List, ListKind, Page, Prompt, Wizard};

#[derive(Debug, Clone, PartialEq)]
pub struct SpellsPage {
    pub spells: List,
    /// The chance to learn each spell Add offers, by its index.
    pub chances: Vec<String>,
    /// The schools a kit limited to some has the wizard eliminate.
    pub eliminated: Option<Schools>,
    /// The spells as the page has them, which Next keeps.
    names: Vec<String>,
}

/// The schools to tick, at most `most`.
#[derive(Debug, Clone, PartialEq)]
pub struct Schools {
    pub title: String,
    pub description: String,
    pub names: Vec<String>,
    pub ticked: Vec<bool>,
    pub full: bool,
}

impl Wizard {
    /// The spells for a wizard, then what comes before the equipment.
    pub(super) fn after_rogue_skills(&mut self) {
        if !self.rules.is_wizard(&self.draft.classes) {
            self.before_equipment();
            return;
        }
        let names = self.draft.spells.clone();
        self.push(Page::Spells(Box::new(SpellsPage {
            spells: List::default(),
            chances: Vec::new(),
            eliminated: None,
            names,
        })));
        // The kit's barred schools, warned about as the page opens; the
        // spells of schools no longer kept go.
        if let Some(warning) = self.rules.kit_barred_schools_warning(&self.draft) {
            self.show(Prompt::ignorable(
                tr("Kit Class Ability Requirements"),
                &warning,
                Arc::new(|w: &mut Wizard| w.draft.barred_schools_ignored = true),
            ));
        }
        self.drop_spells();
    }

    /// The draft's wizard class learning spells at 1st level, knowing
    /// `known`.
    fn learner(&self, known: &[String]) -> Learner {
        let d = &self.draft;
        let rules = &self.rules;
        let slot = d
            .classes
            .iter()
            .position(|c| rules.is_wizard(std::slice::from_ref(c)))
            .unwrap_or(0);
        let class = d.classes.get(slot).cloned().unwrap_or_default();
        let abilities = d.class_abilities.get(slot).cloned().unwrap_or_default();
        let scores = rules.subability_scores(d);
        Learner {
            castable: self.needs.advance.castable(&class, 1, &abilities),
            class,
            abilities,
            kit: d.kit.clone(),
            reason: scores[3][1],
            knowledge: scores[3][2],
            wizard_spells: known.to_vec(),
            priest_spells: Vec::new(),
            entering: self.entering(),
        }
    }

    /// Keeps only the spells of the schools still allowed.
    fn drop_spells(&mut self) {
        let allowed = self.rules.draft_spells(&self.draft);
        if let Page::Spells(page) = self.page_mut() {
            page.names.retain(|n| allowed.contains(n));
        }
        self.refresh_spells();
    }

    fn refresh_spells(&mut self) {
        let Page::Spells(page) = self.page() else {
            return;
        };
        let names = page.names.clone();
        let d = &self.draft;
        let rules = &self.rules;
        let intelligence = rules.draft_scores(d)[3];
        let most = max_spells_per_level(intelligence);
        let description = if self.entering() {
            tr(
                "Spells the character knows, of any level, as the original's Enter \
                Character takes them",
            )
            .to_owned()
        } else if most == usize::MAX {
            tr("1st-level spells the character knows. Intelligence allows any number.").to_owned()
        } else {
            tr!(
                "1st-level spells the character knows. Intelligence {intelligence} allows {most}.",
                intelligence,
                most
            )
        };
        let choices: Vec<String> = rules
            .draft_spells(&Draft {
                classes: d.classes.clone(),
                ..d.clone()
            })
            .into_iter()
            .filter(|n| !names.contains(n))
            .collect();
        // The chance to learn each spell offered (0x630052).
        let learner = self.learner(&names);
        let chances = choices
            .iter()
            .map(|n| self.needs.learn.chance_text(&learner, n, true))
            .collect();
        let spells = List {
            title: tr("Spells Known").to_owned(),
            description,
            rows: names
                .iter()
                .map(|n| (n.clone(), String::new(), None))
                .collect(),
            choices,
            shown: true,
            fixed: false,
        };
        let eliminated = rules.kit_schools_to_eliminate(d).map(|(count, schools)| {
            let ticked: Vec<bool> = schools
                .iter()
                .map(|s| d.eliminated_schools.contains(s))
                .collect();
            Schools {
                title: tr("Eliminated Schools").to_owned(),
                description: KitMagicRules::elimination_text(&d.kit, count),
                full: ticked.iter().filter(|t| **t).count() >= count,
                names: schools,
                ticked,
            }
        });
        if let Page::Spells(page) = self.page_mut() {
            page.spells = spells;
            page.chances = chances;
            page.eliminated = eliminated;
        }
    }

    pub(super) fn spells_input(&mut self, input: Input) {
        let Page::Spells(page) = self.page() else {
            return;
        };
        let page = page.as_ref().clone();
        match input {
            Input::Add(ListKind::Spells, name) if page.spells.choices.contains(&name) => {
                // Each spell picked is checked as the original's Learn
                // does, for the wizard class at 1st level.
                let learner = self.learner(&page.names);
                let learning =
                    self.needs
                        .learn
                        .clone()
                        .learn_rolled(&learner, &name, true, &mut self.dice);
                match learning {
                    Learning::Known => self.message(tr(WARNING), tr(ALREADY_KNOWN)),
                    Learning::Ask(questions) => self.ask_each(
                        questions.into(),
                        Arc::new(move |w: &mut Wizard| {
                            if let Page::Spells(page) = w.page_mut()
                                && !page.names.contains(&name)
                            {
                                page.names.push(name.clone());
                            }
                            w.refresh_spells();
                        }),
                    ),
                }
            }
            Input::Remove(ListKind::Spells, i) if i < page.names.len() => {
                if let Page::Spells(page) = self.page_mut() {
                    page.names.remove(i);
                }
                self.refresh_spells();
            }
            Input::School(i, on) => {
                let Some((count, schools)) = self.rules.kit_schools_to_eliminate(&self.draft)
                else {
                    return;
                };
                let Some(school) = schools.get(i) else {
                    return;
                };
                let eliminated = &self.draft.eliminated_schools;
                if on && !eliminated.contains(school) && eliminated.len() >= count {
                    return;
                }
                self.draft.eliminated_schools = schools
                    .iter()
                    .filter(|s| {
                        if *s == school {
                            on
                        } else {
                            eliminated.contains(s)
                        }
                    })
                    .cloned()
                    .collect();
                self.drop_spells();
            }
            Input::Next => {
                self.draft.spells = page.names;
                self.before_equipment();
            }
            _ => {}
        }
    }
}
