use crate::particle::Particle;
use serde::{Deserialize, Serialize};

/// Particle grid stored as **struct-of-arrays** (P1 / ROADMAP): four parallel
/// arrays instead of `Vec<Particle>`. Scan passes (refresh / reaction-collection
/// / heat) iterate a contiguous `element_ids`/`temperatures` slice, so a cache
/// line delivers 32 ids instead of 4; the single-cell physics pass uses the
/// field accessors. The AoS view survives only at the serialize boundary
/// (`particles_vec` / `with_particles`) so save format v2 is unchanged.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    element_ids: Vec<u16>,
    temperatures: Vec<u16>,
    flags: Vec<u8>,
    lifetimes: Vec<u8>,
}

impl Grid {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize);
        Self {
            width,
            height,
            element_ids: vec![crate::element_id::AIR; size],
            temperatures: vec![293; size],
            flags: vec![0; size],
            lifetimes: vec![0; size],
        }
    }

    /// Construct from an AoS `Vec<Particle>` (save-file load, undo restore).
    /// The vec length must equal `width * height`; otherwise it is resized.
    pub fn with_particles(width: u32, height: u32, particles: Vec<Particle>) -> Self {
        let size = (width as usize) * (height as usize);
        let mut g = Self::new(width, height);
        let n = particles.len().min(size);
        for i in 0..n {
            let p = particles[i];
            g.element_ids[i] = p.element_id;
            g.temperatures[i] = p.temperature;
            g.flags[i] = p.flags;
            g.lifetimes[i] = p.lifetime;
        }
        g
    }

    #[inline]
    pub fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    /// Number of cells (length of every SoA array).
    #[inline]
    pub fn len(&self) -> usize {
        self.element_ids.len()
    }

    // ── coordinate accessors ───────────────────────────────────────────────
    /// Owned `Particle` at `(x, y)`. Constructed on the fly from the four
    /// arrays — `Particle` is `Copy` (8 bytes), so this is cheap and lets the
    /// old `&Particle` call sites keep working with minimal churn.
    pub fn get(&self, x: u32, y: u32) -> Option<Particle> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.particle_at(self.index(x, y)))
    }

    pub fn set(&mut self, x: u32, y: u32, p: Particle) {
        if x >= self.width || y >= self.height {
            return;
        }
        self.set_particle_at(self.index(x, y), p);
    }

    /// In-place mutation via a closure — the SoA replacement for the old
    /// `get_mut -> &mut Particle` (a borrow of one `Particle` cannot span four
    /// arrays). Reads the cell, hands a `&mut Particle` to `f`, writes it back.
    /// Returns false if out of bounds (no callback fired).
    pub fn modify<F: FnOnce(&mut Particle)>(&mut self, x: u32, y: u32, f: F) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let i = self.index(x, y);
        let mut p = self.particle_at(i);
        f(&mut p);
        self.set_particle_at(i, p);
        true
    }

    // ── index-based accessors (hot inner loops) ────────────────────────────
    #[inline]
    pub fn particle_at(&self, i: usize) -> Particle {
        Particle {
            element_id: self.element_ids[i],
            temperature: self.temperatures[i],
            flags: self.flags[i],
            lifetime: self.lifetimes[i],
        }
    }

    #[inline]
    pub fn set_particle_at(&mut self, i: usize, p: Particle) {
        self.element_ids[i] = p.element_id;
        self.temperatures[i] = p.temperature;
        self.flags[i] = p.flags;
        self.lifetimes[i] = p.lifetime;
    }

    #[inline]
    pub fn element_at(&self, i: usize) -> u16 {
        self.element_ids[i]
    }
    #[inline]
    pub fn temperature_at(&self, i: usize) -> u16 {
        self.temperatures[i]
    }
    #[inline]
    pub fn set_element_at(&mut self, i: usize, id: u16) {
        self.element_ids[i] = id;
    }
    #[inline]
    pub fn set_temperature_at(&mut self, i: usize, t: u16) {
        self.temperatures[i] = t;
    }
    #[inline]
    pub fn add_temperature_at(&mut self, i: usize, dt: u16) {
        self.temperatures[i] = self.temperatures[i].saturating_add(dt);
    }
    #[inline]
    pub fn set_lifetime_at(&mut self, i: usize, lt: u8) {
        self.lifetimes[i] = lt;
    }
    #[inline]
    pub fn has_flag_at(&self, i: usize, flag: u8) -> bool {
        self.flags[i] & flag != 0
    }
    #[inline]
    pub fn or_flag_at(&mut self, i: usize, flag: u8) {
        self.flags[i] |= flag;
    }
    #[inline]
    pub fn clear_flag_at(&mut self, i: usize, flag: u8) {
        self.flags[i] &= !flag;
    }

    /// Swap two cells across all four arrays (the physics `swap_cells` inner op).
    #[inline]
    pub fn swap_particles(&mut self, a: usize, b: usize) {
        self.element_ids.swap(a, b);
        self.temperatures.swap(a, b);
        self.flags.swap(a, b);
        self.lifetimes.swap(a, b);
    }

    #[inline]
    pub fn is_empty_at(&self, i: usize) -> bool {
        self.element_ids[i] == crate::element_id::AIR
    }

    // ── contiguous slice accessors (the scan-pass win) ─────────────────────
    #[inline]
    pub fn element_ids(&self) -> &[u16] {
        &self.element_ids
    }
    #[inline]
    pub fn temperatures(&self) -> &[u16] {
        &self.temperatures
    }
    #[inline]
    pub fn temperatures_mut(&mut self) -> &mut [u16] {
        &mut self.temperatures
    }
    #[inline]
    pub fn flags(&self) -> &[u8] {
        &self.flags
    }
    #[inline]
    pub fn lifetimes(&self) -> &[u8] {
        &self.lifetimes
    }

    /// Owned-`Particle` iterator (for the few spots that walked `particles.iter()`).
    pub fn iter_particles(&self) -> impl Iterator<Item = Particle> + '_ {
        (0..self.element_ids.len()).map(move |i| self.particle_at(i))
    }

    /// Materialise an AoS `Vec<Particle>` — the serialize/clone/undo boundary.
    pub fn particles_vec(&self) -> Vec<Particle> {
        (0..self.element_ids.len()).map(|i| self.particle_at(i)).collect()
    }

    /// Overwrite every cell from an AoS `Vec<Particle>` (undo restore). The vec
    /// length must equal `width * height`; shorter vecs leave trailing cells.
    pub fn set_particles_vec(&mut self, particles: &[Particle]) {
        let n = particles.len().min(self.element_ids.len());
        for i in 0..n {
            self.set_particle_at(i, particles[i]);
        }
    }

    // ── bulk operations ────────────────────────────────────────────────────
    pub fn clear(&mut self) {
        self.element_ids.fill(crate::element_id::AIR);
        self.temperatures.fill(293);
        self.flags.fill(0);
        self.lifetimes.fill(0);
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        let mut new_grid = Self::new(new_width, new_height);
        let min_w = self.width.min(new_width);
        let min_h = self.height.min(new_height);
        for y in 0..min_h {
            for x in 0..min_w {
                let old_idx = self.index(x, y);
                let new_idx = new_grid.index(x, y);
                new_grid.set_particle_at(new_idx, self.particle_at(old_idx));
            }
        }
        *self = new_grid;
    }

    pub fn count_non_empty(&self) -> usize {
        self.element_ids.iter().filter(|&&id| id != crate::element_id::AIR).count()
    }

    pub fn to_compact(&self) -> Vec<crate::particle::ParticleData> {
        let mut out = Vec::with_capacity(self.count_non_empty());
        for y in 0..self.height {
            for x in 0..self.width {
                let i = self.index(x, y);
                if !self.is_empty_at(i) {
                    out.push(crate::particle::ParticleData {
                        x,
                        y,
                        particle: self.particle_at(i),
                    });
                }
            }
        }
        out
    }

    pub fn from_compact(width: u32, height: u32, data: &[crate::particle::ParticleData]) -> Self {
        let mut g = Self::new(width, height);
        for pd in data {
            if pd.x < width && pd.y < height {
                g.set(pd.x, pd.y, pd.particle);
            }
        }
        g
    }

    /// Snapshot for renderer - returns RGBA bytes based on element colors.
    pub fn to_rgba_buffer<F>(&self, color_fn: F) -> Vec<u8>
    where
        F: Fn(u16) -> [u8; 4],
    {
        let mut buf = Vec::with_capacity((self.width * self.height * 4) as usize);
        for &id in &self.element_ids {
            let c = color_fn(id);
            buf.extend_from_slice(&c);
        }
        buf
    }
}

/// Grid snapshot for rendering thread - Arc<RwLock> friendly
#[derive(Clone, Debug)]
pub struct GridSnapshot {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA
}

impl GridSnapshot {
    pub fn empty(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        }
    }
}
