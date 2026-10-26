//! Extract Data from CD… of the main menu: copies what the app uses out of
//! a Core Rules 2.0 or Expansion CD (its ISO image or the mounted CD) or an
//! installation into the app's data folder, which the app opens on start.
//! The same copy is made when a read-only CD is opened as a folder.

use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_app::extract;
use cr_i18n::tr;
use gtk::{gio, glib};

/// What the dialog needs from the window.
pub trait Host: 'static {
    fn open(&self, dir: PathBuf);
    fn add_toast(&self, toast: adw::Toast);
    fn parent(&self) -> gtk::Widget;
}

/// Asks whether the data comes from an ISO image or a folder, then for it.
pub fn choose_source(host: &Rc<impl Host>) {
    let alert = adw::AlertDialog::builder()
        .heading(tr("Extract Data from CD"))
        .body(extract::intro())
        .build();
    alert.add_response("cancel", tr("_Cancel"));
    alert.add_response("folder", tr("Choose _Folder…"));
    alert.add_response("image", tr("Choose _ISO Image…"));
    alert.set_response_appearance("image", adw::ResponseAppearance::Suggested);
    alert.set_default_response(Some("image"));
    alert.set_close_response("cancel");
    let parent = host.parent();
    let host = host.clone();
    alert.connect_response(None, move |_, response| {
        let window = host.parent().root().and_downcast::<gtk::Window>();
        let dialog = gtk::FileDialog::builder().modal(true).build();
        let host = host.clone();
        let picked = move |result: Result<gio::File, glib::Error>| {
            if let Some(path) = result.ok().and_then(|f| f.path()) {
                copy(&host, path);
            }
        };
        match response {
            "image" => {
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                for (name, pattern) in extract::image_filters() {
                    let filter = gtk::FileFilter::new();
                    filter.set_name(Some(&name));
                    filter.add_suffix(pattern.trim_start_matches("*."));
                    filters.append(&filter);
                }
                dialog.set_title(tr("Choose the Core Rules CD Image"));
                dialog.set_filters(Some(&filters));
                dialog.open(window.as_ref(), gio::Cancellable::NONE, picked);
            }
            "folder" => {
                dialog.set_title(tr("Choose the Core Rules CD or Installation"));
                dialog.select_folder(window.as_ref(), gio::Cancellable::NONE, picked);
            }
            _ => {}
        }
    });
    alert.present(Some(&parent));
}

/// Copies the data on a worker thread, then opens it.
fn copy(host: &Rc<impl Host>, source: PathBuf) {
    let working = adw::Toast::builder()
        .title(tr("Extracting the Core Rules data…"))
        .timeout(0)
        .build();
    host.add_toast(working.clone());
    let host = host.clone();
    glib::spawn_future_local(async move {
        let done = gio::spawn_blocking(move || extract::extract(&source)).await;
        working.dismiss();
        let dest = match done {
            Ok(Ok(done)) => done,
            Ok(Err(e)) => return failed(&*host, &e),
            Err(_) => return failed(&*host, tr("The data could not be copied")),
        };
        let toast = adw::Toast::new(&extract::extracted(&dest));
        if dest.is_data_folder {
            host.open(dest.path);
        } else {
            // The data folder already holds a package (usually the other
            // CD's), which stays the one opened on start.
            toast.set_button_label(Some(tr("_Open")));
            toast.set_timeout(10);
            let opener = host.clone();
            toast.connect_button_clicked(move |_| opener.open(dest.path.clone()));
        }
        host.add_toast(toast);
    });
}

/// Opens, in place of `source`, an Expansion installation the app cannot
/// write to (a mounted CD or ISO image), the copy of it in the app's data
/// folder, making the copy first if there is none.
pub fn open_local_copy(host: &Rc<impl Host>, source: PathBuf) {
    let working = adw::Toast::builder()
        .title(tr("The CD cannot be written to; copying its data…"))
        .timeout(0)
        .build();
    host.add_toast(working.clone());
    let host = host.clone();
    glib::spawn_future_local(async move {
        let done = gio::spawn_blocking(move || extract::local_copy(&source)).await;
        working.dismiss();
        let copy = match done {
            Ok(Ok(copy)) => copy,
            Ok(Err(e)) => return failed(&*host, &e),
            Err(_) => return failed(&*host, tr("The data could not be copied")),
        };
        let (heading, body) = extract::copied(&copy);
        if !copy.made {
            host.open(copy.path);
            host.add_toast(adw::Toast::new(heading));
            return;
        }
        let alert = adw::AlertDialog::builder()
            .heading(heading)
            .body(body)
            .build();
        alert.add_response("close", tr("_Close"));
        host.open(copy.path);
        alert.present(Some(&host.parent()));
    });
}

fn failed(host: &impl Host, message: &str) {
    let alert = adw::AlertDialog::builder()
        .heading(tr("Could Not Extract the Data"))
        .body(message)
        .build();
    alert.add_response("close", tr("_Close"));
    alert.present(Some(&host.parent()));
}
