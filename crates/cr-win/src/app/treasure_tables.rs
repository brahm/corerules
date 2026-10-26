//! A treasure table of the user databases on a page of the window: the
//! editor of `cr_app::records::tables`, its name and entries, each with
//! its roll range and weight, Add Entry under them, and an entry's
//! details in place of the table while they are open, with "How many to
//! remove?" over them.

use cr_app::records::tables::{
    self, DetailInput, Details, MOST_DICE, MOST_PROBABILITY, MOST_SPELLS, MOST_WEIGHT, Pick,
    TableForm, TableInput,
};
use cr_i18n::tr;
use cr_store::userdb::tables::{contained_line, max_spell_level};
use cr_win::text::plain;
use windows_reactor::*;

use super::create::prompt_dialog;
use super::forms::{heading, two_lines, with_buttons};
use super::record::{self, add_box, dice, number, text_button};
use super::{Message, Shell};
use crate::ui;

fn input(input: TableInput) -> Message {
    Message::TreasureTable(input)
}

fn detail(change: DetailInput) -> Message {
    input(TableInput::Detail(change))
}

/// A source's box and its item's, with Add after them: Add Entry, or
/// (`contents`) Always Contains.
fn add_entry(
    form: &TableForm,
    pick: Pick,
    contents: bool,
    source: impl Fn(usize) -> Message + 'static,
    item: impl Fn(usize) -> Message + 'static,
    add: Message,
    context: &mut ViewContext<Shell>,
) -> View {
    let sources = form.sources().sources(contents);
    let items = form.items(pick, contents).to_vec();
    let chosen = pick.item.and_then(|i| items.get(i)).map(String::as_str);
    let names = items.clone();
    StackPanel::new().spacing(8.0).children((
        ComboBox::new()
            .automation_name(tr("Category"))
            .placeholder_text(tr("Category"))
            .items_source(sources)
            .selected_index(pick.source)
            .min_width(240.0)
            .on_selection_changed(context.callback(move |i: Option<usize>| match i {
                Some(i) => source(i),
                None => Message::Nothing,
            })),
        add_box(
            tr("Item"),
            &items,
            chosen,
            pick.source.is_some(),
            move |name| match names.iter().position(|n| *n == name) {
                Some(i) => item(i),
                None => Message::Nothing,
            },
            add,
            context,
        ),
    ))
}

/// The editor.
pub fn view(form: &TableForm, context: &mut ViewContext<Shell>) -> View {
    let content: View = match &form.details {
        Some(details) => details_view(form, details, context),
        None => table_view(form, context),
    };
    let serial = form.prompt_serial();
    let mut dialogs: Vec<KeyedView> = form
        .prompt()
        .map(|prompt| {
            KeyedView::new(
                format!("prompt{serial}"),
                prompt_dialog(prompt, serial, Message::TreasureTableAnswer, context),
            )
        })
        .into_iter()
        .collect();
    if let Some((index, most, count)) = form.details.as_ref().and_then(|d| d.removing) {
        dialogs.push(KeyedView::new(
            format!("remove{index}"),
            how_many(most, count, context),
        ));
    }
    Grid::new().children((ui::page(content), Grid::new().keyed_children(dialogs)))
}

