//! WinHelp files (`Help/*.hlp`): the Online Manual, the program's help and
//! the books' help versions, as the Help… buttons and the Help menu open
//! them.
//!
//! Read-only, and only the WinHelp 3.1 and 4 layout the installation's
//! files use (`|SYSTEM` minor version above 16). What is read:
//!
//! * the internal file system: a B+ tree directory of internal files;
//! * `|SYSTEM`: the title, copyright, contents topic and whether `|TOPIC`
//!   is LZ77 compressed;
//! * `|TOPIC`: topic headers and the display and table records, their
//!   paragraph formats, text and links, phrase-compressed through
//!   `|Phrases` or `|PhrIndex`/`|PhrImage`;
//! * `|CONTEXT` (context string hashes) and `|CTXOMAP` (the `[MAP]`
//!   numbers a program passes with `HELP_CONTEXT`), both to topic offsets;
//! * `|FONT`: the character formats the records switch between;
//! * `|bmN`: the bitmaps the topics show (device-independent and
//!   device-dependent bitmaps, uncompressed, RLE or LZ77 packed, and
//!   metafiles that wrap a bitmap).
//!
//! Keywords, embedded windows and the macros that do not lead to a topic
//! or a web page ([`Macro`]) are left out. The layout is specified in
//! `docs/formats/help.md`.

use std::collections::HashMap;

use crate::rtf::{Align, Format, Rgb, TabKind, TabStop, cp1252};
use crate::{Error, Result};

const MAGIC: u32 = 0x0003_5F3F;
const BTREE_MAGIC: u16 = 0x293B;
/// `TOPICPOS` block size of WinHelp 3.1 and later, compressed or not.
const POS_BLOCK: usize = 0x4000;
/// `TOPICOFFSET` block size.
const OFFSET_BLOCK: u32 = 0x8000;

/// Where a hotspot leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// A topic by context hash, in `file` when another help file names it;
    /// a popup shows it in a small window.
    Topic {
        hash: u32,
        file: Option<String>,
        popup: bool,
    },
    /// A topic by offset (WinHelp 3.0 hotspots).
    Offset { offset: u32, popup: bool },
    /// A WinHelp macro.
    Macro(String),
}

/// What a hotspot's macro does, for the macros help files use as links.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Macro {
    /// `JumpId`/`JI`, or `PopupId`/`PI` for a popup: topic `context` of
    /// `file`, or of the file shown when there is none.
    Jump {
        file: Option<String>,
        context: String,
        popup: bool,
    },
    /// `JumpContents`: the contents topic of `file`.
    Contents { file: Option<String> },
    /// `Inet`: a web page.
    Web(String),
}

impl Macro {
    /// Reads the first macro of a hotspot's macro string; `None` for
    /// macros that do not lead anywhere.
    pub fn parse(text: &str) -> Option<Self> {
        let (name, args) = text.split_once('(')?;
        let args = args.split_once(')').map_or(args, |(a, _)| a);
        let args: Vec<&str> = args
            .split(',')
            .map(|a| a.trim().trim_matches(|c| c == '`' || c == '\'' || c == '"'))
            .collect();
        // `file>window`: the window does not matter here.
        let file = |arg: Option<&&str>| {
            let file = arg?.split('>').next()?.trim();
            (!file.is_empty()).then(|| file.to_owned())
        };
        let name = name.trim().to_ascii_lowercase();
        match name.as_str() {
            "jumpid" | "ji" | "popupid" | "pi" if args.len() >= 2 => Some(Macro::Jump {
                file: file(args.first()),
                context: args[1].to_owned(),
                popup: name.starts_with('p'),
            }),
            "jumpcontents" | "jc" => Some(Macro::Contents {
                file: file(args.first()),
            }),
            "inet" => args
                .first()
                .filter(|a| !a.is_empty())
                .map(|a| Macro::Web((*a).to_owned())),
            _ => None,
        }
    }
}

/// A run of text in one format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub format: Format,
    pub link: Option<Link>,
}

