//! The add-on details, shown on request: every add-on folder of the
//! community's archive, what its packages brought into the collection and
//! the texts that came with it.

use adw::prelude::*;
use cr_i18n::{tr, trn};
use gtk::glib;

use crate::extras::{self, Addon, Extras};

/// Item names a category shows before the rest are only counted.
const SHOWN: usize = 300;

/// Opens the details over `parent`, unpacking the texts first.
pub fn show(parent: &impl IsA<gtk::Widget>) {
    let nav = adw::NavigationView::new();
    let spinner = gtk::Spinner::builder()
        .spinning(true)
        .width_request(32)
        .height_request(32)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .build();
    nav.add(&page(tr("Add-on Details"), &spinner, None));
    let dialog = adw::Dialog::builder()
        .title(tr("Add-on Details"))
        .content_width(640)
        .content_height(640)
        .child(&nav)
        .build();
    dialog.present(Some(parent));
    glib::spawn_future_local(async move {
        let result = gtk::gio::spawn_blocking(extras::get)
            .await
            .unwrap_or_else(|_| Err(tr("The built-in pictures are damaged.").to_string()));
        match result {
            Ok(extras) => nav.replace(&[list(extras, &nav)]),
            Err(message) => {
                let status = adw::StatusPage::builder()
                    .icon_name("dialog-error-symbolic")
                    .title(tr("The Details Could Not Be Read"))
                    .description(glib::markup_escape_text(&message))
                    .build();
                nav.replace(&[page(tr("Add-on Details"), &status, None)]);
            }
        }
    });
}

/// A page with a header bar over `content`, and a search bar when given.
fn page(
    title: &str,
    content: &impl IsA<gtk::Widget>,
    search: Option<&gtk::SearchEntry>,
) -> adw::NavigationPage {
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    if let Some(search) = search {
        let bar = adw::Clamp::builder()
            .maximum_size(560)
            .child(search)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(6)
            .build();
        view.add_top_bar(&bar);
    }
    view.set_content(Some(content));
    adw::NavigationPage::builder()
        .title(title)
        .child(&view)
        .build()
}

fn scrolled(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(child)
        .build()
}

/// The add-on folders, with a search on their names.
fn list(extras: &'static Extras, nav: &adw::NavigationView) -> adw::NavigationPage {
    let addons: Vec<Addon> = extras
        .addons()
        .into_iter()
        .filter(|a| a.total() > 0 || !a.texts.is_empty())
        .collect();
    let rows = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .valign(gtk::Align::Start)
        .build();
    let addons = std::rc::Rc::new(addons);
    for addon in addons.iter() {
        let mut subtitle = vec![trn!(
            "{count} item",
            "{count} items",
            addon.total() as u64,
            count = crate::thousands(addon.total())
        )];
        if !addon.texts.is_empty() {
            subtitle.push(trn!("{n} text", "{n} texts", addon.texts.len() as u64));
        }
        let parent = addon.folder.rsplit_once('/').map_or("", |(p, _)| p);
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(addon.name()))
            .subtitle(glib::markup_escape_text(&if parent.is_empty() {
                subtitle.join(" · ")
            } else {
                format!("{parent} · {}", subtitle.join(" · "))
            }))
            .activatable(true)
            .build();
        row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
        rows.append(&row);
    }
    rows.connect_row_activated({
        let nav = nav.clone();
        let addons = addons.clone();
        move |_, row| {
            if let Some(addon) = addons.get(row.index() as usize) {
                nav.push(&addon_page(extras, addon, &nav));
            }
        }
    });
    let search = gtk::SearchEntry::builder()
        .placeholder_text(tr("Search the add-ons"))
        .hexpand(true)
        .build();
    rows.set_filter_func({
        let search = search.clone();
        let addons = addons.clone();
        move |row| {
            let text = search.text().to_lowercase();
            text.is_empty()
                || addons
                    .get(row.index() as usize)
                    .is_some_and(|a| a.folder.to_lowercase().contains(&text))
        }
    });
    search.connect_search_changed({
        let rows = rows.clone();
        move |_| rows.invalidate_filter()
    });
    let intro = gtk::Label::builder()
        .label(tr(
            "Each add-on folder of the community's archive, what its packages brought into the \
             collection and the texts its authors wrote. Most texts are in English.",
        ))
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label"])
        .build();
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(6)
        .margin_bottom(18)
        .build();
    column.append(&intro);
    column.append(&rows);
    let clamp = adw::Clamp::builder()
        .maximum_size(560)
        .margin_start(12)
        .margin_end(12)
        .child(&column)
        .build();
    page(tr("Add-on Details"), &scrolled(&clamp), Some(&search))
}

