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
//! A character's portrait is picked from `Bitmaps/Portrait`, the race's
//! portraits first ([`Installation::portrait_choices`]), or made from a
//! picture of the user's ([`Installation::add_portrait`]): a BMP, JPEG,
//! GIF or PNG cut to the shipped portraits' size and kept as a bitmap in
//! that folder, so the original shows it too.

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
}

/// The installation folder's name in the stored paths, whatever the drive
/// or the localized `Program Files` before it.
const ROOT_NAME: &str = "AD&D Core Rules 2.0";

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
        None => vec![
            "Bitmaps",
            match kind {
                Kind::Monster => "Monsters",
                Kind::Portrait => "Portrait",
            },
        ],
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

/// A portrait a character may have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortraitChoice {
    /// The file name in `Bitmaps/Portrait`.
    pub file: String,
    pub path: PathBuf,
    /// Whether the character's race offers it.
    pub race: bool,
}

/// What a character keeps for a portrait in `Bitmaps/Portrait`: the folder
/// as the original writes it, and the file name.
pub fn stored_portrait(file: &str) -> [String; 2] {
    [
        crate::userdb::races::PORTRAIT_FOLDER.to_owned(),
        file.to_owned(),
    ]
}

/// A portrait bitmap made from a BMP, JPEG, GIF or PNG picture: cut to the
/// shipped portraits' proportions around its middle, scaled to their size
/// and kept, as they are, in 8 bits. With `palette`, the colors are those
/// (see [`Installation::program_palette`]), dithered; without, the
/// picture's own 256. `None` for any other file.
pub fn portrait_bitmap(data: &[u8], palette: Option<&[[u8; 3]]>) -> Option<Vec<u8>> {
    use image::ImageFormat;
    let format = image::guess_format(data).ok()?;
    if !matches!(
        format,
        ImageFormat::Bmp | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::Png
    ) {
        return None;
    }
    let picture = image::load_from_memory_with_format(data, format).ok()?;
    let mut rgba = picture
        .resize_to_fill(
            PORTRAIT_WIDTH,
            PORTRAIT_HEIGHT,
            image::imageops::FilterType::Lanczos3,
        )
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
        Some(palette) => (palette.to_vec(), dither(&rgba, palette)),
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
        PORTRAIT_WIDTH,
        PORTRAIT_HEIGHT,
        &palette,
        &indices,
    ))
}

/// A portrait's pixels (RGBA, opaque) in the colors of `palette`, with
/// Floyd-Steinberg dithering so gradients do not band.
fn dither(rgba: &[u8], palette: &[[u8; 3]]) -> Vec<u8> {
    let width = PORTRAIT_WIDTH as usize;
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

/// A file name for a portrait made from `source`: its name in letters,
/// digits, spaces, `-` and `_`, with `.bmp`. The letters are those of
/// Latin-1, which the characters' Windows-1252 strings can hold.
fn portrait_name(source: &Path) -> String {
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
        "" => "Portrait".to_owned(),
        name => name.to_owned(),
    }
}

impl Installation {
    /// The portraits in `Bitmaps/Portrait`: those `race` offers first, its
    /// default then its list, then the rest by name.
    pub fn portrait_choices(&self, race: &str) -> Result<Vec<PortraitChoice>> {
        let mut offered = Vec::new();
        for object in self.list_objects("Race")? {
            if let Object::Race(r) = object
                && r.name.eq_ignore_ascii_case(race)
            {
                offered = std::iter::once(r.portrait.clone())
                    .chain(r.portraits.iter().cloned())
                    .filter(|p| !p.is_empty())
                    .collect();
                break;
            }
        }
        let mut choices: Vec<PortraitChoice> = Vec::new();
        let mut finder = Finder::new(&self.root);
        for file in &offered {
            let name = file.rsplit(['\\', '/']).next().unwrap_or(file);
            if let Some(path) = finder.picture(name, Kind::Portrait)
                && !choices.iter().any(|c| c.path == path)
            {
                choices.push(PortraitChoice {
                    file: path.file_name().unwrap().to_string_lossy().into_owned(),
                    path,
                    race: true,
                });
            }
        }
        let mut rest: Vec<PortraitChoice> = self
            .path("Bitmaps/Portrait")
            .and_then(|dir| fs::read_dir(dir).ok())
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("bmp"))
                    && !choices.iter().any(|c| &c.path == path)
            })
            .map(|path| PortraitChoice {
                file: path.file_name().unwrap().to_string_lossy().into_owned(),
                path,
                race: false,
            })
            .collect();
        rest.sort_by_key(|c| c.file.to_lowercase());
        choices.extend(rest);
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

    /// Makes a portrait in `Bitmaps/Portrait` from a picture of the
    /// user's (see [`portrait_bitmap`]), in the original's palette, and
    /// gives its file name. A bitmap
    /// already in that folder is used as it is.
    pub fn add_portrait(&self, source: &Path) -> Result<String> {
        let folder = match self.path("Bitmaps/Portrait") {
            Some(folder) => folder,
            None => {
                let bitmaps = self
                    .path("Bitmaps")
                    .unwrap_or_else(|| self.root.join("Bitmaps"));
                let folder = bitmaps.join("Portrait");
                fs::create_dir_all(&folder).map_err(|source| Error::Io {
                    path: folder.clone(),
                    source,
                })?;
                folder
            }
        };
        if let (Some(parent), Some(name)) = (source.parent(), source.file_name())
            && fs::canonicalize(parent).ok() == fs::canonicalize(&folder).ok()
            && source
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("bmp"))
        {
            return Ok(name.to_string_lossy().into_owned());
        }
        let data = fs::read(source).map_err(|e| Error::Io {
            path: source.to_owned(),
            source: e,
        })?;
        let bitmap =
            portrait_bitmap(&data, self.program_palette().as_deref()).ok_or_else(|| {
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
        let stem = portrait_name(source);
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
        crate::create::write_new(&folder.join(&file), &bitmap)?;
        Ok(file)
    }

    /// The file of the portrait a character keeps, when it is there.
    pub fn portrait_path(&self, stored: &[String; 2]) -> Option<PathBuf> {
        Finder::new(&self.root).picture(&joined(&stored[0], &stored[1]), Kind::Portrait)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn pictures_become_portrait_bitmaps() {
        let mut png = Vec::new();
        image::RgbaImage::from_pixel(300, 200, image::Rgba([200, 10, 10, 255]))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let bitmap = portrait_bitmap(&png, None).unwrap();
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
        assert_eq!(portrait_bitmap(b"GIF89a but not really", None), None);
        assert_eq!(portrait_bitmap(b"plain text", None), None);
        assert_eq!(
            portrait_name(Path::new("/x/Minha foto (1).JPG")),
            "Minha foto 1"
        );
        assert_eq!(portrait_name(Path::new("/x/João.png")), "João");
        assert_eq!(portrait_name(Path::new("/x/日本.png")), "Portrait");
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
        assert_eq!(
            install.add_portrait(&root.join("face.gif")).unwrap(),
            "face 2.bmp"
        );
        assert_eq!(
            install.add_portrait(&root.join("face.gif")).unwrap(),
            "face 3.bmp"
        );
        assert_eq!(
            install
                .add_portrait(&root.join("BITMAPS/PORTRAIT/Face.BMP"))
                .unwrap(),
            "Face.BMP"
        );
        fs::write(root.join("notes.txt"), b"hello").unwrap();
        assert!(install.add_portrait(&root.join("notes.txt")).is_err());
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
