//! Extract Data from CD…: copies what the app uses out of a Core Rules 2.0
//! or Expansion CD (its ISO image or the mounted CD) or out of an
//! installation into a data package, so the app runs without the CD or
//! Wine.
//!
//! The package holds the rules databases, user databases and characters,
//! the books, the Table Rolls tables, the two program files whose texts the
//! app reads (Table Rolls, NPC names) and the contents files of the help
//! books. It holds TSR/Wizards of the Coast content: it is for the user's
//! own use.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use cr_i18n::tr;

use crate::iso9660::{self, Image};
use crate::{DEFAULT_WINE_PATH, Error, Result, data_package_dir};

/// Which program the data comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    /// Core Rules 2.0 (`Program/Evermore.exe`).
    Core20,
    /// The Expansion (`Program/Tsrp3.exe`).
    Expansion,
}

impl Edition {
    pub fn id(self) -> &'static str {
        match self {
            Edition::Core20 => "core20",
            Edition::Expansion => "expansion",
        }
    }
}

/// A file or folder of the source.
#[derive(Debug, Clone)]
enum Node {
    Path(PathBuf),
    Iso(iso9660::Entry),
}

/// Where the data is read from: a folder (a mounted CD, an installation
/// or a Wine prefix holding one) or an ISO image.
enum Tree {
    Folder,
    Iso(Image),
}

impl Tree {
    fn is_dir(&self, node: &Node) -> bool {
        match node {
            Node::Path(p) => p.is_dir(),
            Node::Iso(e) => e.is_dir,
        }
    }

    fn name(node: &Node) -> String {
        match node {
            Node::Path(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            Node::Iso(e) => e.name.clone(),
        }
    }

    fn list(&self, dir: &Node) -> io::Result<Vec<Node>> {
        match (self, dir) {
            (Tree::Iso(image), Node::Iso(e)) => {
                Ok(image.list(e)?.into_iter().map(Node::Iso).collect())
            }
            (_, Node::Path(p)) => {
                let mut nodes: Vec<Node> = fs::read_dir(p)?
                    .map(|e| e.map(|e| Node::Path(e.path())))
                    .collect::<io::Result<_>>()?;
                nodes.sort_by_key(Tree::name);
                Ok(nodes)
            }
            _ => unreachable!("a folder node in an image"),
        }
    }

    /// The entry named `name` in `dir`, without regard to case.
    fn entry(&self, dir: &Node, name: &str) -> Option<Node> {
        if let Node::Path(p) = dir {
            return crate::find_entry(p, name).map(Node::Path);
        }
        self.list(dir)
            .ok()?
            .into_iter()
            .find(|n| Tree::name(n).eq_ignore_ascii_case(name))
    }

    /// Follows `relative` from `dir`, matching each part without regard to case.
    fn find(&self, dir: &Node, relative: &str) -> Option<Node> {
        relative
            .split('/')
            .try_fold(dir.clone(), |d, name| self.entry(&d, name))
    }

    fn copy_file(&self, node: &Node, to: &Path) -> io::Result<()> {
        // Written fresh rather than with fs::copy, whose copy of the
        // permissions would leave files from a CD read-only.
        let mut out = File::create(to)?;
        match (self, node) {
            (Tree::Iso(image), Node::Iso(e)) => io::Write::write_all(&mut out, &image.read(e)?),
            (_, Node::Path(p)) => io::copy(&mut File::open(p)?, &mut out).map(|_| ()),
            _ => unreachable!("a folder node in an image"),
        }
    }

    /// Copies a folder and everything in it, leaving out the `.bak`
    /// backups an earlier edit left.
    fn copy_tree(&self, node: &Node, to: &Path) -> Result<()> {
        if !self.is_dir(node) {
            return self
                .copy_file(node, to)
                .map_err(|source| io_error(to, source));
        }
        fs::create_dir_all(to).map_err(|source| io_error(to, source))?;
        for child in self.list(node).map_err(|source| io_error(to, source))? {
            let name = Tree::name(&child);
            if !self.is_dir(&child) && name.ends_with(".bak") {
                continue;
            }
            self.copy_tree(&child, &to.join(name))?;
        }
        Ok(())
    }
}

fn io_error(path: &Path, source: io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// A source found to hold Core Rules data, ready to be copied.
pub struct Extraction {
    source: PathBuf,
    tree: Tree,
    /// The folder holding `Database`.
    root: Node,
    edition: Edition,
}

/// Where [`Extraction::install`] put the package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub path: PathBuf,
    /// Whether it is the app's data folder, which the app opens on start.
    pub is_data_folder: bool,
    /// Where the Core Rules 2.0 package that was in the data folder went.
    pub moved: Option<PathBuf>,
}

impl Extraction {
    /// Looks at `source`: an ISO image of a Core Rules CD, the folder of a
    /// mounted CD, an installation (the folder holding `Database`) or a
    /// Wine prefix with Core Rules installed in the default place.
    pub fn probe(source: &Path) -> Result<Self> {
        let (tree, top) = if source.is_file() {
            if !Image::is_image(source) {
                return Err(Error::Invalid(tr!(
                    "{path} is neither a folder nor a CD image (.iso)",
                    path = source.display()
                )));
            }
            let image = Image::open(source).map_err(|e| io_error(source, e))?;
            let root = Node::Iso(image.root().clone());
            (Tree::Iso(image), root)
        } else {
            (Tree::Folder, Node::Path(source.to_path_buf()))
        };
        let root = [None, Some(DEFAULT_WINE_PATH)]
            .into_iter()
            .filter_map(|sub| match sub {
                None => Some(top.clone()),
                Some(sub) => tree.find(&top, sub),
            })
            .find(|root| tree.entry(root, "Database").is_some())
            .ok_or_else(|| Error::NotAnInstallation(source.to_path_buf()))?;
        let edition = match tree.find(&root, "Program/Tsrp3.exe") {
            Some(_) => Edition::Expansion,
            None => Edition::Core20,
        };
        Ok(Self {
            source: source.to_path_buf(),
            tree,
            root,
            edition,
        })
    }

