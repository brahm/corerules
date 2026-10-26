//! The treasure roller (the DM Toolkit's Roll Treasure) with no toolkit
//! in it: how many of each treasure type to roll, the bundle the rolls
//! make with its Calculate summary, and the bundle's edits. Opened on an
//! encounter's treasure, its Use Treasure hands the bundle back.

use std::sync::Arc;

use cr_i18n::tr;
use cr_store::sheet::Sheet;
use cr_store::treasure::{Bundle, Coin, Counts, Rand, TreasureRules};

use crate::bundle::{BundleEditor, BundleInput};

/// The most of one treasure type a roll takes.
pub const MOST_COUNT: u32 = 999;

/// The treasure types of a lair, A to I; those of individuals are J to
/// Z.
pub const LAIR: std::ops::Range<usize> = 0..9;
pub const INDIVIDUAL: std::ops::Range<usize> = 9..26;

/// The Calculate summary of `bundle`, as its titles and values.
pub fn summary(rules: &TreasureRules, bundle: &Bundle) -> Vec<(String, String)> {
    rules
        .summary(bundle)
        .lines()
        .into_iter()
        .map(|line| match line.rsplit_once(": ") {
            Some((title, value)) => (title.to_owned(), value.to_owned()),
            None => (line, String::new()),
        })
        .collect()
}

/// A change in the roller, or one of its buttons.
#[derive(Debug, Clone, PartialEq)]
pub enum TreasureInput {
    /// How many of the treasure type with this index (A is 0).
    Count(usize, u32),
    /// Add new rolls to the treasure instead of replacing it.
    Accumulate(bool),
    /// Electrum rather than platinum on the treasure types' platinum
    /// line.
    Electrum(bool),
    Roll,
    /// Clears the treasure types and the treasure.
    Reset,
    Bundle(BundleInput),
    /// The answer to the bundle editor's dialog with this number.
    BundleAnswer(u64, bool),
    /// Save…: asks for the bundle's name.
    Save,
    /// The name typed in Save….
    Name(String),
    /// Save… answered: Save, or Cancel.
    SaveAnswer(bool),
}

/// What the roller asks of the app.
#[derive(Debug, Clone, PartialEq)]
pub enum TreasureEffect {
    /// Save `bundle` under `name`, in place of the saved bundle `old`.
    Save {
        old: Option<String>,
        name: String,
        bundle: Bundle,
    },
}

/// The roller.
#[derive(Clone)]
pub struct Roller {
    rules: Arc<TreasureRules>,
    pub counts: Counts,
    pub accumulate: bool,
    pub electrum: bool,
    pub bundle: Bundle,
    /// The saved bundle the roller is changing, by name.
    pub saved: Option<String>,
    /// Opened on an encounter's treasure, which Use Treasure sets.
    pub pick: bool,
    pub editor: BundleEditor,
    /// The name Save… asks for, while it asks.
    pub naming: Option<String>,
    /// A save under way.
    pub saving: bool,
    effects: Vec<TreasureEffect>,
}

impl Roller {
    pub fn new(rules: Arc<TreasureRules>, counts: Counts, bundle: Bundle) -> Self {
        Self {
            editor: BundleEditor::new(rules.clone()),
            rules,
            counts,
            accumulate: false,
            electrum: false,
            bundle,
            saved: None,
            pick: false,
            naming: None,
            saving: false,
            effects: Vec::new(),
        }
    }

    /// The roller on the saved bundle `name`.
    pub fn saved(rules: Arc<TreasureRules>, name: &str, bundle: Bundle) -> Self {
        Self {
            saved: Some(name.to_owned()),
            ..Self::new(rules, [0; 26], bundle)
        }
    }

    /// The roller on an encounter's treasure, with its treasure types.
    pub fn pick(rules: Arc<TreasureRules>, counts: Counts, bundle: Bundle) -> Self {
        Self {
            pick: true,
            ..Self::new(rules, counts, bundle)
        }
    }

    pub fn rules(&self) -> &Arc<TreasureRules> {
        &self.rules
    }

    /// The saved bundle's name, or Roll Treasure.
    pub fn title(&self) -> String {
        self.saved
            .clone()
            .unwrap_or_else(|| tr("Roll Treasure").to_owned())
    }

    /// The Calculate summary of the bundle, as its titles and values.
    pub fn summary(&self) -> Vec<(String, String)> {
        summary(&self.rules, &self.bundle)
    }

