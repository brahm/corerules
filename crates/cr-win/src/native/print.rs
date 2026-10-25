//! Printing the character sheet: the Windows print dialog, then the
//! pages drawn with Direct2D and DirectWrite into a print job, laid out by
//! [`cr_win::pages`] as the GTK app lays them out.

use cr_store::sheet::{Border, Sheet};
use cr_win::pages::{
    self, FONT_SIZE, FOOTER, FRAME_PAD, Measure, Page, Printed, TITLE_SIZE, cell_width,
    column_starts, frame_pen,
};
use windows::Win32::Foundation::{GlobalFree, HGLOBAL};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F,
    D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_NONE, D2D1_BITMAP_PROPERTIES1, D2D1_COLOR_SPACE_SRGB,
    D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_OPTIONS,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_INTERPOLATION_MODE_LINEAR,
    D2D1_PRINT_CONTROL_PROPERTIES, D2D1_PRINT_FONT_SUBSET_MODE_DEFAULT, D2D1CreateFactory,
    ID2D1DeviceContext, ID2D1Factory1, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWRITE_TEXT_METRICS, DWRITE_WORD_WRAPPING_NO_WRAP, DWriteCreateFactory, IDWriteFactory,
    IDWriteTextFormat, IDWriteTextLayout,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Gdi::{
    DEVMODEA, DEVMODEW, DeleteDC, GetDeviceCaps, HDC, HORZRES, LOGPIXELSX, LOGPIXELSY,
    PHYSICALHEIGHT, PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH, VERTRES,
};
use windows::Win32::Graphics::Imaging::{CLSID_WICImagingFactory, IWICImagingFactory};
use windows::Win32::Graphics::Printing::PrintTicket::{
    PTCloseProvider, PTConvertDevModeToPrintTicket, PTOpenProvider, kPTJobScope,
};
use windows::Win32::Storage::Xps::Printing::{
    IPrintDocumentPackageTargetFactory, PrintDocumentPackageTargetFactory,
};
use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, IStream, STREAM_SEEK_SET,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::UI::Controls::Dialogs::{
    DEVNAMES, PD_HIDEPRINTTOFILE, PD_NOCURRENTPAGE, PD_NOPAGENUMS, PD_NOSELECTION, PD_RESULT_PRINT,
    PD_RETURNIC, PD_USEDEVMODECOPIESANDCOLLATE, PRINTDLGEXW, PrintDlgExW, START_PAGE_GENERAL,
};
use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
use windows::core::{HSTRING, Interface, PCWSTR, Result, w};
use windows_numerics::{Matrix3x2, Vector2};

use super::Owner;

/// DIPs (what Direct2D draws in, 96 per inch) per point.
const DIPS: f32 = 96.0 / 72.0;
/// The least margin around the printed area, in DIPs: half an inch.
const MARGIN: f32 = 48.0;
/// The fonts the GTK app's "Sans 8" and "Sans Bold 10" stand for.
const FONT: PCWSTR = w!("Segoe UI");

/// Shows the print dialog over `owner` and prints `sheet` on the printer
/// chosen there; false when the user cancels.
pub fn print_sheet(owner: Owner, sheet: &Sheet) -> Result<bool> {
    let Some(printer) = choose_printer(owner)? else {
        return Ok(false);
    };
    let text = Text::new()?;
    let page = printer.page;
    let (width, height) = (page.content.0 / DIPS, page.content.1 / DIPS);
    let laid = pages::items(sheet, &text, f64::from(width));
    let placed = pages::paginate(&laid, f64::from(height) - FOOTER);

    // SAFETY: COM and Direct2D calls on this thread's apartment, with
    // live interfaces only.
    unsafe {
        let factory: ID2D1Factory1 = D2D1CreateFactory(
            D2D1_FACTORY_TYPE_SINGLE_THREADED,
            Some(&D2D1_FACTORY_OPTIONS::default()),
        )?;
        let device = d3d_device()?;
        let d2d = factory.CreateDevice(&device.cast::<IDXGIDevice>()?)?;
        let context = d2d.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let targets: IPrintDocumentPackageTargetFactory = CoCreateInstance(
            &PrintDocumentPackageTargetFactory,
            None,
            CLSCTX_INPROC_SERVER,
        )?;
        let target = targets.CreateDocumentPackageTargetForPrintJob(
            &HSTRING::from(printer.name.as_str()),
            &HSTRING::from(sheet.title.as_str()),
            None,
            &printer.ticket,
        )?;
        let control = d2d.CreatePrintControl(
            &wic,
            &target,
            Some(&D2D1_PRINT_CONTROL_PROPERTIES {
                fontSubset: D2D1_PRINT_FONT_SUBSET_MODE_DEFAULT,
                rasterDPI: 150.0,
                colorSpace: D2D1_COLOR_SPACE_SRGB,
            }),
        )?;
        let painter = Painter {
            context: &context,
            text: &text,
            black: context.CreateSolidColorBrush(&color(0.0), None)?,
            gray: context.CreateSolidColorBrush(&color(0.35), None)?,
            width,
            height,
            border: sheet.border,
        };
        for (number, page_items) in placed.iter().enumerate() {
            let list = context.CreateCommandList()?;
            context.SetTarget(&list);
            context.BeginDraw();
            context.SetTransform(&Matrix3x2 {
                M11: DIPS,
                M12: 0.0,
                M21: 0.0,
                M22: DIPS,
                M31: page.origin.0,
                M32: page.origin.1,
            });
            painter.page(
                &laid,
                page_items,
                &format!("{} · {}", sheet.title, number + 1),
            )?;
            context.EndDraw(None, None)?;
            list.Close()?;
            control.AddPage(
                &list,
                D2D_SIZE_F {
                    width: page.size.0,
                    height: page.size.1,
                },
                None,
                None,
                None,
            )?;
        }
        control.Close()?;
    }
    Ok(true)
}

