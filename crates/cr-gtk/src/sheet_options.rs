//! The Expansion's Character Sheet Options (dialog 414): which sections
//! the sheet prints, in what order, and the border style of its boxes.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use cr_app::sheets::section_name;
use cr_i18n::tr;
use cr_store::help::topic;
use cr_store::sheet::{Border, SheetChoice};

/// A change to the chosen sections.
type Edit = Box<dyn Fn(&mut Vec<u32>)>;

/// Opens the options over `parent`, starting from `choice`; Save hands
/// the new choice to `save`.
pub fn choose(
    parent: &impl IsA<gtk::Widget>,
    choice: SheetChoice,
    save: impl Fn(SheetChoice) + 'static,
) {
    let sections = Rc::new(RefCell::new(choice.sections));

    let border = adw::ComboRow::builder()
        .title(tr("Border Style"))
        .subtitle(tr("How the printed sheet frames its boxes"))
        .model(&gtk::StringList::new(
            Border::ALL.map(|b| b.label()).as_ref(),
        ))
        .build();
    border.set_selected(
        Border::ALL
            .iter()
            .position(|b| *b == choice.border)
            .unwrap_or(2) as u32,
    );
    let style = adw::PreferencesGroup::new();
    style.add(&border);

    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    list.set_placeholder(Some(
        &gtk::Label::builder()
            .label(tr("No sections: the sheet will be empty"))
            .css_classes(["dim-label"])
            .margin_top(12)
            .margin_bottom(12)
            .build(),
    ));

    let offered = cr_app::sheets::offered();
    let menu = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .build();
    for &id in &offered {
        menu.append(
            &gtk::Label::builder()
                .label(section_name(id))
                .xalign(0.0)
                .margin_top(6)
                .margin_bottom(6)
                .margin_start(6)
                .margin_end(6)
                .build(),
        );
    }
    let menu_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .max_content_height(360)
        .child(&menu)
        .build();
    let popover = gtk::Popover::builder().child(&menu_scroll).build();
    let add = gtk::MenuButton::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text(tr("Add Section"))
        .popover(&popover)
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    let chosen = adw::PreferencesGroup::builder()
        .title(tr("Sections"))
        .description(tr(
            "Printed in this order. A section can be added more than once.",
        ))
        .header_suffix(&add)
        .build();
    chosen.add(&list);

    // Lists the chosen sections again, each with its move and remove
    // buttons.
    let refill: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
    *refill.borrow_mut() = Box::new({
        let (list, sections, refill) = (list.clone(), sections.clone(), Rc::downgrade(&refill));
        move || {
            list.remove_all();
            let count = sections.borrow().len();
            for (i, &id) in sections.borrow().iter().enumerate() {
                let row = adw::ActionRow::builder().title(section_name(id)).build();
                let button = |icon: &str, tip: String, enabled: bool| {
                    let b = gtk::Button::builder()
                        .icon_name(icon)
                        .tooltip_text(tip)
                        .css_classes(["flat"])
                        .valign(gtk::Align::Center)
                        .sensitive(enabled)
                        .build();
                    row.add_suffix(&b);
                    b
                };
                let up = button("go-up-symbolic", tr("Move Up").into(), i > 0);
                let down = button("go-down-symbolic", tr("Move Down").into(), i + 1 < count);
                let remove = button("list-remove-symbolic", tr("Remove").into(), true);
                let change = |b: &gtk::Button, edit: Edit| {
                    let (sections, refill) = (sections.clone(), refill.clone());
                    b.connect_clicked(move |_| {
                        edit(&mut sections.borrow_mut());
                        if let Some(refill) = refill.upgrade() {
                            refill.borrow()();
                        }
                    });
                };
                change(&up, Box::new(move |s| s.swap(i - 1, i)));
                change(&down, Box::new(move |s| s.swap(i, i + 1)));
                change(
                    &remove,
                    Box::new(move |s| {
                        s.remove(i);
                    }),
                );
                list.append(&row);
            }
        }
    });
    refill.borrow()();
    menu.connect_row_activated({
        let (sections, refill, popover) = (sections.clone(), refill.clone(), popover.clone());
        move |_, row| {
            if let Some(&id) = offered.get(row.index() as usize) {
                sections.borrow_mut().push(id);
                popover.popdown();
                refill.borrow()();
            }
        }
    });
    menu.set_activate_on_single_click(true);

    let page = adw::PreferencesPage::new();
    page.add(&style);
    page.add(&chosen);
    let cancel = gtk::Button::with_mnemonic(tr("_Cancel"));
    let ok = gtk::Button::builder()
        .label(tr("_Save"))
        .use_underline(true)
        .css_classes(["suggested-action"])
        .build();
    let header = adw::HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .build();
    header.pack_start(&cancel);
    header.pack_end(&ok);
    let view = adw::ToolbarView::new();
    crate::help::add_button(&header, &view, || topic::CHARACTER_SHEET);
    view.add_top_bar(&header);
    view.set_content(Some(&page));
    let dialog = adw::Dialog::builder()
        .title(tr("Character Sheet Options"))
        .content_width(460)
        .content_height(640)
        .child(&view)
        .build();
    cancel.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    let save = Rc::new(move |sections: Vec<u32>, border: u32| {
        save(SheetChoice {
            sections,
            border: Border::ALL
                .get(border as usize)
                .copied()
                .unwrap_or(Border::Normal),
        });
    });
    ok.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            let (chosen, style) = (sections.borrow().clone(), border.selected());
            if !chosen.is_empty() {
                save(chosen, style);
                dialog.close();
                return;
            }
            // An empty sheet is asked about first.
            let alert = adw::AlertDialog::builder()
                .heading(tr("No Sections"))
                .body(tr("No sections: the sheet will be empty"))
                .build();
            alert.add_responses(&[("cancel", tr("_Cancel")), ("save", tr("_Save"))]);
            alert.set_response_appearance("save", adw::ResponseAppearance::Destructive);
            let (dialog, save) = (dialog.clone(), save.clone());
            alert.choose(&dialog.clone(), gtk::gio::Cancellable::NONE, move |r| {
                if r == "save" {
                    save(chosen, style);
                    dialog.close();
                }
            });
        }
    });
    dialog.present(Some(parent));
}
