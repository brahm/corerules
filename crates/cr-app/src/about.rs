//! What both apps' About show: the app's description, its newest release
//! notes, its credits and legal notice, and the original program's About
//! box and Credits window when the installation has them.

use cr_i18n::tr;
use cr_store::help::OriginalAbout;

/// The system the app runs on, which the description names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Linux,
    Windows,
    MacOs,
}

pub const DEVELOPER: &str = "Wagner Brahm";
pub const DEVELOPER_URL: &str = "https://wagnerbrahm.com.br";
/// The landing page; the repository is where issues go.
pub const WEBSITE: &str = "https://brahm.github.io/corerules/";
pub const ISSUES: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/issues");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The copyleft sign, by the author's choice.
pub const COPYRIGHT: &str = "🄯 2026 Wagner Brahm";
pub const ASSISTANT: &str = "Claude (Anthropic)";
pub const ASSISTANT_URL: &str = "https://www.anthropic.com/claude";

/// The README's description, one paragraph each.
pub fn description(system: System) -> [&'static str; 3] {
    [
        match system {
            System::Linux => tr(
                "Core Rules is a complete Linux version of AD&D Core Rules 2.0 and its Expansion \
                (TSR/Evermore, 1998-1999), the character manager and Dungeon Master's toolkit \
                for Advanced Dungeons & Dragons 2nd Edition. It is written in Rust with GTK 4 \
                and libadwaita and runs natively, with no Wine and no Windows.",
            ),
            System::Windows => tr(
                "Core Rules is a complete Windows 11 version of AD&D Core Rules 2.0 and its \
                Expansion (TSR/Evermore, 1998-1999), the character manager and Dungeon Master's \
                toolkit for Advanced Dungeons & Dragons 2nd Edition. It is written in Rust with \
                WinUI 3 and shares its code with the Linux version.",
            ),
            System::MacOs => tr(
                "Core Rules is a complete macOS version of AD&D Core Rules 2.0 and its \
                Expansion (TSR/Evermore, 1998-1999), the character manager and Dungeon Master's \
                toolkit for Advanced Dungeons & Dragons 2nd Edition. Its interface is written \
                in SwiftUI over the Rust code it shares with the Linux and Windows versions.",
            ),
        },
        tr(
            "Every screen of the original program is here: the Character Generator with \
            Create, Enter and Update Character, kits and Player's Option, the character \
            sheet, parties, the NPC, treasure and encounter generators, Table Rolls, editors \
            for every user database, DM Toolkit packages, the rule books and the help. The \
            app reads and writes the original's own data files, so characters, monsters, \
            encounters and packages made in either program open in the other.",
        ),
        tr(
            "This project contains no TSR or Wizards of the Coast content. The rules, books, \
            monsters, pictures and help all come from a copy of AD&D Core Rules 2.0 (and, for \
            everything the app creates and edits, the Expansion) that you already own.",
        ),
    ]
}

/// The legal notice on the original's content.
pub fn legal() -> &'static str {
    tr(
        "AD&D, Advanced Dungeons & Dragons and Core Rules are trademarks of Wizards of the \
         Coast. The rules, books, monsters, pictures and help of AD&D Core Rules 2.0 and its \
         Expansion are copyrighted by Wizards of the Coast and are not covered by this \
         app's license. This project is not affiliated with Wizards of the Coast and \
         contains none of their content: it needs the original discs, or a copy of them, \
         that you own.",
    )
}

/// The catalog's translator credits, one per line; English has none.
pub fn translators() -> Option<&'static str> {
    let credits = tr("translator-credits");
    (credits != "translator-credits").then_some(credits)
}

/// The app's AppStream metainfo, whose newest release About shows as
/// What's New.
pub const METAINFO: &str = include_str!("../../../data/io.github.brahm.CoreRules.metainfo.xml");

/// The description of the metainfo's newest release, the AppStream markup
/// GNOME's About window takes.
pub fn release_notes() -> Option<String> {
    let release = &METAINFO[METAINFO.find("<release ")?..];
    let start = release.find("<description>")? + "<description>".len();
    let end = release.find("</description>")?;
    let notes = release.get(start..end)?;
    Some(notes.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// The newest release notes as text: a line for each paragraph, and one
/// starting with "• " for each item of a list.
pub fn release_notes_text() -> Vec<String> {
    let Some(notes) = release_notes() else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    let mut rest = notes.as_str();
    while let Some(open) = rest.find('<') {
        let Some(close) = rest[open..].find('>') else {
            break;
        };
        let tag = &rest[open + 1..open + close];
        rest = &rest[open + close + 1..];
        let (prefix, end) = match tag {
            "p" => ("", "</p>"),
            "li" => ("• ", "</li>"),
            _ => continue,
        };
        let Some(at) = rest.find(end) else { break };
        lines.push(format!("{prefix}{}", unescape(rest[..at].trim())));
        rest = &rest[at + end.len()..];
    }
    lines
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// The original program's About box and Credits window.
pub struct Original {
    /// The About box, when the installation has its lines or its license:
    /// the product's name, the rest of what it says (version, copyright
    /// and developer), and its license.
    pub about: Option<OriginalBox>,
    /// The Credits window's title and lines.
    pub credits: Option<(String, Vec<String>)>,
}

pub struct OriginalBox {
    pub name: String,
    pub rest: String,
    pub license: Option<String>,
}

impl Original {
    pub fn of(original: &OriginalAbout) -> Self {
        // The product's name, then its version, copyright and developer.
        let split = original.lines.len().min(2);
        let (name, rest) = original.lines.split_at(split);
        let name = name.join(" ");
        let credits = (!original.credits.is_empty()).then(|| {
            // The product's name without the game's, which does not fit.
            let product = original.lines.get(1).unwrap_or(&name);
            (
                tr!("{product} (original program)", product = product),
                original.credits.clone(),
            )
        });
        let about = (!name.is_empty() || original.license.is_some()).then(|| OriginalBox {
            name: tr!("{product} (original program)", product = &name),
            rest: rest.join("\n"),
            license: original.license.clone(),
        });
        Self { about, credits }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// About's What's New is the metainfo's newest release, which has to
    /// be this version.
    #[test]
    fn newest_release_is_this_version() {
        let release = &METAINFO[METAINFO.find("<release ").unwrap()..];
        let version = VERSION.replace('-', "~");
        assert!(
            release.starts_with(&format!("<release version=\"{version}\"")),
            "the metainfo's newest release is not {version}"
        );
        let notes = release_notes().unwrap();
        assert!(
            notes.starts_with("<p>") && notes.ends_with("</p>"),
            "{notes}"
        );
        let text = release_notes_text();
        assert!(!text.is_empty());
        assert!(text.iter().all(|line| !line.contains('<')), "{text:?}");
    }

    #[test]
    fn the_original_names_its_product_then_the_rest() {
        let original = Original::of(&OriginalAbout {
            lines: vec![
                "AD&D".into(),
                "Core Rules 2.0".into(),
                "Version 2.0".into(),
                "© 1998 TSR".into(),
            ],
            license: None,
            credits: vec!["Producer".into()],
        });
        let about = original.about.unwrap();
        assert!(about.name.contains("AD&D Core Rules 2.0"));
        assert_eq!(about.rest, "Version 2.0\n© 1998 TSR");
        let (title, lines) = original.credits.unwrap();
        assert!(title.starts_with("Core Rules 2.0"));
        assert_eq!(lines, ["Producer"]);
        let none = Original::of(&OriginalAbout::default());
        assert!(none.about.is_none() && none.credits.is_none());
    }
}
