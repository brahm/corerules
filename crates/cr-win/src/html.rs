//! Help topics as web pages, which the Windows app shows in a WebView2:
//! WinHelp's fonts, sizes, colours, indents, tab stops, tables and
//! pictures, its non-scrolling region kept at the top, its hotspots as
//! links the page posts back to the app, and its popups shown over the
//! topic, as WinHelp draws them.

use std::fmt::Write;

use cr_formats::rtf::{Align, Format, Rgb, Script};
use cr_formats::winhelp::{HelpFile, Item, Link, Paragraph, Topic};

/// A topic as HTML, with the links its hotspots name by index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    pub html: String,
    pub links: Vec<Link>,
}

/// What a hotspot of a popup posts before its index, apart from the
/// topic's own.
pub const POPUP: &str = "p";

/// The widest a picture shows, as a screen of the original.
const PICTURE_WIDTH: u32 = 560;

/// Topic `topic` of `help` as a whole page. Its hotspots post their index
/// in [`Page::links`]; `showPopup(html)` shows a popup's HTML over it.
pub fn topic(help: &HelpFile, topic: &Topic) -> Page {
    let mut out = Out::new(help, "");
    let paragraphs = &topic.paragraphs;
    // WinHelp keeps the non-scrolling region above the topic.
    let header = paragraphs.iter().take_while(|p| p.non_scrolling).count();
    let mut body = String::new();
    if header > 0 {
        body.push_str("<header>");
        out.paragraphs(&paragraphs[..header], &mut body);
        body.push_str("</header>");
    }
    body.push_str("<main>");
    out.paragraphs(&paragraphs[header..], &mut body);
    body.push_str("</main>");
    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>{STYLE}</style></head>\
         <body>{body}<div id=\"popup\" hidden><div class=\"box\" role=\"dialog\"></div></div>\
         <script>{SCRIPT}</script></body></html>"
    );
    Page {
        html,
        links: out.links,
    }
}

/// Topic `topic` of `help` as the inside of a popup; its hotspots post
/// [`POPUP`] before their index.
pub fn popup(help: &HelpFile, topic: &Topic) -> Page {
    let mut out = Out::new(help, POPUP);
    let mut html = String::new();
    out.paragraphs(&topic.paragraphs, &mut html);
    Page {
        html,
        links: out.links,
    }
}

/// `text` as a JavaScript string literal, for a script the app runs in
/// the page.
pub fn js_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Line and paragraph separators end a line in a script.
            '\u{2028}' | '\u{2029}' => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

const STYLE: &str = "\
:root{color-scheme:light dark}\
html{background:#fff;color:#1a1a1a}\
body{margin:0;padding:18px 24px 24px;font:12pt 'Times New Roman',serif}\
header{position:sticky;top:0;background:#fff;margin:-18px -24px 12px;padding:12px 24px 8px;\
border-bottom:1px solid #d0d0d0}\
.p{white-space:pre-wrap;tab-size:48px;overflow-wrap:break-word}\
.t{display:inline-block;white-space:pre}\
.c{color:var(--l)}\
.h{color:#1c7c3c;text-decoration:underline;cursor:pointer}\
.h.u{text-decoration-style:dotted}\
.h:focus-visible{outline:2px solid #1c7c3c;outline-offset:1px}\
table{border-collapse:collapse;margin:2px 0}\
td{padding:1px 18px 1px 0;vertical-align:top}\
img{max-width:min(100%,560px);height:auto;vertical-align:middle}\
#popup{position:fixed;inset:0;background:rgba(0,0,0,.3);display:flex;align-items:center;\
justify-content:center;padding:24px}\
#popup[hidden]{display:none}\
#popup .box{max-width:480px;max-height:80vh;overflow:auto;background:#fff;border-radius:8px;\
box-shadow:0 8px 32px rgba(0,0,0,.35);padding:12px 18px 18px}\
@media (prefers-color-scheme:dark){\
html,header{background:#1f1f1f;color:#f0f0f0}\
header{border-color:#3a3a3a}\
.c{color:var(--d)}\
.h{color:#8ff0a4}\
.h:focus-visible{outline-color:#8ff0a4}\
#popup .box{background:#2b2b2b}}";

const SCRIPT: &str = "\
function hidePopup(){document.getElementById('popup').hidden=true}\
function showPopup(html){const p=document.getElementById('popup');\
p.querySelector('.box').innerHTML=html;p.hidden=false;p.querySelector('.box').scrollTop=0}\
function follow(e){const a=e.target.closest('[data-link]');if(!a)return false;\
e.preventDefault();hidePopup();window.chrome.webview.postMessage(a.dataset.link);return true}\
document.addEventListener('click',e=>{if(!follow(e)&&!e.target.closest('#popup .box'))hidePopup()});\
document.addEventListener('keydown',e=>{if(e.key==='Escape')hidePopup();\
else if(e.key==='Enter')follow(e)});";