/// One table cell's place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Rows of the topic's tables, counted from 0 through the topic.
    pub row: u32,
    pub column: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Text(Run),
    /// A bitmap from `|bmN`, by number; see [`HelpFile::picture`].
    Picture(u16),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Paragraph {
    pub align: Align,
    /// Indents and spacing in twips.
    pub left_indent: i32,
    pub first_indent: i32,
    pub right_indent: i32,
    pub space_before: i32,
    pub space_after: i32,
    pub tabs: Vec<TabStop>,
    /// The cell, for a paragraph in a table.
    pub cell: Option<Cell>,
    /// In the non-scrolling region at the top of the topic.
    pub non_scrolling: bool,
    pub items: Vec<Item>,
}

impl Paragraph {
    pub fn text(&self) -> String {
        self.items
            .iter()
            .filter_map(|i| match i {
                Item::Text(r) => Some(r.text.as_str()),
                Item::Picture(_) => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Topic {
    /// The `$` title; empty for topics without one.
    pub title: String,
    /// The topic's `TOPICOFFSET`.
    pub offset: u32,
    pub paragraphs: Vec<Paragraph>,
}

impl Topic {
    /// The title, or else the first paragraph with text.
    pub fn display_title(&self) -> String {
        if !self.title.is_empty() {
            return self.title.clone();
        }
        self.paragraphs
            .iter()
            .map(|p| p.text().trim().to_owned())
            .find(|t| !t.is_empty())
            .unwrap_or_default()
    }
}

/// A picture from a `|bmN` file, as a Windows bitmap file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap {
    /// A complete `.bmp` file.
    pub bmp: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct HelpFile {
    pub title: String,
    pub copyright: String,
    /// Font face names, by [`Format::font`].
    pub faces: Vec<String>,
    pub topics: Vec<Topic>,
    /// The contents topic (`|SYSTEM` record 3), as an index into `topics`.
    pub contents: usize,
    contexts: HashMap<u32, u32>,
    map: HashMap<u32, u32>,
    pictures: HashMap<u16, Vec<u8>>,
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8], at: usize) -> Self {
        Self { data, at }
    }

    fn eof(&self, wanted: usize) -> Error {
        Error::UnexpectedEof {
            offset: self.at,
            wanted,
        }
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let b = self
            .data
            .get(self.at..self.at + n)
            .ok_or_else(|| self.eof(n))?;
        self.at += n;
        Ok(b)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn i16(&mut self) -> Result<i16> {
        Ok(self.u16()? as i16)
    }

    fn peek(&self) -> Result<u8> {
        self.data.get(self.at).copied().ok_or_else(|| self.eof(1))
    }

    /// Compressed signed short: one byte when its low bit is clear.
    fn cshort(&mut self) -> Result<i32> {
        Ok(if self.peek()? & 1 != 0 {
            i32::from(self.u16()? >> 1) - 0x4000
        } else {
            i32::from(self.u8()? >> 1) - 0x40
        })
    }

    fn cushort(&mut self) -> Result<u32> {
        Ok(if self.peek()? & 1 != 0 {
            u32::from(self.u16()? >> 1)
        } else {
            u32::from(self.u8()? >> 1)
        })
    }

    /// Compressed signed long: two bytes when its low bit is clear.
    fn clong(&mut self) -> Result<i64> {
        Ok(if self.peek()? & 1 != 0 {
            i64::from(self.u32()? >> 1) - 0x4000_0000
        } else {
            i64::from(self.u16()? >> 1) - 0x4000
        })
    }

    fn cstring(&mut self) -> Result<String> {
        let rest = &self.data[self.at.min(self.data.len())..];
        let end = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| self.eof(1))?;
        self.at += end + 1;
        Ok(ansi(&rest[..end]))
    }
}

fn ansi(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| cp1252(b)).collect()
}

/// WinHelp's LZ77: a flag byte for each eight items, a set bit for a
/// 12-bit back reference with a 4-bit length (3 to 18).
pub fn lz77(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len() * 2);
    let mut i = 0;
    while i < src.len() {
        let flags = src[i];
        i += 1;
        for bit in 0..8 {
            if i >= src.len() {
                break;
            }
            if flags & (1 << bit) != 0 {
                if i + 1 >= src.len() {
                    return out;
                }
                let token = u16::from_le_bytes([src[i], src[i + 1]]);
                i += 2;
                let back = usize::from(token & 0x0FFF) + 1;
                let length = usize::from(token >> 12) + 3;
                for _ in 0..length {
                    let b = out.len().checked_sub(back).map_or(0, |at| out[at]);
                    out.push(b);
                }
            } else {
                out.push(src[i]);
                i += 1;
            }
        }
    }
    out
}

