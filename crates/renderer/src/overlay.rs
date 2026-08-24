//! Diagnostic overlays painted on top of the particle colours.

use aura_lite_core::element_id::{density_for_id, is_radiation};
use aura_lite_core::SimulationState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OverlayMode {
    #[default]
    None,
    Heat,
    Radiation,
    Density,
    Pressure,
}

impl OverlayMode {
    pub fn next(self) -> Self {
        match self {
            OverlayMode::None => OverlayMode::Heat,
            OverlayMode::Heat => OverlayMode::Radiation,
            OverlayMode::Density => OverlayMode::Pressure,
            OverlayMode::Radiation => OverlayMode::Density,
            OverlayMode::Pressure => OverlayMode::None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OverlayMode::None => "None",
            OverlayMode::Heat => "Heat",
            OverlayMode::Radiation => "Radiation",
            OverlayMode::Density => "Density",
            OverlayMode::Pressure => "Pressure",
        }
    }
}

pub fn overlay_color(sim: &SimulationState, idx: usize, mode: OverlayMode) -> Option<[u8; 4]> {
    let p = sim.grid.particles.get(idx)?;
    match mode {
        OverlayMode::None => None,
        OverlayMode::Heat => {
            let t = ((p.temperature as f32 - 273.0) / 2200.0).clamp(0.0, 1.0);
            Some(heat_ramp(t))
        }
        OverlayMode::Radiation => {
            if is_radiation(p.element_id) {
                Some([255, 255, 80, 255])
            } else if p.element_id == 0 {
                Some([8, 8, 12, 255])
            } else {
                let glow = if p.temperature > 600 { 40 } else { 0 };
                Some([20 + glow, 20, 28, 255])
            }
        }
        OverlayMode::Density => {
            let d = (density_for_id(p.element_id) / 20.0).clamp(0.0, 1.0);
            let v = (d * 255.0) as u8;
            Some([v / 3, v / 2, v, 255])
        }
        OverlayMode::Pressure => {
            let pr = sim.pressure.p.get(idx).copied().unwrap_or(0);
            let t = (pr as f32 / 120.0).clamp(0.0, 1.0);
            Some([(t * 255.0) as u8, 20, (80.0 + t * 80.0) as u8, 255])
        }
    }
}

fn heat_ramp(t: f32) -> [u8; 4] {
    if t < 0.33 {
        let f = t / 0.33;
        [(f * 255.0) as u8, 0, 0, 255]
    } else if t < 0.66 {
        let f = (t - 0.33) / 0.33;
        [255, (f * 200.0) as u8, 0, 255]
    } else {
        let f = (t - 0.66) / 0.34;
        [255, 220, (f * 255.0) as u8, 255]
    }
}
