//! The Wild Talents dialog's Test for Wild Talents, Roll for Wild Talents
//! and Add (IDD 358), as Update Character, New Character and Enter
//! Character offer them (Skills & Powers, chapter 9). New Character's
//! Skills page (IDD 353, 0x5d73ef) opens it for a character without a
//! psionicist class.
//!
//! * Test (0x6366b7) asks again when the character has tested before
//!   (+0x1874), then shows the chance (0x6368bb): 1, plus 3 for each of
//!   Wisdom, Constitution and Intelligence at 18 or more and 2 at 16 or
//!   17, plus 1 for a highest class level of 5 to 8 and 2 above 9; halved
//!   for a race other than Human, or else for a priest or wizard class;
//!   .49 and more rounds up. A d100 up to the chance makes a wild talent
//!   (+0x186c). On 97, 98 and 99 the character saves vs. death or loses
//!   1d6 of Wisdom, Intelligence or Constitution and of both its
//!   subabilities, none going below 1; on 100 it saves at −5 or has all
//!   three set to 3.
//! * Roll (0x635aa3) replaces the wild talents with a roll on Table 83,
//!   adding the powers they require (0x6361d0): 1d100 up to 96 is a
//!   devotion at that roll, 97 and 98 two devotions of 1d96, 99 a
//!   science of 1d92 (93 to 95 on another 1d100 none, 96 and more two),
//!   and 100 a devotion and then as 99. A power is the first whose
//!   chance on the table (`unknown_0x40`), added up in the database's
//!   order, reaches the roll; the shipped sciences reach only 88.
//!   New Character rolls only once (0x635aa3); Update Character asks
//!   before rolling again; Enter Character does not ask.
//! * Add (0x6363c7) asks before taking a talent that was not rolled. New
//!   Character has no list to add from: its talents are only rolled, once
//!   the character is a wild talent by a test or by a trait (0x635177).

use cr_formats::character::Character;
use cr_i18n::{tr, tr_noop};

use super::SkillRules;
use crate::create::{Dice, Question};

/// The title of the original's questions.
pub const WARNING_MESSAGE: &str = tr_noop!("Warning Message");
/// The title of the result messages: the dialog's.
const TITLE: &str = tr_noop!("Wild Talents");

/// The highest roll on the devotions and on the sciences.
const DEVOTIONS: u32 = 96;
const SCIENCES: u32 = 92;

/// The class groups that halve the chance of a human: priest and wizard.
const HALVING_GROUPS: [i32; 2] = [2, 3];

/// Where the dialog is open (its +0x60).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WildMode {
    Create,
    Enter,
    Update,
}

/// The wild talents of a character being created or entered, written
/// when it is saved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WildDraft {
    pub talents: Vec<String>,
    pub attacks: Vec<String>,
    pub defenses: Vec<String>,
    /// Made a wild talent by a test, and tested at all.
    pub wild_talent: bool,
    pub tested: bool,
    /// Constitution, Intelligence and Wisdom after a test's loss.
    pub abilities: Option<[[u32; 3]; 3]>,
}

/// What a test did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WildTest {
    /// The messages to show, in order.
    pub notices: Vec<Question>,
    pub wild_talent: bool,
    /// Constitution, Intelligence and Wisdom after the test, each the score
    /// and its two subabilities (`data.abilities[1..4]`).
    pub abilities: [[u32; 3]; 3],
}

fn notice(text: String) -> Question {
    Question::notice(tr(TITLE), text)
}

/// The ability a roll of 97, 98 or 99 lowers, by its index in
/// `WildTest::abilities`.
fn lowered(roll: u32) -> usize {
    match roll {
        97 => 2,
        98 => 1,
        _ => 0,
    }
}

impl SkillRules {
    /// A wild talent by a test or by a trait (0x42e832): Test is no longer
    /// offered.
    pub fn is_wild_talent(&self, character: &Character, tested_wild: bool) -> bool {
        tested_wild
            || character.data.unknown_0x1830[0]
                .iter()
                .any(|t| self.wild_traits.contains(t))
    }

