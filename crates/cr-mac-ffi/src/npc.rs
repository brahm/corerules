//! New NPC and New NPC Group, as the Mac app draws them: the forms of
//! `cr_app::npc` described as forms ([`crate::form`]), as the Windows
//! app's pages draw them, and Create making and saving the NPCs. New NPC
//! chooses the race and class combination, a level for each class, the
//! rules and the ability scores Roll rolls; New NPC Group adds lines of a
//! number, race, classes, kit, alignment and levels.

use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::npc::{GROUPS, GroupForm, GroupInput, MOST_LEVEL, MOST_NUMBER, NpcForm, NpcInput};
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::create::ABILITIES;
use cr_store::npc::{Npc, NpcRules};
use cr_store::treasure::Rand;

use crate::CoreError;
use crate::form::{Acts, Builder, FormPage, FormValue};

/// What Create made.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NpcsCreated {
    /// The file of the last NPC saved, to select it.
    pub file: Option<String>,
    /// "Kara was created", or how many were.
    pub said: String,
    /// The generator's notes on what it could not do.
    pub messages: Option<String>,
}

#[derive(Clone)]
enum Form {
    One(Box<NpcForm>),
    Group(Box<GroupForm>),
}

#[derive(Debug, Clone, PartialEq)]
enum Change {
    Npc(NpcInput),
    Group(GroupInput),
}

/// New NPC or New NPC Group, open in a sheet.
#[derive(uniffi::Object)]
pub struct NpcSession {
    install: Arc<Installation>,
    form: Mutex<Form>,
    /// What the controls of the form last described do.
    acts: Mutex<Acts<Change>>,
    rand: Arc<Mutex<Rand>>,
}

impl NpcSession {
    pub(crate) fn open(
        install: Arc<Installation>,
        rules: Arc<NpcRules>,
        group: bool,
        rand: Arc<Mutex<Rand>>,
    ) -> Arc<Self> {
        let form = if group {
            Form::Group(Box::new(GroupForm::new(rules)))
        } else {
            Form::One(Box::new(NpcForm::new(rules, false)))
        };
        Arc::new(Self {
            install,
            form: Mutex::new(form),
            acts: Mutex::default(),
            rand,
        })
    }

    fn form_lock(&self) -> MutexGuard<'_, Form> {
        self.form.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[uniffi::export]
impl NpcSession {
    pub fn title(&self) -> String {
        match &*self.form_lock() {
            Form::One(_) => tr("New NPC"),
            Form::Group(_) => tr("New NPC Group"),
        }
        .to_owned()
    }

    /// The program help's topic.
    pub fn help(&self) -> u32 {
        match &*self.form_lock() {
            Form::One(_) => cr_store::help::topic::NPC_CREATION,
            Form::Group(_) => cr_store::help::topic::NPC_GROUP,
        }
    }

    /// The form, as it is now.
    pub fn form(&self) -> FormPage {
        let mut builder = Builder::new();
        match &*self.form_lock() {
            Form::One(form) => npc(&mut builder, form, Change::Npc),
            Form::Group(form) => group(&mut builder, form),
        }
        let (groups, acts) = builder.finish();
        *self.acts.lock().unwrap_or_else(|e| e.into_inner()) = acts;
        FormPage {
            groups,
            dialog: None,
        }
    }

    /// Control `id` of the form last described changed to `value`.
    pub fn act(&self, id: u32, value: FormValue) {
        let change = self
            .acts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .input(id, &value);
        let mut form = self.form_lock();
        match (change, &mut *form) {
            (Some(Change::Npc(input)), Form::One(form)) => {
                let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
                form.input(input, &mut rand);
            }
            (Some(Change::Group(input)), Form::Group(form)) => form.input(input),
            _ => {}
        }
    }

