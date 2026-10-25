//! Brings the catalogs in line with the sources: writes `po/corerules.pot`
//! and, in every `po/*.po`, adds the new texts untranslated and drops the
//! ones no longer used. Translations already there are kept.

use cr_i18n::Language;
use cr_i18n::extract::{self, Message};
use cr_i18n::po::{self, Entry};

fn header(language: &str) -> String {
    format!(
        "# Translations of the Core Rules interface.\n\
         # This file is distributed under the same license as the corerules package.\n\
         msgid \"\"\n\
         msgstr \"\"\n\
         \"Project-Id-Version: corerules\\n\"\n\
         \"Language: {language}\\n\"\n\
         \"MIME-Version: 1.0\\n\"\n\
         \"Content-Type: text/plain; charset=UTF-8\\n\"\n\
         \"Content-Transfer-Encoding: 8bit\\n\"\n\
         \"Plural-Forms: nplurals=2; plural={};\\n\"\n",
        match language {
            "fr" => "(n > 1)",
            _ => "(n != 1)",
        }
    )
}

fn write(path: &std::path::Path, language: &str, messages: &[Message], old: &[Entry]) {
    let mut out = header(language);
    for m in messages {
        let kept = old.iter().find(|e| e.key() == m.key());
        let forms = if m.plural.is_some() { 2 } else { 1 };
        let mut entry = Entry {
            context: m.context.clone(),
            msgid: m.msgid.clone(),
            plural: m.plural.clone(),
            msgstr: vec![String::new(); forms],
            ..Entry::default()
        };
        out.push('\n');
        if let Some(kept) = kept {
            for c in kept.comments.iter().filter(|c| c.starts_with("# ")) {
                out.push_str(c);
                out.push('\n');
            }
            entry.fuzzy = kept.fuzzy;
            if kept.msgstr.len() == forms {
                entry.msgstr = kept.msgstr.clone();
            }
        }
        for r in &m.references {
            out.push_str(&format!("#: {r}\n"));
        }
        if entry.fuzzy {
            out.push_str("#, fuzzy\n");
        }
        out.push_str(&po::write_entry(&entry));
    }
    std::fs::write(path, out).expect("write catalog");
}

fn main() {
    let messages = extract::messages();
    let dir = extract::po_dir();
    write(&dir.join("corerules.pot"), "", &messages, &[]);
    for lang in Language::ALL
        .into_iter()
        .filter(|l| *l != Language::English)
    {
        let path = dir.join(format!("{}.po", lang.code()));
        let old = std::fs::read_to_string(&path)
            .map(|t| po::parse(&t))
            .unwrap_or_default();
        write(&path, lang.code(), &messages, &old);
        let missing = po::parse(&std::fs::read_to_string(&path).unwrap())
            .iter()
            .filter(|e| e.fuzzy || e.msgstr.iter().any(String::is_empty))
            .count();
        println!(
            "{}: {} texts, {missing} to translate",
            path.display(),
            messages.len()
        );
    }
}
