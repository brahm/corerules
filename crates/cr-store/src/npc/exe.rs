//! The program image (`Program/Tsrp3.exe` or `Program/Evermore.exe`) the
//! NPC generator reads its texts from at run time: name tables built on
//! the stack, strings the code pushes, and the weapon group tables of the
//! part document's constructor. Nothing of the program's text is kept in
//! this crate; only addresses.

use std::collections::HashMap;

use crate::treasure::Flavor;
use crate::{Error, Installation, Result};

/// The program each flavor runs, its size and FNV-1a 64 hash.
const PROGRAMS: [(Flavor, &str, usize, u64); 2] = [
    (
        Flavor::Expansion,
        "Program/Tsrp3.exe",
        5_033_984,
        0xcf09_6cd3_eb7d_22b1,
    ),
    (
        Flavor::Core20,
        "Program/Evermore.exe",
        3_423_232,
        0x822f_6926_c2e4_f36e,
    ),
];

fn fnv1a(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A section: virtual address, size, file offset, and whether it holds
/// data (not code).
#[derive(Debug, Clone, Copy)]
struct Section {
    va: u32,
    size: u32,
    raw: u32,
    data: bool,
}

/// A loaded program image.
#[derive(Debug)]
pub struct Image {
    bytes: Vec<u8>,
    sections: Vec<Section>,
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

impl Image {
    /// Opens the program of the installation's flavor and checks it is
    /// the one whose addresses this crate knows.
    pub fn open(install: &Installation, flavor: Flavor) -> Result<Self> {
        let (_, relative, size, hash) = PROGRAMS
            .iter()
            .copied()
            .find(|p| p.0 == flavor)
            .expect("both flavors have a program");
        let Some(path) = install.path(relative) else {
            return Err(Error::Invalid(format!(
                "{relative} is missing: the NPC generator reads its names and other texts from the program"
            )));
        };
        let bytes = std::fs::read(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        if bytes.len() != size || fnv1a(&bytes) != hash {
            return Err(Error::Invalid(format!(
                "{}: not the program version the NPC generator knows ({} bytes)",
                path.display(),
                bytes.len()
            )));
        }
        Self::parse(bytes)
            .ok_or_else(|| Error::Invalid(format!("{}: not a Windows program", path.display())))
    }

    fn parse(bytes: Vec<u8>) -> Option<Self> {
        let pe = u32_at(&bytes, 0x3c)? as usize;
        let count = u16_at(&bytes, pe + 6)? as usize;
        let optional = u16_at(&bytes, pe + 20)? as usize;
        let base = u32_at(&bytes, pe + 24 + 28)?;
        let mut sections = Vec::new();
        for i in 0..count {
            let o = pe + 24 + optional + 40 * i;
            let vsize = u32_at(&bytes, o + 8)?;
            let va = u32_at(&bytes, o + 12)?;
            let rsize = u32_at(&bytes, o + 16)?;
            let raw = u32_at(&bytes, o + 20)?;
            let flags = u32_at(&bytes, o + 36)?;
            sections.push(Section {
                va: base.wrapping_add(va),
                size: vsize.max(rsize),
                raw,
                // IMAGE_SCN_CNT_CODE
                data: flags & 0x20 == 0,
            });
        }
        Some(Self { bytes, sections })
    }

    fn section(&self, va: u32) -> Option<&Section> {
        self.sections
            .iter()
            .find(|s| s.va <= va && va < s.va.wrapping_add(s.size))
    }

    /// The bytes from `va` on, up to the end of its section.
    pub fn at(&self, va: u32) -> &[u8] {
        let Some(s) = self.section(va) else {
            return &[];
        };
        let start = (va - s.va + s.raw) as usize;
        let end = (s.raw + s.size) as usize;
        self.bytes
            .get(start..end.min(self.bytes.len()))
            .unwrap_or(&[])
    }

    /// The NUL-terminated text at `va` in a data section (ANSI, read as
    /// Latin-1), or `None` when `va` is not in one.
    pub fn text(&self, va: u32) -> Option<String> {
        if !self.section(va)?.data {
            return None;
        }
        let b = self.at(va);
        let end = b.iter().position(|&c| c == 0)?;
        Some(b[..end].iter().map(|&c| char::from(c)).collect())
    }

    /// The texts `push offset text` puts on the stack in the function at
    /// `va`, in code order, up to `len` bytes.
    pub fn pushed_texts(&self, va: u32, len: usize) -> Vec<String> {
        let code = self.at(va);
        let code = &code[..len.min(code.len())];
        let mut out = Vec::new();
        let mut i = 0;
        while i + 5 <= code.len() {
            if code[i] == 0x68
                && let Some(t) = u32_at(code, i + 1).and_then(|v| self.text(v))
            {
                out.push(t);
                i += 5;
                continue;
            }
            i += 1;
        }
        out
    }

    /// Tables built on the stack of the function at `va` by `mov dword
    /// [ebp+d], offset text` before each call to `picker` with `lea reg,
    /// [ebp+d]`: the first table seen at each displacement, up to its
    /// NULL.
    pub fn stack_tables(&self, va: u32, len: usize, picker: u32) -> HashMap<i32, Vec<String>> {
        let code = self.at(va);
        let code = &code[..len.min(code.len())];
        let mut mem: HashMap<i32, u32> = HashMap::new();
        let mut leas: Vec<i32> = Vec::new();
        let mut out: HashMap<i32, Vec<String>> = HashMap::new();
        let disp8 = |o: usize| i32::from(code[o] as i8);
        let disp32 = |o: usize| u32_at(code, o).map_or(0, |v| v as i32);
        let mut i = 0;
        while i + 10 <= code.len() {
            let (op, modrm) = (code[i], code[i + 1]);
            if op == 0xc7 && modrm == 0x45 {
                mem.insert(disp8(i + 2), u32_at(code, i + 3).unwrap_or(0));
                i += 7;
            } else if op == 0xc7 && modrm == 0x85 {
                mem.insert(disp32(i + 2), u32_at(code, i + 6).unwrap_or(0));
                i += 10;
            } else if op == 0x8d && modrm & 0xc7 == 0x45 {
                leas.push(disp8(i + 2));
                i += 3;
            } else if op == 0x8d && modrm & 0xc7 == 0x85 {
                leas.push(disp32(i + 2));
                i += 6;
            } else if op == 0xe8 {
                let target = (va as i64 + i as i64 + 5 + i64::from(disp32(i + 1))) as u32;
                if target == picker
                    && let Some(&d) = leas.first()
                {
                    out.entry(d).or_insert_with(|| {
                        let mut list = Vec::new();
                        let mut at = d;
                        while let Some(&v) = mem.get(&at).filter(|&&v| v != 0) {
                            list.push(self.text(v).unwrap_or_default());
                            at += 4;
                        }
                        list
                    });
                }
                leas.clear();
                i += 5;
            } else {
                i += 1;
            }
        }
        out
    }

    /// The name-to-id tables the part document's constructor fills
    /// (`push id; push ecx; mov ecx, esp; mov [ebp-x], esp; push offset
    /// name; …; add ecx, table; call add`), by table offset.
    pub fn id_tables(&self, va: u32, len: usize, add: u32) -> HashMap<u32, Vec<(String, i32)>> {
        let code = self.at(va);
        let code = &code[..len.min(code.len())];
        let mut out: HashMap<u32, Vec<(String, i32)>> = HashMap::new();
        let mut pending: Vec<(String, i32)> = Vec::new();
        let mut i = 0;
        while i + 13 <= code.len() {
            if code[i] == 0x6a && code[i + 2..i + 5] == [0x51, 0x8b, 0xcc] && code[i + 5] == 0x89 {
                let k = match code[i + 6] {
                    0x65 => Some(i + 8),
                    0xa5 => Some(i + 11),
                    _ => None,
                };
                if let Some(k) = k
                    && code.get(k) == Some(&0x68)
                    && let Some(t) = u32_at(code, k + 1).and_then(|v| self.text(v))
                {
                    pending.push((t, i32::from(code[i + 1])));
                    i = k + 5;
                    continue;
                }
            }
            if code[i] == 0x81 && code[i + 1] == 0xc1 && code[i + 6] == 0xe8 {
                let table = u32_at(code, i + 2).unwrap_or(0);
                let rel = u32_at(code, i + 7).unwrap_or(0) as i32;
                let target = (va as i64 + i as i64 + 11 + i64::from(rel)) as u32;
                if target == add && !pending.is_empty() {
                    out.entry(table).or_default().append(&mut pending);
                }
                pending.clear();
                i += 11;
                continue;
            }
            i += 1;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_is_fnv1a() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    /// A minimal image: one code section at 0x401000 and one data section
    /// at 0x402000.
    fn image(code: &[u8], data: &[u8]) -> Image {
        let mut b = vec![0u8; 0x400];
        b[0x3c] = 0x40;
        b[0x46] = 2; // sections
        b[0x54] = 0xe0; // optional header size
        b[0x74..0x78].copy_from_slice(&0x40_0000u32.to_le_bytes());
        let mut section = |o: usize, va: u32, size: u32, raw: u32, flags: u32| {
            b[o + 8..o + 12].copy_from_slice(&size.to_le_bytes());
            b[o + 12..o + 16].copy_from_slice(&va.to_le_bytes());
            b[o + 16..o + 20].copy_from_slice(&size.to_le_bytes());
            b[o + 20..o + 24].copy_from_slice(&raw.to_le_bytes());
            b[o + 36..o + 40].copy_from_slice(&flags.to_le_bytes());
        };
        section(0x138, 0x1000, 0x100, 0x200, 0x20);
        section(0x160, 0x2000, 0x100, 0x300, 0x40);
        b[0x200..0x200 + code.len()].copy_from_slice(code);
        b[0x300..0x300 + data.len()].copy_from_slice(data);
        Image::parse(b).unwrap()
    }

    #[test]
    fn stack_tables_and_pushes_are_read_from_the_code() {
        let data = b"Ab\0Cd\0";
        let mut code = Vec::new();
        // mov [ebp-8], 0x402000 ; mov [ebp-4], 0x402003 ; mov [ebp], 0
        code.extend([0xc7, 0x45, 0xf8, 0x00, 0x20, 0x40, 0x00]);
        code.extend([0xc7, 0x45, 0xfc, 0x03, 0x20, 0x40, 0x00]);
        code.extend([0xc7, 0x45, 0x00, 0, 0, 0, 0]);
        // lea eax, [ebp-8] ; call 0x401080
        code.extend([0x8d, 0x45, 0xf8]);
        let at = 0x40_1000 + code.len() as u32 + 5;
        code.push(0xe8);
        code.extend((0x40_1080u32.wrapping_sub(at)).to_le_bytes());
        // push 0x402003
        code.extend([0x68, 0x03, 0x20, 0x40, 0x00]);
        let img = image(&code, data);
        let t = img.stack_tables(0x40_1000, 0x100, 0x40_1080);
        assert_eq!(t[&-8], ["Ab", "Cd"]);
        assert_eq!(img.pushed_texts(0x40_1000, code.len()), ["Cd"]);
        assert_eq!(img.text(0x40_1000), None);
    }
}
