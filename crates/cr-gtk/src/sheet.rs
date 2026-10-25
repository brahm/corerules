//! The character sheet: shown in a dialog, printed or saved as PDF through
//! GTK's print support, exported as the original programs' TXT and HTML
//! files, or exported as Markdown.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_app::action;
use cr_app::pages::{
    self, FOOTER, FRAME_PAD, Measure, Page, Printed, cell_width, column_starts, frame_pen,
};
use cr_app::pdf::Paper;
use cr_i18n::{tr, tr_noop};
use cr_store::help::topic;
use cr_store::sheet::{
    Align, Block, Border, CellKind, ListItem, Sheet, SheetBox, SheetChoice, Table, Timestamp,
};
use gtk::{gio, glib, pango};

/// How the sheet dialog changes the character's sheet options: the
/// choice it starts from, and what saves a new one and builds the sheet
/// again.
pub struct Options {
    pub choice: SheetChoice,
    pub save: SaveChoice,
}

/// Saves a choice and builds the sheet it gives.
pub type SaveChoice = Box<dyn Fn(&SheetChoice) -> Result<Sheet, String>>;

/// Opens a dialog with `sheet` over `parent`; with `options`, its
/// Character Sheet Options button changes the sections and the border;
/// `toast` reports exports.
pub fn show(
    parent: &impl IsA<gtk::Widget>,
    sheet: Sheet,
    options: Option<Options>,
    toast: impl Fn(&str) + 'static,
) {
    let title = sheet.title.clone();
    let flavor = sheet.flavor;
    let current = Rc::new(RefCell::new(Rc::new(sheet)));

    let print = gtk::Button::builder()
        .icon_name("printer-symbolic")
        .tooltip_text(tr("Print"))
        .action_name(action::SHEET_PRINT.detailed)
        .build();
    let menu = gio::Menu::new();
    menu.append(Some(tr("Save as _PDF…")), Some(action::SHEET_PDF.detailed));
    menu.append(
        Some(tr("Export as _HTML…")),
        Some(action::SHEET_HTML.detailed),
    );
    if flavor == cr_store::sheet::Flavor::Expansion {
        menu.append(
            Some(tr("Export as _Text…")),
            Some(action::SHEET_TXT.detailed),
        );
    }
    menu.append(
        Some(tr("Export as _Markdown…")),
        Some(action::SHEET_MD.detailed),
    );
    let export = gtk::MenuButton::builder()
        .icon_name("document-save-symbolic")
        .tooltip_text(tr("Save or Export"))
        .menu_model(&menu)
        .build();
    let header = adw::HeaderBar::new();
    header.pack_end(&export);
    header.pack_end(&print);
    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, || topic::CHARACTER_SHEET);
    view.add_top_bar(&header);
    view.set_content(Some(&content(&current.borrow())));
    let dialog = adw::Dialog::builder()
        .title(&title)
        .content_width(860)
        .content_height(720)
        .child(&view)
        .build();

    let toast: Rc<dyn Fn(&str)> = Rc::new(toast);
    if let Some(options) = options {
        let button = gtk::Button::builder()
            .icon_name("view-list-symbolic")
            .tooltip_text(tr("Character Sheet Options"))
            .build();
        header.pack_start(&button);
        let options = Rc::new(RefCell::new(options));
        let (dialog, current, view, toast) =
            (dialog.clone(), current.clone(), view.clone(), toast.clone());
        button.connect_clicked(move |_| {
            let choice = options.borrow().choice.clone();
            let (options, current, view, toast) = (
                options.clone(),
                current.clone(),
                view.clone(),
                toast.clone(),
            );
            crate::sheet_options::choose(&dialog, choice, move |choice| {
                let built = (options.borrow().save)(&choice);
                match built {
                    Ok(sheet) => {
                        options.borrow_mut().choice = choice;
                        view.set_content(Some(&content(&sheet)));
                        *current.borrow_mut() = Rc::new(sheet);
                    }
                    Err(e) => toast(&e),
                }
            });
        });
    }

    let actions = gio::SimpleActionGroup::new();
    let add = |name: &str, run: Box<dyn Fn()>| {
        let a = gio::SimpleAction::new(name, None);
        a.connect_activate(move |_, _| run());
        actions.add_action(&a);
    };
    add(action::SHEET_PRINT.name, {
        let (current, dialog, toast) = (current.clone(), dialog.clone(), toast.clone());
        Box::new(move || print_sheet(&dialog, &current.borrow().clone(), &*toast))
    });
    add(action::SHEET_PDF.name, {
        let (current, dialog, toast) = (current.clone(), dialog.clone(), toast.clone());
        Box::new(move || ask_pdf(&dialog, &current.borrow().clone(), toast.clone()))
    });
    for (name, extension, kind) in [
        (action::SHEET_HTML.name, "html", tr_noop!("HTML documents")),
        (action::SHEET_TXT.name, "txt", tr_noop!("Text files")),
        (action::SHEET_MD.name, "md", tr_noop!("Markdown documents")),
    ] {
        add(name, {
            let (current, dialog, toast) = (current.clone(), dialog.clone(), toast.clone());
            Box::new(move || {
                let (sheet, toast) = (current.borrow().clone(), toast.clone());
                save_file(
                    &dialog,
                    &sheet.title.clone(),
                    extension,
                    tr(kind),
                    move |path| {
                        let date = Timestamp::now_utc().format(sheet.flavor);
                        let bytes = match extension {
                            "md" => sheet.to_markdown(&date).into_bytes(),
                            _ => sheet.export(extension == "html", &date),
                        };
                        match std::fs::write(&path, bytes) {
                            Ok(()) => toast(tr("Sheet exported")),
                            Err(e) => toast(&tr!(
                                "Could not save {file}: {error}",
                                file = path.display(),
                                error = e
                            )),
                        }
                    },
                );
            })
        });
    }
    dialog.insert_action_group("sheet", Some(&actions));
    dialog.present(Some(parent));
}