/// One add-on: its packages and items, then its texts.
fn addon_page(
    extras: &'static Extras,
    addon: &Addon,
    nav: &adw::NavigationView,
) -> adw::NavigationPage {
    let prefs = adw::PreferencesPage::new();
    let heading = adw::PreferencesGroup::builder()
        .title(glib::markup_escape_text(&addon.folder))
        .build();
    prefs.add(&heading);
    if !addon.texts.is_empty() {
        let group = adw::PreferencesGroup::builder().title(tr("Texts")).build();
        for (path, name) in &addon.texts {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(name))
                .activatable(true)
                .build();
            row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
            row.connect_activated({
                let nav = nav.clone();
                let (path, name) = (path.clone(), name.clone());
                move |_| nav.push(&text_page(&name, &extras.text(&path)))
            });
            group.add(&row);
        }
        prefs.add(&group);
    }
    for package in &addon.packages {
        let file = package.path.rsplit('/').next().unwrap_or(&package.path);
        let group = adw::PreferencesGroup::builder()
            .title(glib::markup_escape_text(file))
            .description(match (package.items.len(), package.total) {
                (kept, total) if kept >= total => trn!(
                    "{count} item, all in the collection",
                    "{count} items, all in the collection",
                    kept as u64,
                    count = crate::thousands(kept)
                ),
                (0, total) => trn!(
                    "{n} item, which a newer add-on replaced",
                    "{n} items, which newer add-ons replaced",
                    total as u64
                ),
                (kept, total) => trn!(
                    "{n} item; {kept} are in the collection as written here",
                    "{n} items; {kept} are in the collection as written here",
                    total as u64,
                    kept
                ),
            })
            .build();
        let mut categories: Vec<(&str, Vec<&str>)> = Vec::new();
        for (category, name) in &package.items {
            match categories.iter_mut().find(|(c, _)| c == category) {
                Some((_, names)) => names.push(name),
                None => categories.push((category, vec![name])),
            }
        }
        for (category, names) in categories {
            let mut text = names
                .iter()
                .take(SHOWN)
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
            if names.len() > SHOWN {
                text.push_str(", ");
                text.push_str(&trn!(
                    "and {n} more",
                    "and {n} more",
                    (names.len() - SHOWN) as u64
                ));
            }
            let row = adw::ExpanderRow::builder()
                .title(glib::markup_escape_text(tr(category)))
                .subtitle(names.len().to_string())
                .build();
            row.add_row(
                &gtk::Label::builder()
                    .label(&text)
                    .wrap(true)
                    .wrap_mode(gtk::pango::WrapMode::WordChar)
                    .xalign(0.0)
                    .selectable(true)
                    .margin_top(12)
                    .margin_bottom(12)
                    .margin_start(12)
                    .margin_end(12)
                    .build(),
            );
            group.add(&row);
        }
        prefs.add(&group);
    }
    page(addon.name(), &prefs, None)
}

/// A text of an add-on, as its author wrote it.
fn text_page(name: &str, text: &str) -> adw::NavigationPage {
    let view = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(12)
        .bottom_margin(12)
        .left_margin(12)
        .right_margin(12)
        .build();
    view.buffer().set_text(text);
    page(name, &scrolled(&view), None)
}
