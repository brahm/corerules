//! New Character and Enter Character, as the Mac app draws them: each
//! page of `cr_app::create` described as a form ([`crate::form`]), as the
//! Windows app's New Character page draws it, its questions as prompts,
//! and Create writing the character.

use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::create::{
    AbilitiesPage, Answer, ClassAbilitiesPage, ClassPage, CombatPage, DetailsPage, Effect,
    EnteredClassPage, EnteredLevels, EnteredScoresPage, EquipmentPage, Input, ListKind,
    MOST_HIT_POINTS, Needs, Page, PickGroup, ProficienciesPage, RacePage, RacialPage, RoguePage,
    SOCIAL_CLASSES, SettingPage, SpellsPage, WildPage, Wizard, none_label,
};
use cr_app::levels::{ExperienceForm, LevelsForm, LevelsInput, MOST_EXPERIENCE, MOST_LEVEL};
use cr_app::skills::{MOST_POINTS as MOST_SKILL_POINTS, SkillsForm};
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::create::{ABILITIES, ENTERED_SCORES, METHOD_VI_DICE, METHOD_X_STRENGTH_STEP};
use cr_store::settings::SETTINGS;

use crate::form::{Acts, Builder, FormDialog, FormPage, FormRow, FormValue, ListInputs};
use crate::{CoreError, PromptInfo};

/// New Character or Enter Character, open in a sheet.
#[derive(uniffi::Object)]
pub struct WizardSession {
    install: Arc<Installation>,
    wizard: Mutex<Wizard>,
    /// What the controls of the form last described do.
    acts: Mutex<Acts<Input>>,
    /// "Do not show this window again" was ticked and answered.
    core_vs_option_off: Mutex<bool>,
    /// The file of the character Create wrote.
    created: Mutex<Option<String>>,
}

impl WizardSession {
    pub(crate) fn open(
        install: Arc<Installation>,
        entering: bool,
        core_vs_option: bool,
    ) -> Result<Arc<Self>, CoreError> {
        let (rules, needs) = Needs::load(&install, entering, None, core_vs_option)?;
        Ok(Arc::new(Self {
            install,
            wizard: Mutex::new(Wizard::new(rules, needs)),
            acts: Mutex::default(),
            core_vs_option_off: Mutex::new(false),
            created: Mutex::new(None),
        }))
    }

    fn wizard(&self) -> MutexGuard<'_, Wizard> {
        self.wizard.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Does what the wizard asks of the app.
    fn settle(&self, wizard: &mut Wizard) {
        for effect in wizard.take_effects() {
            match effect {
                Effect::HideCoreVsOption => {
                    *self
                        .core_vs_option_off
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = true;
                }
                Effect::Create(draft) => {
                    match self.install.create_character(wizard.rules(), &draft) {
                        Ok(file) => {
                            *self.created.lock().unwrap_or_else(|e| e.into_inner()) = Some(file);
                        }
                        Err(e) => wizard.failed(e.to_string()),
                    }
                }
            }
        }
    }

    fn input(&self, input: Input) {
        let mut wizard = self.wizard();
        wizard.input(input);
        self.settle(&mut wizard);
    }
}

#[uniffi::export]
impl WizardSession {
    /// The program help's topic of the page in view.
    pub fn help(&self) -> u32 {
        self.wizard().help()
    }

    /// The wizard's title: New Character or Enter Character.
    pub fn title(&self) -> String {
        self.wizard().title().to_owned()
    }

    /// The title of the page in view.
    pub fn page_title(&self) -> String {
        let wizard = self.wizard();
        wizard.page().title(wizard.entering())
    }

    /// The page in view, as a form.
    pub fn form(&self) -> FormPage {
        let wizard = self.wizard();
        let mut form = Builder::new();
        let dialog = describe(&mut form, &wizard, &self.install);
        if let Some(error) = &wizard.error {
            form.group("", "").banner(error, true);
        }
        let (groups, acts) = form.finish();
        *self.acts.lock().unwrap_or_else(|e| e.into_inner()) = acts;
        FormPage { groups, dialog }
    }

    /// Control `id` of the form last described changed to `value`.
    pub fn act(&self, id: u32, value: FormValue) {
        let input = self
            .acts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .input(id, &value);
        if let Some(input) = input {
            self.input(input);
        }
    }

    /// How many pages in the page in view is, from 1: Back is offered
    /// past the first.
    pub fn depth(&self) -> u32 {
        self.wizard().depth() as u32
    }

    /// Whether Next, or Create, may be pressed.
    pub fn can_go_on(&self) -> bool {
        let wizard = self.wizard();
        wizard.prompt().is_none() && wizard.page().can_go_on()
    }

