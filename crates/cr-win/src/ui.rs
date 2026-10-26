//! The Windows 11 type ramp and cards the pages are made of. Text wraps
//! between words only: in a narrow window a label keeps its words whole
//! rather than breaking one over lines, and a settings card puts its
//! control under its header.

use std::cell::Cell;

use windows_reactor::*;

thread_local! {
    /// The width beside the navigation pane, as the window last had it.
    static ROOM: Cell<f64> = const { Cell::new(f64::INFINITY) };
}

/// Notes the width beside the navigation pane, which the cards lay
/// themselves out by; infinite while the window has not said.
pub fn set_room(width: f64) {
    ROOM.set(width);
}

/// The width beside the navigation pane, as the window last had it;
/// infinite while it has not said.
pub fn room() -> f64 {
    ROOM.get()
}

/// The padding each side of a page's content.
const PAGE_SIDE: f64 = 36.0;

/// The width of a settings card below which its control goes under its
/// header, as the Windows Community Toolkit's SettingsCard does it.
const STACKED_BELOW: f64 = 476.0;

/// The width a setting's header takes beside a control, at the least.
const HEADER_ROOM: f64 = 200.0;

/// The page title (TitleTextBlockStyle).
pub fn title(text: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(text)
        .font_size(28.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .text_wrapping(TextWrapping::WrapWholeWords)
}

/// A section title (SubtitleTextBlockStyle).
pub fn subtitle(text: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(text)
        .font_size(20.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .text_wrapping(TextWrapping::WrapWholeWords)
}

/// The heading of a group of cards (BodyStrongTextBlockStyle).
pub fn strong(text: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(text)
        .font_size(14.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .text_wrapping(TextWrapping::WrapWholeWords)
}

/// Body text.
pub fn body(text: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(text)
        .font_size(14.0)
        .text_wrapping(TextWrapping::WrapWholeWords)
}

/// Secondary text under a heading (CaptionTextBlockStyle, dimmed).
pub fn caption(text: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(text)
        .font_size(12.0)
        .opacity(0.75)
        .text_wrapping(TextWrapping::WrapWholeWords)
}

/// A card: the raised surface settings and details sit on.
pub fn card(content: impl Into<View>) -> View {
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(1.0)
        .corner_radius(4.0)
        .padding(Thickness::new(16.0, 12.0, 16.0, 12.0))
        .content(content)
}

/// A settings card: a header and a description on the left, the control
/// on the right, as Windows Settings lays out its rows.
pub fn setting(header: &str, description: &str, control: impl Into<View>) -> View {
    setting_of(setting_text(header, description), control)
}

/// A settings card with an item's icon before its header.
pub fn icon_setting(
    icon: &EncodedImage,
    header: &str,
    description: &str,
    control: impl Into<View>,
) -> View {
    let side = f64::from(cr_store::pictures::ICON_SIZE);
    // A grid rather than a row, for the text to wrap in a narrow window.
    setting_of(
        Grid::new()
            .columns([GridLength::Auto, GridLength::STAR])
            .column_spacing(12.0)
            .children((
                Image::new()
                    .source_data(icon.clone())
                    .width(side)
                    .height(side)
                    .vertical_alignment(VerticalAlignment::Center),
                Border::new()
                    .grid_column(1)
                    .vertical_alignment(VerticalAlignment::Center)
                    .content(setting_text(header, description)),
            )),
        control,
    )
}

fn setting_text(header: &str, description: &str) -> View {
    if description.is_empty() {
        body(header).into()
    } else {
        StackPanel::new()
            .spacing(2.0)
            .children((body(header), caption(description)))
    }
}

/// A settings card whose control is `width` wide: under its header
/// where the card has no room for both side by side.
pub fn wide_setting(header: &str, description: &str, width: f64, control: impl Into<View>) -> View {
    let text = setting_text(header, description);
    if card_room() < width + HEADER_ROOM {
        stacked(text, control)
    } else {
        setting_of(text, control)
    }
}

/// The width a card leaves for its content, as the window last had it.
pub fn card_room() -> f64 {
    (ROOM.get() - 2.0 * PAGE_SIDE).min(1000.0) - 34.0
}

fn stacked(text: View, control: impl Into<View>) -> View {
    card(
        StackPanel::new().spacing(8.0).children((
            text,
            Border::new()
                .horizontal_alignment(HorizontalAlignment::Left)
                .content(control),
        )),
    )
}

fn setting_of(text: View, control: impl Into<View>) -> View {
    if ROOM.get() - 2.0 * PAGE_SIDE < STACKED_BELOW {
        return stacked(text, control);
    }
    card(
        Grid::new()
            .column_spacing(16.0)
            .columns([GridLength::STAR, GridLength::Auto])
            .children((
                Border::new()
                    .vertical_alignment(VerticalAlignment::Center)
                    .content(text),
                Border::new()
                    .grid_column(1)
                    .vertical_alignment(VerticalAlignment::Center)
                    .content(control),
            )),
    )
}

/// The padding of a page's content, as Windows 11 apps keep it.
pub fn page(content: impl Into<View>) -> View {
    ScrollViewer::new().content(
        Border::new()
            .padding(Thickness::new(PAGE_SIDE, 24.0, PAGE_SIDE, 36.0))
            .content(Border::new().max_width(1000.0).content(content)),
    )
}

/// A command bar button with a symbol. It is wide enough for the
/// longest word of its label, which WinUI would otherwise break over
/// lines under the symbol when the bar opens.
pub fn command(label: &str, symbol: Symbol, enabled: bool, click: Callback<()>) -> View {
    let longest = label
        .split_whitespace()
        .map(|word| word.chars().count())
        .max()
        .unwrap_or(0);
    AppBarButton::new()
        .min_width(68f64.max(7.0 * longest as f64 + 12.0))
        .label(label)
        .is_enabled(enabled)
        .on_click(click)
        .slot(AppBarButtonSlot::Icon, SymbolIcon::new().symbol(symbol))
}

/// A button that is only a symbol, named for screen readers and with its
/// name as a tooltip.
pub fn icon_button<C: Component>(
    name: &str,
    symbol: Symbol,
    message: C::Message,
    context: &mut ViewContext<C>,
) -> View
where
    C::Message: Clone,
{
    Button::new()
        .automation_name(name)
        .on_click(context.message(message))
        .content(SymbolIcon::new().symbol(symbol))
        .tooltip(name)
}

/// A decoded picture as a PNG, since WinUI's decoders do not read every
/// bitmap the original wrote.
pub fn png(image: &cr_formats::bmp::Image) -> Option<EncodedImage> {
    let rgba = image::RgbaImage::from_raw(image.width, image.height, image.rgba.clone())?;
    let mut png = std::io::Cursor::new(Vec::new());
    rgba.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(EncodedImage::new(png.into_inner()))
}

/// A bitmap file of the original as a PNG.
pub fn picture(path: &std::path::Path) -> Option<EncodedImage> {
    png(&cr_app::pictures::load(path)?)
}

/// A bitmap file of the original as a PNG at most `side` pixels high and
/// wide, for a dialog that shows many.
pub fn thumbnail(path: &std::path::Path, side: u32) -> Option<EncodedImage> {
    let image = cr_app::pictures::load(path)?;
    if image.width <= side && image.height <= side {
        return png(&image);
    }
    let rgba = image::RgbaImage::from_raw(image.width, image.height, image.rgba)?;
    let scale = f64::from(side) / f64::from(rgba.width().max(rgba.height()));
    let (width, height) = (
        ((f64::from(rgba.width()) * scale).round() as u32).max(1),
        ((f64::from(rgba.height()) * scale).round() as u32).max(1),
    );
    let small = image::imageops::resize(&rgba, width, height, image::imageops::Triangle);
    let mut png = std::io::Cursor::new(Vec::new());
    small.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(EncodedImage::new(png.into_inner()))
}
