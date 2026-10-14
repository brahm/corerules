//! The subset of Rich Text Format the rule books (`Books/*.rtf`) use.
//!
//! The books are Word 97 output: Windows-1252 text, direct formatting
//! (bold, size, colour) rather than styles, tab-aligned tables with
//! `\tx` stops, Word 95 style list numbers in `{\pntext ...}`, and a
//! few embedded pictures (PNG inside `{\*\shppict}`, or a WMF that only
//! wraps a device-independent bitmap). This reader turns a book into
//! [`Paragraph`]s of formatted [`Inline`]s; it is read-only and makes no
//! attempt at the parts of RTF the books never use (tables with
//! `\trowd`, fields, footnotes, sections).
//!
//! What it does with the rest:
//!
//! * `\par` ends a paragraph; `\pard` resets the paragraph properties
//!   (alignment, indents, spacing, tab stops); `\plain` and a closing
//!   `}` reset or restore the character properties.
//! * Source line breaks are not text: Word wraps the file in the middle of
//!   words.
//! * `\'hh` and raw bytes are Windows-1252, except in fonts with
//!   `\fcharset2` (Symbol, Wingdings), which go through small maps
//!   (`\'b7` in Symbol is a bullet).
//! * `\uN` is UTF-16 (negative N wraps), followed by `\ucN` fallback
//!   characters that are skipped.
//! * Hidden text (`\v`) is dropped; so are the font, colour, style and
//!   list tables, document information, headers, footers and every
//!   unknown `{\*\...}` destination.
//! * Negative tab stops (`\tx-720`, in almost every book paragraph) are
//!   ignored.

use std::sync::OnceLock;

/// A colour from the document's colour table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TabKind {
    #[default]
    Left,
    Center,
    Right,
    Decimal,
}

/// A tab stop, `pos` in twips from the left margin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabStop {
    pub pos: i32,
    pub kind: TabKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Script {
    #[default]
    Normal,
    Super,
    Sub,
}

/// Character formatting of a run of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Format {
    /// Size in half-points (`\fs24` = 12 pt).
    pub size: u16,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub small_caps: bool,
    pub script: Script,
    /// `None` for the automatic colour (`\cf0`).
    pub color: Option<Rgb>,
    /// The `\cfN` index, kept because the books' heading conventions are
    /// described by index.
    pub color_index: u16,
    /// Index into [`Document::fonts`].
    pub font: u16,
}

