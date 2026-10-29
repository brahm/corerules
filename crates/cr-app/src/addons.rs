//! Import Add-ons…: every add-on in a folder brought in at once, with no
//! toolkit in it. The folder is searched with its subfolders and the ZIP
//! archives in it (and in them), as the community shared the add-ons:
//! each DM Toolkit package (`.xpx`, `.xpt`) is read and told apart by what
//! it holds, the Expansion's or Core Rules 2.0's, and checked before
//! anything is written; the pictures beside them go where the records
//! they bring look for them. The packages come in from the oldest file to
//! the newest, so that where two hold a record of the same name the newer
//! one stays, as when they are imported one by one in that order.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cr_formats::Object;
use cr_formats::export::ExportFile;
use cr_i18n::{tr, trn};
use cr_store::Installation;
use cr_store::export::{Category, ImportReport, contents};
use cr_store::pictures::{Kind as PictureKind, joined, relative};

use crate::packages::category_label;

/// How deep ZIP archives inside ZIP archives are opened.
const NESTED_ARCHIVES: usize = 3;

/// The categories a package's description names before "…".
const CATEGORIES_NAMED: usize = 4;

/// Whether a package found in the folder is imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Ready,
    /// The same file as the package found at this path, imported once.
    Same(String),
    /// Not imported, for this reason.
    Refused(String),
}

/// A package found in the folder, or an archive that could not be read.
#[derive(Debug, Clone)]
pub struct Found {
    /// Its path in the folder, `/` between the parts and `›` after an
    /// archive: `Classes/Barbarian.zip › Barbarian.xpx`.
    pub path: String,
    /// Made by the Expansion (or by Core Rules 2.0); `None` when it could
    /// not be read.
    pub expansion: Option<bool>,
    /// The items it holds, by category, sorted by label.
    pub categories: Vec<(Category, usize)>,
    pub items: usize,
    pub state: State,
    /// Its date, in seconds since 1970, which orders the import.
    date: i64,
    /// Its bytes hashed, which tells two copies of it.
    hash: u64,
    file: Option<ExportFile>,
}

impl Found {
    /// What it is and holds, as the list before the import says it:
    /// "Expansion: Classes 3, Kits 12".
    pub fn description(&self) -> String {
        let kind = match self.expansion {
            Some(true) => tr("Expansion"),
            Some(false) => tr("Core Rules 2.0"),
            None => return String::new(),
        };
        let mut named: Vec<String> = self
            .categories
            .iter()
            .take(CATEGORIES_NAMED)
            .map(|(c, n)| format!("{} {n}", category_label(*c)))
            .collect();
        if self.categories.len() > CATEGORIES_NAMED {
            named.push("…".into());
        }
        format!("{kind}: {}", named.join(", "))
    }
}

/// A list of the dialog before the import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub heading: String,
    /// A file's path, and what is said of it.
    pub rows: Vec<(String, String)>,
}

/// What a folder holds for Import Add-ons….
#[derive(Debug, Clone)]
pub struct AddonFolder {
    pub folder: PathBuf,
    /// Whether the installation is the Expansion's.
    pub expansion: bool,
    /// The packages in the order they are imported, then those refused.
    pub found: Vec<Found>,
    pictures: Pictures,
    /// The other files: texts, documents and the like, left alone.
    pub others: usize,
}

