//! Every text wrapped for translation has a translation in every language,
//! with the same placeholders and mnemonic.

use cr_i18n::Language;
use cr_i18n::extract::{self, mnemonics, placeholders};
use cr_i18n::po;

#[test]
fn catalogs_are_complete() {
    let messages = extract::messages();
    assert!(messages.len() > 100, "found only {} texts", messages.len());
    let mut problems = Vec::new();
    for lang in Language::ALL
        .into_iter()
        .filter(|l| *l != Language::English)
    {
        let file = format!("{}.po", lang.code());
        let text = std::fs::read_to_string(extract::po_dir().join(&file)).unwrap();
        let entries = po::parse(&text);
        for m in &messages {
            let Some(e) = entries.iter().find(|e| e.key() == m.key()) else {
                problems.push(format!(
                    "{file}: missing {:?} ({})",
                    m.msgid, m.references[0]
                ));
                continue;
            };
            if e.fuzzy || e.msgstr.is_empty() || e.msgstr.iter().any(String::is_empty) {
                problems.push(format!("{file}: untranslated {:?}", m.msgid));
                continue;
            }
            if m.plural.is_some() != e.plural.is_some()
                || e.msgstr.len() != 1 + usize::from(m.plural.is_some())
            {
                problems.push(format!("{file}: plural forms differ for {:?}", m.msgid));
            }
            for s in &e.msgstr {
                let mut want = placeholders(&m.msgid);
                if m.plural.is_some() {
                    want.retain(|p| p != "n");
                }
                let mut got = placeholders(s);
                got.retain(|p| m.plural.is_none() || p != "n");
                got.dedup();
                want.dedup();
                if want != got {
                    problems.push(format!("{file}: placeholders of {:?} in {s:?}", m.msgid));
                }
                if mnemonics(&m.msgid) == 1 && mnemonics(s) != 1 {
                    problems.push(format!("{file}: mnemonic of {:?} in {s:?}", m.msgid));
                }
                if m.msgid.ends_with('…') != s.ends_with('…') {
                    problems.push(format!("{file}: ellipsis of {:?} in {s:?}", m.msgid));
                }
            }
        }
        for e in &entries {
            if !messages.iter().any(|m| m.key() == e.key()) {
                problems.push(format!("{file}: unused {:?}", e.msgid));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} problems; run `cargo run -p cr-i18n --example update-po` and translate:\n{}",
        problems.len(),
        problems.join("\n")
    );
}
