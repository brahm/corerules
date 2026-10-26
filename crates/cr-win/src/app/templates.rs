//! An encounter template of the user databases on a page of the window:
//! the editor of `cr_app::records::templates`, its pages chosen as the
//! record editor's are, a line's Properties in place of the page while
//! they are open, and over them the pickers its Add buttons open and the
//! numbers of a monster that comes with an item.

use cr_app::records::templates::{
    COIN_RANGES, COINS, ESCORT_RANGES, LineEdit, LineInput, MOST, MOST_HIT_DICE, MOST_TREASURE,
    MonsterFor, Picker, PickerInput, Ranges, SECOND_RANGES, TemplateForm, TemplateInput,
    TemplatePage, item_rows, monster_rows, npc_rows, trap_list,
};
use cr_i18n::tr;
use cr_store::userdb::Filter;
use cr_store::userdb::templates::{self as tpl, Combo};
use cr_win::text::plain;
use windows_reactor::*;

use super::create::prompt_dialog;
use super::forms::{heading, two_lines, with_buttons};
use super::record::{self, number, tabs, text_button};
use super::{Message, Shell};
use crate::ui;

fn input(input: TemplateInput) -> Message {
    Message::Template(input)
}

fn line(change: LineInput) -> Message {
    input(TemplateInput::Line(change))
}

fn pick(change: PickerInput) -> Message {
    input(TemplateInput::Picker(change))
}

/// Four NumberBoxes for "N d S * M + B", each kept in its range.
fn dice(
    name: &str,
    values: [i32; 4],
    ranges: Ranges,
    changed: impl Fn(usize, i32) -> Message + Clone + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    let parts = [tr("Dice"), tr("Sides"), tr("Multiplier"), tr("Bonus")];
    let mut children: Vec<KeyedView> = Vec::new();
    for k in 0..4 {
        if k > 0 {
            children.push(KeyedView::new(
                format!("sep{k}"),
                ui::body(["d", "*", "+"][k - 1]).vertical_alignment(VerticalAlignment::Center),
            ));
        }
        let changed = changed.clone();
        let (min, max) = ranges[k];
        children.push(KeyedView::new(
            format!("n{k}"),
            number(
                &format!("{name}: {}", parts[k]),
                values[k].into(),
                (min.into(), max.into()),
                true,
                move |v| changed(k, v as i32),
                context,
            )
            .min_width(96.0),
        ));
    }
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .keyed_children(children)
}

