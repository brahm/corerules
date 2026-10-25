//! The details page: name, player, gender, portrait, alignment, kit,
//! social class, background, age, height, weight, hit points, exceptional
//! Strength and, with Skills & Powers, the subabilities. Enter Character's
//! has the classes' levels instead of the hit point rolls, with the Levels
//! dialog and the hit point and PSP arrows that stand in for the rolls.

use std::sync::Arc;

use cr_formats::character::Character;
use cr_i18n::{tr, tr_noop};
use cr_store::advance::{AdvanceRules, adjust_psionic_points, psionic_total};
use cr_store::create::{
    ABILITIES, ALIGNMENTS, CreationRules, Draft, RollMethod, SUBABILITIES, SUBABILITY_SPREAD,
};

use super::{Input, Page, Prompt, Wizard};
use crate::levels::{ClassRow, LevelsForm, class_rows, psionic};

/// The social classes, in the original's order.
pub const SOCIAL_CLASSES: [&str; 4] = [
    tr_noop!("Lower class"),
    tr_noop!("Lower middle class"),
    tr_noop!("Upper middle class"),
    tr_noop!("Upper class"),
];

/// "Fighter 7+1 (d10+1) · Constitution adjusts it on the sheet": each
/// class's hit die roll, with the kit's hit points on each (0x42d8f7).
pub fn hit_dice_text(
    rules: &CreationRules,
    classes: &[String],
    rolls: &[u32],
    kit: &str,
) -> String {
    let bonus = match rules.kit_hit_points(kit) {
        0 => String::new(),
        b => format!("{b:+}"),
    };
    let text: Vec<String> = classes
        .iter()
        .zip(rolls)
        .map(|(c, roll)| {
            let die = rules.class(c).map_or(0, |c| c.hit_die);
            format!("{c} {roll}{bonus} (d{die}{bonus})")
        })
        .collect();
    tr!(
        "{rolls} · Constitution adjusts it on the sheet",
        rolls = text.join(", ")
    )
}

/// Each ability's row with Skills & Powers' subabilities spread as
/// `spreads`: "Strength 14", and "Stamina 15 · Muscle 13" under it.
pub fn subability_rows(
    rules: &CreationRules,
    draft: &Draft,
    spreads: [i32; 6],
) -> [(String, String); 6] {
    let mut probe = draft.clone();
    probe.subabilities = spreads;
    let scores = rules.subability_scores(&probe);
    std::array::from_fn(|a| {
        (
            format!("{} {}", tr(ABILITIES[a]), scores[a][0]),
            format!(
                "{} {} · {} {}",
                tr(SUBABILITIES[a][0]),
                scores[a][1],
                tr(SUBABILITIES[a][1]),
                scores[a][2]
            ),
        )
    })
}

/// What the details page holds until Next.
#[derive(Debug, Clone, PartialEq)]
pub struct DetailsPage {
    /// "Human Fighter".
    pub heading: String,
    pub name: String,
    pub player: String,
    pub male: bool,
    /// Whether Random Name is offered (the program could be read).
    pub random_names: bool,
    /// The portrait's folder and file name, empty for none.
    pub portrait: [String; 2],
    pub alignments: Vec<String>,
    pub alignment: String,
    /// The kits offered, the one chosen listed.
    pub kits: Vec<String>,
    pub kit: String,
    /// In [`SOCIAL_CLASSES`].
    pub social_class: usize,
    /// The Background… events, empty when there are none to choose.
    pub backgrounds: Vec<String>,
    pub background: String,
    pub history: String,
    pub age: u32,
    pub height: u32,
    pub weight: u32,
    pub hit_dice: Vec<u32>,
    /// The hit die rolls, or nothing with Enter Character, which has the
    /// levels' hit points instead (0x5bc9dc).
    pub hit_dice_text: Option<String>,
    /// Exceptional Strength, for a warrior with Strength 18; with Method X
    /// bought with the ability points, so not rolled here.
    pub exceptional_shown: bool,
    pub exceptional_bought: bool,
    pub exceptional: u32,
    /// Skills & Powers: how far each first subability is above its score,
    /// and the rows showing them.
    pub subabilities: Option<[i32; 6]>,
    pub subability_rows: [(String, String); 6],
    /// Enter Character's levels.
    pub levels: Option<EnteredLevels>,
    pub ok: bool,
}

