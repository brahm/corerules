//! Pictures records keep: a row that shows one, and a dialog that picks one
//! of the installation's or makes one from a picture of the user's (BMP,
//! JPEG, GIF or PNG). A character's portrait (New Character, Edit
//! Character) and a race's are among those in `Bitmaps/Portrait`, the
//! race's first; a monster type's among those in `Bitmaps/Monsters`, the
//! user's own in `UserBMP` first.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::pictures::{
    Kind, PORTRAIT_HEIGHT, PictureChoice, joined, split_stored, stored_portrait,
};
use gtk::gio;

use crate::sheet::texture;

/// The pictures Choose File… offers: MIME type and suffixes.
const PICTURES: [(&str, &[&str]); 4] = [
    ("image/bmp", &["bmp"]),
    ("image/jpeg", &["jpg", "jpeg"]),
    ("image/gif", &["gif"]),
    ("image/png", &["png"]),
];

/// What a row does with the picture chosen (see [`row`]).
type Set = dyn Fn(Option<&Path>) -> String;

/// The portrait chosen: its folder and file name, both empty for none.
pub type Chosen = Rc<RefCell<[String; 2]>>;

/// The Portrait row for a character of `race` that has `current`.
pub fn portrait_row(root: PathBuf, race: String, current: [String; 2]) -> (adw::ActionRow, Chosen) {
    let stored = joined(&current[0], &current[1]);
    let chosen: Chosen = Rc::new(RefCell::new(current));
    let (row, _) = row(root, Kind::Portrait, race, tr("Portrait"), stored, {
        let chosen = chosen.clone();
        move |path| {
            let mut chosen = chosen.borrow_mut();
            *chosen = match path.and_then(Path::file_name) {
                Some(file) => stored_portrait(&file.to_string_lossy()),
                None => Default::default(),
            };
            joined(&chosen[0], &chosen[1])
        }
    });
    (row, chosen)
}

/// A row titled `title` that shows the picture of `kind` a record keeps,
/// `current` (a path as records store it, empty for none), and lets the
/// user choose another or none. `set` gets the picture chosen, `None` for
/// none, and gives what the record now keeps. The function returned shows
/// what the record keeps after it changed elsewhere.
pub fn row(
    root: PathBuf,
    kind: Kind,
    race: String,
    title: &str,
    current: String,
    set: impl Fn(Option<&Path>) -> String + 'static,
) -> (adw::ActionRow, Rc<dyn Fn(String)>) {
    let row = adw::ActionRow::builder().title(title).build();
    let thumbnail = gtk::Image::builder()
        .pixel_size(48)
        .valign(gtk::Align::Center)
        .build();
    row.add_prefix(&thumbnail);
    let clear = gtk::Button::builder()
        .icon_name("edit-clear-symbolic")
        .tooltip_text(match kind {
            Kind::Portrait => tr("No Portrait"),
            Kind::Monster => tr("No Picture"),
        })
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

    let show: Rc<dyn Fn(String)> = Rc::new({
        let (row, thumbnail, clear, root) =
            (row.clone(), thumbnail.clone(), clear.clone(), root.clone());
        move |stored| {
            let path = Installation::open(&root)
                .ok()
                .and_then(|i| i.picture_path(&stored, kind));
            match split_stored(&stored).1 {
                "" => row.set_subtitle(tr("None")),
                file if path.is_none() => {
                    row.set_subtitle(&tr!("{file} (not in this installation)", file = file))
                }
                file => row.set_subtitle(file),
            }
            let texture = path.as_deref().and_then(texture);
            match &texture {
                Some(texture) => thumbnail.set_paintable(Some(texture)),
                None => thumbnail.set_icon_name(Some(icon(kind))),
            }
            if texture.is_some() {
                thumbnail.remove_css_class("dim-label");
            } else {
                thumbnail.add_css_class("dim-label");
            }
            clear.set_visible(!split_stored(&stored).1.is_empty());
        }
    });
    show(current);
    let set: Rc<Set> = Rc::new(set);
    clear.connect_clicked({
        let (set, show) = (set.clone(), show.clone());
        move |_| show(set(None))
    });
    choose.connect_clicked({
        let show = show.clone();
        move |button| {
            let (set, show) = (set.clone(), show.clone());
            dialog(button, &root, kind, &race, move |path| {
                show(set(Some(&path)))
            });
        }
    });
    (row, show)
}