/// A setting whose control is a number in `min..=max`.
fn count(
    label: &str,
    description: &str,
    value: i32,
    (min, max): (i32, i32),
    changed: impl Fn(i32) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> View {
    ui::setting(
        label,
        description,
        number(
            label,
            value.into(),
            (min.into(), max.into()),
            true,
            move |v| changed(v as i32),
            context,
        ),
    )
}

/// A ComboBox of `items` with `selected` chosen.
fn combo(
    name: &str,
    items: Vec<String>,
    selected: usize,
    changed: impl Fn(usize) -> Message + 'static,
    context: &mut ViewContext<Shell>,
) -> ComboBox {
    ComboBox::new()
        .automation_name(name)
        .items_source(items)
        .selected_index(Some(selected))
        .min_width(240.0)
        .on_selection_changed(context.callback(move |i: Option<usize>| match i {
            Some(i) => changed(i),
            None => Message::Nothing,
        }))
}

/// The rows of a list, each with its buttons, or "None".
fn rows(rows: Vec<KeyedView>) -> View {
    if rows.is_empty() {
        ui::card(ui::body(tr("None")).opacity(0.6))
    } else {
        StackPanel::new().spacing(4.0).keyed_children(rows)
    }
}

/// A row of a list: its text, and the buttons after it.
fn row(key: String, text: View, buttons: View) -> KeyedView {
    KeyedView::new(
        key,
        ui::card(
            Grid::new()
                .columns([GridLength::STAR, GridLength::Auto])
                .column_spacing(16.0)
                .children((
                    Border::new()
                        .vertical_alignment(VerticalAlignment::Center)
                        .content(text),
                    Border::new()
                        .grid_column(1)
                        .vertical_alignment(VerticalAlignment::Center)
                        .content(buttons),
                )),
        ),
    )
}

/// The buttons of a line: Edit… opens its Properties, the other removes
/// it.
fn line_buttons(
    remove: &str,
    open: Option<Message>,
    removed: Message,
    context: &mut ViewContext<Shell>,
) -> View {
    let open: View = match open {
        Some(open) => text_button(&plain(tr("_Edit…")), true, open, context),
        None => View::empty(),
    };
    with_buttons(
        open,
        ui::icon_button(remove, Symbol::Delete, removed, context),
    )
}

/// The editor.
pub fn view(form: &TemplateForm, context: &mut ViewContext<Shell>) -> View {
    let content: View = match &form.line {
        Some(edit) => line_view(edit, context),
        None => page_view(form, context),
    };
    // WinUI opens one ContentDialog at a time: a message first.
    let serial = form.prompt_serial();
    let dialog: Option<KeyedView> = if let Some(prompt) = form.prompt() {
        Some(KeyedView::new(
            format!("prompt{serial}"),
            prompt_dialog(prompt, serial, Message::TemplateAnswer, context),
        ))
    } else if let Some(escort) = &form.escort {
        Some(KeyedView::new(
            format!("escort{}", form.dialog_serial),
            escort_dialog(&escort.monster.name, escort.dice, context),
        ))
    } else {
        form.picker.as_ref().map(|picker| {
            KeyedView::new(
                format!("picker{}", form.dialog_serial),
                picker_dialog(form, picker, context),
            )
        })
    };
    Grid::new().children((
        ui::page(content),
        Grid::new().keyed_children(dialog.into_iter().collect::<Vec<_>>()),
    ))
}

fn page_view(form: &TemplateForm, context: &mut ViewContext<Shell>) -> View {
    let pages: Vec<(usize, String)> = TemplatePage::ALL
        .iter()
        .enumerate()
        .map(|(i, p)| (i, p.name().to_owned()))
        .collect();
    let shown = TemplatePage::ALL
        .iter()
        .position(|p| *p == form.page)
        .unwrap_or(0);
    let content: View = match form.page {
        TemplatePage::Template => template_page(form, context),
        TemplatePage::Monsters => monsters_page(form, context),
        TemplatePage::Npcs => npcs_page(form, context),
        TemplatePage::Items => items_page(form, context),
        TemplatePage::Traps => traps_page(form, context),
        TemplatePage::Treasure => treasure_page(form, context),
        TemplatePage::Table => table_page(form, context),
    };
    StackPanel::new().spacing(4.0).children((
        record::header(form.title(), context),
        tabs(pages, shown, |p| input(TemplateInput::Page(p)), context),
        // A page of its own for each, so that its controls start anew.
        Grid::new().keyed_children([KeyedView::new(form.page.name(), content)]),
        record::problem(form.error.as_deref()),
        record::buttons(
            !form.saving,
            form.can_save(),
            input(TemplateInput::Save),
            context,
        ),
    ))
}

fn template_page(form: &TemplateForm, context: &mut ViewContext<Shell>) -> View {
    let (types, chosen) = form.type_choices();
    StackPanel::new().spacing(4.0).children((
        ui::setting(
            tr("Encounter Template Name"),
            "",
            TextBox::new()
                .automation_name(tr("Encounter Template Name"))
                .text(form.template.name.clone())
                .min_width(300.0)
                .on_text_changed(context.callback(|text: String| input(TemplateInput::Name(text)))),
        ),
        ui::setting(
            tr("Monster Type"),
            "",
            combo(
                tr("Monster Type"),
                types,
                chosen,
                |i| input(TemplateInput::MonsterType(i)),
                context,
            ),
        ),
        heading(tr("Additional Notes for this Encounter"), None),
        TextBox::new()
            .automation_name(tr("Additional Notes for this Encounter"))
            .text(form.template.notes.clone())
            .accepts_return(true)
            .text_wrapping(TextWrapping::Wrap)
            .min_height(220.0)
            .on_text_changed(context.callback(|text: String| input(TemplateInput::Notes(text)))),
    ))
}

/// A page of lines: its heading, the lines and Add.
fn list_page(
    title: &str,
    lines: View,
    add: &str,
    added: Message,
    context: &mut ViewContext<Shell>,
) -> View {
    StackPanel::new().spacing(4.0).children((
        heading(title, None),
        lines,
        Border::new()
            .margin(Thickness::new(0.0, 8.0, 0.0, 0.0))
            .content(text_button(add, true, added, context)),
    ))
}

fn monsters_page(form: &TemplateForm, context: &mut ViewContext<Shell>) -> View {
    let lines: Vec<KeyedView> = monster_rows(&form.template)
        .into_iter()
        .map(|(text, i)| {
            row(
                format!("{i}\u{1}{text}"),
                ui::body(text.trim_start().to_owned()).into(),
                line_buttons(
                    tr("Delete Monster"),
                    Some(input(TemplateInput::OpenMonster(i))),
                    input(TemplateInput::RemoveMonster(i)),
                    context,
                ),
            )
        })
        .collect();
    list_page(
        tr("Monsters associated with this Encounter Template"),
        rows(lines),
        tr("Add Monster"),
        input(TemplateInput::AddMonster),
        context,
    )
}

fn npcs_page(form: &TemplateForm, context: &mut ViewContext<Shell>) -> View {
    let lines: Vec<KeyedView> = npc_rows(&form.template)
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            row(
                format!("{i}\u{1}{text}"),
                ui::body(text.trim_start().to_owned()).into(),
                line_buttons(
                    tr("Delete NPC"),
                    Some(input(TemplateInput::OpenNpc(i))),
                    input(TemplateInput::RemoveNpc(i)),
                    context,
                ),
            )
        })
        .collect();
    list_page(
        tr("NPCs associated with this Encounter Template"),
        rows(lines),
        tr("Add NPC"),
        input(TemplateInput::AddNpc),
        context,
    )
}