/// The printer chosen in the print dialog.
struct Printer {
    name: String,
    /// The dialog's settings (copies, paper, orientation) as a print
    /// ticket for the job.
    ticket: IStream,
    page: PageGeometry,
}

/// A page in DIPs: its size, where the printed area starts and its size.
#[derive(Clone, Copy)]
struct PageGeometry {
    size: (f32, f32),
    origin: (f32, f32),
    content: (f32, f32),
}

/// Shows the print dialog; `None` when the user cancels.
fn choose_printer(owner: Owner) -> Result<Option<Printer>> {
    let mut owner = owner.hwnd();
    if owner.is_invalid() {
        // The dialog needs an owner.
        owner = unsafe { GetDesktopWindow() };
    }
    let mut dialog = PRINTDLGEXW {
        lStructSize: std::mem::size_of::<PRINTDLGEXW>() as u32,
        hwndOwner: owner,
        Flags: PD_RETURNIC
            | PD_NOPAGENUMS
            | PD_NOSELECTION
            | PD_NOCURRENTPAGE
            | PD_HIDEPRINTTOFILE
            | PD_USEDEVMODECOPIESANDCOLLATE,
        nStartPage: START_PAGE_GENERAL,
        ..Default::default()
    };
    // SAFETY: the dialog fills the structure; its global memory and the
    // information context are freed below whatever the outcome.
    unsafe {
        PrintDlgExW(&mut dialog)?;
        let printer = (dialog.dwResultAction == PD_RESULT_PRINT)
            .then(|| printer(&dialog))
            .transpose();
        if !dialog.hDC.is_invalid() {
            let _ = DeleteDC(dialog.hDC);
        }
        for memory in [dialog.hDevMode, dialog.hDevNames] {
            if !memory.is_invalid() {
                let _ = GlobalFree(Some(memory));
            }
        }
        printer
    }
}

/// The printer, settings and page the dialog chose.
unsafe fn printer(dialog: &PRINTDLGEXW) -> Result<Printer> {
    unsafe {
        let names = GlobalLock(dialog.hDevNames) as *const u16;
        let offset = (*(names as *const DEVNAMES)).wDeviceOffset as usize;
        let name = PCWSTR(names.add(offset)).to_string().unwrap_or_default();
        let _ = GlobalUnlock(dialog.hDevNames);
        let ticket = ticket(&name, dialog.hDevMode)?;
        Ok(Printer {
            page: geometry(dialog.hDC),
            name,
            ticket,
        })
    }
}

/// The print dialog's settings as a print ticket.
unsafe fn ticket(printer: &str, devmode: HGLOBAL) -> Result<IStream> {
    unsafe {
        let stream = CreateStreamOnHGlobal(HGLOBAL::default(), true)?;
        let mode = GlobalLock(devmode) as *const DEVMODEW;
        if !mode.is_null() {
            let size = u32::from((*mode).dmSize) + u32::from((*mode).dmDriverExtra);
            let provider = PTOpenProvider(&HSTRING::from(printer), 1)?;
            // The metadata types the DEVMODE as the ANSI one; the function
            // takes the wide one the dialog returns.
            let converted = PTConvertDevModeToPrintTicket(
                provider,
                size,
                mode as *const DEVMODEA,
                kPTJobScope,
                &stream,
            );
            let _ = PTCloseProvider(provider);
            let _ = GlobalUnlock(devmode);
            converted?;
        }
        stream.Seek(0, STREAM_SEEK_SET, None)?;
        Ok(stream)
    }
}

