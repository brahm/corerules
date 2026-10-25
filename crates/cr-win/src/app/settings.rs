//! Settings, at the foot of the navigation: the interface language, as in
//! the GTK app's Preferences, and About inside, as Windows 11 apps keep it.

use cr_i18n::{Language, tr};
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::{prefs, ui};

pub fn view(shell: &Shell, context: &mut ViewContext<Shell>) -> View {
    let follow = tr!(
        "System Language ({language})",
        language = prefs::system_language().native_name()
    );
    let languages: Vec<String> = std::iter::once(follow)
        .chain(Language::ALL.iter().map(|l| l.native_name().to_owned()))
        .collect();
    let selected = prefs::chosen_language()
        .and_then(|l| Language::ALL.iter().position(|&x| x == l))
        .map_or(0, |i| i + 1);
    let installation = shell
        .install
        .as_ref()
        .map(|i| i.root().display().to_string())
        .unwrap_or_else(|| tr("No Core Rules Installation").to_owned());
    // Where the data package goes, unless the installation is that package.
    let data_folder: View = if shell
        .install
        .as_ref()
        .is_some_and(|i| cr_store::data_package_dir().as_deref() == Some(i.root()))
    {
        View::empty()
    } else {
        ui::setting(
            tr("Data folder"),
            cr_store::DATA_PACKAGE_SHOWN,
            View::empty(),
        )
    };
    ui::page(
        StackPanel::new().spacing(4.0).children((
            ui::title(tr("Settings")).margin(Thickness::new(0.0, 0.0, 0.0, 16.0)),
            ui::strong(tr("Interface")).margin(Thickness::new(0.0, 0.0, 0.0, 4.0)),
            ui::setting(
                tr("Language"),
                tr("Game data from the original files stays in English"),
                ComboBox::new()
                    .automation_name(tr("Language"))
                    .items_source(languages)
                    .selected_index(selected)
                    .min_width(220.0)
                    .on_selection_changed(context.callback(Message::Language)),
            ),
            ui::strong(plain(tr("_About Core Rules"))).margin(Thickness::new(0.0, 20.0, 0.0, 4.0)),
            ui::setting(
                &format!("Core Rules {}", env!("CARGO_PKG_VERSION")),
                "🄯 2026 Wagner Brahm · MIT",
                website(),
            ),
            ui::setting(tr("Installation"), &installation, View::empty()),
            data_folder,
            ui::card(StackPanel::new().spacing(4.0).children((
                ui::body(tr("AD&D Core Rules")),
                ui::caption(tr(
                    "AD&D, Advanced Dungeons & Dragons and Core Rules are trademarks of Wizards of the \
                     Coast. The rules, books, monsters, pictures and help of AD&D Core Rules 2.0 and its \
                     Expansion are copyrighted by Wizards of the Coast and are not covered by this \
                     app's license. This project is not affiliated with Wizards of the Coast and \
                     contains none of their content: it needs the original discs, or a copy of them, \
                     that you own.",
                )),
            ))),
        )),
    )
}

/// The landing page, as the GTK app's About links it.
fn website() -> View {
    match HyperlinkButton::new().navigate_uri("https://brahm.github.io/corerules/") {
        Ok(link) => link.content("brahm.github.io/corerules"),
        Err(_) => View::empty(),
    }
}
