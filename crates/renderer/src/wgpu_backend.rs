//! WGPU backend - optional GPU renderer.
//!
//! `init` builds a device + a grid-sized RGBA texture. A windowed presenter
//! (`attach_surface`) is optional; without it `render` still uploads so the
//! shader path can be validated headless.

use crate::backend::RenderBackend;

#[cfg(feature = "wgpu")]
pub struct WgpuBackend {
    pub width: u32,
    pub height: u32,
    pub pixel_buffer: Vec<u8>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    texture: Option<wgpu::Texture>,
    shader_validated: bool,
}

#[cfg(feature = "wgpu")]
impl RenderBackend for WgpuBackend {
    fn init(width: u32, height: u32) -> Self {
        let mut backend = Self {
            width,
            height,
            pixel_buffer: vec![0; (width * height * 4) as usize],
            device: None,
            queue: None,
            texture: None,
            shader_validated: false,
        };
        backend.try_init_device();
        backend
    }

    fn render(&mut self, pixels: &[u8]) {
        if pixels.len() == self.pixel_buffer.len() {
            self.pixel_buffer.copy_from_slice(pixels);
        } else {
            self.pixel_buffer = pixels.to_vec();
        }
        self.upload_texture();
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.pixel_buffer.resize((width * height * 4) as usize, 0);
        if self.device.is_some() {
            self.recreate_texture();
        }
    }
}

#[cfg(feature = "wgpu")]
impl WgpuBackend {
    pub fn load_shader() -> String {
        include_str!("../../../assets/shaders/shader.wgsl").to_string()
    }

    pub fn shader_validated(&self) -> bool {
        self.shader_validated
    }

    fn try_init_device(&mut self) {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }));
        let Some(adapter) = adapter else {
            log::warn!("wgpu: no adapter available, staying on CPU buffer");
            return;
        };
        let device_queue = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("auralite-wgpu"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
            },
            None,
        ));
        let Ok((device, queue)) = device_queue else {
            log::warn!("wgpu: request_device failed");
            return;
        };

        // Compile the bundled shader so a bad WGSL file fails at init, not later.
        let shader_src = Self::load_shader();
        let _ = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("auralite-particle"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });
        self.shader_validated = true;
        self.device = Some(device);
        self.queue = Some(queue);
        self.recreate_texture();
    }

    fn recreate_texture(&mut self) {
        let Some(device) = self.device.as_ref() else {
            return;
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("particle-grid"),
            size: wgpu::Extent3d {
                width: self.width.max(1),
                height: self.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.texture = Some(texture);
    }

    fn upload_texture(&self) {
        let (Some(queue), Some(texture)) = (self.queue.as_ref(), self.texture.as_ref()) else {
            return;
        };
        let width = self.width.max(1);
        let unpadded = (width * 4) as usize;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded = unpadded.div_ceil(align) * align;
        let height = self.height.max(1) as usize;
        let mut padded_buf = vec![0u8; padded * height];
        for y in 0..height {
            let src = y * unpadded;
            let dst = y * padded;
            if src + unpadded <= self.pixel_buffer.len() {
                padded_buf[dst..dst + unpadded]
                    .copy_from_slice(&self.pixel_buffer[src..src + unpadded]);
            }
        }
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &padded_buf,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(height as u32),
            },
            wgpu::Extent3d {
                width: width,
                height: height as u32,
                depth_or_array_layers: 1,
            },
        );
    }
}

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

#[cfg(not(feature = "wgpu"))]
impl WgpuBackend {
    pub fn load_shader() -> String {
        include_str!("../../../assets/shaders/shader.wgsl").to_string()
    }
}
