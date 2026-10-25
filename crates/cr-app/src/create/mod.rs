//! New Character and Enter Character with no toolkit in it: the
//! original's creation wizard as pages that each keep what is chosen on
//! them until Next, and the questions, warnings and messages the original
//! puts between them. The Windows app draws the pages and the questions;
//! the GNOME app's dialog takes its texts and its steps from here.
//!
//! Enter Character is the same wizard in its other mode (0x379), for a
//! character that exists on paper: the scores are typed in as they are,
//! any race and classes may be chosen, and the details page gives the
//! classes their levels.

mod details;
mod equipment;
mod proficiencies;
mod rogue;
mod spells;
mod start;
mod wild;

use std::collections::VecDeque;
use std::sync::Arc;

use cr_i18n::{tr, tr_noop, trn};
use cr_store::Installation;
use cr_store::advance::AdvanceRules;
use cr_store::create::{CreationRules, Dice, Draft, Question};
use cr_store::help::topic;
use cr_store::learn::LearnRules;
use cr_store::npc::{NpcRules, Rand};
use cr_store::settings::SETTINGS;
use cr_store::skills::SkillRules;

use crate::levels::LevelsInput;

pub use details::{
    DetailsPage, EnteredLevels, MOST_HIT_POINTS, SOCIAL_CLASSES, hit_dice_text, subability_rows,
};
pub use equipment::{EquipmentPage, MOST_ITEMS};
pub use proficiencies::{
    Choice, CombatPage, Lending, List, ListKind, PickGroup, ProficienciesPage, Unarmed,
    disadvantages_text, languages_text, nonweapon_subtitle, pick_texts, specialization_costs,
    weapon_subtitle,
};
pub use rogue::{RoguePage, RoguePicks, RogueRow};
pub use spells::{Schools, SpellsPage};
pub use start::{
    AbilitiesPage, ClassAbilitiesPage, ClassAbilityGroup, ClassPage, EnteredClassPage,
    EnteredScoresPage, GROUPS, RacePage, RacialPage, SettingPage, ability_rows, class_choice,
    race_subtitle, start_details,
};
pub use wild::{PsionicsPage, WildPage};

/// Abbreviations for the ability summaries.
pub const SHORT: [&str; 6] = [
    tr_noop!("Str"),
    tr_noop!("Dex"),
    tr_noop!("Con"),
    tr_noop!("Int"),
    tr_noop!("Wis"),
    tr_noop!("Cha"),
];

/// The most points a rogue skill takes in Enter Character, which has no
/// limits of its own (0x62b739): a skill's whole range.
pub const ENTERED_ROGUE_POINTS: u32 = 100;

/// The wizard's title in its mode.
pub fn title(entering: bool) -> &'static str {
    if entering {
        tr("Enter Character")
    } else {
        tr("New Character")
    }
}

