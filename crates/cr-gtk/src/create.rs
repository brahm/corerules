//! The New Character dialog: the original's creation wizard as a series of
//! pages, starting from the campaign setting, which narrows the races and
//! classes every later page offers.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_store::create::{
    ABILITIES, CreationRules, DISADVANTAGE_POINTS, Dice, Draft, METHOD_VII_POINTS,
    METHOD_VIII_DICE, MOST_DISADVANTAGES, RollMethod, STANDARD_PACK, SUBABILITIES,
    SUBABILITY_SPREAD, max_spells_per_level, point_table, roll_dice_pool,
};
use cr_store::settings::{SETTINGS, Setting};
use gtk::glib;

use crate::lists::{RowList, RowSpec};

/// Abbreviations for the ability summaries.
const SHORT: [&str; 6] = ["Str", "Dex", "Con", "Int", "Wis", "Cha"];

const SOCIAL_CLASSES: [&str; 4] = [
    "Lower class",
    "Lower middle class",
    "Upper middle class",
    "Upper class",
];

type Create = dyn Fn(&Draft) -> Result<(), String>;

struct Inner {
    rules: CreationRules,
    draft: RefCell<Draft>,
    dice: RefCell<Dice>,
    nav: adw::NavigationView,
    dialog: adw::Dialog,
    create: Box<Create>,
}

#[derive(Clone)]
struct Wizard(Rc<Inner>);

/// Opens the dialog; `create` writes the finished character and returns
/// why it could not, if it could not.
pub fn new_character(
    parent: &impl IsA<gtk::Widget>,
    rules: CreationRules,
    create: impl Fn(&Draft) -> Result<(), String> + 'static,
) {
    let nav = adw::NavigationView::new();
    let dialog = adw::Dialog::builder()
        .title("New Character")
        .content_width(520)
        .content_height(640)
        .child(&nav)
        .build();
    let wizard = Wizard(Rc::new(Inner {
        rules,
        draft: RefCell::new(Draft::new(&SETTINGS[0])),
        dice: RefCell::new(Dice::default()),
        nav,
        dialog: dialog.clone(),
        create: Box::new(create),
    }));
    let first = wizard.setting_page();
    wizard.0.nav.add(&first);
    dialog.present(Some(parent));
}

/// A page of the dialog: a header bar with `button` at the end and
/// `groups` below it.
fn step(
    title: &str,
    groups: &[&adw::PreferencesGroup],
    button: &gtk::Button,
    banner: Option<&adw::Banner>,
) -> adw::NavigationPage {
    let content = adw::PreferencesPage::new();
    for group in groups {
        content.add(*group);
    }
    let header = adw::HeaderBar::new();
    header.pack_end(button);
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    if let Some(banner) = banner {
        view.add_top_bar(banner);
    }
    view.set_content(Some(&content));
    adw::NavigationPage::builder()
        .title(title)
        .child(&view)
        .build()
}

fn next_button(label: &str) -> gtk::Button {
    gtk::Button::builder()
        .label(label)
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build()
}

fn roll_button(tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text(tooltip)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
}

fn escape(text: &str) -> String {
    glib::markup_escape_text(text).to_string()
}

/// Adds one row per choice with a radio button; `chosen` gets the index
/// of the selected one. Returns the rows.
fn radio_rows(
    group: &adw::PreferencesGroup,
    choices: &[(String, String)],
    selected: usize,
    chosen: impl Fn(usize) + 'static,
) -> Vec<adw::ActionRow> {
    let chosen = Rc::new(chosen);
    let mut first: Option<gtk::CheckButton> = None;
    let mut rows = Vec::new();
    for (i, (title, subtitle)) in choices.iter().enumerate() {
        let check = gtk::CheckButton::builder()
            .valign(gtk::Align::Center)
            .active(i == selected)
            .build();
        if let Some(first) = &first {
            check.set_group(Some(first));
        } else {
            first = Some(check.clone());
        }
        let row = adw::ActionRow::builder()
            .title(escape(title))
            .activatable_widget(&check)
            .build();
        if !subtitle.is_empty() {
            row.set_subtitle(&escape(subtitle));
        }
        row.add_prefix(&check);
        check.connect_toggled({
            let chosen = chosen.clone();
            move |check| {
                if check.is_active() {
                    chosen(i);
                }
            }
        });
        group.add(&row);
        rows.push(row);
    }
    rows
}

fn combo_row(title: &str, items: &[String], selected: &str) -> adw::ComboRow {
    let model = gtk::StringList::new(&items.iter().map(String::as_str).collect::<Vec<_>>());
    let row = adw::ComboRow::builder().title(title).model(&model).build();
    if let Some(i) = items.iter().position(|i| i == selected) {
        row.set_selected(i as u32);
    }
    row
}

fn combo_value(row: &adw::ComboRow) -> String {
    row.selected_item()
        .and_downcast::<gtk::StringObject>()
        .map(|s| s.string().to_string())
        .unwrap_or_default()
}

fn spin_row(title: &str, subtitle: &str, value: u32, min: u32, max: u32) -> adw::SpinRow {
    adw::SpinRow::builder()
        .title(title)
        .subtitle(subtitle)
        .numeric(true)
        .adjustment(&gtk::Adjustment::new(
            value.into(),
            min.into(),
            max.into(),
            1.0,
            5.0,
            0.0,
        ))
        .build()
}

/// "34 gp 6 sp 7 cp" for an amount in copper pieces.
fn money(copper: u32) -> String {
    let parts: Vec<String> = [
        (copper / 100, "gp"),
        (copper / 10 % 10, "sp"),
        (copper % 10, "cp"),
    ]
    .iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, unit)| format!("{n} {unit}"))
    .collect();
    if parts.is_empty() {
        "Free".into()
    } else {
        parts.join(" ")
    }
}

fn slots(n: u32) -> String {
    if n == 1 {
        "1 slot".into()
    } else {
        format!("{n} slots")
    }
}

/// "Axe bonus" for "Dwarf, Axe bonus (5)": the race or class and the cost
/// are shown elsewhere.
fn ability_title(name: &str) -> String {
    let name = name.split_once(", ").map_or(name, |(_, rest)| rest);
    match name.rsplit_once(" (") {
        Some((title, cost)) if cost.trim_end_matches(')').parse::<i32>().is_ok() => title,
        _ => name,
    }
    .to_owned()
}

fn points(n: i64) -> String {
    if n.abs() == 1 {
        format!("{n} point")
    } else {
        format!("{n} points")
    }
    .replace('-', "−")
}

/// One check row per ability with its cost; `changed` gets the names
/// checked after every toggle.
fn ability_rows(
    group: &adw::PreferencesGroup,
    choices: &[(String, i32)],
    selected: &[String],
    changed: impl Fn(Vec<String>) + 'static,
) {
    let checks: Rc<RefCell<Vec<(String, gtk::CheckButton)>>> = Rc::default();
    let changed = Rc::new(changed);
    for (name, cost) in choices {
        // The databases have some abilities in two versions that differ
        // only in cost (Followers (5) and (10)); those keep it.
        let mut title = ability_title(name);
        if choices
            .iter()
            .any(|(other, _)| other != name && ability_title(other) == title)
        {
            title = name
                .split_once(", ")
                .map_or(name.as_str(), |n| n.1)
                .to_owned();
        }
        let check = gtk::CheckButton::builder()
            .valign(gtk::Align::Center)
            .active(selected.contains(name))
            .build();
        let row = adw::ActionRow::builder()
            .title(escape(&title))
            .subtitle(if *cost < 0 {
                format!("Gives {}", points(i64::from(-cost)))
            } else {
                points(i64::from(*cost))
            })
            .activatable_widget(&check)
            .build();
        row.add_prefix(&check);
        check.connect_toggled({
            let (checks, changed) = (checks.clone(), changed.clone());
            move |_| {
                let names = checks
                    .borrow()
                    .iter()
                    .filter(|(_, c)| c.is_active())
                    .map(|(n, _)| n.clone())
                    .collect();
                changed(names);
            }
        });
        group.add(&row);
        checks.borrow_mut().push((name.clone(), check));
    }
}