/// The hash WinHelp keeps of a context string.
pub fn context_hash(name: &str) -> u32 {
    name.bytes().fold(0u32, |hash, b| {
        let value: i32 = match b {
            b'1'..=b'9' => i32::from(b - b'0'),
            b'0' => 10,
            b'.' => 12,
            b'_' => 13,
            b'a'..=b'z' => i32::from(b - b'a' + b'A') - 0x30,
            _ => i32::from(b as i8) - 0x30,
        };
        hash.wrapping_mul(43).wrapping_add(value as u32)
    })
}

/// The leaf entries of a B+ tree internal file, read by `entry`.
fn btree<T>(file: &[u8], mut entry: impl FnMut(&mut Reader) -> Result<T>) -> Result<Vec<T>> {
    let mut r = Reader::new(file, 0);
    if r.u16()? != BTREE_MAGIC {
        return Err(Error::Malformed("not a WinHelp B+ tree".into()));
    }
    r.u16()?;
    let page_size = usize::from(r.u16()?);
    r.bytes(16 + 2 + 2)?;
    let root = r.u16()?;
    r.u16()?;
    r.u16()?;
    let levels = r.u16()?;
    r.u32()?;
    let pages = &file[38.min(file.len())..];
    let page_at = |page: u16| {
        let start = usize::from(page) * page_size;
        pages
            .get(start..start + page_size)
            .ok_or(Error::Malformed("WinHelp B+ tree page out of range".into()))
    };
    let mut page = root;
    for _ in 1..levels {
        let mut r = Reader::new(page_at(page)?, 4);
        page = r.u16()?;
    }
    let mut out = Vec::new();
    let mut seen = 0;
    while page != 0xFFFF {
        seen += 1;
        if seen > pages.len() / page_size.max(1) + 1 {
            return Err(Error::Malformed("WinHelp B+ tree loops".into()));
        }
        let mut r = Reader::new(page_at(page)?, 2);
        let n = r.u16()?;
        r.u16()?;
        let next = r.u16()?;
        for _ in 0..n {
            out.push(entry(&mut r)?);
        }
        page = next;
    }
    Ok(out)
}

/// The phrases of a phrase-compressed file, and how they are referenced.
enum Phrases {
    None,
    /// `|Phrases`: two bytes per reference, odd numbers add a space.
    Old(Vec<Vec<u8>>),
    /// `|PhrIndex` and `|PhrImage` (Hall compression).
    Hall(Vec<Vec<u8>>),
}

impl Phrases {
    fn expand(&self, data: &[u8], size: usize) -> Vec<u8> {
        if data.len() >= size {
            return data.to_vec();
        }
        let mut out = Vec::with_capacity(size);
        let mut i = 0;
        let phrase = |list: &Vec<Vec<u8>>, n: usize| list.get(n).cloned().unwrap_or_default();
        match self {
            Phrases::None => return data.to_vec(),
            Phrases::Old(list) => {
                while i < data.len() && out.len() < size {
                    let b = data[i];
                    if (1..0x10).contains(&b) && i + 1 < data.len() {
                        let n = usize::from(b - 1) * 256 + usize::from(data[i + 1]);
                        out.extend(phrase(list, n >> 1));
                        if n & 1 != 0 {
                            out.push(b' ');
                        }
                        i += 2;
                    } else {
                        out.push(b);
                        i += 1;
                    }
                }
            }
            Phrases::Hall(list) => {
                while i < data.len() && out.len() < size {
                    let b = data[i];
                    if b & 1 == 0 {
                        out.extend(phrase(list, usize::from(b >> 1)));
                        i += 1;
                    } else if b & 3 == 1 {
                        let next = data.get(i + 1).copied().unwrap_or(0);
                        out.extend(phrase(
                            list,
                            128 + usize::from(b >> 2) * 256 + usize::from(next),
                        ));
                        i += 2;
                    } else if b & 7 == 3 {
                        let n = usize::from(b >> 3) + 1;
                        let end = (i + 1 + n).min(data.len());
                        out.extend_from_slice(&data[i + 1..end]);
                        i = end;
                    } else if b & 15 == 7 {
                        out.extend(std::iter::repeat_n(b' ', usize::from(b >> 4) + 1));
                        i += 1;
                    } else {
                        out.extend(std::iter::repeat_n(0, usize::from(b >> 4) + 1));
                        i += 1;
                    }
                }
            }
        }
        out
    }
}

