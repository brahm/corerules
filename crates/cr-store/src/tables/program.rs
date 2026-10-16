//! The text the originals print from their own code (column headings,
//! the rows and footnotes of the tables they work out themselves, the
//! options of the parameter dialogs). It is read from the installation's
//! program at run time, at the addresses of [`super::strings`], and only
//! from the builds those addresses were taken from.

use std::fmt;
use std::sync::Arc;

use crate::Installation;
use crate::sheet::Flavor;

/// A string of the programs: its virtual address in Tsrp3.exe and in
/// Evermore.exe (0: that program has none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Txt(pub u32, pub u32);

/// The builds the addresses belong to: program, size and FNV-1a hash.
const BUILDS: [(Flavor, &str, usize, u64); 2] = [
    (
        Flavor::Expansion,
        "Program/Tsrp3.exe",
        5_033_984,
        0xcf09_6cd3_eb7d_22b1,
    ),
    (
        Flavor::Core20,
        "Program/Evermore.exe",
        3_423_232,
        0x822f_6926_c2e4_f36e,
    ),
];

/// The program of an installation, loaded once.
#[derive(Clone)]
pub struct Program {
    flavor: Flavor,
    image: Arc<Vec<u8>>,
    /// (virtual address, raw offset, raw size) of each section.
    sections: Vec<(u32, u32, u32)>,
    /// Tests: made-up text by Tsrp3 address instead of an image.
    #[cfg(test)]
    fake: Option<Arc<std::collections::HashMap<u32, String>>>,
}

impl fmt::Debug for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Program")
            .field("flavor", &self.flavor)
            .field("bytes", &self.image.len())
            .finish()
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &b in bytes {
        h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
    }
    h
}

impl Program {
    /// The installation's program, or why it cannot be used.
    pub fn load(install: &Installation, flavor: Flavor) -> Result<Program, String> {
        let (_, relative, size, hash) = BUILDS
            .iter()
            .copied()
            .find(|b| b.0 == flavor)
            .expect("both flavors have a build");
        if install.path(relative).is_none() {
            return Err(format!("{relative} is missing"));
        }
        let image = install
            .read(relative)
            .map_err(|e| format!("{relative} cannot be read ({e})"))?;
        if image.len() != size || fnv1a(&image) != hash {
            return Err(format!("{relative} is not the build this program knows"));
        }
        Program::from_image(flavor, image)
            .ok_or_else(|| format!("{relative} is not a Windows program"))
    }

    /// A program from its bytes, without the build check (tests).
    pub fn from_image(flavor: Flavor, image: Vec<u8>) -> Option<Program> {
        let sections = sections(&image)?;
        Some(Program {
            flavor,
            image: Arc::new(image),
            sections,
            #[cfg(test)]
            fake: None,
        })
    }

    /// Tests: every address of [`super::strings`] holds a made-up text
    /// ("<address>"), but for `text`.
    #[cfg(test)]
    pub fn fake(text: &[(Txt, &str)]) -> Program {
        let mut map: std::collections::HashMap<u32, String> = super::strings::all()
            .into_iter()
            .map(|t| (t.0, format!("<{:x}>", t.0)))
            .collect();
        for (t, s) in text {
            map.insert(t.0, (*s).to_owned());
        }
        Program {
            flavor: Flavor::Expansion,
            image: Arc::new(Vec::new()),
            sections: Vec::new(),
            fake: Some(Arc::new(map)),
        }
    }

    /// The string at `t` in this program; `None` when it has none there or
    /// the bytes are not plain text.
    pub fn get(&self, t: Txt) -> Option<&str> {
        #[cfg(test)]
        if let Some(fake) = &self.fake {
            return fake.get(&t.0).map(String::as_str);
        }
        let va = match self.flavor {
            Flavor::Expansion => t.0,
            Flavor::Core20 => t.1,
        };
        if va == 0 {
            return None;
        }
        let (base, raw, size) = self
            .sections
            .iter()
            .copied()
            .find(|(v, _, s)| (*v..v.saturating_add(*s)).contains(&va))?;
        let start = (va - base + raw) as usize;
        let end = (raw + size) as usize;
        let bytes = self.image.get(start..end.min(self.image.len()))?;
        let len = bytes.iter().position(|b| *b == 0)?;
        let bytes = &bytes[..len];
        if bytes.iter().all(|b| (0x20..0x7f).contains(b)) {
            std::str::from_utf8(bytes).ok()
        } else {
            None
        }
    }
}

