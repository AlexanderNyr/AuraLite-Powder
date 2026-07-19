use crate::particle::Particle;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub particles: Vec<Particle>,
}

impl Grid {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize);
        Self {
            width,
            height,
            particles: vec![Particle::air(); size],
        }
    }

    #[inline]
    pub fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    pub fn get(&self, x: u32, y: u32) -> Option<&Particle> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(&self.particles[self.index(x, y)])
    }

    pub fn get_mut(&mut self, x: u32, y: u32) -> Option<&mut Particle> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = self.index(x, y);
        Some(&mut self.particles[idx])
    }

    pub fn set(&mut self, x: u32, y: u32, p: Particle) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = self.index(x, y);
        self.particles[idx] = p;
    }

    pub fn clear(&mut self) {
        for p in &mut self.particles {
            *p = Particle::air();
        }
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        let mut new_grid = Self::new(new_width, new_height);
        // migrate existing particles clamped to new size
        let min_w = self.width.min(new_width);
        let min_h = self.height.min(new_height);
        for y in 0..min_h {
            for x in 0..min_w {
                let old_idx = self.index(x, y);
                let new_idx = new_grid.index(x, y);
                new_grid.particles[new_idx] = self.particles[old_idx];
            }
        }
        *self = new_grid;
    }

    pub fn count_non_empty(&self) -> usize {
        self.particles.iter().filter(|p| !p.is_empty()).count()
    }

    pub fn to_compact(&self) -> Vec<crate::particle::ParticleData> {
        let mut out = Vec::with_capacity(self.count_non_empty());
        for y in 0..self.height {
            for x in 0..self.width {
                let p = self.particles[self.index(x, y)];
                if !p.is_empty() {
                    out.push(crate::particle::ParticleData { x, y, particle: p });
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

    /// Snapshot for renderer - returns RGBA bytes based on element colors
    /// Uses registry from elements crate if available, else fallback
    pub fn to_rgba_buffer<F>(&self, color_fn: F) -> Vec<u8>
    where
        F: Fn(u16) -> [u8; 4],
    {
        let mut buf = Vec::with_capacity((self.width * self.height * 4) as usize);
        for p in &self.particles {
            let c = color_fn(p.element_id);
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
