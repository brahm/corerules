//! A treasure bundle as the Mac app's forms draw it, from the bundle
//! editor of `cr_app::bundle`, as the Windows app's bundle.rs does: each
//! item with Remove, Move Into and Edit Spells, the spells of a scroll
//! with Remove and Copy To, the items an item holds under it, and the
//! dialog the edit waits on.

use cr_app::bundle::{BundleEditor, BundleInput, Row, Step, scroll_limit_text};
use cr_i18n::tr;

use crate::form::{Builder, FormButton, FormDialog, FormRow};

/// The bundle's items, each with its edits; `editable` is false where
/// they are only shown. `input` makes the input of an edit.
pub(crate) fn rows<I: Clone + Send + Sync + 'static>(
    form: &mut Builder<I>,
    rows: &[Row],
    editable: bool,
    input: fn(BundleInput) -> I,
) {
    if rows.is_empty() {
        form.none();
    }
    for row in rows {
        add_row(form, row, 0, editable, input);
    }
}

fn add_row<I: Clone + Send + Sync + 'static>(
    form: &mut Builder<I>,
    row: &Row,
    depth: u32,
    editable: bool,
    input: fn(BundleInput) -> I,
) {
    let mut buttons = Vec::new();
    if editable {
        buttons.push(form.button(
            tr("_Remove…"),
            "minus.circle",
            true,
            input(BundleInput::Remove(row.path.clone())),
        ));
        buttons.push(form.button(
            tr("_Move Into…"),
            "folder",
            true,
            input(BundleInput::MoveInto(row.path.clone())),
        ));
        if row.edit_spells {
            buttons.push(form.button(
                tr("Edit _Spells…"),
                "pencil",
                true,
                input(BundleInput::EditSpells(row.path.clone())),
            ));
        }
    }
    form.row(FormRow::Tree {
        title: row.title.clone(),
        subtitle: row.subtitle.clone(),
        depth,
        dimmed: row.flag,
        buttons,
    });
    for (i, (name, pages)) in row.spells.iter().enumerate() {
        let mut buttons = Vec::new();
        if editable {
            buttons.push(form.button(
                tr("Remove"),
                "minus.circle",
                true,
                input(BundleInput::RemoveSpell(row.path.clone(), i)),
            ));
            buttons.push(form.button(
                tr("_Copy To…"),
                "doc.on.doc",
                true,
                input(BundleInput::CopySpell(row.path.clone(), i)),
            ));
        }
        form.row(FormRow::Tree {
            title: name.clone(),
            subtitle: pages.clone(),
            depth: depth + 1,
            dimmed: false,
            buttons,
        });
    }
    for inner in &row.rows {
        add_row(form, inner, depth + 1, editable, input);
    }
}

