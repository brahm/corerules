//! Finds the texts wrapped for translation in the workspace's sources, the
//! job xgettext does for languages it knows.

use std::path::{Path, PathBuf};

/// A text to translate, with where it is used.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub context: Option<String>,
    pub msgid: String,
    pub plural: Option<String>,
    /// `crates/cr-gtk/src/main.rs:807` for each use.
    pub references: Vec<String>,
}

impl Message {
    pub fn key(&self) -> String {
        match &self.context {
            Some(c) => crate::po::context_key(c, &self.msgid),
            None => self.msgid.clone(),
        }
    }
}

/// The workspace root, two levels above this crate.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The directory of the catalogs.
pub fn po_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("po")
}

/// Every wrapped text of the crates' `src` directories, in order of first
/// use.
pub fn messages() -> Vec<Message> {
    let root = workspace_root();
    let mut files = Vec::new();
    let Ok(crates) = std::fs::read_dir(root.join("crates")) else {
        return Vec::new();
    };
    let mut crates: Vec<PathBuf> = crates.flatten().map(|e| e.path()).collect();
    crates.sort();
    for dir in crates {
        if dir.ends_with("cr-i18n") {
            continue;
        }
        rust_files(&dir.join("src"), &mut files);
    }
    let mut out: Vec<Message> = Vec::new();
    for file in files {
        let Ok(src) = std::fs::read_to_string(&file) else {
            continue;
        };
        let rel = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        for (line, msg) in scan(&src) {
            let reference = format!("{rel}:{line}");
            match out.iter_mut().find(|m| m.key() == msg.key()) {
                Some(m) => {
                    m.references.push(reference);
                    if m.plural.is_none() {
                        m.plural = msg.plural;
                    }
                }
                None => out.push(Message {
                    references: vec![reference],
                    ..msg
                }),
            }
        }
    }
    out
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// The wrapped texts of one source file, with their line numbers.
pub fn scan(src: &str) -> Vec<(usize, Message)> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let line_at = |i: usize| src[..i].bytes().filter(|&c| c == b'\n').count() + 1;
    while i < b.len() {
        let c = b[i];
        if b[i..].starts_with(b"//") {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i..].starts_with(b"/*") {
            i = src[i + 2..].find("*/").map_or(b.len(), |j| i + 2 + j + 2);
        } else if c == b'"' {
            i = string_literal(src, i).map_or(b.len(), |(_, end)| end);
        } else if c == b'\'' {
            // A char literal, or else a lifetime.
            if b.get(i + 1) == Some(&b'\\') {
                i = src[i + 2..].find('\'').map_or(b.len(), |j| i + 2 + j + 1);
            } else if let Some(ch) = src[i + 1..].chars().next() {
                let after = i + 1 + ch.len_utf8();
                i = if b.get(after) == Some(&b'\'') {
                    after + 1
                } else {
                    i + 1
                };
            } else {
                i += 1;
            }
        } else if is_ident(c) && (i == 0 || !is_ident(b[i - 1])) {
            let start = i;
            while i < b.len() && is_ident(b[i]) {
                i += 1;
            }
            let word = &src[start..i];
            if word == "r" && matches!(b.get(i), Some(b'"' | b'#')) {
                i = raw_literal(src, i).map_or(i, |(_, end)| end);
                continue;
            }
            let wanted = match word {
                "tr" | "tr_noop" => 1,
                "trc" | "trn" => 2,
                _ => continue,
            };
            let mut j = i;
            if b.get(j) == Some(&b'!') {
                j += 1;
            }
            if b.get(j) != Some(&b'(') {
                continue;
            }
            j += 1;
            let mut args = Vec::new();
            while args.len() < wanted {
                while j < b.len() && b[j].is_ascii_whitespace() {
                    j += 1;
                }
                let lit = match b.get(j) {
                    Some(b'"') => string_literal(src, j),
                    Some(b'r') => raw_literal(src, j + 1),
                    _ => None,
                };
                let Some((text, end)) = lit else { break };
                args.push(text);
                j = end;
                while j < b.len() && b[j].is_ascii_whitespace() {
                    j += 1;
                }
                if b.get(j) == Some(&b',') {
                    j += 1;
                }
            }
            if args.len() == wanted {
                let mut args = args.into_iter();
                let first = args.next().unwrap_or_default();
                let second = args.next();
                let msg = match word {
                    "trc" => Message {
                        context: Some(first),
                        msgid: second.unwrap_or_default(),
                        plural: None,
                        references: Vec::new(),
                    },
                    _ => Message {
                        context: None,
                        msgid: first,
                        plural: second,
                        references: Vec::new(),
                    },
                };
                out.push((line_at(start), msg));
                i = j;
            }
        } else {
            i += src[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
    out
}

/// The value of the string literal starting at `start` (its quote) and the
/// index after it.
fn string_literal(src: &str, start: usize) -> Option<(String, usize)> {
    let mut out = String::new();
    let mut chars = src[start + 1..].char_indices().peekable();
    while let Some((k, c)) = chars.next() {
        match c {
            '"' => return Some((out, start + 1 + k + 1)),
            '\\' => match chars.next()?.1 {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                '0' => out.push('\0'),
                'x' => {
                    let hex: String = (0..2).filter_map(|_| chars.next().map(|c| c.1)).collect();
                    out.push(u8::from_str_radix(&hex, 16).ok()? as char);
                }
                'u' => {
                    let mut hex = String::new();
                    for (_, c) in chars.by_ref() {
                        match c {
                            '{' => {}
                            '}' => break,
                            c => hex.push(c),
                        }
                    }
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                '\n' => {
                    while chars.peek().is_some_and(|c| c.1.is_whitespace()) {
                        chars.next();
                    }
                }
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

/// The value of a raw string literal whose `#`s or quote start at `start`
/// (just after the `r`).
fn raw_literal(src: &str, start: usize) -> Option<(String, usize)> {
    let hashes = src[start..].bytes().take_while(|&c| c == b'#').count();
    let open = start + hashes;
    if src.as_bytes().get(open) != Some(&b'"') {
        return None;
    }
    let close = format!("\"{}", "#".repeat(hashes));
    let end = src[open + 1..].find(&close)? + open + 1;
    Some((src[open + 1..end].to_string(), end + close.len()))
}

/// The placeholder names of a text, sorted, for comparing a translation
/// with its original.
pub fn placeholders(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text.replace("{{", "").replace("}}", "");
    while let Some(i) = rest.find('{') {
        let Some(j) = rest[i..].find('}') else { break };
        names.push(rest[i + 1..i + j].to_string());
        rest = rest[i + j + 1..].to_string();
    }
    names.sort();
    names
}

/// How many mnemonic underscores (`_N`ew) a text has.
pub fn mnemonics(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    chars
        .windows(2)
        .filter(|w| w[0] == '_' && w[1].is_alphanumeric())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_wrapped_texts() {
        let src = r#####"
            // tr("in a comment")
            let a = tr("Save");
            let q = '"'; let l: &'static str = "x";
            let b = tr!("Deleted {name}", name);
            let c = cr_i18n::trn!(
                "{n} entry",
                "{n} entries", rows.len());
            let d = trc("verb", "Open");
            const E: [&str; 1] = [tr_noop!("Gold \"coins\"\u{2026}")];
            let f = tr(dynamic);
            let g = gtk::Label::new(Some(tr("Line one\n\
                line two")));
            let h = tr(r#"raw "text""#);
        "#####;
        let found: Vec<_> = scan(src)
            .into_iter()
            .map(|(l, m)| (l, m.msgid, m.plural))
            .collect();
        assert_eq!(
            found,
            [
                (3, "Save".into(), None),
                (5, "Deleted {name}".into(), None),
                (6, "{n} entry".into(), Some("{n} entries".into())),
                (9, "Open".into(), None),
                (10, "Gold \"coins\"…".into(), None),
                (12, "Line one\nline two".into(), None),
                (14, "raw \"text\"".into(), None),
            ]
        );
    }

    #[test]
    fn compares_texts() {
        assert_eq!(placeholders("{b} and {a} {{x}}"), ["a", "b"]);
        assert_eq!(mnemonics("_New Character…"), 1);
        assert_eq!(mnemonics("PartsU.dat _"), 0);
    }
}