/// "34 gp 6 sp 7 cp" for an amount in copper pieces.
pub fn money(copper: u32) -> String {
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

pub fn slots(n: u32) -> String {
    trn!("{n} slot", "{n} slots", n)
}

/// "5 points", with a true minus sign when negative.
pub fn points(n: i64) -> String {
    let count = n.to_string().replace('-', "−");
    trn!("{count} point", "{count} points", n.unsigned_abs(), count)
}

/// "Axe bonus" for "Dwarf, Axe bonus (5)": the race or class and the cost
/// are shown elsewhere.
pub fn ability_title(name: &str) -> String {
    let name = name.split_once(", ").map_or(name, |(_, rest)| rest);
    match name.rsplit_once(" (") {
        Some((title, cost)) if cost.trim_end_matches(')').parse::<i32>().is_ok() => title,
        _ => name,
    }
    .to_owned()
}

/// "Str 12 · Dex 9 · …".
pub fn scores_summary(scores: &[u32; 6]) -> String {
    SHORT
        .iter()
        .zip(scores)
        .map(|(a, s)| format!("{} {s}", tr(a)))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// What the wizard needs besides the creation rules.
pub struct Needs {
    /// Enter Character (0x44aeab) rather than New Character.
    pub entering: bool,
    pub advance: Arc<AdvanceRules>,
    pub learn: Arc<LearnRules>,
    /// The item check of the Equipment page and the name generator, when
    /// the installation's program could be read.
    pub items: Option<Arc<NpcRules>>,
    /// The Psionics and Wild Talents steps' rules, when the Expansion's
    /// were read.
    pub skills: Option<Arc<SkillRules>>,
    /// The Background… dialog's events, "NONE" first; empty when the
    /// program could not be read.
    pub backgrounds: Vec<String>,
    /// Whether the Expansion's Core Rules Vs Player's Option message is
    /// shown before the scores are rolled.
    pub core_vs_option: bool,
}

impl Needs {
    /// Reads what the wizard needs from `install`; `items` are the
    /// program's item rules when the app has them already.
    pub fn load(
        install: &Installation,
        entering: bool,
        items: Option<Arc<NpcRules>>,
        core_vs_option: bool,
    ) -> cr_store::Result<(CreationRules, Self)> {
        let rules = install.creation_rules()?;
        let advance = Arc::new(install.advance_rules()?);
        let learn = Arc::new(install.learn_rules()?);
        // Without the Expansion's psionics there is no Wild Talents step.
        let skills = rules
            .expansion
            .then(|| install.skill_rules().ok().map(Arc::new))
            .flatten();
        let items = items.or_else(|| install.npc_rules().ok().map(Arc::new));
        let needs = Self {
            entering,
            advance,
            learn,
            items,
            skills,
            backgrounds: cr_store::edit::backgrounds(install).unwrap_or_default(),
            core_vs_option,
        };
        Ok((rules, needs))
    }
}

/// What answering a prompt does.
type Then = Arc<dyn Fn(&mut Wizard) + Send + Sync>;

/// A question, warning or message the page waits on, as the original's
/// message boxes and small dialogs.
#[derive(Clone)]
pub struct Prompt {
    pub kind: PromptKind,
    pub title: String,
    pub text: String,
    yes: Option<Then>,
    no: Option<Then>,
}

/// How a prompt is answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    /// OK only; what follows runs once it closes.
    Message,
    /// Yes and No; No, or Yes, the default.
    Question { yes_default: bool },
    /// The original's IDD 245: OK refuses what was asked, Ignore allows
    /// it for good.
    Ignorable,
    /// The original's "Error Message" about an item the character may not
    /// use (0x5e43f9): Yes ignores it, No (the default) leaves it out.
    ItemError,
    /// "Add one point to any ability score" (IDD 393): one choice per
    /// ability, the score beside it, and no Cancel.
    RaiseOne { labels: [String; 6] },
    /// The Expansion's "Core Rules Vs Player's Option" window (IDD 403),
    /// with "Do not show this window again".
    CoreVsOption { off: bool },
    /// Enter Character's "All Classes..." (0x5b52d9): one class of each
    /// group, or none, by index (0 for none).
    AllClasses {
        groups: Vec<(&'static str, Vec<String>)>,
        chosen: Vec<usize>,
    },
}

/// The answer to a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Yes, OK, Ignore or Close: the button that goes on.
    Yes,
    /// No, Cancel, or OK on a warning: the one that does not.
    No,
    /// One of a choice's options, by index.
    Choice(usize),
}

impl Prompt {
    fn new(kind: PromptKind, title: &str, text: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.to_owned(),
            text: text.into(),
            yes: None,
            no: None,
        }
    }

    /// A message with OK.
    pub(crate) fn message(title: &str, text: impl Into<String>) -> Self {
        Self::new(PromptKind::Message, title, text)
    }

    /// A message with OK that runs `then` once closed.
    pub(crate) fn notice(title: &str, text: impl Into<String>, then: Then) -> Self {
        Self {
            yes: Some(then),
            ..Self::message(title, text)
        }
    }

    /// A Yes/No question, No the default as in the original.
    pub(crate) fn ask(title: &str, text: impl Into<String>, yes: Then, no: Option<Then>) -> Self {
        Self {
            yes: Some(yes),
            no,
            ..Self::new(PromptKind::Question { yes_default: false }, title, text)
        }
    }

    /// The original's "Error Message" about an item the character may not
    /// use: Yes runs `ignore`, No leaves the item out.
    pub(crate) fn item_error(text: &str, ignore: Then) -> Self {
        let text = format!("{text}\n\n{}", tr("Do you wish to ignore this message?"));
        Self {
            yes: Some(ignore),
            ..Self::new(PromptKind::ItemError, tr("Error Message"), text)
        }
    }

    /// A warning the user may ignore: Ignore runs `ignore`, OK nothing.
    pub(crate) fn ignorable(title: &str, text: &str, ignore: Then) -> Self {
        let text = format!(
            "{text}\n\n{}",
            tr(
                "If you choose to ignore this message and save this character, this message will \
                not appear for this character again."
            )
        );
        Self {
            yes: Some(ignore),
            ..Self::new(PromptKind::Ignorable, title, text)
        }
    }

    /// What answering with the first button does: Yes, OK or Ignore.
    pub fn yes_label(&self) -> &'static str {
        match self.kind {
            PromptKind::Message => tr("_OK"),
            PromptKind::Question { .. } | PromptKind::ItemError => tr("_Yes"),
            PromptKind::Ignorable => tr("_Ignore"),
            PromptKind::RaiseOne { .. } => "",
            PromptKind::CoreVsOption { .. } | PromptKind::AllClasses { .. } => tr("_Close"),
        }
    }

    /// The other button's label, if there is one.
    pub fn no_label(&self) -> Option<&'static str> {
        match self.kind {
            PromptKind::Question { .. } | PromptKind::ItemError => Some(tr("_No")),
            PromptKind::Ignorable => Some(tr("_OK")),
            _ => None,
        }
    }
}