fn scores_summary(scores: &[u32; 6]) -> String {
    SHORT
        .iter()
        .zip(scores)
        .map(|(a, s)| format!("{a} {s}"))
        .collect::<Vec<_>>()
        .join(" · ")
}

impl Wizard {
    fn rules(&self) -> &CreationRules {
        &self.0.rules
    }

    fn push(&self, page: adw::NavigationPage) {
        self.0.nav.push(&page);
    }

    /// The rules and the campaign setting.
    fn setting_page(&self) -> adw::NavigationPage {
        let rules_group = adw::PreferencesGroup::builder()
            .title("Rules")
            .description("The original's Core Rules and Player's Option buttons")
            .build();
        let skills_and_powers = Rc::new(RefCell::new(false));
        radio_rows(
            &rules_group,
            &[
                (
                    "Core Rules".into(),
                    "Player's Handbook: proficiency slots".into(),
                ),
                (
                    "Player's Option".into(),
                    "Skills & Powers: subabilities, and character points for racial and \
                     class abilities, proficiencies and traits"
                        .into(),
                ),
            ],
            0,
            {
                let skills_and_powers = skills_and_powers.clone();
                move |i| *skills_and_powers.borrow_mut() = i == 1
            },
        );
        // Core Rules 2.0 has no Player's Option.
        rules_group.set_visible(self.rules().expansion);

        let group = adw::PreferencesGroup::builder()
            .title("Campaign Setting")
            .description(
                "The setting limits the races and classes offered on the next pages. \
                 The original program has no settings; these follow each setting's \
                 campaign book.",
            )
            .build();
        let choices: Vec<(String, String)> = SETTINGS
            .iter()
            .map(|s| (s.name.to_owned(), s.description.to_owned()))
            .collect();
        let chosen: Rc<RefCell<&'static Setting>> = Rc::new(RefCell::new(&SETTINGS[0]));
        radio_rows(&group, &choices, 0, {
            let chosen = chosen.clone();
            move |i| *chosen.borrow_mut() = &SETTINGS[i]
        });
        let next = next_button("_Next");
        next.connect_clicked({
            let this = self.clone();
            move |_| {
                {
                    let mut d = this.0.draft.borrow_mut();
                    d.setting = *chosen.borrow();
                    d.skills_and_powers = *skills_and_powers.borrow();
                }
                this.push(this.abilities_page());
            }
        });
        step("New Character", &[&rules_group, &group], &next, None)
    }

    /// Rolling the ability scores.
    fn abilities_page(&self) -> adw::NavigationPage {
        let skills_and_powers = self.0.draft.borrow().skills_and_powers;
        let methods = RollMethod::offered(skills_and_powers);
        let names: Vec<String> = methods.iter().map(|m| m.name().into()).collect();
        let method_row = combo_row("Method", &names, RollMethod::FourDice.name());
        let method_group = adw::PreferencesGroup::new();
        method_group.add(&method_row);

        let roll = roll_button("Roll Again");
        let scores_group = adw::PreferencesGroup::builder()
            .title("Ability Scores")
            .header_suffix(&roll)
            .build();
        let scores: Rc<RefCell<[u32; 6]>> = Rc::new(RefCell::new([10; 6]));
        let method = Rc::new(RefCell::new(RollMethod::FourDice));
        // Points to divide and the highest score (Methods VII and IX), and
        // the 2d6 that gave them.
        let budget = Rc::new(RefCell::new((METHOD_VII_POINTS, 18, 0)));
        // Whether Method VIII's scores were rolled with the dice shown.
        let pool_rolled = Rc::new(RefCell::new(false));
        let next = next_button("_Next");

        // Rolled scores show as labels, moved up and down when the method
        // lets the player arrange them; typed or divided scores use spin
        // rows, and Method VIII has a spin row of dice per ability.
        let mut labels = Vec::new();
        let mut rolled_rows = Vec::new();
        let mut arrows = Vec::new();
        let entered: Vec<adw::SpinRow> = ABILITIES
            .iter()
            .map(|a| spin_row(a, "", 10, 3, 18))
            .collect();
        let dice_rows: Vec<adw::SpinRow> = ABILITIES
            .iter()
            .map(|a| spin_row(a, "4 dice", 4, 3, 6))
            .collect();
        let pool_labels: Vec<gtk::Label> = (0..6)
            .map(|_| gtk::Label::builder().css_classes(["title-4"]).build())
            .collect();
        for (a, name) in ABILITIES.iter().enumerate() {
            let label = gtk::Label::builder().css_classes(["title-4"]).build();
            let up = gtk::Button::builder()
                .icon_name("go-up-symbolic")
                .tooltip_text("Move Up")
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .sensitive(a > 0)
                .build();
            let down = gtk::Button::builder()
                .icon_name("go-down-symbolic")
                .tooltip_text("Move Down")
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .sensitive(a < 5)
                .build();
            let row = adw::ActionRow::builder().title(*name).build();
            row.add_suffix(&label);
            row.add_suffix(&up);
            row.add_suffix(&down);
            dice_rows[a].add_prefix(&pool_labels[a]);
            scores_group.add(&row);
            scores_group.add(&entered[a]);
            scores_group.add(&dice_rows[a]);
            labels.push(label);
            rolled_rows.push(row);
            arrows.push((up, down));
        }
        let refresh = Rc::new({
            let (scores, method, labels) = (scores.clone(), method.clone(), labels.clone());
            let (rolled_rows, entered, arrows, roll) = (
                rolled_rows.clone(),
                entered.clone(),
                arrows.clone(),
                roll.clone(),
            );
            let (dice_rows, pool_labels, budget, pool_rolled) = (
                dice_rows.clone(),
                pool_labels.clone(),
                budget.clone(),
                pool_rolled.clone(),
            );
            let (method_row, scores_group, next) =
                (method_row.clone(), scores_group.clone(), next.clone());
            move || {
                let m = *method.borrow();
                let typed = m == RollMethod::Enter || m.divided();
                let pool = m == RollMethod::DicePool;
                method_row.set_subtitle(m.description());
                roll.set_visible(!matches!(m, RollMethod::Enter | RollMethod::Points));
                let (points, highest, rolled) = *budget.borrow();
                for a in 0..6 {
                    labels[a].set_label(&scores.borrow()[a].to_string());
                    rolled_rows[a].set_visible(!typed && !pool);
                    entered[a].set_visible(typed);
                    dice_rows[a].set_visible(pool);
                    arrows[a].0.set_visible(m.arranged());
                    arrows[a].1.set_visible(m.arranged());
                    if m.divided() {
                        entered[a].adjustment().set_upper(highest.into());
                    } else {
                        entered[a].adjustment().set_upper(18.0);
                    }
                    let n = dice_rows[a].value() as u32;
                    dice_rows[a].set_subtitle(&format!("{n} dice"));
                    pool_labels[a].set_label(&if *pool_rolled.borrow() {
                        scores.borrow()[a].to_string()
                    } else {
                        "–".to_owned()
                    });
                }
                let ok = if m.divided() {
                    let total: u32 = entered.iter().map(|e| e.value() as u32).sum();
                    let left = points as i64 - total as i64;
                    let roll = if m == RollMethod::RolledPoints {
                        format!("2d6 rolled {rolled}: ")
                    } else {
                        String::new()
                    };
                    scores_group.set_description(Some(&format!(
                        "{roll}{left} of {points} points left · highest score {highest}"
                    )));
                    left == 0
                } else if pool {
                    let total: u32 = dice_rows.iter().map(|e| e.value() as u32).sum();
                    let left = METHOD_VIII_DICE as i64 - total as i64;
                    scores_group.set_description(Some(&format!(
                        "{left} of {METHOD_VIII_DICE} dice left · roll when all are given out"
                    )));
                    left == 0 && *pool_rolled.borrow()
                } else {
                    scores_group.set_description(None);
                    true
                };
                next.set_sensitive(ok);
            }
        });
        let reroll = Rc::new({
            let (this, scores, method, refresh) = (
                self.clone(),
                scores.clone(),
                method.clone(),
                refresh.clone(),
            );
            let (budget, pool_rolled, dice_rows) =
                (budget.clone(), pool_rolled.clone(), dice_rows.clone());
            move || {
                let m = *method.borrow();
                let mut dice = this.0.dice.borrow_mut();
                match m {
                    RollMethod::Enter => {}
                    RollMethod::Points => *budget.borrow_mut() = (METHOD_VII_POINTS, 18, 0),
                    RollMethod::RolledPoints => {
                        let roll = dice.roll(2, 6);
                        let (points, highest) = point_table(roll);
                        *budget.borrow_mut() = (points, highest, roll);
                    }
                    RollMethod::DicePool => {
                        let counts: [u32; 6] = std::array::from_fn(|a| dice_rows[a].value() as u32);
                        if counts.iter().sum::<u32>() == METHOD_VIII_DICE {
                            *scores.borrow_mut() = roll_dice_pool(counts, &mut dice);
                            *pool_rolled.borrow_mut() = true;
                        }
                    }
                    _ => *scores.borrow_mut() = m.roll(&mut dice),
                }
                drop(dice);
                refresh();
            }
        });
        for (a, (up, down)) in arrows.iter().enumerate() {
            for (button, other) in [(up, a.wrapping_sub(1)), (down, a + 1)] {
                button.connect_clicked({
                    let (scores, refresh) = (scores.clone(), refresh.clone());
                    move |_| {
                        scores.borrow_mut().swap(a, other);
                        refresh();
                    }
                });
            }
        }
        for row in &entered {
            let refresh = refresh.clone();
            row.connect_value_notify(move |_| refresh());
        }
        for row in &dice_rows {
            let (refresh, pool_rolled) = (refresh.clone(), pool_rolled.clone());
            row.connect_value_notify(move |_| {
                *pool_rolled.borrow_mut() = false;
                refresh();
            });
        }
        method_row.connect_selected_notify({
            let (method, reroll) = (method.clone(), reroll.clone());
            move |row| {
                *method.borrow_mut() = methods[row.selected() as usize];
                reroll();
            }
        });
        roll.connect_clicked({
            let reroll = reroll.clone();
            move |_| reroll()
        });
        reroll();

        next.connect_clicked({
            let this = self.clone();
            move |_| {
                let m = *method.borrow();
                let values = if m == RollMethod::Enter || m.divided() {
                    std::array::from_fn(|a| entered[a].value() as u32)
                } else {
                    *scores.borrow()
                };
                this.0.draft.borrow_mut().scores = values;
                this.push(this.race_page());
            }
        });
        step(
            "Ability Scores",
            &[&method_group, &scores_group],
            &next,
            None,
        )
    }

