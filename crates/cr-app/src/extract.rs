//! Extract Data from CD… of the main menu, with no toolkit in it: what the
//! app says around the copy `cr-store` makes of a Core Rules 2.0 or
//! Expansion CD (its ISO image or the mounted CD) or of an installation,
//! and when a folder opened is copied first because nothing can be saved
//! into it.

use std::path::Path;

use cr_i18n::tr;
use cr_store::Installation;
use cr_store::extract::{Destination, Extraction, LocalCopy};

/// What Extract Data from CD… does, asked before the source is chosen.
pub fn intro() -> &'static str {
    tr(
        "Copies the rules databases, books and tables the app uses from a Core Rules 2.0 \
         or Expansion CD into the app's data folder, so the CD is no longer needed. \
         Choose the CD's ISO image, or the folder of the mounted CD or of an installation.",
    )
}

/// The Open dialog's file types for a CD's image.
pub fn image_filters() -> Vec<(String, String)> {
    vec![(tr("CD Images (*.iso)").to_owned(), "*.iso".to_owned())]
}

/// Copies the data of `source` where the app finds it.
pub fn extract(source: &Path) -> Result<Destination, String> {
    Extraction::probe(source)
        .and_then(|extraction| extraction.install())
        .map_err(|e| e.to_string())
}

/// What the copy did, once it is done.
pub fn extracted(dest: &Destination) -> String {
    match &dest.moved {
        Some(moved) => tr!(
            "Data extracted into {path}; the Core Rules 2.0 data moved to {moved}",
            path = dest.path.display(),
            moved = moved.display()
        ),
        None => tr!("Data extracted into {path}", path = dest.path.display()),
    }
}

/// Whether the app opens, in place of `install`, its copy in the app's
/// data folder: an Expansion it cannot save into (a mounted CD or ISO
/// image). Core Rules 2.0 is only browsed, so it opens from the CD.
pub fn opens_a_copy(install: &Installation) -> bool {
    !install.is_core20() && !install.is_writable()
}

/// The copy of `source` in the app's data folder, made first if there is
/// none.
pub fn local_copy(source: &Path) -> Result<LocalCopy, String> {
    cr_store::extract::local_copy(source).map_err(|e| e.to_string())
}

/// What the app says on opening `copy`: a title and text, for a copy made
/// just now, or one line for the copy already there.
pub fn copied(copy: &LocalCopy) -> (&'static str, String) {
    if copy.made {
        (
            tr("The CD Was Copied to This Computer"),
            tr!(
                "The CD cannot be written to, so its data was copied into {path}. \
                 The characters and everything else you make are saved there, and \
                 the app opens it on start without the CD.",
                path = copy.path.display()
            ),
        )
    } else {
        (
            tr("The CD cannot be written to: opened your copy of its data"),
            String::new(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn the_copy_says_where_it_went() {
        let dest = Destination {
            path: PathBuf::from("data"),
            is_data_folder: true,
            moved: None,
        };
        assert_eq!(extracted(&dest), "Data extracted into data");
        let dest = Destination {
            moved: Some(PathBuf::from("core20")),
            ..dest
        };
        assert_eq!(
            extracted(&dest),
            "Data extracted into data; the Core Rules 2.0 data moved to core20"
        );
    }

    #[test]
    fn a_copy_made_now_is_explained() {
        let copy = LocalCopy {
            path: PathBuf::from("data"),
            made: true,
            moved: None,
        };
        let (title, text) = copied(&copy);
        assert_eq!(title, "The CD Was Copied to This Computer");
        assert!(text.contains("data"));
        let (_, text) = copied(&LocalCopy {
            made: false,
            ..copy
        });
        assert!(text.is_empty());
    }

    #[test]
    fn a_folder_that_is_not_an_installation_is_refused() {
        let dir = std::env::temp_dir().join(format!("cr-extract-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(extract(&dir).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
