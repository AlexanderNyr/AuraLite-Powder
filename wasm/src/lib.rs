//! WASM entry - exports start_sim() and a live simulation handle.

use aura_lite_web::{WasmSimulation, WebSimulation};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(start)]
pub fn main_js() {
    log("AuraLite WASM module loaded");
}

/// One-shot helper: place the reactor demo and draw a single frame.
#[wasm_bindgen]
pub fn start_sim(canvas_id: String) -> Result<(), JsValue> {
    aura_lite_web::start_sim(&canvas_id)
}

#[wasm_bindgen]
pub fn create_simulation(width: u32, height: u32) -> WasmSimulation {
    let mut sim = WasmSimulation::new(width, height);
    sim.setup_demo();
    sim
}

/// Run a simple benchmark in WASM.
#[wasm_bindgen]
pub fn run_tick_test(width: u32, height: u32, ticks: u32) -> u32 {
    let mut sim = WebSimulation::new(width, height);
    for y in 0..height / 2 {
        for x in 0..width {
            if (x + y) % 3 == 0 {
                sim.set_particle(x, y, 1);
            }
        }
    }
    for _ in 0..ticks {
        sim.tick();
    }
    sim.sim.grid.count_non_empty() as u32
}
