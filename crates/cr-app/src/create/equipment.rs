//! The Equipment page, the last: the starting money, the standard pack
//! and what the money buys, then Create.

use std::collections::VecDeque;
use std::sync::Arc;

use cr_i18n::{tr, trn};
use cr_store::create::{Draft, Question, SPELLBOOK, STANDARD_PACK};

use super::{Effect, Input, List, ListKind, Page, Prompt, Wizard, money};

/// The most of an item bought at once.
pub const MOST_ITEMS: u32 = 999;

#[derive(Debug, Clone, PartialEq)]
pub struct EquipmentPage {
    /// The gold rolled and what is left of it, or how much is short.
    pub money: String,
    pub pack: bool,
    /// What the standard pack holds and costs.
    pub pack_text: String,
    /// What comes free, with why: a wizard's spellbook, the kit's items.
    pub free: Vec<(String, String)>,
    pub items: List,
    /// The money covers what is bought.
    pub ok: bool,
    gold: u32,
    purchases: Vec<(String, u32)>,
}

impl Wizard {
    pub(super) fn equipment_page(&mut self) {
        let rules = self.rules.clone();
        // Like the original, start with one of each weapon the character
        // is proficient with.
        if self.draft.purchases.is_empty() {
            self.draft.purchases = self
                .draft
                .weapon_proficiencies
                .iter()
                .filter(|w| rules.ware(w).is_some())
                .map(|w| (w.clone(), 1))
                .collect();
        }
        let d = &self.draft;
        let pack_price = rules.spent(&Draft {
            pack: true,
            purchases: Vec::new(),
            ..d.clone()
        });
        let mut free = Vec::new();
        if rules.is_wizard(&d.classes) {
            free.push((SPELLBOOK.to_owned(), tr("Free for wizards").to_owned()));
        }
        for item in rules.kit_free_items(&d.kit) {
            free.push((item, tr("Free from the kit").to_owned()));
        }
        let page = EquipmentPage {
            money: String::new(),
            pack: d.pack,
            pack_text: tr!(
                "{pack}: dry rations for a week, flint and steel, three torches and a wineskin · {price}",
                pack = STANDARD_PACK.0,
                price = money(pack_price)
            ),
            free,
            items: List::default(),
            ok: false,
            gold: d.gold,
            purchases: d.purchases.clone(),
        };
        // The kit's text and what it requires, as the page opens (494875,
        // 5ea499), New Character only; then that the kit changes what
        // items cost (5e2ba2).
        let entering = self.entering();
        let mut notices: VecDeque<Question> = rules
            .kit_equipment_text(&d.kit)
            .filter(|_| !entering)
            .map(|text| Question::notice(&d.kit, text.to_owned()))
            .into_iter()
            .collect();
        if let Some(notice) = rules.kit_price_notice(&d.kit) {
            notices.push_back(Question::notice(&d.kit, notice.to_owned()));
        }
        if !entering {
            notices.extend(rules.kit_equipment_notices(d));
        }
        self.push(Page::Equipment(Box::new(page)));
        self.refresh_equipment();
        if !notices.is_empty() {
            self.ask_each(notices, Arc::new(|_: &mut Wizard| {}));
        }
    }

    /// The draft with the page's money, pack and purchases.
    fn equipment_probe(&self, page: &EquipmentPage) -> Draft {
        Draft {
            gold: page.gold,
            pack: page.pack,
            purchases: page.purchases.clone(),
            ..self.draft.clone()
        }
    }

    fn refresh_equipment(&mut self) {
        let Page::Equipment(page) = self.page() else {
            return;
        };
        let rules = &self.rules;
        let probe = self.equipment_probe(page);
        let total = page.gold * 100;
        let spent = rules.spent(&probe);
        let gold = page.gold;
        let text = if spent <= total {
            trn!(
                "{n} gold piece · {money} left",
                "{n} gold pieces · {money} left",
                gold,
                money = money(total - spent)
            )
        } else {
            trn!(
                "{n} gold piece · {money} short",
                "{n} gold pieces · {money} short",
                gold,
                money = money(spent - total)
            )
        };
        let kit = &probe.kit;
        let items = List {
            title: tr("Equipment").to_owned(),
            description: tr(
                "Weapons go to the items readied, armor to the items worn, the rest to the \
                 items carried",
            )
            .to_owned(),
            rows: page
                .purchases
                .iter()
                .map(|(name, n)| {
                    let subtitle = rules.ware(name).map_or(String::new(), |w| {
                        format!("{} · {}", w.part_type, money(rules.price(kit, name, 1)))
                    });
                    (name.clone(), subtitle, Some(*n))
                })
                .collect(),
            choices: rules
                .wares
                .iter()
                .map(|w| w.name.clone())
                .filter(|n| !page.purchases.iter().any(|(p, _)| p == n))
                .collect(),
            shown: true,
            fixed: false,
        };
        if let Page::Equipment(page) = self.page_mut() {
            page.money = text;
            page.items = items;
            page.ok = spent <= total;
        }
    }