/// What the app does for the wizard.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// "Do not show this window again" was ticked: Preferences keeps the
    /// Core Rules Vs Player's Option message off.
    HideCoreVsOption,
    /// Create: the app writes the character the draft describes, and
    /// says why it could not with [`Wizard::failed`].
    Create(Box<Draft>),
}

/// A page of the wizard and what it holds until Next.
#[derive(Clone)]
pub enum Page {
    Setting(SettingPage),
    Abilities(AbilitiesPage),
    EnteredScores(EnteredScoresPage),
    Race(RacePage),
    Racial(RacialPage),
    Class(ClassPage),
    EnteredClass(EnteredClassPage),
    ClassAbilities(ClassAbilitiesPage),
    Details(Box<DetailsPage>),
    Proficiencies(Box<ProficienciesPage>),
    Combat(Box<CombatPage>),
    Rogue(Box<RoguePage>),
    Spells(Box<SpellsPage>),
    Wild(Box<WildPage>),
    Psionics(Box<PsionicsPage>),
    Equipment(Box<EquipmentPage>),
}

impl Page {
    /// The page's title.
    pub fn title(&self, entering: bool) -> String {
        match self {
            Self::Setting(_) => title(entering).to_owned(),
            Self::Abilities(_) | Self::EnteredScores(_) => tr("Ability Scores").to_owned(),
            Self::Race(_) => tr("Race").to_owned(),
            Self::Racial(page) => page.title.clone(),
            Self::Class(_) | Self::EnteredClass(_) => tr("Class").to_owned(),
            Self::ClassAbilities(_) => tr("Class Abilities").to_owned(),
            Self::Details(_) => tr("Details").to_owned(),
            Self::Proficiencies(_) => tr("Proficiencies").to_owned(),
            Self::Combat(_) => tr("Combat").to_owned(),
            Self::Rogue(_) => tr("Rogue Skills").to_owned(),
            Self::Spells(_) => tr("Spells").to_owned(),
            Self::Wild(_) => tr("Wild Talents").to_owned(),
            Self::Psionics(_) => tr("Psionics").to_owned(),
            Self::Equipment(_) => tr("Equipment").to_owned(),
        }
    }

