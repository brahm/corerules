//! The character sheet as a PDF, written here so that the GNOME and the
//! Windows app save the same file: the pages of [`crate::pages`], drawn
//! as both apps print them, in the PDF's built-in Helvetica. A built-in
//! font needs no font file in the program or in the PDF, and its
//! WinAnsiEncoding holds every character of the original's data and of
//! the app's six languages; the few others the sheet uses have stand-ins.

mod helvetica;

use std::fmt::Write as _;
use std::io::Write as _;

use cr_store::sheet::{Border, Sheet};
use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::pages::{
    self, FONT_SIZE, FOOTER, FRAME_PAD, Measure, Page, Picture, Printed, TITLE_SIZE, cell_width,
    column_starts, frame_pen,
};

/// The paper the PDF is laid out for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paper {
    A4,
    Letter,
}

impl Paper {
    /// Width and height in points.
    fn size(self) -> (f64, f64) {
        match self {
            Paper::A4 => (595.28, 841.89),
            Paper::Letter => (612.0, 792.0),
        }
    }
}

/// The margin on every side, in points: half an inch.
const MARGIN: f64 = 36.0;
/// A line's height and the top of a line to its baseline, in font sizes:
/// Helvetica's bounding box, 931 above the baseline and 225 below.
const LINE: f64 = 1.156;
const ASCENT: f64 = 0.931;
/// The gray of a table's header cells.
const HEADER_GRAY: f64 = 0.35;

/// The sheet as a PDF on `paper`.
pub fn sheet(sheet: &Sheet, paper: Paper) -> Vec<u8> {
    let (paper_width, paper_height) = paper.size();
    let (width, height) = (paper_width - 2.0 * MARGIN, paper_height - 2.0 * MARGIN);
    let laid = pages::items(sheet, &Font::TEXT, width);
    let placed = pages::paginate(&laid, height - FOOTER);

    let mut pdf = Objects::default();
    let catalog = pdf.reserve();
    let tree = pdf.reserve();
    let text = pdf.add(font_object("Helvetica"));
    let bold = pdf.add(font_object("Helvetica-Bold"));
    let mut kids = Vec::new();
    for (number, page) in placed.iter().enumerate() {
        let mut painter = Painter {
            content: String::new(),
            pictures: Vec::new(),
            page_height: paper_height,
            width,
            border: sheet.border,
        };
        painter.page(
            &laid,
            page,
            height,
            &format!("{} · {}", sheet.title, number + 1),
        );
        let mut pictures = String::new();
        for (i, picture) in painter.pictures.iter().enumerate() {
            let id = image(&mut pdf, picture);
            let _ = write!(pictures, " /Im{i} {id} 0 R");
        }
        let content = pdf.add(stream("", painter.content.as_bytes()));
        kids.push(
            pdf.add(
                format!(
                    "<< /Type /Page /Parent {tree} 0 R /Contents {content} 0 R \
                 /Resources << /Font << /F1 {text} 0 R /F2 {bold} 0 R >> \
                 /XObject <<{pictures} >> >> >>"
                )
                .into_bytes(),
            ),
        );
    }
    let kids: Vec<String> = kids.iter().map(|id| format!("{id} 0 R")).collect();
    pdf.set(
        tree,
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} /MediaBox [0 0 {} {}] >>",
            kids.join(" "),
            kids.len(),
            number(paper_width),
            number(paper_height)
        )
        .into_bytes(),
    );
    pdf.set(
        catalog,
        format!("<< /Type /Catalog /Pages {tree} 0 R >>").into_bytes(),
    );
    let info = pdf.add(
        format!(
            "<< /Title {} /Producer (Core Rules {}) >>",
            text_string(&sheet.title),
            env!("CARGO_PKG_VERSION")
        )
        .into_bytes(),
    );
    pdf.finish(catalog, info)
}

/// A font object of one of the built-in fonts.
fn font_object(name: &str) -> Vec<u8> {
    format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} /Encoding /WinAnsiEncoding >>")
        .into_bytes()
}

