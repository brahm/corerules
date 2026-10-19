//! The pictures records name: a monster type's bitmap, a race's or a
//! character's portrait.
//!
//! The original stores the Windows path the picture had on the computer
//! that made the record (`C:\Program Files\AD&D Core Rules 2.0\Bitmaps\
//! Monsters\vaprak.bmp`, `G:\Bitmaps\Portrait\MALE1.bmp` on the CD, `C:\AD&D
//! Pics\troll.bmp`), or only a file name for the shipped pictures
//! (`6.bmp`). [`relative`] turns either into a path inside the
//! installation, the same one the add-ons installer writes the picture to.
//!
//! A character's portrait, a race's image and a monster type's picture
//! are picked from the shipped pictures ([`Installation::picture_choices`]),
//! or made from a picture of the user's ([`Installation::add_picture`]): a
//! BMP, JPEG, GIF or PNG cut to the shipped pictures' size and kept as an
//! 8-bit bitmap in the original's palette, so the original shows it too.
//! Portraits go to `Bitmaps/Portrait`, as races and characters keep only
//! a file name there; monster pictures go to `UserBMP`, the original's
//! folder for custom bitmaps.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use cr_formats::Object;

use crate::{Error, Installation, Result};

/// Where a bare file name is looked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `Bitmaps/Monsters`, the monster types' pictures.
    Monster,
    /// `Bitmaps/Portrait`, the races' and characters' portraits.
    Portrait,
    /// `Bitmaps/Equip`, the items' icons, in strips (see [`Icon`]).
    Item,
}

impl Kind {
    /// The folder of the shipped pictures.
    pub fn folder(self) -> &'static str {
        match self {
            Kind::Monster => "Bitmaps/Monsters",
            Kind::Portrait => "Bitmaps/Portrait",
            Kind::Item => "Bitmaps/Equip",
        }
    }

    /// The shipped pictures' size, which pictures a user adds are cut to.
    pub fn size(self) -> (u32, u32) {
        match self {
            Kind::Monster => (300, 300),
            Kind::Portrait => (PORTRAIT_WIDTH, PORTRAIT_HEIGHT),
            Kind::Item => (ICON_SIZE, ICON_SIZE),
        }
    }

    /// The folder pictures a user adds go to.
    fn user_folder(self) -> &'static str {
        match self {
            Kind::Monster | Kind::Item => "UserBMP",
            Kind::Portrait => "Bitmaps/Portrait",
        }
    }
}

/// The installation folder's name in the stored paths, whatever the drive
/// or the localized `Program Files` before it.
const ROOT_NAME: &str = "AD&D Core Rules 2.0";

/// The installation folder as the original writes it in the paths it
/// stores.
const ROOT_PATH: &str = "C:\\Program Files\\AD&D Core Rules 2.0\\";

/// The path inside an installation, with `/` between the parts, of the
/// picture a record stores:
///
/// * a path inside a Core Rules installation, on any drive, keeps what
///   follows the installation folder (`Bitmaps/Monsters/vaprak.bmp`);
/// * a path into the CD's `Bitmaps` keeps it (`Bitmaps/Portrait/MALE1.bmp`);
/// * any other path goes under `UserBMP`, without the drive
///   (`UserBMP/AD&D Pics/troll.bmp`);
/// * a bare file name is in the shipped folder for `kind`.
///
/// `None` for an empty path.
pub fn relative(stored: &str, kind: Kind) -> Option<String> {
    let parts: Vec<&str> = stored
        .trim()
        .split(['\\', '/'])
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    let (&file, folders) = parts.split_last()?;
    let mut out: Vec<&str> = match folders.first() {
        None => kind.folder().split('/').collect(),
        Some(drive) if drive.ends_with(':') => {
            let rest = &folders[1..];
            if let Some(at) = rest.iter().position(|p| p.eq_ignore_ascii_case(ROOT_NAME)) {
                rest[at + 1..].to_vec()
            } else if rest
                .first()
                .is_some_and(|p| p.eq_ignore_ascii_case("Bitmaps"))
            {
                rest.to_vec()
            } else {
                std::iter::once("UserBMP")
                    .chain(rest.iter().copied())
                    .collect()
            }
        }
        Some(_) => folders.to_vec(),
    };
    out.push(file);
    Some(out.join("/"))
}

/// The picture a record stores, combined from a folder and a file name as
/// races and characters keep them; either may be empty.
pub fn joined(folder: &str, file: &str) -> String {
    match (folder.trim_end_matches(['\\', '/']), file) {
        (_, "") => String::new(),
        ("", file) => file.to_owned(),
        (folder, file) => format!("{folder}\\{file}"),
    }
}

