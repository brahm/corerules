//! The character editor's Combat page: the Expansion's Weapon
//! Proficiencies page of Update Character, with its Specialization, Shield
//! and Armor and Fighting Styles dialogs as groups, and the traits and
//! disadvantages of its Nonweapon Proficiencies page. Taking a weapon
//! proficiency, a specialization or an unarmed style asks what the
//! original asks and gives the kit's warnings, as New Character does.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::character::Character;
use cr_i18n::{tr, trn};
use cr_store::combat::{CharacterCombat, CombatRules, Purchase};
use cr_store::create::{SHIELDS, UNARMED};
use gtk::glib;

use crate::abilities::{cost_text, names_list};
use crate::create::{ask, ask_each, ignorable, message, notice};
use crate::lists::{RowList, RowSpec};

/// The character's class levels as the editor has them now.
pub type Levels = Rc<dyn Fn() -> Vec<u32>>;

/// A combo of "None" and some names.
#[derive(Clone)]
pub(crate) struct WeaponCombo {
    pub(crate) row: adw::ComboRow,
    options: Rc<RefCell<Vec<String>>>,
}

impl WeaponCombo {
    pub(crate) fn new(
        title: &str,
        subtitle: &str,
        options: Vec<String>,
        selected: Option<&String>,
    ) -> Self {
        let row = adw::ComboRow::builder()
            .title(title)
            .subtitle(subtitle)
            .build();
        let combo = Self {
            row,
            options: Rc::default(),
        };
        combo.fill(options, selected);
        combo
    }

    fn fill(&self, options: Vec<String>, selected: Option<&String>) {
        let mut labels = vec![tr("None")];
        labels.extend(options.iter().map(String::as_str));
        let at = selected
            .and_then(|s| options.iter().position(|o| o == s))
            .map_or(0, |i| i + 1);
        self.row.set_model(Some(&gtk::StringList::new(&labels)));
        *self.options.borrow_mut() = options;
        self.row.set_selected(at as u32);
    }

    /// Offers `options`, keeping the weapon chosen when it is among them.
    pub(crate) fn refill(&self, options: Vec<String>) {
        if *self.options.borrow() != options {
            let selected = self.value();
            self.fill(options, selected.as_ref());
        }
    }

    pub(crate) fn value(&self) -> Option<String> {
        let i = self.row.selected() as usize;
        (i > 0)
            .then(|| self.options.borrow().get(i - 1).cloned())
            .flatten()
    }
}

/// The page's widgets, cheap to clone into callbacks.
#[derive(Clone)]
struct State {
    rules: Rc<CombatRules>,
    character: Rc<Character>,
    levels: Levels,
    parent: gtk::Widget,
    original: Rc<CharacterCombat>,
    /// What the widgets do not hold: the kit's warnings ignored, the
    /// every-other flag and what was charged.
    kit: Rc<RefCell<CharacterCombat>>,
    /// The weapon the kit has the character focus in.
    focus: WeaponCombo,
    /// The weapons the kit has the character pick, while still to pick.
    picks: [WeaponCombo; 2],
    kit_group: adw::PreferencesGroup,
    /// Turning an unarmed switch on asks first; this marks the page's own
    /// changes.
    quiet: Rc<Cell<bool>>,
    weapons: RowList<()>,
    /// Skills & Powers: the broad and tight weapon groups.
    broad: Option<RowList<()>>,
    tight: Option<RowList<()>>,
    weapon_of_choice: Option<WeaponCombo>,
    expertise: Option<WeaponCombo>,
    mastery: Option<WeaponCombo>,
    specializations: RowList<()>,
    unarmed: Vec<adw::SwitchRow>,
    shields: Vec<adw::SwitchRow>,
    armor: Option<RowList<()>>,
    styles: Option<RowList<()>>,
    traits: Option<RowList<()>>,
    disadvantages: Option<RowList<()>>,
}

