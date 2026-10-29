//! A page of a form as the Mac app draws it: groups of rows (choices,
//! numbers, text, check boxes, lists with Add and Remove), each control
//! with the number of what it does. The pages of New Character, and of
//! the other forms as the Mac app gets them, are described here from
//! `cr-app`'s pages, so that the Swift side draws any of them with one
//! view and keeps no rule; what a control does is kept on this side, by
//! its number, until the form is described again.

use cr_app::create::{Choice, List, ListKind, MOST_ITEMS, Unarmed};
use cr_i18n::tr;
use cr_store::create::UNARMED;

/// A button beside a row's control or a group's heading.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FormButton {
    pub id: u32,
    pub label: String,
    /// The SF Symbol it shows, or empty for its label only.
    pub icon: String,
    pub enabled: bool,
}

/// A choice of a radio group, with a line under it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FormOption {
    pub title: String,
    pub subtitle: String,
}

/// A whole number beside a list's row: an item's quantity or a
/// proficiency's rank.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FormNumber {
    pub id: u32,
    pub label: String,
    pub value: i64,
    pub min: i64,
    pub max: i64,
}

/// A row of a group.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum FormRow {
    /// A line of text; `secondary` for one that says there is nothing.
    Note { text: String, secondary: bool },
    /// A label and a value that is only shown.
    Value {
        title: String,
        subtitle: String,
        value: String,
        buttons: Vec<FormButton>,
    },
    /// A choice of one, in a pop-up menu.
    Choice {
        id: u32,
        title: String,
        subtitle: String,
        options: Vec<String>,
        selected: Option<u32>,
        buttons: Vec<FormButton>,
    },
    /// A choice of one, as radio buttons each with its line.
    Radio {
        id: u32,
        options: Vec<FormOption>,
        selected: Option<u32>,
    },
    Number {
        id: u32,
        title: String,
        subtitle: String,
        value: i64,
        min: i64,
        max: i64,
        enabled: bool,
        buttons: Vec<FormButton>,
    },
    Text {
        id: u32,
        title: String,
        value: String,
        multiline: bool,
        buttons: Vec<FormButton>,
    },
    /// A switch.
    Toggle {
        id: u32,
        title: String,
        subtitle: String,
        on: bool,
        enabled: bool,
    },
    /// A check box of a list of them.
    Check {
        id: u32,
        title: String,
        subtitle: String,
        on: bool,
        enabled: bool,
    },
    /// A button on a row of its own, with what it does beside.
    Button {
        title: String,
        subtitle: String,
        button: FormButton,
    },
    /// A row of a list, with its number and Remove when it has them.
    Item {
        title: String,
        subtitle: String,
        number: Option<FormNumber>,
        remove: Option<u32>,
    },
    /// Add on a list: what may be added, each chosen adds itself.
    Add {
        id: u32,
        label: String,
        options: Vec<String>,
    },
    /// The character's portrait, with Choose; `race` is the race whose
    /// portraits the chooser shows first.
    Portrait {
        id: u32,
        path: Option<String>,
        subtitle: String,
        race: String,
    },
    /// A warning or an error over the form.
    Banner { text: String, error: bool },
}

/// A group of rows under a heading.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FormGroup {
    pub title: String,
    pub caption: String,
    pub buttons: Vec<FormButton>,
    pub rows: Vec<FormRow>,
}

/// A dialog over the form, as Enter Character's Levels: its groups and
/// the control that closes it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FormDialog {
    pub title: String,
    pub groups: Vec<FormGroup>,
    pub close: FormButton,
}

/// A form's page: its groups, and the dialog over it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FormPage {
    pub groups: Vec<FormGroup>,
    pub dialog: Option<FormDialog>,
}

/// What a control was changed to.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum FormValue {
    /// A button pressed.
    Press,
    /// The choice of a menu or radio group, or of Add, by its place.
    Index {
        index: u32,
    },
    Number {
        value: i64,
    },
    Text {
        text: String,
    },
    On {
        on: bool,
    },
}

impl FormValue {
    fn index(&self) -> Option<usize> {
        match self {
            Self::Index { index } => Some(*index as usize),
            _ => None,
        }
    }

    fn number(&self) -> Option<i64> {
        match self {
            Self::Number { value } => Some(*value),
            _ => None,
        }
    }

    fn text(&self) -> Option<String> {
        match self {
            Self::Text { text } => Some(text.clone()),
            _ => None,
        }
    }

    fn on(&self) -> Option<bool> {
        match self {
            Self::On { on } => Some(*on),
            _ => None,
        }
    }
}

