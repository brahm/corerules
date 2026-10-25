//! The wizard's first pages: the rules and the campaign setting, the
//! ability scores, the race, the class and, with Skills & Powers, the
//! racial and class abilities; and what the details page starts with.

use std::sync::Arc;

use cr_i18n::{tr, tr_noop};
use cr_store::create::{
    ABILITIES, ALIGNMENTS, CreationRules, Dice, Draft, ENTERED_SCORES, METHOD_VI_DICE,
    METHOD_VII_POINTS, METHOD_VIII_DICE, METHOD_X_STRENGTH_STEP, RollMethod, point_table,
    roll_dice_pool, roll_seven_dice, seven_dice_scores,
};
use cr_store::settings::{SETTINGS, Setting};

use super::{
    DetailsPage, Input, Page, Prompt, PromptKind, SHORT, Wizard, ability_title, points,
    scores_summary,
};

/// The groups "All Classes..." picks one class from.
pub const GROUPS: [&str; 5] = [
    tr_noop!("Warriors"),
    tr_noop!("Rogues"),
    tr_noop!("Priests"),
    tr_noop!("Wizards"),
    tr_noop!("Psionicists"),
];

/// The rules and the campaign setting, which narrows the races and
/// classes every later page offers.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingPage {
    /// Whether the Core Rules and Player's Option are offered: Core Rules
    /// 2.0 has no Player's Option.
    pub expansion: bool,
    pub skills_and_powers: bool,
    /// The setting, in [`SETTINGS`].
    pub setting: usize,
}

impl SettingPage {
    pub fn new(expansion: bool) -> Self {
        Self {
            expansion,
            skills_and_powers: false,
            setting: 0,
        }
    }
}

/// Rolling the ability scores (New Character).
#[derive(Debug, Clone, PartialEq)]
pub struct AbilitiesPage {
    pub methods: Vec<RollMethod>,
    pub method: RollMethod,
    /// The rolled scores (Method VI's and VIII's too).
    pub scores: [u32; 6],
    /// The scores typed in or divided (Methods VII, IX and X).
    pub typed: [u32; 6],
    /// Method VIII: the dice each ability gets.
    pub pool: [u32; 6],
    /// Method VIII's scores were rolled with the dice shown.
    pub pool_rolled: bool,
    /// Methods VII and IX: the points to divide, the highest score and
    /// the 2d6 that gave them.
    pub budget: (u32, u32, u32),
    /// Method VI: the seven dice and the ability each is added to.
    pub seven: [u32; METHOD_VI_DICE],
    pub seven_to: [usize; METHOD_VI_DICE],
    /// Method X: exceptional Strength bought, in steps of 10%.
    pub exceptional: u32,
    /// What the scores' group says, and whether Next may be pressed.
    pub description: Option<String>,
    pub ok: bool,
}

impl AbilitiesPage {
    fn new(expansion: bool, dice: &mut Dice) -> Self {
        let mut page = Self {
            methods: RollMethod::offered(expansion),
            method: RollMethod::FourDice,
            scores: [10; 6],
            typed: [10; 6],
            pool: [4; 6],
            pool_rolled: false,
            budget: (METHOD_VII_POINTS, 18, 0),
            seven: [0; METHOD_VI_DICE],
            seven_to: std::array::from_fn(|d| d % 6),
            exceptional: 0,
            description: None,
            ok: true,
        };
        page.reroll(dice);
        page
    }

    /// Whether the method rolls, so that Roll Again is offered.
    pub fn rolls(&self) -> bool {
        !matches!(
            self.method,
            RollMethod::Enter
                | RollMethod::Points
                | RollMethod::PointsAndStrength
                | RollMethod::PlayersChoice
        )
    }

    /// Whether the scores are typed in or divided rather than rolled.
    pub fn typed_in(&self) -> bool {
        self.method == RollMethod::Enter || self.method.divided()
    }

    /// Method VIII: a number of dice per ability.
    pub fn pooled(&self) -> bool {
        self.method == RollMethod::DicePool
    }

    /// Player's Choice rolls the scores after the class.
    pub fn later(&self) -> bool {
        self.method == RollMethod::PlayersChoice
    }

    /// Method X's exceptional Strength is bought at Strength 18.
    pub fn exceptional_shown(&self) -> bool {
        self.method == RollMethod::PointsAndStrength && self.typed[0] == 18
    }

    /// The highest score a typed one may reach.
    pub fn highest(&self) -> u32 {
        if self.method.divided() {
            self.budget.1
        } else {
            18
        }
    }

    fn reroll(&mut self, dice: &mut Dice) {
        match self.method {
            RollMethod::Enter | RollMethod::PlayersChoice => {}
            RollMethod::Points | RollMethod::PointsAndStrength => {
                self.budget = (METHOD_VII_POINTS, 18, 0)
            }
            RollMethod::SevenDice => self.seven = roll_seven_dice(dice),
            RollMethod::RolledPoints => {
                let roll = dice.roll(2, 6);
                let (points, highest) = point_table(roll);
                self.budget = (points, highest, roll);
            }
            RollMethod::DicePool => {
                if self.pool.iter().sum::<u32>() == METHOD_VIII_DICE {
                    self.scores = roll_dice_pool(self.pool, dice);
                    self.pool_rolled = true;
                }
            }
            m => self.scores = m.roll(dice),
        }
        self.refresh();
    }

