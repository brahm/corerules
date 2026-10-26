//! Allowed Classes… on a race, on a page of the window: the classes and
//! multi-classes of `cr_app::records::allowed`, the shipped ones as they
//! are and the user's with a level limit and Remove, and the boxes that
//! add a class or a multi-class.

use cr_app::records::allowed::{AllowedForm, AllowedInput, MOST_LIMIT};
use cr_i18n::tr;
use windows_reactor::*;

use super::create::prompt_dialog;
use super::forms::{heading, with_buttons};
use super::record::{self, add_box, number};
use super::{Message, Shell};
use crate::ui;

fn input(input: AllowedInput) -> Message {
    Message::Allowed(input)
}

pub fn view(form: &AllowedForm, context: &mut ViewContext<Shell>) -> View {
    let enabled = !form.saving;
    let rows: Vec<KeyedView> = form
        .rows()
        .into_iter()
        .map(|row| {
            let i = row.index;
            let controls: View = if row.standard {
                View::empty()
            } else {
                with_buttons(
                    number(
                        &format!("{}: {}", row.name, tr("Level Limit (0: none)")),
                        row.limit.into(),
                        (0, MOST_LIMIT.into()),
                        enabled,
                        move |v| input(AllowedInput::Limit(i, v as i32)),
                        context,
                    ),
                    ui::icon_button(
                        tr("Remove"),
                        Symbol::Delete,
                        input(AllowedInput::Remove(i)),
                        context,
                    ),
                )
            };
            let origin = if row.standard {
                tr("Standard")
            } else {
                tr("Custom")
            };
            KeyedView::new(
                format!("{i}\u{1}{}", row.name),
                ui::setting(&row.name, origin, controls),
            )
        })
        .collect();
    let rows: View = if rows.is_empty() {
        ui::card(ui::body(tr("None")).opacity(0.6))
    } else {
        StackPanel::new().spacing(4.0).keyed_children(rows)
    };
    let adding = |multi: bool, context: &mut ViewContext<Shell>| {
        let label = if multi {
            tr("Multi-Classes")
        } else {
            tr("Available Classes")
        };
        add_box(
            label,
            &form.choices(multi),
            form.adding(multi),
            enabled,
            move |name| input(AllowedInput::Adding(multi, name)),
            input(AllowedInput::Add(multi)),
            context,
        )
    };
    let classes = adding(false, context);
    let multi = adding(true, context);
    let serial = form.prompt_serial();
    let dialogs: Vec<KeyedView> = form
        .prompt()
        .map(|prompt| {
            KeyedView::new(
                format!("prompt{serial}"),
                prompt_dialog(prompt, serial, Message::AllowedAnswer, context),
            )
        })
        .into_iter()
        .collect();
    Grid::new().children((
        ui::page(StackPanel::new().spacing(4.0).children((
            record::header(&form.title(), context),
            heading(
                tr("Classes Allowed"),
                Some(tr("Standard entries are kept as shipped")),
            ),
            rows,
            heading(
                cr_win::text::plain(tr("Add _Class…")).trim_end_matches('…'),
                None,
            ),
            classes,
            heading(
                cr_win::text::plain(tr("Add _Multi-Class…")).trim_end_matches('…'),
                None,
            ),
            multi,
            record::problem(form.error.as_deref()),
            record::buttons(
                !form.saving,
                form.can_save(),
                input(AllowedInput::Save),
                context,
            ),
        ))),
        Grid::new().keyed_children(dialogs),
    ))
}