    /// The percent chance to become a wild talent (0x6368bb), from the
    /// scores in `character`.
    pub fn wild_chance(&self, character: &Character) -> u32 {
        let d = &character.data;
        let mut chance = 1.0f32;
        for score in [d.abilities[3][0], d.abilities[1][0], d.abilities[2][0]] {
            chance += match score {
                18.. => 3.0,
                16 | 17 => 2.0,
                _ => 0.0,
            };
        }
        let highest = (0..d.classes.names.len())
            .filter_map(|g| self.advance.level_of(character, g).ok())
            .max()
            .unwrap_or(0);
        // A 9th level adds nothing, as in the original.
        chance += match highest {
            5..=8 => 1.0,
            10.. => 2.0,
            _ => 0.0,
        };
        let halved = d.race != "Human"
            || self
                .classes(character)
                .any(|c| HALVING_GROUPS.contains(&c.group));
        if halved {
            chance /= 2.0;
        }
        (chance + 0.51) as u32
    }

    /// What Test asks before rolling: whether to test again, and whether
    /// to go on with the chance shown.
    pub fn test_questions(&self, character: &Character, tested: bool) -> Vec<Question> {
        let mut questions = Vec::new();
        if tested {
            questions.push(Question::new(
                tr(WARNING_MESSAGE),
                tr(
                    "Only under very rare circumstances would a character be allowed to make \
                    more than one test for Wild Talents.\nTest again anyway?",
                )
                .to_owned(),
            ));
        }
        questions.push(Question::new(
            tr("There is no undo"),
            tr!(
                "You have a {chance} percent chance to become a wild talent.\nThere is a 4 \
                 percent chance something bad will happen to your character.\nAre you sure you \
                 wish to continue?",
                chance = self.wild_chance(character)
            ),
        ));
        questions
    }

    /// The test itself, once asked: d100 against the chance, and the
    /// saving throws a roll of 97 or more calls for.
    pub fn test_wild_talent(&self, character: &Character, dice: &mut Dice) -> WildTest {
        let chance = self.wild_chance(character);
        let a = &character.data.abilities;
        let mut test = WildTest {
            notices: Vec::new(),
            wild_talent: false,
            abilities: [a[1], a[2], a[3]],
        };
        let roll = dice.roll(1, 100);
        if roll <= chance {
            test.notices.push(notice(tr!(
                "Congratulations, you rolled successfully. (A roll of {roll}).\nYou may now \
                 select Roll for Wild Talents.",
                roll
            )));
            test.wild_talent = true;
            return test;
        }
        if roll < 97 {
            test.notices.push(notice(tr!(
                "You are not a Wild Talent.\nYou needed to roll under {needed}. (Your roll \
                 {roll}).",
                needed = chance + 1,
                roll
            )));
            return test;
        }
        let target = self.sheet.death_save(character).unwrap_or(20);
        if roll == 100 {
            test.notices.push(notice(
                tr(
                    "You rolled a 100. That's the worst you can do. You must now save vs. death \
                    or have your character's Wisdom, Intelligence, and Constitution permanently \
                    set to 3 points. Select OK to roll",
                )
                .to_owned(),
            ));
            if !self.save(target, -5, dice, &mut test.notices) {
                test.abilities = [[3; 3]; 3];
            }
            return test;
        }
        test.notices.push(notice(
            match roll {
                97 => tr(
                    "You rolled a 97. You must now save vs. death or have your character's Wisdom \
                 permanently reduced by 1d6 points. Select OK to roll",
                ),
                98 => tr(
                    "You rolled a 98. You must now save vs. death or have your character's \
                 Intelligence permanently reduced by 1d6 points. Select OK to roll",
                ),
                _ => tr(
                    "You rolled a 99. You must now save vs. death or have your character's \
                 Constitution permanently reduced by 1d6 points. Select OK to roll",
                ),
            }
            .to_owned(),
        ));
        if self.save(target, 0, dice, &mut test.notices) {
            return test;
        }
        let scores = &mut test.abilities[lowered(roll)];
        let lowest = scores.iter().copied().min().unwrap_or(0);
        let loss = dice.roll(1, 6).min(lowest.saturating_sub(1));
        test.notices.push(notice(match (roll, loss) {
            (97, 0) => tr(
                "Due to your character's pathetic Wisdom score, no adjustments have been made to \
                 your character.",
            )
            .to_owned(),
            (98, 0) => tr(
                "Due to your character's pathetic Intelligence score, no adjustments have been \
                 made to your character.",
            )
            .to_owned(),
            (_, 0) => tr(
                "Due to your character's pathetic Constitution score, no adjustments have been \
                 made to your character.",
            )
            .to_owned(),
            (97, loss) => tr!("You just lost {loss} from your Wisdom score.", loss),
            (98, loss) => tr!("You just lost {loss} from your Intelligence score.", loss),
            (_, loss) => tr!("You just lost {loss} from your Constitution score.", loss),
        }));
        for score in scores.iter_mut() {
            *score -= loss;
        }
        test
    }

