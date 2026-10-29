//! Edit Character, as the Mac app draws it: the editor of
//! `cr_app::editor`, each of its pages described as a form
//! ([`crate::form`]) as the Windows app's Edit Character page draws it,
//! its questions as prompts, and Save writing the character. The Details
//! page has the Name page of Update Character, the classes with Add
//! Class, the experience with Levels… and Add…, the appearance, the
//! ability scores and the history; the Abilities page the subrace, the
//! racial and class abilities; the Combat page the weapon proficiencies
//! and groups, the specializations, unarmed combat, the kit's weapons,
//! shields, armor, fighting styles, traits and disadvantages; the
//! Proficiencies page the nonweapon proficiencies and the languages; the
//! Skills page the rogue skills, the psionics and the wild talents; the
//! Spells page the kit's schools and spheres, the spells known, the
//! spellbooks and the spells memorized; the Equipment page the spending
//! money and the items of each bundle.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::create::{Answer, ListKind, none_label, points};
use cr_app::editor::{
    COIN_NAMES, Checks, EditEffect, EditInput, EditNeeds, Editor, MOST_AGE, MOST_COINS,
    MOST_HEIGHT, MOST_POINTS, MOST_QUANTITY, MOST_WEIGHT, Tab, social_class_shown,
};
use cr_app::levels::LevelsInput;
use cr_i18n::{tr, tr_noop};
use cr_store::Installation;
use cr_store::create::{ABILITIES, SUBABILITIES};
use cr_store::edit::ABILITY_RANGE;

use crate::form::{Acts, Builder, FormDialog, FormPage, FormValue, ListInputs};
use crate::wizard::{SkillsInputs, experience_fields, levels_dialog, skills_form};
use crate::{CoreError, PromptInfo};

const GENDERS: [&str; 2] = [tr_noop!("Male"), tr_noop!("Female")];

/// What Save came to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SaveState {
    /// The character was written.
    Saved { name: String },
    /// A question or message waits on an answer first.
    Asking,
    /// Nothing was saved: nothing changed, or the form has a problem.
    NotSaved,
}

/// Edit Character, open in a sheet.
#[derive(uniffi::Object)]
pub struct EditSession {
    install: Arc<Installation>,
    editor: Mutex<Editor>,
    /// What the controls of the form last described do.
    acts: Mutex<Acts<EditInput>>,
    /// The icons of the items the bundles have held, as PNG, by name.
    icons: Mutex<HashMap<String, Option<Vec<u8>>>>,
}

impl EditSession {
    pub(crate) fn open(install: Arc<Installation>, file: &str) -> Result<Arc<Self>, CoreError> {
        let character = install.character(file)?;
        let needs = EditNeeds::load(&install)?;
        let editor = Editor::new(Arc::new(needs), file, character);
        Ok(Arc::new(Self {
            install,
            editor: Mutex::new(editor),
            acts: Mutex::default(),
            icons: Mutex::default(),
        }))
    }

    fn editor(&self) -> MutexGuard<'_, Editor> {
        self.editor.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Does what the editor asks of the app: writes the character.
    fn settle(&self, editor: &mut Editor) -> SaveState {
        let mut state = if editor.prompt().is_some() {
            SaveState::Asking
        } else {
            SaveState::NotSaved
        };
        for effect in editor.take_effects() {
            match effect {
                EditEffect::Save(saving) => {
                    match self.install.save_character(
                        &saving.file,
                        &saving.details,
                        Some(&saving.lists),
                    ) {
                        Ok(()) => {
                            state = SaveState::Saved {
                                name: saving.details.name.trim().to_owned(),
                            }
                        }
                        Err(e) => editor.failed(e.to_string()),
                    }
                }
            }
        }
        state
    }
}

#[uniffi::export]
impl EditSession {
    /// The program help's topic of the page in view.
    pub fn help(&self) -> u32 {
        self.editor().help()
    }

    /// The character's name as it was read, for the sheet's title.
    pub fn title(&self) -> String {
        self.editor().character().data.name.trim().to_owned()
    }

    /// The pages' titles, in order.
    pub fn tabs(&self) -> Vec<String> {
        Tab::ALL.iter().map(|t| tr(t.title()).to_owned()).collect()
    }

    /// The page in view, by its place in [`Self::tabs`].
    pub fn tab(&self) -> u32 {
        let tab = self.editor().tab;
        Tab::ALL.iter().position(|t| *t == tab).unwrap_or(0) as u32
    }

