//! A treasure table of the user databases as the Mac app edits it, in the
//! record's sheet: the editor of `cr_app::records::tables` described as a
//! form, as the Windows app's treasure_tables.rs draws it, its name and
//! entries, each with its roll range and weight, Add Entry under them,
//! and an entry's details in a dialog over them, with "How many to
//! remove?" in its place while it asks.

use cr_app::records::tables::{
    self, DetailInput, Details, MOST_DICE, MOST_PROBABILITY, MOST_SPELLS, MOST_WEIGHT, Pick,
    TableForm, TableInput,
};
use cr_i18n::tr;
use cr_store::userdb::tables::{contained_line, max_spell_level, source_label};

use crate::form::{Builder, FormDialog, FormRow};
use crate::record::{In, dice};

fn input(input: TableInput) -> In {
    In::Table(input)
}

fn detail(change: DetailInput) -> In {
    input(TableInput::Detail(change))
}

/// The editor's page, and the details over it while they are open.
pub(crate) fn describe(form: &mut Builder<In>, f: &TableForm) -> Option<FormDialog> {
    let enabled = !f.saving;
    form.group("", "");
    form.text(tr("Name"), &f.table.name, false, Vec::new(), |name| {
        input(TableInput::Name(name))
    });
    form.group(
        tr("Entries"),
        tr("Each entry is rolled in proportion to its weight"),
    );
    let rows = f.rows();
    if rows.is_empty() {
        form.row(FormRow::Note {
            text: tr("No entries").to_owned(),
            secondary: true,
        });
    }
    for (i, row) in rows.into_iter().enumerate() {
        let mut buttons = Vec::new();
        if row.details.is_some() {
            buttons.push(form.button(
                tr("Details…"),
                "slider.horizontal.3",
                enabled,
                input(TableInput::Details(i)),
            ));
        }
        buttons.push(form.button(
            tr("Remove Entry"),
            "minus.circle",
            enabled,
            input(TableInput::Remove(i)),
        ));
        let subtitle = match &row.details {
            Some(summary) => format!("{}\n{summary}", row.range),
            None => row.range.clone(),
        };
        form.number(
            (&row.title, &subtitle),
            row.weight.into(),
            (0, MOST_WEIGHT.into()),
            buttons,
            move |v| input(TableInput::Weight(i, v as u32)),
        )
        .enabled(enabled);
    }
    form.group(tr("Add Entry"), "");
    add_entry(
        form,
        f,
        f.pick,
        false,
        |s| input(TableInput::AddSource(s)),
        |i| input(TableInput::AddItem(i)),
        input(TableInput::Add),
    );
    let details = f.details.as_ref()?;
    let page = form.take_groups();
    let dialog = match details.removing {
        Some((_, most, count)) => how_many(form, most, count),
        None => details_dialog(form, f, details),
    };
    form.put_groups(page);
    Some(dialog)
}

/// A source's menu and its item's, with Add beside the item: Add Entry,
/// or (`contents`) Always Contains.
fn add_entry(
    form: &mut Builder<In>,
    f: &TableForm,
    pick: Pick,
    contents: bool,
    source: impl Fn(usize) -> In + Send + Sync + 'static,
    item: impl Fn(usize) -> In + Send + Sync + 'static,
    add: In,
) {
    let sources: Vec<String> = f
        .sources()
        .sources(contents)
        .iter()
        .map(|s| source_label(s))
        .collect();
    form.choice(
        (tr("Category"), ""),
        sources,
        pick.source,
        Vec::new(),
        source,
    );
    if pick.source.is_some() {
        let items = f.items(pick, contents).to_vec();
        let add = form.button(tr("_Add"), "", pick.item.is_some(), add);
        form.choice((tr("Item"), ""), items, pick.item, vec![add], item);
    }
}

/// A whole number of the details in `0..=most`.
fn count(
    form: &mut Builder<In>,
    (title, subtitle): (&str, &str),
    value: u32,
    most: u32,
    changed: fn(u32) -> DetailInput,
) {
    form.number(
        (title, subtitle),
        value.into(),
        (0, most.into()),
        Vec::new(),
        move |v| detail(changed(v as u32)),
    );
}