/// A picture as an image, with its transparency as a soft mask when it
/// has any; its id.
fn image(pdf: &mut Objects, picture: &Picture) -> usize {
    let (w, h) = picture.pixels;
    let rgb: Vec<u8> = picture
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let alpha: Vec<u8> = picture
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p[3])
        .collect();
    let mask = if alpha.iter().all(|&a| a == 255) {
        String::new()
    } else {
        let id = pdf.add(stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width {w} /Height {h} \
                 /ColorSpace /DeviceGray /BitsPerComponent 8"
            ),
            &alpha,
        ));
        format!(" /SMask {id} 0 R")
    };
    pdf.add(stream(
        &format!(
            "/Type /XObject /Subtype /Image /Width {w} /Height {h} \
             /ColorSpace /DeviceRGB /BitsPerComponent 8{mask}"
        ),
        &rgb,
    ))
}

/// A compressed stream object with the entries `dictionary` besides its
/// length and filter.
fn stream(dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    let _ = encoder.write_all(data);
    let data = encoder.finish().unwrap_or_default();
    let mut out = format!(
        "<< {dictionary} /Length {} /Filter /FlateDecode >>\nstream\n",
        data.len()
    )
    .into_bytes();
    out.extend_from_slice(&data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// A number as the PDF writes it: at most two decimals, no trailing
/// zeros.
fn number(value: f64) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".into()
    } else {
        text.into()
    }
}

/// A text string outside the page's content, as the document's title: in
/// UTF-16 with its byte order mark.
fn text_string(text: &str) -> String {
    let mut out = String::from("<FEFF");
    for unit in text.encode_utf16() {
        let _ = write!(out, "{unit:04X}");
    }
    out.push('>');
    out
}

/// The PDF's objects by number, from 1.
#[derive(Default)]
struct Objects {
    bodies: Vec<Vec<u8>>,
}

impl Objects {
    /// A number for an object written later.
    fn reserve(&mut self) -> usize {
        self.bodies.push(Vec::new());
        self.bodies.len()
    }

    fn add(&mut self, body: Vec<u8>) -> usize {
        self.bodies.push(body);
        self.bodies.len()
    }

    fn set(&mut self, id: usize, body: Vec<u8>) {
        self.bodies[id - 1] = body;
    }

    /// The file: the objects, their cross-reference table and the
    /// trailer.
    fn finish(self, root: usize, info: usize) -> Vec<u8> {
        let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let mut offsets = Vec::new();
        for (i, body) in self.bodies.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1);
        for offset in offsets {
            let _ = writeln!(table, "{offset:010} 00000 n ");
        }
        let _ = write!(
            table,
            "trailer\n<< /Size {} /Root {root} 0 R /Info {info} 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            self.bodies.len() + 1
        );
        out.extend_from_slice(table.as_bytes());
        out
    }
}

/// The text in WinAnsiEncoding, with a stand-in for each character it
/// does not have.
fn encode(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        match win_ansi(c) {
            Some(byte) => out.push(byte),
            None => out.extend_from_slice(stand_in(c).as_bytes()),
        }
    }
    out
}

/// The character's code in WinAnsiEncoding.
fn win_ansi(c: char) -> Option<u8> {
    let code = u32::from(c);
    match code {
        0x20..=0x7e | 0xa0..=0xff => Some(code as u8),
        _ => Some(match c {
            '€' => 0x80,
            '‚' => 0x82,
            'ƒ' => 0x83,
            '„' => 0x84,
            '…' => 0x85,
            '†' => 0x86,
            '‡' => 0x87,
            'ˆ' => 0x88,
            '‰' => 0x89,
            'Š' => 0x8a,
            '‹' => 0x8b,
            'Œ' => 0x8c,
            'Ž' => 0x8e,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            '˜' => 0x98,
            '™' => 0x99,
            'š' => 0x9a,
            '›' => 0x9b,
            'œ' => 0x9c,
            'ž' => 0x9e,
            'Ÿ' => 0x9f,
            _ => return None,
        }),
    }
}

/// What stands in for a character WinAnsiEncoding lacks.
fn stand_in(c: char) -> &'static str {
    match c {
        '\t' => " ",
        '′' => "'",
        '″' => "\"",
        '−' | '‐' | '‑' | '‒' => "-",
        '⁄' => "/",
        '≤' => "<=",
        '≥' => ">=",
        '≠' => "/=",
        '→' => "->",
        '←' => "<-",
        '\u{200b}' | '\u{feff}' => "",
        _ => "?",
    }
}

/// One of the sheet's two fonts at its size.
#[derive(Clone, Copy)]
struct Font {
    widths: &'static [u16; 224],
    size: f64,
    /// Its name in a page's resources.
    resource: &'static str,
}

