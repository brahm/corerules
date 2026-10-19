//! The community add-ons installer: one window that imports the
//! consolidated add-ons, built into the program, into the Core Rules data
//! folder. The add-ons and their documents are in Portuguese, and so is
//! this window.

mod install;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_store::Installation;
use gtk::{gdk, gio, glib};
use install::Folder;

const APP_ID: &str = "io.github.brahm.CoreRules.Addons";

static ICON: &[u8] = include_bytes!(
    "../../../data/addons/icons/hicolor/256x256/apps/io.github.brahm.CoreRules.Addons.png"
);

fn main() -> glib::ExitCode {
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
    folder_row: adw::ActionRow,
    notice: gtk::Label,
    install_button: gtk::Button,
    progress: adw::StatusPage,
    done: adw::StatusPage,
    log: Rc<RefCell<Option<PathBuf>>>,
    dir: Rc<RefCell<Option<PathBuf>>>,
}

impl Window {
    fn new(app: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Addons da comunidade")
            .default_width(560)
            .default_height(640)
            .width_request(360)
            .height_request(294)
            .build();

        let folder_row = adw::ActionRow::builder()
            .title("Pasta de dados do Core Rules")
            .subtitle_selectable(true)
            .build();
        let choose = gtk::Button::builder()
            .label("Escolher…")
            .valign(gtk::Align::Center)
            .build();
        folder_row.add_suffix(&choose);
        let group = adw::PreferencesGroup::new();
        group.add(&folder_row);
        let notice = gtk::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label"])
            .build();
        let install_button = gtk::Button::builder()
            .label("Instalar")
            .halign(gtk::Align::Center)
            .css_classes(["pill", "suggested-action"])
            .build();
        let form = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(18)
            .build();
        form.append(&group);
        form.append(&notice);
        form.append(&install_button);
        let clamp = adw::Clamp::builder().maximum_size(480).child(&form).build();

        let start = adw::StatusPage::builder()
            .title("Addons da comunidade")
            .description(
                "Os 686 addons feitos pela comunidade para o AD&amp;D Core Rules 2.0 e a \
                 Expansion, num só pacote: raças, classes, kits, magias, monstros, \
                 equipamentos, encontros, os personagens dos módulos e a User Database. \
                 São 36.716 itens.",
            )
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
            .title("Instalando os addons")
            .child(&spinner)
            .build();

        let open_log = gtk::Button::builder().label("Abrir o registro").build();
        let close = gtk::Button::builder()
            .label("Fechar")
            .css_classes(["suggested-action"])
            .build();
        let buttons = gtk::Box::builder()
            .spacing(12)
            .halign(gtk::Align::Center)
            .css_classes(["pill"])
            .build();
        buttons.append(&open_log);
        buttons.append(&close);
        let done = adw::StatusPage::builder()
            .icon_name("emblem-ok-symbolic")
            .title("Addons instalados")
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
            folder_row,
            notice,
            install_button,
            progress,
            done,
            log: Rc::default(),
            dir: Rc::default(),
        };
        choose.connect_clicked({
            let this = this.clone();
            move |_| this.choose_folder()
        });
        this.install_button.connect_clicked({
            let this = this.clone();
            move |_| this.confirm_and_install()
        });
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
        close.connect_clicked({
            let window = this.window.clone();
            move |_| window.close()
        });
        let found = Installation::discover().map(|i| i.root().to_path_buf());
        this.set_folder(found);
        this
    }

    fn set_folder(&self, dir: Option<PathBuf>) {
        let ready = match &dir {
            None => {
                self.folder_row
                    .set_subtitle("Não encontrei os dados do Core Rules");
                self.notice.set_label(
                    "Abra o Core Rules e use Extrair dados do CD… no menu para criar a pasta \
                     de dados; depois volte aqui. Se os dados já estão em outro lugar, como \
                     uma instalação da Expansion no Wine, escolha a pasta que tem Database.",
                );
                false
            }
            Some(dir) => {
                self.folder_row
                    .set_subtitle(&glib::markup_escape_text(&dir.to_string_lossy()));
                let (ready, text) = match install::check(dir) {
                    Folder::Ready { again: true, .. } => (
                        true,
                        "Os addons já foram instalados nesta pasta. Instalar de novo \
                         atualiza os registros sem duplicar os personagens.",
                    ),
                    Folder::Ready { .. } => (
                        true,
                        "Antes de importar, o instalador guarda uma cópia de UserDbas e \
                         Chars ao lado da pasta. Um registro seu com o mesmo nome de um \
                         addon é substituído pelo do addon.",
                    ),
                    Folder::Core20 => (
                        false,
                        "Esta pasta é do Core Rules 2.0. Os addons são para a Expansion.",
                    ),
                    Folder::NotAnInstallation => (
                        false,
                        "Esta pasta não tem os dados do Core Rules (falta a pasta Database).",
                    ),
                };
                self.notice.set_label(text);
                ready
            }
        };
        self.install_button.set_sensitive(ready);
        self.dir.replace(dir);
    }

    fn choose_folder(&self) {
        let dialog = gtk::FileDialog::builder()
            .title("Pasta de dados do Core Rules")
            .modal(true)
            .build();
        if let Some(dir) = self.dir.borrow().as_ref() {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            if let Ok(file) = dialog.select_folder_future(Some(&this.window)).await
                && let Some(path) = file.path()
            {
                this.set_folder(Some(path));
            }
        });
    }

    fn confirm_and_install(&self) {
        let Some(dir) = self.dir.borrow().clone() else {
            return;
        };
        let Folder::Ready { install, wine, .. } = install::check(&dir) else {
            return;
        };
        let this = self.clone();
        glib::spawn_future_local(async move {
            if wine {
                let dialog = adw::AlertDialog::new(
                    Some("Instalar no programa original?"),
                    Some(
                        "Esta pasta é a instalação do programa original dentro do Wine. \
                         Os addons também funcionam nele, mas o Import do original leva \
                         cerca de 35 minutos para ler o mesmo pacote.",
                    ),
                );
                dialog.add_responses(&[("cancel", "Cancelar"), ("install", "Instalar")]);
                dialog.set_response_appearance("install", adw::ResponseAppearance::Suggested);
                if dialog.choose_future(&this.window).await != "install" {
                    return;
                }
            }
            this.run(install).await;
        });
    }

    async fn run(&self, install: Installation) {
        self.stack.set_visible_child_name("progress");
        self.window.set_deletable(false);
        let stamp = glib::DateTime::now_local()
            .and_then(|t| t.format("%Y%m%d-%H%M%S"))
            .map_or_else(|_| "addons".into(), |s| s.to_string());

        self.progress
            .set_description(Some("Guardando uma cópia de UserDbas e Chars…"));
        let result = async {
            let (install, backup) = {
                let stamp = stamp.clone();
                gio::spawn_blocking(move || {
                    let backup = install::backup(&install, &stamp);
                    (install, backup)
                })
                .await
                .map_err(|_| "A cópia de segurança falhou.".to_string())?
            };
            let backup = backup?;
            self.progress
                .set_description(Some("Importando os addons. Isso leva menos de um minuto."));
            let outcome = gio::spawn_blocking(move || {
                let file = install::package()?;
                install::import(&install, &file, &stamp)
            })
            .await
            .map_err(|_| "A importação parou com um erro inesperado.".to_string())??;
            Ok::<_, String>((backup, outcome))
        }
        .await;
        self.window.set_deletable(true);

        match result {
            Ok((backup, outcome)) => {
                let imported = match outcome.characters {
                    0 => format!("{} itens importados.", thousands(outcome.imported)),
                    n => format!(
                        "{} itens importados, {n} deles personagens.",
                        thousands(outcome.imported)
                    ),
                };
                let markup = format!(
                    "{imported}\n\nA cópia de segurança ficou em {}. Se o Core Rules \
                     estiver aberto, feche e abra de novo para ver os addons.",
                    glib::markup_escape_text(&backup.to_string_lossy()),
                );
                self.done.set_description(Some(&markup));
                self.log.replace(Some(outcome.log));
                self.stack.set_visible_child_name("done");
            }
            Err(message) => {
                self.stack.set_visible_child_name("start");
                let dialog =
                    adw::AlertDialog::new(Some("Os addons não foram instalados"), Some(&message));
                dialog.add_response("close", "Fechar");
                dialog.present(Some(&self.window));
                let dir = self.dir.borrow().clone();
                self.set_folder(dir);
            }
        }
    }
}

/// 36716 → "36.716", as Portuguese writes it.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn thousands_are_dotted() {
        assert_eq!(super::thousands(36_716), "36.716");
        assert_eq!(super::thousands(105), "105");
        assert_eq!(super::thousands(1_000_000), "1.000.000");
    }
}