/// The page of the printer's information context, in DIPs, with margins
/// of at least half an inch.
unsafe fn geometry(ic: HDC) -> PageGeometry {
    let caps = |index| unsafe { GetDeviceCaps(Some(ic), index) } as f32;
    let (dpi_x, dpi_y) = (caps(LOGPIXELSX).max(1.0), caps(LOGPIXELSY).max(1.0));
    let (x, y) = (|v: f32| v * 96.0 / dpi_x, |v: f32| v * 96.0 / dpi_y);
    let size = (x(caps(PHYSICALWIDTH)), y(caps(PHYSICALHEIGHT)));
    let printable = (x(caps(HORZRES)), y(caps(VERTRES)));
    let offset = (x(caps(PHYSICALOFFSETX)), y(caps(PHYSICALOFFSETY)));
    let left = offset.0.max(MARGIN);
    let top = offset.1.max(MARGIN);
    let right = (size.0 - offset.0 - printable.0).max(MARGIN);
    let bottom = (size.1 - offset.1 - printable.1).max(MARGIN);
    PageGeometry {
        size,
        origin: (left, top),
        content: (size.0 - left - right, size.1 - top - bottom),
    }
}

/// A Direct3D device for Direct2D, on the graphics card or else in
/// software.
unsafe fn d3d_device() -> Result<ID3D11Device> {
    let create = |driver: D3D_DRIVER_TYPE| {
        let mut device = None;
        unsafe {
            D3D11CreateDevice(
                None,
                driver,
                Default::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
        }
        .map(|()| device.unwrap())
    };
    create(D3D_DRIVER_TYPE_HARDWARE).or_else(|_| create(D3D_DRIVER_TYPE_WARP))
}

fn color(gray: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: gray,
        g: gray,
        b: gray,
        a: 1.0,
    }
}

/// The sheet's text, in points.
struct Text {
    factory: IDWriteFactory,
    normal: IDWriteTextFormat,
    bold: IDWriteTextFormat,
}

impl Text {
    fn new() -> Result<Self> {
        // SAFETY: DirectWrite objects made and used on this thread.
        unsafe {
            let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let format = |weight, size: f64| {
                factory.CreateTextFormat(
                    FONT,
                    None,
                    weight,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    size as f32,
                    w!(""),
                )
            };
            Ok(Self {
                normal: format(DWRITE_FONT_WEIGHT_NORMAL, FONT_SIZE)?,
                bold: format(DWRITE_FONT_WEIGHT_BOLD, TITLE_SIZE)?,
                factory,
            })
        }
    }

    /// `text` laid out, wrapped at `wrap` when given and set right when
    /// `right`.
    fn layout(
        &self,
        text: &str,
        bold: bool,
        wrap: Option<f64>,
        right: bool,
    ) -> Result<IDWriteTextLayout> {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        let format = if bold { &self.bold } else { &self.normal };
        // SAFETY: as in `new`.
        unsafe {
            let layout = self.factory.CreateTextLayout(
                &utf16,
                format,
                wrap.map_or(f32::MAX, |w| w as f32),
                f32::MAX,
            )?;
            if wrap.is_none() {
                layout.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            }
            if right {
                layout.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_TRAILING)?;
            }
            Ok(layout)
        }
    }

    fn size(&self, text: &str, bold: bool, wrap: Option<f64>) -> (f64, f64) {
        let mut metrics = DWRITE_TEXT_METRICS::default();
        match self.layout(text, bold, wrap, false) {
            // SAFETY: as in `new`.
            Ok(layout) if unsafe { layout.GetMetrics(&mut metrics) }.is_ok() => {
                (f64::from(metrics.width), f64::from(metrics.height))
            }
            _ => (0.0, FONT_SIZE * 1.3),
        }
    }
}

impl Measure for Text {
    fn measure(&self, text: &str, wrap: Option<f64>) -> (f64, f64) {
        self.size(text, false, wrap)
    }
}

/// Draws pages as the GTK app's `draw_page` does.
struct Painter<'a> {
    context: &'a ID2D1DeviceContext,
    text: &'a Text,
    black: ID2D1SolidColorBrush,
    gray: ID2D1SolidColorBrush,
    /// The printed area, in points.
    width: f32,
    height: f32,
    border: Option<Border>,
}

