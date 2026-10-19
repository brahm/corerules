//! The New Character dialog: the original's creation wizard as a series of
//! pages, starting from the campaign setting, which narrows the races and
//! classes every later page offers.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::{tr, tr_noop, trn};
use cr_store::create::{
    ABILITIES, CreationRules, DISADVANTAGE_POINTS, Dice, Draft, METHOD_VI_DICE, METHOD_VII_POINTS,
    METHOD_VIII_DICE, METHOD_X_STRENGTH_STEP, MOST_DISADVANTAGES, MOST_FIGHTING_STYLES,
    ROGUE_SKILLS, RollMethod, SHIELDS, STANDARD_PACK, SUBABILITIES, SUBABILITY_SPREAD, UNARMED,
    max_spells_per_level, point_table, roll_dice_pool, roll_seven_dice, seven_dice_scores,
};
use cr_store::pictures::ItemIcons;
use cr_store::settings::{SETTINGS, Setting};
use gtk::glib;

use crate::lists::{RowList, RowSpec};

/// Abbreviations for the ability summaries.
const SHORT: [&str; 6] = [
    tr_noop!("Str"),
    tr_noop!("Dex"),
    tr_noop!("Con"),
    tr_noop!("Int"),
    tr_noop!("Wis"),
    tr_noop!("Cha"),
];

const SOCIAL_CLASSES: [&str; 4] = [
    tr_noop!("Lower class"),
    tr_noop!("Lower middle class"),
    tr_noop!("Upper middle class"),
    tr_noop!("Upper class"),
];

type Create = dyn Fn(&Draft) -> Result<(), String>;

/// What a check row sets on the draft: its index and whether it is on.
type SetCheck = dyn Fn(&mut Draft, usize, bool);

