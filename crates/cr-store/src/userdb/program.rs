//! Lists the originals build into their document object (the Tsrp3
//! constructor at 0x677978, Evermore's around 0x55b000): historical
//! settings, equipment categories, school schemes, spheres, NWP groups,
//! sub-abilities, social classes, armor types, damage types and the broad
//! and tight weapon groups. The editors offer them in combos and lists.
//!
//! The text stays in the program: this module only knows where each
//! string sits (the address the constructor pushes before adding it) and
//! reads it from the installation's `Program/Tsrp3.exe` or
//! `Program/Evermore.exe` at run time.

use super::Flavor;
use crate::{Error, Installation, Result};

/// One list: the name editors ask for, the document offset it lives at
/// and the address of each entry in Tsrp3.exe and Evermore.exe (0: an
/// empty string).
struct ProgramList {
    name: &'static str,
    /// The `CStringArray` / `CObArray` member of the document.
    #[allow(dead_code)]
    offset: u32,
    t3: &'static [u32],
    ev: &'static [u32],
}

const LISTS: &[ProgramList] = &[
    ProgramList {
        name: "settings",
        offset: 0x284,
        t3: &[
            0x84dd40, 0x84dd54, 0x84dd68, 0x84dd84, 0x84dd90, 0x84dda0, 0x84ddc0, 0x84ddd8,
        ],
        ev: &[
            0x6d77d8, 0x6d77c4, 0x6d77a8, 0x6d779c, 0x6d778c, 0x6d776c, 0x6d7754, 0x6d7740,
        ],
    },
    ProgramList {
        name: "equipment-categories",
        offset: 0x298,
        t3: &[
            0, 0x84ddec, 0x84ddfc, 0x84de14, 0x84de2c, 0x84de48, 0x84de50, 0x84de5c, 0x84de64,
            0x84de70, 0x84de78, 0x84de84,
        ],
        ev: &[
            0, 0x6c4398, 0x6c4364, 0x6c4680, 0x6c4664, 0x6c43a8, 0x6c43b8, 0x6c43b0, 0x6c4384,
            0x6c4390, 0x6c3be0, 0x6c437c,
        ],
    },
    ProgramList {
        // Tsrp3 adds an empty CString first (its address is not a char
        // pointer); Evermore an empty literal.
        name: "racial-equipment",
        offset: 0x2ac,
        t3: &[0, 0x84de8c, 0x84dea0, 0x84deb0, 0x84dec4],
        ev: &[0, 0x6d68ec, 0x6d68c8, 0x6d68d8, 0x6d68b4],
    },
    ProgramList {
        name: "school-schemes",
        offset: 0x2c0,
        t3: &[0x84ded8, 0x84def0, 0x84df04, 0x84df1c],
        ev: &[0x6d7728, 0x6d7714, 0x6d76fc, 0x6d76e0],
    },
    ProgramList {
        name: "spheres",
        offset: 0x2d4,
        t3: &[
            0x84df38, 0x84df3c, 0x84df44, 0x84df4c, 0x84df54, 0x84df5c, 0x84df64, 0x84df70,
            0x84df7c, 0x84df8c, 0x84dfa0, 0x84dfb0, 0x84dfc4, 0x84dfd0, 0x84dfd8, 0x84dfdc,
            0x84dfe8, 0x84dff0, 0x84dff8, 0x84e004, 0x84e010, 0x84e014, 0x84e01c, 0x84e024,
            0x84e030, 0x84e034, 0x84e03c,
        ],
        ev: &[
            0x6c73e0, 0x6c43a8, 0x6d76d8, 0x6cb2e4, 0x6d76d0, 0x6c35f4, 0x6d76c4, 0x6ccc98,
            0x6d76b4, 0x6d76a0, 0x6d7690, 0x6d767c, 0x6d5c48, 0x6cd0cc, 0x6d7678, 0x6d766c,
            0x6d7664, 0x6d765c, 0x6d7650, 0x6d7644, 0x6d7640, 0x6d7638, 0x6c3a10, 0x6d762c,
            0x6d7628, 0x6d7620, 0x6d7618,
        ],
    },
    ProgramList {
        name: "nwp-groups",
        offset: 0x2e8,
        t3: &[0x84e044, 0x84e04c, 0x84e054, 0x84e05c, 0x84e064],
        ev: &[0x6d7610, 0x6c73a4, 0x6d7608, 0x6d7600, 0x6d75f8],
    },
    ProgramList {
        name: "sub-abilities",
        offset: 0x2fc,
        t3: &[
            0x84e06c, 0x84e080, 0x84e090, 0x84e0a0, 0x84e0b4, 0x84e0c8, 0x84e0e0, 0x84e0f4,
            0x84e10c, 0x84e120, 0x84e134, 0x84e148,
        ],
        ev: &[
            0x6c2754, 0x6c2744, 0x6c2734, 0x6c2720, 0x6c270c, 0x6c26f4, 0x6c26e0, 0x6c26c8,
            0x6c26b4, 0x6c26a0, 0x6c268c, 0x6c2678,
        ],
    },
    ProgramList {
        name: "social-classes",
        offset: 0x310,
        t3: &[0x84e15c, 0x84e168, 0x84e17c, 0x84e190],
        ev: &[0x6c2134, 0x6c52d0, 0x6c52bc, 0x6c52b0],
    },
    ProgramList {
        // Ids 0..4, in order.
        name: "armor-types",
        offset: 0x220,
        t3: &[0x84d990, 0x84d998, 0x84d9a0, 0x84d9a8, 0x84d9b0],
        ev: &[0x6c1c94, 0x6c3334, 0x6cea68, 0x6cbb04, 0x6cea50],
    },
    ProgramList {
        name: "damage-types",
        offset: 0x20c,
        t3: &[
            0x84d9b8, 0x84d9c0, 0x84d9c4, 0x84d9c8, 0x84d9cc, 0x84d9d0, 0x84d9d4, 0x84d9d8,
        ],
        ev: &[
            0x6c1c94, 0x6d7b48, 0x6c46c0, 0x6d6070, 0x6d7b44, 0x6d7b40, 0x6d7b3c, 0x6d7b34,
        ],
    },
    ProgramList {
        // Ids 0..7, in order.
        name: "broad-weapon-groups",
        offset: 0x234,
        t3: &[
            0x84d9e0, 0x84d9e8, 0x84d9f0, 0x84da0c, 0x84da28, 0x84da34, 0x84da48, 0x84da50,
        ],
        ev: &[
            0x6c1c94, 0x6d7b2c, 0x6d7b10, 0x6d7af4, 0x6d7ae8, 0x6d7ad4, 0x6d7acc, 0x6d7ac0,
        ],
    },
    ProgramList {
        // Ids 1..35, in order.
        name: "tight-weapon-groups",
        offset: 0x248,
        t3: &[
            0x84da5c, 0x84da64, 0x84da70, 0x84da7c, 0x84da8c, 0x84da9c, 0x84daa4, 0x84dac4,
            0x84dae4, 0x84db08, 0x84db14, 0x84db28, 0x84db30, 0x84db50, 0x84db64, 0x84db74,
            0x84db88, 0x84db9c, 0x84dbb0, 0x84dbcc, 0x84dbe8, 0x84dc08, 0x84dc18, 0x84dc28,
            0x84dc40, 0x84dc54, 0x84dc64, 0x84dc74, 0x84dc84, 0x84dc9c, 0x84dcb4, 0x84dccc,
            0x84dcec, 0x84dd04, 0x84dd1c,
        ],
        ev: &[
            0x6d7b2c, 0x6d7ab4, 0x6d7aa8, 0x6d7a98, 0x6d7a88, 0x6c2118, 0x6d7a68, 0x6d7a48,
            0x6d7a24, 0x6d7a18, 0x6d7a04, 0x6d79fc, 0x6d79dc, 0x6d79c8, 0x6d79b8, 0x6d79a4,
            0x6d7990, 0x6d797c, 0x6d7960, 0x6d7944, 0x6d7924, 0x6d7914, 0x6d7904, 0x6d78ec,
            0x6d78d8, 0x6d78c8, 0x6d78b8, 0x6d78a8, 0x6d7890, 0x6d7878, 0x6d7860, 0x6d7840,
            0x6d7828, 0x6d7810, 0x6d77ec,
        ],
    },
    ProgramList {
        // Tsrp3 only (the kit's required secondary skills); filled at
        // 0x463737.
        name: "secondary-skills",
        offset: 0x1adc,
        t3: &[
            0x818000, 0x818008, 0x818018, 0x818020, 0x818028, 0x818034, 0x81803c, 0x818044,
            0x81804c, 0x818054, 0x818064, 0x818074, 0x81807c, 0x818084, 0x818090, 0x818098,
            0x8180a0, 0x8180ac, 0x8180bc, 0x8180d0, 0x8180e0, 0x8180f0, 0x8180fc, 0x818114,
        ],
        ev: &[],
    },
];

