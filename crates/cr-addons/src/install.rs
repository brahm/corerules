//! Installing the add-ons into a data folder, without the interface: the
//! same steps as the old `instalar-addons.sh`.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use cr_formats::export::ExportFile;
use cr_i18n::tr;
use cr_store::Installation;
use cr_store::export::{Category, ImportReport, contents};

/// The consolidated add-ons, compressed with xz.
static PACKAGE: &[u8] = include_bytes!("../../../addons/Comunidade.xpx.xz");

/// Written into `UserDbas/` once the add-ons are in; a second install then
/// leaves the characters out, which would otherwise come in twice.
const MARKER: &str = ".addons-comunidade";

/// What a folder offers for the install.
pub enum Folder {
    Ready {
        install: Installation,
        /// The add-ons were installed there before.
        again: bool,
        /// The folder is the original program's, inside a Wine prefix.
        wine: bool,
    },
    /// A Core Rules 2.0 installation; the add-ons are the Expansion's.
    Core20,
    NotAnInstallation,
}

pub fn check(dir: &Path) -> Folder {
    let Ok(install) = Installation::open(dir) else {
        return Folder::NotAnInstallation;
    };
    if install.is_core20() {
        return Folder::Core20;
    }
    let again = install
        .path("UserDbas")
        .is_some_and(|d| d.join(MARKER).exists());
    let wine = install.root().to_string_lossy().contains("/drive_c/");
    Folder::Ready {
        install,
        again,
        wine,
    }
}

/// The built-in package, unpacked and read.
pub fn package() -> Result<ExportFile, String> {
    let mut data = Vec::new();
    lzma_rs::xz_decompress(&mut io::Cursor::new(PACKAGE), &mut data)
        .map_err(|e| tr!("The built-in add-ons are damaged: {e}", e))?;
    ExportFile::parse(&data).map_err(|e| tr!("Could not read the built-in add-ons: {e}", e))
}

/// Copies `UserDbas/` and `Chars/` into a folder beside the data folder;
/// returns that folder.
pub fn backup(install: &Installation, stamp: &str) -> Result<PathBuf, String> {
    let to = beside(install.root(), &format!("before-addons-{stamp}"));
    for name in ["UserDbas", "Chars"] {
        if let Some(from) = install.path(name) {
            let target = to.join(from.file_name().unwrap_or(name.as_ref()));
            copy_dir(&from, &target)
                .map_err(|e| tr!("Could not copy {folder}: {e}", folder = from.display(), e))?;
        }
    }
    Ok(to)
}

/// What the install did.
pub struct Outcome {
    pub imported: usize,
    pub characters: usize,
    /// Pictures written by [`pictures`].
    pub pictures: usize,
    pub log: PathBuf,
}

/// Imports `file` into `install`, then the classes again: a race/class
/// combination that names two new classes only comes in once both exist.
pub fn import(install: &Installation, file: &ExportFile, stamp: &str) -> Result<Outcome, String> {
    let items = contents(file);
    let again = matches!(check(install.root()), Folder::Ready { again: true, .. });
    let first: Option<Vec<usize>> = again.then(|| {
        (0..items.len())
            .filter(|&n| items[n].category != Category::Character)
            .collect()
    });
    let failed = |e: cr_store::Error| tr!("The import failed: {e}", e);
    let report = install.import(file, first.as_deref()).map_err(failed)?;
    let classes: Vec<usize> = (0..items.len())
        .filter(|&n| items[n].category == Category::Class)
        .collect();
    let second = install.import(file, Some(&classes)).map_err(failed)?;

    let mut log = String::new();
    write_report(&mut log, &report);
    log.push_str("=== second pass (classes)\n");
    write_report(&mut log, &second);
    let path = beside(install.root(), &format!("addons-{stamp}.log"));
    fs::write(&path, log)
        .map_err(|e| tr!("Could not write {file}: {e}", file = path.display(), e))?;
    if let Some(userdbas) = install.path("UserDbas") {
        let _ = fs::write(userdbas.join(MARKER), stamp);
    }
    Ok(Outcome {
        imported: report.imported.len(),
        characters: report.characters.len(),
        pictures: 0,
        log: path,
    })
}

/// Writes the add-ons' pictures into `install`, beside nothing that is
/// there already, and adds the files to the log.
pub fn pictures(install: &Installation, outcome: &mut Outcome) -> Result<(), String> {
    let written = crate::extras::get()?.install_pictures(install)?;
    let mut log = String::from("=== pictures\n");
    for file in &written {
        let _ = writeln!(log, "wrote {}", file.display());
    }
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&outcome.log)
        .map_err(|e| {
            tr!(
                "Could not write {file}: {e}",
                file = outcome.log.display(),
                e
            )
        })?;
    io::Write::write_all(&mut file, log.as_bytes()).map_err(|e| {
        tr!(
            "Could not write {file}: {e}",
            file = outcome.log.display(),
            e
        )
    })?;
    outcome.pictures = written.len();
    Ok(())
}

/// The report in the words `cr import` prints.
fn write_report(out: &mut String, report: &ImportReport) {
    for c in &report.imported {
        let _ = writeln!(out, "imported {}: {}", c.category.label(), c.name);
    }
    for (category, name) in &report.replaced {
        let _ = writeln!(out, "replaced {}: {name}", category.label());
    }
    for c in &report.skipped {
        let _ = writeln!(out, "skipped {}: {}", c.category.label(), c.name);
    }
    for (c, name) in &report.renamed {
        let _ = writeln!(out, "renamed {}: {} to {name}", c.category.label(), c.name);
    }
    for file in &report.characters {
        let _ = writeln!(out, "wrote Chars/{file}");
    }
    for warning in &report.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
}

/// `data` → `data-SUFFIX`, in the same parent folder.
fn beside(root: &Path, suffix: &str) -> PathBuf {
    let name = root
        .file_name()
        .map_or("data".into(), |n| n.to_string_lossy());
    root.with_file_name(format!("{name}-{suffix}"))
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_package_reads() {
        let file = package().unwrap();
        assert!(file.is_expansion());
        let items = contents(&file);
        assert_eq!(items.len(), 36_525);
        let characters = items.iter().filter(|c| c.category == Category::Character);
        assert_eq!(characters.count(), 48);
    }

    #[test]
    fn backups_and_logs_sit_beside_the_folder() {
        let root = Path::new("/home/u/.local/share/corerules/data");
        assert_eq!(
            beside(root, "addons-1.log"),
            Path::new("/home/u/.local/share/corerules/data-addons-1.log")
        );
    }
}