    /// Whether the page in view is the last, whose button is Create.
    pub fn last(&self) -> bool {
        matches!(self.wizard().page(), Page::Equipment(_))
    }

    /// The question or message in view.
    pub fn prompt(&self) -> Option<PromptInfo> {
        self.wizard().prompt().map(PromptInfo::of)
    }

    /// "Do not show this window again", on the Core Rules Vs Player's
    /// Option window.
    pub fn prompt_off(&self, off: bool) {
        self.input(Input::PromptOff(off));
    }

    /// All Classes…: the class chosen in group `group`, "None" 0.
    pub fn prompt_group(&self, group: u32, index: u32) {
        self.input(Input::PromptGroup(group as usize, index as usize));
    }

    /// Next, or Create on the last page.
    pub fn next(&self) {
        self.input(Input::Next);
    }

    /// Back.
    pub fn back(&self) {
        self.wizard().back();
    }

    /// Answers the prompt in view: Yes, OK or Close, else No.
    pub fn answer(&self, yes: bool) {
        let mut wizard = self.wizard();
        wizard.answer(if yes { Answer::Yes } else { Answer::No });
        self.settle(&mut wizard);
    }

    /// Answers the prompt in view with one of its choices.
    pub fn answer_choice(&self, index: u32) {
        let mut wizard = self.wizard();
        wizard.answer(Answer::Choice(index as usize));
        self.settle(&mut wizard);
    }