    fn refresh(&mut self) {
        let m = self.method;
        let highest = self.highest();
        for score in &mut self.typed {
            *score = (*score).clamp(3, highest);
        }
        let fitted = seven_dice_scores(&self.seven, &self.seven_to);
        if m == RollMethod::SevenDice
            && let Some(rolled) = fitted
        {
            self.scores = rolled;
        }
        let (points, highest, rolled) = self.budget;
        (self.description, self.ok) = if m == RollMethod::PlayersChoice {
            (None, true)
        } else if m == RollMethod::SevenDice {
            let fits = fitted.is_some();
            let text = if fits {
                tr("Every score starts at 8")
            } else {
                tr("No score may pass 18: give a die to another ability")
            };
            (Some(text.to_owned()), fits)
        } else if m.divided() {
            let mut total: u32 = self.typed.iter().sum();
            if self.exceptional_shown() {
                total += self.exceptional;
            }
            let left = i64::from(points) - i64::from(total);
            let text = if m == RollMethod::RolledPoints {
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
            (Some(text), left == 0)
        } else if m == RollMethod::DicePool {
            let total: u32 = self.pool.iter().sum();
            let left = i64::from(METHOD_VIII_DICE) - i64::from(total);
            let text = tr!(
                "{left} of {total} dice left · roll when all are given out",
                left,
                total = METHOD_VIII_DICE
            );
            (Some(text), left == 0 && self.pool_rolled)
        } else {
            (None, true)
        };
    }
}

/// Enter Character's scores (IDD 227 in mode 0x379): typed in from 1 to
/// 25 and kept as typed, with exceptional Strength from 0 to 100 at
/// Strength 18 whatever the class, and no points to spend.
#[derive(Debug, Clone, PartialEq)]
pub struct EnteredScoresPage {
    pub scores: [u32; 6],
    pub exceptional: u32,
}

impl EnteredScoresPage {
    pub fn exceptional_shown(&self) -> bool {
        self.scores[0] == 18
    }
}

/// The race, among those whose limits the scores meet; with Player's
/// Choice or Enter Character any race, the scores being rolled after the
/// class or kept as entered.
#[derive(Debug, Clone, PartialEq)]
pub struct RacePage {
    pub standard_only: bool,
    pub description: String,
    /// The races offered, each with its adjustments.
    pub races: Vec<(String, String)>,
    pub chosen: usize,
    pub subraces: Vec<String>,
    pub subrace: usize,
}

impl RacePage {
    /// Whether the race chosen has subraces to choose from.
    pub fn subraces_shown(&self) -> bool {
        match self.races.get(self.chosen) {
            Some((race, _)) => self.subraces.len() > 1 || self.subraces.first() != Some(race),
            None => false,
        }
    }
}

/// What a race row says under its name: its ability adjustments.
pub fn race_subtitle(rules: &CreationRules, race: &str, entering: bool) -> String {
    if entering {
        return String::new();
    }
    let race = rules.race(race);
    let mut adjustments: Vec<String> = race
        .map(|r| r.adjustments)
        .unwrap_or_default()
        .iter()
        .enumerate()
        .filter(|(_, v)| **v != 0)
        .map(|(a, v)| format!("{} {v:+}", tr(SHORT[a])))
        .collect();
    if race.is_some_and(|r| r.any_point) {
        adjustments.push(tr("+1 to any ability").to_owned());
    }
    if adjustments.is_empty() {
        tr("No ability adjustments").to_owned()
    } else {
        adjustments.join(" · ").replace('-', "−")
    }
}

/// "Fighter/Mage" with "Multi-class · d10/d4" for a class choice.
pub fn class_choice(rules: &CreationRules, classes: &[String]) -> (String, String) {
    let dice: Vec<String> = classes
        .iter()
        .filter_map(|c| rules.class(c))
        .map(|c| format!("d{}", c.hit_die))
        .collect();
    let dice = dice.join("/");
    let subtitle = if classes.len() > 1 {
        tr!("Multi-class · {dice}", dice)
    } else {
        tr!("Hit die · {dice}", dice)
    };
    (classes.join("/"), subtitle)
}

/// The title and the cost line of each ability to tick, as the original's
/// lists show them.
pub fn ability_rows(choices: &[(String, i32)]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|(name, cost)| {
            // The databases have some abilities in two versions that
            // differ only in cost (Followers (5) and (10)); those keep it.
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
            let cost = if *cost < 0 {
                tr!("Gives {points}", points = points(i64::from(-cost)))
            } else {
                points(i64::from(*cost))
            };
            (title, cost)
        })
        .collect()
}

/// Skills & Powers: racial abilities bought with the race's points,
/// starting from the subrace's package as the original does.
#[derive(Debug, Clone, PartialEq)]
pub struct RacialPage {
    pub title: String,
    pub total: u32,
    pub saved: u32,
    /// The abilities to tick; empty when the race's are fixed.
    pub choices: Vec<(String, i32)>,
    pub rows: Vec<(String, String)>,
    pub chosen: Vec<String>,
    pub description: String,
    pub ok: bool,
}

