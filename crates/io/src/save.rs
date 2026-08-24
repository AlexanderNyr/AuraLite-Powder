use crate::error::IoError;
use aura_lite_core::simulation::SimulationSettings;
use aura_lite_core::{Grid, NeutronEvent, ParticleData, SimulationState};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;

pub const CURRENT_VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveFile {
    pub version: u32,
    pub timestamp: u64,
    pub grid_width: u32,
    pub grid_height: u32,
    pub tick_rate: u32,
    pub seed: u64,
    pub particles: Vec<ParticleData>,
    pub settings: SimulationSettings,
    pub full_grid: Option<Vec<aura_lite_core::Particle>>,
    #[serde(default)]
    pub tick: u64,
    #[serde(default)]
    pub neutron_queue: Vec<NeutronEvent>,
    #[serde(default)]
    pub reaction_count: u64,
    #[serde(default)]
    pub fission_count: u64,
    #[serde(default)]
    pub fusion_count: u64,
    #[serde(default)]
    pub decay_count: u64,
    #[serde(default)]
    pub vel_x: Vec<i8>,
    #[serde(default)]
    pub vel_y: Vec<i8>,
    #[serde(default)]
    pub pressure: Vec<u16>,
    #[serde(default)]
    pub power: f32,
    #[serde(default)]
    pub mission: Option<aura_lite_core::MissionSave>,
}

/// On-disk layout used by version-1 `.aura` files (no simulation counters).
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SaveFileV1 {
    version: u32,
    timestamp: u64,
    grid_width: u32,
    grid_height: u32,
    tick_rate: u32,
    seed: u64,
    particles: Vec<ParticleData>,
    settings: SimulationSettings,
    full_grid: Option<Vec<aura_lite_core::Particle>>,
}

impl From<SaveFileV1> for SaveFile {
    fn from(v1: SaveFileV1) -> Self {
        Self {
            version: v1.version,
            timestamp: v1.timestamp,
            grid_width: v1.grid_width,
            grid_height: v1.grid_height,
            tick_rate: v1.tick_rate,
            seed: v1.seed,
            particles: v1.particles,
            settings: v1.settings,
            full_grid: v1.full_grid,
            tick: 0,
            neutron_queue: Vec::new(),
            reaction_count: 0,
            fission_count: 0,
            fusion_count: 0,
            decay_count: 0,
            vel_x: Vec::new(),
            vel_y: Vec::new(),
            pressure: Vec::new(),
            power: 0.0,
            mission: None,
        }
    }
}

