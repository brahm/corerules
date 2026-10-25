//! The editor's Abilities page: the Expansion's Race Abilities and Class
//! Abilities pages of Update Character, with its Schools and Spheres
//! dialogs as groups of each class. Each group ticks what the character
//! has among what it may take, with what the kit tells about the schools
//! and spheres.

use cr_formats::character::Character;
use cr_i18n::tr;
use cr_store::abilities::{SchoolRule, SphereRule};

use super::{EditInput, Editor};
use crate::create::{ability_rows, cost_text};

/// A group of the Abilities page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AbilityGroup {
    /// Skills & Powers: the racial abilities bought on top of the package.
    Racial,
    /// A class's abilities, by its slot.
    Class(usize),
    Schools(usize),
    MinorSpheres(usize),
    MajorSpheres(usize),
}

/// A choice to tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub title: String,
    /// What it costs.
    pub subtitle: String,
    pub on: bool,
    pub enabled: bool,
}

/// A group of choices to tick, as the page shows it: an [`AbilityGroup`]
/// or one of the Spells page's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checks<G = AbilityGroup> {
    pub group: G,
    pub title: String,
    pub description: String,
    pub checks: Vec<Check>,
}

/// `offered` and the names of `taken` it lacks, in order.
fn with_taken(mut offered: Vec<String>, taken: &[String]) -> Vec<String> {
    for name in taken {
        if !offered.contains(name) {
            offered.push(name.clone());
        }
    }
    offered
}

impl Editor {
    /// The subraces the race page offers.
    pub fn subraces(&self) -> Vec<String> {
        self.needs.abilities.subraces(&self.original)
    }

    pub fn subrace_index(&self) -> Option<usize> {
        self.subraces()
            .iter()
            .position(|s| *s == self.abilities.subrace)
    }

    /// The abilities the subrace as chosen gives, "None" when it gives
    /// none.
    pub fn package(&self) -> Vec<String> {
        let mut names = self
            .needs
            .abilities
            .package(&self.original.data.race, &self.abilities.subrace);
        if names.is_empty() {
            names.push(tr("None").into());
        }
        names
    }

