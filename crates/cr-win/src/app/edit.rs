//! Edit Character: the editor of `cr_app::editor` on a page of the
//! window, its pages chosen above them, with Save and Cancel under them.
//! The Details page has the Name page of Update Character, the classes
//! with Add Class, the experience with Levels… and Add…, the appearance,
//! the ability scores and the history.

use cr_app::create::none_label;
use cr_app::editor::{EditInput, Editor, MOST_AGE, MOST_HEIGHT, MOST_POINTS, MOST_WEIGHT, Tab};
use cr_app::levels::LevelsInput;
use cr_i18n::{tr, tr_noop};
use cr_store::create::{ABILITIES, SUBABILITIES};
use cr_store::edit::ABILITY_RANGE;
use cr_win::text::plain;
use windows_reactor::*;

use super::create::{heading, prompt_dialog, with_buttons};
use super::portrait::Shown;
use super::{Message, Shell};
use crate::ui;

const GENDERS: [&str; 2] = [tr_noop!("Male"), tr_noop!("Female")];

/// An editor input as the shell's message.
fn input(input: EditInput) -> Message {
    Message::Editor(input)
}

fn levels(change: LevelsInput) -> Message {
    input(EditInput::Levels(change))
}

fn experience(change: LevelsInput) -> Message {
    input(EditInput::Experience(change))
}

/// A NumberBox for a whole number in `0..=max`.
fn number(
    name: &str,
    value: u32,
    (min, max): (u32, u32),
    changed: fn(u32) -> EditInput,
    context: &mut ViewContext<Shell>,
) -> NumberBox {
    NumberBox::new()
        .automation_name(name)
        .value(f64::from(value))
        .minimum(f64::from(min))
        .maximum(f64::from(max))
        .min_width(120.0)
        .on_value_changed(context.callback(move |v: Option<f64>| match v {
            Some(v) => input(changed((v.round().max(0.0) as u32).clamp(min, max))),
            None => Message::Nothing,
        }))
}

fn text(
    name: &str,
    value: &str,
    changed: fn(String) -> EditInput,
    context: &mut ViewContext<Shell>,
) -> TextBox {
    TextBox::new()
        .automation_name(name)
        .text(value)
        .min_width(260.0)
        .on_text_changed(context.callback(move |text: String| input(changed(text))))
}

fn combo(
    name: &str,
    items: Vec<String>,
    selected: Option<usize>,
    changed: fn(usize) -> EditInput,
    context: &mut ViewContext<Shell>,
) -> ComboBox {
    ComboBox::new()
        .automation_name(name)
        .items_source(items)
        .selected_index(selected)
        .min_width(220.0)
        .on_selection_changed(context.callback(move |i: Option<usize>| match i {
            Some(i) => input(changed(i)),
            None => Message::Nothing,
        }))
}