    /// The program help's topic for the page.
    pub fn help(&self) -> u32 {
        match self {
            Self::Setting(_) => topic::CHARACTER_MANAGEMENT,
            Self::Abilities(page) => topic::of_roll_method(page.method),
            Self::EnteredScores(_) => topic::of_roll_method(cr_store::create::RollMethod::Enter),
            Self::Race(_) | Self::Class(_) | Self::EnteredClass(_) => topic::RACE_AND_CLASS,
            Self::Racial(_) => topic::SUBRACE,
            Self::ClassAbilities(_) => topic::CLASS_ABILITIES,
            Self::Details(_) => topic::CHARACTER_INFORMATION,
            Self::Proficiencies(_) => topic::NONWEAPON_PROFICIENCIES,
            Self::Combat(_) => topic::WEAPON_PROFICIENCIES,
            Self::Rogue(_) => topic::ROGUE_SKILLS,
            Self::Spells(_) => topic::CHARACTER_SPELLS,
            Self::Wild(_) | Self::Psionics(_) => topic::SKILLS,
            Self::Equipment(_) => topic::EQUIPMENT,
        }
    }

    /// Whether Next (or Create) may be pressed.
    pub fn can_go_on(&self) -> bool {
        match self {
            Self::Setting(_) | Self::EnteredScores(_) | Self::EnteredClass(_) => true,
            Self::Abilities(page) => page.ok,
            Self::Race(page) => !page.races.is_empty(),
            Self::Racial(page) => page.ok,
            Self::Class(page) => !page.options.is_empty(),
            Self::ClassAbilities(page) => page.ok,
            Self::Details(page) => page.ok,
            Self::Proficiencies(page) => page.ok,
            Self::Combat(page) => page.ok,
            Self::Rogue(page) => page.ok,
            Self::Spells(_) | Self::Wild(_) | Self::Psionics(_) => true,
            Self::Equipment(page) => page.ok,
        }
    }
}

