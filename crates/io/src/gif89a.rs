//! Tiny GIF89a writer (3-3-2 palette, looping) used for in-game recording.

use std::io::{self, Write};

fn rgb332(r: u8, g: u8, b: u8) -> u8 {
    ((r >> 5) << 5) | ((g >> 5) << 2) | (b >> 6)
}

fn palette_332() -> [u8; 768] {
    let mut pal = [0u8; 768];
    for i in 0..256 {
        let r = (((i >> 5) & 7) * 255 / 7) as u8;
        let g = (((i >> 2) & 7) * 255 / 7) as u8;
        let b = ((i & 3) * 255 / 3) as u8;
        pal[i * 3] = r;
        pal[i * 3 + 1] = g;
        pal[i * 3 + 2] = b;
    }
    pal
}

/// Encode RGBA frames (`w*h*4` each) into a looping GIF.
pub fn encode_rgba_frames(
    frames: &[Vec<u8>],
    w: u16,
    h: u16,
    delay_cs: u16,
) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(b"GIF89a");
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.push(0xF7); // global table, 8-bit
    out.push(0); // bg
    out.push(0); // aspect
    out.extend_from_slice(&palette_332());
    // Netscape loop
    out.extend_from_slice(&[0x21, 0xFF, 0x0B]);
    out.extend_from_slice(b"NETSCAPE2.0");
    out.extend_from_slice(&[0x03, 0x01, 0x00, 0x00, 0x00]);

    for frame in frames {
        // P9b fuzz fix: a frame whose length is not a multiple of 4 leaves a
        // trailing partial pixel — index it defensively instead of panicking
        // (missing channels read as 0).
        let indexed: Vec<u8> = frame
            .chunks(4)
            .map(|px| {
                rgb332(
                    px.first().copied().unwrap_or(0),
                    px.get(1).copied().unwrap_or(0),
                    px.get(2).copied().unwrap_or(0),
                )
            })
            .collect();
        // Graphic Control
        out.extend_from_slice(&[0x21, 0xF9, 0x04, 0x00]);
        out.extend_from_slice(&delay_cs.to_le_bytes());
        out.extend_from_slice(&[0x00, 0x00]);
        // Image descriptor
        out.push(0x2C);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&h.to_le_bytes());
        out.push(0x00);
        lzw_encode(&indexed, 8, &mut out)?;
    }
    out.push(0x3B);
    Ok(out)
}

fn lzw_encode(indices: &[u8], min_code: u8, out: &mut Vec<u8>) -> io::Result<()> {
    out.push(min_code);
    let clear = 1u16 << min_code;
    let end = clear + 1;
    let mut code_size = min_code + 1;
    let mut next_code = end + 1;
    let mut dict: std::collections::HashMap<Vec<u8>, u16> = std::collections::HashMap::new();
    for i in 0..clear {
        dict.insert(vec![i as u8], i);
    }

    let mut bw = BitWriter::new();
    bw.write(clear, code_size);
    let mut w: Vec<u8> = Vec::new();
    for &k in indices {
        let mut wk = w.clone();
        wk.push(k);
        if dict.contains_key(&wk) {
            w = wk;
        } else {
            if let Some(&code) = dict.get(&w) {
                bw.write(code, code_size);
            }
            if next_code < 4096 {
                dict.insert(wk, next_code);
                next_code += 1;
                // The decoder's code table lags the encoder's by one entry, so the
                // code width must be increased one entry *after* the table fills the
                // current width. Using `== (1 << code_size)` bumps one entry too
                // early, desynchronising the bit stream and producing a GIF that no
                // standard decoder (browsers, libgif, Pillow) can read once the
                // dictionary grows past the first code-size boundary.
                if next_code == (1 << code_size) + 1 && code_size < 12 {
                    code_size += 1;
                }
            } else {
                bw.write(clear, code_size);
                dict.clear();
                for i in 0..clear {
                    dict.insert(vec![i as u8], i);
                }
                code_size = min_code + 1;
                next_code = end + 1;
            }
            w = vec![k];
        }
    }
    if !w.is_empty() {
        if let Some(&code) = dict.get(&w) {
            bw.write(code, code_size);
        }
    }
    bw.write(end, code_size);
    bw.flush_blocks(out);
    Ok(())
}