    pub fn set_tab(&self, index: u32) {
        if let Some(tab) = Tab::ALL.get(index as usize) {
            self.editor().input(EditInput::Tab(*tab));
        }
    }

    /// The page in view, as a form, with the Levels or Add Experience
    /// dialog when one is open over it.
    pub fn form(&self) -> FormPage {
        let editor = self.editor();
        let mut icons = self.icons.lock().unwrap_or_else(|e| e.into_inner());
        let mut icon = |name: &str| -> Option<Vec<u8>> {
            icons
                .entry(name.to_owned())
                .or_insert_with(|| {
                    let icon = editor.needs().icons.get(name)?;
                    crate::png(&cr_app::pictures::icon(icon)?)
                })
                .clone()
        };
        let mut form = Builder::new();
        match editor.tab {
            Tab::Details => details(&mut form, &editor, &self.install),
            Tab::Abilities => abilities_tab(&mut form, &editor),
            Tab::Combat => combat_tab(&mut form, &editor),
            Tab::Proficiencies => {
                form.list(ListKind::Nonweapon, &editor.nonweapon_list(), &[], LISTS)
                    .list(ListKind::Learned, &editor.languages_list(), &[], LISTS);
            }
            Tab::Skills => skills_tab(&mut form, &editor),
            Tab::Spells => spells_tab(&mut form, &editor),
            Tab::Equipment => equipment_tab(&mut form, &editor, &mut icon),
        }
        let dialog = dialog(&mut form, &editor);
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
            self.editor().input(input);
        }
    }

    /// Whether anything was changed.
    pub fn changed(&self) -> bool {
        self.editor().changed()
    }

    /// Why the character cannot be saved as it is, if it cannot.
    pub fn problem(&self) -> Option<String> {
        self.editor().problem()
    }

    /// Whether Save may be pressed.
    pub fn can_save(&self) -> bool {
        self.editor().can_save()
    }

    /// Whether the character is being written: Cancel waits.
    pub fn saving(&self) -> bool {
        self.editor().saving
    }

    /// Why the character could not be written, when it could not.
    pub fn error(&self) -> Option<String> {
        self.editor().error.clone()
    }

    /// The question or message in view.
    pub fn prompt(&self) -> Option<PromptInfo> {
        self.editor().prompt().map(PromptInfo::of)
    }

    /// Save: checks the character, asks what the original asks, and
    /// writes it.
    pub fn save(&self) -> SaveState {
        let mut editor = self.editor();
        editor.error = None;
        editor.input(EditInput::Save);
        self.settle(&mut editor)
    }

    /// Answers the prompt in view, which may go on to save.
    pub fn answer(&self, yes: bool) -> SaveState {
        let mut editor = self.editor();
        editor.answer(if yes { Answer::Yes } else { Answer::No });
        self.settle(&mut editor)
    }
}

/// The editor's lists' inputs.
const LISTS: ListInputs<EditInput> = ListInputs {
    add: EditInput::Add,
    remove: EditInput::Remove,
    times: EditInput::Times,
};

/// The Levels dialog, or Add Experience, when one is open; its controls
/// are numbered with the page's.
fn dialog(form: &mut Builder<EditInput>, editor: &Editor) -> Option<FormDialog> {
    let page = form.take_groups();
    let dialog = if let Some(levels) = &editor.levels {
        levels_dialog(form, levels, EditInput::Levels);
        let close = form.button(tr("_Close"), "", true, EditInput::CloseLevels);
        Some(FormDialog {
            title: tr("Levels").to_owned(),
            groups: form.take_groups(),
            close,
            accept: None,
        })
    } else if let Some(experience) = &editor.experience {
        form.group("", "");
        experience_fields(form, experience, EditInput::Experience);
        let close = form.button(
            tr("_Cancel"),
            "",
            true,
            EditInput::Experience(LevelsInput::Cancel),
        );
        let accept = form.button(
            tr("_Add"),
            "",
            true,
            EditInput::Experience(LevelsInput::Give),
        );
        Some(FormDialog {
            title: tr("Add Experience").to_owned(),
            groups: form.take_groups(),
            close,
            accept: Some(accept),
        })
    } else {
        None
    };
    form.put_groups(page);
    dialog
}

