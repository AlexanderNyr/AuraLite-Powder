//! Softbuffer backend - primary rendering

use crate::backend::RenderBackend;

#[cfg(feature = "softbuffer")]
pub struct SoftbufferBackend {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u8>, // RGBA buffer
}

#[cfg(feature = "softbuffer")]
impl RenderBackend for SoftbufferBackend {
    fn init(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            buffer: vec![0; (width * height * 4) as usize],
        }
    }

    fn render(&mut self, pixels: &[u8]) {
        // pixels expected as RGBA byte slice length width*height*4
        if pixels.len() == self.buffer.len() {
            self.buffer.copy_from_slice(pixels);
        } else {
            // If size mismatch, resize
            self.buffer = pixels.to_vec();
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.buffer.resize((width * height * 4) as usize, 0);
    }
}

#[cfg(feature = "softbuffer")]
impl SoftbufferBackend {
    pub fn new(width: u32, height: u32) -> Self {
        Self::init(width, height)
    }

    pub fn frame_buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn frame_buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    /// Helper for pixels crate integration: writes RGBA into pixels frame (which is RGBA as well)
    pub fn blit_to_pixels_frame(&self, pixels_frame: &mut [u8]) {
        // Both are RGBA, but pixels crate might expect RGBA, we can copy
        let len = self.buffer.len().min(pixels_frame.len());
        pixels_frame[..len].copy_from_slice(&self.buffer[..len]);
    }
}

// Dummy impl when softbuffer feature not enabled but trait needed
#[cfg(not(feature = "softbuffer"))]
pub struct SoftbufferBackend {
    width: u32,
    height: u32,
}

#[cfg(not(feature = "softbuffer"))]
impl RenderBackend for SoftbufferBackend {
    fn init(width: u32, height: u32) -> Self {
        Self { width, height }
    }
    fn render(&mut self, _pixels: &[u8]) {}
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}
