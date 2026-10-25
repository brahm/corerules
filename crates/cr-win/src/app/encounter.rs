//! Encounters on pages of the window: New Encounter's templates, the
//! Encounter page of `cr_app::encounter` (monsters and NPCs with their
//! numbers and hit points, treasure, items, traps, totals and the
//! description), the 2-20 table editor, and Group Encounter's dialog.

use std::sync::Arc;

use cr_app::action;
use cr_app::bundle::BundleInput;
use cr_app::encounter::{
    CLIMATES, EncounterForm, EncounterInput, FREQUENCIES, LAIR_MODES, MOST_COUNT, Picker, SOURCES,
    TERRAINS, TableEditInput, TableEditor,
};
use cr_i18n::tr;
use cr_store::encounter::{Encounter, EncounterRules, Package};
use cr_store::userdb::Filter;
use cr_win::text::plain;
use windows_reactor::*;

use super::bundle::{self, BundleMessages};
use super::forms::{self, heading};
use super::{Message, Shell};
use crate::ui;

fn input(input: EncounterInput) -> Message {
    Message::Encounter(input)
}

fn table(input: TableEditInput) -> Message {
    Message::EncounterTable(input)
}

/// The items' edits.
const ITEMS: BundleMessages = BundleMessages {
    input: |i| input(EncounterInput::Items(i)),
    answer: |serial, yes| input(EncounterInput::ItemsAnswer(serial, yes)),
};

/// The treasure, which only the roller changes.
const TREASURE: BundleMessages = BundleMessages {
    input: |_| Message::Nothing,
    answer: |_, _| Message::Nothing,
};

/// The key of the table editor's Climate and Terrain when it is open.
pub const PLACES: &str = "places";

/// A page's title with its commands beside it.
fn header(title: String, commands: Vec<KeyedView>) -> View {
    Grid::new()
        .columns([GridLength::Auto, GridLength::STAR])
        .column_spacing(16.0)
        .margin(Thickness::new(0.0, 0.0, 0.0, 8.0))
        .children((
            ui::title(title).vertical_alignment(VerticalAlignment::Center),
            CommandBar::new()
                .horizontal_alignment(HorizontalAlignment::Right)
                .grid_column(1)
                .collection_slot(CommandBarSlot::PrimaryCommands, commands),
        ))
}

/// A heading with buttons at its end.
fn heading_with(title: &str, description: Option<&str>, buttons: Vec<KeyedView>) -> View {
    Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .column_spacing(8.0)
        .children((
            heading(title, description),
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .grid_column(1)
                .vertical_alignment(VerticalAlignment::Bottom)
                .keyed_children(buttons),
        ))
}

fn button(label: &str, message: Message, context: &mut ViewContext<Shell>) -> View {
    Button::new()
        .on_click(context.message(message))
        .content(plain(label))
}

/// New Encounter: the original's "Select an Encounter Template", with an
/// empty encounter first.
#[derive(Clone)]
pub struct Chooser {
    pub rules: Arc<EncounterRules>,
    pub templates: Vec<(String, String)>,
    pub search: String,
    /// The template chosen; `Some("")` for the empty encounter.
    pub chosen: Option<String>,
}

