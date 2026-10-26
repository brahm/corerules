//! Roll Treasure: the roller of `cr_app::treasure` on a page of the
//! window. How many of each treasure type to roll, the coin and
//! Accumulate Rolls, then the treasure with its edits and the Calculate
//! summary; the bar rolls, resets, saves, prints and deletes, and on an
//! encounter's treasure Use Treasure hands the bundle back.

use cr_app::action;
use cr_app::bundle::BundleInput;
use cr_app::treasure::{INDIVIDUAL, LAIR, MOST_COUNT, Roller, TreasureInput};
use cr_i18n::tr;
use cr_store::treasure::LETTERS;
use cr_win::text::plain;
use windows_reactor::*;

use super::bundle::{self, BundleMessages};
use super::forms::{self, heading};
use super::{Message, Shell};
use crate::ui;

fn input(input: TreasureInput) -> Message {
    Message::Treasure(input)
}

/// The roller's bundle edits.
pub const MESSAGES: BundleMessages = BundleMessages {
    input: |i| input(TreasureInput::Bundle(i)),
    answer: |serial, yes| input(TreasureInput::BundleAnswer(serial, yes)),
};

/// The treasure types of `range`, each letter with how many to roll, as
/// many to a row as the window has room for.
fn letters(
    roller: &Roller,
    range: std::ops::Range<usize>,
    context: &mut ViewContext<Shell>,
) -> View {
    let room = ui::room();
    let per_row = if room.is_finite() {
        ((room - 72.0) / 170.0).floor().clamp(2.0, 6.0) as usize
    } else {
        3
    };
    let cells: Vec<KeyedView> = range
        .clone()
        .enumerate()
        .map(|(at, i)| {
            let letter = LETTERS[i];
            KeyedView::new(
                letter.to_string(),
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(8.0)
                    .grid_row((at / per_row) as i32)
                    .grid_column((at % per_row) as i32)
                    .children((
                        ui::strong(letter.to_string())
                            .width(16.0)
                            .vertical_alignment(VerticalAlignment::Center),
                        forms::number(
                            &tr!("Treasure type {letter}", letter),
                            roller.counts[i],
                            (0, MOST_COUNT),
                            move |n| input(TreasureInput::Count(i, n)),
                            context,
                        ),
                    )),
            )
        })
        .collect();
    let rows = range.len().div_ceil(per_row);
    ui::card(
        Grid::new()
            .columns(vec![GridLength::Auto; per_row])
            .rows(vec![GridLength::Auto; rows])
            .column_spacing(24.0)
            .row_spacing(8.0)
            .keyed_children(cells),
    )
}

