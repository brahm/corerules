//! Edit Character: the details the GTK app's editor changes that need no
//! other screen (name, player, social class, gender, alignment, age,
//! height, weight and the ability scores), as settings cards.

use std::sync::Arc;

use cr_i18n::{tr, tr_noop};
use cr_store::Installation;
use cr_store::create::{ABILITIES, CreationRules, SUBABILITIES};
use cr_store::edit::{ABILITY_RANGE, CharacterDetails};
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::ui;

/// The character being edited.
#[derive(Clone)]
pub struct Edit {
    pub file: String,
    pub original: CharacterDetails,
    pub details: CharacterDetails,
    /// The alignments its classes allow, its own first if they do not.
    pub alignments: Vec<String>,
    classes: Vec<String>,
    rules: Arc<CreationRules>,
    pub saving: bool,
}

/// A change to one field of the form.
#[derive(Clone)]
pub enum Field {
    Name(String),
    Player(String),
    SocialClass(String),
    Gender(Option<usize>),
    Alignment(Option<usize>),
    Age(Option<f64>),
    Height(Option<f64>),
    Weight(Option<f64>),
    /// An ability (0 to 5) and which of its score and subabilities.
    Ability(usize, usize, Option<f64>),
    Exceptional(Option<f64>),
}

const GENDERS: [&str; 2] = [tr_noop!("Male"), tr_noop!("Female")];

impl Edit {
    /// Reads the character of `Chars/<file>` and the rules its form needs.
    pub fn load(install: &Installation, file: &str) -> Result<Self, String> {
        let character = install.character(file).map_err(|e| e.to_string())?;
        let rules = install.creation_rules().map_err(|e| e.to_string())?;
        let details = CharacterDetails::of(&character);
        let classes = character.data.classes.names.clone();
        let mut alignments = rules.alignments(&classes);
        if !alignments.contains(&details.alignment) {
            alignments.insert(0, details.alignment.clone());
        }
        Ok(Self {
            file: file.to_owned(),
            original: details.clone(),
            details,
            alignments,
            classes,
            rules: Arc::new(rules),
            saving: false,
        })
    }

    pub fn changed(&self) -> bool {
        self.details != self.original
    }

    /// Why the details cannot be saved, if they cannot.
    pub fn problem(&self) -> Option<String> {
        self.details.problem()
    }

    /// Whether the character may have exceptional strength: a warrior with
    /// Strength 18.
    fn exceptional_allowed(&self) -> bool {
        self.rules
            .has_exceptional_strength(&self.classes, self.details.abilities[0][0])
    }

    pub fn set(&mut self, field: Field) {
        let whole =
            |value: Option<f64>, max: u32| value.map(|v| (v.round().max(0.0) as u32).min(max));
        let d = &mut self.details;
        match field {
            Field::Name(text) => d.name = text,
            Field::Player(text) => d.player = text,
            Field::SocialClass(text) => d.social_class = text,
            Field::Gender(Some(i)) => d.male = i == 0,
            Field::Alignment(Some(i)) => {
                if let Some(a) = self.alignments.get(i) {
                    d.alignment.clone_from(a);
                }
            }
            Field::Gender(None) | Field::Alignment(None) => {}
            Field::Age(v) => d.age = whole(v, 9999).unwrap_or(d.age),
            Field::Height(v) => d.height = whole(v, 999).unwrap_or(d.height),
            Field::Weight(v) => d.weight = whole(v, 99999).unwrap_or(d.weight),
            Field::Ability(ability, part, v) => {
                if let Some(v) = whole(v, *ABILITY_RANGE.end()) {
                    let v = v.max(*ABILITY_RANGE.start());
                    // With the Core Rules the score sets both subabilities,
                    // as the original keeps them.
                    if d.skills_and_powers {
                        d.abilities[ability][part] = v;
                    } else {
                        d.abilities[ability] = [v; 3];
                    }
                }
            }
            Field::Exceptional(v) => d.exceptional_strength = whole(v, 100).unwrap_or(0),
        }
        if !self.exceptional_allowed() {
            self.details.exceptional_strength = 0;
        }
    }
}

