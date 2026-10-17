//! The character sheet: shown in a dialog, printed or saved as PDF through
//! GTK's print support, and exported as the original programs' TXT and
//! HTML files.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_store::sheet::{Align, Block, CellKind, ListItem, Sheet, SheetBox, Table, Timestamp};
use gtk::{gio, pango};

/// Opens a dialog with `sheet` over `parent`; `toast` reports exports.
pub fn show(parent: &impl IsA<gtk::Widget>, sheet: Sheet, toast: impl Fn(&str) + 'static) {
    let sheet = Rc::new(sheet);
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(18)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();
    for section in &sheet.sections {
        if let cr_store::sheet::SectionContent::Columns(columns) = &section.content {
            content.append(&columns_widget(columns));
        }
    }
    let clamp = adw::Clamp::builder()
        .maximum_size(900)
        .child(&content)
        .build();
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .vexpand(true)
        .build();

    let print = gtk::Button::builder()
        .icon_name("printer-symbolic")
        .tooltip_text("Print")
        .action_name("sheet.print")
        .build();
    let menu = gio::Menu::new();
    menu.append(Some("Save as _PDF…"), Some("sheet.pdf"));
    menu.append(Some("Export as _HTML…"), Some("sheet.html"));
    if sheet.flavor == cr_store::sheet::Flavor::Expansion {
        menu.append(Some("Export as _Text…"), Some("sheet.txt"));
    }
    let export = gtk::MenuButton::builder()
        .icon_name("document-save-symbolic")
        .tooltip_text("Save or Export")
        .menu_model(&menu)
        .build();
    let header = adw::HeaderBar::new();
    header.pack_end(&export);
    header.pack_end(&print);
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&scroll));
    let dialog = adw::Dialog::builder()
        .title(&sheet.title)
        .content_width(860)
        .content_height(720)
        .child(&view)
        .build();

    let toast: Rc<dyn Fn(&str)> = Rc::new(toast);
    let actions = gio::SimpleActionGroup::new();
    let action = |name: &str, run: Box<dyn Fn()>| {
        let a = gio::SimpleAction::new(name, None);
        a.connect_activate(move |_, _| run());
        actions.add_action(&a);
    };
    action("print", {
        let (sheet, dialog, toast) = (sheet.clone(), dialog.clone(), toast.clone());
        Box::new(move || print_sheet(&dialog, &sheet, None, &*toast))
    });
    action("pdf", {
        let (sheet, dialog, toast) = (sheet.clone(), dialog.clone(), toast.clone());
        Box::new(move || {
            let (sheet, dialog2, toast) = (sheet.clone(), dialog.clone(), toast.clone());
            let title = sheet.title.clone();
            save_file(&dialog, &title, "pdf", "PDF documents", move |path| {
                print_sheet(&dialog2, &sheet, Some(path), &*toast)
            });
        })
    });
    for (name, extension, kind) in [
        ("html", "html", "HTML documents"),
        ("txt", "txt", "Text files"),
    ] {
        action(name, {
            let (sheet, dialog, toast) = (sheet.clone(), dialog.clone(), toast.clone());
            Box::new(move || {
                let (sheet, toast) = (sheet.clone(), toast.clone());
                save_file(
                    &dialog,
                    &sheet.title.clone(),
                    extension,
                    kind,
                    move |path| {
                        let date = Timestamp::now_utc().format(sheet.flavor);
                        let bytes = sheet.export(extension == "html", &date);
                        match std::fs::write(&path, bytes) {
                            Ok(()) => toast("Sheet exported"),
                            Err(e) => toast(&format!("Could not save {}: {e}", path.display())),
                        }
                    },
                );
            })
        });
    }
    dialog.insert_action_group("sheet", Some(&actions));
    dialog.present(Some(parent));
}

/// Asks where to save, suggesting the file name the original uses
/// (`Borin_Pedraforte.txt`).
pub(crate) fn save_file(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    extension: &'static str,
    kind: &str,
    chosen: impl Fn(std::path::PathBuf) + 'static,
) {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(kind));
    filter.add_suffix(extension);
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    let dialog = gtk::FileDialog::builder()
        .title("Save As")
        .initial_name(format!("{}.{extension}", title.replace(' ', "_")))
        .filters(&filters)
        .modal(true)
        .build();
    let window = parent.root().and_downcast::<gtk::Window>();
    dialog.save(window.as_ref(), gio::Cancellable::NONE, move |result| {
        if let Ok(Some(path)) = result.map(|f| f.path()) {
            chosen(path);
        }
    });
}

