//! New Character, as the Mac app draws it: the pages and questions come
//! from `cr_app::create`. The prototype draws the first page; the pages
//! after it are drawn as the Mac app gets them.

use std::sync::{Arc, Mutex, MutexGuard};

use cr_app::create::{Answer, Effect, Input, Needs, Page, Wizard};
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::settings::SETTINGS;

use crate::{CoreError, PromptInfo};

/// A campaign setting the first page offers.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SettingInfo {
    pub name: String,
    pub description: String,
}

/// The page in view.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum WizardPage {
    /// The first page: the rules and the campaign setting.
    Setting {
        /// Whether the Core Rules and Player's Option are offered (Core
        /// Rules 2.0 has no Player's Option).
        expansion: bool,
        skills_and_powers: bool,
        /// What the rules chosen bring.
        rules_text: String,
        settings: Vec<SettingInfo>,
        setting: u32,
    },
    /// A page the Mac app does not draw yet, by its title.
    Other { title: String },
}

/// New Character, open in a sheet.
#[derive(uniffi::Object)]
pub struct WizardSession {
    wizard: Mutex<Wizard>,
    /// "Do not show this window again" was ticked and answered.
    core_vs_option_off: Mutex<bool>,
}

impl WizardSession {
    pub(crate) fn open(
        install: &Installation,
        core_vs_option: bool,
    ) -> Result<Arc<Self>, CoreError> {
        let (rules, needs) = Needs::load(install, false, None, core_vs_option)?;
        Ok(Arc::new(Self {
            wizard: Mutex::new(Wizard::new(rules, needs)),
            core_vs_option_off: Mutex::new(false),
        }))
    }

    fn wizard(&self) -> MutexGuard<'_, Wizard> {
        self.wizard.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Takes what the wizard asks of the app.
    fn settle(&self, wizard: &mut Wizard) {
        for effect in wizard.take_effects() {
            match effect {
                Effect::HideCoreVsOption => {
                    *self
                        .core_vs_option_off
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = true;
                }
                // Create is on the last page, which the prototype does
                // not reach.
                Effect::Create(_) => {
                    wizard.failed(tr("The Mac version does not create characters yet").to_owned());
                }
            }
        }
    }

    fn input(&self, input: Input) {
        let mut wizard = self.wizard();
        wizard.input(input);
        self.settle(&mut wizard);
    }
}

#[uniffi::export]
impl WizardSession {
    /// The wizard's title: New Character.
    pub fn title(&self) -> String {
        self.wizard().title().to_owned()
    }

    /// The page in view.
    pub fn page(&self) -> WizardPage {
        let wizard = self.wizard();
        match wizard.page() {
            Page::Setting(page) => WizardPage::Setting {
                expansion: page.expansion,
                skills_and_powers: page.skills_and_powers,
                rules_text: if page.skills_and_powers {
                    tr(
                        "Skills & Powers: subabilities, and character points for racial and \
                        class abilities, proficiencies and traits",
                    )
                } else {
                    tr("Player's Handbook: proficiency slots")
                }
                .to_owned(),
                settings: SETTINGS
                    .iter()
                    .map(|s| SettingInfo {
                        name: tr(s.name).to_owned(),
                        description: tr(s.description).to_owned(),
                    })
                    .collect(),
                setting: page.setting as u32,
            },
            page => WizardPage::Other {
                title: page.title(wizard.entering()),
            },
        }
    }

    /// How many pages in the page in view is, from 1: Back is offered
    /// past the first.
    pub fn depth(&self) -> u32 {
        self.wizard().depth() as u32
    }

    /// Whether Next may be pressed.
    pub fn can_go_on(&self) -> bool {
        let wizard = self.wizard();
        wizard.prompt().is_none() && wizard.page().can_go_on()
    }

    /// The question or message in view.
    pub fn prompt(&self) -> Option<PromptInfo> {
        self.wizard().prompt().map(PromptInfo::of)
    }

    /// Core Rules (0) or Player's Option (1).
    pub fn choose_rules(&self, index: u32) {
        self.input(Input::Rules(index as usize));
    }

    /// The campaign setting, by its place in the list.
    pub fn choose_setting(&self, index: u32) {
        self.input(Input::Setting(index as usize));
    }

    /// "Do not show this window again", on the Core Rules Vs Player's
    /// Option window.
    pub fn prompt_off(&self, off: bool) {
        self.input(Input::PromptOff(off));
    }

    /// Next.
    pub fn next(&self) {
        self.input(Input::Next);
    }

    /// Back.
    pub fn back(&self) {
        self.wizard().back();
    }

    /// Answers the prompt in view: Yes, OK or Close, else No.
    pub fn answer(&self, yes: bool) {
        let mut wizard = self.wizard();
        wizard.answer(if yes { Answer::Yes } else { Answer::No });
        self.settle(&mut wizard);
    }

    /// Whether the Core Rules Vs Player's Option window was turned off, for
    /// the app to keep in its preferences.
    pub fn core_vs_option_off(&self) -> bool {
        *self
            .core_vs_option_off
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }
}