/// Whether `name` is one of the program's lists.
pub fn is_program_list(name: &str) -> bool {
    LISTS.iter().any(|l| l.name == name)
}

/// The executable of an installation's flavor.
pub fn exe_path(flavor: Flavor) -> &'static str {
    match flavor {
        Flavor::Expansion => "Program/Tsrp3.exe",
        Flavor::Core20 => "Program/Evermore.exe",
    }
}

/// The list `name` as the installation's program builds it; empty when the
/// executable is missing.
pub fn program_list(install: &Installation, flavor: Flavor, name: &str) -> Result<Vec<String>> {
    let Some(list) = LISTS.iter().find(|l| l.name == name) else {
        return Ok(Vec::new());
    };
    let relative = exe_path(flavor);
    if install.path(relative).is_none() {
        return Ok(Vec::new());
    }
    let image = install.read(relative)?;
    let addresses = match flavor {
        Flavor::Expansion => list.t3,
        Flavor::Core20 => list.ev,
    };
    strings_at(&image, addresses)
        .ok_or_else(|| Error::Invalid(format!("{relative}: unexpected program (list {name})")))
}

/// The NUL-terminated strings at `addresses` of a PE image, or `None`
/// when one is out of the image or not plain text (another build).
pub fn strings_at(image: &[u8], addresses: &[u32]) -> Option<Vec<String>> {
    let sections = sections(image)?;
    addresses
        .iter()
        .map(|&va| {
            if va == 0 {
                return Some(String::new());
            }
            let (base, raw, _) = sections
                .iter()
                .find(|(v, _, s)| (*v..v + s).contains(&va))?;
            let start = (va - base + raw) as usize;
            let len = image.get(start..)?.iter().position(|b| *b == 0)?;
            let bytes = &image[start..start + len];
            (len < 200 && bytes.iter().all(|b| (0x20..0x7f).contains(b)))
                .then(|| String::from_utf8_lossy(bytes).into_owned())
        })
        .collect()
}