impl Chooser {
    pub fn new(rules: Arc<EncounterRules>) -> Self {
        Self {
            templates: cr_app::encounter::templates(&rules),
            rules,
            search: String::new(),
            chosen: None,
        }
    }

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

pub fn chooser_view(chooser: &Chooser, busy: bool, context: &mut ViewContext<Shell>) -> View {
    let rows = chooser.shown();
    let selected = chooser
        .chosen
        .as_ref()
        .and_then(|c| rows.iter().position(|(name, _)| name == c));
    let names: Vec<String> = rows.iter().map(|(name, _)| name.clone()).collect();
    let commands = vec![KeyedView::new(
        "create",
        ui::command(
            tr("Create Encounter"),
            Symbol::Add,
            selected.is_some() && !busy,
            context.message(Message::CreateEncounter),
        ),
    )];
    let list: View = if rows.is_empty() {
        ui::caption(tr("No Results")).margin(12.0).into()
    } else {
        ListView::new()
            .automation_name(tr("Templates"))
            .selected_index(selected)
            .on_selection_changed(context.callback(move |i: Option<usize>| {
                match i.and_then(|i| names.get(i)) {
                    Some(name) => Message::ChooseTemplate(name.clone()),
                    None => Message::Nothing,
                }
            }))
            .collection_slot(
                ListViewSlot::Items,
                rows.iter().enumerate().map(|(i, (name, kind))| {
                    let text = if name.is_empty() {
                        forms::two_lines(
                            tr("Empty Encounter"),
                            tr("Add monsters and characters yourself"),
                        )
                    } else {
                        forms::two_lines(name, kind)
                    };
                    KeyedView::new(
                        format!("{i}\u{1}{name}"),
                        ListViewItem::new().content(
                            Border::new()
                                .padding(Thickness::new(0.0, 6.0, 0.0, 6.0))
                                .content(text),
                        ),
                    )
                }),
            )
    };
    ui::page(
        StackPanel::new().spacing(8.0).children((
            header(tr("New Encounter").to_owned(), commands),
            TextBox::new()
                .automation_name(tr("Search Templates"))
                .placeholder_text(tr("Search Templates"))
                .text(chooser.search.clone())
                .on_text_changed(context.callback(Message::TemplateSearch)),
            // A new list for each search, which would otherwise keep a
            // selection made in another.
            Grid::new().keyed_children([KeyedView::new(chooser.search.clone(), list)]),
        )),
    )
}

/// The key that a package's card and its open state go by: its place
/// and name, so that a card removed above leaves the others as they were.
fn package_key(encounter: &Encounter, package: Package) -> String {
    let name = match package {
        Package::Monster(i) => &encounter.monsters[i].monster.name,
        Package::Npc(i) => &encounter.npcs[i].character.data.name,
    };
    format!("{package:?}\u{1}{name}")
}

/// An Expander open as the shell keeps it under `key`: WinUI's Expander
/// opens only when its state says so.
fn expander(
    key: String,
    name: &str,
    open: &[String],
    (header, content): (View, View),
    context: &mut ViewContext<Shell>,
) -> View {
    Expander::new()
        .automation_name(name)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .is_expanded(open.contains(&key))
        .on_is_expanded_changed(context.callback(move |on| Message::RowOpen(key.clone(), on)))
        .slots([
            SlotView::new(ExpanderSlot::Header, header),
            SlotView::new(ExpanderSlot::Content, content),
        ])
}

/// The card of a package: its title and number, and, opened, its number,
/// hit points and stat line.
fn package_view(
    row: &cr_app::encounter::PackageRow,
    key: String,
    open: &[String],
    context: &mut ViewContext<Shell>,
) -> View {
    let package = row.package;
    let mut buttons = Vec::new();
    if let Package::Npc(i) = package {
        // View NPC… and Print NPC… (0x63EA49, 0x63EB3A): the NPC's
        // character sheet.
        buttons.push(KeyedView::new(
            "view",
            ui::icon_button(
                tr("View NPC"),
                Symbol::Preview,
                Message::NpcSheet(i, false),
                context,
            ),
        ));
        buttons.push(KeyedView::new(
            "print",
            ui::icon_button(
                tr("Print NPC"),
                Symbol::Print,
                Message::NpcSheet(i, true),
                context,
            ),
        ));
    }
    buttons.push(KeyedView::new(
        "remove",
        ui::icon_button(
            tr("Remove"),
            Symbol::Delete,
            input(EncounterInput::Remove(package)),
            context,
        ),
    ));
    let title = Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .column_spacing(12.0)
        .children((
            Border::new()
                .vertical_alignment(VerticalAlignment::Center)
                .content(forms::two_lines(&row.title, &row.subtitle)),
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(4.0)
                .grid_column(1)
                .keyed_children(buttons),
        ));
    let mut parts: Vec<KeyedView> = vec![KeyedView::new(
        "number",
        ui::setting(
            tr("Number"),
            if matches!(package, Package::Monster(_)) {
                tr("Changing it rolls the hit points again")
            } else {
                ""
            },
            forms::signed_number(
                tr("Number"),
                row.count,
                (1, MOST_COUNT),
                move |n| input(EncounterInput::Count(package, n)),
                context,
            ),
        ),
    )];
    // The Hit Points group of "Monsters for the Encounter" (IDD 292): a
    // value, how many monsters have it, and Re-roll.
    if let (Package::Monster(i), Some(hp)) = (package, &row.hit_points) {
        parts.push(KeyedView::new(
            "hit-points",
            ui::setting(
                tr("Hit Points"),
                "",
                forms::with_buttons(
                    forms::signed_number(
                        tr("Hit Points"),
                        hp.value,
                        (hp.low, hp.high.max(hp.low)),
                        move |v| input(EncounterInput::HitPoints(i, v)),
                        context,
                    ),
                    Button::new()
                        .on_click(context.message(input(EncounterInput::Reroll(i))))
                        .content(plain(tr("_Re-roll"))),
                ),
            ),
        ));
        // A new box for each value, whose most it takes.
        parts.push(KeyedView::new(
            format!("with{}\u{1}{}", hp.value, hp.most_with),
            ui::setting(
                tr("Monsters"),
                &hp.note(),
                forms::signed_number(
                    tr("Monsters"),
                    hp.with,
                    (0, hp.most_with.max(0)),
                    move |n| input(EncounterInput::With(i, n)),
                    context,
                ),
            ),
        ));
    }
    parts.push(KeyedView::new(
        "stats",
        ui::card(ui::body(row.stats.clone()).is_text_selection_enabled(true)),
    ));
    expander(
        key,
        &row.title,
        open,
        (title, StackPanel::new().spacing(4.0).keyed_children(parts)),
        context,
    )
}

/// Add Monster, Add NPC or Add Trap: a search over the list, and Add
/// once a name is chosen.
fn picker_dialog(picker: &Picker, context: &mut ViewContext<Shell>) -> View {
    let shown = picker.shown();
    let items: Vec<(String, String)> = shown.iter().map(|&i| picker.items[i].clone()).collect();
    let names: Vec<String> = items.iter().map(|(name, _)| name.clone()).collect();
    let selected = picker
        .chosen
        .as_ref()
        .and_then(|c| names.iter().position(|n| n == c));
    let list: View = if picker.items.is_empty() {
        ui::caption(tr("Nothing to Add")).margin(12.0).into()
    } else if items.is_empty() {
        ui::caption(tr("No Results")).margin(12.0).into()
    } else {
        ListView::new()
            .automation_name(picker.title())
            .max_height(360.0)
            .selected_index(selected)
            .on_selection_changed(context.callback(move |i: Option<usize>| {
                match i.and_then(|i| names.get(i)) {
                    Some(name) => input(EncounterInput::PickChoose(name.clone())),
                    None => Message::Nothing,
                }
            }))
            .collection_slot(
                ListViewSlot::Items,
                items.iter().enumerate().map(|(i, (name, under))| {
                    KeyedView::new(
                        format!("{i}\u{1}{name}"),
                        ListViewItem::new().content(forms::two_lines(name, under)),
                    )
                }),
            )
    };
    let filter: View = match picker.filter() {
        Some(filter) => ComboBox::new()
            .automation_name(tr("Show Standard or Custom Traps"))
            .items_source(Filter::NAMES.map(|n| tr(n).to_owned()))
            .selected_index(Some(filter))
            .min_width(200.0)
            .on_selection_changed(context.callback(|i: Option<usize>| match i {
                Some(i) => input(EncounterInput::PickFilter(i)),
                None => Message::Nothing,
            }))
            .into(),
        None => View::empty(),
    };
    let content = StackPanel::new().spacing(8.0).children((
        filter,
        TextBox::new()
            .automation_name(picker.placeholder())
            .placeholder_text(picker.placeholder())
            .text(picker.search.clone())
            .on_text_changed(
                context.callback(|text: String| input(EncounterInput::PickSearch(text))),
            ),
        // A new list for each search and filter.
        Grid::new().keyed_children([KeyedView::new(
            format!("{}\u{1}{:?}", picker.search, picker.filter()),
            list,
        )]),
    ));
    let title = picker.title();
    Grid::new().keyed_children([KeyedView::new(
        title,
        ContentDialog::new()
            .title(title)
            .primary_button_text(if selected.is_some() {
                plain(tr("_Add"))
            } else {
                String::new()
            })
            .close_button_text(plain(tr("_Cancel")))
            .is_open(true)
            .on_closed(context.callback(|result| {
                input(EncounterInput::PickAnswer(
                    result == ContentDialogResult::Primary,
                ))
            }))
            .content(ScrollViewer::new().max_height(480.0).content(content)),
    )])
}

/// The Encounter page; `open` holds the keys of the cards opened.
pub fn view(
    form: &EncounterForm,
    open: &[String],
    busy: bool,
    context: &mut ViewContext<Shell>,
) -> View {
    let encounter = &form.encounter;
    let mut commands: Vec<KeyedView> = Vec::new();
    let mut add = |key: &'static str, label: String, symbol: Symbol, enabled: bool, message| {
        commands.push(KeyedView::new(
            key,
            ui::command(&label, symbol, enabled, context.message(message)),
        ));
    };
    add(
        "save",
        plain(tr("_Save")),
        Symbol::Save,
        !busy && !form.saving,
        input(EncounterInput::Save),
    );
    add(
        "print",
        plain(tr("_Print…")),
        Symbol::Print,
        !busy,
        Message::Act(action::ENCOUNTER_PRINT),
    );
    add(
        "pdf",
        plain(tr("Save as P_DF…")),
        Symbol::SaveLocal,
        !busy,
        Message::Act(action::ENCOUNTER_PDF),
    );
    add(
        "delete",
        plain(tr("_Delete Encounter")),
        Symbol::Delete,
        !busy && form.saved.is_some(),
        Message::Act(action::ENCOUNTER_DELETE),
    );
    let template = if encounter.template.is_empty() {
        tr("None").to_owned()
    } else {
        encounter.template.clone()
    };
    let about = StackPanel::new().spacing(4.0).children((
        heading(tr("Encounter"), None),
        ui::setting(
            tr("Name"),
            "",
            TextBox::new()
                .automation_name(tr("Name"))
                .text(encounter.name.clone())
                .min_width(240.0)
                .on_text_changed(
                    context.callback(|text: String| input(EncounterInput::Name(text))),
                ),
        ),
        ui::setting(tr("Template"), "", ui::body(template)),
        ui::setting(
            tr("Monster Type"),
            "",
            TextBox::new()
                .automation_name(tr("Monster Type"))
                .text(encounter.monster_type.clone())
                .min_width(240.0)
                .on_text_changed(
                    context.callback(|text: String| input(EncounterInput::MonsterType(text))),
                ),
        ),
    ));
    let totals: Vec<KeyedView> = form
        .totals()
        .into_iter()
        .map(|(title, value)| KeyedView::new(title, ui::setting(title, "", ui::body(value))))
        .collect();
    let totals = StackPanel::new().spacing(4.0).children((
        heading(tr("Encounter Totals"), None),
        StackPanel::new().spacing(4.0).keyed_children(totals),
    ));

