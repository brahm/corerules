//! Import…, Export…, New Package… and Edit Package… of the main menu: the
//! DM Toolkit's packages (`.xpx` from the Expansion, `.xpt` from Core
//! Rules 2.0), and Import Core Rules 2.0…, which brings a 2.0
//! installation's content into the Expansion as a 2.0 package would.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_formats::export::ExportFile;
use cr_i18n::{tr, trn};
use cr_store::Installation;
use cr_store::export::{Category, Clash, Content, Exportable, ImportReport, contents, remove};
use cr_store::help::topic;
use cr_store::userdb::UserDb;
use cr_store::userdb::parts::{Parts, name_taken};
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

/// A dialog with a list of groups under a header bar holding `action`;
/// Help opens program help topic `help`.
fn dialog(title: &str, action: &gtk::Button, groups: &gtk::Box, help: u32) -> adw::Dialog {
    let header = adw::HeaderBar::builder()
        .show_end_title_buttons(false)
        .show_start_title_buttons(false)
        .build();
    let cancel = gtk::Button::with_mnemonic(tr("_Cancel"));
    header.pack_start(&cancel);
    header.pack_end(action);
    let dialog = framed(title, &header, groups, help);
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    dialog
}

fn framed(title: &str, header: &adw::HeaderBar, groups: &gtk::Box, help: u32) -> adw::Dialog {
    let view = adw::ToolbarView::new();
    crate::help::add_button(header, &view, move || help);
    view.add_top_bar(header);
    view.set_content(Some(
        &gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&adw::Clamp::builder().child(groups).build())
            .build(),
    ));
    adw::Dialog::builder()
        .title(title)
        .content_width(480)
        .content_height(620)
        .child(&view)
        .build()
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
    filter.set_name(Some(tr("DM Toolkit Packages")));
    filter.add_suffix("xpx");
    filter.add_suffix("xpt");
    let only = gtk::FileFilter::new();
    if expansion {
        only.set_name(Some(tr("Expansion Packages (*.xpx)")));
        only.add_suffix("xpx");
    } else {
        only.set_name(Some(tr("Core Rules 2.0 Packages (*.xpt)")));
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
    alert.add_response("close", tr("_Close"));
    alert.present(Some(&host.parent()));
}

fn is_expansion(host: &impl Host) -> Option<bool> {
    Some(
        host.installation()
            .as_ref()?
            .path("UserDbas/AllowedU.dat")
            .is_some(),
    )
}

fn write_package(path: &std::path::Path, file: &ExportFile) -> cr_store::Result<()> {
    let bytes = file.to_bytes().map_err(|source| cr_store::Error::Format {
        path: path.to_path_buf(),
        source,
    })?;
    std::fs::write(path, bytes).map_err(|source| cr_store::Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Asks where to save a new package; `done` gets the path, with the
/// extension added when it has none.
fn ask_path(host: &impl Host, expansion: bool, done: impl FnOnce(PathBuf) + 'static) {
    let extension = if expansion { "xpx" } else { "xpt" };
    let picker = gtk::FileDialog::builder()
        .title(tr("Save Package"))
        .initial_name(format!("export.{extension}"))
        .filters(&package_filter(expansion))
        .modal(true)
        .build();
    picker.save(window_of(host).as_ref(), gio::Cancellable::NONE, move |r| {
        if let Ok(Some(mut path)) = r.map(|f| f.path()) {
            if path.extension().is_none() {
                path.set_extension(extension);
            }
            done(path);
        }
    });
}

/// A dialog listing the user's records and characters with check boxes;
/// `action` gets the dialog and the items checked.
fn choose_items<H: Host + 'static>(
    host: &Rc<H>,
    title: &str,
    label: &str,
    action: impl Fn(&adw::Dialog, Vec<Exportable>) + 'static,
) {
    let offered = {
        let install = host.installation();
        let Some(install) = install.as_ref() else {
            return;
        };
        match install.exportable() {
            Ok(offered) => offered,
            Err(e) => return host.toast(&e.to_string()),
        }
    };
    let chosen: Rc<RefCell<Vec<(Exportable, gtk::CheckButton)>>> = Rc::default();
    let button = gtk::Button::builder()
        .label(label)
        .use_underline(true)
        .sensitive(false)
        .css_classes(["suggested-action"])
        .build();
    let update = Rc::new({
        let (chosen, button) = (chosen.clone(), button.clone());
        move || {
            let any = chosen.borrow().iter().any(|(_, c)| c.is_active());
            button.set_sensitive(any);
        }
    });
    let groups = groups_box();
    if offered.is_empty() {
        groups.append(
            &adw::StatusPage::builder()
                .icon_name("document-send-symbolic")
                .title(tr("No Records"))
                .description(tr("Packages take the records you made and your characters"))
                .build(),
        );
    } else {
        let all = adw::SwitchRow::builder()
            .title(tr("Select _All"))
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
    let mut categories: Vec<Category> = offered.iter().map(|e| e.category).collect();
    categories.dedup();
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
        groups.append(&group);
    }
    let dialog = dialog(title, &button, &groups, topic::EXPORT);
    button.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            let items: Vec<Exportable> = chosen
                .borrow()
                .iter()
                .filter(|(_, c)| c.is_active())
                .map(|(e, _)| e.clone())
                .collect();
            action(&dialog, items);
        }
    });
    dialog.present(Some(&host.parent()));
}

