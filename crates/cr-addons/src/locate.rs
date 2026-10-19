//! Finding the Core Rules app on this computer, however it was installed,
//! and the data folder each copy reads: the .deb, .rpm and AppImage use
//! `~/.local/share/corerules/data`, the Flatpak its own copy under
//! `~/.var/app`, and each falls back to the original program in the Wine
//! prefix, as `Installation::discover` does.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cr_i18n::tr;
use cr_store::{DEFAULT_WINE_PATH, Installation};

const APP_ID: &str = "io.github.brahm.CoreRules";

/// How a copy of the app was installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Flatpak,
    Deb,
    Rpm,
    /// A program in the system path that no package manager owns.
    Other(PathBuf),
    AppImage(PathBuf),
}

/// One copy of the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub kind: Kind,
    pub version: Option<String>,
    /// The folder this copy opens: its data folder, else the Wine prefix
    /// when the original program is installed there.
    pub data: PathBuf,
    /// Whether `data` is the original program in the Wine prefix.
    pub wine: bool,
    /// Whether `data` holds Core Rules data yet.
    pub ready: bool,
}

impl App {
    pub fn label(&self) -> String {
        let how = match &self.kind {
            Kind::Flatpak => "Flatpak".to_string(),
            Kind::Deb => tr(".deb package").to_string(),
            Kind::Rpm => tr(".rpm package").to_string(),
            Kind::Other(path) => path.display().to_string(),
            Kind::AppImage(path) => match path.parent() {
                Some(dir) => tr!("AppImage in {folder}", folder = crate::home_relative(dir)),
                None => "AppImage".to_string(),
            },
        };
        match &self.version {
            Some(v) => format!("Core Rules {v} ({how})"),
            None => format!("Core Rules ({how})"),
        }
    }

    /// Whether this version reads every add-on: 0.7.2 learned the classes
    /// the patched Expansion writes.
    pub fn reads_addons(&self) -> bool {
        self.version
            .as_deref()
            .is_none_or(|v| version_at_least(v, (0, 7, 2)))
    }

    /// Starts this copy of the app, outside the installer's own AppImage.
    pub fn launch(&self) -> std::io::Result<()> {
        let mut command = match &self.kind {
            Kind::Flatpak => {
                let mut c = Command::new("flatpak");
                c.args(["run", APP_ID]);
                c
            }
            Kind::Deb | Kind::Rpm => Command::new("corerules"),
            Kind::Other(path) | Kind::AppImage(path) => Command::new(path),
        };
        clean_env(&mut command);
        command.spawn().map(|_| ())
    }
}

/// Where to look; the real values come from [`Places::here`], tests build
/// their own.
pub struct Places {
    pub home: PathBuf,
    /// `$XDG_DATA_HOME`, when set.
    pub data_home: Option<PathBuf>,
    /// `$WINEPREFIX`, when set.
    pub wine_prefix: Option<PathBuf>,
    /// The folders of `$PATH`, without the installer's own.
    pub path: Vec<PathBuf>,
    /// The system root, `/`.
    pub root: PathBuf,
    /// Folders where people keep AppImages.
    pub appimage_dirs: Vec<PathBuf>,
}

impl Places {
    pub fn here() -> Self {
        let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
        let own = std::env::var_os("APPDIR").map(PathBuf::from);
        let path = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter(|d| own.as_ref().is_none_or(|o| !d.starts_with(o)))
            .collect();
        let mut appimage_dirs = vec![
            home.join("Applications"),
            home.join("AppImages"),
            home.join(".local/bin"),
            home.join("bin"),
            home.join("Downloads"),
            home.join("Desktop"),
            home.clone(),
            PathBuf::from("/opt"),
        ];
        for dir in [
            gtk::glib::UserDirectory::Downloads,
            gtk::glib::UserDirectory::Desktop,
        ] {
            if let Some(d) = gtk::glib::user_special_dir(dir) {
                appimage_dirs.push(d);
            }
        }
        // The installer's own folder: it is often downloaded beside the app.
        if let Some(own) = std::env::var_os("APPIMAGE").map(PathBuf::from)
            && let Some(dir) = own.parent()
        {
            appimage_dirs.push(dir.to_path_buf());
        }
        Self {
            home,
            data_home: std::env::var_os("XDG_DATA_HOME")
                .filter(|d| !d.is_empty())
                .map(PathBuf::from),
            wine_prefix: std::env::var_os("WINEPREFIX")
                .filter(|d| !d.is_empty())
                .map(PathBuf::from),
            path,
            root: PathBuf::from("/"),
            appimage_dirs,
        }
    }

    /// The data folder of the .deb, .rpm and AppImage.
    fn native_data(&self) -> PathBuf {
        self.data_home
            .clone()
            .unwrap_or_else(|| self.home.join(".local/share"))
            .join("corerules/data")
    }