fn icon(kind: Kind) -> &'static str {
    match kind {
        Kind::Portrait => "avatar-default-symbolic",
        Kind::Monster => "image-x-generic-symbolic",
    }
}

/// The Choose Portrait or Choose Picture dialog, offering the pictures of
/// `kind` (portraits for `race` first, when there is one); `pick` gets the
/// picture chosen.
fn dialog(
    parent: &impl IsA<gtk::Widget>,
    root: &Path,
    kind: Kind,
    race: &str,
    pick: impl Fn(PathBuf) + 'static,
) {
    let pick: Rc<dyn Fn(PathBuf)> = Rc::new(pick);
    let choices = match Installation::open(root).and_then(|i| i.picture_choices(kind, race)) {
        Ok(choices) => choices,
        Err(e) => {
            let heading = match kind {
                Kind::Portrait => tr("Could Not List the Portraits"),
                Kind::Monster => tr("Could Not List the Pictures"),
            };
            return alert(parent, heading, &e.to_string());
        }
    };
    let dialog = adw::Dialog::builder()
        .title(match kind {
            Kind::Portrait => tr("Choose Portrait"),
            Kind::Monster => tr("Choose Picture"),
        })
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
    let (first, rest): (Vec<_>, Vec<_>) = choices.into_iter().partition(|c| c.first);
    let groups = match kind {
        Kind::Portrait if race.is_empty() => {
            [(String::new(), first), (tr("Portraits").to_owned(), rest)]
        }
        Kind::Portrait => [
            (tr!("Portraits for {race}", race = race), first),
            (tr("Other Portraits").to_owned(), rest),
        ],
        Kind::Monster => [
            (tr("Your Pictures").to_owned(), first),
            (tr("Monster Pictures").to_owned(), rest),
        ],
    };
    let mut shown = false;
    for (title, choices) in groups {
        if choices.is_empty() {
            continue;
        }
        shown = true;
        if !title.is_empty() {
            content.append(
                &gtk::Label::builder()
                    .label(&title)
                    .xalign(0.0)
                    .css_classes(["heading"])
                    .build(),
            );
        }
        content.append(&tiles(&dialog, kind, choices, &pick));
    }
    if !shown {
        content.append(
            &adw::StatusPage::builder()
                .icon_name(icon(kind))
                .title(match kind {
                    Kind::Portrait => tr("No Portraits"),
                    Kind::Monster => tr("No Pictures"),
                })
                .description(tr("Choose a picture of yours to make one"))
                .build(),
        );
    }

    let from_file = gtk::Button::builder()
        .label(tr("Choose _File…"))
        .use_underline(true)
        .tooltip_text(match kind {
            Kind::Portrait => tr("A BMP, JPEG, GIF or PNG picture, copied to the portraits"),
            Kind::Monster => tr("A BMP, JPEG, GIF or PNG picture, copied to UserBMP"),
        })
        .build();
    from_file.connect_clicked({
        let (dialog, root, pick) = (dialog.clone(), root.to_owned(), pick.clone());
        move |_| choose_file(&dialog, &root, kind, pick.clone())
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

/// A grid of pictures; activating one picks it and closes the dialog.
fn tiles(
    dialog: &adw::Dialog,
    kind: Kind,
    choices: Vec<PictureChoice>,
    pick: &Rc<dyn Fn(PathBuf)>,
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
    // Portraits at their shipped size, monster pictures at half theirs,
    // whatever the bitmap's.
    let size = match kind {
        Kind::Portrait => PORTRAIT_HEIGHT,
        Kind::Monster => kind.size().1 / 2,
    };
    for choice in &choices {
        let tile = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .tooltip_text(&choice.file)
            .build();
        let picture = gtk::Image::builder().pixel_size(size as i32).build();
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
    let paths: Vec<PathBuf> = choices.into_iter().map(|c| c.path).collect();
    flow.connect_child_activated({
        let (dialog, pick) = (dialog.clone(), pick.clone());
        move |_, child| {
            if let Some(path) = usize::try_from(child.index())
                .ok()
                .and_then(|i| paths.get(i))
            {
                pick(path.clone());
                dialog.close();
            }
        }
    });
    flow
}

/// Choose File…: a picture of the user's becomes one of `kind`, which is
/// picked.
fn choose_file(dialog: &adw::Dialog, root: &Path, kind: Kind, pick: Rc<dyn Fn(PathBuf)>) {
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
        match Installation::open(&root).and_then(|i| i.add_picture(kind, &path)) {
            Ok(path) => {
                pick(path);
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