impl State {
    fn combat(&self) -> CharacterCombat {
        let titles = |list: &Option<RowList<()>>, old: &[String]| {
            list.as_ref()
                .map_or_else(|| old.to_vec(), RowList::rows_titles)
        };
        let weapon = |combo: &Option<WeaponCombo>, old: &Option<String>| {
            combo
                .as_ref()
                .map_or_else(|| old.clone(), WeaponCombo::value)
        };
        let o = &self.original;
        let kit = self.kit.borrow();
        let focus = if self.focus.row.get_visible() {
            self.focus.value()
        } else {
            o.focus.clone()
        };
        CharacterCombat {
            weapons: self.weapons.rows_titles(),
            broad_groups: titles(&self.broad, &o.broad_groups),
            tight_groups: titles(&self.tight, &o.tight_groups),
            weapon_of_choice: weapon(&self.weapon_of_choice, &o.weapon_of_choice),
            expertise: weapon(&self.expertise, &o.expertise),
            mastery: weapon(&self.mastery, &o.mastery),
            specializations: self.specializations.rows_titles(),
            unarmed: std::array::from_fn(|i| self.unarmed[i].is_active()),
            shields: std::array::from_fn(|i| {
                self.shields.get(i).map_or(o.shields[i], |s| s.is_active())
            }),
            armor: titles(&self.armor, &o.armor),
            styles: titles(&self.styles, &o.styles),
            traits: titles(&self.traits, &o.traits),
            disadvantages: titles(&self.disadvantages, &o.disadvantages),
            ignored: kit.ignored.clone(),
            every_other_listed: kit.every_other_listed,
            focus,
            kit_picks: std::array::from_fn(|i| {
                o.kit_picks[i].clone().or_else(|| self.picks[i].value())
            }),
            bought: kit.bought.clone(),
        }
    }

    /// Takes a weapon proficiency once the questions are answered Yes.
    fn add_weapon(&self, name: String) {
        let purchase =
            self.rules
                .weapon_purchase(&self.character, &self.combat(), &(self.levels)(), &name);
        let this = self.clone();
        self.buy(purchase, move |cost| this.weapon_warned(name.clone(), cost));
    }

    /// Asks a purchase's questions, then `take`s it for what it costs or
    /// for nothing.
    fn buy(&self, purchase: Purchase, take: impl Fn(i64) + 'static) {
        let parent = self.parent.clone();
        let Purchase {
            questions,
            free,
            cost,
        } = purchase;
        let take = Rc::new(take);
        ask_each(
            &self.parent,
            questions.into(),
            Rc::new(move || match &free {
                Some(free) => {
                    let take = take.clone();
                    ask(&parent, &free.title, &free.text, move || take(0), || {});
                }
                None => take(cost),
            }),
        );
    }

    /// Takes a weapon proficiency for `cost` once each of the kit's
    /// warnings is ignored.
    fn weapon_warned(&self, name: String, cost: i64) {
        let mut new = self.combat();
        let levels = (self.levels)();
        let warning = self
            .rules
            .weapon_warnings(&self.character, &new, &levels, &name)
            .into_iter()
            .next();
        if let Some(warning) = warning {
            let this = self.clone();
            ignorable(
                &self.parent,
                tr("Kit Weapon Proficiency Requirements"),
                &warning.text,
                move || {
                    this.kit.borrow_mut().ignored.push(warning.check);
                    this.weapon_warned(name.clone(), cost);
                },
            );
            return;
        }
        self.rules
            .bought(&self.character, &mut new, &levels, &name, cost);
        {
            let mut kit = self.kit.borrow_mut();
            kit.every_other_listed = new.every_other_listed;
            kit.bought = new.bought;
        }
        self.weapons.add(row(
            name,
            self.points(cost).map(cost_text).unwrap_or_default(),
        ));
    }

