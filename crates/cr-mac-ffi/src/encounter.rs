//! Encounters as the Mac app draws them in a window of their own: New
//! Encounter's templates, then the Encounter page of `cr_app::encounter`
//! described as a form ([`crate::form`]), as the Windows app's pages
//! have them: the monsters and NPCs with their numbers and hit points,
//! the treasure, the items, the traps, the totals and the description.
//! The 2-20 table editor has a window of its own ([`TableEditSession`]).

use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::bundle::BundleInput;
use cr_app::encounter::{
    CLIMATES, EncounterEffect, EncounterForm, EncounterInput, FREQUENCIES, LAIR_MODES, MOST_COUNT,
    Pick, Picker, SOURCES, TERRAINS, TableEditInput, TableEditor, TableEffect,
};
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::encounter::{Encounter, EncounterRules, FrequencyTable, Package, check_name};
use cr_store::treasure::Rand;
use cr_store::userdb::traps::Traps;
use cr_store::userdb::{Filter, Origin, UserDb};

use crate::CoreError;
use crate::form::{Acts, Builder, FormDialog, FormPage, FormRow, FormValue};
use crate::record::RecordAct;
use crate::sheet::SheetSession;
use crate::treasure::TreasureSession;

/// The most names a list of a dialog shows; a search narrows it.
pub(crate) const MOST_SHOWN: usize = 200;

/// What a Save came to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SaveResult {
    Saved { name: String },
    Failed { error: String },
}

/// What the app says, as the original's message boxes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct EncounterNotice {
    pub title: String,
    pub text: String,
}

/// What a button of the Encounter page asks of the app.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum EncounterRequest {
    /// View NPC… or, with `print`, Print NPC…: the sheet of the NPC
    /// package `index` ([`EncounterSession::npc_sheet`]).
    NpcSheet { index: u32, print: bool },
    /// Roll Treasure…: the roller on the encounter's treasure
    /// ([`EncounterSession::treasure`]).
    Treasure,
    /// New Trap or New Monster from Add Trap or Add Monster: the editor
    /// of a new record of `section`, which, saved, goes in the encounter
    /// ([`EncounterSession::add_new_trap`],
    /// [`EncounterSession::add_new_monster`]).
    NewRecord { section: String },
    /// Edit, Copy or Delete on the trap Add Trap chose.
    Trap {
        act: RecordAct,
        name: String,
        custom: bool,
    },
}

/// New Encounter: the original's "Select an Encounter Template", with an
/// empty encounter first.
struct Chooser {
    templates: Vec<(String, String)>,
    search: String,
    /// The template chosen; `Some("")` for the empty encounter.
    chosen: Option<String>,
}

impl Chooser {
    /// The rows shown: the empty encounter without a search, then the
    /// templates the search lets through, each by name and monster type.
    fn shown(&self) -> Vec<(String, String)> {
        let mut rows = Vec::new();
        if self.search.trim().is_empty() {
            rows.push((String::new(), String::new()));
        }
        rows.extend(
            self.templates
                .iter()
                .filter(|(name, kind)| {
                    cr_app::encounter::template_matches(name, kind, &self.search)
                })
                .cloned(),
        );
        rows
    }
}

enum State {
    Choose(Chooser),
    Form(Box<EncounterForm>),
}

/// A control of the window.
#[derive(Clone)]
enum In {
    Search(String),
    Template(String),
    Form(EncounterInput),
    AddNpc,
    AddTrap,
    NpcSheet(usize, bool),
    Treasure,
    NewRecord(&'static str),
    Trap(RecordAct),
}

fn input(input: EncounterInput) -> In {
    In::Form(input)
}

/// New Encounter, or an encounter, open in its window.
#[derive(uniffi::Object)]
pub struct EncounterSession {
    install: Arc<Installation>,
    rules: Arc<EncounterRules>,
    state: Mutex<State>,
    /// What the controls of the form last described do.
    acts: Mutex<Acts<In>>,
    rand: Arc<Mutex<Rand>>,
    notices: Mutex<Vec<EncounterNotice>>,
    request: Mutex<Option<EncounterRequest>>,
    /// What a Save came to, until the app takes it.
    saved: Mutex<Option<SaveResult>>,
}

/// What generating said, as one notice.
fn said(messages: Vec<String>) -> Vec<EncounterNotice> {
    if messages.is_empty() {
        return Vec::new();
    }
    vec![EncounterNotice {
        title: tr("New Encounter").to_owned(),
        text: messages.join("\n"),
    }]
}

impl EncounterSession {
    fn open(
        install: Arc<Installation>,
        rules: Arc<EncounterRules>,
        state: State,
        rand: Arc<Mutex<Rand>>,
        notices: Vec<EncounterNotice>,
    ) -> Arc<Self> {
        Arc::new(Self {
            install,
            rules,
            state: Mutex::new(state),
            acts: Mutex::default(),
            rand,
            notices: Mutex::new(notices),
            request: Mutex::new(None),
            saved: Mutex::new(None),
        })
    }