impl Default for Format {
    fn default() -> Self {
        Self {
            size: 24,
            bold: false,
            italic: false,
            underline: false,
            small_caps: false,
            script: Script::Normal,
            color: None,
            color_index: 0,
            font: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureFormat {
    Png,
    Jpeg,
    /// A Windows bitmap file (`BM` header), made from the bitmap inside a
    /// WMF.
    Bmp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub format: PictureFormat,
    pub data: Vec<u8>,
    /// Display size in twips (`\picwgoal` × `\picscalex` %), 0 when the
    /// file does not say.
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String, Format),
    Picture(Picture),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Paragraph {
    pub align: Align,
    /// Indents in twips.
    pub left_indent: i32,
    pub first_indent: i32,
    pub right_indent: i32,
    /// Spacing before and after, in twips.
    pub space_before: i32,
    pub space_after: i32,
    /// Tab stops at positive positions, in file order.
    pub tabs: Vec<TabStop>,
    /// Keep with next (`\keepn`), as Word uses it for captions.
    pub keep_next: bool,
    pub items: Vec<Inline>,
}

impl Paragraph {
    /// The paragraph's text, pictures left out.
    pub fn text(&self) -> String {
        let mut text = String::new();
        for item in &self.items {
            if let Inline::Text(t, _) = item {
                text.push_str(t);
            }
        }
        text
    }

    /// The text runs with their formats.
    pub fn runs(&self) -> impl Iterator<Item = (&str, &Format)> {
        self.items.iter().filter_map(|i| match i {
            Inline::Text(t, f) => Some((t.as_str(), f)),
            Inline::Picture(_) => None,
        })
    }

    pub fn pictures(&self) -> impl Iterator<Item = &Picture> {
        self.items.iter().filter_map(|i| match i {
            Inline::Picture(p) => Some(p),
            Inline::Text(..) => None,
        })
    }

    fn push_text(&mut self, text: &str, format: Format) {
        if let Some(Inline::Text(t, f)) = self.items.last_mut()
            && *f == format
        {
            t.push_str(text);
            return;
        }
        self.items.push(Inline::Text(text.to_owned(), format));
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Font {
    pub name: String,
    /// `\fcharsetN`: 0 ANSI, 2 symbol.
    pub charset: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    pub paragraphs: Vec<Paragraph>,
    /// Indexed by `\fN`; missing numbers are default fonts.
    pub fonts: Vec<Font>,
    /// Indexed by `\cfN`; entry 0 is usually the automatic colour.
    pub colors: Vec<Option<Rgb>>,
}

impl Document {
    /// Parses an RTF file. Malformed input never fails: unbalanced groups
    /// and unknown control words are tolerated as Word and WordPad do.
    pub fn parse(data: &[u8]) -> Self {
        Parser::new(data).run()
    }

    /// The font family of a run, for display.
    pub fn font_name(&self, format: &Format) -> &str {
        self.fonts
            .get(format.font as usize)
            .map_or("", |f| f.name.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dest {
    /// Body text.
    Text,
    /// A destination whose content is dropped.
    Skip,
    FontTable,
    /// Inside one `{\fN ...}` of the font table: the text is its name.
    FontEntry,
    ColorTable,
    /// `{\pict ...}`: control words describe the picture, text is hex.
    Picture,
    /// `{\object ...}`: dropped except for its `{\result ...}`.
    Object,
}

#[derive(Debug, Clone, Copy)]
struct GroupState {
    format: Format,
    dest: Dest,
    hidden: bool,
    /// `\ucN`: fallback characters after `\uN`.
    uc: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct PictState {
    kind: Option<PictureFormat>,
    /// A WMF (`\wmetafileN`), converted to BMP when it wraps a DIB.
    wmf: bool,
    /// A bare device-independent bitmap (`\dibitmap`).
    dib: bool,
    picw: i32,
    pich: i32,
    goal_w: i32,
    goal_h: i32,
    scale_x: i32,
    scale_y: i32,
}

#[derive(Debug, Clone, Default)]
struct ParaState {
    align: Align,
    left_indent: i32,
    first_indent: i32,
    right_indent: i32,
    space_before: i32,
    space_after: i32,
    tabs: Vec<TabStop>,
    pending_tab: TabKind,
    keep_next: bool,
}

struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
    doc: Document,
    state: GroupState,
    stack: Vec<GroupState>,
    para: ParaState,
    /// The paragraph properties saved while inside `{\pntext ...}`, whose
    /// own `\pard` must not reset the paragraph it numbers.
    pntext_saved: Vec<(usize, ParaState)>,
    current: Paragraph,
    /// Set at `{`, cleared by the first token of the group: a destination
    /// keyword must come first.
    group_start: bool,
    /// After `\*`: an unknown destination is skipped.
    starred: bool,
    /// `\uN` fallback characters still to skip.
    skip_chars: u32,
    /// The colour being defined in the colour table.
    rgb: (Option<u8>, Option<u8>, Option<u8>),
    font_number: usize,
    font_name: Vec<u8>,
    pict: PictState,
    hex: Vec<u8>,
    hex_high: Option<u8>,
    /// Depth of the group that opened the current picture.
    pict_depth: usize,
}

impl<'a> Parser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            doc: Document::default(),
            state: GroupState {
                format: Format::default(),
                dest: Dest::Text,
                hidden: false,
                uc: 1,
            },
            stack: Vec::new(),
            para: ParaState::default(),
            pntext_saved: Vec::new(),
            current: Paragraph::default(),
            group_start: false,
            starred: false,
            skip_chars: 0,
            rgb: (None, None, None),
            font_number: 0,
            font_name: Vec::new(),
            pict: PictState::default(),
            hex: Vec::new(),
            hex_high: None,
            pict_depth: 0,
        }
    }

    fn run(mut self) -> Document {
        while self.pos < self.data.len() {
            let byte = self.data[self.pos];
            match byte {
                b'{' => {
                    self.pos += 1;
                    self.stack.push(self.state);
                    self.group_start = true;
                    self.starred = false;
                }
                b'}' => {
                    self.pos += 1;
                    self.close_group();
                }
                b'\\' => self.control(),
                b'\r' | b'\n' => self.pos += 1,
                _ => {
                    let start = self.pos;
                    while self.pos < self.data.len()
                        && !matches!(self.data[self.pos], b'{' | b'}' | b'\\' | b'\r' | b'\n')
                    {
                        self.pos += 1;
                    }
                    self.group_start = false;
                    self.text_bytes(start, self.pos);
                }
            }
        }
        if !self.current.items.is_empty() {
            self.end_paragraph();
        }
        self.doc
    }

    fn close_group(&mut self) {
        let closing = self.state.dest;
        let depth = self.stack.len();
        match closing {
            Dest::FontEntry => self.finish_font(),
            Dest::Picture if depth == self.pict_depth => self.finish_picture(),
            _ => {}
        }
        if let Some(&(d, _)) = self.pntext_saved.last()
            && d == depth
        {
            let (_, saved) = self.pntext_saved.pop().unwrap();
            self.para = saved;
        }
        if let Some(state) = self.stack.pop() {
            // A font entry written without its own group ends at the
            // closing brace of the font table.
            if closing == Dest::FontTable && !self.font_name.is_empty() {
                self.finish_font();
            }
            self.state = state;
        }
        self.group_start = false;
        self.starred = false;
    }

    /// One control word, control symbol or `\'hh` escape.
    fn control(&mut self) {
        let data = self.data;
        self.pos += 1;
        let Some(&c) = data.get(self.pos) else {
            return;
        };
        if c.is_ascii_alphabetic() {
            let start = self.pos;
            while self.pos < data.len() && data[self.pos].is_ascii_alphabetic() {
                self.pos += 1;
            }
            let word = &data[start..self.pos];
            let mut param = None;
            let num_start = self.pos;
            if self.pos < data.len() && data[self.pos] == b'-' {
                self.pos += 1;
            }
            while self.pos < data.len() && data[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if self.pos > num_start && data[num_start..self.pos] != *b"-" {
                param = std::str::from_utf8(&data[num_start..self.pos])
                    .ok()
                    .and_then(|s| s.parse::<i64>().ok())
                    .map(|n| n.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
            } else {
                self.pos = num_start;
            }
            if self.pos < data.len() && data[self.pos] == b' ' {
                self.pos += 1;
            }
            let first = std::mem::take(&mut self.group_start);
            let starred = std::mem::take(&mut self.starred);
            // `\bin N`: N raw bytes follow.
            if word == b"bin" {
                self.pos = (self.pos + param.unwrap_or(0).max(0) as usize).min(data.len());
                return;
            }
            self.word(word, param, first, starred);
            return;
        }
        self.pos += 1;
        match c {
            b'\'' => {
                let hex = data.get(self.pos..self.pos + 2).and_then(|h| {
                    std::str::from_utf8(h)
                        .ok()
                        .and_then(|h| u8::from_str_radix(h, 16).ok())
                });
                if let Some(value) = hex {
                    self.pos += 2;
                    self.group_start = false;
                    if self.skip_chars > 0 {
                        self.skip_chars -= 1;
                        return;
                    }
                    self.byte_char(value);
                }
            }
            b'*' => {
                self.starred = true;
            }
            b'\r' | b'\n' => {
                // `\` before a line break is a paragraph mark.
                self.group_start = false;
                if self.in_text() {
                    self.end_paragraph();
                }
            }
            _ => {
                self.group_start = false;
                let text = match c {
                    b'~' => "\u{a0}",
                    b'_' => "\u{2011}",
                    b'-' => "",
                    b'\\' => "\\",
                    b'{' => "{",
                    b'}' => "}",
                    b'\t' => "\t",
                    _ => "",
                };
                match self.state.dest {
                    Dest::FontEntry if matches!(c, b'\\' | b'{' | b'}') => self.font_name.push(c),
                    _ if !text.is_empty() => self.emit(text),
                    _ => {}
                }
            }
        }
    }

    fn in_text(&self) -> bool {
        self.state.dest == Dest::Text
    }

    fn word(&mut self, word: &[u8], param: Option<i32>, first: bool, starred: bool) {
        let n = param.unwrap_or(1);
        let on = param != Some(0);
        match self.state.dest {
            Dest::Skip => return,
            Dest::FontTable => {
                if word == b"f" {
                    if !self.font_name.is_empty() {
                        self.finish_font();
                    }
                    self.font_number = n.max(0) as usize;
                    if first {
                        self.state.dest = Dest::FontEntry;
                    }
                }
                return;
            }
            Dest::FontEntry => {
                match word {
                    b"f" => self.font_number = n.max(0) as usize,
                    b"fcharset" => self.set_charset(n),
                    // `{\*\panose ...}` and `{\*\falt ...}` are their own
                    // groups; skip them.
                    _ if first => self.state.dest = Dest::Skip,
                    _ => {}
                }
                return;
            }
            Dest::ColorTable => {
                let v = Some(n.clamp(0, 255) as u8);
                match word {
                    b"red" => self.rgb.0 = v,
                    b"green" => self.rgb.1 = v,
                    b"blue" => self.rgb.2 = v,
                    _ => {}
                }
                return;
            }
            Dest::Picture => {
                if first && word != b"pict" {
                    // `{\*\picprop ...}`, `{\*\blipuid ...}`.
                    self.state.dest = Dest::Skip;
                    return;
                }
                self.picture_word(word, n);
                return;
            }
            Dest::Object => {
                if first && word == b"result" {
                    self.state.dest = Dest::Text;
                } else if first {
                    self.state.dest = Dest::Skip;
                }
                return;
            }
            Dest::Text => {}
        }

        if first {
            match word {
                b"fonttbl" => {
                    self.state.dest = Dest::FontTable;
                    return;
                }
                b"colortbl" => {
                    self.state.dest = Dest::ColorTable;
                    return;
                }
                b"pict" => {
                    self.state.dest = Dest::Picture;
                    self.pict = PictState::default();
                    self.hex.clear();
                    self.hex_high = None;
                    self.pict_depth = self.stack.len();
                    return;
                }
                b"object" => {
                    self.state.dest = Dest::Object;
                    return;
                }
                // Word writes the picture twice: PNG here, a WMF copy in
                // `\nonshppict` for older readers.
                b"shppict" => return,
                b"pntext" => {
                    self.pntext_saved
                        .push((self.stack.len(), self.para.clone()));
                    return;
                }
                b"stylesheet"
                | b"info"
                | b"listtable"
                | b"listoverridetable"
                | b"revtbl"
                | b"header"
                | b"headerl"
                | b"headerr"
                | b"headerf"
                | b"footer"
                | b"footerl"
                | b"footerr"
                | b"footerf"
                | b"pn"
                | b"pnseclvl"
                | b"nonshppict"
                | b"filetbl"
                | b"rsidtbl"
                | b"objdata"
                | b"footnote"
                | b"annotation"
                | b"atnid"
                | b"fldinst"
                | b"xe"
                | b"tc"
                | b"bkmkstart"
                | b"bkmkend"
                | b"template"
                | b"generator"
                | b"latentstyles"
                | b"themedata"
                | b"colorschememapping"
                | b"datastore"
                | b"xmlnstbl"
                | b"listpicture"
                | b"pgdsctbl" => {
                    self.state.dest = Dest::Skip;
                    return;
                }
                _ if starred => {
                    self.state.dest = Dest::Skip;
                    return;
                }
                _ => {}
            }
        }

        let format = &mut self.state.format;
        match word {
            b"par" | b"sect" | b"page" => self.end_paragraph(),
            b"line" => self.emit("\n"),
            b"tab" => self.emit("\t"),
            b"emdash" => self.emit("\u{2014}"),
            b"endash" => self.emit("\u{2013}"),
            b"lquote" => self.emit("\u{2018}"),
            b"rquote" => self.emit("\u{2019}"),
            b"ldblquote" => self.emit("\u{201c}"),
            b"rdblquote" => self.emit("\u{201d}"),
            b"bullet" => self.emit("\u{2022}"),
            b"emspace" | b"enspace" | b"qmspace" => self.emit(" "),
            b"u" => {
                let unit = (param.unwrap_or(0) as i64).rem_euclid(65536) as u32;
                let c = char::from_u32(unit).unwrap_or('\u{fffd}');
                let mut buf = [0u8; 4];
                self.emit(c.encode_utf8(&mut buf));
                self.skip_chars = self.state.uc;
            }
            b"uc" => self.state.uc = param.unwrap_or(1).max(0) as u32,
            b"pard" => {
                self.para = ParaState::default();
            }
            b"plain" => {
                *format = Format::default();
                self.state.hidden = false;
            }
            b"b" => format.bold = on,
            b"i" => format.italic = on,
            b"ul" | b"uldb" | b"ulw" | b"uld" | b"uldash" | b"ulth" | b"ulwave" => {
                format.underline = on
            }
            b"ulnone" => format.underline = false,
            b"scaps" => format.small_caps = on,
            b"super" => format.script = Script::Super,
            b"sub" => format.script = Script::Sub,
            b"nosupersub" => format.script = Script::Normal,
            b"up" => format.script = if n > 0 { Script::Super } else { Script::Normal },
            b"dn" => format.script = if n > 0 { Script::Sub } else { Script::Normal },
            b"fs" => format.size = param.unwrap_or(24).clamp(1, 4000) as u16,
            b"f" => format.font = n.clamp(0, u16::MAX as i32) as u16,
            b"cf" => {
                let index = n.max(0) as usize;
                format.color_index = index as u16;
                format.color = self.doc.colors.get(index).copied().flatten();
            }
            b"v" => self.state.hidden = on,
            b"ql" => self.para.align = Align::Left,
            b"qc" => self.para.align = Align::Center,
            b"qr" => self.para.align = Align::Right,
            b"qj" => self.para.align = Align::Justify,
            b"li" => self.para.left_indent = n,
            b"fi" => self.para.first_indent = n,
            b"ri" => self.para.right_indent = n,
            b"sb" => self.para.space_before = n,
            b"sa" => self.para.space_after = n,
            b"keepn" => self.para.keep_next = on,
            b"tqc" => self.para.pending_tab = TabKind::Center,
            b"tqr" => self.para.pending_tab = TabKind::Right,
            b"tqdec" => self.para.pending_tab = TabKind::Decimal,
            b"tx" | b"tb" => {
                let kind = std::mem::take(&mut self.para.pending_tab);
                if n > 0 {
                    self.para.tabs.push(TabStop { pos: n, kind });
                }
            }
            _ => {}
        }
    }

    fn set_charset(&mut self, n: i32) {
        let font = self.font_entry();
        font.charset = n.clamp(0, 255) as u8;
    }

    fn font_entry(&mut self) -> &mut Font {
        let index = self.font_number.min(4096);
        if self.doc.fonts.len() <= index {
            self.doc.fonts.resize(index + 1, Font::default());
        }
        &mut self.doc.fonts[index]
    }

    fn finish_font(&mut self) {
        let raw = std::mem::take(&mut self.font_name);
        let name: String = raw
            .iter()
            .map(|&b| cp1252(b))
            .collect::<String>()
            .trim()
            .trim_end_matches(';')
            .trim()
            .to_owned();
        if !name.is_empty() {
            self.font_entry().name = name;
        }
    }

    fn picture_word(&mut self, word: &[u8], n: i32) {
        let p = &mut self.pict;
        match word {
            b"pngblip" => p.kind = Some(PictureFormat::Png),
            b"jpegblip" => p.kind = Some(PictureFormat::Jpeg),
            b"wmetafile" => p.wmf = true,
            b"dibitmap" => p.dib = true,
            b"picw" => p.picw = n,
            b"pich" => p.pich = n,
            b"picwgoal" => p.goal_w = n,
            b"pichgoal" => p.goal_h = n,
            b"picscalex" => p.scale_x = n,
            b"picscaley" => p.scale_y = n,
            _ => {}
        }
    }

    fn finish_picture(&mut self) {
        let hex = std::mem::take(&mut self.hex);
        let p = self.pict;
        if self.state.hidden || hex.is_empty() {
            return;
        }
        let picture = match (p.kind, p.wmf, p.dib) {
            (Some(format), _, _) => Some((format, hex)),
            (None, true, _) => wmf_bitmap(&hex).map(|bmp| (PictureFormat::Bmp, bmp)),
            (None, false, true) => Some((PictureFormat::Bmp, dib_to_bmp(&hex))),
            _ => None,
        };
        let Some((format, data)) = picture else {
            return;
        };
        let scale = |goal: i32, s: i32| {
            if s > 0 {
                (goal as i64 * s as i64 / 100) as i32
            } else {
                goal
            }
        };
        // Leave the picture where its group sits in the text; the parent
        // destination is the text the picture belongs to.
        let parent = self.stack.last().map_or(Dest::Text, |s| s.dest);
        if parent != Dest::Text && parent != Dest::Picture {
            return;
        }
        self.current.items.push(Inline::Picture(Picture {
            format,
            data,
            width: scale(p.goal_w, p.scale_x),
            height: scale(p.goal_h, p.scale_y),
        }));
    }

    fn text_bytes(&mut self, start: usize, end: usize) {
        match self.state.dest {
            Dest::Skip | Dest::FontTable | Dest::Object => {}
            Dest::FontEntry => self.font_name.extend_from_slice(&self.data[start..end]),
            Dest::ColorTable => {
                for &b in &self.data[start..end] {
                    if b == b';' {
                        let color = match self.rgb {
                            (None, None, None) => None,
                            (r, g, b) => Some(Rgb(r.unwrap_or(0), g.unwrap_or(0), b.unwrap_or(0))),
                        };
                        self.doc.colors.push(color);
                        self.rgb = (None, None, None);
                    }
                }
            }
            Dest::Picture => {
                for i in start..end {
                    let Some(v) = (self.data[i] as char).to_digit(16) else {
                        continue;
                    };
                    match self.hex_high.take() {
                        Some(high) => self.hex.push(high << 4 | v as u8),
                        None => self.hex_high = Some(v as u8),
                    }
                }
            }
            Dest::Text => {
                let mut start = start;
                while self.skip_chars > 0 && start < end {
                    self.skip_chars -= 1;
                    start += 1;
                }
                if start == end {
                    return;
                }
                let bytes = &self.data[start..end];
                let text: String = if self.symbol_font().is_some() {
                    bytes.iter().map(|&b| self.map_byte(b)).collect()
                } else {
                    bytes.iter().map(|&b| cp1252(b)).collect()
                };
                self.emit(&text);
            }
        }
    }

    /// `\'hh`.
    fn byte_char(&mut self, value: u8) {
        match self.state.dest {
            Dest::FontEntry => self.font_name.push(value),
            Dest::Text => {
                let c = self.map_byte(value);
                let mut buf = [0u8; 4];
                self.emit(c.encode_utf8(&mut buf));
            }
            _ => {}
        }
    }

    /// Whether the current font is a symbol font, and whether it is
    /// Wingdings.
    fn symbol_font(&self) -> Option<bool> {
        let font = self.doc.fonts.get(self.state.format.font as usize)?;
        (font.charset == 2).then(|| font.name.contains("Wingdings"))
    }

    fn map_byte(&self, b: u8) -> char {
        match self.symbol_font() {
            Some(true) => wingdings(b).unwrap_or_else(|| cp1252(b)),
            Some(false) => symbol(b).unwrap_or_else(|| cp1252(b)),
            None => cp1252(b),
        }
    }

    fn emit(&mut self, text: &str) {
        if self.state.dest != Dest::Text || self.state.hidden {
            return;
        }
        let format = self.state.format;
        self.current.push_text(text, format);
    }

    fn end_paragraph(&mut self) {
        if self.state.dest != Dest::Text {
            return;
        }
        let mut paragraph = std::mem::take(&mut self.current);
        let p = &self.para;
        paragraph.align = p.align;
        paragraph.left_indent = p.left_indent;
        paragraph.first_indent = p.first_indent;
        paragraph.right_indent = p.right_indent;
        paragraph.space_before = p.space_before;
        paragraph.space_after = p.space_after;
        paragraph.tabs = p.tabs.clone();
        paragraph.keep_next = p.keep_next;
        self.doc.paragraphs.push(paragraph);
    }
}

/// Windows-1252, with the five unassigned bytes as their C1 controls.
pub fn cp1252(b: u8) -> char {
    static TABLE: OnceLock<[char; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = ['\0'; 256];
        let bytes: Vec<u8> = (0..=255).collect();
        let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
        for (slot, c) in table.iter_mut().zip(text.chars()) {
            *slot = c;
        }
        table
    })[b as usize]
}

/// The Symbol font (`\fcharset2`) to Unicode, for the characters the
/// books use and the usual Greek letters.
fn symbol(b: u8) -> Option<char> {
    const UPPER: &str = "ΑΒΧΔΕΦΓΗΙϑΚΛΜΝΟΠΘΡΣΤΥςΩΞΨΖ";
    const LOWER: &str = "αβχδεφγηιϕκλμνοπθρστυϖωξψζ";
    Some(match b {
        0xb7 => '\u{2022}',
        0xa7 => '\u{2663}',
        0xae => '\u{2192}',
        0xb0 => '\u{b0}',
        0xb1 => '\u{b1}',
        0xb4 => '\u{d7}',
        0xb8 => '\u{f7}',
        0xa3 => '\u{2264}',
        0xb3 => '\u{2265}',
        0xb9 => '\u{2260}',
        0xd6 => '\u{221a}',
        0x2d => '\u{2212}',
        0x41..=0x5a => UPPER.chars().nth((b - 0x41) as usize)?,
        0x61..=0x7a => LOWER.chars().nth((b - 0x61) as usize)?,
        _ => return None,
    })
}

fn wingdings(b: u8) -> Option<char> {
    Some(match b {
        0xa7 => '\u{25aa}',
        0x6c => '\u{25cf}',
        0x6e => '\u{25a0}',
        0xfc => '\u{2714}',
        0xd8 => '\u{27a2}',
        _ => return None,
    })
}

/// The bitmap inside a WMF whose drawing is one `META_STRETCHDIB`
/// (0x0f43) or `META_DIBSTRETCHBLT` (0x0b41) record, as a BMP file.
pub fn wmf_bitmap(wmf: &[u8]) -> Option<Vec<u8>> {
    let u16_at = |o: usize| wmf.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |o: usize| {
        wmf.get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    // Optional placeable header (0x9ac6cdd7), then the 18-byte header
    // whose second word is its size in words.
    let mut offset = if u32_at(0)? == 0x9ac6_cdd7 { 22 } else { 0 };
    offset += u16_at(offset + 2)? as usize * 2;
    while let (Some(size), Some(function)) = (u32_at(offset), u16_at(offset + 4)) {
        let size = size as usize * 2;
        if size < 6 || offset + size > wmf.len() {
            break;
        }
        let params = match function {
            0x0f43 => Some(22),
            0x0b41 => Some(20),
            _ => None,
        };
        if let Some(params) = params {
            let dib = &wmf[offset + 6 + params..offset + size];
            return Some(dib_to_bmp(dib));
        }
        offset += size;
    }
    None
}

/// Puts a 14-byte `BITMAPFILEHEADER` in front of a packed DIB.
pub fn dib_to_bmp(dib: &[u8]) -> Vec<u8> {
    let u32_at = |o: usize| {
        dib.get(o..o + 4)
            .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let u16_at = |o: usize| {
        dib.get(o..o + 2)
            .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
    };
    let header = u32_at(0);
    let bits = u16_at(14) as u32;
    let used = u32_at(32);
    let colors = if used != 0 {
        used
    } else if bits <= 8 {
        1 << bits
    } else {
        0
    };
    // BI_BITFIELDS keeps three masks after a 40-byte header.
    let masks = if header == 40 && u32_at(16) == 3 {
        12
    } else {
        0
    };
    let offset = 14 + header + masks + 4 * colors;
    let mut bmp = Vec::with_capacity(14 + dib.len());
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(14 + dib.len() as u32).to_le_bytes());
    bmp.extend_from_slice(&[0, 0, 0, 0]);
    bmp.extend_from_slice(&offset.to_le_bytes());
    bmp.extend_from_slice(dib);
    bmp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(rtf: &str) -> Document {
        Document::parse(rtf.as_bytes())
    }

    fn texts(d: &Document) -> Vec<String> {
        d.paragraphs.iter().map(Paragraph::text).collect()
    }

    const HEADER: &str = r"{\rtf1\ansi\ansicpg1252\uc1\deff0{\fonttbl{\f0\froman\fcharset0{\*\panose 02020603050405020304}Times New Roman;}{\f3\froman\fcharset2 Symbol;}{\f5\fnil\fcharset2 Wingdings;}}{\colortbl;\red0\green0\blue0;\red255\green0\blue0;}{\stylesheet{\s1 heading 1;}}{\info{\title Secret}}";

    #[test]
    fn paragraphs_and_line_breaks_in_the_source() {
        let d = doc(&format!(
            "{HEADER}\\pard Each c\r\nharacter\\par Second\\par}}"
        ));
        assert_eq!(texts(&d), ["Each character", "Second"]);
        assert_eq!(d.fonts[0].name, "Times New Roman");
        assert_eq!(d.fonts[3].charset, 2);
        assert_eq!(d.colors, [None, Some(Rgb(0, 0, 0)), Some(Rgb(255, 0, 0))]);
    }

    #[test]
    fn character_formatting_is_scoped_by_groups() {
        let d = doc(&format!(
            "{HEADER}{{\\b\\fs36\\cf2 Big {{\\i red}}\\par }}plain {{\\ul u}}{{\\scaps s}}x{{\\super 2}}\\plain\\fs20 small\\par}}"
        ));
        let p = &d.paragraphs[0];
        let runs: Vec<_> = p.runs().collect();
        assert_eq!(runs[0].0, "Big ");
        assert!(runs[0].1.bold && runs[0].1.size == 36);
        assert_eq!(runs[0].1.color, Some(Rgb(255, 0, 0)));
        assert_eq!(runs[0].1.color_index, 2);
        assert_eq!(runs[1].0, "red");
        assert!(runs[1].1.italic && runs[1].1.bold);
        let q: Vec<_> = d.paragraphs[1].runs().collect();
        assert_eq!(q[0].0, "plain ");
        assert!(!q[0].1.bold && q[0].1.size == 24);
        assert!(q[1].1.underline);
        assert!(q[2].1.small_caps);
        assert_eq!(q[4].1.script, Script::Super);
        assert_eq!(q[5].0, "small");
        assert_eq!(q[5].1.size, 20);
    }

    #[test]
    fn paragraph_properties_and_tab_stops() {
        let d = doc(&format!(
            "{HEADER}\\pard\\qc\\li720\\fi-360\\ri100\\sb60\\sa120\\keepn\\tx-720\\tx1440\\tqr\\tx2880 A\\tab B\\par \\pard C\\par}}"
        ));
        let p = &d.paragraphs[0];
        assert_eq!(p.align, Align::Center);
        assert_eq!(
            (p.left_indent, p.first_indent, p.right_indent),
            (720, -360, 100)
        );
        assert_eq!((p.space_before, p.space_after), (60, 120));
        assert!(p.keep_next);
        assert_eq!(
            p.tabs,
            [
                TabStop {
                    pos: 1440,
                    kind: TabKind::Left
                },
                TabStop {
                    pos: 2880,
                    kind: TabKind::Right
                }
            ]
        );
        assert_eq!(p.text(), "A\tB");
        let q = &d.paragraphs[1];
        assert_eq!(q.align, Align::Left);
        assert!(q.tabs.is_empty() && !q.keep_next);
    }

    #[test]
    fn windows_1252_symbols_and_unicode() {
        let d = doc(&format!(
            "{HEADER}\\'86 \\'99 caf\\'e9\\~x\\_y\\-z \\emdash\\endash\\lquote\\rquote\\ldblquote\\rdblquote\\bullet\\par{{\\f3 \\'b7 a}}\\par{{\\f5 \\'a7}}\\par\\u-3913 ?ok\\uc2\\u8364 xyz\\par\\{{\\}}\\\\\\par}}"
        ));
        assert_eq!(
            texts(&d),
            [
                "\u{2020} \u{2122} café\u{a0}x\u{2011}yz \u{2014}\u{2013}\u{2018}\u{2019}\u{201c}\u{201d}\u{2022}",
                "\u{2022} α",
                "\u{25aa}",
                "\u{f0b7}ok\u{20ac}z",
                "{}\\",
            ]
        );
    }

    #[test]
    fn hidden_text_and_skipped_destinations() {
        let d = doc(&format!(
            "{HEADER}{{\\*\\listtable{{\\list x}}}}{{\\header head}}{{\\*\\unknown z}}{{\\uldb Table 6}}{{\\v Table_6_TOPIC}} shown\\par}}"
        ));
        assert_eq!(texts(&d), ["Table 6 shown"]);
    }

    #[test]
    fn list_numbers_keep_the_paragraph_properties() {
        let d = doc(&format!(
            "{HEADER}\\pard\\li720\\fi-360{{\\pntext\\pard\\plain\\f3\\fs20 \\'b7\\tab}}{{\\*\\pn \\pnlvlblt{{\\pntxtb x}}}}Item\\par}}"
        ));
        assert_eq!(texts(&d), ["\u{2022}\tItem"]);
        assert_eq!(d.paragraphs[0].left_indent, 720);
        assert_eq!(d.paragraphs[0].runs().next().unwrap().1.size, 20);
    }

    #[test]
    fn png_pictures_and_the_duplicate_metafile() {
        let png = "89504e470d0a1a0a0000";
        let d = doc(&format!(
            "{HEADER}A{{\\*\\shppict{{\\pict{{\\*\\picprop\\shplid1{{\\sp{{\\sn x}}}}}}\\picscalex50\\picscaley200\\picw10\\pich10\\picwgoal1440\\pichgoal720\\pngblip\r\n{png}\r\n}}}}{{\\nonshppict{{\\pict\\wmetafile8 0102}}}}B\\par}}"
        ));
        let p = &d.paragraphs[0];
        assert_eq!(p.text(), "AB");
        let pics: Vec<_> = p.pictures().collect();
        assert_eq!(pics.len(), 1);
        assert_eq!(pics[0].format, PictureFormat::Png);
        assert_eq!(pics[0].data[..4], [0x89, b'P', b'N', b'G']);
        assert_eq!((pics[0].width, pics[0].height), (720, 1440));
        assert!(matches!(p.items[1], Inline::Picture(_)));
    }

    /// A WMF with one META_STRETCHDIB record holding a 1×1 8-bit DIB.
    fn tiny_wmf() -> Vec<u8> {
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&1i32.to_le_bytes());
        dib.extend_from_slice(&1i32.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&8u16.to_le_bytes());
        dib.extend_from_slice(&[0; 16]);
        dib.extend_from_slice(&2u32.to_le_bytes()); // colours used
        dib.extend_from_slice(&[0; 4]);
        dib.extend_from_slice(&[0, 0, 0, 0, 255, 255, 255, 0]);
        dib.extend_from_slice(&[1, 0, 0, 0]);
        let mut record = Vec::new();
        let words = (6 + 22 + dib.len()) / 2;
        record.extend_from_slice(&(words as u32).to_le_bytes());
        record.extend_from_slice(&0x0f43u16.to_le_bytes());
        record.extend_from_slice(&[0; 22]);
        record.extend_from_slice(&dib);
        let mut wmf = vec![1, 0, 9, 0, 0, 3];
        wmf.extend_from_slice(&[0; 12]);
        wmf.extend_from_slice(&record);
        wmf.extend_from_slice(&[3, 0, 0, 0, 0, 0]);
        wmf
    }

    #[test]
    fn metafile_bitmaps_become_bmp_files() {
        let hex: String = tiny_wmf().iter().map(|b| format!("{b:02x}")).collect();
        let d = doc(&format!(
            "{HEADER}{{\\object\\objemb{{\\*\\objclass Word.Picture.8}}{{\\*\\objdata 0102}}{{\\result{{\\pict\\wmetafile8\\picwgoal300\\pichgoal300 {hex}}}}}}}\\par}}"
        ));
        let pic = d.paragraphs[0].pictures().next().expect("picture");
        assert_eq!(pic.format, PictureFormat::Bmp);
        assert_eq!(&pic.data[..2], b"BM");
        let offset = u32::from_le_bytes(pic.data[10..14].try_into().unwrap());
        assert_eq!(offset, 14 + 40 + 8);
        assert_eq!(pic.data[offset as usize], 1);
        assert_eq!(pic.width, 300);
    }

    #[test]
    fn malformed_input_does_not_panic() {
        for input in [
            "",
            "{",
            "}}}",
            "{\\rtf1 \\'",
            "{\\rtf1 \\'zz",
            "\\",
            "{\\pict 1}",
            "{\\u",
            "{\\fonttbl\\f",
        ] {
            let _ = Document::parse(input.as_bytes());
        }
        assert_eq!(texts(&doc("{\\rtf1 unbalanced")), ["unbalanced"]);
    }
}
