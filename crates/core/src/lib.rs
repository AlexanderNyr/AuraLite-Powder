//! Core simulation kernel for AuraLite Powder
//! Owns Grid, Particle, SimulationState. Zero rendering / zero UI knowledge.

pub mod chunk;
pub mod devices;
pub mod element_id;
pub mod grid;
pub mod hydro;
pub mod missions;
pub mod particle;
pub mod physics;
pub mod reactions;
pub mod scenarios;
pub mod simulation;

pub use chunk::{ChunkMeta, ChunkPool, CHUNK_SIZE};
pub use element_id::*;
pub use grid::{Grid, GridSnapshot};
pub use missions::{Mission, MissionId, MissionSave, MissionStatus};
pub use particle::{Particle, ParticleData};
pub use reactions::NeutronEnergy;
pub use scenarios::Scenario;
pub use simulation::{NeutronEvent, SimulationSettings, SimulationState};

/// Re-export utils
pub use aura_lite_utils as utils;

/// Version constant for save files
pub const CORE_VERSION: u32 = 2;