    /// The file of the character Create wrote, once it has.
    pub fn created(&self) -> Option<String> {
        self.created
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Whether this is Enter Character.
    pub fn entering(&self) -> bool {
        self.wizard().entering()
    }

    /// Whether the Core Rules Vs Player's Option window was turned off, for
    /// the app to keep in its preferences.
    pub fn core_vs_option_off(&self) -> bool {
        *self
            .core_vs_option_off
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }
}

/// The wizard's lists' inputs.
const LISTS: ListInputs<Input> = ListInputs {
    add: Input::Add,
    remove: Input::Remove,
    times: |_, row, n| Input::Times(row, n),
};

/// Describes the page in view in `form`; the Levels dialog, when it is
/// open over it.
fn describe(
    form: &mut Builder<Input>,
    wizard: &Wizard,
    install: &Installation,
) -> Option<FormDialog> {
    match wizard.page() {
        Page::Setting(page) => setting(form, page),
        Page::Abilities(page) => abilities(form, page),
        Page::EnteredScores(page) => entered_scores(form, page),
        Page::Race(page) => race(form, page),
        Page::Racial(page) => racial(form, page),
        Page::Class(page) => class(form, page),
        Page::EnteredClass(page) => entered_class(form, page),
        Page::ClassAbilities(page) => class_abilities(form, page),
        Page::Details(page) => {
            return details(form, page, install, &wizard.draft().race);
        }
        Page::Proficiencies(page) => proficiencies(form, page),
        Page::Combat(page) => combat(form, page),
        Page::Rogue(page) => rogue(form, page),
        Page::Spells(page) => spells(form, page),
        Page::Wild(page) => wild(form, page),
        Page::Psionics(page) => skills(form, &page.form),
        Page::Equipment(page) => equipment(form, page),
    }
    None
}

fn setting(form: &mut Builder<Input>, page: &SettingPage) {
    // Core Rules 2.0 has no Player's Option.
    if page.expansion {
        let rules_text = if page.skills_and_powers {
            tr(
                "Skills & Powers: subabilities, and character points for racial and \
                class abilities, proficiencies and traits",
            )
        } else {
            tr("Player's Handbook: proficiency slots")
        };
        form.group(
            tr("Rules"),
            tr("The original's Core Rules and Player's Option buttons"),
        )
        .radio(
            &[
                (tr("Core Rules").to_owned(), String::new()),
                (tr("Player's Option").to_owned(), rules_text.to_owned()),
            ],
            Some(usize::from(page.skills_and_powers)),
            Input::Rules,
        );
    }
    let settings: Vec<(String, String)> = SETTINGS
        .iter()
        .map(|s| (tr(s.name).to_owned(), tr(s.description).to_owned()))
        .collect();
    form.group(
        tr("Campaign Setting"),
        tr(
            "The setting limits the races and classes offered on the next pages. \
            The original program has no settings; these follow each setting's \
            campaign book.",
        ),
    )
    .radio(&settings, Some(page.setting), Input::Setting);
}

fn abilities(form: &mut Builder<Input>, page: &AbilitiesPage) {
    let names: Vec<String> = page
        .methods
        .iter()
        .map(|m| tr(m.name()).to_owned())
        .collect();
    let methods = page.methods.clone();
    form.group(tr("Method"), tr(page.method.description()))
        .choice(
            (tr("Method"), ""),
            names,
            page.methods.iter().position(|m| *m == page.method),
            Vec::new(),
            move |i| Input::Method(i.min(methods.len().saturating_sub(1))),
        );
    if page.later() {
        return;
    }
    form.group(
        tr("Ability Scores"),
        page.description.as_deref().unwrap_or_default(),
    );
    if page.rolls() {
        let roll = form.roll(tr("Roll Again"), Input::Roll);
        form.heading_button(roll);
    }
    for (a, ability) in ABILITIES.iter().enumerate() {
        let name = tr(ability);
        if page.typed_in() {
            form.number(
                (name, ""),
                page.typed[a].into(),
                (3, page.highest().into()),
                Vec::new(),
                move |v| Input::Score(a, v as u32),
            );
        } else if page.pooled() {
            let score = if page.pool_rolled {
                page.scores[a].to_string()
            } else {
                "–".to_owned()
            };
            // The dice of the ability, and the score they rolled.
            form.number(
                (name, &score),
                page.pool[a].into(),
                (3, 6),
                Vec::new(),
                move |v| Input::PoolDice(a, v as u32),
            );
        } else {
            let buttons = if page.method.arranged() {
                vec![
                    form.button(
                        tr("Move Up"),
                        "arrow.up",
                        a > 0,
                        Input::Swap(a, a.saturating_sub(1)),
                    ),
                    form.button(
                        tr("Move Down"),
                        "arrow.down",
                        a < 5,
                        Input::Swap(a, (a + 1).min(5)),
                    ),
                ]
            } else {
                Vec::new()
            };
            form.row(FormRow::Value {
                title: name.to_owned(),
                subtitle: String::new(),
                value: page.scores[a].to_string(),
                buttons,
            });
        }
        if a == 0 && page.exceptional_shown() {
            form.number(
                (
                    tr("Exceptional Strength"),
                    tr("For warriors with Strength 18, 10% a point"),
                ),
                page.exceptional.into(),
                (0, (100 / METHOD_X_STRENGTH_STEP).into()),
                Vec::new(),
                |v| Input::Exceptional(v as u32),
            );
        }
    }
    if page.method == cr_store::create::RollMethod::SevenDice {
        let ability_names: Vec<String> = ABILITIES.iter().map(|a| tr(a).to_owned()).collect();
        form.group(
            tr("Dice"),
            tr("Each die is added to the ability chosen for it"),
        );
        for d in 0..METHOD_VI_DICE {
            form.choice(
                (
                    &tr!("Die {number}", number = d + 1),
                    &tr!("Rolled {roll}", roll = page.seven[d]),
                ),
                ability_names.clone(),
                Some(page.seven_to[d]),
                Vec::new(),
                move |a| Input::SevenDie(d, a),
            );
        }
    }
}

fn entered_scores(form: &mut Builder<Input>, page: &EnteredScoresPage) {
    form.group(
        tr("Set your ability scores"),
        tr("The scores are kept as entered, with no racial adjustments"),
    );
    for (a, ability) in ABILITIES.iter().enumerate() {
        form.number(
            (tr(ability), ""),
            page.scores[a].into(),
            (ENTERED_SCORES.0.into(), ENTERED_SCORES.1.into()),
            Vec::new(),
            move |v| Input::Score(a, v as u32),
        );
        if a == 0 && page.exceptional_shown() {
            form.number(
                (
                    tr("Exceptional Strength"),
                    tr("1 to 99 for 18/01 to 18/99, 100 for 18/00, 0 for none"),
                ),
                page.exceptional.into(),
                (0, 100),
                Vec::new(),
                |v| Input::Exceptional(v as u32),
            );
        }
    }
}

fn race(form: &mut Builder<Input>, page: &RacePage) {
    form.group("", "").toggle(
        (
            tr("Standard Races Only"),
            tr("Leave out the monster races, as the original's switch does"),
        ),
        page.standard_only,
        Input::StandardOnly,
    );
    form.group(tr("Race"), &page.description);
    if page.races.is_empty() {
        form.value(
            tr("No race allows these scores"),
            tr("Go back and roll again"),
            "",
        );
    } else {
        form.radio(&page.races, Some(page.chosen), Input::Race);
    }
    if page.subraces_shown() {
        form.choice(
            (tr("Subrace"), ""),
            page.subraces.clone(),
            Some(page.subrace),
            Vec::new(),
            Input::Subrace,
        );
    }
}

/// Check boxes for the abilities of a group, with their costs.
fn ability_checks(
    form: &mut Builder<Input>,
    slot: usize,
    rows: &[(String, String)],
    choices: &[(String, i32)],
    chosen: &[String],
) {
    let rows: Vec<(String, String, bool, bool)> = rows
        .iter()
        .zip(choices)
        .map(|((title, cost), (name, _))| {
            (title.clone(), cost.clone(), chosen.contains(name), true)
        })
        .collect();
    form.checks(&rows, move |i, on| Input::Ability(slot, i, on));
}

fn racial(form: &mut Builder<Input>, page: &RacialPage) {
    form.group(tr("Racial Abilities"), &page.description);
    if page.fixed() {
        if page.chosen.is_empty() {
            form.none();
        }
        for name in &page.chosen {
            form.note(&cr_app::create::ability_title(name));
        }
    } else {
        ability_checks(form, 0, &page.rows, &page.choices, &page.chosen);
    }
}

fn class(form: &mut Builder<Input>, page: &ClassPage) {
    form.group(tr("Class"), &page.description);
    if page.options.is_empty() {
        form.value(
            tr("No class allows these scores"),
            tr("Go back and choose another race or roll again"),
            "",
        );
        return;
    }
    form.radio(&page.rows, Some(page.chosen), Input::Class);
    if page.choice {
        form.group(tr("Ability Scores"), "");
        let roll = form.roll(tr("Roll Again"), Input::Roll);
        form.heading_button(roll)
            .value(&page.rolled_text.0, &page.rolled_text.1, "");
    }
}

fn entered_class(form: &mut Builder<Input>, page: &EnteredClassPage) {
    form.group(tr("Class"), &page.race);
    let all = form.button(tr("All Classes…"), "", true, Input::AllClasses);
    form.heading_button(all)
        .radio(&page.rows, Some(page.chosen), Input::Class);
}

fn class_abilities(form: &mut Builder<Input>, page: &ClassAbilitiesPage) {
    for (slot, group) in page.groups.iter().enumerate() {
        let chosen = page.chosen.get(slot).map(Vec::as_slice).unwrap_or_default();
        form.group(&group.title, &group.description);
        ability_checks(form, slot, &group.rows, &group.choices, chosen);
    }
}

fn details(
    form: &mut Builder<Input>,
    page: &DetailsPage,
    install: &Installation,
    race: &str,
) -> Option<FormDialog> {
    let random = if page.random_names {
        vec![form.roll(tr("Random Name"), Input::RandomName)]
    } else {
        Vec::new()
    };
    form.group(&page.heading, "")
        .text(tr("Name"), &page.name, false, random, Input::Name)
        .text(tr("Player"), &page.player, false, Vec::new(), Input::Player)
        .choice(
            (tr("Gender"), ""),
            vec![tr("Male").to_owned(), tr("Female").to_owned()],
            Some(usize::from(!page.male)),
            Vec::new(),
            Input::Gender,
        );
    form.portrait(install, &page.portrait, race, Input::Portrait);

    let social_roll = form.roll(tr("Roll Social Class"), Input::RollSocialClass);
    form.group(tr("Background"), "")
        .choice(
            (tr("Alignment"), ""),
            page.alignments.iter().map(|a| tr(a).to_owned()).collect(),
            page.alignment_index(),
            Vec::new(),
            Input::Alignment,
        )
        .choice(
            (tr("Kit"), ""),
            page.kits.iter().map(|k| none_label(k)).collect(),
            page.kit_index(),
            Vec::new(),
            Input::Kit,
        )
        .choice(
            (tr("Social Class"), ""),
            SOCIAL_CLASSES.iter().map(|s| tr(s).to_owned()).collect(),
            Some(page.social_class),
            vec![social_roll],
            Input::SocialClass,
        );
    if !page.backgrounds.is_empty() {
        let random = form.roll(tr("Select Randomly"), Input::RandomBackground);
        form.choice(
            (tr("Background Event"), ""),
            page.backgrounds.iter().map(|b| none_label(b)).collect(),
            page.background_index(),
            vec![random],
            Input::Background,
        );
    }

    form.group(tr("Appearance"), "");
    let rolls = [
        form.roll(tr("Roll Age"), Input::RollAge),
        form.roll(tr("Roll Height"), Input::RollHeight),
        form.roll(tr("Roll Weight"), Input::RollWeight),
    ];
    let [age, height, weight] = rolls;
    form.number(
        (tr("Age"), tr("Years")),
        page.age.into(),
        (1, 9999),
        vec![age],
        |v| Input::Age(v as u32),
    )
    .number(
        (tr("Height"), tr("Inches")),
        page.height.into(),
        (1, 999),
        vec![height],
        |v| Input::Height(v as u32),
    )
    .number(
        (tr("Weight"), tr("Pounds")),
        page.weight.into(),
        (1, 9999),
        vec![weight],
        |v| Input::Weight(v as u32),
    );

    // Enter Character's levels stand in for the rolls.
    let dialog = page
        .levels
        .as_ref()
        .and_then(|levels| entered_levels(form, levels));

    form.group(tr("Combat"), "");
    if let Some(rolls) = &page.hit_dice_text {
        let roll = form.roll(tr("Roll Hit Points"), Input::RollHitPoints);
        form.row(FormRow::Value {
            title: tr("Hit Die Roll").to_owned(),
            subtitle: rolls.clone(),
            value: String::new(),
            buttons: vec![roll],
        });
    }
    if page.exceptional_shown {
        if page.exceptional_bought {
            form.value(
                tr("Exceptional Strength"),
                tr("Bought with the ability points"),
                &page.exceptional.to_string(),
            );
        } else {
            let roll = form.roll(tr("Roll Exceptional Strength"), Input::RollExceptional);
            form.number(
                (tr("Exceptional Strength"), tr("18/01 to 18/00")),
                page.exceptional.into(),
                (1, 100),
                vec![roll],
                |v| Input::Exceptional(v as u32),
            );
        }
    }
    if let Some(spreads) = page.subabilities {
        form.group(
            tr("Subabilities"),
            tr("Raising one subability lowers the other as much"),
        );
        for (a, ((title, subtitle), spread)) in page.subability_rows.iter().zip(spreads).enumerate()
        {
            form.number(
                (title, subtitle),
                spread.into(),
                (-2, 2),
                Vec::new(),
                move |v| Input::Subability(a, v as i32),
            );
        }
    }
    if !page.backgrounds.is_empty() {
        form.group(tr("History"), "").text(
            tr("History"),
            &page.history,
            true,
            Vec::new(),
            Input::History,
        );
    }
    dialog
}

/// Enter Character's Levels group: each class with its level, Levels…,
/// and the hit points and PSPs; the Levels dialog, when it is open.
fn entered_levels(form: &mut Builder<Input>, levels: &EnteredLevels) -> Option<FormDialog> {
    form.group(tr("Levels"), "");
    let open = form.button(tr("Levels…"), "", levels.error.is_none(), Input::OpenLevels);
    form.heading_button(open);
    if let Some(error) = &levels.error {
        form.row(FormRow::Note {
            text: error.clone(),
            secondary: true,
        });
        return None;
    }
    for class in &levels.classes {
        form.value(&class.name, &class.text, "");
    }
    if let Some(hp) = levels.hit_points {
        form.number(
            (
                tr("Hit Points"),
                tr("Changes the rolls of the highest level"),
            ),
            hp.into(),
            (1, MOST_HIT_POINTS.into()),
            Vec::new(),
            |v| Input::HitPoints(v as u32),
        );
    }
    if let Some(psp) = levels.psp {
        form.number(
            (
                tr("PSPs"),
                tr("Changes those of the highest level that has them"),
            ),
            psp.into(),
            (0, MOST_HIT_POINTS.into()),
            Vec::new(),
            |v| Input::Psp(v as u32),
        );
    }
    let dialog = levels.dialog.as_ref()?;
    // The dialog's controls are numbered with the page's.
    let page = form.take_groups();
    levels_dialog(form, dialog, Input::Levels);
    let groups = form.take_groups();
    form.put_groups(page);
    let close = form.button(tr("_Close"), "", true, Input::CloseLevels);
    Some(FormDialog {
        title: tr("Levels").to_owned(),
        groups,
        close,
        accept: None,
    })
}

/// The Levels dialog: each class's level, the prime requisite bonus, the
/// level limits and, with Skills & Powers, the character points; Add
/// Experience in it. `map` makes the input of a change.
pub(crate) fn levels_dialog<I: Clone + Send + Sync + 'static>(
    form: &mut Builder<I>,
    levels: &LevelsForm,
    map: fn(LevelsInput) -> I,
) {
    form.group(
        "",
        tr(
            "A level puts the class at the experience it takes; each level gained rolls its hit \
            points and PSPs",
        ),
    );
    for (g, class) in levels.classes.iter().enumerate() {
        form.number(
            (&class.name, &class.text),
            class.level.into(),
            (1, MOST_LEVEL.into()),
            Vec::new(),
            move |v| map(LevelsInput::Level(g, v as u32)),
        );
    }
    match &levels.experience {
        Some(experience) => {
            form.group(tr("Add Experience"), "");
            experience_fields(form, experience, map);
            let cancel = form.button(tr("_Cancel"), "", true, map(LevelsInput::Cancel));
            let add = form.button(tr("_Add"), "", true, map(LevelsInput::Give));
            form.heading_button(cancel).heading_button(add);
        }
        None => {
            let add = form.button(
                tr("Add Experience…"),
                "",
                true,
                map(LevelsInput::AddExperience),
            );
            form.button_row(tr("Add Experience"), "", add);
        }
    }
    form.group("", "");
    if let Some(text) = &levels.notice {
        form.banner(text, false);
    }
    form.toggle(
        (
            tr("Apply Prime Requisite Experience Bonuses"),
            tr("10% more for a class whose prime requisites are all 16 or more"),
        ),
        levels.prime_bonus,
        move |on| map(LevelsInput::PrimeBonus(on)),
    );
    if levels.limits_shown {
        form.choice(
            (tr("Level Limits"), ""),
            vec![
                tr("None").to_owned(),
                tr("Racial Limits").to_owned(),
                tr("Racial Limits and Prime Requisite Bonus").to_owned(),
            ],
            Some(levels.limits),
            Vec::new(),
            move |i| map(LevelsInput::Limits(i)),
        );
    }
    if levels.skills_and_powers {
        form.number(
            (tr("Character Points"), tr("Unspent")),
            levels.character_points.into(),
            (0, 9999),
            Vec::new(),
            move |v| map(LevelsInput::CharacterPoints(v as u32)),
        )
        .number(
            (tr("Character Points per Level"), ""),
            levels.points_per_level.into(),
            (0, 10),
            Vec::new(),
            move |v| map(LevelsInput::PointsPerLevel(v as u32)),
        );
    }
}

