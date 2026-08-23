//! CPU rasterizer for egui meshes so the native UI is actually visible in the
//! `pixels` framebuffer (egui-winit only produces tessellated shapes).

use egui::epaint::{ImageData, Primitive};
use egui::{ClippedPrimitive, Color32, TextureId, TexturesDelta};
use std::collections::HashMap;

#[derive(Clone)]
struct Texture {
    width: usize,
    height: usize,
    pixels: Vec<Color32>,
}

/// GPU-less texture atlas driven by `TexturesDelta`.
#[derive(Default)]
pub struct EguiTextures {
    images: HashMap<TextureId, Texture>,
}

impl EguiTextures {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, delta: &TexturesDelta) {
        for (id, image_delta) in &delta.set {
            let (src_w, src_h, src_pixels) = flatten_image(&image_delta.image);
            if let Some(pos) = image_delta.pos {
                if let Some(existing) = self.images.get_mut(id) {
                    blit_into(existing, pos[0], pos[1], src_w, src_h, &src_pixels);
                    continue;
                }
            }
            self.images.insert(
                *id,
                Texture {
                    width: src_w,
                    height: src_h,
                    pixels: src_pixels,
                },
            );
        }
        for id in &delta.free {
            self.images.remove(id);
        }
    }
}

fn flatten_image(image: &ImageData) -> (usize, usize, Vec<Color32>) {
    match image {
        ImageData::Color(color) => {
            let size = color.size;
            (size[0], size[1], color.pixels.clone())
        }
        ImageData::Font(font) => {
            let size = font.size;
            let pixels: Vec<Color32> = font.srgba_pixels(None).collect();
            (size[0], size[1], pixels)
        }
    }
}

fn blit_into(
    dest: &mut Texture,
    dx: usize,
    dy: usize,
    src_w: usize,
    src_h: usize,
    src: &[Color32],
) {
    for sy in 0..src_h {
        let dyi = dy + sy;
        if dyi >= dest.height {
            break;
        }
        for sx in 0..src_w {
            let dxi = dx + sx;
            if dxi >= dest.width {
                break;
            }
            dest.pixels[dyi * dest.width + dxi] = src[sy * src_w + sx];
        }
    }
}

/// Rasterize tessellated egui primitives on top of an RGBA8 framebuffer.
pub fn rasterize(
    frame: &mut [u8],
    frame_w: u32,
    frame_h: u32,
    pixels_per_point: f32,
    textures: &EguiTextures,
    primitives: &[ClippedPrimitive],
) {
    let fw = frame_w as i32;
    let fh = frame_h as i32;
    if fw <= 0 || fh <= 0 {
        return;
    }
    let ppp = if pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };

    for clipped in primitives {
        let clip = clipped.clip_rect;
        let min_x = (clip.min.x * ppp).floor() as i32;
        let min_y = (clip.min.y * ppp).floor() as i32;
        let max_x = (clip.max.x * ppp).ceil() as i32;
        let max_y = (clip.max.y * ppp).ceil() as i32;
        let clip_min_x = min_x.clamp(0, fw);
        let clip_min_y = min_y.clamp(0, fh);
        let clip_max_x = max_x.clamp(0, fw);
        let clip_max_y = max_y.clamp(0, fh);
        if clip_min_x >= clip_max_x || clip_min_y >= clip_max_y {
            continue;
        }

        match &clipped.primitive {
            Primitive::Mesh(mesh) => {
                let tex = textures.images.get(&mesh.texture_id);
                for tri in mesh.indices.chunks_exact(3) {
                    let v0 = mesh.vertices[tri[0] as usize];
                    let v1 = mesh.vertices[tri[1] as usize];
                    let v2 = mesh.vertices[tri[2] as usize];
                    raster_triangle(
                        frame,
                        fw,
                        fh,
                        ppp,
                        clip_min_x,
                        clip_min_y,
                        clip_max_x,
                        clip_max_y,
                        v0,
                        v1,
                        v2,
                        tex,
                    );
                }
            }
            Primitive::Callback(_) => {}
        }
    }
}

