//! The app's own settings, in the same key file as the GTK app keeps
//! (`[interface]`, `language`), here in `Core Rules\settings.ini` of the
//! user's local application data, which an MSIX package keeps apart for
//! the app.

use std::path::PathBuf;
use std::sync::OnceLock;

use cr_i18n::Language;

const GROUP: &str = "[interface]";
const LANGUAGE: &str = "language";

/// The system's language, read once.
static SYSTEM: OnceLock<Language> = OnceLock::new();

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(base).join("Core Rules").join("settings.ini"))
}

/// The value of `key` in the `[interface]` group.
fn value(text: &str, key: &str) -> Option<String> {
    let mut in_group = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_group = line == GROUP;
        } else if in_group
            && let Some((k, v)) = line.split_once('=')
            && k.trim() == key
        {
            return Some(v.trim().to_owned());
        }
    }
    None
}

/// The language chosen in Settings, or `None` to follow Windows.
pub fn chosen_language() -> Option<Language> {
    let text = std::fs::read_to_string(path()?).ok()?;
    Language::from_code(&value(&text, LANGUAGE)?)
}

/// Keeps `language` for the next start, or forgets the choice for `None`.
pub fn save_language(language: Option<Language>) -> std::io::Result<()> {
    let path = path().ok_or_else(|| std::io::Error::other("LOCALAPPDATA is not set"))?;
    let mut text = GROUP.to_owned();
    text.push('\n');
    if let Some(language) = language {
        text.push_str(&format!("{LANGUAGE}={}\n", language.code()));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text)
}

/// The language Windows asks for.
pub fn system_language() -> Language {
    *SYSTEM.get_or_init(cr_i18n::system_language)
}

/// Picks the interface language before any text is shown: the one chosen
/// in Settings, else the Windows display language.
pub fn apply_language() {
    let chosen = chosen_language();
    cr_i18n::set_language(chosen.unwrap_or_else(system_language));
    winui_language(chosen);
}

/// Asks WinUI for its own texts (the Settings item, Back, More options,
/// the number boxes) in the chosen language too, or in the Windows display
/// language for `None`. Windows keeps the override for a packaged app
/// only; the process's preferred languages cover a run without a package.
fn winui_language(chosen: Option<Language>) {
    use windows::Globalization::ApplicationLanguages;
    use windows::Win32::Globalization::{MUI_LANGUAGE_NAME, SetProcessPreferredUILanguages};
    use windows::core::{HSTRING, PCWSTR};

    let tag = chosen.map_or(String::new(), |l| l.code().replace('_', "-"));
    // Fails without a package, where there is nothing to keep.
    let _ = ApplicationLanguages::SetPrimaryLanguageOverride(&HSTRING::from(tag.as_str()));
    if !tag.is_empty() {
        // A list of names ending in an empty one.
        let names: Vec<u16> = tag.encode_utf16().chain([0, 0]).collect();
        // SAFETY: `names` is a double-terminated list that outlives the call.
        let _ = unsafe {
            SetProcessPreferredUILanguages(MUI_LANGUAGE_NAME, PCWSTR(names.as_ptr()), None)
        };
    }
}
