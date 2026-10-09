//! First cut of the GNOME app: lists the characters of the Core Rules
//! installation found in the Wine prefix.

use adw::prelude::*;
use cr_store::Installation;

const APP_ID: &str = "io.github.brahm.CoreRules";

fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_window);
    app.run()
}

fn build_window(app: &adw::Application) {
    let content = match Installation::discover() {
        Some(install) => character_list(&install),
        None => adw::StatusPage::builder()
            .icon_name("folder-symbolic")
            .title("No Core Rules Installation Found")
            .description("Install AD&D Core Rules 2.0 with Wine, then open this app again.")
            .build()
            .upcast(),
    };

    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&content));

    adw::ApplicationWindow::builder()
        .application(app)
        .title("Core Rules")
        .default_width(720)
        .default_height(560)
        .content(&view)
        .build()
        .present();
}

fn character_list(install: &Installation) -> gtk::Widget {
    let group = adw::PreferencesGroup::builder().title("Characters").build();
    match install.characters() {
        Ok(index) if index.entries.is_empty() => {
            group.set_description(Some("No characters yet."));
        }
        Ok(index) => {
            for entry in index.entries {
                let subtitle = entry.details.get(3..).unwrap_or_default().join(" · ");
                let row = adw::ActionRow::builder()
                    .title(&entry.name)
                    .subtitle(subtitle)
                    .build();
                group.add(&row);
            }
        }
        Err(e) => group.set_description(Some(&e.to_string())),
    }
    let page = adw::PreferencesPage::new();
    page.add(&group);
    page.upcast()
}
