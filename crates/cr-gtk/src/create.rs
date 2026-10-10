//! The New Character dialog: the original's creation wizard as a series of
//! pages, starting from the campaign setting, which narrows the races and
//! classes every later page offers.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_store::create::{
    ABILITIES, CreationRules, Dice, Draft, RollMethod, STANDARD_PACK, max_spells_per_level,
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

    /// The campaign setting.
    fn setting_page(&self) -> adw::NavigationPage {
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
                this.0.draft.borrow_mut().setting = *chosen.borrow();
                this.push(this.abilities_page());
            }
        });
        step("New Character", &[&group], &next, None)
    }

    /// Rolling the ability scores.
    fn abilities_page(&self) -> adw::NavigationPage {
        let methods: Vec<String> = RollMethod::ALL.iter().map(|m| m.name().into()).collect();
        let method_row = combo_row("Method", &methods, RollMethod::FourDice.name());
        let method_group = adw::PreferencesGroup::new();
        method_group.add(&method_row);

        let roll = roll_button("Roll Again");
        let scores_group = adw::PreferencesGroup::builder()
            .title("Ability Scores")
            .header_suffix(&roll)
            .build();
        let scores: Rc<RefCell<[u32; 6]>> = Rc::new(RefCell::new([10; 6]));
        let method = Rc::new(RefCell::new(RollMethod::FourDice));

        // Rolled scores show as labels, moved up and down when the method
        // lets the player arrange them; typed scores use spin rows.
        let mut labels = Vec::new();
        let mut rolled_rows = Vec::new();
        let mut arrows = Vec::new();
        let entered: Vec<adw::SpinRow> = ABILITIES
            .iter()
            .map(|a| spin_row(a, "", 10, 3, 18))
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
            scores_group.add(&row);
            scores_group.add(&entered[a]);
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
            let method_row = method_row.clone();
            move || {
                let m = *method.borrow();
                method_row.set_subtitle(m.description());
                roll.set_visible(m != RollMethod::Enter);
                for a in 0..6 {
                    labels[a].set_label(&scores.borrow()[a].to_string());
                    rolled_rows[a].set_visible(m != RollMethod::Enter);
                    entered[a].set_visible(m == RollMethod::Enter);
                    arrows[a].0.set_visible(m.arranged());
                    arrows[a].1.set_visible(m.arranged());
                }
            }
        });
        let reroll = Rc::new({
            let (this, scores, method, refresh) = (
                self.clone(),
                scores.clone(),
                method.clone(),
                refresh.clone(),
            );
            move || {
                let m = *method.borrow();
                if m != RollMethod::Enter {
                    *scores.borrow_mut() = m.roll(&mut this.0.dice.borrow_mut());
                }
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
        method_row.connect_selected_notify({
            let (method, reroll) = (method.clone(), reroll.clone());
            move |row| {
                *method.borrow_mut() = RollMethod::ALL[row.selected() as usize];
                reroll();
            }
        });
        roll.connect_clicked({
            let reroll = reroll.clone();
            move |_| reroll()
        });
        reroll();

        let next = next_button("_Next");
        next.connect_clicked({
            let this = self.clone();
            move |_| {
                let values = if *method.borrow() == RollMethod::Enter {
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
                {
                    let mut d = this.0.draft.borrow_mut();
                    d.race = race;
                    d.subrace = subrace;
                }
                this.push(this.class_page());
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
                this.push(this.details_page());
            }
        });
        step("Class", &[&group], &next, None)
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
        d.languages = rules.native_languages(&race);
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
                }
                this.push(this.proficiencies_page());
            }
        });
        step(
            "Details",
            &[&who, &background, &appearance, &combat],
            &next,
            None,
        )
    }

    /// Nonweapon proficiencies, languages and weapon proficiencies.
    fn proficiencies_page(&self) -> adw::NavigationPage {
        let d = self.0.draft.borrow().clone();
        let rules = self.rules();
        let adjusted = rules.adjusted(&d.race, &d.scores);
        let nonweapon_slots = rules.nonweapon_slots(&d.classes, adjusted[3]);
        let weapon_slots = rules.weapon_slots(&d.classes);
        let native = rules.native_languages(&d.race);

        let nonweapon_spec = {
            let this = self.clone();
            move |name: String| {
                let cost = this.rules().nonweapon_cost(&name);
                RowSpec {
                    value: name.clone(),
                    title: name,
                    subtitle: slots(cost),
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
                        .iter()
                        .filter(|(n, _)| !taken.borrow().contains(n))
                        .map(|(n, _)| n.clone())
                        .collect()
                }
            },
            move |name| Some(nonweapon_spec(name)),
        );
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
        let weapon_spec = |name: String| RowSpec {
            value: name.clone(),
            title: name,
            subtitle: String::new(),
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

        let next = next_button("_Next");
        let names = |list: &RowList<String>| -> Vec<String> {
            list.values().into_iter().map(|(v, _)| v).collect()
        };
        let update = Rc::new({
            let (this, nonweapon, weapons, next) = (
                self.clone(),
                nonweapon.clone(),
                weapons.clone(),
                next.clone(),
            );
            let (taken_nonweapon, taken_weapons) = (taken_nonweapon.clone(), taken_weapons.clone());
            move || {
                *taken_nonweapon.borrow_mut() = names(&nonweapon);
                *taken_weapons.borrow_mut() = names(&weapons);
                let mut probe = this.0.draft.borrow().clone();
                probe.nonweapon_proficiencies = names(&nonweapon);
                let spent = this.rules().nonweapon_spent(&probe);
                let weapons_taken = names(&weapons).len() as u32;
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
            }
        });
        for list in [&nonweapon, &weapons] {
            let update = update.clone();
            list.connect_changed(move || update());
        }
        next.connect_clicked({
            let this = self.clone();
            let (nonweapon, weapons) = (nonweapon.clone(), weapons.clone());
            move |_| {
                {
                    let mut d = this.0.draft.borrow_mut();
                    d.nonweapon_proficiencies = names(&nonweapon);
                    d.languages = native.clone();
                    let chosen = names(&weapons);
                    if d.weapon_proficiencies != chosen {
                        // The equipment page starts with one of each.
                        d.purchases.clear();
                    }
                    d.weapon_proficiencies = chosen;
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
            &[&nonweapon.group, &languages, &weapons.group],
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
