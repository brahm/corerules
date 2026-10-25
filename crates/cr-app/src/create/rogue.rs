//! The Rogue Skills page: the discretionary points added to the thieving
//! skills, with the scores they give, and the kit's bonus and chosen
//! skills.

use cr_i18n::tr;
use cr_store::create::{Draft, ROGUE_SKILLS, rogue_skill_of};
use cr_store::skills::MOST_ROGUE_SCORE;

use super::{ENTERED_ROGUE_POINTS, Input, Page, Prompt, Wizard, points};

/// A skill's row: its points and the scores they give.
#[derive(Debug, Clone, PartialEq)]
pub struct RogueRow {
    /// The skill, in [`ROGUE_SKILLS`].
    pub skill: usize,
    pub name: String,
    pub points: u32,
    pub scores: String,
}

/// The skills a kit names, picked as a group.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RoguePicks {
    pub title: String,
    pub description: String,
    pub names: Vec<String>,
    pub picked: Vec<bool>,
    /// As many are ticked as may be: the others are off.
    pub full: bool,
    pub shown: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoguePage {
    pub rows: Vec<RogueRow>,
    /// The most points a skill takes.
    pub most: u32,
    pub description: String,
    /// The kit's bonus to one skill: one is picked.
    pub bonus: RoguePicks,
    /// The kit's chosen skills: up to its count are ticked.
    pub chosen: RoguePicks,
    pub ok: bool,
    /// The points as the page has them, which Next keeps.
    points: [u32; 8],
    /// The kit's warning was ignored on this visit (62c144).
    ignored: bool,
}

/// A kit's name for a skill, as the skill's own name.
fn skill_title(name: &str) -> String {
    rogue_skill_of(name).map_or_else(|| name.to_owned(), |i| tr(ROGUE_SKILLS[i].0).to_owned())
}

impl Wizard {
    /// Rogue Skills when the classes have them, then the spells.
    pub(super) fn after_combat(&mut self) {
        if self.rules.rogue_skills(&self.draft).is_empty() {
            self.after_rogue_skills();
            return;
        }
        let points = self.draft.rogue_points;
        self.push(Page::Rogue(Box::new(RoguePage {
            rows: Vec::new(),
            most: 0,
            description: String::new(),
            bonus: RoguePicks::default(),
            chosen: RoguePicks::default(),
            ok: false,
            points,
            ignored: false,
        })));
        self.refresh_rogue();
    }

    /// The draft with the page's points.
    fn rogue_probe(&self, points: [u32; 8]) -> Draft {
        let mut probe = self.draft.clone();
        probe.rogue_points = [0; 8];
        for (_, skill) in self.rules.rogue_skills(&self.draft) {
            probe.rogue_points[skill] = points[skill];
        }
        probe
    }

