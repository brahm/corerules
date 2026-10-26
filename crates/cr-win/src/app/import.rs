//! Import… and Import Core Rules 2.0…: the package chosen in the Open
//! dialog (or opened from File Explorer), or what a Core Rules 2.0
//! installation hands over, each item taken or left as the GTK app's
//! Import dialog has it; Name Conflict for the parts a standard part's
//! name hides; and the report of the import.

use std::path::PathBuf;
use std::sync::Arc;

use cr_app::packages::{ImportChoice, ROWS_SHOWN, Renaming, category_label, file_name};
use cr_formats::export::ExportFile;
use cr_i18n::{tr, trn};
use cr_store::Installation;
use cr_store::export::{Clash, ImportReport, contents};
use cr_store::userdb::UserDb;
use cr_store::userdb::parts::{self, Parts};
use cr_win::text::plain;
use windows_reactor::*;

use super::encounter::expander;
use super::forms::{heading, two_lines};
use super::{Message, Notice, Shell};
use crate::ui;

/// What the Import page shows: the choice, where it comes from, and the
/// names being asked before the import runs.
#[derive(Clone)]
pub struct ImportPage {
    pub choice: ImportChoice,
    /// The package's file, or the Core Rules 2.0 installation's folder.
    pub source: PathBuf,
    pub core20: bool,
    pub asking: Option<Asking>,
}

/// Name Conflict, open over the page.
#[derive(Clone)]
pub struct Asking {
    pub renaming: Renaming,
    pub parts: Arc<UserDb<Parts>>,
    /// The item asked about, by index into the preview.
    pub n: usize,
    /// The name typed.
    pub name: String,
    /// Counts the times it was asked, for a dialog of its own each time.
    pub serial: u64,
}

/// The Open dialog's file types for an installation of the Expansion or
/// of Core Rules 2.0, as the GTK app offers them.
pub fn filters(expansion: bool) -> Vec<(String, String)> {
    let mut filters = vec![(
        tr("DM Toolkit Packages").to_owned(),
        if expansion { "*.xpx;*.xpt" } else { "*.xpt" }.to_owned(),
    )];
    if expansion {
        filters.push((tr("Expansion Packages (*.xpx)").to_owned(), "*.xpx".into()));
    }
    filters.push((
        tr("Core Rules 2.0 Packages (*.xpt)").to_owned(),
        "*.xpt".into(),
    ));
    filters
}

/// Reads the package at `path` and what importing it into `install`
/// would do.
pub fn open(install: Option<&Installation>, path: PathBuf) -> Result<Box<ImportPage>, String> {
    let install = install.ok_or_else(|| tr("No Core Rules Installation").to_owned())?;
    let file = cr_app::packages::read(&path)?;
    page(install, file, path, false)
}

/// What a Core Rules 2.0 installation at `path` hands over to `install`,
/// the Expansion's; `None` when it has nothing of its own.
pub fn open_core20(
    install: &Installation,
    path: PathBuf,
) -> Result<Option<Box<ImportPage>>, String> {
    let export = Installation::open(&path)
        .and_then(|i| i.core20_package())
        .map_err(|e| e.to_string())?;
    if contents(&export.file).is_empty() {
        return Ok(None);
    }
    page(install, export.file, path, true).map(Some)
}

fn page(
    install: &Installation,
    file: ExportFile,
    source: PathBuf,
    core20: bool,
) -> Result<Box<ImportPage>, String> {
    let preview = install.import_preview(&file).map_err(|e| e.to_string())?;
    Ok(Box::new(ImportPage {
        choice: ImportChoice::new(file, preview, !install.is_core20()),
        source,
        core20,
        asking: None,
    }))
}

/// Whether the import must ask Name Conflict's questions, and so read the
/// parts first: a chosen part a standard part's name hides.
pub fn renames(choice: &ImportChoice) -> bool {
    choice
        .chosen()
        .iter()
        .any(|&n| choice.preview[n].clash == Clash::Standard)
}

/// The key under which the shell keeps a large category's list open.
fn open_key(category: usize) -> String {
    format!("import\u{1}{category}")
}

/// The CheckBox of item `n`.
fn item_check(choice: &ImportChoice, n: usize, context: &mut ViewContext<Shell>) -> View {
    let item = &choice.preview[n];
    CheckBox::new()
        .automation_name(item.name.as_str())
        .is_checked(choice.checked[n])
        .on_is_checked_changed(context.callback(move |on| Message::ImportCheck(n, on)))
        .content(two_lines(&item.name, choice.note(n)))
}

