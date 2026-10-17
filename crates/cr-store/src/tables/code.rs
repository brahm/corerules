//! The tables the originals work out in their own code rather than from a
//! table file (the helpers of 0x50c88e and 0x51d5fd), and the dice text
//! of the Roll Results dialog (0x519d97 and 0x52bf8d).
//!
//! Numbers, dice and thresholds are the code's; the text it prints (rows,
//! headings, footnotes) is read from the installation's program at the
//! addresses of [`super::strings`].

use cr_formats::tbl::split_lines;

use super::program::{cut, layout};
use super::strings as s;
use super::{Block, Code, Dice, DiceList, Run, Title, Txt, d, range};
use crate::treasure::Rand;
use cr_i18n::tr;

/// The dice patterns of the dice text, in the order they are replaced:
/// text, count, sides, plus.
const DICE_A: &[(&str, i32, i32, i32)] = &[
    ("1d6+4 x 10", 1, 6, 4),
    ("1d4+1", 1, 4, 1),
    ("2d4+2", 2, 4, 2),
    ("2d4+1", 2, 4, 1),
    ("1d6", 1, 6, 0),
    ("4d6", 4, 6, 0),
    ("1d3", 1, 3, 0),
    ("1d4", 1, 4, 0),
    ("2d6", 2, 6, 0),
    ("2d4", 2, 4, 0),
    ("2d8", 2, 8, 0),
    ("3d8", 3, 8, 0),
    ("4d8", 4, 8, 0),
    ("5d8", 5, 8, 0),
    ("6d8", 6, 8, 0),
    ("7d8", 7, 8, 0),
    ("2d4", 2, 4, 0),
    ("1d8", 1, 8, 0),
];
const DICE_B: &[(&str, i32, i32, i32)] = &[
    ("1d6+4 x 10", 1, 6, 4),
    ("1d4+1", 1, 4, 1),
    ("2d4+2", 2, 4, 2),
    ("2d4+1", 2, 4, 1),
    ("1d6", 1, 6, 0),
    ("4d6", 4, 6, 0),
    ("1d3", 1, 3, 0),
    ("1d4", 1, 4, 0),
    ("2d6", 2, 6, 0),
    ("2d4", 2, 4, 0),
    ("2d8", 2, 8, 0),
    ("3d8", 3, 8, 0),
    ("4d8", 4, 8, 0),
    ("5d8", 5, 8, 0),
    ("6d8", 6, 8, 0),
    ("7d8", 7, 8, 0),
    ("2d4", 2, 4, 0),
    ("3d6", 3, 6, 0),
    ("2d10", 2, 10, 0),
    ("3d4", 3, 4, 0),
    ("1d2", 1, 2, 0),
];

/// `s.Mid(start, n)` of MFC: bytes `start..start + n`, clamped.
fn mid(s: &str, start: usize, n: usize) -> &str {
    let end = start.saturating_add(n).min(s.len());
    s.get(start.min(s.len())..end).unwrap_or("")
}

/// The dice text (0x519d97 with 0x51a7c6; 0x52bf8d with 0x52cb6f): every
/// dice pattern of the list in the text is replaced with a roll, the
/// patterns in turn, each until it no longer occurs. A result of 1 makes
/// the unit after it singular ("1 round"); "wish spells" keeps a stray
/// "l" as in the original ("1 wish spellls"). The Expansion's first list
/// also knows "turns"; its padding for the High-Level Campaigns disease
/// table is left out, as columns here are aligned anyway.
///
/// The patterns match inside longer dice too ("1d2" in "1d20"), as they
/// do in the original.
pub(super) fn dice_text(rand: &mut Rand, text: &str, list: DiceList) -> String {
    let (patterns, turns) = match list {
        DiceList::A => (DICE_A, true),
        DiceList::B => (DICE_B, false),
    };
    let mut text = text.to_owned();
    for &(pattern, count, sides, plus) in patterns {
        while let Some(found) = text.find(pattern) {
            let mut value = 0i32;
            for _ in 0..count {
                value = value.wrapping_add(range(rand, 1, sides));
            }
            value = value.wrapping_add(plus);
            if pattern == "1d6+4 x 10" {
                value = value.wrapping_mul(10);
            }
            let mut out = format!("{}{value}", &text[..found]);
            let mut pos = found + pattern.len();
            if value == 1 {
                // (word, bytes kept from the space before it, bytes skipped)
                let mut units: Vec<(&str, usize, usize)> = vec![
                    ("rounds", 6, 7),
                    ("points", 6, 7),
                    ("hours", 5, 6),
                    ("days", 4, 5),
                    ("weeks", 5, 6),
                    ("wish spells", 11, 10),
                ];
                if turns {
                    units.push(("turns", 5, 6));
                }
                for (word, keep, skip) in units {
                    if mid(&text, pos + 1, word.len()) == word {
                        out.push_str(mid(&text, pos, keep));
                        pos += skip;
                        break;
                    }
                }
            }
            out.push_str(mid(&text, pos, usize::MAX));
            text = out;
        }
    }
    text
}

