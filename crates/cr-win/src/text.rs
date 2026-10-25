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
}
