//! Pictures records keep: a row that shows one, and a dialog that picks one
//! of the installation's or makes one from a picture of the user's (BMP,
//! JPEG, GIF or PNG). A character's portrait (New Character, Edit
//! Character) and a race's are among those in `Bitmaps/Portrait`, the
//! race's first; a monster type's among those in `Bitmaps/Monsters`, the
//! user's own in `UserBMP` first. An item's icon is a square of a strip
//! in `Bitmaps/Equip`, or a bitmap of the user's in `UserBMP`, which come
//! first.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::{tr, tr_noop};
use cr_store::Installation;
use cr_store::help::topic;
use cr_store::pictures::{
    CUSTOM_ICON, ICON_SIZE, Icon, IconChoice, IconGroup, Kind, PORTRAIT_HEIGHT, PictureChoice,
    icon_number, joined, split_stored, stored_icon, stored_portrait,
};
use gtk::{gdk, gio, glib};

use crate::sheet::texture;

/// The pictures Choose File… offers: MIME type and suffixes.
const PICTURES: [(&str, &[&str]); 4] = [
    ("image/bmp", &["bmp"]),
    ("image/jpeg", &["jpg", "jpeg"]),
    ("image/gif", &["gif"]),
    ("image/png", &["png"]),
];

/// What a row does with the picture chosen (see [`chooser`]).
type Set<P> = dyn Fn(Option<P>) -> String;

/// What a row shows for what a record keeps: a subtitle and a thumbnail.
type Look = dyn Fn(&str) -> (String, Option<gdk::MemoryTexture>);

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
    let look = {
        let root = root.clone();
        move |stored: &str| {
            let path = Installation::open(&root)
                .ok()
                .and_then(|i| i.picture_path(stored, kind));
            let subtitle = match split_stored(stored).1 {
                "" => tr("None").to_owned(),
                file if path.is_none() => tr!("{file} (not in this installation)", file = file),
                file => file.to_owned(),
            };
            (subtitle, path.as_deref().and_then(texture))
        }
    };
    chooser(
        title,
        icon(kind),
        no_picture(kind),
        current,
        Rc::new(look),
        Rc::new(move |path: Option<PathBuf>| set(path.as_deref())),
        move |button, pick| dialog(button, &root, kind, &race, move |path| pick(path)),
    )
}

/// A row titled `title` that shows the icon an item keeps, `current`
/// (`12`, `CRE_espada.bmp`, empty for none), and lets the user choose
/// another or none. `set` gets what the item is to keep, `None` for none,
/// and gives what it now keeps. The function returned shows what the item
/// keeps after it changed elsewhere.
pub fn icon_row(
    root: PathBuf,
    title: &str,
    current: String,
    set: impl Fn(Option<String>) -> String + 'static,
) -> (adw::ActionRow, Rc<dyn Fn(String)>) {
    let look = {
        let root = root.clone();
        move |stored: &str| {
            let stored = stored.trim();
            if stored.is_empty() {
                return (tr("None").to_owned(), None);
            }
            let icon = Installation::open(&root).ok().and_then(|i| i.icon(stored));
            let name = icon_name(stored);
            match icon.as_ref().and_then(icon_texture) {
                Some(texture) => (name, Some(texture)),
                None => (tr!("{file} (not in this installation)", file = name), None),
            }
        }
    };
    chooser(
        title,
        icon(Kind::Item),
        no_picture(Kind::Item),
        current,
        Rc::new(look),
        Rc::new(set),
        move |button, pick| icon_dialog(button, &root, pick),
    )
}

/// An icon as the user reads it: its file, or its number.
fn icon_name(stored: &str) -> String {
    match stored.get(..CUSTOM_ICON.len()) {
        Some(prefix) if prefix.eq_ignore_ascii_case(CUSTOM_ICON) => {
            stored[CUSTOM_ICON.len()..].to_owned()
        }
        _ => match icon_number(stored) {
            Some(number) => tr!("Icon {number}", number),
            None => stored.to_owned(),
        },
    }
}