/// Reads every add-on in `folder` and checks it for an installation of
/// the Expansion (`expansion`) or of Core Rules 2.0. Only a folder that
/// cannot be read fails; a file that cannot is listed as refused.
pub fn scan(folder: &Path, expansion: bool) -> Result<AddonFolder, String> {
    let mut scan = Scan::default();
    scan.dir(folder, "")
        .map_err(|e| format!("{}: {e}", folder.display()))?;
    let mut found = scan.found;
    // Each file once, the oldest copy of it imported.
    found.sort_by(|a, b| (a.date, &a.path).cmp(&(b.date, &b.path)));
    let mut seen: HashMap<u64, String> = HashMap::new();
    for f in &mut found {
        if f.state != State::Ready {
            continue;
        }
        if f.expansion == Some(true) && !expansion {
            f.state = State::Refused(
                tr("Made by the Expansion, which Core Rules 2.0 cannot import").to_owned(),
            );
        } else if let Some(first) = seen.get(&f.hash) {
            f.state = State::Same(first.clone());
        } else {
            seen.insert(f.hash, f.path.clone());
        }
    }
    // The refused ones after those imported, in the order of their paths.
    found.sort_by_key(|f| f.state != State::Ready && !matches!(f.state, State::Same(_)));
    Ok(AddonFolder {
        folder: folder.to_owned(),
        expansion,
        found,
        pictures: scan.pictures,
        others: scan.others,
    })
}

/// The pictures found, and where a `pictures.tsv` beside them puts them.
#[derive(Debug, Clone, Default)]
struct Pictures {
    files: Vec<Vec<u8>>,
    /// Each one by its file name in lower case, the first found of a name.
    by_name: HashMap<String, usize>,
    /// Each one by its path as shown, in lower case.
    by_path: HashMap<String, usize>,
    /// (picture's path as shown in lower case, place in the
    /// installation), from the lines `picture<TAB>place` of a
    /// `pictures.tsv`, whose pictures are named from its folder.
    placements: Vec<(String, String)>,
}

impl Pictures {
    fn add(&mut self, name: &str, shown: &str, bytes: Vec<u8>) {
        let n = self.files.len();
        self.files.push(bytes);
        self.by_name.entry(name.to_lowercase()).or_insert(n);
        self.by_path.insert(shown.to_lowercase(), n);
    }

    fn read_placements(&mut self, shown: &str, bytes: &[u8]) {
        for line in String::from_utf8_lossy(bytes).lines() {
            if let Some((picture, place)) = line.trim_end_matches('\r').split_once('\t')
                && !picture.is_empty()
                && !place.is_empty()
            {
                let picture = sibling(shown, &picture.replace('\\', "/"));
                self.placements
                    .push((picture.to_lowercase(), place.replace('\\', "/")));
            }
        }
    }
}

/// The path, shown as [`Found::path`] is, of `relative` in the folder (or
/// archive) of the file shown at `shown`.
fn sibling(shown: &str, relative: &str) -> String {
    let slash = shown.rfind('/').map(|at| (at, 1));
    let archive = shown.rfind(" › ").map(|at| (at, " › ".len()));
    match slash.max(archive) {
        Some((at, len)) => format!("{}{relative}", &shown[..at + len]),
        None => relative.to_owned(),
    }
}

/// The files found while the folder is searched.
#[derive(Default)]
struct Scan {
    found: Vec<Found>,
    pictures: Pictures,
    others: usize,
}

