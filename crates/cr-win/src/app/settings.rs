//! Settings, at the foot of the navigation: the interface language and
//! the Core Rules Vs Player's Option message, as in the GTK app's
//! Preferences, and About inside, as Windows 11 apps keep it, with what
//! the GTK app's About window shows.

use cr_app::about::{self, Original, System};
use cr_i18n::{Language, tr, trc};
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
    let heading = |text: &str| ui::strong(text).margin(Thickness::new(0.0, 20.0, 0.0, 4.0));
    ui::page(
        StackPanel::new().spacing(4.0).children((
            ui::title(trc("preferences", "Settings")).margin(Thickness::new(0.0, 0.0, 0.0, 16.0)),
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
            heading(tr("Character Creation")),
            ui::setting(
                tr("Core Rules Vs Player's Option Message"),
                tr("Shown in New Character after the rules are chosen"),
                ToggleSwitch::new()
                    .automation_name(tr("Core Rules Vs Player's Option Message"))
                    .is_on(prefs::core_vs_option_shown())
                    .on_toggled(context.callback(Message::CoreVsOption)),
            ),
            heading(&plain(tr("_About Core Rules"))),
            about_card(),
            ui::setting(tr("Installation"), &installation, View::empty()),
            data_folder,
            whats_new(shell, context),
            credits(shell, context),
            legal(shell, context),
            troubleshooting(shell, context),
        )),
    )
}

/// The app's name, version and description, with its website.
fn about_card() -> View {
    let mut paragraphs: Vec<KeyedView> = about::description(System::Windows)
        .into_iter()
        .enumerate()
        .map(|(i, text)| KeyedView::new(i.to_string(), ui::body(text)))
        .collect();
    paragraphs.insert(
        0,
        KeyedView::new(
            "name",
            StackPanel::new().spacing(2.0).children((
                ui::subtitle(format!("Core Rules {}", about::VERSION)),
                ui::caption(format!("{} · MIT", about::COPYRIGHT)),
            )),
        ),
    );
    paragraphs.push(KeyedView::new(
        "website",
        link(about::WEBSITE, "brahm.github.io/corerules"),
    ));
    ui::card(StackPanel::new().spacing(10.0).keyed_children(paragraphs))
}

/// The newest release's notes.
fn whats_new(shell: &Shell, context: &mut ViewContext<Shell>) -> View {
    let lines: Vec<KeyedView> = about::release_notes_text()
        .into_iter()
        .enumerate()
        .map(|(i, line)| KeyedView::new(i.to_string(), ui::body(line)))
        .collect();
    if lines.is_empty() {
        return View::empty();
    }
    expander(
        (shell, context, Section::WhatsNew),
        &tr!("What's New in {version}", version = about::VERSION),
        StackPanel::new().spacing(8.0).keyed_children(lines),
    )
}

/// Who made the app, and the original program's Credits window.
fn credits(shell: &Shell, context: &mut ViewContext<Shell>) -> View {
    let mut rows = vec![
        KeyedView::new(
            "developer",
            credit(
                tr("Developer"),
                link(about::DEVELOPER_URL, about::DEVELOPER),
            ),
        ),
        KeyedView::new(
            "assistant",
            credit(
                tr("Assistant"),
                link(about::ASSISTANT_URL, about::ASSISTANT),
            ),
        ),
    ];
    if let Some(translators) = about::translators() {
        rows.push(KeyedView::new(
            "translators",
            credit(tr("Translators"), ui::body(translators).into()),
        ));
    }
    if let Some((title, lines)) = shell
        .install
        .as_ref()
        .and_then(|i| Original::of(&i.original_about()).credits)
    {
        rows.push(KeyedView::new(
            "original",
            credit(&title, ui::body(lines.join("\n")).into()),
        ));
    }
    expander(
        (shell, context, Section::Credits),
        tr("Credits"),
        StackPanel::new().spacing(12.0).keyed_children(rows),
    )
}

