//! A character's sheet, or the page of an encounter, encounter table,
//! template, treasure table or saved treasure, shown as the GNOME app
//! shows it: boxes stacked in columns. A character's sheet is also
//! exported from here, and its sections and border chosen in Character
//! Sheet Options.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use cr_app::action;
use cr_app::sheets::{Export, section_name};
use cr_i18n::tr;
use cr_store::sheet::{
    Align, Block, Border as Frame, CellKind, ListItem, SectionContent, Sheet, SheetBox,
    SheetChoice, Table,
};
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::ui;

/// A sheet on its page.
pub struct SheetPage {
    pub sheet: Arc<Sheet>,
    /// The character's file, for a character's sheet.
    pub file: Option<String>,
    /// Its options, when they can be changed.
    pub choice: Option<SheetChoice>,
    /// The pictures of its image blocks, by file, made once.
    pub images: HashMap<PathBuf, EncodedImage>,
}

impl SheetPage {
    pub fn new(sheet: Sheet, file: Option<String>, choice: Option<SheetChoice>) -> Self {
        let mut images = HashMap::new();
        for section in &sheet.sections {
            let SectionContent::Columns(columns) = &section.content else {
                continue;
            };
            for block in columns.iter().flatten().flat_map(|b| &b.body) {
                if let Block::Image {
                    file: Some(file), ..
                } = block
                    && !images.contains_key(file)
                    && let Some(image) = ui::picture(file)
                {
                    images.insert(file.clone(), image);
                }
            }
        }
        Self {
            sheet: Arc::new(sheet),
            file,
            choice,
            images,
        }
    }
}

/// The action that exports a character's sheet to `export`.
pub fn export_action(export: Export) -> action::Action {
    match export {
        Export::Html => action::SHEET_HTML,
        Export::Text => action::SHEET_TXT,
        Export::Markdown => action::SHEET_MD,
    }
}

pub fn view(shell: &Shell, page: &SheetPage, context: &mut ViewContext<Shell>) -> View {
    let busy = shell.busy;
    let mut primary = vec![
        KeyedView::new(
            "print",
            ui::command(
                tr("Print"),
                Symbol::Print,
                !busy,
                context.message(Message::Act(action::SHEET_PRINT)),
            ),
        ),
        KeyedView::new(
            "pdf",
            ui::command(
                &plain(tr("Save as _PDF…")),
                Symbol::Save,
                !busy,
                context.message(Message::Act(action::SHEET_PDF)),
            ),
        ),
    ];
    if page.choice.is_some() && shell.can_write {
        primary.push(KeyedView::new(
            "options",
            ui::command(
                tr("Character Sheet Options"),
                Symbol::Setting,
                !busy,
                context.message(Message::SheetOptions),
            ),
        ));
    }
    let secondary: Vec<KeyedView> = if page.file.is_some() {
        Export::of(page.sheet.flavor)
            .into_iter()
            .map(|export| {
                KeyedView::new(
                    export.extension(),
                    ui::command(
                        &plain(export.label()),
                        Symbol::Upload,
                        !busy,
                        context.message(Message::Act(export_action(export))),
                    ),
                )
            })
            .collect()
    } else {
        Vec::new()
    };
    let header = Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .margin(Thickness::new(36.0, 24.0, 24.0, 12.0))
        .children((
            ui::title(page.sheet.title.clone()).vertical_alignment(VerticalAlignment::Center),
            CommandBar::new()
                .horizontal_alignment(HorizontalAlignment::Right)
                .grid_column(1)
                .slots([
                    SlotView::collection(CommandBarSlot::PrimaryCommands, primary),
                    SlotView::collection(CommandBarSlot::SecondaryCommands, secondary),
                ]),
        ));
    let sections = page
        .sheet
        .sections
        .iter()
        .enumerate()
        .filter_map(|(i, section)| match &section.content {
            SectionContent::Columns(columns) => {
                Some(KeyedView::new(i, columns_view(columns, page)))
            }
            SectionContent::PageBreak => None,
        });
    Grid::new()
        .rows([GridLength::Auto, GridLength::STAR])
        .children((
            header,
            ScrollViewer::new().grid_row(1).content(
                StackPanel::new()
                    .spacing(18.0)
                    .margin(Thickness::new(36.0, 0.0, 36.0, 36.0))
                    .max_width(1100.0)
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .keyed_children(sections),
            ),
        ))
}

/// A row of columns, each a stack of boxes.
fn columns_view(columns: &[Vec<SheetBox>], page: &SheetPage) -> View {
    let count = columns.len().max(1);
    Grid::new()
        .columns(vec![GridLength::STAR; count])
        .column_spacing(18.0)
        .keyed_children(columns.iter().enumerate().map(|(c, column)| {
            KeyedView::new(
                c,
                StackPanel::new()
                    .spacing(18.0)
                    .vertical_alignment(VerticalAlignment::Top)
                    .grid_column(c as i32)
                    .keyed_children(
                        column
                            .iter()
                            .enumerate()
                            .map(|(b, sheet_box)| KeyedView::new(b, box_view(sheet_box, page))),
                    ),
            )
        }))
}