/// A row titled `title` with a thumbnail, Choose… and a button that
/// clears it, showing `current`; `look` gives what it shows for what a
/// record keeps, `set` stores a choice, `open` opens the dialog that makes
/// one from the Choose… button.
fn chooser<P: 'static>(
    title: &str,
    placeholder: &'static str,
    clear_tooltip: &str,
    current: String,
    look: Rc<Look>,
    set: Rc<Set<P>>,
    open: impl Fn(&gtk::Button, Rc<dyn Fn(P)>) + 'static,
) -> (adw::ActionRow, Rc<dyn Fn(String)>) {
    let row = adw::ActionRow::builder().title(title).build();
    let thumbnail = gtk::Image::builder()
        .pixel_size(48)
        .valign(gtk::Align::Center)
        .build();
    row.add_prefix(&thumbnail);
    let clear = gtk::Button::builder()
        .icon_name("edit-clear-symbolic")
        .tooltip_text(clear_tooltip)
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
        let (row, thumbnail, clear) = (row.clone(), thumbnail.clone(), clear.clone());
        move |stored| {
            let (subtitle, texture) = look(&stored);
            row.set_subtitle(&glib::markup_escape_text(&subtitle));
            match &texture {
                Some(texture) => {
                    thumbnail.set_paintable(Some(texture));
                    thumbnail.remove_css_class("dim-label");
                }
                None => {
                    thumbnail.set_icon_name(Some(placeholder));
                    thumbnail.add_css_class("dim-label");
                }
            }
            clear.set_visible(!split_stored(&stored).1.trim().is_empty());
        }
    });
    show(current);
    clear.connect_clicked({
        let (set, show) = (set.clone(), show.clone());
        move |_| show(set(None))
    });
    choose.connect_clicked({
        let show = show.clone();
        move |button| {
            let (set, show) = (set.clone(), show.clone());
            open(button, Rc::new(move |picked| show(set(Some(picked)))));
        }
    });
    (row, show)
}

thread_local! {
    /// The strips of `Bitmaps/Equip` decoded so far, `None` for one that
    /// does not decode; a list of items cuts many icons from each.
    static STRIPS: RefCell<HashMap<PathBuf, Option<Rc<cr_formats::bmp::Image>>>> =
        RefCell::default();
}

/// An item's icon as a texture, `None` when its bitmap does not decode.
pub fn icon_texture(icon: &Icon) -> Option<gdk::MemoryTexture> {
    let Some(left) = icon.left else {
        return texture(&icon.path);
    };
    let strip = STRIPS.with_borrow_mut(|strips| {
        strips
            .entry(icon.path.clone())
            .or_insert_with(|| {
                std::fs::read(&icon.path)
                    .ok()
                    .and_then(|data| cr_formats::bmp::decode(&data))
                    .map(Rc::new)
            })
            .clone()
    })?;
    let width = ICON_SIZE.min(strip.width.checked_sub(left)?) as usize;
    let height = ICON_SIZE.min(strip.height) as usize;
    if width == 0 {
        return None;
    }
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let at = (y * strip.width as usize + left as usize) * 4;
        rgba.extend_from_slice(&strip.rgba[at..at + width * 4]);
    }
    Some(gdk::MemoryTexture::new(
        width as i32,
        height as i32,
        gdk::MemoryFormat::R8g8b8a8,
        &glib::Bytes::from_owned(rgba),
        width * 4,
    ))
}

fn icon(kind: Kind) -> &'static str {
    match kind {
        Kind::Portrait => "avatar-default-symbolic",
        Kind::Monster | Kind::Item => "image-x-generic-symbolic",
    }
}