impl RacialPage {
    /// Monster races come with fixed abilities.
    pub fn fixed(&self) -> bool {
        self.total == 0 || self.choices.is_empty()
    }
}

/// The class or multi-class combination. With Player's Choice the
/// classes are those the race's best scores reach, and the scores are
/// rolled here to fit the class chosen.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassPage {
    pub description: String,
    pub options: Vec<Vec<String>>,
    pub rows: Vec<(String, String)>,
    pub chosen: usize,
    /// Player's Choice: the scores rolled for the class chosen, and what
    /// the race makes of them.
    pub choice: bool,
    pub rolled: [u32; 6],
    pub rolled_text: (String, String),
}

/// Enter Character's classes (IDD 205 in mode 0x379): every combination
/// the race may take, the classes' minimums not checked, and with "All
/// Classes..." any other, one class of each group.
#[derive(Debug, Clone, PartialEq)]
pub struct EnteredClassPage {
    pub race: String,
    pub options: Vec<Vec<String>>,
    pub rows: Vec<(String, String)>,
    pub chosen: usize,
}

/// One class's abilities on the class abilities page.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassAbilityGroup {
    pub title: String,
    pub choices: Vec<(String, i32)>,
    pub rows: Vec<(String, String)>,
    pub description: String,
}

/// Skills & Powers: class abilities bought with each class's points,
/// starting from the class's defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassAbilitiesPage {
    pub groups: Vec<ClassAbilityGroup>,
    /// What is ticked, per class.
    pub chosen: Vec<Vec<String>>,
    pub ok: bool,
}

/// Rolls what the details page starts with for `classes`, as the original
/// does when it reaches its name page; with Enter Character new classes
/// start at 1st level. Says why when the character cannot be made.
pub fn start_details(
    rules: &CreationRules,
    d: &mut Draft,
    classes: Vec<String>,
    entering: bool,
    dice: &mut Dice,
) -> Result<(), String> {
    start_details_keeping(rules, d, classes, entering, dice, None).map(|_| ())
}

/// What the rolls of the details page depend on: the race, the gender,
/// the classes, the kit and the method.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RollKey {
    race: String,
    subrace: String,
    male: bool,
    classes: Vec<String>,
    kit: String,
    method: RollMethod,
}

