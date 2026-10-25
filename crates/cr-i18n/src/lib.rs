//! Translations of the interface. The texts are gettext catalogs
//! (`po/*.po`) built into the program, so every package carries them and
//! the language can change without any installed files.
//!
//! Interface texts are written in American English and wrapped where they
//! are written: `tr("Save")`, `tr!("Deleted {name}", name)` and
//! `trn!("{n} entry", "{n} entries", count)`. Until [`set_language`] runs,
//! or for English, they come back unchanged, so the command line and the
//! exports, which never set a language, stay in English. Game data (spell,
//! monster and item names from the original files) is never translated.
//!
//! `cargo test -p cr-i18n` fails when a wrapped text lacks a translation;
//! `cargo run -p cr-i18n --example update-po` adds the missing entries to
//! every catalog and drops the ones no longer used.

#[doc(hidden)]
pub mod extract;
pub mod po;

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::OnceLock;

/// The languages of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    PortugueseBrazil,
    Spanish,
    Italian,
    French,
    German,
}

impl Language {
    pub const ALL: [Language; 6] = [
        Language::English,
        Language::PortugueseBrazil,
        Language::Spanish,
        Language::Italian,
        Language::French,
        Language::German,
    ];

    /// The locale code, as in `pt_BR.UTF-8`, and the catalog's file name.
    pub fn code(self) -> &'static str {
        match self {
            Language::English => "en_US",
            Language::PortugueseBrazil => "pt_BR",
            Language::Spanish => "es",
            Language::Italian => "it",
            Language::French => "fr",
            Language::German => "de",
        }
    }

    /// The language's name in itself, for the language choice.
    pub fn native_name(self) -> &'static str {
        match self {
            Language::English => "English (US)",
            Language::PortugueseBrazil => "Português (Brasil)",
            Language::Spanish => "Español",
            Language::Italian => "Italiano",
            Language::French => "Français",
            Language::German => "Deutsch",
        }
    }

    pub fn from_code(code: &str) -> Option<Language> {
        Language::ALL.into_iter().find(|l| l.code() == code)
    }

    /// The language a locale name such as `pt_BR.UTF-8`, `fr_CA` or `de`
    /// asks for, if it is one of ours. Portuguese from anywhere gets the
    /// Brazilian translation.
    pub fn from_locale(locale: &str) -> Option<Language> {
        let base = locale.split(['.', '@']).next().unwrap_or("");
        let lang = base.split(['_', '-']).next().unwrap_or("");
        match lang {
            "en" => Some(Language::English),
            "pt" => Some(Language::PortugueseBrazil),
            "es" => Some(Language::Spanish),
            "it" => Some(Language::Italian),
            "fr" => Some(Language::French),
            "de" => Some(Language::German),
            _ => None,
        }
    }

    fn catalog_source(self) -> &'static str {
        match self {
            Language::English => "",
            Language::PortugueseBrazil => include_str!("../po/pt_BR.po"),
            Language::Spanish => include_str!("../po/es.po"),
            Language::Italian => include_str!("../po/it.po"),
            Language::French => include_str!("../po/fr.po"),
            Language::German => include_str!("../po/de.po"),
        }
    }

    /// Whether `n` takes the plural form. French counts zero as singular;
    /// Brazilian Portuguese, as its grammar has it, does not ("0 pontos").
    fn plural(self, n: u64) -> bool {
        match self {
            Language::French => n > 1,
            _ => n != 1,
        }
    }

    /// What separates the thousands of a number.
    fn thousands_separator(self) -> char {
        match self {
            Language::English => ',',
            // A narrow no-break space, as French typography has it.
            Language::French => '\u{202f}',
            _ => '.',
        }
    }
}

/// `n` with its thousands separated as the interface language does it:
/// "12,500", "12.500".
pub fn thousands(n: u64) -> String {
    let separator = language().thousands_separator();
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(c);
    }
    out
}

