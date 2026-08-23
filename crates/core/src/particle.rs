use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Particle {
    pub element_id: u16,
    pub temperature: u16,
    pub flags: u8,
    pub lifetime: u8,
}

impl Particle {
    /// Set after a particle moves during the physics pass so it is not stepped twice.
    pub const FLAG_MOVED: u8 = 1 << 0;
    /// Set after a reaction consumes this cell so a second reaction cannot fire.
    pub const FLAG_REACTED: u8 = 1 << 1;

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

    #[inline]
    pub fn has_flag(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    #[inline]
    pub fn set_flag(&mut self, flag: u8) {
        self.flags |= flag;
    }

    #[inline]
    pub fn clear_flag(&mut self, flag: u8) {
        self.flags &= !flag;
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