fn no_picture(kind: Kind) -> &'static str {
    match kind {
        Kind::Portrait => tr("No Portrait"),
        Kind::Monster | Kind::Item => tr("No Picture"),
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
                Kind::Monster | Kind::Item => tr("Could Not List the Pictures"),
            };
            return alert(parent, heading, &e.to_string());
        }
    };
    let dialog = adw::Dialog::builder()
        .title(match kind {
            Kind::Portrait => tr("Choose Portrait"),
            Kind::Monster | Kind::Item => tr("Choose Picture"),
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
        Kind::Monster | Kind::Item => [
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
                    Kind::Monster | Kind::Item => tr("No Pictures"),
                })
                .description(tr("Choose a picture of yours to make one"))
                .build(),
        );
    }

    let header = adw::HeaderBar::new();
    header.pack_start(&from_file_button(&dialog, root, kind, pick));
    let view = adw::ToolbarView::new();
    // The original's Character Portraits screen has no help topic.
    match kind {
        Kind::Portrait => {}
        Kind::Monster => crate::help::add_button(&header, &view, || topic::SELECT_IMAGE),
        Kind::Item => crate::help::add_button(&header, &view, || topic::EQUIPMENT_PICTURE),
    }
    view.add_top_bar(&header);
    view.set_content(Some(&scrolled(&content)));
    dialog.set_child(Some(&view));
    dialog.present(Some(parent));
}

/// The Choose File… button of a dialog choosing a picture of `kind`.
fn from_file_button(
    dialog: &adw::Dialog,
    root: &Path,
    kind: Kind,
    pick: Rc<dyn Fn(PathBuf)>,
) -> gtk::Button {
    let from_file = gtk::Button::builder()
        .label(tr("Choose _File…"))
        .use_underline(true)
        .tooltip_text(match kind {
            Kind::Portrait => tr("A BMP, JPEG, GIF or PNG picture, copied to the portraits"),
            Kind::Monster | Kind::Item => tr("A BMP, JPEG, GIF or PNG picture, copied to UserBMP"),
        })
        .build();
    from_file.connect_clicked({
        let (dialog, root) = (dialog.clone(), root.to_owned());
        move |_| choose_file(&dialog, &root, kind, pick.clone())
    });
    from_file
}

fn scrolled(content: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(content)
        .vexpand(true)
        .build()
}

/// The part types' names, for the groups of icons.
const PART_TYPE_NAMES: [&str; 6] = [
    tr_noop!("Weapons"),
    tr_noop!("Armor"),
    tr_noop!("Equipment"),
    tr_noop!("Magical Items"),
    tr_noop!("Gems and Art"),
    tr_noop!("Currency Types"),
];

/// The Choose Picture dialog of an item, offering the bitmaps in
/// `UserBMP` and the icons of the strips by the part type of the items
/// that have them, with a search by item name; `pick` gets what the item
/// is to keep.
fn icon_dialog(parent: &impl IsA<gtk::Widget>, root: &Path, pick: Rc<dyn Fn(String)>) {
    let choices = match Installation::open(root).and_then(|i| i.icon_choices()) {
        Ok(choices) => choices,
        Err(e) => return alert(parent, tr("Could Not List the Pictures"), &e.to_string()),
    };
    let dialog = adw::Dialog::builder()
        .title(tr("Choose Picture"))
        .content_width(640)
        .content_height(640)
        .build();
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();
    let mut groups: Vec<(IconGroup, Vec<IconChoice>)> = Vec::new();
    for choice in choices {
        match groups.last_mut() {
            Some((group, members)) if *group == choice.group => members.push(choice),
            _ => groups.push((choice.group, vec![choice])),
        }
    }
    let query: Rc<RefCell<String>> = Rc::default();
    let mut shown: Vec<(gtk::Label, gtk::FlowBox, Rc<Vec<String>>)> = Vec::new();
    for (group, choices) in groups {
        let title = match group {
            IconGroup::User => tr("Your Pictures"),
            IconGroup::PartType(part_type) => cr_store::pictures::ICON_PART_TYPES
                .iter()
                .position(|t| *t == part_type)
                .map_or(part_type, |at| tr(PART_TYPE_NAMES[at])),
            IconGroup::Unused => tr("Other Pictures"),
        };
        let heading = gtk::Label::builder()
            .label(title)
            .xalign(0.0)
            .css_classes(["heading"])
            .build();
        content.append(&heading);
        let (flow, searched) = icon_tiles(&dialog, choices, &pick, &query);
        content.append(&flow);
        shown.push((heading, flow, searched));
    }

    let search = gtk::SearchEntry::builder()
        .placeholder_text(tr("Search"))
        .hexpand(true)
        .build();
    search.connect_search_changed(move |search| {
        *query.borrow_mut() = search.text().to_lowercase();
        let query = query.borrow();
        for (heading, flow, searched) in &shown {
            let any = searched.iter().any(|s| s.contains(query.as_str()));
            heading.set_visible(any);
            flow.set_visible(any);
            flow.invalidate_filter();
        }
    });
    let header = adw::HeaderBar::new();
    header.pack_start(&from_file_button(
        &dialog,
        root,
        Kind::Item,
        Rc::new(move |path: PathBuf| {
            pick(stored_icon(
                &path.file_name().unwrap_or_default().to_string_lossy(),
            ))
        }),
    ));
    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, || topic::EQUIPMENT_PICTURE);
    view.add_top_bar(&header);
    let bar = gtk::Box::builder()
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();
    bar.append(&search);
    view.add_top_bar(&bar);
    view.set_content(Some(&scrolled(&content)));
    dialog.set_child(Some(&view));
    dialog.present(Some(parent));
    search.grab_focus();
}