    /// The race, among those whose limits the scores meet.
    fn race_page(&self) -> adw::NavigationPage {
        let (setting, scores) = {
            let d = self.0.draft.borrow();
            (d.setting, d.scores)
        };
        let standard = adw::SwitchRow::builder()
            .title("Standard Races Only")
            .subtitle("Leave out the monster races, as the original's switch does")
            .build();
        let options_group = adw::PreferencesGroup::new();
        options_group.add(&standard);
        let races_group = adw::PreferencesGroup::builder()
            .title("Race")
            .description(format!(
                "{} · races whose ability limits these scores meet",
                scores_summary(&scores)
            ))
            .build();
        let subrace_group = adw::PreferencesGroup::new();
        let subrace_row = adw::ComboRow::builder().title("Subrace").build();
        subrace_group.add(&subrace_row);
        let next = next_button("_Next");
        let chosen: Rc<RefCell<String>> = Rc::default();
        let rows: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::default();
        let empty = adw::ActionRow::builder()
            .title("No race allows these scores")
            .subtitle("Go back and roll again")
            .css_classes(["dim-label"])
            .build();
        races_group.add(&empty);

        let show_subraces = Rc::new({
            let (this, subrace_row, subrace_group) =
                (self.clone(), subrace_row.clone(), subrace_group.clone());
            move |race: &str| {
                let subraces = this.rules().subraces(setting, race);
                subrace_group.set_visible(subraces.len() > 1 || subraces[0] != race);
                let model =
                    gtk::StringList::new(&subraces.iter().map(String::as_str).collect::<Vec<_>>());
                subrace_row.set_model(Some(&model));
                subrace_row.set_selected(0);
            }
        });
        let fill = Rc::new({
            let (this, races_group, rows, chosen, next, empty) = (
                self.clone(),
                races_group.clone(),
                rows.clone(),
                chosen.clone(),
                next.clone(),
                empty.clone(),
            );
            let show_subraces = show_subraces.clone();
            move |standard_only: bool| {
                for row in rows.borrow_mut().drain(..) {
                    races_group.remove(&row);
                }
                let races = this.rules().races(setting, &scores, standard_only);
                let choices: Vec<(String, String)> = races
                    .iter()
                    .map(|r| {
                        let adjustments: Vec<String> = this
                            .rules()
                            .race(r)
                            .map(|r| r.adjustments)
                            .unwrap_or_default()
                            .iter()
                            .enumerate()
                            .filter(|(_, v)| **v != 0)
                            .map(|(a, v)| format!("{} {v:+}", SHORT[a]))
                            .collect();
                        let subtitle = if adjustments.is_empty() {
                            "No ability adjustments".to_owned()
                        } else {
                            adjustments.join(" · ").replace('-', "−")
                        };
                        (r.clone(), subtitle)
                    })
                    .collect();
                let previous = chosen.borrow().clone();
                let selected = races.iter().position(|r| *r == previous).unwrap_or(0);
                *chosen.borrow_mut() = races.get(selected).cloned().unwrap_or_default();
                *rows.borrow_mut() = radio_rows(&races_group, &choices, selected, {
                    let (chosen, races, show_subraces) =
                        (chosen.clone(), races.clone(), show_subraces.clone());
                    move |i| {
                        *chosen.borrow_mut() = races[i].clone();
                        show_subraces(&races[i]);
                    }
                });
                empty.set_visible(races.is_empty());
                next.set_sensitive(!races.is_empty());
                if let Some(race) = races.get(selected) {
                    show_subraces(race);
                } else {
                    races_group.set_visible(true);
                }
            }
        });
        *chosen.borrow_mut() = self.0.draft.borrow().race.clone();
        fill(false);
        standard.connect_active_notify({
            let fill = fill.clone();
            move |row| fill(row.is_active())
        });
        next.connect_clicked({
            let this = self.clone();
            move |_| {
                let race = chosen.borrow().clone();
                let subraces = this.rules().subraces(setting, &race);
                let subrace = subraces
                    .get(subrace_row.selected() as usize)
                    .cloned()
                    .unwrap_or_else(|| race.clone());
                let skills_and_powers = {
                    let mut d = this.0.draft.borrow_mut();
                    if d.race != race || d.subrace != subrace {
                        d.racial_abilities = this.rules().racial_package(&race, &subrace);
                    }
                    d.race = race;
                    d.subrace = subrace;
                    d.skills_and_powers
                };
                if skills_and_powers {
                    this.push(this.racial_page());
                } else {
                    this.push(this.class_page());
                }
            }
        });
        step(
            "Race",
            &[&options_group, &races_group, &subrace_group],
            &next,
            None,
        )
    }