/// The language the system asks for, the way gettext reads it: the first
/// of `LANGUAGE` (a colon-separated preference list), `LC_ALL`,
/// `LC_MESSAGES` and `LANG` that is set. On Windows, where these are
/// usually not set, the first of the display languages chosen in Settings
/// that is one of ours. English when that language is not one of ours.
pub fn system_language() -> Language {
    let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
    let locale = var("LC_ALL")
        .or_else(|| var("LC_MESSAGES"))
        .or_else(|| var("LANG"))
        .unwrap_or_default();
    if locale.is_empty()
        && let Some(language) = display_languages()
            .iter()
            .find_map(|l| Language::from_locale(l))
    {
        return language;
    }
    // gettext ignores LANGUAGE under the C locale.
    let preferences = match locale.as_str() {
        "" | "C" | "POSIX" => None,
        _ => var("LANGUAGE"),
    };
    preferences
        .iter()
        .flat_map(|list| list.split(':'))
        .chain(std::iter::once(locale.as_str()))
        .find_map(Language::from_locale)
        .unwrap_or(Language::English)
}

/// The display languages chosen in Windows Settings, in order of
/// preference, as names such as `pt-BR`.
#[cfg(windows)]
fn display_languages() -> Vec<String> {
    use windows_sys::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    let (mut count, mut len) = (0, 0);
    // SAFETY: the first call only reports the buffer length; the second
    // fills a buffer of that length.
    unsafe {
        if GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            std::ptr::null_mut(),
            &mut len,
        ) == 0
        {
            return Vec::new();
        }
        let mut buffer = vec![0u16; len as usize];
        if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, buffer.as_mut_ptr(), &mut len)
            == 0
        {
            return Vec::new();
        }
        // The names, each ended by a NUL, and one more NUL at the end.
        buffer
            .split(|&c| c == 0)
            .filter(|name| !name.is_empty())
            .map(String::from_utf16_lossy)
            .collect()
    }
}

#[cfg(not(windows))]
fn display_languages() -> Vec<String> {
    Vec::new()
}

static LANGUAGE: OnceLock<Language> = OnceLock::new();
static CATALOG: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();

/// Sets the interface language for the rest of the run. Only the first
/// call counts, and it must come before the first translated text.
pub fn set_language(language: Language) {
    let _ = LANGUAGE.set(language);
}

/// The interface language: English unless [`set_language`] chose another.
pub fn language() -> Language {
    *LANGUAGE.get_or_init(|| Language::English)
}

fn catalog() -> &'static HashMap<String, Vec<String>> {
    CATALOG.get_or_init(|| {
        po::parse(language().catalog_source())
            .into_iter()
            .filter(|e| !e.fuzzy && e.msgstr.iter().all(|s| !s.is_empty()))
            .map(|e| (e.key(), e.msgstr))
            .collect()
    })
}

/// The translation of an interface text, or the text itself.
pub fn tr(msgid: &str) -> &str {
    match catalog().get(msgid) {
        Some(forms) => &forms[0],
        None => msgid,
    }
}

/// The English texts whose translation is `text`, for matching what the
/// interface shows against English names (the original program's).
pub fn sources(text: &str) -> Vec<&'static str> {
    static SOURCES: OnceLock<HashMap<&'static str, Vec<&'static str>>> = OnceLock::new();
    SOURCES
        .get_or_init(|| {
            let mut sources: HashMap<&str, Vec<&str>> = HashMap::new();
            for (key, forms) in catalog() {
                // A text with a context: the msgid after it.
                let msgid = key.rsplit('\u{4}').next().unwrap_or(key);
                sources.entry(forms[0].as_str()).or_default().push(msgid);
            }
            sources
        })
        .get(text)
        .cloned()
        .unwrap_or_default()
}

/// The translation of a text that reads differently by context, as in
/// gettext's `pgettext`.
pub fn trc<'a>(context: &str, msgid: &'a str) -> &'a str {
    match catalog().get(&po::context_key(context, msgid)) {
        Some(forms) => &forms[0],
        None => msgid,
    }
}

/// The singular or plural translation for `n`.
pub fn trn<'a>(singular: &'a str, plural: &'a str, n: u64) -> &'a str {
    let lang = language();
    match catalog().get(singular) {
        Some(forms) if forms.len() > 1 => &forms[usize::from(lang.plural(n))],
        _ if n == 1 => singular,
        _ => plural,
    }
}

