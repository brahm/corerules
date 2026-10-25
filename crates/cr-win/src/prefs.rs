//! The app's own settings, in the same key file as the GTK app keeps
//! (`[interface]`, `language`), here in `Core Rules\settings.ini` of the
//! user's local application data, which an MSIX package keeps apart for
//! the app.

use std::path::PathBuf;
use std::sync::OnceLock;

use cr_i18n::Language;

const GROUP: &str = "[interface]";
const LANGUAGE: &str = "language";
/// The original's `Flags\CoreVsOptionOff`, set by the message's "Do not
/// show this window again" and cleared in Settings, as in the GTK app.
const CORE_VS_OPTION_OFF: &str = "core-vs-option-off";

/// The system's language, read once.
static SYSTEM: OnceLock<Language> = OnceLock::new();

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(base).join("Core Rules").join("settings.ini"))
}

fn load() -> String {
    path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default()
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

/// `text` with `key` of the `[interface]` group set to `value`, or left
/// out for `None`; other lines stay as they are.
fn with_value(text: &str, key: &str, value: Option<&str>) -> String {
    let mut out = Vec::new();
    let (mut in_group, mut seen_group, mut done) = (false, false, false);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if in_group && !done {
                out.extend(value.map(|v| format!("{key}={v}")));
                done = true;
            }
            in_group = trimmed == GROUP;
            seen_group |= in_group;
        } else if in_group
            && trimmed
                .split_once('=')
                .is_some_and(|(k, _)| k.trim() == key)
        {
            if !done {
                out.extend(value.map(|v| format!("{key}={v}")));
                done = true;
            }
            continue;
        }
        out.push(line.to_owned());
    }
    if !done && let Some(value) = value {
        if !seen_group {
            out.push(GROUP.to_owned());
        }
        out.push(format!("{key}={value}"));
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

fn save(key: &str, value: Option<&str>) -> std::io::Result<()> {
    let path = path().ok_or_else(|| std::io::Error::other("LOCALAPPDATA is not set"))?;
    let text = with_value(&load(), key, value);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text)
}

/// The language chosen in Settings, or `None` to follow Windows.
pub fn chosen_language() -> Option<Language> {
    Language::from_code(&value(&load(), LANGUAGE)?)
}

/// Keeps `language` for the next start, or forgets the choice for `None`.
pub fn save_language(language: Option<Language>) -> std::io::Result<()> {
    save(LANGUAGE, language.map(|l| l.code()))
}

/// Whether New Character shows the "Core Rules Vs Player's Option"
/// message: until "Do not show this window again" is ticked, as in the
/// original.
pub fn core_vs_option_shown() -> bool {
    value(&load(), CORE_VS_OPTION_OFF).is_none_or(|v| v != "true")
}

pub fn set_core_vs_option_shown(shown: bool) -> std::io::Result<()> {
    save(CORE_VS_OPTION_OFF, (!shown).then_some("true"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_set_where_it_was_and_the_rest_stays() {
        let text = "[interface]\nlanguage=de\n[other]\nkey=1\n";
        assert_eq!(
            with_value(text, LANGUAGE, Some("fr")),
            "[interface]\nlanguage=fr\n[other]\nkey=1\n"
        );
        assert_eq!(
            with_value(text, CORE_VS_OPTION_OFF, Some("true")),
            "[interface]\nlanguage=de\ncore-vs-option-off=true\n[other]\nkey=1\n"
        );
        assert_eq!(
            with_value(text, LANGUAGE, None),
            "[interface]\n[other]\nkey=1\n"
        );
        assert_eq!(
            with_value("", LANGUAGE, Some("it")),
            "[interface]\nlanguage=it\n"
        );
        assert_eq!(
            value("[interface]\nlanguage = es\n", LANGUAGE).as_deref(),
            Some("es")
        );
    }
}