    /// The class or multi-class combination.
    fn class_page(&self) -> adw::NavigationPage {
        let (setting, race, adjusted, previous) = {
            let d = self.0.draft.borrow();
            let adjusted = self.rules().adjusted(&d.race, &d.scores);
            (d.setting, d.race.clone(), adjusted, d.classes.clone())
        };
        let options = self.rules().class_options(setting, &race, &adjusted);
        let group = adw::PreferencesGroup::builder()
            .title("Class")
            .description(format!(
                "{race} · {} after racial adjustments",
                scores_summary(&adjusted)
            ))
            .build();
        let choices: Vec<(String, String)> = options
            .iter()
            .map(|classes| {
                let dice: Vec<String> = classes
                    .iter()
                    .filter_map(|c| self.rules().class(c))
                    .map(|c| format!("d{}", c.hit_die))
                    .collect();
                let what = if classes.len() > 1 {
                    "Multi-class"
                } else {
                    "Hit die"
                };
                (classes.join("/"), format!("{what} · {}", dice.join("/")))
            })
            .collect();
        let selected = options.iter().position(|o| *o == previous).unwrap_or(0);
        let chosen = Rc::new(RefCell::new(selected));
        radio_rows(&group, &choices, selected, {
            let chosen = chosen.clone();
            move |i| *chosen.borrow_mut() = i
        });
        let next = next_button("_Next");
        if options.is_empty() {
            group.add(
                &adw::ActionRow::builder()
                    .title("No class allows these scores")
                    .subtitle("Go back and choose another race or roll again")
                    .css_classes(["dim-label"])
                    .build(),
            );
            next.set_sensitive(false);
        }
        next.connect_clicked({
            let this = self.clone();
            move |_| {
                let Some(classes) = options.get(*chosen.borrow()).cloned() else {
                    return;
                };
                this.start_details(classes);
                if this.0.draft.borrow().skills_and_powers {
                    this.push(this.class_abilities_page());
                } else {
                    this.push(this.details_page());
                }
            }
        });
        step("Class", &[&group], &next, None)
    }

    /// Skills & Powers: racial abilities bought with the race's points,
    /// starting from the subrace's package as the original does.
    fn racial_page(&self) -> adw::NavigationPage {
        let d = self.0.draft.borrow().clone();
        let rules = self.rules();
        let (total, saved) = rules.racial_points(&d.race, &d.subrace);
        let choices = rules.racial_ability_choices(&d.race, &d.subrace);
        let group = adw::PreferencesGroup::builder()
            .title("Racial Abilities")
            .build();
        let next = next_button("_Next");
        let chosen = Rc::new(RefCell::new(d.racial_abilities.clone()));
        let update = Rc::new({
            let (this, group, next, chosen) =
                (self.clone(), group.clone(), next.clone(), chosen.clone());
            move || {
                let mut probe = this.0.draft.borrow().clone();
                probe.racial_abilities = chosen.borrow().clone();
                let left = this.rules().racial_points_left(&probe);
                group.set_description(Some(&format!(
                    "{} of {} left; up to {} kept for proficiencies",
                    points(left),
                    total,
                    saved
                )));
                next.set_sensitive(left >= 0);
            }
        });
        if total == 0 || choices.is_empty() {
            // Monster races come with fixed abilities.
            group.set_description(Some("The race's abilities are fixed"));
            for name in &d.racial_abilities {
                group.add(
                    &adw::ActionRow::builder()
                        .title(escape(&ability_title(name)))
                        .build(),
                );
            }
            if d.racial_abilities.is_empty() {
                group.add(
                    &adw::ActionRow::builder()
                        .title("None")
                        .css_classes(["dim-label"])
                        .build(),
                );
            }
        } else {
            ability_rows(&group, &choices, &d.racial_abilities, {
                let (chosen, update) = (chosen.clone(), update.clone());
                move |names| {
                    *chosen.borrow_mut() = names;
                    update();
                }
            });
            update();
        }
        next.connect_clicked({
            let this = self.clone();
            move |_| {
                this.0.draft.borrow_mut().racial_abilities = chosen.borrow().clone();
                this.push(this.class_page());
            }
        });
        step(&format!("{} Abilities", d.subrace), &[&group], &next, None)
    }

    /// Skills & Powers: class abilities bought with each class's points,
    /// starting from the class's defaults.
    fn class_abilities_page(&self) -> adw::NavigationPage {
        let d = self.0.draft.borrow().clone();
        let next = next_button("_Next");
        let chosen = Rc::new(RefCell::new(d.class_abilities.clone()));
        let groups: Vec<adw::PreferencesGroup> = d
            .classes
            .iter()
            .map(|class| {
                adw::PreferencesGroup::builder()
                    .title(escape(&format!("{class} Abilities")))
                    .build()
            })
            .collect();
        let update = Rc::new({
            let (this, groups, next, chosen) =
                (self.clone(), groups.clone(), next.clone(), chosen.clone());
            move || {
                let mut probe = this.0.draft.borrow().clone();
                probe.class_abilities = chosen.borrow().clone();
                let mut fine = true;
                for (slot, group) in groups.iter().enumerate() {
                    let total = probe
                        .classes
                        .get(slot)
                        .and_then(|c| this.rules().class(c))
                        .map_or(0, |c| c.points.max(0));
                    let left = this.rules().class_points_left(&probe, slot);
                    fine &= left >= 0;
                    group.set_description(Some(&format!(
                        "{} of {total} left; what is left goes to proficiencies",
                        points(left)
                    )));
                }
                next.set_sensitive(fine);
            }
        });
        for (slot, (class, group)) in d.classes.iter().zip(&groups).enumerate() {
            let choices = self.rules().class_ability_choices(class);
            let selected = d.class_abilities.get(slot).cloned().unwrap_or_default();
            ability_rows(group, &choices, &selected, {
                let (chosen, update) = (chosen.clone(), update.clone());
                move |names| {
                    if let Some(abilities) = chosen.borrow_mut().get_mut(slot) {
                        *abilities = names;
                    }
                    update();
                }
            });
        }
        update();
        next.connect_clicked({
            let this = self.clone();
            move |_| {
                {
                    let mut d = this.0.draft.borrow_mut();
                    d.class_abilities = chosen.borrow().clone();
                    if this.rules().specialization_cost(&d).is_none() {
                        d.specializations.clear();
                    }
                }
                this.push(this.details_page());
            }
        });
        let groups: Vec<&adw::PreferencesGroup> = groups.iter().collect();
        step("Class Abilities", &groups, &next, None)
    }