/// The name of a record after the dice text, split into its lines.
pub(super) fn text_lines(rand: &mut Rand, name: &str, list: Option<DiceList>) -> String {
    match list {
        Some(list) => split_lines(&dice_text(rand, name, list)).join("\n"),
        None => split_lines(name).join("\n"),
    }
}

/// Values of the turning undead table (0x5145da): a number to roll on
/// d20, or one of these.
const T: i32 = 1;
const D: i32 = 22;
const DS: i32 = 23;
const NO: i32 = 99;

/// PHB Table 61 as the code has it, one row per type of undead
/// ([`s::UNDEAD`]) and one value per entry of the level list. Some rows
/// differ from the book (Shadow 4th level 13 for 10; Wight, Ghast,
/// Wraith, Mummy and Spectre have a number repeated or left out), and
/// those of the code are kept.
const UNDEAD: [[i32; 12]; 13] = [
    [10, 7, 4, T, T, D, D, DS, DS, DS, DS, DS],
    [13, 10, 7, 4, T, T, D, D, DS, DS, DS, DS],
    [16, 13, 10, 7, 4, T, T, D, D, DS, DS, DS],
    [19, 16, 13, 13, 7, 4, T, T, D, D, DS, DS],
    [20, 19, 16, 16, 13, 7, 4, T, T, D, D, DS],
    [NO, 20, 19, 19, 16, 10, 7, 4, T, T, D, D],
    [NO, NO, 20, 20, 19, 13, 10, 7, 4, T, T, D],
    [NO, NO, NO, NO, 20, 16, 13, 10, 7, 4, T, T],
    [NO, NO, NO, NO, NO, 19, 16, 13, 10, 7, 4, T],
    [NO, NO, NO, NO, NO, 20, 19, 16, 13, 10, 7, 4],
    [NO, NO, NO, NO, NO, NO, 20, 19, 16, 13, 10, 7],
    [NO, NO, NO, NO, NO, NO, NO, 20, 19, 16, 13, 10],
    [NO, NO, NO, NO, NO, NO, NO, NO, 20, 19, 16, 13],
];

/// The value classes of DMG Table 85 by the upper end of their rolls, in
/// the order of [`s::GEMS`].
const GEM_CLASSES: [i32; 6] = [25, 50, 70, 90, 99, 100];

/// Initial funds by class group (0x514280): the dice, times 10.
const FUNDS: [Dice; 4] = [d(5, 4), d(1, 4).plus(1), d(2, 6), d(3, 6)];