    pub fn edition(&self) -> Edition {
        self.edition
    }

    /// `corerules-data-core20` or `corerules-data-expansion`.
    pub fn package_name(&self) -> String {
        format!("corerules-data-{}", self.edition.id())
    }

    /// Writes the package where the app finds it: into the app's data
    /// folder while it is free, else into a folder named after the package
    /// next to it (`~/.local/share/corerules/corerules-data-core20`).
    ///
    /// The Expansion is where the app makes and changes things, so it takes
    /// the data folder over from a Core Rules 2.0 package, which moves next
    /// to it under its package name.
    pub fn install(&self) -> Result<Destination> {
        let data = data_package_dir()
            .ok_or_else(|| Error::Invalid(tr("The home folder is not known").to_owned()))?;
        let beside = |name: &str| data.with_file_name(name);
        if !data.exists() {
            self.write(&data)?;
            return Ok(Destination {
                path: data,
                is_data_folder: true,
                moved: None,
            });
        }
        let path = beside(&self.package_name());
        self.write(&path)?;
        let aside = beside(&format!("corerules-data-{}", Edition::Core20.id()));
        let replaces_core20 = self.edition == Edition::Expansion
            && crate::Installation::open(&data).is_ok_and(|i| i.is_core20())
            && !aside.exists();
        if replaces_core20 {
            fs::rename(&data, &aside).map_err(|e| io_error(&data, e))?;
            fs::rename(&path, &data).map_err(|e| io_error(&data, e))?;
            return Ok(Destination {
                path: data,
                is_data_folder: true,
                moved: Some(aside),
            });
        }
        Ok(Destination {
            path,
            is_data_folder: false,
            moved: None,
        })
    }

    /// The source as the package's README names it.
    fn root_display(&self) -> String {
        match &self.root {
            Node::Path(p) => p.display().to_string(),
            Node::Iso(_) => self.source.display().to_string(),
        }
    }

    /// Writes the package into the folder `dest`, which must not exist.
    /// It is put together next to `dest` and renamed into place, so an
    /// interrupted copy leaves nothing half-written at `dest`.
    pub fn write(&self, dest: &Path) -> Result<()> {
        if dest.exists() {
            return Err(Error::Invalid(tr!(
                "{path} already exists; move it away first",
                path = dest.display()
            )));
        }
        let name = dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stage = dest.with_file_name(format!(".{name}.partial"));
        if stage.exists() {
            fs::remove_dir_all(&stage).map_err(|e| io_error(&stage, e))?;
        }
        let written = self
            .write_into(&stage)
            .and_then(|()| fs::rename(&stage, dest).map_err(|e| io_error(dest, e)));
        if written.is_err() {
            let _ = fs::remove_dir_all(&stage);
        }
        written
    }