fn items_page(form: &TemplateForm, context: &mut ViewContext<Shell>) -> View {
    let lines: Vec<KeyedView> = item_rows(&form.template)
        .into_iter()
        .enumerate()
        .map(|(i, (name, under))| {
            row(
                format!("{i}\u{1}{name}\u{1}{under}"),
                two_lines(&name, &under),
                line_buttons(
                    tr("Delete Item"),
                    Some(input(TemplateInput::OpenItem(i))),
                    input(TemplateInput::RemoveItem(i)),
                    context,
                ),
            )
        })
        .collect();
    list_page(
        tr("Items associated with this Encounter Template"),
        rows(lines),
        tr("Add Item"),
        input(TemplateInput::AddItem),
        context,
    )
}

fn traps_page(form: &TemplateForm, context: &mut ViewContext<Shell>) -> View {
    let lines: Vec<KeyedView> = trap_list(&form.template)
        .into_iter()
        .enumerate()
        .map(|(i, trap)| {
            row(
                format!("{i}\u{1}{}", trap.name),
                two_lines(&trap.name, trap.damage.trim()),
                line_buttons(
                    tr("Remove Trap"),
                    None,
                    input(TemplateInput::RemoveTrap(i)),
                    context,
                ),
            )
        })
        .collect();
    list_page(
        tr("Traps"),
        rows(lines),
        tr("Add Trap"),
        input(TemplateInput::AddTrap),
        context,
    )
}

/// As many to a row as the window has room for, `width` each, at most
/// `most`.
fn per_row(width: f64, most: usize) -> usize {
    let room = ui::room();
    if room.is_finite() {
        ((room - 120.0) / width).floor().clamp(1.0, most as f64) as usize
    } else {
        2
    }
}

fn treasure_page(form: &TemplateForm, context: &mut ViewContext<Shell>) -> View {
    let treasure = form.treasure();
    let per_row = per_row(200.0, 6);
    let cells: Vec<KeyedView> = treasure
        .iter()
        .enumerate()
        .map(|(i, (letter, n))| {
            let letter = letter.to_string();
            KeyedView::new(
                format!("{i}\u{1}{per_row}"),
                Border::new()
                    .grid_row((i / per_row) as i32)
                    .grid_column((i % per_row) as i32)
                    .content(
                        StackPanel::new()
                            .orientation(Orientation::Horizontal)
                            .spacing(8.0)
                            .children((
                                ui::strong(letter.as_str())
                                    .min_width(16.0)
                                    .vertical_alignment(VerticalAlignment::Center),
                                number(
                                    &letter,
                                    (*n).into(),
                                    (0, MOST_TREASURE.into()),
                                    true,
                                    move |v| input(TemplateInput::Treasure(i, v as i32)),
                                    context,
                                ),
                            )),
                    ),
            )
        })
        .collect();
    StackPanel::new().spacing(4.0).children((
        heading(
            tr("Treasure Types"),
            Some(tr("How many of each treasure type the lair holds")),
        ),
        ui::card(
            Grid::new()
                .columns(vec![GridLength::STAR; per_row])
                .rows(vec![GridLength::Auto; treasure.len().div_ceil(per_row)])
                .column_spacing(12.0)
                .row_spacing(8.0)
                .keyed_children(cells),
        ),
    ))
}

