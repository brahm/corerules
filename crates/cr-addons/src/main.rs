//! Core Rules Community Add-ons Collection: one window that imports the
//! consolidated add-ons, built into the program, into the Core Rules data
//! folder. The window speaks the app's languages and follows the one chosen
//! in its Preferences; the add-ons themselves are in Portuguese.

mod details;
mod extras;
mod install;
mod locate;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use cr_i18n::{Language, tr, trn};
use cr_store::Installation;
use gtk::{gdk, gio, glib};
use install::Folder;
use locate::{App, Places};

const APP_ID: &str = "io.github.brahm.CoreRules.Addons";

static ICON: &[u8] = include_bytes!(
    "../../../data/addons/icons/hicolor/256x256/apps/io.github.brahm.CoreRules.Addons.png"
);

fn main() -> glib::ExitCode {
    apply_language();
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        gtk::Window::set_default_icon_name(APP_ID);
        Window::new(app).window.present();
    });
    app.run()
}

#[derive(Clone)]
struct Window {
    window: adw::ApplicationWindow,
    stack: gtk::Stack,
    group: adw::PreferencesGroup,
    rows: Rc<RefCell<Vec<adw::ActionRow>>>,
    notice: gtk::Label,
    install_button: gtk::Button,
    start_app: gtk::Button,
    progress: adw::StatusPage,
    done: adw::StatusPage,
    done_app: gtk::Button,
    log: Rc<RefCell<Option<PathBuf>>>,
    /// The copies of the app found on this computer.
    apps: Rc<RefCell<Vec<App>>>,
    /// A folder picked with "Outra pasta…", which replaces the ones found.
    chosen: Rc<RefCell<Option<PathBuf>>>,
    /// The data folders the add-ons go into.
    targets: Rc<RefCell<Vec<PathBuf>>>,
}

impl Window {
    fn new(app: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(tr("Core Rules Community Add-ons Collection"))
            .default_width(560)
            .default_height(680)
            .width_request(360)
            .height_request(294)
            .build();

        let group = adw::PreferencesGroup::builder()
            .title(tr("Core Rules on This Computer"))
            .build();
        let rescan = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text(tr("Search Again"))
            .css_classes(["flat"])
            .valign(gtk::Align::Center)
            .build();
        group.set_header_suffix(Some(&rescan));
        let notice = gtk::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label"])
            .build();
        let install_button = gtk::Button::builder()
            .label(tr("Install"))
            .css_classes(["pill", "suggested-action"])
            .build();
        let start_app = gtk::Button::builder()
            .label(tr("Open Core Rules"))
            .css_classes(["pill"])
            .build();
        let actions = gtk::Box::builder()
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();
        actions.append(&start_app);
        actions.append(&install_button);
        let other = gtk::Button::builder()
            .label(tr("Other Folder…"))
            .css_classes(["flat"])
            .build();
        let details = gtk::Button::builder()
            .label(tr("Add-on Details…"))
            .css_classes(["flat"])
            .build();
        details.connect_clicked(details::show);
        let more = gtk::Box::builder()
            .spacing(6)
            .halign(gtk::Align::Center)
            .build();
        more.append(&other);
        more.append(&details);
        let form = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(18)
            .build();
        form.append(&group);
        form.append(&notice);
        form.append(&actions);
        form.append(&more);
        let clamp = adw::Clamp::builder().maximum_size(480).child(&form).build();

        let start = adw::StatusPage::builder()
            .title(tr("Core Rules Community Add-ons Collection"))
            .description(tr!(
                "The {addons} add-ons the community made for AD&amp;D Core Rules 2.0 and the \
                 Expansion, in one package: races, classes, kits, spells, monsters, \
                 equipment, encounters, the characters of the modules and the User \
                 Database. {items} items in all, with the pictures that came with them.",
                addons = 672,
                items = thousands(36_525)
            ))
            .child(&clamp)
            .build();
        start.set_paintable(
            gdk::Texture::from_bytes(&glib::Bytes::from_static(ICON))
                .ok()
                .as_ref(),
        );
        start.add_css_class("compact");

        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .width_request(32)
            .height_request(32)
            .halign(gtk::Align::Center)
            .build();
        let progress = adw::StatusPage::builder()
            .title(tr("Installing the Add-ons"))
            .child(&spinner)
            .build();

        let open_log = gtk::Button::builder().label(tr("Open the Log")).build();
        let done_app = gtk::Button::builder()
            .label(tr("Open Core Rules"))
            .css_classes(["suggested-action"])
            .build();
        let buttons = gtk::Box::builder()
            .spacing(12)
            .halign(gtk::Align::Center)
            .css_classes(["pill"])
            .build();
        buttons.append(&open_log);
        buttons.append(&done_app);
        let done = adw::StatusPage::builder()
            .icon_name("emblem-ok-symbolic")
            .title(tr("Add-ons Installed"))
            .child(&buttons)
            .build();

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
        stack.add_named(&start, Some("start"));
        stack.add_named(&progress, Some("progress"));
        stack.add_named(&done, Some("done"));
        let view = adw::ToolbarView::new();
        view.add_top_bar(&adw::HeaderBar::builder().show_title(false).build());
        view.set_content(Some(&stack));
        window.set_content(Some(&view));

        let this = Self {
            window,
            stack,
            group,
            rows: Rc::default(),
            notice,
            install_button,
            start_app,
            progress,
            done,
            done_app,
            log: Rc::default(),
            apps: Rc::default(),
            chosen: Rc::default(),
            targets: Rc::default(),
        };
        rescan.connect_clicked({
            let this = this.clone();
            move |_| {
                this.chosen.replace(None);
                this.scan();
            }
        });
        other.connect_clicked({
            let this = this.clone();
            move |_| this.choose_folder()
        });
        this.install_button.connect_clicked({
            let this = this.clone();
            move |_| this.confirm_and_install()
        });
        for button in [&this.start_app, &this.done_app] {
            button.connect_clicked({
                let this = this.clone();
                move |_| this.launch_app()
            });
        }
        open_log.connect_clicked({
            let this = this.clone();
            move |_| {
                if let Some(log) = this.log.borrow().as_ref() {
                    gtk::FileLauncher::new(Some(&gio::File::for_path(log))).launch(
                        Some(&this.window),
                        gio::Cancellable::NONE,
                        |_| {},
                    );
                }
            }
        });
        this.scan();
        this
    }