    /// The data folder of the Flatpak, whose `$XDG_DATA_HOME` is its own.
    fn flatpak_data(&self) -> PathBuf {
        self.home
            .join(".var/app")
            .join(APP_ID)
            .join("data/corerules/data")
    }

    fn wine(&self, prefix: Option<&Path>) -> PathBuf {
        prefix
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.home.join(".wine"))
            .join(DEFAULT_WINE_PATH)
    }
}

/// Every copy of the app found, the Flatpak first.
pub fn apps(places: &Places) -> Vec<App> {
    let mut found = Vec::new();
    let native = places.native_data();
    // The Flatpak does not see $WINEPREFIX; it looks in ~/.wine.
    for base in [
        places.home.join(".local/share/flatpak"),
        places.root.join("var/lib/flatpak"),
    ] {
        let active = base.join("app").join(APP_ID).join("current/active");
        if active.join("files").is_dir() {
            let version = metainfo_version(&active.join("files/share/metainfo"));
            found.push(app(
                Kind::Flatpak,
                version,
                &places.flatpak_data(),
                &places.wine(None),
            ));
            break;
        }
    }
    let wine = places.wine(places.wine_prefix.as_deref());
    if let Some(bin) = places
        .path
        .iter()
        .map(|d| d.join("corerules"))
        .find(|b| b.is_file())
    {
        let prefix = bin.parent().and_then(Path::parent).unwrap_or(&places.root);
        let version = metainfo_version(&prefix.join("share/metainfo"));
        let kind = if places
            .root
            .join("var/lib/dpkg/info/corerules.list")
            .exists()
        {
            Kind::Deb
        } else if prefix == places.root.join("usr") && rpm_owns(&bin) {
            Kind::Rpm
        } else {
            Kind::Other(bin.clone())
        };
        found.push(app(kind, version, &native, &wine));
    }
    for image in appimages(places) {
        let version = appimage_version(&image);
        found.push(app(Kind::AppImage(image), version, &native, &wine));
    }
    found
}

fn app(kind: Kind, version: Option<String>, data: &Path, wine: &Path) -> App {
    let has = |dir: &Path| Installation::open(dir).is_ok();
    let (data, wine, ready) = if has(data) {
        (data.to_path_buf(), false, true)
    } else if has(wine) {
        (wine.to_path_buf(), true, true)
    } else {
        (data.to_path_buf(), false, false)
    };
    App {
        kind,
        version,
        data,
        wine,
        ready,
    }
}

/// `CoreRules-0.8.0-x86_64.AppImage` files, newest version first; the
/// installer's own (`CoreRules-Addons-…`) is not one.
fn appimages(places: &Places) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    for dir in &places.appimage_dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            if lower.starts_with("corerules")
                && !lower.starts_with("corerules-addons")
                && lower.ends_with(".appimage")
                && entry.path().is_file()
            {
                let path = entry.path();
                let real = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                if !found
                    .iter()
                    .any(|f| fs::canonicalize(f).unwrap_or_else(|_| f.clone()) == real)
                {
                    found.push(path);
                }
            }
        }
    }
    found.sort_by_key(|p| {
        std::cmp::Reverse(
            appimage_version(p)
                .map(|v| version_key(&v))
                .unwrap_or_default(),
        )
    });
    found
}

/// `CoreRules-0.8.0-x86_64.AppImage` → `0.8.0`.
fn appimage_version(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy();
    name.split('-')
        .find(|part| part.starts_with(|c: char| c.is_ascii_digit()) && part.contains('.'))
        .map(str::to_string)
}

/// The newest `<release version="…">` of the app's metainfo.
fn metainfo_version(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(dir.join(format!("{APP_ID}.metainfo.xml"))).ok()?;
    let start = text.find("<release version=\"")? + "<release version=\"".len();
    let end = text[start..].find('"')?;
    Some(text[start..start + end].to_string())
}

fn rpm_owns(bin: &Path) -> bool {
    let mut command = Command::new("rpm");
    command.arg("-qf").arg(bin);
    clean_env(&mut command);
    command.output().is_ok_and(|o| o.status.success())
}