/// The Details page.
fn details(form: &mut Builder<EditInput>, editor: &Editor, install: &Installation) {
    let d = &editor.details;
    form.group("", "")
        .text(tr("Name"), &d.name, false, Vec::new(), EditInput::Name)
        .text(
            tr("Player"),
            &d.player,
            false,
            Vec::new(),
            EditInput::Player,
        )
        .text(
            tr("Social Class"),
            &social_class_shown(&d.social_class),
            false,
            Vec::new(),
            EditInput::SocialClass,
        )
        .portrait(
            install,
            &d.portrait,
            &editor.character().data.race,
            EditInput::Portrait,
        )
        .choice(
            (tr("Gender"), ""),
            GENDERS.iter().map(|g| tr(g).to_owned()).collect(),
            Some(usize::from(!d.male)),
            Vec::new(),
            EditInput::Gender,
        );

    form.group(tr("Background"), "")
        .choice(
            (tr("Alignment"), ""),
            editor.alignments.iter().map(|a| tr(a).to_owned()).collect(),
            editor.alignment_index(),
            Vec::new(),
            EditInput::Alignment,
        )
        .choice(
            (
                tr("Kit"),
                tr("A new kit rolls the social class on its table"),
            ),
            editor.kits.iter().map(|k| none_label(k)).collect(),
            editor.kit_index(),
            Vec::new(),
            EditInput::Kit,
        );
    if !editor.backgrounds.is_empty() {
        let random = form.roll(tr("Select Randomly"), EditInput::RandomBackground);
        form.choice(
            (tr("Background Event"), ""),
            editor.backgrounds.iter().map(|b| none_label(b)).collect(),
            editor.background_index(),
            vec![random],
            EditInput::Background,
        );
    }

    classes(form, editor);
    experience(form, editor);

    form.group(tr("Appearance"), "")
        .number(
            (tr("Age"), tr("Years")),
            d.age.into(),
            (0, MOST_AGE.into()),
            Vec::new(),
            |v| EditInput::Age(v as u32),
        )
        .number(
            (tr("Height"), tr("Inches")),
            d.height.into(),
            (0, MOST_HEIGHT.into()),
            Vec::new(),
            |v| EditInput::Height(v as u32),
        )
        .number(
            (tr("Weight"), tr("Pounds")),
            d.weight.into(),
            (0, MOST_WEIGHT.into()),
            Vec::new(),
            |v| EditInput::Weight(v as u32),
        );

    ability_scores(form, editor);

    if !editor.backgrounds.is_empty() || !d.history.is_empty() {
        form.group(tr("History"), "").text(
            tr("History"),
            &d.history,
            true,
            Vec::new(),
            EditInput::History,
        );
    }
}

/// The classes, and Add Class for one more when the race may dual-class.
fn classes(form: &mut Builder<EditInput>, editor: &Editor) {
    let offer = editor.class_offer();
    form.group(tr("Classes"), &offer.description);
    for name in &editor.character().data.classes.names {
        form.note(name);
    }
    match &editor.new_class {
        Some(name) => {
            let remove = form.button(tr("Remove"), "minus.circle", true, EditInput::RemoveClass);
            form.row(crate::form::FormRow::Value {
                title: name.clone(),
                subtitle: tr("New, level 1").to_owned(),
                value: String::new(),
                buttons: vec![remove],
            });
        }
        // One class at a time: the next one depends on this one.
        None if !offer.names.is_empty() => {
            let id = form.on_index(EditInput::AddClass);
            form.row(crate::form::FormRow::Add {
                id,
                label: tr("Add Class").to_owned(),
                options: offer.names.clone(),
            });
        }
        None => {}
    }
}

/// Experience, levels, hit points and PSPs, with Levels… and Add….
fn experience(form: &mut Builder<EditInput>, editor: &Editor) {
    form.group(tr("Experience"), "");
    let levels = form.button(tr("Levels…"), "", true, EditInput::OpenLevels);
    let add = form.button(tr("Add…"), "", true, EditInput::OpenExperience);
    form.heading_button(levels).heading_button(add);
    for class in editor.class_rows() {
        form.value(&class.name, &class.text, "");
    }
    if let Some(hp) = editor.hit_points() {
        form.number(
            (
                tr("Hit Points"),
                tr("Changes the rolls of the highest level"),
            ),
            hp.into(),
            (0, MOST_POINTS.into()),
            Vec::new(),
            |v| EditInput::HitPoints(v as u32),
        );
    }
    if let Some(psp) = editor.psp() {
        form.number(
            (
                tr("PSPs"),
                tr("Changes those of the highest level that has them"),
            ),
            psp.into(),
            (0, 999),
            Vec::new(),
            |v| EditInput::Psp(v as u32),
        );
    }
    if let Some(points) = editor.character_points() {
        form.value(tr("Character Points"), tr("Unspent"), &points.to_string());
    }
}