/// Export…: choose what to take, then where to save it.
pub fn export<H: Host + 'static>(host: &Rc<H>) {
    let Some(expansion) = is_expansion(&**host) else {
        return;
    };
    choose_items(host, tr("Export"), tr("_Export…"), {
        let host = host.clone();
        move |dialog, items| {
            let (host, dialog) = (host.clone(), dialog.clone());
            ask_path(&*host.clone(), expansion, move |path| {
                let written = {
                    let install = host.installation();
                    let Some(install) = install.as_ref() else {
                        return;
                    };
                    install.export(&items).and_then(|export| {
                        write_package(&path, &export.file)?;
                        Ok(export.warnings)
                    })
                };
                match written {
                    Ok(warnings) => {
                        dialog.close();
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        if warnings.is_empty() {
                            host.toast(&tr!("Exported to {name}", name));
                        } else {
                            alert(
                                &*host,
                                &tr!("Exported to {name}", name),
                                &warnings.join("\n"),
                            );
                        }
                    }
                    Err(e) => alert(&*host, tr("Export Failed"), &e.to_string()),
                }
            });
        }
    });
}

/// Opens a package the user picks and hands it to `done`.
fn open_package<H: Host + 'static>(
    host: &Rc<H>,
    title: &str,
    done: impl FnOnce(&Rc<H>, PathBuf, ExportFile) + 'static,
) {
    let Some(expansion) = is_expansion(&**host) else {
        return;
    };
    let picker = gtk::FileDialog::builder()
        .title(title)
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
                Ok(file) => done(&host, path, file),
                Err(e) => alert(
                    &*host,
                    tr("Not a Package"),
                    &format!("{}: {e}", path.display()),
                ),
            }
        },
    );
}

/// Import…: choose a package, then what to take from it.
pub fn import<H: Host + 'static>(host: &Rc<H>) {
    open_package(host, tr("Import Package"), |host, _, file| {
        if let Some(expansion) = is_expansion(&**host) {
            choose_imports(host, file, expansion);
        }
    });
}

/// Import Core Rules 2.0…: choose a 2.0 installation, then which of its
/// characters and records to bring into the open Expansion installation.
pub fn import_core20<H: Host + 'static>(host: &Rc<H>) {
    let picker = gtk::FileDialog::builder()
        .title(tr("Import Core Rules 2.0 Installation"))
        .modal(true)
        .build();
    let host = host.clone();
    picker.select_folder(
        window_of(&*host).as_ref(),
        gio::Cancellable::NONE,
        move |r| {
            let Ok(Some(path)) = r.map(|f| f.path()) else {
                return;
            };
            let package = Installation::open(&path).and_then(|i| i.core20_package());
            match package {
                Ok(export) if contents(&export.file).is_empty() => alert(
                    &*host,
                    tr("Nothing to Import"),
                    tr("This Core Rules 2.0 installation has no characters or records of its own."),
                ),
                Ok(export) => choose_imports(&host, export.file, true),
                Err(e) => alert(&*host, tr("Cannot Import"), &e.to_string()),
            }
        },
    );
}