/// An `OLDFONT` descriptor of `|FONT`.
#[derive(Debug, Clone, Copy)]
struct FontDescriptor {
    bold: bool,
    italic: bool,
    underline: bool,
    small_caps: bool,
    half_points: u8,
    face: u16,
    color: Rgb,
}

impl FontDescriptor {
    fn format(&self) -> Format {
        let Rgb(r, g, b) = self.color;
        // The help compiler's automatic colour.
        let automatic = r <= 1 && g <= 1 && b <= 1;
        Format {
            size: u16::from(self.half_points.max(8)),
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            small_caps: self.small_caps,
            color: (!automatic).then_some(self.color),
            font: self.face,
            ..Format::default()
        }
    }
}

struct Parser<'a> {
    data: &'a [u8],
    files: HashMap<String, usize>,
    minor: u16,
    flags: u16,
}

impl<'a> Parser<'a> {
    fn file(&self, name: &str) -> Option<&'a [u8]> {
        let &at = self.files.get(name)?;
        let mut r = Reader::new(self.data, at + 4);
        let used = r.u32().ok()? as usize;
        self.data.get(at + 9..at + 9 + used)
    }

    fn phrases(&self) -> Result<Phrases> {
        if let (Some(index), Some(image)) = (self.file("|PhrIndex"), self.file("|PhrImage")) {
            let mut r = Reader::new(index, 4);
            let n = r.u32()? as usize;
            r.u32()?;
            let image_size = r.u32()? as usize;
            let compressed_size = r.u32()? as usize;
            r.u32()?;
            let bits = u32::from(r.u16()? & 0xF);
            let image = if compressed_size < image_size {
                lz77(image)
            } else {
                image.to_vec()
            };
            let words = index.get(28..).unwrap_or_default();
            let mut at = 0usize;
            let mut bit = || {
                let byte = words.get(at / 8).copied().unwrap_or(0);
                at += 1;
                (byte >> ((at - 1) % 8)) & 1 != 0
            };
            let mut offsets = vec![0usize];
            for _ in 0..n {
                let mut k = 1usize;
                while bit() {
                    k += 1 << bits;
                }
                for (min_bits, add) in [(0, 1), (1, 2), (2, 4), (3, 8), (4, 16)] {
                    if bits > min_bits && bit() {
                        k += add;
                    }
                }
                offsets.push(offsets.last().unwrap() + k);
            }
            let list = offsets
                .windows(2)
                .map(|w| image.get(w[0]..w[1]).unwrap_or_default().to_vec())
                .collect();
            return Ok(Phrases::Hall(list));
        }
        if let Some(file) = self.file("|Phrases") {
            let mut r = Reader::new(file, 0);
            let n = usize::from(r.u16()?);
            r.u16()?;
            r.u32()?;
            let offsets = (0..=n).map(|_| r.u16()).collect::<Result<Vec<_>>>()?;
            let text = lz77(&file[r.at..]);
            let base = 2 * (n + 1) + 4;
            let list = offsets
                .windows(2)
                .map(|w| {
                    let (a, b) = (usize::from(w[0]), usize::from(w[1]));
                    text.get(a.saturating_sub(base)..b.saturating_sub(base))
                        .unwrap_or_default()
                        .to_vec()
                })
                .collect();
            return Ok(Phrases::Old(list));
        }
        Ok(Phrases::None)
    }

    /// The font face names and the character formats of `|FONT`.
    fn fonts(&self) -> Result<(Vec<String>, Vec<FontDescriptor>)> {
        let Some(file) = self.file("|FONT") else {
            return Ok((Vec::new(), Vec::new()));
        };
        let mut r = Reader::new(file, 0);
        let faces = usize::from(r.u16()?);
        let count = r.u16()?;
        let faces_at = usize::from(r.u16()?);
        let at = usize::from(r.u16()?);
        let width = at.saturating_sub(faces_at) / faces.max(1);
        let names = (0..faces)
            .map(|i| {
                let name = file
                    .get(faces_at + i * width..faces_at + (i + 1) * width)
                    .unwrap_or_default();
                ansi(name.split(|&b| b == 0).next().unwrap_or_default())
            })
            .collect();
        let mut r = Reader::new(file, at);
        let descriptors = (0..count)
            .map(|_| {
                let attributes = r.u8()?;
                let half_points = r.u8()?;
                r.u8()?;
                let face = r.u16()?;
                let fg = r.bytes(3)?;
                r.bytes(3)?;
                Ok(FontDescriptor {
                    bold: attributes & 1 != 0,
                    italic: attributes & 2 != 0,
                    underline: attributes & (4 | 16) != 0,
                    small_caps: attributes & 32 != 0,
                    half_points,
                    face,
                    color: Rgb(fg[0], fg[1], fg[2]),
                })
            })
            .collect::<Result<_>>()?;
        Ok((names, descriptors))
    }
}

