//! Export… and Import… of the main menu: the DM Toolkit's packages
//! (`.xpx` from the Expansion, `.xpt` from Core Rules 2.0).

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::export::ExportFile;
use cr_store::Installation;
use cr_store::export::{Category, Clash, Content, Exportable, ImportReport, Source};
use gtk::{gio, glib};

/// What the dialogs need from the window.
pub trait Host {
    fn installation(&self) -> std::cell::Ref<'_, Option<Installation>>;
    fn toast(&self, message: &str);
    /// Reloads the browser after an import.
    fn reloaded(&self);
    fn parent(&self) -> gtk::Widget;
}

fn row(title: &str, subtitle: &str, active: bool) -> (adw::ActionRow, gtk::CheckButton) {
    let check = gtk::CheckButton::builder()
        .active(active)
        .valign(gtk::Align::Center)
        .build();
    let row = adw::ActionRow::builder()
        .title(glib::markup_escape_text(title).as_str())
        .activatable_widget(&check)
        .build();
    if !subtitle.is_empty() {
        row.set_subtitle(glib::markup_escape_text(subtitle).as_str());
    }
    row.add_prefix(&check);
    (row, check)
}

/// A dialog with a list of groups under a header bar holding `action`.
fn dialog(title: &str, action: &gtk::Button, groups: &gtk::Box) -> adw::Dialog {
    let header = adw::HeaderBar::builder()
        .show_end_title_buttons(false)
        .show_start_title_buttons(false)
        .build();
    let cancel = gtk::Button::with_mnemonic("_Cancel");
    header.pack_start(&cancel);
    header.pack_end(action);
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(
        &gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&adw::Clamp::builder().child(groups).build())
            .build(),
    ));
    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(480)
        .content_height(620)
        .child(&view)
        .build();
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    dialog
}

fn groups_box() -> gtk::Box {
    gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build()
}

fn package_filter(expansion: bool) -> gio::ListStore {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("DM Toolkit Packages"));
    filter.add_suffix("xpx");
    filter.add_suffix("xpt");
    let only = gtk::FileFilter::new();
    if expansion {
        only.set_name(Some("Expansion Packages (*.xpx)"));
        only.add_suffix("xpx");
    } else {
        only.set_name(Some("Core Rules 2.0 Packages (*.xpt)"));
        only.add_suffix("xpt");
    }
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    filters.append(&only);
    filters
}

fn window_of(host: &impl Host) -> Option<gtk::Window> {
    host.parent().root().and_downcast::<gtk::Window>()
}

fn alert(host: &impl Host, heading: &str, body: &str) {
    let alert = adw::AlertDialog::builder()
        .heading(heading)
        .body(body)
        .build();
    alert.add_response("close", "_Close");
    alert.present(Some(&host.parent()));
}

