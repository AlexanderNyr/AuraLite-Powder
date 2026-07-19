use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub fn to_u32(&self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }
    pub fn from_u32(v: u32) -> Self {
        Self {
            r: ((v >> 24) & 0xFF) as u8,
            g: ((v >> 16) & 0xFF) as u8,
            b: ((v >> 8) & 0xFF) as u8,
            a: (v & 0xFF) as u8,
        }
    }
    pub fn to_array(&self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Default for Rgba {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

/// Common palette
pub mod palette {
    use super::Rgba;
    pub const AIR: Rgba = Rgba::new(0, 0, 0, 0);
    pub const SAND: Rgba = Rgba::rgb(194, 178, 128);
    pub const WATER: Rgba = Rgba::rgb(64, 164, 223);
    pub const STONE: Rgba = Rgba::rgb(120, 120, 120);
}