/// Shows `sheet` over `parent` as the original's print preview does, to
/// print or save as PDF; `toast` reports the result.
pub fn preview(parent: &impl IsA<gtk::Widget>, sheet: Sheet, toast: impl Fn(&str) + 'static) {
    let sheet = Rc::new(sheet);
    let print = crate::print::button();
    let pdf = gtk::Button::builder()
        .icon_name("document-save-symbolic")
        .tooltip_text(tr("Save as PDF"))
        .build();
    let header = adw::HeaderBar::new();
    header.pack_start(&print);
    header.pack_start(&pdf);
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&content(&sheet)));
    let dialog = adw::Dialog::builder()
        .title(&sheet.title)
        .content_width(860)
        .content_height(720)
        .child(&view)
        .build();
    let toast: Rc<dyn Fn(&str)> = Rc::new(toast);
    print.connect_clicked({
        let (sheet, dialog, toast) = (sheet.clone(), dialog.clone(), toast.clone());
        move |_| print_sheet(&dialog, &sheet, &*toast)
    });
    pdf.connect_clicked({
        let (sheet, dialog, toast) = (sheet.clone(), dialog.clone(), toast.clone());
        move |_| ask_pdf(&dialog, &sheet, toast.clone())
    });
    dialog.present(Some(parent));
}

