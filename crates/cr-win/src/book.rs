//! A rule book as one web page, which the Windows Book page shows in a
//! WebView2: the book's fonts, sizes, colours, indents, tab stops and
//! pictures, laid out as the GNOME reader lays them out, and the script
//! the app drives the page with. The script goes to a paragraph, zooms,
//! highlights what Find found and says which paragraph is at the top.
//!
//! Every character of the book's text is in the page, a picture standing
//! for one, so that a match Find makes in the book's folded text
//! ([`cr_app::books::Folded`]) is at the same offset of its paragraph
//! there.

use std::collections::HashMap;
use std::fmt::Write;

use cr_formats::rtf::{Align, Document, Font, Format, Inline, Paragraph, Picture, PictureFormat};

use crate::html::{base64, escape, hex, on_dark};

/// What the page posts once it has loaded.
pub const READY: &str = "ready";
/// What the page posts before the paragraph at its top, a moment after
/// scrolling stops.
pub const TOP: &str = "top:";
/// What the page posts before a key of the reader pressed in it, which
/// the window does not see: `find`, `zoom-in`, `zoom-out`, `zoom-reset`,
/// `contents`, `close` or `help`.
pub const KEY: &str = "key:";

/// The script that clears the highlighted matches.
pub const UNMARK: &str = "unmark()";

/// `doc` as a whole page.
pub fn page(doc: &Document) -> String {
    let mut out = Out::new(doc);
    let mut body = String::with_capacity(doc.paragraphs.len() * 96);
    for (i, p) in doc.paragraphs.iter().enumerate() {
        out.paragraph(i, p, &mut body);
    }
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>{STYLE}{}</style></head>\
         <body><main>{body}</main><script>{SCRIPT}</script></body></html>",
        out.classes()
    )
}

/// The script that shows the page at `zoom` from paragraph `paragraph`.
pub fn start(zoom: f64, paragraph: usize) -> String {
    format!("start({zoom},{paragraph})")
}

/// The script that goes to paragraph `paragraph`.
pub fn go(paragraph: usize) -> String {
    format!("go({paragraph})")
}

/// The script that zooms the page, keeping the paragraph at its top.
pub fn zoom(zoom: f64) -> String {
    format!("zoomTo({zoom})")
}

/// The script that highlights matches `len` characters long, each at a
/// paragraph and an offset in it.
pub fn mark(matches: &[(usize, usize)], len: usize) -> String {
    let mut script = String::with_capacity(matches.len() * 12 + 16);
    script.push_str("mark([");
    for (i, (paragraph, offset)) in matches.iter().enumerate() {
        if i > 0 {
            script.push(',');
        }
        let _ = write!(script, "{paragraph},{offset}");
    }
    let _ = write!(script, "],{len})");
    script
}

/// The script that shows the match at `offset` of `paragraph`, `len`
/// characters long, as the current one.
pub fn show(paragraph: usize, offset: usize, len: usize) -> String {
    format!("show({paragraph},{offset},{len})")
}

/// The width of the text's column at most, in pixels: `main`'s, less its
/// padding.
const COLUMN: i32 = 760 - 2 * 24;

const STYLE: &str = "\
:root{color-scheme:light dark}\
html{background:#fff;color:#1a1a1a}\
body{margin:0}\
main{box-sizing:border-box;max-width:760px;margin:0 auto;padding:24px 24px 48px;\
font:12pt 'Times New Roman',serif}\
main>div{white-space:pre-wrap;overflow-wrap:break-word;tab-size:48px}\
.s{display:flex;align-items:baseline}\
.t{display:inline-block;flex:none;white-space:pre;text-indent:0;tab-size:0}\
.r{flex:1 1 0;min-width:0;text-indent:0}\
img{max-width:100%;height:auto;vertical-align:middle}\
::highlight(m){background-color:rgba(246,211,45,.45)}\
::highlight(c){background-color:rgba(255,120,0,.8)}\
@media (prefers-color-scheme:dark){html{background:#1f1f1f;color:#f0f0f0}}";