/// What a topic's paragraphs go into.
struct Out<'a> {
    help: &'a HelpFile,
    prefix: &'static str,
    links: Vec<Link>,
}

impl<'a> Out<'a> {
    fn new(help: &'a HelpFile, prefix: &'static str) -> Self {
        Self {
            help,
            prefix,
            links: Vec::new(),
        }
    }

    fn paragraphs(&mut self, paragraphs: &[Paragraph], html: &mut String) {
        let mut i = 0;
        while i < paragraphs.len() {
            let p = &paragraphs[i];
            let Some(cell) = p.cell else {
                self.paragraph(p, html);
                i += 1;
                continue;
            };
            // A table: the cells of rows that follow one another.
            let first = i;
            let mut last_row = cell.row;
            while let Some(c) = paragraphs.get(i).and_then(|p| p.cell) {
                if c.row != last_row && c.row != last_row + 1 {
                    break;
                }
                last_row = c.row;
                i += 1;
            }
            self.table(&paragraphs[first..i], html);
        }
    }

    fn paragraph(&mut self, p: &Paragraph, html: &mut String) {
        let _ = write!(html, "<div class=\"p\" style=\"{}\">", paragraph_style(p));
        if p.items.is_empty() {
            html.push_str("<br>");
        } else {
            // Explicit tab stops: each stretch of text runs to the next.
            let has_tabs = !p.tabs.is_empty()
                && p.items
                    .iter()
                    .any(|item| matches!(item, Item::Text(run) if run.text.contains('\t')));
            let widths = has_tabs.then(|| tab_widths(p));
            self.items(&p.items, widths.as_deref(), false, html);
        }
        html.push_str("</div>");
    }

    fn table(&mut self, cells: &[Paragraph], html: &mut String) {
        html.push_str("<table>");
        let mut row = None;
        let mut column = None;
        for p in cells {
            let c = p.cell.expect("a table holds cells");
            if row != Some(c.row) {
                if row.is_some() {
                    html.push_str("</td></tr>");
                }
                html.push_str("<tr>");
                row = Some(c.row);
                column = None;
            }
            if column != Some(c.column) {
                if column.is_some() {
                    html.push_str("</td>");
                }
                // A cell left out of the row keeps its place.
                let skipped = column.map_or(c.column, |at| c.column.saturating_sub(at + 1));
                for _ in 0..skipped {
                    html.push_str("<td></td>");
                }
                html.push_str("<td>");
                column = Some(c.column);
            }
            let align = match p.align {
                Align::Center => "center",
                Align::Right => "right",
                _ => "left",
            };
            let _ = write!(html, "<div style=\"text-align:{align}\">");
            self.items(&p.items, None, true, html);
            html.push_str("</div>");
        }
        if row.is_some() {
            html.push_str("</td></tr>");
        }
        html.push_str("</table>");
    }

    /// The runs and pictures of a paragraph; in a cell, its line ends and
    /// tabs are spaces.
    fn items(&mut self, items: &[Item], widths: Option<&[i32]>, cell: bool, html: &mut String) {
        let mut tabs = widths.map(|w| w.iter());
        if let Some(width) = tabs.as_mut().and_then(Iterator::next) {
            let _ = write!(html, "<span class=\"t\" style=\"min-width:{width}px\">");
        }
        for item in items {
            match item {
                Item::Text(run) => {
                    let text = if cell {
                        run.text.replace(['\n', '\t'], " ")
                    } else {
                        run.text.clone()
                    };
                    for (n, piece) in text.split('\t').enumerate() {
                        if n > 0 && tabs.is_some() {
                            html.push_str("</span>");
                            match tabs.as_mut().and_then(Iterator::next) {
                                Some(width) => {
                                    let _ = write!(
                                        html,
                                        "<span class=\"t\" style=\"min-width:{width}px\">"
                                    );
                                }
                                // Past the last stop, a tab is a space.
                                None => {
                                    tabs = None;
                                    html.push(' ');
                                }
                            }
                        } else if n > 0 {
                            html.push('\t');
                        }
                        if !piece.is_empty() {
                            self.run(piece, &run.format, run.link.as_ref(), html);
                        }
                    }
                }
                Item::Picture(n) => {
                    if let Some((src, width)) = picture(self.help, *n) {
                        let _ = write!(html, "<img alt=\"\" width=\"{width}\" src=\"{src}\">");
                    }
                }
            }
        }
        if tabs.is_some() {
            html.push_str("</span>");
        }
    }