impl Font {
    /// The text of the sheet, as the GNOME app's "Sans 8".
    const TEXT: Font = Font {
        widths: &helvetica::HELVETICA,
        size: FONT_SIZE,
        resource: "F1",
    };
    /// The boxes' titles, as "Sans Bold 10".
    const TITLE: Font = Font {
        widths: &helvetica::HELVETICA_BOLD,
        size: TITLE_SIZE,
        resource: "F2",
    };

    fn width(self, text: &[u8]) -> f64 {
        let units: u32 = text
            .iter()
            .map(|&b| {
                b.checked_sub(32)
                    .map_or(0, |i| u32::from(self.widths[usize::from(i)]))
            })
            .sum();
        f64::from(units) * self.size / 1000.0
    }

    fn line_height(self) -> f64 {
        LINE * self.size
    }

    /// The lines `text` takes, each with its width: one per line of the
    /// text, and with `wrap` broken between words to fit it, or inside a
    /// word too long for a line of its own. A line ends at `\n`, `\r\n`
    /// (as in the original's files) or `\r`, as Pango and DirectWrite
    /// take them.
    fn lines(self, text: &str, wrap: Option<f64>) -> Vec<(Vec<u8>, f64)> {
        let mut out = Vec::new();
        let paragraphs = text
            .split('\n')
            .flat_map(|line| line.strip_suffix('\r').unwrap_or(line).split('\r'));
        for paragraph in paragraphs {
            let bytes = encode(paragraph);
            let Some(wrap) = wrap else {
                out.push((bytes.clone(), self.width(&bytes)));
                continue;
            };
            let mut line: Vec<u8> = Vec::new();
            for word in bytes.split_inclusive(|&b| b == b' ') {
                let mut word = word.to_vec();
                if self.width(trim(&[&line[..], &word[..]].concat())) <= wrap {
                    line.extend_from_slice(&word);
                    continue;
                }
                if !trim(&line).is_empty() {
                    out.push(self.finished(&line));
                    line.clear();
                }
                // A word wider than the line goes on in pieces.
                while self.width(trim(&word)) > wrap && word.len() > 1 {
                    let mut fit = 1;
                    while fit < word.len() && self.width(&word[..=fit]) <= wrap {
                        fit += 1;
                    }
                    out.push(self.finished(&word[..fit]));
                    word.drain(..fit);
                }
                line = word;
            }
            out.push(self.finished(&line));
        }
        out
    }

    fn finished(self, line: &[u8]) -> (Vec<u8>, f64) {
        let line = trim(line).to_vec();
        let width = self.width(&line);
        (line, width)
    }
}

/// The text without the spaces that end it.
fn trim(text: &[u8]) -> &[u8] {
    let end = text.iter().rposition(|&b| b != b' ').map_or(0, |i| i + 1);
    &text[..end]
}

impl Measure for Font {
    fn measure(&self, text: &str, wrap: Option<f64>) -> (f64, f64) {
        let lines = self.lines(text, wrap);
        let width = lines.iter().map(|(_, w)| *w).fold(0.0, f64::max);
        (width, lines.len() as f64 * self.line_height())
    }
}

/// Draws a page's items into its content stream, in the content area's
/// coordinates: from its top left corner, down.
struct Painter {
    content: String,
    /// The pictures on the page, by number.
    pictures: Vec<Picture>,
    page_height: f64,
    width: f64,
    border: Option<Border>,
}