/// Enter Character's levels on the details page (0x5bc9dc, 0x5bcaa3), on
/// the character as it would be saved.
#[derive(Debug, Clone, PartialEq)]
pub struct EnteredLevels {
    pub classes: Vec<ClassRow>,
    /// The hit points, when the levels give them.
    pub hit_points: Option<u32>,
    /// The PSPs, for a character with any.
    pub psp: Option<u32>,
    /// Why the character could not be made, in place of the rest.
    pub error: Option<String>,
    /// The Levels dialog, while it is open.
    pub dialog: Option<Box<LevelsForm>>,
    character: Option<Box<Character>>,
}

/// The most hit points and PSPs the arrows take.
pub const MOST_HIT_POINTS: u32 = 999;

impl EnteredLevels {
    fn new(rules: &CreationRules, advance: &AdvanceRules, draft: &Draft) -> Self {
        let mut levels = Self {
            classes: Vec::new(),
            hit_points: None,
            psp: None,
            error: None,
            dialog: None,
            character: None,
        };
        match rules.build(draft) {
            Ok(character) => levels.character = Some(Box::new(character)),
            Err(e) => levels.error = Some(e.to_string()),
        }
        levels.refresh(advance);
        levels
    }

    fn refresh(&mut self, advance: &AdvanceRules) {
        let Some(character) = &self.character else {
            return;
        };
        self.classes = class_rows(advance, character);
        self.hit_points = advance
            .hit_points(character)
            .ok()
            .map(|hp| hp.max(0) as u32);
        self.psp = psionic(advance, character).then(|| psionic_total(character));
    }
}

/// A kit or background event as the user reads it: the data's "None" and
/// "NONE" for none, translated.
pub fn none_label(name: &str) -> String {
    if name.eq_ignore_ascii_case("none") {
        tr("None").to_owned()
    } else {
        name.to_owned()
    }
}

impl DetailsPage {
    pub(super) fn new(w: &mut Wizard) -> Self {
        let rules = w.rules.clone();
        let entering = w.entering();
        let d = &w.draft;
        let mut alignments = if entering {
            ALIGNMENTS.map(String::from).to_vec()
        } else {
            rules.alignments(&d.classes)
        };
        if !alignments.contains(&d.alignment) {
            alignments.push(d.alignment.clone());
        }
        let strength = rules.draft_scores(d)[0];
        let mut backgrounds = w.needs.backgrounds.clone();
        if !backgrounds.is_empty() && !backgrounds.contains(&d.background) {
            backgrounds.push(d.background.clone());
        }
        let mut page = Self {
            heading: format!("{} {}", d.race, d.class_title()),
            name: d.name.clone(),
            player: d.player.clone(),
            male: d.male,
            random_names: w.needs.items.is_some(),
            portrait: d.portrait.clone(),
            alignments,
            alignment: d.alignment.clone(),
            kits: Vec::new(),
            kit: d.kit.clone(),
            social_class: SOCIAL_CLASSES
                .iter()
                .position(|s| *s == d.social_class)
                .unwrap_or(0),
            backgrounds,
            background: d.background.clone(),
            history: d.history.clone(),
            age: d.age,
            height: d.height,
            weight: d.weight,
            hit_dice: d.hit_dice.clone(),
            hit_dice_text: None,
            exceptional_shown: !entering && rules.has_exceptional_strength(&d.classes, strength),
            exceptional_bought: d.roll_method == RollMethod::PointsAndStrength,
            exceptional: d.exceptional_strength.clamp(1, 100),
            subabilities: d.skills_and_powers.then_some(d.subabilities),
            subability_rows: Default::default(),
            levels: entering.then(|| EnteredLevels::new(&rules, &w.needs.advance, d)),
            ok: true,
        };
        // The page opens with a name when the character has none
        // (0x5bc584).
        if page.name.trim().is_empty() {
            page.name = w.random_name(page.male);
        }
        w.refill_kits(&mut page);
        w.refresh_details(&mut page);
        page
    }