/// The paragraph format of a display or table record.
fn paragraph_format(r: &mut Reader, table: bool) -> Result<Option<(Paragraph, u16)>> {
    let mut column = 0;
    if table {
        let c = r.i16()?;
        if c == -1 {
            return Ok(None);
        }
        column = c as u16;
        r.bytes(3)?;
    }
    // Two unknown bytes and the paragraph's id.
    r.bytes(4)?;
    let bits = r.u16()?;
    if bits & 1 != 0 {
        r.clong()?;
    }
    let mut p = Paragraph::default();
    // The help compiler's units are tenths of a point's twips: 1 = 10.
    let mut field = |bit: u16, slot: &mut i32| -> Result<()> {
        if bits & bit != 0 {
            *slot = r.cshort()? * 10;
        }
        Ok(())
    };
    let mut spacing = 0;
    field(0x02, &mut p.space_before)?;
    field(0x04, &mut p.space_after)?;
    field(0x08, &mut spacing)?;
    field(0x10, &mut p.left_indent)?;
    field(0x20, &mut p.right_indent)?;
    field(0x40, &mut p.first_indent)?;
    if bits & 0x100 != 0 {
        r.bytes(3)?;
    }
    if bits & 0x200 != 0 {
        let n = r.cshort()?;
        for _ in 0..n.max(0) {
            let stop = r.cushort()?;
            let kind = if stop & 0x4000 != 0 {
                match r.cushort()? {
                    1 => TabKind::Right,
                    2 => TabKind::Center,
                    _ => TabKind::Left,
                }
            } else {
                TabKind::Left
            };
            p.tabs.push(TabStop {
                pos: (stop & 0x3FFF) as i32 * 10,
                kind,
            });
        }
    }
    p.align = if bits & 0x400 != 0 {
        Align::Right
    } else if bits & 0x800 != 0 {
        Align::Center
    } else {
        Align::Left
    };
    Ok(Some((p, column)))
}

/// Topic records being turned into paragraphs.
struct Builder<'f> {
    fonts: &'f [FontDescriptor],
    format: Format,
    link: Option<Link>,
    paragraphs: Vec<Paragraph>,
}