fn table_view(form: &TableForm, context: &mut ViewContext<Shell>) -> View {
    let enabled = !form.saving;
    let rows: Vec<KeyedView> = form
        .rows()
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let details: View = match &row.details {
                Some(_) => text_button(
                    tr("Details…"),
                    enabled,
                    input(TableInput::Details(i)),
                    context,
                ),
                None => View::empty(),
            };
            let controls = StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .vertical_alignment(VerticalAlignment::Center)
                .children((
                    number(
                        &format!("{}: {}", row.title, tr("Weight")),
                        row.weight.into(),
                        (0, MOST_WEIGHT.into()),
                        enabled,
                        move |v| input(TableInput::Weight(i, v as u32)),
                        context,
                    ),
                    details,
                    ui::icon_button(
                        tr("Remove Entry"),
                        Symbol::Delete,
                        input(TableInput::Remove(i)),
                        context,
                    ),
                ));
            let text = StackPanel::new().spacing(2.0).children((
                two_lines(&row.title, &row.range),
                match &row.details {
                    Some(summary) => ui::caption(summary.clone())
                        .text_wrapping(TextWrapping::Wrap)
                        .into(),
                    None => View::empty(),
                },
            ));
            KeyedView::new(
                format!("{i}\u{1}{}", row.title),
                ui::card(
                    Grid::new()
                        .columns([GridLength::STAR, GridLength::Auto])
                        .column_spacing(16.0)
                        .children((
                            Border::new()
                                .vertical_alignment(VerticalAlignment::Center)
                                .content(text),
                            Border::new().grid_column(1).content(controls),
                        )),
                ),
            )
        })
        .collect();
    let rows: View = if rows.is_empty() {
        ui::card(ui::body(tr("No entries")).opacity(0.6))
    } else {
        StackPanel::new().spacing(4.0).keyed_children(rows)
    };
    StackPanel::new().spacing(4.0).children((
        record::header(form.title(), context),
        ui::setting(
            tr("Name"),
            "",
            TextBox::new()
                .automation_name(tr("Name"))
                .text(form.table.name.clone())
                .min_width(300.0)
                .is_enabled(enabled)
                .on_text_changed(context.callback(|text: String| input(TableInput::Name(text)))),
        ),
        heading(
            tr("Entries"),
            Some(tr("Each entry is rolled in proportion to its weight")),
        ),
        rows,
        heading(tr("Add Entry"), None),
        add_entry(
            form,
            form.pick,
            false,
            |s| input(TableInput::AddSource(s)),
            |i| input(TableInput::AddItem(i)),
            input(TableInput::Add),
            context,
        ),
        record::problem(form.error.as_deref()),
        record::buttons(
            !form.saving,
            form.can_save(),
            input(TableInput::Save),
            context,
        ),
    ))
}

/// A setting whose control is a number in `0..=most`.
fn count(
    label: &str,
    description: &str,
    value: u32,
    most: u32,
    changed: impl Fn(u32) -> DetailInput + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    ui::setting(
        label,
        description,
        number(
            label,
            value.into(),
            (0, most.into()),
            true,
            move |v| detail(changed(v as u32)),
            context,
        ),
    )
}

fn dice_setting(
    label: &str,
    values: [u32; 3],
    changed: impl Fn(usize, u32) -> DetailInput + Clone + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    let most = i64::from(MOST_DICE);
    ui::setting(
        label,
        "",
        dice(
            label,
            values.map(i64::from),
            [(0, most); 3],
            true,
            move |k, v| detail(changed(k, v as u32)),
            context,
        ),
    )
}