fn table_page(form: &TemplateForm, context: &mut ViewContext<Shell>) -> View {
    let t = &form.template;
    let custom = form.custom_terrain();
    let terrains = form.data().terrains.clone();
    let category = with_buttons(
        TextBox::new()
            .automation_name(tr("Category"))
            .text(t.climate_terrain.clone())
            .min_width(220.0)
            .on_text_changed(context.callback(|text: String| input(TemplateInput::Category(text)))),
        ComboBox::new()
            .automation_name(tr!("Choose {field}", field = tr("Category")))
            .placeholder_text(plain(tr("_Choose…")))
            .items_source(terrains.clone())
            .selected_index(None)
            .min_width(140.0)
            .is_enabled(!terrains.is_empty())
            .on_selection_changed(context.callback(move |i: Option<usize>| {
                match i.and_then(|i| terrains.get(i)) {
                    Some(c) => input(TemplateInput::Category(c.clone())),
                    None => Message::Nothing,
                }
            })),
    );
    let climates = form.climates();
    let per_row = per_row(220.0, 4);
    let places: Vec<KeyedView> = climates
        .iter()
        .enumerate()
        .map(|(i, (place, on))| {
            KeyedView::new(
                format!("{i}\u{1}{per_row}"),
                Border::new()
                    .grid_row((i / per_row) as i32)
                    .grid_column((i % per_row) as i32)
                    .content(
                        CheckBox::new()
                            .automation_name(*place)
                            .is_checked(*on)
                            .is_enabled(custom)
                            .on_is_checked_changed(
                                context.callback(move |on| input(TemplateInput::Climate(i, on))),
                            )
                            .content(*place),
                    ),
            )
        })
        .collect();
    StackPanel::new().spacing(4.0).children((
        heading(
            tr("Select the category under which this Encounter Template will appear"),
            None,
        ),
        ui::setting(
            tr("Encountered"),
            "",
            combo(
                tr("Encountered"),
                tpl::PLACEMENT_NAMES.map(|n| tr(n).to_owned()).to_vec(),
                tpl::placement_choice(&t.placement),
                |i| input(TemplateInput::Placement(i)),
                context,
            ),
        ),
        ui::setting(
            tr("Frequency"),
            "",
            combo(
                tr("Frequency"),
                tpl::FREQUENCY_NAMES.map(|n| tr(n).to_owned()).to_vec(),
                tpl::frequency_choice(t.frequency),
                |i| input(TemplateInput::Frequency(i)),
                context,
            ),
        ),
        count(
            tr("Number Of Hit Dice"),
            "",
            t.hit_dice,
            (0, MOST_HIT_DICE),
            |v| input(TemplateInput::HitDice(v)),
            context,
        ),
        heading(tr("Climate and Terrain"), None),
        ui::setting(tr("Category"), "", category),
        heading(
            tr("Climate and Terrain"),
            Some(tr("Chosen one by one in the ~Custom category")),
        ),
        ui::card(
            Grid::new()
                .columns(vec![GridLength::STAR; per_row])
                .rows(vec![GridLength::Auto; climates.len().div_ceil(per_row)])
                .column_spacing(12.0)
                .keyed_children(places),
        ),
    ))
}

