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
    /// A prompt the Mac app does not show yet.
    Other,
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
            prompt::PromptKind::CoreVsOption { off } => PromptKind::CoreVsOption { off: *off },
            _ => PromptKind::Other,
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
