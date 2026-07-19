//! IO crate - save/load, serialization, compression

pub mod save;
pub mod error;

pub use save::{SaveFile, save_to_bytes, load_from_bytes, save_to_file, load_from_file};
pub use aura_lite_core::simulation::SimulationSettings;
pub use error::IoError;

/// Re-export for convenience
pub use aura_lite_core::{Grid, Particle};

