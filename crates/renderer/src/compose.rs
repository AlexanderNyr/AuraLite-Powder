//! CPU compose: project the particle grid through the camera into an RGBA frame
//! and bake the temperature glow so backends do not need a second pass.

use crate::camera::Camera;
use crate::color_map::color_for_element;
use crate::overlay::{overlay_color, OverlayMode};
use aura_lite_core::SimulationState;
use aura_lite_utils::Vec2;

const EMPTY_RGB: [u8; 3] = [10, 10, 15];
const OUTSIDE_RGB: [u8; 3] = [5, 5, 10];

/// Rasterize `sim` into `frame` (tight RGBA8, `frame_w * frame_h * 4` bytes).
pub fn render_simulation(
    sim: &SimulationState,
    frame: &mut [u8],
    frame_w: u32,
    frame_h: u32,
    camera: &Camera,
) {
    render_simulation_ex(sim, frame, frame_w, frame_h, camera, OverlayMode::None);
}

pub fn render_simulation_ex(
    sim: &SimulationState,
    frame: &mut [u8],
    frame_w: u32,
    frame_h: u32,
    camera: &Camera,
    overlay: OverlayMode,
) {
    let fw = frame_w as usize;
    let fh = frame_h as usize;
    let gw = sim.grid.width as usize;
    let gh = sim.grid.height as usize;
    if fw == 0 || fh == 0 || gw == 0 || gh == 0 {
        return;
    }

    for y in 0..fh {
        for x in 0..fw {
            let idx = (y * fw + x) * 4;
            if idx + 3 >= frame.len() {
                return;
            }
            let world = camera.screen_to_world(Vec2::new(x as f32 + 0.5, y as f32 + 0.5));
            let gx = world.x.floor() as i32;
            let gy = world.y.floor() as i32;
            if gx >= 0 && gy >= 0 && (gx as usize) < gw && (gy as usize) < gh {
                let gidx = gy as usize * gw + gx as usize;
                let p = sim.grid.particles[gidx];
                let mut col = if let Some(ov) = overlay_color(sim, gidx, overlay) {
                    ov
                } else if p.is_empty() {
                    [EMPTY_RGB[0], EMPTY_RGB[1], EMPTY_RGB[2], 255]
                } else {
                    let mut c = color_for_element(p.element_id);
                    c[3] = 255;
                    c
                };
                if overlay == OverlayMode::None && !p.is_empty() && p.temperature > 800 {
                    let factor = ((p.temperature as f32 - 800.0) / 2000.0).clamp(0.0, 1.0);
                    col[0] = col[0].saturating_add((factor * 200.0) as u8);
                    col[1] = col[1].saturating_add((factor * 100.0) as u8);
                }
                frame[idx] = col[0];
                frame[idx + 1] = col[1];
                frame[idx + 2] = col[2];
                frame[idx + 3] = 255;
            } else {
                frame[idx] = OUTSIDE_RGB[0];
                frame[idx + 1] = OUTSIDE_RGB[1];
                frame[idx + 2] = OUTSIDE_RGB[2];
                frame[idx + 3] = 255;
            }
        }
    }
}

/// Grid-sized RGBA buffer with temperature glow, used as a GPU texture upload.
pub fn render_grid_with_glow(sim: &SimulationState) -> Vec<u8> {
    let w = sim.grid.width as usize;
    let h = sim.grid.height as usize;
    let mut buf = vec![0u8; w * h * 4];
    for (i, p) in sim.grid.particles.iter().enumerate() {
        let base = i * 4;
        let mut col = if p.is_empty() {
            [EMPTY_RGB[0], EMPTY_RGB[1], EMPTY_RGB[2], 255]
        } else {
            let mut c = color_for_element(p.element_id);
            c[3] = 255;
            c
        };
        if !p.is_empty() && p.temperature > 800 {
            let factor = ((p.temperature as f32 - 800.0) / 2000.0).clamp(0.0, 1.0);
            col[0] = col[0].saturating_add((factor * 200.0) as u8);
            col[1] = col[1].saturating_add((factor * 100.0) as u8);
        }
        buf[base] = col[0];
        buf[base + 1] = col[1];
        buf[base + 2] = col[2];
        buf[base + 3] = 255;
    }
    buf
}
