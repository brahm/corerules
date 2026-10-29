//! Every action is in every interface: the GNOME app refers to each
//! action of [`cr_app::action`], the Windows app to each one that
//! [`NOT_YET_ON_WINDOWS`] does not list, and the Mac app to each one that
//! [`NOT_YET_ON_MACOS`] does not list. An interface refers to an action
//! only through its constant, so the Rust sources are searched for
//! `action::NAME` and the Swift ones for `Action.NAME`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use cr_app::action::{ALL, NOT_YET_ON_MACOS, NOT_YET_ON_WINDOWS};

/// The files with extension `extension` under `dir`, read.
fn read_all(dir: &Path, extension: &str) -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, extension: &str, out: &mut Vec<(PathBuf, String)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, extension, out);
            } else if path.extension().is_some_and(|e| e == extension) {
                let text = std::fs::read_to_string(&path).unwrap();
                out.push((path, text));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, extension, &mut out);
    out
}

/// The repository's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The sources of the crate `name`, next to this one.
fn sources(name: &str) -> Vec<(PathBuf, String)> {
    let dir = root().join("crates").join(name).join("src");
    let out = read_all(&dir, "rs");
    assert!(!out.is_empty(), "no sources in {}", dir.display());
    out
}

/// The constants the sources refer to as `action::NAME`.
fn used(sources: &[(PathBuf, String)]) -> BTreeSet<String> {
    used_as(sources, "action::")
}

/// The constants the sources refer to as `prefix` and the name.
fn used_as(sources: &[(PathBuf, String)], prefix: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (_, text) in sources {
        for (at, _) in text.match_indices(prefix) {
            let name: String = text[at + prefix.len()..]
                .chars()
                .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
                .collect();
            if !name.is_empty() {
                found.insert(name);
            }
        }
    }
    found
}

fn names<'a>(actions: impl IntoIterator<Item = &'a cr_app::action::Action>) -> BTreeSet<String> {
    actions.into_iter().map(|a| a.constant.to_owned()).collect()
}

#[test]
fn the_gnome_app_has_every_action() {
    let missing: Vec<_> = names(ALL)
        .difference(&used(&sources("cr-gtk")))
        .cloned()
        .collect();
    assert!(
        missing.is_empty(),
        "actions the GNOME app does not refer to: {missing:?}"
    );
}

#[test]
fn the_windows_app_has_every_action_not_listed_as_pending() {
    let used = used(&sources("cr-win"));
    let expected = names(ALL)
        .difference(&names(NOT_YET_ON_WINDOWS))
        .cloned()
        .collect::<BTreeSet<_>>();
    let missing: Vec<_> = expected.difference(&used).cloned().collect();
    assert!(
        missing.is_empty(),
        "actions the Windows app should have but does not refer to: {missing:?}"
    );
    // An action ported to Windows comes off the list of pending ones.
    let ported: Vec<_> = names(NOT_YET_ON_WINDOWS)
        .intersection(&used)
        .cloned()
        .collect();
    assert!(
        ported.is_empty(),
        "actions the Windows app has that NOT_YET_ON_WINDOWS still lists: {ported:?}"
    );
}

#[test]
fn the_mac_app_has_every_action_not_listed_as_pending() {
    // The Swift sources of the Mac app.
    let used = used_as(&read_all(&root().join("macos"), "swift"), "Action.");
    let expected = names(ALL)
        .difference(&names(NOT_YET_ON_MACOS))
        .cloned()
        .collect::<BTreeSet<_>>();
    let missing: Vec<_> = expected.difference(&used).cloned().collect();
    assert!(
        missing.is_empty(),
        "actions the Mac app should have but does not refer to: {missing:?}"
    );
    let ported: Vec<_> = names(NOT_YET_ON_MACOS)
        .intersection(&used)
        .cloned()
        .collect();
    assert!(
        ported.is_empty(),
        "actions the Mac app has that NOT_YET_ON_MACOS still lists: {ported:?}"
    );
}

/// The GNOME app names no action of these groups by its string, which
/// would add an action the catalog and the Windows app do not know.
#[test]
fn the_gnome_app_names_actions_only_through_the_catalog() {
    let groups: BTreeSet<_> = ALL.iter().map(|a| a.group).collect();
    let mut found = Vec::new();
    for (path, text) in sources("cr-gtk") {
        for group in &groups {
            for (n, line) in text.lines().enumerate() {
                if line.contains(&format!("\"{group}.")) {
                    found.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "action names written out instead of taken from cr_app::action:\n{}",
        found.join("\n")
    );
}