/// A row of columns, each a stack of boxes; columns stack on narrow
/// screens.
fn columns_widget(columns: &[Vec<SheetBox>]) -> gtk::Widget {
    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .homogeneous(true)
        .column_spacing(18)
        .row_spacing(18)
        .min_children_per_line(1)
        .max_children_per_line(columns.len().max(1) as u32)
        .build();
    for column in columns {
        let stack = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(18)
            .valign(gtk::Align::Start)
            .build();
        for b in column {
            stack.append(&box_widget(b));
        }
        let child = gtk::FlowBoxChild::builder()
            .child(&stack)
            .focusable(false)
            .build();
        flow.append(&child);
    }
    flow.upcast()
}

fn box_widget(sheet_box: &SheetBox) -> gtk::Widget {
    let outer = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .build();
    if !sheet_box.title.is_empty() {
        outer.append(
            &gtk::Label::builder()
                .label(&sheet_box.title)
                .xalign(0.0)
                .css_classes(["heading"])
                .build(),
        );
    }
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .css_classes(["card"])
        .build();
    let inner = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(12)
        .margin_end(12)
        .build();
    card.append(&inner);
    let mut empty = true;
    for block in &sheet_box.body {
        let widget = match block {
            Block::Table(table) if !table.rows.is_empty() => table_widget(table),
            Block::List(items) if !items.is_empty() => list_widget(items, 0),
            Block::Paragraph(text) if !text.trim().is_empty() => {
                text_label(text.trim_end().replace('\t', "    ").as_str(), true)
            }
            Block::Lines(lines) if !lines.is_empty() => text_label(&lines.join("\n"), true),
            Block::Image(path) => match portrait(path) {
                Some(picture) => picture,
                None => continue,
            },
            _ => continue,
        };
        inner.append(&widget);
        empty = false;
    }
    if empty {
        inner.append(
            &gtk::Label::builder()
                .label("None")
                .xalign(0.0)
                .css_classes(["dim-label"])
                .build(),
        );
    }
    outer.append(&card);
    outer.upcast()
}

fn text_label(text: &str, wrap: bool) -> gtk::Widget {
    gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .wrap(wrap)
        .wrap_mode(pango::WrapMode::WordChar)
        .selectable(true)
        .build()
        .upcast()
}

fn table_widget(table: &Table) -> gtk::Widget {
    let grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(4)
        .build();
    for (r, row) in table.rows.iter().enumerate() {
        let mut column = 0;
        for cell in row {
            let span = cell.colspan.max(1) as i32;
            if !cell.text.is_empty() {
                let label = gtk::Label::builder()
                    .label(&cell.text)
                    .wrap(true)
                    .wrap_mode(pango::WrapMode::WordChar)
                    .xalign(match cell.align {
                        Align::Right => 1.0,
                        Align::Center => 0.5,
                        _ => 0.0,
                    })
                    .hexpand(span > 1)
                    .build();
                label.add_css_class("numeric");
                match cell.kind {
                    CellKind::Header => label.add_css_class("dim-label"),
                    CellKind::Large => label.add_css_class("title-3"),
                    _ => {}
                }
                grid.attach(&label, column, r as i32, span, 1);
            }
            column += span;
        }
    }
    grid.upcast()
}

fn list_widget(items: &[ListItem], depth: i32) -> gtk::Widget {
    let list = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .margin_start(depth * 18)
        .build();
    for item in items {
        list.append(&text_label(&item.text, true));
        if let Some(children) = &item.children {
            list.append(&list_widget(children, depth + 1));
        }
    }
    list.upcast()
}

/// The portrait, when its file is on this computer. The original stores
/// a Windows path; only its file name is looked for next to the
/// installation's pictures, so most portraits are not shown.
fn portrait(path: &str) -> Option<gtk::Widget> {
    let path = std::path::Path::new(path);
    path.exists().then(|| {
        gtk::Picture::builder()
            .file(&gio::File::for_path(path))
            .halign(gtk::Align::Start)
            .height_request(200)
            .build()
            .upcast()
    })
}