impl Painter {
    fn page(&mut self, laid: &[(Printed, f64)], page: &Page, height: f64, footer: &str) {
        let pen = frame_pen(self.border);
        let pad = if pen.is_some() { FRAME_PAD } else { 0.0 };
        let width = self.width;
        // Where the open box's frame starts on this page.
        let mut open = page.continued.then_some(0.0);
        for &(i, y) in &page.placed {
            match &laid[i].0 {
                Printed::Title(text) => match self.border {
                    // The other pages: a rule under the title.
                    None => {
                        self.text(text, 0.0, y, None, false, Font::TITLE);
                        self.line(0.5, (0.0, y + 15.0), (width, y + 15.0));
                    }
                    Some(Border::None) => self.text(text, 0.0, y, None, false, Font::TITLE),
                    // A rectangle with the title inside.
                    Some(Border::Thin) => {
                        self.text(text, pad, y + 3.0, None, false, Font::TITLE);
                        self.line(pen.unwrap_or(0.35), (0.0, y), (width, y));
                        open = Some(y);
                    }
                    // A frame whose top line runs through the title.
                    Some(Border::Normal) => {
                        let pen = pen.unwrap_or(0.8);
                        let top = y + 7.0;
                        if text.is_empty() {
                            self.line(pen, (0.0, top), (width, top));
                        } else {
                            self.text(text, pad + 4.0, y, None, false, Font::TITLE);
                            let w = Font::TITLE.measure(text, None).0;
                            self.line(pen, (0.0, top), (pad, top));
                            self.line(pen, (pad + 8.0 + w, top), (width, top));
                        }
                        open = Some(top);
                    }
                },
                Printed::BoxEnd => {
                    if let (Some(top), Some(pen)) = (open.take(), pen) {
                        let bottom = y + 3.0;
                        self.line(pen, (0.0, top), (0.0, bottom));
                        self.line(pen, (width, top), (width, bottom));
                        self.line(pen, (0.0, bottom), (width, bottom));
                    }
                }
                Printed::Row(cells, widths) => {
                    let starts = column_starts(widths);
                    for cell in cells {
                        if cell.column >= widths.len() {
                            continue;
                        }
                        let w = cell_width(widths, cell.column, cell.span).max(1.0);
                        if cell.header {
                            let _ = writeln!(self.content, "{} g", number(HEADER_GRAY));
                        }
                        let x = pad + starts[cell.column];
                        self.text(&cell.text, x, y, Some(w), cell.right, Font::TEXT);
                        if cell.header {
                            self.content.push_str("0 g\n");
                        }
                    }
                }
                Printed::Text(text, indent) => self.text(
                    text,
                    pad + indent,
                    y,
                    Some(width - 2.0 * pad - indent),
                    false,
                    Font::TEXT,
                ),
                Printed::Picture(picture) => {
                    let _ = writeln!(
                        self.content,
                        "q {} 0 0 {} {} {} cm /Im{} Do Q",
                        number(picture.width),
                        number(picture.height),
                        number(MARGIN + pad),
                        number(self.y(y + picture.height)),
                        self.pictures.len()
                    );
                    self.pictures.push(picture.clone());
                }
                Printed::PageBreak => {}
            }
        }
        // A box that runs on to the next page keeps its sides to the foot
        // of this one.
        if let (Some(top), Some(pen)) = (open, pen) {
            let bottom = height - FOOTER;
            self.line(pen, (0.0, top), (0.0, bottom));
            self.line(pen, (width, top), (width, bottom));
        }
        // Footer: name and page.
        self.text(footer, 0.0, height - 10.0, Some(width), true, Font::TEXT);
    }

    /// The page's height from its foot for `y` down from the top of the
    /// content area.
    fn y(&self, y: f64) -> f64 {
        self.page_height - MARGIN - y
    }

    fn line(&mut self, pen: f64, from: (f64, f64), to: (f64, f64)) {
        let _ = writeln!(
            self.content,
            "{} w {} {} m {} {} l S",
            number(pen),
            number(MARGIN + from.0),
            number(self.y(from.1)),
            number(MARGIN + to.0),
            number(self.y(to.1))
        );
    }

    /// Writes `text` from (x, y), its lines wrapped at `wrap` and, with
    /// `right`, ending at its right edge.
    fn text(&mut self, text: &str, x: f64, y: f64, wrap: Option<f64>, right: bool, font: Font) {
        for (i, (line, w)) in font.lines(text, wrap).into_iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            let left = match (right, wrap) {
                (true, Some(wrap)) => x + wrap - w,
                _ => x,
            };
            let baseline = y + ASCENT * font.size + i as f64 * font.line_height();
            let _ = writeln!(
                self.content,
                "BT /{} {} Tf {} {} Td ({}) Tj ET",
                font.resource,
                number(font.size),
                number(MARGIN + left),
                number(self.y(baseline)),
                literal(&line)
            );
        }
    }
}