/// (virtual address, raw offset, raw size) of each section.
fn sections(image: &[u8]) -> Option<Vec<(u32, u32, u32)>> {
    let u16_at = |o: usize| Some(u16::from_le_bytes(image.get(o..o + 2)?.try_into().ok()?));
    let u32_at = |o: usize| Some(u32::from_le_bytes(image.get(o..o + 4)?.try_into().ok()?));
    let pe = u32_at(0x3c)? as usize;
    if image.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    let count = u16_at(pe + 6)? as usize;
    let optional = u16_at(pe + 20)? as usize;
    let base = u32_at(pe + 24 + 28)?;
    let table = pe + 24 + optional;
    (0..count)
        .map(|i| {
            let s = table + 40 * i;
            let virtual_size = u32_at(s + 8)?;
            let address = u32_at(s + 12)?;
            let raw_size = u32_at(s + 16)?;
            let raw = u32_at(s + 20)?;
            Some((base + address, raw, virtual_size.min(raw_size)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_list_has_both_programs() {
        for l in LISTS {
            // Evermore has no list where it has no editor using it.
            assert!(l.ev.is_empty() || l.t3.len() == l.ev.len(), "{}", l.name);
        }
        assert!(is_program_list("spheres"));
        assert!(!is_program_list("Weapons"));
    }

    #[test]
    fn reads_strings_from_a_minimal_image() {
        // A PE header with one section at 0x401000 holding "Abc\0".
        let mut image = vec![0u8; 0x400];
        image[0x3c] = 0x80;
        image[0x80..0x84].copy_from_slice(b"PE\0\0");
        image[0x86] = 1;
        image[0x94] = 0xe0;
        image[0x98 + 28..0x98 + 32].copy_from_slice(&0x400000u32.to_le_bytes());
        let s = 0x98 + 0xe0;
        image[s + 8..s + 12].copy_from_slice(&0x100u32.to_le_bytes());
        image[s + 12..s + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        image[s + 16..s + 20].copy_from_slice(&0x100u32.to_le_bytes());
        image[s + 20..s + 24].copy_from_slice(&0x300u32.to_le_bytes());
        image[0x310..0x314].copy_from_slice(b"Abc\0");
        assert_eq!(
            strings_at(&image, &[0x401010, 0]),
            Some(vec!["Abc".into(), String::new()])
        );
        image[0x311] = 1;
        assert_eq!(strings_at(&image, &[0x401010]), None);
    }

    /// Both programs build the same lists (CORERULES_DIR; the Core Rules
    /// 2.0 program is read when the folder has it).
    #[test]
    fn real_programs_build_the_lists() {
        let Ok(dir) = std::env::var("CORERULES_DIR") else {
            return;
        };
        let install = Installation::open(&dir).unwrap();
        for flavor in [Flavor::Expansion, Flavor::Core20] {
            if install.path(exe_path(flavor)).is_none() {
                continue;
            }
            for l in LISTS {
                let items = program_list(&install, flavor, l.name).unwrap();
                let expected = match flavor {
                    Flavor::Expansion => l.t3.len(),
                    Flavor::Core20 => l.ev.len(),
                };
                assert_eq!(items.len(), expected, "{}", l.name);
                assert!(items.iter().skip(1).all(|s| !s.is_empty()), "{}", l.name);
            }
            let spheres = program_list(&install, flavor, "spheres").unwrap();
            assert_eq!(spheres[0], "All");
        }
    }
}
