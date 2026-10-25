//! The character sheet laid out on printed pages: boxes one under the
//! other, tables with their columns sized to fit the page, a title kept
//! with what follows it and a footer on every page. Text is measured
//! through [`Measure`], so nothing here depends on how it is drawn: the
//! GNOME app measures with Pango, the Windows app with DirectWrite and
//! the PDF with the metrics of its own font.
//!
//! Lengths are in points.

use std::path::Path;
use std::rc::Rc;

use cr_store::sheet::{Align, Block, Border, CellKind, ListItem, SectionContent, Sheet};

/// How far a framed box's contents sit inside its frame.
pub const FRAME_PAD: f64 = 8.0;
/// The height kept free at the foot of a page for its footer.
pub const FOOTER: f64 = 16.0;
/// The space between table columns.
pub const GAP: f64 = 6.0;
/// The size of the text and of the titles.
pub const FONT_SIZE: f64 = 8.0;
pub const TITLE_SIZE: f64 = 10.0;

/// Measures text in the sheet's font: its width and height, wrapped at
/// `wrap` when given.
pub trait Measure {
    fn measure(&self, text: &str, wrap: Option<f64>) -> (f64, f64);
}

/// A cell of a table row: its text, first column, columns spanned,
/// right alignment and whether it is a header.
#[derive(Debug, Clone, PartialEq)]
pub struct RowCell {
    pub text: String,
    pub column: usize,
    pub span: usize,
    pub right: bool,
    pub header: bool,
}

/// A picture: its pixels (RGBA) and size in pixels, and the size it is
/// printed at.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub rgba: Rc<Vec<u8>>,
    pub pixels: (u32, u32),
    pub width: f64,
    pub height: f64,
}

/// One thing drawn on a page.
#[derive(Debug, Clone, PartialEq)]
pub enum Printed {
    Title(String),
    /// A table row, with the column widths of its table.
    Row(Vec<RowCell>, Rc<Vec<f64>>),
    /// Text and its indent.
    Text(String, f64),
    Picture(Picture),
    PageBreak,
    /// The end of a framed box.
    BoxEnd,
}

/// Whether `border` draws frames, and with what pen: the original's
/// 16-twip pen for Normal and 7 twips for Thin (0x6d728a).
pub fn frame_pen(border: Option<Border>) -> Option<f64> {
    match border {
        Some(Border::Normal) => Some(0.8),
        Some(Border::Thin) => Some(0.35),
        Some(Border::None) | None => None,
    }
}