/// A line's Properties, in place of the page.
fn line_view(edit: &LineEdit, context: &mut ViewContext<Shell>) -> View {
    let (first, percent) = edit.first();
    let item = edit.item();
    let first_title = if item.is_some() {
        tr("Number of Items")
    } else {
        tr("Number Appearing")
    };
    let item_name: View = match item {
        Some(p) => heading(&p.part.head.name, None),
        None => View::empty(),
    };
    let (dependent, per, beyond) = edit.dependent();
    let dependent_rows: View = if dependent {
        StackPanel::new().spacing(4.0).children((
            count(
                tr("For every"),
                tr("Enter 0 for a one time addition"),
                per,
                (0, MOST),
                |v| line(LineInput::Per(v)),
                context,
            ),
            count(
                tr("Beyond"),
                "",
                beyond,
                (0, MOST),
                |v| line(LineInput::Beyond(v)),
                context,
            ),
            ui::setting(
                tr("Monster or Monster Type"),
                &edit.reference(),
                text_button(
                    &plain(tr("_Choose…")),
                    true,
                    line(LineInput::ChooseReference),
                    context,
                ),
            ),
        ))
    } else {
        View::empty()
    };
    let extra: View = match edit {
        LineEdit::Item { .. } => item_extra(edit, context),
        _ => random_and_second(edit, context),
    };
    StackPanel::new().spacing(4.0).children((
        record::header(edit.title(), context),
        item_name,
        ui::setting(
            first_title,
            "",
            dice(
                first_title,
                first,
                edit.first_ranges(),
                |k, v| line(LineInput::Dice(k, v)),
                context,
            ),
        ),
        count(
            tr("Percent Chance"),
            tr("There is a percent chance that these appear"),
            percent,
            (1, 100),
            |v| line(LineInput::Percent(v)),
            context,
        ),
        ui::card(
            CheckBox::new()
                .automation_name(tr("Dependent on the following"))
                .is_checked(dependent)
                .on_is_checked_changed(context.callback(|on| line(LineInput::Dependent(on))))
                .content(ui::body(tr("Dependent on the following"))),
        ),
        dependent_rows,
        extra,
        StackPanel::new()
            .margin(Thickness::new(0.0, 16.0, 0.0, 0.0))
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .horizontal_alignment(HorizontalAlignment::Right)
            .children((
                Button::new()
                    .on_click(context.message(input(TemplateInput::CloseLine(false))))
                    .content(plain(tr("_Cancel"))),
                Button::new()
                    .style(ButtonStyle::Accent)
                    .on_click(context.message(input(TemplateInput::CloseLine(true))))
                    .content(plain(tr("_OK"))),
            )),
    ))
}

/// A monster or NPC line's random list, and its combo with the second
/// box it shows.
fn random_and_second(edit: &LineEdit, context: &mut ViewContext<Shell>) -> View {
    let npc = matches!(edit, LineEdit::Npc { .. });
    let random: Vec<KeyedView> = edit
        .random()
        .into_iter()
        .enumerate()
        .map(|(k, (title, under, removable))| {
            row(
                format!("{k}\u{1}{title}\u{1}{under}"),
                two_lines(&title, &under),
                if removable {
                    ui::icon_button(
                        tr("Remove"),
                        Symbol::Delete,
                        line(LineInput::RemoveRandom(k)),
                        context,
                    )
                } else {
                    View::empty()
                },
            )
        })
        .collect();
    let combo_index = edit.combo().unwrap_or(Combo::Appear);
    let second: View = if combo_index == Combo::Appear {
        View::empty()
    } else {
        let (who, choose) = if npc {
            (tr("NPC"), plain(tr("_NPC…")))
        } else {
            (tr("Monster"), plain(tr("_Monster…")))
        };
        StackPanel::new().spacing(4.0).children((
            ui::setting(
                tr("Number Appearing"),
                "",
                dice(
                    tr("Number Appearing"),
                    edit.second_dice(),
                    SECOND_RANGES,
                    |k, v| line(LineInput::SecondDice(k, v)),
                    context,
                ),
            ),
            ui::setting(
                who,
                &edit.second(),
                text_button(&choose, true, line(LineInput::ChooseSecond), context),
            ),
        ))
    };
    StackPanel::new().spacing(4.0).children((
        heading(
            tr("Randomly selected from the following list"),
            Some(if npc {
                tr("Each appearance picks one of these NPCs")
            } else {
                tr("Each appearance picks one of these monsters")
            }),
        ),
        rows(random),
        Border::new()
            .margin(Thickness::new(0.0, 8.0, 0.0, 0.0))
            .content(text_button(
                if npc {
                    tr("Add NPC")
                } else {
                    tr("Add Monster")
                },
                true,
                line(LineInput::AddRandom),
                context,
            )),
        ui::setting(
            tr("These then"),
            "",
            combo(
                tr("These then"),
                Combo::NAMES.map(|n| tr(n).to_owned()).to_vec(),
                combo_index.index() as usize,
                |i| line(LineInput::Combo(i)),
                context,
            ),
        ),
        second,
    ))
}