/// The editor; `portrait` is the one the Details page shows.
pub fn view(editor: &Editor, portrait: &Shown, context: &mut ViewContext<Shell>) -> View {
    // The pages to choose from, once there are several.
    let tabs: View = if Tab::ALL.len() > 1 {
        let items: Vec<KeyedView> = Tab::ALL
            .iter()
            .map(|tab| {
                KeyedView::new(
                    tab.title(),
                    SelectorBarItem::new()
                        .text(tr(tab.title()))
                        .is_selected(*tab == editor.tab),
                )
            })
            .collect();
        SelectorBar::new()
            .margin(Thickness::new(0.0, 0.0, 0.0, 12.0))
            .on_selected_text_changed(context.callback(|text: Option<String>| {
                match Tab::ALL
                    .iter()
                    .find(|t| text.as_deref() == Some(tr(t.title())))
                {
                    Some(tab) => input(EditInput::Tab(*tab)),
                    None => Message::Nothing,
                }
            }))
            .slots([SlotView::collection(SelectorBarSlot::Items, items)])
    } else {
        View::empty()
    };
    let content = match editor.tab {
        Tab::Details => details(editor, portrait, context),
    };
    let problem: View = match editor.error.clone().or_else(|| editor.problem()) {
        Some(problem) => InfoBar::new()
            .severity(if editor.error.is_some() {
                InfoBarSeverity::Error
            } else {
                InfoBarSeverity::Warning
            })
            .title(problem)
            .is_open(true)
            .is_closable(false)
            .into(),
        None => View::empty(),
    };
    let buttons = StackPanel::new()
        .margin(Thickness::new(0.0, 16.0, 0.0, 0.0))
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Right)
        .children((
            Button::new()
                .is_enabled(!editor.saving)
                .on_click(context.message(Message::Back))
                .content(plain(tr("_Cancel"))),
            Button::new()
                .style(ButtonStyle::Accent)
                .is_enabled(editor.can_save())
                .on_click(context.message(input(EditInput::Save)))
                .content(plain(tr("_Save"))),
        ));
    // Each question is a dialog of its own, so that one asked right after
    // another opens; so is each try of Add Experience.
    let serial = editor.prompt_serial();
    let mut dialogs: Vec<KeyedView> = editor
        .prompt()
        .map(|prompt| {
            KeyedView::new(
                format!("prompt{serial}"),
                prompt_dialog(prompt, serial, Message::EditorAnswer, context),
            )
        })
        .into_iter()
        .collect();
    if let Some(form) = &editor.levels {
        dialogs.push(KeyedView::new(
            "levels",
            super::levels::dialog(form, levels, input(EditInput::CloseLevels), context),
        ));
    }
    if let Some(form) = &editor.experience {
        dialogs.push(KeyedView::new(
            format!("experience{}", editor.experience_serial),
            super::levels::experience_dialog(form, experience, context),
        ));
    }
    Grid::new().children((
        ui::page(StackPanel::new().spacing(4.0).children((
            ui::title(tr("Edit Character")).margin(Thickness::new(0.0, 0.0, 0.0, 16.0)),
            tabs,
            content,
            problem,
            buttons,
        ))),
        Grid::new().keyed_children(dialogs),
    ))
}

/// The Details page.
fn details(editor: &Editor, portrait: &Shown, context: &mut ViewContext<Shell>) -> View {
    let d = &editor.details;
    let who = StackPanel::new().spacing(4.0).children((
        ui::setting(
            tr("Name"),
            "",
            text(tr("Name"), &d.name, EditInput::Name, context),
        ),
        ui::setting(
            tr("Player"),
            "",
            text(tr("Player"), &d.player, EditInput::Player, context),
        ),
        ui::setting(
            tr("Social Class"),
            "",
            text(
                tr("Social Class"),
                &d.social_class,
                EditInput::SocialClass,
                context,
            ),
        ),
        super::portrait::row(portrait, context),
        ui::setting(
            tr("Gender"),
            "",
            combo(
                tr("Gender"),
                GENDERS.iter().map(|g| tr(g).to_owned()).collect(),
                Some(usize::from(!d.male)),
                EditInput::Gender,
                context,
            ),
        ),
    ));
    let background_event: View = if editor.backgrounds.is_empty() {
        View::empty()
    } else {
        ui::setting(
            tr("Background Event"),
            "",
            with_buttons(
                combo(
                    tr("Background Event"),
                    editor.backgrounds.iter().map(|b| none_label(b)).collect(),
                    editor.background_index(),
                    EditInput::Background,
                    context,
                ),
                ui::icon_button(
                    tr("Select Randomly"),
                    Symbol::Refresh,
                    input(EditInput::RandomBackground),
                    context,
                ),
            ),
        )
    };
    let background = StackPanel::new().spacing(4.0).children((
        heading(tr("Background"), None),
        ui::setting(
            tr("Alignment"),
            "",
            combo(
                tr("Alignment"),
                editor.alignments.iter().map(|a| tr(a).to_owned()).collect(),
                editor.alignment_index(),
                EditInput::Alignment,
                context,
            ),
        ),
        ui::setting(
            tr("Kit"),
            tr("A new kit rolls the social class on its table"),
            combo(
                tr("Kit"),
                editor.kits.iter().map(|k| none_label(k)).collect(),
                editor.kit_index(),
                EditInput::Kit,
                context,
            ),
        ),
        background_event,
    ));
    let appearance = StackPanel::new().spacing(4.0).children((
        heading(tr("Appearance"), None),
        ui::setting(
            tr("Age"),
            tr("Years"),
            number(tr("Age"), d.age, (0, MOST_AGE), EditInput::Age, context),
        ),
        ui::setting(
            tr("Height"),
            tr("Inches"),
            number(
                tr("Height"),
                d.height,
                (0, MOST_HEIGHT),
                EditInput::Height,
                context,
            ),
        ),
        ui::setting(
            tr("Weight"),
            tr("Pounds"),
            number(
                tr("Weight"),
                d.weight,
                (0, MOST_WEIGHT),
                EditInput::Weight,
                context,
            ),
        ),
    ));
    let history: View = if editor.backgrounds.is_empty() && d.history.is_empty() {
        View::empty()
    } else {
        StackPanel::new().spacing(4.0).children((
            heading(tr("History"), None),
            TextBox::new()
                .automation_name(tr("History"))
                .text(d.history.clone())
                .accepts_return(true)
                .text_wrapping(TextWrapping::Wrap)
                .min_height(96.0)
                .on_text_changed(context.callback(|text: String| input(EditInput::History(text)))),
        ))
    };
    StackPanel::new().spacing(4.0).children((
        who,
        background,
        classes(editor, context),
        experience_group(editor, context),
        appearance,
        abilities(editor, context),
        history,
    ))
}