/// Finds pictures in an installation without regard to case, listing
/// each folder once.
pub struct Finder {
    root: PathBuf,
    /// Each folder looked in, by its relative path in lower case: its
    /// files by name in lower case.
    folders: HashMap<String, Option<HashMap<String, PathBuf>>>,
}

impl Finder {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_owned(),
            folders: HashMap::new(),
        }
    }

    /// The file for a path [`relative`] made, when it is there.
    pub fn find(&mut self, relative: &str) -> Option<PathBuf> {
        let (folder, file) = relative.rsplit_once('/').unwrap_or(("", relative));
        let root = &self.root;
        self.folders
            .entry(folder.to_lowercase())
            .or_insert_with(|| {
                let dir = folder
                    .split('/')
                    .filter(|p| !p.is_empty())
                    .try_fold(root.clone(), |dir, name| crate::find_entry(&dir, name))?;
                Some(
                    fs::read_dir(dir)
                        .ok()?
                        .filter_map(|e| e.ok())
                        .map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.path()))
                        .collect(),
                )
            })
            .as_ref()?
            .get(&file.to_lowercase())
            .cloned()
    }

    /// The file of a picture a record stores, when it is there.
    pub fn picture(&mut self, stored: &str, kind: Kind) -> Option<PathBuf> {
        self.find(&relative(stored, kind)?)
    }
}

/// The size of the shipped portraits, which pictures a user adds are cut
/// to.
pub const PORTRAIT_WIDTH: u32 = 124;
pub const PORTRAIT_HEIGHT: u32 = 125;

/// A picture a record may have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureChoice {
    /// The file name.
    pub file: String,
    pub path: PathBuf,
    /// Whether it comes first: a portrait the character's race offers, a
    /// monster picture the user added.
    pub first: bool,
}

/// What a character keeps for a portrait in `Bitmaps/Portrait`: the folder
/// as the original writes it, and the file name.
pub fn stored_portrait(file: &str) -> [String; 2] {
    [
        crate::userdb::races::PORTRAIT_FOLDER.to_owned(),
        file.to_owned(),
    ]
}

/// What a record keeps for the picture at `path`, inside the installation
/// at `root`: the Windows path the original writes for it, under
/// `C:\Program Files\AD&D Core Rules 2.0`. `None` for a path outside.
pub fn stored_path(root: &Path, path: &Path) -> Option<String> {
    let inside = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = inside
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    (!parts.is_empty()).then(|| format!("{ROOT_PATH}{}", parts.join("\\")))
}

/// A stored path's folder, with its last backslash, and file name.
pub fn split_stored(stored: &str) -> (&str, &str) {
    match stored.rfind(['\\', '/']) {
        Some(at) => (&stored[..=at], &stored[at + 1..]),
        None => ("", stored),
    }
}

/// A bitmap of `kind` made from a BMP, JPEG, GIF or PNG picture: cut to the
/// shipped pictures' proportions around its middle, scaled to their size
/// and kept, as they are, in 8 bits. With `palette`, the colors are those
/// (see [`Installation::program_palette`]), dithered; without, the
/// picture's own 256. `None` for any other file.
pub fn picture_bitmap(data: &[u8], kind: Kind, palette: Option<&[[u8; 3]]>) -> Option<Vec<u8>> {
    use image::ImageFormat;
    let format = image::guess_format(data).ok()?;
    if !matches!(
        format,
        ImageFormat::Bmp | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::Png
    ) {
        return None;
    }
    let picture = image::load_from_memory_with_format(data, format).ok()?;
    let (width, height) = kind.size();
    let mut rgba = picture
        .resize_to_fill(width, height, image::imageops::FilterType::Lanczos3)
        .to_rgba8()
        .into_raw();
    // Transparent parts go over white.
    for p in rgba.as_chunks_mut::<4>().0 {
        let alpha = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = ((u32::from(*c) * alpha + 255 * (255 - alpha)) / 255) as u8;
        }
        p[3] = 255;
    }
    let (palette, indices) = match palette.filter(|p| !p.is_empty()) {
        Some(palette) => (palette.to_vec(), dither(&rgba, width as usize, palette)),
        None => {
            let quant = color_quant::NeuQuant::new(1, 256, &rgba);
            let indices = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| quant.index_of(p) as u8)
                .collect();
            let palette = quant.color_map_rgb().as_chunks::<3>().0.to_vec();
            (palette, indices)
        }
    };
    Some(cr_formats::bmp::encode_indexed(
        width, height, &palette, &indices,
    ))
}

