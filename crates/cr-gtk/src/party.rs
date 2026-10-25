//! Deleting characters and the Manage Parties dialog (the Character
//! dialog's Delete and Party Management…).

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use cr_app::parties::PartyLists;
use cr_i18n::{tr, trn};
use cr_store::help::topic;
use cr_store::party::{ALL, Deleted};
use cr_store::{Installation, Result};
use gtk::glib;

pub trait Host {
    fn with_install<T>(&self, run: impl FnOnce(&Installation) -> Result<T>) -> Option<Result<T>>;
    fn toast(&self, message: &str);
    fn add_toast(&self, toast: adw::Toast);
    /// Reloads the sections and shows the entry `key` of section `section`.
    fn reload(&self, section: &str, key: Option<String>);
    fn parent(&self) -> gtk::Widget;
    /// The key of the entry on the detail page.
    fn current_key(&self) -> String;
    /// Group Encounter on the NPCs of `party` (Chars/ files `members`),
    /// over `parent`.
    fn group_encounter(&self, parent: &gtk::Widget, party: &str, members: Vec<String>);
}

fn errors<T>(host: &impl Host, result: Option<Result<T>>) -> Option<T> {
    match result? {
        Ok(value) => Some(value),
        Err(e) => {
            host.toast(&e.to_string());
            None
        }
    }
}

/// Deletes the character of `Chars/<file>`, with Undo on the toast.
pub fn delete_character<H: Host + 'static>(host: &Rc<H>, file: &str) {
    let Some(deleted) = errors(&**host, host.with_install(|i| i.delete_character(file))) else {
        return;
    };
    host.reload("characters", None);
    let message = tr!("“{name}” deleted", name = deleted.name.trim());
    undo_toast(host, &message, vec![deleted]);
}

/// A toast that puts `deleted` back on Undo, and otherwise removes their
/// files when it goes.
fn undo_toast<H: Host + 'static>(host: &Rc<H>, message: &str, deleted: Vec<Deleted>) {
    let root: Option<PathBuf> = host
        .with_install(|i| Ok(i.root().to_path_buf()))
        .and_then(|r| r.ok());
    let toast = adw::Toast::builder()
        .title(glib::markup_escape_text(message).as_str())
        .button_label(tr("_Undo"))
        .timeout(10)
        .use_markup(true)
        .build();
    let deleted = Rc::new(deleted);
    let undone = Rc::new(Cell::new(false));
    toast.connect_button_clicked({
        let (host, deleted, undone) = (host.clone(), deleted.clone(), undone.clone());
        move |_| {
            undone.set(true);
            let result = host.with_install(|install| {
                if Some(install.root()) != root.as_deref() {
                    return Err(cr_store::Error::Invalid(
                        tr("Another installation is open").into(),
                    ));
                }
                install.restore_characters(&deleted)
            });
            if errors(&*host, result).is_some() {
                host.reload("characters", deleted.first().map(|d| d.file.clone()));
            }
        }
    });
    toast.connect_dismissed({
        let host = host.clone();
        move |_| {
            if !undone.get() {
                errors(&*host, host.with_install(|i| i.forget_deleted(&deleted)));
            }
        }
    });
    host.add_toast(toast);
}

/// What the dialog shows: the parties and who is in which.
struct Manager<H: Host + 'static> {
    host: Rc<H>,
    dialog: adw::Dialog,
    toasts: adw::ToastOverlay,
    page: adw::PreferencesPage,
    party_row: adw::ComboRow,
    lists: RefCell<PartyLists>,
    /// The member and unassigned groups, rebuilt on every change.
    groups: RefCell<Vec<adw::PreferencesGroup>>,
    updating: Cell<bool>,
}