    let rows = form.rows();
    let packages: View = if rows.is_empty() {
        ui::card(ui::body(tr("No monsters or characters")).opacity(0.6))
    } else {
        StackPanel::new()
            .spacing(4.0)
            .keyed_children(rows.iter().map(|row| {
                let key = package_key(encounter, row.package);
                KeyedView::new(key.clone(), package_view(row, key, open, context))
            }))
    };
    let add_monster = button(
        tr("Add _Monster…"),
        input(EncounterInput::AddMonster),
        context,
    );
    let add_npc = button(tr("Add _NPC…"), Message::AddNpc, context);
    let packages = StackPanel::new().spacing(4.0).children((
        heading_with(
            tr("Monsters and Nonplayer Characters"),
            None,
            vec![
                KeyedView::new("monster", add_monster),
                KeyedView::new("npc", add_npc),
            ],
        ),
        packages,
    ));

    let pending = form.pending_rows();
    let pending: View = if pending.is_empty() {
        View::empty()
    } else {
        StackPanel::new().spacing(4.0).children((
            heading(
                tr("Characters Not Generated"),
                Some(tr(
                    "The template also calls for these nonplayer characters, but the NPC \
                     generator could not make them, for the reason under each one. Add saved \
                     characters with Add NPC instead.",
                )),
            ),
            StackPanel::new()
                .spacing(4.0)
                .keyed_children(pending.iter().enumerate().map(|(i, (title, subtitle))| {
                    KeyedView::new(i.to_string(), ui::setting(title, subtitle, View::empty()))
                })),
        ))
    };