impl Scan {
    /// The files of `dir` and of its subfolders, shown under `shown`.
    fn dir(&mut self, dir: &Path, shown: &str) -> io::Result<()> {
        let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            // Hidden files, and the Mac's resource forks in archives.
            if name.starts_with('.') {
                continue;
            }
            let path = entry.path();
            let shown = join(shown, &name);
            if path.is_dir() {
                // A folder that cannot be read is left out, not the rest.
                let _ = self.dir(&path, &shown);
                continue;
            }
            let date = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs() as i64);
            match Type::of(&name) {
                Type::Other => self.others += 1,
                _ => match fs::read(&path) {
                    Ok(bytes) => self.file(&name, shown, bytes, date, 0),
                    Err(e) => self.refused(shown, e.to_string()),
                },
            }
        }
        Ok(())
    }

    /// A file named `name`, from the folder or an archive.
    fn file(&mut self, name: &str, shown: String, bytes: Vec<u8>, date: i64, depth: usize) {
        match Type::of(name) {
            Type::Package => match ExportFile::parse(&bytes) {
                Ok(file) => {
                    let items = contents(&file);
                    let mut categories: Vec<(Category, usize)> = Vec::new();
                    for item in &items {
                        match categories.iter_mut().find(|(c, _)| *c == item.category) {
                            Some((_, n)) => *n += 1,
                            None => categories.push((item.category, 1)),
                        }
                    }
                    categories.sort_by_key(|(c, _)| category_label(*c));
                    let state = if items.is_empty() {
                        State::Refused(tr("Holds nothing to import").to_owned())
                    } else {
                        State::Ready
                    };
                    let mut hasher = DefaultHasher::new();
                    bytes.hash(&mut hasher);
                    self.found.push(Found {
                        path: shown,
                        expansion: Some(file.is_expansion()),
                        categories,
                        items: items.len(),
                        state,
                        date,
                        hash: hasher.finish(),
                        file: Some(file),
                    });
                }
                Err(e) => self.refused(shown, tr!("Not a package: {e}", e)),
            },
            Type::Archive if depth < NESTED_ARCHIVES => {
                if let Err(e) = self.archive(&shown, bytes, depth + 1) {
                    self.refused(shown, tr!("Could not open the archive: {e}", e));
                }
            }
            Type::Picture => self.pictures.add(name, &shown, bytes),
            Type::Placements => self.pictures.read_placements(&shown, &bytes),
            Type::Archive | Type::Other => self.others += 1,
        }
    }

    /// The files of the ZIP archive `bytes`, shown under `shown`.
    fn archive(&mut self, shown: &str, bytes: Vec<u8>, depth: usize) -> zip::result::ZipResult<()> {
        let mut archive = zip::ZipArchive::new(io::Cursor::new(bytes))?;
        for i in 0..archive.len() {
            let name = archive.name_for_index(i).unwrap_or_default().to_owned();
            let mut entry = match archive.by_index(i) {
                Ok(entry) => entry,
                Err(e) => {
                    self.refused(format!("{shown} › {name}"), e.to_string());
                    continue;
                }
            };
            if entry.is_dir() {
                continue;
            }
            let path = entry.name().replace('\\', "/");
            let name = path.rsplit('/').next().unwrap_or_default().to_owned();
            if name.is_empty() || name.starts_with('.') || path.starts_with("__MACOSX/") {
                continue;
            }
            let inner = format!("{shown} › {path}");
            if Type::of(&name) == Type::Other {
                self.others += 1;
                continue;
            }
            let date = entry.last_modified().map_or(0, |d| {
                seconds(
                    d.year().into(),
                    d.month().into(),
                    d.day().into(),
                    d.hour().into(),
                    d.minute().into(),
                    d.second().into(),
                )
            });
            let mut data = Vec::with_capacity(entry.size() as usize);
            match entry.read_to_end(&mut data) {
                Ok(_) => self.file(&name, inner, data, date, depth),
                Err(e) => self.refused(inner, e.to_string()),
            }
        }
        Ok(())
    }

    fn refused(&mut self, shown: String, why: String) {
        self.found.push(Found {
            path: shown,
            expansion: None,
            categories: Vec::new(),
            items: 0,
            state: State::Refused(why),
            date: 0,
            hash: 0,
            file: None,
        });
    }
}

/// What a file is, by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type {
    Package,
    Archive,
    Picture,
    /// `pictures.tsv`: where the pictures beside it go.
    Placements,
    Other,
}

impl Type {
    fn of(name: &str) -> Type {
        if name.eq_ignore_ascii_case("pictures.tsv") {
            return Type::Placements;
        }
        let extension = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
        match extension.as_deref() {
            Some("xpx" | "xpt") => Type::Package,
            Some("zip") => Type::Archive,
            Some("bmp") => Type::Picture,
            _ => Type::Other,
        }
    }
}

fn join(shown: &str, name: &str) -> String {
    if shown.is_empty() {
        name.to_owned()
    } else {
        format!("{shown}/{name}")
    }
}

