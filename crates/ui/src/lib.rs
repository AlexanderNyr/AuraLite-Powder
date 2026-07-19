//! UI crate - component framework, palette, property panel

pub mod camera;
pub mod brush;
pub mod components;

pub use camera::Camera;
pub use brush::{BrushTool, BrushSettings};
pub use components::app_state::{AppState, SimulationController, ToolPanel, PalettePanel, PropertyInspector, SaveLoadPanel, InfoPanel};

#[cfg(feature = "native-ui")]
pub mod egui_ui;