    fn refresh_rogue(&mut self) {
        let Page::Rogue(page) = self.page() else {
            return;
        };
        let probe = self.rogue_probe(page.points);
        let rules = &self.rules;
        let entering = self.entering();
        let d = &self.draft;
        let (total, most) = rules.rogue_points(d);
        let rows = rules
            .rogue_skills(d)
            .into_iter()
            .map(|(_, skill)| {
                let (score, unarmored) = rules.rogue_skill_score(&probe, skill);
                RogueRow {
                    skill,
                    name: tr(ROGUE_SKILLS[skill].0).to_owned(),
                    points: probe.rogue_points[skill],
                    scores: tr!("{score}% · {unarmored}% without armor", score, unarmored),
                }
            })
            .collect();
        let left = rules.rogue_points_left(&probe);
        // Enter Character has no limits (0x62b739).
        let (description, most, ok) = if entering {
            (
                tr(
                    "Any points, as in the original's Enter Character; the race and \
                    Dexterity adjust the scores",
                )
                .to_owned(),
                ENTERED_ROGUE_POINTS,
                true,
            )
        } else {
            (
                tr!(
                    "{left} of {total} left, at most {most} a skill; the race and Dexterity \
                     adjust the scores",
                    left = points(left),
                    total,
                    most
                ),
                most,
                left >= 0,
            )
        };
        let bonus = match rules.kit_rogue_bonus(d) {
            Some((amount, names)) => RoguePicks {
                title: tr("Kit Bonus").to_owned(),
                description: tr!(
                    "The selected kit allows you to add {points} point(s) to one rogue skill \
                     from the following list at level {level}.",
                    points = amount,
                    level = 1
                ),
                picked: names
                    .iter()
                    .map(|n| d.rogue_bonus.as_ref() == Some(n))
                    .collect(),
                names: names.iter().map(|n| skill_title(n)).collect(),
                full: false,
                shown: true,
            },
            None => RoguePicks::default(),
        };
        let chosen = match rules.kit_chosen_skills(d) {
            Some((count, bonus, names)) => {
                let picked: Vec<bool> = names.iter().map(|n| d.rogue_chosen.contains(n)).collect();
                RoguePicks {
                    title: tr("Chosen Rogue Skills").to_owned(),
                    description: tr!(
                        "The selected kit allows you to select {count} skills as 'chosen' for \
                         a {bonus}% bonus.",
                        count,
                        bonus
                    ),
                    full: picked.iter().filter(|p| **p).count() >= count,
                    picked,
                    names: names.iter().map(|n| skill_title(n)).collect(),
                    shown: true,
                }
            }
            None => RoguePicks::default(),
        };
        if let Page::Rogue(page) = self.page_mut() {
            page.rows = rows;
            page.most = most;
            page.description = description;
            page.bonus = bonus;
            page.chosen = chosen;
            page.ok = ok;
        }
    }

    pub(super) fn rogue_input(&mut self, input: Input) {
        let Page::Rogue(page) = self.page() else {
            return;
        };
        let page = page.as_ref().clone();
        match input {
            Input::RoguePoints(row, wanted) => {
                let Some(skill) = page.rows.get(row).map(|r| r.skill) else {
                    return;
                };
                let before = page.points[skill];
                if wanted == before || wanted > page.most {
                    return;
                }
                let mut points = page.points;
                points[skill] = wanted;
                if wanted > before && !self.entering() {
                    // Points past what is left, or a score past 95%, are
                    // refused.
                    let probe = self.rogue_probe(points);
                    if self.rules.rogue_points_left(&probe) < 0
                        || self.rules.rogue_skill_score(&probe, skill).0 > MOST_ROGUE_SCORE
                    {
                        self.refresh_rogue();
                        return;
                    }
                }
                // Points outside the kit's list bring its warning, until
                // ignored on this visit (62c144).
                if wanted > before
                    && !page.ignored
                    && let Some(warning) = self.rules.kit_rogue_restriction(&self.draft, skill)
                {
                    self.show(Prompt::ignorable(
                        tr("Kit Requirements"),
                        &warning,
                        std::sync::Arc::new(move |w: &mut Wizard| {
                            if let Page::Rogue(page) = w.page_mut() {
                                page.ignored = true;
                                page.points[skill] = wanted;
                            }
                            w.refresh_rogue();
                        }),
                    ));
                    self.refresh_rogue();
                    return;
                }
                if let Page::Rogue(page) = self.page_mut() {
                    page.points = points;
                }
                self.refresh_rogue();
            }
            Input::RogueBonus(i) => {
                if let Some((_, names)) = self.rules.kit_rogue_bonus(&self.draft)
                    && let Some(name) = names.get(i)
                {
                    self.draft.rogue_bonus = Some(name.clone());
                    self.refresh_rogue();
                }
            }
            Input::RogueChosen(i, on) => {
                let Some((count, _, names)) = self.rules.kit_chosen_skills(&self.draft) else {
                    return;
                };
                let Some(name) = names.get(i) else {
                    return;
                };
                let chosen = &self.draft.rogue_chosen;
                // No more than the count may be ticked.
                if on && !chosen.contains(name) && chosen.len() >= count {
                    return;
                }
                // In the list's order.
                self.draft.rogue_chosen = names
                    .iter()
                    .filter(|n| if *n == name { on } else { chosen.contains(n) })
                    .cloned()
                    .collect();
                self.refresh_rogue();
            }
            Input::Next if page.ok => {
                self.draft.rogue_points = self.rogue_probe(page.points).rogue_points;
                self.after_rogue_skills();
            }
            _ => {}
        }
    }
}
