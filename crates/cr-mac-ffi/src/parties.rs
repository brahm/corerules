//! Manage Parties as the Mac app draws it in a sheet: the lists of
//! `cr_app::parties` described as a form ([`crate::form`]), as the
//! Windows app's page has them. The party chosen and New Party over the
//! party's members and the characters in no party, each with the button
//! that moves it to the other list; Group Encounter's dialog, and Delete
//! Members.

use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::parties::{Member, PartyLists};
use cr_i18n::{tr, trn};
use cr_store::Installation;

use crate::CoreError;
use crate::form::{Acts, Builder, FormDialog, FormPage, FormRow, FormValue};

/// Group Encounter, as OK asks for it: an encounter of the party's NPCs
/// to make and save, or to print.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GroupRequest {
    pub party: String,
    /// The NPCs' files, in order.
    pub members: Vec<String>,
    pub print: bool,
    /// One stat line for each NPC.
    pub condensed: bool,
    /// Show the page before printing it.
    pub preview: bool,
}

#[derive(Clone)]
enum In {
    Party(String),
    Name(String),
    NewParty,
    /// A character's file, and the party it moves to; none when empty.
    Move(String, String),
    Group,
    GroupPrint(bool),
    Condensed(bool),
    Preview(bool),
    GroupAnswer(bool),
}

struct State {
    lists: PartyLists,
    /// The name typed in New Party, and why it was refused.
    name: String,
    problem: Option<String>,
    /// Group Encounter's dialog, while it is open.
    group: Option<GroupRequest>,
    /// What its OK asked for, until the app takes it.
    asked: Option<GroupRequest>,
}

/// Manage Parties, open in its sheet.
#[derive(uniffi::Object)]
pub struct PartiesSession {
    install: Arc<Installation>,
    state: Mutex<State>,
    acts: Mutex<Acts<In>>,
    /// What went wrong last, until the app takes it.
    error: Mutex<Option<String>>,
}

impl PartiesSession {
    pub(crate) fn open(install: Arc<Installation>) -> Result<Arc<Self>, CoreError> {
        let lists = PartyLists::read(&install, "", &[])?;
        Ok(Arc::new(Self {
            install,
            state: Mutex::new(State {
                lists,
                name: String::new(),
                problem: None,
                group: None,
                asked: None,
            }),
            acts: Mutex::default(),
            error: Mutex::default(),
        }))
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn fail(&self, error: impl ToString) {
        *self.error.lock().unwrap_or_else(|e| e.into_inner()) = Some(error.to_string());
    }

    /// Reads the lists again, on the party `selected`.
    fn read(&self, state: &mut State, selected: &str) {
        match PartyLists::read(&self.install, selected, &state.lists.new) {
            Ok(lists) => state.lists = lists,
            Err(e) => self.fail(e),
        }
    }

    fn input(&self, input: In) {
        let mut state = self.state();
        match input {
            In::Party(party) => {
                if !party.eq_ignore_ascii_case(&state.lists.selected) {
                    self.read(&mut state, &party);
                }
            }
            In::Name(name) => {
                state.name = name;
                state.problem = None;
            }
            In::NewParty => {
                let name = state.name.trim().to_owned();
                match self.install.party_name_problem(&name, &state.lists.new) {
                    Ok(None) => {
                        let mut new = state.lists.new.clone();
                        new.push(name.clone());
                        match PartyLists::read(&self.install, &name, &new) {
                            Ok(lists) => {
                                state.lists = lists;
                                state.name.clear();
                            }
                            Err(e) => self.fail(e),
                        }
                    }
                    Ok(Some(problem)) => state.problem = Some(problem),
                    Err(e) => self.fail(e),
                }
            }
            In::Move(file, party) => match self.install.set_party(&file, &party) {
                Ok(_) => {
                    let selected = state.lists.selected.clone();
                    self.read(&mut state, &selected);
                }
                Err(e) => self.fail(e),
            },
            In::Group => {
                state.group = Some(GroupRequest {
                    party: state.lists.selected.clone(),
                    members: state.lists.npcs.clone(),
                    print: false,
                    condensed: false,
                    preview: false,
                });
            }
            In::GroupPrint(print) => {
                if let Some(group) = &mut state.group {
                    group.print = print;
                }
            }
            In::Condensed(on) => {
                if let Some(group) = &mut state.group {
                    group.condensed = on;
                }
            }
            In::Preview(on) => {
                if let Some(group) = &mut state.group {
                    group.preview = on;
                }
            }
            In::GroupAnswer(yes) => {
                let group = state.group.take();
                if yes {
                    state.asked = group;
                }
            }
        }
    }
}

#[uniffi::export]
impl PartiesSession {
    /// The program help's topic.
    pub fn help(&self) -> u32 {
        cr_store::help::topic::PARTY_MANAGEMENT
    }