const SCRIPT: &str = "\
const W=window.chrome.webview,P=document.querySelectorAll('main>div');let timer=0;\
function topmost(){let lo=0,hi=P.length-1;if(hi<0)return 0;while(lo<hi){const m=(lo+hi)>>1;\
if(P[m].getBoundingClientRect().bottom<=2)lo=m+1;else hi=m}return lo}\
function go(i){const p=P[Math.min(i,P.length-1)];if(p)p.scrollIntoView({block:'start'})}\
function start(z,i){document.documentElement.style.zoom=z;go(i)}\
function zoomTo(z){const i=topmost();document.documentElement.style.zoom=z;go(i)}\
function at(root,o){const w=document.createTreeWalker(root,NodeFilter.SHOW_TEXT|NodeFilter.SHOW_ELEMENT,\
{acceptNode:n=>n.nodeType===3||n.nodeName==='IMG'?NodeFilter.FILTER_ACCEPT:NodeFilter.FILTER_SKIP});\
let k=0,n;while((n=w.nextNode())){if(n.nodeType===1){if(k===o)return[n,-1,0];k++;continue}\
let i=0;for(const c of n.data){if(k===o)return[n,i,c.length];i+=c.length;k++}}return null}\
function range(p,o,len){const root=P[p];if(!root||len<1)return null;\
const a=at(root,o),b=at(root,o+len-1);if(!a||!b)return null;const r=document.createRange();\
if(a[1]<0)r.setStartBefore(a[0]);else r.setStart(a[0],a[1]);\
if(b[1]<0)r.setEndAfter(b[0]);else r.setEnd(b[0],b[1]+b[2]);return r}\
function mark(list,len){const h=new Highlight();for(let i=0;i+1<list.length;i+=2){\
const r=range(list[i],list[i+1],len);if(r)h.add(r)}CSS.highlights.set('m',h);CSS.highlights.delete('c')}\
function unmark(){CSS.highlights.clear()}\
function show(p,o,len){const r=range(p,o,len);if(!r)return;CSS.highlights.set('c',new Highlight(r));\
const e=r.startContainer.nodeType===1?r.startContainer:r.startContainer.parentElement;\
if(e)e.scrollIntoView({block:'center'})}\
addEventListener('scroll',()=>{clearTimeout(timer);timer=setTimeout(()=>W.postMessage('top:'+topmost()),600)},\
{passive:true});\
addEventListener('keydown',e=>{const c=e.ctrlKey&&!e.altKey,k=e.key.toLowerCase();let a=null;\
if(e.key==='F1')a='help';else if(e.key==='F9')a='contents';else if(c&&k==='f')a='find';\
else if(c&&(k==='+'||k==='='))a='zoom-in';else if(c&&k==='-')a='zoom-out';\
else if(c&&k==='0')a='zoom-reset';else if(c&&k==='w')a='close';\
if(a){e.preventDefault();W.postMessage('key:'+a)}});\
W.postMessage('ready');";

/// A paragraph's layout, which paragraphs share as a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Layout {
    align: Align,
    left: i32,
    indent: i32,
    right: i32,
    before: i32,
    after: i32,
}

impl Layout {
    fn of(p: &Paragraph) -> Self {
        let left = px(p.left_indent.max(0));
        Self {
            align: p.align,
            left,
            // A hanging indent cannot reach left of the margin.
            indent: px(p.first_indent).max(-left),
            right: px(p.right_indent.max(0)),
            before: px(p.space_before.max(0)),
            after: px(p.space_after.max(0)),
        }
    }
}

/// What the paragraphs go into: the book's fonts as CSS families, and
/// the classes its paragraphs and runs share.
struct Out {
    families: Vec<String>,
    layouts: HashMap<Layout, usize>,
    formats: HashMap<Format, usize>,
}