    fn write_into(&self, dest: &Path) -> Result<()> {
        let tree = &self.tree;
        let mkdir = |dir: &Path| fs::create_dir_all(dir).map_err(|e| io_error(dir, e));
        mkdir(dest)?;
        // Rules databases, user databases and characters.
        for dir in ["Database", "UserDbas", "Chars"] {
            match tree.entry(&self.root, dir) {
                Some(found) => tree.copy_tree(&found, &dest.join(dir))?,
                None => mkdir(&dest.join(dir))?,
            }
        }
        // The book reader and Table Rolls.
        for dir in ["Books", "Tables"] {
            if let Some(found) = tree.entry(&self.root, dir) {
                tree.copy_tree(&found, &dest.join(dir))?;
            }
        }
        // Texts the original keeps inside its program (Table Rolls, NPC names).
        if let Some(program) = tree.entry(&self.root, "Program") {
            let to = dest.join("Program");
            mkdir(&to)?;
            for exe in ["Tsrp3.exe", "Evermore.exe"] {
                if let Some(found) = tree.entry(&program, exe) {
                    let path = to.join(Tree::name(&found));
                    tree.copy_file(&found, &path)
                        .map_err(|e| io_error(&path, e))?;
                }
            }
        }
        // Contents files of the help books (book outlines).
        if let Some(help) = tree.entry(&self.root, "Help") {
            let to = dest.join("Help");
            mkdir(&to)?;
            for file in tree.list(&help).map_err(|e| io_error(&to, e))? {
                let name = Tree::name(&file);
                let is_contents = name
                    .rsplit_once('.')
                    .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("cnt"));
                if is_contents && !tree.is_dir(&file) {
                    let path = to.join(name);
                    tree.copy_file(&file, &path)
                        .map_err(|e| io_error(&path, e))?;
                }
            }
        }
        let date = crate::sheet::Timestamp::now_utc();
        let readme = format!(
            "Core Rules data package ({edition}), made by Core Rules from\n\
             {root}\n\
             on {year:04}-{month:02}-{day:02}.\n\
             \n\
             It holds TSR/Wizards of the Coast content from your own copy of AD&D Core\n\
             Rules 2.0. Keep it for your own use; do not redistribute it.\n\
             \n\
             The app finds it in ~/.local/share/corerules/data (or open this folder with\n\
             \"Open Folder…\"). Characters and user databases you edit are saved here.\n",
            edition = self.edition.id(),
            root = self.root_display(),
            year = date.year,
            month = date.month,
            day = date.day,
        );
        let path = dest.join("README.txt");
        fs::write(&path, readme).map_err(|e| io_error(&path, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_what_the_app_uses_from_a_folder() {
        let base = std::env::temp_dir().join(format!("cr-extract-test-{}", std::process::id()));
        let cd = base.join("cd");
        for dir in [
            "DATABASE",
            "USERDBAS",
            "TABLES/SUB",
            "PROGRAM",
            "HELP/X",
            "SOUNDS",
        ] {
            fs::create_dir_all(cd.join(dir)).unwrap();
        }
        for file in [
            "DATABASE/RACE.DAT",
            "DATABASE/RACE.DAT.bak",
            "USERDBAS/ALLOWEDU.DAT",
            "TABLES/SUB/DMG1.TBL",
            "PROGRAM/TSRP3.EXE",
            "PROGRAM/OTHER.DLL",
            "HELP/PHB.CNT",
            "HELP/PHB.HLP",
            "SOUNDS/A.WAV",
        ] {
            fs::write(cd.join(file), file).unwrap();
        }
        let extraction = Extraction::probe(&cd).unwrap();
        assert_eq!(extraction.edition(), Edition::Expansion);
        let out = base.join("out");
        extraction.write(&out).unwrap();

        let mut found = Vec::new();
        fn walk(dir: &Path, base: &Path, found: &mut Vec<String>) {
            for e in fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
                if p.is_dir() {
                    found.push(format!("{rel}/"));
                    walk(&p, base, found);
                } else {
                    found.push(rel);
                }
            }
        }
        walk(&out, &out, &mut found);
        found.sort();
        assert_eq!(
            found,
            [
                "Chars/",
                "Database/",
                "Database/RACE.DAT",
                "Help/",
                "Help/PHB.CNT",
                "Program/",
                "Program/TSRP3.EXE",
                "README.txt",
                "Tables/",
                "Tables/SUB/",
                "Tables/SUB/DMG1.TBL",
                "UserDbas/",
                "UserDbas/ALLOWEDU.DAT",
            ]
        );
        assert_eq!(
            fs::read_to_string(out.join("Tables/SUB/DMG1.TBL")).unwrap(),
            "TABLES/SUB/DMG1.TBL"
        );
        assert!(
            extraction.write(&out).is_err(),
            "an existing folder is kept"
        );
        assert!(matches!(
            Extraction::probe(&base.join("out/Chars")),
            Err(Error::NotAnInstallation(_))
        ));
        fs::remove_dir_all(base).unwrap();
    }
}