/// Export…: choose what to take, then where to save it.
pub fn export<H: Host + 'static>(host: &Rc<H>) {
    let (offered, expansion) = {
        let install = host.installation();
        let Some(install) = install.as_ref() else {
            return;
        };
        let expansion = install.path("UserDbas/AllowedU.dat").is_some();
        match install.exportable() {
            Ok(offered) => (offered, expansion),
            Err(e) => return host.toast(&e.to_string()),
        }
    };
    let chosen: Rc<RefCell<Vec<(Exportable, gtk::CheckButton)>>> = Rc::default();
    let button = gtk::Button::builder()
        .label("_Export…")
        .use_underline(true)
        .sensitive(false)
        .css_classes(["suggested-action"])
        .build();
    let update = {
        let (chosen, button) = (chosen.clone(), button.clone());
        move || {
            let any = chosen.borrow().iter().any(|(_, c)| c.is_active());
            button.set_sensitive(any);
        }
    };
    let update = Rc::new(update);
    let groups = groups_box();
    let mut categories: Vec<Category> = offered.iter().map(|e| e.category).collect();
    categories.dedup();
    if !categories.contains(&Category::CampaignMap) {
        categories.push(Category::CampaignMap);
    }
    if offered.is_empty() {
        groups.append(
            &adw::StatusPage::builder()
                .icon_name("document-send-symbolic")
                .title("Nothing to Export")
                .description(
                    "Export takes the records you made, your characters and \
                     Campaign Mapper drawings",
                )
                .build(),
        );
    }
    if !offered.is_empty() {
        let all = adw::SwitchRow::builder()
            .title("Select _All")
            .use_underline(true)
            .build();
        all.connect_active_notify({
            let chosen = chosen.clone();
            move |all| {
                for (_, check) in chosen.borrow().iter() {
                    check.set_active(all.is_active());
                }
            }
        });
        let group = adw::PreferencesGroup::new();
        group.add(&all);
        groups.append(&group);
    }
    let mut map_group = None;
    for category in categories {
        let group = adw::PreferencesGroup::builder()
            .title(category.label())
            .build();
        for item in offered.iter().filter(|e| e.category == category) {
            let (row, check) = row(&item.name, "", false);
            check.connect_toggled({
                let update = update.clone();
                move |_| update()
            });
            group.add(&row);
            chosen.borrow_mut().push((item.clone(), check));
        }
        if category == Category::CampaignMap {
            map_group = Some(group.clone());
        }
        groups.append(&group);
    }
    if let Some(group) = map_group {
        let add = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("Add a Drawing")
            .css_classes(["flat"])
            .build();
        group.set_header_suffix(Some(&add));
        group.set_description(Some("Drawings made with the Campaign Mapper (.FCW)"));
        add.connect_clicked({
            let (host, chosen, group, update) =
                (host.clone(), chosen.clone(), group.clone(), update.clone());
            move |_| {
                let filter = gtk::FileFilter::new();
                filter.set_name(Some("Campaign Mapper Drawings (*.fcw)"));
                filter.add_suffix("fcw");
                filter.add_suffix("FCW");
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                let picker = gtk::FileDialog::builder()
                    .title("Add a Drawing")
                    .filters(&filters)
                    .modal(true)
                    .build();
                let (chosen, group, update) = (chosen.clone(), group.clone(), update.clone());
                picker.open(
                    window_of(&*host).as_ref(),
                    gio::Cancellable::NONE,
                    move |r| {
                        let Ok(Some(path)) = r.map(|f| f.path()) else {
                            return;
                        };
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        let (row, check) = row(&name, &path.display().to_string(), true);
                        check.connect_toggled({
                            let update = update.clone();
                            move |_| update()
                        });
                        group.add(&row);
                        chosen.borrow_mut().push((
                            Exportable {
                                category: Category::CampaignMap,
                                name: name.into_owned(),
                                source: Source::Map(path),
                            },
                            check,
                        ));
                        update();
                    },
                );
            }
        });
    }
    let dialog = dialog("Export", &button, &groups);
    button.connect_clicked({
        let (host, chosen, dialog) = (host.clone(), chosen.clone(), dialog.clone());
        move |_| {
            let items: Vec<Exportable> = chosen
                .borrow()
                .iter()
                .filter(|(_, c)| c.is_active())
                .map(|(e, _)| e.clone())
                .collect();
            let extension = if expansion { "xpx" } else { "xpt" };
            let picker = gtk::FileDialog::builder()
                .title("Save Package")
                .initial_name(format!("export.{extension}"))
                .filters(&package_filter(expansion))
                .modal(true)
                .build();
            let (host, dialog) = (host.clone(), dialog.clone());
            picker.save(
                window_of(&*host).as_ref(),
                gio::Cancellable::NONE,
                move |r| {
                    let Ok(Some(mut path)) = r.map(|f| f.path()) else {
                        return;
                    };
                    if path.extension().is_none() {
                        path.set_extension(extension);
                    }
                    let written = {
                        let install = host.installation();
                        let Some(install) = install.as_ref() else {
                            return;
                        };
                        install.export(&items).and_then(|export| {
                            let bytes = export.file.to_bytes().map_err(|source| {
                                cr_store::Error::Format {
                                    path: path.clone(),
                                    source,
                                }
                            })?;
                            std::fs::write(&path, bytes).map_err(|source| cr_store::Error::Io {
                                path: path.clone(),
                                source,
                            })?;
                            Ok(export.warnings)
                        })
                    };
                    match written {
                        Ok(warnings) => {
                            dialog.close();
                            let name = path.file_name().unwrap_or_default().to_string_lossy();
                            if warnings.is_empty() {
                                host.toast(&format!("Exported to {name}"));
                            } else {
                                alert(&*host, &format!("Exported to {name}"), &warnings.join("\n"));
                            }
                        }
                        Err(e) => alert(&*host, "Export Failed", &e.to_string()),
                    }
                },
            );
        }
    });
    dialog.present(Some(&host.parent()));
}

/// Import…: choose a package, then what to take from it.
pub fn import<H: Host + 'static>(host: &Rc<H>) {
    let expansion = match host.installation().as_ref() {
        Some(install) => install.path("UserDbas/AllowedU.dat").is_some(),
        None => return,
    };
    let picker = gtk::FileDialog::builder()
        .title("Import Package")
        .filters(&package_filter(expansion))
        .modal(true)
        .build();
    let host = host.clone();
    picker.open(
        window_of(&*host).as_ref(),
        gio::Cancellable::NONE,
        move |r| {
            let Ok(Some(path)) = r.map(|f| f.path()) else {
                return;
            };
            let file = std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|d| ExportFile::parse(&d).map_err(|e| e.to_string()));
            match file {
                Ok(file) => choose_imports(&host, file, expansion),
                Err(e) => alert(&*host, "Not a Package", &format!("{}: {e}", path.display())),
            }
        },
    );
}