impl Out {
    fn new(doc: &Document) -> Self {
        Self {
            families: doc.fonts.iter().map(family).collect(),
            layouts: HashMap::new(),
            formats: HashMap::new(),
        }
    }

    fn paragraph(&mut self, index: usize, p: &Paragraph, html: &mut String) {
        let layout = self.layout(p);
        let empty = p.items.iter().all(|item| match item {
            Inline::Text(text, _) => text.is_empty(),
            Inline::Picture(_) => false,
        });
        if empty {
            let _ = write!(html, "<div id=\"p{index}\" class=\"a{layout}\"><br></div>");
            return;
        }
        // With tab stops of its own, the text before each tab is a
        // stretch that runs to the stop; the tab itself stays in it, as
        // nothing, for Find. The stretches make a row that does not wrap,
        // as a table's columns, and the text after the last tab wraps in
        // the rest of it.
        let mut tabs: usize = p
            .items
            .iter()
            .map(|item| match item {
                Inline::Text(text, _) => text.matches('\t').count(),
                Inline::Picture(_) => 0,
            })
            .sum();
        let mut stops = (tabs > 0 && !p.tabs.is_empty())
            .then(|| Stops::of(p))
            .filter(|stops| fits(p, stops.clone()));
        if let Some(stops) = &mut stops {
            let _ = write!(html, "<div id=\"p{index}\" class=\"a{layout} s\">");
            // A row has no first line to indent: its first stretch is.
            let indent = px(stops.at);
            stretch(stops, html, indent);
        } else {
            let _ = write!(html, "<div id=\"p{index}\" class=\"a{layout}\">");
        }
        for item in &p.items {
            match item {
                Inline::Text(text, format) => {
                    let class = self.format(format);
                    for (n, piece) in text.split('\t').enumerate() {
                        if n > 0 {
                            html.push('\t');
                            tabs -= 1;
                            if let Some(stops) = &mut stops {
                                html.push_str("</span>");
                                if tabs > 0 {
                                    stretch(stops, html, 0);
                                } else {
                                    html.push_str("<span class=\"r\">");
                                }
                            }
                        }
                        if !piece.is_empty() {
                            let _ =
                                write!(html, "<span class=\"f{class}\">{}</span>", escape(piece));
                        }
                    }
                }
                Inline::Picture(picture) => self::picture(picture, html),
            }
        }
        if stops.is_some() {
            html.push_str("</span>");
        }
        html.push_str("</div>");
    }

    fn layout(&mut self, p: &Paragraph) -> usize {
        let next = self.layouts.len();
        *self.layouts.entry(Layout::of(p)).or_insert(next)
    }

    fn format(&mut self, format: &Format) -> usize {
        let next = self.formats.len();
        *self.formats.entry(*format).or_insert(next)
    }

    /// The CSS of the classes the paragraphs use.
    fn classes(&self) -> String {
        let mut css = String::new();
        for (layout, n) in &self.layouts {
            let align = match layout.align {
                Align::Left => "left",
                Align::Center => "center",
                Align::Right => "right",
                Align::Justify => "justify",
            };
            let _ = write!(
                css,
                ".a{n}{{margin:0 {}px 0 {}px;padding:{}px 0 {}px;text-indent:{}px;text-align:{align}}}",
                layout.right, layout.left, layout.before, layout.after, layout.indent
            );
        }
        let mut dark = String::new();
        for (format, n) in &self.formats {
            let family = self
                .families
                .get(usize::from(format.font))
                .map_or("serif", String::as_str);
            let points = f32::from(format.size) / 2.0;
            let _ = write!(css, ".f{n}{{font-family:{family}");
            match format.script {
                cr_formats::rtf::Script::Normal => {
                    let _ = write!(css, ";font-size:{points}pt");
                }
                cr_formats::rtf::Script::Super => {
                    let _ = write!(css, ";font-size:{}pt;vertical-align:super", points * 0.7);
                }
                cr_formats::rtf::Script::Sub => {
                    let _ = write!(css, ";font-size:{}pt;vertical-align:sub", points * 0.7);
                }
            }
            if format.bold {
                css.push_str(";font-weight:700");
            }
            if format.italic {
                css.push_str(";font-style:italic");
            }
            if format.underline {
                css.push_str(";text-decoration:underline");
            }
            if format.small_caps {
                css.push_str(";font-variant:small-caps");
            }
            // Black is the theme's text colour; dark colours are
            // lightened on a dark background.
            if let Some(color) = format.color.filter(|c| (c.0, c.1, c.2) != (0, 0, 0)) {
                let _ = write!(css, ";color:{}", hex(color));
                let _ = write!(dark, ".f{n}{{color:{}}}", hex(on_dark(color)));
            }
            css.push('}');
        }
        if !dark.is_empty() {
            let _ = write!(css, "@media (prefers-color-scheme:dark){{{dark}}}");
        }
        css
    }
}