impl SaveFile {
    pub fn from_grid(
        grid: &Grid,
        settings: &SimulationSettings,
        seed: u64,
        tick_rate: u32,
        full: bool,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            version: CURRENT_VERSION,
            timestamp,
            grid_width: grid.width,
            grid_height: grid.height,
            tick_rate,
            seed,
            particles: grid.to_compact(),
            settings: settings.clone(),
            full_grid: if full {
                Some(grid.particles.clone())
            } else {
                None
            },
            tick: 0,
            neutron_queue: Vec::new(),
            reaction_count: 0,
            fission_count: 0,
            fusion_count: 0,
            decay_count: 0,
            vel_x: Vec::new(),
            vel_y: Vec::new(),
            pressure: Vec::new(),
            power: 0.0,
            mission: None,
        }
    }

    pub fn from_simulation(sim: &SimulationState, full: bool) -> Self {
        let mut save = Self::from_grid(&sim.grid, &sim.settings, sim.seed, sim.tick_rate, full);
        save.tick = sim.tick;
        save.neutron_queue = sim.neutron_queue.iter().copied().collect();
        save.reaction_count = sim.reaction_count;
        save.fission_count = sim.fission_count;
        save.fusion_count = sim.fusion_count;
        save.decay_count = sim.decay_count;
        save.vel_x = sim.velocities.vx.clone();
        save.vel_y = sim.velocities.vy.clone();
        save.pressure = sim.pressure.p.clone();
        save.power = sim.power;
        save.mission = sim.mission.clone();
        save
    }

    pub fn to_grid(&self) -> Result<(Grid, SimulationSettings), IoError> {
        if self.version > CURRENT_VERSION {
            return Err(IoError::VersionMismatch {
                file: self.version,
                current: CURRENT_VERSION,
            });
        }
        let grid = if let Some(full) = &self.full_grid {
            Grid {
                width: self.grid_width,
                height: self.grid_height,
                particles: full.clone(),
            }
        } else {
            Grid::from_compact(self.grid_width, self.grid_height, &self.particles)
        };
        Ok((grid, self.settings.clone()))
    }

    /// Restore grid, settings, neutron queue and counters onto an existing simulation.
    pub fn apply_to(&self, sim: &mut SimulationState) -> Result<(), IoError> {
        let (grid, settings) = self.to_grid()?;
        sim.grid = grid;
        sim.settings = settings;
        sim.tick_rate = self.tick_rate;
        sim.seed = self.seed;
        sim.tick = self.tick;
        sim.neutron_queue = VecDeque::from(self.neutron_queue.clone());
        sim.reaction_count = self.reaction_count;
        sim.fission_count = self.fission_count;
        sim.fusion_count = self.fusion_count;
        sim.decay_count = self.decay_count;
        sim.power = self.power;
        sim.mission = self.mission.clone();
        let n = sim.grid.particles.len();
        sim.velocities.sync_len(n);
        if self.vel_x.len() == n && self.vel_y.len() == n {
            sim.velocities.vx = self.vel_x.clone();
            sim.velocities.vy = self.vel_y.clone();
        }
        sim.pressure.sync_len(n);
        if self.pressure.len() == n {
            sim.pressure.p = self.pressure.clone();
        }
        sim.chunk_pool = aura_lite_core::ChunkPool::new(sim.grid.width, sim.grid.height);
        Ok(())
    }
}

fn encode_save(save: &SaveFile, use_compression: bool) -> Result<Vec<u8>, IoError> {
    let encoded = bincode::serde::encode_to_vec(save, bincode::config::standard())
        .map_err(|e| IoError::Serialization(e.to_string()))?;

    if use_compression {
        #[cfg(feature = "compression")]
        {
            let compressed = zstd::encode_all(encoded.as_slice(), 3)
                .map_err(|e| IoError::Compression(e.to_string()))?;
            return Ok(compressed);
        }
        #[cfg(not(feature = "compression"))]
        {
            log::warn!("compression feature not enabled, saving uncompressed");
            return Ok(encoded);
        }
    }
    Ok(encoded)
}

fn decode_save_bytes(data: &[u8]) -> Result<SaveFile, IoError> {
    let current: Result<(SaveFile, usize), _> =
        bincode::serde::decode_from_slice(data, bincode::config::standard());
    if let Ok((save, _)) = current {
        return Ok(save);
    }
    let legacy: Result<(SaveFileV1, usize), _> =
        bincode::serde::decode_from_slice(data, bincode::config::standard());
    match legacy {
        Ok((v1, _)) => Ok(SaveFile::from(v1)),
        Err(e) => Err(IoError::Serialization(e.to_string())),
    }
}

/// Save to bytes using bincode (binary) + optional zstd.
pub fn save_to_bytes(
    grid: &Grid,
    settings: &SimulationSettings,
    use_compression: bool,
) -> Result<Vec<u8>, IoError> {
    let save = SaveFile::from_grid(grid, settings, 42, 60, false);
    encode_save(&save, use_compression)
}

pub fn save_simulation_to_bytes(
    sim: &SimulationState,
    use_compression: bool,
) -> Result<Vec<u8>, IoError> {
    let save = SaveFile::from_simulation(sim, false);
    encode_save(&save, use_compression)
}

pub fn load_from_bytes(
    bytes: &[u8],
    was_compressed: bool,
) -> Result<(Grid, SimulationSettings), IoError> {
    load_save_from_bytes(bytes, was_compressed)?.to_grid()
}