/// The dialog the bundle editor waits on, if any; its controls are
/// numbered with the form's. `answer` makes the input of its buttons.
pub(crate) fn dialog<I: Clone + Send + Sync + 'static>(
    form: &mut Builder<I>,
    editor: &BundleEditor,
    input: fn(BundleInput) -> I,
    answer: fn(u64, bool) -> I,
) -> Option<FormDialog> {
    let step = editor.step()?;
    let serial = editor.serial();
    let page = form.take_groups();
    let button = |form: &mut Builder<I>, label: &str, enabled: bool, yes: bool| FormButton {
        enabled,
        ..form.button(label, "", true, answer(serial, yes))
    };
    let (title, close, accept) = match step {
        Step::Add(add) => {
            let parts = add.shown();
            let selected = add
                .chosen
                .as_ref()
                .and_then(|c| parts.iter().position(|p| p == c));
            form.group("", "")
                .choice(
                    (tr("Category"), ""),
                    add.category_names(),
                    Some(add.category),
                    Vec::new(),
                    move |i| input(BundleInput::Category(i)),
                )
                .text(tr("Search"), &add.search, false, Vec::new(), move |text| {
                    input(BundleInput::Search(text))
                });
            if parts.is_empty() {
                form.note(tr("No Results"));
            } else {
                let chosen = parts.clone();
                let options: Vec<(String, String)> =
                    parts.into_iter().map(|p| (p, String::new())).collect();
                form.radio(&options, selected, move |i| {
                    input(BundleInput::Part(
                        chosen.get(i).cloned().unwrap_or_default(),
                    ))
                });
            }
            (
                tr("Add Item").to_owned(),
                button(form, tr("_Cancel"), true, false),
                Some(button(form, tr("_Add"), selected.is_some(), true)),
            )
        }
        Step::HowMany(how) => {
            form.group("", "").number(
                (tr("How many?"), ""),
                how.value.into(),
                (how.min.into(), how.max.into()),
                Vec::new(),
                move |v| input(BundleInput::Count(v as u32)),
            );
            (
                how.title.to_owned(),
                button(form, tr("_Cancel"), true, false),
                Some(button(form, how.action, true, true)),
            )
        }
        Step::Choose(choose) => {
            // The nodes an item holds are indented under it.
            let options: Vec<(String, String)> = choose
                .nodes
                .iter()
                .map(|n| {
                    (
                        format!("{}{}", "    ".repeat(n.depth), n.label),
                        String::new(),
                    )
                })
                .collect();
            form.group("", "").radio(&options, choose.chosen, move |i| {
                input(BundleInput::Choose(i))
            });
            (
                choose.title.to_owned(),
                button(form, tr("_Cancel"), true, false),
                Some(button(form, choose.action, choose.chosen.is_some(), true)),
            )
        }
        Step::Notice { title, text } => {
            form.group("", "").note(text);
            (title.clone(), button(form, tr("_OK"), true, false), None)
        }
        Step::Ask(ask) => {
            form.group("", "").note(ask.text);
            (
                ask.title.to_owned(),
                button(form, tr("_No"), true, false),
                Some(button(form, tr("_Yes"), true, true)),
            )
        }
        Step::Scroll(scroll) => {
            form.group(
                &scroll.heading(),
                tr("Select Spells that will be in the scroll."),
            );
            if scroll.selected.is_empty() {
                form.none();
            }
            for (i, name) in scroll.selected.iter().enumerate() {
                let remove = form.button(
                    tr("Remove"),
                    "minus.circle",
                    true,
                    input(BundleInput::ScrollRemove(i)),
                );
                form.row(FormRow::Tree {
                    title: name.clone(),
                    subtitle: String::new(),
                    depth: 0,
                    dimmed: false,
                    buttons: vec![remove],
                });
            }
            // Past the scroll's limit, Add asks first.
            if scroll.asking {
                let yes = form.button(tr("_Yes"), "", true, input(BundleInput::ScrollLimit(true)));
                let no = form.button(tr("_No"), "", true, input(BundleInput::ScrollLimit(false)));
                form.banner(scroll_limit_text(), false).row(FormRow::Value {
                    title: tr("Warning").to_owned(),
                    subtitle: String::new(),
                    value: String::new(),
                    buttons: vec![no, yes],
                });
            }
            let adding = scroll
                .adding
                .as_ref()
                .and_then(|a| scroll.choices.iter().position(|c| c == a));
            let add = FormButton {
                enabled: adding.is_some() && !scroll.asking,
                ..form.button(tr("_Add"), "", true, input(BundleInput::ScrollAdd))
            };
            let choices = scroll.choices.clone();
            form.choice(
                (tr("Add Spell…"), ""),
                scroll.choices.clone(),
                adding,
                vec![add],
                move |i| {
                    input(BundleInput::ScrollAdding(
                        choices.get(i).cloned().unwrap_or_default(),
                    ))
                },
            );
            (
                scroll.title.clone(),
                button(form, tr("_Cancel"), true, false),
                Some(button(form, tr("_Done"), true, true)),
            )
        }
    };
    let groups = form.take_groups();
    form.put_groups(page);
    Some(FormDialog {
        title,
        groups,
        close,
        accept,
    })
}
