//! The character editor's Combat page: the Expansion's Weapon
//! Proficiencies page of Update Character, with its Specialization, Shield
//! and Armor and Fighting Styles dialogs as groups, and the traits and
//! disadvantages of its Nonweapon Proficiencies page.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::character::Character;
use cr_i18n::tr;
use cr_store::combat::{CharacterCombat, CombatRules};
use cr_store::create::{SHIELDS, UNARMED};

use crate::abilities::{cost_text, names_list};
use crate::lists::RowList;

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
    original: Rc<CharacterCombat>,
    weapons: RowList<()>,
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
        CharacterCombat {
            weapons: self.weapons.rows_titles(),
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
        }
    }
}

pub struct CombatEditor {
    pub page: adw::PreferencesPage,
    state: State,
}

fn switch_row(title: &str, subtitle: &str, active: bool) -> adw::SwitchRow {
    adw::SwitchRow::builder()
        .title(title)
        .subtitle(subtitle)
        .active(active)
        .build()
}

impl CombatEditor {
    pub fn new(rules: Rc<CombatRules>, character: &Character) -> Self {
        let skills_and_powers = character.skills_and_powers();
        let original = Rc::new(rules.combat_of(character));
        let page = adw::PreferencesPage::new();
        let character = Rc::new(character.clone());
        // Core Rules characters pay slots, which the original only counts.
        let points_of = move |points: i64| skills_and_powers.then_some(points);

        let points = gtk::Label::builder().css_classes(["numeric"]).build();
        if skills_and_powers {
            let group = adw::PreferencesGroup::builder()
                .description(tr(
                    "Removing a weapon proficiency refunds nothing; everything \
                     else on this page is refunded when removed.",
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
            let costs = (rules.clone(), character.clone());
            let offer = rules.weapon_names();
            names_list(
                tr("Weapon Proficiencies"),
                &original.weapons,
                move |name| points_of(costs.0.weapon_cost(&costs.1, name)),
                move |_| offer.clone(),
            )
        };
        page.add(&weapons.group);

        let specialization = rules.specialization_cost(&character);
        let specializations = {
            let weapons = weapons.clone();
            names_list(
                tr("Weapon Specializations"),
                &original.specializations,
                move |_| points_of(specialization),
                move |_| weapons.rows_titles(),
            )
        };

        let group = adw::PreferencesGroup::builder()
            .title(tr("Specialization"))
            .build();
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
                    original.weapons.clone(),
                    original.weapon_of_choice.as_ref(),
                ),
                combo(
                    tr("Weapon Expertise"),
                    rules.expertise_cost(&character),
                    original.weapons.clone(),
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
            String::new()
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
            original,
            weapons,
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
        let refresh = Rc::new({
            let state = state.clone();
            move || {
                let left = rules.points_left(&character, &state.combat());
                points.set_label(&left.to_string());
                if left < 0 {
                    points.add_css_class("error");
                } else {
                    points.remove_css_class("error");
                }
            }
        });
        // The combos offer the proficiencies and specializations listed.
        state.weapons.connect_changed({
            let (state, refresh) = (state.clone(), refresh.clone());
            move || {
                let weapons = state.weapons.rows_titles();
                for combo in [&state.weapon_of_choice, &state.expertise]
                    .into_iter()
                    .flatten()
                {
                    combo.refill(weapons.clone());
                }
                refresh();
            }
        });
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
        for row in state.unarmed.iter().chain(&state.shields) {
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
        Self { page, state }
    }

    /// The choices as edited.
    pub fn combat(&self) -> CharacterCombat {
        self.state.combat()
    }
}