    /// Looks for the app again and shows what it found.
    fn scan(&self) {
        self.apps.replace(locate::apps(&Places::here()));
        self.show();
    }

    fn show(&self) {
        for row in self.rows.borrow_mut().drain(..) {
            self.group.remove(&row);
        }
        let apps = self.apps.borrow();
        let mut rows = Vec::new();
        let mut targets: Vec<PathBuf> = Vec::new();
        let add_row = |rows: &mut Vec<adw::ActionRow>, title: &str, subtitle: &str| {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(title))
                .subtitle(glib::markup_escape_text(subtitle))
                .subtitle_selectable(true)
                .build();
            self.group.add(&row);
            rows.push(row);
        };
        // A folder the app reads, as the add-ons see it.
        let describe = |dir: &Path, wine: bool| -> (bool, String) {
            let place = home_relative(dir);
            match install::check(dir) {
                Folder::Ready { again, .. } => {
                    let text = match (wine, again) {
                        (false, false) => tr!("Data in {place}", place),
                        (false, true) => tr!(
                            "Data in {place}. The add-ons were installed here before.",
                            place
                        ),
                        (true, false) => {
                            tr!("Data of the original program in Wine: {place}", place)
                        }
                        (true, true) => tr!(
                            "Data of the original program in Wine: {place}. The add-ons were \
                             installed here before.",
                            place
                        ),
                    };
                    (true, text)
                }
                Folder::Core20 => (
                    false,
                    tr!(
                        "{place} has Core Rules 2.0; the add-ons are for the Expansion.",
                        place
                    ),
                ),
                Folder::NotAnInstallation => (
                    false,
                    tr("No game data yet: open it and use Extract Data from CD… in the menu.")
                        .to_string(),
                ),
            }
        };
        if let Some(dir) = self.chosen.borrow().as_ref() {
            let (ready, text) = describe(dir, dir.to_string_lossy().contains("/drive_c/"));
            add_row(&mut rows, tr("Chosen Folder"), &text);
            if ready {
                targets.push(dir.clone());
            }
        } else {
            for app in apps.iter() {
                let (ready, mut text) = describe(&app.data, app.wine);
                if !app.reads_addons() {
                    text.push(' ');
                    text.push_str(tr(
                        "Update the app to version 0.7.2 or newer to see all the add-ons.",
                    ));
                }
                add_row(&mut rows, &app.label(), &text);
                if ready && !targets.contains(&app.data) {
                    targets.push(app.data.clone());
                }
            }
            // Data the app would open even when no copy of it was found
            // (built from source, say).
            if apps.is_empty()
                && let Some(install) = Installation::discover()
            {
                let dir = install.root().to_path_buf();
                let (ready, text) = describe(&dir, dir.to_string_lossy().contains("/drive_c/"));
                add_row(&mut rows, tr("Core Rules Data"), &text);
                if ready {
                    targets.push(dir);
                }
            }
        }
        if rows.is_empty() {
            add_row(
                &mut rows,
                tr("No Core Rules Found"),
                tr(
                    "I looked for the Flatpak, the .deb and .rpm packages and AppImages in the \
                    usual folders (Downloads, Applications, Desktop and the home folder).",
                ),
            );
        }
        self.rows.replace(rows);