    /// Create: makes the NPCs and saves them, in the party or group
    /// named. Takes a moment; the app calls it off the main thread.
    pub fn create(&self) -> Result<NpcsCreated, CoreError> {
        let form = self.form_lock().clone();
        let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
        let (npcs, party) = match &form {
            Form::One(form) => (
                form.generate(&mut rand).map(|npc| vec![npc]),
                form.party.trim().to_owned(),
            ),
            Form::Group(form) => (form.generate(&mut rand), form.name.trim().to_owned()),
        };
        drop(rand);
        let npcs: Vec<Npc> = npcs.map_err(CoreError::new)?;
        let mut file = None;
        for npc in &npcs {
            file = Some(
                self.install
                    .save_npc(npc, &party)
                    .map_err(|e| CoreError::new(e.to_string()))?,
            );
        }
        let said = match (&form, npcs.first()) {
            (Form::One(_), Some(npc)) => {
                tr!("{name} was created", name = npc.character.data.name.clone())
            }
            _ => cr_app::npc::created_text(npcs.len()),
        };
        Ok(NpcsCreated {
            file,
            said,
            messages: cr_app::npc::messages(&npcs),
        })
    }
}

/// The Rules choice: the Core Rules or Player's Option.
fn rules<I: 'static>(
    form: &mut Builder<I>,
    players_option: bool,
    make: impl Fn(bool) -> I + Send + Sync + 'static,
) {
    form.choice(
        (tr("Rules"), ""),
        vec![tr("Core Rules").to_owned(), "Player's Option".to_owned()],
        Some(usize::from(players_option)),
        Vec::new(),
        move |i| make(i == 1),
    );
}

/// What stopped the form, over its buttons.
fn error<I: 'static>(form: &mut Builder<I>, error: &Option<String>) {
    if let Some(text) = error {
        form.group("", "").banner(text, true);
    }
}

/// A choice that may be off, shown as its value while it is.
fn maybe_choice(
    form: &mut Builder<Change>,
    title: &str,
    items: Vec<String>,
    selected: usize,
    enabled: bool,
    make: impl Fn(usize) -> Change + Send + Sync + 'static,
) {
    if enabled {
        form.choice((title, ""), items, Some(selected), Vec::new(), make);
    } else {
        let value = items.get(selected).cloned().unwrap_or_default();
        form.value(title, "", &value);
    }
}

/// The New NPC form.
pub(crate) fn npc<I: Clone + Send + Sync + 'static>(
    form: &mut Builder<I>,
    npc: &NpcForm,
    wrap: fn(NpcInput) -> I,
) {
    form.group(tr("Character"), "")
        .choice(
            (tr("Race"), ""),
            npc.races().to_vec(),
            Some(npc.race_index()),
            Vec::new(),
            move |i| wrap(NpcInput::Race(i)),
        )
        .toggle(
            (tr("All Races"), tr("Include races that are not standard")),
            npc.all_races,
            move |on| wrap(NpcInput::AllRaces(on)),
        )
        .choice(
            (tr("Class"), ""),
            npc.class_names(),
            Some(npc.class_index()),
            Vec::new(),
            move |i| wrap(NpcInput::Class(i)),
        )
        .toggle(
            (
                tr("All Classes"),
                tr("Include classes the race may not take"),
            ),
            npc.all_classes,
            move |on| wrap(NpcInput::AllClasses(on)),
        );
    rules(form, npc.players_option, move |on| {
        wrap(NpcInput::PlayersOption(on))
    });

    // A level for each class of the combination, named for it.
    form.group(tr("Levels"), "");
    for (i, (class, level)) in npc.classes().iter().zip(&npc.levels).enumerate() {
        form.number(
            (class, ""),
            (*level).max(1).into(),
            (1, MOST_LEVEL.into()),
            Vec::new(),
            move |v| wrap(NpcInput::Level(i, v as i32)),
        );
    }

    // In template mode the NPC has no ability scores (0x4bd39a).
    if !npc.template {
        abilities(form, npc, wrap);
    }

    if npc.party_shown() {
        form.group("", "")
            .text(tr("Party"), &npc.party, false, Vec::new(), move |text| {
                wrap(NpcInput::Party(text))
            });
    }
    error(form, &npc.error);
}

/// New NPC's ability scores, which Roll rolls.
fn abilities<I: Clone + Send + Sync + 'static>(
    form: &mut Builder<I>,
    npc: &NpcForm,
    wrap: fn(NpcInput) -> I,
) {
    let description = if npc.abilities.is_some() {
        tr("The NPC keeps these scores")
    } else {
        tr("Rolled with the NPC unless you roll them now")
    };
    form.group(tr("Ability Scores"), description);
    let roll = form.button(tr("_Roll"), "", true, wrap(NpcInput::Roll));
    form.heading_button(roll);
    if npc.abilities.is_some() {
        let clear = form.button(tr("_Clear"), "", true, wrap(NpcInput::Clear));
        form.heading_button(clear);
    }
    match npc.abilities {
        Some(rolled) => {
            for (i, (name, (lo, hi))) in ABILITIES.iter().zip(npc.ability_ranges()).enumerate() {
                form.number(
                    (tr(name), ""),
                    rolled.scores[i].into(),
                    (lo.into(), hi.max(lo).into()),
                    Vec::new(),
                    move |v| wrap(NpcInput::Ability(i, v as i32)),
                );
            }
            if npc.exceptional_shown() {
                form.number(
                    (
                        tr("Exceptional Strength"),
                        tr("1 to 100 for 18/01 to 18/00"),
                    ),
                    rolled.exceptional.into(),
                    (1, 100),
                    Vec::new(),
                    move |v| wrap(NpcInput::Exceptional(v as i32)),
                );
            }
        }
        None => {
            form.none();
        }
    }
}