    /// Whether the money covers `purchases`; Enter Character's are free.
    fn affordable(&self, page: &EquipmentPage, purchases: Vec<(String, u32)>) -> bool {
        self.entering()
            || self.rules.spent(&Draft {
                purchases,
                ..self.equipment_probe(page)
            }) <= page.gold * 100
    }

    /// An item the money does not cover is refused, as Edit Character
    /// does.
    fn too_expensive(&mut self) {
        self.message(tr("Error Message!"), tr("This item is too expensive."));
    }

    pub(super) fn equipment_input(&mut self, input: Input) {
        let Page::Equipment(page) = self.page() else {
            return;
        };
        let page = page.as_ref().clone();
        match input {
            Input::RollMoney => {
                let gold = self.rules.roll_gold(&self.draft, &mut self.dice);
                if let Page::Equipment(page) = self.page_mut() {
                    page.gold = gold;
                }
                self.refresh_equipment();
            }
            Input::Pack(on) => {
                if let Page::Equipment(page) = self.page_mut() {
                    page.pack = on;
                }
                self.refresh_equipment();
            }
            Input::Add(ListKind::Items, name) if page.items.choices.contains(&name) => {
                // Buying an item the character may not use asks first
                // (5d9331).
                let problem = self.needs.items.as_ref().and_then(|items| {
                    let user = self.rules.item_user(&self.draft);
                    items.item_problem(&user, &name, true)
                });
                let add = Arc::new(move |w: &mut Wizard| {
                    let Page::Equipment(page) = w.page() else {
                        return;
                    };
                    let mut purchases = page.purchases.clone();
                    purchases.push((name.clone(), 1));
                    if !w.affordable(page, purchases.clone()) {
                        w.too_expensive();
                        return;
                    }
                    if let Page::Equipment(page) = w.page_mut() {
                        page.purchases = purchases;
                    }
                    w.refresh_equipment();
                });
                match problem {
                    Some(text) => self.show(Prompt::item_error(&text, add)),
                    None => add(self),
                }
            }
            Input::Remove(ListKind::Items, row) if row < page.purchases.len() => {
                if let Page::Equipment(page) = self.page_mut() {
                    page.purchases.remove(row);
                }
                self.refresh_equipment();
            }
            Input::Times(row, n) if row < page.purchases.len() => {
                let n = n.clamp(1, MOST_ITEMS);
                let mut purchases = page.purchases.clone();
                let more = n > purchases[row].1;
                purchases[row].1 = n;
                if more && !self.affordable(&page, purchases.clone()) {
                    self.too_expensive();
                    self.refresh_equipment();
                    return;
                }
                if let Page::Equipment(page) = self.page_mut() {
                    page.purchases = purchases;
                }
                self.refresh_equipment();
            }
            Input::Next if page.ok => {
                self.draft.gold = page.gold;
                self.draft.pack = page.pack;
                self.draft.purchases = page.purchases.clone();
                // The money kept, against what the kit allows (5e33cb).
                let kept = (page.gold * 100).saturating_sub(self.rules.spent(&self.draft));
                match self.rules.kit_retained_funds_warning(&self.draft, kept) {
                    Some(warning) => self.show(Prompt::ignorable(
                        tr("Kit Restriction"),
                        &warning,
                        Arc::new(|w: &mut Wizard| w.create()),
                    )),
                    None => self.create(),
                }
            }
            _ => {}
        }
    }

    /// Asks the app to write the character; [`Wizard::failed`] says why
    /// it could not.
    fn create(&mut self) {
        self.error = None;
        self.effects
            .push(Effect::Create(Box::new(self.draft.clone())));
    }
}
