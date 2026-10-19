//! Extract Data from CD… of the main menu: copies what the app uses out of
//! a Core Rules 2.0 or Expansion CD (its ISO image or the mounted CD) or an
//! installation into the app's data folder, which the app opens on start.

use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::tr;
use cr_store::extract::{Destination, Extraction};
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
        .body(tr(
            "Copies the rules databases, books and tables the app uses from a Core Rules 2.0 \
             or Expansion CD into the app's data folder, so the CD is no longer needed. \
             Choose the CD's ISO image, or the folder of the mounted CD or of an installation.",
        ))
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
                extract(&host, path);
            }
        };
        match response {
            "image" => {
                let filter = gtk::FileFilter::new();
                filter.set_name(Some(tr("CD Images (*.iso)")));
                filter.add_suffix("iso");
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
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
fn extract(host: &Rc<impl Host>, source: PathBuf) {
    let working = adw::Toast::builder()
        .title(tr("Extracting the Core Rules data…"))
        .timeout(0)
        .build();
    host.add_toast(working.clone());
    let host = host.clone();
    glib::spawn_future_local(async move {
        let done = gio::spawn_blocking(move || -> cr_store::Result<Destination> {
            Extraction::probe(&source)?.install()
        })
        .await;
        working.dismiss();
        let dest = match done {
            Ok(Ok(done)) => done,
            Ok(Err(e)) => return failed(&*host, &e.to_string()),
            Err(_) => return failed(&*host, tr("The data could not be copied")),
        };
        let toast = adw::Toast::new(&match &dest.moved {
            Some(moved) => cr_i18n::tr!(
                "Data extracted into {path}; the Core Rules 2.0 data moved to {moved}",
                path = dest.path.display(),
                moved = moved.display()
            ),
            None => cr_i18n::tr!("Data extracted into {path}", path = dest.path.display()),
        });
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

fn failed(host: &impl Host, message: &str) {
    let alert = adw::AlertDialog::builder()
        .heading(tr("Could Not Extract the Data"))
        .body(message)
        .build();
    alert.add_response("close", tr("_Close"));
    alert.present(Some(&host.parent()));
}
