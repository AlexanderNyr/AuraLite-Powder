use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Particle {
    pub element_id: u16,
    pub temperature: u16,
    pub flags: u8,
    pub lifetime: u8,
}

impl Particle {
    pub const fn new(element_id: u16, temperature: u16) -> Self {
        Self {
            element_id,
            temperature,
            flags: 0,
            lifetime: 0,
        }
    }
    pub fn air() -> Self {
        Self {
            element_id: crate::element_id::AIR,
            temperature: 293, // ~20C in Kelvin-like scale? Using 293 as baseline
            flags: 0,
            lifetime: 0,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.element_id == crate::element_id::AIR
    }
    pub fn with_temp(mut self, t: u16) -> Self {
        self.temperature = t;
        self
    }
    pub fn with_lifetime(mut self, lt: u8) -> Self {
        self.lifetime = lt;
        self
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self::air()
    }
}

/// Compact snapshot of non-empty particle for saving
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ParticleData {
    pub x: u32,
    pub y: u32,
    pub particle: Particle,
}
