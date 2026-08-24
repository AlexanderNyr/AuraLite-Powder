//! Web crate - thin shim that binds core to a <canvas> element.

#[cfg(feature = "web")]
use wasm_bindgen::prelude::*;

use aura_lite_core::SimulationState;

#[cfg(feature = "web")]
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

pub struct WebSimulation {
    pub sim: SimulationState,
    pub canvas_width: u32,
    pub canvas_height: u32,
}

impl WebSimulation {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            sim: SimulationState::new(width, height, 42),
            canvas_width: width,
            canvas_height: height,
        }
    }

    pub fn tick(&mut self) {
        self.sim.tick();
    }

    pub fn set_particle(&mut self, x: u32, y: u32, element_id: u16) {
        self.set_particle_temp(x, y, element_id, 293);
    }

    pub fn set_particle_temp(&mut self, x: u32, y: u32, element_id: u16, temperature: u16) {
        if x < self.sim.grid.width && y < self.sim.grid.height {
            self.sim
                .grid
                .set(x, y, aura_lite_core::Particle::new(element_id, temperature));
        }
    }

    pub fn paint(&mut self, cx: i32, cy: i32, element_id: u16, radius: u32, temperature: u16) {
        let r = radius as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                let x = cx + dx;
                let y = cy + dy;
                if x >= 0 && y >= 0 {
                    self.set_particle_temp(x as u32, y as u32, element_id, temperature);
                }
            }
        }
    }

    pub fn get_rgba_buffer(&self) -> Vec<u8> {
        self.sim
            .grid
            .to_rgba_buffer(|id| aura_lite_elements::registry::color_for_id(id))
    }

    #[cfg(feature = "web")]
    pub fn render_to_canvas(&self, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
        let ctx = canvas
            .get_context("2d")?
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()?;

        let buffer = self.get_rgba_buffer();
        let clamped = wasm_bindgen::Clamped(buffer.as_slice());
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            clamped,
            self.sim.grid.width,
            self.sim.grid.height,
        )?;
        ctx.put_image_data(&image_data, 0.0, 0.0)?;
        Ok(())
    }
}

#[cfg(feature = "web")]
#[wasm_bindgen]
pub struct WasmSimulation {
    inner: WebSimulation,
}

#[cfg(feature = "web")]
#[wasm_bindgen]
impl WasmSimulation {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            inner: WebSimulation::new(width, height),
        }
    }

    pub fn tick(&mut self) {
        self.inner.tick();
    }

    pub fn set_particle(&mut self, x: u32, y: u32, element_id: u16) {
        self.inner.set_particle(x, y, element_id);
    }

    pub fn paint(&mut self, x: i32, y: i32, element_id: u16, radius: u32, temperature: u16) {
        self.inner.paint(x, y, element_id, radius, temperature);
    }

    pub fn clear(&mut self) {
        self.inner.sim.grid.clear();
    }

    pub fn setup_demo(&mut self) {
        self.inner.sim.setup_reactor_demo();
    }

    pub fn width(&self) -> u32 {
        self.inner.sim.grid.width
    }

    pub fn height(&self) -> u32 {
        self.inner.sim.grid.height
    }

    pub fn particle_count(&self) -> u32 {
        self.inner.sim.grid.count_non_empty() as u32
    }

    pub fn fission_count(&self) -> u32 {
        self.inner.sim.fission_count.min(u32::MAX as u64) as u32
    }

    pub fn fusion_count(&self) -> u32 {
        self.inner.sim.fusion_count.min(u32::MAX as u64) as u32
    }

    pub fn k_effective(&self) -> f32 {
        self.inner.sim.k_effective
    }

    pub fn load_scene(&mut self, name: &str) {
        let scene = match name {
            "reactor" => aura_lite_core::Scenario::Reactor,
            "rods" => aura_lite_core::Scenario::ControlledReactor,
            "bomb" => aura_lite_core::Scenario::Bomb,
            "ice" => aura_lite_core::Scenario::IceMelt,
            "hourglass" => aura_lite_core::Scenario::Hourglass,
            "fusion" => aura_lite_core::Scenario::FusionCell,
            "loop" => aura_lite_core::Scenario::CoolantLoop,
            "fire" => aura_lite_core::Scenario::ForestFire,
            _ => aura_lite_core::Scenario::Empty,
        };
        self.inner.sim.load_scenario(scene);
    }

    pub fn save_bytes(&self) -> Vec<u8> {
        aura_lite_io::save_simulation_to_bytes(&self.inner.sim, false).unwrap_or_default()
    }

    pub fn load_bytes(&mut self, data: &[u8]) {
        if let Ok(save) = aura_lite_io::load_save_from_bytes(data, false) {
            let _ = save.apply_to(&mut self.inner.sim);
        }
    }

    #[cfg(feature = "web")]
    pub fn render(&self, canvas_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("no document"))?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("canvas not found"))?
            .dyn_into::<HtmlCanvasElement>()?;
        canvas.set_width(self.inner.sim.grid.width);
        canvas.set_height(self.inner.sim.grid.height);
        self.inner.render_to_canvas(&canvas)
    }
}

#[cfg(feature = "web")]
pub fn start_sim(canvas_id: &str) -> Result<(), JsValue> {
    let mut sim = WebSimulation::new(256, 256);
    sim.sim.setup_reactor_demo();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str("canvas not found"))?
        .dyn_into::<HtmlCanvasElement>()?;
    canvas.set_width(sim.sim.grid.width);
    canvas.set_height(sim.sim.grid.height);
    sim.render_to_canvas(&canvas)?;
    web_sys::console::log_1(&JsValue::from_str(
        "AuraLite WASM simulation started (one frame). Use create_simulation() for a live loop.",
    ));

    Ok(())
}

#[cfg(not(feature = "web"))]
pub fn start_sim(_canvas_id: &str) -> Result<(), String> {
    Err("web feature not enabled".into())
}