pub fn view(edit: &Edit, context: &mut ViewContext<Shell>) -> View {
    let d = &edit.details;
    let field =
        |make: fn(String) -> Field| context.callback(move |text: String| Message::Edit(make(text)));
    let text = |value: &str, make: fn(String) -> Field| {
        TextBox::new()
            .text(value)
            .min_width(260.0)
            .on_text_changed(field(make))
    };
    let number = |value: u32, min: f64, max: f64, make: fn(Option<f64>) -> Field| {
        NumberBox::new()
            .value(f64::from(value))
            .minimum(min)
            .maximum(max)
            .min_width(120.0)
            .on_value_changed(context.callback(move |v| Message::Edit(make(v))))
    };
    let abilities = (0..6).map(|a| {
        let scores = d.abilities[a];
        let score = NumberBox::new()
            .value(f64::from(scores[0]))
            .minimum(f64::from(*ABILITY_RANGE.start()))
            .maximum(f64::from(*ABILITY_RANGE.end()))
            .min_width(120.0)
            .on_value_changed(context.callback(move |v| Message::Edit(Field::Ability(a, 0, v))));
        let control: View = if d.skills_and_powers {
            let sub = |part: usize| {
                StackPanel::new().spacing(2.0).children((
                    ui::caption(tr(SUBABILITIES[a][part - 1])),
                    NumberBox::new()
                        .value(f64::from(scores[part]))
                        .minimum(f64::from(*ABILITY_RANGE.start()))
                        .maximum(f64::from(*ABILITY_RANGE.end()))
                        .min_width(100.0)
                        .on_value_changed(
                            context.callback(move |v| Message::Edit(Field::Ability(a, part, v))),
                        ),
                ))
            };
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .children((
                    StackPanel::new()
                        .spacing(2.0)
                        .children((ui::caption(tr("Score")), score)),
                    sub(1),
                    sub(2),
                ))
        } else {
            score.into()
        };
        KeyedView::new(a, ui::setting(tr(ABILITIES[a]), "", control))
    });
    let problem: View = match edit.problem() {
        Some(problem) => InfoBar::new()
            .severity(InfoBarSeverity::Warning)
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
                .style(ButtonStyle::Accent)
                .is_enabled(edit.problem().is_none() && edit.changed() && !edit.saving)
                .on_click(context.message(Message::Save))
                .content(plain(tr("_Save"))),
            Button::new()
                .is_enabled(!edit.saving)
                .on_click(context.message(Message::Back))
                .content(plain(tr("_Cancel"))),
        ));
    ui::page(
        StackPanel::new().spacing(4.0).children((
            ui::title(tr("Edit Character")).margin(Thickness::new(0.0, 0.0, 0.0, 16.0)),
            ui::setting(tr("Name"), "", text(&d.name, Field::Name)),
            ui::setting(tr("Player"), "", text(&d.player, Field::Player)),
            ui::setting(
                tr("Social Class"),
                "",
                text(&d.social_class, Field::SocialClass),
            ),
            ui::setting(
                tr("Gender"),
                "",
                ComboBox::new()
                    .items_source(GENDERS.iter().map(|g| tr(g).to_owned()))
                    .selected_index(usize::from(!d.male))
                    .min_width(200.0)
                    .on_selection_changed(context.callback(|i| Message::Edit(Field::Gender(i)))),
            ),
            ui::setting(
                tr("Alignment"),
                "",
                ComboBox::new()
                    .items_source(edit.alignments.clone())
                    .selected_index(edit.alignments.iter().position(|a| *a == d.alignment))
                    .min_width(200.0)
                    .on_selection_changed(context.callback(|i| Message::Edit(Field::Alignment(i)))),
            ),
            ui::setting(
                tr("Age"),
                tr("Years"),
                number(d.age, 0.0, 9999.0, Field::Age),
            ),
            ui::setting(
                tr("Height"),
                tr("Inches"),
                number(d.height, 0.0, 999.0, Field::Height),
            ),
            ui::setting(
                tr("Weight"),
                tr("Pounds"),
                number(d.weight, 0.0, 99999.0, Field::Weight),
            ),
            ui::strong(tr("Abilities")).margin(Thickness::new(0.0, 20.0, 0.0, 4.0)),
            StackPanel::new().spacing(4.0).keyed_children(abilities),
            ui::setting(
                tr("Exceptional Strength"),
                tr("18/01 to 18/00, 0 for none"),
                NumberBox::new()
                    .value(f64::from(d.exceptional_strength))
                    .minimum(0.0)
                    .maximum(100.0)
                    .min_width(120.0)
                    .is_enabled(edit.exceptional_allowed())
                    .on_value_changed(context.callback(|v| Message::Edit(Field::Exceptional(v)))),
            ),
            problem,
            buttons,
        )),
    )
}
