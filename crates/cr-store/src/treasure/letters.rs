//! Treasure letters: the 26 counts the roller takes, and a monster's
//! treasure as the Monster screen shows it ("S, T (H)").

use cr_formats::monster::Monster;

use super::{Counts, LETTERS};

/// A monster's treasure types: the letters written outside parentheses
/// (`CMonsterOb` +0x2c4) and inside them (+0x25c), each a count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MonsterTreasure {
    pub outside: [i32; 26],
    pub inside: [i32; 26],
}

impl MonsterTreasure {
    pub fn of(monster: &Monster) -> Self {
        Self {
            outside: monster.unknown_0x25c.map(|(_, outside)| outside),
            inside: monster.unknown_0x25c.map(|(inside, _)| inside),
        }
    }

    /// Reads the text [`Display`](std::fmt::Display) writes: letters with
    /// an optional `x<count>`, separated by commas or spaces, then the
    /// in-parentheses letters. Repeated letters add up; case and an empty
    /// or "None" text are accepted.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        let (outside, inside) = match text.split_once('(') {
            Some((outside, rest)) => {
                let inside = rest
                    .strip_suffix(')')
                    .or_else(|| rest.trim_end().strip_suffix(')'))
                    .ok_or_else(|| format!("{text}: missing )"))?;
                (outside, inside)
            }
            None => (text, ""),
        };
        Ok(Self {
            outside: parse_counts(outside)?,
            inside: parse_counts(inside)?,
        })
    }

    /// The counts an encounter rolls for `number` of these monsters: only
    /// the outside letters, each times the number appearing (0x63de14).
    pub fn encounter_counts(&self, number: u32) -> Counts {
        self.outside.map(|n| (n.max(0) as u32).wrapping_mul(number))
    }
}

/// The counts of an encounter: the template's own letters plus each
/// monster's outside letters times its number appearing (0x63de14).
pub fn encounter_counts(template: &Counts, monsters: &[(MonsterTreasure, u32)]) -> Counts {
    let mut counts = *template;
    for (treasure, number) in monsters {
        for (c, n) in counts.iter_mut().zip(treasure.encounter_counts(*number)) {
            *c = c.wrapping_add(n);
        }
    }
    counts
}

/// As the Monster screen writes it (0x67e7f5): "W (G)"; the outside
/// letters keep a trailing space when there are no inside ones.
impl std::fmt::Display for MonsterTreasure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let outside = format_counts(&self.outside);
        if !outside.is_empty() {
            write!(f, "{outside} ")?;
        }
        if self.inside.iter().fold(0i32, |a, &n| a.wrapping_add(n)) != 0 {
            write!(f, "({})", format_counts(&self.inside))?;
        }
        Ok(())
    }
}

fn format_counts(counts: &[i32; 26]) -> String {
    let mut parts = Vec::new();
    for (letter, &n) in LETTERS.iter().zip(counts) {
        match n {
            0 => {}
            1 => parts.push(letter.to_string()),
            _ => parts.push(format!("{letter}x{n}")),
        }
    }
    parts.join(", ")
}

/// Letters such as "A B B Q", "Bx2, Q" or "a,b": the 26 counts.
pub fn parse_letters(text: &str) -> Result<Counts, String> {
    Ok(parse_counts(text)?.map(|n| n.max(0) as u32))
}

fn parse_counts(text: &str) -> Result<[i32; 26], String> {
    let mut counts = [0i32; 26];
    let mut last: Option<usize> = None;
    let text = text.trim();
    if text.eq_ignore_ascii_case("none") {
        return Ok(counts);
    }
    for token in text.split([',', ' ', '\t']).filter(|t| !t.is_empty()) {
        let bytes = token.as_bytes();
        // "x3" alone multiplies the letter before it ("A x3").
        let (letter, rest) = if (bytes[0] == b'x' || bytes[0] == b'X')
            && bytes.len() > 1
            && bytes[1].is_ascii_digit()
        {
            let letter = last.ok_or_else(|| format!("{token}: no letter before it"))?;
            counts[letter] -= 1;
            (letter, &token[1..])
        } else {
            let c = bytes[0].to_ascii_uppercase();
            if !c.is_ascii_uppercase() {
                return Err(format!("{token}: not a treasure letter"));
            }
            let rest = &token[1..];
            let rest =
                rest.strip_prefix(['x', 'X'])
                    .unwrap_or(if rest.is_empty() { "1" } else { rest });
            ((c - b'A') as usize, rest)
        };
        let n: i32 = rest
            .parse()
            .map_err(|_| format!("{token}: not a treasure letter"))?;
        counts[letter] += n;
        last = Some(letter);
    }
    Ok(counts)
}
