//! The program image (`Program/Tsrp3.exe` or `Program/Evermore.exe`) the
//! NPC generator and the treasure roller read their texts from at run
//! time: name tables built on the stack, strings the code pushes, the
//! weapon group tables of the part document's constructor and plain
//! strings. Nothing of the program's text is kept in this crate; only
//! addresses.

use std::collections::HashMap;

use cr_i18n::tr;

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
    /// The image base, which resource data addresses are relative to.
    base: u32,
    /// The address of the resource directory, if there is one.
    resources: Option<u32>,
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
            return Err(Error::Invalid(tr!(
                "{file} is missing: some texts are read from the program",
                file = relative
            )));
        };
        let bytes = std::fs::read(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        if bytes.len() != size || fnv1a(&bytes) != hash {
            return Err(Error::Invalid(tr!(
                "{file}: not the program version whose texts this app knows ({size} bytes)",
                file = path.display(),
                size = bytes.len()
            )));
        }
        Self::parse(bytes).ok_or_else(|| {
            Error::Invalid(tr!("{file}: not a Windows program", file = path.display()))
        })
    }

    fn parse(bytes: Vec<u8>) -> Option<Self> {
        let pe = u32_at(&bytes, 0x3c)? as usize;
        let count = u16_at(&bytes, pe + 6)? as usize;
        let optional = u16_at(&bytes, pe + 20)? as usize;
        let base = u32_at(&bytes, pe + 24 + 28)?;
        // Data directory 2 (resources) of a PE32 optional header.
        let resources = u32_at(&bytes, pe + 24 + 96 + 2 * 8)
            .filter(|&rva| rva != 0 && optional >= 96 + 3 * 8)
            .map(|rva| base.wrapping_add(rva));
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
        Some(Self {
            bytes,
            sections,
            base,
            resources,
        })
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

    /// The caption of dialog resource `id` and its static texts, top to
    /// bottom and left to right (`&` mnemonics removed).
    pub fn dialog_texts(&self, id: u16) -> Option<(String, Vec<StaticText>)> {
        dialog_texts(self.resource(5, id)?)
    }

    /// Resource `id` of type `kind`, in its first language.
    fn resource(&self, kind: u16, id: u16) -> Option<&[u8]> {
        let root = self.resources?;
        let dir = self.at(root);
        // The target of entry `id` (or of the first entry) of the
        // directory at `at`.
        let find = |at: usize, id: Option<u16>| -> Option<u32> {
            let entries = usize::from(u16_at(dir, at + 12)?) + usize::from(u16_at(dir, at + 14)?);
            (0..entries).find_map(|i| {
                let e = at + 16 + 8 * i;
                let name = u32_at(dir, e)?;
                let wanted = id.is_none_or(|id| name == u32::from(id));
                wanted.then(|| u32_at(dir, e + 4)).flatten()
            })
        };
        let sub =
            |target: u32| (target & 0x8000_0000 != 0).then_some((target & 0x7FFF_FFFF) as usize);
        let names = sub(find(0, Some(kind))?)?;
        let languages = sub(find(names, Some(id))?)?;
        let leaf = find(languages, None)? as usize;
        let rva = u32_at(dir, leaf)?;
        let size = u32_at(dir, leaf + 4)? as usize;
        self.at(self.base.wrapping_add(rva)).get(..size)
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

/// A static text control of a dialog template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticText {
    pub style: u32,
    pub text: String,
}

/// A UTF-16 string or an ordinal (`0xFFFF` and a u16) of a dialog
/// template; `None` for an ordinal or an empty string.
fn sz_or_ord(d: &[u8], at: &mut usize) -> Option<Option<String>> {
    if u16_at(d, *at)? == 0xFFFF {
        *at += 4;
        return Some(None);
    }
    let mut units = Vec::new();
    loop {
        let u = u16_at(d, *at)?;
        *at += 2;
        if u == 0 {
            break;
        }
        units.push(u);
    }
    Some(Some(String::from_utf16_lossy(&units)).filter(|t| !t.is_empty()))
}

/// Removes the `&` that marks a mnemonic; `&&` is an `&`.
fn without_mnemonics(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                out.push('&');
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The caption and the static controls of a `DLGTEMPLATE` or
/// `DLGTEMPLATEEX`.
fn dialog_texts(d: &[u8]) -> Option<(String, Vec<StaticText>)> {
    let ex = u16_at(d, 0)? == 1 && u16_at(d, 2)? == 0xFFFF;
    let (style, count, mut at) = if ex {
        (u32_at(d, 12)?, u16_at(d, 16)?, 26)
    } else {
        (u32_at(d, 0)?, u16_at(d, 8)?, 18)
    };
    sz_or_ord(d, &mut at)?; // menu
    sz_or_ord(d, &mut at)?; // class
    let caption = sz_or_ord(d, &mut at)?.unwrap_or_default();
    // DS_SETFONT: point size (and weight, italic, charset), face name.
    if style & 0x40 != 0 {
        at += if ex { 6 } else { 2 };
        sz_or_ord(d, &mut at)?;
    }
    let mut texts = Vec::new();
    for _ in 0..count {
        at = at.next_multiple_of(4);
        // helpID, exStyle, style, x, y, cx, cy, id (u32), or style,
        // exStyle, x, y, cx, cy, id (u16).
        let place = if ex { at + 12 } else { at + 8 };
        let style = u32_at(d, if ex { at + 8 } else { at })?;
        let x = u16_at(d, place)? as i16;
        let y = u16_at(d, place + 2)? as i16;
        at += if ex { 24 } else { 18 };
        let class = if u16_at(d, at)? == 0xFFFF {
            Some(u16_at(d, at + 2)?)
        } else {
            None
        };
        sz_or_ord(d, &mut at)?;
        let text = sz_or_ord(d, &mut at)?;
        at += 2 + usize::from(u16_at(d, at)?);
        // 0x82: a static control.
        if let (Some(0x82), Some(text)) = (class, text) {
            // SS_NOPREFIX keeps the `&`.
            let text = if style & 0x80 != 0 {
                text
            } else {
                without_mnemonics(&text)
            };
            texts.push((y, x, StaticText { style, text }));
        }
    }
    texts.sort_by_key(|t| (t.0, t.1));
    Some((
        without_mnemonics(&caption),
        texts.into_iter().map(|(_, _, t)| t).collect(),
    ))
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
    fn dialog_templates_give_their_static_texts_in_place_order() {
        let utf16 = |t: &str| -> Vec<u8> {
            t.encode_utf16()
                .chain([0])
                .flat_map(u16::to_le_bytes)
                .collect()
        };
        let mut d = Vec::new();
        d.extend(0x40u32.to_le_bytes()); // style: DS_SETFONT
        d.extend(0u32.to_le_bytes());
        d.extend(2u16.to_le_bytes());
        d.extend([0; 8]);
        d.extend([0, 0, 0, 0]); // no menu, no class
        d.extend(utf16("About"));
        d.extend(8u16.to_le_bytes());
        d.extend(utf16("MS Sans Serif"));
        for (y, text) in [(20i16, "Second"), (5, "A && B")] {
            d.resize(d.len().next_multiple_of(4), 0);
            d.extend(0x5002_0000u32.to_le_bytes());
            d.extend(0u32.to_le_bytes());
            d.extend(0i16.to_le_bytes());
            d.extend(y.to_le_bytes());
            d.extend([0; 6]);
            d.extend([0xFF, 0xFF, 0x82, 0]);
            d.extend(utf16(text));
            d.extend(0u16.to_le_bytes());
        }
        let (caption, texts) = dialog_texts(&d).unwrap();
        assert_eq!(caption, "About");
        let texts: Vec<&str> = texts.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["A & B", "Second"]);
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