    fn run(&mut self, text: &str, format: &Format, link: Option<&Link>, html: &mut String) {
        let face = self
            .help
            .faces
            .get(usize::from(format.font))
            .map_or("", String::as_str);
        let mut style = format!(
            "font-family:{};font-size:{}pt",
            family(face),
            f32::from(format.size) / 2.0
        );
        if format.bold {
            style.push_str(";font-weight:700");
        }
        if format.italic {
            style.push_str(";font-style:italic");
        }
        if format.small_caps {
            style.push_str(";font-variant:small-caps");
        }
        match format.script {
            Script::Super => style.push_str(";vertical-align:super;font-size:smaller"),
            Script::Sub => style.push_str(";vertical-align:sub;font-size:smaller"),
            Script::Normal => {}
        }
        let mut classes = String::new();
        match link {
            Some(link) => {
                // WinHelp's hotspots: green and underlined, dotted for a
                // popup.
                classes.push('h');
                if popup_link(link) {
                    classes.push_str(" u");
                }
            }
            None => {
                if format.underline {
                    style.push_str(";text-decoration:underline");
                }
                if let Some(color) = format.color {
                    classes.push('c');
                    let _ = write!(style, ";--l:{};--d:{}", hex(color), hex(on_dark(color)));
                }
            }
        }
        let text = escape(text);
        match link {
            Some(link) => {
                let index = self.links.len();
                self.links.push(link.clone());
                let _ = write!(
                    html,
                    "<span class=\"{classes}\" style=\"{style}\" tabindex=\"0\" role=\"link\" \
                     data-link=\"{}{index}\">{text}</span>",
                    self.prefix
                );
            }
            None if classes.is_empty() => {
                let _ = write!(html, "<span style=\"{style}\">{text}</span>");
            }
            None => {
                let _ = write!(
                    html,
                    "<span class=\"{classes}\" style=\"{style}\">{text}</span>"
                );
            }
        }
    }
}

fn popup_link(link: &Link) -> bool {
    match link {
        Link::Topic { popup, .. } | Link::Offset { popup, .. } => *popup,
        Link::Macro(text) => matches!(
            cr_formats::winhelp::Macro::parse(text),
            Some(cr_formats::winhelp::Macro::Jump { popup: true, .. })
        ),
    }
}

/// Twips as pixels at 96 dots per inch.
fn px(twips: i32) -> i32 {
    twips / 15
}

fn paragraph_style(p: &Paragraph) -> String {
    let align = match p.align {
        Align::Center => "center",
        Align::Right => "right",
        Align::Justify => "justify",
        Align::Left => "left",
    };
    format!(
        "margin:0 {}px 0 {}px;padding:{}px 0 {}px;text-indent:{}px;text-align:{align}",
        px(p.right_indent.max(0)),
        px(p.left_indent.max(0)),
        px(p.space_before.max(0)),
        px(p.space_after.max(0)),
        px(p.first_indent),
    )
}

/// How wide each stretch of a paragraph is up to its next tab stop, the
/// first from where its first line starts.
fn tab_widths(p: &Paragraph) -> Vec<i32> {
    let mut at = px(p.first_indent);
    p.tabs
        .iter()
        .map(|stop| {
            let stop = px(stop.pos);
            let width = (stop - at).max(0);
            at = at.max(stop);
            width
        })
        .collect()
}

/// A WinHelp face as a CSS font family, with a generic family after it.
fn family(face: &str) -> String {
    let name = face.to_ascii_lowercase();
    let generic = if name.contains("courier") || name.contains("mono") {
        "monospace"
    } else if ["arial", "helvetica", "sans", "verdana", "tahoma", "ms sans"]
        .iter()
        .any(|m| name.contains(m))
    {
        "sans-serif"
    } else if face.is_empty() {
        return "sans-serif".to_owned();
    } else {
        "serif"
    };
    format!("'{}',{generic}", face.replace(['\'', '"', '\\', ';'], ""))
}