/// Opens the stretch up to the next tab stop, `margin` pixels from the
/// one before.
fn stretch(stops: &mut Stops, html: &mut String, margin: i32) {
    let _ = write!(
        html,
        "<span class=\"t\" style=\"min-width:{}px",
        stops.next_width()
    );
    if margin != 0 {
        let _ = write!(html, ";margin-left:{margin}px");
    }
    html.push_str("\">");
}

/// Whether the paragraph's stretches stay in the text's column and the
/// text before each of its tabs fits, about, in its stretch. A paragraph
/// with a tab after a long text, as some books have to start a new line,
/// or with more tabs than the line has room for, flows as text instead
/// of a row.
fn fits(p: &Paragraph, mut stops: Stops) -> bool {
    let left = p.left_indent.max(0);
    // Half an em a character, in pixels.
    let mut width = 0.0;
    for item in &p.items {
        let Inline::Text(text, format) = item else {
            continue;
        };
        let em = f64::from(format.size) / 2.0 * 96.0 / 72.0;
        for (n, piece) in text.split('\t').enumerate() {
            if n > 0 {
                if width > 2.0 * f64::from(stops.next_width()) + em || px(left + stops.at) > COLUMN
                {
                    return false;
                }
                width = 0.0;
            }
            width += piece.chars().count() as f64 * em / 2.0;
        }
    }
    true
}

/// A paragraph's tab stops, in twips from its left indent, and where its
/// text has got to on the first line.
#[derive(Clone)]
struct Stops {
    at: i32,
    stops: Vec<i32>,
}

impl Stops {
    /// The paragraph's own stops; a hanging indent's, at the left indent,
    /// as Word makes one; then Word's default stops every half inch.
    fn of(p: &Paragraph) -> Self {
        let left = p.left_indent.max(0);
        let indent = p.first_indent.max(-left);
        let mut stops: Vec<i32> = p
            .tabs
            .iter()
            .map(|t| t.pos - left)
            .filter(|&pos| pos > 0)
            .collect();
        if indent < 0 {
            stops.push(0);
        }
        let last = p.tabs.iter().map(|t| t.pos).max().unwrap_or(0);
        let mut default = (last / 720 + 1) * 720;
        while default <= 14400 {
            if default > left {
                stops.push(default - left);
            }
            default += 720;
        }
        stops.sort_unstable();
        stops.dedup();
        Self { at: indent, stops }
    }

    /// How wide the stretch from where the text is to the next stop is,
    /// in pixels; past the last stop, half an inch.
    fn next_width(&mut self) -> i32 {
        let stop = self
            .stops
            .iter()
            .copied()
            .find(|&s| s > self.at)
            .unwrap_or(self.at + 720);
        let width = stop - self.at;
        self.at = stop;
        px(width)
    }
}

/// Twips as pixels at 96 dots per inch.
fn px(twips: i32) -> i32 {
    (f64::from(twips) / 15.0).round() as i32
}

