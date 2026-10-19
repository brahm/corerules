//! A character's portrait, in New Character and Edit Character: a row that
//! shows it, and a dialog that picks one of the portraits in
//! `Bitmaps/Portrait`, the race's first, or makes one from a picture of
//! the user's (BMP, JPEG, GIF or PNG).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::pictures::{PORTRAIT_HEIGHT, PortraitChoice, stored_portrait};
use gtk::gio;

use crate::sheet::texture;

/// The pictures Choose File… offers: MIME type and suffixes.
const PICTURES: [(&str, &[&str]); 4] = [
    ("image/bmp", &["bmp"]),
    ("image/jpeg", &["jpg", "jpeg"]),
    ("image/gif", &["gif"]),
    ("image/png", &["png"]),
];

/// The portrait chosen: its folder and file name, both empty for none.
pub type Chosen = Rc<RefCell<[String; 2]>>;

/// The Portrait row for a character of `race` that has `current`.
pub fn row(root: PathBuf, race: String, current: [String; 2]) -> (adw::ActionRow, Chosen) {
    let chosen: Chosen = Rc::new(RefCell::new(current));
    let row = adw::ActionRow::builder().title(tr("Portrait")).build();
    let thumbnail = gtk::Image::builder()
        .pixel_size(48)
        .valign(gtk::Align::Center)
        .build();
    row.add_prefix(&thumbnail);
    let clear = gtk::Button::builder()
        .icon_name("edit-clear-symbolic")
        .tooltip_text(tr("No Portrait"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let choose = gtk::Button::builder()
        .label(tr("_Choose…"))
        .use_underline(true)
        .valign(gtk::Align::Center)
        .build();
    row.add_suffix(&clear);
    row.add_suffix(&choose);
    row.set_activatable_widget(Some(&choose));

    let show: Rc<dyn Fn()> = Rc::new({
        let (row, thumbnail, clear, chosen, root) = (
            row.clone(),
            thumbnail.clone(),
            clear.clone(),
            chosen.clone(),
            root.clone(),
        );
        move || {
            let stored = chosen.borrow().clone();
            let path = Installation::open(&root)
                .ok()
                .and_then(|i| i.portrait_path(&stored));
            match &stored[1] {
                file if file.is_empty() => row.set_subtitle(tr("None")),
                file if path.is_none() => {
                    row.set_subtitle(&tr!("{file} (not in this installation)", file = file))
                }
                file => row.set_subtitle(file),
            }
            let texture = path.as_deref().and_then(texture);
            match &texture {
                Some(texture) => thumbnail.set_paintable(Some(texture)),
                None => thumbnail.set_icon_name(Some("avatar-default-symbolic")),
            }
            if texture.is_some() {
                thumbnail.remove_css_class("dim-label");
            } else {
                thumbnail.add_css_class("dim-label");
            }
            clear.set_visible(!stored[1].is_empty());
        }
    });
    show();
    clear.connect_clicked({
        let (chosen, show) = (chosen.clone(), show.clone());
        move |_| {
            *chosen.borrow_mut() = Default::default();
            show();
        }
    });
    choose.connect_clicked({
        let chosen = chosen.clone();
        move |button| {
            let (chosen, show) = (chosen.clone(), show.clone());
            dialog(button, &root, &race, move |stored| {
                *chosen.borrow_mut() = stored;
                show();
            });
        }
    });
    (row, chosen)
}

/// The Choose Portrait dialog; `pick` gets the portrait chosen.
fn dialog(
    parent: &impl IsA<gtk::Widget>,
    root: &Path,
    race: &str,
    pick: impl Fn([String; 2]) + 'static,
) {
    let pick: Rc<dyn Fn([String; 2])> = Rc::new(pick);
    let choices = match Installation::open(root).and_then(|i| i.portrait_choices(race)) {
        Ok(choices) => choices,
        Err(e) => return alert(parent, tr("Could Not List the Portraits"), &e.to_string()),
    };
    let dialog = adw::Dialog::builder()
        .title(tr("Choose Portrait"))
        .content_width(600)
        .content_height(620)
        .build();
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();
    let (offered, rest): (Vec<_>, Vec<_>) = choices.into_iter().partition(|c| c.race);
    let groups = [
        (tr!("Portraits for {race}", race = race), offered),
        (tr("Other Portraits").to_owned(), rest),
    ];
    let mut shown = false;
    for (title, choices) in groups {
        if choices.is_empty() {
            continue;
        }
        shown = true;
        content.append(
            &gtk::Label::builder()
                .label(&title)
                .xalign(0.0)
                .css_classes(["heading"])
                .build(),
        );
        content.append(&tiles(&dialog, &choices, &pick));
    }
    if !shown {
        content.append(
            &adw::StatusPage::builder()
                .icon_name("avatar-default-symbolic")
                .title(tr("No Portraits"))
                .description(tr("Choose a picture of yours to make one"))
                .build(),
        );
    }

    let from_file = gtk::Button::builder()
        .label(tr("Choose _File…"))
        .use_underline(true)
        .tooltip_text(tr(
            "A BMP, JPEG, GIF or PNG picture, copied to the portraits",
        ))
        .build();
    from_file.connect_clicked({
        let (dialog, root, pick) = (dialog.clone(), root.to_owned(), pick.clone());
        move |_| choose_file(&dialog, &root, pick.clone())
    });
    let header = adw::HeaderBar::new();
    header.pack_start(&from_file);
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(
        &gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&content)
            .vexpand(true)
            .build(),
    ));
    dialog.set_child(Some(&view));
    dialog.present(Some(parent));
}