    /// The bundle as a printable page: the items, nested, then the
    /// summary.
    pub fn sheet(&self) -> Sheet {
        self.rules
            .sheet(self.saved.as_deref().unwrap_or("Treasure"), &self.bundle)
    }

    /// The bundle was saved under `name`.
    pub fn saved_as(&mut self, name: String) {
        self.saved = Some(name);
        self.saving = false;
    }

    pub fn take_effects(&mut self) -> Vec<TreasureEffect> {
        std::mem::take(&mut self.effects)
    }

    pub fn input(&mut self, input: TreasureInput, rand: &mut Rand) {
        match input {
            TreasureInput::Count(i, n) => {
                if let Some(count) = self.counts.get_mut(i) {
                    *count = n.min(MOST_COUNT);
                }
            }
            TreasureInput::Accumulate(on) => self.accumulate = on,
            TreasureInput::Electrum(on) => self.electrum = on,
            // Without Accumulate the treasure is replaced (0x5416ec).
            TreasureInput::Roll => {
                if !self.accumulate {
                    self.bundle = Bundle::default();
                }
                let coin = if self.electrum {
                    Coin::Electrum
                } else {
                    Coin::Platinum
                };
                self.rules.roll(rand, &self.counts, coin, &mut self.bundle);
            }
            TreasureInput::Reset => {
                self.counts = [0; 26];
                self.bundle = Bundle::default();
            }
            TreasureInput::Bundle(input) => self.editor.input(&mut self.bundle, input),
            TreasureInput::BundleAnswer(serial, yes) => {
                self.editor.answer(&mut self.bundle, serial, yes)
            }
            TreasureInput::Save => {
                self.naming = Some(self.saved.clone().unwrap_or_default());
            }
            TreasureInput::Name(name) => {
                if self.naming.is_some() {
                    self.naming = Some(name);
                }
            }
            TreasureInput::SaveAnswer(yes) => {
                let Some(name) = self.naming.take() else {
                    return;
                };
                if yes && !self.saving {
                    self.saving = true;
                    self.effects.push(TreasureEffect::Save {
                        old: self.saved.clone(),
                        name,
                        bundle: self.bundle.clone(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Option<Arc<TreasureRules>> {
        let dir = std::env::var_os("CORERULES_DIR")?;
        let install = cr_store::Installation::open(std::path::Path::new(&dir)).ok()?;
        install.treasure_rules().ok().map(Arc::new)
    }

    #[test]
    fn rolls_replace_the_treasure_unless_they_accumulate() {
        let Some(rules) = rules() else {
            return;
        };
        let mut rand = Rand::seeded(5);
        let mut roller = Roller::new(rules, [0; 26], Bundle::default());
        roller.input(TreasureInput::Count(0, 2), &mut rand);
        roller.input(TreasureInput::Roll, &mut rand);
        assert!(!roller.bundle.is_empty());
        assert!(roller.summary()[0].1.chars().any(|c| c.is_ascii_digit()));
        let first = roller.bundle.clone();
        roller.input(TreasureInput::Accumulate(true), &mut rand);
        roller.input(TreasureInput::Roll, &mut rand);
        let total = |b: &Bundle| b.items.iter().map(|i| i.quantity).sum::<u32>();
        assert!(total(&roller.bundle) > total(&first));
        roller.input(TreasureInput::Reset, &mut rand);
        assert!(roller.bundle.is_empty() && roller.counts == [0; 26]);
    }

    #[test]
    fn save_asks_for_a_name_and_keeps_the_old_one() {
        let Some(rules) = rules() else {
            return;
        };
        let mut rand = Rand::seeded(5);
        let mut roller = Roller::saved(rules, "Hoard", Bundle::default());
        assert_eq!(roller.title(), "Hoard");
        roller.input(TreasureInput::Save, &mut rand);
        assert_eq!(roller.naming.as_deref(), Some("Hoard"));
        roller.input(TreasureInput::Name("Dragon Hoard".into()), &mut rand);
        roller.input(TreasureInput::SaveAnswer(true), &mut rand);
        assert_eq!(
            roller.take_effects(),
            [TreasureEffect::Save {
                old: Some("Hoard".into()),
                name: "Dragon Hoard".into(),
                bundle: Bundle::default(),
            }]
        );
        assert!(roller.naming.is_none());
    }
}