    /// Buys a broad (else tight) weapon group once the kit's warning is
    /// ignored and the questions are answered Yes, or says why the
    /// character may not.
    fn add_group(&self, broad: bool, name: String) {
        let new = self.combat();
        let levels = (self.levels)();
        let warning = self
            .rules
            .group_warnings(&self.character, &new, &levels, broad, &name)
            .into_iter()
            .next();
        if let Some(warning) = warning {
            let this = self.clone();
            ignorable(
                &self.parent,
                tr("Kit Weapon Proficiency Requirements"),
                &warning.text,
                move || {
                    this.kit.borrow_mut().ignored.push(warning.check);
                    this.add_group(broad, name.clone());
                },
            );
            return;
        }
        let purchase = match self
            .rules
            .group_purchase(&self.character, &new, &levels, broad)
        {
            Ok(purchase) => purchase,
            Err(text) => {
                message(&self.parent, tr("Weapon Groups"), &text);
                return;
            }
        };
        let this = self.clone();
        self.buy(purchase, move |cost| {
            let mut new = this.combat();
            this.rules.group_bought(&mut new, &name, cost);
            this.kit.borrow_mut().bought = new.bought;
            let list = if broad { &this.broad } else { &this.tight };
            if let Some(list) = list {
                let weapons = this.rules.group_weapons(broad, &name).join(", ");
                list.add(row(name.clone(), format!("{}: {weapons}", cost_text(cost))));
            }
        });
    }

    /// Specializes in a weapon once the questions are answered Yes and
    /// the kit's warnings ignored.
    fn specialize(&self, name: String) {
        let questions = self.rules.specialization_questions(
            &self.character,
            &self.combat(),
            &(self.levels)(),
            &name,
        );
        let this = self.clone();
        ask_each(
            &self.parent,
            questions.into(),
            Rc::new(move || this.specialization_warned(name.clone())),
        );
    }

    fn specialization_warned(&self, name: String) {
        let warning = self
            .rules
            .specialization_warnings(&self.character, &self.combat(), &(self.levels)(), &name)
            .into_iter()
            .next();
        if let Some(warning) = warning {
            let this = self.clone();
            ignorable(
                &self.parent,
                tr("Kit Specialization Requirements"),
                &warning.text,
                move || {
                    this.kit.borrow_mut().ignored.push(warning.check);
                    this.specialization_warned(name.clone());
                },
            );
            return;
        }
        let cost = self.rules.specialization_cost(&self.character);
        self.specializations.add(row(
            name,
            self.points(cost).map(cost_text).unwrap_or_default(),
        ));
    }

    /// Turns unarmed style `i` on once the questions are answered Yes.
    fn take_unarmed(&self, i: usize) {
        let purchase =
            self.rules
                .unarmed_purchase(&self.character, &self.combat(), &(self.levels)());
        let this = self.clone();
        self.buy(purchase, move |cost| {
            let mut new = this.combat();
            this.rules.bought(
                &this.character,
                &mut new,
                &(this.levels)(),
                UNARMED[i],
                cost,
            );
            this.kit.borrow_mut().bought = new.bought;
            this.quiet.set(true);
            this.unarmed[i].set_active(true);
            this.quiet.set(false);
        });
    }

    /// The weapons held, the groups' too.
    fn covered_weapons(&self) -> Vec<String> {
        self.rules
            .covered_weapons(&self.character, &self.combat(), &(self.levels)())
    }

    /// Points are shown for Skills & Powers characters only.
    fn points(&self, points: i64) -> Option<i64> {
        self.character.skills_and_powers().then_some(points)
    }

