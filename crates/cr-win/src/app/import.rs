//! Import…: the package chosen in the Open dialog (or opened from File
//! Explorer), what it would bring in, and the report of the import.

use std::path::PathBuf;
use std::sync::Arc;

use cr_formats::export::ExportFile;
use cr_i18n::{tr, trn};
use cr_store::Installation;
use cr_store::export::{Clash, ImportReport};
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Notice, Package, Shell};
use crate::ui;

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
pub fn open(install: Option<&Installation>, path: PathBuf) -> Result<Arc<Package>, String> {
    let install = install.ok_or_else(|| tr("No Core Rules Installation").to_owned())?;
    let file = std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|d| ExportFile::parse(&d).map_err(|e| e.to_string()))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let preview = install.import_preview(&file).map_err(|e| e.to_string())?;
    Ok(Arc::new(Package {
        path,
        file,
        preview,
        expansion: !install.is_core20(),
    }))
}

pub fn view(shell: &Shell, package: &Package, context: &mut ViewContext<Shell>) -> View {
    // Core Rules 2.0 cannot import what the Expansion made.
    let refused = package.file.is_expansion() && !package.expansion;
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
    let maps: View = if package.file.map_maker.count > 0 {
        ui::caption(trn!(
            "The package also holds {n} Map Maker II map, which is not imported.",
            "The package also holds {n} Map Maker II maps, which are not imported.",
            package.file.map_maker.count
        ))
        .into()
    } else {
        View::empty()
    };
    let items = package.preview.iter().enumerate().map(|(i, item)| {
        let note = match (item.clash, package.expansion) {
            (Clash::None, _) => String::new(),
            (Clash::Standard, _) => tr("Renamed with a * as a standard part has this name").into(),
            (Clash::Custom, true) => tr("Replaces your record of this name").into(),
            (Clash::Custom, false) => tr("Not imported: you have a record of this name").into(),
        };
        let subtitle = if note.is_empty() {
            item.category.label().to_owned()
        } else {
            format!("{} · {note}", item.category.label())
        };
        KeyedView::new(
            i,
            ListViewItem::new().content(
                StackPanel::new()
                    .spacing(2.0)
                    .margin(Thickness::new(0.0, 4.0, 0.0, 4.0))
                    .children((ui::body(item.name.clone()), ui::caption(subtitle))),
            ),
        )
    });
    let name = package
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let progress: View = if shell.busy {
        ProgressRing::new()
            .is_active(true)
            .width(24.0)
            .height(24.0)
            .into()
    } else {
        View::empty()
    };
    ui::page(
        StackPanel::new().spacing(12.0).children((
            StackPanel::new()
                .spacing(2.0)
                .children((ui::title(tr("Import Package")), ui::caption(name))),
            warning,
            // A large package holds tens of thousands of items, which a
            // ListView draws only as they scroll into view.
            ui::card(
                ListView::new()
                    .selection_mode(ListViewSelectionMode::None)
                    .max_height(420.0)
                    .collection_slot(ListViewSlot::Items, items),
            ),
            maps,
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .children((
                    progress,
                    Button::new()
                        .style(ButtonStyle::Accent)
                        .is_enabled(!refused && !shell.busy && !package.preview.is_empty())
                        .on_click(context.message(Message::Import))
                        .content(plain(tr("_Import"))),
                    Button::new()
                        .is_enabled(!shell.busy)
                        .on_click(context.message(Message::Back))
                        .content(plain(tr("_Cancel"))),
                )),
        )),
    )
}

/// What an import did, as the GTK app reports it.
pub fn report(report: &ImportReport) -> Notice {
    let label = |c: &cr_store::export::Content| format!("{} ({})", c.name, c.category.label());
    let mut lines = Vec::new();
    for (c, name) in &report.renamed {
        lines.push(tr!("{item} was renamed {name}.", item = label(c), name));
    }
    if !report.replaced.is_empty() {
        lines.push(tr!(
            "Replaced your records: {names}.",
            names = report
                .replaced
                .iter()
                .map(|(c, n)| format!("{n} ({})", c.label()))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !report.skipped.is_empty() {
        lines.push(tr!(
            "Not imported, as you have records of these names: {names}.",
            names = report
                .skipped
                .iter()
                .map(label)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    lines.extend(report.warnings.iter().cloned());
    Notice::new(
        if report.warnings.is_empty() {
            InfoBarSeverity::Success
        } else {
            InfoBarSeverity::Warning
        },
        trn!(
            "Imported {n} Item",
            "Imported {n} Items",
            report.imported.len()
        ),
        lines.join("\n"),
    )
}
