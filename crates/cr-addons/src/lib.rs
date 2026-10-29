//! The add-ons and how they are installed, without an interface: the
//! consolidated package, its pictures and texts, built in, and the steps
//! that copy them into a data folder. The GNOME window (`main.rs`), the
//! Windows and the Mac installers each draw their own around these.

pub mod extras;
pub mod install;

use cr_i18n::Language;

/// 36716 → "36,716" in English, "36.716" in Portuguese.
pub fn thousands(n: usize) -> String {
    group_digits(n, cr_i18n::language())
}

fn group_digits(n: usize, language: Language) -> String {
    let separator = match language {
        Language::English => ',',
        Language::French => '\u{202f}',
        _ => '.',
    };
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

#[cfg(test)]
mod tests {
    use super::group_digits;
    use cr_i18n::Language;

    #[test]
    pub fn thousands_are_grouped_as_each_language_writes_them() {
        assert_eq!(group_digits(36_716, Language::PortugueseBrazil), "36.716");
        assert_eq!(group_digits(36_716, Language::English), "36,716");
        assert_eq!(group_digits(36_716, Language::French), "36\u{202f}716");
        assert_eq!(group_digits(105, Language::German), "105");
        assert_eq!(group_digits(1_000_000, Language::Spanish), "1.000.000");
    }
}