fn choose_imports<H: Host + 'static>(host: &Rc<H>, file: ExportFile, expansion: bool) {
    let (preview, maps_folder) = {
        let install = host.installation();
        let Some(install) = install.as_ref() else {
            return;
        };
        match install.import_preview(&file) {
            Ok(p) => (p, install.campaign_mapper_folder()),
            Err(e) => return host.toast(&e.to_string()),
        }
    };
    if file.is_expansion() && !expansion {
        return alert(
            &**host,
            "Expansion Package",
            "This package was made by the Expansion, which Core Rules 2.0 cannot import.",
        );
    }
    let checks: Rc<RefCell<Vec<(usize, gtk::CheckButton)>>> = Rc::default();
    let button = gtk::Button::builder()
        .label("_Import")
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let groups = groups_box();
    let mut categories: Vec<Category> = preview.iter().map(|c| c.category).collect();
    categories.sort_by_key(|c| c.label());
    categories.dedup();
    let mut map_group = None;
    for category in categories {
        let group = adw::PreferencesGroup::builder()
            .title(category.label())
            .build();
        for (n, item) in preview
            .iter()
            .enumerate()
            .filter(|(_, c)| c.category == category)
        {
            let note = match (item.clash, expansion) {
                (Clash::None, _) => "",
                (Clash::Standard, _) => "Renamed with a * as a standard part has this name",
                (Clash::Custom, true) => "Replaces your record of this name",
                (Clash::Custom, false) => "Not imported: you have a record of this name",
            };
            let (row, check) = row(&item.name, note, true);
            group.add(&row);
            checks.borrow_mut().push((n, check));
        }
        if category == Category::CampaignMap {
            map_group = Some(group.clone());
        }
        groups.append(&group);
    }
    let maps: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(maps_folder));
    if let Some(group) = map_group {
        let folder_row = adw::ActionRow::builder()
            .title("Save In")
            .subtitle(folder_text(maps.borrow().as_ref()))
            .activatable(true)
            .build();
        folder_row.add_suffix(&gtk::Image::from_icon_name("folder-open-symbolic"));
        folder_row.connect_activated({
            let (host, maps) = (host.clone(), maps.clone());
            move |row| {
                let picker = gtk::FileDialog::builder()
                    .title("Save the Drawings In")
                    .modal(true)
                    .build();
                let (row, maps) = (row.clone(), maps.clone());
                picker.select_folder(
                    window_of(&*host).as_ref(),
                    gio::Cancellable::NONE,
                    move |r| {
                        if let Ok(Some(path)) = r.map(|f| f.path()) {
                            row.set_subtitle(&folder_text(Some(&path)));
                            *maps.borrow_mut() = Some(path);
                        }
                    },
                );
            }
        });
        group.add(&folder_row);
    }
    if file.map_maker.count > 0 {
        groups.append(
            &adw::PreferencesGroup::builder()
                .description(format!(
                    "The package also holds {} Map Maker II maps, which are not imported.",
                    file.map_maker.count
                ))
                .build(),
        );
    }
    let dialog = dialog("Import", &button, &groups);
    button.connect_clicked({
        let (host, dialog) = (host.clone(), dialog.clone());
        move |_| {
            let chosen: Vec<usize> = checks
                .borrow()
                .iter()
                .filter(|(_, c)| c.is_active())
                .map(|(n, _)| *n)
                .collect();
            let report = {
                let install = host.installation();
                let Some(install) = install.as_ref() else {
                    return;
                };
                install.import(&file, Some(&chosen), maps.borrow().as_deref())
            };
            match report {
                Ok(report) => {
                    dialog.close();
                    host.reloaded();
                    report_alert(&*host, &report);
                }
                Err(e) => alert(&*host, "Import Failed", &e.to_string()),
            }
        }
    });
    dialog.present(Some(&host.parent()));
}

fn folder_text(folder: Option<&PathBuf>) -> String {
    match folder {
        Some(f) => f.display().to_string(),
        None => "None chosen: the drawings are not imported".into(),
    }
}

fn report_alert(host: &impl Host, report: &ImportReport) {
    let label = |c: &Content| format!("{} ({})", c.name, c.category.label());
    let mut lines = Vec::new();
    for (c, name) in &report.renamed {
        lines.push(format!("{} was renamed {name}.", label(c)));
    }
    if !report.replaced.is_empty() {
        lines.push(format!(
            "Replaced your records: {}.",
            report
                .replaced
                .iter()
                .map(|(c, n)| format!("{n} ({})", c.label()))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !report.skipped.is_empty() {
        lines.push(format!(
            "Not imported, as you have records of these names: {}.",
            report
                .skipped
                .iter()
                .map(label)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for map in &report.maps {
        lines.push(format!("Saved {}.", map.display()));
    }
    lines.extend(report.warnings.iter().cloned());
    let heading = match report.imported.len() {
        1 => "Imported 1 Item".to_owned(),
        n => format!("Imported {n} Items"),
    };
    if lines.is_empty() {
        host.toast(&heading);
    } else {
        alert(host, &heading, &lines.join("\n"));
    }
}