    /// The saving throw (0x636b2b): 1d20 plus `modifier`, made at
    /// `target` or more.
    fn save(
        &self,
        target: i32,
        modifier: i32,
        dice: &mut Dice,
        notices: &mut Vec<Question>,
    ) -> bool {
        let roll = dice.roll(1, 20) as i32 + modifier;
        let made = roll >= target;
        notices.push(notice(if made {
            tr!(
                "Congratulations, you made your Saving Throw.\n(You needed to roll {target} or \
                 higher and you had a roll of {roll}).",
                target,
                roll
            )
        } else if modifier != 0 {
            tr!(
                "You failed to save.\n(You needed to roll {target} or higher and you had a \
                 modified roll of {roll}).\nPrepare to face the consequences.",
                target,
                roll
            )
        } else {
            tr!(
                "You failed to save.\n(You needed to roll {target} or higher and you had a roll \
                 of {roll}).\nPrepare to face the consequences.",
                target,
                roll
            )
        }));
        made
    }

    /// What Roll asks first: a character with wild talents may roll only
    /// once. New Character refuses (the error), Update Character asks.
    pub fn roll_questions(
        &self,
        taken: &[String],
        mode: WildMode,
    ) -> Result<Vec<Question>, Question> {
        if taken.is_empty() {
            return Ok(Vec::new());
        }
        match mode {
            WildMode::Create => Err(notice(
                tr("You may only roll once for your Wild Talent").to_owned(),
            )),
            WildMode::Enter => Ok(Vec::new()),
            WildMode::Update => Ok(vec![Question::new(
                tr(WARNING_MESSAGE),
                tr("You may only roll once for your Wild Talent.\nRoll again anyway?").to_owned(),
            )]),
        }
    }

    /// Writes the wild talents of a new character (as the dialog's OK,
    /// 0x635645): the powers as sciences and devotions, the PSPs they give,
    /// the attack and defense modes, the test and its losses.
    pub fn apply_wild(&self, character: &mut Character, wild: &WildDraft, dice: &mut Dice) {
        let d = &mut character.data;
        if let Some(abilities) = wild.abilities {
            d.abilities[1..4].copy_from_slice(&abilities);
        }
        d.unknown_0x186c = wild.wild_talent.into();
        d.unknown_0x1874 = wild.tested.into();
        let (devotions, sciences): (Vec<String>, Vec<String>) = wild
            .talents
            .iter()
            .cloned()
            .partition(|n| self.power(n).is_some_and(|p| p.devotion));
        self.wild_points(d, &[], &wild.talents, dice);
        d.unknown_0x196c[1] = sciences;
        d.unknown_0x196c[2] = devotions;
        d.unknown_0xac[6].clone_from(&wild.attacks);
        d.unknown_0x19c4.clone_from(&wild.defenses);
    }

