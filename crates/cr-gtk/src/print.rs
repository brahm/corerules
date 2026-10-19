//! Printing a text buffer as the help window shows it (a WinHelp topic,
//! or the text of the Custom Help box): its paragraphs one under the
//! other, with their margins, tabs, fonts, colours and pictures.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::tr;
use gtk::{gdk, pango};

/// Screen pixels (96 per inch) in points.
const POINTS: f64 = 0.75;

/// The space the help window keeps beside the text, which the page's own
/// margins replace.
const MARGIN: i32 = 24;

/// A paragraph of the buffer, ready to lay out.
struct Paragraph {
    text: String,
    attrs: pango::AttrList,
    /// Left margin, first-line indent (negative: hanging), right margin,
    /// space before and after, in points.
    left: f64,
    indent: f64,
    right: f64,
    above: f64,
    below: f64,
    align: pango::Alignment,
    tabs: Option<pango::TabArray>,
    /// The pictures: their byte index in `text`, and their texture and
    /// size in points.
    pictures: Vec<(i32, gdk::Texture, f64, f64)>,
}

/// A paragraph laid out on a page: where it starts and its layout.
type Placed = (f64, Rc<Paragraph>, pango::Layout);

/// Prints `buffer` under the job name `job`; `pictures` are the textures
/// in the buffer's child anchors, with their width and height in pixels.
/// `toast` reports a failure.
pub(crate) fn print_buffer(
    parent: &impl IsA<gtk::Widget>,
    job: &str,
    buffer: &gtk::TextBuffer,
    pictures: &[(gtk::TextChildAnchor, gdk::Texture, i32, i32)],
    toast: &dyn Fn(&str),
) {
    let paragraphs: Vec<Rc<Paragraph>> = paragraphs(buffer, pictures)
        .into_iter()
        .map(Rc::new)
        .collect();
    let operation = gtk::PrintOperation::builder()
        .job_name(job)
        .embed_page_setup(true)
        .use_full_page(false)
        .unit(gtk::Unit::Points)
        .build();
    let pages: Rc<RefCell<Vec<Vec<Placed>>>> = Rc::default();
    operation.connect_begin_print({
        let pages = pages.clone();
        move |operation, context| {
            let (width, height) = (context.width(), context.height());
            let mut all: Vec<Vec<Placed>> = vec![Vec::new()];
            let mut y = 0.0;
            for p in &paragraphs {
                let layout = layout(context, p, width);
                let h = f64::from(layout.pixel_size().1) + p.above + p.below;
                if y + h > height && y > 0.0 {
                    all.push(Vec::new());
                    y = 0.0;
                }
                all.last_mut().unwrap().push((y, p.clone(), layout));
                y += h;
            }
            operation.set_n_pages(all.len() as i32);
            *pages.borrow_mut() = all;
        }
    });
    operation.connect_draw_page({
        let pages = pages.clone();
        move |_, context, page| {
            let cr = context.cairo_context();
            for (y, p, layout) in pages.borrow().get(page as usize).into_iter().flatten() {
                let x = p.left + p.indent.min(0.0);
                let top = y + p.above;
                cr.move_to(x, top);
                pangocairo::functions::show_layout(&cr, layout);
                for (index, texture, w, h) in &p.pictures {
                    let at = layout.index_to_pos(*index);
                    let scale = f64::from(pango::SCALE);
                    draw_texture(
                        &cr,
                        texture,
                        x + f64::from(at.x()) / scale,
                        top + f64::from(at.y()) / scale,
                        *w,
                        *h,
                    );
                }
            }
        }
    });
    let window = parent.root().and_downcast::<gtk::Window>();
    if let Err(e) = operation.run(gtk::PrintOperationAction::PrintDialog, window.as_ref()) {
        toast(&tr!("Could not print: {error}", error = e));
    }
}

/// A paragraph's layout on a page `width` points wide.
fn layout(context: &gtk::PrintContext, p: &Paragraph, width: f64) -> pango::Layout {
    let layout = context.create_pango_layout();
    let scale = f64::from(pango::SCALE);
    let indent = p.indent.min(0.0);
    let wrap = (width - p.left - indent - p.right).max(36.0);
    layout.set_width((wrap * scale) as i32);
    layout.set_wrap(pango::WrapMode::WordChar);
    layout.set_alignment(p.align);
    // A negative indent puts the first line left of the margin: the
    // layout starts there and indents the lines after it.
    layout.set_indent((p.indent.abs() * scale * p.indent.signum()) as i32);
    layout.set_tabs(p.tabs.as_ref());
    layout.set_text(&p.text);
    layout.set_attributes(Some(&p.attrs));
    layout
}