/// Lays the sheet out for a page `width` wide: each thing to draw and
/// the height it takes.
pub fn items(sheet: &Sheet, measure: &impl Measure, width: f64) -> Vec<(Printed, f64)> {
    let framed = frame_pen(sheet.border).is_some();
    let width = if framed {
        width - 2.0 * FRAME_PAD
    } else {
        width
    };
    let mut items = Vec::new();
    for section in &sheet.sections {
        let columns = match &section.content {
            SectionContent::Columns(columns) => columns,
            SectionContent::PageBreak => {
                items.push((Printed::PageBreak, 0.0));
                continue;
            }
        };
        for sheet_box in columns.iter().flatten() {
            if framed {
                items.push((Printed::Title(sheet_box.title.clone()), 20.0));
            } else if !sheet_box.title.is_empty() {
                items.push((Printed::Title(sheet_box.title.clone()), 18.0));
            }
            for block in &sheet_box.body {
                match block {
                    Block::Table(table) => {
                        let n = table
                            .rows
                            .iter()
                            .map(|r| r.iter().map(|c| c.colspan.max(1) as usize).sum())
                            .max()
                            .unwrap_or(0)
                            .max(table.columns);
                        let mut widths = vec![0.0f64; n];
                        for row in &table.rows {
                            let mut c = 0;
                            for cell in row {
                                let span = cell.colspan.max(1) as usize;
                                if span == 1 && c < n {
                                    widths[c] = widths[c].max(measure.measure(&cell.text, None).0);
                                }
                                c += span;
                            }
                        }
                        // Then widen columns so that spanning cells fit.
                        for row in &table.rows {
                            let mut c = 0;
                            for cell in row {
                                let span = cell.colspan.max(1) as usize;
                                let end = (c + span).min(n);
                                if span > 1 && c < end {
                                    let have: f64 = widths[c..end].iter().sum::<f64>()
                                        + GAP * (end - c - 1) as f64;
                                    let need = measure.measure(&cell.text, None).0;
                                    if need > have {
                                        let extra = (need - have) / (end - c) as f64;
                                        widths[c..end].iter_mut().for_each(|w| *w += extra);
                                    }
                                }
                                c += span;
                            }
                        }
                        let total: f64 = widths.iter().sum::<f64>() + GAP * n as f64;
                        if total > width {
                            let scale = (width - GAP * n as f64).max(1.0)
                                / widths.iter().sum::<f64>().max(1.0);
                            widths.iter_mut().for_each(|w| *w *= scale);
                        }
                        let widths = Rc::new(widths);
                        for row in &table.rows {
                            let mut c = 0;
                            let mut cells = Vec::new();
                            let mut height: f64 = 10.0;
                            for cell in row {
                                let span = cell.colspan.max(1) as usize;
                                let w: f64 =
                                    widths[c.min(n)..(c + span).min(n)].iter().sum::<f64>()
                                        + GAP * (span - 1) as f64;
                                height =
                                    height.max(measure.measure(&cell.text, Some(w.max(1.0))).1);
                                cells.push(RowCell {
                                    text: cell.text.clone(),
                                    column: c,
                                    span,
                                    right: cell.align == Align::Right,
                                    header: cell.kind == CellKind::Header,
                                });
                                c += span;
                            }
                            items.push((Printed::Row(cells, widths.clone()), height + 2.0));
                        }
                    }
                    Block::List(list) => {
                        let mut lines = Vec::new();
                        flatten(list, 0, &mut lines);
                        for (text, depth) in lines {
                            let indent = 12.0 * depth as f64;
                            let h = measure.measure(&text, Some(width - indent)).1;
                            items.push((Printed::Text(text, indent), h + 1.0));
                        }
                    }
                    Block::Paragraph(text) => {
                        let text = text.replace('\t', "    ");
                        let h = measure.measure(&text, Some(width)).1;
                        items.push((Printed::Text(text, 0.0), h + 2.0));
                    }
                    Block::Lines(lines) => {
                        let text = lines.join("\n");
                        let h = measure.measure(&text, Some(width)).1;
                        items.push((Printed::Text(text, 0.0), h + 2.0));
                    }
                    Block::Image { file, .. } => {
                        // At most 2.5 inches high, as wide as it fits.
                        if let Some(picture) = file.as_deref().and_then(|f| picture(f, width)) {
                            let h = picture.height;
                            items.push((Printed::Picture(picture), h + 4.0));
                        }
                    }
                }
            }
            if framed {
                items.push((Printed::BoxEnd, 6.0));
            }
            items.push((Printed::Text(String::new(), 0.0), 8.0));
        }
    }
    items
}

fn flatten(items: &[ListItem], depth: usize, out: &mut Vec<(String, usize)>) {
    for item in items {
        out.push((item.text.clone(), depth));
        if let Some(children) = &item.children {
            flatten(children, depth + 1, out);
        }
    }
}

/// A bitmap of the original, decoded here as the GTK app does, at most
/// 2.5 inches high and `width` wide.
fn picture(path: &Path, width: f64) -> Option<Picture> {
    let image = cr_formats::bmp::decode(&std::fs::read(path).ok()?)?;
    let (w, h) = (f64::from(image.width), f64::from(image.height));
    let scale = (180.0 / h).min(width / w).min(1.0);
    Some(Picture {
        rgba: Rc::new(image.rgba),
        pixels: (image.width, image.height),
        width: w * scale,
        height: h * scale,
    })
}

/// One printed page: the index of each item on it and where it starts,
/// and whether a framed box from the page before runs on into it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Page {
    pub placed: Vec<(usize, f64)>,
    pub continued: bool,
}