    let roll = button(tr("_Roll Treasure…"), Message::EncounterTreasure, context);
    let treasure_rows: View = if encounter.treasure.is_empty() {
        View::empty()
    } else {
        bundle::rows(
            &cr_app::bundle::rows(&encounter.treasure),
            false,
            TREASURE,
            context,
        )
    };
    let treasure = StackPanel::new().spacing(4.0).children((
        heading_with(
            tr("Treasure"),
            Some(&form.treasure_note()),
            vec![KeyedView::new("roll", roll)],
        ),
        treasure_rows,
    ));

    let add_item = button(
        tr("_Add Item…"),
        input(EncounterInput::Items(BundleInput::Add)),
        context,
    );
    let item_rows: View = if encounter.items.is_empty() {
        View::empty()
    } else {
        bundle::rows(
            &cr_app::bundle::rows(&encounter.items),
            true,
            ITEMS,
            context,
        )
    };
    let items = StackPanel::new().spacing(4.0).children((
        heading_with(
            tr("Items"),
            encounter
                .items
                .is_empty()
                .then(|| tr("Items found with the encounter")),
            vec![KeyedView::new("add", add_item)],
        ),
        item_rows,
    ));

    let add_trap = button(tr("_Add Trap…"), Message::AddTrap, context);
    let traps: Vec<KeyedView> = encounter
        .traps
        .iter()
        .enumerate()
        .map(|(i, trap)| {
            KeyedView::new(
                format!("{i}\u{1}{}", trap.name),
                ui::setting(
                    &trap.name,
                    &trap.damage,
                    ui::icon_button(
                        tr("Remove Trap"),
                        Symbol::Remove,
                        input(EncounterInput::RemoveTrap(trap.name.clone())),
                        context,
                    ),
                ),
            )
        })
        .collect();
    let traps = StackPanel::new().spacing(4.0).children((
        heading_with(
            tr("Traps"),
            encounter.traps.is_empty().then(|| tr("No traps")),
            vec![KeyedView::new("add", add_trap)],
        ),
        StackPanel::new().spacing(4.0).keyed_children(traps),
    ));