/// The buffer's paragraphs (its lines), with what their tags set.
fn paragraphs(
    buffer: &gtk::TextBuffer,
    pictures: &[(gtk::TextChildAnchor, gdk::Texture, i32, i32)],
) -> Vec<Paragraph> {
    let mut all = Vec::new();
    let mut line = buffer.start_iter();
    loop {
        let mut end = line;
        if !end.ends_line() {
            end.forward_to_line_end();
        }
        all.push(paragraph(buffer, &line, &end, pictures));
        if !line.forward_line() {
            break;
        }
    }
    all
}

fn paragraph(
    buffer: &gtk::TextBuffer,
    start: &gtk::TextIter,
    end: &gtk::TextIter,
    pictures: &[(gtk::TextChildAnchor, gdk::Texture, i32, i32)],
) -> Paragraph {
    let text = buffer.slice(start, end, true).to_string();
    let byte = |iter: &gtk::TextIter| -> u32 {
        let chars = (iter.offset() - start.offset()) as usize;
        text.char_indices()
            .nth(chars)
            .map_or(text.len(), |(b, _)| b) as u32
    };
    let px = |v: i32| f64::from(v) * POINTS;
    let mut p = Paragraph {
        text: text.clone(),
        attrs: pango::AttrList::new(),
        left: 0.0,
        indent: 0.0,
        right: 0.0,
        above: 0.0,
        below: 0.0,
        align: pango::Alignment::Left,
        tabs: None,
        pictures: Vec::new(),
    };
    // The paragraph's own tag is on its first character (or, for an
    // empty line, its newline).
    for tag in start.tags() {
        if tag.is_left_margin_set() {
            p.left = px((tag.left_margin() - MARGIN).max(0));
        }
        if tag.is_indent_set() {
            p.indent = px(tag.indent());
        }
        if tag.is_right_margin_set() {
            p.right = px((tag.right_margin() - MARGIN).max(0));
        }
        if tag.is_pixels_above_lines_set() {
            p.above = px(tag.pixels_above_lines());
        }
        if tag.is_pixels_below_lines_set() {
            p.below = px(tag.pixels_below_lines());
        }
        if tag.is_justification_set() {
            p.align = match tag.justification() {
                gtk::Justification::Center => pango::Alignment::Center,
                gtk::Justification::Right => pango::Alignment::Right,
                _ => pango::Alignment::Left,
            };
        }
        if tag.is_tabs_set()
            && let Some(tabs) = tag.tabs()
        {
            let mut scaled = pango::TabArray::new(tabs.size(), true);
            for i in 0..tabs.size() {
                let (align, at) = tabs.tab(i);
                scaled.set_tab(i, align, (f64::from(at) * POINTS) as i32);
            }
            p.tabs = Some(scaled);
        }
    }
    // The runs: each stretch between tag toggles.
    let mut at = *start;
    while at < *end {
        let mut next = at;
        if !next.forward_to_tag_toggle(None::<&gtk::TextTag>) || next > *end {
            next = *end;
        }
        let (from, to) = (byte(&at), byte(&next));
        for tag in at.tags() {
            for mut attr in run_attrs(&tag) {
                attr.set_start_index(from);
                attr.set_end_index(to);
                p.attrs.insert(attr);
            }
        }
        at = next;
    }
    // The pictures, each a character of its own.
    let mut at = *start;
    while at < *end {
        if let Some(anchor) = at.child_anchor()
            && let Some((_, texture, w, h)) = pictures.iter().find(|(a, ..)| *a == anchor)
        {
            let (w, h) = (px(*w), px(*h));
            let scale = f64::from(pango::SCALE);
            let rect = pango::Rectangle::new(
                0,
                (-h * scale) as i32,
                (w * scale) as i32,
                (h * scale) as i32,
            );
            let mut shape = pango::AttrShape::new(&rect, &rect);
            let index = byte(&at);
            shape.set_start_index(index);
            shape.set_end_index(index + '\u{fffc}'.len_utf8() as u32);
            p.attrs.insert(shape);
            p.pictures.push((index as i32, texture.clone(), w, h));
        }
        at.forward_char();
    }
    p
}

