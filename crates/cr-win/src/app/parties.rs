//! Manage Parties: the lists of `cr_app::parties` on a page of the
//! window. The party chosen and New Party over the party's members and
//! the characters in no party, each with the button that moves it to
//! the other list; Group Encounter makes or prints an encounter of the
//! party's NPCs and Delete Members deletes the party's characters.

use cr_app::parties::{Member, PartyLists};
use cr_i18n::tr;
use cr_win::text::plain;
use windows_reactor::*;

use super::forms::heading;
use super::{Message, Shell};
use crate::ui;

/// The page: the lists once read, the name typed in New Party and why
/// that name was refused.
#[derive(Clone, Default)]
pub struct PartiesPage {
    pub lists: Option<PartyLists>,
    pub name: String,
    pub problem: Option<String>,
}

impl PartiesPage {
    /// The party chosen, empty when there is none.
    pub fn selected(&self) -> &str {
        self.lists.as_ref().map_or("", |l| l.selected.as_str())
    }

    /// The parties made here that have no members yet.
    pub fn new_parties(&self) -> Vec<String> {
        self.lists
            .as_ref()
            .map(|l| l.new.clone())
            .unwrap_or_default()
    }
}

/// The cards of `members`, each with the button that moves it to `to`
/// (no party when empty); none without a party chosen.
fn members(
    members: &[Member],
    button: Option<(&str, Symbol, &str)>,
    context: &mut ViewContext<Shell>,
) -> View {
    let cards: Vec<KeyedView> = members
        .iter()
        .map(|member| {
            let control: View = match button {
                Some((name, symbol, to)) => ui::icon_button(
                    name,
                    symbol,
                    Message::MoveToParty(member.file.clone(), to.to_owned()),
                    context,
                ),
                None => View::empty(),
            };
            KeyedView::new(
                member.file.clone(),
                ui::setting(&member.name, &member.subtitle, control),
            )
        })
        .collect();
    StackPanel::new().spacing(4.0).keyed_children(cards)
}

pub fn view(page: &PartiesPage, busy: bool, context: &mut ViewContext<Shell>) -> View {
    let title = ui::title(tr("Manage Parties")).margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
    let Some(lists) = &page.lists else {
        return ui::page(
            StackPanel::new().spacing(4.0).children((
                title,
                ProgressRing::new()
                    .is_active(true)
                    .horizontal_alignment(HorizontalAlignment::Left),
            )),
        );
    };
    let party = lists.selected.as_str();
    let parties = lists.parties.clone();
    let choose = ui::setting(
        tr("Party"),
        "",
        ComboBox::new()
            .automation_name(tr("Party"))
            .items_source(parties.clone())
            .selected_index(lists.position())
            .is_enabled(!parties.is_empty())
            .min_width(240.0)
            .on_selection_changed(context.callback(move |i: Option<usize>| {
                match i.and_then(|i| parties.get(i)) {
                    Some(party) => Message::PartyChosen(party.clone()),
                    None => Message::Nothing,
                }
            })),
    );
    let new_party = ui::setting(
        tr("New Party"),
        "",
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                TextBox::new()
                    .automation_name(tr("New Party"))
                    .placeholder_text(tr("Name"))
                    .text(page.name.clone())
                    .min_width(200.0)
                    .on_text_changed(context.callback(Message::PartyName)),
                Button::new()
                    .is_enabled(!page.name.trim().is_empty() && !busy)
                    .on_click(context.message(Message::NewParty))
                    .content(plain(tr("_Add"))),
            )),
    );
    let problem: View = match &page.problem {
        Some(problem) => InfoBar::new()
            .severity(InfoBarSeverity::Warning)
            .title(problem.clone())
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    };
    let member_list: View = if party.is_empty() {
        View::empty()
    } else {
        let group: View = if lists.group_encounter {
            Button::new()
                .is_enabled(!busy)
                .on_click(context.message(Message::GroupEncounter))
                .content(plain(tr("_Group Encounter…")))
        } else {
            View::empty()
        };
        let delete: View = if lists.members.is_empty() {
            View::empty()
        } else {
            ui::icon_button(
                tr("Delete Members…"),
                Symbol::Delete,
                Message::AskDeleteParty,
                context,
            )
        };
        StackPanel::new().spacing(4.0).children((
            Grid::new()
                .columns([GridLength::STAR, GridLength::Auto])
                .children((
                    heading(tr("Members"), lists.members_note()),
                    StackPanel::new()
                        .grid_column(1)
                        .orientation(Orientation::Horizontal)
                        .spacing(8.0)
                        .vertical_alignment(VerticalAlignment::Bottom)
                        .children((group, delete)),
                )),
            members(
                &lists.members,
                Some((tr("Remove from Party"), Symbol::Remove, "")),
                context,
            ),
        ))
    };
    let without = StackPanel::new().spacing(4.0).children((
        heading(tr("Without a Party"), lists.without_note()),
        members(
            &lists.without,
            (!party.is_empty()).then_some((tr("Add to Party"), Symbol::Add, party)),
            context,
        ),
    ));
    ui::page(
        StackPanel::new().spacing(4.0).children((
            title,
            ui::caption(tr("Empty parties are removed when you leave this page."))
                .margin(Thickness::new(0.0, 0.0, 0.0, 8.0)),
            choose,
            new_party,
            problem,
            member_list,
            without,
        )),
    )
}