/// Dice of the details: count, sides and bonus.
fn dice_row(
    form: &mut Builder<In>,
    title: &str,
    values: [u32; 3],
    changed: impl Fn(usize, u32) -> DetailInput + Clone + Send + Sync + 'static,
) {
    let most = i64::from(MOST_DICE);
    let numbers = dice(
        form,
        title,
        values.map(i64::from),
        [(0, most); 3],
        move |k, v| detail(changed(k, v as u32)),
    );
    form.row(FormRow::Numbers {
        title: title.to_owned(),
        numbers,
        separators: vec!["d".to_owned(), "+".to_owned()],
        enabled: true,
    });
}

/// Treasure Table Entry Details.
fn details_dialog(form: &mut Builder<In>, f: &TableForm, d: &Details) -> FormDialog {
    let e = &d.entry;
    form.group("", "");
    count(
        form,
        (tr("Probability"), ""),
        e.weight,
        MOST_PROBABILITY,
        DetailInput::Probability,
    );
    dice_row(
        form,
        tr("Number Found"),
        e.quantity_dice,
        DetailInput::Found,
    );
    count(
        form,
        (
            tr("Chance Intelligent"),
            tr("Percent chance of an intelligent weapon"),
        ),
        e.intelligent_chance.into(),
        100,
        DetailInput::Intelligent,
    );
    form.group(
        tr("Random Contents"),
        tr("The item holds what these tables roll"),
    );
    for k in 0..2 {
        let (choices, chosen) = f.sub_table_choices(k);
        let label = if k == 0 {
            tr("First Table")
        } else {
            tr("Second Table")
        };
        form.choice((label, ""), choices, Some(chosen), Vec::new(), move |i| {
            detail(DetailInput::SubTable(k, i))
        });
        dice_row(
            form,
            &format!("{label}: {}", tr("Number Found")),
            tables::sub_table(e, k).1,
            move |n, v| DetailInput::SubDice(k, n, v),
        );
    }
    form.group(tr("Always Contains"), "");
    if d.contained.is_empty() {
        form.row(FormRow::Note {
            text: tr("Nothing").to_owned(),
            secondary: true,
        });
    }
    for (i, item) in d.contained.iter().enumerate() {
        let remove = form.on_press(detail(DetailInput::Remove(i)));
        form.row(FormRow::Item {
            title: contained_line(item),
            subtitle: String::new(),
            number: None,
            remove: Some(remove),
            icon: None,
        });
    }
    add_entry(
        form,
        f,
        d.pick,
        true,
        |s| detail(DetailInput::Source(s)),
        |i| detail(DetailInput::Item(i)),
        detail(DetailInput::Add),
    );
    form.group(
        tr("Scroll Spells"),
        tr("Scrolls of protection should not have any of these set. This is only for spell scrolls."),
    );
    let wizard = e.spell_wizard != 0;
    form.choice(
        (tr("Spell Category"), ""),
        vec![tr("Wizard").to_owned(), tr("Priest").to_owned()],
        Some(usize::from(!wizard)),
        Vec::new(),
        |i| detail(DetailInput::Wizard(i == 0)),
    );
    let highest = max_spell_level(wizard);
    count(
        form,
        (tr("Lowest Level"), ""),
        e.spell_min_level,
        highest,
        DetailInput::Lowest,
    );
    count(
        form,
        (tr("Highest Level"), ""),
        e.spell_max_level,
        highest,
        DetailInput::Highest,
    );
    count(
        form,
        (tr("Number of Spells"), tr("0 for none")),
        e.spell_count,
        MOST_SPELLS,
        DetailInput::Spells,
    );
    let close = form.button(
        tr("_Cancel"),
        "",
        true,
        input(TableInput::CloseDetails(false)),
    );
    let accept = form.button(tr("_OK"), "", true, input(TableInput::CloseDetails(true)));
    FormDialog {
        title: e.item.name.clone(),
        groups: form.take_groups(),
        close,
        accept: Some(accept),
    }
}

/// "How many to remove?" of a contained item there are several of.
fn how_many(form: &mut Builder<In>, most: u32, count: u32) -> FormDialog {
    form.group("", "");
    self::count(
        form,
        (tr("How many to remove?"), ""),
        count,
        most,
        DetailInput::RemoveCount,
    );
    let close = form.button(
        tr("_Cancel"),
        "",
        true,
        detail(DetailInput::ConfirmRemove(false)),
    );
    let accept = form.button(
        tr("_Remove"),
        "",
        true,
        detail(DetailInput::ConfirmRemove(true)),
    );
    FormDialog {
        title: tr("How many to remove?").to_owned(),
        groups: form.take_groups(),
        close,
        accept: Some(accept),
    }
}