fn choose_imports<H: Host + 'static>(host: &Rc<H>, file: ExportFile, expansion: bool) {
    let preview = {
        let install = host.installation();
        let Some(install) = install.as_ref() else {
            return;
        };
        match install.import_preview(&file) {
            Ok(p) => p,
            Err(e) => return host.toast(&e.to_string()),
        }
    };
    if file.is_expansion() && !expansion {
        return alert(
            &**host,
            tr("Expansion Package"),
            tr("This package was made by the Expansion, which Core Rules 2.0 cannot import."),
        );
    }
    let checks: Rc<RefCell<Vec<(usize, gtk::CheckButton)>>> = Rc::default();
    let button = gtk::Button::builder()
        .label(tr("_Import"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let groups = groups_box();
    let mut categories: Vec<Category> = preview.iter().map(|c| c.category).collect();
    categories.sort_by_key(|c| c.label());
    categories.dedup();
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
                (Clash::Standard, _) => tr("Renamed with a * as a standard part has this name"),
                (Clash::Custom, true) => tr("Replaces your record of this name"),
                (Clash::Custom, false) => tr("Not imported: you have a record of this name"),
            };
            let (row, check) = row(&item.name, note, true);
            group.add(&row);
            checks.borrow_mut().push((n, check));
        }
        groups.append(&group);
    }
    if file.map_maker.count > 0 {
        groups.append(
            &adw::PreferencesGroup::builder()
                .description(trn!(
                    "The package also holds {n} Map Maker II map, which is not imported.",
                    "The package also holds {n} Map Maker II maps, which are not imported.",
                    file.map_maker.count
                ))
                .build(),
        );
    }
    let dialog = dialog(tr("Import"), &button, &groups, topic::IMPORT);
    let (file, preview) = (Rc::new(file), Rc::new(preview));
    button.connect_clicked({
        let (host, dialog) = (host.clone(), dialog.clone());
        move |_| {
            let chosen: Vec<usize> = checks
                .borrow()
                .iter()
                .filter(|(_, c)| c.is_active())
                .map(|(n, _)| *n)
                .collect();
            let parts = {
                let install = host.installation();
                let Some(install) = install.as_ref() else {
                    return;
                };
                match UserDb::<Parts>::load(install) {
                    Ok(parts) => Rc::new(parts),
                    Err(e) => return alert(&*host, tr("Import Failed"), &e.to_string()),
                }
            };
            let names = Rename {
                host: host.clone(),
                dialog: dialog.clone(),
                file: file.clone(),
                preview: preview.clone(),
                parts,
                chosen: Rc::new(chosen),
            };
            names.next(0, HashMap::new());
        }
    });
    dialog.present(Some(&host.parent()));
}

/// The names the parts a standard part's name hides take, asked one by one
/// in the original's order before the import runs (0x49d704): the name
/// with a "*", or what Name Conflict asks for when a part (a standard one,
/// one of the user's or one imported before it) has that name.
struct Rename<H: Host> {
    host: Rc<H>,
    dialog: adw::Dialog,
    file: Rc<ExportFile>,
    preview: Rc<Vec<Content>>,
    parts: Rc<UserDb<Parts>>,
    chosen: Rc<Vec<usize>>,
}

impl<H: Host> Clone for Rename<H> {
    fn clone(&self) -> Self {
        Self {
            host: self.host.clone(),
            dialog: self.dialog.clone(),
            file: self.file.clone(),
            preview: self.preview.clone(),
            parts: self.parts.clone(),
            chosen: self.chosen.clone(),
        }
    }
}

impl<H: Host + 'static> Rename<H> {
    /// Asks for the name of the `at`th chosen item, when it needs one,
    /// with `names` given so far.
    fn next(self, at: usize, names: HashMap<usize, String>) {
        let Some(&n) = self.chosen.get(at) else {
            return self.run(&names);
        };
        let item = &self.preview[n];
        // Only the Expansion renames (the preview says so).
        if item.clash != Clash::Standard {
            return self.next(at + 1, names);
        }
        // The parts imported before this one, under their new names.
        let before: Vec<String> = self.chosen[..at]
            .iter()
            .filter(|m| matches!(self.preview[**m].category, Category::Part(_)))
            .map(|m| names.get(m).unwrap_or(&self.preview[*m].name).clone())
            .collect();
        let name = format!("{}*", item.name);
        let taken = {
            let (parts, before) = (self.parts.clone(), before.clone());
            move |name: &str| name_taken(&parts, name, &before)
        };
        if !taken(&name) {
            return self.next(at + 1, names);
        }
        let parent = self.host.parent();
        let done = Rc::new(move |name: String| {
            let mut names = names.clone();
            names.insert(n, name);
            self.clone().next(at + 1, names);
        });
        crate::userdb::name_conflict(&parent, &name, Rc::new(taken), done);
    }

    fn run(&self, names: &HashMap<usize, String>) {
        let report = {
            let install = self.host.installation();
            let Some(install) = install.as_ref() else {
                return;
            };
            install.import_named(&self.file, Some(&self.chosen), names)
        };
        match report {
            Ok(report) => {
                self.dialog.close();
                self.host.reloaded();
                report_alert(&*self.host, &report);
            }
            Err(e) => alert(&*self.host, tr("Import Failed"), &e.to_string()),
        }
    }
}