/// An item's coins and the monsters that come with it.
fn item_extra(edit: &LineEdit, context: &mut ViewContext<Shell>) -> View {
    let (coin, coin_dice) = edit.coins();
    let escort_list = |per_item: bool, context: &mut ViewContext<Shell>| -> View {
        let lines: Vec<KeyedView> = edit
            .escorts(per_item)
            .into_iter()
            .enumerate()
            .map(|(k, text)| {
                row(
                    format!("{k}\u{1}{text}"),
                    ui::body(text.clone()).into(),
                    ui::icon_button(
                        tr("Delete"),
                        Symbol::Delete,
                        line(LineInput::RemoveEscort(per_item, k)),
                        context,
                    ),
                )
            })
            .collect();
        StackPanel::new().spacing(4.0).children((
            heading(
                if per_item {
                    tr("For each Item...")
                } else {
                    tr("These Monsters will appear if the previous Item(s) appear...")
                },
                None,
            ),
            rows(lines),
            Border::new()
                .margin(Thickness::new(0.0, 8.0, 0.0, 0.0))
                .content(text_button(
                    tr("Add Monster"),
                    true,
                    line(LineInput::AddEscort(per_item)),
                    context,
                )),
        ))
    };
    let escorts = escort_list(false, context);
    let per_item = escort_list(true, context);
    let doubles: Vec<KeyedView> = edit
        .doubles()
        .iter()
        .enumerate()
        .map(|(k, name)| {
            row(
                format!("{k}\u{1}{name}"),
                ui::body(name.clone()).into(),
                ui::icon_button(
                    tr("Delete"),
                    Symbol::Delete,
                    line(LineInput::RemoveDouble(k)),
                    context,
                ),
            )
        })
        .collect();
    StackPanel::new().spacing(4.0).children((
        heading(tr("Coins"), Some(tr("Coins each item holds"))),
        ui::setting(
            tr("Containing"),
            "",
            dice(
                tr("Containing"),
                coin_dice,
                COIN_RANGES,
                |k, v| line(LineInput::CoinDice(k, v)),
                context,
            ),
        ),
        ui::setting(
            tr("Coins"),
            "",
            combo(
                tr("Coins"),
                COINS.map(str::to_owned).to_vec(),
                coin,
                |i| line(LineInput::Coin(i)),
                context,
            ),
        ),
        escorts,
        per_item,
        heading(tr("Double the number of these Monsters..."), None),
        rows(doubles),
        Border::new()
            .margin(Thickness::new(0.0, 8.0, 0.0, 0.0))
            .content(text_button(
                tr("Add Monster"),
                true,
                line(LineInput::AddDouble),
                context,
            )),
    ))
}