fn version_key(version: &str) -> (u32, u32, u32) {
    let mut parts = version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .map(|p| p.parse().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn version_at_least(version: &str, wanted: (u32, u32, u32)) -> bool {
    version_key(version) >= wanted
}

/// Takes out of a child's environment what the installer's AppImage set
/// up for itself (its libraries, schemas and data folders), so the app it
/// starts uses its own.
fn clean_env(command: &mut Command) {
    let Some(own) = std::env::var_os("APPDIR").map(PathBuf::from) else {
        return;
    };
    for (key, value) in std::env::vars_os() {
        let parts: Vec<PathBuf> = std::env::split_paths(&value).collect();
        if !parts.iter().any(|p| p.starts_with(&own)) {
            continue;
        }
        let kept: Vec<PathBuf> = parts.into_iter().filter(|p| !p.starts_with(&own)).collect();
        match std::env::join_paths(&kept) {
            Ok(joined) if !kept.is_empty() => command.env(&key, joined),
            _ => command.env_remove(&key),
        };
    }
    for key in ["APPDIR", "APPIMAGE", "ARGV0", "OWD"] {
        command.env_remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        dir: PathBuf,
    }

    impl Fake {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("cr-addons-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("home")).unwrap();
            Self { dir }
        }

        fn places(&self) -> Places {
            let home = self.dir.join("home");
            Places {
                appimage_dirs: vec![home.join("Applications"), home.join("Downloads")],
                home,
                data_home: None,
                wine_prefix: None,
                path: vec![self.dir.join("usr/bin")],
                root: self.dir.clone(),
            }
        }

        fn file(&self, relative: &str, text: &str) {
            let path = self.dir.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }

        /// A data folder `Installation::open` accepts, of the Expansion.
        fn data(&self, relative: &str) {
            self.file(&format!("{relative}/Database/Spells.dat"), "");
            self.file(&format!("{relative}/UserDbas/AllowedU.dat"), "");
        }
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn metainfo(version: &str) -> String {
        format!(
            "<component><releases><release version=\"{version}\" date=\"x\"/></releases></component>"
        )
    }

    #[test]
    fn the_flatpak_reads_its_own_data_folder() {
        let fake = Fake::new("flatpak");
        let active = "home/.local/share/flatpak/app/io.github.brahm.CoreRules/current/active";
        fake.file(
            &format!("{active}/files/share/metainfo/io.github.brahm.CoreRules.metainfo.xml"),
            &metainfo("0.8.0"),
        );
        fake.data("home/.var/app/io.github.brahm.CoreRules/data/corerules/data");
        let found = apps(&fake.places());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, Kind::Flatpak);
        assert_eq!(found[0].version.as_deref(), Some("0.8.0"));
        assert!(found[0].ready && !found[0].wine);
        assert!(
            found[0]
                .data
                .ends_with(".var/app/io.github.brahm.CoreRules/data/corerules/data")
        );
        assert_eq!(found[0].label(), "Core Rules 0.8.0 (Flatpak)");
    }

    #[test]
    fn a_deb_reads_the_shared_data_folder_or_the_wine_prefix() {
        let fake = Fake::new("deb");
        fake.file("usr/bin/corerules", "");
        fake.file(
            "usr/share/metainfo/io.github.brahm.CoreRules.metainfo.xml",
            &metainfo("0.7.1"),
        );
        fake.file("var/lib/dpkg/info/corerules.list", "");
        fake.data("home/.wine/drive_c/Program Files/AD&D Core Rules 2.0");
        let found = apps(&fake.places());
        assert_eq!(found[0].kind, Kind::Deb);
        assert!(found[0].ready && found[0].wine);
        assert!(!found[0].reads_addons());

        fake.data("home/.local/share/corerules/data");
        let found = apps(&fake.places());
        assert!(found[0].ready && !found[0].wine);
        assert!(found[0].data.ends_with(".local/share/corerules/data"));
    }

    #[test]
    fn appimages_are_found_by_name_and_share_one_folder() {
        let fake = Fake::new("appimage");
        fake.file("home/Downloads/CoreRules-0.7.2-x86_64.AppImage", "");
        fake.file("home/Applications/CoreRules-0.8.0-x86_64.AppImage", "");
        fake.file("home/Downloads/CoreRules-Addons-0.8.0-x86_64.AppImage", "");
        fake.data("home/.local/share/corerules/data");
        let found = apps(&fake.places());
        let versions: Vec<_> = found.iter().map(|a| a.version.clone().unwrap()).collect();
        assert_eq!(versions, ["0.8.0", "0.7.2"]);
        assert_eq!(found[0].data, found[1].data);
    }

    #[test]
    fn nothing_installed_finds_nothing() {
        let fake = Fake::new("none");
        assert!(apps(&fake.places()).is_empty());
    }

    #[test]
    fn a_copy_without_data_is_not_a_target() {
        let fake = Fake::new("nodata");
        fake.file("usr/bin/corerules", "");
        let found = apps(&fake.places());
        assert_eq!(found.len(), 1);
        assert!(!found[0].ready);
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(version_at_least("0.10.0", (0, 7, 2)));
        assert!(version_at_least("0.7.2", (0, 7, 2)));
        assert!(!version_at_least("0.7.1", (0, 7, 2)));
        assert_eq!(
            appimage_version(Path::new("CoreRules-0.8.0-x86_64.AppImage")).as_deref(),
            Some("0.8.0")
        );
    }
}
