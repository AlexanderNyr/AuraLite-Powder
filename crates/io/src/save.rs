use crate::error::IoError;
use aura_lite_core::simulation::SimulationSettings;
use aura_lite_core::{Grid, ParticleData};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveFile {
    pub version: u32,
    pub timestamp: u64,
    pub grid_width: u32,
    pub grid_height: u32,
    pub tick_rate: u32,
    pub seed: u64,
    pub particles: Vec<ParticleData>, // compact mode: only non-air
    pub settings: SimulationSettings,
    pub full_grid: Option<Vec<aura_lite_core::Particle>>, // optional full mode
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
        }
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
}

/// Save to bytes using bincode (binary) + optional zstd
pub fn save_to_bytes(
    grid: &Grid,
    settings: &SimulationSettings,
    use_compression: bool,
) -> Result<Vec<u8>, IoError> {
    let save = SaveFile::from_grid(grid, settings, 42, 60, false);
    let encoded = bincode::serde::encode_to_vec(&save, bincode::config::standard())
        .map_err(|e| IoError::Serialization(e.to_string()))?;

    if use_compression {
        #[cfg(feature = "compression")]
        {
            let compressed = zstd::encode_all(encoded.as_slice(), 3)
                .map_err(|e| IoError::Compression(e.to_string()))?;
            Ok(compressed)
        }
        #[cfg(not(feature = "compression"))]
        {
            log::warn!("compression feature not enabled, saving uncompressed");
            Ok(encoded)
        }
    } else {
        Ok(encoded)
    }
}

pub fn load_from_bytes(
    bytes: &[u8],
    was_compressed: bool,
) -> Result<(Grid, SimulationSettings), IoError> {
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
    let (save, _): (SaveFile, usize) =
        bincode::serde::decode_from_slice(&data, bincode::config::standard())
            .map_err(|e| IoError::Serialization(e.to_string()))?;
    save.to_grid()
}

/// Save to file with automatic extension handling
pub fn save_to_file<P: AsRef<Path>>(
    path: P,
    grid: &Grid,
    settings: &SimulationSettings,
    compress: bool,
) -> Result<(), IoError> {
    let path = path.as_ref();
    let bytes = save_to_bytes(grid, settings, compress)?;
    // If json extension, save json
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

pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<(Grid, SimulationSettings), IoError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    if path.extension().and_then(|s| s.to_str()) == Some("json") {
        let s = String::from_utf8(bytes).map_err(|_| IoError::InvalidFormat)?;
        let save: SaveFile =
            serde_json::from_str(&s).map_err(|e| IoError::Serialization(e.to_string()))?;
        save.to_grid()
    } else {
        // try both compressed and uncompressed: try decode as bincode, if fails try zstd
        match load_from_bytes(&bytes, false) {
            Ok(res) => Ok(res),
            Err(_) => {
                // try compressed
                load_from_bytes(&bytes, true)
            }
        }
    }
}

// For serde bincode compat, also allow json load
#[cfg(test)]
mod tests {
    use super::*;
    use aura_lite_core::Grid;

    #[test]
    fn test_save_load_roundtrip() {
        let mut grid = Grid::new(10, 10);
        grid.set(5, 5, aura_lite_core::Particle::new(1, 293));
        let settings = SimulationSettings::default();
        let bytes = save_to_bytes(&grid, &settings, false).unwrap();
        let (loaded_grid, _) = load_from_bytes(&bytes, false).unwrap();
        assert_eq!(loaded_grid.get(5, 5).unwrap().element_id, 1);
    }
}
