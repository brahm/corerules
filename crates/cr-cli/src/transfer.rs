//! `cr export` and `cr import`: the DM Toolkit's packages (`.xpx`, `.xpt`).

use std::path::{Path, PathBuf};

use cr_formats::export::ExportFile;
use cr_store::export::{Category, Clash, Content, Exportable, Source};
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
        .filter(|i| name(i).eq_ignore_ascii_case(text.trim()))
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
            "usage: cr export INSTALL_DIR OUTPUT (--all | ITEM...) [--map FILE]...",
        ));
    };
    let mut chosen: Vec<Exportable> = Vec::new();
    let mut rest = args[1..].iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--all" => chosen.extend(all.iter().cloned()),
            "--map" => {
                let path = PathBuf::from(rest.next().ok_or_else(|| invalid("--map needs a file"))?);
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                chosen.push(Exportable {
                    category: Category::CampaignMap,
                    name: name.into_owned(),
                    source: Source::Map(path.clone()),
                });
            }
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
    let bytes = export.file.to_bytes().map_err(|source| Error::Format {
        path: path.clone(),
        source,
    })?;
    std::fs::write(&path, bytes).map_err(|source| Error::Io {
        path: path.clone(),
        source,
    })?;
    for warning in &export.warnings {
        eprintln!("cr: {warning}");
    }
    println!("{}: {} items", path.display(), chosen.len());
    Ok(())
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
            "usage: cr import INSTALL_DIR FILE [--list] [--maps DIR] [ITEM...]",
        ));
    };
    let file = read_package(Path::new(input))?;
    let preview = install.import_preview(&file)?;
    let mut chosen: Vec<usize> = Vec::new();
    let mut maps = None;
    let mut list = false;
    let mut rest = args[1..].iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--list" => list = true,
            "--maps" => {
                maps = Some(PathBuf::from(
                    rest.next()
                        .ok_or_else(|| invalid("--maps needs a folder"))?,
                ))
            }
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
    let report = install.import(&file, chosen.as_deref(), maps.as_deref())?;
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
    for map in &report.maps {
        println!("wrote {}", map.display());
    }
    if maps.is_none() && !file.campaign_maps.is_empty() {
        println!("drawings skipped; --maps DIR writes them");
    }
    for warning in &report.warnings {
        eprintln!("cr: {warning}");
    }
    Ok(())
}
