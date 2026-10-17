//! The Table Rolls tables (`Tables/*.TBL`) of the DM Tool Kit.
//!
//! A table is plain Windows-1252 text with CRLF line ends: a sequence of
//! four-line records.
//!
//! ```text
//! <name>                        Item0 when the row has no text
//! <item1>                       Item1 when unused
//! <item2>                       Item2 when unused
//! <lo> <hi> <v3> <v4> <v5> <v6>
//! ```
//!
//! Record 0 is a header whose name is the table's title, which the Roll
//! Results dialog shows for follow-up rolls (see `cr_store::tables`).
//! The originals (0x5376ab) read a fixed number of records per table (a
//! count compiled into the program, see `cr_store::tables`) and never look
//! for the end of the file, so this reader does the same when given a
//! count: past the end every string is empty and every number 0. The
//! names are read with MFC's `CStdioFile::ReadString` (up to `\n`, a CR
//! before it dropped); the numbers one byte at a time up to a space or
//! `\n`, CRs ignored, converted with `atoi`. A number line with extra
//! numbers (DMG47) therefore spills into the next record exactly as it
//! did in the original.
//!
//! A name may hold `/n1` to `/n9` markers that split it into lines shown
//! under each other ([`TableRecord::lines`]).

/// One record. The placeholders `Item0`, `Item1` and `Item2` and empty
/// lines read as `None`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableRecord {
    pub name: Option<String>,
    pub item1: Option<String>,
    pub item2: Option<String>,
    /// Die range of the entry; 0 to 0 for rows looked up by position.
    pub lo: i32,
    pub hi: i32,
    /// Numbers whose meaning depends on the table: an experience value, a
    /// base value, dice, or the id of a table to roll on next.
    pub v3: i32,
    pub v4: i32,
    pub v5: i32,
    /// The item number of a magic item in the program's database.
    pub v6: i32,
}

impl TableRecord {
    /// The name split at its `/nK` markers: the first line, then the lines
    /// printed under it. A trailing empty line is dropped.
    pub fn lines(&self) -> Vec<&str> {
        match &self.name {
            Some(name) => split_lines(name),
            None => Vec::new(),
        }
    }

    /// The numbers v3 to v5.
    pub fn values(&self) -> [i32; 3] {
        [self.v3, self.v4, self.v5]
    }
}

/// A name split at its `/nK` markers, as [`TableRecord::lines`] does.
pub fn split_lines(name: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let bytes = name.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'n' && (b'1'..=b'9').contains(&bytes[i + 2]) {
            lines.push(name[start..i].trim_end());
            i += 3;
            start = i;
        } else {
            i += 1;
        }
    }
    lines.push(name[start..].trim_end());
    if lines.len() > 1 && lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

/// A whole table: the header record and the entries after it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableFile {
    /// The header's name (internal, e.g. a file label).
    pub title: String,
    pub entries: Vec<TableRecord>,
}

impl TableFile {
    /// Reads `count` records (header included) as the originals do, or
    /// every record up to the end of the file when `count` is `None`.
    pub fn parse(data: &[u8], count: Option<usize>) -> Self {
        let mut reader = Reader { data, pos: 0 };
        let mut records = Vec::new();
        loop {
            match count {
                Some(n) if records.len() >= n => break,
                None if reader.rest_is_blank() => break,
                _ => {}
            }
            records.push(reader.record());
        }
        let mut records = records.into_iter();
        let header = records.next().unwrap_or_default();
        Self {
            title: header.name.unwrap_or_default(),
            entries: records.collect(),
        }
    }

    /// The entry a roll selects: the first whose `hi` is at least the roll,
    /// as 0x50c88e scans them. `None` when the roll is above every entry.
    pub fn lookup(&self, roll: i32) -> Option<&TableRecord> {
        self.entries.iter().find(|e| e.hi >= roll)
    }