/// A grid of portraits; activating one picks it and closes the dialog.
fn tiles(
    dialog: &adw::Dialog,
    choices: &[PortraitChoice],
    pick: &Rc<dyn Fn([String; 2])>,
) -> gtk::FlowBox {
    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .activate_on_single_click(true)
        .homogeneous(true)
        .column_spacing(6)
        .row_spacing(6)
        .min_children_per_line(2)
        .max_children_per_line(8)
        .build();
    for choice in choices {
        let tile = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .tooltip_text(&choice.file)
            .build();
        // Shown at the shipped portraits' size, whatever the bitmap's.
        let picture = gtk::Image::builder()
            .pixel_size(PORTRAIT_HEIGHT as i32)
            .build();
        picture.set_paintable(texture(&choice.path).as_ref());
        tile.append(&picture);
        tile.append(
            &gtk::Label::builder()
                .label(&choice.file)
                .ellipsize(gtk::pango::EllipsizeMode::Middle)
                .max_width_chars(14)
                .css_classes(["caption"])
                .build(),
        );
        let child = gtk::FlowBoxChild::builder().child(&tile).build();
        child.update_property(&[gtk::accessible::Property::Label(&choice.file)]);
        flow.append(&child);
    }
    let files: Vec<String> = choices.iter().map(|c| c.file.clone()).collect();
    flow.connect_child_activated({
        let (dialog, pick) = (dialog.clone(), pick.clone());
        move |_, child| {
            if let Some(file) = usize::try_from(child.index())
                .ok()
                .and_then(|i| files.get(i))
            {
                pick(stored_portrait(file));
                dialog.close();
            }
        }
    });
    flow
}

/// Choose File…: a picture of the user's becomes a portrait, which is
/// picked.
fn choose_file(dialog: &adw::Dialog, root: &Path, pick: Rc<dyn Fn([String; 2])>) {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(tr("Pictures (BMP, JPEG, GIF, PNG)")));
    for (mime, suffixes) in PICTURES {
        filter.add_mime_type(mime);
        for suffix in suffixes {
            filter.add_suffix(suffix);
        }
    }
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    let picker = gtk::FileDialog::builder()
        .title(tr("Choose a Picture"))
        .filters(&filters)
        .modal(true)
        .build();
    let window = dialog.root().and_downcast::<gtk::Window>();
    let (dialog, root) = (dialog.clone(), root.to_owned());
    picker.open(window.as_ref(), gio::Cancellable::NONE, move |r| {
        let Ok(Some(path)) = r.map(|f| f.path()) else {
            return;
        };
        match Installation::open(&root).and_then(|i| i.add_portrait(&path)) {
            Ok(file) => {
                pick(stored_portrait(&file));
                dialog.close();
            }
            Err(e) => alert(&dialog, tr("Could Not Use the Picture"), &e.to_string()),
        }
    });
}

fn alert(parent: &impl IsA<gtk::Widget>, heading: &str, body: &str) {
    let alert = adw::AlertDialog::builder()
        .heading(heading)
        .body(body)
        .build();
    alert.add_response("close", tr("_Close"));
    alert.present(Some(parent));
}