/// A grid of icons that shows those whose names hold `query`; activating
/// one picks what an item keeps for it and closes the dialog. Also gives
/// each icon's names in lower case, as searched.
fn icon_tiles(
    dialog: &adw::Dialog,
    choices: Vec<IconChoice>,
    pick: &Rc<dyn Fn(String)>,
    query: &Rc<RefCell<String>>,
) -> (gtk::FlowBox, Rc<Vec<String>>) {
    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .activate_on_single_click(true)
        .homogeneous(true)
        .column_spacing(6)
        .row_spacing(6)
        .min_children_per_line(3)
        .max_children_per_line(10)
        .build();
    for choice in &choices {
        let name = choice
            .names
            .first()
            .cloned()
            .unwrap_or_else(|| icon_name(&choice.stored));
        let tile = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .build();
        const SHOWN: usize = 12;
        let mut tooltip: Vec<&str> = choice
            .names
            .iter()
            .take(SHOWN)
            .map(String::as_str)
            .collect();
        if choice.names.len() > SHOWN {
            tooltip.push("…");
        }
        if tooltip.is_empty() {
            tile.set_tooltip_text(Some(&name));
        } else {
            tile.set_tooltip_text(Some(&tooltip.join("\n")));
        }
        let picture = gtk::Image::builder().pixel_size(ICON_SIZE as i32).build();
        picture.set_paintable(icon_texture(&choice.icon).as_ref());
        tile.append(&picture);
        tile.append(
            &gtk::Label::builder()
                .label(&name)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .max_width_chars(10)
                .css_classes(["caption"])
                .build(),
        );
        let child = gtk::FlowBoxChild::builder().child(&tile).build();
        child.update_property(&[gtk::accessible::Property::Label(&name)]);
        flow.append(&child);
    }
    let searched: Rc<Vec<String>> = Rc::new(
        choices
            .iter()
            .map(|c| {
                std::iter::once(icon_name(&c.stored))
                    .chain(c.names.iter().cloned())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .to_lowercase()
            })
            .collect(),
    );
    flow.set_filter_func({
        let (searched, query) = (searched.clone(), query.clone());
        move |child| {
            usize::try_from(child.index())
                .ok()
                .and_then(|i| searched.get(i))
                .is_some_and(|s| s.contains(query.borrow().as_str()))
        }
    });
    let stored: Vec<String> = choices.into_iter().map(|c| c.stored).collect();
    flow.connect_child_activated({
        let (dialog, pick) = (dialog.clone(), pick.clone());
        move |_, child| {
            if let Some(stored) = usize::try_from(child.index())
                .ok()
                .and_then(|i| stored.get(i))
            {
                pick(stored.clone());
                dialog.close();
            }
        }
    });
    (flow, searched)
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
        Kind::Item => ICON_SIZE,
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