impl AddonFolder {
    /// The packages that are imported.
    pub fn ready(&self) -> impl Iterator<Item = &Found> {
        self.found.iter().filter(|f| f.state == State::Ready)
    }

    pub fn can_import(&self) -> bool {
        self.ready().next().is_some()
    }

    /// The items the packages imported hold.
    pub fn items(&self) -> usize {
        self.ready().map(|f| f.items).sum()
    }

    /// The pictures found.
    pub fn pictures(&self) -> usize {
        self.pictures.files.len()
    }

    /// The title of the dialog before the import.
    pub fn title(&self) -> &'static str {
        if self.can_import() {
            tr("Import Add-ons")
        } else {
            tr("Nothing to Import")
        }
    }

    /// The lists the dialog before the import shows: what is imported,
    /// what is found twice and what is refused, each a heading over rows
    /// of a path and what it says of it.
    pub fn groups(&self) -> Vec<Group> {
        let mut groups = Vec::new();
        let ready: Vec<&Found> = self.ready().collect();
        groups.push(Group {
            heading: if ready.is_empty() {
                tr("The folder holds no add-on that can be imported.").to_owned()
            } else {
                trn!(
                    "{n} add-on, with {items} items, will be imported:",
                    "{n} add-ons, with {items} items, will be imported:",
                    ready.len(),
                    items = self.items()
                )
            },
            rows: ready
                .iter()
                .map(|f| (f.path.clone(), f.description()))
                .collect(),
        });
        let same: Vec<(String, String)> = self
            .found
            .iter()
            .filter_map(|f| match &f.state {
                State::Same(first) => Some((f.path.clone(), tr!("Same as {file}", file = first))),
                _ => None,
            })
            .collect();
        if !same.is_empty() {
            groups.push(Group {
                heading: tr("Imported once, as the same file is found twice:").to_owned(),
                rows: same,
            });
        }
        let refused: Vec<(String, String)> = self
            .found
            .iter()
            .filter_map(|f| match &f.state {
                State::Refused(why) => Some((f.path.clone(), why.clone())),
                _ => None,
            })
            .collect();
        if !refused.is_empty() {
            groups.push(Group {
                heading: tr("Not imported:").to_owned(),
                rows: refused,
            });
        }
        groups
    }

    /// What the dialog before the import says under the lists.
    pub fn notes(&self) -> Vec<String> {
        let mut notes = Vec::new();
        if self.pictures() > 0 {
            notes.push(trn!(
                "{n} picture found: it is copied where a record that names it looks for it.",
                "{n} pictures found: they are copied where the records that name them look \
                 for them.",
                self.pictures()
            ));
        }
        if self.others > 0 {
            notes.push(trn!(
                "{n} other file (texts, documents) is left as it is.",
                "{n} other files (texts, documents) are left as they are.",
                self.others
            ));
        }
        if self.can_import() {
            notes.push(
                tr(
                    "The user databases and the characters are copied to a backup first. The \
                    add-ons come in from the oldest file to the newest, and characters already \
                    in the list are left out.",
                )
                .to_owned(),
            );
        }
        notes
    }

    /// The dialog before the import as one text, for the apps that show
    /// it so: its title and text.
    pub fn summary(&self) -> (String, String) {
        let mut parts = Vec::new();
        for group in self.groups() {
            let mut text = group.heading;
            for (path, detail) in &group.rows {
                let _ = write!(text, "\n• {path}: {detail}");
            }
            parts.push(text);
        }
        let notes = self.notes();
        if !notes.is_empty() {
            parts.push(notes.join("\n"));
        }
        (self.title().to_owned(), parts.join("\n\n"))
    }

    /// Imports every package that is ready into `install`, after copying
    /// its user databases and characters to a backup beside it.
    /// `progress` is told of each package as it starts: the number done,
    /// how many there are and its path.
    pub fn import(
        &self,
        install: &Installation,
        progress: &dyn Fn(usize, usize, &str),
    ) -> Result<Imported, String> {
        let stamp = stamp(SystemTime::now());
        let backup = backup(install, &stamp)?;
        let ready: Vec<&Found> = self.ready().collect();
        let mut known: HashSet<String> = install
            .characters()
            .map_err(|e| e.to_string())?
            .entries
            .iter()
            .map(|e| e.name.trim().to_lowercase())
            .collect();
        let mut done = Imported {
            backup: Some(backup),
            ..Default::default()
        };
        for (n, found) in ready.iter().enumerate() {
            progress(n, ready.len(), &found.path);
            let Some(file) = &found.file else { continue };
            // A character already in the list (or brought by an earlier
            // add-on) would come in twice.
            let items = contents(file);
            let mut left_out = Vec::new();
            for item in items.iter().filter(|c| c.category == Category::Character) {
                if !known.insert(item.name.trim().to_lowercase()) {
                    left_out.push(item.name.clone());
                }
            }
            let chosen: Option<Vec<usize>> = (!left_out.is_empty()).then(|| {
                let left: HashSet<&str> = left_out.iter().map(String::as_str).collect();
                (0..items.len())
                    .filter(|&i| {
                        items[i].category != Category::Character
                            || !left.contains(items[i].name.as_str())
                    })
                    .collect()
            });
            match install.import(file, chosen.as_deref()) {
                Ok(report) => {
                    done.characters_left.extend(left_out);
                    match self.place_pictures(install, file) {
                        Ok(written) => done.pictures.extend(written),
                        Err(e) => done.report_warnings.push(e),
                    }
                    done.packages.push((found.path.clone(), report));
                }
                Err(e) => done.failed.push((found.path.clone(), e.to_string())),
            }
        }
        if !done.packages.is_empty() {
            match self.place_listed(install) {
                Ok(written) => done.pictures.extend(written),
                Err(e) => done.report_warnings.push(e),
            }
        }
        progress(ready.len(), ready.len(), "");
        let log = beside(install.root(), &format!("addons-{stamp}.log"));
        if fs::write(&log, done.log()).is_ok() {
            done.log = Some(log);
        }
        Ok(done)
    }

    /// Writes the pictures the records of `file` name where they look for
    /// them, never over a file that is there; returns the files written.
    fn place_pictures(
        &self,
        install: &Installation,
        file: &ExportFile,
    ) -> Result<Vec<PathBuf>, String> {
        let mut written = Vec::new();
        for (stored, kind) in picture_references(file) {
            let Some(place) = relative(&stored, kind) else {
                continue;
            };
            let name = place.rsplit('/').next().unwrap_or_default().to_lowercase();
            if let Some(&n) = self.pictures.by_name.get(&name) {
                written.extend(write_picture(install, &place, &self.pictures.files[n])?);
            }
        }
        Ok(written)
    }

    /// Writes the pictures a `pictures.tsv` places, never over a file
    /// that is there; returns the files written.
    fn place_listed(&self, install: &Installation) -> Result<Vec<PathBuf>, String> {
        let mut written = Vec::new();
        for (picture, place) in &self.pictures.placements {
            if let Some(&n) = self.pictures.by_path.get(picture) {
                written.extend(write_picture(install, place, &self.pictures.files[n])?);
            }
        }
        Ok(written)
    }
}