/// (virtual address, raw offset, raw size) of each section of a PE image.
fn sections(image: &[u8]) -> Option<Vec<(u32, u32, u32)>> {
    let u16_at = |o: usize| Some(u16::from_le_bytes(image.get(o..o + 2)?.try_into().ok()?));
    let u32_at = |o: usize| Some(u32::from_le_bytes(image.get(o..o + 4)?.try_into().ok()?));
    let pe = u32_at(0x3c)? as usize;
    if image.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    let count = u16_at(pe + 6)? as usize;
    let optional = u16_at(pe + 20)? as usize;
    let base = u32_at(pe + 24 + 28)?;
    let table = pe + 24 + optional;
    (0..count)
        .map(|i| {
            let s = table + 40 * i;
            let virtual_size = u32_at(s + 8)?;
            let address = u32_at(s + 12)?;
            let raw_size = u32_at(s + 16)?;
            let raw = u32_at(s + 20)?;
            Some((base + address, raw, virtual_size.min(raw_size)))
        })
        .collect()
}

/// Where the columns of a ruler line ("----  -------") start.
fn starts(ruler: &str) -> Vec<usize> {
    let bytes = ruler.as_bytes();
    (0..bytes.len())
        .filter(|&i| bytes[i] == b'-' && (i == 0 || bytes[i - 1] == b' '))
        .collect()
}

fn is_ruler(line: &str) -> bool {
    line.contains('-') && line.bytes().all(|b| b == b'-' || b == b' ')
}

/// A fixed-width line cut at the column starts; a cut that would split a
/// word moves to the start of the word, or to its end when the word
/// starts in an earlier column.
pub(super) fn cut(line: &str, starts: &[usize]) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut bounds = Vec::new();
    let mut last = 0;
    for &s in starts.iter().skip(1) {
        let mut b = s.max(last);
        if b < bytes.len() && b > 0 && bytes[b - 1] != b' ' && bytes[b] != b' ' {
            let mut start = b;
            while start > 0 && bytes[start - 1] != b' ' {
                start -= 1;
            }
            if start > last {
                b = start;
            } else {
                while b < bytes.len() && bytes[b] != b' ' {
                    b += 1;
                }
            }
        }
        bounds.push(b.min(bytes.len()));
        last = b;
    }
    let mut cells = Vec::new();
    let mut from = 0;
    for b in bounds {
        cells.push(line.get(from..b).unwrap_or("").trim().to_owned());
        from = b.max(from);
    }
    cells.push(
        line.get(from.min(line.len())..)
            .unwrap_or("")
            .trim()
            .to_owned(),
    );
    cells
}