    /// The kit chosen's index in the kits offered.
    pub fn kit_index(&self) -> Option<usize> {
        self.kits.iter().position(|k| *k == self.kit)
    }

    pub fn alignment_index(&self) -> Option<usize> {
        self.alignments.iter().position(|a| *a == self.alignment)
    }

    pub fn background_index(&self) -> Option<usize> {
        self.backgrounds.iter().position(|b| *b == self.background)
    }
}

impl Wizard {
    /// A name from the generator, by the subrace (0x4b9da3); none without
    /// the program.
    fn random_name(&mut self, male: bool) -> String {
        let Some(npc) = self.needs.items.clone() else {
            return String::new();
        };
        let d = &self.draft;
        let subrace = if d.subrace.is_empty() {
            &d.race
        } else {
            &d.subrace
        };
        npc.random_name(&mut self.names, subrace, male)
    }

    /// The kits the gender and the alignment allow, the one chosen kept.
    fn refill_kits(&self, page: &mut DetailsPage) {
        let mut probe = self.draft.clone();
        probe.male = page.male;
        probe.alignment = page.alignment.clone();
        page.kits = self.rules.kits_for(&probe);
        if !page.kits.contains(&page.kit) {
            page.kits.push(page.kit.clone());
        }
    }

    fn refresh_details(&self, page: &mut DetailsPage) {
        page.hit_dice_text = (!self.entering())
            .then(|| hit_dice_text(&self.rules, &self.draft.classes, &page.hit_dice, &page.kit));
        if let Some(spreads) = page.subabilities {
            page.subability_rows = subability_rows(&self.rules, &self.draft, spreads);
        }
        // With the name generator an empty name gets a random one on Next.
        page.ok = page.random_names || !page.name.trim().is_empty();
    }

    fn details(&self) -> Option<DetailsPage> {
        match self.page() {
            Page::Details(page) => Some((**page).clone()),
            _ => None,
        }
    }

    /// Keeps what the levels changed in the draft, as the character
    /// Enter Character saves: the Levels dialog's results and each class's
    /// level.
    fn store_levels(&mut self, levels: &mut EnteredLevels) {
        let advance = self.needs.advance.clone();
        let Some(character) = &levels.character else {
            return;
        };
        let d = &mut self.draft;
        let mut entered = d.entered.clone().unwrap_or_default();
        entered.take(character);
        let class_levels: Vec<u32> = advance
            .levels(character)
            .map(|l| l.iter().map(|c| c.level).collect())
            .unwrap_or_default();
        d.entered = Some(entered);
        d.level = class_levels.iter().copied().max().unwrap_or(1).max(1);
        d.class_levels = class_levels;
        levels.refresh(&advance);
    }

    /// A change to Enter Character's levels.
    fn levels_input(&mut self, levels: &mut EnteredLevels, input: Input) {
        let advance = self.needs.advance.clone();
        match input {
            Input::OpenLevels => {
                let Some(character) = &levels.character else {
                    return;
                };
                let entered = self.draft.entered.clone().unwrap_or_default();
                levels.dialog = Some(Box::new(LevelsForm::new(
                    &advance,
                    (**character).clone(),
                    entered.prime_bonus,
                    entered.points_per_level,
                )));
            }
            Input::Levels(change) => {
                if let Some(dialog) = &mut levels.dialog {
                    dialog.input(&advance, change, &mut self.dice);
                }
            }
            Input::CloseLevels => {
                let Some(dialog) = levels.dialog.take() else {
                    return;
                };
                let (prime, per_level) = (dialog.prime_bonus, dialog.points_per_level);
                levels.character = Some(Box::new(dialog.close(&advance)));
                let entered = self.draft.entered.get_or_insert_with(Default::default);
                entered.prime_bonus = prime;
                entered.points_per_level = per_level;
                self.store_levels(levels);
            }
            Input::HitPoints(wanted) => {
                let Some(character) = &mut levels.character else {
                    return;
                };
                let wanted = wanted.clamp(1, MOST_HIT_POINTS) as i32;
                let changed = advance
                    .hit_points(character)
                    .and_then(|hp| advance.adjust_hit_points(character, wanted - hp));
                if changed.is_ok() {
                    self.store_levels(levels);
                }
            }
            Input::Psp(wanted) => {
                let Some(character) = &mut levels.character else {
                    return;
                };
                let delta =
                    i64::from(wanted.min(MOST_HIT_POINTS)) - i64::from(psionic_total(character));
                adjust_psionic_points(character, delta as i32);
                self.store_levels(levels);
            }
            _ => {}
        }
    }