    /// What the page says about the subrace.
    pub fn subrace_description(&self) -> &'static str {
        if self.original.skills_and_powers() {
            tr(
                "A new subrace brings its package and drops the racial abilities bought on top \
                 of the old one, refunding their points.",
            )
        } else {
            ""
        }
    }

    /// The names a group offers to tick, those the character has first
    /// among them.
    fn group_names(&self, group: AbilityGroup) -> Vec<String> {
        let rules = &self.needs.abilities;
        let classes = &self.original.data.classes.names;
        let a = &self.abilities;
        match group {
            AbilityGroup::Racial => {
                let package = rules.package(&self.original.data.race, &a.subrace);
                rules
                    .racial_choices(&self.original)
                    .into_iter()
                    .map(|c| c.0)
                    .filter(|n| !package.contains(n))
                    .collect()
            }
            AbilityGroup::Class(slot) => with_taken(
                rules.class_choices(&classes[slot]),
                &a.classes[slot].abilities,
            ),
            AbilityGroup::Schools(slot) => with_taken(
                rules.school_choices(&classes[slot]),
                &a.classes[slot].schools,
            ),
            AbilityGroup::MinorSpheres(slot) => {
                with_taken(rules.sphere_choices(), &a.classes[slot].minor_spheres)
            }
            AbilityGroup::MajorSpheres(slot) => {
                with_taken(rules.sphere_choices(), &a.classes[slot].major_spheres)
            }
        }
    }

    /// The list a group ticks.
    fn group_list(&mut self, group: AbilityGroup) -> Option<&mut Vec<String>> {
        let a = &mut self.abilities;
        Some(match group {
            AbilityGroup::Racial => &mut a.racial,
            AbilityGroup::Class(slot) => &mut a.classes.get_mut(slot)?.abilities,
            AbilityGroup::Schools(slot) => &mut a.classes.get_mut(slot)?.schools,
            AbilityGroup::MinorSpheres(slot) => &mut a.classes.get_mut(slot)?.minor_spheres,
            AbilityGroup::MajorSpheres(slot) => &mut a.classes.get_mut(slot)?.major_spheres,
        })
    }

    /// The groups of the page, in order: the racial abilities with Skills
    /// & Powers, then each class's abilities, schools and spheres.
    pub fn ability_groups(&self) -> Vec<Checks> {
        let rules = &self.needs.abilities;
        let character = &self.original;
        let skills_and_powers = character.skills_and_powers();
        let a = &self.abilities;
        let mut groups = Vec::new();
        if skills_and_powers {
            let offered = self.group_names(AbilityGroup::Racial);
            let choices: Vec<(String, i32)> = rules
                .racial_choices(character)
                .into_iter()
                .filter(|(n, _)| offered.contains(n))
                .map(|(n, c)| (n, c as i32))
                .collect();
            let checks = ability_rows(&choices)
                .into_iter()
                .zip(&choices)
                .map(|((title, subtitle), (name, _))| Check {
                    title,
                    subtitle,
                    on: a.racial.contains(name),
                    enabled: true,
                })
                .collect();
            groups.push(Checks {
                group: AbilityGroup::Racial,
                title: tr("Racial Abilities Bought").into(),
                description: String::new(),
                checks,
            });
        }
        for (slot, class) in character
            .data
            .classes
            .names
            .iter()
            .enumerate()
            .take(a.classes.len())
        {
            let taken = &a.classes[slot];
            let names = self.group_names(AbilityGroup::Class(slot));
            let choices: Vec<(String, i32)> = names
                .iter()
                .map(|n| (n.clone(), rules.class_ability_cost(n) as i32))
                .collect();
            groups.push(Checks {
                group: AbilityGroup::Class(slot),
                title: tr!("Class Abilities ({class})", class = class.as_str()),
                description: String::new(),
                checks: ability_rows(&choices)
                    .into_iter()
                    .zip(&names)
                    .map(|((title, subtitle), name)| Check {
                        title,
                        subtitle,
                        on: taken.abilities.contains(name),
                        enabled: true,
                    })
                    .collect(),
            });
            // Core Rules characters take schools and spheres for nothing.
            let cost = |points: u32| {
                if skills_and_powers {
                    cost_text(points.into())
                } else {
                    String::new()
                }
            };
            if let Some(rule) = rules.school_rule(class) {
                let single = rule == SchoolRule::Single;
                groups.push(Checks {
                    group: AbilityGroup::Schools(slot),
                    title: tr!("Schools ({class})", class = class.as_str()),
                    description: if single {
                        tr("This class may have a single school.").into()
                    } else {
                        String::new()
                    },
                    checks: self
                        .group_names(AbilityGroup::Schools(slot))
                        .into_iter()
                        .map(|name| {
                            let on = taken.schools.contains(&name);
                            Check {
                                subtitle: cost(rules.school_cost(class, &name)),
                                enabled: on || !single || taken.schools.is_empty(),
                                title: name,
                                on,
                            }
                        })
                        .collect(),
                });
            }
            let spheres = |group: AbilityGroup, major: bool, single: bool| {
                let (own, other) = if major {
                    (&taken.major_spheres, &taken.minor_spheres)
                } else {
                    (&taken.minor_spheres, &taken.major_spheres)
                };
                self.group_names(group)
                    .into_iter()
                    .map(|name| {
                        let on = own.contains(&name);
                        let (minor, full) = rules.sphere_costs(class, &name);
                        // A sphere is either minor or major; a mage has one.
                        let room = !single || (own.is_empty() && other.is_empty());
                        Check {
                            subtitle: cost(if major { full } else { minor }),
                            enabled: on || (!other.contains(&name) && room),
                            title: name,
                            on,
                        }
                    })
                    .collect::<Vec<Check>>()
            };
            match rules.sphere_rule(class) {
                Some(SphereRule::Full) => {
                    groups.push(Checks {
                        group: AbilityGroup::MinorSpheres(slot),
                        title: tr!("Minor Spheres ({class})", class = class.as_str()),
                        description: String::new(),
                        checks: spheres(AbilityGroup::MinorSpheres(slot), false, false),
                    });
                    groups.push(Checks {
                        group: AbilityGroup::MajorSpheres(slot),
                        title: tr!("Major Spheres ({class})", class = class.as_str()),
                        description: String::new(),
                        checks: spheres(AbilityGroup::MajorSpheres(slot), true, false),
                    });
                }
                Some(SphereRule::Single) => groups.push(Checks {
                    group: AbilityGroup::MajorSpheres(slot),
                    title: tr!("Sphere ({class})", class = class.as_str()),
                    description: tr("This class may buy major access to a single sphere.").into(),
                    checks: spheres(AbilityGroup::MajorSpheres(slot), true, true),
                }),
                None => {}
            }
        }
        // What the kit tells about the schools and spheres, as the
        // original's Schools and Spheres pages do.
        for group in &mut groups {
            let taken = group.checks.iter().filter(|c| c.on).count();
            let notices = self.kit_notices(group.group, taken);
            if !notices.is_empty() {
                let text: Vec<&str> = Some(group.description.as_str())
                    .filter(|d| !d.is_empty())
                    .into_iter()
                    .chain(notices.iter().map(String::as_str))
                    .collect();
                group.description = text.join("\n\n");
            }
        }
        groups
    }

    /// What the kit tells about a group's schools, or its minor or major
    /// spheres with `taken` of them.
    fn kit_notices(&self, group: AbilityGroup, taken: usize) -> Vec<String> {
        let rules = &self.needs.kit_magic;
        let (kit, magic) = (self.details.kit.as_str(), &self.kit_magic);
        match group {
            AbilityGroup::Schools(_) => rules.schools_notice(kit, magic).into_iter().collect(),
            AbilityGroup::MinorSpheres(_) => rules.spheres_notice(kit, magic, false, taken),
            AbilityGroup::MajorSpheres(_) => rules.spheres_notice(kit, magic, true, taken),
            AbilityGroup::Racial | AbilityGroup::Class(_) => Vec::new(),
        }
    }

    /// The character points left after the abilities, the combat choices
    /// and, with Skills & Powers, the nonweapon proficiencies as edited,
    /// which spend the same points; negative when they cost more than the
    /// character has.
    pub fn points_left(&self) -> i64 {
        i64::from(self.original.data.unknown_0x1778[0])
            - self.needs.abilities.spent(&self.original, &self.abilities)
            - self.needs.combat.spent(&self.original, &self.combat)
            - self.nonweapon_points()
    }

    /// What the nonweapon proficiencies as edited spend of the character
    /// points beyond what they cost before, with Skills & Powers.
    pub(super) fn nonweapon_points(&self) -> i64 {
        if !self.original.skills_and_powers() {
            return 0;
        }
        let before: i64 = self
            .original
            .data
            .nonweapon_costs
            .iter()
            .map(|&c| i64::from(c))
            .sum();
        let now: i64 = self
            .nonweapon
            .entries
            .iter()
            .map(|e| i64::from(e.spent))
            .sum();
        now - before
    }

    /// `character` with what the other pages spend of the character
    /// points taken off its unspent ones, `own` being what the page asking
    /// spends: the store's rules for one page then see what is left to
    /// all of them.
    pub(super) fn sharing_points(&self, character: &Character, own: i64) -> Character {
        let spent = i64::from(self.original.data.unknown_0x1778[0]) - self.points_left();
        let mut shared = character.clone();
        let points = &mut shared.data.unknown_0x1778[0];
        *points = (i64::from(*points) - (spent - own)).max(0) as u32;
        shared
    }

    /// Why the abilities and combat choices cannot be saved, if they
    /// cannot.
    pub(super) fn points_problem(&self) -> Option<String> {
        let left = self.points_left();
        if left < 0 {
            return Some(tr!(
                "Not enough character points: the new abilities and combat choices need {more} \
                 more",
                more = -left
            ));
        }
        self.needs
            .abilities
            .problem(&self.original, &self.abilities)
            .or_else(|| self.needs.combat.problem(&self.original, &self.combat))
    }

    pub(super) fn abilities_input(&mut self, input: EditInput) {
        match input {
            EditInput::Subrace(i) => {
                let Some(subrace) = self.subraces().get(i).cloned() else {
                    return;
                };
                if subrace != self.abilities.subrace {
                    // The racial abilities bought on top of the old
                    // package go, refunded.
                    self.abilities.subrace = subrace;
                    self.abilities.racial.clear();
                }
            }
            EditInput::Tick(group, i, on) => {
                let Some(name) = self.group_names(group).get(i).cloned() else {
                    return;
                };
                let Some(list) = self.group_list(group) else {
                    return;
                };
                if on {
                    if !list.contains(&name) {
                        list.push(name);
                    }
                } else {
                    list.retain(|n| *n != name);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::editor;
    use super::*;

    #[test]
    fn a_class_ability_ticked_spends_points_and_untick_refunds_them() {
        let Some(mut e) = editor() else {
            return;
        };
        let before = e.points_left();
        let groups = e.ability_groups();
        let Some((group, i)) = groups.iter().find_map(|g| {
            matches!(g.group, AbilityGroup::Class(_))
                .then(|| g.checks.iter().position(|c| !c.on).map(|i| (g.group, i)))
                .flatten()
        }) else {
            return;
        };
        e.input(EditInput::Tick(group, i, true));
        assert!(e.changed());
        let ticked = e.ability_groups();
        let check = &ticked.iter().find(|g| g.group == group).unwrap().checks[i];
        assert!(check.on);
        e.input(EditInput::Tick(group, i, false));
        assert_eq!(e.points_left(), before);
        assert!(!e.changed());
    }

    #[test]
    fn a_new_subrace_drops_the_racial_abilities_bought() {
        let Some(mut e) = editor() else {
            return;
        };
        let subraces = e.subraces();
        let Some(other) = subraces.iter().position(|s| *s != e.abilities.subrace) else {
            return;
        };
        e.abilities.racial.push("Something".into());
        e.input(EditInput::Subrace(other));
        assert_eq!(e.abilities.subrace, subraces[other]);
        assert!(e.abilities.racial.is_empty());
    }
}
