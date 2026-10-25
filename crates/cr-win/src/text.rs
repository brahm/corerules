//! Texts as the Windows interface shows them.

/// A text of the catalogs without its GTK mnemonic: `_Next` is "Next".
/// WinUI marks access keys on the control, not in its label, and a
/// doubled underscore stands for one.
pub fn plain(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '_' {
            if chars.peek() == Some(&'_') {
                out.push('_');
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Whether a list row whose title and subtitle are these matches what
/// was typed in the search box, case ignored, as in the GTK app: the
/// title or the subtitle holds it.
pub fn matches(search: &str, title: &str, subtitle: &str) -> bool {
    let needle = search.to_lowercase();
    needle.is_empty()
        || title.to_lowercase().contains(&needle)
        || subtitle.to_lowercase().contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mnemonics_are_dropped() {
        assert_eq!(plain("_Next"), "Next");
        assert_eq!(plain("Extract Data from _CD…"), "Extract Data from CD…");
        assert_eq!(plain("a__b"), "a_b");
        assert_eq!(plain("Print"), "Print");
    }

    #[test]
    fn the_title_or_the_subtitle_holds_the_search() {
        assert!(matches("", "Lirael", "Elf Ranger"));
        assert!(matches("elf", "Lirael", "Elf Ranger"));
        assert!(matches("LIR", "Lirael", "Elf Ranger"));
        assert!(!matches("lirael elf", "Lirael", "Elf Ranger"));
    }
}