/// A picture's pixels (RGBA, opaque, `width` a row) in the colors of
/// `palette`, with Floyd-Steinberg dithering so gradients do not band.
fn dither(rgba: &[u8], width: usize, palette: &[[u8; 3]]) -> Vec<u8> {
    let mut pixels: Vec<[i32; 3]> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| [p[0].into(), p[1].into(), p[2].into()])
        .collect();
    let mut indices = Vec::with_capacity(pixels.len());
    for at in 0..pixels.len() {
        let want = pixels[at].map(|c| c.clamp(0, 255));
        let (index, got) = palette
            .iter()
            .enumerate()
            .min_by_key(|(_, c)| {
                (0..3)
                    .map(|i| (want[i] - i32::from(c[i])).pow(2))
                    .sum::<i32>()
            })
            .map(|(i, c)| (i, c.map(i32::from)))
            .unwrap();
        indices.push(index as u8);
        let error = [want[0] - got[0], want[1] - got[1], want[2] - got[2]];
        let (x, last) = (at % width, at + width >= pixels.len());
        let mut spread = |to: usize, sixteenths: i32| {
            for i in 0..3 {
                pixels[to][i] += error[i] * sixteenths / 16;
            }
        };
        if x + 1 < width {
            spread(at + 1, 7);
        }
        if !last {
            if x > 0 {
                spread(at + width - 1, 3);
            }
            spread(at + width, 5);
            if x + 1 < width {
                spread(at + width + 1, 1);
            }
        }
    }
    indices
}

/// A file name for a picture made from `source`: its name in letters,
/// digits, spaces, `-` and `_`, with `.bmp`. The letters are those of
/// Latin-1, which the characters' Windows-1252 strings can hold.
fn picture_name(source: &Path) -> String {
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut name = String::new();
    for c in stem.chars() {
        if (c.is_alphanumeric() && (c as u32) < 0x100) || c == '-' || c == '_' {
            name.push(c);
        } else if !name.ends_with(' ') && !name.is_empty() {
            name.push(' ');
        }
        if name.len() >= 32 {
            break;
        }
    }
    match name.trim() {
        "" => "Picture".to_owned(),
        name => name.to_owned(),
    }
}

/// The bitmaps in `dir`, and with `deep` in its folders too.
fn bitmaps_in(dir: &Path, deep: bool, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
        if path.is_dir() {
            if deep {
                bitmaps_in(&path, deep, found);
            }
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("bmp"))
        {
            found.push(path);
        }
    }
}

/// A bitmap's width and height, read from its header.
fn bitmap_size(path: &Path) -> Option<(u32, u32)> {
    let mut header = Vec::with_capacity(26);
    fs::File::open(path)
        .and_then(|f| f.take(26).read_to_end(&mut header))
        .ok()?;
    let width = i32::from_le_bytes(header.get(18..22)?.try_into().ok()?);
    let height = i32::from_le_bytes(header.get(22..26)?.try_into().ok()?);
    Some((width.unsigned_abs(), height.unsigned_abs()))
}

/// Whether a bitmap is a strip of pictures side by side, as the Monstrous
/// Manual's pages are drawn from (`5to34.bmp`, 5100 × 300).
fn is_strip(path: &Path) -> bool {
    bitmap_size(path).is_some_and(|(width, height)| width > 3 * height.max(1))
}

/// The size of an item's icon, square.
pub const ICON_SIZE: u32 = 64;

/// How many icons a strip of `Bitmaps/Equip` holds, side by side.
const ICONS_PER_STRIP: u32 = 50;

/// What an item keeps, before a file name, for an icon of the user's in
/// `UserBMP` (`CRE_espada.bmp`).
pub const CUSTOM_ICON: &str = "CRE_";

/// The part types whose records keep an icon.
pub const ICON_PART_TYPES: [&str; 6] = [
    "Weapons",
    "Armor",
    "Equipment",
    "Magical Items",
    "Gems and Art",
    "Currency Types",
];

/// An item's icon: a square of a strip in `Bitmaps/Equip`, or a bitmap of
/// the user's in `UserBMP`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Icon {
    pub path: PathBuf,
    /// The square's left edge in a strip; `None` for a bitmap of its own,
    /// drawn whole.
    pub left: Option<u32>,
}

/// The icon number an item keeps (its member +0x234, "12"), read as the
/// original reads it: the digits it starts with ("435a" is 435). `None`
/// for anything else, such as an icon of the user's.
pub fn icon_number(stored: &str) -> Option<u32> {
    let digits = stored.trim_start();
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    digits[..end].parse().ok().filter(|&n| n > 0)
}

/// The path inside an installation of the bitmap an item's icon is in,
/// and the icon's left edge there (`None` for a bitmap of its own): the
/// strip `Bitmaps/Equip/E051_100.bmp` holds icons 51 to 100.
pub fn icon_relative(stored: &str) -> Option<(String, Option<u32>)> {
    let stored = stored.trim();
    if let Some(file) = stored
        .get(..CUSTOM_ICON.len())
        .filter(|p| p.eq_ignore_ascii_case(CUSTOM_ICON))
        .map(|_| &stored[CUSTOM_ICON.len()..])
    {
        return (!file.is_empty()).then(|| (format!("UserBMP/{file}"), None));
    }
    let n = icon_number(stored)? - 1;
    let first = n / ICONS_PER_STRIP * ICONS_PER_STRIP + 1;
    Some((
        format!(
            "Bitmaps/Equip/E{first:03}_{:03}.bmp",
            first + ICONS_PER_STRIP - 1
        ),
        Some(n % ICONS_PER_STRIP * ICON_SIZE),
    ))
}