/// A book font as a CSS font family: the font itself, then a generic
/// family. Symbol fonts were mapped to Unicode by the reader, so they
/// show in the generic family, as in the GNOME reader.
fn family(font: &Font) -> String {
    let name = font.name.to_ascii_lowercase();
    let generic = if ["courier", "mono", "consol"]
        .iter()
        .any(|m| name.contains(m))
    {
        "monospace"
    } else if [
        "arial",
        "helvetica",
        "sans",
        "verdana",
        "tahoma",
        "univers",
        "futura",
    ]
    .iter()
    .any(|m| name.contains(m))
    {
        "sans-serif"
    } else {
        "serif"
    };
    if font.charset == 2 || font.name.is_empty() || font.name.contains(',') {
        generic.to_owned()
    } else {
        format!(
            "'{}',{generic}",
            font.name.replace(['\'', '"', '\\', ';', '{', '}', '<'], "")
        )
    }
}

/// A picture of the book; one that does not decode still takes its
/// place, so that the text after it keeps its offsets.
fn picture(picture: &Picture, html: &mut String) {
    let src = match picture.format {
        PictureFormat::Png => Some(format!("data:image/png;base64,{}", base64(&picture.data))),
        PictureFormat::Jpeg => Some(format!("data:image/jpeg;base64,{}", base64(&picture.data))),
        PictureFormat::Bmp => {
            image::load_from_memory_with_format(&picture.data, image::ImageFormat::Bmp)
                .ok()
                .and_then(|image| {
                    let mut png = std::io::Cursor::new(Vec::new());
                    image.write_to(&mut png, image::ImageFormat::Png).ok()?;
                    Some(format!(
                        "data:image/png;base64,{}",
                        base64(&png.into_inner())
                    ))
                })
        }
    };
    html.push_str("<img alt=\"\"");
    if picture.width > 0 && picture.height > 0 {
        let _ = write!(
            html,
            " width=\"{}\" height=\"{}\"",
            px(picture.width),
            px(picture.height)
        );
    }
    if let Some(src) = src {
        let _ = write!(html, " src=\"{src}\"");
    }
    html.push('>');
}

#[cfg(test)]
mod tests {
    use super::*;
    use cr_app::books::Folded;

    fn doc(body: &str) -> Document {
        Document::parse(
            format!(
                r"{{\rtf1\ansi{{\fonttbl{{\f0\froman Times New Roman;}}{{\f1\fswiss Arial;}}{{\f2\fcharset2 Symbol;}}}}{{\colortbl;\red0\green0\blue0;\red128\green0\blue0;}}{body}}}"
            )
            .as_bytes(),
        )
    }

    /// The text of `html`, tags left out and entities read, as the
    /// page's script counts it: a picture is one character.
    fn shown_text(html: &str) -> String {
        let main = &html[html.find("<main>").unwrap() + 6..html.find("</main>").unwrap()];
        let mut text = String::new();
        let mut rest = main;
        while let Some(open) = rest.find('<') {
            text.push_str(&rest[..open]);
            let close = rest[open..].find('>').unwrap() + open;
            let tag = &rest[open..=close];
            if tag.starts_with("<img") {
                text.push('\u{fffc}');
            } else if tag == "</div>" {
                text.push('\n');
            }
            rest = &rest[close + 1..];
        }
        text.push_str(rest);
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&amp;", "&")
    }

    #[test]
    fn every_character_is_where_find_counts_it() {
        let d = doc(concat!(
            r"\tx1440\tx2880 {\b Name}\tab Level\tab XP & <more>\par",
            r"\par",
            r"{\cf2 Red} text {\pict\pngblip\picwgoal300\pichgoal150 89504e47}after\par",
            r"\li720\fi-360 \bullet\tab Hanging\par",
        ));
        let html = page(&d);
        let folded = Folded::of(&d);
        let text: String = folded.chars.iter().collect();
        assert_eq!(shown_text(&html).to_lowercase(), text);
        // Paragraphs are numbered for the script.
        assert!(html.contains("<div id=\"p0\""), "{html}");
        assert!(html.contains("<div id=\"p3\""));
    }