    /// Rolls what the details page starts with for the chosen classes, as
    /// the original does when it reaches its name page.
    fn start_details(&self, classes: Vec<String>) {
        let rules = self.rules();
        let mut dice = self.0.dice.borrow_mut();
        let mut d = self.0.draft.borrow_mut();
        let changed = d.classes != classes;
        d.classes = classes;
        let race = d.race.clone();
        let alignments = rules.alignments(&d.classes);
        if !alignments.contains(&d.alignment) {
            d.alignment = alignments.first().cloned().unwrap_or_default();
        }
        if !rules.kits(&race, &d.subrace, &d.classes).contains(&d.kit) {
            d.kit = "None".into();
        }
        d.social_class = rules.roll_social_class(&d.kit, &mut dice);
        d.age = rules.roll_age(&race, &mut dice);
        d.height = rules.roll_height(&race, d.male, &mut dice);
        d.weight = rules.roll_weight(&race, d.male, &mut dice);
        d.hit_dice = rules.roll_hit_dice(&d.classes, &mut dice);
        d.exceptional_strength = dice.roll(1, 100);
        d.gold = rules.roll_gold(&d.classes, &mut dice);
        if changed {
            d.nonweapon_proficiencies.clear();
            d.weapon_proficiencies.clear();
            d.spells.clear();
            d.purchases.clear();
        }
        if changed || d.class_abilities.len() != d.classes.len() {
            d.class_abilities = d
                .classes
                .iter()
                .map(|c| rules.default_class_abilities(c))
                .collect();
            d.specializations.clear();
        }
        d.languages = rules.native_languages(&d);
    }

    /// Name, player, gender, alignment, kit, social class, age, height,
    /// weight and hit points.
    fn details_page(&self) -> adw::NavigationPage {
        let d = self.0.draft.borrow().clone();
        let rules = self.rules();
        let name = adw::EntryRow::builder().title("Name").text(&d.name).build();
        let player = adw::EntryRow::builder()
            .title("Player")
            .text(&d.player)
            .build();
        let genders = ["Male".to_owned(), "Female".to_owned()];
        let gender = combo_row("Gender", &genders, if d.male { "Male" } else { "Female" });
        let who = adw::PreferencesGroup::builder()
            .title(escape(&format!("{} {}", d.race, d.class_title())))
            .build();
        who.add(&name);
        who.add(&player);
        who.add(&gender);

        let alignment = combo_row("Alignment", &rules.alignments(&d.classes), &d.alignment);
        let kit = combo_row("Kit", &rules.kits(&d.race, &d.subrace, &d.classes), &d.kit);
        let social: Vec<String> = SOCIAL_CLASSES.iter().map(|s| s.to_string()).collect();
        let social_class = combo_row("Social Class", &social, &d.social_class);
        let social_roll = roll_button("Roll Social Class");
        social_class.add_suffix(&social_roll);
        let background = adw::PreferencesGroup::new();
        background.add(&alignment);
        background.add(&kit);
        background.add(&social_class);

        let age = spin_row("Age", "Years", d.age, 1, 9999);
        let height = spin_row("Height", "Inches", d.height, 1, 999);
        let weight = spin_row("Weight", "Pounds", d.weight, 1, 9999);
        let rolls = [
            roll_button("Roll Age"),
            roll_button("Roll Height"),
            roll_button("Roll Weight"),
        ];
        for (row, button) in [&age, &height, &weight].iter().zip(&rolls) {
            row.add_suffix(button);
        }
        let appearance = adw::PreferencesGroup::builder().title("Appearance").build();
        appearance.add(&age);
        appearance.add(&height);
        appearance.add(&weight);

        let hit_dice = Rc::new(RefCell::new(d.hit_dice.clone()));
        let hit_points = adw::ActionRow::builder()
            .title("Hit Die Roll")
            .subtitle_selectable(true)
            .build();
        let hp_roll = roll_button("Roll Hit Points");
        hit_points.add_suffix(&hp_roll);
        let show_hit_dice = Rc::new({
            let (this, hit_points, hit_dice) = (self.clone(), hit_points.clone(), hit_dice.clone());
            let classes = d.classes.clone();
            move || {
                let text: Vec<String> = classes
                    .iter()
                    .zip(hit_dice.borrow().iter())
                    .map(|(c, roll)| {
                        let die = this.rules().class(c).map_or(0, |c| c.hit_die);
                        format!("{c} {roll} (d{die})")
                    })
                    .collect();
                hit_points.set_subtitle(&format!(
                    "{} · Constitution adjusts it on the sheet",
                    text.join(", ")
                ));
            }
        });
        show_hit_dice();
        let strength = rules.adjusted(&d.race, &d.scores)[0];
        let exceptional = spin_row(
            "Exceptional Strength",
            "18/01 to 18/00",
            d.exceptional_strength.clamp(1, 100),
            1,
            100,
        );
        let exceptional_roll = roll_button("Roll Exceptional Strength");
        exceptional.add_suffix(&exceptional_roll);
        exceptional.set_visible(rules.has_exceptional_strength(&d.classes, strength));
        let combat = adw::PreferencesGroup::new();
        combat.add(&hit_points);
        combat.add(&exceptional);

        // Skills & Powers splits each score in two subabilities, up to
        // two points apart either way.
        let subabilities = adw::PreferencesGroup::builder()
            .title("Subabilities")
            .description("Raising one subability lowers the other as much")
            .visible(d.skills_and_powers)
            .build();
        let spread_rows: Vec<adw::SpinRow> = (0..6)
            .map(|a| {
                let row = adw::SpinRow::builder()
                    .numeric(true)
                    .adjustment(&gtk::Adjustment::new(
                        d.subabilities[a].into(),
                        (-SUBABILITY_SPREAD).into(),
                        SUBABILITY_SPREAD.into(),
                        1.0,
                        1.0,
                        0.0,
                    ))
                    .build();
                subabilities.add(&row);
                row
            })
            .collect();
        let spreads = Rc::new({
            let spread_rows = spread_rows.clone();
            move || -> [i32; 6] { std::array::from_fn(|a| spread_rows[a].value() as i32) }
        });
        let show_subabilities = Rc::new({
            let (this, spread_rows, spreads) = (self.clone(), spread_rows.clone(), spreads.clone());
            move || {
                let mut probe = this.0.draft.borrow().clone();
                probe.subabilities = spreads();
                let scores = this.rules().subability_scores(&probe);
                for (a, row) in spread_rows.iter().enumerate() {
                    row.set_title(&format!("{} {}", ABILITIES[a], scores[a][0]));
                    row.set_subtitle(&format!(
                        "{} {} · {} {}",
                        SUBABILITIES[a][0], scores[a][1], SUBABILITIES[a][1], scores[a][2]
                    ));
                }
            }
        });
        show_subabilities();
        for row in &spread_rows {
            let show_subabilities = show_subabilities.clone();
            row.connect_value_notify(move |_| show_subabilities());
        }

        let race = d.race.clone();
        let male = Rc::new({
            let gender = gender.clone();
            move || gender.selected() == 0
        });
        social_roll.connect_clicked({
            let (this, social_class, kit) = (self.clone(), social_class.clone(), kit.clone());
            move |_| {
                let rolled = this
                    .rules()
                    .roll_social_class(&combo_value(&kit), &mut this.0.dice.borrow_mut());
                if let Some(i) = SOCIAL_CLASSES.iter().position(|s| *s == rolled) {
                    social_class.set_selected(i as u32);
                }
            }
        });
        rolls[0].connect_clicked({
            let (this, age, race) = (self.clone(), age.clone(), race.clone());
            move |_| {
                let rolled = this.rules().roll_age(&race, &mut this.0.dice.borrow_mut());
                age.set_value(rolled.into());
            }
        });
        let roll_height = Rc::new({
            let (this, height, race, male) =
                (self.clone(), height.clone(), race.clone(), male.clone());
            move || {
                let rolled = this
                    .rules()
                    .roll_height(&race, male(), &mut this.0.dice.borrow_mut());
                height.set_value(rolled.into());
            }
        });
        let roll_weight = Rc::new({
            let (this, weight, race, male) =
                (self.clone(), weight.clone(), race.clone(), male.clone());
            move || {
                let rolled = this
                    .rules()
                    .roll_weight(&race, male(), &mut this.0.dice.borrow_mut());
                weight.set_value(rolled.into());
            }
        });
        rolls[1].connect_clicked({
            let roll_height = roll_height.clone();
            move |_| roll_height()
        });
        rolls[2].connect_clicked({
            let roll_weight = roll_weight.clone();
            move |_| roll_weight()
        });
        // Height and weight depend on the gender.
        gender.connect_selected_notify(move |_| {
            roll_height();
            roll_weight();
        });
        hp_roll.connect_clicked({
            let (this, hit_dice, classes) = (self.clone(), hit_dice.clone(), d.classes.clone());
            move |_| {
                *hit_dice.borrow_mut() = this
                    .rules()
                    .roll_hit_dice(&classes, &mut this.0.dice.borrow_mut());
                show_hit_dice();
            }
        });
        exceptional_roll.connect_clicked({
            let (this, exceptional) = (self.clone(), exceptional.clone());
            move |_| exceptional.set_value(this.0.dice.borrow_mut().roll(1, 100).into())
        });

        let next = next_button("_Next");
        next.set_sensitive(!d.name.trim().is_empty());
        name.connect_changed({
            let next = next.clone();
            move |name| next.set_sensitive(!name.text().trim().is_empty())
        });
        next.connect_clicked({
            let this = self.clone();
            move |_| {
                {
                    let mut d = this.0.draft.borrow_mut();
                    d.name = name.text().trim().to_owned();
                    d.player = player.text().trim().to_owned();
                    d.male = male();
                    d.alignment = combo_value(&alignment);
                    d.kit = combo_value(&kit);
                    d.social_class = combo_value(&social_class);
                    d.age = age.value() as u32;
                    d.height = height.value() as u32;
                    d.weight = weight.value() as u32;
                    d.hit_dice = hit_dice.borrow().clone();
                    d.exceptional_strength = exceptional.value() as u32;
                    if d.skills_and_powers {
                        d.subabilities = spreads();
                    }
                }
                this.push(this.proficiencies_page());
            }
        });
        step(
            "Details",
            &[&who, &background, &appearance, &combat, &subabilities],
            &next,
            None,
        )
    }