/// A change made on a page, or a button pressed on it.
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    /// Next, or Create on the last page.
    Next,
    // The rules and setting.
    Rules(usize),
    Setting(usize),
    // The scores.
    Method(usize),
    Roll,
    /// Swaps two rolled scores (Move Up and Move Down).
    Swap(usize, usize),
    /// A typed or divided score, or an entered one.
    Score(usize, u32),
    /// Method VIII: the dice of an ability.
    PoolDice(usize, u32),
    /// Method VI: the ability a die goes to.
    SevenDie(usize, usize),
    /// Method X's bought exceptional Strength, or Enter Character's.
    Exceptional(u32),
    // The race.
    StandardOnly(bool),
    Race(usize),
    Subrace(usize),
    // Racial and class abilities: the class (0 on the racial page), the
    // ability and whether it is ticked.
    Ability(usize, usize, bool),
    // The class.
    Class(usize),
    AllClasses,
    // Details.
    Name(String),
    RandomName,
    Player(String),
    /// The portrait's folder and file name, empty for none.
    Portrait([String; 2]),
    Gender(usize),
    Alignment(usize),
    Kit(usize),
    SocialClass(usize),
    RollSocialClass,
    Background(usize),
    RandomBackground,
    History(String),
    Age(u32),
    Height(u32),
    Weight(u32),
    RollAge,
    RollHeight,
    RollWeight,
    RollHitPoints,
    RollExceptional,
    Subability(usize, i32),
    // Enter Character's levels.
    /// "Levels…".
    OpenLevels,
    /// A change in the Levels dialog.
    Levels(LevelsInput),
    /// Close on the Levels dialog.
    CloseLevels,
    /// The hit points the rolls are changed to give.
    HitPoints(u32),
    /// The PSPs, all levels together.
    Psp(u32),
    // Prompts' own controls.
    /// "Do not show this window again".
    PromptOff(bool),
    /// "All Classes...": the class chosen in a group.
    PromptGroup(usize, usize),
    // Proficiencies and Combat.
    /// Add on a list: what was chosen.
    Add(ListKind, String),
    /// Remove on a row of a list.
    Remove(ListKind, usize),
    /// How many times a nonweapon proficiency is taken, by its row.
    Times(usize, u32),
    /// A proficiency of a pick list ticked or not: the list, the name.
    Pick(usize, usize, bool),
    /// The nonweapon slots the weapons use.
    Lent(u32),
    /// Weapon Focus, by index, "None" first.
    Focus(usize),
    /// The kit's free specialization, by index, "None" first.
    FreeSpecialization(usize),
    /// An unarmed style ticked or not.
    Unarmed(usize, bool),
    WeaponOfChoice(usize),
    Expertise(usize),
    /// Skills & Powers' weapon specialization, by index, "None" first.
    Specialization(usize),
    Shield(usize, bool),
    // Rogue skills.
    /// The points of a skill, by its row.
    RoguePoints(usize, u32),
    /// The skill the kit's bonus goes to, by index.
    RogueBonus(usize),
    /// A skill of the kit's chosen ones ticked or not.
    RogueChosen(usize, bool),
    /// A school of those to eliminate ticked or not.
    School(usize, bool),
    // Psionics and Wild Talents.
    /// The points of a rogue skill on the Skills page, by its row.
    SkillPoints(usize, i32),
    /// A kit's bonus of a level given to a skill: the level's group, the
    /// skill.
    KitBonus(usize, usize),
    /// A primary discipline: the first or second, the discipline by
    /// index, "None" first.
    Primary(usize, usize),
    /// Test for Wild Talents.
    Test,
    // Equipment.
    RollMoney,
    /// The standard pack bought or not.
    Pack(bool),
}

/// The wizard: the pages so far, the draft Next has kept, and the
/// question the page waits on.
#[derive(Clone)]
pub struct Wizard {
    rules: Arc<CreationRules>,
    needs: Arc<Needs>,
    /// What Next has kept so far.
    draft: Draft,
    dice: Dice,
    /// The name generator's and Select Randomly's stream.
    names: Rand,
    /// The pages shown; the last is in view and Back returns to the one
    /// before.
    pages: Vec<Page>,
    prompt: Option<Prompt>,
    /// Counts the prompts shown, for an app to tell one from the next.
    serial: u64,
    /// Whether the Core Rules Vs Player's Option message is still shown.
    core_vs_option: bool,
    effects: Vec<Effect>,
    /// Why the character could not be written, shown over the last page.
    pub error: Option<String>,
}

impl Wizard {
    pub fn new(rules: CreationRules, needs: Needs) -> Self {
        let mut draft = Draft::new(&SETTINGS[0]);
        if needs.entering {
            draft.entered = Some(cr_store::create::Entered::default());
            draft.scores = [cr_store::create::ENTERED_DEFAULT_SCORE; 6];
            draft.roll_method = cr_store::create::RollMethod::Enter;
        }
        let core_vs_option = needs.core_vs_option;
        let first = Page::Setting(SettingPage::new(rules.expansion));
        Self {
            rules: Arc::new(rules),
            needs: Arc::new(needs),
            draft,
            dice: Dice::default(),
            names: Rand::default(),
            pages: vec![first],
            prompt: None,
            serial: 0,
            core_vs_option,
            effects: Vec::new(),
            error: None,
        }
    }

    /// With fixed dice, for tests.
    #[cfg(test)]
    fn seeded(mut self, seed: u64) -> Self {
        self.dice = Dice::seeded(seed);
        self
    }

    pub fn rules(&self) -> &CreationRules {
        &self.rules
    }

    pub fn entering(&self) -> bool {
        self.needs.entering
    }