impl Run<'_> {
    pub(super) fn code(&mut self, code: Code) {
        match code {
            Code::Disease => self.by_rank(s::DISEASE_HEAD, s::DISEASE_ROWS, s::DISEASE_NOTE),
            Code::Crush => self.by_rank(s::CRUSH_HEAD, s::CRUSH_ROWS, &[]),
            Code::Wounding => self.by_rank(s::WOUNDING_HEAD, s::WOUNDING_ROWS, s::WOUNDING_NOTE),
            Code::TurnUndead => self.turn_undead(),
            Code::Funds => self.funds(),
            Code::SpellFailure => self.threshold(&[95, 85, 75, 65, 50], s::SPELL_FAILURE),
            Code::ScrollUse => {
                self.threshold(&[10, 20, 30, 40, 50, 60, 70, 80], s::SCROLL_USE);
            }
            Code::Languages => self.languages(),
            Code::FieldSearch => self.field_search(),
            Code::Purchase => self.purchase(),
            Code::WholeBody => self.whole_body(),
            Code::Severity => self.severity(),
            Code::Treasure => {
                self.out.treasure = true;
                let title = self.title(Title::Entry, None);
                self.out.blocks.push(Block {
                    title,
                    notes: vec![
                        tr("The original opens the treasure generator here (Treasure menu).")
                            .into(),
                    ],
                    ..Default::default()
                });
            }
            Code::SpecificTrait => {
                let roll = range(self.rand, 1, 100);
                let header = self.head(s::TRAIT_HEAD, 2);
                let trait_ = self.txt(s::TRAITS[roll as usize]);
                self.out.blocks.push(Block {
                    header,
                    rows: vec![vec![roll.to_string(), trait_]],
                    ..Default::default()
                });
            }
            Code::Gems => self.gems(),
            Code::IounStones => self.ioun_stones(),
            Code::WildTalents => self.wild_talents(),
            Code::Powers => self.powers(),
            Code::InfiniteSpells => self.infinite_spells(),
            Code::PrayerBeads => self.prayer_beads(),
            Code::UsefulItems => self.useful_items(),
        }
    }

    /// Lines of the program cut into the columns of a heading.
    fn cells(&mut self, head: &[Txt], line: &str) -> Vec<String> {
        let lines = self.raws(head);
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        cut(line, &layout(&lines))
    }

    /// DM Option: High-Level Campaigns Tables 14, 16 and 17 (0x510d57,
    /// 0x5110a2, 0x51134b): the line for the rank with its dice rolled.
    fn by_rank(&mut self, head: &[Txt], rows: &[Txt], notes: &[Txt]) {
        let rank = self.param(0).min(rows.len() - 1);
        let line = self.raw(rows[rank]);
        let cells = self.cells(head, &line);
        // One text, as the original rolls the dice of the whole line in
        // the order of its pattern list.
        let rolled = dice_text(self.rand, &cells.join("\t"), DiceList::A);
        let row: Vec<String> = rolled.split('\t').map(|c| c.trim().to_owned()).collect();
        let header = self.head(head, row.len());
        let notes = self.notes(notes);
        let title = self.title(Title::Entry, None);
        self.out.blocks.push(Block {
            title,
            header,
            rows: vec![row],
            notes,
            ..Default::default()
        });
    }

    /// Turning undead (0x5145da): one d20 for every type of undead.
    fn turn_undead(&mut self) {
        let level = self.param(1).min(11);
        let roll = range(self.rand, 1, 20);
        let mut rows = Vec::new();
        for (i, values) in UNDEAD.iter().enumerate() {
            let value = values[level];
            // 0x51608a: the number turned.
            let mut turned = 0;
            if roll >= value && value <= 20 {
                turned = range(self.rand, 1, 6) + range(self.rand, 1, 6);
            }
            if value == DS {
                turned = range(self.rand, 1, 4) + range(self.rand, 1, 4);
            }
            // 0x5163bb: the number dispelled.
            let mut dispelled = 0;
            if value == D || value == DS {
                dispelled = range(self.rand, 1, 6) + range(self.rand, 1, 6);
            }
            let cell = |n: i32| {
                if value == NO {
                    "-".to_owned()
                } else {
                    n.to_string()
                }
            };
            let name = self.txt(s::UNDEAD[i]);
            rows.push(vec![roll.to_string(), name, cell(turned), cell(dispelled)]);
        }
        let header = self.head(s::TURN_HEAD, 4);
        let notes = self.notes(s::TURN_NOTE);
        let title = self.title(Title::Entry, None);
        self.out.blocks.push(Block {
            title,
            header,
            rows,
            notes,
            ..Default::default()
        });
    }

    /// PHB Table 43 (0x514280): the dice of the class group, times 10;
    /// the priest's note when the heading names a priest.
    fn funds(&mut self) {
        let group = self.param(0).min(3);
        let dice = FUNDS[group];
        let mut sum = 0;
        for _ in 0..dice.count {
            sum += range(self.rand, 1, dice.sides);
        }
        let gold = (sum + dice.plus) * 10;
        let heading = self.raw(s::FUNDS_FOR) + &self.txt(s::CLASS_GROUPS[group]);
        let notes = if group == 3 {
            self.notes(s::FUNDS_NOTE)
        } else {
            Vec::new()
        };
        let title = self.title(Title::Entry, None);
        self.out.blocks.push(Block {
            title,
            header: vec![heading.trim().to_owned()],
            rows: vec![vec![gold.to_string()]],
            notes,
            ..Default::default()
        });
    }

    /// A d100 roll against the chance the first parameter picks: DMG
    /// Table 112 (0x518ead), S&P Tables 25 and 31 (0x5190d3); `results`
    /// holds the line for a roll within the chance, then the other.
    fn threshold(&mut self, chances: &[i32], results: &[Txt]) {
        let chance = chances[self.param(0).min(chances.len() - 1)];
        let roll = range(self.rand, 1, 100);
        let result = self.txt(results[usize::from(roll > chance)]);
        let header = self.head(s::RESULT_HEAD, 2);
        let title = self.title(Title::Entry, None);
        self.out.blocks.push(Block {
            title,
            header,
            rows: vec![vec![roll.to_string(), result]],
            ..Default::default()
        });
    }

    /// DMG Table 118 (0x52b11c).
    fn languages(&mut self) {
        let roll = range(self.rand, 1, 100);
        let count = match roll {
            ..=40 => "1",
            41..=70 => "2",
            71..=85 => "3",
            86..=95 => "4",
            96..=99 => "5",
            _ => "6*",
        };
        let notes = if roll == 100 {
            self.notes(s::LANGUAGES_NOTE)
        } else {
            Vec::new()
        };
        let header = self.head(s::LANGUAGES_HEAD, 2);
        let title = self.title(Title::Entry, None);
        self.out.blocks.push(Block {
            title,
            header,
            rows: vec![vec![roll.to_string(), count.into()]],
            notes,
            ..Default::default()
        });
    }

    /// S&M Table 14 (0x519355): base turns by rarity, a d100 against the
    /// chance each of them, then the better chance each turn after. The
    /// first turn after the base is counted twice, as in the original.
    fn field_search(&mut self) {
        let (turns, chance, more) = match self.param(0) {
            0 => (range(self.rand, 1, 6), 75, 10),
            1 => (
                range(self.rand, 1, 6) + range(self.rand, 1, 6) + range(self.rand, 1, 6),
                50,
                5,
            ),
            _ => (range(self.rand, 1, 4) + 1, 25, 2),
        };
        let mut turn = 1;
        let mut found = false;
        while turn <= turns {
            if range(self.rand, 1, 100) <= chance {
                found = true;
                break;
            }
            turn += 1;
        }
        while !found {
            turn += 1;
            found = range(self.rand, 1, 100) <= chance + more;
        }
        // The program's words around the turns, plural when more than one.
        let [found_in, unit, plural, stop] = [0, 1, 2, 3].map(|i| self.raw(s::FOUND_IN[i]));
        let plural = if turn > 1 { plural } else { String::new() };
        let title = self.title(Title::Entry, None);
        self.out.blocks.push(Block {
            title,
            notes: vec![format!("{found_in}{turn}{unit}{plural}{stop}")],
            ..Default::default()
        });
    }

    /// S&M Table 15 (0x519575): one d100 against the chance in a village,
    /// a town, a city and a major city.
    fn purchase(&mut self) {
        let chances = match self.param(0) {
            0 => [80, 90, 100, 100],
            1 => [50, 60, 70, 80],
            _ => [20, 30, 40, 50],
        };
        let roll = range(self.rand, 1, 100);
        let notes = (0..4)
            .map(|i| {
                let t = if roll <= chances[i] {
                    s::CAN[i]
                } else {
                    s::CANNOT[i]
                };
                self.txt(t)
            })
            .collect();
        let header = self.head(s::PURCHASE_HEAD, 1);
        let title = self.title(Title::Entry, None);
        self.out.blocks.push(Block {
            title,
            header,
            rows: vec![vec![roll.to_string()]],
            notes,
            ..Default::default()
        });
    }

    /// S&M Table 47 (0x51983f): the area and how many locations it hits.
    fn whole_body(&mut self) {
        let index = self.param(0).min(3);
        let count = match index {
            0 => None,
            1 => Some(range(self.rand, 1, 3)),
            2 => Some(range(self.rand, 1, 4)),
            _ => Some(range(self.rand, 1, 6) + 1),
        };
        let area = self.raw(s::AREAS[index]);
        let row = match count {
            // The first line holds both columns.
            None => self.cells(s::WHOLE_HEAD, &area),
            Some(n) => {
                let size = self.txt(s::AREA_SIZES[index - 1]);
                vec![area.trim().to_owned(), format!("{n}{size}")]
            }
        };
        let header = self.head(s::WHOLE_HEAD, 2);
        let title = self.title(Title::Entry, None);
        self.out.blocks.push(Block {
            title,
            header,
            rows: vec![row],
            ..Default::default()
        });
    }

    /// S&M Table 48 (0x519b0a): the severity dice of the damage potential;
    /// the row repeats the option chosen.
    fn severity(&mut self) {
        const DICE: [Dice; 4] = [d(1, 6), d(2, 4), d(2, 6), d(2, 8)];
        let index = self.param(0).min(3);
        let dice = DICE[index];
        let mut roll = 0;
        for _ in 0..dice.count {
            roll += range(self.rand, 1, dice.sides);
        }
        let label = self.txt(s::MAX_DAMAGE[index]);
        let header = self.head(s::SEVERITY_HEAD, 2);
        let title = self.title(Title::Entry, None);
        self.out.blocks.push(Block {
            title,
            header,
            rows: vec![vec![label, roll.to_string()]],
            ..Default::default()
        });
    }

    /// DMG Table 85 (0x52d0f4): the value class from the table, then one
    /// of the class's gems.
    fn gems(&mut self) {
        let Some(table) = self.rolls.table(20085) else {
            return self.missing(20085, Title::Table(85));
        };
        let roll = self.session.roll(self.rand, d(1, 100));
        let name = table
            .lookup(roll)
            .and_then(|r| r.name.clone())
            .unwrap_or_default();
        let (value, class) = match name.find("  ") {
            Some(i) => (name[..i].trim(), name[i..].trim()),
            None => (name.trim(), ""),
        };
        let gem = match GEM_CLASSES.iter().position(|hi| roll <= *hi) {
            Some(class) => {
                let names = s::GEMS[class];
                let pick = range(self.rand, 0, names.len() as i32 - 1) as usize;
                self.txt(names[pick])
            }
            None => String::new(),
        };
        let header = self.head(s::GEM_HEAD, 4);
        let title = self.title(Title::Table(85), Some(table));
        self.out.blocks.push(Block {
            title,
            header,
            rows: vec![vec![roll.to_string(), value.into(), class.into(), gem]],
            ..Default::default()
        });
    }

    /// Ioun stones (0x52dbc9): 1d10 stones of d20; a second stone of a
    /// kind and 15 to 20 are dead stones. Each kind prints its lines; the
    /// last of the 2d6 spell levels is put into its line.
    fn ioun_stones(&mut self) {
        let count = range(self.rand, 1, 10);
        let mut have = [false; 14];
        let mut dead = 0;
        for _ in 0..count {
            let r = range(self.rand, 1, 20) as usize;
            if (1..=14).contains(&r) && !have[r - 1] {
                have[r - 1] = true;
            } else {
                dead += 1;
            }
        }
        let mut rows: Vec<Vec<String>> = Vec::new();
        for (i, lines) in s::IOUN.iter().enumerate() {
            if !have[i] {
                continue;
            }
            let texts: Vec<String> = if i == 12 {
                let levels = range(self.rand, 1, 6) + range(self.rand, 1, 6);
                vec![self.raw(lines[0]) + &levels.to_string() + &self.raw(lines[1])]
            } else {
                self.raws(lines)
            };
            for text in texts {
                let cells = self.cells(s::IOUN_HEAD, &text);
                // A line with only the last column continues the row.
                match rows.last_mut() {
                    Some(last)
                        if cells.len() > 1
                            && cells[..cells.len() - 1].iter().all(String::is_empty) =>
                    {
                        if let Some(cell) = last.last_mut() {
                            cell.push('\n');
                            cell.push_str(&cells[cells.len() - 1]);
                        }
                    }
                    _ => rows.push(cells),
                }
            }
        }
        if dead > 0 {
            let line = dead.to_string() + &self.raw(s::IOUN_DEAD);
            rows.push(self.cells(s::IOUN_HEAD, &line));
        }
        let mut notes = Vec::new();
        if have[..7].iter().any(|h| *h) {
            notes.extend(self.notes(s::IOUN_MAX));
        }
        if have[10] {
            notes.extend(self.notes(s::IOUN_ABSORB_4));
        }
        if have[11] {
            notes.extend(self.notes(s::IOUN_ABSORB_8));
        }
        let title = count.to_string() + &self.raw(s::IOUN_TITLE);
        let header = self.head(s::IOUN_HEAD, 3);
        self.out.blocks.push(Block {
            title: Some(title),
            header,
            rows,
            notes,
            ..Default::default()
        });
    }

    /// One roll on a table with the session's rule, as the dialog's
    /// Output does, in a block of its own.
    fn output(&mut self, id: i32, dice: Dice, times: i32, title: Title, head: &[Txt]) -> Vec<i32> {
        let Some(table) = self.rolls.table(id) else {
            self.missing(id, title);
            return Vec::new();
        };
        let rolls: Vec<i32> = (0..times)
            .map(|_| self.session.roll(self.rand, dice))
            .collect();
        let rows = rolls
            .iter()
            .map(|&r| {
                let text = table
                    .lookup(r)
                    .map(|e| e.lines().join("\n"))
                    .unwrap_or_default();
                vec![r.to_string(), text]
            })
            .collect();
        let header = self.head(head, 2);
        let title = self.title(title, Some(table));
        self.out.blocks.push(Block {
            title,
            header,
            rows,
            ..Default::default()
        });
        rolls
    }

    /// S&P Table 83 (0x522489): the rows 97 to 100 roll again.
    fn wild_talents(&mut self) {
        let devotion = s::WILD_DEVOTION_HEAD;
        let science = s::WILD_SCIENCE_HEAD;
        let roll = self
            .output(31083, d(1, 100), 1, Title::Table(83), devotion)
            .first()
            .copied()
            .unwrap_or(0);
        match roll {
            99 => {
                let s = self.output(32083, d(1, 100), 1, Title::None, science);
                if s.first().is_some_and(|&s| s >= 96) {
                    self.output(32083, d(1, 95), 2, Title::None, science);
                }
            }
            97 | 98 => {
                self.output(31083, d(1, 96), 2, Title::None, devotion);
            }
            100 => {
                self.output(31083, d(1, 96), 1, Title::None, devotion);
                let s = self.output(32083, d(1, 100), 1, Title::None, science);
                if s.first().is_some_and(|&s| s >= 96) {
                    self.output(32083, d(1, 95), 2, Title::None, science);
                }
            }
            _ => {}
        }
    }

    /// DMG Table 116 (0x521d95): 95 to 97 roll twice ignoring them; 100
    /// adds a special purpose power (Table 117) with its dice rolled.
    fn powers(&mut self) {
        let roll = self
            .output(20116, d(1, 100), 1, Title::Table(116), s::POWER_HEAD)
            .first()
            .copied()
            .unwrap_or(0);
        if (95..=97).contains(&roll) {
            self.output(20116, d(1, 88), 2, Title::None, s::POWER_HEAD);
        } else if roll == 100 {
            let spec = super::RollSpec::new(
                22117,
                d(1, 100),
                Title::None,
                s::SPECIAL_POWER_HEAD,
                &[super::Col::Name],
            )
            .dice_text(DiceList::B);
            self.roll(&spec, None);
        }
    }

    /// Book of Infinite Spells (0x524767, Output 0x529707 for 90018):
    /// 1d8+22 pages, each with a spell level (0x52a231) and a spell
    /// (0x52a35e). Pages without a spell print nothing, as in the
    /// original.
    fn infinite_spells(&mut self) {
        let Some(table) = self.rolls.table(90018) else {
            return self.missing(90018, Title::Header);
        };
        let pages = range(self.rand, 1, 8) + 22;
        let rolls: Vec<i32> = (0..pages)
            .map(|_| self.session.roll(self.rand, d(1, 100)))
            .collect();
        let mut rows = Vec::new();
        for roll in rolls {
            let name = table
                .lookup(roll)
                .and_then(|r| r.name.clone())
                .unwrap_or_default();
            // 0x52a231 and 0x52a35e test the page's text for these words.
            let priest = name.contains("Priest");
            let wizard = name.contains("Wizard");
            let mut level = 0;
            if priest {
                level = range(self.rand, 1, 10);
                if level >= 8 {
                    level = range(self.rand, 1, 6);
                }
            }
            if wizard {
                level = range(self.rand, 1, 12);
                if level >= 10 {
                    level = range(self.rand, 1, 8);
                }
            }
            if !priest && !wizard {
                continue;
            }
            let Some(spell) = self.rolls.random_spell(self.rand, !priest, level) else {
                continue;
            };
            let level = if level == 0 {
                "-".to_owned()
            } else {
                level.to_string()
            };
            rows.push(vec![roll.to_string(), name.trim().to_owned(), level, spell]);
        }
        let header = self.head(s::PAGES_HEAD, 4);
        let subtitle = pages.to_string() + &self.raw(s::PAGES);
        let title = self.title(Title::Header, Some(table));
        self.out.blocks.push(Block {
            title,
            subtitle: Some(subtitle.trim().to_owned()),
            header,
            rows,
            ..Default::default()
        });
    }

    /// Prayer beads (0x524aef): the beads, then the special ones.
    fn prayer_beads(&mut self) {
        let total = range(self.rand, 1, 6) + 24;
        let fancy = total - (f64::from(total) * 0.6) as i32;
        let special = range(self.rand, 1, 4) + 2;
        // The first line counts all the beads, as the original prints it.
        let notes = [total, fancy, special]
            .into_iter()
            .zip(s::BEADS)
            .map(|(n, t)| (n.to_string() + &self.raw(*t)).trim().to_owned())
            .collect();
        self.out.blocks.push(Block {
            title: self.rolls.table(90034).map(|t| t.title.trim().to_owned()),
            notes,
            ..Default::default()
        });
        self.output(90034, d(1, 20), special, Title::None, s::BEAD_HEAD);
    }

    /// Robe of Useful Items (0x525039): 4d4 extra patches; "roll twice
    /// more" adds two rolls of d96 at the end.
    fn useful_items(&mut self) {
        let Some(table) = self.rolls.table(90039) else {
            return self.missing(90039, Title::Header);
        };
        let patches: i32 = (0..4).map(|_| range(self.rand, 1, 4)).sum();
        let rolls = self.output(90039, d(1, 100), patches, Title::None, s::PATCH_HEAD);
        let more: i32 = rolls
            .iter()
            .filter(|&&r| table.lookup(r).is_some_and(|e| e.v4 == 2))
            .count() as i32
            * 2;
        let subtitle = patches.to_string() + &self.raw(s::PATCHES);
        if let Some(block) = self.out.blocks.last_mut() {
            block.title = Some(table.title.trim().to_owned());
            block.subtitle = Some(subtitle.trim().to_owned());
        }
        if more > 0 {
            self.output(90039, d(1, 96), more, Title::None, s::PATCH_HEAD);
        }
    }
}