/// What a control does with its value: the input of the form, if any.
type Act<I> = Box<dyn Fn(&FormValue) -> Option<I> + Send + Sync>;

/// What a form's controls do, by their numbers, kept until the form is
/// described again.
pub struct Acts<I> {
    acts: Vec<Act<I>>,
}

impl<I> Default for Acts<I> {
    fn default() -> Self {
        Self { acts: Vec::new() }
    }
}

impl<I> Acts<I> {
    /// The input of control `id` changed to `value`.
    pub fn input(&self, id: u32, value: &FormValue) -> Option<I> {
        self.acts.get(id as usize).and_then(|act| act(value))
    }
}

/// Describes a form: its groups as they are added, and what each control
/// does.
pub struct Builder<I> {
    groups: Vec<FormGroup>,
    acts: Vec<Act<I>>,
}

impl<I: 'static> Default for Builder<I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: 'static> Builder<I> {
    pub fn new() -> Self {
        Self {
            groups: Vec::new(),
            acts: Vec::new(),
        }
    }

    /// The number of a control that does `act`.
    fn act(&mut self, act: impl Fn(&FormValue) -> Option<I> + Send + Sync + 'static) -> u32 {
        self.acts.push(Box::new(act));
        (self.acts.len() - 1) as u32
    }

    /// A control whose choice by index makes `make`'s input.
    pub fn on_index(&mut self, make: impl Fn(usize) -> I + Send + Sync + 'static) -> u32 {
        self.act(move |v| v.index().map(&make))
    }

    /// A control whose whole number makes `make`'s input, kept within
    /// `min..=max`.
    pub fn on_number(
        &mut self,
        (min, max): (i64, i64),
        make: impl Fn(i64) -> I + Send + Sync + 'static,
    ) -> u32 {
        self.act(move |v| v.number().map(|n| make(n.clamp(min, max))))
    }

    pub fn on_text(&mut self, make: impl Fn(String) -> I + Send + Sync + 'static) -> u32 {
        self.act(move |v| v.text().map(&make))
    }

    pub fn on_toggle(&mut self, make: impl Fn(bool) -> I + Send + Sync + 'static) -> u32 {
        self.act(move |v| v.on().map(&make))
    }

    /// A button that makes `input`.
    pub fn on_press(&mut self, input: I) -> u32
    where
        I: Clone + Send + Sync,
    {
        self.act(move |v| matches!(v, FormValue::Press).then(|| input.clone()))
    }

    /// A button labelled `label`, with the SF Symbol `icon`.
    pub fn button(&mut self, label: &str, icon: &str, enabled: bool, input: I) -> FormButton
    where
        I: Clone + Send + Sync,
    {
        FormButton {
            id: self.on_press(input),
            label: label.to_owned(),
            icon: icon.to_owned(),
            enabled,
        }
    }

    /// A Roll Again button with the dice.
    pub fn roll(&mut self, label: &str, input: I) -> FormButton
    where
        I: Clone + Send + Sync,
    {
        self.button(label, "dice", true, input)
    }

    /// Starts a group under `title`, with `caption` under it.
    pub fn group(&mut self, title: &str, caption: &str) -> &mut Self {
        self.groups.push(FormGroup {
            title: title.to_owned(),
            caption: caption.to_owned(),
            buttons: Vec::new(),
            rows: Vec::new(),
        });
        self
    }

    /// Gives the group being described a button beside its heading.
    pub fn heading_button(&mut self, button: FormButton) -> &mut Self {
        if self.groups.is_empty() {
            self.group("", "");
        }
        if let Some(group) = self.groups.last_mut() {
            group.buttons.push(button);
        }
        self
    }

    /// Adds `row` to the group being described.
    pub fn row(&mut self, row: FormRow) -> &mut Self {
        if self.groups.is_empty() {
            self.group("", "");
        }
        if let Some(group) = self.groups.last_mut() {
            group.rows.push(row);
        }
        self
    }

    pub fn note(&mut self, text: &str) -> &mut Self {
        self.row(FormRow::Note {
            text: text.to_owned(),
            secondary: false,
        })
    }

    /// "None", for a group with nothing in it.
    pub fn none(&mut self) -> &mut Self {
        self.row(FormRow::Note {
            text: tr("None").to_owned(),
            secondary: true,
        })
    }

    pub fn value(&mut self, title: &str, subtitle: &str, value: &str) -> &mut Self {
        self.row(FormRow::Value {
            title: title.to_owned(),
            subtitle: subtitle.to_owned(),
            value: value.to_owned(),
            buttons: Vec::new(),
        })
    }