/// The notice on the original's content, and the original program's
/// About box and license.
/// The license of Microsoft.Web.WebView2.Core.dll, which the app ships.
const WEBVIEW2_LICENSE: &str = include_str!("../../package/WebView2-LICENSE.txt");
/// The license of the Material Design Icons the navigation draws.
const ICONS_LICENSE: &str = include_str!("../../package/MaterialDesignIcons-LICENSE.txt");

fn legal(shell: &Shell, context: &mut ViewContext<Shell>) -> View {
    let mut rows = vec![KeyedView::new(
        "notice",
        credit(tr("AD&D Core Rules"), ui::body(about::legal()).into()),
    )];
    if let Some(original) = shell
        .install
        .as_ref()
        .and_then(|i| Original::of(&i.original_about()).about)
    {
        let text = [original.rest, original.license.unwrap_or_default()]
            .into_iter()
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        rows.push(KeyedView::new(
            "original",
            credit(
                &original.name,
                ui::body(text).is_text_selection_enabled(true).into(),
            ),
        ));
    }
    // The BSD license of the WebView2 file shipped with the app.
    rows.push(KeyedView::new(
        "webview2",
        credit(
            "Microsoft WebView2",
            ui::body(WEBVIEW2_LICENSE.trim_end())
                .is_text_selection_enabled(true)
                .into(),
        ),
    ));
    rows.push(KeyedView::new(
        "icons",
        credit(
            "Material Design Icons",
            ui::body(ICONS_LICENSE.trim_end())
                .is_text_selection_enabled(true)
                .into(),
        ),
    ));
    expander(
        (shell, context, Section::Legal),
        tr("Legal"),
        StackPanel::new().spacing(12.0).keyed_children(rows),
    )
}

/// What a bug report needs, to copy, and where reports go.
fn troubleshooting(shell: &Shell, context: &mut ViewContext<Shell>) -> View {
    let content = StackPanel::new().spacing(12.0).children((
        ui::body(tr(
            "A bug report helps most with these details and what you did when it happened.",
        )),
        ui::caption(debug_info(shell)).is_text_selection_enabled(true),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                Button::new()
                    .on_click(context.message(Message::CopyDebugInfo))
                    .content(tr("Copy Details")),
                link(about::ISSUES, tr("Report an Issue")),
            )),
    ));
    expander(
        (shell, context, Section::Troubleshooting),
        tr("Troubleshooting"),
        content,
    )
}

/// What a bug report needs: the versions, how the app was installed and
/// which data it opened, as the GTK app's About gives them.
pub fn debug_info(shell: &Shell) -> String {
    let package = if std::env::current_exe()
        .is_ok_and(|exe| exe.to_string_lossy().contains("\\WindowsApps\\"))
    {
        "MSIX"
    } else {
        "unpackaged"
    };
    let data = shell
        .install
        .as_ref()
        .map_or_else(|| "none".to_owned(), |i| i.root().display().to_string());
    format!(
        "Core Rules {}\nPackage: {package}\nWindows App SDK 2.4\nLanguage: {}\nData: {data}\n",
        about::VERSION,
        cr_i18n::language().code(),
    )
}

/// The sections of About that open and close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    WhatsNew,
    Credits,
    Legal,
    Troubleshooting,
}

/// A section of About, open as the shell keeps it: WinUI's Expander
/// opens only when its state says so.
fn expander(
    (shell, context, section): (&Shell, &mut ViewContext<Shell>, Section),
    header: &str,
    content: impl Into<View>,
) -> View {
    Expander::new()
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .is_expanded(shell.about_open.contains(&section))
        .on_is_expanded_changed(context.callback(move |open| Message::AboutSection(section, open)))
        .slots([
            SlotView::new(ExpanderSlot::Header, ui::body(header)),
            SlotView::new(ExpanderSlot::Content, content),
        ])
}

/// A credit: who or what, under its heading.
fn credit(heading: &str, content: View) -> View {
    StackPanel::new()
        .spacing(2.0)
        .children((ui::caption(heading), content))
}

fn link(uri: &str, text: &str) -> View {
    match HyperlinkButton::new().navigate_uri(uri) {
        Ok(link) => link.content(text.to_owned()),
        Err(_) => ui::body(text).into(),
    }
}
