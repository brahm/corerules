//! The questions, warnings and messages a form waits on, as the Mac app
//! shows them in an alert.

use cr_app::prompt::{self, Prompt};

/// How a prompt is answered.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum PromptKind {
    /// OK only.
    Message,
    /// Yes and No.
    Question { yes_default: bool },
    /// The Expansion's Core Rules Vs Player's Option window, with "Do not
    /// show this window again" ticked or not.
    CoreVsOption { off: bool },
    /// "Add one point to any ability score": one button per ability, the
    /// score beside it, and no Cancel.
    RaiseOne { labels: Vec<String> },
    /// Enter Character's All Classes…: one class of each group, or none.
    AllClasses { groups: Vec<PromptGroup> },
}

/// A group of All Classes…: its classes, "None" first, and the one
/// chosen.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PromptGroup {
    pub name: String,
    pub classes: Vec<String>,
    pub chosen: u32,
}

/// A question, warning or message in view.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PromptInfo {
    pub kind: PromptKind,
    pub title: String,
    pub text: String,
    pub yes_label: String,
    pub no_label: Option<String>,
}

impl PromptInfo {
    pub(crate) fn of<T>(p: &Prompt<T>) -> Self {
        let kind = match &p.kind {
            prompt::PromptKind::Message => PromptKind::Message,
            prompt::PromptKind::Question { yes_default } => PromptKind::Question {
                yes_default: *yes_default,
            },
            // OK refuses and Ignore allows, or No leaves the item out:
            // the labels say which.
            prompt::PromptKind::Ignorable | prompt::PromptKind::ItemError => {
                PromptKind::Question { yes_default: false }
            }
            prompt::PromptKind::CoreVsOption { off } => PromptKind::CoreVsOption { off: *off },
            prompt::PromptKind::RaiseOne { labels } => PromptKind::RaiseOne {
                labels: labels.to_vec(),
            },
            prompt::PromptKind::AllClasses { groups, chosen } => PromptKind::AllClasses {
                groups: groups
                    .iter()
                    .zip(chosen)
                    .map(|((name, classes), &chosen)| PromptGroup {
                        name: cr_i18n::tr(name).to_owned(),
                        classes: std::iter::once(cr_i18n::tr("None").to_owned())
                            .chain(classes.iter().cloned())
                            .collect(),
                        chosen: chosen as u32,
                    })
                    .collect(),
            },
        };
        Self {
            kind,
            title: p.title.clone(),
            text: p.text.clone(),
            yes_label: p.yes_label().to_owned(),
            no_label: p.no_label().map(str::to_owned),
        }
    }
}