    /// Nonweapon proficiencies, languages and weapon proficiencies; with
    /// Skills & Powers also traits, disadvantages and a weapon
    /// specialization, all paid in character points.
    fn proficiencies_page(&self) -> adw::NavigationPage {
        let d = self.0.draft.borrow().clone();
        let rules = self.rules();
        let sp = d.skills_and_powers;
        let adjusted = rules.adjusted(&d.race, &d.scores);
        let nonweapon_slots = rules.nonweapon_slots(&d.classes, adjusted[3]);
        let weapon_slots = rules.weapon_slots(&d.classes);
        let weapon_cost = rules.weapon_cost(&d.classes);
        let native = rules.native_languages(&d);

        let nonweapon_spec = {
            let (this, draft) = (self.clone(), d.clone());
            move |name: String| {
                let cost = this.rules().nonweapon_cost(&draft, &name);
                RowSpec {
                    value: name.clone(),
                    title: name,
                    subtitle: if sp { points(cost.into()) } else { slots(cost) },
                    number: None,
                }
            }
        };
        let taken_nonweapon: Rc<RefCell<Vec<String>>> = Rc::default();
        let nonweapon = RowList::new(
            "Nonweapon Proficiencies",
            d.nonweapon_proficiencies
                .iter()
                .cloned()
                .map(&nonweapon_spec)
                .collect(),
            {
                let (this, taken) = (self.clone(), taken_nonweapon.clone());
                move || {
                    this.rules()
                        .nonweapon_proficiencies()
                        .into_iter()
                        .filter(|n| !taken.borrow().contains(n))
                        .collect()
                }
            },
            move |name| Some(nonweapon_spec(name)),
        );

        // Traits cost points; disadvantages give them.
        let trait_spec = {
            let this = self.clone();
            move |name: String| RowSpec {
                value: name.clone(),
                subtitle: points(this.rules().trait_cost(&name).into()),
                title: name,
                number: None,
            }
        };
        let taken_traits: Rc<RefCell<Vec<String>>> = Rc::default();
        let traits = RowList::new(
            "Traits",
            d.traits.iter().cloned().map(&trait_spec).collect(),
            {
                let (this, taken) = (self.clone(), taken_traits.clone());
                move || {
                    this.rules()
                        .traits()
                        .iter()
                        .map(|t| t.0.clone())
                        .filter(|n| !taken.borrow().contains(n))
                        .collect()
                }
            },
            move |name| Some(trait_spec(name)),
        );
        traits.group.set_visible(sp);
        let disadvantage_spec = {
            let this = self.clone();
            move |name: String| RowSpec {
                value: name.clone(),
                subtitle: format!(
                    "Gives {}",
                    points(this.rules().disadvantage_points(&name).into())
                ),
                title: name,
                number: None,
            }
        };
        let taken_disadvantages: Rc<RefCell<Vec<String>>> = Rc::default();
        let disadvantages = RowList::new(
            "Disadvantages",
            d.disadvantages
                .iter()
                .cloned()
                .map(&disadvantage_spec)
                .collect(),
            {
                let (this, taken) = (self.clone(), taken_disadvantages.clone());
                move || {
                    if taken.borrow().len() >= MOST_DISADVANTAGES {
                        return Vec::new();
                    }
                    this.rules()
                        .disadvantages()
                        .iter()
                        .map(|t| t.0.clone())
                        .filter(|n| !taken.borrow().contains(n))
                        .collect()
                }
            },
            move |name| Some(disadvantage_spec(name)),
        );
        disadvantages.group.set_visible(sp);

        // Native languages come with the race; others are learned through
        // proficiencies such as Modern Languages, as in the original.
        let languages = adw::PreferencesGroup::builder()
            .title("Native Languages")
            .description(
                "Other languages are learned through proficiencies such as Modern Languages",
            )
            .build();
        for language in &native {
            languages.add(&adw::ActionRow::builder().title(escape(language)).build());
        }
        if native.is_empty() {
            languages.add(
                &adw::ActionRow::builder()
                    .title("None")
                    .css_classes(["dim-label"])
                    .build(),
            );
        }
        let weapon_spec = move |name: String| RowSpec {
            value: name.clone(),
            title: name,
            subtitle: if sp {
                points(weapon_cost.into())
            } else {
                String::new()
            },
            number: None,
        };
        let taken_weapons: Rc<RefCell<Vec<String>>> = Rc::default();
        let weapons = RowList::new(
            "Weapon Proficiencies",
            d.weapon_proficiencies
                .iter()
                .cloned()
                .map(weapon_spec)
                .collect(),
            {
                let (this, taken, classes) =
                    (self.clone(), taken_weapons.clone(), d.classes.clone());
                move || {
                    this.rules()
                        .weapons(&classes)
                        .into_iter()
                        .filter(|n| !taken.borrow().contains(n))
                        .collect()
                }
            },
            move |name| Some(weapon_spec(name)),
        );

        // A fighter with Weapon specialization may specialize in one of
        // the weapons chosen.
        let specialization_cost = rules.specialization_cost(&d);
        let specialization = adw::ComboRow::builder()
            .title("Weapon Specialization")
            .subtitle(points(specialization_cost.unwrap_or(0).into()))
            .model(&gtk::StringList::new(&["None"]))
            .visible(specialization_cost.is_some())
            .build();
        let specialized = Rc::new(RefCell::new(d.specializations.first().cloned()));
        let filling = Rc::new(std::cell::Cell::new(false));
        specialization.connect_selected_notify({
            let (specialized, filling) = (specialized.clone(), filling.clone());
            move |row| {
                if !filling.get() {
                    let value = combo_value(row);
                    *specialized.borrow_mut() = (value != "None").then_some(value);
                }
            }
        });
        let points_row = adw::ActionRow::builder()
            .title("Character Points Left")
            .subtitle("Kept on the character for later levels")
            .build();
        let points_label = gtk::Label::builder()
            .css_classes(["numeric"])
            .valign(gtk::Align::Center)
            .build();
        points_row.add_suffix(&points_label);
        let points_group = adw::PreferencesGroup::builder().visible(sp).build();
        points_group.add(&specialization);
        points_group.add(&points_row);

        let next = next_button("_Next");
        let names = |list: &RowList<String>| -> Vec<String> {
            list.values().into_iter().map(|(v, _)| v).collect()
        };
        // The draft as the page has it.
        let current = Rc::new({
            let (this, nonweapon, traits, disadvantages, weapons, specialized) = (
                self.clone(),
                nonweapon.clone(),
                traits.clone(),
                disadvantages.clone(),
                weapons.clone(),
                specialized.clone(),
            );
            move || {
                let mut probe = this.0.draft.borrow().clone();
                probe.nonweapon_proficiencies = names(&nonweapon);
                probe.weapon_proficiencies = names(&weapons);
                probe.languages = native.clone();
                if sp {
                    probe.traits = names(&traits);
                    probe.disadvantages = names(&disadvantages);
                    probe.specializations = specialized
                        .borrow()
                        .iter()
                        .filter(|w| probe.weapon_proficiencies.contains(w))
                        .cloned()
                        .collect();
                }
                probe
            }
        });
        let update = Rc::new({
            let (this, nonweapon, weapons, next, current) = (
                self.clone(),
                nonweapon.clone(),
                weapons.clone(),
                next.clone(),
                current.clone(),
            );
            let (traits, disadvantages) = (traits.clone(), disadvantages.clone());
            let (taken_nonweapon, taken_weapons) = (taken_nonweapon.clone(), taken_weapons.clone());
            let (taken_traits, taken_disadvantages) =
                (taken_traits.clone(), taken_disadvantages.clone());
            let (specialization, specialized) = (specialization.clone(), specialized.clone());
            let filling = filling.clone();
            let shown: Rc<RefCell<Vec<String>>> = Rc::default();
            move || {
                *taken_nonweapon.borrow_mut() = names(&nonweapon);
                *taken_weapons.borrow_mut() = names(&weapons);
                *taken_traits.borrow_mut() = names(&traits);
                *taken_disadvantages.borrow_mut() = names(&disadvantages);
                let probe = current();
                let rules = this.rules();
                if !sp {
                    let spent = rules.nonweapon_spent(&probe);
                    let weapons_taken = probe.weapon_proficiencies.len() as u32;
                    nonweapon.group.set_description(Some(&format!(
                        "{} of {} left: the class's slots and the languages Intelligence allows",
                        nonweapon_slots as i64 - spent as i64,
                        slots(nonweapon_slots)
                    )));
                    weapons.group.set_description(Some(&format!(
                        "{} of {} left",
                        weapon_slots as i64 - weapons_taken as i64,
                        slots(weapon_slots)
                    )));
                    next.set_sensitive(spent <= nonweapon_slots && weapons_taken <= weapon_slots);
                    return;
                }
                let left = rules.proficiency_points_left(&probe);
                nonweapon.group.set_description(Some(&format!(
                    "{} of {} left for proficiencies and traits",
                    points(left),
                    rules.proficiency_points(&probe)
                )));
                let given: u32 = probe
                    .disadvantages
                    .iter()
                    .map(|d| rules.disadvantage_points(d))
                    .sum();
                disadvantages.group.set_description(Some(&format!(
                    "Up to {MOST_DISADVANTAGES}, giving at most {DISADVANTAGE_POINTS} points; \
                     these give {}",
                    given.min(DISADVANTAGE_POINTS)
                )));
                weapons.group.set_description(Some(&format!(
                    "{} each; {} come with the class, and points left from the \
                     proficiencies pay for more",
                    points(weapon_cost.into()),
                    points((weapon_slots * weapon_cost).into())
                )));
                // The specialization is one of the weapons chosen.
                let mut items = vec!["None".to_owned()];
                items.extend(probe.weapon_proficiencies.iter().cloned());
                // Only a new list of weapons rebuilds the menu: doing it
                // while the menu reports a choice would hang GTK.
                if *shown.borrow() != items {
                    filling.set(true);
                    specialization.set_model(Some(&gtk::StringList::new(
                        &items.iter().map(String::as_str).collect::<Vec<_>>(),
                    )));
                    let selected = probe
                        .specializations
                        .first()
                        .and_then(|w| items.iter().position(|i| i == w))
                        .unwrap_or(0);
                    specialization.set_selected(selected as u32);
                    filling.set(false);
                    if selected == 0 {
                        *specialized.borrow_mut() = None;
                    }
                    *shown.borrow_mut() = items;
                }
                let total = rules.points_left(&probe);
                points_label.set_label(&total.to_string().replace('-', "−"));
                next.set_sensitive(
                    left >= 0 && total >= 0 && probe.disadvantages.len() <= MOST_DISADVANTAGES,
                );
            }
        });
        for list in [&nonweapon, &traits, &disadvantages, &weapons] {
            let update = update.clone();
            list.connect_changed(move || update());
        }
        specialization.connect_selected_notify({
            let (update, filling) = (update.clone(), filling.clone());
            move |_| {
                if !filling.get() {
                    update();
                }
            }
        });
        next.connect_clicked({
            let this = self.clone();
            move |_| {
                let probe = current();
                {
                    let mut d = this.0.draft.borrow_mut();
                    if d.weapon_proficiencies != probe.weapon_proficiencies {
                        // The equipment page starts with one of each.
                        d.purchases.clear();
                    }
                    d.nonweapon_proficiencies = probe.nonweapon_proficiencies;
                    d.languages = probe.languages;
                    d.weapon_proficiencies = probe.weapon_proficiencies;
                    d.traits = probe.traits;
                    d.disadvantages = probe.disadvantages;
                    d.specializations = probe.specializations;
                }
                if this.rules().is_wizard(&this.0.draft.borrow().classes) {
                    this.push(this.spells_page());
                } else {
                    this.push(this.equipment_page());
                }
            }
        });
        step(
            "Proficiencies",
            &[
                &nonweapon.group,
                &traits.group,
                &disadvantages.group,
                &languages,
                &weapons.group,
                &points_group,
            ],
            &next,
            None,
        )
    }