impl Painter<'_> {
    fn line(&self, x0: f64, y0: f64, x1: f64, y1: f64, pen: f32) {
        // SAFETY: drawing between BeginDraw and EndDraw.
        unsafe {
            self.context.DrawLine(
                Vector2 {
                    X: x0 as f32,
                    Y: y0 as f32,
                },
                Vector2 {
                    X: x1 as f32,
                    Y: y1 as f32,
                },
                &self.black,
                pen,
                None,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw(
        &self,
        text: &str,
        x: f64,
        y: f64,
        wrap: Option<f64>,
        right: bool,
        bold: bool,
        gray: bool,
    ) -> Result<()> {
        let layout = self.text.layout(text, bold, wrap, right)?;
        let brush = if gray { &self.gray } else { &self.black };
        // SAFETY: as in `line`.
        unsafe {
            self.context.DrawTextLayout(
                Vector2 {
                    X: x as f32,
                    Y: y as f32,
                },
                &layout,
                brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
        }
        Ok(())
    }

    fn page(&self, items: &[(Printed, f64)], page: &Page, footer: &str) -> Result<()> {
        let width = f64::from(self.width);
        let pen = frame_pen(self.border);
        let stroke = pen.unwrap_or(0.5) as f32;
        let pad = if pen.is_some() { FRAME_PAD } else { 0.0 };
        // Where the open box's frame starts on this page.
        let mut open = page.continued.then_some(0.0);
        for &(i, y) in &page.placed {
            match &items[i].0 {
                Printed::Title(text) => match self.border {
                    // The other pages: a rule under the title.
                    None => {
                        self.draw(text, 0.0, y, None, false, true, false)?;
                        self.line(0.0, y + 15.0, width, y + 15.0, 0.5);
                    }
                    Some(Border::None) => self.draw(text, 0.0, y, None, false, true, false)?,
                    // A rectangle with the title inside.
                    Some(Border::Thin) => {
                        self.draw(text, pad, y + 3.0, None, false, true, false)?;
                        self.line(0.0, y, width, y, stroke);
                        open = Some(y);
                    }
                    // A frame whose top line runs through the title.
                    Some(Border::Normal) => {
                        let top = y + 7.0;
                        if text.is_empty() {
                            self.line(0.0, top, width, top, stroke);
                        } else {
                            self.draw(text, pad + 4.0, y, None, false, true, false)?;
                            let w = self.text.size(text, true, None).0;
                            self.line(0.0, top, pad, top, stroke);
                            self.line(pad + 8.0 + w, top, width, top, stroke);
                        }
                        open = Some(top);
                    }
                },
                Printed::BoxEnd => {
                    if let Some(top) = open.take() {
                        let bottom = y + 3.0;
                        self.line(0.0, top, 0.0, bottom, stroke);
                        self.line(width, top, width, bottom, stroke);
                        self.line(0.0, bottom, width, bottom, stroke);
                    }
                }
                Printed::Row(cells, widths) => {
                    let starts = column_starts(widths);
                    for cell in cells {
                        if cell.column >= widths.len() {
                            continue;
                        }
                        let w = cell_width(widths, cell.column, cell.span);
                        self.draw(
                            &cell.text,
                            pad + starts[cell.column],
                            y,
                            Some(w.max(1.0)),
                            cell.right,
                            false,
                            cell.header,
                        )?;
                    }
                }
                Printed::Text(text, indent) => self.draw(
                    text,
                    pad + indent,
                    y,
                    Some(width - 2.0 * pad - indent),
                    false,
                    false,
                    false,
                )?,
                Printed::Picture(picture) => self.picture(picture, pad, y)?,
                Printed::PageBreak => {}
            }
        }
        // A box that runs on to the next page keeps its sides to the foot
        // of this one.
        if let Some(top) = open {
            let bottom = f64::from(self.height) - FOOTER;
            self.line(0.0, top, 0.0, bottom, stroke);
            self.line(width, top, width, bottom, stroke);
        }
        // Footer: name and page.
        self.draw(
            footer,
            0.0,
            f64::from(self.height) - 10.0,
            Some(width),
            true,
            false,
            false,
        )
    }

    fn picture(&self, picture: &pages::Picture, x: f64, y: f64) -> Result<()> {
        let (w, h) = picture.pixels;
        // Direct2D takes premultiplied BGRA.
        let pixels: Vec<u8> = picture
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| {
                let a = u16::from(p[3]);
                let pre = |c: u8| ((u16::from(c) * a + 127) / 255) as u8;
                [pre(p[2]), pre(p[1]), pre(p[0]), p[3]]
            })
            .collect();
        // SAFETY: as in `line`; the pixels outlive the call that copies
        // them into the bitmap.
        unsafe {
            let bitmap = self.context.CreateBitmap(
                D2D_SIZE_U {
                    width: w,
                    height: h,
                },
                Some(pixels.as_ptr() as *const _),
                w * 4,
                &D2D1_BITMAP_PROPERTIES1 {
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    dpiX: 96.0,
                    dpiY: 96.0,
                    bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
                    colorContext: std::mem::ManuallyDrop::new(None),
                },
            )?;
            self.context.DrawBitmap(
                &bitmap,
                Some(&D2D_RECT_F {
                    left: x as f32,
                    top: y as f32,
                    right: (x + picture.width) as f32,
                    bottom: (y + picture.height) as f32,
                }),
                1.0,
                D2D1_INTERPOLATION_MODE_LINEAR,
                None,
                None,
            );
        }
        Ok(())
    }
}
