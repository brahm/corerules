//! Windows bitmaps (`.bmp`), the pictures of monster types, races and
//! characters: decoded here so showing them does not depend on which
//! image loaders the system has.
//!
//! Every picture of the CD and of the community add-ons is an
//! uncompressed 8-bit bitmap with a `BITMAPINFOHEADER`; 1, 4, 24 and 32
//! bits are read too. Compressed bitmaps are not. [`encode_indexed`]
//! writes the 8-bit bitmaps the portraits a user adds are kept as.

/// A decoded picture: rows from the top, 4 bytes (R, G, B, A) a pixel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Decodes a bitmap file; `None` when it is not one this reads.
pub fn decode(data: &[u8]) -> Option<Image> {
    let u16_at = |at: usize| Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
    if data.get(..2)? != b"BM" {
        return None;
    }
    let offset = u32_at(10)? as usize;
    let header = u32_at(14)? as usize;
    if header < 40 {
        return None;
    }
    let width = u32_at(18)? as i32;
    let raw_height = u32_at(22)? as i32;
    let bits = u16_at(28)?;
    let compression = u32_at(30)?;
    if width <= 0 || raw_height == 0 || compression != 0 || width > 16_384 {
        return None;
    }
    let (width, height) = (width as usize, raw_height.unsigned_abs() as usize);
    if height > 16_384 {
        return None;
    }
    let palette = if bits <= 8 {
        palette(data)?
    } else {
        Vec::new()
    };
    let stride = (width * bits as usize).div_ceil(32) * 4;
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let row = if raw_height > 0 { height - 1 - y } else { y };
        let start = offset + row * stride;
        let line = data.get(start..start + stride)?;
        for x in 0..width {
            let rgb = match bits {
                1 | 4 | 8 => {
                    let bit = x * bits as usize;
                    let byte = line[bit / 8];
                    let shift = 8 - bits as usize - bit % 8;
                    let index = (byte >> shift) as usize & ((1 << bits) - 1);
                    *palette.get(index).unwrap_or(&[0, 0, 0])
                }
                24 | 32 => {
                    let p = &line[x * bits as usize / 8..];
                    [p[2], p[1], p[0]]
                }
                _ => return None,
            };
            rgba.extend_from_slice(&rgb);
            rgba.push(255);
        }
    }
    Some(Image {
        width: width as u32,
        height: height as u32,
        rgba,
    })
}

/// The colors of a bitmap of 8 bits or fewer; `None` for other files.
/// Reading the header and the palette is enough.
pub fn palette(data: &[u8]) -> Option<Vec<[u8; 3]>> {
    let u16_at = |at: usize| Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
    if data.get(..2)? != b"BM" {
        return None;
    }
    let header = u32_at(14)? as usize;
    let bits = u16_at(28)?;
    if header < 40 || !matches!(bits, 1 | 4 | 8) {
        return None;
    }
    let used = match u32_at(46)? {
        0 => 1 << bits,
        n => n as usize,
    };
    (0..used.min(256))
        .map(|i| {
            let at = 14 + header + i * 4;
            data.get(at..at + 3).map(|b| [b[2], b[1], b[0]])
        })
        .collect()
}

/// An 8-bit bitmap, as the shipped portraits are and the original wants
/// them (it refuses pictures with more than 256 colors): `indices` holds
/// one palette index a pixel, rows from the top, and `palette` up to 256
/// colors.
pub fn encode_indexed(width: u32, height: u32, palette: &[[u8; 3]], indices: &[u8]) -> Vec<u8> {
    let (w, h) = (width as usize, height as usize);
    assert_eq!(indices.len(), w * h, "one index a pixel");
    assert!(palette.len() <= 256, "at most 256 colors");
    let stride = w.div_ceil(4) * 4;
    let offset = 14 + 40 + 256 * 4;
    let size = offset + stride * h;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(offset as u32).to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&8u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((stride * h) as u32).to_le_bytes());
    // No resolution, and every one of the 256 palette entries used, as in
    // most of the shipped portraits.
    out.extend_from_slice(&[0; 16]);
    for i in 0..256 {
        let [r, g, b] = palette.get(i).copied().unwrap_or_default();
        out.extend_from_slice(&[b, g, r, 0]);
    }
    if w > 0 {
        for row in indices.chunks_exact(w).rev() {
            out.extend_from_slice(row);
            out.resize(out.len() + stride - w, 0);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bitmap `width` × `height`, 8 bits, from rows given bottom first.
    fn bitmap(width: u32, height: i32, palette: &[[u8; 3]], rows: &[&[u8]]) -> Vec<u8> {
        let stride = (width as usize).div_ceil(4) * 4;
        let offset = 14 + 40 + palette.len() * 4;
        let mut out = b"BM".to_vec();
        out.extend_from_slice(&((offset + stride * rows.len()) as u32).to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&8u16.to_le_bytes());
        out.extend_from_slice(&[0; 12]);
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(palette.len() as u32).to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        for [r, g, b] in palette {
            out.extend_from_slice(&[*b, *g, *r, 0]);
        }
        for row in rows {
            let mut line = row.to_vec();
            line.resize(stride, 0);
            out.extend_from_slice(&line);
        }
        out
    }

    #[test]
    fn eight_bit_bitmaps_decode_from_the_bottom_row() {
        let palette = [[255, 0, 0], [0, 0, 255]];
        let data = bitmap(3, 2, &palette, &[&[0, 0, 1], &[1, 1, 0]]);
        let image = decode(&data).unwrap();
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!(
            image.rgba,
            [
                0, 0, 255, 255, 0, 0, 255, 255, 255, 0, 0, 255, // top row
                255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255,
            ]
        );
        let top_down = bitmap(3, -2, &palette, &[&[1, 1, 0], &[0, 0, 1]]);
        assert_eq!(decode(&top_down).unwrap(), image);
    }

    #[test]
    fn other_files_are_refused() {
        assert_eq!(decode(b"PNG"), None);
        let mut data = bitmap(1, 1, &[[0, 0, 0]], &[&[0]]);
        data.truncate(data.len() - 4);
        assert_eq!(decode(&data), None);
    }

    #[test]
    fn indexed_bitmaps_decode_back() {
        let palette = [[255, 0, 0], [0, 255, 0], [10, 20, 30]];
        // Rows from the top; 5 pixels pad each row to 8 bytes.
        let indices = [0, 1, 2, 1, 0, 2, 2, 2, 2, 2];
        let data = encode_indexed(5, 2, &palette, &indices);
        assert_eq!(data.len(), 14 + 40 + 1024 + 8 * 2);
        assert_eq!(u16::from_le_bytes([data[28], data[29]]), 8);
        let back = decode(&data).unwrap();
        assert_eq!((back.width, back.height), (5, 2));
        let pixels: Vec<[u8; 3]> = back.rgba.chunks(4).map(|p| [p[0], p[1], p[2]]).collect();
        let want: Vec<[u8; 3]> = indices.iter().map(|&i| palette[i as usize]).collect();
        assert_eq!(pixels, want);
        assert_eq!(super::palette(&data).unwrap()[..3], palette);
    }
}