/// The sheet's sections, scrolling.
fn content(sheet: &Sheet) -> gtk::ScrolledWindow {
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
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .vexpand(true)
        .build()
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
        .title(tr("Save As"))
        .initial_name(cr_app::file_name(title, extension))
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
    // A heading alone, as the group encounter printout's title.
    if sheet_box.body.is_empty() {
        return outer.upcast();
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
            Block::Image { file, .. } => match file.as_deref().and_then(|f| picture(f, 200)) {
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
                .label(tr("None"))
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

/// A picture file, decoded here since the bitmaps of the original are
/// not something every system's image loaders read; `None` when it does
/// not decode.
pub(crate) fn picture(path: &std::path::Path, height: i32) -> Option<gtk::Widget> {
    let texture = texture(path)?;
    Some(
        gtk::Picture::builder()
            .halign(gtk::Align::Start)
            .height_request(height.min(texture.height()))
            .paintable(&texture)
            .can_shrink(true)
            .build()
            .upcast(),
    )
}

/// A bitmap file as a texture.
pub(crate) fn texture(path: &std::path::Path) -> Option<gtk::gdk::MemoryTexture> {
    let image = cr_formats::bmp::decode(&std::fs::read(path).ok()?)?;
    Some(memory_texture(&image.rgba, (image.width, image.height)))
}

/// RGBA pixels as a texture.
fn memory_texture(rgba: &[u8], (width, height): (u32, u32)) -> gtk::gdk::MemoryTexture {
    gtk::gdk::MemoryTexture::new(
        width as i32,
        height as i32,
        gtk::gdk::MemoryFormat::R8g8b8a8,
        &glib::Bytes::from(rgba),
        width as usize * 4,
    )
}

const FONT: &str = "Sans 8";
const TITLE_FONT: &str = "Sans Bold 10";

/// Measures the sheet's text with Pango, for [`cr_app::pages`].
struct PangoMeasure(pango::Layout);

impl Measure for PangoMeasure {
    fn measure(&self, text: &str, wrap: Option<f64>) -> (f64, f64) {
        self.0.set_text(text);
        self.0
            .set_width(wrap.map_or(-1, |w| (w * f64::from(pango::SCALE)) as i32));
        let (_, logical) = self.0.extents();
        (
            f64::from(logical.width()) / f64::from(pango::SCALE),
            f64::from(logical.height()) / f64::from(pango::SCALE),
        )
    }
}

/// Saves the sheet's PDF at `path`, made by [`cr_app::pdf`] as every
/// version of the app makes it, on the paper GTK takes for the user's
/// region.
pub(crate) fn save_pdf(sheet: &Sheet, path: &std::path::Path, toast: &dyn Fn(&str)) {
    let paper = match gtk::PaperSize::new(None).name().as_str() {
        "na_letter" | "na_legal" => Paper::Letter,
        _ => Paper::A4,
    };
    match std::fs::write(path, cr_app::pdf::sheet(sheet, paper)) {
        Ok(()) => toast(tr("Sheet saved as PDF")),
        Err(e) => toast(&tr!(
            "Could not save {file}: {error}",
            file = path.display(),
            error = e
        )),
    }
}

/// Asks where to save the sheet's PDF, then saves it.
pub(crate) fn ask_pdf(parent: &impl IsA<gtk::Widget>, sheet: &Rc<Sheet>, toast: Rc<dyn Fn(&str)>) {
    let sheet = sheet.clone();
    save_file(
        parent,
        &sheet.title.clone(),
        "pdf",
        tr("PDF documents"),
        move |path| save_pdf(&sheet, &path, &*toast),
    );
}

/// Prints the sheet through the print dialog.
pub(crate) fn print_sheet(parent: &impl IsA<gtk::Widget>, sheet: &Rc<Sheet>, toast: &dyn Fn(&str)) {
    let operation = gtk::PrintOperation::builder()
        .job_name(&sheet.title)
        .embed_page_setup(true)
        .use_full_page(false)
        .unit(gtk::Unit::Points)
        .build();
    let pages: Rc<RefCell<Vec<Page>>> = Rc::default();
    let items: Rc<RefCell<Vec<(Printed, f64)>>> = Rc::default();
    operation.connect_begin_print({
        let (sheet, pages, items) = (sheet.clone(), pages.clone(), items.clone());
        move |operation, context| {
            let layout = context.create_pango_layout();
            layout.set_font_description(Some(&pango::FontDescription::from_string(FONT)));
            // The foot of each page is left for the footer.
            let laid = pages::items(&sheet, &PangoMeasure(layout), context.width());
            let all = pages::paginate(&laid, context.height() - FOOTER);
            operation.set_n_pages(all.len() as i32);
            *pages.borrow_mut() = all;
            *items.borrow_mut() = laid;
        }
    });
    operation.connect_draw_page({
        let (pages, items, title) = (pages.clone(), items.clone(), sheet.title.clone());
        let border = sheet.border;
        move |_, context, page| {
            let cr = context.cairo_context();
            let layout = context.create_pango_layout();
            let font = pango::FontDescription::from_string(FONT);
            let bold = pango::FontDescription::from_string(TITLE_FONT);
            let width = context.width();
            let items = items.borrow();
            let pen = frame_pen(border);
            let pad = if pen.is_some() { FRAME_PAD } else { 0.0 };
            let line = |x0: f64, y0: f64, x1: f64, y1: f64| {
                cr.move_to(x0, y0);
                cr.line_to(x1, y1);
                let _ = cr.stroke();
            };
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
            let pages = pages.borrow();
            let Some(this) = pages.get(page as usize) else {
                return;
            };
            // Where the open box's frame starts on this page.
            let mut open = this.continued.then_some(0.0);
            if let Some(pen) = pen {
                cr.set_line_width(pen);
            }
            for &(i, y) in &this.placed {
                match &items[i].0 {
                    Printed::Title(text) => match border {
                        // The other pages: a rule under the title.
                        None => {
                            draw(text, 0.0, y, None, false, true);
                            cr.set_line_width(0.5);
                            line(0.0, y + 15.0, width, y + 15.0);
                        }
                        Some(Border::None) => draw(text, 0.0, y, None, false, true),
                        // A rectangle with the title inside.
                        Some(Border::Thin) => {
                            draw(text, pad, y + 3.0, None, false, true);
                            line(0.0, y, width, y);
                            open = Some(y);
                        }
                        // A frame whose top line runs through the title.
                        Some(Border::Normal) => {
                            let top = y + 7.0;
                            if text.is_empty() {
                                line(0.0, top, width, top);
                            } else {
                                draw(text, pad + 4.0, y, None, false, true);
                                let (_, logical) = layout.extents();
                                let w = f64::from(logical.width()) / f64::from(pango::SCALE);
                                line(0.0, top, pad, top);
                                line(pad + 8.0 + w, top, width, top);
                            }
                            open = Some(top);
                        }
                    },
                    Printed::BoxEnd => {
                        if let Some(top) = open.take() {
                            let bottom = y + 3.0;
                            line(0.0, top, 0.0, bottom);
                            line(width, top, width, bottom);
                            line(0.0, bottom, width, bottom);
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
                                cr.set_source_rgb(0.35, 0.35, 0.35);
                            }
                            let x = pad + starts[cell.column];
                            draw(&cell.text, x, y, Some(w), cell.right, false);
                            cr.set_source_rgb(0.0, 0.0, 0.0);
                        }
                    }
                    Printed::Text(text, indent) => draw(
                        text,
                        pad + indent,
                        y,
                        Some(width - 2.0 * pad - indent),
                        false,
                        false,
                    ),
                    Printed::Picture(picture) => {
                        let texture = memory_texture(&picture.rgba, picture.pixels);
                        crate::print::draw_texture(
                            &cr,
                            texture.upcast_ref(),
                            pad,
                            y,
                            picture.width,
                            picture.height,
                        )
                    }
                    Printed::PageBreak => {}
                }
            }
            // A box that runs on to the next page keeps its sides to the
            // foot of this one.
            if let Some(top) = open {
                let bottom = context.height() - FOOTER;
                line(0.0, top, 0.0, bottom);
                line(width, top, width, bottom);
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
    crate::print::report_failure(&operation, parent);
    let window = parent.root().and_downcast::<gtk::Window>();
    if let Err(e) = operation.run(gtk::PrintOperationAction::PrintDialog, window.as_ref()) {
        toast(&tr!("Could not print: {error}", error = e));
    }
}
