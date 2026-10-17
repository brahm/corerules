//! The app's own settings, kept in a key file in the user's configuration
//! folder, and the Preferences dialog. The only setting so far is the
//! interface language.

use std::path::PathBuf;
use std::sync::OnceLock;

use adw::prelude::*;
use cr_i18n::{Language, tr};
use gtk::glib;

const GROUP: &str = "interface";
const LANGUAGE: &str = "language";

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

fn save_language(language: Option<Language>) -> Result<(), glib::Error> {
    let file = load();
    match language {
        Some(l) => file.set_string(GROUP, LANGUAGE, l.code()),
        None => {
            let _ = file.remove_key(GROUP, LANGUAGE);
        }
    }
    let path = path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    file.save_to_file(path)
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
    row.connect_selected_notify(move |row| {
        let choice = match row.selected() {
            0 => None,
            i => Language::ALL.get(i as usize - 1).copied(),
        };
        match save_language(choice) {
            Ok(()) => notify(tr("Restart Core Rules to use the new language")),
            Err(e) => notify(&tr!("Could not save the preferences: {error}", error = e)),
        }
    });
    group.add(&row);
    page.add(&group);
    dialog.add(&page);
    dialog.present(Some(parent));
}