    /// The largest `hi`: the die the table is made for.
    pub fn max_roll(&self) -> i32 {
        self.entries.iter().map(|e| e.hi).max().unwrap_or(0)
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn rest_is_blank(&self) -> bool {
        self.data[self.pos..].iter().all(u8::is_ascii_whitespace)
    }

    /// `CStdioFile::ReadString`: up to `\n` (consumed), CR before it
    /// dropped.
    fn line(&mut self) -> String {
        let rest = &self.data[self.pos..];
        let end = rest.iter().position(|&b| b == b'\n');
        let (line, used) = match end {
            Some(e) => (&rest[..e], e + 1),
            None => (rest, rest.len()),
        };
        self.pos += used;
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        encoding_rs::WINDOWS_1252.decode(line).0.into_owned()
    }

    /// One number: bytes up to a space or `\n`, CRs skipped, then `atoi`.
    fn number(&mut self) -> i32 {
        let mut text = Vec::new();
        while let Some(&b) = self.data.get(self.pos) {
            self.pos += 1;
            match b {
                b' ' | b'\n' => break,
                b'\r' => {}
                _ => text.push(b),
            }
        }
        crate::atol(&String::from_utf8_lossy(&text))
    }

    fn record(&mut self) -> TableRecord {
        let placeholder = |s: String, p: &str| {
            let t = s.trim_end();
            (!t.is_empty() && t != p).then_some(s)
        };
        let name = placeholder(self.line(), "Item0");
        let item1 = placeholder(self.line(), "Item1");
        let item2 = placeholder(self.line(), "Item2");
        let [lo, hi, v3, v4, v5, v6] = std::array::from_fn(|_| self.number());
        TableRecord {
            name,
            item1,
            item2,
            lo,
            hi,
            v3,
            v4,
            v5,
            v6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"Example header\r\nItem1\r\nItem2\r\n0 0 0 0 0 0\r\n\
Copper ring\r\nItem1\r\nItem2\r\n1 3 20106 0 0 193\r\n\
Lamp /n1(lit)/n2\r\nsmall\r\nItem2\r\n4 6 500 90001 0 7\r\n\
Item0\r\nItem1\r\nItem2\r\n7 10 1 6 100 0\r\n";

    #[test]
    fn reads_records_and_placeholders() {
        let t = TableFile::parse(SAMPLE, None);
        assert_eq!(t.title, "Example header");
        assert_eq!(t.entries.len(), 3);
        let e = &t.entries[0];
        assert_eq!(e.name.as_deref(), Some("Copper ring"));
        assert_eq!((e.item1.as_deref(), e.item2.as_deref()), (None, None));
        assert_eq!(
            (e.lo, e.hi, e.v3, e.v4, e.v5, e.v6),
            (1, 3, 20106, 0, 0, 193)
        );
        assert_eq!(t.entries[1].item1.as_deref(), Some("small"));
        assert_eq!(t.entries[1].lines(), ["Lamp", "(lit)"]);
        assert_eq!(t.entries[2].name, None);
        assert!(t.entries[2].lines().is_empty());
        assert_eq!(t.entries[2].values(), [1, 6, 100]);
        assert_eq!(t.max_roll(), 10);
    }

    #[test]
    fn lookup_takes_the_first_entry_reaching_the_roll() {
        let t = TableFile::parse(SAMPLE, None);
        assert_eq!(t.lookup(1).unwrap().name.as_deref(), Some("Copper ring"));
        assert_eq!(t.lookup(3).unwrap().name.as_deref(), Some("Copper ring"));
        assert_eq!(t.lookup(5).unwrap().v3, 500);
        assert_eq!(t.lookup(10).unwrap().name, None);
        assert!(t.lookup(11).is_none());
    }

    #[test]
    fn a_fixed_count_reads_past_the_end_as_empty_records() {
        let t = TableFile::parse(SAMPLE, Some(6));
        assert_eq!(t.entries.len(), 5);
        assert_eq!(t.entries[3], TableRecord::default());
        let t = TableFile::parse(SAMPLE, Some(2));
        assert_eq!(t.entries.len(), 1);
    }

    #[test]
    fn extra_numbers_spill_into_the_next_record() {
        // The number line of the first entry has eight numbers, as in the
        // DMG47 defect; the reader takes the extra "7 8" as the next name.
        let data = b"H\r\nItem1\r\nItem2\r\n0 0 0 0 0 0\r\n\
A\r\nItem1\r\nItem2\r\n1 2 3 4 5 6 7 8\r\nB\r\nItem2\r\n0 1 9 0 0 0\r\n";
        let t = TableFile::parse(data, Some(3));
        assert_eq!(t.entries[0].v6, 6);
        assert_eq!(t.entries[1].name.as_deref(), Some("7 8"));
        assert_eq!(t.entries[1].item1.as_deref(), Some("B"));
        assert_eq!(t.entries[1].item2, None);
        assert_eq!(t.entries[1].v3, 9);
    }

    #[test]
    fn numbers_follow_atoi_and_text_is_windows_1252() {
        let data = b"H\r\nItem1\r\nItem2\r\n0 0 0 0 0 0\r\n\
Caf\xe9 \x96 x\r\nItem1\r\nItem2\r\n 1 x3 -4 7abc  \r\n";
        let t = TableFile::parse(data, Some(2));
        let e = &t.entries[0];
        assert_eq!(e.name.as_deref(), Some("Café \u{2013} x"));
        // " 1" is one token only up to the space: the leading space ends
        // an empty token, so lo = 0, hi = 1.
        assert_eq!((e.lo, e.hi, e.v3, e.v4, e.v5, e.v6), (0, 1, 0, -4, 7, 0));
    }

    #[test]
    fn continuation_markers() {
        let r = |name: &str| TableRecord {
            name: Some(name.into()),
            ..Default::default()
        };
        assert_eq!(r("A /n1B/n2").lines(), ["A", "B"]);
        assert_eq!(r("A/n1/n2C").lines(), ["A", "", "C"]);
        assert_eq!(r("one/n").lines(), ["one/n"]);
        assert_eq!(r("x/n0y").lines(), ["x/n0y"]);
    }
}