/// The New NPC Group form.
fn group(form: &mut Builder<Change>, group: &GroupForm) {
    form.group(tr("Group"), "")
        .text(tr("Group Name"), &group.name, false, Vec::new(), |text| {
            Change::Group(GroupInput::Name(text))
        })
        .choice(
            (tr("Race"), ""),
            group.race_items(),
            Some(group.race_index()),
            Vec::new(),
            |i| Change::Group(GroupInput::Race(i)),
        )
        .toggle(
            (tr("All Races"), tr("Include races that are not standard")),
            group.all_races,
            |on| Change::Group(GroupInput::AllRaces(on)),
        )
        .number(
            (tr("Fewest"), ""),
            group.number.0.max(1).into(),
            (1, MOST_NUMBER.into()),
            Vec::new(),
            |v| Change::Group(GroupInput::Fewest(v as i32)),
        )
        .number(
            (tr("Most"), ""),
            group.number.1.max(1).into(),
            (1, MOST_NUMBER.into()),
            Vec::new(),
            |v| Change::Group(GroupInput::Most(v as i32)),
        );
    rules(form, group.players_option, |on| {
        Change::Group(GroupInput::PlayersOption(on))
    });

    form.group(tr("Classes"), tr("A class of each group, or none"))
        .toggle(
            (
                tr("Allowed Combination"),
                tr("Each NPC takes a class combination its race allows"),
            ),
            group.allowed,
            |on| Change::Group(GroupInput::Allowed(on)),
        );
    for (g, name) in GROUPS.iter().enumerate() {
        maybe_choice(
            form,
            tr(name),
            group.class_items(g),
            group.class_index(g),
            group.classes_enabled(),
            move |i| Change::Group(GroupInput::Class(g, i)),
        );
    }
    form.choice(
        (tr("Kit"), ""),
        group.kit_items(),
        Some(group.kit_index()),
        Vec::new(),
        |i| Change::Group(GroupInput::Kit(i)),
    )
    .toggle(
        (
            tr("All Kits"),
            tr("Include kits the race, classes or alignment may not take"),
        ),
        group.all_kits,
        |on| Change::Group(GroupInput::AllKits(on)),
    );
    maybe_choice(
        form,
        tr("Alignment"),
        group.alignment_items(),
        group.alignment_index(),
        group.alignment_enabled(),
        |i| Change::Group(GroupInput::Alignment(i)),
    );
    form.toggle(
        (
            tr("Any Alignment"),
            tr("Include alignments the classes, race or kit do not allow"),
        ),
        group.any_alignment,
        |on| Change::Group(GroupInput::AnyAlignment(on)),
    );

    form.group(
        tr("Levels"),
        tr("Lowest and highest level for each class group"),
    );
    for (g, (name, (low, high))) in GROUPS.iter().zip(group.levels).enumerate() {
        let name = tr(name);
        form.number(
            (&tr!("Lowest {name} Level", name), ""),
            low.max(1).into(),
            (1, MOST_LEVEL.into()),
            Vec::new(),
            move |v| Change::Group(GroupInput::Low(g, v as i32)),
        )
        .number(
            (&tr!("Highest {name} Level", name), ""),
            high.max(1).into(),
            (1, MOST_LEVEL.into()),
            Vec::new(),
            move |v| Change::Group(GroupInput::High(g, v as i32)),
        );
    }

    form.group(
        tr("Lines"),
        tr("Without lines, Create makes the line above"),
    );
    let add = form.button(
        tr("_Add Line"),
        "",
        true,
        Change::Group(GroupInput::AddLine),
    );
    form.heading_button(add);
    let lines = group.lines();
    if lines.is_empty() {
        form.none();
    }
    for line in &lines {
        form.note(line);
    }
    error(form, &group.error);
}