    let description = StackPanel::new().spacing(4.0).children((
        heading(tr("Description"), None),
        TextBox::new()
            .automation_name(tr("Description"))
            .text(encounter.description.clone())
            .accepts_return(true)
            .text_wrapping(TextWrapping::Wrap)
            .min_height(100.0)
            .on_text_changed(
                context.callback(|text: String| input(EncounterInput::Description(text))),
            ),
    ));

    let picker: View = match &form.picker {
        Some(picker) => picker_dialog(picker, context),
        None => View::empty(),
    };
    Grid::new().children((
        ui::page(StackPanel::new().spacing(4.0).children((
            header(form.title(), commands),
            about,
            totals,
            packages,
            pending,
            treasure,
            items,
            traps,
            description,
        ))),
        bundle::dialog(&form.items, ITEMS, context),
        picker,
    ))
}

/// Picking the template of a roll: the templates of a frequency that
/// pass the filter, or (None), and OK once one is chosen.
fn pick_dialog(pick: &cr_app::encounter::RollPick, context: &mut ViewContext<Shell>) -> View {
    let mut names = vec![String::new()];
    names.extend(pick.names.iter().cloned());
    let selected = pick
        .chosen
        .as_ref()
        .and_then(|c| names.iter().position(|n| n == c));
    let chosen = names.clone();
    let list = ListView::new()
        .automation_name(tr("Templates"))
        .max_height(360.0)
        .selected_index(selected)
        .on_selection_changed(context.callback(move |i: Option<usize>| {
            match i.and_then(|i| chosen.get(i)) {
                Some(name) => table(TableEditInput::PickChoose(name.clone())),
                None => Message::Nothing,
            }
        }))
        .collection_slot(
            ListViewSlot::Items,
            names.iter().enumerate().map(|(i, name)| {
                let text = if name.is_empty() { tr("(None)") } else { name };
                KeyedView::new(
                    format!("{i}\u{1}{name}"),
                    ListViewItem::new().content(ui::body(text)),
                )
            }),
        );
    let empty: View = if pick.names.is_empty() {
        ui::caption(tr("None of this frequency pass the filter")).into()
    } else {
        View::empty()
    };
    let content = StackPanel::new().spacing(8.0).children((
        ComboBox::new()
            .automation_name(tr("Frequency"))
            .items_source(FREQUENCIES.map(|f| tr(f).to_owned()))
            .selected_index(Some(pick.frequency))
            .min_width(200.0)
            .on_selection_changed(context.callback(|i: Option<usize>| match i {
                Some(i) => table(TableEditInput::PickFrequency(i)),
                None => Message::Nothing,
            })),
        // A new list for each frequency.
        Grid::new().keyed_children([KeyedView::new(pick.frequency.to_string(), list)]),
        empty,
    ));
    let title = tr!("Roll {roll}", roll = pick.index + 2);
    Grid::new().keyed_children([KeyedView::new(
        format!("roll{}", pick.index),
        ContentDialog::new()
            .title(title)
            .primary_button_text(if selected.is_some() {
                plain(tr("_OK"))
            } else {
                String::new()
            })
            .close_button_text(plain(tr("_Cancel")))
            .is_open(true)
            .on_closed(context.callback(|result| {
                table(TableEditInput::PickAnswer(
                    result == ContentDialogResult::Primary,
                ))
            }))
            .content(ScrollViewer::new().max_height(480.0).content(content)),
    )])
}