/// Add Experience's amount, the classes it goes to, its own prime
/// requisite bonus and points per level when it has them, and why it
/// could not be given.
pub(crate) fn experience_fields<I: Clone + Send + Sync + 'static>(
    form: &mut Builder<I>,
    experience: &ExperienceForm,
    map: fn(LevelsInput) -> I,
) {
    form.note(tr(
        "Each level gained rolls its hit points. Levels lost keep their rolls, as in the \
         original.",
    ))
    .number(
        (tr("Experience"), tr("Negative to take experience away")),
        experience.amount,
        (-MOST_EXPERIENCE, MOST_EXPERIENCE),
        Vec::new(),
        move |v| map(LevelsInput::Amount(v)),
    )
    .choice(
        (tr("Give To"), ""),
        experience.targets.clone(),
        Some(experience.target),
        Vec::new(),
        move |i| map(LevelsInput::Target(i)),
    );
    if let Some(settings) = &experience.settings {
        form.toggle(
            (
                tr("Prime Requisite Bonus"),
                tr("10% more for a class whose prime requisites are all 16 or more"),
            ),
            settings.prime_bonus,
            move |on| map(LevelsInput::PrimeBonus(on)),
        );
        if settings.skills_and_powers {
            form.number(
                (tr("Character Points per Level"), ""),
                settings.points_per_level.into(),
                (0, 10),
                Vec::new(),
                move |v| map(LevelsInput::PointsPerLevel(v as u32)),
            );
        }
    }
    if let Some(error) = &experience.error {
        form.banner(error, true);
    }
}