/// Writes `bytes` at `place` (`/` between its parts) in `install`, unless
/// a file is there in any case; returns the file written.
fn write_picture(
    install: &Installation,
    place: &str,
    bytes: &[u8],
) -> Result<Option<PathBuf>, String> {
    let target = existing_case(
        &place
            .split('/')
            .filter(|p| !p.is_empty() && *p != "..")
            .fold(install.root().to_owned(), |p, c| p.join(c)),
    );
    if target.exists() {
        return Ok(None);
    }
    let write = || -> io::Result<()> {
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&target, bytes)
    };
    write().map_err(|e| tr!("Could not write {file}: {e}", file = target.display(), e))?;
    Ok(Some(target))
}

/// The pictures the records and characters of `file` name, as stored,
/// each with where a bare file name is looked for.
pub fn picture_references(file: &ExportFile) -> Vec<(String, PictureKind)> {
    let mut out = Vec::new();
    for o in &file.monster_types {
        if let Object::MonsterType(t) = o {
            out.push((t.picture.clone(), PictureKind::Monster));
            out.push((t.page_picture.clone(), PictureKind::Monster));
        }
    }
    for o in &file.races {
        if let Object::Race(r) = o {
            out.push((
                joined(&r.portrait_folder, &r.portrait),
                PictureKind::Portrait,
            ));
            for p in &r.portraits {
                out.push((joined(&r.portrait_folder, p), PictureKind::Portrait));
            }
        }
    }
    for c in &file.characters {
        let [folder, name] = &c.data.unknown_0x1db8;
        out.push((joined(folder, name), PictureKind::Portrait));
    }
    out.retain(|(stored, _)| !stored.trim().is_empty());
    out
}