pub fn load_save_from_bytes(bytes: &[u8], was_compressed: bool) -> Result<SaveFile, IoError> {
    let data = if was_compressed {
        #[cfg(feature = "compression")]
        {
            zstd::decode_all(bytes).map_err(|e| IoError::Compression(e.to_string()))?
        }
        #[cfg(not(feature = "compression"))]
        {
            return Err(IoError::Compression(
                "zstd feature not enabled but file is compressed".into(),
            ));
        }
    } else {
        bytes.to_vec()
    };
    decode_save_bytes(&data)
}

/// Save to file with automatic extension handling.
pub fn save_to_file<P: AsRef<Path>>(
    path: P,
    grid: &Grid,
    settings: &SimulationSettings,
    compress: bool,
) -> Result<(), IoError> {
    let path = path.as_ref();
    let bytes = save_to_bytes(grid, settings, compress)?;
    if path.extension().and_then(|s| s.to_str()) == Some("json") {
        let save = SaveFile::from_grid(grid, settings, 42, 60, false);
        let json = serde_json::to_string_pretty(&save)
            .map_err(|e| IoError::Serialization(e.to_string()))?;
        std::fs::write(path, json)?;
    } else {
        std::fs::write(path, bytes)?;
    }
    Ok(())
}

pub fn save_simulation_to_file<P: AsRef<Path>>(
    path: P,
    sim: &SimulationState,
    compress: bool,
) -> Result<(), IoError> {
    let path = path.as_ref();
    if path.extension().and_then(|s| s.to_str()) == Some("json") {
        let save = SaveFile::from_simulation(sim, false);
        let json = serde_json::to_string_pretty(&save)
            .map_err(|e| IoError::Serialization(e.to_string()))?;
        std::fs::write(path, json)?;
    } else {
        let bytes = save_simulation_to_bytes(sim, compress)?;
        std::fs::write(path, bytes)?;
    }
    Ok(())
}

pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<(Grid, SimulationSettings), IoError> {
    load_save_from_file(path)?.to_grid()
}

pub fn load_save_from_file<P: AsRef<Path>>(path: P) -> Result<SaveFile, IoError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    if path.extension().and_then(|s| s.to_str()) == Some("json") {
        let s = String::from_utf8(bytes).map_err(|_| IoError::InvalidFormat)?;
        let save: SaveFile =
            serde_json::from_str(&s).map_err(|e| IoError::Serialization(e.to_string()))?;
        Ok(save)
    } else {
        match load_save_from_bytes(&bytes, false) {
            Ok(res) => Ok(res),
            Err(_) => load_save_from_bytes(&bytes, true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aura_lite_core::{element_id, NeutronEnergy, Particle};

    #[test]
    fn test_save_load_roundtrip() {
        let mut grid = Grid::new(10, 10);
        grid.set(5, 5, aura_lite_core::Particle::new(1, 293));
        let settings = SimulationSettings::default();
        let bytes = save_to_bytes(&grid, &settings, false).unwrap();
        let (loaded_grid, _) = load_from_bytes(&bytes, false).unwrap();
        assert_eq!(loaded_grid.get(5, 5).unwrap().element_id, 1);
    }

    #[test]
    fn test_save_simulation_preserves_queue_and_counters() {
        let mut sim = SimulationState::new(16, 16, 7);
        sim.grid
            .set(3, 3, Particle::new(element_id::U235, 400));
        sim.neutron_queue.push_back(aura_lite_core::NeutronEvent {
            x: 4,
            y: 4,
            delay: 2,
            energy: NeutronEnergy::Fast,
        });
        sim.tick = 12;
        sim.fission_count = 3;
        let bytes = save_simulation_to_bytes(&sim, false).unwrap();
        let save = load_save_from_bytes(&bytes, false).unwrap();
        let mut loaded = SimulationState::new(8, 8, 0);
        save.apply_to(&mut loaded).unwrap();
        assert_eq!(loaded.tick, 12);
        assert_eq!(loaded.fission_count, 3);
        assert_eq!(loaded.neutron_queue.len(), 1);
        assert_eq!(loaded.grid.get(3, 3).unwrap().element_id, element_id::U235);
    }
}