impl Builder<'_> {
    fn text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let p = self.paragraphs.last_mut().unwrap();
        if let Some(Item::Text(run)) = p.items.last_mut()
            && run.format == self.format
            && run.link == self.link
        {
            run.text.push_str(text);
            return;
        }
        p.items.push(Item::Text(Run {
            text: text.to_owned(),
            format: self.format,
            link: self.link.clone(),
        }));
    }

    /// Reads one paragraph's formatting commands, taking the text before
    /// each from `texts`.
    fn commands<'t>(
        &mut self,
        r: &mut Reader,
        texts: &mut impl Iterator<Item = &'t [u8]>,
        template: &Paragraph,
    ) -> Result<()> {
        loop {
            if let Some(t) = texts.next() {
                self.text(&ansi(t));
            }
            if r.at >= r.data.len() {
                return Ok(());
            }
            match r.u8()? {
                0xFF => return Ok(()),
                0x20 => {
                    r.u32()?;
                }
                0x21 => {
                    r.u16()?;
                }
                0x80 => {
                    let font = r.u16()?;
                    self.format = self
                        .fonts
                        .get(usize::from(font))
                        .map_or_else(Format::default, FontDescriptor::format);
                }
                0x81 => self.text("\n"),
                0x82 => self.paragraphs.push(Paragraph {
                    items: Vec::new(),
                    ..template.clone()
                }),
                0x83 => self.text("\t"),
                0x86..=0x88 => {
                    let kind = r.u8()?;
                    let size = r.clong()?.max(0) as usize;
                    if kind == 0x22 {
                        r.cushort()?;
                    }
                    let data = r.bytes(size)?;
                    // A reference to `|bmN`: embedded flag, then the number.
                    if (kind == 3 || kind == 0x22) && data.len() >= 4 {
                        let n = u16::from_le_bytes([data[2], data[3]]);
                        self.paragraphs
                            .last_mut()
                            .unwrap()
                            .items
                            .push(Item::Picture(n));
                    }
                }
                0x89 => self.link = None,
                0x8B => self.text("\u{a0}"),
                0x8C => self.text("\u{2011}"),
                0xC8 | 0xCC => {
                    let n = usize::from(r.u16()?);
                    let macro_text = r.bytes(n)?;
                    let end = macro_text.iter().position(|&b| b == 0).unwrap_or(n);
                    self.link = Some(Link::Macro(ansi(&macro_text[..end])));
                }
                c @ (0xE0 | 0xE1) => {
                    self.link = Some(Link::Offset {
                        offset: r.u32()?,
                        popup: c == 0xE0,
                    });
                }
                c @ (0xE2 | 0xE3 | 0xE6 | 0xE7) => {
                    self.link = Some(Link::Topic {
                        hash: r.u32()?,
                        file: None,
                        popup: c == 0xE2 || c == 0xE6,
                    });
                }
                c @ (0xEA | 0xEB | 0xEE | 0xEF) => {
                    let n = usize::from(r.u16()?);
                    let body = r.bytes(n)?;
                    let mut b = Reader::new(body, 0);
                    let kind = b.u8()?;
                    let hash = b.u32()?;
                    if kind == 1 {
                        b.u8()?;
                    }
                    let file = if kind == 4 || kind == 6 {
                        Some(b.cstring()?).filter(|f| !f.is_empty())
                    } else {
                        None
                    };
                    self.link = Some(Link::Topic {
                        hash,
                        file,
                        popup: c == 0xEA || c == 0xEE,
                    });
                }
                c => {
                    return Err(Error::Malformed(format!(
                        "unknown WinHelp formatting command {c:#04x}"
                    )));
                }
            }
        }
    }
}