    /// New Character's text on the Wild Talents dialog (0x635177), for a
    /// Skills & Powers character not yet a wild talent.
    pub fn wild_instructions(&self) -> &'static str {
        tr(
            "If you wish to purchase Wild Talents, you need to return to the Non-Weapon \
            Proficiencies Screen. There, you may purchase Wild Talents for 9 character \
            points. Here, you may Test for Wild Talents instead of purchasing them. Testing \
            for Wild Talents involves risk. Testing for Wild Talents should only be done once \
            per character. For more information please consult Chapter 9 of Skills & Powers.",
        )
    }

    /// What Add asks before a talent that was not rolled.
    pub fn add_question(&self) -> Question {
        Question::new(
            tr(WARNING_MESSAGE),
            tr("You should roll for Wild Talents.\nTake this Wild Talent anyway?").to_owned(),
        )
    }

    /// The power of the devotions or sciences at `roll` on the table (0x635ed0,
    /// 0x636050): the first whose chances so far reach it. A roll off the
    /// table counts as 1.
    fn wild_power(&self, devotion: bool, roll: u32) -> Option<&(bool, u32, String, Vec<String>)> {
        let most = if devotion { DEVOTIONS } else { SCIENCES };
        let roll = if (1..=most).contains(&roll) { roll } else { 1 };
        let mut sum = 0;
        self.wild_table
            .iter()
            .filter(|p| p.0 == devotion)
            .find(|p| {
                sum += p.1;
                sum >= roll
            })
    }

    /// Two rolls of `sides` for two powers: the second, when the same,
    /// one less, and when that is 0, 3d50 more (as the original does).
    fn two_rolls(dice: &mut Dice, sides: u32) -> [u32; 2] {
        let first = dice.roll(1, sides);
        let mut second = dice.roll(1, sides);
        if second == first {
            second -= 1;
            if second < 1 {
                second += dice.roll(3, 50);
            }
        }
        [first, second]
    }

    /// Rolls the wild talents on Table 83, with the powers they require
    /// after them, and the messages to show.
    pub fn roll_wild_talents(&self, dice: &mut Dice) -> (Vec<String>, Vec<Question>) {
        let mut rolls: Vec<(bool, u32)> = Vec::new();
        let mut notices = Vec::new();
        let roll = dice.roll(1, 100);
        match roll {
            ..97 => rolls.push((true, roll)),
            97 | 98 => rolls.extend(Self::two_rolls(dice, DEVOTIONS).map(|r| (true, r))),
            _ => {
                if roll == 100 {
                    rolls.push((true, dice.roll(1, DEVOTIONS)));
                }
                match dice.roll(1, 100) {
                    ..93 => rolls.push((false, dice.roll(1, SCIENCES))),
                    93..96 => notices.push(notice(tr("No science gained").to_owned())),
                    _ => rolls.extend(Self::two_rolls(dice, SCIENCES).map(|r| (false, r))),
                }
            }
        }
        let mut names: Vec<String> = Vec::new();
        let mut rolled = Vec::new();
        for (devotion, roll) in rolls {
            match self.wild_power(devotion, roll) {
                Some(power) => rolled.push(power),
                // The table's chances may not reach the roll.
                None => notices.push(notice(tr("Could not roll Wild Talents").to_owned())),
            }
        }
        for name in rolled
            .iter()
            .map(|p| &p.2)
            .chain(rolled.iter().flat_map(|p| &p.3))
        {
            if !name.is_empty() && !names.contains(name) {
                names.push(name.clone());
            }
        }
        (names, notices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_roll_lowers_its_own_ability() {
        assert_eq!(lowered(97), 2);
        assert_eq!(lowered(98), 1);
        assert_eq!(lowered(99), 0);
    }

    #[test]
    fn two_rolls_differ() {
        let mut dice = Dice::seeded(7);
        for _ in 0..200 {
            let [a, b] = SkillRules::two_rolls(&mut dice, 6);
            assert_ne!(a, b);
        }
    }
}