fn hex(Rgb(r, g, b): Rgb) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// A colour as shown on a dark background: dark colours are lightened,
/// as the GNOME app's help does.
fn on_dark(color: Rgb) -> Rgb {
    let Rgb(r, g, b) = color;
    let luminance = 0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b);
    if luminance >= 127.5 {
        return color;
    }
    let lift = |c: u8| (f32::from(c) + (255.0 - f32::from(c)) * 0.55).round() as u8;
    Rgb(lift(r), lift(g), lift(b))
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// Picture `n` of `help` as a data URI, and the width it shows at: a
/// PNG, which takes less room than the bitmap, or the bitmap itself when
/// it does not decode here.
fn picture(help: &HelpFile, n: u16) -> Option<(String, u32)> {
    let bitmap = help.picture(n)?;
    let png = image::load_from_memory_with_format(&bitmap.bmp, image::ImageFormat::Bmp)
        .ok()
        .and_then(|image| {
            let mut png = std::io::Cursor::new(Vec::new());
            image.write_to(&mut png, image::ImageFormat::Png).ok()?;
            Some(png.into_inner())
        });
    let src = match png {
        Some(png) => format!("data:image/png;base64,{}", base64(&png)),
        None => format!("data:image/bmp;base64,{}", base64(&bitmap.bmp)),
    };
    Some((src, bitmap.width.min(PICTURE_WIDTH)))
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(TABLE[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cr_formats::rtf::{TabKind, TabStop};
    use cr_formats::winhelp::{Cell, Run};

    fn run(text: &str, link: Option<Link>) -> Item {
        Item::Text(Run {
            text: text.into(),
            format: Format::default(),
            link,
        })
    }

    fn paragraph(items: Vec<Item>) -> Paragraph {
        Paragraph {
            items,
            ..Paragraph::default()
        }
    }

    fn help(paragraphs: Vec<Paragraph>) -> (HelpFile, Topic) {
        let topic = Topic {
            paragraphs,
            ..Topic::default()
        };
        let mut help = HelpFile::default();
        help.faces = vec!["Arial".into()];
        help.topics = vec![topic.clone()];
        (help, topic)
    }

    #[test]
    fn hotspots_post_their_index_and_text_is_escaped() {
        let link = Link::Offset {
            offset: 7,
            popup: true,
        };
        let (help, t) = help(vec![paragraph(vec![
            run("A <b> & ", None),
            run("term", Some(link.clone())),
        ])]);
        let page = topic(&help, &t);
        assert!(page.html.contains("A &lt;b&gt; &amp; "), "{}", page.html);
        assert!(page.html.contains("data-link=\"0\">term</span>"));
        assert!(page.html.contains("class=\"h u\""));
        assert!(page.html.contains("'Arial',sans-serif"));
        assert_eq!(page.links, [link]);
        let inside = popup(&help, &t);
        assert!(inside.html.contains("data-link=\"p0\""));
        assert!(!inside.html.contains("<script>"));
    }

    #[test]
    fn tab_stops_make_stretches_up_to_the_next() {
        let mut p = paragraph(vec![run("Name\tLevel\tXP", None)]);
        p.tabs = vec![
            TabStop {
                pos: 1500,
                kind: TabKind::Left,
            },
            TabStop {
                pos: 3000,
                kind: TabKind::Left,
            },
        ];
        let (help, t) = help(vec![p]);
        let html = topic(&help, &t).html;
        assert!(html.contains("min-width:100px\"><span"), "{html}");
        assert_eq!(html.matches("class=\"t\"").count(), 2, "{html}");
    }

    #[test]
    fn cells_make_a_table() {
        let cell = |row, column, text: &str| Paragraph {
            cell: Some(Cell { row, column }),
            ..paragraph(vec![run(text, None)])
        };
        let (help, t) = help(vec![
            cell(0, 0, "a"),
            cell(0, 1, "b"),
            cell(1, 1, "d"),
            paragraph(vec![run("after", None)]),
        ]);
        let html = topic(&help, &t).html;
        let table = &html[html.find("<table>").unwrap()..html.find("</table>").unwrap()];
        assert_eq!(table.matches("<tr>").count(), 2);
        // The row without its first cell keeps the second in place.
        assert!(table.contains("<tr><td></td><td>"), "{table}");
        assert!(html.contains(">after</span>"));
    }

    #[test]
    fn dark_colours_are_lightened_on_a_dark_background() {
        assert_eq!(on_dark(Rgb(255, 255, 0)), Rgb(255, 255, 0));
        let Rgb(r, ..) = on_dark(Rgb(128, 0, 0));
        assert!(r > 128);
    }

    #[test]
    fn scripts_take_any_text() {
        assert_eq!(js_string("a\"b\\c\nd"), r#""a\"b\\c\nd""#);
        assert_eq!(js_string("\u{2028}"), r#""\u2028""#);
    }

    #[test]
    fn base64_pads_the_last_bytes() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b""), "");
    }
}