    /// Shows what depends on the levels and the weapons held: the slots
    /// left, the focus and the kit's picks.
    fn refresh_kit(&self) {
        let new = self.combat();
        let levels = (self.levels)();
        let slots = self
            .rules
            .core_slots_left(&self.character, &new, &levels)
            .map(|left| trn!("{n} slot left", "{n} slots left", left));
        self.weapons.group.set_description(slots.as_deref());
        let focus = self.rules.focus_choices(&self.character, &new, &levels);
        let focusing = focus.is_some();
        self.focus.row.set_visible(focusing);
        if let Some((prompt, weapons)) = focus {
            self.focus.row.set_subtitle(&prompt);
            self.focus.refill(weapons);
        }
        let picks = self.rules.kit_picks(&self.character, &new, &levels);
        for (slot, combo) in self.picks.iter().enumerate() {
            match picks.iter().find(|p| p.slot == slot) {
                Some(pick) => {
                    combo.row.set_title(&pick.title);
                    combo.row.set_subtitle(&pick.text);
                    combo.refill(pick.choices.clone());
                    combo.row.set_visible(true);
                }
                None => {
                    combo.refill(Vec::new());
                    combo.row.set_visible(false);
                }
            }
        }
        self.kit_group.set_visible(focusing || !picks.is_empty());
    }
}

/// A row of a name with `subtitle` under it.
fn row(name: String, subtitle: String) -> RowSpec<()> {
    RowSpec {
        value: (),
        title: name,
        subtitle,
        number: None,
    }
}

/// A list whose Add hands the name chosen from `offer()` to `add`, which
/// adds the row itself once done asking.
fn asking_list(
    title: &str,
    rows: Vec<RowSpec<()>>,
    offer: impl Fn() -> Vec<String> + 'static,
    add: impl Fn(String) + 'static,
) -> RowList<()> {
    RowList::new(title, rows, offer, move |name| {
        add(name);
        None
    })
}

#[derive(Clone)]
pub struct CombatEditor {
    pub page: adw::PreferencesPage,
    state: State,
    refresh: Rc<dyn Fn()>,
}

fn switch_row(title: &str, subtitle: &str, active: bool) -> adw::SwitchRow {
    adw::SwitchRow::builder()
        .title(title)
        .subtitle(subtitle)
        .active(active)
        .build()
}