    /// The party chosen, empty when there is none.
    pub fn party(&self) -> String {
        self.state().lists.selected.clone()
    }

    /// Whether the party chosen has members, for Delete Members.
    pub fn has_members(&self) -> bool {
        !self.state().lists.members.is_empty()
    }

    /// The lists as a form, with Group Encounter's dialog over them.
    pub fn form(&self) -> FormPage {
        let state = self.state();
        let mut form = Builder::new();
        describe(&mut form, &state);
        let dialog = state
            .group
            .as_ref()
            .map(|group| group_dialog(&mut form, group));
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

    /// What Group Encounter's OK asked for, once.
    pub fn take_group(&self) -> Option<GroupRequest> {
        self.state().asked.take()
    }

    /// What went wrong last, once.
    pub fn take_error(&self) -> Option<String> {
        self.error.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    /// Delete Members: deletes every character of the party chosen, and
    /// the party with them; what the app says once it is done.
    pub fn delete_members(&self) -> Result<String, CoreError> {
        let party = self.party();
        let deleted = self.install.delete_party(&party)?;
        // Without the GTK app's Undo, the kept files go at once, as when
        // its toast closes.
        self.install.forget_deleted(&deleted)?;
        let n = deleted.len();
        Ok(trn!(
            "{n} character of “{party}” deleted",
            "{n} characters of “{party}” deleted",
            n,
            party
        ))
    }
}

/// The rows of `members`, each with the button that moves it to `to`
/// (no party when empty); none without a party chosen.
fn members(form: &mut Builder<In>, members: &[Member], button: Option<(&str, &str, &str)>) {
    for member in members {
        let buttons = match button {
            Some((label, icon, to)) => vec![form.button(
                label,
                icon,
                true,
                In::Move(member.file.clone(), to.to_owned()),
            )],
            None => Vec::new(),
        };
        form.row(FormRow::Tree {
            title: member.name.clone(),
            subtitle: member.subtitle.clone(),
            depth: 0,
            dimmed: false,
            buttons,
        });
    }
}

fn describe(form: &mut Builder<In>, state: &State) {
    let lists = &state.lists;
    let parties = lists.parties.clone();
    form.group(
        "",
        tr("Empty parties are removed when you leave this page."),
    )
    .choice(
        (tr("Party"), ""),
        lists.parties.clone(),
        lists.position(),
        Vec::new(),
        move |i| In::Party(parties.get(i).cloned().unwrap_or_default()),
    );
    let add = form.button(tr("_Add"), "", !state.name.trim().is_empty(), In::NewParty);
    form.text(tr("New Party"), &state.name, false, vec![add], In::Name);
    if let Some(problem) = &state.problem {
        form.banner(problem, false);
    }
    let party = lists.selected.as_str();
    if !party.is_empty() {
        form.group(tr("Members"), lists.members_note().unwrap_or(""));
        if lists.group_encounter {
            let group = form.button(tr("_Group Encounter…"), "", true, In::Group);
            form.heading_button(group);
        }
        members(
            form,
            &lists.members,
            Some((tr("Remove from Party"), "minus.circle", "")),
        );
    }
    form.group(tr("Without a Party"), lists.without_note().unwrap_or(""));
    members(
        form,
        &lists.without,
        (!party.is_empty()).then_some((tr("Add to Party"), "plus.circle", party)),
    );
}

/// Group Encounter (0x44e30b): make an encounter of the party's NPCs,
/// or print them as one.
fn group_dialog(form: &mut Builder<In>, group: &GroupRequest) -> FormDialog {
    let page = form.take_groups();
    form.group("", "")
        .note(&tr!(
            "Create or print an encounter of the NPCs of “{party}”.",
            party = group.party.clone()
        ))
        .radio(
            &[
                (
                    tr("_Create and Save NPC Group Encounter").to_owned(),
                    String::new(),
                ),
                (tr("_Print Group Encounter").to_owned(), String::new()),
            ],
            Some(usize::from(group.print)),
            |i| In::GroupPrint(i == 1),
        )
        .checks(
            &[
                (
                    tr("Co_ndensed Format").to_owned(),
                    tr("One stat line for each NPC").to_owned(),
                    group.condensed,
                    group.print,
                ),
                (
                    tr("Print P_review").to_owned(),
                    String::new(),
                    group.preview,
                    group.print,
                ),
            ],
            |i, on| {
                if i == 0 {
                    In::Condensed(on)
                } else {
                    In::Preview(on)
                }
            },
        );
    let close = form.button(tr("_Cancel"), "", true, In::GroupAnswer(false));
    let accept = form.button(tr("_OK"), "", true, In::GroupAnswer(true));
    let groups = form.take_groups();
    form.put_groups(page);
    FormDialog {
        title: tr("Group Encounter").to_owned(),
        groups,
        close,
        accept: Some(accept),
    }
}
