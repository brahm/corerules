//! The Windows 11 type ramp and cards the pages are made of.

use windows_reactor::*;

/// The page title (TitleTextBlockStyle).
pub fn title(text: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(text)
        .font_size(28.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .text_wrapping(TextWrapping::Wrap)
}

/// A section title (SubtitleTextBlockStyle).
pub fn subtitle(text: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(text)
        .font_size(20.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .text_wrapping(TextWrapping::Wrap)
}

/// The heading of a group of cards (BodyStrongTextBlockStyle).
pub fn strong(text: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(text)
        .font_size(14.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .text_wrapping(TextWrapping::Wrap)
}

/// Body text.
pub fn body(text: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(text)
        .font_size(14.0)
        .text_wrapping(TextWrapping::Wrap)
}

/// Secondary text under a heading (CaptionTextBlockStyle, dimmed).
pub fn caption(text: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(text)
        .font_size(12.0)
        .opacity(0.75)
        .text_wrapping(TextWrapping::Wrap)
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
    let text: View = if description.is_empty() {
        body(header).into()
    } else {
        StackPanel::new()
            .spacing(2.0)
            .children((body(header), caption(description)))
    };
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
            .padding(Thickness::new(36.0, 24.0, 36.0, 36.0))
            .content(Border::new().max_width(1000.0).content(content)),
    )
}

/// A command bar button with a symbol.
pub fn command(label: &str, symbol: Symbol, enabled: bool, click: Callback<()>) -> View {
    AppBarButton::new()
        .label(label)
        .is_enabled(enabled)
        .on_click(click)
        .slot(AppBarButtonSlot::Icon, SymbolIcon::new().symbol(symbol))
}
