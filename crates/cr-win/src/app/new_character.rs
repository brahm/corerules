//! The first page of New Character: the rules and the campaign setting,
//! with the GTK app's texts.

use cr_i18n::tr;
use cr_store::settings::SETTINGS;
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::ui;

pub fn view(
    shell: &Shell,
    skills_and_powers: bool,
    setting: usize,
    context: &mut ViewContext<Shell>,
) -> View {
    // Core Rules 2.0 has no Player's Option.
    let expansion = shell.install.as_ref().is_some_and(|i| !i.is_core20());
    let rules_text = if skills_and_powers {
        tr(
            "Skills & Powers: subabilities, and character points for racial and \
            class abilities, proficiencies and traits",
        )
    } else {
        tr("Player's Handbook: proficiency slots")
    };
    let rules: View = if expansion {
        StackPanel::new().spacing(4.0).children((
            ui::strong(tr("Rules")),
            ui::caption(tr("The original's Core Rules and Player's Option buttons")),
            ui::card(
                StackPanel::new().spacing(8.0).children((
                    RadioButtons::new()
                        .items_source([plain(tr("Core Rules")), plain(tr("Player's Option"))])
                        .selected_index(usize::from(skills_and_powers))
                        .on_selection_changed(context.callback(Message::Rules)),
                    ui::caption(rules_text),
                )),
            ),
        ))
    } else {
        View::empty()
    };
    let chosen = &SETTINGS[setting.min(SETTINGS.len() - 1)];
    ui::page(
        StackPanel::new().spacing(20.0).children((
            ui::title(tr("New Character")),
            rules,
            StackPanel::new().spacing(4.0).children((
                ui::strong(tr("Campaign Setting")),
                ui::caption(tr(
                    "The setting limits the races and classes offered on the next pages. \
                     The original program has no settings; these follow each setting's \
                     campaign book.",
                )),
                ui::card(
                    StackPanel::new().spacing(8.0).children((
                        RadioButtons::new()
                            .items_source(SETTINGS.iter().map(|s| tr(s.name).to_owned()))
                            .selected_index(setting)
                            .max_columns(2)
                            .on_selection_changed(context.callback(Message::Setting)),
                        ui::caption(tr(chosen.description)),
                    )),
                ),
            )),
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .children((
                    Button::new()
                        .style(ButtonStyle::Accent)
                        .on_click(context.message(Message::Next))
                        .content(plain(tr("_Next"))),
                    Button::new()
                        .on_click(context.message(Message::Back))
                        .content(plain(tr("_Cancel"))),
                )),
        )),
    )
}