    /// Puts `page` back as the details in view, refreshed.
    fn set_details(&mut self, mut page: DetailsPage) {
        // The kit chosen may change the experience the levels take.
        if let Some(levels) = &mut page.levels
            && let Some(character) = &mut levels.character
            && character.data.kit != page.kit
        {
            character.data.kit = page.kit.clone();
            self.store_levels(levels);
        }
        self.refresh_details(&mut page);
        if let Page::Details(shown) = self.page_mut() {
            **shown = page;
        }
    }

    pub(super) fn details_input(&mut self, input: Input) {
        let Some(mut page) = self.details() else {
            return;
        };
        let entering = self.entering();
        let race = self.draft.race.clone();
        match input {
            Input::Name(text) => page.name = text,
            Input::RandomName => page.name = self.random_name(page.male),
            Input::Player(text) => page.player = text,
            Input::Portrait(portrait) => page.portrait = portrait,
            Input::Gender(i) => {
                let male = i == 0;
                if male == page.male {
                    return;
                }
                // A gender the kit does not allow is refused (0x5be7d3);
                // Enter Character sets it freely.
                if !entering && !self.rules.kit_allows_gender(&page.kit, male) {
                    self.message(
                        tr("Unable to change"),
                        if male {
                            tr("The currently selected kit is not available to male characters.")
                        } else {
                            tr("The currently selected kit is not available to female characters.")
                        },
                    );
                    return;
                }
                // Height and weight depend on the gender.
                page.male = male;
                page.height = self.rules.roll_height(&race, male, &mut self.dice);
                page.weight = self.rules.roll_weight(&race, male, &mut self.dice);
                self.refill_kits(&mut page);
            }
            Input::Alignment(i) => {
                let Some(chosen) = page.alignments.get(i).cloned() else {
                    return;
                };
                if chosen == page.alignment {
                    return;
                }
                if entering || self.rules.kit_allows_alignment(&page.kit, &chosen) {
                    page.alignment = chosen;
                    self.refill_kits(&mut page);
                } else {
                    // An alignment the kit does not allow takes a yes
                    // (0x5beb8f).
                    let yes: super::Then = Arc::new(move |w: &mut Wizard| {
                        if let Some(mut page) = w.details() {
                            page.alignment = chosen.clone();
                            w.refill_kits(&mut page);
                            w.set_details(page);
                        }
                    });
                    self.show(Prompt::ask(
                        tr("Warning"),
                        tr("The chosen alignment is not compatible with the chosen kit. Select anyway?"),
                        yes,
                        None,
                    ));
                    return;
                }
            }
            Input::Kit(i) => {
                let Some(chosen) = page.kits.get(i).cloned() else {
                    return;
                };
                if chosen.is_empty() || chosen == page.kit {
                    return;
                }
                if !self.draft.proficiencies_started {
                    page.kit = chosen.clone();
                    self.set_details(page);
                    self.kit_information(&chosen);
                    return;
                }
                // Once Next has gone on, a new kit resets the proficiencies
                // (0x5be41b), after a yes.
                let yes: super::Then = Arc::new(move |w: &mut Wizard| {
                    w.draft.reset_proficiencies();
                    if let Some(mut page) = w.details() {
                        page.kit = chosen.clone();
                        w.set_details(page);
                    }
                    w.kit_information(&chosen);
                });
                self.show(Prompt::ask(
                    tr("Warning"),
                    tr(
                        "Your kit adjusts the cost of proficiencies. Changing your kit will reset \
                        any selections of weapon and non-weapon proficiencies. Do you want to \
                        continue?",
                    ),
                    yes,
                    None,
                ));
                return;
            }
            Input::SocialClass(i) if i < SOCIAL_CLASSES.len() => page.social_class = i,
            Input::RollSocialClass => {
                let rolled = self.rules.roll_social_class(&page.kit, &mut self.dice);
                if let Some(i) = SOCIAL_CLASSES.iter().position(|s| *s == rolled) {
                    page.social_class = i;
                }
            }
            Input::Background(i) => {
                if let Some(event) = page.backgrounds.get(i) {
                    page.background = event.clone();
                }
            }
            Input::RandomBackground => {
                if let Some(event) =
                    cr_store::edit::random_background(&mut self.names, &page.backgrounds)
                {
                    page.background = event.to_owned();
                }
            }
            Input::History(text) => page.history = text,
            Input::Age(v) => page.age = v.clamp(1, 9999),
            Input::Height(v) => page.height = v.clamp(1, 999),
            Input::Weight(v) => page.weight = v.clamp(1, 9999),
            Input::RollAge => page.age = self.rules.roll_age(&race, &mut self.dice),
            Input::RollHeight => {
                page.height = self.rules.roll_height(&race, page.male, &mut self.dice)
            }
            Input::RollWeight => {
                page.weight = self.rules.roll_weight(&race, page.male, &mut self.dice)
            }
            Input::RollHitPoints => {
                page.hit_dice = self
                    .rules
                    .roll_hit_dice(&self.draft.classes, &mut self.dice)
            }
            Input::Exceptional(v) if !page.exceptional_bought => page.exceptional = v.clamp(1, 100),
            Input::RollExceptional if !page.exceptional_bought => {
                page.exceptional = self.dice.roll(1, 100)
            }
            Input::Subability(a, v) if a < 6 => {
                if let Some(spreads) = &mut page.subabilities {
                    spreads[a] = v.clamp(-SUBABILITY_SPREAD, SUBABILITY_SPREAD);
                }
            }
            Input::Next if page.ok => {
                self.details_next(page);
                return;
            }
            Input::OpenLevels
            | Input::Levels(_)
            | Input::CloseLevels
            | Input::HitPoints(_)
            | Input::Psp(_) => {
                let Some(mut levels) = page.levels.take() else {
                    return;
                };
                self.levels_input(&mut levels, input);
                page.levels = Some(levels);
            }
            _ => return,
        }
        self.set_details(page);
    }

