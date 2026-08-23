//! Renderer crate - abstract RenderBackend with Softbuffer + Wgpu

pub mod backend;
pub mod camera;
pub mod color_map;
pub mod compose;
pub mod softbuffer_backend;
pub mod wgpu_backend;

pub use backend::RenderBackend;
pub use camera::Camera;
pub use color_map::color_for_element;
pub use compose::{render_grid_with_glow, render_simulation};

#[cfg(feature = "softbuffer")]
pub use softbuffer_backend::SoftbufferBackend;

#[cfg(feature = "wgpu")]
pub use wgpu_backend::WgpuBackend;

/// Create RGBA buffer from grid using element colors
pub fn render_grid_to_buffer(grid: &aura_lite_core::Grid) -> Vec<u8> {
    grid.to_rgba_buffer(color_for_element)
}
