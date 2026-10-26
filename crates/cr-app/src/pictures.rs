//! The original's pictures cut out as the apps show them: an item's
//! icon from its strip of icons, and a monster type's frames from its
//! picture page.

use std::path::Path;

use cr_formats::bmp::Image;
use cr_i18n::{tr, tr_noop};
use cr_store::pictures::{
    CUSTOM_ICON, ICON_PART_TYPES, ICON_SIZE, Icon, IconGroup, icon_number, split_stored,
};

/// A bitmap file decoded.
pub fn load(path: &Path) -> Option<Image> {
    cr_formats::bmp::decode(&std::fs::read(path).ok()?)
}

/// The icon at `left` in `strip`, a row of icons [`ICON_SIZE`] pixels
/// square.
pub fn icon_in(strip: &Image, left: u32) -> Option<Image> {
    let width = ICON_SIZE.min(strip.width.checked_sub(left)?);
    let height = ICON_SIZE.min(strip.height);
    if width == 0 {
        return None;
    }
    let (w, row) = (width as usize * 4, strip.width as usize * 4);
    let mut rgba = Vec::with_capacity(w * height as usize);
    for y in 0..height as usize {
        let at = y * row + left as usize * 4;
        rgba.extend_from_slice(&strip.rgba[at..at + w]);
    }
    Some(Image {
        width,
        height,
        rgba,
    })
}

/// An item's icon: a bitmap of its own, or a square of a strip.
pub fn icon(icon: &Icon) -> Option<Image> {
    let image = load(&icon.path)?;
    match icon.left {
        Some(left) => icon_in(&image, left),
        None => Some(image),
    }
}

/// An icon as the user reads it: its file, or its number.
pub fn icon_name(stored: &str) -> String {
    match stored.get(..CUSTOM_ICON.len()) {
        Some(prefix) if prefix.eq_ignore_ascii_case(CUSTOM_ICON) => {
            stored[CUSTOM_ICON.len()..].to_owned()
        }
        _ => match icon_number(stored) {
            Some(number) => tr!("Icon {number}", number),
            None => stored.to_owned(),
        },
    }
}

/// What a picture's row says of the one a record keeps, `stored` (an
/// item's icon with `icon`), found in the installation or not.
pub fn picture_line(stored: &str, icon: bool, found: bool) -> String {
    let name = if icon {
        match stored.trim() {
            "" => String::new(),
            stored => icon_name(stored),
        }
    } else {
        split_stored(stored).1.to_owned()
    };
    match name.as_str() {
        "" => tr("None").to_owned(),
        _ if !found => tr!("{file} (not in this installation)", file = name),
        _ => name,
    }
}

/// The part types' names, for the groups of icons.
const PART_TYPE_NAMES: [&str; 6] = [
    tr_noop!("Weapons"),
    tr_noop!("Armor"),
    tr_noop!("Equipment"),
    tr_noop!("Magical Items"),
    tr_noop!("Gems and Art"),
    tr_noop!("Currency Types"),
];

/// The heading of a group of icons in Choose Picture.
pub fn icon_group_title(group: IconGroup) -> &'static str {
    match group {
        IconGroup::User => tr("Your Pictures"),
        IconGroup::PartType(part_type) => ICON_PART_TYPES
            .iter()
            .position(|t| *t == part_type)
            .map_or(part_type, |at| tr(PART_TYPE_NAMES[at])),
        IconGroup::Unused => tr("Other Pictures"),
    }
}

/// Square `index` of a strip of squares as tall as the strip, a monster
/// type's picture page. The pages key the background in magenta, which
/// is made white as the single pictures have it.
pub fn frame_in(page: &Image, index: u32) -> Option<Image> {
    let (width, side) = (page.width as usize, page.height as usize);
    let x = index as usize * side;
    if side == 0 || x + side > width {
        return None;
    }
    let mut rgba = Vec::with_capacity(side * side * 4);
    for row in page.rgba.chunks_exact(width * 4) {
        rgba.extend_from_slice(&row[x * 4..(x + side) * 4]);
    }
    for px in rgba.as_chunks_mut::<4>().0 {
        if px[..3] == [255, 0, 255] {
            px[..3].copy_from_slice(&[255, 255, 255]);
        }
    }
    Some(Image {
        width: side as u32,
        height: side as u32,
        rgba,
    })
}

/// Frame `index` of the picture page at `page`.
pub fn frame(page: &Path, index: u32) -> Option<Image> {
    frame_in(&load(page)?, index)
}

/// What a monster's picture shows first: as the Monsters screen
/// (0x6d2390), the first of its type's frames, which need not be the
/// type's own picture, or else the picture itself.
pub fn monster(picture: &Path, frames: Option<&cr_store::catalog::Frames>) -> Option<Image> {
    frames
        .and_then(|f| frame(&f.page, f.first))
        .or_else(|| load(picture))
}

/// The frame after frame `at` (counted from the first) of a monster
/// type's frames, going back to the first after the last: Next Image,
/// the Monsters screen's Alternate Image(s) (0x6d2e79).
pub fn next_frame(frames: &cr_store::catalog::Frames, at: u32) -> u32 {
    if frames.first + at + 1 > frames.last {
        0
    } else {
        at + 1
    }
}

/// Where the Monster Full Screen dialog (IDD 282, 0x6fb7d0) draws the
/// picture on its background of 794 by 550 pixels: at 174, 49, 450
/// pixels square.
pub const FULL_SCREEN: (f64, f64, f64, f64, f64) = (794.0, 550.0, 174.0, 49.0, 450.0);

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(width: u32, height: u32) -> Image {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&[x as u8, y as u8, 255, 255]);
            }
        }
        Image {
            width,
            height,
            rgba,
        }
    }

    #[test]
    fn icons_and_frames_are_cut_from_their_strips() {
        let icons = strip(200, 64);
        let second = icon_in(&icons, 64).unwrap();
        assert_eq!((second.width, second.height), (64, 64));
        assert_eq!(&second.rgba[..4], &[64, 0, 255, 255]);
        assert!(icon_in(&icons, 300).is_none());
        let mut page = strip(30, 10);
        page.rgba[(10 * 4)..(10 * 4 + 3)].copy_from_slice(&[255, 0, 255]);
        let frame = frame_in(&page, 1).unwrap();
        assert_eq!((frame.width, frame.height), (10, 10));
        assert_eq!(&frame.rgba[..4], &[255, 255, 255, 255]);
        assert!(frame_in(&page, 3).is_none());
    }

    #[test]
    fn next_image_goes_back_to_the_first() {
        let frames = cr_store::catalog::Frames {
            page: "5to34.bmp".into(),
            first: 5,
            last: 7,
        };
        assert_eq!(next_frame(&frames, 0), 1);
        assert_eq!(next_frame(&frames, 2), 0);
    }
}