    /// A pop-up menu of `options`; `make` makes the input of the one
    /// chosen.
    pub fn choice(
        &mut self,
        (title, subtitle): (&str, &str),
        options: Vec<String>,
        selected: Option<usize>,
        buttons: Vec<FormButton>,
        make: impl Fn(usize) -> I + Send + Sync + 'static,
    ) -> &mut Self {
        let id = self.on_index(make);
        self.row(FormRow::Choice {
            id,
            title: title.to_owned(),
            subtitle: subtitle.to_owned(),
            options,
            selected: selected.map(|s| s as u32),
            buttons,
        })
    }

    /// Radio buttons, each with a line under it.
    pub fn radio(
        &mut self,
        options: &[(String, String)],
        selected: Option<usize>,
        make: impl Fn(usize) -> I + Send + Sync + 'static,
    ) -> &mut Self {
        let id = self.on_index(make);
        self.row(FormRow::Radio {
            id,
            options: options
                .iter()
                .map(|(title, subtitle)| FormOption {
                    title: title.clone(),
                    subtitle: subtitle.clone(),
                })
                .collect(),
            selected: selected.map(|s| s as u32),
        })
    }

    /// A whole number in `min..=max`.
    pub fn number(
        &mut self,
        (title, subtitle): (&str, &str),
        value: i64,
        range: (i64, i64),
        buttons: Vec<FormButton>,
        make: impl Fn(i64) -> I + Send + Sync + 'static,
    ) -> &mut Self {
        let id = self.on_number(range, make);
        self.row(FormRow::Number {
            id,
            title: title.to_owned(),
            subtitle: subtitle.to_owned(),
            value,
            min: range.0,
            max: range.1,
            enabled: true,
            buttons,
        })
    }

    pub fn text(
        &mut self,
        title: &str,
        value: &str,
        multiline: bool,
        buttons: Vec<FormButton>,
        make: impl Fn(String) -> I + Send + Sync + 'static,
    ) -> &mut Self {
        let id = self.on_text(make);
        self.row(FormRow::Text {
            id,
            title: title.to_owned(),
            value: value.to_owned(),
            multiline,
            buttons,
        })
    }

    pub fn toggle(
        &mut self,
        (title, subtitle): (&str, &str),
        on: bool,
        make: impl Fn(bool) -> I + Send + Sync + 'static,
    ) -> &mut Self {
        let id = self.on_toggle(make);
        self.row(FormRow::Toggle {
            id,
            title: title.to_owned(),
            subtitle: subtitle.to_owned(),
            on,
            enabled: true,
        })
    }

    /// Check boxes: (title, line under it, ticked, may change); `make`
    /// makes the input of one ticked or not.
    pub fn checks(
        &mut self,
        rows: &[(String, String, bool, bool)],
        make: impl Fn(usize, bool) -> I + Clone + Send + Sync + 'static,
    ) -> &mut Self {
        for (i, (title, subtitle, on, enabled)) in rows.iter().enumerate() {
            let make = make.clone();
            let id = self.on_toggle(move |on| make(i, on));
            self.row(FormRow::Check {
                id,
                title: title.clone(),
                subtitle: subtitle.clone(),
                on: *on,
                enabled: *enabled,
            });
        }
        self
    }

    /// A row with a button, `subtitle` saying what it does.
    pub fn button_row(&mut self, title: &str, subtitle: &str, button: FormButton) -> &mut Self {
        self.row(FormRow::Button {
            title: title.to_owned(),
            subtitle: subtitle.to_owned(),
            button,
        })
    }

    pub fn banner(&mut self, text: &str, error: bool) -> &mut Self {
        self.row(FormRow::Banner {
            text: text.to_owned(),
            error,
        })
    }

    /// The form's groups and what its controls do; `dialog` is the one
    /// over it, if any, its controls among these.
    pub fn finish(self) -> (Vec<FormGroup>, Acts<I>) {
        (self.groups, Acts { acts: self.acts })
    }

    /// The groups described so far, taken out, for a dialog whose
    /// controls are numbered with the form's.
    pub fn take_groups(&mut self) -> Vec<FormGroup> {
        std::mem::take(&mut self.groups)
    }

    /// Puts back groups taken out with [`Self::take_groups`], before
    /// those described since.
    pub fn put_groups(&mut self, mut groups: Vec<FormGroup>) {
        groups.append(&mut self.groups);
        self.groups = groups;
    }
}

/// What a form's lists do: a name added, a row removed, a row's number
/// changed.
pub struct ListInputs<I> {
    pub add: fn(ListKind, String) -> I,
    pub remove: fn(ListKind, usize) -> I,
    pub times: fn(ListKind, usize, u32) -> I,
}