/// Bytes as the inside of a PDF literal string.
fn literal(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(char::from(b));
            }
            0x20..=0x7e => out.push(char::from(b)),
            _ => {
                let _ = write!(out, "\\{b:03o}");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cr_store::sheet::{Block, Cell, Flavor, Section, SectionContent, SheetBox, Table};

    fn sheet(rows: usize) -> Sheet {
        Sheet {
            title: "Borin Pedraforte".into(),
            flavor: Flavor::Expansion,
            border: Some(Border::Normal),
            sections: vec![Section {
                id: 0,
                content: SectionContent::Columns(vec![vec![SheetBox {
                    title: "Inventário".into(),
                    body: vec![
                        Block::Table(Table {
                            columns: 2,
                            rows: (0..rows)
                                .map(|i| {
                                    vec![Cell::header(format!("Tocha ×{i}")), Cell::new("3′ 11″")]
                                })
                                .collect(),
                        }),
                        Block::Paragraph("(Raças) \\ e mais".into()),
                    ],
                }]]),
            }],
        }
    }

    #[test]
    fn widths_come_from_the_font() {
        // "Ai" in Helvetica: 667 + 222 thousandths.
        assert!((Font::TEXT.width(b"Ai") - 0.889 * FONT_SIZE).abs() < 1e-9);
        assert_eq!(
            Font::TEXT.width(&encode("é")),
            Font::TEXT.width(&encode("e"))
        );
    }

    #[test]
    fn text_is_encoded_for_the_built_in_font() {
        assert_eq!(encode("Ação ×3"), b"A\xe7\xe3o \xd73");
        assert_eq!(encode("3′ 11″ – “x”"), b"3' 11\" \x96 \x93x\x94");
        assert_eq!(literal(b"(a)\\\xe7"), "\\(a\\)\\\\\\347");
    }

    #[test]
    fn lines_wrap_between_words_and_inside_long_ones() {
        let font = Font::TEXT;
        let one = font.width(b"word");
        let lines = font.lines("word word word", Some(one * 2.5));
        let texts: Vec<&[u8]> = lines.iter().map(|(l, _)| &l[..]).collect();
        assert_eq!(texts, [&b"word word"[..], b"word"]);
        let long = font.lines("abcdefghij", Some(font.width(b"abcd")));
        assert!(long.len() >= 3 && long.iter().all(|(_, w)| *w <= font.width(b"abcd")));
        assert_eq!(font.lines("a\nb", None).len(), 2);
        let ends: Vec<_> = font.lines("a\r\nb\rc\n", None);
        let ends: Vec<&[u8]> = ends.iter().map(|(l, _)| &l[..]).collect();
        assert_eq!(ends, [&b"a"[..], b"b", b"c", b""]);
        assert_eq!(font.measure("", Some(100.0)).1, font.line_height());
    }

    #[test]
    fn the_file_is_a_pdf_with_a_page_each() {
        let short = super::sheet(&sheet(3), Paper::A4);
        assert!(short.starts_with(b"%PDF-1.4\n"));
        assert!(short.ends_with(b"%%EOF\n"));
        let text = String::from_utf8_lossy(&short);
        assert!(text.contains("/Count 1 /MediaBox [0 0 595.28 841.89]"));
        assert!(text.contains("/BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding"));
        assert!(text.contains("/Title <FEFF0042006F00720069006E"));

        let long = super::sheet(&sheet(120), Paper::Letter);
        let text = String::from_utf8_lossy(&long);
        assert!(text.contains("/Count 3 /MediaBox [0 0 612 792]"));
        // The same sheet makes the same file.
        assert_eq!(long, super::sheet(&sheet(120), Paper::Letter));
    }

    #[test]
    fn the_cross_reference_table_points_at_the_objects() {
        let file = super::sheet(&sheet(3), Paper::A4);
        let at = |offset: usize, text: &str| file[offset..].starts_with(text.as_bytes());
        let tail = String::from_utf8_lossy(&file[file.len() - 40..]).into_owned();
        let start: usize = tail
            .split("startxref\n")
            .nth(1)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(at(start, "xref\n"));
        let table = String::from_utf8(file[start..].to_vec()).unwrap();
        let entries = table.lines().skip(3).take_while(|l| l.ends_with(" n "));
        for (i, line) in entries.enumerate() {
            let offset: usize = line[..10].parse().unwrap();
            assert!(at(offset, &format!("{} 0 obj", i + 1)), "{line}");
        }
    }

    #[test]
    fn numbers_are_short() {
        assert_eq!(number(612.0), "612");
        assert_eq!(number(595.276), "595.28");
        assert_eq!(number(0.35), "0.35");
        assert_eq!(number(-0.001), "0");
    }
}