/// One thing drawn on a printed page.
enum Printed {
    Title(String),
    /// Rows of cells (text, column, span, right-aligned), with the
    /// column widths of the table in points.
    Row(Vec<(String, usize, usize, bool, bool)>, Rc<Vec<f64>>),
    Text(String, f64),
    PageBreak,
}

const FONT: &str = "Sans 8";
const TITLE_FONT: &str = "Sans Bold 10";

/// Lays the sheet out for printing: boxes one under the other, tables
/// with their columns sized to fit the page.
fn print_items(sheet: &Sheet, context: &gtk::PrintContext, width: f64) -> Vec<(Printed, f64)> {
    let layout = context.create_pango_layout();
    let font = pango::FontDescription::from_string(FONT);
    layout.set_font_description(Some(&font));
    let measure = |text: &str, wrap: Option<f64>| {
        layout.set_text(text);
        layout.set_width(wrap.map_or(-1, |w| (w * f64::from(pango::SCALE)) as i32));
        let (_, logical) = layout.extents();
        (
            f64::from(logical.width()) / f64::from(pango::SCALE),
            f64::from(logical.height()) / f64::from(pango::SCALE),
        )
    };
    let gap = 6.0;
    let mut items = Vec::new();
    for section in &sheet.sections {
        let columns = match &section.content {
            cr_store::sheet::SectionContent::Columns(columns) => columns,
            cr_store::sheet::SectionContent::PageBreak => {
                items.push((Printed::PageBreak, 0.0));
                continue;
            }
        };
        for sheet_box in columns.iter().flatten() {
            if !sheet_box.title.is_empty() {
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
                                    widths[c] = widths[c].max(measure(&cell.text, None).0);
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
                                        + gap * (end - c - 1) as f64;
                                    let need = measure(&cell.text, None).0;
                                    if need > have {
                                        let extra = (need - have) / (end - c) as f64;
                                        widths[c..end].iter_mut().for_each(|w| *w += extra);
                                    }
                                }
                                c += span;
                            }
                        }
                        let total: f64 = widths.iter().sum::<f64>() + gap * n as f64;
                        if total > width {
                            let scale = (width - gap * n as f64).max(1.0)
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
                                        + gap * (span - 1) as f64;
                                height = height.max(measure(&cell.text, Some(w.max(1.0))).1);
                                cells.push((
                                    cell.text.clone(),
                                    c,
                                    span,
                                    cell.align == Align::Right,
                                    cell.kind == CellKind::Header,
                                ));
                                c += span;
                            }
                            items.push((Printed::Row(cells, widths.clone()), height + 2.0));
                        }
                    }
                    Block::List(list) => {
                        fn flatten(
                            items: &[ListItem],
                            depth: usize,
                            out: &mut Vec<(String, usize)>,
                        ) {
                            for item in items {
                                out.push((item.text.clone(), depth));
                                if let Some(children) = &item.children {
                                    flatten(children, depth + 1, out);
                                }
                            }
                        }
                        let mut lines = Vec::new();
                        flatten(list, 0, &mut lines);
                        for (text, depth) in lines {
                            let indent = 12.0 * depth as f64;
                            let h = measure(&text, Some(width - indent)).1;
                            items.push((Printed::Text(text, indent), h + 1.0));
                        }
                    }
                    Block::Paragraph(text) => {
                        let text = text.replace('\t', "    ");
                        let h = measure(&text, Some(width)).1;
                        items.push((Printed::Text(text, 0.0), h + 2.0));
                    }
                    Block::Lines(lines) => {
                        let text = lines.join("\n");
                        let h = measure(&text, Some(width)).1;
                        items.push((Printed::Text(text, 0.0), h + 2.0));
                    }
                    Block::Image(_) => {}
                }
            }
            items.push((Printed::Text(String::new(), 0.0), 8.0));
        }
    }
    items
}

/// The items of one page: their index and vertical position.
type Pages = Vec<(usize, f64)>;

