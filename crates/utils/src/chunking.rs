use serde::{Deserialize, Serialize};

pub const CHUNK_SIZE: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkMeta {
    pub x: u32,
    pub y: u32,
    /// Bounding box of non-empty particles within chunk, for culling
    pub dirty_min_x: u32,
    pub dirty_min_y: u32,
    pub dirty_max_x: u32,
    pub dirty_max_y: u32,
    pub is_empty: bool,
    pub active: bool,
}

impl ChunkMeta {
    pub fn new(chunk_x: u32, chunk_y: u32) -> Self {
        Self {
            x: chunk_x,
            y: chunk_y,
            dirty_min_x: CHUNK_SIZE as u32,
            dirty_min_y: CHUNK_SIZE as u32,
            dirty_max_x: 0,
            dirty_max_y: 0,
            is_empty: true,
            active: false,
        }
    }
    pub fn mark_dirty(&mut self, local_x: u32, local_y: u32) {
        self.is_empty = false;
        self.active = true;
        if local_x < self.dirty_min_x {
            self.dirty_min_x = local_x;
        }
        if local_y < self.dirty_min_y {
            self.dirty_min_y = local_y;
        }
        if local_x > self.dirty_max_x {
            self.dirty_max_x = local_x;
        }
        if local_y > self.dirty_max_y {
            self.dirty_max_y = local_y;
        }
    }
    pub fn clear(&mut self) {
        self.dirty_min_x = CHUNK_SIZE as u32;
        self.dirty_min_y = CHUNK_SIZE as u32;
        self.dirty_max_x = 0;
        self.dirty_max_y = 0;
        self.is_empty = true;
        self.active = false;
    }
}

#[derive(Clone, Debug)]
pub struct ChunkPool {
    pub chunks_x: u32,
    pub chunks_y: u32,
    pub metas: Vec<ChunkMeta>,
}

impl Default for ChunkPool {
    fn default() -> Self {
        Self {
            chunks_x: 0,
            chunks_y: 0,
            metas: Vec::new(),
        }
    }
}

impl ChunkPool {
    pub fn new(grid_width: u32, grid_height: u32) -> Self {
        let chunks_x = grid_width.div_ceil(CHUNK_SIZE as u32);
        let chunks_y = grid_height.div_ceil(CHUNK_SIZE as u32);
        let mut metas = Vec::with_capacity((chunks_x * chunks_y) as usize);
        for cy in 0..chunks_y {
            for cx in 0..chunks_x {
                metas.push(ChunkMeta::new(cx, cy));
            }
        }
        Self {
            chunks_x,
            chunks_y,
            metas,
        }
    }
    pub fn index(&self, chunk_x: u32, chunk_y: u32) -> usize {
        (chunk_y * self.chunks_x + chunk_x) as usize
    }
    pub fn get_mut(&mut self, cx: u32, cy: u32) -> Option<&mut ChunkMeta> {
        if cx >= self.chunks_x || cy >= self.chunks_y {
            return None;
        }
        let idx = self.index(cx, cy);
        self.metas.get_mut(idx)
    }
    pub fn rebuild(&mut self, grid_width: u32, grid_height: u32) {
        *self = Self::new(grid_width, grid_height);
    }
    /// Return list of active chunk indices for parallel processing
    pub fn active_chunks(&self) -> Vec<(u32, u32)> {
        self.metas
            .iter()
            .filter(|m| m.active || !m.is_empty)
            .map(|m| (m.x, m.y))
            .collect()
    }

    /// Active chunks plus a `halo` ring so particles can fall into empty neighbours.
    pub fn expanded_active(&self, halo: i32) -> Vec<(u32, u32)> {
        use std::collections::BTreeSet;
        let mut set = BTreeSet::new();
        for m in &self.metas {
            if !(m.active || !m.is_empty) {
                continue;
            }
            for dy in -halo..=halo {
                for dx in -halo..=halo {
                    let cx = m.x as i32 + dx;
                    let cy = m.y as i32 + dy;
                    if cx >= 0
                        && cy >= 0
                        && (cx as u32) < self.chunks_x
                        && (cy as u32) < self.chunks_y
                    {
                        set.insert((cx as u32, cy as u32));
                    }
                }
            }
        }
        set.into_iter().collect()
    }
}