fn box_view(sheet_box: &SheetBox, page: &SheetPage) -> View {
    let title: View = if sheet_box.title.is_empty() {
        View::empty()
    } else {
        ui::strong(sheet_box.title.clone()).into()
    };
    // A heading alone, as the group encounter printout's title.
    if sheet_box.body.is_empty() {
        return title;
    }
    let blocks: Vec<KeyedView> = sheet_box
        .body
        .iter()
        .enumerate()
        .filter_map(|(i, block)| {
            let view: View = match block {
                Block::Table(table) if !table.rows.is_empty() => table_view(table),
                Block::List(items) if !items.is_empty() => list_view(items, 0),
                Block::Paragraph(text) if !text.trim().is_empty() => {
                    text_view(text.trim_end().replace('\t', "    "))
                }
                Block::Lines(lines) if !lines.is_empty() => text_view(lines.join("\n")),
                Block::Image {
                    file: Some(file), ..
                } => Image::new()
                    .source_data(page.images.get(file)?.clone())
                    .stretch(Stretch::Uniform)
                    .max_height(200.0)
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .into(),
                _ => return None,
            };
            Some(KeyedView::new(i, view))
        })
        .collect();
    let body: View = if blocks.is_empty() {
        ui::caption(tr("None")).into()
    } else {
        StackPanel::new().spacing(6.0).keyed_children(blocks)
    };
    StackPanel::new()
        .spacing(6.0)
        .children((title, ui::card(body)))
}

fn text_view(text: String) -> View {
    ui::body(text).is_text_selection_enabled(true).into()
}

fn table_view(table: &Table) -> View {
    let columns = table
        .rows
        .iter()
        .map(|r| r.iter().map(|c| c.colspan.max(1) as usize).sum::<usize>())
        .max()
        .unwrap_or(0)
        .max(table.columns)
        .max(1);
    let mut cells = Vec::new();
    for (r, row) in table.rows.iter().enumerate() {
        let mut column = 0;
        for (c, cell) in row.iter().enumerate() {
            let span = cell.colspan.max(1) as usize;
            if !cell.text.is_empty() && column < columns {
                let mut text = TextBlock::new()
                    .text(cell.text.clone())
                    .text_wrapping(TextWrapping::Wrap)
                    .is_text_selection_enabled(true)
                    .font_size(14.0);
                match cell.kind {
                    CellKind::Header => text = text.opacity(0.75),
                    CellKind::Large => {
                        text = text.font_size(20.0).font_weight(FontWeight::SEMI_BOLD)
                    }
                    _ => {}
                }
                let text = text
                    .horizontal_alignment(match cell.align {
                        Align::Right => HorizontalAlignment::Right,
                        Align::Center => HorizontalAlignment::Center,
                        _ => HorizontalAlignment::Left,
                    })
                    .grid_row(r as i32)
                    .grid_column(column as i32)
                    .grid_column_span(span.min(columns - column) as i32);
                cells.push(KeyedView::new(format!("{r}:{c}"), text));
            }
            column += span;
        }
    }
    // The last column takes the room left and wraps in it, so that a
    // wide table (a group encounter's NPCs) stays inside its card.
    let mut widths = vec![GridLength::Auto; columns];
    widths[columns - 1] = GridLength::STAR;
    Grid::new()
        .columns(widths)
        .rows(vec![GridLength::Auto; table.rows.len()])
        .column_spacing(12.0)
        .row_spacing(4.0)
        .keyed_children(cells)
}

fn list_view(items: &[ListItem], depth: u32) -> View {
    StackPanel::new()
        .spacing(2.0)
        .margin(Thickness::new(f64::from(depth) * 18.0, 0.0, 0.0, 0.0))
        .keyed_children(items.iter().enumerate().flat_map(|(i, item)| {
            let mut views = vec![KeyedView::new(format!("{i}"), text_view(item.text.clone()))];
            if let Some(children) = &item.children {
                views.push(KeyedView::new(
                    format!("{i}+"),
                    list_view(children, depth + 1),
                ));
            }
            views
        }))
}

/// Character Sheet Options being changed.
#[derive(Clone)]
pub struct SheetOptions {
    pub file: String,
    pub sections: Vec<u32>,
    pub border: Frame,
    /// What the options started from, to tell whether they changed.
    pub start: SheetChoice,
    pub saving: bool,
}

impl SheetOptions {
    pub fn new(file: String, choice: SheetChoice) -> Self {
        Self {
            file,
            sections: choice.sections.clone(),
            border: choice.border,
            start: choice,
            saving: false,
        }
    }

