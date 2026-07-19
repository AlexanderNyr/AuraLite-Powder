//! Abstract RenderBackend trait

/// Abstract render backend as per spec
pub trait RenderBackend: Send {
    fn init(width: u32, height: u32) -> Self
    where
        Self: Sized;
    fn render(&mut self, pixels: &[u8]);
    fn resize(&mut self, width: u32, height: u32);
}

/// Extra trait for backends that need window handle
pub trait WindowRenderBackend: RenderBackend {
    fn present(&mut self);
}
