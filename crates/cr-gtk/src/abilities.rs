//! The character editor's Abilities page: the Expansion's Race Abilities
//! and Class Abilities pages of Update Character, with its Schools and
//! Spheres dialogs as groups of each class.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::character::Character;
use cr_i18n::tr;
use cr_store::abilities::{AbilityRules, CharacterAbilities, SchoolRule, SphereRule};
use gtk::glib;

use crate::create::{combo_row, combo_value};
use crate::lists::{RowList, RowSpec};

/// One class's lists.
#[derive(Clone)]
struct ClassLists {
    abilities: RowList<()>,
    schools: Option<RowList<()>>,
    minor: Option<RowList<()>>,
    major: Option<RowList<()>>,
}

/// The page's widgets, cheap to clone into callbacks.
#[derive(Clone)]
struct State {
    rules: Rc<AbilityRules>,
    original: Rc<CharacterAbilities>,
    subrace: adw::ComboRow,
    racial: Option<RowList<()>>,
    classes: Vec<ClassLists>,
}

impl State {
    fn abilities(&self) -> CharacterAbilities {
        let titles = |list: &Option<RowList<()>>, old: &[String]| {
            list.as_ref()
                .map_or_else(|| old.to_vec(), RowList::rows_titles)
        };
        CharacterAbilities {
            subrace: combo_value(&self.subrace),
            racial: titles(&self.racial, &self.original.racial),
            classes: self
                .original
                .classes
                .iter()
                .zip(&self.classes)
                .map(|(old, lists)| cr_store::abilities::ClassAbilities {
                    abilities: lists.abilities.rows_titles(),
                    schools: titles(&lists.schools, &old.schools),
                    minor_spheres: titles(&lists.minor, &old.minor_spheres),
                    major_spheres: titles(&lists.major, &old.major_spheres),
                })
                .collect(),
        }
    }
}

pub struct AbilityEditor {
    pub page: adw::PreferencesPage,
    state: State,
}

/// "10 points", or "gives 5 points" for a restriction.
pub(crate) fn cost_text(points: i64) -> String {
    if points < 0 {
        tr!("gives {points} points", points = -points)
    } else {
        tr!("{points} points", points)
    }
}

fn row(title: &str, points: Option<i64>) -> RowSpec<()> {
    RowSpec {
        value: (),
        title: title.to_owned(),
        subtitle: points.map(cost_text).unwrap_or_default(),
        number: None,
    }
}

/// A list of names whose picker offers `offer()` minus the names listed.
pub(crate) fn names_list(
    title: &str,
    names: &[String],
    cost: impl Fn(&str) -> Option<i64> + 'static,
    offer: impl Fn(&RowList<()>) -> Vec<String> + 'static,
) -> RowList<()> {
    let cost = Rc::new(cost);
    let rows = names.iter().map(|n| row(n, cost(n))).collect();
    // The picker needs the list it fills, which exists only after `new`.
    let this: Rc<RefCell<Option<RowList<()>>>> = Rc::default();
    let list = RowList::new(
        title,
        rows,
        {
            let this = this.clone();
            move || {
                let this = this.borrow();
                let Some(list) = this.as_ref() else {
                    return Vec::new();
                };
                let taken = list.rows_titles();
                offer(list)
                    .into_iter()
                    .filter(|n| !taken.contains(n))
                    .collect()
            }
        },
        move |name| Some(row(&name, cost(&name))),
    );
    *this.borrow_mut() = Some(list.clone());
    list
}