    pub fn choice(&self) -> SheetChoice {
        SheetChoice {
            sections: self.sections.clone(),
            border: self.border,
        }
    }

    pub fn changed(&self) -> bool {
        self.choice() != self.start
    }
}

/// A change to the options.
#[derive(Clone, Debug)]
pub enum OptionsEdit {
    Border(Option<usize>),
    /// Adds the section at this place of [`cr_app::sheets::offered`].
    Add(Option<usize>),
    Up(usize),
    Down(usize),
    Remove(usize),
}

impl SheetOptions {
    pub fn edit(&mut self, edit: OptionsEdit) {
        match edit {
            OptionsEdit::Border(Some(i)) => {
                if let Some(&border) = Frame::ALL.get(i) {
                    self.border = border;
                }
            }
            OptionsEdit::Add(Some(i)) => {
                if let Some(&id) = cr_app::sheets::offered().get(i) {
                    self.sections.push(id);
                }
            }
            OptionsEdit::Up(i) if i > 0 && i < self.sections.len() => self.sections.swap(i - 1, i),
            OptionsEdit::Down(i) if i + 1 < self.sections.len() => self.sections.swap(i, i + 1),
            OptionsEdit::Remove(i) if i < self.sections.len() => {
                self.sections.remove(i);
            }
            _ => {}
        }
    }
}

pub fn options_view(options: &SheetOptions, context: &mut ViewContext<Shell>) -> View {
    let border = ui::setting(
        tr("Border Style"),
        tr("How the printed sheet frames its boxes"),
        ComboBox::new()
            .items_source(Frame::ALL.map(|b| b.label()))
            .selected_index(Frame::ALL.iter().position(|b| *b == options.border))
            .automation_name(tr("Border Style"))
            .on_selection_changed(context.callback(|i| Message::Options(OptionsEdit::Border(i)))),
    );
    let offered = cr_app::sheets::offered();
    // Picking a section adds it; the box goes back to its placeholder,
    // being made anew with each section added.
    let add = Grid::new().keyed_children([KeyedView::new(
        options.sections.len(),
        ComboBox::new()
            .items_source(offered.iter().map(|&id| section_name(id)))
            .placeholder_text(tr("Add Section"))
            .automation_name(tr("Add Section"))
            .on_selection_changed(context.callback(|i| Message::Options(OptionsEdit::Add(i)))),
    )]);
    let count = options.sections.len();
    let rows: Vec<KeyedView> = options
        .sections
        .iter()
        .enumerate()
        .map(|(i, &id)| {
            let button = |name: &str, glyph: &str, enabled: bool, edit: OptionsEdit| {
                Button::new()
                    .automation_name(name)
                    .is_enabled(enabled)
                    .on_click(context.message(Message::Options(edit)))
                    .content(FontIcon::new().glyph(glyph))
                    .tooltip(name)
            };
            KeyedView::new(
                format!("{i}:{id}"),
                ui::card(
                    Grid::new()
                        .columns([GridLength::STAR, GridLength::Auto])
                        .children((
                            ui::body(section_name(id))
                                .vertical_alignment(VerticalAlignment::Center),
                            StackPanel::new()
                                .orientation(Orientation::Horizontal)
                                .spacing(4.0)
                                .grid_column(1)
                                .children((
                                    button(tr("Move Up"), "\u{E74A}", i > 0, OptionsEdit::Up(i)),
                                    button(
                                        tr("Move Down"),
                                        "\u{E74B}",
                                        i + 1 < count,
                                        OptionsEdit::Down(i),
                                    ),
                                    button(tr("Remove"), "\u{E738}", true, OptionsEdit::Remove(i)),
                                )),
                        )),
                ),
            )
        })
        .collect();
    let list: View = if rows.is_empty() {
        ui::caption(tr("No sections: the sheet will be empty")).into()
    } else {
        StackPanel::new().spacing(4.0).keyed_children(rows)
    };
    ui::page(
        StackPanel::new().spacing(12.0).children((
            ui::title(tr("Character Sheet Options")),
            border,
            Grid::new()
                .columns([GridLength::STAR, GridLength::Auto])
                .margin(Thickness::new(0.0, 12.0, 0.0, 0.0))
                .children((
                    StackPanel::new().spacing(2.0).children((
                        ui::strong(tr("Sections")),
                        ui::caption(tr(
                            "Printed in this order. A section can be added more than once.",
                        )),
                    )),
                    Border::new()
                        .grid_column(1)
                        .vertical_alignment(VerticalAlignment::Center)
                        .content(add),
                )),
            list,
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .children((
                    Button::new()
                        .on_click(context.message(Message::Back))
                        .content(plain(tr("_Cancel"))),
                    Button::new()
                        .style(ButtonStyle::Accent)
                        .is_enabled(!options.saving)
                        .on_click(context.message(Message::SaveOptions))
                        .content(plain(tr("_Save"))),
                )),
        )),
    )
}