impl HelpFile {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data, 0);
        if r.u32()? != MAGIC {
            return Err(Error::Malformed("not a WinHelp file".into()));
        }
        let directory = r.u32()? as usize;
        let mut d = Reader::new(data, directory + 4);
        let used = d.u32()? as usize;
        let dir = data
            .get(directory + 9..directory + 9 + used)
            .ok_or(Error::Malformed("WinHelp directory out of range".into()))?;
        let files = btree(dir, |r| Ok((r.cstring()?, r.u32()? as usize)))?
            .into_iter()
            .collect();
        let mut parser = Parser {
            data,
            files,
            minor: 0,
            flags: 0,
        };
        let system = parser
            .file("|SYSTEM")
            .ok_or(Error::Malformed("WinHelp file without |SYSTEM".into()))?;
        let mut s = Reader::new(system, 2);
        parser.minor = s.u16()?;
        s.u16()?;
        s.u32()?;
        parser.flags = s.u16()?;
        if parser.minor <= 16 {
            return Err(Error::Malformed(
                "WinHelp 3.0 files are not supported".into(),
            ));
        }
        let (mut title, mut copyright, mut contents_offset) = (String::new(), String::new(), 0);
        while s.at + 4 <= system.len() {
            let kind = s.u16()?;
            let size = usize::from(s.u16()?);
            let record = s.bytes(size)?;
            let text = || ansi(record.split(|&b| b == 0).next().unwrap_or_default());
            match kind {
                1 => title = text(),
                2 => copyright = text(),
                3 if record.len() >= 4 => {
                    contents_offset = u32::from_le_bytes(record[..4].try_into().unwrap());
                }
                _ => {}
            }
        }
        let (faces, fonts) = parser.fonts()?;
        let phrases = parser.phrases()?;
        let topics = parser.topics(&fonts, &phrases)?;
        let contexts = parser
            .file("|CONTEXT")
            .map(|f| btree(f, |r| Ok((r.u32()?, r.u32()?))))
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .collect();
        let mut map = HashMap::new();
        if let Some(f) = parser.file("|CTXOMAP") {
            let mut r = Reader::new(f, 0);
            for _ in 0..r.u16()? {
                let id = r.u32()?;
                map.insert(id, r.u32()?);
            }
        }
        let mut pictures = HashMap::new();
        for name in parser.files.keys() {
            if let Some(n) = name.strip_prefix("|bm").and_then(|n| n.parse().ok())
                && let Some(f) = parser.file(name)
            {
                pictures.insert(n, f.to_vec());
            }
        }
        let mut help = HelpFile {
            title,
            copyright,
            faces,
            topics,
            contents: 0,
            contexts,
            map,
            pictures,
        };
        help.contents = help.topic_at(contents_offset).unwrap_or(0);
        Ok(help)
    }

    /// The topic holding `offset`.
    pub fn topic_at(&self, offset: u32) -> Option<usize> {
        match self.topics.binary_search_by_key(&offset, |t| t.offset) {
            Ok(i) => Some(i),
            Err(0) => None,
            Err(i) => Some(i - 1),
        }
    }

    /// The topic of a context string's hash.
    pub fn topic_of_hash(&self, hash: u32) -> Option<usize> {
        self.topic_at(*self.contexts.get(&hash)?)
    }

    /// The topic of a context string (`Online_Manual`).
    pub fn topic_of_context(&self, name: &str) -> Option<usize> {
        self.topic_of_hash(context_hash(name))
    }

    /// The topic of a `[MAP]` number, as `HELP_CONTEXT` passes it.
    pub fn topic_of_number(&self, number: u32) -> Option<usize> {
        self.topic_at(*self.map.get(&number)?)
    }

    /// The `[MAP]` numbers.
    pub fn numbers(&self) -> impl Iterator<Item = u32> + '_ {
        self.map.keys().copied()
    }

    /// A link's topic in this file, when it is one.
    pub fn link_topic(&self, link: &Link) -> Option<usize> {
        match link {
            Link::Topic {
                hash, file: None, ..
            } => self.topic_of_hash(*hash),
            Link::Offset { offset, .. } => self.topic_at(*offset),
            _ => None,
        }
    }

    /// Bitmap `n` (`|bmN`) as a `.bmp` file: the first picture in it that
    /// is a bitmap.
    pub fn picture(&self, n: u16) -> Option<Bitmap> {
        crate::winhelp_bitmap::bitmap(self.pictures.get(&n)?)
    }
}

