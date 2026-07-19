//! WASM entry - exports start_sim()

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

/// Exported start_sim per spec
#[wasm_bindgen]
pub fn start_sim(canvas_id: String) -> Result<(), JsValue> {
    aura_lite_web::start_sim(&canvas_id)
}

#[wasm_bindgen]
pub fn create_simulation(width: u32, height: u32) -> WasmSimulation {
    WasmSimulation::new(width, height)
}

/// Run a simple benchmark in WASM
#[wasm_bindgen]
pub fn run_tick_test(width: u32, height: u32, ticks: u32) -> u32 {
    let mut sim = WebSimulation::new(width, height);
    // populate some particles
    for y in 0..height / 2 {
        for x in 0..width {
            if (x + y) % 3 == 0 {
                sim.set_particle(x, y, 1); // sand
            }
        }
    }
    for _ in 0..ticks {
        sim.tick();
    }
    sim.sim.grid.count_non_empty() as u32
}