/// The ability scores, with Skills & Powers' subabilities, and
/// exceptional strength.
fn ability_scores(form: &mut Builder<EditInput>, editor: &Editor) {
    let d = &editor.details;
    let range = (
        i64::from(*ABILITY_RANGE.start()),
        i64::from(*ABILITY_RANGE.end()),
    );
    form.group(tr("Abilities"), "");
    for (a, scores) in d.abilities.iter().enumerate().take(6) {
        if d.skills_and_powers {
            form.number(
                (tr(ABILITIES[a]), tr("Score")),
                scores[0].into(),
                range,
                Vec::new(),
                move |v| EditInput::Ability(a, 0, v as u32),
            );
            for (part, sub) in SUBABILITIES[a].iter().enumerate() {
                form.number(
                    (tr(sub), tr(ABILITIES[a])),
                    scores[part + 1].into(),
                    range,
                    Vec::new(),
                    move |v| EditInput::Ability(a, part + 1, v as u32),
                );
            }
        } else {
            form.number(
                (tr(ABILITIES[a]), ""),
                scores[0].into(),
                range,
                Vec::new(),
                move |v| EditInput::Ability(a, 0, v as u32),
            );
        }
    }
    form.number(
        (tr("Exceptional Strength"), tr("18/01 to 18/00, 0 for none")),
        d.exceptional_strength.into(),
        (0, 100),
        Vec::new(),
        |v| EditInput::Exceptional(v as u32),
    )
    .enabled(editor.exceptional_allowed());
}

/// The Abilities page: the subrace and its package, then each group of
/// abilities to tick.
fn abilities_tab(form: &mut Builder<EditInput>, editor: &Editor) {
    form.points_left(&points(editor.points_left()), None)
        .group(tr("Race"), editor.subrace_description())
        .choice(
            (tr("Subrace"), ""),
            editor.subraces(),
            editor.subrace_index(),
            Vec::new(),
            EditInput::Subrace,
        )
        .group(tr("Racial Package"), "");
    let package = editor.package();
    if package.is_empty() {
        form.none();
    }
    for name in &package {
        form.note(name);
    }
    check_groups(form, editor.ability_groups(), EditInput::Tick);
}

/// Groups of check boxes, each under its heading; `tick` makes the input
/// of one ticked or not.
fn check_groups<G: Copy + Send + Sync + 'static>(
    form: &mut Builder<EditInput>,
    groups: Vec<Checks<G>>,
    tick: fn(G, usize, bool) -> EditInput,
) {
    for group in groups {
        form.group(&group.title, &group.description);
        if group.checks.is_empty() {
            form.none();
            continue;
        }
        let rows: Vec<(String, String, bool, bool)> = group
            .checks
            .into_iter()
            .map(|c| (c.title, c.subtitle, c.on, c.enabled))
            .collect();
        let which = group.group;
        form.checks(&rows, move |i, on| tick(which, i, on));
    }
}

/// The Combat page.
fn combat_tab(form: &mut Builder<EditInput>, editor: &Editor) {
    let v = editor.combat_view();
    let weapon = tr("Weapon");
    if let Some(left) = v.points_left {
        form.points_left(&points(left), Some(&v.description));
    }
    form.list(ListKind::Weapons, &v.weapons, &[], LISTS)
        .list(ListKind::Broad, &v.broad, &[], LISTS)
        .list(ListKind::Tight, &v.tight, &[], LISTS)
        .picked(&v.weapon_of_choice, weapon, EditInput::WeaponOfChoice)
        .picked(&v.expertise, weapon, EditInput::Expertise)
        .list(ListKind::Specializations, &v.specializations, &[], LISTS)
        .picked(&v.mastery, weapon, EditInput::Mastery)
        .unarmed(&v.unarmed, EditInput::Unarmed)
        .picked(&v.focus, weapon, EditInput::Focus)
        .picked(&v.picks[0], weapon, |i| EditInput::KitPick(0, i))
        .picked(&v.picks[1], weapon, |i| EditInput::KitPick(1, i));
    if !v.shields.is_empty() {
        let rows: Vec<(String, String, bool, bool)> = v
            .shields
            .iter()
            .map(|(name, cost, on)| (name.clone(), cost.clone(), *on, true))
            .collect();
        form.group(tr("Shield Proficiencies"), "")
            .checks(&rows, EditInput::Shield);
    }
    form.list(ListKind::Armor, &v.armor, &[], LISTS)
        .list(ListKind::Styles, &v.styles, &[], LISTS)
        .list(ListKind::Traits, &v.traits, &[], LISTS)
        .list(ListKind::Disadvantages, &v.disadvantages, &[], LISTS);
}