fn report_alert(host: &impl Host, report: &ImportReport) {
    let label = |c: &Content| format!("{} ({})", c.name, c.category.label());
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
    let heading = trn!(
        "Imported {n} Item",
        "Imported {n} Items",
        report.imported.len()
    );
    if lines.is_empty() {
        host.toast(&heading);
    } else {
        alert(host, &heading, &lines.join("\n"));
    }
}

/// New Package…: an empty package in the editor.
pub fn new_package<H: Host + 'static>(host: &Rc<H>) {
    if let Some(expansion) = is_expansion(&**host) {
        editor(host, ExportFile::new(expansion), None);
    }
}

/// Edit Package…: a package the user picks, in the editor.
pub fn edit_package<H: Host + 'static>(host: &Rc<H>) {
    open_package(host, tr("Edit Package"), |host, path, file| {
        // 2.0 content is only imported, converted to the Expansion's
        // layout on the way in.
        if file.is_expansion() {
            editor(host, file, Some(path));
        } else {
            alert(
                &**host,
                tr("Core Rules 2.0 Package"),
                tr(
                    "Packages made by Core Rules 2.0 are only imported. Use Import… to bring \
                    their content into this installation, then make a new package from it.",
                ),
            );
        }
    });
}

/// The package being edited.
struct Package {
    file: ExportFile,
    /// Where it is saved; `None` until a new package is saved.
    path: Option<PathBuf>,
    changed: bool,
    /// What the changes left out, shown when it is saved.
    warnings: Vec<String>,
}