/// Fills the `{name}` placeholders of a translated text. `{{` and `}}`
/// stand for braces.
pub fn fill(template: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(i) = rest.find(['{', '}']) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        if tail.starts_with("{{") || tail.starts_with("}}") {
            out.push_str(&tail[..1]);
            rest = &tail[2..];
            continue;
        }
        if let Some(end) = tail.find('}').filter(|_| tail.starts_with('{')) {
            let name = &tail[1..end];
            if let Some((_, value)) = args.iter().find(|(n, _)| *n == name) {
                out.push_str(&value.to_string());
                rest = &tail[end + 1..];
                continue;
            }
        }
        out.push_str(&tail[..1]);
        rest = &tail[1..];
    }
    out.push_str(rest);
    out
}

#[doc(hidden)]
#[macro_export]
macro_rules! __arg {
    ($name:ident) => {
        $name
    };
    ($name:ident = $value:expr) => {
        $value
    };
}

/// Translates a text and fills its placeholders, which are named:
/// `tr!("Saved {name} in {folder}", name, folder = dir.display())`. With
/// no arguments it is [`tr`].
#[macro_export]
macro_rules! tr {
    ($msgid:literal) => {
        $crate::tr($msgid)
    };
    ($msgid:literal $(, $name:ident $(= $value:expr)?)+ $(,)?) => {
        $crate::fill(
            $crate::tr($msgid),
            &[$((stringify!($name), &$crate::__arg!($name $(= $value)?) as &dyn ::std::fmt::Display)),+],
        )
    };
}

/// Like [`tr!`], for a text that reads differently by context.
#[macro_export]
macro_rules! trc {
    ($context:literal, $msgid:literal) => {
        $crate::trc($context, $msgid)
    };
    ($context:literal, $msgid:literal $(, $name:ident $(= $value:expr)?)+ $(,)?) => {
        $crate::fill(
            $crate::trc($context, $msgid),
            &[$((stringify!($name), &$crate::__arg!($name $(= $value)?) as &dyn ::std::fmt::Display)),+],
        )
    };
}

/// Picks the singular or plural form for a count and fills its
/// placeholders. The count is always available as `{n}`:
/// `trn!("{n} entry", "{n} entries", rows.len())`.
#[macro_export]
macro_rules! trn {
    ($singular:literal, $plural:literal, $n:expr $(, $name:ident $(= $value:expr)?)* $(,)?) => {{
        let n = $n;
        $crate::fill(
            $crate::trn($singular, $plural, n as u64),
            &[("n", &n as &dyn ::std::fmt::Display) $(, (stringify!($name), &$crate::__arg!($name $(= $value)?) as &dyn ::std::fmt::Display))*],
        )
    }};
}

/// Marks a text for translation where it is written, in a constant for
/// instance, and leaves it as is; [`tr`] translates it where it is shown.
#[macro_export]
macro_rules! tr_noop {
    ($msgid:literal) => {
        $msgid
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_placeholders() {
        let name = "Borin";
        assert_eq!(
            fill(
                "Saved {name} in {dir}",
                &[("name", &name), ("dir", &"Chars")]
            ),
            "Saved Borin in Chars"
        );
        assert_eq!(fill("{{literal}} {x}", &[("x", &3)]), "{literal} 3");
        assert_eq!(fill("{unknown} }", &[]), "{unknown} }");
        assert_eq!(tr!("Deleted {name}", name), "Deleted Borin");
        assert_eq!(trn!("{n} entry", "{n} entries", 2usize), "2 entries");
    }

    #[test]
    fn reads_locales() {
        assert_eq!(
            Language::from_locale("pt_BR.UTF-8"),
            Some(Language::PortugueseBrazil)
        );
        assert_eq!(
            Language::from_locale("pt_PT"),
            Some(Language::PortugueseBrazil)
        );
        assert_eq!(Language::from_locale("de_AT@euro"), Some(Language::German));
        assert_eq!(Language::from_locale("fr-CA"), Some(Language::French));
        assert_eq!(Language::from_locale("ja_JP.UTF-8"), None);
        assert_eq!(Language::from_locale("C"), None);
    }

    #[test]
    fn plural_rules() {
        assert!(!Language::French.plural(0));
        assert!(Language::English.plural(0));
        assert!(!Language::German.plural(1));
        assert!(Language::PortugueseBrazil.plural(2));
        assert!(Language::PortugueseBrazil.plural(0));
        assert!(!Language::PortugueseBrazil.plural(1));
    }
}