    #[test]
    fn tab_stops_make_stretches_but_the_last_text_flows() {
        let d = doc(r"\tx1500\tx3000 Name\tab Level\tab A long last column that wraps\par");
        let html = page(&d);
        assert_eq!(html.matches("class=\"t\"").count(), 2, "{html}");
        assert!(html.contains("min-width:100px\"><span"), "{html}");
        // The stretches make a row; the text after the last tab is not in
        // a stretch, and wraps in the rest of the row.
        assert!(html.contains("class=\"a0 s\""), "{html}");
        assert!(
            html.contains("</span>\t</span><span class=\"r\"><span class=\"f0\">A long"),
            "{html}"
        );
        assert!(html.contains("wraps</span></span></div>"), "{html}");
        // Without stops of its own, a tab is the default half inch.
        let plain = page(&doc(r"Name\tab Level\par"));
        assert!(!plain.contains("class=\"t\""));
    }

    #[test]
    fn a_long_text_before_a_tab_flows() {
        let d = doc(concat!(
            r"\tx240 A first paragraph that runs on far past its tab stop, as one",
            r" that only tabs to start the next does.\tab The next.\par"
        ));
        let html = page(&d);
        assert!(!html.contains("class=\"t\""), "{html}");
        assert!(!html.contains(" s\""), "{html}");
        // Tabs past the column's edge.
        let tabs = r"\tab".repeat(20);
        let html = page(&doc(&format!(r"\tx2430\tx3600 {tabs} TOTAL\par")));
        assert!(!html.contains(" s\""), "{html}");
    }

    #[test]
    fn a_hanging_indent_has_a_stop_at_the_left_indent() {
        let d = doc(r"\li720\fi-360\tx2000 \bullet\tab Text\par");
        let html = page(&d);
        // From -360 to the indent's own stop at 0: 24 pixels, from 24
        // pixels left of the indent.
        assert!(
            html.contains("min-width:24px;margin-left:-24px\">"),
            "{html}"
        );
        assert!(html.contains("text-indent:-24px"), "{html}");
    }

    #[test]
    fn paragraphs_and_runs_share_classes() {
        let d = doc(concat!(
            r"\qc {\f1\b\fs32 Chapter}\par",
            r"\pard Plain {\cf1 black} {\cf2 red}\par",
            r"\pard Plain again\par",
        ));
        let html = page(&d);
        assert_eq!(html.matches("{margin:0 ").count(), 2, "{html}");
        assert!(html.contains("font-family:'Arial',sans-serif;font-size:16pt;font-weight:700"));
        assert!(html.contains("text-align:center"));
        // Black is the theme's colour; dark red is lightened in dark mode.
        assert_eq!(html.matches("pt;color:").count(), 1, "{html}");
        assert!(html.contains(";color:#800000"));
        assert!(html.contains("@media (prefers-color-scheme:dark){.f"));
    }

    #[test]
    fn symbol_fonts_show_in_the_generic_family() {
        let font = |name: &str, charset| Font {
            name: name.into(),
            charset,
        };
        assert_eq!(family(&font("Symbol", 2)), "serif");
        assert_eq!(family(&font("Courier New", 0)), "'Courier New',monospace");
        assert_eq!(
            family(&font("Times New Roman", 0)),
            "'Times New Roman',serif"
        );
    }

    #[test]
    fn scripts_name_paragraphs_and_offsets() {
        assert_eq!(mark(&[(3, 0), (40, 12)], 4), "mark([3,0,40,12],4)");
        assert_eq!(show(40, 12, 4), "show(40,12,4)");
        assert_eq!(start(1.25, 7), "start(1.25,7)");
        assert_eq!(zoom(1.0), "zoomTo(1)");
    }
}