/// A list the classes or the kit have the user pick from: once as many
/// are ticked as may be, the others are off.
fn pick_group(form: &mut Builder<Input>, g: usize, group: &PickGroup) {
    let rows: Vec<(String, String, bool, bool)> = group
        .names
        .iter()
        .zip(&group.picked)
        .map(|(name, &on)| (name.clone(), String::new(), on, on || !group.full))
        .collect();
    form.group(&group.title, &group.description)
        .checks(&rows, move |i, on| Input::Pick(g, i, on));
}

fn proficiencies(form: &mut Builder<Input>, page: &ProficienciesPage) {
    form.list(ListKind::Nonweapon, &page.nonweapon, &[], LISTS);
    for (g, pick) in page.picks.iter().enumerate() {
        pick_group(form, g, pick);
    }
    form.list(ListKind::Traits, &page.traits, &[], LISTS).list(
        ListKind::Disadvantages,
        &page.disadvantages,
        &[],
        LISTS,
    );
    form.group(tr("Native Languages"), "");
    if page.native.is_empty() {
        form.none();
    }
    for name in &page.native {
        form.note(name);
    }
    form.list(ListKind::Learned, &page.learned, &[], LISTS);
    if !page.requirements.is_empty() {
        form.group(
            tr("Kit Weapon Proficiency Requirements"),
            &page.requirements_text,
        );
        for text in &page.requirements {
            form.note(text);
        }
    }
    if let Some(lending) = &page.lending {
        form.group(tr("Nonweapon Slots for Weapons"), &lending.description)
            .number(
                (tr("Nonweapon slots used"), ""),
                lending.value.into(),
                (lending.least.into(), lending.most.into()),
                Vec::new(),
                |v| Input::Lent(v as u32),
            );
    }
    form.list(ListKind::Weapons, &page.weapons, &[], LISTS)
        .list(ListKind::Broad, &page.broad, &[], LISTS)
        .list(ListKind::Tight, &page.tight, &[], LISTS)
        .picked(&page.focus, tr("Weapon"), Input::Focus)
        .list(ListKind::Specializations, &page.specializations, &[], LISTS)
        .picked(&page.free, tr("Weapon"), Input::FreeSpecialization)
        .unarmed(&page.unarmed, Input::Unarmed);
    if let Some(points) = &page.points_left {
        form.points_left(points, None);
    }
}