    /// Shows the kit's information text when it has one, as the original
    /// does whenever a kit is chosen.
    fn kit_information(&mut self, kit: &str) {
        if let Some(text) = self.rules.kit_information(kit) {
            let text = text.to_owned();
            self.message(tr("Kit Information"), text);
        }
    }

    fn details_next(&mut self, mut page: DetailsPage) {
        let entering = self.entering();
        if page.name.trim().is_empty() {
            page.name = self.random_name(page.male);
        }
        let d = &mut self.draft;
        d.name = page.name.trim().to_owned();
        d.player = page.player.trim().to_owned();
        d.portrait = page.portrait.clone();
        if !page.backgrounds.is_empty() {
            d.background = page.background.clone();
            d.history = page.history.clone();
        }
        d.male = page.male;
        d.alignment = page.alignment.clone();
        let new_kit = d.kit != page.kit;
        if new_kit {
            d.barred_schools_ignored = false;
            d.eliminated_schools.clear();
        }
        d.kit = page.kit.clone();
        // The page's rolls are for the gender and the kit it chose.
        if let Some(key) = &mut self.rolled {
            key.set_details(d.male, &d.kit);
        }
        d.social_class = SOCIAL_CLASSES[page.social_class].to_owned();
        // The starting money follows the kit (0x4330ab).
        if new_kit {
            d.gold = self.rules.roll_gold(d, &mut self.dice);
        }
        d.age = page.age;
        d.height = page.height;
        d.weight = page.weight;
        d.hit_dice = page.hit_dice.clone();
        if d.roll_method != RollMethod::PointsAndStrength && !entering {
            d.exceptional_strength = page.exceptional;
        }
        if let Some(spreads) = page.subabilities {
            d.subabilities = spreads;
        }
        // The details as Next left them, for Back.
        self.set_details(page);
        self.push_proficiencies();
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::tests::wizard;
    use super::super::{Answer, PromptKind};
    use super::*;

    /// A human fighter on the details page.
    fn at_details(entering: bool) -> Option<Wizard> {
        at_details_with(entering, false)
    }

    /// A human fighter on the details page; `sp` with Skills & Powers.
    pub(in super::super) fn at_details_with(entering: bool, sp: bool) -> Option<Wizard> {
        let mut w = wizard(entering)?;
        if sp {
            w.input(Input::Rules(1));
        }
        w.input(Input::Next);
        if !entering {
            w.answer(Answer::Yes);
            let Page::Abilities(page) = w.page() else {
                return None;
            };
            if let Some(vii) = page.methods.iter().position(|m| *m == RollMethod::Points) {
                w.input(Input::Method(vii));
                for (a, score) in [18, 12, 13, 10, 10, 12].into_iter().enumerate() {
                    w.input(Input::Score(a, score));
                }
            }
        }
        // Next through the pages, a human fighter on the way.
        for _ in 0..12 {
            if w.prompt().is_some() {
                w.answer(Answer::Yes);
                continue;
            }
            w.input(Input::Next);
            match w.page() {
                Page::Race(page) => {
                    let human = page.races.iter().position(|(r, _)| r == "Human")?;
                    w.input(Input::Race(human));
                }
                Page::Class(page) => {
                    let fighter = page.options.iter().position(|o| *o == ["Fighter"])?;
                    w.input(Input::Class(fighter));
                }
                Page::EnteredClass(page) => {
                    let fighter = page.options.iter().position(|o| *o == ["Fighter"])?;
                    w.input(Input::Class(fighter));
                }
                Page::Details(_) => return Some(w),
                _ => {}
            }
        }
        None
    }

    fn details(w: &Wizard) -> &DetailsPage {
        match w.page() {
            Page::Details(page) => page,
            _ => panic!("not the details"),
        }
    }

    #[test]
    fn the_details_open_with_a_name_and_the_rolls() {
        let Some(mut w) = at_details(false) else {
            return;
        };
        let page = details(&w);
        assert_eq!(page.heading, "Human Fighter");
        assert!(page.random_names);
        assert!(!page.name.is_empty());
        assert!(page.exceptional_shown);
        assert!(page.hit_dice_text.as_ref().unwrap().starts_with("Fighter "));
        assert!(page.ok);
        w.input(Input::Name(String::new()));
        w.input(Input::Player("Brahm".into()));
        w.input(Input::Age(30));
        w.input(Input::Exceptional(77));
        w.input(Input::Next);
        assert!(matches!(w.page(), Page::Proficiencies(_)));
        let d = w.draft();
        // An empty name gets a random one on Next.
        assert!(!d.name.is_empty());
        assert_eq!(d.player, "Brahm");
        assert_eq!(d.age, 30);
        assert_eq!(d.exceptional_strength, 77);
    }

    #[test]
    fn back_to_the_class_and_next_keep_the_details() {
        let Some(mut w) = at_details(false) else {
            return;
        };
        w.input(Input::Player("Brahm".into()));
        w.input(Input::History("Born in a storm".into()));
        let before = details(&w).clone();
        assert!(w.back());
        w.input(Input::Next);
        while w.prompt().is_some() {
            w.answer(Answer::Yes);
        }
        assert_eq!(*details(&w), before);
        // Past the details and back, the rolls stay as Next kept them.
        w.input(Input::Next);
        while w.prompt().is_some() {
            w.answer(Answer::Yes);
        }
        assert!(w.back());
        assert!(w.back());
        w.input(Input::Next);
        while w.prompt().is_some() {
            w.answer(Answer::Yes);
        }
        let page = details(&w);
        assert_eq!(
            (page.height, page.weight, &page.hit_dice, page.social_class),
            (
                before.height,
                before.weight,
                &before.hit_dice,
                before.social_class
            )
        );
        assert_eq!(page.player, "Brahm");
    }

    #[test]
    fn a_gender_chosen_on_the_details_keeps_them_past_the_class() {
        let Some(mut w) = at_details(false) else {
            return;
        };
        w.input(Input::Gender(1));
        while w.prompt().is_some() {
            w.answer(Answer::Yes);
        }
        w.input(Input::Age(30));
        let before = details(&w).clone();
        w.input(Input::Next);
        while w.prompt().is_some() {
            w.answer(Answer::Yes);
        }
        assert!(w.back());
        assert!(w.back());
        w.input(Input::Next);
        while w.prompt().is_some() {
            w.answer(Answer::Yes);
        }
        let page = details(&w);
        assert_eq!(
            (
                page.male,
                page.age,
                page.height,
                page.weight,
                page.social_class
            ),
            (
                before.male,
                30,
                before.height,
                before.weight,
                before.social_class
            )
        );
        assert_eq!(page.hit_dice, before.hit_dice);
    }

    #[test]
    fn an_alignment_the_kit_bars_takes_a_yes() {
        let Some(mut w) = at_details(false) else {
            return;
        };
        let page = details(&w).clone();
        // A kit that bars some alignment the class allows.
        let found = page.kits.iter().enumerate().find_map(|(k, kit)| {
            let barred = page
                .alignments
                .iter()
                .position(|a| !w.rules().kit_allows_alignment(kit, a))?;
            Some((k, barred))
        });
        let Some((kit, barred)) = found else {
            return;
        };
        w.input(Input::Kit(kit));
        // The kit's information, if it has any.
        if w.prompt().is_some() {
            w.answer(Answer::Yes);
        }
        w.input(Input::Alignment(barred));
        assert!(matches!(
            w.prompt().unwrap().kind,
            PromptKind::Question { .. }
        ));
        w.answer(Answer::No);
        assert_ne!(details(&w).alignment, page.alignments[barred]);
        w.input(Input::Alignment(barred));
        w.answer(Answer::Yes);
        assert_eq!(details(&w).alignment, page.alignments[barred]);
    }

    #[test]
    fn enter_character_has_no_hit_die_rolls() {
        let Some(w) = at_details(true) else {
            return;
        };
        let page = details(&w);
        assert!(page.hit_dice_text.is_none());
        assert!(!page.exceptional_shown);
        assert_eq!(page.alignments.len(), ALIGNMENTS.len());
    }

    #[test]
    fn enter_character_sets_the_levels_and_hit_points() {
        use crate::levels::LevelsInput;
        let Some(mut w) = at_details(true) else {
            return;
        };
        let levels = details(&w).levels.clone().unwrap();
        assert!(levels.error.is_none(), "{levels:?}");
        assert_eq!(levels.classes[0].level, 1);
        w.input(Input::OpenLevels);
        w.input(Input::Levels(LevelsInput::Level(0, 5)));
        let dialog = details(&w).levels.clone().unwrap().dialog.unwrap();
        assert_eq!(dialog.classes[0].level, 5);
        // Add Experience: a level more.
        w.input(Input::Levels(LevelsInput::AddExperience));
        w.input(Input::Levels(LevelsInput::Amount(1_000_000)));
        w.input(Input::Levels(LevelsInput::Give));
        let dialog = details(&w).levels.clone().unwrap().dialog.unwrap();
        assert!(dialog.experience.is_none());
        let level = dialog.classes[0].level;
        assert!(level > 5, "{dialog:?}");
        w.input(Input::CloseLevels);
        let levels = details(&w).levels.clone().unwrap();
        assert!(levels.dialog.is_none());
        assert_eq!(levels.classes[0].level, level);
        assert_eq!(w.draft().class_levels, [level]);
        assert_eq!(w.draft().level, level);
        // The hit points move the rolls.
        let hp = levels.hit_points.unwrap();
        w.input(Input::HitPoints(hp + 7));
        assert_eq!(details(&w).levels.clone().unwrap().hit_points, Some(hp + 7));
        let character = w.rules().build(w.draft()).unwrap();
        assert_eq!(
            w.needs.advance.hit_points(&character).unwrap(),
            (hp + 7) as i32
        );
    }
}