impl Parser<'_> {
    fn topics(&self, fonts: &[FontDescriptor], phrases: &Phrases) -> Result<Vec<Topic>> {
        let Some(file) = self.file("|TOPIC") else {
            return Ok(Vec::new());
        };
        let compressed = self.flags & (4 | 8) != 0;
        let block_size = if self.flags == 8 { 2048 } else { 4096 };
        let mut buffer = Vec::new();
        let mut starts = Vec::new();
        for block in file.chunks(block_size) {
            starts.push(buffer.len());
            let body = block.get(12..).unwrap_or_default();
            if compressed {
                buffer.extend(lz77(body));
            } else {
                buffer.extend_from_slice(body);
            }
        }
        let position = |pos: u32| -> Option<usize> {
            let (block, at) = (pos as usize / POS_BLOCK, pos as usize % POS_BLOCK);
            Some(starts.get(block)? + at.checked_sub(12)?)
        };
        let mut topics: Vec<Topic> = Vec::new();
        let mut builder = Builder {
            fonts,
            format: Format::default(),
            link: None,
            paragraphs: Vec::new(),
        };
        let (mut block, mut chars) = (u32::MAX, 0u32);
        let mut row = 0u32;
        let mut non_scrolling = 0u32;
        let mut pos = 12u32;
        let mut seen = 0usize;
        while let Some(at) = position(pos) {
            seen += 1;
            if at + 21 > buffer.len() || seen > buffer.len() {
                break;
            }
            let mut r = Reader::new(&buffer, at);
            let size = r.u32()? as usize;
            let size2 = r.u32()? as usize;
            r.u32()?;
            let next = r.u32()?;
            let size1 = r.u32()? as usize;
            let kind = r.u8()?;
            let end = (at + size).min(buffer.len());
            let data1 = buffer.get(at + 21..at + size1).unwrap_or_default();
            let data2 = phrases.expand(buffer.get(at + size1..end).unwrap_or_default(), size2);
            let b = pos / POS_BLOCK as u32;
            if b != block {
                (block, chars) = (b, 0);
            }
            let offset = block * OFFSET_BLOCK + chars;
            match kind {
                2 => {
                    if let Some(t) = topics.last_mut() {
                        t.paragraphs = std::mem::take(&mut builder.paragraphs);
                    }
                    let mut h = Reader::new(data1, 16);
                    non_scrolling = h.u32().unwrap_or(u32::MAX);
                    let title = data2.split(|&b| b == 0).next().unwrap_or_default();
                    topics.push(Topic {
                        title: ansi(title),
                        offset,
                        paragraphs: Vec::new(),
                    });
                    row = 0;
                }
                0x20 | 0x23 => {
                    let mut r = Reader::new(data1, 0);
                    r.clong()?;
                    let length = r.cushort()?;
                    let table = kind == 0x23;
                    if table {
                        let columns = r.u8()?;
                        let table_kind = r.u8()?;
                        if table_kind == 0 || table_kind == 2 {
                            r.u16()?;
                        }
                        r.bytes(4 * usize::from(columns))?;
                    }
                    let mut texts = data2.split(|&b| b == 0);
                    let scrolling = non_scrolling == u32::MAX || pos >= non_scrolling;
                    while r.at < data1.len() {
                        let Some((mut template, column)) = paragraph_format(&mut r, table)? else {
                            break;
                        };
                        template.cell = table.then_some(Cell { row, column });
                        template.non_scrolling = !scrolling;
                        builder.paragraphs.push(template.clone());
                        builder.commands(&mut r, &mut texts, &template)?;
                        // A paragraph end before the record's 0xFF leaves an
                        // empty paragraph behind.
                        if builder
                            .paragraphs
                            .last()
                            .is_some_and(|p| p.items.is_empty())
                        {
                            builder.paragraphs.pop();
                        }
                        if !table {
                            break;
                        }
                    }
                    if table {
                        row += 1;
                    }
                    chars += length;
                }
                _ => {}
            }
            if next == 0 || next == u32::MAX || next == pos {
                break;
            }
            pos = next;
        }
        if let Some(t) = topics.last_mut() {
            t.paragraphs = std::mem::take(&mut builder.paragraphs);
        }
        Ok(topics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_macros_jump_or_open_a_page() {
        assert_eq!(
            Macro::parse("JI(`players.hlp',`Dwarves_PH')"),
            Some(Macro::Jump {
                file: Some("players.hlp".into()),
                context: "Dwarves_PH".into(),
                popup: false,
            })
        );
        assert_eq!(
            Macro::parse("PopupId(`', `Note')"),
            Some(Macro::Jump {
                file: None,
                context: "Note".into(),
                popup: true,
            })
        );
        assert_eq!(
            Macro::parse("JumpContents(`Tome.hlp>main')"),
            Some(Macro::Contents {
                file: Some("Tome.hlp".into())
            })
        );
        assert_eq!(
            Macro::parse("Inet(\"http://www.Evermore88.com/\")"),
            Some(Macro::Web("http://www.Evermore88.com/".into()))
        );
        assert_eq!(Macro::parse("Exit()"), None);
    }

    #[test]
    fn context_hashes_are_winhelp_s() {
        // Every context string hashes to a value; the empty one to 0.
        assert_eq!(context_hash(""), 0);
        assert_eq!(context_hash("a"), context_hash("A"));
        assert_eq!(context_hash("1"), 1);
        assert_eq!(context_hash("0"), 10);
        assert_eq!(context_hash("_."), 13 * 43 + 12);
    }

    #[test]
    fn lz77_expands_back_references() {
        // "ab" then a reference 2 back of length 4: "ababab".
        let token: u16 = 1 | (1 << 12);
        let src = [0b100, b'a', b'b', token as u8, (token >> 8) as u8];
        assert_eq!(lz77(&src), b"ababab");
    }
}