/// Treasure Table Entry Details, in place of the table.
fn details_view(form: &TableForm, d: &Details, context: &mut ViewContext<Shell>) -> View {
    let e = &d.entry;
    let random: Vec<KeyedView> = (0..2)
        .map(|k| {
            let (choices, chosen) = form.sub_table_choices(k);
            let dice_values = tables::sub_table(e, k).1;
            let label = if k == 0 {
                tr("First Table")
            } else {
                tr("Second Table")
            };
            KeyedView::new(
                format!("sub{k}"),
                StackPanel::new().spacing(4.0).children((
                    ui::setting(
                        label,
                        "",
                        ComboBox::new()
                            .automation_name(label)
                            .items_source(choices)
                            .selected_index(Some(chosen))
                            .min_width(260.0)
                            .on_selection_changed(context.callback(
                                move |i: Option<usize>| match i {
                                    Some(i) => detail(DetailInput::SubTable(k, i)),
                                    None => Message::Nothing,
                                },
                            )),
                    ),
                    dice_setting(
                        &format!("{label}: {}", tr("Number Found")),
                        dice_values,
                        move |n, v| DetailInput::SubDice(k, n, v),
                        context,
                    ),
                )),
            )
        })
        .collect();
    let contained: Vec<KeyedView> = d
        .contained
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let line = contained_line(item);
            KeyedView::new(
                format!("{i}\u{1}{line}"),
                ui::setting(
                    &line,
                    "",
                    ui::icon_button(
                        tr("Remove"),
                        Symbol::Delete,
                        detail(DetailInput::Remove(i)),
                        context,
                    ),
                ),
            )
        })
        .collect();
    let contained: View = if contained.is_empty() {
        ui::card(ui::body(tr("Nothing")).opacity(0.6))
    } else {
        StackPanel::new().spacing(4.0).keyed_children(contained)
    };
    let wizard = e.spell_wizard != 0;
    let highest = max_spell_level(wizard);
    StackPanel::new().spacing(4.0).children((
        record::header(&e.item.name, context),
        count(
            tr("Probability"),
            "",
            e.weight,
            MOST_PROBABILITY,
            DetailInput::Probability,
            context,
        ),
        dice_setting(
            tr("Number Found"),
            e.quantity_dice,
            DetailInput::Found,
            context,
        ),
        count(
            tr("Chance Intelligent"),
            tr("Percent chance of an intelligent weapon"),
            e.intelligent_chance.into(),
            100,
            DetailInput::Intelligent,
            context,
        ),
        heading(
            tr("Random Contents"),
            Some(tr("The item holds what these tables roll")),
        ),
        StackPanel::new().spacing(8.0).keyed_children(random),
        heading(tr("Always Contains"), None),
        contained,
        add_entry(
            form,
            d.pick,
            true,
            |s| detail(DetailInput::Source(s)),
            |i| detail(DetailInput::Item(i)),
            detail(DetailInput::Add),
            context,
        ),
        heading(
            tr("Scroll Spells"),
            Some(tr(
                "Scrolls of protection should not have any of these set. This is only for spell scrolls.",
            )),
        ),
        ui::setting(
            tr("Spell Category"),
            "",
            ComboBox::new()
                .automation_name(tr("Spell Category"))
                .items_source(vec![tr("Wizard").to_owned(), tr("Priest").to_owned()])
                .selected_index(Some(usize::from(!wizard)))
                .min_width(160.0)
                .on_selection_changed(context.callback(|i: Option<usize>| match i {
                    Some(i) => detail(DetailInput::Wizard(i == 0)),
                    None => Message::Nothing,
                })),
        ),
        count(
            tr("Lowest Level"),
            "",
            e.spell_min_level,
            highest,
            DetailInput::Lowest,
            context,
        ),
        count(
            tr("Highest Level"),
            "",
            e.spell_max_level,
            highest,
            DetailInput::Highest,
            context,
        ),
        count(
            tr("Number of Spells"),
            tr("0 for none"),
            e.spell_count,
            MOST_SPELLS,
            DetailInput::Spells,
            context,
        ),
        StackPanel::new()
            .margin(Thickness::new(0.0, 16.0, 0.0, 0.0))
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .horizontal_alignment(HorizontalAlignment::Right)
            .children((
                Button::new()
                    .on_click(context.message(input(TableInput::CloseDetails(false))))
                    .content(plain(tr("_Cancel"))),
                Button::new()
                    .style(ButtonStyle::Accent)
                    .on_click(context.message(input(TableInput::CloseDetails(true))))
                    .content(plain(tr("_OK"))),
            )),
    ))
}

/// "How many to remove?" of a contained item there are several of.
fn how_many(most: u32, count: u32, context: &mut ViewContext<Shell>) -> View {
    ContentDialog::new()
        .title(tr("How many to remove?"))
        .primary_button_text(plain(tr("_Remove")))
        .close_button_text(plain(tr("_Cancel")))
        .is_open(true)
        .on_closed(context.callback(|result| {
            detail(DetailInput::ConfirmRemove(
                result == ContentDialogResult::Primary,
            ))
        }))
        .content(with_buttons(
            number(
                tr("How many to remove?"),
                count.into(),
                (0, most.into()),
                true,
                |v| detail(DetailInput::RemoveCount(v as u32)),
                context,
            ),
            View::empty(),
        ))
}