/// A picker: its filters, the search and the list; Add or OK keeps what
/// is chosen.
fn picker_dialog(form: &TemplateForm, picker: &Picker, context: &mut ViewContext<Shell>) -> View {
    let rows = form.picker_rows();
    let selected = picker
        .chosen()
        .and_then(|c| rows.iter().position(|r| r.index == c));
    let mut filters: Vec<KeyedView> = Vec::new();
    let filter_key = match picker {
        Picker::Monster(p) => {
            filters.push(KeyedView::new(
                "origin",
                ComboBox::new()
                    .automation_name(tr("Show Standard or Custom Records"))
                    .items_source(Filter::NAMES.map(|n| tr(n).to_owned()))
                    .selected_index(Some(match p.origin {
                        Filter::All => 0,
                        Filter::Standard => 1,
                        Filter::Custom => 2,
                    }))
                    .min_width(200.0)
                    .on_selection_changed(context.callback(|i: Option<usize>| match i {
                        Some(i) => pick(PickerInput::Origin(i)),
                        None => Message::Nothing,
                    })),
            ));
            if p.purpose == MonsterFor::Reference {
                filters.push(KeyedView::new(
                    "instead",
                    CheckBox::new()
                        .automation_name(tr("Select Type instead of Monster"))
                        .is_checked(p.instead)
                        .on_is_checked_changed(
                            context.callback(|on| pick(PickerInput::Instead(on))),
                        )
                        .content(ui::body(tr("Select Type instead of Monster"))),
                ));
            }
            if !p.instead {
                filters.push(KeyedView::new(
                    "kind",
                    combo(
                        tr("Monster Types"),
                        form.picker_types(),
                        p.kind,
                        |i| pick(PickerInput::Kind(i)),
                        context,
                    ),
                ));
            }
            format!("{:?}\u{1}{}\u{1}{}", p.origin, p.kind, p.instead)
        }
        Picker::Item(p) => {
            let categories: Vec<String> = form
                .data()
                .categories
                .iter()
                .map(|(_, name)| tr(name).to_owned())
                .collect();
            filters.push(KeyedView::new(
                "category",
                combo(
                    tr("Part Type"),
                    categories,
                    p.category,
                    |i| pick(PickerInput::Category(i)),
                    context,
                ),
            ));
            p.category.to_string()
        }
        Picker::Trap(_) => String::new(),
    };
    let search = match picker {
        Picker::Monster(p) => p.search.clone(),
        Picker::Item(p) => p.search.clone(),
        Picker::Trap(p) => p.search.clone(),
    };
    let indices: Vec<usize> = rows.iter().map(|r| r.index).collect();
    let list: View = if rows.is_empty() {
        ui::caption(tr("No Results")).margin(12.0).into()
    } else {
        ListView::new()
            .automation_name(picker.title())
            .max_height(360.0)
            .selected_index(selected)
            .on_selection_changed(context.callback(move |i: Option<usize>| {
                match i.and_then(|i| indices.get(i)) {
                    Some(index) => pick(PickerInput::Choose(*index)),
                    None => Message::Nothing,
                }
            }))
            .collection_slot(
                ListViewSlot::Items,
                rows.iter().enumerate().map(|(i, r)| {
                    let under = match (r.custom, r.subtitle.is_empty()) {
                        (false, _) => r.subtitle.clone(),
                        (true, true) => tr("Custom").to_owned(),
                        (true, false) => format!("{} · {}", tr("Custom"), r.subtitle),
                    };
                    KeyedView::new(
                        format!("{i}\u{1}{}", r.index),
                        ListViewItem::new()
                            .automation_name(r.title.as_str())
                            .content(two_lines(&r.title, &under)),
                    )
                }),
            )
    };
    let adds = !matches!(
        picker,
        Picker::Monster(p) if matches!(p.purpose, MonsterFor::Second | MonsterFor::Reference)
    );
    let content = StackPanel::new().spacing(8.0).children((
        StackPanel::new().spacing(8.0).keyed_children(filters),
        TextBox::new()
            .automation_name(tr("Search"))
            .placeholder_text(tr("Search"))
            .text(search.clone())
            .on_text_changed(context.callback(|text: String| pick(PickerInput::Search(text)))),
        // A new list for each search and filter.
        Grid::new().keyed_children([KeyedView::new(format!("{search}\u{1}{filter_key}"), list)]),
    ));
    ContentDialog::new()
        .title(picker.title())
        .primary_button_text(match (selected.is_some(), adds) {
            (false, _) => String::new(),
            (true, true) => plain(tr("_Add")),
            (true, false) => plain(tr("_OK")),
        })
        .close_button_text(plain(tr("_Cancel")))
        .is_open(true)
        .on_closed(context.callback(|result| {
            input(TemplateInput::ClosePicker(
                result == ContentDialogResult::Primary,
            ))
        }))
        .content(ScrollViewer::new().content(content))
}

/// IDD 313 once its monster is picked: how many come with the item.
fn escort_dialog(monster: &str, values: [i32; 4], context: &mut ViewContext<Shell>) -> View {
    ContentDialog::new()
        .title(tr("Select a Monster and the number appearing"))
        .primary_button_text(plain(tr("_Add")))
        .close_button_text(plain(tr("_Cancel")))
        .is_open(true)
        .on_closed(context.callback(|result| {
            input(TemplateInput::CloseEscort(
                result == ContentDialogResult::Primary,
            ))
        }))
        .content(StackPanel::new().spacing(12.0).children((
            two_lines(tr("Monster"), monster),
            StackPanel::new().spacing(4.0).children((
                ui::body(tr("Number Appearing")),
                dice(
                    tr("Number Appearing"),
                    values,
                    ESCORT_RANGES,
                    |k, v| input(TemplateInput::EscortDice(k, v)),
                    context,
                ),
            )),
        )))
}