impl AbilityEditor {
    pub fn new(rules: Rc<AbilityRules>, character: &Character) -> Self {
        let d = &character.data;
        let skills_and_powers = d.skills_and_powers != 0;
        let original = Rc::new(rules.abilities_of(character));
        let page = adw::PreferencesPage::new();

        let points = gtk::Label::builder().css_classes(["numeric"]).build();
        let points_row = adw::ActionRow::builder()
            .title(tr("Character Points Left"))
            .build();
        points_row.add_suffix(&points);
        let race = adw::PreferencesGroup::builder().title(tr("Race")).build();
        if skills_and_powers {
            race.set_description(Some(tr(
                "A new subrace brings its package and drops the racial abilities \
                 bought on top of the old one, refunding their points.",
            )));
        }
        race.add(&points_row);
        let subrace = combo_row(tr("Subrace"), &rules.subraces(character), &d.subrace);
        race.add(&subrace);
        page.add(&race);

        // The package cannot be removed while its subrace is chosen.
        let package = adw::PreferencesGroup::builder()
            .title(tr("Racial Package"))
            .build();
        let package_rows: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::default();
        let show_package = {
            let (rules, package, package_rows) =
                (rules.clone(), package.clone(), package_rows.clone());
            let race = d.race.clone();
            move |subrace: &str| {
                for row in package_rows.borrow_mut().drain(..) {
                    package.remove(&row);
                }
                let mut names = rules.package(&race, subrace);
                if names.is_empty() {
                    names.push(tr("None").into());
                }
                for name in names {
                    let row = adw::ActionRow::builder()
                        .title(glib::markup_escape_text(&name).as_str())
                        .build();
                    package.add(&row);
                    package_rows.borrow_mut().push(row);
                }
            }
        };
        show_package(&d.subrace);
        page.add(&package);

        let racial = skills_and_powers.then(|| {
            let choices: Vec<(String, i64)> = rules.racial_choices(character);
            let costs = choices.clone();
            let (offer_rules, offer_race, subrace) =
                (rules.clone(), d.race.clone(), subrace.clone());
            let list = names_list(
                tr("Racial Abilities Bought"),
                &original.racial,
                move |name| costs.iter().find(|c| c.0 == name).map(|c| c.1),
                move |_| {
                    let package = offer_rules.package(&offer_race, &combo_value(&subrace));
                    choices
                        .iter()
                        .map(|c| c.0.clone())
                        .filter(|n| !package.contains(n))
                        .collect()
                },
            );
            page.add(&list.group);
            list
        });

        let mut classes = Vec::new();
        for (slot, class) in d
            .classes
            .names
            .iter()
            .enumerate()
            .take(original.classes.len())
        {
            let old = &original.classes[slot];
            let ability_costs = rules.clone();
            let offer = rules.class_choices(class);
            let abilities = names_list(
                &tr!("Class Abilities ({class})", class),
                &old.abilities,
                move |name| Some(ability_costs.class_ability_cost(name)),
                move |_| offer.clone(),
            );
            page.add(&abilities.group);

            let free = !skills_and_powers;
            let schools = rules.school_rule(class).map(|rule| {
                let (costs, class_name) = (rules.clone(), class.clone());
                let offer = rules.school_choices(class);
                let list = names_list(
                    &tr!("Schools ({class})", class),
                    &old.schools,
                    move |name| (!free).then(|| costs.school_cost(&class_name, name).into()),
                    move |list| match rule {
                        SchoolRule::Single if !list.rows_titles().is_empty() => Vec::new(),
                        _ => offer.clone(),
                    },
                );
                if rule == SchoolRule::Single {
                    list.group
                        .set_description(Some(tr("This class may have a single school.")));
                }
                page.add(&list.group);
                list
            });

            let sphere_list = |title: String, names: &[String], major: bool, single: bool| {
                let (costs, class_name) = (rules.clone(), class.clone());
                let offer = rules.sphere_choices();
                let taken: Rc<RefCell<Vec<RowList<()>>>> = Rc::default();
                let list = names_list(
                    &title,
                    names,
                    move |name| {
                        let (minor, full) = costs.sphere_costs(&class_name, name);
                        (!free).then(|| if major { full } else { minor }.into())
                    },
                    {
                        let taken = taken.clone();
                        move |list| {
                            if single && !list.rows_titles().is_empty() {
                                return Vec::new();
                            }
                            // A sphere is either minor or major.
                            let others: Vec<String> = taken
                                .borrow()
                                .iter()
                                .flat_map(RowList::rows_titles)
                                .collect();
                            offer
                                .iter()
                                .filter(|s| !others.contains(s))
                                .cloned()
                                .collect()
                        }
                    },
                );
                page.add(&list.group);
                (list, taken)
            };
            let (minor, major) = match rules.sphere_rule(class) {
                Some(SphereRule::Full) => {
                    let (minor, minor_taken) = sphere_list(
                        tr!("Minor Spheres ({class})", class),
                        &old.minor_spheres,
                        false,
                        false,
                    );
                    let (major, major_taken) = sphere_list(
                        tr!("Major Spheres ({class})", class),
                        &old.major_spheres,
                        true,
                        false,
                    );
                    minor_taken.borrow_mut().push(major.clone());
                    major_taken.borrow_mut().push(minor.clone());
                    (Some(minor), Some(major))
                }
                Some(SphereRule::Single) => {
                    let (major, _) = sphere_list(
                        tr!("Sphere ({class})", class),
                        &old.major_spheres,
                        true,
                        true,
                    );
                    major.group.set_description(Some(tr(
                        "This class may buy major access to a single sphere.",
                    )));
                    (None, Some(major))
                }
                None => (None, None),
            };
            classes.push(ClassLists {
                abilities,
                schools,
                minor,
                major,
            });
        }

        let state = State {
            rules,
            original,
            subrace: subrace.clone(),
            racial,
            classes,
        };
        let character = Rc::new(character.clone());
        let refresh = Rc::new({
            let (state, character) = (state.clone(), character.clone());
            move || {
                let left = state.rules.points_left(&character, &state.abilities());
                points.set_label(&left.to_string());
                if left < 0 {
                    points.add_css_class("error");
                } else {
                    points.remove_css_class("error");
                }
            }
        });
        let lists = state
            .racial
            .iter()
            .chain(state.classes.iter().flat_map(|c| {
                [
                    Some(&c.abilities),
                    c.schools.as_ref(),
                    c.minor.as_ref(),
                    c.major.as_ref(),
                ]
                .into_iter()
                .flatten()
            }));
        for list in lists {
            let refresh = refresh.clone();
            list.connect_changed(move || refresh());
        }
        subrace.connect_selected_notify({
            let (racial, refresh) = (state.racial.clone(), refresh.clone());
            move |row| {
                show_package(&combo_value(row));
                if let Some(racial) = &racial {
                    racial.clear();
                }
                refresh();
            }
        });
        refresh();
        Self { page, state }
    }

    /// The choices as edited.
    pub fn abilities(&self) -> CharacterAbilities {
        self.state.abilities()
    }
}