/// What Import Add-ons… did.
#[derive(Debug, Default)]
pub struct Imported {
    /// The packages imported, by path, with what each import did.
    pub packages: Vec<(String, ImportReport)>,
    /// The packages whose import failed, with why.
    pub failed: Vec<(String, String)>,
    /// The characters left out, as the list has them already.
    pub characters_left: Vec<String>,
    /// The pictures written.
    pub pictures: Vec<PathBuf>,
    /// What went wrong with the pictures.
    pub report_warnings: Vec<String>,
    /// Where the user databases and characters were copied.
    pub backup: Option<PathBuf>,
    /// The file with everything the import did.
    pub log: Option<PathBuf>,
}

impl Imported {
    /// What the report says: its heading and text.
    pub fn report(&self) -> (String, String) {
        let count = |f: fn(&ImportReport) -> usize| -> usize {
            self.packages.iter().map(|(_, r)| f(r)).sum()
        };
        let items = count(|r| r.imported.len());
        let mut lines = vec![trn!("{n} item came in.", "{n} items came in.", items)];
        let characters = count(|r| r.characters.len());
        if characters > 0 {
            lines.push(trn!(
                "{n} character joined the character list.",
                "{n} characters joined the character list.",
                characters
            ));
        }
        if !self.pictures.is_empty() {
            lines.push(trn!(
                "{n} picture was copied.",
                "{n} pictures were copied.",
                self.pictures.len()
            ));
        }
        let replaced = count(|r| r.replaced.len());
        if replaced > 0 {
            lines.push(trn!(
                "{n} record of the same name was replaced.",
                "{n} records of the same names were replaced.",
                replaced
            ));
        }
        let renamed = count(|r| r.renamed.len());
        if renamed > 0 {
            lines.push(trn!(
                "{n} item was renamed with a *, as a standard part has its name.",
                "{n} items were renamed with a *, as standard parts have their names.",
                renamed
            ));
        }
        let skipped = count(|r| r.skipped.len());
        if skipped > 0 {
            lines.push(trn!(
                "{n} item was not imported, as you have a record of its name.",
                "{n} items were not imported, as you have records of their names.",
                skipped
            ));
        }
        if !self.characters_left.is_empty() {
            lines.push(trn!(
                "{n} character was left out, as the list has it already.",
                "{n} characters were left out, as the list has them already.",
                self.characters_left.len()
            ));
        }
        for (path, why) in &self.failed {
            lines.push(tr!("{file} was not imported: {why}", file = path, why));
        }
        lines.extend(self.report_warnings.iter().cloned());
        let warnings = count(|r| r.warnings.len());
        if warnings > 0 {
            lines.push(trn!(
                "{n} warning, in the log.",
                "{n} warnings, in the log.",
                warnings
            ));
        }
        if let Some(backup) = &self.backup {
            lines.push(tr!(
                "The user databases and characters from before are in {folder}.",
                folder = backup.display()
            ));
        }
        if let Some(log) = &self.log {
            lines.push(tr!(
                "Everything the import did is in {file}.",
                file = log.display()
            ));
        }
        let heading = trn!(
            "Imported {n} Add-on",
            "Imported {n} Add-ons",
            self.packages.len()
        );
        (heading, lines.join("\n"))
    }

