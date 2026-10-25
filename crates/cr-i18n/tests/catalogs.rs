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

/// Texts shown side by side as choices, whose translations must differ:
/// Windows' SelectorBar tells its tabs apart by their text alone.
#[test]
fn texts_shown_side_by_side_stay_apart() {
    // The character editor's tabs.
    let groups: &[&[&str]] = &[&[
        "Details",
        "Abilities",
        "Combat",
        "Proficiencies",
        "Skills",
        "Spells",
        "Equipment",
    ]];
    let mut problems = Vec::new();
    for lang in Language::ALL
        .into_iter()
        .filter(|l| *l != Language::English)
    {
        let file = format!("{}.po", lang.code());
        let text = std::fs::read_to_string(extract::po_dir().join(&file)).unwrap();
        let entries = po::parse(&text);
        let translated = |msgid: &str| {
            entries
                .iter()
                .find(|e| e.key() == msgid)
                .map_or(msgid.to_owned(), |e| e.msgstr[0].clone())
        };
        for group in groups {
            for (i, a) in group.iter().enumerate() {
                for b in &group[i + 1..] {
                    if translated(a) == translated(b) {
                        problems.push(format!(
                            "{file}: {a:?} and {b:?} are both {:?}",
                            translated(a)
                        ));
                    }
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The metainfo tells software centers which languages the app speaks;
/// the catalogs are built into the program, so nothing generates the list.
#[test]
fn metainfo_lists_every_language() {
    let path = extract::po_dir().join("../../../data/io.github.brahm.CoreRules.metainfo.xml");
    let metainfo = std::fs::read_to_string(path).unwrap();
    let listed: Vec<&str> = metainfo
        .split("<lang")
        .skip(1)
        .filter_map(|s| s.split_once('>')?.1.split_once("</lang>"))
        .map(|(code, _)| code)
        .collect();
    let codes: Vec<&str> = Language::ALL.iter().map(|l| l.code()).collect();
    assert_eq!(listed, codes);
}