impl Finder {
    /// The icon an item keeps, when its bitmap is there.
    pub fn icon(&mut self, stored: &str) -> Option<Icon> {
        let (relative, left) = icon_relative(stored)?;
        Some(Icon {
            path: self.find(&relative)?,
            left,
        })
    }
}

/// The icons of the items in an installation, by item name.
#[derive(Debug, Clone, Default)]
pub struct ItemIcons {
    /// By name in lower case.
    icons: HashMap<String, Icon>,
}

impl ItemIcons {
    /// The icon of the item named `name`, whatever its case.
    pub fn get(&self, name: &str) -> Option<&Icon> {
        self.icons.get(&name.to_lowercase())
    }
}

/// What an item keeps for its icon in `UserBMP` named `file`.
pub fn stored_icon(file: &str) -> String {
    format!("{CUSTOM_ICON}{file}")
}

/// One icon in the strips or `UserBMP`, read the same whatever the case
/// or the letter after the number: what [`IconChoice`]s and items are
/// matched by.
fn icon_key(stored: &str) -> Option<String> {
    let (relative, left) = icon_relative(stored)?;
    Some(format!("{}@{left:?}", relative.to_lowercase()))
}

/// Where an icon an item may have comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconGroup {
    /// A bitmap of the user's in `UserBMP`.
    User,
    /// A strip's icon an item of this part type (one of
    /// [`ICON_PART_TYPES`]) has, the first type that has it.
    PartType(&'static str),
    /// A strip's icon no item has.
    Unused,
}

/// An icon an item may have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconChoice {
    /// What the item keeps for it: `12`, or `CRE_espada.bmp`.
    pub stored: String,
    pub icon: Icon,
    /// The names of the items that have it, in order; for an icon of the
    /// user's its file name comes first.
    pub names: Vec<String>,
    pub group: IconGroup,
}
impl Installation {
    /// The pictures a record of `kind` may have. Portraits are those in
    /// `Bitmaps/Portrait`, the ones `race` offers first (its default, then
    /// its list). Monster pictures are the bitmaps in `UserBMP` first, then
    /// those in `Bitmaps/Monsters` and its folders, leaving out the strips
    /// the books' pages are cut from. Each group is sorted by name.
    pub fn picture_choices(&self, kind: Kind, race: &str) -> Result<Vec<PictureChoice>> {
        let mut choices: Vec<PictureChoice> = Vec::new();
        let mut first = Vec::new();
        if kind == Kind::Portrait {
            for object in self.list_objects("Race")? {
                if let Object::Race(r) = object
                    && r.name.eq_ignore_ascii_case(race)
                {
                    first = std::iter::once(r.portrait.clone())
                        .chain(r.portraits.iter().cloned())
                        .filter(|p| !p.is_empty())
                        .collect();
                    break;
                }
            }
        }
        let mut finder = Finder::new(&self.root);
        for file in &first {
            let name = file.rsplit(['\\', '/']).next().unwrap_or(file);
            if let Some(path) = finder.picture(name, Kind::Portrait)
                && !choices.iter().any(|c| c.path == path)
            {
                choices.push(PictureChoice {
                    file: path.file_name().unwrap().to_string_lossy().into_owned(),
                    path,
                    first: true,
                });
            }
        }
        let folders: &[(&str, bool)] = match kind {
            Kind::Portrait => &[("Bitmaps/Portrait", false)],
            Kind::Monster => &[("UserBMP", true), ("Bitmaps/Monsters", false)],
            // The strips' icons are not pictures of their own (see
            // `icon_choices`).
            Kind::Item => &[("UserBMP", true)],
        };
        for &(folder, first) in folders {
            let mut found = Vec::new();
            if let Some(dir) = self.path(folder) {
                bitmaps_in(&dir, kind == Kind::Monster, &mut found);
            }
            let mut group: Vec<PictureChoice> = found
                .into_iter()
                .filter(|path| !choices.iter().any(|c| &c.path == path))
                .filter(|path| kind == Kind::Portrait || !is_strip(path))
                .map(|path| PictureChoice {
                    file: path.file_name().unwrap().to_string_lossy().into_owned(),
                    path,
                    first,
                })
                .collect();
            group.sort_by_key(|c| c.file.to_lowercase());
            choices.extend(group);
        }
        Ok(choices)
    }

