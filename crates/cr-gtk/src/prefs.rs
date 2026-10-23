//! The app's own settings, kept in a key file in the user's configuration
//! folder, and the Preferences dialog: the interface language, and whether
//! New Character shows the original's "Core Rules Vs Player's Option"
//! message.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;

use adw::prelude::*;
use cr_i18n::{Language, tr};
use gtk::glib;

const GROUP: &str = "interface";
const LANGUAGE: &str = "language";
/// The original's `Flags\CoreVsOptionOff`, set by the message's "Do not
/// show this window again" and cleared by User Options' "Turn 'Core Rules
/// Vs Player's Option' message back on".
const CORE_VS_OPTION_OFF: &str = "core-vs-option-off";

/// The system's language, read before a chosen one overrides `LANGUAGE`.
static SYSTEM: OnceLock<Language> = OnceLock::new();

fn path() -> PathBuf {
    glib::user_config_dir()
        .join("corerules")
        .join("settings.ini")
}

fn load() -> glib::KeyFile {
    let file = glib::KeyFile::new();
    let _ = file.load_from_file(path(), glib::KeyFileFlags::KEEP_COMMENTS);
    file
}

/// The language chosen in Preferences, or `None` to follow the system.
pub fn chosen_language() -> Option<Language> {
    let code = load().string(GROUP, LANGUAGE).ok()?;
    Language::from_code(&code)
}

fn save(file: &glib::KeyFile) -> Result<(), glib::Error> {
    let path = path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    file.save_to_file(path)
}

fn save_language(language: Option<Language>) -> Result<(), glib::Error> {
    let file = load();
    match language {
        Some(l) => file.set_string(GROUP, LANGUAGE, l.code()),
        None => {
            let _ = file.remove_key(GROUP, LANGUAGE);
        }
    }
    save(&file)
}

/// Whether New Character shows the "Core Rules Vs Player's Option"
/// message: until "Do not show this window again" is ticked, as in the
/// original.
pub fn core_vs_option_shown() -> bool {
    !load().boolean(GROUP, CORE_VS_OPTION_OFF).unwrap_or(false)
}

pub fn set_core_vs_option_shown(shown: bool) -> Result<(), glib::Error> {
    let file = load();
    if shown {
        let _ = file.remove_key(GROUP, CORE_VS_OPTION_OFF);
    } else {
        file.set_boolean(GROUP, CORE_VS_OPTION_OFF, true);
    }
    save(&file)
}

/// Picks the interface language before any text is shown: the one chosen
/// in Preferences, else the system's. A chosen language is also passed to
/// GTK, so its own dialogs (the file chooser) speak it too.
pub fn apply_language() {
    let system = *SYSTEM.get_or_init(cr_i18n::system_language);
    let language = match chosen_language() {
        Some(language) => {
            // SAFETY: called first thing in main, before any thread starts.
            unsafe { std::env::set_var("LANGUAGE", language.code()) };
            language
        }
        None => system,
    };
    cr_i18n::set_language(language);
}

/// Shows the Preferences dialog over `parent`. `notify` shows a message,
/// as a toast.
pub fn show(parent: &impl IsA<gtk::Widget>, notify: impl Fn(&str) + 'static) {
    let dialog = adw::PreferencesDialog::new();
    dialog.set_title(tr("Preferences"));
    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::builder()
        .title(tr("Interface"))
        .build();

    let system = *SYSTEM.get_or_init(cr_i18n::system_language);
    let follow = tr!(
        "System Language ({language})",
        language = system.native_name()
    );
    let mut names = vec![follow.as_str()];
    names.extend(Language::ALL.iter().map(|l| l.native_name()));
    let row = adw::ComboRow::builder()
        .title(tr("Language"))
        .subtitle(tr("Game data from the original files stays in English"))
        .model(&gtk::StringList::new(&names))
        .build();
    let selected = chosen_language()
        .and_then(|l| Language::ALL.iter().position(|&x| x == l))
        .map_or(0, |i| i + 1);
    row.set_selected(selected as u32);
    let notify = Rc::new(notify);
    row.connect_selected_notify({
        let notify = notify.clone();
        move |row| {
            let choice = match row.selected() {
                0 => None,
                i => Language::ALL.get(i as usize - 1).copied(),
            };
            match save_language(choice) {
                Ok(()) => notify(tr("Restart Core Rules to use the new language")),
                Err(e) => notify(&tr!("Could not save the preferences: {error}", error = e)),
            }
        }
    });
    group.add(&row);
    page.add(&group);

    // User Options' "Turn 'Core Rules Vs Player's Option' message back on".
    let creation = adw::PreferencesGroup::builder()
        .title(tr("Character Creation"))
        .build();
    let message = adw::SwitchRow::builder()
        .title(tr("Core Rules Vs Player's Option Message"))
        .subtitle(tr("Shown in New Character after the rules are chosen"))
        .active(core_vs_option_shown())
        .build();
    message.connect_active_notify(move |row| {
        if let Err(e) = set_core_vs_option_shown(row.is_active()) {
            notify(&tr!("Could not save the preferences: {error}", error = e));
        }
    });
    creation.add(&message);
    page.add(&creation);
    dialog.add(&page);
    dialog.present(Some(parent));
}