/// The 2-20 table editor; `open` holds [`PLACES`] when Climate and
/// Terrain is opened.
pub fn table_view(
    editor: &TableEditor,
    open: &[String],
    busy: bool,
    context: &mut ViewContext<Shell>,
) -> View {
    let filter = &editor.table.filter;
    let mut commands: Vec<KeyedView> = Vec::new();
    let mut add = |key: &'static str, label: String, symbol: Symbol, enabled: bool, message| {
        commands.push(KeyedView::new(
            key,
            ui::command(&label, symbol, enabled, context.message(message)),
        ));
    };
    add(
        "save",
        plain(tr("_Save")),
        Symbol::Save,
        !busy && !editor.saving,
        table(TableEditInput::Save),
    );
    add(
        "fill",
        plain(tr("_Fill")),
        Symbol::Bullets,
        true,
        table(TableEditInput::Fill),
    );
    add(
        "roll",
        plain(tr("_Roll")),
        Symbol::Shuffle,
        !busy,
        table(TableEditInput::Roll),
    );
    if editor.saved.is_some() {
        add(
            "delete",
            tr("Delete Table").to_owned(),
            Symbol::Delete,
            !busy,
            Message::AskDeleteTable,
        );
    }
    let error: View = match &editor.error {
        Some(error) => InfoBar::new()
            .severity(InfoBarSeverity::Warning)
            .title(error.clone())
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    };
    let combo = |name: &str,
                 items: Vec<String>,
                 selected: usize,
                 chosen: fn(usize) -> TableEditInput,
                 context: &mut ViewContext<Shell>| {
        ComboBox::new()
            .automation_name(name)
            .items_source(items)
            .selected_index(Some(selected))
            .min_width(220.0)
            .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                Some(i) => table(chosen(i)),
                None => Message::Nothing,
            }))
    };
    // As many to a row as the window has room for.
    let room = ui::room();
    let per_row = if room.is_finite() {
        ((room - 120.0) / 220.0).floor().clamp(1.0, 4.0) as usize
    } else {
        2
    };
    let places: Vec<KeyedView> = CLIMATES
        .iter()
        .chain(TERRAINS.iter())
        .enumerate()
        .map(|(i, place)| {
            KeyedView::new(
                format!("{i}\u{1}{per_row}"),
                Border::new()
                    .grid_row((i / per_row) as i32)
                    .grid_column((i % per_row) as i32)
                    .content(
                        CheckBox::new()
                            .automation_name(tr(place))
                            .is_checked(filter.climates[i] != 0)
                            .on_is_checked_changed(
                                context.callback(move |on| table(TableEditInput::Place(i, on))),
                            )
                            .content(tr(place)),
                    ),
            )
        })
        .collect();
    let count = CLIMATES.len() + TERRAINS.len();
    let places = expander(
        PLACES.to_owned(),
        tr("Climate and Terrain"),
        open,
        (
            forms::two_lines(
                tr("Climate and Terrain"),
                tr("Where the templates may be encountered"),
            ),
            Grid::new()
                .columns(vec![GridLength::STAR; per_row])
                .rows(vec![GridLength::Auto; count.div_ceil(per_row)])
                .column_spacing(12.0)
                .keyed_children(places),
        ),
        context,
    );
    let filter_view = StackPanel::new().spacing(4.0).children((
        heading(
            tr("Filter"),
            Some(tr(
                "Fill picks templates that pass the filter, by frequency",
            )),
        ),
        ui::setting(
            tr("Database"),
            "",
            combo(
                tr("Database"),
                SOURCES.map(|s| tr(s).to_owned()).to_vec(),
                editor.source(),
                TableEditInput::Source,
                context,
            ),
        ),
        ui::setting(
            tr("Fewest Hit Dice"),
            "",
            forms::signed_number(
                tr("Fewest Hit Dice"),
                filter.hit_dice.0,
                (0, 100),
                |n| table(TableEditInput::Fewest(n)),
                context,
            ),
        ),
        ui::setting(
            tr("Most Hit Dice"),
            "",
            forms::signed_number(
                tr("Most Hit Dice"),
                filter.hit_dice.1,
                (0, 100),
                |n| table(TableEditInput::Most(n)),
                context,
            ),
        ),
        ui::setting(
            tr("Lair and/or Wandering"),
            "",
            combo(
                tr("Lair and/or Wandering"),
                LAIR_MODES.map(|s| tr(s).to_owned()).to_vec(),
                editor.lair(),
                TableEditInput::Lair,
                context,
            ),
        ),
        places,
    ));
    let slots: Vec<KeyedView> = editor
        .slots()
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            KeyedView::new(
                i.to_string(),
                ui::setting(
                    &(i + 2).to_string(),
                    &name,
                    Button::new()
                        .automation_name(tr!("Roll {roll}", roll = i + 2))
                        .on_click(context.message(table(TableEditInput::PickRoll(i))))
                        .content(plain(tr("_Choose…"))),
                ),
            )
        })
        .collect();
    let rolls = StackPanel::new().spacing(4.0).children((
        heading(
            tr("Rolls"),
            Some(tr(
                "Pick a roll to choose its template from those that pass the filter",
            )),
        ),
        StackPanel::new().spacing(4.0).keyed_children(slots),
    ));
    let pick: View = match &editor.picking {
        Some(pick) => pick_dialog(pick, context),
        None => View::empty(),
    };
    Grid::new().children((
        ui::page(
            StackPanel::new().spacing(4.0).children((
                header(editor.title(), commands),
                error,
                ui::setting(
                    tr("Name"),
                    "",
                    TextBox::new()
                        .automation_name(tr("Name"))
                        .text(editor.table.name.clone())
                        .min_width(240.0)
                        .on_text_changed(
                            context.callback(|text: String| table(TableEditInput::Name(text))),
                        ),
                ),
                filter_view,
                rolls,
            )),
        ),
        pick,
    ))
}