    /// New Encounter's templates.
    pub(crate) fn choose(
        install: Arc<Installation>,
        rules: Arc<EncounterRules>,
        rand: Arc<Mutex<Rand>>,
    ) -> Arc<Self> {
        let chooser = Chooser {
            templates: cr_app::encounter::templates(&rules),
            search: String::new(),
            chosen: None,
        };
        Self::open(install, rules, State::Choose(chooser), rand, Vec::new())
    }

    /// An encounter made from the template `name`, saying `first` before
    /// what generating says.
    pub(crate) fn generate(
        install: Arc<Installation>,
        rules: Arc<EncounterRules>,
        name: &str,
        first: Option<String>,
        rand: Arc<Mutex<Rand>>,
    ) -> Arc<Self> {
        let (form, messages) = made(&rules, name, &rand);
        let mut all: Vec<String> = first.into_iter().collect();
        all.extend(messages);
        Self::open(install, rules, State::Form(form), rand, said(all))
    }

    /// The page on `encounter`, under no name: Group Encounter's, whose
    /// name is left for the user to give (0x44e759).
    pub(crate) fn unsaved(
        install: Arc<Installation>,
        rules: Arc<EncounterRules>,
        encounter: Encounter,
        rand: Arc<Mutex<Rand>>,
    ) -> Arc<Self> {
        let form = EncounterForm::new(rules.clone(), encounter, None, Vec::new());
        Self::open(
            install,
            rules,
            State::Form(Box::new(form)),
            rand,
            Vec::new(),
        )
    }

    /// The saved encounter `name`; with `copy`, under no name, so that
    /// Save adds a new one once it is named (the original's Copy).
    pub(crate) fn saved(
        install: Arc<Installation>,
        rules: Arc<EncounterRules>,
        name: String,
        copy: bool,
        rand: Arc<Mutex<Rand>>,
    ) -> Result<Arc<Self>, CoreError> {
        let mut encounter = install
            .saved_encounters()?
            .into_iter()
            .find(|e| e.name == name)
            .ok_or_else(|| CoreError::new(tr("Could Not Read")))?;
        if copy {
            encounter.name.clear();
        }
        let saved = (!copy).then_some(name);
        let form = EncounterForm::new(rules.clone(), encounter, saved, Vec::new());
        Ok(Self::open(
            install,
            rules,
            State::Form(Box::new(form)),
            rand,
            Vec::new(),
        ))
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn notice(&self, title: &str, text: String) {
        self.notices
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(EncounterNotice {
                title: title.to_owned(),
                text,
            });
    }

    fn input(&self, input: In) {
        let mut state = self.state();
        match (&mut *state, input) {
            (State::Choose(chooser), In::Search(search)) => {
                chooser.search = search;
                chooser.chosen = None;
            }
            (State::Choose(chooser), In::Template(name)) => chooser.chosen = Some(name),
            (State::Form(form), In::Form(input)) => {
                let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
                form.input(input, &mut rand);
                drop(rand);
                self.effects(form);
            }
            (State::Form(form), In::AddNpc) => match self.install.characters() {
                Ok(index) => form.open_npcs(
                    index
                        .entries
                        .into_iter()
                        .map(|e| (e.name, e.file))
                        .collect(),
                ),
                Err(e) => self.notice(tr("Add NPC"), e.to_string()),
            },
            (State::Form(form), In::AddTrap) => match UserDb::<Traps>::load(&self.install) {
                Ok(db) => form.open_traps(Arc::new(db)),
                Err(e) => self.notice(tr("Add Trap"), e.to_string()),
            },
            (State::Form(_), In::NpcSheet(index, print)) => {
                *self.request.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(EncounterRequest::NpcSheet {
                        index: index as u32,
                        print,
                    });
            }
            (State::Form(_), In::Treasure) => {
                *self.request.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(EncounterRequest::Treasure);
            }
            // The picker closes for the editor, as the GNOME app's does.
            (State::Form(form), In::NewRecord(section)) => {
                form.picker = None;
                *self.request.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(EncounterRequest::NewRecord {
                        section: section.to_owned(),
                    });
            }
            (State::Form(form), In::Trap(act)) => {
                let chosen = form.picker.as_ref().and_then(|p| p.chosen_trap());
                if let Some((name, origin)) = chosen {
                    form.picker = None;
                    *self.request.lock().unwrap_or_else(|e| e.into_inner()) =
                        Some(EncounterRequest::Trap {
                            act,
                            name,
                            custom: origin == Origin::Custom,
                        });
                }
            }
            _ => {}
        }
    }

    /// Does what the page asks: reads the NPC Add NPC chose, and saves.
    fn effects(&self, form: &mut EncounterForm) {
        for effect in form.take_effects() {
            match effect {
                EncounterEffect::AddNpc(file) => match self.install.character(&file) {
                    Ok(character) => form.add_npc(character),
                    Err(e) => self.notice(tr("Add NPC"), e.to_string()),
                },
                EncounterEffect::Save { old, encounter } => {
                    let said = match save(&self.install, old, *encounter) {
                        Ok(name) => {
                            form.saved_as(name.clone());
                            SaveResult::Saved { name }
                        }
                        Err(error) => {
                            form.saving = false;
                            SaveResult::Failed { error }
                        }
                    };
                    *self.saved.lock().unwrap_or_else(|e| e.into_inner()) = Some(said);
                }
            }
        }
        if let Some((title, text)) = form.notice.take() {
            self.notice(&title, text);
        }
    }
}

/// An encounter made from the template `name`, with what generating
/// said.
fn made(
    rules: &Arc<EncounterRules>,
    name: &str,
    rand: &Mutex<Rand>,
) -> (Box<EncounterForm>, Vec<String>) {
    let mut rand = rand.lock().unwrap_or_else(|e| e.into_inner());
    let made = cr_app::encounter::from_template(rules, name, &mut rand);
    let form = EncounterForm::new(rules.clone(), made.encounter, None, made.pending);
    (Box::new(form), made.messages)
}

/// Writes `encounter` in place of the saved encounter `old`, or as a
/// new one: its name, or why not.
fn save(
    install: &Installation,
    old: Option<String>,
    encounter: Encounter,
) -> Result<String, String> {
    let mut list = install.saved_encounters().map_err(|e| e.to_string())?;
    let except = old
        .as_ref()
        .and_then(|o| list.iter().position(|e| &e.name == o));
    check_name(&list, &encounter.name, except).map_err(|e| e.to_string())?;
    let name = encounter.name.clone();
    match except {
        Some(i) => list[i] = encounter,
        None => list.push(encounter),
    }
    install
        .write_saved_encounters(&list)
        .map_err(|e| e.to_string())?;
    Ok(name)
}

#[uniffi::export]
impl EncounterSession {
    /// The saved encounter's name, or New Encounter.
    pub fn title(&self) -> String {
        match &*self.state() {
            State::Choose(_) => tr("New Encounter").to_owned(),
            State::Form(form) => form.title(),
        }
    }