    /// The log, in the words `cr import` prints, package by package.
    fn log(&self) -> String {
        let mut out = String::new();
        for (path, report) in &self.packages {
            let _ = writeln!(out, "=== {path}");
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
        for (path, why) in &self.failed {
            let _ = writeln!(out, "=== {path}\nfailed: {why}");
        }
        if !self.characters_left.is_empty() {
            let _ = writeln!(out, "=== characters already in the list");
            for name in &self.characters_left {
                let _ = writeln!(out, "left out: {name}");
            }
        }
        if !self.pictures.is_empty() {
            let _ = writeln!(out, "=== pictures");
            for file in &self.pictures {
                let _ = writeln!(out, "wrote {}", file.display());
            }
        }
        out
    }
}

/// Copies `UserDbas/` and `Chars/` into a folder beside the data folder;
/// returns that folder.
fn backup(install: &Installation, stamp: &str) -> Result<PathBuf, String> {
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

/// `path`, with each part that is already there in another case spelled
/// as it is on disk: the original's folders are `BITMAPS` on one
/// computer and `Bitmaps` on the next, and its file names ignore case.
fn existing_case(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        let next = out.join(part);
        if next.exists() || out.as_os_str().is_empty() {
            out = next;
            continue;
        }
        let name = part.as_os_str().to_string_lossy();
        let found = fs::read_dir(&out).ok().and_then(|entries| {
            entries
                .filter_map(|e| e.ok())
                .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(&name))
                .map(|e| e.path())
        });
        out = found.unwrap_or(next);
    }
    out
}

