//! Allowed Classes… on a race as the Mac app edits it, in the record's
//! sheet: the classes and multi-classes of `cr_app::records::allowed`
//! described as a form, as the Windows app's allowed.rs draws them, the
//! shipped ones as they are and the user's with a level limit and
//! Remove, and the menus that add a class or a multi-class.

use cr_app::records::allowed::{AllowedForm, AllowedInput, MOST_LIMIT};
use cr_i18n::tr;

use crate::form::{Builder, FormButton, FormRow};
use crate::record::In;

fn input(input: AllowedInput) -> In {
    In::Allowed(input)
}

/// The race's classes, and the menus that add one.
pub(crate) fn describe(form: &mut Builder<In>, f: &AllowedForm) {
    let enabled = !f.saving;
    form.group(
        tr("Classes Allowed"),
        tr("Standard entries are kept as shipped"),
    );
    let rows = f.rows();
    if rows.is_empty() {
        form.none();
    }
    for row in rows {
        let i = row.index;
        if row.standard {
            form.row(FormRow::Value {
                title: row.name,
                subtitle: tr("Standard").to_owned(),
                value: String::new(),
                buttons: Vec::new(),
            });
            continue;
        }
        let remove: FormButton = form.button(
            tr("Remove"),
            "minus.circle",
            enabled,
            input(AllowedInput::Remove(i)),
        );
        form.number(
            (
                &row.name,
                &format!("{}; {}", tr("Custom"), tr("Level Limit (0: none)")),
            ),
            row.limit.into(),
            (0, MOST_LIMIT.into()),
            vec![remove],
            move |v| input(AllowedInput::Limit(i, v as i32)),
        )
        .enabled(enabled);
    }
    for multi in [false, true] {
        let title = if multi {
            tr("Add _Multi-Class…")
        } else {
            tr("Add _Class…")
        };
        form.group(plain(title).trim_end_matches('…'), "");
        let choices = f.choices(multi);
        let selected = f
            .adding(multi)
            .and_then(|a| choices.iter().position(|c| c == a));
        let add = form.button(
            tr("_Add"),
            "",
            enabled && selected.is_some(),
            input(AllowedInput::Add(multi)),
        );
        let label = if multi {
            tr("Multi-Classes")
        } else {
            tr("Available Classes")
        };
        let names = choices.clone();
        form.choice((label, ""), choices, selected, vec![add], move |i| {
            input(AllowedInput::Adding(multi, names[i].clone()))
        });
    }
}

/// A label without its mnemonic underscore.
fn plain(label: &str) -> String {
    label.replacen('_', "", 1)
}
