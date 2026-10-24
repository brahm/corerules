//! `cr export` and `cr import`: the DM Toolkit's packages (`.xpx`, `.xpt`).

use std::path::{Path, PathBuf};

use cr_store::export::remove;

use cr_formats::export::ExportFile;
use cr_store::export::{Category, Clash, Content, Exportable};
use cr_store::{Error, Installation, Result};

/// Picks the items an argument names: `CATEGORY:NAME` or a bare name.
fn pick<'a, T>(
    items: &'a [T],
    arg: &str,
    category: impl Fn(&T) -> Category,
    name: impl Fn(&T) -> &str,
) -> Result<&'a T> {
    let (wanted, text) = match arg.split_once(':') {
        Some((c, n)) if Category::parse(c).is_some() => (Category::parse(c), n),
        _ => (None, arg),
    };
    let found: Vec<&T> = items
        .iter()
        .filter(|i| wanted.is_none_or(|c| category(i) == c))
        .filter(|i| name(i).trim().eq_ignore_ascii_case(text.trim()))
        .collect();
    match found.as_slice() {
        [one] => Ok(one),
        [] => Err(Error::Invalid(format!("nothing named {arg}"))),
        _ => Err(Error::Invalid(format!(
            "{arg} names several items; write CATEGORY:NAME ({})",
            found
                .iter()
                .map(|i| format!("{}:{}", category(i).label(), name(i)))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

pub fn cmd_export(install: &Installation, args: &[String]) -> Result<()> {
    let invalid = |m: &str| Error::Invalid(m.into());
    let all = install.exportable()?;
    if args.first().map(String::as_str) == Some("--list") {
        for item in &all {
            println!("{}\t{}", item.category.label(), item.name);
        }
        return Ok(());
    }
    let Some(output) = args.first() else {
        return Err(invalid(
            "usage: cr export INSTALL_DIR OUTPUT (--all | ITEM...)",
        ));
    };
    let mut chosen: Vec<Exportable> = Vec::new();
    for arg in &args[1..] {
        match arg.as_str() {
            "--all" => chosen.extend(all.iter().cloned()),
            item => chosen.push(pick(&all, item, |e| e.category, |e| &e.name)?.clone()),
        }
    }
    if chosen.is_empty() {
        return Err(invalid("name the items to export, or --all"));
    }
    let export = install.export(&chosen)?;
    let mut path = PathBuf::from(output);
    if path.extension().is_none() {
        path.set_extension(if export.file.is_expansion() {
            "xpx"
        } else {
            "xpt"
        });
    }
    write_package(&path, &export.file)?;
    for warning in &export.warnings {
        eprintln!("cr: {warning}");
    }
    println!("{}: {} items", path.display(), chosen.len());
    Ok(())
}

fn write_package(path: &Path, file: &ExportFile) -> Result<()> {
    let bytes = file.to_bytes().map_err(|source| Error::Format {
        path: path.to_path_buf(),
        source,
    })?;
    std::fs::write(path, bytes).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub fn read_package(path: &Path) -> Result<ExportFile> {
    let data = std::fs::read(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    ExportFile::parse(&data).map_err(|source| Error::Format {
        path: path.to_path_buf(),
        source,
    })
}

pub fn cmd_import(install: &Installation, args: &[String]) -> Result<()> {
    let invalid = |m: &str| Error::Invalid(m.into());
    let Some(input) = args.first() else {
        return Err(invalid(
            "usage: cr import INSTALL_DIR (FILE | CORE_RULES_2.0_DIR) [--list] [ITEM...]",
        ));
    };
    // A folder is a Core Rules 2.0 installation to bring over.
    let file = if Path::new(input).is_dir() {
        Installation::open(input)?.core20_package()?.file
    } else {
        read_package(Path::new(input))?
    };
    let preview = install.import_preview(&file)?;
    if preview.is_empty() {
        return Err(invalid(if Path::new(input).is_dir() {
            "nothing to import: this Core Rules 2.0 installation has no characters or records of \
             its own"
        } else {
            "nothing to import: the package is empty"
        }));
    }
    let mut chosen: Vec<usize> = Vec::new();
    let mut list = false;
    for arg in &args[1..] {
        match arg.as_str() {
            "--list" => list = true,
            item => {
                let found = pick(&preview, item, |c| c.category, |c| &c.name)?;
                let n = preview.iter().position(|c| c == found).unwrap_or_default();
                chosen.push(n);
            }
        }
    }
    if list {
        for c in &preview {
            let mark = match (c.clash, file.is_expansion()) {
                (Clash::None, _) => "",
                (Clash::Standard, _) => "\trenamed (a standard part has the name)",
                (Clash::Custom, true) => "\treplaces",
                (Clash::Custom, false) => "\tname in use",
            };
            println!("{}\t{}{mark}", c.category.label(), c.name);
        }
        if file.map_maker.count > 0 {
            println!("({} Map Maker II maps, not imported)", file.map_maker.count);
        }
        return Ok(());
    }
    let chosen = (!chosen.is_empty()).then_some(chosen);
    let report = install.import(&file, chosen.as_deref())?;
    let label = |c: &Content| format!("{}: {}", c.category.label(), c.name);
    for c in &report.imported {
        println!("imported {}", label(c));
    }
    for (category, name) in &report.replaced {
        println!("replaced {}: {name}", category.label());
    }
    for c in &report.skipped {
        println!("skipped {} (a custom record has the name)", label(c));
    }
    for (c, name) in &report.renamed {
        println!("renamed {} to {name}", label(c));
    }
    for file in &report.characters {
        println!("wrote Chars/{file}");
    }
    for warning in &report.warnings {
        eprintln!("cr: {warning}");
    }
    Ok(())
}

pub fn cmd_package(install: &Installation, args: &[String]) -> Result<()> {
    let invalid = |m: &str| Error::Invalid(m.into());
    let usage = "usage: cr package INSTALL_DIR FILE [--new] [--add ITEM]... [--remove ITEM]...";
    let Some(output) = args.first() else {
        return Err(invalid(usage));
    };
    let (mut new, mut add, mut take) = (false, Vec::new(), Vec::new());
    let mut rest = args[1..].iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--new" => new = true,
            "--add" => add.push(rest.next().ok_or_else(|| invalid("--add needs an item"))?),
            "--remove" => take.push(
                rest.next()
                    .ok_or_else(|| invalid("--remove needs an item"))?,
            ),
            _ => return Err(invalid(usage)),
        }
    }
    let mut path = PathBuf::from(output);
    let mut file = if new {
        let expansion = install.path("UserDbas/AllowedU.dat").is_some();
        if path.extension().is_none() {
            path.set_extension(if expansion { "xpx" } else { "xpt" });
        }
        ExportFile::new(expansion)
    } else {
        read_package(&path)?
    };
    let changes = new || !add.is_empty() || !take.is_empty();
    if changes && !file.is_expansion() {
        // 2.0 content is only imported, and converted to the Expansion's
        // layout on the way in.
        return Err(invalid(
            "Core Rules 2.0 packages are only imported (cr import INSTALL_DIR FILE); \
             new and edited packages are the Expansion's",
        ));
    }
    let mut warnings = Vec::new();
    if !take.is_empty() {
        let items = cr_store::export::contents(&file);
        let chosen = take
            .iter()
            .map(|t| pick(&items, t, |c| c.category, |c| &c.name).cloned())
            .collect::<Result<Vec<Content>>>()?;
        warnings.extend(remove(&mut file, &chosen));
    }
    if !add.is_empty() {
        let offered = install.exportable()?;
        let chosen = add
            .iter()
            .map(|a| pick(&offered, a, |e| e.category, |e| &e.name).cloned())
            .collect::<Result<Vec<Exportable>>>()?;
        warnings.extend(install.add_to(&mut file, &chosen)?);
    }
    if changes {
        write_package(&path, &file)?;
    }
    for c in cr_store::export::contents(&file) {
        println!("{}\t{}", c.category.label(), c.name);
    }
    for warning in &warnings {
        eprintln!("cr: {warning}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_ignores_spaces_around_a_name() {
        let items = [(Category::Class, "Sword Dancer "), (Category::Race, "Drow")];
        let found = pick(&items, "classes:sword dancer", |i| i.0, |i| i.1).unwrap();
        assert_eq!(found.1, "Sword Dancer ");
    }
}
