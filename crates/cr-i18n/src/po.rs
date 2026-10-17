//! Reads and writes the subset of the gettext PO format the catalogs use:
//! `msgctxt`, `msgid`, `msgid_plural`, `msgstr` and `msgstr[n]`, with
//! comments and the `fuzzy` flag.

/// One entry of a catalog.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Entry {
    /// Translator and reference comments, written back as they were.
    pub comments: Vec<String>,
    pub fuzzy: bool,
    pub context: Option<String>,
    pub msgid: String,
    pub plural: Option<String>,
    /// One form, or one per plural form.
    pub msgstr: Vec<String>,
}

impl Entry {
    /// The lookup key: the msgid, after the context when there is one.
    pub fn key(&self) -> String {
        match &self.context {
            Some(c) => context_key(c, &self.msgid),
            None => self.msgid.clone(),
        }
    }
}

/// The key of a text with a context, as gettext forms it.
pub fn context_key(context: &str, msgid: &str) -> String {
    format!("{context}\u{4}{msgid}")
}

#[derive(Clone, Copy)]
enum Field {
    None,
    Context,
    Id,
    Plural,
    Str(usize),
}

/// The entries of a catalog, without the header (the entry with an empty
/// msgid). Lines it does not understand are skipped.
pub fn parse(text: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut entry = Entry::default();
    let mut field = Field::None;
    let mut started = false;
    let finish = |entry: &mut Entry, started: &mut bool, entries: &mut Vec<Entry>| {
        if *started && !entry.msgid.is_empty() {
            entries.push(std::mem::take(entry));
        }
        *entry = Entry::default();
        *started = false;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            finish(&mut entry, &mut started, &mut entries);
            field = Field::None;
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            if started {
                finish(&mut entry, &mut started, &mut entries);
            }
            if let Some(flags) = comment.strip_prefix(", ") {
                entry.fuzzy |= flags.split(", ").any(|f| f.trim() == "fuzzy");
            }
            entry.comments.push(line.to_string());
            continue;
        }
        let (keyword, rest) = match line.find(' ') {
            Some(i) => (&line[..i], line[i + 1..].trim()),
            None => (line, ""),
        };
        let value = if keyword.starts_with('"') {
            unquote(line)
        } else {
            match keyword {
                "msgctxt" => {
                    if started {
                        finish(&mut entry, &mut started, &mut entries);
                    }
                    field = Field::Context;
                }
                "msgid" => {
                    if started && entry.context.is_none() {
                        finish(&mut entry, &mut started, &mut entries);
                    }
                    field = Field::Id;
                }
                "msgid_plural" => field = Field::Plural,
                "msgstr" => field = Field::Str(0),
                k if k.starts_with("msgstr[") => {
                    let n = k[7..].trim_end_matches(']').parse().unwrap_or(0);
                    field = Field::Str(n);
                }
                _ => continue,
            }
            started = true;
            unquote(rest)
        };
        match field {
            Field::None => {}
            Field::Context => entry.context.get_or_insert_default().push_str(&value),
            Field::Id => entry.msgid.push_str(&value),
            Field::Plural => entry.plural.get_or_insert_default().push_str(&value),
            Field::Str(n) => {
                if entry.msgstr.len() <= n {
                    entry.msgstr.resize(n + 1, String::new());
                }
                entry.msgstr[n].push_str(&value);
            }
        }
    }
    finish(&mut entry, &mut started, &mut entries);
    entries
}

/// The text of a quoted PO string.
fn unquote(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix('"').unwrap_or(s);
    let s = s.strip_suffix('"').unwrap_or(s);
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// A PO string, split after newlines the way msgmerge writes long texts.
pub fn quote(keyword: &str, s: &str) -> String {
    let escape = |s: &str| {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
    };
    let lines: Vec<&str> = s.split_inclusive('\n').collect();
    if lines.len() <= 1 {
        return format!("{keyword} \"{}\"\n", escape(s));
    }
    let mut out = format!("{keyword} \"\"\n");
    for line in lines {
        out.push_str(&format!("\"{}\"\n", escape(line)));
    }
    out
}

/// Writes one entry, without its comments.
pub fn write_entry(e: &Entry) -> String {
    let mut out = String::new();
    if let Some(c) = &e.context {
        out.push_str(&quote("msgctxt", c));
    }
    out.push_str(&quote("msgid", &e.msgid));
    match &e.plural {
        Some(p) => {
            out.push_str(&quote("msgid_plural", p));
            for (i, s) in e.msgstr.iter().enumerate() {
                out.push_str(&quote(&format!("msgstr[{i}]"), s));
            }
        }
        None => out.push_str(&quote("msgstr", e.msgstr.first().map_or("", |s| s))),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let text = "msgid \"\"\nmsgstr \"Content-Type: text/plain; charset=UTF-8\\n\"\n\n\
            #: a.rs:1\nmsgid \"Save \\\"{name}\\\"\"\nmsgstr \"Salvar \\\"{name}\\\"\"\n\n\
            #, fuzzy\nmsgctxt \"verb\"\nmsgid \"Open\"\nmsgstr \"Abrir\"\n\n\
            msgid \"{n} entry\"\nmsgid_plural \"{n} entries\"\nmsgstr[0] \"{n} entrada\"\nmsgstr[1] \"{n} entradas\"\n\n\
            msgid \"\"\n\"two\\n\"\n\"lines\"\nmsgstr \"\"\n\"duas\\n\"\n\"linhas\"\n";
        let entries = parse(text);
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].msgid, "Save \"{name}\"");
        assert_eq!(entries[0].msgstr, ["Salvar \"{name}\""]);
        assert_eq!(entries[0].comments, ["#: a.rs:1"]);
        assert!(entries[1].fuzzy);
        assert_eq!(entries[1].key(), "verb\u{4}Open");
        assert_eq!(entries[2].msgstr, ["{n} entrada", "{n} entradas"]);
        assert_eq!(entries[3].msgid, "two\nlines");
        for e in &entries {
            assert_eq!(parse(&write_entry(e))[0].msgstr, e.msgstr);
        }
    }
}