/// The editor: the package's items by category, each with a button that
/// takes it out, and Add Items… to put the user's records and characters
/// in.
fn editor<H: Host + 'static>(host: &Rc<H>, file: ExportFile, path: Option<PathBuf>) {
    let Some(expansion) = is_expansion(&**host) else {
        return;
    };
    let same_layout = file.is_expansion() == expansion;
    let title = path
        .as_ref()
        .and_then(|p| p.file_name())
        .map_or(tr("New Package").to_owned(), |n| {
            n.to_string_lossy().into_owned()
        });
    let package = Rc::new(RefCell::new(Package {
        file,
        changed: path.is_none(),
        path,
        warnings: Vec::new(),
    }));
    let header = adw::HeaderBar::new();
    let add = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text(tr("Add Items"))
        .sensitive(same_layout)
        .build();
    header.pack_start(&add);
    let save = gtk::Button::builder()
        .label(tr("_Save"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    header.pack_end(&save);
    let groups = groups_box();
    let dialog = framed(&title, &header, &groups, topic::EXPORT);
    let notes = adw::PreferencesGroup::builder()
        .visible(!same_layout)
        .build();
    if !same_layout {
        notes.set_description(Some(if file_is_expansion(&package) {
            tr(
                "This package was made by the Expansion; items of Core Rules 2.0 cannot be \
                added to it.",
            )
        } else {
            tr(
                "This package was made by Core Rules 2.0; items of the Expansion cannot be \
                added to it.",
            )
        }));
    }
    let list = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .build();
    groups.append(&notes);
    groups.append(&list);
    let view = View {
        package: package.clone(),
        list,
        dialog: dialog.clone(),
    };
    view.fill();
    add.connect_clicked({
        let (host, view) = (host.clone(), view.clone());
        move |_| {
            let view = view.clone();
            let host2 = host.clone();
            choose_items(&host, tr("Add Items"), tr("_Add"), move |chooser, items| {
                let added = {
                    let install = host2.installation();
                    let Some(install) = install.as_ref() else {
                        return;
                    };
                    install.add_to(&mut view.package.borrow_mut().file, &items)
                };
                match added {
                    Ok(warnings) => {
                        chooser.close();
                        let mut p = view.package.borrow_mut();
                        p.changed = true;
                        p.warnings.extend(warnings);
                        drop(p);
                        view.fill();
                    }
                    Err(e) => alert(&*host2, tr("Items Not Added"), &e.to_string()),
                }
            });
        }
    });
    save.connect_clicked({
        let (host, view) = (host.clone(), view.clone());
        move |_| {
            let path = view.package.borrow().path.clone();
            match path {
                Some(path) => view.save(&*host, path),
                None => {
                    let (host2, view) = (host.clone(), view.clone());
                    ask_path(&*host, expansion, move |path| view.save(&*host2, path));
                }
            }
        }
    });
    dialog.connect_close_attempt({
        let package = package.clone();
        move |dialog| {
            if !package.borrow().changed {
                dialog.force_close();
                return;
            }
            let alert = adw::AlertDialog::builder()
                .heading(tr("Discard Changes?"))
                .body(tr("The package has changes that are not saved."))
                .build();
            alert.add_responses(&[("cancel", tr("_Cancel")), ("discard", tr("_Discard"))]);
            alert.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
            let dialog = dialog.clone();
            alert.choose(&dialog.clone(), gio::Cancellable::NONE, move |r| {
                if r == "discard" {
                    dialog.force_close();
                }
            });
        }
    });
    dialog.set_can_close(false);
    dialog.present(Some(&host.parent()));
}

fn file_is_expansion(package: &Rc<RefCell<Package>>) -> bool {
    package.borrow().file.is_expansion()
}

#[derive(Clone)]
struct View {
    package: Rc<RefCell<Package>>,
    list: gtk::Box,
    dialog: adw::Dialog,
}

impl View {
    /// Lists the package's items again.
    fn fill(&self) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let items = contents(&self.package.borrow().file);
        if items.is_empty() {
            self.list.append(
                &adw::StatusPage::builder()
                    .icon_name("package-x-generic-symbolic")
                    .title(tr("Empty Package"))
                    .description(tr("Add records and characters with the + button"))
                    .build(),
            );
        }
        let mut categories: Vec<Category> = items.iter().map(|c| c.category).collect();
        categories.sort_by_key(|c| c.label());
        categories.dedup();
        for category in categories {
            let group = adw::PreferencesGroup::builder()
                .title(category.label())
                .build();
            for item in items.iter().filter(|c| c.category == category) {
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&item.name).as_str())
                    .build();
                let delete = gtk::Button::builder()
                    .icon_name("user-trash-symbolic")
                    .tooltip_text(tr("Remove"))
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                delete.connect_clicked({
                    let (view, item) = (self.clone(), item.clone());
                    move |_| {
                        let mut p = view.package.borrow_mut();
                        let warnings = remove(&mut p.file, std::slice::from_ref(&item));
                        p.warnings.extend(warnings);
                        p.changed = true;
                        drop(p);
                        view.fill();
                    }
                });
                row.add_suffix(&delete);
                group.add(&row);
            }
            self.list.append(&group);
        }
        let map_maker = self.package.borrow().file.map_maker.count;
        if map_maker > 0 {
            self.list.append(
                &adw::PreferencesGroup::builder()
                    .description(trn!(
                        "The package also holds {n} Map Maker II map, which a change to the \
                         package leaves out.",
                        "The package also holds {n} Map Maker II maps, which a change to the \
                         package leaves out.",
                        map_maker
                    ))
                    .build(),
            );
        }
    }

    fn save(&self, host: &impl Host, path: PathBuf) {
        let written = write_package(&path, &self.package.borrow().file);
        match written {
            Ok(()) => {
                let warnings = {
                    let mut p = self.package.borrow_mut();
                    p.changed = false;
                    p.path = Some(path.clone());
                    std::mem::take(&mut p.warnings)
                };
                self.dialog.force_close();
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if warnings.is_empty() {
                    host.toast(&tr!("Saved {name}", name));
                } else {
                    alert(host, &tr!("Saved {name}", name), &warnings.join("\n"));
                }
            }
            Err(e) => alert(host, tr("Package Not Saved"), &e.to_string()),
        }
    }
}