        let text = if !targets.is_empty() {
            trn!(
                "The add-ons will be installed in the data folder above. First the installer \
                 keeps a copy of UserDbas and Chars beside it. A record of yours with the same \
                 name as an add-on is replaced by the add-on.",
                "The add-ons will be installed in the {n} data folders above. First the \
                 installer keeps a copy of UserDbas and Chars beside each one. A record of \
                 yours with the same name as an add-on is replaced by the add-on.",
                targets.len() as u64
            )
        } else if apps.is_empty() && self.chosen.borrow().is_none() {
            tr(
                "Install Core Rules 0.7.2 or newer from the project's releases page, open it \
                and use Extract Data from CD… in the menu; then search again here.",
            )
            .to_string()
        } else {
            tr(
                "Open Core Rules and use Extract Data from CD… in the menu to create the data \
                folder; then search again here.",
            )
            .to_string()
        };
        self.notice.set_label(&text);
        self.install_button.set_sensitive(!targets.is_empty());
        self.start_app
            .set_visible(targets.is_empty() && !apps.is_empty());
        self.done_app.set_visible(!apps.is_empty());
        self.targets.replace(targets);
    }

    fn choose_folder(&self) {
        let dialog = gtk::FileDialog::builder()
            .title(tr("Core Rules Data Folder"))
            .modal(true)
            .build();
        if let Some(dir) = self.targets.borrow().first() {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            if let Ok(file) = dialog.select_folder_future(Some(&this.window)).await
                && let Some(path) = file.path()
            {
                this.chosen.replace(Some(path));
                this.show();
            }
        });
    }

    /// Starts the copy of the app that reads the first folder installed
    /// into, else the first one found.
    fn launch_app(&self) {
        let apps = self.apps.borrow();
        let targets = self.targets.borrow();
        let app = apps
            .iter()
            .find(|a| targets.first() == Some(&a.data))
            .or_else(|| apps.first());
        if let Some(app) = app
            && let Err(e) = app.launch()
        {
            let dialog =
                adw::AlertDialog::new(Some(tr("Could not open Core Rules")), Some(&e.to_string()));
            dialog.add_response("close", tr("Close"));
            dialog.present(Some(&self.window));
        }
    }

    fn confirm_and_install(&self) {
        let mut installs = Vec::new();
        let mut wine = false;
        for dir in self.targets.borrow().iter() {
            if let Folder::Ready {
                install, wine: w, ..
            } = install::check(dir)
            {
                wine |= w;
                installs.push(install);
            }
        }
        if installs.is_empty() {
            return;
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            if wine {
                let dialog = adw::AlertDialog::new(
                    Some(tr("Install into the Original Program?")),
                    Some(tr(
                        "One of the folders is the original program's installation \
                        inside Wine. The add-ons work there too, but the original's Import \
                        takes about 35 minutes to read the same package.",
                    )),
                );
                dialog.add_responses(&[("cancel", tr("Cancel")), ("install", tr("Install"))]);
                dialog.set_response_appearance("install", adw::ResponseAppearance::Suggested);
                if dialog.choose_future(&this.window).await != "install" {
                    return;
                }
            }
            this.run(installs).await;
        });
    }

    async fn run(&self, installs: Vec<Installation>) {
        self.stack.set_visible_child_name("progress");
        self.window.set_deletable(false);
        let stamp = glib::DateTime::now_local()
            .and_then(|t| t.format("%Y%m%d-%H%M%S"))
            .map_or_else(|_| "addons".into(), |s| s.to_string());

        let mut done = Vec::new();
        let mut failure = None;
        for install in installs {
            let place = home_relative(install.root());
            self.progress
                .set_description(Some(&glib::markup_escape_text(&tr!(
                    "Keeping a copy of UserDbas and Chars from {place}…",
                    place
                ))));
            let result = async {
                let (install, backup) = {
                    let stamp = stamp.clone();
                    gio::spawn_blocking(move || {
                        let backup = install::backup(&install, &stamp);
                        (install, backup)
                    })
                    .await
                    .map_err(|_| tr("The backup failed.").to_string())?
                };
                let backup = backup?;
                self.progress
                    .set_description(Some(&glib::markup_escape_text(&tr!(
                        "Importing the add-ons into {place}. This takes less than a minute.",
                        place
                    ))));
                let stamp = stamp.clone();
                let outcome = gio::spawn_blocking(move || {
                    let file = install::package()?;
                    let mut outcome = install::import(&install, &file, &stamp)?;
                    install::pictures(&install, &mut outcome)?;
                    Ok::<_, String>(outcome)
                })
                .await
                .map_err(|_| tr("The import stopped with an unexpected error.").to_string())??;
                Ok::<_, String>((place.clone(), backup, outcome))
            }
            .await;
            match result {
                Ok(d) => done.push(d),
                Err(message) => {
                    failure = Some(format!("{place}: {message}"));
                    break;
                }
            }
        }
        self.window.set_deletable(true);

        if let Some(message) = failure {
            self.stack.set_visible_child_name("start");
            let dialog =
                adw::AlertDialog::new(Some(tr("The Add-ons Were Not Installed")), Some(&message));
            dialog.add_response("close", tr("Close"));
            dialog.present(Some(&self.window));
            self.scan();
            return;
        }
        let mut text = String::new();
        for (place, backup, outcome) in &done {
            let items = thousands(outcome.imported);
            let backup = home_relative(backup);
            text.push_str(&match outcome.characters {
                0 => tr!(
                    "{items} items imported into {place}. The backup is in {backup}.",
                    items,
                    place,
                    backup
                ),
                n => trn!(
                    "{items} items imported into {place}, {n} of them a character. The backup \
                     is in {backup}.",
                    "{items} items imported into {place}, {n} of them characters. The backup \
                     is in {backup}.",
                    n as u64,
                    items,
                    place,
                    backup
                ),
            });
            if outcome.pictures > 0 {
                text.push(' ');
                text.push_str(&trn!(
                    "{n} picture of the add-ons was copied there.",
                    "{n} pictures of the add-ons were copied there.",
                    outcome.pictures as u64
                ));
            }
            text.push_str("\n\n");
        }
        text.push_str(tr(
            "If Core Rules is open, close it and open it again to see the add-ons.",
        ));
        self.done
            .set_description(Some(&glib::markup_escape_text(&text)));
        self.log
            .replace(done.last().map(|(_, _, outcome)| outcome.log.clone()));
        self.stack.set_visible_child_name("done");
    }
}

