//! IO crate - save/load, serialization, compression

pub mod error;
pub mod save;

pub use aura_lite_core::simulation::SimulationSettings;
pub use error::IoError;
pub use save::{
    load_from_bytes, load_from_file, load_save_from_bytes, load_save_from_file,
    save_simulation_to_bytes, save_simulation_to_file, save_to_bytes, save_to_file, SaveFile,
};

/// Re-export for convenience
pub use aura_lite_core::{Grid, Particle};