/// Splits on runs of two or more spaces.
fn split_wide(line: &str) -> Vec<String> {
    line.split("  ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The layout of a heading: the column starts of its ruler line.
pub(super) fn layout(lines: &[&str]) -> Vec<usize> {
    lines
        .iter()
        .rev()
        .find(|l| is_ruler(l))
        .map(|r| starts(r))
        .unwrap_or_else(|| vec![0])
}

/// The column labels of a heading (its lines, the ruler last), fitted to
/// `n` columns: extra columns join their neighbour over the narrowest gap
/// of the ruler (a space for a one-space gap, a comma otherwise) but never
/// the first, missing
/// ones are blank before the last.
pub(super) fn heading(lines: &[&str], n: usize) -> Vec<String> {
    // The ruler's gap before each column.
    let mut gaps: Vec<usize> = Vec::new();
    let mut labels: Vec<String> = match lines.iter().position(|l| is_ruler(l)) {
        Some(r) => {
            let starts = starts(lines[r]);
            let ruler = lines[r].as_bytes();
            gaps = starts
                .iter()
                .map(|&s| ruler[..s].iter().rev().take_while(|b| **b == b' ').count())
                .collect();
            let mut labels = vec![String::new(); starts.len()];
            for line in &lines[..r] {
                for (label, part) in labels.iter_mut().zip(cut(line, &starts)) {
                    if part.is_empty() {
                        continue;
                    }
                    if !label.is_empty() {
                        label.push(' ');
                    }
                    label.push_str(&part);
                }
            }
            labels
        }
        None => lines.iter().flat_map(|l| split_wide(l)).collect(),
    };
    if n == 0 {
        return labels;
    }
    gaps.resize(labels.len(), 2);
    while labels.len() > n {
        // The rightmost of the narrowest gaps; the first column (the
        // roll) stays as it is.
        let first = if n > 1 { 2 } else { 1 };
        let i = (first..labels.len())
            .rev()
            .min_by_key(|&i| gaps[i])
            .expect("more than one column");
        let right = labels.remove(i);
        let gap = gaps.remove(i);
        let left = &mut labels[i - 1];
        if !right.is_empty() {
            if !left.is_empty() {
                left.push_str(if gap == 1 { " " } else { ", " });
            }
            left.push_str(&right);
        }
    }
    while labels.len() < n {
        let at = labels.len().saturating_sub(1);
        labels.insert(at, String::new());
    }
    labels
}

/// Footnote lines as paragraphs: a line that starts with `*` starts a
/// new one.
pub(super) fn paragraphs(lines: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in lines {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(last) if !line.starts_with('*') => {
                last.push(' ');
                last.push_str(&line);
            }
            _ => out.push(line),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PE image with one section at 0x401000 holding `strings` from
    /// offset 0x10 on, NUL-separated; the address of each.
    pub(crate) fn image(strings: &[&str]) -> (Vec<u8>, Vec<u32>) {
        let mut image = vec![0u8; 0x300];
        image[0x3c] = 0x80;
        image[0x80..0x84].copy_from_slice(b"PE\0\0");
        image[0x86] = 1;
        image[0x94] = 0xe0;
        image[0x98 + 28..0x98 + 32].copy_from_slice(&0x400000u32.to_le_bytes());
        let mut body = vec![0u8; 0x10];
        let mut addresses = Vec::new();
        for s in strings {
            addresses.push(0x401000 + body.len() as u32);
            body.extend_from_slice(s.as_bytes());
            body.push(0);
        }
        let size = body.len() as u32;
        let s = 0x98 + 0xe0;
        image[s + 8..s + 12].copy_from_slice(&size.to_le_bytes());
        image[s + 12..s + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        image[s + 16..s + 20].copy_from_slice(&size.to_le_bytes());
        image[s + 20..s + 24].copy_from_slice(&0x300u32.to_le_bytes());
        image.extend(body);
        (image, addresses)
    }

    #[test]
    fn reads_strings_by_address_and_flavor() {
        let (image, at) = image(&["Abc", "D\u{1}"]);
        let program = Program::from_image(Flavor::Expansion, image.clone()).unwrap();
        assert_eq!(program.get(Txt(at[0], 0)), Some("Abc"));
        // Inside a string: its tail.
        assert_eq!(program.get(Txt(at[0] + 1, 0)), Some("bc"));
        // Not text, or not in this program.
        assert_eq!(program.get(Txt(at[1], 0)), None);
        assert_eq!(program.get(Txt(0x500000, 0)), None);
        let core = Program::from_image(Flavor::Core20, image).unwrap();
        assert_eq!(core.get(Txt(at[0], 0)), None);
        assert_eq!(core.get(Txt(0, at[0])), Some("Abc"));
    }

    #[test]
    fn headings_follow_the_ruler() {
        assert_eq!(
            heading(&["Roll  Aaa Bbb      Cc", "----  -------      --"], 3),
            ["Roll", "Aaa Bbb", "Cc"]
        );
        // Two lines over a ruler with single-space gaps.
        assert_eq!(
            heading(&["     Xx", "Roll Yyyy     Zz", "---- -----     --"], 0),
            ["Roll", "Xx Yyyy", "Zz"]
        );
        // Fitted: extra columns join, missing ones are blank.
        assert_eq!(heading(&["Aa  Bb  Cc", "--  --  --"], 2), ["Aa", "Bb, Cc"]);
        assert_eq!(heading(&["Aa  Bb Cc", "--  -- --"], 2), ["Aa", "Bb Cc"]);
        assert_eq!(heading(&["Aa Bb  Cc", "-- --  --"], 2), ["Aa", "Bb, Cc"]);
        assert_eq!(heading(&["Aa Bb", "-- --"], 1), ["Aa Bb"]);
        assert_eq!(heading(&["Aa  Bb", "--  --"], 3), ["Aa", "", "Bb"]);
        // No ruler: wide gaps.
        assert_eq!(heading(&["Aa  Bb Cc"], 0), ["Aa", "Bb Cc"]);
    }

    #[test]
    fn rows_cut_at_the_columns() {
        let starts = layout(&["Aaaa      Bb  Cc", "----      --  --"]);
        assert_eq!(starts, [0, 10, 14]);
        assert_eq!(cut("x y z     b   c d", &starts), ["x y z", "b", "c d"]);
        // A word over a column start is kept whole.
        assert_eq!(cut("abcdefghijkl c", &starts), ["abcdefghijkl", "c", ""]);
        // A word that starts a little early belongs to its column.
        assert_eq!(cut("a        bb   c", &starts), ["a", "bb", "c"]);
        assert_eq!(cut("short", &starts), ["short", "", ""]);
    }

    #[test]
    fn footnotes_become_paragraphs() {
        assert_eq!(
            paragraphs(&["* One  ", "two.", " ", "** Three", "four"]),
            ["* One two.", "** Three four"]
        );
    }
}