/// The seconds since 1970 of a date and time, as a ZIP archive keeps them.
fn seconds(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> i64 {
    // Howard Hinnant's days_from_civil.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    ((days * 24 + hour) * 60 + minute) * 60 + second
}

/// `2026-10-29-232615`: the time, in UTC, in the names of the backup and
/// the log.
fn stamp(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}-{:02}{:02}{:02}",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_stamps_agree() {
        let at = seconds(2026, 10, 29, 23, 26, 15);
        assert_eq!(at, 1_793_316_375);
        let time = UNIX_EPOCH + std::time::Duration::from_secs(at as u64);
        assert_eq!(stamp(time), "2026-10-29-232615");
        assert_eq!(seconds(1970, 1, 1, 0, 0, 0), 0);
    }

    #[test]
    fn files_are_told_by_their_extension() {
        assert_eq!(Type::of("Barbarian.XPX"), Type::Package);
        assert_eq!(Type::of("old.xpt"), Type::Package);
        assert_eq!(Type::of("Witch.ZIP"), Type::Archive);
        assert_eq!(Type::of("troll.bmp"), Type::Picture);
        assert_eq!(Type::of("readme.txt"), Type::Other);
        assert_eq!(Type::of("README"), Type::Other);
        assert_eq!(Type::of("Pictures.TSV"), Type::Placements);
    }

    #[test]
    fn a_placement_names_its_picture_from_its_folder() {
        assert_eq!(sibling("pictures.tsv", "a/b.bmp"), "a/b.bmp");
        assert_eq!(sibling("x/pictures.tsv", "a/b.bmp"), "x/a/b.bmp");
        assert_eq!(
            sibling("x/y.zip › pictures.tsv", "a/b.bmp"),
            "x/y.zip › a/b.bmp"
        );
        assert_eq!(
            sibling("x/y.zip › Top/pictures.tsv", "a/b.bmp"),
            "x/y.zip › Top/a/b.bmp"
        );
    }

    /// A folder with a package of each kind, the same one again inside an
    /// archive, a damaged one and a text.
    fn folder() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cr-addons-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        let expansion = ExportFile::new(true).to_bytes().unwrap();
        let core20 = ExportFile::new(false).to_bytes().unwrap();
        fs::write(dir.join("empty.xpx"), &expansion).unwrap();
        fs::write(dir.join("sub/empty.xpt"), &core20).unwrap();
        fs::write(dir.join("broken.xpx"), b"not a package").unwrap();
        fs::write(dir.join("readme.txt"), b"hello").unwrap();
        let mut zip = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("inner/Copy.XPX", options).unwrap();
        io::Write::write_all(&mut zip, &expansion).unwrap();
        zip.start_file("inner/notes.doc", options).unwrap();
        zip.start_file("inner/Troll.BMP", options).unwrap();
        io::Write::write_all(&mut zip, b"BM").unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        fs::write(dir.join("sub/pack.zip"), bytes).unwrap();
        dir
    }

    #[test]
    fn the_scan_reads_subfolders_and_archives() {
        let dir = folder();
        let found = scan(&dir, true).unwrap();
        let paths: Vec<&str> = found.found.iter().map(|f| f.path.as_str()).collect();
        assert!(
            paths.contains(&"sub/pack.zip › inner/Copy.XPX"),
            "{paths:?}"
        );
        assert!(paths.contains(&"sub/empty.xpt"), "{paths:?}");
        // Empty packages hold nothing to import; the damaged one is no
        // package.
        assert!(!found.can_import());
        let broken = found.found.iter().find(|f| f.path == "broken.xpx").unwrap();
        assert!(matches!(&broken.state, State::Refused(why) if why.starts_with("Not a package")));
        assert_eq!(broken.expansion, None);
        let core20 = found
            .found
            .iter()
            .find(|f| f.path == "sub/empty.xpt")
            .unwrap();
        assert_eq!(core20.expansion, Some(false));
        assert_eq!(found.pictures(), 1);
        assert_eq!(found.others, 2);
        let (title, text) = found.summary();
        assert_eq!(title, "Nothing to Import");
        assert!(text.contains("Not imported:"), "{text}");
        let _ = fs::remove_dir_all(&dir);
    }

    /// With `CORERULES_DIR` pointing at an Expansion installation and
    /// `CORERULES_ADDONS` at a folder of add-ons, they all go into a copy
    /// of its user databases.
    #[test]
    fn a_folder_of_addons_comes_in_at_once() {
        let (Some(dir), Some(addons)) = (
            std::env::var_os("CORERULES_DIR"),
            std::env::var_os("CORERULES_ADDONS"),
        ) else {
            return;
        };
        let original = Installation::open(Path::new(&dir)).unwrap();
        let root = std::env::temp_dir().join(format!("cr-addons-folder-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let data = root.join("data");
        fs::create_dir_all(&data).unwrap();
        for name in ["UserDbas", "Chars"] {
            if let Some(from) = original.path(name) {
                copy_dir(&from, &data.join(name)).unwrap();
            }
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(original.path("Database").unwrap(), data.join("Database"))
            .unwrap();
        let install = Installation::open(&data).unwrap();
        let found = scan(Path::new(&addons), !install.is_core20()).unwrap();
        let (title, text) = found.summary();
        eprintln!("{title}\n{text}");
        assert!(found.can_import());
        let steps = std::cell::Cell::new(0);
        let done = found
            .import(&install, &|_, _, _| steps.set(steps.get() + 1))
            .unwrap();
        let (heading, report) = done.report();
        eprintln!("{heading}\n{report}");
        assert_eq!(
            done.packages.len() + done.failed.len(),
            found.ready().count()
        );
        assert!(done.backup.as_ref().is_some_and(|b| b.exists()));
        assert!(done.log.as_ref().is_some_and(|l| l.exists()));
        // A second time, the characters are in the list already.
        let again = found.import(&install, &|_, _, _| {}).unwrap();
        let characters: usize = again.packages.iter().map(|(_, r)| r.characters.len()).sum();
        assert_eq!(characters, 0);
        let _ = fs::remove_dir_all(&root);
    }
}
