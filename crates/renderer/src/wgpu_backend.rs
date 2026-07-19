//! WGPU backend - optional GPU renderer

use crate::backend::RenderBackend;

#[cfg(feature = "wgpu")]
pub struct WgpuBackend {
    pub width: u32,
    pub height: u32,
    pub pixel_buffer: Vec<u8>,
    // Actual wgpu fields would be here, but for MVP we keep minimal
    // device: Option<Device>, queue, surface etc.
}

#[cfg(feature = "wgpu")]
impl RenderBackend for WgpuBackend {
    fn init(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixel_buffer: vec![0; (width * height * 4) as usize],
        }
    }

    fn render(&mut self, pixels: &[u8]) {
        if pixels.len() == self.pixel_buffer.len() {
            self.pixel_buffer.copy_from_slice(pixels);
        } else {
            self.pixel_buffer = pixels.to_vec();
        }
        // In real implementation, upload to GPU storage texture and render full-screen triangle
        // using WGSL shader from assets/shaders/
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.pixel_buffer.resize((width * height * 4) as usize, 0);
    }
}

#[cfg(feature = "wgpu")]
impl WgpuBackend {
    pub fn load_shader() -> String {
        // In real implementation, include_str! from assets/shaders/shader.wgsl
        // Fallback to minimal shader
        include_str!("../../../assets/shaders/shader.wgsl").to_string()
    }
}

// Dummy when wgpu feature disabled but something tries to use
#[cfg(not(feature = "wgpu"))]
pub struct WgpuBackend {
    width: u32,
    height: u32,
}

#[cfg(not(feature = "wgpu"))]
impl RenderBackend for WgpuBackend {
    fn init(width: u32, height: u32) -> Self {
        Self { width, height }
    }
    fn render(&mut self, _pixels: &[u8]) {}
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}
