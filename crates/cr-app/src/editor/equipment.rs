//! The editor's Equipment page: the Expansion's Equipment page of Update
//! Character. The spending money's coins and the items of each other
//! bundle, with how many of each; with Buy and Sell on, what is added is
//! paid for from the spending money and what is removed is sold, at the
//! kit's prices, and only what can be bought is offered (5d9331, 5dde7f,
//! 5ddd19).

use std::sync::Arc;

use cr_i18n::{tr, tr_noop};
use cr_store::lists::{BUNDLE_NAMES, COINS, Item, MONEY};
use cr_store::npc::ItemUser;
use cr_store::purse;

use super::{EditInput, Editor, Prompt, Then};
use crate::create::{List, ListKind, money};

/// The most of an item a bundle holds.
pub const MOST_QUANTITY: u32 = 99_999;

/// The most coins of a kind.
pub const MOST_COINS: u32 = 9_999_999;

/// The spending money's coins, as [`CharacterLists::coins`] has them.
///
/// [`CharacterLists::coins`]: cr_store::lists::CharacterLists::coins
pub const COIN_NAMES: [&str; 5] = [
    tr_noop!("Copper"),
    tr_noop!("Silver"),
    tr_noop!("Electrum"),
    tr_noop!("Gold"),
    tr_noop!("Platinum"),
];

impl Editor {
    /// The spending money's coins, copper to platinum.
    pub fn coins(&self) -> [u32; 5] {
        self.lists.coins
    }

    /// The items of each bundle.
    pub fn equipment(&self) -> &[Vec<Item>; 9] {
        &self.lists.equipment
    }

    /// The bundles but the spending money, each by its index with its
    /// items and what each item Add offers costs while Buy and Sell is on.
    pub fn bundles(&self) -> Vec<(usize, List, Vec<String>)> {
        let offered = if self.trading {
            &self.needs.wares
        } else {
            &self.needs.items
        };
        let prices: Vec<String> = if self.trading {
            offered
                .iter()
                .map(|name| money(self.price(name, 1)))
                .collect()
        } else {
            Vec::new()
        };
        (0..BUNDLE_NAMES.len())
            .filter(|&b| b != MONEY)
            .map(|b| {
                let list = List {
                    title: tr(BUNDLE_NAMES[b]).into(),
                    description: String::new(),
                    rows: self.lists.equipment[b]
                        .iter()
                        .map(|item| {
                            let holds = if item.contents.is_empty() {
                                String::new()
                            } else {
                                tr!("Holds {items}", items = item.contents.join(", "))
                            };
                            (item.name.clone(), holds, Some(item.quantity))
                        })
                        .collect(),
                    choices: offered.clone(),
                    shown: true,
                    fixed: false,
                };
                (b, list, prices.clone())
            })
            .collect()
    }

    /// What `quantity` of an item costs, at the kit's percentage.
    fn price(&self, name: &str, quantity: u32) -> u32 {
        self.needs.creation.price(&self.details.kit, name, quantity)
    }

