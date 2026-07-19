//! Web crate - thin shim that binds core to a <canvas> element

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
        if x < self.sim.grid.width && y < self.sim.grid.height {
            self.sim
                .grid
                .set(x, y, aura_lite_core::Particle::new(element_id, 293));
        }
    }

    pub fn get_rgba_buffer(&self) -> Vec<u8> {
        // use element colors
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
        // web-sys ImageData expects Uint8ClampedArray
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

    pub fn width(&self) -> u32 {
        self.inner.sim.grid.width
    }

    pub fn height(&self) -> u32 {
        self.inner.sim.grid.height
    }
}

#[cfg(feature = "web")]
pub fn start_sim(canvas_id: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str("canvas not found"))?
        .dyn_into::<HtmlCanvasElement>()?;

    let mut sim = WebSimulation::new(256, 256);
    // simple demo: place some uranium and neutrons
    sim.set_particle(100, 100, aura_lite_core::element_id::U235);
    sim.set_particle(101, 100, aura_lite_core::element_id::U235);
    sim.set_particle(102, 100, aura_lite_core::element_id::NEUTRON_THERMAL);

    // For simplicity, we do one render, real loop would use requestAnimationFrame
    sim.render_to_canvas(&canvas)?;
    web_sys::console::log_1(&JsValue::from_str("AuraLite WASM simulation started"));

    Ok(())
}

#[cfg(not(feature = "web"))]
pub fn start_sim(_canvas_id: &str) -> Result<(), String> {
    Err("web feature not enabled".into())
}
