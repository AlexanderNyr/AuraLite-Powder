//! UI crate - component framework, palette, property panel

pub mod brush;
pub mod camera;
pub mod components;

pub use brush::{BrushSettings, BrushTool};
pub use camera::Camera;
pub use components::app_state::{
    AppState, InfoPanel, PalettePanel, PropertyInspector, SaveLoadPanel, SimulationController,
    ToolPanel,
};

#[cfg(feature = "native-ui")]
pub mod egui_ui;