/// The attributes of a run's tag: its font, underline and colour.
fn run_attrs(tag: &gtk::TextTag) -> Vec<pango::Attribute> {
    let mut attrs: Vec<pango::Attribute> = Vec::new();
    let mut font = pango::FontDescription::new();
    let mut any = false;
    if tag.is_family_set()
        && let Some(family) = tag.family()
    {
        font.set_family(&family);
        any = true;
    }
    if tag.is_size_set() {
        font.set_size((tag.size_points() * f64::from(pango::SCALE)) as i32);
        any = true;
    }
    if tag.is_weight_set() {
        font.set_weight(if tag.weight() >= 600 {
            pango::Weight::Bold
        } else {
            pango::Weight::Normal
        });
        any = true;
    }
    if tag.is_style_set() {
        font.set_style(tag.style());
        any = true;
    }
    if tag.is_variant_set() {
        font.set_variant(tag.variant());
        any = true;
    }
    if any {
        attrs.push(pango::AttrFontDesc::new(&font).upcast());
    }
    if tag.is_underline_set() {
        attrs.push(pango::AttrInt::new_underline(tag.underline()).upcast());
    }
    if tag.is_foreground_set()
        && let Some(c) = tag.foreground_rgba()
    {
        let channel = |v: f32| (v.clamp(0.0, 1.0) * 65535.0) as u16;
        attrs.push(
            pango::AttrColor::new_foreground(
                channel(c.red()),
                channel(c.green()),
                channel(c.blue()),
            )
            .upcast(),
        );
    }
    attrs
}

/// Draws `texture` at (`x`, `y`), `w` by `h` points.
pub(crate) fn draw_texture(
    cr: &gtk::cairo::Context,
    texture: &gdk::Texture,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let (tw, th) = (texture.width(), texture.height());
    if tw <= 0 || th <= 0 {
        return;
    }
    let stride = tw as usize * 4;
    let mut data = vec![0u8; stride * th as usize];
    // The default memory format is cairo's ARGB32.
    texture.download(&mut data, stride);
    let Ok(surface) = gtk::cairo::ImageSurface::create_for_data(
        data,
        gtk::cairo::Format::ARgb32,
        tw,
        th,
        stride as i32,
    ) else {
        return;
    };
    let _ = cr.save();
    cr.translate(x, y);
    cr.scale(w / f64::from(tw), h / f64::from(th));
    let _ = cr.set_source_surface(&surface, 0.0, 0.0);
    let _ = cr.paint();
    let _ = cr.restore();
}

/// A Print button, as the original's dialogs have.
pub(crate) fn button() -> gtk::Button {
    gtk::Button::builder()
        .icon_name("printer-symbolic")
        .tooltip_text(tr("Print"))
        .build()
}

/// Prints `text` under the heading `title`, as the original's Custom
/// Help box prints its record; a failure shows over `parent`.
pub(crate) fn print_text(parent: &impl IsA<gtk::Widget>, title: &str, text: &str) {
    let buffer = gtk::TextBuffer::new(None);
    // Arial 11 points, as the original prints it.
    let font = [("family", &"Sans" as &dyn ToValue), ("size-points", &11.0)];
    let heading = buffer.create_tag(None, &[("weight", &700)]);
    let body = buffer.create_tag(None, &font);
    let mut end = buffer.end_iter();
    let tags: Vec<&gtk::TextTag> = heading.iter().chain(&body).collect();
    buffer.insert_with_tags(&mut end, title, &tags);
    buffer.insert(&mut end, "\n\n");
    let tags: Vec<&gtk::TextTag> = body.iter().collect();
    buffer.insert_with_tags(&mut end, text, &tags);
    let widget = parent.clone().upcast::<gtk::Widget>();
    let alert = move |message: &str| {
        adw::AlertDialog::new(Some(tr("Could not print")), Some(message)).present(Some(&widget));
    };
    print_buffer(parent, title, &buffer, &[], &alert);
}