/// Prints the sheet, or with `pdf` saves it there as a PDF.
pub(crate) fn print_sheet(
    parent: &impl IsA<gtk::Widget>,
    sheet: &Rc<Sheet>,
    pdf: Option<std::path::PathBuf>,
    toast: &dyn Fn(&str),
) {
    let operation = gtk::PrintOperation::builder()
        .job_name(&sheet.title)
        .embed_page_setup(true)
        .use_full_page(false)
        .unit(gtk::Unit::Points)
        .build();
    // Per page, the items on it and where they start.
    let pages: Rc<RefCell<Vec<Pages>>> = Rc::default();
    let items: Rc<RefCell<Vec<(Printed, f64)>>> = Rc::default();
    operation.connect_begin_print({
        let (sheet, pages, items) = (sheet.clone(), pages.clone(), items.clone());
        move |operation, context| {
            let (width, height) = (context.width(), context.height());
            let laid = print_items(&sheet, context, width);
            let mut all = vec![Vec::new()];
            let mut y = 0.0;
            for (i, (item, h)) in laid.iter().enumerate() {
                let page_break = matches!(item, Printed::PageBreak);
                // Keep a title with what follows it.
                let need = if matches!(item, Printed::Title(_)) {
                    h + 40.0
                } else {
                    *h
                };
                if page_break || (y + need > height && y > 0.0) {
                    all.push(Vec::new());
                    y = 0.0;
                    if page_break {
                        continue;
                    }
                }
                all.last_mut().unwrap().push((i, y));
                y += h;
            }
            operation.set_n_pages(all.len() as i32);
            *pages.borrow_mut() = all;
            *items.borrow_mut() = laid;
        }
    });
    operation.connect_draw_page({
        let (pages, items, title) = (pages.clone(), items.clone(), sheet.title.clone());
        move |_, context, page| {
            let cr = context.cairo_context();
            let layout = context.create_pango_layout();
            let font = pango::FontDescription::from_string(FONT);
            let bold = pango::FontDescription::from_string(TITLE_FONT);
            let width = context.width();
            let items = items.borrow();
            let draw = |text: &str, x: f64, y: f64, w: Option<f64>, right: bool, strong: bool| {
                layout.set_font_description(Some(if strong { &bold } else { &font }));
                layout.set_text(text);
                layout.set_width(w.map_or(-1, |w| (w * f64::from(pango::SCALE)) as i32));
                layout.set_alignment(if right {
                    pango::Alignment::Right
                } else {
                    pango::Alignment::Left
                });
                cr.move_to(x, y);
                pangocairo::functions::show_layout(&cr, &layout);
            };
            for &(i, y) in pages.borrow().get(page as usize).into_iter().flatten() {
                match &items[i].0 {
                    Printed::Title(text) => {
                        draw(text, 0.0, y, None, false, true);
                        cr.set_line_width(0.5);
                        cr.move_to(0.0, y + 15.0);
                        cr.line_to(width, y + 15.0);
                        let _ = cr.stroke();
                    }
                    Printed::Row(cells, widths) => {
                        let starts: Vec<f64> = widths
                            .iter()
                            .scan(0.0, |x, w| {
                                let start = *x;
                                *x += w + 6.0;
                                Some(start)
                            })
                            .collect();
                        for (text, c, span, right, header) in cells {
                            let (c, span) = (*c, *span);
                            if c >= widths.len() {
                                continue;
                            }
                            let end = (c + span).min(widths.len());
                            let w: f64 =
                                widths[c..end].iter().sum::<f64>() + 6.0 * (end - c - 1) as f64;
                            layout.set_font_description(Some(&font));
                            if *header {
                                cr.set_source_rgb(0.35, 0.35, 0.35);
                            }
                            draw(text, starts[c], y, Some(w.max(1.0)), *right, false);
                            cr.set_source_rgb(0.0, 0.0, 0.0);
                        }
                    }
                    Printed::Text(text, indent) => {
                        draw(text, *indent, y, Some(width - indent), false, false)
                    }
                    Printed::PageBreak => {}
                }
            }
            // Footer: name and page.
            draw(
                &format!("{title} · {}", page + 1),
                0.0,
                context.height() - 10.0,
                Some(width),
                true,
                false,
            );
        }
    });
    let action = match &pdf {
        Some(path) => {
            operation.set_export_filename(path);
            gtk::PrintOperationAction::Export
        }
        None => gtk::PrintOperationAction::PrintDialog,
    };
    let window = parent.root().and_downcast::<gtk::Window>();
    match operation.run(action, window.as_ref()) {
        Ok(gtk::PrintOperationResult::Apply) if pdf.is_some() => toast("Sheet saved as PDF"),
        Ok(_) => {}
        Err(e) => toast(&format!("Could not print: {e}")),
    }
}
