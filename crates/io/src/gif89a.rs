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
pub fn encode_rgba_frames(frames: &[Vec<u8>], w: u16, h: u16, delay_cs: u16) -> io::Result<Vec<u8>> {
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
        let indexed: Vec<u8> = frame
            .chunks(4)
            .map(|px| rgb332(px[0], px[1], px[2]))
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
                if next_code == (1 << code_size) && code_size < 12 {
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