    pub fn title(&self) -> &'static str {
        title(self.needs.entering)
    }

    pub fn draft(&self) -> &Draft {
        &self.draft
    }

    /// The page in view.
    pub fn page(&self) -> &Page {
        self.pages.last().expect("the wizard has a page")
    }

    fn page_mut(&mut self) -> &mut Page {
        self.pages.last_mut().expect("the wizard has a page")
    }

    /// How many pages in it is, from 1.
    pub fn depth(&self) -> usize {
        self.pages.len()
    }

    /// The question or message in view.
    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    /// Which prompt is in view: each one shown has a number of its own,
    /// so that an answer to one that is gone is not taken for the next.
    pub fn prompt_serial(&self) -> u64 {
        self.serial
    }

    /// What the app should do now, once.
    pub fn take_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }

    /// Back to the page before; false on the first page, which the app
    /// leaves the wizard from.
    pub fn back(&mut self) -> bool {
        if self.prompt.is_some() {
            return true;
        }
        if self.pages.len() < 2 {
            return false;
        }
        self.pages.pop();
        self.error = None;
        true
    }

    /// The character could not be written, for `why`.
    pub fn failed(&mut self, why: String) {
        self.error = Some(why);
    }

    fn push(&mut self, page: Page) {
        self.pages.push(page);
        self.error = None;
    }

    /// Shows a prompt; its answer runs what it says.
    fn show(&mut self, prompt: Prompt) {
        self.prompt = Some(prompt);
        self.serial += 1;
    }

    /// Shows a message with OK.
    fn message(&mut self, title: &str, text: impl Into<String>) {
        self.show(Prompt::message(title, text));
    }

    /// Asks the questions in turn and runs `then` once each is answered
    /// Yes (a notice only needs OK).
    fn ask_each(&mut self, mut questions: VecDeque<Question>, then: Then) {
        let Some(question) = questions.pop_front() else {
            then(self);
            return;
        };
        let next: Then =
            Arc::new(move |w: &mut Wizard| w.ask_each(questions.clone(), then.clone()));
        self.show(if question.notice {
            Prompt::notice(&question.title, question.text, next)
        } else {
            Prompt::ask(&question.title, question.text, next, None)
        });
    }

    /// Answers the prompt in view.
    pub fn answer(&mut self, answer: Answer) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        let then = match (&prompt.kind, answer) {
            (PromptKind::RaiseOne { .. }, Answer::Choice(a)) if a < 6 => {
                self.draft.raised = Some(a);
                self.after_race();
                return;
            }
            // The original's dialog has no Cancel.
            (PromptKind::RaiseOne { .. }, _) => {
                self.show(prompt);
                return;
            }
            (PromptKind::CoreVsOption { off }, _) => {
                if *off {
                    self.core_vs_option = false;
                    self.effects.push(Effect::HideCoreVsOption);
                }
                prompt.yes.clone()
            }
            (PromptKind::AllClasses { groups, chosen }, Answer::Yes) => {
                let classes: Vec<String> = groups
                    .iter()
                    .zip(chosen)
                    .filter_map(|((_, classes), &c)| c.checked_sub(1).and_then(|c| classes.get(c)))
                    .cloned()
                    .collect();
                if !classes.is_empty() {
                    self.all_classes_chosen(classes);
                }
                return;
            }
            (PromptKind::AllClasses { .. }, _) => return,
            // A message's OK goes on, however it is closed.
            (PromptKind::Message, _) | (_, Answer::Yes) => prompt.yes.clone(),
            (_, _) => prompt.no.clone(),
        };
        if let Some(then) = then {
            then(self);
        }
    }

    /// Makes a change on the page in view, or presses one of its buttons.
    pub fn input(&mut self, input: Input) {
        if let Some(prompt) = &mut self.prompt {
            match (&mut prompt.kind, input) {
                (PromptKind::CoreVsOption { off }, Input::PromptOff(on)) => *off = on,
                (PromptKind::AllClasses { groups, chosen }, Input::PromptGroup(g, c)) => {
                    if let (Some(slot), Some((_, classes))) = (chosen.get_mut(g), groups.get(g))
                        && c <= classes.len()
                    {
                        *slot = c;
                    }
                }
                _ => {}
            }
            return;
        }
        match self.page() {
            Page::Setting(_) => self.setting_input(input),
            Page::Abilities(_) => self.abilities_input(input),
            Page::EnteredScores(_) => self.entered_scores_input(input),
            Page::Race(_) => self.race_input(input),
            Page::Racial(_) => self.racial_input(input),
            Page::Class(_) => self.class_input(input),
            Page::EnteredClass(_) => self.entered_class_input(input),
            Page::ClassAbilities(_) => self.class_abilities_input(input),
            Page::Details(_) => self.details_input(input),
            Page::Proficiencies(_) => self.proficiencies_input(input),
            Page::Combat(_) => self.combat_input(input),
            Page::Rogue(_) => self.rogue_input(input),
            Page::Spells(_) => self.spells_input(input),
            Page::Wild(_) => self.wild_input(input),
            Page::Psionics(_) => self.psionics_input(input),
            Page::Equipment(_) => self.equipment_input(input),
        }
    }

    /// The help topic of the page in view.
    pub fn help(&self) -> u32 {
        self.page().help()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The wizard on the installation `CORERULES_DIR` names, if it is set
    /// (an Expansion's), with fixed dice.
    pub(crate) fn wizard(entering: bool) -> Option<Wizard> {
        let dir = std::env::var_os("CORERULES_DIR")?;
        let install = Installation::open(std::path::Path::new(&dir)).ok()?;
        if install.is_core20() {
            return None;
        }
        let (rules, needs) = Needs::load(&install, entering, None, true).ok()?;
        Some(Wizard::new(rules, needs).seeded(7))
    }

    #[test]
    fn money_names_the_coins_there_are() {
        assert_eq!(money(3467), "34 gp 6 sp 7 cp");
        assert_eq!(money(500), "5 gp");
        assert_eq!(money(0), "Free");
    }

    #[test]
    fn points_have_a_true_minus() {
        assert_eq!(points(5), "5 points");
        assert_eq!(points(1), "1 point");
        assert_eq!(points(-3), "−3 points");
    }

    #[test]
    fn ability_titles_drop_the_owner_and_the_cost() {
        assert_eq!(ability_title("Dwarf, Axe bonus (5)"), "Axe bonus");
        assert_eq!(ability_title("Fighter, Followers (10)"), "Followers");
        assert_eq!(ability_title("Thief, Climb walls*"), "Climb walls*");
        assert_eq!(ability_title("Elf (half)"), "Elf (half)");
    }

    #[test]
    fn scores_read_in_order() {
        assert_eq!(
            scores_summary(&[12, 9, 14, 10, 8, 16]),
            "Str 12 · Dex 9 · Con 14 · Int 10 · Wis 8 · Cha 16"
        );
    }

    #[test]
    fn questions_are_asked_in_turn_and_then_runs_after_the_last_yes() {
        let Some(mut w) = wizard(false) else {
            return;
        };
        let questions: VecDeque<Question> = [
            Question::notice("Note", "Read this".into()),
            Question::new("Sure?", "Really?".into()),
        ]
        .into();
        w.ask_each(
            questions.clone(),
            Arc::new(|w: &mut Wizard| w.draft.name = "done".into()),
        );
        assert_eq!(w.prompt().unwrap().kind, PromptKind::Message);
        w.answer(Answer::No);
        assert_eq!(w.prompt().unwrap().title, "Sure?");
        w.answer(Answer::No);
        assert!(w.prompt().is_none());
        assert_eq!(w.draft.name, "");
        w.ask_each(
            questions,
            Arc::new(|w: &mut Wizard| w.draft.name = "done".into()),
        );
        w.answer(Answer::Yes);
        w.answer(Answer::Yes);
        assert_eq!(w.draft.name, "done");
    }
}