impl CombatEditor {
    pub fn new(rules: Rc<CombatRules>, character: &Character, levels: Levels) -> Self {
        let skills_and_powers = character.skills_and_powers();
        let original = Rc::new(rules.combat_of(character));
        let page = adw::PreferencesPage::new();
        let character = Rc::new(character.clone());
        // Core Rules characters pay slots, which the original only counts.
        let points_of = move |points: i64| skills_and_powers.then_some(points);
        let subtitle = move |points: i64| points_of(points).map(cost_text).unwrap_or_default();
        // The lists ask before adding, which needs the page's state.
        let cell: Rc<RefCell<Option<State>>> = Rc::default();
        let state_of = {
            let cell = cell.clone();
            move || cell.borrow().clone()
        };

        let points = gtk::Label::builder().css_classes(["numeric"]).build();
        if skills_and_powers {
            let group = adw::PreferencesGroup::builder()
                .description(tr(
                    "Removing a weapon proficiency or weapon group refunds nothing; \
                     everything else on this page is refunded when removed.",
                ))
                .build();
            let row = adw::ActionRow::builder()
                .title(tr("Character Points Left"))
                .build();
            row.add_suffix(&points);
            group.add(&row);
            page.add(&group);
        }

        let weapons = {
            let held = (*levels)();
            let rows = original
                .weapons
                .iter()
                .map(|name| {
                    let cost = rules.weapon_price(&character, &original, &held, name);
                    row(name.clone(), subtitle(cost))
                })
                .collect();
            let offer = rules.weapon_names();
            let (offer_state, add_state) = (state_of.clone(), state_of.clone());
            asking_list(
                tr("Weapon Proficiencies"),
                rows,
                move || {
                    let taken = offer_state()
                        .map(|s| s.weapons.rows_titles())
                        .unwrap_or_default();
                    offer
                        .iter()
                        .filter(|n| !taken.contains(n))
                        .cloned()
                        .collect()
                },
                move |name| {
                    if let Some(state) = add_state() {
                        state.add_weapon(name);
                    }
                },
            )
        };
        page.add(&weapons.group);

        // Skills & Powers: the broad and tight weapon groups (IDD 308).
        let group_list = |broad: bool| {
            let taken = if broad {
                &original.broad_groups
            } else {
                &original.tight_groups
            };
            let (offer_state, add_state) = (state_of.clone(), state_of.clone());
            let list = asking_list(
                if broad {
                    tr("Broad Weapon Groups")
                } else {
                    tr("Tight Weapon Groups")
                },
                taken
                    .iter()
                    .map(|name| row(name.clone(), rules.group_weapons(broad, name).join(", ")))
                    .collect(),
                move || {
                    offer_state().map_or_else(Vec::new, |s| {
                        s.rules
                            .group_choices(&s.character, &s.combat(), &(s.levels)(), broad)
                    })
                },
                move |name| {
                    if let Some(state) = add_state() {
                        state.add_group(broad, name);
                    }
                },
            );
            list.group
                .set_description(Some(&cost_text(if broad { 6 } else { 4 })));
            page.add(&list.group);
            list
        };
        let groups = skills_and_powers
            && !rules
                .group_choices(&character, &original, &(*levels)(), true)
                .is_empty();
        let (broad, tight) = if groups || !original.broad_groups.is_empty() {
            (Some(group_list(true)), Some(group_list(false)))
        } else {
            (None, None)
        };

        let specialization = rules.specialization_cost(&character);
        let specializations = {
            let (offer_state, add_state) = (state_of.clone(), state_of.clone());
            asking_list(
                tr("Weapon Specializations"),
                original
                    .specializations
                    .iter()
                    .map(|name| row(name.clone(), subtitle(specialization)))
                    .collect(),
                move || {
                    offer_state().map_or_else(Vec::new, |s| {
                        let taken = s.specializations.rows_titles();
                        s.covered_weapons()
                            .into_iter()
                            .filter(|n| !taken.contains(n))
                            .collect()
                    })
                },
                move |name| {
                    if let Some(state) = add_state() {
                        state.specialize(name);
                    }
                },
            )
        };

        let group = adw::PreferencesGroup::builder()
            .title(tr("Specialization"))
            .build();
        let covered = rules.covered_weapons(&character, &original, &(*levels)());
        let (weapon_of_choice, expertise, mastery) = if skills_and_powers {
            let combo = |title: &str, cost: i64, options: Vec<String>, selected| {
                let combo = WeaponCombo::new(title, &cost_text(cost), options, selected);
                group.add(&combo.row);
                Some(combo)
            };
            (
                combo(
                    tr("Weapon of Choice"),
                    rules.weapon_of_choice_cost(&character),
                    covered.clone(),
                    original.weapon_of_choice.as_ref(),
                ),
                combo(
                    tr("Weapon Expertise"),
                    rules.expertise_cost(&character),
                    covered.clone(),
                    original.expertise.as_ref(),
                ),
                combo(
                    tr("Weapon Mastery"),
                    rules.mastery_cost(&character),
                    original.specializations.clone(),
                    original.mastery.as_ref(),
                ),
            )
        } else {
            (None, None, None)
        };
        let unarmed_subtitle = if skills_and_powers {
            cost_text(specialization)
        } else {
            trn!("{n} slot", "{n} slots", 2)
        };
        let unarmed: Vec<adw::SwitchRow> = UNARMED
            .iter()
            .zip(original.unarmed)
            .map(|(name, active)| {
                let row = switch_row(name, &unarmed_subtitle, active);
                group.add(&row);
                row
            })
            .collect();
        page.add(&group);
        page.add(&specializations.group);

        // The weapons the kit has the character focus in and pick.
        let kit_group = adw::PreferencesGroup::builder()
            .title(tr("Kit Weapons"))
            .build();
        let focus = WeaponCombo::new(
            tr("Weapon Focus"),
            "",
            original.weapons.clone(),
            original.focus.as_ref(),
        );
        kit_group.add(&focus.row);
        let picks = std::array::from_fn(|_| {
            let combo = WeaponCombo::new("", "", Vec::new(), None);
            kit_group.add(&combo.row);
            combo
        });
        page.add(&kit_group);

        let mut shields = Vec::new();
        let (mut armor, mut styles, mut traits, mut disadvantages) = (None, None, None, None);
        if skills_and_powers {
            let group = adw::PreferencesGroup::builder()
                .title(tr("Shield Proficiencies"))
                .build();
            let cost = cost_text(rules.shield_cost(&character));
            for (name, active) in SHIELDS.iter().zip(original.shields) {
                let row = switch_row(name, &cost, active);
                group.add(&row);
                shields.push(row);
            }
            page.add(&group);

            let cost = rules.armor_cost(&character);
            let offer = rules.armor_names().to_vec();
            let list = names_list(
                tr("Armor Proficiencies"),
                &original.armor,
                move |_| Some(cost),
                move |_| offer.clone(),
            );
            page.add(&list.group);
            armor = Some(list);

            let single = rules.single_style(&character);
            let (costs, offer) = ((rules.clone(), character.clone()), rules.style_names());
            let list = names_list(
                tr("Fighting Styles"),
                &original.styles,
                move |name| Some(costs.0.style_cost(&costs.1, name)),
                move |list| {
                    if single && !list.rows_titles().is_empty() {
                        Vec::new()
                    } else {
                        offer.clone()
                    }
                },
            );
            if single {
                list.group.set_description(Some(tr(
                    "A character without a warrior class may have a single style.",
                )));
            }
            page.add(&list.group);
            styles = Some(list);

            let (costs, offer) = ((rules.clone(), character.clone()), rules.trait_names());
            let list = names_list(
                tr("Traits"),
                &original.traits,
                move |name| Some(costs.0.trait_cost(&costs.1, name)),
                move |_| offer.clone(),
            );
            page.add(&list.group);
            traits = Some(list);

            let (costs, offer) = (
                (rules.clone(), character.clone()),
                rules.disadvantage_names(),
            );
            let list = names_list(
                tr("Disadvantages"),
                &original.disadvantages,
                move |name| Some(-costs.0.disadvantage_points(&costs.1, name)),
                move |_| offer.clone(),
            );
            page.add(&list.group);
            disadvantages = Some(list);
        }

        let state = State {
            rules: rules.clone(),
            character: character.clone(),
            levels,
            parent: page.clone().upcast(),
            kit: Rc::new(RefCell::new((*original).clone())),
            original,
            focus,
            picks,
            kit_group,
            quiet: Rc::default(),
            weapons,
            broad,
            tight,
            weapon_of_choice,
            expertise,
            mastery,
            specializations,
            unarmed,
            shields,
            armor,
            styles,
            traits,
            disadvantages,
        };
        *cell.borrow_mut() = Some(state.clone());
        let refresh: Rc<dyn Fn()> = Rc::new({
            let state = state.clone();
            move || {
                state.refresh_kit();
                let left = rules.points_left(&character, &state.combat());
                points.set_label(&left.to_string());
                if left < 0 {
                    points.add_css_class("error");
                } else {
                    points.remove_css_class("error");
                }
            }
        });
        // Skills & Powers: removing a proficiency the character had
        // refunds nothing, which the original says once.
        if skills_and_powers {
            let told = Rc::new(Cell::new(false));
            let lists = [
                (Some(state.weapons.clone()), None),
                (state.broad.clone(), Some(true)),
                (state.tight.clone(), Some(false)),
            ];
            for (list, group) in lists {
                let Some(list) = list else { continue };
                let (state, told) = (state.clone(), told.clone());
                list.connect_remove(move |_, remove| {
                    // A group stays while there is specialization
                    // information.
                    if let Some(broad) = group
                        && let Some(text) = state.rules.group_removal(
                            &state.character,
                            &state.combat(),
                            &(state.levels)(),
                            broad,
                        )
                    {
                        message(&state.parent, tr("Weapon Groups"), &text);
                        return;
                    }
                    let remove = RefCell::new(Some(remove));
                    let remove = move || {
                        if let Some(remove) = remove.borrow_mut().take() {
                            remove();
                        }
                    };
                    if told.replace(true) {
                        remove();
                        return;
                    }
                    notice(
                        &state.parent,
                        tr("Warning"),
                        tr(
                            "In Update, character points are not returned for removing Weapon \
                            Proficiencies, Tight Groups, or Broad Groups.",
                        ),
                        remove,
                    );
                });
            }
        }
        // The combos offer the proficiencies and specializations listed.
        let weapon_lists = [
            Some(&state.weapons),
            state.broad.as_ref(),
            state.tight.as_ref(),
        ];
        for list in weapon_lists.into_iter().flatten() {
            let (state, refresh) = (state.clone(), refresh.clone());
            list.connect_changed(move || {
                let weapons = state.covered_weapons();
                for combo in [&state.weapon_of_choice, &state.expertise]
                    .into_iter()
                    .flatten()
                {
                    combo.refill(weapons.clone());
                }
                refresh();
            });
        }
        state.specializations.connect_changed({
            let (state, refresh) = (state.clone(), refresh.clone());
            move || {
                if let Some(mastery) = &state.mastery {
                    mastery.refill(state.specializations.rows_titles());
                }
                refresh();
            }
        });
        for combo in [&state.weapon_of_choice, &state.expertise, &state.mastery]
            .into_iter()
            .flatten()
        {
            let refresh = refresh.clone();
            combo.row.connect_selected_notify(move |_| refresh());
        }
        for (i, row) in state.unarmed.iter().enumerate() {
            let (state, refresh) = (state.clone(), refresh.clone());
            row.connect_active_notify(move |row| {
                // Undone at once and asked about once the switch is done
                // switching.
                if row.is_active() && !state.quiet.get() {
                    state.quiet.set(true);
                    row.set_active(false);
                    state.quiet.set(false);
                    let state = state.clone();
                    glib::idle_add_local_once(move || state.take_unarmed(i));
                }
                refresh();
            });
        }
        for row in &state.shields {
            let refresh = refresh.clone();
            row.connect_active_notify(move |_| refresh());
        }
        for list in [
            &state.armor,
            &state.styles,
            &state.traits,
            &state.disadvantages,
        ]
        .into_iter()
        .flatten()
        {
            let refresh = refresh.clone();
            list.connect_changed(move || refresh());
        }
        refresh();
        Self {
            page,
            state,
            refresh,
        }
    }

    /// The choices as edited.
    pub fn combat(&self) -> CharacterCombat {
        self.state.combat()
    }

    /// Shows again what depends on the class levels, once they changed.
    pub fn levels_changed(&self) {
        (self.refresh)();
    }

    /// Gives the kit's warnings about the specializations, as leaving the
    /// original's page does, then runs `then` once each is ignored.
    pub fn check_leaving(&self, then: Rc<dyn Fn()>) {
        let state = &self.state;
        let warning = state
            .rules
            .leaving_warnings(&state.character, &state.combat(), &(state.levels)())
            .into_iter()
            .next();
        let Some(warning) = warning else {
            then();
            return;
        };
        let this = self.clone();
        ignorable(
            &state.parent,
            tr("Kit Weapon Proficiency Requirements"),
            &warning.text,
            move || {
                this.state.kit.borrow_mut().ignored.push(warning.check);
                this.check_leaving(then.clone());
            },
        );
    }
}