struct BitWriter {
    acc: u32,
    bits: u8,
    bytes: Vec<u8>,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            acc: 0,
            bits: 0,
            bytes: Vec::new(),
        }
    }
    fn write(&mut self, val: u16, width: u8) {
        self.acc |= (val as u32) << self.bits;
        self.bits += width;
        while self.bits >= 8 {
            self.bytes.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.bits -= 8;
        }
    }
    fn flush_blocks(&mut self, out: &mut Vec<u8>) {
        if self.bits > 0 {
            self.bytes.push((self.acc & 0xFF) as u8);
            self.acc = 0;
            self.bits = 0;
        }
        for chunk in self.bytes.chunks(255) {
            out.push(chunk.len() as u8);
            out.extend_from_slice(chunk);
        }
        out.push(0);
    }
}

pub fn write_gif_file<P: AsRef<std::path::Path>>(
    path: P,
    frames: &[Vec<u8>],
    w: u16,
    h: u16,
    delay_cs: u16,
) -> io::Result<()> {
    let bytes = encode_rgba_frames(frames, w, h, delay_cs)?;
    let mut f = std::fs::File::create(path)?;
    f.write_all(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal LSB-first GIF LZW decoder, used only to round-trip our encoder.
    fn lzw_decode(data: &[u8], min_code: u8) -> Option<Vec<u8>> {
        let clear = 1u16 << min_code;
        let eoi = clear + 1;
        let mut width: u32 = min_code as u32 + 1;
        let mut acc: u64 = 0;
        let mut bits: u32 = 0;
        let mut pos: usize = 0;
        // Codes 0..clear are colour entries; clear/eoi are empty placeholders so
        // that table[c] can be indexed directly by code value.
        let mut table: Vec<Vec<u8>> = (0..=(eoi))
            .map(|i| if i < clear { vec![i as u8] } else { vec![] })
            .collect();
        let mut next_code = eoi + 1;
        let mut out: Vec<u8> = Vec::new();
        let mut prev: Option<u16> = None;
        loop {
            while bits < width {
                if pos >= data.len() {
                    return Some(out);
                }
                acc |= (data[pos] as u64) << bits;
                pos += 1;
                bits += 8;
            }
            let c = (acc & ((1u64 << width) - 1)) as u16;
            acc >>= width;
            bits -= width;

            if c == eoi {
                break;
            }
            if c == clear {
                table = (0..=(eoi))
                    .map(|i| if i < clear { vec![i as u8] } else { vec![] })
                    .collect();
                next_code = eoi + 1;
                width = min_code as u32 + 1;
                prev = None;
                continue;
            }
            let entry = if let Some(p) = prev {
                if (c as usize) < table.len() && !table[c as usize].is_empty() {
                    table[c as usize].clone()
                } else if c == next_code {
                    let mut e = table[p as usize].clone();
                    e.push(table[p as usize][0]);
                    e
                } else {
                    return None;
                }
            } else {
                table.get(c as usize).cloned().filter(|e| !e.is_empty())?
            };
            out.extend_from_slice(&entry);
            if let Some(p) = prev {
                let mut new_entry = table[p as usize].clone();
                new_entry.push(entry[0]);
                table.push(new_entry);
                next_code += 1;
                // The decoder's code table lags the encoder's by one entry, so its
                // width-bump threshold is `2^width` (Rule A) while the encoder uses
                // `2^width + 1`. Using the encoder's threshold here desynchronises
                // the stream — this is exactly the off-by-one the fix addresses.
                if next_code == (1u16 << width) && width < 12 {
                    width += 1;
                }
            }
            prev = Some(c);
        }
        Some(out)
    }

    #[test]
    fn roundtrip_crosses_code_size_boundary() {
        // A noisy 4-colour image big enough to grow the LZW dictionary well past
        // the first code-size boundary — exactly the case the off-by-one broke.
        let (w, h) = (64u16, 64u16);
        let palette: [[u8; 3]; 4] = [[200, 30, 30], [30, 200, 30], [30, 30, 200], [220, 220, 40]];
        let mut state: u64 = 0x9e3779b97f4a7c15;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as u32
        };
        let raw: Vec<usize> = (0..(w as usize * h as usize))
            .map(|_| (next() & 3) as usize)
            .collect();
        // `encode_rgba_frames` re-quantises each pixel through the 3-3-2 palette,
        // so the decoded indices are rgb332(colour), not the original colour id.
        let expected: Vec<u8> = raw
            .iter()
            .map(|&i| {
                let c = palette[i];
                ((c[0] >> 5) << 5) | ((c[1] >> 5) << 2) | (c[2] >> 6)
            })
            .collect();
        let frame: Vec<u8> = raw
            .iter()
            .flat_map(|&i| {
                let c = palette[i];
                [c[0], c[1], c[2], 255]
            })
            .collect();

        let gif = encode_rgba_frames(&[frame], w, h, 5).unwrap();
        // Locate the single image's LZW sub-block stream.
        let mut i = gif.iter().copied().position(|b| b == 0x2C).unwrap() + 10;
        let min_code = gif[i];
        i += 1;
        let mut lzw = Vec::new();
        loop {
            let n = gif[i] as usize;
            i += 1;
            if n == 0 {
                break;
            }
            lzw.extend_from_slice(&gif[i..i + n]);
            i += n;
        }
        let decoded = lzw_decode(&lzw, min_code).expect("LZW stream must decode");
        assert_eq!(decoded.len(), expected.len());
        assert_eq!(
            decoded, expected,
            "round-tripped indices must match exactly"
        );
    }

    /// P9b encode fuzz: arbitrary frame contents (and even mismatched lengths)
    /// must never panic the encoder — it must always produce a well-formed
    /// GIF (header + trailer) or a clean error.
    #[test]
    fn encode_fuzz_arbitrary_frames() {
        let mut st: u64 = 0xfeed_beef;
        let mut next = || {
            st ^= st << 13;
            st ^= st >> 7;
            st ^= st << 17;
            st
        };
        for i in 0..200 {
            let (w, h) = ((next() % 40) as u16, (next() % 40) as u16);
            // Sometimes deliberately wrong buffer lengths.
            let len = if i % 7 == 0 {
                (next() % 700) as usize
            } else {
                w as usize * h as usize * 4
            };
            let frame: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            let bytes = encode_rgba_frames(&[frame], w, h, 5).expect("encode must not panic");
            assert!(bytes.starts_with(b"GIF89a"));
            assert_eq!(*bytes.last().unwrap(), 0x3B);
        }
    }

    /// P9b round-trip fuzz: random frames of the CORRECT length must encode
    /// and decode back to the rgb332-quantised original indices.
    #[test]
    fn roundtrip_fuzz_random_frames() {
        let mut st: u64 = 0x5eed_1234;
        let mut next = || {
            st ^= st << 13;
            st ^= st >> 7;
            st ^= st << 17;
            st
        };
        for _ in 0..60 {
            let (w, h) = ((next() % 24 + 1) as u16, (next() % 24 + 1) as u16);
            let n = w as usize * h as usize;
            let frame: Vec<u8> = (0..n * 4).map(|_| next() as u8).collect();
            let expected: Vec<u8> = frame
                .chunks(4)
                .map(|px| rgb332(px[0], px[1], px[2]))
                .collect();
            let gif = encode_rgba_frames(&[frame], w, h, 4).expect("encode");
            // Locate the single image's LZW sub-block stream.
            let mut i = gif.iter().copied().position(|b| b == 0x2C).unwrap() + 10;
            let min_code = gif[i];
            i += 1;
            let mut lzw = Vec::new();
            loop {
                let n = gif[i] as usize;
                i += 1;
                if n == 0 {
                    break;
                }
                lzw.extend_from_slice(&gif[i..i + n]);
                i += n;
            }
            let decoded = lzw_decode(&lzw, min_code).expect("decode must not panic");
            assert_eq!(decoded.len(), expected.len());
            assert_eq!(decoded, expected, "round-trip must be exact");
        }
    }
}