/// Puts the laid-out items on pages `height` high (the footer left out).
pub fn paginate(items: &[(Printed, f64)], height: f64) -> Vec<Page> {
    let mut pages = vec![Page::default()];
    let mut y = 0.0;
    let mut open = false;
    for (i, (item, h)) in items.iter().enumerate() {
        let gap = matches!(item, Printed::Text(t, _) if t.is_empty());
        // Keep a title with what follows it.
        let need = if matches!(item, Printed::Title(_)) {
            h + 40.0
        } else {
            *h
        };
        // A break on a page that has nothing yet, or the gap after a box
        // landing at the top of one, would leave it blank.
        if matches!(item, Printed::PageBreak) {
            if y > 0.0 {
                pages.push(Page::default());
                y = 0.0;
            }
            continue;
        }
        if y == 0.0 && gap {
            continue;
        }
        // A box's closing line and the gap after it stay with what came
        // before, even a little past the bottom.
        let trailing = matches!(item, Printed::BoxEnd) || gap;
        if y + need > height && y > 0.0 && !trailing {
            pages.push(Page {
                placed: Vec::new(),
                continued: open,
            });
            y = 0.0;
        }
        match item {
            Printed::Title(_) => open = true,
            Printed::BoxEnd => open = false,
            _ => {}
        }
        pages.last_mut().unwrap().placed.push((i, y));
        y += h;
    }
    pages
}

/// Where each column of a table starts.
pub fn column_starts(widths: &[f64]) -> Vec<f64> {
    widths
        .iter()
        .scan(0.0, |x, w| {
            let start = *x;
            *x += w + GAP;
            Some(start)
        })
        .collect()
}

/// The width of a cell spanning `span` columns from `column`.
pub fn cell_width(widths: &[f64], column: usize, span: usize) -> f64 {
    let end = (column + span).min(widths.len());
    widths[column.min(end)..end].iter().sum::<f64>() + GAP * end.saturating_sub(column + 1) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use cr_store::sheet::{Cell, Flavor, Section, SheetBox, Table};

    /// Every character 5 points wide, every line 10 high.
    struct Fixed;

    impl Measure for Fixed {
        fn measure(&self, text: &str, wrap: Option<f64>) -> (f64, f64) {
            let width = 5.0 * text.chars().count() as f64;
            let lines = match wrap {
                Some(w) if width > w => (width / w).ceil(),
                _ => 1.0,
            };
            (width.min(wrap.unwrap_or(f64::MAX)), 10.0 * lines)
        }
    }

    fn sheet(boxes: Vec<SheetBox>, border: Option<Border>) -> Sheet {
        Sheet {
            title: "Lirael".into(),
            flavor: Flavor::Expansion,
            border,
            sections: vec![Section {
                id: 0,
                content: SectionContent::Columns(vec![boxes]),
            }],
        }
    }

    fn table_box(title: &str, rows: usize) -> SheetBox {
        SheetBox {
            title: title.into(),
            body: vec![Block::Table(Table {
                columns: 2,
                rows: (0..rows)
                    .map(|i| vec![Cell::header(format!("Row {i}")), Cell::new("12").right()])
                    .collect(),
            })],
        }
    }

    #[test]
    fn columns_fit_their_widest_cell() {
        let items = items(&sheet(vec![table_box("Abilities", 2)], None), &Fixed, 500.0);
        let Printed::Row(cells, widths) = &items[1].0 else {
            panic!("{items:?}")
        };
        assert_eq!(**widths, [25.0, 10.0]);
        assert!(cells[0].header && cells[1].right);
        assert_eq!(column_starts(widths), [0.0, 31.0]);
        assert_eq!(cell_width(widths, 0, 2), 41.0);
    }

    #[test]
    fn a_long_box_runs_on_to_the_next_page_with_its_frame() {
        let laid = items(
            &sheet(vec![table_box("Items", 30)], Some(Border::Normal)),
            &Fixed,
            500.0,
        );
        let pages = paginate(&laid, 200.0);
        assert_eq!(pages.len(), 2);
        assert!(!pages[0].continued && pages[1].continued);
        // The title and the rows that fit are on the first page.
        assert_eq!(pages[0].placed[0], (0, 0.0));
        assert!(pages[0].placed.iter().all(|(_, y)| *y < 200.0));
    }

    #[test]
    fn a_title_does_not_end_a_page() {
        let laid = items(
            &sheet(vec![table_box("First", 14), table_box("Second", 2)], None),
            &Fixed,
            500.0,
        );
        let pages = paginate(&laid, 200.0);
        let second = laid
            .iter()
            .position(|(p, _)| *p == Printed::Title("Second".into()))
            .unwrap();
        assert_eq!(pages[1].placed[0], (second, 0.0));
    }
}