/// [`start_details`], which keeps the draft's rolls when they were made
/// for `rolled`, the same race, gender, classes, kit and method: Back
/// past the details page and Next again do not roll them anew. Gives
/// what the rolls are for.
pub(super) fn start_details_keeping(
    rules: &CreationRules,
    d: &mut Draft,
    classes: Vec<String>,
    entering: bool,
    dice: &mut Dice,
    rolled: Option<&RollKey>,
) -> Result<RollKey, String> {
    let changed = d.classes != classes;
    d.classes = classes;
    let race = d.race.clone();
    let alignments = rules.alignments(&d.classes);
    // Enter Character sets the alignment freely (0x5beb8f).
    let free = entering && ALIGNMENTS.contains(&d.alignment.as_str());
    if !free && !alignments.contains(&d.alignment) {
        d.alignment = alignments
            .first()
            .cloned()
            .unwrap_or_else(|| ALIGNMENTS[4].to_owned());
    }
    if !rules.kits_for(d).contains(&d.kit) {
        d.kit = "None".into();
    }
    let key = RollKey {
        race: race.clone(),
        subrace: d.subrace.clone(),
        male: d.male,
        classes: d.classes.clone(),
        kit: d.kit.clone(),
        method: d.roll_method,
    };
    if rolled != Some(&key) {
        d.social_class = rules.roll_social_class(&d.kit, dice);
        d.age = rules.roll_age(&race, dice);
        d.height = rules.roll_height(&race, d.male, dice);
        d.weight = rules.roll_weight(&race, d.male, dice);
        d.hit_dice = rules.roll_hit_dice(&d.classes, dice);
        d.psionic_roll = rules.roll_psionic(&d.classes, dice);
        // Bought with Method X; typed in with Enter Character.
        if d.roll_method != RollMethod::PointsAndStrength && !entering {
            d.exceptional_strength = dice.roll(1, 100);
        }
        d.gold = rules.roll_gold(d, dice);
    }
    if changed {
        d.reset_proficiencies();
        d.spells.clear();
        d.purchases.clear();
        d.rogue_points = [0; 8];
        d.rogue_bonus = None;
        d.rogue_chosen.clear();
        d.barred_schools_ignored = false;
        d.eliminated_schools.clear();
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
    d.languages = rules.native_languages(d);
    if entering && (changed || d.class_levels.len() != d.classes.len()) {
        rules.start_entered(d).map_err(|e| e.to_string())?;
    }
    Ok(key)
}

impl Wizard {
    fn setting_page(&mut self) -> Option<&mut SettingPage> {
        match self.page_mut() {
            Page::Setting(page) => Some(page),
            _ => None,
        }
    }

    pub(super) fn setting_input(&mut self, input: Input) {
        let Some(page) = self.setting_page() else {
            return;
        };
        match input {
            Input::Rules(i) => page.skills_and_powers = i == 1 && page.expansion,
            Input::Setting(i) if i < SETTINGS.len() => page.setting = i,
            Input::Next => {
                let (setting, sp) = (page.setting, page.skills_and_powers);
                self.draft.setting = &SETTINGS[setting];
                self.draft.skills_and_powers = sp;
                if self.entering() {
                    let page = EnteredScoresPage {
                        scores: self
                            .draft
                            .scores
                            .map(|s| s.clamp(ENTERED_SCORES.0, ENTERED_SCORES.1)),
                        exceptional: self.draft.exceptional_strength.min(100),
                    };
                    self.push(Page::EnteredScores(page));
                } else if self.rules.expansion && self.core_vs_option {
                    let mut prompt = Prompt::message(
                        tr("Core Rules Vs Player's Option"),
                        if sp {
                            tr("You are about to create a Player's Option character.")
                        } else {
                            tr("You are about to create a Core Rules character.")
                        },
                    );
                    prompt.kind = PromptKind::CoreVsOption { off: false };
                    prompt.yes = Some(Arc::new(|w: &mut Wizard| w.push_abilities()));
                    self.show(prompt);
                } else {
                    self.push_abilities();
                }
            }
            _ => {}
        }
    }

    fn push_abilities(&mut self) {
        let page = AbilitiesPage::new(self.rules.expansion, &mut self.dice);
        self.push(Page::Abilities(page));
    }

    pub(super) fn abilities_input(&mut self, input: Input) {
        let Wizard { pages, dice, .. } = self;
        let Some(Page::Abilities(page)) = pages.last_mut() else {
            return;
        };
        match input {
            Input::Method(i) => {
                if let Some(&m) = page.methods.get(i) {
                    page.method = m;
                    page.reroll(dice);
                }
            }
            Input::Roll => page.reroll(dice),
            Input::Swap(a, b) if a < 6 && b < 6 && page.method.arranged() => {
                page.scores.swap(a, b);
                page.refresh();
            }
            Input::Score(a, v) if a < 6 => {
                page.typed[a] = v;
                page.refresh();
            }
            Input::PoolDice(a, v) if a < 6 => {
                page.pool[a] = v.clamp(3, 6);
                page.pool_rolled = false;
                page.refresh();
            }
            Input::SevenDie(d, a) if d < METHOD_VI_DICE && a < 6 => {
                page.seven_to[d] = a;
                page.refresh();
            }
            Input::Exceptional(v) => {
                page.exceptional = v.min(100 / METHOD_X_STRENGTH_STEP);
                page.refresh();
            }
            Input::Next if page.ok => {
                let m = page.method;
                let values = if page.typed_in() {
                    page.typed
                } else {
                    page.scores
                };
                let exceptional = if page.exceptional_shown() {
                    page.exceptional * METHOD_X_STRENGTH_STEP
                } else {
                    0
                };
                self.draft.scores = values;
                self.draft.roll_method = m;
                if m == RollMethod::PointsAndStrength {
                    self.draft.exceptional_strength = exceptional;
                }
                self.push_race();
            }
            _ => {}
        }
    }

    pub(super) fn entered_scores_input(&mut self, input: Input) {
        let Some(Page::EnteredScores(page)) = self.pages.last_mut() else {
            return;
        };
        let (low, high) = ENTERED_SCORES;
        match input {
            Input::Score(a, v) if a < 6 => page.scores[a] = v.clamp(low, high),
            Input::Exceptional(v) => page.exceptional = v.min(100),
            Input::Next => {
                self.draft.scores = page.scores;
                self.draft.exceptional_strength = if page.exceptional_shown() {
                    page.exceptional
                } else {
                    0
                };
                self.push_race();
            }
            _ => {}
        }
    }

    fn push_race(&mut self) {
        let d = &self.draft;
        let choice = d.roll_method == RollMethod::PlayersChoice;
        let description = if choice {
            tr("The scores are rolled after the class, to fit both").to_owned()
        } else if self.entering() {
            tr!(
                "{scores} · kept as entered, with no racial adjustments",
                scores = scores_summary(&d.scores)
            )
        } else {
            tr!(
                "{scores} · races whose ability limits these scores meet",
                scores = scores_summary(&d.scores)
            )
        };
        let mut page = RacePage {
            standard_only: false,
            description,
            races: Vec::new(),
            chosen: 0,
            subraces: Vec::new(),
            subrace: 0,
        };
        let previous = d.race.clone();
        self.fill_races(&mut page, &previous);
        self.push(Page::Race(page));
    }

    /// Lists the races the standard switch and the scores allow, keeping
    /// `previous` chosen when it is still offered.
    fn fill_races(&self, page: &mut RacePage, previous: &str) {
        let d = &self.draft;
        let entering = self.entering();
        let races = if d.roll_method == RollMethod::PlayersChoice || entering {
            self.rules.all_races(d.setting, page.standard_only)
        } else {
            self.rules.races(d.setting, &d.scores, page.standard_only)
        };
        page.chosen = races.iter().position(|r| r == previous).unwrap_or(0);
        page.races = races
            .into_iter()
            .map(|r| {
                let subtitle = race_subtitle(&self.rules, &r, entering);
                (r, subtitle)
            })
            .collect();
        self.fill_subraces(page, d.setting);
    }

    fn fill_subraces(&self, page: &mut RacePage, setting: &Setting) {
        page.subraces = match page.races.get(page.chosen) {
            Some((race, _)) => self.rules.subraces(setting, race),
            None => Vec::new(),
        };
        page.subrace = 0;
    }

    pub(super) fn race_input(&mut self, input: Input) {
        let Some(Page::Race(mut page)) = self.pages.last().cloned() else {
            return;
        };
        let setting = self.draft.setting;
        match input {
            Input::StandardOnly(on) => {
                let previous = page
                    .races
                    .get(page.chosen)
                    .map(|r| r.0.clone())
                    .unwrap_or_default();
                page.standard_only = on;
                self.fill_races(&mut page, &previous);
            }
            Input::Race(i) if i < page.races.len() => {
                page.chosen = i;
                self.fill_subraces(&mut page, setting);
            }
            Input::Subrace(i) if i < page.subraces.len() => page.subrace = i,
            Input::Next => {
                let Some((race, _)) = page.races.get(page.chosen).cloned() else {
                    return;
                };
                let subrace = page
                    .subraces
                    .get(page.subrace)
                    .cloned()
                    .unwrap_or_else(|| race.clone());
                let d = &mut self.draft;
                if d.race != race || d.subrace != subrace {
                    d.racial_abilities = self.rules.racial_package(&race, &subrace);
                }
                d.race = race;
                d.subrace = subrace;
                d.raised = None;
                let any_point = self.rules.race(&d.race).is_some_and(|r| r.any_point);
                // New Character only (0x5b54d0 runs in mode 0x378).
                if any_point && !self.entering() {
                    self.raise_one();
                } else {
                    self.after_race();
                }
                return;
            }
            _ => return,
        }
        *self.page_mut() = Page::Race(page);
    }

    /// The original's "Add one point to any ability score" (IDD 393) for a
    /// race such as the Mongrelman.
    fn raise_one(&mut self) {
        let d = &self.draft;
        let scores = self.rules.adjusted(&d.race, &d.scores);
        let later = d.roll_method == RollMethod::PlayersChoice;
        // Player's Choice rolls the scores after the class.
        let labels = std::array::from_fn(|a| {
            if later {
                tr(ABILITIES[a]).to_owned()
            } else {
                format!("{} {}", tr(ABILITIES[a]), scores[a])
            }
        });
        let mut prompt = Prompt::message(
            tr("Add one point to any ability score"),
            tr(
                "The race you have selected allows you to add one point to any ability \
                score. Please select the ability score you wish to have increased.",
            ),
        );
        prompt.kind = PromptKind::RaiseOne { labels };
        self.show(prompt);
    }

    /// The page after the race: Skills & Powers racial abilities, or the
    /// class.
    pub(super) fn after_race(&mut self) {
        if self.draft.skills_and_powers {
            self.push_racial();
        } else {
            self.push_class();
        }
    }

    fn push_racial(&mut self) {
        let d = &self.draft;
        let (total, saved) = self.rules.racial_points(&d.race, &d.subrace);
        let choices = self.rules.racial_ability_choices(&d.race, &d.subrace);
        let mut page = RacialPage {
            title: tr!("{race} Abilities", race = d.subrace),
            total,
            saved,
            rows: ability_rows(&choices),
            choices,
            chosen: d.racial_abilities.clone(),
            description: String::new(),
            ok: true,
        };
        if page.fixed() {
            page.choices.clear();
            page.rows.clear();
        }
        self.refresh_racial(&mut page);
        self.push(Page::Racial(page));
    }

    fn refresh_racial(&self, page: &mut RacialPage) {
        if page.fixed() {
            page.description = tr("The race's abilities are fixed").to_owned();
            page.ok = true;
            return;
        }
        // Enter Character charges no points (0x5b8181).
        if self.entering() {
            page.description = tr("Free with Enter Character, as in the original").to_owned();
            page.ok = true;
            return;
        }
        let mut probe = self.draft.clone();
        probe.racial_abilities = page.chosen.clone();
        let left = self.rules.racial_points_left(&probe);
        page.description = tr!(
            "{left} of {total} left; up to {saved} kept for proficiencies",
            left = points(left),
            total = page.total,
            saved = page.saved
        );
        page.ok = left >= 0;
    }

    /// The names ticked once `index` of `choices` is (un)ticked, in the
    /// list's order.
    fn ticked(choices: &[(String, i32)], chosen: &[String], index: usize, on: bool) -> Vec<String> {
        choices
            .iter()
            .enumerate()
            .filter(|(i, (name, _))| {
                if *i == index {
                    on
                } else {
                    chosen.contains(name)
                }
            })
            .map(|(_, (name, _))| name.clone())
            .collect()
    }

    pub(super) fn racial_input(&mut self, input: Input) {
        let Some(Page::Racial(mut page)) = self.pages.last().cloned() else {
            return;
        };
        match input {
            Input::Ability(_, i, on) if i < page.choices.len() => {
                let names = Self::ticked(&page.choices, &page.chosen, i, on);
                let mut probe = self.draft.clone();
                probe.racial_abilities = names.clone();
                // A change the points do not cover is refused.
                let after = self.rules.racial_points_left(&probe);
                if !self.entering() && after < 0 {
                    probe.racial_abilities = page.chosen.clone();
                    let left = self.rules.racial_points_left(&probe);
                    self.not_enough_points(left - after, left);
                    return;
                }
                page.chosen = names;
                self.refresh_racial(&mut page);
                *self.page_mut() = Page::Racial(page);
            }
            Input::Next if page.ok => {
                self.draft.racial_abilities = page.chosen;
                self.push_class();
            }
            _ => {}
        }
    }

    /// The class page of the mode.
    fn push_class(&mut self) {
        if self.entering() {
            self.push_entered_class();
            return;
        }
        let d = &self.draft;
        let choice = d.roll_method == RollMethod::PlayersChoice;
        let scores = if choice {
            self.rules.best_scores(&d.race)
        } else {
            d.scores
        };
        let adjusted = self.rules.raised(&d.race, &scores, d.raised);
        let options = self
            .rules
            .class_options(d.setting, &d.race, &d.subrace, &adjusted);
        let description = if choice {
            d.race.clone()
        } else {
            tr!(
                "{race} · {scores} after racial adjustments",
                race = d.race,
                scores = scores_summary(&adjusted)
            )
        };
        let mut page = ClassPage {
            description,
            rows: options
                .iter()
                .map(|c| class_choice(&self.rules, c))
                .collect(),
            chosen: options.iter().position(|o| *o == d.classes).unwrap_or(0),
            options,
            choice,
            rolled: scores,
            rolled_text: Default::default(),
        };
        if choice {
            self.roll_for_class(&mut page);
        }
        self.push(Page::Class(page));
    }

    /// Player's Choice: 3d6 in order, fitted to the race and class.
    fn roll_for_class(&mut self, page: &mut ClassPage) {
        let Some(classes) = page.options.get(page.chosen) else {
            return;
        };
        let d = &self.draft;
        let scores = self.rules.players_choice(&d.race, classes, &mut self.dice);
        page.rolled_text = (
            scores_summary(&scores),
            tr!(
                "{scores} after racial adjustments",
                scores = scores_summary(&self.rules.raised(&d.race, &scores, d.raised))
            ),
        );
        page.rolled = scores;
    }

    pub(super) fn class_input(&mut self, input: Input) {
        let Some(Page::Class(mut page)) = self.pages.last().cloned() else {
            return;
        };
        match input {
            Input::Class(i) if i < page.options.len() => {
                page.chosen = i;
                if page.choice {
                    self.roll_for_class(&mut page);
                }
            }
            Input::Roll if page.choice => self.roll_for_class(&mut page),
            Input::Next => {
                let Some(classes) = page.options.get(page.chosen).cloned() else {
                    return;
                };
                if page.choice {
                    self.draft.scores = page.rolled;
                }
                self.classes_chosen(classes);
                return;
            }
            _ => return,
        }
        *self.page_mut() = Page::Class(page);
    }

    fn push_entered_class(&mut self) {
        let d = &self.draft;
        let mut options = self.rules.entered_class_options(d.setting, &d.race);
        if !d.classes.is_empty() && !options.contains(&d.classes) {
            options.push(d.classes.clone());
        }
        let page = EnteredClassPage {
            race: d.race.clone(),
            rows: options
                .iter()
                .map(|c| class_choice(&self.rules, c))
                .collect(),
            chosen: options.iter().position(|o| *o == d.classes).unwrap_or(0),
            options,
        };
        self.push(Page::EnteredClass(page));
    }

    pub(super) fn entered_class_input(&mut self, input: Input) {
        let Some(Page::EnteredClass(page)) = self.pages.last_mut() else {
            return;
        };
        match input {
            Input::Class(i) if i < page.options.len() => page.chosen = i,
            Input::AllClasses => {
                let groups = self.rules.classes_by_group(self.draft.setting);
                let groups: Vec<(&'static str, Vec<String>)> =
                    GROUPS.into_iter().zip(groups).collect();
                let mut prompt = Prompt::message(
                    tr("Allowed Multiclasses"),
                    tr(
                        "Select the Class or combination of Classes for your character and click \
                        on the Close button",
                    ),
                );
                prompt.kind = PromptKind::AllClasses {
                    chosen: vec![0; groups.len()],
                    groups,
                };
                self.show(prompt);
            }
            Input::Next => {
                if let Some(classes) = page.options.get(page.chosen).cloned() {
                    self.classes_chosen(classes);
                }
            }
            _ => {}
        }
    }

    /// "All Classes..." closed with `classes`: offered, and chosen.
    pub(super) fn all_classes_chosen(&mut self, classes: Vec<String>) {
        let rows = class_choice(&self.rules, &classes);
        let Some(Page::EnteredClass(page)) = self.pages.last_mut() else {
            return;
        };
        page.chosen = match page.options.iter().position(|o| *o == classes) {
            Some(at) => at,
            None => {
                page.options.push(classes);
                page.rows.push(rows);
                page.options.len() - 1
            }
        };
    }

    /// Next on a class page: what the details start with, then the class
    /// abilities or the details.
    fn classes_chosen(&mut self, classes: Vec<String>) {
        let entering = self.entering();
        match start_details_keeping(
            &self.rules,
            &mut self.draft,
            classes,
            entering,
            &mut self.dice,
            self.rolled.as_ref(),
        ) {
            Ok(key) => self.rolled = Some(key),
            Err(e) => {
                self.message(super::title(entering), e);
                return;
            }
        }
        if self.draft.skills_and_powers {
            self.push_class_abilities();
        } else {
            self.push_details();
        }
    }

    fn push_class_abilities(&mut self) {
        let d = &self.draft;
        let groups = d
            .classes
            .iter()
            .map(|class| {
                let choices = self.rules.class_ability_choices(class);
                ClassAbilityGroup {
                    title: tr!("{class} Abilities", class),
                    rows: ability_rows(&choices),
                    choices,
                    description: String::new(),
                }
            })
            .collect();
        let mut page = ClassAbilitiesPage {
            groups,
            chosen: d.class_abilities.clone(),
            ok: true,
        };
        page.chosen.resize(d.classes.len(), Vec::new());
        self.refresh_class_abilities(&mut page);
        self.push(Page::ClassAbilities(page));
    }

    fn refresh_class_abilities(&self, page: &mut ClassAbilitiesPage) {
        // Enter Character charges no points (0x5ba6f9).
        if self.entering() {
            for group in &mut page.groups {
                group.description = tr("Free with Enter Character, as in the original").to_owned();
            }
            page.ok = true;
            return;
        }
        let mut probe = self.draft.clone();
        probe.class_abilities = page.chosen.clone();
        let mut fine = true;
        for (slot, group) in page.groups.iter_mut().enumerate() {
            let total = probe
                .classes
                .get(slot)
                .and_then(|c| self.rules.class(c))
                .map_or(0, |c| c.points.max(0));
            let left = self.rules.class_points_left(&probe, slot);
            fine &= left >= 0;
            group.description = tr!(
                "{left} of {total} left; what is left goes to proficiencies",
                left = points(left),
                total
            );
        }
        page.ok = fine;
    }

    pub(super) fn class_abilities_input(&mut self, input: Input) {
        let Some(Page::ClassAbilities(mut page)) = self.pages.last().cloned() else {
            return;
        };
        match input {
            Input::Ability(slot, i, on) => {
                let Some(group) = page.groups.get(slot) else {
                    return;
                };
                if i >= group.choices.len() {
                    return;
                }
                let names = Self::ticked(&group.choices, &page.chosen[slot], i, on);
                let mut probe = self.draft.clone();
                probe.class_abilities = page.chosen.clone();
                let left = self.rules.class_points_left(&probe, slot);
                probe.class_abilities[slot] = names.clone();
                let after = self.rules.class_points_left(&probe, slot);
                if !self.entering() && after < 0 {
                    self.not_enough_points(left - after, left);
                    return;
                }
                page.chosen[slot] = names;
                self.refresh_class_abilities(&mut page);
                *self.page_mut() = Page::ClassAbilities(page);
            }
            Input::Next if page.ok => {
                let d = &mut self.draft;
                d.class_abilities = page.chosen;
                if self.rules.specialization_cost(d).is_none() {
                    d.specializations.clear();
                }
                self.push_details();
            }
            _ => {}
        }
    }

    /// An ability that costs more points than are left is refused, as
    /// the original says.
    fn not_enough_points(&mut self, cost: i64, left: i64) {
        self.message(
            tr("Not Allowed"),
            tr!(
                "The selected ability requires {cost} character points, only {left} are \
                 available.",
                cost,
                left = left.max(0)
            ),
        );
    }

    fn push_details(&mut self) {
        let page = DetailsPage::new(self);
        self.push(Page::Details(Box::new(page)));
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::wizard;
    use super::super::{Answer, Effect};
    use super::*;

    #[test]
    fn new_character_rolls_and_offers_the_races_the_scores_allow() {
        let Some(mut w) = wizard(false) else {
            return;
        };
        w.input(Input::Next);
        // The Expansion asks Core Rules or Player's Option first.
        assert!(matches!(
            w.prompt().unwrap().kind,
            PromptKind::CoreVsOption { off: false }
        ));
        w.input(Input::PromptOff(true));
        w.answer(Answer::Yes);
        assert_eq!(w.take_effects(), [Effect::HideCoreVsOption]);
        let Page::Abilities(page) = w.page() else {
            panic!("not the scores");
        };
        assert!(page.ok);
        assert!(page.scores.iter().all(|s| (3..=18).contains(s)));
        // Method VII divides 75 points.
        let vii = page
            .methods
            .iter()
            .position(|m| *m == RollMethod::Points)
            .unwrap();
        w.input(Input::Method(vii));
        let Page::Abilities(page) = w.page() else {
            panic!();
        };
        assert!(!page.ok);
        for (a, score) in [15, 12, 12, 12, 12, 12].into_iter().enumerate() {
            w.input(Input::Score(a, score));
        }
        let Page::Abilities(page) = w.page() else {
            panic!();
        };
        assert!(page.ok, "{:?}", page.description);
        w.input(Input::Next);
        assert_eq!(w.draft().scores, [15, 12, 12, 12, 12, 12]);
        let Page::Race(page) = w.page() else {
            panic!("not the race");
        };
        assert!(page.races.iter().any(|(r, _)| r == "Human"));
        assert!(page.description.starts_with("Str 15 · Dex 12"));
        // Back keeps the scores page as it was.
        assert!(w.back());
        assert!(matches!(w.page(), Page::Abilities(p) if p.typed[0] == 15));
    }

    #[test]
    fn a_human_fighter_reaches_the_details() {
        let Some(mut w) = wizard(false) else {
            return;
        };
        w.input(Input::Next);
        w.answer(Answer::Yes);
        let Page::Abilities(page) = w.page() else {
            panic!();
        };
        let vii = page
            .methods
            .iter()
            .position(|m| *m == RollMethod::Points)
            .unwrap();
        w.input(Input::Method(vii));
        for (a, score) in [17, 12, 14, 10, 10, 12].into_iter().enumerate() {
            w.input(Input::Score(a, score));
        }
        w.input(Input::Next);
        let Page::Race(page) = w.page() else {
            panic!();
        };
        let human = page.races.iter().position(|(r, _)| r == "Human").unwrap();
        w.input(Input::Race(human));
        w.input(Input::Next);
        let Page::Class(page) = w.page() else {
            panic!("not the class");
        };
        let fighter = page.options.iter().position(|o| *o == ["Fighter"]).unwrap();
        w.input(Input::Class(fighter));
        w.input(Input::Next);
        assert!(matches!(w.page(), Page::Details(_)));
        let d = w.draft();
        assert_eq!(d.classes, ["Fighter"]);
        assert!(d.age > 0 && d.height > 0 && d.weight > 0);
        assert_eq!(d.hit_dice.len(), 1);
        assert!(!d.alignment.is_empty());
    }

    #[test]
    fn skills_and_powers_buys_racial_abilities_with_the_race_points() {
        let Some(mut w) = wizard(false) else {
            return;
        };
        w.input(Input::Rules(1));
        w.input(Input::Next);
        w.answer(Answer::Yes);
        w.input(Input::Next);
        let Page::Race(page) = w.page() else {
            panic!();
        };
        let Some(elf) = page.races.iter().position(|(r, _)| r == "Elf") else {
            return;
        };
        w.input(Input::Race(elf));
        let Page::Race(page) = w.page() else {
            panic!();
        };
        let Some(high) = page.subraces.iter().position(|s| s == "High elf") else {
            return;
        };
        w.input(Input::Subrace(high));
        w.input(Input::Next);
        let Page::Racial(page) = w.page() else {
            panic!("not the racial abilities");
        };
        // The package costs 40 of the elf's 45 points.
        assert!(
            page.description.starts_with("5 points of 45 left"),
            "{}",
            page.description
        );
        assert!(page.ok);
        // Something past the 5 points left is refused.
        let dear = page
            .choices
            .iter()
            .position(|(n, cost)| *cost > 5 && !page.chosen.contains(n));
        if let Some(dear) = dear {
            let before = page.chosen.clone();
            w.input(Input::Ability(0, dear, true));
            let Page::Racial(page) = w.page() else {
                panic!();
            };
            assert_eq!(page.chosen, before);
        }
    }

    #[test]
    fn enter_character_keeps_scores_as_typed_and_offers_all_classes() {
        let Some(mut w) = wizard(true) else {
            return;
        };
        w.input(Input::Next);
        let Page::EnteredScores(page) = w.page() else {
            panic!("not the entered scores");
        };
        assert_eq!(page.scores, [12; 6]);
        w.input(Input::Score(0, 18));
        w.input(Input::Exceptional(50));
        w.input(Input::Score(1, 30));
        w.input(Input::Next);
        assert_eq!(w.draft().scores[..2], [18, 25]);
        assert_eq!(w.draft().exceptional_strength, 50);
        let Page::Race(page) = w.page() else {
            panic!();
        };
        let human = page.races.iter().position(|(r, _)| r == "Human").unwrap();
        w.input(Input::Race(human));
        w.input(Input::Next);
        assert!(matches!(w.page(), Page::EnteredClass(_)));
        w.input(Input::AllClasses);
        let PromptKind::AllClasses { groups, .. } = &w.prompt().unwrap().kind else {
            panic!("not All Classes");
        };
        let fighter = groups[0].1.iter().position(|c| c == "Fighter").unwrap();
        let thief = groups[1].1.iter().position(|c| c == "Thief").unwrap();
        w.input(Input::PromptGroup(0, fighter + 1));
        w.input(Input::PromptGroup(1, thief + 1));
        w.answer(Answer::Yes);
        let Page::EnteredClass(page) = w.page() else {
            panic!();
        };
        assert_eq!(page.options[page.chosen], ["Fighter", "Thief"]);
    }
}