/// The classes, and Add Class for one more when the race may dual-class.
fn classes(editor: &Editor, context: &mut ViewContext<Shell>) -> View {
    let offer = editor.class_offer();
    let rows: Vec<KeyedView> = editor
        .character()
        .data
        .classes
        .names
        .iter()
        .enumerate()
        .map(|(g, name)| KeyedView::new(format!("{g}\u{1}{name}"), ui::card(ui::body(name))))
        .collect();
    let added: View = match &editor.new_class {
        Some(name) => ui::setting(
            name,
            tr("New, level 1"),
            ui::icon_button(
                tr("Remove"),
                Symbol::Delete,
                input(EditInput::RemoveClass),
                context,
            ),
        ),
        // One class at a time: the next one depends on this one.
        None if !offer.names.is_empty() => ui::setting(
            tr("Add Class"),
            "",
            ComboBox::new()
                .automation_name(tr("Add Class"))
                .placeholder_text(tr("Choose a class"))
                .items_source(offer.names.clone())
                .min_width(220.0)
                .on_selection_changed(context.callback(|i: Option<usize>| match i {
                    Some(i) => input(EditInput::AddClass(i)),
                    None => Message::Nothing,
                })),
        ),
        None => View::empty(),
    };
    StackPanel::new().spacing(4.0).children((
        heading(tr("Classes"), Some(&offer.description)),
        StackPanel::new().spacing(4.0).keyed_children(rows),
        added,
    ))
}