pub fn view(page: &ImportPage, shell: &Shell, context: &mut ViewContext<Shell>) -> View {
    let choice = &page.choice;
    let refused = choice.refused();
    let warning: View = if refused {
        InfoBar::new()
            .severity(InfoBarSeverity::Warning)
            .title(tr("Expansion Package"))
            .message(tr(
                "This package was made by the Expansion, which Core Rules 2.0 cannot import.",
            ))
            .is_open(true)
            .is_closable(false)
            .into()
    } else {
        View::empty()
    };
    let maps: View = match choice.maps_note() {
        Some(note) => ui::caption(note).into(),
        None => View::empty(),
    };
    let categories: Vec<KeyedView> = choice
        .categories()
        .into_iter()
        .enumerate()
        .map(|(c, (category, items))| {
            let label = category_label(category);
            let content: View = if items.len() <= ROWS_SHOWN {
                let rows: Vec<KeyedView> = items
                    .iter()
                    .map(|&n| KeyedView::new(n, item_check(choice, n, context)))
                    .collect();
                ui::card(StackPanel::new().spacing(4.0).keyed_children(rows))
            } else {
                // One choice for them all, its items listed when opened.
                let key = open_key(c);
                let open = shell.open_rows.contains(&key);
                let all = choice.all(&items);
                let header = CheckBox::new()
                    .automation_name(label)
                    .is_checked(all)
                    .on_is_checked_changed(
                        context.callback(move |on| Message::ImportCategory(c, on)),
                    )
                    .content(ui::body(trn!("{n} item", "{n} items", items.len() as u64)));
                // A ListView draws only the rows in view.
                let list: View = if open {
                    ListView::new()
                        .selection_mode(ListViewSelectionMode::None)
                        .max_height(360.0)
                        .collection_slot(
                            ListViewSlot::Items,
                            items.iter().map(|&n| {
                                KeyedView::new(
                                    n,
                                    ListViewItem::new().content(item_check(choice, n, context)),
                                )
                            }),
                        )
                } else {
                    View::empty()
                };
                expander(key, label, &shell.open_rows, (header, list), context)
            };
            KeyedView::new(
                category.label(),
                StackPanel::new()
                    .spacing(4.0)
                    .children((heading(label, None), content)),
            )
        })
        .collect();
    let progress: View = if shell.busy {
        ProgressRing::new()
            .is_active(true)
            .width(24.0)
            .height(24.0)
            .into()
    } else {
        View::empty()
    };
    let title = if page.core20 {
        tr("Import Core Rules 2.0 Installation")
    } else {
        tr("Import Package")
    };
    let source = if page.core20 {
        page.source.display().to_string()
    } else {
        file_name(&page.source)
    };
    let dialog: Vec<KeyedView> = page
        .asking
        .as_ref()
        .map(|asking| {
            KeyedView::new(
                format!("conflict{}", asking.serial),
                conflict_dialog(asking, context),
            )
        })
        .into_iter()
        .collect();
    Grid::new().children((
        ui::page(
            StackPanel::new().spacing(12.0).children((
                StackPanel::new()
                    .spacing(2.0)
                    .children((ui::title(title), ui::caption(source))),
                warning,
                StackPanel::new().spacing(8.0).keyed_children(categories),
                maps,
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(8.0)
                    .horizontal_alignment(HorizontalAlignment::Right)
                    .children((
                        progress,
                        Button::new()
                            .style(ButtonStyle::Accent)
                            .is_enabled(choice.can_import() && !shell.busy)
                            .on_click(context.message(Message::Import))
                            .content(plain(tr("_Import"))),
                        Button::new()
                            .is_enabled(!shell.busy)
                            .on_click(context.message(Message::Back))
                            .content(plain(tr("_Cancel"))),
                    )),
            )),
        ),
        Grid::new().keyed_children(dialog),
    ))
}

/// Name Conflict (0x49d704): the name for a part a standard part's name
/// hides, asked again while a part has it. Help opens its topic.
fn conflict_dialog(asking: &Asking, context: &mut ViewContext<Shell>) -> View {
    let serial = asking.serial;
    ContentDialog::new()
        .title(tr("Name Conflict"))
        .primary_button_text(plain(tr("_Close")))
        .secondary_button_text(plain(tr("_Help")))
        .is_open(true)
        .on_closed(context.callback(move |result| {
            Message::ConflictAnswered(serial, result == ContentDialogResult::Secondary)
        }))
        .content(
            StackPanel::new().spacing(12.0).children((
                ui::body(tr(parts::NAME_CONFLICT)).text_wrapping(TextWrapping::Wrap),
                TextBox::new()
                    .automation_name(tr("Name"))
                    .text(asking.name.clone())
                    .on_text_changed(context.callback(Message::ConflictName)),
            )),
        )
}

/// What an import did, as the GTK app reports it.
pub fn report(report: &ImportReport) -> Notice {
    let (heading, lines) = cr_app::packages::report(report);
    Notice::new(
        if report.warnings.is_empty() {
            InfoBarSeverity::Success
        } else {
            InfoBarSeverity::Warning
        },
        heading,
        lines.join("\n"),
    )
}