    /// The program help's topic.
    pub fn help(&self) -> u32 {
        match &*self.state() {
            State::Choose(_) => cr_store::help::topic::ENCOUNTERS,
            State::Form(_) => cr_store::help::topic::ENCOUNTER,
        }
    }

    /// Whether New Encounter's templates are shown, for Create
    /// Encounter.
    pub fn choosing(&self) -> bool {
        matches!(&*self.state(), State::Choose(_))
    }

    /// Whether a template, or the empty encounter, is chosen.
    pub fn can_create(&self) -> bool {
        matches!(&*self.state(), State::Choose(c) if c.chosen.is_some())
    }

    /// Create Encounter: the encounter of the template chosen, or an
    /// empty one.
    pub fn create(&self) {
        let mut state = self.state();
        let State::Choose(chooser) = &*state else {
            return;
        };
        let Some(name) = chooser.chosen.clone() else {
            return;
        };
        let (form, messages) = if name.is_empty() {
            let form =
                EncounterForm::new(self.rules.clone(), Encounter::default(), None, Vec::new());
            (Box::new(form), Vec::new())
        } else {
            made(&self.rules, &name, &self.rand)
        };
        *state = State::Form(form);
        self.notices
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend(said(messages));
    }

    /// The saved encounter's name.
    pub fn saved_name(&self) -> Option<String> {
        match &*self.state() {
            State::Form(form) => form.saved.clone(),
            State::Choose(_) => None,
        }
    }

    pub fn can_save(&self) -> bool {
        matches!(&*self.state(), State::Form(form) if !form.saving)
    }

    /// Whether closing would lose changes.
    pub fn changed(&self) -> bool {
        matches!(&*self.state(), State::Form(form) if form.changed())
    }