struct Inner {
    rules: CreationRules,
    /// The installation, for the portraits and the items' icons.
    root: PathBuf,
    /// The items' icons, read when the Equipment step first needs them.
    icons: std::cell::OnceCell<Rc<ItemIcons>>,
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
    root: PathBuf,
    create: impl Fn(&Draft) -> Result<(), String> + 'static,
) {
    let nav = adw::NavigationView::new();
    let dialog = adw::Dialog::builder()
        .title(tr("New Character"))
        .content_width(520)
        .content_height(640)
        .child(&nav)
        .build();
    let wizard = Wizard(Rc::new(Inner {
        rules,
        root,
        icons: Default::default(),
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
    let parts: Vec<String> = [copper / 100, copper / 10 % 10, copper % 10]
        .into_iter()
        .enumerate()
        .filter(|(_, amount)| *amount > 0)
        .map(|(unit, amount)| match unit {
            0 => tr!("{amount} gp", amount),
            1 => tr!("{amount} sp", amount),
            _ => tr!("{amount} cp", amount),
        })
        .collect();
    if parts.is_empty() {
        tr("Free").into()
    } else {
        parts.join(" ")
    }
}

fn slots(n: u32) -> String {
    trn!("{n} slot", "{n} slots", n)
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
    let count = n.to_string().replace('-', "−");
    trn!("{count} point", "{count} points", n.unsigned_abs(), count)
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
                tr!("Gives {points}", points = points(i64::from(-cost)))
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
        .map(|(a, s)| format!("{} {s}", tr(a)))
        .collect::<Vec<_>>()
        .join(" · ")
}

impl Wizard {
    /// The items' icons; none when they cannot be read.
    fn icons(&self) -> Rc<ItemIcons> {
        self.0
            .icons
            .get_or_init(|| {
                let icons = cr_store::Installation::open(&self.0.root).and_then(|i| i.item_icons());
                Rc::new(icons.unwrap_or_default())
            })
            .clone()
    }

    fn rules(&self) -> &CreationRules {
        &self.0.rules
    }

    fn push(&self, page: adw::NavigationPage) {
        self.0.nav.push(&page);
    }

    /// The rules and the campaign setting.
    fn setting_page(&self) -> adw::NavigationPage {
        let rules_group = adw::PreferencesGroup::builder()
            .title(tr("Rules"))
            .description(tr("The original's Core Rules and Player's Option buttons"))
            .build();
        let skills_and_powers = Rc::new(RefCell::new(false));
        radio_rows(
            &rules_group,
            &[
                (
                    tr("Core Rules").into(),
                    tr("Player's Handbook: proficiency slots").into(),
                ),
                (
                    tr("Player's Option").into(),
                    tr(
                        "Skills & Powers: subabilities, and character points for racial and \
                     class abilities, proficiencies and traits",
                    )
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
            .title(tr("Campaign Setting"))
            .description(tr(
                "The setting limits the races and classes offered on the next pages. \
                 The original program has no settings; these follow each setting's \
                 campaign book.",
            ))
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
        let next = next_button(tr("_Next"));
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
        step(tr("New Character"), &[&rules_group, &group], &next, None)
    }

    /// Rolling the ability scores.
    fn abilities_page(&self) -> adw::NavigationPage {
        let methods = RollMethod::offered(self.rules().expansion);
        let names: Vec<String> = methods.iter().map(|m| m.name().into()).collect();
        let method_row = combo_row(tr("Method"), &names, RollMethod::FourDice.name());
        let method_group = adw::PreferencesGroup::new();
        method_group.add(&method_row);

        let roll = roll_button(tr("Roll Again"));
        let scores_group = adw::PreferencesGroup::builder()
            .title(tr("Ability Scores"))
            .header_suffix(&roll)
            .build();
        let scores: Rc<RefCell<[u32; 6]>> = Rc::new(RefCell::new([10; 6]));
        let method = Rc::new(RefCell::new(RollMethod::FourDice));
        // Points to divide and the highest score (Methods VII and IX), and
        // the 2d6 that gave them.
        let budget = Rc::new(RefCell::new((METHOD_VII_POINTS, 18, 0)));
        // Whether Method VIII's scores were rolled with the dice shown.
        let pool_rolled = Rc::new(RefCell::new(false));
        let next = next_button(tr("_Next"));

        // Rolled scores show as labels, moved up and down when the method
        // lets the player arrange them; typed or divided scores use spin
        // rows, and Method VIII has a spin row of dice per ability.
        let mut labels = Vec::new();
        let mut rolled_rows = Vec::new();
        let mut arrows = Vec::new();
        let entered: Vec<adw::SpinRow> = ABILITIES
            .iter()
            .map(|a| spin_row(tr(a), "", 10, 3, 18))
            .collect();
        let dice_rows: Vec<adw::SpinRow> = ABILITIES
            .iter()
            .map(|a| spin_row(tr(a), &trn!("{n} die", "{n} dice", 4u32), 4, 3, 6))
            .collect();
        let pool_labels: Vec<gtk::Label> = (0..6)
            .map(|_| gtk::Label::builder().css_classes(["title-4"]).build())
            .collect();
        // Method VI: the seven dice, each given to an ability.
        let seven: Rc<RefCell<[u32; METHOD_VI_DICE]>> = Rc::default();
        let ability_names: Vec<String> = ABILITIES.iter().map(|a| tr(a).to_string()).collect();
        let seven_rows: Vec<adw::ComboRow> = (0..METHOD_VI_DICE)
            .map(|d| {
                combo_row(
                    &tr!("Die {number}", number = d + 1),
                    &ability_names,
                    tr(ABILITIES[d % 6]),
                )
            })
            .collect();
        let seven_group = adw::PreferencesGroup::builder()
            .title(tr("Dice"))
            .description(tr("Each die is added to the ability chosen for it"))
            .build();
        for row in &seven_rows {
            seven_group.add(row);
        }
        // Method X: exceptional Strength bought with the points.
        let exceptional = spin_row(
            tr("Exceptional Strength"),
            tr("For warriors with Strength 18, 10% a point"),
            0,
            0,
            100 / METHOD_X_STRENGTH_STEP,
        );
        for (a, name) in ABILITIES.iter().enumerate() {
            let label = gtk::Label::builder().css_classes(["title-4"]).build();
            let up = gtk::Button::builder()
                .icon_name("go-up-symbolic")
                .tooltip_text(tr("Move Up"))
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .sensitive(a > 0)
                .build();
            let down = gtk::Button::builder()
                .icon_name("go-down-symbolic")
                .tooltip_text(tr("Move Down"))
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .sensitive(a < 5)
                .build();
            let row = adw::ActionRow::builder().title(tr(name)).build();
            row.add_suffix(&label);
            row.add_suffix(&up);
            row.add_suffix(&down);
            dice_rows[a].add_prefix(&pool_labels[a]);
            scores_group.add(&row);
            scores_group.add(&entered[a]);
            scores_group.add(&dice_rows[a]);
            if a == 0 {
                scores_group.add(&exceptional);
            }
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
            let (seven, seven_rows, seven_group, exceptional) = (
                seven.clone(),
                seven_rows.clone(),
                seven_group.clone(),
                exceptional.clone(),
            );
            move || {
                let m = *method.borrow();
                let typed = m == RollMethod::Enter || m.divided();
                let pool = m == RollMethod::DicePool;
                let choice = m == RollMethod::PlayersChoice;
                method_row.set_subtitle(m.description());
                roll.set_visible(!matches!(
                    m,
                    RollMethod::Enter
                        | RollMethod::Points
                        | RollMethod::PointsAndStrength
                        | RollMethod::PlayersChoice
                ));
                scores_group.set_visible(!choice);
                seven_group.set_visible(m == RollMethod::SevenDice);
                if m == RollMethod::SevenDice {
                    let assignment: [usize; METHOD_VI_DICE] =
                        std::array::from_fn(|d| seven_rows[d].selected() as usize);
                    for (d, row) in seven_rows.iter().enumerate() {
                        row.set_subtitle(&tr!("Rolled {roll}", roll = seven.borrow()[d]));
                    }
                    if let Some(rolled) = seven_dice_scores(&seven.borrow(), &assignment) {
                        *scores.borrow_mut() = rolled;
                    }
                }
                exceptional.set_visible(
                    m == RollMethod::PointsAndStrength && entered[0].value() as u32 == 18,
                );
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
                    dice_rows[a].set_subtitle(&trn!("{n} die", "{n} dice", n));
                    pool_labels[a].set_label(&if *pool_rolled.borrow() {
                        scores.borrow()[a].to_string()
                    } else {
                        "–".to_owned()
                    });
                }
                let ok = if choice {
                    true
                } else if m == RollMethod::SevenDice {
                    let assignment: [usize; METHOD_VI_DICE] =
                        std::array::from_fn(|d| seven_rows[d].selected() as usize);
                    let fits = seven_dice_scores(&seven.borrow(), &assignment).is_some();
                    scores_group.set_description(Some(if fits {
                        tr("Every score starts at 8")
                    } else {
                        tr("No score may pass 18: give a die to another ability")
                    }));
                    fits
                } else if m.divided() {
                    let mut total: u32 = entered.iter().map(|e| e.value() as u32).sum();
                    if exceptional.is_visible() {
                        total += exceptional.value() as u32;
                    }
                    let left = points as i64 - total as i64;
                    let description = if m == RollMethod::RolledPoints {
                        tr!(
                            "2d6 rolled {roll}: {left} of {points} points left · highest score {highest}",
                            roll = rolled,
                            left,
                            points,
                            highest
                        )
                    } else {
                        tr!(
                            "{left} of {points} points left · highest score {highest}",
                            left,
                            points,
                            highest
                        )
                    };
                    scores_group.set_description(Some(&description));
                    left == 0
                } else if pool {
                    let total: u32 = dice_rows.iter().map(|e| e.value() as u32).sum();
                    let left = METHOD_VIII_DICE as i64 - total as i64;
                    scores_group.set_description(Some(&tr!(
                        "{left} of {total} dice left · roll when all are given out",
                        left,
                        total = METHOD_VIII_DICE
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
            let seven = seven.clone();
            move || {
                let m = *method.borrow();
                let mut dice = this.0.dice.borrow_mut();
                match m {
                    RollMethod::Enter | RollMethod::PlayersChoice => {}
                    RollMethod::Points | RollMethod::PointsAndStrength => {
                        *budget.borrow_mut() = (METHOD_VII_POINTS, 18, 0)
                    }
                    RollMethod::SevenDice => *seven.borrow_mut() = roll_seven_dice(&mut dice),
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
        for row in entered.iter().chain([&exceptional]) {
            let refresh = refresh.clone();
            row.connect_value_notify(move |_| refresh());
        }
        for row in &seven_rows {
            let refresh = refresh.clone();
            row.connect_selected_notify(move |_| refresh());
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
                {
                    let mut d = this.0.draft.borrow_mut();
                    d.scores = values;
                    d.roll_method = m;
                    if m == RollMethod::PointsAndStrength {
                        d.exceptional_strength = if exceptional.is_visible() {
                            exceptional.value() as u32 * METHOD_X_STRENGTH_STEP
                        } else {
                            0
                        };
                    }
                }
                this.push(this.race_page());
            }
        });
        step(
            tr("Ability Scores"),
            &[&method_group, &scores_group, &seven_group],
            &next,
            None,
        )
    }

    /// The race, among those whose limits the scores meet; with Player's
    /// Choice any race, the scores being rolled after the class.
    fn race_page(&self) -> adw::NavigationPage {
        let (setting, scores, choice) = {
            let d = self.0.draft.borrow();
            (
                d.setting,
                d.scores,
                d.roll_method == RollMethod::PlayersChoice,
            )
        };
        let standard = adw::SwitchRow::builder()
            .title(tr("Standard Races Only"))
            .subtitle(tr(
                "Leave out the monster races, as the original's switch does",
            ))
            .build();
        let options_group = adw::PreferencesGroup::new();
        options_group.add(&standard);
        let races_group = adw::PreferencesGroup::builder()
            .title(tr("Race"))
            .description(if choice {
                tr("The scores are rolled after the class, to fit both").to_owned()
            } else {
                tr!(
                    "{scores} · races whose ability limits these scores meet",
                    scores = scores_summary(&scores)
                )
            })
            .build();
        let subrace_group = adw::PreferencesGroup::new();
        let subrace_row = adw::ComboRow::builder().title(tr("Subrace")).build();
        subrace_group.add(&subrace_row);
        let next = next_button(tr("_Next"));
        let chosen: Rc<RefCell<String>> = Rc::default();
        let rows: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::default();
        let empty = adw::ActionRow::builder()
            .title(tr("No race allows these scores"))
            .subtitle(tr("Go back and roll again"))
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
                let races = if choice {
                    this.rules().all_races(setting, standard_only)
                } else {
                    this.rules().races(setting, &scores, standard_only)
                };
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
                            .map(|(a, v)| format!("{} {v:+}", tr(SHORT[a])))
                            .collect();
                        let subtitle = if adjustments.is_empty() {
                            tr("No ability adjustments").to_owned()
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
            tr("Race"),
            &[&options_group, &races_group, &subrace_group],
            &next,
            None,
        )
    }

    /// The class or multi-class combination. With Player's Choice the
    /// classes are those the race's best scores reach, and the scores are
    /// rolled here to fit the class chosen.
    fn class_page(&self) -> adw::NavigationPage {
        let (setting, race, scores, previous, choice) = {
            let d = self.0.draft.borrow();
            let choice = d.roll_method == RollMethod::PlayersChoice;
            let scores = if choice {
                self.rules().best_scores(&d.race)
            } else {
                d.scores
            };
            (d.setting, d.race.clone(), scores, d.classes.clone(), choice)
        };
        let adjusted = self.rules().adjusted(&race, &scores);
        let options = self.rules().class_options(setting, &race, &adjusted);
        let group = adw::PreferencesGroup::builder()
            .title(tr("Class"))
            .description(if choice {
                race.clone()
            } else {
                tr!(
                    "{race} · {scores} after racial adjustments",
                    race,
                    scores = scores_summary(&adjusted)
                )
            })
            .build();
        let choices: Vec<(String, String)> = options
            .iter()
            .map(|classes| {
                let dice: Vec<String> = classes
                    .iter()
                    .filter_map(|c| self.rules().class(c))
                    .map(|c| format!("d{}", c.hit_die))
                    .collect();
                let dice = dice.join("/");
                let subtitle = if classes.len() > 1 {
                    tr!("Multi-class · {dice}", dice)
                } else {
                    tr!("Hit die · {dice}", dice)
                };
                (classes.join("/"), subtitle)
            })
            .collect();
        let selected = options.iter().position(|o| *o == previous).unwrap_or(0);
        let chosen = Rc::new(RefCell::new(selected));
        // Player's Choice: 3d6 in order, fitted to the race and class.
        let roll = roll_button(tr("Roll Again"));
        let scores_group = adw::PreferencesGroup::builder()
            .title(tr("Ability Scores"))
            .header_suffix(&roll)
            .visible(choice && !options.is_empty())
            .build();
        let scores_row = adw::ActionRow::new();
        scores_group.add(&scores_row);
        let rolled: Rc<RefCell<[u32; 6]>> = Rc::new(RefCell::new(scores));
        let reroll = Rc::new({
            let (this, options, chosen, rolled, scores_row, race) = (
                self.clone(),
                options.clone(),
                chosen.clone(),
                rolled.clone(),
                scores_row.clone(),
                race.clone(),
            );
            move || {
                let Some(classes) = options.get(*chosen.borrow()) else {
                    return;
                };
                let scores =
                    this.rules()
                        .players_choice(&race, classes, &mut this.0.dice.borrow_mut());
                scores_row.set_title(&scores_summary(&scores));
                scores_row.set_subtitle(&tr!(
                    "{scores} after racial adjustments",
                    scores = scores_summary(&this.rules().adjusted(&race, &scores))
                ));
                *rolled.borrow_mut() = scores;
            }
        });
        radio_rows(&group, &choices, selected, {
            let (chosen, reroll) = (chosen.clone(), reroll.clone());
            move |i| {
                *chosen.borrow_mut() = i;
                if choice {
                    reroll();
                }
            }
        });
        roll.connect_clicked({
            let reroll = reroll.clone();
            move |_| reroll()
        });
        if choice {
            reroll();
        }
        let next = next_button(tr("_Next"));
        if options.is_empty() {
            group.add(
                &adw::ActionRow::builder()
                    .title(tr("No class allows these scores"))
                    .subtitle(tr("Go back and choose another race or roll again"))
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
                if choice {
                    this.0.draft.borrow_mut().scores = *rolled.borrow();
                }
                this.start_details(classes);
                if this.0.draft.borrow().skills_and_powers {
                    this.push(this.class_abilities_page());
                } else {
                    this.push(this.details_page());
                }
            }
        });
        step(tr("Class"), &[&group, &scores_group], &next, None)
    }

    /// Skills & Powers: racial abilities bought with the race's points,
    /// starting from the subrace's package as the original does.
    fn racial_page(&self) -> adw::NavigationPage {
        let d = self.0.draft.borrow().clone();
        let rules = self.rules();
        let (total, saved) = rules.racial_points(&d.race, &d.subrace);
        let choices = rules.racial_ability_choices(&d.race, &d.subrace);
        let group = adw::PreferencesGroup::builder()
            .title(tr("Racial Abilities"))
            .build();
        let next = next_button(tr("_Next"));
        let chosen = Rc::new(RefCell::new(d.racial_abilities.clone()));
        let update = Rc::new({
            let (this, group, next, chosen) =
                (self.clone(), group.clone(), next.clone(), chosen.clone());
            move || {
                let mut probe = this.0.draft.borrow().clone();
                probe.racial_abilities = chosen.borrow().clone();
                let left = this.rules().racial_points_left(&probe);
                group.set_description(Some(&tr!(
                    "{left} of {total} left; up to {saved} kept for proficiencies",
                    left = points(left),
                    total,
                    saved
                )));
                next.set_sensitive(left >= 0);
            }
        });
        if total == 0 || choices.is_empty() {
            // Monster races come with fixed abilities.
            group.set_description(Some(tr("The race's abilities are fixed")));
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
                        .title(tr("None"))
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
        step(
            &tr!("{race} Abilities", race = d.subrace),
            &[&group],
            &next,
            None,
        )
    }

    /// Skills & Powers: class abilities bought with each class's points,
    /// starting from the class's defaults.
    fn class_abilities_page(&self) -> adw::NavigationPage {
        let d = self.0.draft.borrow().clone();
        let next = next_button(tr("_Next"));
        let chosen = Rc::new(RefCell::new(d.class_abilities.clone()));
        let groups: Vec<adw::PreferencesGroup> = d
            .classes
            .iter()
            .map(|class| {
                adw::PreferencesGroup::builder()
                    .title(escape(&tr!("{class} Abilities", class)))
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
                    group.set_description(Some(&tr!(
                        "{left} of {total} left; what is left goes to proficiencies",
                        left = points(left),
                        total
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
        step(tr("Class Abilities"), &groups, &next, None)
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
        if d.roll_method != RollMethod::PointsAndStrength {
            d.exceptional_strength = dice.roll(1, 100);
        }
        d.gold = rules.roll_gold(&d.classes, &mut dice);
        if changed {
            d.nonweapon_proficiencies.clear();
            d.weapon_proficiencies.clear();
            d.spells.clear();
            d.purchases.clear();
            d.learned_languages.clear();
            d.rogue_points = [0; 8];
            d.weapon_of_choice = None;
            d.expertise = None;
            d.unarmed.clear();
            d.shields = [false; 4];
            d.armor_proficiencies.clear();
            d.fighting_styles.clear();
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
        let name = adw::EntryRow::builder()
            .title(tr("Name"))
            .text(&d.name)
            .build();
        let player = adw::EntryRow::builder()
            .title(tr("Player"))
            .text(&d.player)
            .build();
        let genders = [tr("Male").to_owned(), tr("Female").to_owned()];
        let gender = combo_row(tr("Gender"), &genders, &genders[usize::from(!d.male)]);
        let who = adw::PreferencesGroup::builder()
            .title(escape(&format!("{} {}", d.race, d.class_title())))
            .build();
        who.add(&name);
        who.add(&player);
        who.add(&gender);
        let (portrait_row, portrait) =
            crate::picture::portrait_row(self.0.root.clone(), d.race.clone(), d.portrait.clone());
        who.add(&portrait_row);

        let alignment = combo_row(tr("Alignment"), &rules.alignments(&d.classes), &d.alignment);
        let kit = combo_row(
            tr("Kit"),
            &rules.kits(&d.race, &d.subrace, &d.classes),
            &d.kit,
        );
        // The combo row shows the classes translated; the draft keeps them
        // in English, by position.
        let social: Vec<String> = SOCIAL_CLASSES.iter().map(|s| tr(s).to_owned()).collect();
        let social_class = combo_row(tr("Social Class"), &social, tr(&d.social_class));
        let social_roll = roll_button(tr("Roll Social Class"));
        social_class.add_suffix(&social_roll);
        let background = adw::PreferencesGroup::new();
        background.add(&alignment);
        background.add(&kit);
        background.add(&social_class);

        let age = spin_row(tr("Age"), tr("Years"), d.age, 1, 9999);
        let height = spin_row(tr("Height"), tr("Inches"), d.height, 1, 999);
        let weight = spin_row(tr("Weight"), tr("Pounds"), d.weight, 1, 9999);
        let rolls = [
            roll_button(tr("Roll Age")),
            roll_button(tr("Roll Height")),
            roll_button(tr("Roll Weight")),
        ];
        for (row, button) in [&age, &height, &weight].iter().zip(&rolls) {
            row.add_suffix(button);
        }
        let appearance = adw::PreferencesGroup::builder()
            .title(tr("Appearance"))
            .build();
        appearance.add(&age);
        appearance.add(&height);
        appearance.add(&weight);

        let hit_dice = Rc::new(RefCell::new(d.hit_dice.clone()));
        let hit_points = adw::ActionRow::builder()
            .title(tr("Hit Die Roll"))
            .subtitle_selectable(true)
            .build();
        let hp_roll = roll_button(tr("Roll Hit Points"));
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
                hit_points.set_subtitle(&tr!(
                    "{rolls} · Constitution adjusts it on the sheet",
                    rolls = text.join(", ")
                ));
            }
        });
        show_hit_dice();
        let strength = rules.adjusted(&d.race, &d.scores)[0];
        let exceptional = spin_row(
            tr("Exceptional Strength"),
            tr("18/01 to 18/00"),
            d.exceptional_strength.clamp(1, 100),
            1,
            100,
        );
        let exceptional_roll = roll_button(tr("Roll Exceptional Strength"));
        exceptional.add_suffix(&exceptional_roll);
        exceptional.set_visible(rules.has_exceptional_strength(&d.classes, strength));
        if d.roll_method == RollMethod::PointsAndStrength {
            // Bought with the ability points.
            exceptional.set_sensitive(false);
            exceptional.set_subtitle(tr("Bought with the ability points"));
            exceptional_roll.set_visible(false);
        }
        let combat = adw::PreferencesGroup::new();
        combat.add(&hit_points);
        combat.add(&exceptional);

        // Skills & Powers splits each score in two subabilities, up to
        // two points apart either way.
        let subabilities = adw::PreferencesGroup::builder()
            .title(tr("Subabilities"))
            .description(tr("Raising one subability lowers the other as much"))
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
                    row.set_title(&format!("{} {}", tr(ABILITIES[a]), scores[a][0]));
                    row.set_subtitle(&format!(
                        "{} {} · {} {}",
                        tr(SUBABILITIES[a][0]),
                        scores[a][1],
                        tr(SUBABILITIES[a][1]),
                        scores[a][2]
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

        let next = next_button(tr("_Next"));
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
                    d.portrait = portrait.borrow().clone();
                    d.male = male();
                    d.alignment = combo_value(&alignment);
                    d.kit = combo_value(&kit);
                    if let Some(class) = SOCIAL_CLASSES.get(social_class.selected() as usize) {
                        d.social_class = (*class).to_owned();
                    }
                    d.age = age.value() as u32;
                    d.height = height.value() as u32;
                    d.weight = weight.value() as u32;
                    d.hit_dice = hit_dice.borrow().clone();
                    if d.roll_method != RollMethod::PointsAndStrength {
                        d.exceptional_strength = exceptional.value() as u32;
                    }
                    if d.skills_and_powers {
                        d.subabilities = spreads();
                    }
                }
                this.push(this.proficiencies_page());
            }
        });
        step(
            tr("Details"),
            &[&who, &background, &appearance, &combat, &subabilities],
            &next,
            None,
        )
    }

    /// Nonweapon proficiencies, languages and weapon proficiencies; with
    /// Skills & Powers also traits and disadvantages, all paid in
    /// character points.
    fn proficiencies_page(&self) -> adw::NavigationPage {
        let d = self.0.draft.borrow().clone();
        let rules = self.rules();
        let sp = d.skills_and_powers;
        let adjusted = rules.adjusted(&d.race, &d.scores);
        let nonweapon_slots = rules.nonweapon_slots(&d.classes, adjusted[3]);
        let weapon_slots = rules.weapon_slots(&d.classes);
        let weapon_cost = rules.weapon_cost(&d.classes);
        let native = rules.native_languages(&d);

        // A proficiency may be taken more than once: more slots with the
        // Core Rules, more ranks at 1 point each with Skills & Powers.
        let nonweapon_spec = {
            let (this, draft) = (self.clone(), d.clone());
            move |name: String, times: u32| {
                let cost = this.rules().nonweapon_cost(&draft, &name);
                RowSpec {
                    value: name.clone(),
                    title: name,
                    subtitle: if sp {
                        tr!("{cost}, 1 more a further rank", cost = points(cost.into()))
                    } else {
                        tr!("{slots} each time", slots = slots(cost))
                    },
                    number: Some((times, 1, 10)),
                }
            }
        };
        let taken_nonweapon: Rc<RefCell<Vec<String>>> = Rc::default();
        let nonweapon = RowList::new(
            tr("Nonweapon Proficiencies"),
            rules
                .nonweapon_ranks(&d)
                .into_iter()
                .map(|(name, times)| nonweapon_spec(name, times))
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
            move |name| Some(nonweapon_spec(name, 1)),
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
            tr("Traits"),
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
                subtitle: tr!(
                    "Gives {points}",
                    points = points(this.rules().disadvantage_points(&name).into())
                ),
                title: name,
                number: None,
            }
        };
        let taken_disadvantages: Rc<RefCell<Vec<String>>> = Rc::default();
        let disadvantages = RowList::new(
            tr("Disadvantages"),
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
            .title(tr("Native Languages"))
            .description(tr(
                "Other languages are learned through proficiencies such as Modern Languages",
            ))
            .build();
        for language in &native {
            languages.add(&adw::ActionRow::builder().title(escape(language)).build());
        }
        if native.is_empty() {
            languages.add(
                &adw::ActionRow::builder()
                    .title(tr("None"))
                    .css_classes(["dim-label"])
                    .build(),
            );
        }
        // One language for each slot or rank of Modern Languages and the
        // like.
        let language_spec = |name: String| RowSpec {
            value: name.clone(),
            title: name,
            subtitle: String::new(),
            number: None,
        };
        let taken_languages: Rc<RefCell<Vec<String>>> = Rc::default();
        let language_room = Rc::new(std::cell::Cell::new(0usize));
        let learned = RowList::new(
            tr("Learned Languages"),
            d.learned_languages
                .iter()
                .cloned()
                .map(language_spec)
                .collect(),
            {
                let (this, taken, room, native) = (
                    self.clone(),
                    taken_languages.clone(),
                    language_room.clone(),
                    native.clone(),
                );
                move || {
                    if taken.borrow().len() >= room.get() {
                        return Vec::new();
                    }
                    this.rules()
                        .languages()
                        .iter()
                        .filter(|l| !taken.borrow().contains(l) && !native.contains(l))
                        .cloned()
                        .collect()
                }
            },
            move |name| Some(language_spec(name)),
        );
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
            tr("Weapon Proficiencies"),
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

        let points_row = adw::ActionRow::builder()
            .title(tr("Character Points Left"))
            .subtitle(tr("Kept on the character for later levels"))
            .build();
        let points_label = gtk::Label::builder()
            .css_classes(["numeric"])
            .valign(gtk::Align::Center)
            .build();
        points_row.add_suffix(&points_label);
        let points_group = adw::PreferencesGroup::builder().visible(sp).build();
        points_group.add(&points_row);

        let next = next_button(tr("_Next"));
        let names = |list: &RowList<String>| -> Vec<String> {
            list.values().into_iter().map(|(v, _)| v).collect()
        };
        // Each proficiency as many times as taken.
        let repeated = |list: &RowList<String>| -> Vec<String> {
            list.values()
                .into_iter()
                .flat_map(|(v, n)| std::iter::repeat_n(v, n.unwrap_or(1) as usize))
                .collect()
        };
        // The draft as the page has it.
        let current = Rc::new({
            let (this, nonweapon, traits, disadvantages, weapons, learned) = (
                self.clone(),
                nonweapon.clone(),
                traits.clone(),
                disadvantages.clone(),
                weapons.clone(),
                learned.clone(),
            );
            move || {
                let mut probe = this.0.draft.borrow().clone();
                probe.nonweapon_proficiencies = repeated(&nonweapon);
                probe.weapon_proficiencies = names(&weapons);
                probe.languages = native.clone();
                probe.learned_languages = names(&learned);
                if sp {
                    probe.traits = names(&traits);
                    probe.disadvantages = names(&disadvantages);
                    // The combat options keep only weapons still taken.
                    let kept = |w: &Option<String>| {
                        w.clone().filter(|w| probe.weapon_proficiencies.contains(w))
                    };
                    probe.weapon_of_choice = kept(&probe.weapon_of_choice);
                    probe.expertise = kept(&probe.expertise);
                    let weapons = probe.weapon_proficiencies.clone();
                    probe.specializations.retain(|w| weapons.contains(w));
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
            let (learned, taken_languages, language_room) = (
                learned.clone(),
                taken_languages.clone(),
                language_room.clone(),
            );
            move || {
                *taken_nonweapon.borrow_mut() = names(&nonweapon);
                *taken_weapons.borrow_mut() = names(&weapons);
                *taken_traits.borrow_mut() = names(&traits);
                *taken_disadvantages.borrow_mut() = names(&disadvantages);
                *taken_languages.borrow_mut() = names(&learned);
                let probe = current();
                let rules = this.rules();
                let room = rules.language_slots(&probe);
                language_room.set(room);
                let languages_left = room as i64 - probe.learned_languages.len() as i64;
                learned.group.set_description(Some(&tr!(
                    "{left} of {room} left: one for each slot or rank of Modern \
                     Languages and the like",
                    left = languages_left,
                    room
                )));
                if !sp {
                    let spent = rules.nonweapon_spent(&probe);
                    let weapons_taken = probe.weapon_proficiencies.len() as u32;
                    nonweapon.group.set_description(Some(&tr!(
                        "{left} of {slots} left: the class's slots and the languages Intelligence allows",
                        left = nonweapon_slots as i64 - spent as i64,
                        slots = slots(nonweapon_slots)
                    )));
                    weapons.group.set_description(Some(&tr!(
                        "{left} of {slots} left",
                        left = weapon_slots as i64 - weapons_taken as i64,
                        slots = slots(weapon_slots)
                    )));
                    next.set_sensitive(
                        spent <= nonweapon_slots
                            && weapons_taken <= weapon_slots
                            && languages_left >= 0,
                    );
                    return;
                }
                let left = rules.proficiency_points_left(&probe);
                nonweapon.group.set_description(Some(&tr!(
                    "{left} of {total} left for proficiencies and traits",
                    left = points(left),
                    total = rules.proficiency_points(&probe)
                )));
                let given: u32 = probe
                    .disadvantages
                    .iter()
                    .map(|d| rules.disadvantage_points(d))
                    .sum();
                disadvantages.group.set_description(Some(&tr!(
                    "Up to {most}, giving at most {limit} points; these give {given}",
                    most = MOST_DISADVANTAGES,
                    limit = DISADVANTAGE_POINTS,
                    given = given.min(DISADVANTAGE_POINTS)
                )));
                weapons.group.set_description(Some(&tr!(
                    "{cost} each; {free} come with the class, and points left from the \
                     proficiencies pay for more",
                    cost = points(weapon_cost.into()),
                    free = points(rules.weapon_points(&probe.classes).into())
                )));
                let total = rules.points_left(&probe);
                points_label.set_label(&total.to_string().replace('-', "−"));
                next.set_sensitive(
                    left >= 0
                        && total >= 0
                        && languages_left >= 0
                        && probe.disadvantages.len() <= MOST_DISADVANTAGES,
                );
            }
        });
        learned.connect_changed({
            let update = update.clone();
            move || update()
        });
        for list in [&nonweapon, &traits, &disadvantages, &weapons] {
            let update = update.clone();
            list.connect_changed(move || update());
        }
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
                    d.learned_languages = probe.learned_languages;
                    d.weapon_proficiencies = probe.weapon_proficiencies;
                    d.traits = probe.traits;
                    d.disadvantages = probe.disadvantages;
                    d.specializations = probe.specializations;
                    d.weapon_of_choice = probe.weapon_of_choice;
                    d.expertise = probe.expertise;
                }
                if sp {
                    this.push(this.combat_page());
                } else {
                    this.after_combat();
                }
            }
        });
        step(
            tr("Proficiencies"),
            &[
                &nonweapon.group,
                &traits.group,
                &disadvantages.group,
                &languages,
                &learned.group,
                &weapons.group,
                &points_group,
            ],
            &next,
            None,
        )
    }

    /// Skills & Powers: weapon of choice, expertise and specialization,
    /// unarmed combat, shield and armor proficiencies and fighting styles,
    /// paid from the points the proficiencies leave, as the original's
    /// Specialization, Shield and Armor and Fighting Styles dialogs.
    fn combat_page(&self) -> adw::NavigationPage {
        let d = self.0.draft.borrow().clone();
        let rules = self.rules();
        let classes = d.classes.clone();
        let state = Rc::new(RefCell::new(d.clone()));
        let next = next_button(tr("_Next"));
        let points_label = gtk::Label::builder()
            .css_classes(["numeric"])
            .valign(gtk::Align::Center)
            .build();
        let points_row = adw::ActionRow::builder()
            .title(tr("Character Points Left"))
            .subtitle(tr("Kept on the character for later levels"))
            .build();
        points_row.add_suffix(&points_label);
        let points_group = adw::PreferencesGroup::new();
        points_group.add(&points_row);
        let update = Rc::new({
            let (this, state, next, points_label, points_group) = (
                self.clone(),
                state.clone(),
                next.clone(),
                points_label.clone(),
                points_group.clone(),
            );
            move || {
                let probe = state.borrow().clone();
                let rules = this.rules();
                let left = rules.points_left(&probe);
                points_label.set_label(&left.to_string().replace('-', "−"));
                let problem = rules.combat_problem(&probe);
                points_group.set_description(problem.as_deref());
                next.set_sensitive(left >= 0 && problem.is_none());
            }
        });

        // A weapon of choice, expertise and specialization among the
        // weapons taken.
        // The first item is "None"; the others are the weapons.
        let mut weapon_items = vec![tr("None").to_owned()];
        weapon_items.extend(d.weapon_proficiencies.iter().cloned());
        let weapon_row = |title: &str, subtitle: String, chosen: &Option<String>| {
            let row = combo_row(
                title,
                &weapon_items,
                chosen.as_deref().unwrap_or(&weapon_items[0]),
            );
            row.set_subtitle(&subtitle);
            row
        };
        let choice = weapon_row(
            tr("Weapon of Choice"),
            points(rules.weapon_of_choice_cost(&classes).into()),
            &d.weapon_of_choice,
        );
        let expertise = weapon_row(
            tr("Weapon Expertise"),
            tr!(
                "{cost}, 1 less in the weapon of choice",
                cost = points(
                    rules
                        .expertise_cost(&Draft {
                            expertise: None,
                            ..d.clone()
                        })
                        .into()
                )
            ),
            &d.expertise,
        );
        let specialization_cost = rules.specialization_cost(&d);
        let specialization = weapon_row(
            tr("Weapon Specialization"),
            points(specialization_cost.unwrap_or(0).into()),
            &d.specializations.first().cloned(),
        );
        specialization.set_visible(specialization_cost.is_some());
        let picked = |row: &adw::ComboRow| (row.selected() > 0).then(|| combo_value(row));
        for (row, field) in [(&choice, 0), (&expertise, 1), (&specialization, 2)] {
            let (state, update) = (state.clone(), update.clone());
            row.connect_selected_notify(move |row| {
                {
                    let mut probe = state.borrow_mut();
                    let value = picked(row);
                    match field {
                        0 => probe.weapon_of_choice = value,
                        1 => probe.expertise = value,
                        _ => probe.specializations = value.into_iter().collect(),
                    }
                }
                update();
            });
        }
        let weapons = adw::PreferencesGroup::builder()
            .title(tr("Weapons"))
            .description(if specialization_cost.is_none() && classes.iter().any(|c| c == "Fighter") {
                tr("Only a single-classed fighter with Weapon specialization may specialize at 1st level")
            } else {
                tr("Among the weapon proficiencies taken")
            })
            .build();
        weapons.add(&choice);
        weapons.add(&expertise);
        weapons.add(&specialization);

        // Check rows for the unarmed styles and the shields.
        let checks = |group: &adw::PreferencesGroup,
                      names: &[&str],
                      cost: u32,
                      active: &dyn Fn(usize) -> bool,
                      set: Rc<SetCheck>| {
            for (i, name) in names.iter().enumerate() {
                let check = gtk::CheckButton::builder()
                    .valign(gtk::Align::Center)
                    .active(active(i))
                    .build();
                let row = adw::ActionRow::builder()
                    .title(*name)
                    .subtitle(points(cost.into()))
                    .activatable_widget(&check)
                    .build();
                row.add_prefix(&check);
                let (state, update, set) = (state.clone(), update.clone(), set.clone());
                check.connect_toggled(move |c| {
                    set(&mut state.borrow_mut(), i, c.is_active());
                    update();
                });
                group.add(&row);
            }
        };
        let unarmed_cost = rules.unarmed_cost(&classes);
        let unarmed = adw::PreferencesGroup::builder()
            .title(tr("Unarmed Combat"))
            .description(tr("Specialization in an unarmed style, for fighters"))
            .visible(unarmed_cost.is_some())
            .build();
        checks(
            &unarmed,
            &UNARMED,
            unarmed_cost.unwrap_or(0),
            &|i| d.unarmed.iter().any(|u| u == UNARMED[i]),
            Rc::new(|probe: &mut Draft, i, on| {
                probe.unarmed.retain(|u| u != UNARMED[i]);
                if on {
                    probe.unarmed.push(UNARMED[i].to_owned());
                }
            }),
        );
        let shields = adw::PreferencesGroup::builder()
            .title(tr("Shield Proficiencies"))
            .build();
        checks(
            &shields,
            &SHIELDS,
            rules.shield_cost(&classes),
            &|i| d.shields[i],
            Rc::new(|probe: &mut Draft, i, on| probe.shields[i] = on),
        );

        // Armor and fighting styles, added from lists.
        let armor_cost = rules.shield_cost(&classes);
        let armor_spec = move |name: String| RowSpec {
            value: name.clone(),
            title: name,
            subtitle: points(armor_cost.into()),
            number: None,
        };
        let taken_armor: Rc<RefCell<Vec<String>>> = Rc::default();
        let armor = RowList::new(
            tr("Armor Proficiencies"),
            d.armor_proficiencies
                .iter()
                .cloned()
                .map(armor_spec)
                .collect(),
            {
                let (this, taken) = (self.clone(), taken_armor.clone());
                move || {
                    this.rules()
                        .armors()
                        .into_iter()
                        .filter(|a| !taken.borrow().contains(a))
                        .collect()
                }
            },
            move |name| Some(armor_spec(name)),
        );
        let style_spec = {
            let (this, classes) = (self.clone(), classes.clone());
            move |name: String| RowSpec {
                subtitle: points(this.rules().style_cost(&classes, &name).unwrap_or(0).into()),
                value: name.clone(),
                title: name,
                number: None,
            }
        };
        let taken_styles: Rc<RefCell<Vec<String>>> = Rc::default();
        let styles = RowList::new(
            tr("Fighting Styles"),
            d.fighting_styles.iter().cloned().map(&style_spec).collect(),
            {
                let (this, taken, classes) = (self.clone(), taken_styles.clone(), classes.clone());
                move || {
                    if taken.borrow().len() >= MOST_FIGHTING_STYLES {
                        return Vec::new();
                    }
                    this.rules()
                        .styles(&classes)
                        .into_iter()
                        .filter(|s| !taken.borrow().contains(s))
                        .collect()
                }
            },
            move |name| Some(style_spec(name)),
        );
        styles.group.set_description(Some(tr(
            "The styles the class groups may learn; the \u{201c}(+2)\u{201d} ones are \
             specializations in them",
        )));
        let names = |list: &RowList<String>| -> Vec<String> {
            list.values().into_iter().map(|(v, _)| v).collect()
        };
        for (list, taken, field) in [(&armor, &taken_armor, 0), (&styles, &taken_styles, 1)] {
            let (list2, taken, state, update) =
                (list.clone(), taken.clone(), state.clone(), update.clone());
            list.connect_changed(move || {
                let values = names(&list2);
                *taken.borrow_mut() = values.clone();
                if field == 0 {
                    state.borrow_mut().armor_proficiencies = values;
                } else {
                    state.borrow_mut().fighting_styles = values;
                }
                update();
            });
        }
        update();
        next.connect_clicked({
            let this = self.clone();
            move |_| {
                let probe = state.borrow().clone();
                {
                    let mut d = this.0.draft.borrow_mut();
                    d.weapon_of_choice = probe.weapon_of_choice;
                    d.expertise = probe.expertise;
                    d.specializations = probe.specializations;
                    d.unarmed = probe.unarmed;
                    d.shields = probe.shields;
                    d.armor_proficiencies = probe.armor_proficiencies;
                    d.fighting_styles = probe.fighting_styles;
                }
                this.after_combat();
            }
        });
        step(
            tr("Combat"),
            &[
                &weapons,
                &unarmed,
                &shields,
                &armor.group,
                &styles.group,
                &points_group,
            ],
            &next,
            None,
        )
    }

    /// The page after the proficiencies and combat options: rogue skills
    /// for a character with any, then spells for a wizard, then equipment.
    fn after_combat(&self) {
        let d = self.0.draft.borrow().clone();
        if !self.rules().rogue_skills(&d).is_empty() {
            self.push(self.rogue_page());
        } else {
            self.after_rogue_skills();
        }
    }

    fn after_rogue_skills(&self) {
        if self.rules().is_wizard(&self.0.draft.borrow().classes) {
            self.push(self.spells_page());
        } else {
            self.push(self.equipment_page());
        }
    }

    /// The discretionary points added to the rogue skills, with the scores
    /// they give after the race's and Dexterity's adjustments.
    fn rogue_page(&self) -> adw::NavigationPage {
        let d = self.0.draft.borrow().clone();
        let rules = self.rules();
        let (total, most) = rules.rogue_points(&d);
        let skills = rules.rogue_skills(&d);
        let group = adw::PreferencesGroup::builder()
            .title(tr("Rogue Skills"))
            .build();
        let next = next_button(tr("_Next"));
        let rows: Vec<(usize, adw::SpinRow)> = skills
            .iter()
            .map(|&(_, skill)| {
                let row = spin_row(
                    tr(ROGUE_SKILLS[skill].0),
                    "",
                    d.rogue_points[skill],
                    0,
                    most,
                );
                group.add(&row);
                (skill, row)
            })
            .collect();
        let current = Rc::new({
            let (this, rows) = (self.clone(), rows.clone());
            move || {
                let mut probe = this.0.draft.borrow().clone();
                probe.rogue_points = [0; 8];
                for (skill, row) in &rows {
                    probe.rogue_points[*skill] = row.value() as u32;
                }
                probe
            }
        });
        let update = Rc::new({
            let (this, rows, group, next, current) = (
                self.clone(),
                rows.clone(),
                group.clone(),
                next.clone(),
                current.clone(),
            );
            move || {
                let probe = current();
                let rules = this.rules();
                for (skill, row) in &rows {
                    let (score, unarmored) = rules.rogue_skill_score(&probe, *skill);
                    row.set_subtitle(&tr!(
                        "{score}% · {unarmored}% without armor",
                        score,
                        unarmored
                    ));
                }
                let left = rules.rogue_points_left(&probe);
                group.set_description(Some(&tr!(
                    "{left} of {total} left, at most {most} a skill; the race and Dexterity \
                     adjust the scores",
                    left = points(left),
                    total,
                    most
                )));
                next.set_sensitive(left >= 0);
            }
        });
        for (_, row) in &rows {
            let update = update.clone();
            row.connect_value_notify(move |_| update());
        }
        update();
        next.connect_clicked({
            let this = self.clone();
            move |_| {
                let probe = current();
                this.0.draft.borrow_mut().rogue_points = probe.rogue_points;
                this.after_rogue_skills();
            }
        });
        step(tr("Rogue Skills"), &[&group], &next, None)
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
            tr("Spells Known"),
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
        let next = next_button(tr("_Next"));
        spells.connect_changed({
            let (spells, next, taken) = (spells.clone(), next.clone(), taken.clone());
            move || {
                let names: Vec<String> = spells.values().into_iter().map(|(v, _)| v).collect();
                let description = if most == usize::MAX {
                    tr("1st-level spells the character knows. Intelligence allows any number.")
                        .to_owned()
                } else {
                    tr!(
                        "1st-level spells the character knows. Intelligence {intelligence} allows {most}.",
                        intelligence,
                        most
                    )
                };
                spells.group.set_description(Some(&description));
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
        step(tr("Spells"), &[&group], &next, None)
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
        let money_row = adw::ActionRow::builder()
            .title(tr("Starting Money"))
            .build();
        let money_roll = roll_button(tr("Roll Starting Money"));
        money_row.add_suffix(&money_roll);
        let pack_price = {
            let mut probe = d.clone();
            probe.pack = true;
            probe.purchases.clear();
            rules.spent(&probe)
        };
        let pack = adw::SwitchRow::builder()
            .title(tr("Standard Pack"))
            .subtitle(tr!(
                "{pack}: dry rations for a week, flint and steel, three torches and a wineskin · {price}",
                pack = STANDARD_PACK.0,
                price = money(pack_price)
            ))
            .active(d.pack)
            .build();
        let money_group = adw::PreferencesGroup::builder().title(tr("Money")).build();
        money_group.add(&money_row);
        money_group.add(&pack);
        if rules.is_wizard(&d.classes) {
            money_group.add(
                &adw::ActionRow::builder()
                    .title(cr_store::create::SPELLBOOK)
                    .subtitle(tr("Free for wizards"))
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
            tr("Equipment"),
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
        items.show_icons(self.icons());
        items.group.set_description(Some(tr(
            "Weapons go to the items readied, armor to the items worn, the rest to the \
             items carried",
        )));

        let banner = adw::Banner::new("");
        let create = next_button(tr("C_reate"));
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
                let gold = *gold.borrow();
                money_row.set_subtitle(&if spent <= total {
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
            tr("Equipment"),
            &[&money_group, &items_group],
            &create,
            Some(&banner),
        )
    }
}