fn combat(form: &mut Builder<Input>, page: &CombatPage) {
    form.group(tr("Weapons"), &page.weapons_text);
    for (choice, make) in [
        (
            &page.weapon_of_choice,
            Input::WeaponOfChoice as fn(usize) -> Input,
        ),
        (&page.expertise, Input::Expertise),
        (&page.specialization, Input::Specialization),
    ] {
        if choice.shown {
            form.choice(
                (&choice.title, &choice.description),
                choice.items.clone(),
                Some(choice.selected),
                Vec::new(),
                make,
            );
        }
    }
    form.unarmed(&page.unarmed, Input::Unarmed);
    let shields: Vec<(String, String, bool, bool)> = page
        .shields
        .iter()
        .map(|(title, subtitle, on)| (title.clone(), subtitle.clone(), *on, true))
        .collect();
    form.group(tr("Shield Proficiencies"), "")
        .checks(&shields, Input::Shield)
        .list(ListKind::Armor, &page.armor, &[], LISTS)
        .list(ListKind::Styles, &page.styles, &[], LISTS)
        .points_left(&page.points_left, page.problem.as_deref());
}

fn rogue(form: &mut Builder<Input>, page: &RoguePage) {
    form.group(tr("Rogue Skills"), &page.description);
    for (i, row) in page.rows.iter().enumerate() {
        form.number(
            (&row.name, &row.scores),
            row.points.into(),
            (0, page.most.into()),
            Vec::new(),
            move |v| Input::RoguePoints(i, v as u32),
        );
    }
    if page.bonus.shown {
        let rows: Vec<(String, String)> = page
            .bonus
            .names
            .iter()
            .map(|n| (n.clone(), String::new()))
            .collect();
        form.group(&page.bonus.title, &page.bonus.description)
            .radio(
                &rows,
                page.bonus.picked.iter().position(|p| *p),
                Input::RogueBonus,
            );
    }
    if page.chosen.shown {
        let full = page.chosen.full;
        let rows: Vec<(String, String, bool, bool)> = page
            .chosen
            .names
            .iter()
            .zip(&page.chosen.picked)
            .map(|(n, &on)| (n.clone(), String::new(), on, on || !full))
            .collect();
        form.group(&page.chosen.title, &page.chosen.description)
            .checks(&rows, Input::RogueChosen);
    }
}