/// Experience, levels, hit points and PSPs, with Levels… and Add….
fn experience_group(editor: &Editor, context: &mut ViewContext<Shell>) -> View {
    let header = Grid::new()
        .margin(Thickness::new(0.0, 16.0, 0.0, 4.0))
        .columns([GridLength::STAR, GridLength::Auto])
        .children((
            ui::strong(tr("Experience")).vertical_alignment(VerticalAlignment::Center),
            StackPanel::new()
                .grid_column(1)
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .children((
                    Button::new()
                        .on_click(context.message(input(EditInput::OpenLevels)))
                        .content(plain(tr("Levels…"))),
                    Button::new()
                        .on_click(context.message(input(EditInput::OpenExperience)))
                        .content(plain(tr("Add…")))
                        .tooltip(tr("Add or take away experience")),
                )),
        ));
    let rows: Vec<KeyedView> = editor
        .class_rows()
        .into_iter()
        .enumerate()
        .map(|(g, class)| {
            KeyedView::new(
                format!("{g}\u{1}{}", class.name),
                ui::setting(&class.name, &class.text, View::empty()),
            )
        })
        .collect();
    let hit_points: View = match editor.hit_points() {
        Some(hp) => ui::setting(
            tr("Hit Points"),
            tr("Changes the rolls of the highest level"),
            number(
                tr("Hit Points"),
                hp,
                (0, MOST_POINTS),
                EditInput::HitPoints,
                context,
            ),
        ),
        None => View::empty(),
    };
    let psp: View = match editor.psp() {
        Some(psp) => ui::setting(
            tr("PSPs"),
            tr("Changes those of the highest level that has them"),
            number(tr("PSPs"), psp, (0, 999), EditInput::Psp, context),
        ),
        None => View::empty(),
    };
    let points: View = match editor.character_points() {
        Some(points) => ui::setting(
            tr("Character Points"),
            tr("Unspent"),
            ui::body(points.to_string()),
        ),
        None => View::empty(),
    };
    StackPanel::new().spacing(4.0).children((
        header,
        StackPanel::new().spacing(4.0).keyed_children(rows),
        hit_points,
        psp,
        points,
    ))
}

/// The ability scores, with Skills & Powers' subabilities, and
/// exceptional strength.
fn abilities(editor: &Editor, context: &mut ViewContext<Shell>) -> View {
    let d = &editor.details;
    let (low, high) = (*ABILITY_RANGE.start(), *ABILITY_RANGE.end());
    let score =
        move |name: &str, value: u32, a: usize, part: usize, context: &mut ViewContext<Shell>| {
            NumberBox::new()
                .automation_name(name)
                .value(f64::from(value))
                .minimum(f64::from(low))
                .maximum(f64::from(high))
                .min_width(100.0)
                .on_value_changed(context.callback(move |v: Option<f64>| match v {
                    Some(v) => input(EditInput::Ability(
                        a,
                        part,
                        (v.round().max(0.0) as u32).clamp(low, high),
                    )),
                    None => Message::Nothing,
                }))
        };
    let rows: Vec<KeyedView> = (0..6)
        .map(|a| {
            let scores = d.abilities[a];
            let control: View = if d.skills_and_powers {
                let labeled = |label: &str, control: NumberBox| {
                    StackPanel::new()
                        .spacing(2.0)
                        .children((ui::caption(label), control))
                };
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(8.0)
                    .children((
                        labeled(
                            tr("Score"),
                            score(tr(ABILITIES[a]), scores[0], a, 0, context),
                        ),
                        labeled(
                            tr(SUBABILITIES[a][0]),
                            score(tr(SUBABILITIES[a][0]), scores[1], a, 1, context),
                        ),
                        labeled(
                            tr(SUBABILITIES[a][1]),
                            score(tr(SUBABILITIES[a][1]), scores[2], a, 2, context),
                        ),
                    ))
            } else {
                score(tr(ABILITIES[a]), scores[0], a, 0, context).into()
            };
            KeyedView::new(a, ui::setting(tr(ABILITIES[a]), "", control))
        })
        .collect();
    StackPanel::new().spacing(4.0).children((
        heading(tr("Abilities"), None),
        StackPanel::new().spacing(4.0).keyed_children(rows),
        ui::setting(
            tr("Exceptional Strength"),
            tr("18/01 to 18/00, 0 for none"),
            NumberBox::new()
                .automation_name(tr("Exceptional Strength"))
                .value(f64::from(d.exceptional_strength))
                .minimum(0.0)
                .maximum(100.0)
                .min_width(120.0)
                .is_enabled(editor.exceptional_allowed())
                .on_value_changed(context.callback(|v: Option<f64>| match v {
                    Some(v) => input(EditInput::Exceptional(v.round().max(0.0) as u32)),
                    None => Message::Nothing,
                })),
        ),
    ))
}
