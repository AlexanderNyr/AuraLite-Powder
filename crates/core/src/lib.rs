//! Core simulation kernel for AuraLite Powder
//! Owns Grid, Particle, SimulationState. Zero rendering / zero UI knowledge.

pub mod element_id;
pub mod particle;
pub mod grid;
pub mod simulation;
pub mod chunk;

pub use element_id::*;
pub use particle::{Particle, ParticleData};
pub use grid::{Grid, GridSnapshot};
pub use simulation::{SimulationState, SimulationSettings, NeutronEvent, NeutronEnergy};
pub use chunk::{ChunkPool, ChunkMeta, CHUNK_SIZE};

/// Re-export utils
pub use aura_lite_utils as utils;

/// Version constant for save files
pub const CORE_VERSION: u32 = 1;