fn spells(form: &mut Builder<Input>, page: &SpellsPage) {
    if let Some(schools) = &page.eliminated {
        let full = schools.full;
        let rows: Vec<(String, String, bool, bool)> = schools
            .names
            .iter()
            .zip(&schools.ticked)
            .map(|(n, &on)| (n.clone(), String::new(), on, on || !full))
            .collect();
        form.group(&schools.title, &schools.description)
            .checks(&rows, Input::School);
    }
    form.list(ListKind::Spells, &page.spells, &page.chances, LISTS);
}

/// The Wild Talents step: Test and Roll, the talents and, for a wild
/// talent, the psionic attack and defense modes.
fn wild(form: &mut Builder<Input>, page: &WildPage) {
    form.group(tr("Wild Talents"), &page.description);
    if page.test_shown {
        let test = form.button(tr("Test"), "", page.can_test, Input::Test);
        form.button_row(tr("Test for Wild Talents"), &page.chance, test);
    }
    let roll = form.button(tr("Roll"), "", page.can_roll, Input::Roll);
    form.button_row(
        tr("Roll for Wild Talents"),
        tr("Table 83: Wild Talents"),
        roll,
    )
    .list(ListKind::WildTalents, &page.talents, &[], LISTS)
    .list(ListKind::Attacks, &page.attacks, &[], LISTS)
    .list(ListKind::Defenses, &page.defenses, &[], LISTS);
}

