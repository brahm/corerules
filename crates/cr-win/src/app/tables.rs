//! Roll on a Table: the roller of `cr_app::tables` on a page of the
//! window. A book and one of its tables, the table's options, and the
//! results of the roll; the bar rolls, rolls again on the same table,
//! copies the results, and reads the table in its book or opens the
//! treasure generator where the roll says to.

use std::hash::{DefaultHasher, Hash, Hasher};

use cr_app::tables::{TableInput, TableRoller};
use cr_i18n::tr;
use cr_store::tables::{Block, RollResult};
use cr_win::text::plain;
use windows_reactor::*;

use super::forms::heading;
use super::{Message, Shell};
use crate::ui;

fn input(input: TableInput) -> Message {
    Message::Table(input)
}

/// A ComboBox of `items`; `chosen` makes the message of the one chosen.
fn combo(
    name: &str,
    items: Vec<String>,
    selected: usize,
    chosen: impl Fn(usize) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    ComboBox::new()
        .automation_name(name)
        .items_source(items)
        .selected_index(Some(selected))
        .min_width(240.0)
        .max_width(480.0)
        .on_selection_changed(context.callback(move |i: Option<usize>| match i {
            Some(i) => chosen(i),
            None => Message::Nothing,
        }))
        .into()
}

/// A block of the results: its title and subtitle, its columns as a
/// grid, and its notes.
fn block_view(block: &Block) -> View {
    let mut parts: Vec<View> = Vec::new();
    if let Some(title) = &block.title {
        parts.push(
            ui::strong(title.clone())
                .is_text_selection_enabled(true)
                .into(),
        );
    }
    if let Some(subtitle) = &block.subtitle {
        parts.push(
            ui::caption(subtitle.clone())
                .is_text_selection_enabled(true)
                .into(),
        );
    }
    if !block.header.is_empty() {
        let columns = block.header.len();
        let mut cells = Vec::new();
        for (r, row) in std::iter::once(&block.header)
            .chain(&block.rows)
            .enumerate()
        {
            for (c, cell) in row.iter().enumerate().take(columns) {
                // The columns never wrap; the notes under them do.
                let text = TextBlock::new()
                    .text(cell.clone())
                    .font_size(14.0)
                    .is_text_selection_enabled(true)
                    .opacity(if r == 0 { 0.75 } else { 1.0 })
                    .grid_row(r as i32)
                    .grid_column(c as i32);
                cells.push(KeyedView::new(format!("{r}:{c}"), text));
            }
        }
        parts.push(
            ScrollViewer::new()
                .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)
                .vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                .content(
                    Grid::new()
                        .columns(vec![GridLength::Auto; columns])
                        .rows(vec![GridLength::Auto; block.rows.len() + 1])
                        .column_spacing(16.0)
                        .row_spacing(4.0)
                        .margin(Thickness::new(0.0, 4.0, 0.0, 4.0))
                        .keyed_children(cells),
                ),
        );
    }
    for note in &block.notes {
        parts.push(
            ui::body(note.clone())
                .is_text_selection_enabled(true)
                .into(),
        );
    }
    StackPanel::new().spacing(4.0).keyed_children(
        parts
            .into_iter()
            .enumerate()
            .map(|(i, part)| KeyedView::new(i.to_string(), part)),
    )
}

fn results_view(result: Option<&RollResult>) -> View {
    let Some(result) = result else {
        return ui::card(ui::body(tr("Choose a table and roll.")).opacity(0.75));
    };
    ui::card(
        StackPanel::new().spacing(16.0).keyed_children(
            result
                .blocks
                .iter()
                .enumerate()
                .map(|(i, block)| KeyedView::new(i.to_string(), block_view(block))),
        ),
    )
}

pub fn view(roller: &TableRoller, context: &mut ViewContext<Shell>) -> View {
    let result = roller.result.as_ref();
    let mut commands: Vec<KeyedView> = Vec::new();
    let mut add = |key: &'static str, label: String, symbol: Symbol, enabled: bool, message| {
        commands.push(KeyedView::new(
            key,
            ui::command(&label, symbol, enabled, context.message(message)),
        ));
    };
    add(
        "roll",
        plain(tr("_Roll")),
        Symbol::Shuffle,
        true,
        input(TableInput::Roll),
    );
    add(
        "again",
        plain(tr("Roll _Again")),
        Symbol::RepeatAll,
        roller.again_enabled(),
        input(TableInput::Again),
    );
    add(
        "copy",
        tr("Copy Results").to_owned(),
        Symbol::Copy,
        result.is_some(),
        Message::CopyResults,
    );
    if result.is_some_and(|r| r.reference.is_some()) {
        add(
            "read",
            plain(tr("Read in _Book")),
            Symbol::Read,
            true,
            Message::ReadTable,
        );
    }
    if result.is_some_and(|r| r.treasure) {
        add(
            "treasure",
            plain(tr("Open _Treasure Generator")),
            Symbol::Shop,
            true,
            Message::TableTreasure,
        );
    }
    let header = Grid::new()
        .columns([GridLength::Auto, GridLength::STAR])
        .column_spacing(16.0)
        .margin(Thickness::new(0.0, 0.0, 0.0, 8.0))
        .children((
            ui::title(tr("Roll on a Table")).vertical_alignment(VerticalAlignment::Center),
            CommandBar::new()
                .horizontal_alignment(HorizontalAlignment::Right)
                .grid_column(1)
                .collection_slot(CommandBarSlot::PrimaryCommands, commands),
        ));
    let choose = StackPanel::new().spacing(4.0).children((
        ui::setting(
            tr("Book"),
            "",
            combo(
                tr("Book"),
                roller.book_titles(),
                roller.book,
                |i| input(TableInput::Book(i)),
                context,
            ),
        ),
        // A new list for each book.
        ui::setting(
            tr("Table"),
            "",
            Grid::new().keyed_children([KeyedView::new(
                roller.book.to_string(),
                combo(
                    tr("Table"),
                    roller.table_titles().to_vec(),
                    roller.table,
                    |i| input(TableInput::Table(i)),
                    context,
                ),
            )]),
        ),
    ));
    // The options of each table, and those worked out again from an
    // earlier choice, are a list of their own, which would otherwise keep
    // a choice made on another.
    let params = roller.params();
    let options: View = if params.is_empty() {
        View::empty()
    } else {
        let rows: Vec<KeyedView> = params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let selected = roller.chosen.get(i).copied().unwrap_or(0);
                let mut hasher = DefaultHasher::new();
                param.options.hash(&mut hasher);
                KeyedView::new(
                    format!(
                        "{}\u{1}{}\u{1}{i}\u{1}{}\u{1}{}",
                        roller.book,
                        roller.table,
                        param.label,
                        hasher.finish()
                    ),
                    ui::setting(
                        &param.label,
                        "",
                        combo(
                            &param.label,
                            param.options.clone(),
                            selected,
                            move |o| input(TableInput::Option(i, o)),
                            context,
                        ),
                    ),
                )
            })
            .collect();
        StackPanel::new().spacing(4.0).children((
            heading(tr("Options"), None),
            StackPanel::new().spacing(4.0).keyed_children(rows),
        ))
    };
    ui::page(StackPanel::new().spacing(4.0).children((
        header,
        choose,
        options,
        heading(tr("Results"), None),
        results_view(result),
    )))
}