    /// Why `cost` copper pieces cannot be paid, if they cannot (5d9705):
    /// the spending money is short, but maybe not the money kept in the
    /// bundles.
    fn short(&self, cost: u64) -> Option<&'static str> {
        let spending = purse::worth(&self.lists.coins);
        if spending >= cost {
            return None;
        }
        let kept: u64 = self
            .lists
            .equipment
            .iter()
            .flatten()
            .filter_map(|item| {
                COINS
                    .iter()
                    .find(|c| c.0 == item.name)
                    .map(|c| u64::from(item.quantity) * u64::from(c.1))
            })
            .sum();
        Some(if spending + kept >= cost {
            tr(
                "You must move some of your money into Spending Money before you can purchase \
                 this item.",
            )
        } else {
            tr("This item is too expensive.")
        })
    }

    /// Pays `cost` from the spending money, or says why it cannot.
    fn pay(&mut self, cost: u32) -> bool {
        if let Some(text) = self.short(cost.into()) {
            self.message(tr("Error Message!"), text);
            return false;
        }
        purse::pay(&mut self.lists.coins, cost)
    }

    /// Who uses the items, for the program's check: the character at the
    /// levels the form has now with its kit, class abilities and weapons
    /// as edited.
    fn item_user(&self) -> ItemUser {
        let d = &self.advanced.data;
        ItemUser {
            race: d.race.clone(),
            classes: d
                .classes
                .names
                .iter()
                .cloned()
                .zip(self.levels().into_iter().map(|l| l as i32))
                .collect(),
            skills_and_powers: self.advanced.skills_and_powers(),
            kit: self.details.kit.clone(),
            class_abilities: self
                .abilities
                .classes
                .iter()
                .map(|c| c.abilities.clone())
                .collect(),
            weapon_proficiencies: self.combat.weapons.clone(),
            focus_weapon: self.combat.focus.clone().unwrap_or_default(),
        }
    }

    /// Adds an item to bundle `b`, bought while Buy and Sell is on; one
    /// the character may not use asks first, as the original's Update
    /// Character does (0x43d7bc in its mode).
    fn add_item(&mut self, b: usize, name: String) {
        let problem = self
            .needs
            .item_rules
            .as_ref()
            .and_then(|rules| rules.item_problem(&self.item_user(), &name, false));
        let add: Then = Arc::new(move |e: &mut Editor| {
            if e.trading && !e.pay(e.price(&name, 1)) {
                return;
            }
            e.lists.equipment[b].push(Item::new(&name));
        });
        match problem {
            Some(text) => self.show(Prompt::item_error(&text, add)),
            None => add(self),
        }
    }

    /// Takes row `row` out of bundle `b`, sold while Buy and Sell is on.
    fn remove_item(&mut self, b: usize, row: usize) {
        if row >= self.lists.equipment[b].len() {
            return;
        }
        let item = self.lists.equipment[b].remove(row);
        if self.trading {
            let price = self.price(&item.name, item.quantity);
            purse::refund(&mut self.lists.coins, price);
        }
    }

    /// Sets how many of row `row` bundle `b` has, paying for more or
    /// selling the rest while Buy and Sell is on.
    fn set_quantity(&mut self, b: usize, row: usize, quantity: u32) {
        let Some(item) = self.lists.equipment[b].get(row) else {
            return;
        };
        let quantity = quantity.clamp(1, MOST_QUANTITY);
        if self.trading {
            let was = self.price(&item.name, item.quantity);
            let is = self.price(&item.name, quantity);
            if is > was {
                if !self.pay(is - was) {
                    return;
                }
            } else {
                purse::refund(&mut self.lists.coins, was - is);
            }
        }
        self.lists.equipment[b][row].quantity = quantity;
    }

    /// Opens the page: the kit's text about equipment comes first, once
    /// (494875).
    pub(super) fn open_equipment(&mut self) {
        if std::mem::replace(&mut self.equipped, true) {
            return;
        }
        let kit = self.details.kit.clone();
        if let Some(text) = self.needs.creation.kit_equipment_text(&kit) {
            let text = text.to_owned();
            self.message(&kit, text);
        }
    }

    pub(super) fn equipment_input(&mut self, input: EditInput) {
        match input {
            EditInput::Coin(coin, n) if coin < COIN_NAMES.len() => {
                self.lists.coins[coin] = n.min(MOST_COINS);
            }
            EditInput::Trading(on) => {
                self.trading = on;
                // That the kit changes what items cost, as the mode is
                // chosen (5e2ba2).
                let kit = self.details.kit.clone();
                if on && let Some(text) = self.needs.creation.kit_price_notice(&kit) {
                    self.message(&kit, text);
                }
            }
            EditInput::Add(ListKind::Bundle(b), name) if b < BUNDLE_NAMES.len() && b != MONEY => {
                let offered = if self.trading {
                    &self.needs.wares
                } else {
                    &self.needs.items
                };
                if offered.contains(&name) {
                    self.add_item(b, name);
                }
            }
            EditInput::Remove(ListKind::Bundle(b), row) if b < BUNDLE_NAMES.len() => {
                self.remove_item(b, row);
            }
            EditInput::Times(ListKind::Bundle(b), row, n) if b < BUNDLE_NAMES.len() => {
                self.set_quantity(b, row, n);
            }
            _ => {}
        }
    }

    /// Whether an input is the Equipment page's.
    pub(super) fn is_equipment_input(input: &EditInput) -> bool {
        matches!(
            input,
            EditInput::Add(ListKind::Bundle(_), _)
                | EditInput::Remove(ListKind::Bundle(_), _)
                | EditInput::Times(ListKind::Bundle(_), ..)
                | EditInput::Coin(..)
                | EditInput::Trading(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::editor;
    use super::*;
    use crate::prompt::Answer;

    #[test]
    fn buying_pays_and_selling_gives_the_price_back() {
        let Some(mut e) = editor() else {
            return;
        };
        e.input(EditInput::Coin(3, 1000));
        e.input(EditInput::Trading(true));
        while e.prompt().is_some() {
            e.answer(Answer::Yes);
        }
        let (b, list, prices) = e.bundles().into_iter().next().unwrap();
        assert_eq!(list.choices.len(), prices.len());
        let Some(name) = list.choices.iter().find(|n| e.price(n, 1) > 0).cloned() else {
            return;
        };
        let before = purse::worth(&e.coins());
        let count = e.lists.equipment[b].len();
        e.input(EditInput::Add(ListKind::Bundle(b), name.clone()));
        while e.prompt().is_some() {
            e.answer(Answer::Yes);
        }
        assert_eq!(e.lists.equipment[b].len(), count + 1);
        let price = u64::from(e.price(&name, 1));
        assert_eq!(purse::worth(&e.coins()), before - price);
        e.input(EditInput::Times(ListKind::Bundle(b), count, 3));
        assert_eq!(e.lists.equipment[b][count].quantity, 3);
        assert_eq!(
            purse::worth(&e.coins()),
            before - u64::from(e.price(&name, 3))
        );
        e.input(EditInput::Remove(ListKind::Bundle(b), count));
        assert_eq!(e.lists.equipment[b].len(), count);
        assert_eq!(purse::worth(&e.coins()), before);
    }

    #[test]
    fn without_trading_items_come_free() {
        let Some(mut e) = editor() else {
            return;
        };
        let coins = e.coins();
        let (b, list, prices) = e.bundles().into_iter().next().unwrap();
        assert!(prices.is_empty());
        let Some(name) = list.choices.first().cloned() else {
            return;
        };
        e.input(EditInput::Add(ListKind::Bundle(b), name));
        while e.prompt().is_some() {
            e.answer(Answer::Yes);
        }
        assert_eq!(e.coins(), coins);
        assert!(e.changed());
    }
}