    /// The wizard spells known at 1st level.
    fn spells_page(&self) -> adw::NavigationPage {
        let d = self.0.draft.borrow().clone();
        let rules = self.rules();
        let intelligence = rules.adjusted(&d.race, &d.scores)[3];
        let most = max_spells_per_level(intelligence);
        let spec = |name: String| RowSpec {
            value: name.clone(),
            title: name,
            subtitle: String::new(),
            number: None,
        };
        let taken: Rc<RefCell<Vec<String>>> = Rc::default();
        let spells = RowList::new(
            "Spells Known",
            d.spells.iter().cloned().map(spec).collect(),
            {
                let (this, taken, classes) = (self.clone(), taken.clone(), d.classes.clone());
                move || {
                    this.rules()
                        .spells(&classes)
                        .into_iter()
                        .filter(|n| !taken.borrow().contains(n))
                        .collect()
                }
            },
            move |name| Some(spec(name)),
        );
        let next = next_button("_Next");
        spells.connect_changed({
            let (spells, next, taken) = (spells.clone(), next.clone(), taken.clone());
            move || {
                let names: Vec<String> = spells.values().into_iter().map(|(v, _)| v).collect();
                let limit = if most == usize::MAX {
                    "Intelligence allows any number".to_owned()
                } else {
                    format!("Intelligence {intelligence} allows {most}")
                };
                spells.group.set_description(Some(&format!(
                    "1st-level spells the character knows. {limit}."
                )));
                next.set_sensitive(names.len() <= most);
                *taken.borrow_mut() = names;
            }
        });
        let group = spells.group.clone();
        next.connect_clicked({
            let this = self.clone();
            move |_| {
                this.0.draft.borrow_mut().spells =
                    spells.values().into_iter().map(|(v, _)| v).collect();
                this.push(this.equipment_page());
            }
        });
        step("Spells", &[&group], &next, None)
    }