/// `/home/ana/.local/share/corerules/data` → `~/.local/share/corerules/data`.
fn home_relative(path: &Path) -> String {
    match std::env::var_os("HOME").map(PathBuf::from) {
        Some(home) if path.starts_with(&home) && home != Path::new("/") => {
            format!("~/{}", path.strip_prefix(&home).unwrap_or(path).display())
        }
        _ => path.display().to_string(),
    }
}

/// Picks the language before any text is shown: the one chosen in the
/// app's Preferences (of the native app, else of the Flatpak), else the
/// system's. A chosen language is passed to GTK too, for its own dialogs.
fn apply_language() {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let files = [
        Some(glib::user_config_dir()),
        home.map(|h| h.join(".var/app/io.github.brahm.CoreRules/config")),
    ];
    let chosen = files.into_iter().flatten().find_map(|dir| {
        let file = glib::KeyFile::new();
        file.load_from_file(dir.join("corerules/settings.ini"), glib::KeyFileFlags::NONE)
            .ok()?;
        Language::from_code(&file.string("interface", "language").ok()?)
    });
    let language = match chosen {
        Some(language) => {
            // SAFETY: called first thing in main, before any thread starts.
            unsafe { std::env::set_var("LANGUAGE", language.code()) };
            language
        }
        None => cr_i18n::system_language(),
    };
    cr_i18n::set_language(language);
}

/// 36716 → "36,716" in English, "36.716" in Portuguese.
fn thousands(n: usize) -> String {
    group_digits(n, cr_i18n::language())
}

fn group_digits(n: usize, language: Language) -> String {
    let separator = match language {
        Language::English => ',',
        Language::French => '\u{202f}',
        _ => '.',
    };
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::group_digits;
    use cr_i18n::Language;

    #[test]
    fn thousands_are_grouped_as_each_language_writes_them() {
        assert_eq!(group_digits(36_716, Language::PortugueseBrazil), "36.716");
        assert_eq!(group_digits(36_716, Language::English), "36,716");
        assert_eq!(group_digits(36_716, Language::French), "36\u{202f}716");
        assert_eq!(group_digits(105, Language::German), "105");
        assert_eq!(group_digits(1_000_000, Language::Spanish), "1.000.000");
    }
}
