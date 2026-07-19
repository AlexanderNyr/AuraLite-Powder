//! Renderer crate - abstract RenderBackend with Softbuffer + Wgpu

pub mod backend;
pub mod softbuffer_backend;
pub mod wgpu_backend;
pub mod color_map;
pub mod camera;

pub use backend::RenderBackend;
pub use color_map::color_for_element;
pub use camera::Camera;

#[cfg(feature = "softbuffer")]
pub use softbuffer_backend::SoftbufferBackend;

#[cfg(feature = "wgpu")]
pub use wgpu_backend::WgpuBackend;

/// Create RGBA buffer from grid using element colors
pub fn render_grid_to_buffer(grid: &aura_lite_core::Grid) -> Vec<u8> {
    grid.to_rgba_buffer(color_for_element)
}