/// The Manage Parties dialog.
pub fn manage<H: Host + 'static>(host: &Rc<H>) {
    let party_row = adw::ComboRow::builder().title(tr("Party")).build();
    let new_row = adw::EntryRow::builder()
        .title(tr("New Party"))
        .show_apply_button(true)
        .build();
    let top = adw::PreferencesGroup::builder()
        .description(tr("Empty parties are removed when this window is closed."))
        .build();
    top.add(&party_row);
    top.add(&new_row);
    let page = adw::PreferencesPage::new();
    page.add(&top);
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&page));
    let header = adw::HeaderBar::new();
    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, || topic::PARTY_MANAGEMENT);
    view.add_top_bar(&header);
    view.set_content(Some(&toasts));
    let dialog = adw::Dialog::builder()
        .title(tr("Manage Parties"))
        .content_width(480)
        .content_height(620)
        .child(&view)
        .build();
    let manager = Rc::new(Manager {
        host: host.clone(),
        dialog: dialog.clone(),
        toasts,
        page,
        party_row: party_row.clone(),
        lists: RefCell::default(),
        groups: RefCell::default(),
        updating: Cell::new(false),
    });
    party_row.connect_selected_notify({
        let manager = Rc::downgrade(&manager);
        move |row| {
            let Some(manager) = manager.upgrade() else {
                return;
            };
            if manager.updating.get() {
                return;
            }
            let lists = manager.lists.borrow();
            let party = lists
                .parties
                .get(row.selected() as usize)
                .cloned()
                .unwrap_or_default();
            let new = lists.new.clone();
            drop(lists);
            manager.refresh(&party, &new);
        }
    });
    new_row.connect_apply({
        let manager = Rc::downgrade(&manager);
        move |row| {
            let Some(manager) = manager.upgrade() else {
                return;
            };
            let name = row.text().trim().to_owned();
            let mut new = manager.lists.borrow().new.clone();
            let problem = manager
                .host
                .with_install(|i| i.party_name_problem(&name, &new));
            match problem {
                Some(Ok(None)) => {
                    new.push(name.clone());
                    row.set_text("");
                    manager.refresh(&name, &new);
                }
                Some(Ok(Some(problem))) => manager.notice(&problem),
                Some(Err(e)) => manager.notice(&e.to_string()),
                None => {}
            }
        }
    });
    // The rows' buttons hold the manager; closing the dialog lets it go.
    dialog.connect_closed({
        let manager = RefCell::new(Some(manager.clone()));
        move |_| {
            if let Some(manager) = manager.borrow_mut().take() {
                for group in manager.groups.borrow_mut().drain(..) {
                    manager.page.remove(&group);
                }
            }
        }
    });
    manager.refresh("", &[]);
    dialog.present(Some(&host.parent()));
}