    /// The palette the original draws its pictures with, whatever their
    /// own: the one most of its bitmaps in `Bitmaps/Required` share. A
    /// portrait in other colors shows banded there.
    pub fn program_palette(&self) -> Option<Vec<[u8; 3]>> {
        let mut counts: Vec<(Vec<[u8; 3]>, usize)> = Vec::new();
        for entry in fs::read_dir(self.path("Bitmaps/Required")?).ok()? {
            let Ok(entry) = entry else { continue };
            let mut header = Vec::with_capacity(1078);
            let read =
                fs::File::open(entry.path()).and_then(|f| f.take(1078).read_to_end(&mut header));
            let Some(palette) = read.ok().and_then(|_| cr_formats::bmp::palette(&header)) else {
                continue;
            };
            match counts.iter_mut().find(|(p, _)| *p == palette) {
                Some((_, n)) => *n += 1,
                None => counts.push((palette, 1)),
            }
        }
        counts
            .into_iter()
            .max_by_key(|(_, n)| *n)
            .map(|(palette, _)| palette)
    }

    /// Makes a picture of `kind` from one of the user's (see
    /// [`picture_bitmap`]), in the original's palette, in the folder such
    /// pictures go to, and gives its path. A bitmap already in that folder
    /// is used as it is.
    pub fn add_picture(&self, kind: Kind, source: &Path) -> Result<PathBuf> {
        let folder = match self.path(kind.user_folder()) {
            Some(folder) => folder,
            None => {
                let mut folder = self.root.clone();
                for name in kind.user_folder().split('/') {
                    folder = crate::find_entry(&folder, name).unwrap_or_else(|| folder.join(name));
                }
                fs::create_dir_all(&folder).map_err(|source| Error::Io {
                    path: folder.clone(),
                    source,
                })?;
                folder
            }
        };
        if let Some(parent) = source.parent()
            && fs::canonicalize(parent).ok() == fs::canonicalize(&folder).ok()
            && source
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("bmp"))
        {
            return Ok(source.to_owned());
        }
        let data = fs::read(source).map_err(|e| Error::Io {
            path: source.to_owned(),
            source: e,
        })?;
        let bitmap =
            picture_bitmap(&data, kind, self.program_palette().as_deref()).ok_or_else(|| {
                Error::Invalid(cr_i18n::tr!(
                    "{file} is not a BMP, JPEG, GIF or PNG picture",
                    file = source.display()
                ))
            })?;
        let taken: Vec<String> = fs::read_dir(&folder)
            .map_err(|e| Error::Io {
                path: folder.clone(),
                source: e,
            })?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_lowercase())
            .collect();
        let stem = picture_name(source);
        let file = (1..)
            .map(|n| match n {
                1 => format!("{stem}.bmp"),
                n => format!("{stem} {n}.bmp"),
            })
            .find(|f| {
                let lower = f.to_lowercase();
                !taken.contains(&lower) && !taken.contains(&format!("{lower}.tmp"))
            })
            .unwrap();
        let path = folder.join(&file);
        crate::create::write_new(&path, &bitmap)?;
        Ok(path)
    }

    /// The file of a picture a record of `kind` keeps, when it is there.
    pub fn picture_path(&self, stored: &str, kind: Kind) -> Option<PathBuf> {
        Finder::new(&self.root).picture(stored, kind)
    }

    /// The file of the portrait a character keeps, when it is there.
    pub fn portrait_path(&self, stored: &[String; 2]) -> Option<PathBuf> {
        Finder::new(&self.root).picture(&joined(&stored[0], &stored[1]), Kind::Portrait)
    }

    /// The icons an item may have: the bitmaps in `UserBMP` first, as the
    /// original's Select a Custom Bitmap lists them, then those of the
    /// strips in `Bitmaps/Equip`, grouped by the part type of the first
    /// item that has each (in [`ICON_PART_TYPES`]' order), those no item
    /// has last. Each group is in order of file name or number.
    pub fn icon_choices(&self) -> Result<Vec<IconChoice>> {
        let mut users: HashMap<String, (usize, Vec<String>)> = HashMap::new();
        let files = [
            Some(self.load_parts().map_err(Error::Invalid)?),
            self.load_user_parts().map_err(Error::Invalid)?,
        ];
        for file in files.iter().flatten() {
            for (at, part_type) in ICON_PART_TYPES.iter().enumerate() {
                for object in file.of_type(part_type).unwrap_or_default() {
                    if let Object::Part(p) = object
                        && let Some(key) = icon_key(&p.head.unknown_0x234)
                    {
                        let (first, names) = users.entry(key).or_insert((at, Vec::new()));
                        *first = (*first).min(at);
                        if !names.contains(&p.head.name) {
                            names.push(p.head.name.clone());
                        }
                    }
                }
            }
        }
        let mut choices = Vec::new();

        let mut found = Vec::new();
        if let Some(dir) = self.path(Kind::Item.user_folder()) {
            bitmaps_in(&dir, false, &mut found);
        }
        found.sort_by_key(|p| p.file_name().map(|f| f.to_string_lossy().to_lowercase()));
        for path in found {
            let file = path.file_name().unwrap().to_string_lossy().into_owned();
            let stored = stored_icon(&file);
            let mut names = vec![file];
            if let Some((_, items)) = icon_key(&stored).and_then(|k| users.get(&k)) {
                names.extend(items.iter().cloned());
            }
            choices.push(IconChoice {
                stored,
                icon: Icon { path, left: None },
                names,
                group: IconGroup::User,
            });
        }

        let mut strips: Vec<(u32, PathBuf, u32)> = Vec::new();
        if let Some(dir) = self.path(Kind::Item.folder()) {
            for path in fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
            {
                let name = path.file_name().unwrap().to_string_lossy().to_lowercase();
                let first = name
                    .strip_prefix('e')
                    .and_then(|n| n.strip_suffix(".bmp"))
                    .and_then(|n| n.split_once('_'))
                    .and_then(|(first, _)| first.parse::<u32>().ok())
                    .filter(|&n| n % ICONS_PER_STRIP == 1);
                if let (Some(first), Some((width, _))) = (first, bitmap_size(&path)) {
                    strips.push((first, path, width / ICON_SIZE));
                }
            }
        }
        strips.sort();
        let mut shipped: Vec<(usize, IconChoice)> = Vec::new();
        for (first, path, count) in strips {
            for i in 0..count.min(ICONS_PER_STRIP) {
                let stored = (first + i).to_string();
                let (at, names) = icon_key(&stored)
                    .and_then(|k| users.get(&k))
                    .map_or((ICON_PART_TYPES.len(), Vec::new()), |(at, names)| {
                        (*at, names.clone())
                    });
                shipped.push((
                    at,
                    IconChoice {
                        stored,
                        icon: Icon {
                            path: path.clone(),
                            left: Some(i * ICON_SIZE),
                        },
                        names,
                        group: ICON_PART_TYPES
                            .get(at)
                            .map_or(IconGroup::Unused, |t| IconGroup::PartType(t)),
                    },
                ));
            }
        }
        // Stable, so each group stays in order of number.
        shipped.sort_by_key(|(at, _)| *at);
        choices.extend(shipped.into_iter().map(|(_, c)| c));
        Ok(choices)
    }

    /// The icon an item keeps, when its bitmap is there.
    pub fn icon(&self, stored: &str) -> Option<Icon> {
        Finder::new(&self.root).icon(stored)
    }

    /// Makes an icon from a picture of the user's, as [`Self::add_picture`]
    /// does for [`Kind::Item`], and gives what an item keeps for it.
    pub fn add_icon(&self, source: &Path) -> Result<String> {
        let path = self.add_picture(Kind::Item, source)?;
        Ok(stored_icon(&path.file_name().unwrap().to_string_lossy()))
    }

    /// The icons of the shipped items and the user's (`UserDbas/PartsU.dat`),
    /// the user's winning when both have an item of that name. Items whose
    /// icon is not there are left out.
    pub fn item_icons(&self) -> Result<ItemIcons> {
        let mut finder = Finder::new(&self.root);
        let mut icons = HashMap::new();
        let files = [
            Some(self.load_parts().map_err(Error::Invalid)?),
            self.load_user_parts().map_err(Error::Invalid)?,
        ];
        for file in files.iter().flatten() {
            for part_type in ICON_PART_TYPES {
                for object in file.of_type(part_type).unwrap_or_default() {
                    if let Object::Part(p) = object
                        && let Some(icon) = finder.icon(&p.head.unknown_0x234)
                    {
                        icons.insert(p.head.name.to_lowercase(), icon);
                    }
                }
            }
        }
        Ok(ItemIcons { icons })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_map_into_the_strips_and_userbmp() {
        let cases = [
            ("1", "Bitmaps/Equip/E001_050.bmp", Some(0)),
            ("12", "Bitmaps/Equip/E001_050.bmp", Some(11 * 64)),
            ("50", "Bitmaps/Equip/E001_050.bmp", Some(49 * 64)),
            ("51", "Bitmaps/Equip/E051_100.bmp", Some(0)),
            ("435a", "Bitmaps/Equip/E401_450.bmp", Some(34 * 64)),
            ("678", "Bitmaps/Equip/E651_700.bmp", Some(27 * 64)),
            ("CRE_espada.bmp", "UserBMP/espada.bmp", None),
            ("cre_Espada.BMP", "UserBMP/Espada.BMP", None),
        ];
        for (stored, path, left) in cases {
            assert_eq!(
                icon_relative(stored),
                Some((path.to_owned(), left)),
                "{stored}"
            );
        }
        for stored in ["", "0", "CRE_", "a12"] {
            assert_eq!(icon_relative(stored), None, "{stored}");
        }
        assert_eq!(icon_number("395v"), Some(395));
        assert_eq!(icon_number("CRE_1.bmp"), None);
    }

    #[test]
    fn stored_paths_map_into_the_installation() {
        let cases = [
            (
                "C:\\Program Files\\AD&D Core Rules 2.0\\Bitmaps\\Monsters\\vaprak.bmp",
                Kind::Monster,
                "Bitmaps/Monsters/vaprak.bmp",
            ),
            (
                "C:\\Ohjelmatiedostot\\AD&D Core Rules 2.0\\Bitmaps\\Portrait\\\\NA.bmp",
                Kind::Portrait,
                "Bitmaps/Portrait/NA.bmp",
            ),
            (
                "F:\\AD&D\\AD&D Core Rules 2.0\\Bitmaps\\Monsters\\MCA3\\x.bmp",
                Kind::Monster,
                "Bitmaps/Monsters/MCA3/x.bmp",
            ),
            (
                "C:\\Program Files\\AD&D Core Rules 2.0\\0.BMP",
                Kind::Monster,
                "0.BMP",
            ),
            (
                "G:\\Bitmaps\\Portrait\\MALE1.bmp",
                Kind::Portrait,
                "Bitmaps/Portrait/MALE1.bmp",
            ),
            (
                "Q:\\BITMAPS\\MONSTERS\\191.bmp",
                Kind::Portrait,
                "BITMAPS/MONSTERS/191.bmp",
            ),
            (
                "C:\\AD&D Pics\\troll.bmp",
                Kind::Monster,
                "UserBMP/AD&D Pics/troll.bmp",
            ),
            ("6.bmp", Kind::Monster, "Bitmaps/Monsters/6.bmp"),
            (
                "Aarakocr.bmp",
                Kind::Portrait,
                "Bitmaps/Portrait/Aarakocr.bmp",
            ),
        ];
        for (stored, kind, want) in cases {
            assert_eq!(relative(stored, kind).as_deref(), Some(want), "{stored}");
        }
        assert_eq!(relative("", Kind::Monster), None);
        assert_eq!(
            joined("G:\\Bitmaps\\Portrait\\", "MALE1.bmp"),
            "G:\\Bitmaps\\Portrait\\MALE1.bmp"
        );
        assert_eq!(joined("G:\\Bitmaps\\Portrait\\", ""), "");
    }

    #[test]
    fn pictures_are_found_without_regard_to_case() {
        let root = std::env::temp_dir().join(format!("cr-pictures-{}", std::process::id()));
        fs::create_dir_all(root.join("BITMAPS/Monsters")).unwrap();
        fs::write(root.join("BITMAPS/Monsters/Vaprak.BMP"), b"BM").unwrap();
        let mut finder = Finder::new(&root);
        assert_eq!(
            finder.picture(
                "C:\\Program Files\\AD&D Core Rules 2.0\\Bitmaps\\Monsters\\vaprak.bmp",
                Kind::Monster
            ),
            Some(root.join("BITMAPS/Monsters/Vaprak.BMP"))
        );
        assert_eq!(finder.picture("6.bmp", Kind::Monster), None);
        assert_eq!(finder.picture("x.bmp", Kind::Portrait), None);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pictures_become_bitmaps_of_the_shipped_size() {
        let mut png = Vec::new();
        image::RgbaImage::from_pixel(300, 200, image::Rgba([200, 10, 10, 255]))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let bitmap = picture_bitmap(&png, Kind::Portrait, None).unwrap();
        let image = cr_formats::bmp::decode(&bitmap).unwrap();
        assert_eq!(
            (image.width, image.height),
            (PORTRAIT_WIDTH, PORTRAIT_HEIGHT)
        );
        assert_eq!(bitmap[28], 8, "8 bits a pixel");
        assert!(
            image.rgba[..3]
                .iter()
                .zip([200u8, 10, 10])
                .all(|(a, b)| a.abs_diff(b) <= 2),
            "{:?}",
            &image.rgba[..4]
        );
        let monster = picture_bitmap(&png, Kind::Monster, None).unwrap();
        assert_eq!(
            cr_formats::bmp::decode(&monster).map(|i| (i.width, i.height)),
            Some((300, 300))
        );
        assert_eq!(
            picture_bitmap(b"GIF89a but not really", Kind::Portrait, None),
            None
        );
        assert_eq!(picture_bitmap(b"plain text", Kind::Monster, None), None);
        assert_eq!(
            picture_name(Path::new("/x/Minha foto (1).JPG")),
            "Minha foto 1"
        );
        assert_eq!(picture_name(Path::new("/x/João.png")), "João");
        assert_eq!(picture_name(Path::new("/x/日本.png")), "Picture");
    }

    #[test]
    fn added_portraits_get_free_names() {
        let root = std::env::temp_dir().join(format!("cr-portraits-{}", std::process::id()));
        fs::create_dir_all(root.join("BITMAPS/PORTRAIT")).unwrap();
        fs::write(root.join("BITMAPS/PORTRAIT/Face.BMP"), b"BM").unwrap();
        let mut gif = Vec::new();
        image::RgbaImage::from_pixel(10, 10, image::Rgba([0, 0, 255, 255]))
            .write_to(&mut std::io::Cursor::new(&mut gif), image::ImageFormat::Gif)
            .unwrap();
        fs::write(root.join("face.gif"), &gif).unwrap();
        // The original's palette is the one most of its bitmaps share.
        fs::create_dir_all(root.join("BITMAPS/Required")).unwrap();
        let program = [[0, 0, 0], [0, 0, 250], [255, 255, 255]];
        let other = [[9, 9, 9]];
        for (name, palette) in [
            ("a.bmp", &program[..]),
            ("b.bmp", &other),
            ("c.bmp", &program),
        ] {
            let bitmap = cr_formats::bmp::encode_indexed(1, 1, palette, &[0]);
            fs::write(root.join("BITMAPS/Required").join(name), bitmap).unwrap();
        }
        let install = Installation { root: root.clone() };
        assert_eq!(install.program_palette().unwrap()[..3], program);
        let add = |source: &str| install.add_picture(Kind::Portrait, &root.join(source));
        assert_eq!(
            add("face.gif").unwrap(),
            root.join("BITMAPS/PORTRAIT/face 2.bmp")
        );
        assert_eq!(
            add("face.gif").unwrap(),
            root.join("BITMAPS/PORTRAIT/face 3.bmp")
        );
        assert_eq!(
            add("BITMAPS/PORTRAIT/Face.BMP").unwrap(),
            root.join("BITMAPS/PORTRAIT/Face.BMP")
        );
        fs::write(root.join("notes.txt"), b"hello").unwrap();
        assert!(add("notes.txt").is_err());
        // Monster pictures go to UserBMP, made when missing, and are kept
        // with the path the original writes.
        let added = install
            .add_picture(Kind::Monster, &root.join("face.gif"))
            .unwrap();
        assert_eq!(added, root.join("UserBMP/face.bmp"));
        let stored = stored_path(&root, &added).unwrap();
        assert_eq!(
            stored,
            "C:\\Program Files\\AD&D Core Rules 2.0\\UserBMP\\face.bmp"
        );
        assert_eq!(
            split_stored(&stored),
            (
                "C:\\Program Files\\AD&D Core Rules 2.0\\UserBMP\\",
                "face.bmp"
            )
        );
        assert_eq!(install.picture_path(&stored, Kind::Monster), Some(added));
        assert_eq!(stored_path(&root, Path::new("/elsewhere/x.bmp")), None);
        // Monster pictures offered: the user's first, then the shipped
        // ones in any folder, but not the books' strips.
        fs::create_dir_all(root.join("BITMAPS/Monsters/Extra")).unwrap();
        let tile =
            |w: u32| cr_formats::bmp::encode_indexed(w, 300, &program, &vec![0; w as usize * 300]);
        fs::write(root.join("BITMAPS/Monsters/6.bmp"), tile(300)).unwrap();
        fs::write(root.join("BITMAPS/Monsters/5to34.bmp"), tile(1300)).unwrap();
        fs::write(root.join("BITMAPS/Monsters/Extra/x.bmp"), tile(300)).unwrap();
        let choices = install.picture_choices(Kind::Monster, "").unwrap();
        let files: Vec<(&str, bool)> = choices.iter().map(|c| (c.file.as_str(), c.first)).collect();
        assert_eq!(
            files,
            [("face.bmp", true), ("6.bmp", false), ("x.bmp", false)]
        );
        let stored = stored_portrait("face 2.bmp");
        assert_eq!(
            install.portrait_path(&stored),
            Some(root.join("BITMAPS/PORTRAIT/face 2.bmp"))
        );
        let added = fs::read(root.join("BITMAPS/PORTRAIT/face 3.bmp")).unwrap();
        let image = cr_formats::bmp::decode(&added).unwrap();
        assert_eq!(
            (image.width, image.height),
            (PORTRAIT_WIDTH, PORTRAIT_HEIGHT)
        );
        assert_eq!(cr_formats::bmp::palette(&added).unwrap()[..3], program);
        assert_eq!(image.rgba[..4], [0, 0, 250, 255]);
        fs::remove_dir_all(root).unwrap();
    }
}