/// What the Skills form says: a rogue skill's points, the kit's bonus
/// given, one of its chosen skills ticked, a primary discipline, and its
/// lists' changes.
pub(crate) struct SkillsInputs<I> {
    pub points: fn(usize, i32) -> I,
    pub bonus: fn(usize, usize) -> I,
    pub chosen: fn(usize, bool) -> I,
    pub primary: fn(usize, usize) -> I,
    pub lists: ListInputs<I>,
}

impl<I> Clone for SkillsInputs<I> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I> Copy for SkillsInputs<I> {}

/// The Skills form: the rogue skills, the kit's bonuses and chosen
/// skills, the psionics, the disciplines and powers, then what `wild`
/// adds (Edit Character's Test and Roll) with the wild talents, and the
/// psionic attack and defense modes.
pub(crate) fn skills_form<I: Clone + Send + Sync + 'static>(
    form: &mut Builder<I>,
    skills: &SkillsForm,
    inputs: SkillsInputs<I>,
    wild: impl FnOnce(&mut Builder<I>),
) {
    if let Some(group) = skills.rogue() {
        form.group(tr("Rogue Skills"), &group.description).value(
            tr("Points Left"),
            "",
            &group.left.to_string().replace('-', "−"),
        );
        for (i, row) in group.rows.iter().enumerate() {
            form.number(
                (&row.label, &row.subtitle),
                row.points.into(),
                (row.least.into(), MOST_SKILL_POINTS.into()),
                Vec::new(),
                move |v| (inputs.points)(i, v as i32),
            );
        }
        for warning in &group.warnings {
            form.banner(warning, false);
        }
    }
    for (g, group) in skills.kit_bonuses().into_iter().enumerate() {
        let rows: Vec<(String, String)> = group
            .names
            .iter()
            .map(|n| (n.clone(), String::new()))
            .collect();
        form.group(&group.title, &group.description)
            .radio(&rows, group.picked, move |i| (inputs.bonus)(g, i));
    }
    if let Some(group) = skills.chosen() {
        let full = group.full;
        let rows: Vec<(String, String, bool, bool)> = group
            .names
            .iter()
            .zip(&group.ticked)
            .map(|(n, &on)| (n.clone(), String::new(), on, on || !full))
            .collect();
        form.group(&group.title, &group.description)
            .checks(&rows, inputs.chosen);
    }
    if let Some(description) = skills.psionics() {
        form.group(tr("Psionics"), &description);
        for (slot, choice) in skills.primary().into_iter().enumerate() {
            form.choice(
                (&choice.title, ""),
                choice.items.clone(),
                Some(choice.selected),
                Vec::new(),
                move |i| (inputs.primary)(slot, i),
            );
        }
    }
    for kind in [
        ListKind::Disciplines,
        ListKind::Sciences,
        ListKind::Devotions,
    ] {
        form.list(kind, &skills.list(kind), &[], inputs.lists);
    }
    wild(form);
    for kind in [ListKind::WildTalents, ListKind::Attacks, ListKind::Defenses] {
        form.list(kind, &skills.list(kind), &[], inputs.lists);
    }
}

/// The Psionics step.
fn skills(form: &mut Builder<Input>, skills: &SkillsForm) {
    skills_form(
        form,
        skills,
        SkillsInputs {
            points: Input::SkillPoints,
            bonus: Input::KitBonus,
            chosen: Input::RogueChosen,
            primary: Input::Primary,
            lists: LISTS,
        },
        |_| {},
    );
}

/// The last step: the starting money, the standard pack, what comes free
/// and what the money buys.
fn equipment(form: &mut Builder<Input>, page: &EquipmentPage) {
    let roll = form.roll(tr("Roll Starting Money"), Input::RollMoney);
    form.group(tr("Money"), "")
        .row(FormRow::Value {
            title: tr("Starting Money").to_owned(),
            subtitle: String::new(),
            value: page.money.clone(),
            buttons: vec![roll],
        })
        .toggle(
            (tr("Standard Pack"), &page.pack_text),
            page.pack,
            Input::Pack,
        );
    for (name, why) in &page.free {
        form.value(name, why, "");
    }
    form.list(ListKind::Items, &page.items, &[], LISTS);
}