fn raster_triangle(
    frame: &mut [u8],
    fw: i32,
    fh: i32,
    ppp: f32,
    clip_min_x: i32,
    clip_min_y: i32,
    clip_max_x: i32,
    clip_max_y: i32,
    v0: egui::epaint::Vertex,
    v1: egui::epaint::Vertex,
    v2: egui::epaint::Vertex,
    tex: Option<&Texture>,
) {
    let p0 = [v0.pos.x * ppp, v0.pos.y * ppp];
    let p1 = [v1.pos.x * ppp, v1.pos.y * ppp];
    let p2 = [v2.pos.x * ppp, v2.pos.y * ppp];

    let min_x = p0[0].min(p1[0]).min(p2[0]).floor() as i32;
    let min_y = p0[1].min(p1[1]).min(p2[1]).floor() as i32;
    let max_x = p0[0].max(p1[0]).max(p2[0]).ceil() as i32;
    let max_y = p0[1].max(p1[1]).max(p2[1]).ceil() as i32;

    let min_x = min_x.max(clip_min_x).max(0);
    let min_y = min_y.max(clip_min_y).max(0);
    let max_x = max_x.min(clip_max_x).min(fw);
    let max_y = max_y.min(clip_max_y).min(fh);
    if min_x >= max_x || min_y >= max_y {
        return;
    }

    let area = edge(p0, p1, p2);
    if area.abs() < 1e-5 {
        return;
    }

    for y in min_y..max_y {
        for x in min_x..max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let p = [px, py];
            let w0 = edge(p1, p2, p) / area;
            let w1 = edge(p2, p0, p) / area;
            let w2 = edge(p0, p1, p) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let uv = [
                w0 * v0.uv.x + w1 * v1.uv.x + w2 * v2.uv.x,
                w0 * v0.uv.y + w1 * v1.uv.y + w2 * v2.uv.y,
            ];
            let vc = lerp_color(v0.color, v1.color, v2.color, w0, w1, w2);
            let sampled = sample_texture(tex, uv);
            let src = modulate(vc, sampled);
            blend_pixel(frame, fw, fh, x, y, src);
        }
    }
}

fn edge(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0])
}

fn lerp_color(c0: Color32, c1: Color32, c2: Color32, w0: f32, w1: f32, w2: f32) -> [u8; 4] {
    [
        (w0 * c0.r() as f32 + w1 * c1.r() as f32 + w2 * c2.r() as f32) as u8,
        (w0 * c0.g() as f32 + w1 * c1.g() as f32 + w2 * c2.g() as f32) as u8,
        (w0 * c0.b() as f32 + w1 * c1.b() as f32 + w2 * c2.b() as f32) as u8,
        (w0 * c0.a() as f32 + w1 * c1.a() as f32 + w2 * c2.a() as f32) as u8,
    ]
}

fn sample_texture(tex: Option<&Texture>, uv: [f32; 2]) -> [u8; 4] {
    let Some(tex) = tex else {
        return [255, 255, 255, 255];
    };
    if tex.width == 0 || tex.height == 0 {
        return [255, 255, 255, 255];
    }
    let u = uv[0].clamp(0.0, 1.0);
    let v = uv[1].clamp(0.0, 1.0);
    let x = ((u * tex.width as f32) as usize).min(tex.width - 1);
    let y = ((v * tex.height as f32) as usize).min(tex.height - 1);
    let c = tex.pixels[y * tex.width + x];
    [c.r(), c.g(), c.b(), c.a()]
}

fn modulate(vertex: [u8; 4], texel: [u8; 4]) -> [u8; 4] {
    [
        ((vertex[0] as u16 * texel[0] as u16) / 255) as u8,
        ((vertex[1] as u16 * texel[1] as u16) / 255) as u8,
        ((vertex[2] as u16 * texel[2] as u16) / 255) as u8,
        ((vertex[3] as u16 * texel[3] as u16) / 255) as u8,
    ]
}

fn blend_pixel(frame: &mut [u8], fw: i32, fh: i32, x: i32, y: i32, src: [u8; 4]) {
    if x < 0 || y < 0 || x >= fw || y >= fh {
        return;
    }
    let idx = ((y * fw + x) * 4) as usize;
    if idx + 3 >= frame.len() {
        return;
    }
    let a = src[3] as u32;
    if a == 0 {
        return;
    }
    if a >= 255 {
        frame[idx] = src[0];
        frame[idx + 1] = src[1];
        frame[idx + 2] = src[2];
        frame[idx + 3] = 255;
        return;
    }
    let ia = 255 - a;
    frame[idx] = ((src[0] as u32 * a + frame[idx] as u32 * ia) / 255) as u8;
    frame[idx + 1] = ((src[1] as u32 * a + frame[idx + 1] as u32 * ia) / 255) as u8;
    frame[idx + 2] = ((src[2] as u32 * a + frame[idx + 2] as u32 * ia) / 255) as u8;
    frame[idx + 3] = 255;
}