// Function pointers copy whatever the input is.
impl<I> Clone for ListInputs<I> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I> Copy for ListInputs<I> {}

impl<I: Clone + Send + Sync + 'static> Builder<I> {
    /// A list the user adds to and removes from, as a group of its own;
    /// each of Add's choices has its line of `details`, by its index, if
    /// any.
    pub fn list(
        &mut self,
        kind: ListKind,
        list: &List,
        details: &[String],
        inputs: ListInputs<I>,
    ) -> &mut Self {
        if !list.shown {
            return self;
        }
        self.group(&list.title, &list.description);
        let (label, least, most) = if kind == ListKind::Items {
            (tr("Quantity"), 1, MOST_ITEMS)
        } else {
            (tr("Rank"), 1, 10)
        };
        if list.rows.is_empty() {
            self.none();
        }
        for (i, (title, subtitle, count)) in list.rows.iter().enumerate() {
            let number = count.map(|n| FormNumber {
                id: self.on_number((least.into(), most.into()), move |v| {
                    (inputs.times)(kind, i, v as u32)
                }),
                label: label.to_owned(),
                value: n.into(),
                min: least.into(),
                max: most.into(),
            });
            let remove = (!list.fixed).then(|| self.on_press((inputs.remove)(kind, i)));
            self.row(FormRow::Item {
                title: title.clone(),
                subtitle: subtitle.clone(),
                number,
                remove,
            });
        }
        // Rows only rolled have nothing to add.
        if !list.fixed {
            let choices = list.choices.clone();
            let id = self.act(move |v| {
                let name = choices.get(v.index()?)?;
                Some((inputs.add)(kind, name.clone()))
            });
            let options = list
                .choices
                .iter()
                .enumerate()
                .map(|(i, choice)| match details.get(i) {
                    Some(detail) if !detail.is_empty() => format!("{choice} · {detail}"),
                    _ => choice.clone(),
                })
                .collect();
            self.row(FormRow::Add {
                id,
                label: tr!("Add to {list}", list = list.title.clone()),
                options,
            });
        }
        self
    }

    /// A choice of one, "None" first, as a group named after it.
    pub fn picked(
        &mut self,
        choice: &Choice,
        label: &str,
        make: impl Fn(usize) -> I + Send + Sync + 'static,
    ) -> &mut Self {
        if !choice.shown {
            return self;
        }
        self.group(&choice.title, &choice.description).choice(
            (label, ""),
            choice.items.clone(),
            Some(choice.selected),
            Vec::new(),
            make,
        )
    }

    /// The unarmed styles to tick, each with what it costs.
    pub fn unarmed(
        &mut self,
        unarmed: &Unarmed,
        make: impl Fn(usize, bool) -> I + Clone + Send + Sync + 'static,
    ) -> &mut Self {
        if !unarmed.shown {
            return self;
        }
        let rows: Vec<(String, String, bool, bool)> = UNARMED
            .iter()
            .zip(unarmed.taken)
            .map(|(name, on)| ((*name).to_owned(), unarmed.cost.clone(), on, true))
            .collect();
        self.group(
            tr("Unarmed Combat"),
            tr("Specialization in an unarmed style, not together with a weapon specialization"),
        )
        .checks(&rows, make)
    }

    /// Skills & Powers: the character points left, kept for later levels.
    pub fn points_left(&mut self, points: &str, problem: Option<&str>) -> &mut Self {
        self.group(tr("Character Points Left"), problem.unwrap_or_default())
            .value(
                tr("Character Points Left"),
                tr("Kept on the character for later levels"),
                points,
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_make_their_inputs_by_number() {
        let mut form: Builder<String> = Builder::new();
        form.group("Group", "").choice(
            ("Choice", ""),
            vec!["a".into(), "b".into()],
            Some(0),
            Vec::new(),
            |i| format!("choice {i}"),
        );
        let roll = form.roll("Roll", "rolled".to_owned());
        form.number(("Number", ""), 3, (1, 5), vec![roll], |n| {
            format!("number {n}")
        });
        let (groups, acts) = form.finish();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].rows.len(), 2);
        let FormRow::Number { id, buttons, .. } = &groups[0].rows[1] else {
            panic!("not a number");
        };
        assert_eq!(
            acts.input(*id, &FormValue::Number { value: 9 }),
            Some("number 5".to_owned())
        );
        assert_eq!(acts.input(*id, &FormValue::Press), None);
        assert_eq!(
            acts.input(buttons[0].id, &FormValue::Press),
            Some("rolled".to_owned())
        );
        assert_eq!(
            acts.input(0, &FormValue::Index { index: 1 }),
            Some("choice 1".to_owned())
        );
    }
}