    /// The templates, or the encounter, as a form, with the dialog over
    /// it.
    pub fn form(&self) -> FormPage {
        let state = self.state();
        let mut builder = Builder::new();
        let dialog = match &*state {
            State::Choose(chooser) => {
                describe_chooser(&mut builder, chooser);
                None
            }
            State::Form(form) => {
                describe(&mut builder, form);
                crate::bundle::dialog(
                    &mut builder,
                    &form.items,
                    |i| input(EncounterInput::Items(i)),
                    |serial, yes| input(EncounterInput::ItemsAnswer(serial, yes)),
                )
                .or_else(|| {
                    form.picker
                        .as_ref()
                        .map(|picker| picker_dialog(&mut builder, picker))
                })
            }
        };
        let (groups, acts) = builder.finish();
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

    pub fn save(&self) {
        self.input(In::Form(EncounterInput::Save));
    }

    /// What the last Save came to, once.
    pub fn take_saved(&self) -> Option<SaveResult> {
        self.saved.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    /// What the window has to say, once.
    pub fn take_notices(&self) -> Vec<EncounterNotice> {
        std::mem::take(&mut *self.notices.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// What a button asked of the app, once.
    pub fn take_request(&self) -> Option<EncounterRequest> {
        self.request
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// The sheet of the NPC package `index`, to view or print.
    pub fn npc_sheet(&self, index: u32) -> Result<Arc<SheetSession>, CoreError> {
        let character = match &*self.state() {
            State::Form(form) => form
                .encounter
                .npcs
                .get(index as usize)
                .map(|p| p.character.clone()),
            State::Choose(_) => None,
        }
        .ok_or_else(|| CoreError::new(tr("Could Not Read")))?;
        let sheet = self.install.npc_sheet(&character)?;
        Ok(SheetSession::of(self.install.clone(), sheet))
    }

    /// Roll Treasure… on the monsters' treasure types and the
    /// encounter's treasure, with Use Treasure.
    pub fn treasure(&self) -> Option<Arc<TreasureSession>> {
        let State::Form(form) = &*self.state() else {
            return None;
        };
        Some(TreasureSession::pick(
            self.install.clone(),
            form.treasure_rules().clone(),
            form.treasure_counts(),
            form.encounter.treasure.clone(),
            self.rand.clone(),
        ))
    }

    /// The trap New Trap saved as `name`, read again, goes in the
    /// encounter, as the original puts it there.
    pub fn add_new_trap(&self, name: String) {
        let db = match UserDb::<Traps>::load(&self.install) {
            Ok(db) => db,
            Err(e) => return self.notice(tr("Add Trap"), e.to_string()),
        };
        if let State::Form(form) = &mut *self.state() {
            form.add_new_trap(&db, &name);
            if let Some((title, text)) = form.notice.take() {
                self.notice(&title, text);
            }
        }
    }

    /// The monster New Monster saved as `name` goes in the encounter,
    /// whose Add Monster lists it from now.
    pub fn add_new_monster(&self, name: String) {
        let rules = match self.install.encounter_rules() {
            Ok(rules) => Arc::new(rules),
            Err(e) => return self.notice(tr("Add Monster"), e.to_string()),
        };
        if let State::Form(form) = &mut *self.state() {
            let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
            form.add_new_monster(rules, &name, &mut rand);
        }
    }

    /// Use Treasure: the treasure of `roller` is the encounter's.
    pub fn use_treasure(&self, roller: Arc<TreasureSession>) {
        self.input(In::Form(EncounterInput::Treasure(roller.bundle())));
    }

    /// The encounter as a PDF, on Letter paper or A4.
    pub fn pdf(&self, letter: bool) -> Vec<u8> {
        let paper = if letter {
            cr_app::pdf::Paper::Letter
        } else {
            cr_app::pdf::Paper::A4
        };
        match &*self.state() {
            State::Form(form) => cr_app::pdf::sheet(&form.sheet(), paper),
            State::Choose(_) => Vec::new(),
        }
    }

    /// Deletes the saved encounter.
    pub fn delete_saved(&self) -> Result<(), CoreError> {
        let Some(name) = self.saved_name() else {
            return Ok(());
        };
        let mut list = self.install.saved_encounters()?;
        list.retain(|e| e.name != name);
        Ok(self.install.write_saved_encounters(&list)?)
    }
}

/// New Encounter's page: the search, and the templates.
fn describe_chooser(form: &mut Builder<In>, chooser: &Chooser) {
    form.group(
        tr("Templates"),
        tr("Choose a template to generate the encounter from"),
    )
    .text(
        tr("Search Templates"),
        &chooser.search,
        false,
        Vec::new(),
        In::Search,
    );
    let rows = chooser.shown();
    if rows.is_empty() {
        form.note(tr("No Results"));
        return;
    }
    let shown: Vec<(String, String)> = rows.iter().take(MOST_SHOWN).cloned().collect();
    let names: Vec<String> = shown.iter().map(|(name, _)| name.clone()).collect();
    let options: Vec<(String, String)> = shown
        .into_iter()
        .map(|(name, kind)| {
            if name.is_empty() {
                (
                    tr("Empty Encounter").to_owned(),
                    tr("Add monsters and characters yourself").to_owned(),
                )
            } else {
                (name, kind)
            }
        })
        .collect();
    let selected = chooser
        .chosen
        .as_ref()
        .and_then(|c| names.iter().position(|n| n == c));
    form.radio(&options, selected, move |i| {
        In::Template(names.get(i).cloned().unwrap_or_default())
    });
    more_shown(form, rows.len());
}

/// What a list cut at [`MOST_SHOWN`] says under it.
pub(crate) fn more_shown<I: 'static>(form: &mut Builder<I>, count: usize) {
    if count > MOST_SHOWN {
        form.row(crate::form::FormRow::Note {
            text: tr!(
                "Showing {shown} of {count}; search to narrow the list",
                shown = MOST_SHOWN,
                count
            ),
            secondary: true,
        });
    }
}

/// The Encounter page.
fn describe(form: &mut Builder<In>, f: &EncounterForm) {
    let encounter = &f.encounter;
    let template = if encounter.template.is_empty() {
        tr("None").to_owned()
    } else {
        encounter.template.clone()
    };
    form.group(tr("Encounter"), "")
        .text(tr("Name"), &encounter.name, false, Vec::new(), |text| {
            input(EncounterInput::Name(text))
        })
        .value(tr("Template"), "", &template)
        .text(
            tr("Monster Type"),
            &encounter.monster_type,
            false,
            Vec::new(),
            |text| input(EncounterInput::MonsterType(text)),
        );
    form.group(tr("Encounter Totals"), "");
    for (title, value) in f.totals() {
        form.value(title, "", &value);
    }

    let add_monster = form.button(
        tr("Add _Monster…"),
        "",
        true,
        input(EncounterInput::AddMonster),
    );
    let add_npc = form.button(tr("Add _NPC…"), "", true, In::AddNpc);
    form.group(tr("Monsters and Nonplayer Characters"), "")
        .heading_button(add_monster)
        .heading_button(add_npc);
    let rows = f.rows();
    if rows.is_empty() {
        form.row(crate::form::FormRow::Note {
            text: tr("No monsters or characters").to_owned(),
            secondary: true,
        });
    }
    for row in &rows {
        package(form, row);
    }

    let pending = f.pending_rows();
    if !pending.is_empty() {
        form.group(
            tr("Characters Not Generated"),
            tr(
                "The template also calls for these nonplayer characters, but the NPC generator \
                 could not make them, for the reason under each one. Add saved characters with \
                 Add NPC instead.",
            ),
        );
        for (title, subtitle) in &pending {
            form.value(title, subtitle, "");
        }
    }

    let roll = form.button(tr("_Roll Treasure…"), "", true, In::Treasure);
    form.group(tr("Treasure"), &f.treasure_note())
        .heading_button(roll);
    if !encounter.treasure.is_empty() {
        crate::bundle::rows(
            form,
            &cr_app::bundle::rows(&encounter.treasure),
            false,
            |i| input(EncounterInput::Items(i)),
        );
    }

    let add_item = form.button(
        tr("_Add Item…"),
        "",
        true,
        input(EncounterInput::Items(BundleInput::Add)),
    );
    let caption = if encounter.items.is_empty() {
        tr("Items found with the encounter")
    } else {
        ""
    };
    form.group(tr("Items"), caption).heading_button(add_item);
    if !encounter.items.is_empty() {
        crate::bundle::rows(form, &cr_app::bundle::rows(&encounter.items), true, |i| {
            input(EncounterInput::Items(i))
        });
    }

    let add_trap = form.button(tr("_Add Trap…"), "", true, In::AddTrap);
    let caption = if encounter.traps.is_empty() {
        tr("No traps")
    } else {
        ""
    };
    form.group(tr("Traps"), caption).heading_button(add_trap);
    for trap in &encounter.traps {
        let remove = form.button(
            tr("Remove Trap"),
            "minus.circle",
            true,
            input(EncounterInput::RemoveTrap(trap.name.clone())),
        );
        form.row(crate::form::FormRow::Tree {
            title: trap.name.clone(),
            subtitle: trap.damage.clone(),
            depth: 0,
            dimmed: false,
            buttons: vec![remove],
        });
    }

    form.group(tr("Description"), "").text(
        tr("Description"),
        &encounter.description,
        true,
        Vec::new(),
        |text| input(EncounterInput::Description(text)),
    );
}

/// A package's group: its title and buttons, its number, a monster
/// package's hit points, and its stat line.
fn package(form: &mut Builder<In>, row: &cr_app::encounter::PackageRow) {
    let package = row.package;
    form.group(&row.title, &row.subtitle);
    if let Package::Npc(i) = package {
        // View NPC… and Print NPC… (0x63EA49, 0x63EB3A): the NPC's
        // character sheet.
        let view = form.button(tr("View NPC"), "doc.richtext", true, In::NpcSheet(i, false));
        let print = form.button(tr("Print NPC"), "printer", true, In::NpcSheet(i, true));
        form.heading_button(view).heading_button(print);
    }
    let remove = form.button(
        tr("Remove"),
        "minus.circle",
        true,
        input(EncounterInput::Remove(package)),
    );
    form.heading_button(remove);
    let note = if matches!(package, Package::Monster(_)) {
        tr("Changing it rolls the hit points again")
    } else {
        ""
    };
    form.number(
        (tr("Number"), note),
        row.count.into(),
        (1, MOST_COUNT.into()),
        Vec::new(),
        move |n| input(EncounterInput::Count(package, n as i32)),
    );
    // The Hit Points group of "Monsters for the Encounter" (IDD 292): a
    // value, how many monsters have it, and Re-roll.
    if let (Package::Monster(i), Some(hp)) = (package, &row.hit_points) {
        let reroll = form.button(tr("_Re-roll"), "", true, input(EncounterInput::Reroll(i)));
        form.number(
            (tr("Hit Points"), ""),
            hp.value.into(),
            (hp.low.into(), hp.high.max(hp.low).into()),
            vec![reroll],
            move |v| input(EncounterInput::HitPoints(i, v as i32)),
        )
        .number(
            (tr("Monsters"), &hp.note()),
            hp.with.into(),
            (0, hp.most_with.max(0).into()),
            Vec::new(),
            move |n| input(EncounterInput::With(i, n as i32)),
        );
    }
    form.note(&row.stats);
}

/// Add Monster, Add NPC or Add Trap: a search over the list, and Add
/// once a name is chosen.
fn picker_dialog(form: &mut Builder<In>, picker: &Picker) -> FormDialog {
    let page = form.take_groups();
    form.group("", "");
    if let Some(filter) = picker.filter() {
        form.choice(
            (tr("Show Standard or Custom Traps"), ""),
            Filter::NAMES.iter().map(|n| tr(n).to_owned()).collect(),
            Some(filter),
            Vec::new(),
            |i| input(EncounterInput::PickFilter(i)),
        );
    }
    form.text(
        picker.placeholder(),
        &picker.search,
        false,
        Vec::new(),
        |text| input(EncounterInput::PickSearch(text)),
    );
    // New Trap, and Edit, Copy and Delete on the trap chosen (the
    // standard ones are only copied); New Monster.
    match &picker.pick {
        Pick::Trap(..) => {
            let chosen = picker.chosen_trap().map(|(_, origin)| origin);
            let custom = chosen == Some(Origin::Custom);
            let buttons = [
                (tr("New Trap"), true, In::NewRecord("traps")),
                (tr("_Edit…"), custom, In::Trap(RecordAct::Edit)),
                (tr("_Copy…"), chosen.is_some(), In::Trap(RecordAct::Copy)),
                (tr("_Delete"), custom, In::Trap(RecordAct::Delete)),
            ]
            .map(|(label, enabled, input)| form.button(label, "", enabled, input));
            form.row(FormRow::Value {
                title: tr("Traps").to_owned(),
                subtitle: String::new(),
                value: String::new(),
                buttons: buttons.to_vec(),
            });
        }
        Pick::Monster => {
            let new = form.button(tr("New Monster"), "", true, In::NewRecord("monsters"));
            form.button_row(tr("Monsters"), "", new);
        }
        Pick::Npc(_) => {}
    }
    let shown = picker.shown();
    let items: Vec<(String, String)> = shown
        .iter()
        .take(MOST_SHOWN)
        .map(|&i| picker.items[i].clone())
        .collect();
    let names: Vec<String> = items.iter().map(|(name, _)| name.clone()).collect();
    let selected = picker
        .chosen
        .as_ref()
        .and_then(|c| names.iter().position(|n| n == c));
    if picker.items.is_empty() {
        form.note(tr("Nothing to Add"));
    } else if items.is_empty() {
        form.note(tr("No Results"));
    } else {
        form.radio(&items, selected, move |i| {
            input(EncounterInput::PickChoose(
                names.get(i).cloned().unwrap_or_default(),
            ))
        });
        more_shown(form, shown.len());
    }
    let close = form.button(
        tr("_Cancel"),
        "",
        true,
        input(EncounterInput::PickAnswer(false)),
    );
    let accept = form.button(
        tr("_Add"),
        "",
        selected.is_some(),
        input(EncounterInput::PickAnswer(true)),
    );
    let groups = form.take_groups();
    form.put_groups(page);
    FormDialog {
        title: picker.title().to_owned(),
        groups,
        close,
        accept: Some(accept),
    }
}

/// What Roll on the table editor generates: the encounter of a
/// template, saying that.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TableGenerate {
    pub template: String,
    pub said: String,
}

/// The 2-20 table editor, open in its window.
#[derive(uniffi::Object)]
pub struct TableEditSession {
    install: Arc<Installation>,
    editor: Mutex<TableEditor>,
    acts: Mutex<Acts<TableEditInput>>,
    rand: Arc<Mutex<Rand>>,
    notices: Mutex<Vec<String>>,
    generate: Mutex<Option<TableGenerate>>,
    saved: Mutex<Option<SaveResult>>,
}

impl TableEditSession {
    /// The editor on the saved table `name`, or on a new one.
    pub(crate) fn open(
        install: Arc<Installation>,
        rules: Arc<EncounterRules>,
        name: Option<String>,
        rand: Arc<Mutex<Rand>>,
    ) -> Result<Arc<Self>, CoreError> {
        let table = match name {
            Some(name) => Some(
                install
                    .frequency_tables()?
                    .table(&name)
                    .cloned()
                    .ok_or_else(|| CoreError::new(tr("Could Not Read")))?,
            ),
            None => None,
        };
        Ok(Arc::new(Self {
            install,
            editor: Mutex::new(TableEditor::new(rules, table)),
            acts: Mutex::default(),
            rand,
            notices: Mutex::default(),
            generate: Mutex::default(),
            saved: Mutex::default(),
        }))
    }

    fn editor(&self) -> MutexGuard<'_, TableEditor> {
        self.editor.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn input(&self, input: TableEditInput) {
        let mut editor = self.editor();
        let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
        editor.input(input, &mut rand);
        drop(rand);
        for effect in editor.take_effects() {
            match effect {
                TableEffect::Notice(text) => self
                    .notices
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(text),
                TableEffect::Generate { template, said } => {
                    *self.generate.lock().unwrap_or_else(|e| e.into_inner()) =
                        Some(TableGenerate { template, said })
                }
                TableEffect::Save { old, table } => {
                    let said = match save_table(&self.install, old, *table) {
                        Ok(name) => {
                            editor.saved_as(name.clone());
                            SaveResult::Saved { name }
                        }
                        Err(error) => {
                            editor.saving = false;
                            editor.error = Some(error.clone());
                            SaveResult::Failed { error }
                        }
                    };
                    *self.saved.lock().unwrap_or_else(|e| e.into_inner()) = Some(said);
                }
            }
        }
    }
}

/// Writes `table` in place of the saved table `old`, or as a new one:
/// its name, or why not.
fn save_table(
    install: &Installation,
    old: Option<String>,
    table: FrequencyTable,
) -> Result<String, String> {
    let mut tables = install.frequency_tables().map_err(|e| e.to_string())?;
    let clash = tables
        .tables
        .iter()
        .any(|x| x.name.eq_ignore_ascii_case(&table.name) && Some(&x.name) != old.as_ref());
    if clash {
        return Err(tr("Name already used.").to_owned());
    }
    let name = table.name.clone();
    match old
        .as_ref()
        .and_then(|o| tables.tables.iter().position(|x| &x.name == o))
    {
        Some(i) => tables.tables[i] = table,
        None => tables.tables.push(table),
    }
    install
        .write_frequency_tables(&tables)
        .map_err(|e| e.to_string())?;
    Ok(name)
}

#[uniffi::export]
impl TableEditSession {
    /// The saved table's name, or New Encounter Table.
    pub fn title(&self) -> String {
        self.editor().title()
    }

    /// The program help's topic.
    pub fn help(&self) -> u32 {
        cr_store::help::topic::ENCOUNTER_TABLE
    }

    pub fn saved_name(&self) -> Option<String> {
        self.editor().saved.clone()
    }

    pub fn can_save(&self) -> bool {
        !self.editor().saving
    }

    /// Whether closing would lose changes.
    pub fn changed(&self) -> bool {
        self.editor().changed()
    }

    /// The table as a form: its name, the filter, the rolls, and the
    /// dialog that picks a roll's template.
    pub fn form(&self) -> FormPage {
        let editor = self.editor();
        let mut form = Builder::new();
        describe_table(&mut form, &editor);
        let dialog = editor
            .picking
            .as_ref()
            .map(|pick| pick_dialog(&mut form, pick));
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

    pub fn save(&self) {
        self.input(TableEditInput::Save);
    }

    /// Fill: templates that pass the filter, by frequency.
    pub fn fill(&self) {
        self.input(TableEditInput::Fill);
    }

    /// Roll: the encounter of the template rolled
    /// ([`TableEditSession::take_generate`]).
    pub fn roll(&self) {
        self.input(TableEditInput::Roll);
    }

    pub fn take_saved(&self) -> Option<SaveResult> {
        self.saved.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    /// What the editor has to say, once.
    pub fn take_notices(&self) -> Vec<String> {
        std::mem::take(&mut *self.notices.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// The encounter Roll asks for, once.
    pub fn take_generate(&self) -> Option<TableGenerate> {
        self.generate
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// Deletes the saved table.
    pub fn delete_saved(&self) -> Result<(), CoreError> {
        let Some(name) = self.saved_name() else {
            return Ok(());
        };
        let mut tables = self.install.frequency_tables()?;
        tables.tables.retain(|t| t.name != name);
        Ok(self.install.write_frequency_tables(&tables)?)
    }
}

/// The table editor's page.
fn describe_table(form: &mut Builder<TableEditInput>, editor: &TableEditor) {
    let filter = &editor.table.filter;
    form.group("", "");
    if let Some(error) = &editor.error {
        form.banner(error, false);
    }
    form.text(
        tr("Name"),
        &editor.table.name,
        false,
        Vec::new(),
        TableEditInput::Name,
    );
    form.group(
        tr("Filter"),
        tr("Fill picks templates that pass the filter, by frequency"),
    )
    .choice(
        (tr("Database"), ""),
        SOURCES.iter().map(|s| tr(s).to_owned()).collect(),
        Some(editor.source()),
        Vec::new(),
        TableEditInput::Source,
    )
    .number(
        (tr("Fewest Hit Dice"), ""),
        filter.hit_dice.0.into(),
        (0, 100),
        Vec::new(),
        |n| TableEditInput::Fewest(n as i32),
    )
    .number(
        (tr("Most Hit Dice"), ""),
        filter.hit_dice.1.into(),
        (0, 100),
        Vec::new(),
        |n| TableEditInput::Most(n as i32),
    )
    .choice(
        (tr("Lair and/or Wandering"), ""),
        LAIR_MODES.iter().map(|s| tr(s).to_owned()).collect(),
        Some(editor.lair()),
        Vec::new(),
        TableEditInput::Lair,
    );
    let places: Vec<(String, String, bool, bool)> = CLIMATES
        .iter()
        .chain(TERRAINS.iter())
        .enumerate()
        .map(|(i, place)| {
            (
                tr(place).to_owned(),
                String::new(),
                filter.climates.get(i).is_some_and(|&f| f != 0),
                true,
            )
        })
        .collect();
    form.group(
        tr("Climate and Terrain"),
        tr("Where the templates may be encountered"),
    )
    .checks(&places, TableEditInput::Place);
    form.group(
        tr("Rolls"),
        tr("Pick a roll to choose its template from those that pass the filter"),
    );
    for (i, name) in editor.slots().into_iter().enumerate() {
        let choose = form.button(tr("_Choose…"), "", true, TableEditInput::PickRoll(i));
        form.row(crate::form::FormRow::Value {
            title: (i + 2).to_string(),
            subtitle: String::new(),
            value: name,
            buttons: vec![choose],
        });
    }
}

/// Picking the template of a roll: the templates of a frequency that
/// pass the filter, or (None), and OK once one is chosen.
fn pick_dialog(
    form: &mut Builder<TableEditInput>,
    pick: &cr_app::encounter::RollPick,
) -> FormDialog {
    let page = form.take_groups();
    let mut names = vec![String::new()];
    names.extend(pick.names.iter().cloned());
    let selected = pick
        .chosen
        .as_ref()
        .and_then(|c| names.iter().position(|n| n == c));
    form.group("", "").choice(
        (tr("Frequency"), ""),
        FREQUENCIES.iter().map(|f| tr(f).to_owned()).collect(),
        Some(pick.frequency),
        Vec::new(),
        TableEditInput::PickFrequency,
    );
    if pick.names.is_empty() {
        form.row(crate::form::FormRow::Note {
            text: tr("None of this frequency pass the filter").to_owned(),
            secondary: true,
        });
    }
    let options: Vec<(String, String)> = names
        .iter()
        .map(|n| {
            let shown = if n.is_empty() { tr("(None)") } else { n };
            (shown.to_owned(), String::new())
        })
        .collect();
    form.radio(&options, selected, move |i| {
        TableEditInput::PickChoose(names.get(i).cloned().unwrap_or_default())
    });
    let close = form.button(tr("_Cancel"), "", true, TableEditInput::PickAnswer(false));
    let accept = form.button(
        tr("_OK"),
        "",
        selected.is_some(),
        TableEditInput::PickAnswer(true),
    );
    let groups = form.take_groups();
    form.put_groups(page);
    FormDialog {
        title: tr!("Roll {roll}", roll = pick.index + 2),
        groups,
        close,
        accept: Some(accept),
    }
}