/// The Skills page, with the Wild Talents dialog's Test and Roll for a
/// character who is no psionicist.
fn skills_tab(form: &mut Builder<EditInput>, editor: &Editor) {
    let inputs = SkillsInputs {
        points: EditInput::SkillPoints,
        bonus: EditInput::KitBonus,
        chosen: EditInput::RogueChosen,
        primary: EditInput::Primary,
        lists: LISTS,
    };
    let wild = editor.skills.wild();
    skills_form(form, &editor.skills, inputs, |form| {
        let Some(wild) = wild else {
            return;
        };
        let test = form.button(tr("Test"), "", wild.can_test, EditInput::TestWild);
        let roll = form.button(tr("Roll"), "", true, EditInput::RollWild);
        form.group(tr("Wild Talents"), "")
            .button_row(tr("Test for Wild Talents"), &wild.chance, test)
            .button_row(
                tr("Roll for Wild Talents"),
                tr("Table 83: Wild Talents"),
                roll,
            );
    });
}

/// The Spells page: the kit's barred schools, the schools to eliminate,
/// the spheres to keep and the free sphere; the wizard and priest spells
/// known, each offered with the chance to learn it; the spellbooks; and
/// the spells each class that casts has memorized.
fn spells_tab(form: &mut Builder<EditInput>, editor: &Editor) {
    let magic = editor.kit_magic_view();
    if let Some((warning, ignored)) = &magic.barred {
        form.group(tr("Kit Class Ability Requirements"), warning)
            .toggle(
                (
                    tr("Ignore"),
                    tr(
                        "Keeps these schools; otherwise they are removed when the character is \
                         saved.",
                    ),
                ),
                *ignored,
                EditInput::IgnoreBarred,
            );
    }
    check_groups(form, magic.groups, EditInput::MagicTick);
    form.picked(
        &magic.free_sphere,
        tr("Selected Free Sphere"),
        EditInput::FreeSphere,
    );
    for (kind, wizard) in [(ListKind::Spells, true), (ListKind::PriestSpells, false)] {
        let (list, chances) = editor.spells_known(wizard);
        form.list(kind, &list, &chances, LISTS);
    }
    for (b, (list, most)) in editor.spellbooks().into_iter().enumerate() {
        form.list_with(
            ListKind::Spellbook(b),
            &list,
            &[],
            (tr("Pages"), 1, most),
            &mut |_| None,
            LISTS,
        );
    }
    for (slot, list) in editor.memorized() {
        form.list(ListKind::Memorized(slot), &list, &[], LISTS);
    }
}

/// The Equipment page: the spending money with Buy and Sell, then the
/// items of each other bundle with their icons, by `icon`.
fn equipment_tab(
    form: &mut Builder<EditInput>,
    editor: &Editor,
    icon: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) {
    let coins = editor.coins();
    form.group(tr("Spending Money"), "").toggle(
        (
            tr("Buy and Sell"),
            tr("Items added are paid for with the spending money, items removed are sold"),
        ),
        editor.trading,
        EditInput::Trading,
    );
    for (c, name) in COIN_NAMES.iter().enumerate() {
        form.number(
            (tr(name), ""),
            coins[c].into(),
            (0, MOST_COINS.into()),
            Vec::new(),
            move |v| EditInput::Coin(c, v as u32),
        );
    }
    for (b, list, prices) in editor.bundles() {
        form.list_with(
            ListKind::Bundle(b),
            &list,
            &prices,
            (tr("Quantity"), 1, MOST_QUANTITY),
            icon,
            LISTS,
        );
    }
}
