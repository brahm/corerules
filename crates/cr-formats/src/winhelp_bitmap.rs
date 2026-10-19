//! The pictures of a WinHelp file's `|bmN` internal files: the help
//! compiler's SHG/MRB container of device-independent (6) and
//! device-dependent (5) bitmaps and metafiles (8), each packed as is (0),
//! run-length (1), LZ77 (2) or LZ77 then run-length (3).

use crate::rtf::{dib_to_bmp, wmf_bitmap};
use crate::winhelp::{Bitmap, lz77};

struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn u8(&mut self) -> Option<u8> {
        let b = *self.data.get(self.at)?;
        self.at += 1;
        Some(b)
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from(self.u8()?) | u16::from(self.u8()?) << 8)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from(self.u16()?) | u32::from(self.u16()?) << 16)
    }

    fn cushort(&mut self) -> Option<u32> {
        if *self.data.get(self.at)? & 1 != 0 {
            Some(u32::from(self.u16()? >> 1))
        } else {
            Some(u32::from(self.u8()? >> 1))
        }
    }

    fn culong(&mut self) -> Option<u32> {
        if *self.data.get(self.at)? & 1 != 0 {
            Some(self.u32()? >> 1)
        } else {
            Some(u32::from(self.u16()? >> 1))
        }
    }
}

/// WinHelp's run-length packing: a count byte with its high bit set is
/// followed by that many literal bytes, otherwise by one byte repeated.
fn unrle(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len() * 2);
    let mut i = 0;
    while i < src.len() {
        let n = src[i];
        i += 1;
        if n & 0x80 != 0 {
            let end = (i + usize::from(n & 0x7F)).min(src.len());
            out.extend_from_slice(&src[i..end]);
            i = end;
        } else if let Some(&b) = src.get(i) {
            out.extend(std::iter::repeat_n(b, usize::from(n)));
            i += 1;
        }
    }
    out
}

/// The first bitmap in an SHG/MRB file, as a `.bmp` file.
pub(crate) fn bitmap(file: &[u8]) -> Option<Bitmap> {
    let mut c = Cursor { data: file, at: 0 };
    let magic = c.u16()?;
    if magic != 0x506C && magic != 0x706C {
        return None;
    }
    let count = c.u16()?;
    let offsets: Vec<u32> = (0..count).map(|_| c.u32()).collect::<Option<_>>()?;
    offsets
        .into_iter()
        .find_map(|offset| picture(file, offset as usize))
}

fn picture(file: &[u8], start: usize) -> Option<Bitmap> {
    let mut c = Cursor {
        data: file,
        at: start,
    };
    let kind = c.u8()?;
    let packing = c.u8()?;
    let unpack = |packed: &[u8]| match packing {
        0 => Some(packed.to_vec()),
        1 => Some(unrle(packed)),
        2 => Some(lz77(packed)),
        3 => Some(unrle(&lz77(packed))),
        _ => None,
    };
    if kind == 8 {
        // A metafile: only one that wraps a bitmap is shown.
        let _mapping = c.cushort()?;
        let _width = c.u16()?;
        let _height = c.u16()?;
        let _size = c.culong()?;
        let packed_size = c.culong()? as usize;
        let _hotspot_size = c.culong()?;
        let data_offset = c.u32()? as usize;
        let at = start + data_offset;
        let bmp = wmf_bitmap(&unpack(file.get(at..at + packed_size)?)?)?;
        let dim = |o: usize| u32::from_le_bytes(bmp.get(o..o + 4)?.try_into().ok()?).into();
        let width: Option<u32> = dim(18);
        let height: Option<u32> = dim(22);
        return Some(Bitmap {
            width: width?,
            height: (height? as i32).unsigned_abs(),
            bmp,
        });
    }
    if kind != 5 && kind != 6 {
        return None;
    }
    let x_dpi = c.culong()?;
    let y_dpi = c.culong()?;
    let _planes = c.cushort()?;
    let bit_count = c.cushort()?;
    let width = c.culong()?;
    let height = c.culong()?;
    let colors_used = c.culong()?;
    let colors_important = c.culong()?;
    let size = c.culong()? as usize;
    let _hotspot_size = c.culong()?;
    let data_offset = c.u32()? as usize;
    let _hotspot_offset = c.u32()?;
    if width == 0 || height == 0 || width > 0x8000 || height > 0x8000 || bit_count > 32 {
        return None;
    }
    let colors = if colors_used != 0 {
        colors_used
    } else if bit_count <= 8 {
        1 << bit_count
    } else {
        0
    };
    let palette = if kind == 6 {
        let start = c.at;
        file.get(start..start + 4 * colors as usize)?.to_vec()
    } else if bit_count == 1 {
        // A monochrome device-dependent bitmap: black and white.
        vec![0, 0, 0, 0, 255, 255, 255, 0]
    } else {
        // Other device-dependent bitmaps would need the display's palette.
        return None;
    };
    let packed = file.get(start + data_offset..start + data_offset + size)?;
    let bits = unpack(packed)?;
    let stride = (width as usize * bit_count as usize).div_ceil(32) * 4;
    let mut pixels = vec![0u8; stride * height as usize];
    if kind == 6 {
        let n = pixels.len().min(bits.len());
        pixels[..n].copy_from_slice(&bits[..n]);
    } else {
        // Device-dependent rows are word aligned and top-down.
        let ddb_stride = (width as usize * bit_count as usize).div_ceil(16) * 2;
        for y in 0..height as usize {
            let Some(row) = bits.get(y * ddb_stride..(y + 1) * ddb_stride) else {
                break;
            };
            let to = (height as usize - 1 - y) * stride;
            let n = row.len().min(stride);
            pixels[to..to + n].copy_from_slice(&row[..n]);
        }
    }
    // A packed DIB: BITMAPINFOHEADER, palette, bits.
    let dots_per_metre = |dpi: u32| (u64::from(dpi) * 10_000 / 254) as u32;
    let mut dib = Vec::with_capacity(40 + palette.len() + pixels.len());
    dib.extend_from_slice(&40u32.to_le_bytes());
    dib.extend_from_slice(&width.to_le_bytes());
    dib.extend_from_slice(&height.to_le_bytes());
    dib.extend_from_slice(&1u16.to_le_bytes());
    dib.extend_from_slice(&(bit_count as u16).to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
    dib.extend_from_slice(&dots_per_metre(x_dpi).to_le_bytes());
    dib.extend_from_slice(&dots_per_metre(y_dpi).to_le_bytes());
    dib.extend_from_slice(&(palette.len() as u32 / 4).to_le_bytes());
    dib.extend_from_slice(&colors_important.to_le_bytes());
    dib.extend_from_slice(&palette);
    dib.extend_from_slice(&pixels);
    Some(Bitmap {
        bmp: dib_to_bmp(&dib),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_length_packing_repeats_and_copies() {
        assert_eq!(unrle(&[3, b'x', 0x82, b'a', b'b']), b"xxxab");
    }
}
