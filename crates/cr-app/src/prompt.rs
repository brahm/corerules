//! The questions, warnings and messages a form waits on, as the original's
//! message boxes and small dialogs: New Character's wizard and the
//! character editor alike. `T` is the form an answer acts on.

use std::sync::Arc;

use cr_i18n::tr;

/// What answering a prompt does to the form.
pub type Then<T> = Arc<dyn Fn(&mut T) + Send + Sync>;

/// A question, warning or message the form waits on.
pub struct Prompt<T> {
    pub kind: PromptKind,
    pub title: String,
    pub text: String,
    pub(crate) yes: Option<Then<T>>,
    pub(crate) no: Option<Then<T>>,
}

impl<T> Clone for Prompt<T> {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind.clone(),
            title: self.title.clone(),
            text: self.text.clone(),
            yes: self.yes.clone(),
            no: self.no.clone(),
        }
    }
}

/// How a prompt is answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    /// OK only; what follows runs once it closes.
    Message,
    /// Yes and No; No, or Yes, the default.
    Question { yes_default: bool },
    /// The original's IDD 245: OK refuses what was asked, Ignore allows
    /// it for good.
    Ignorable,
    /// The original's "Error Message" about an item the character may not
    /// use (0x5e43f9): Yes ignores it, No (the default) leaves it out.
    ItemError,
    /// "Add one point to any ability score" (IDD 393): one choice per
    /// ability, the score beside it, and no Cancel.
    RaiseOne { labels: [String; 6] },
    /// The Expansion's "Core Rules Vs Player's Option" window (IDD 403),
    /// with "Do not show this window again".
    CoreVsOption { off: bool },
    /// Enter Character's "All Classes..." (0x5b52d9): one class of each
    /// group, or none, by index (0 for none).
    AllClasses {
        groups: Vec<(&'static str, Vec<String>)>,
        chosen: Vec<usize>,
    },
}

/// The answer to a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Yes, OK, Ignore or Close: the button that goes on.
    Yes,
    /// No, Cancel, or OK on a warning: the one that does not.
    No,
    /// One of a choice's options, by index.
    Choice(usize),
}

impl<T> Prompt<T> {
    pub(crate) fn new(kind: PromptKind, title: &str, text: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.to_owned(),
            text: text.into(),
            yes: None,
            no: None,
        }
    }

    /// A message with OK.
    pub(crate) fn message(title: &str, text: impl Into<String>) -> Self {
        Self::new(PromptKind::Message, title, text)
    }

    /// A message with OK that runs `then` once closed.
    pub(crate) fn notice(title: &str, text: impl Into<String>, then: Then<T>) -> Self {
        Self {
            yes: Some(then),
            ..Self::message(title, text)
        }
    }

    /// A Yes/No question, No the default as in the original.
    pub(crate) fn ask(
        title: &str,
        text: impl Into<String>,
        yes: Then<T>,
        no: Option<Then<T>>,
    ) -> Self {
        Self {
            yes: Some(yes),
            no,
            ..Self::new(PromptKind::Question { yes_default: false }, title, text)
        }
    }

    /// The original's "Error Message" about an item the character may not
    /// use: Yes runs `ignore`, No leaves the item out.
    pub(crate) fn item_error(text: &str, ignore: Then<T>) -> Self {
        let text = format!("{text}\n\n{}", tr("Do you wish to ignore this message?"));
        Self {
            yes: Some(ignore),
            ..Self::new(PromptKind::ItemError, tr("Error Message"), text)
        }
    }

    /// A warning the user may ignore: Ignore runs `ignore`, OK nothing.
    pub(crate) fn ignorable(title: &str, text: &str, ignore: Then<T>) -> Self {
        let text = format!(
            "{text}\n\n{}",
            tr(
                "If you choose to ignore this message and save this character, this message will \
                not appear for this character again."
            )
        );
        Self {
            yes: Some(ignore),
            ..Self::new(PromptKind::Ignorable, title, text)
        }
    }

    /// What answering with the first button does: Yes, OK or Ignore.
    pub fn yes_label(&self) -> &'static str {
        match self.kind {
            PromptKind::Message => tr("_OK"),
            PromptKind::Question { .. } | PromptKind::ItemError => tr("_Yes"),
            PromptKind::Ignorable => tr("_Ignore"),
            PromptKind::RaiseOne { .. } => "",
            PromptKind::CoreVsOption { .. } | PromptKind::AllClasses { .. } => tr("_Close"),
        }
    }

    /// The other button's label, if there is one.
    pub fn no_label(&self) -> Option<&'static str> {
        match self.kind {
            PromptKind::Question { .. } | PromptKind::ItemError => Some(tr("_No")),
            PromptKind::Ignorable => Some(tr("_OK")),
            _ => None,
        }
    }

    /// What the answer runs: Yes or OK the first, No the other; a
    /// message's OK goes on however it is closed.
    pub(crate) fn then(&self, answer: Answer) -> Option<Then<T>> {
        match (&self.kind, answer) {
            (PromptKind::Message, _) | (_, Answer::Yes) => self.yes.clone(),
            _ => self.no.clone(),
        }
    }
}