    /// Starting money and what it buys; the last page.
    fn equipment_page(&self) -> adw::NavigationPage {
        let rules = self.rules();
        {
            // Like the original, start with one of each weapon the
            // character is proficient with.
            let mut d = self.0.draft.borrow_mut();
            if d.purchases.is_empty() {
                d.purchases = d
                    .weapon_proficiencies
                    .iter()
                    .filter(|w| rules.ware(w).is_some())
                    .map(|w| (w.clone(), 1))
                    .collect();
            }
        }
        let d = self.0.draft.borrow().clone();
        let gold = Rc::new(RefCell::new(d.gold));
        let money_row = adw::ActionRow::builder().title("Starting Money").build();
        let money_roll = roll_button("Roll Starting Money");
        money_row.add_suffix(&money_roll);
        let pack_price = {
            let mut probe = d.clone();
            probe.pack = true;
            probe.purchases.clear();
            rules.spent(&probe)
        };
        let pack = adw::SwitchRow::builder()
            .title("Standard Pack")
            .subtitle(format!(
                "{}: dry rations for a week, flint and steel, three torches and a wineskin · {}",
                STANDARD_PACK.0,
                money(pack_price)
            ))
            .active(d.pack)
            .build();
        let money_group = adw::PreferencesGroup::builder().title("Money").build();
        money_group.add(&money_row);
        money_group.add(&pack);
        if rules.is_wizard(&d.classes) {
            money_group.add(
                &adw::ActionRow::builder()
                    .title(cr_store::create::SPELLBOOK)
                    .subtitle("Free for wizards")
                    .build(),
            );
        }

        let spec = {
            let this = self.clone();
            move |name: String, quantity: u32| {
                let ware = this.rules().ware(&name);
                RowSpec {
                    value: name.clone(),
                    subtitle: ware.map_or(String::new(), |w| {
                        format!("{} · {}", w.part_type, money(w.cost))
                    }),
                    title: name,
                    number: Some((quantity, 1, 999)),
                }
            }
        };
        let items = RowList::new(
            "Equipment",
            d.purchases
                .iter()
                .map(|(name, n)| spec(name.clone(), *n))
                .collect(),
            {
                let this = self.clone();
                move || this.rules().wares.iter().map(|w| w.name.clone()).collect()
            },
            move |name| Some(spec(name, 1)),
        );
        items.group.set_description(Some(
            "Weapons go to the items readied, armor to the items worn, the rest to the \
             items carried",
        ));

        let banner = adw::Banner::new("");
        let create = next_button("C_reate");
        let purchases = |items: &RowList<String>| -> Vec<(String, u32)> {
            items
                .values()
                .into_iter()
                .map(|(name, n)| (name, n.unwrap_or(1)))
                .collect()
        };
        let update = Rc::new({
            let (this, items, pack, gold, money_row, create) = (
                self.clone(),
                items.clone(),
                pack.clone(),
                gold.clone(),
                money_row.clone(),
                create.clone(),
            );
            move || {
                let mut probe = this.0.draft.borrow().clone();
                probe.pack = pack.is_active();
                probe.purchases = purchases(&items);
                let total = *gold.borrow() * 100;
                let spent = this.rules().spent(&probe);
                money_row.set_subtitle(&if spent <= total {
                    format!(
                        "{} gold pieces · {} left",
                        gold.borrow(),
                        money(total - spent)
                    )
                } else {
                    format!(
                        "{} gold pieces · {} short",
                        gold.borrow(),
                        money(spent - total)
                    )
                });
                create.set_sensitive(spent <= total);
            }
        });
        items.connect_changed({
            let update = update.clone();
            move || update()
        });
        pack.connect_active_notify({
            let update = update.clone();
            move |_| update()
        });
        money_roll.connect_clicked({
            let (this, gold, update) = (self.clone(), gold.clone(), update.clone());
            move |_| {
                let classes = this.0.draft.borrow().classes.clone();
                *gold.borrow_mut() = this
                    .rules()
                    .roll_gold(&classes, &mut this.0.dice.borrow_mut());
                update();
            }
        });
        update();

        let items_group = items.group.clone();
        create.connect_clicked({
            let (this, banner) = (self.clone(), banner.clone());
            move |_| {
                let draft = {
                    let mut d = this.0.draft.borrow_mut();
                    d.gold = *gold.borrow();
                    d.pack = pack.is_active();
                    d.purchases = purchases(&items);
                    d.clone()
                };
                match (this.0.create)(&draft) {
                    Ok(()) => {
                        this.0.dialog.close();
                    }
                    Err(message) => {
                        banner.set_title(&escape(&message));
                        banner.set_revealed(true);
                    }
                }
            }
        });
        step(
            "Equipment",
            &[&money_group, &items_group],
            &create,
            Some(&banner),
        )
    }
}