impl<H: Host + 'static> Manager<H> {
    fn notice(&self, message: &str) {
        self.toasts.add_toast(adw::Toast::new(message));
    }

    /// Reads the parties again and shows `selected`, with the parties
    /// `new` made here.
    fn refresh(self: &Rc<Self>, selected: &str, new: &[String]) {
        let Some(lists) = errors(
            &*self.host,
            self.host
                .with_install(|i| PartyLists::read(i, selected, new)),
        ) else {
            return;
        };
        self.updating.set(true);
        let names: Vec<&str> = lists.parties.iter().map(String::as_str).collect();
        self.party_row
            .set_model(Some(&gtk::StringList::new(&names)));
        self.party_row
            .set_selected(lists.position().unwrap_or(0) as u32);
        self.party_row.set_sensitive(!lists.parties.is_empty());
        self.updating.set(false);
        *self.lists.borrow_mut() = lists;
        self.fill();
    }

    /// Rebuilds the member and unassigned lists.
    fn fill(self: &Rc<Self>) {
        for group in self.groups.borrow_mut().drain(..) {
            self.page.remove(&group);
        }
        let lists = self.lists.borrow();
        let party = lists.selected.clone();
        let members = adw::PreferencesGroup::builder()
            .title(tr("Members"))
            .build();
        let others = adw::PreferencesGroup::builder()
            .title(tr("Without a Party"))
            .build();
        let rows = [
            (
                &members,
                &lists.members,
                "list-remove-symbolic",
                tr("Remove from Party"),
                "",
            ),
            (
                &others,
                &lists.without,
                "list-add-symbolic",
                tr("Add to Party"),
                party.as_str(),
            ),
        ];
        for (group, list, icon, tooltip, to) in rows {
            for member in list {
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&member.name).as_str())
                    .subtitle(glib::markup_escape_text(&member.subtitle).as_str())
                    .build();
                if !party.is_empty() {
                    let button = gtk::Button::builder()
                        .icon_name(icon)
                        .tooltip_text(tooltip)
                        .valign(gtk::Align::Center)
                        .css_classes(["flat"])
                        .build();
                    let (manager, file, to) = (self.clone(), member.file.clone(), to.to_owned());
                    button.connect_clicked(move |_| manager.move_to(&file, &to));
                    row.add_suffix(&button);
                    row.set_activatable_widget(Some(&button));
                }
                group.add(&row);
            }
        }
        others.set_description(lists.without_note());
        if !party.is_empty() {
            members.set_description(lists.members_note());
            let buttons = gtk::Box::builder().spacing(6).build();
            if lists.group_encounter {
                let group = gtk::Button::builder()
                    .label(tr("_Group Encounter…"))
                    .use_underline(true)
                    .tooltip_text(tr("Make or print an encounter of the party's NPCs"))
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                let (manager, party, files) = (self.clone(), party.clone(), lists.npcs.clone());
                group.connect_clicked(move |_| {
                    let parent = manager.dialog.clone().upcast();
                    manager.host.group_encounter(&parent, &party, files.clone());
                });
                buttons.append(&group);
            }
            if !lists.members.is_empty() {
                let delete = gtk::Button::builder()
                    .icon_name("user-trash-symbolic")
                    .tooltip_text(tr("Delete Members…"))
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                let manager = self.clone();
                delete.connect_clicked(move |_| manager.confirm_delete());
                buttons.append(&delete);
            }
            members.set_header_suffix(Some(&buttons));
            self.page.add(&members);
            self.groups.borrow_mut().push(members);
        }
        self.page.add(&others);
        self.groups.borrow_mut().push(others);
    }

    /// Puts the character of `file` in `party` (none when empty).
    fn move_to(self: &Rc<Self>, file: &str, party: &str) {
        debug_assert!(!party.eq_ignore_ascii_case(ALL));
        let Some(new) = errors(
            &*self.host,
            self.host.with_install(|i| i.set_party(file, party)),
        ) else {
            return;
        };
        let current = self.host.current_key();
        let key = if current.eq_ignore_ascii_case(file) {
            new
        } else {
            current
        };
        self.host.reload("characters", Some(key));
        let lists = self.lists.borrow();
        let (selected, new) = (lists.selected.clone(), lists.new.clone());
        drop(lists);
        self.refresh(&selected, &new);
    }

    fn confirm_delete(self: &Rc<Self>) {
        let party = self.lists.borrow().selected.clone();
        let alert = adw::AlertDialog::builder()
            .heading(tr("Delete Party Members?"))
            .body(tr!(
                "Every character in “{party}” will be deleted, and the party with them.",
                party
            ))
            .default_response("cancel")
            .close_response("cancel")
            .build();
        alert.add_responses(&[("cancel", tr("_Cancel")), ("delete", tr("_Delete"))]);
        alert.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        let manager = self.clone();
        alert.connect_response(Some("delete"), move |_, _| {
            let host = &manager.host;
            let Some(deleted) = errors(&**host, host.with_install(|i| i.delete_party(&party)))
            else {
                return;
            };
            host.reload("characters", None);
            let message = trn!(
                "{n} character of “{party}” deleted",
                "{n} characters of “{party}” deleted",
                deleted.len(),
                party
            );
            undo_toast(host, &message, deleted);
            manager.dialog.close();
        });
        alert.present(Some(&self.dialog));
    }
}