/// Group Encounter (0x44e30b): make an encounter of a party's NPCs, or
/// print them as one.
#[derive(Clone, PartialEq)]
pub struct GroupDialog {
    pub party: String,
    /// The NPCs' files, in order.
    pub members: Vec<String>,
    pub print: bool,
    pub condensed: bool,
    pub preview: bool,
}

pub fn group_dialog(group: &GroupDialog, context: &mut ViewContext<Shell>) -> View {
    let content = StackPanel::new().spacing(12.0).children((
        ui::body(tr!(
            "Create or print an encounter of the NPCs of “{party}”.",
            party = group.party.clone()
        )),
        RadioButtons::new()
            .items_source([
                plain(tr("_Create and Save NPC Group Encounter")),
                plain(tr("_Print Group Encounter")),
            ])
            .selected_index(Some(usize::from(group.print)))
            .on_selection_changed(
                context.callback(|i: Option<usize>| Message::GroupPrint(i == Some(1))),
            ),
        CheckBox::new()
            .is_enabled(group.print)
            .is_checked(group.condensed)
            .on_is_checked_changed(context.callback(Message::GroupCondensed))
            .content(forms::two_lines(
                &plain(tr("Co_ndensed Format")),
                tr("One stat line for each NPC"),
            )),
        CheckBox::new()
            .is_enabled(group.print)
            .is_checked(group.preview)
            .on_is_checked_changed(context.callback(Message::GroupPreview))
            .content(plain(tr("Print P_review"))),
    ));
    ContentDialog::new()
        .title(tr("Group Encounter"))
        .primary_button_text(plain(tr("_OK")))
        .close_button_text(plain(tr("_Cancel")))
        .is_open(true)
        .on_closed(
            context
                .callback(|result| Message::GroupAnswered(result == ContentDialogResult::Primary)),
        )
        .content(content)
}