pub fn view(roller: &Roller, busy: bool, context: &mut ViewContext<Shell>) -> View {
    let mut commands: Vec<KeyedView> = Vec::new();
    let mut add = |key: &'static str, label: String, symbol: Symbol, enabled: bool, message| {
        commands.push(KeyedView::new(
            key,
            ui::command(&label, symbol, enabled, context.message(message)),
        ));
    };
    if roller.pick {
        add(
            "use",
            plain(tr("_Use Treasure")),
            Symbol::Accept,
            true,
            Message::UseTreasure,
        );
    }
    add(
        "roll",
        plain(tr("_Roll")),
        Symbol::Shuffle,
        true,
        input(TreasureInput::Roll),
    );
    add(
        "reset",
        plain(tr("Re_set")),
        Symbol::Clear,
        true,
        input(TreasureInput::Reset),
    );
    add(
        "save",
        plain(tr("_Save…")),
        Symbol::Save,
        !busy && !roller.saving,
        Message::Act(action::TREASURE_SAVE),
    );
    add(
        "print",
        plain(tr("_Print…")),
        Symbol::Print,
        !busy,
        Message::Act(action::TREASURE_PRINT),
    );
    add(
        "pdf",
        plain(tr("Save as P_DF…")),
        Symbol::SaveLocal,
        !busy,
        Message::Act(action::TREASURE_PDF),
    );
    add(
        "delete",
        plain(tr("_Delete Saved Treasure")),
        Symbol::Delete,
        !busy && roller.saved.is_some(),
        Message::Act(action::TREASURE_DELETE),
    );
    let header = Grid::new()
        .columns([GridLength::Auto, GridLength::STAR])
        .column_spacing(16.0)
        .margin(Thickness::new(0.0, 0.0, 0.0, 8.0))
        .children((
            ui::title(roller.title()).vertical_alignment(VerticalAlignment::Center),
            CommandBar::new()
                .horizontal_alignment(HorizontalAlignment::Right)
                .grid_column(1)
                .collection_slot(CommandBarSlot::PrimaryCommands, commands),
        ));
    let types = StackPanel::new().spacing(4.0).children((
        heading(tr("Lair"), Some(tr("Treasure types A to I"))),
        letters(roller, LAIR, context),
        heading(tr("Individual"), Some(tr("Treasure types J to Z"))),
        letters(roller, INDIVIDUAL, context),
        heading(tr("Options"), None),
        ui::setting(
            tr("Accumulate Rolls"),
            tr("Add new rolls to the treasure instead of replacing it"),
            ToggleSwitch::new()
                .automation_name(tr("Accumulate Rolls"))
                .is_on(roller.accumulate)
                .on_toggled(context.callback(|on| input(TreasureInput::Accumulate(on)))),
        ),
        ui::setting(
            tr("Coin"),
            tr("The coin of the treasure types' platinum line"),
            ComboBox::new()
                .automation_name(tr("Coin"))
                .items_source([tr("Platinum").to_owned(), tr("Electrum").to_owned()])
                .selected_index(Some(usize::from(roller.electrum)))
                .min_width(200.0)
                .on_selection_changed(context.callback(|i: Option<usize>| match i {
                    Some(i) => input(TreasureInput::Electrum(i == 1)),
                    None => Message::Nothing,
                })),
        ),
    ));
    let add_item = Button::new()
        .on_click(context.message(input(TreasureInput::Bundle(BundleInput::Add))))
        .content(plain(tr("_Add Item…")));
    let treasure: View = if roller.bundle.is_empty() {
        StackPanel::new().spacing(4.0).children((
            heading(
                tr("No Treasure"),
                Some(tr(
                    "Choose how many of each treasure type to roll, then press Roll, or add \
                     items yourself",
                )),
            ),
            add_item,
        ))
    } else {
        let summary: Vec<KeyedView> = roller
            .summary()
            .into_iter()
            .enumerate()
            .map(|(i, (title, value))| {
                KeyedView::new(
                    format!("{i}\u{1}{title}"),
                    ui::setting(&title, "", ui::body(value)),
                )
            })
            .collect();
        StackPanel::new().spacing(4.0).children((
            heading(tr("Treasure"), None),
            bundle::rows(
                &cr_app::bundle::rows(&roller.bundle),
                true,
                MESSAGES,
                context,
            ),
            Border::new()
                .margin(Thickness::new(0.0, 4.0, 0.0, 0.0))
                .content(add_item),
            heading(tr("Summary"), None),
            StackPanel::new().spacing(4.0).keyed_children(summary),
        ))
    };
    let naming: View = match &roller.naming {
        Some(name) => ContentDialog::new()
            .title(tr("Save Treasure"))
            .primary_button_text(plain(tr("_Save")))
            .close_button_text(plain(tr("_Cancel")))
            .is_open(true)
            .on_closed(context.callback(|result| {
                input(TreasureInput::SaveAnswer(
                    result == ContentDialogResult::Primary,
                ))
            }))
            .content(
                StackPanel::new().spacing(12.0).children((
                    ui::body(tr(
                        "Saved treasure appears under Saved Treasure, also in the original \
                         programs",
                    )),
                    TextBox::new()
                        .automation_name(tr("Name"))
                        .placeholder_text(tr("Name"))
                        .text(name.clone())
                        .on_text_changed(
                            context.callback(|text: String| input(TreasureInput::Name(text))),
                        ),
                )),
            ),
        None => View::empty(),
    };
    Grid::new().children((
        ui::page(
            StackPanel::new()
                .spacing(4.0)
                .children((header, types, treasure)),
        ),
        bundle::dialog(&roller.editor, MESSAGES, context),
        naming,
    ))
}
