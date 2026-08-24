//! Built-in scenes the player can load from the UI.

use crate::element_id::*;
use crate::particle::Particle;
use crate::reactions;
use crate::simulation::SimulationState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    Empty,
    Reactor,
    ControlledReactor,
    Bomb,
    IceMelt,
    Hourglass,
    FusionCell,
    CoolantLoop,
    ForestFire,
}

impl Scenario {
    pub fn all() -> &'static [Scenario] {
        &[
            Scenario::Empty,
            Scenario::Reactor,
            Scenario::ControlledReactor,
            Scenario::Bomb,
            Scenario::IceMelt,
            Scenario::Hourglass,
            Scenario::FusionCell,
            Scenario::CoolantLoop,
            Scenario::ForestFire,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            Scenario::Empty => "Empty",
            Scenario::Reactor => "Bare reactor",
            Scenario::ControlledReactor => "Control-rod reactor",
            Scenario::Bomb => "Critical pit",
            Scenario::IceMelt => "Ice melt",
            Scenario::Hourglass => "Hourglass",
            Scenario::FusionCell => "Fusion cell",
            Scenario::CoolantLoop => "Coolant loop",
            Scenario::ForestFire => "Forest fire",
        }
    }
}

impl SimulationState {
    pub fn load_scenario(&mut self, scene: Scenario) {
        self.grid.clear();
        self.neutron_queue.clear();
        self.fission_count = 0;
        self.fusion_count = 0;
        self.decay_count = 0;
        self.reaction_count = 0;
        self.tick = 0;
        match scene {
            Scenario::Empty => {}
            Scenario::Reactor => self.setup_reactor_demo(),
            Scenario::ControlledReactor => setup_controlled_reactor(self),
            Scenario::Bomb => setup_bomb(self),
            Scenario::IceMelt => setup_ice_melt(self),
            Scenario::Hourglass => setup_hourglass(self),
            Scenario::FusionCell => setup_fusion_cell(self),
            Scenario::CoolantLoop => setup_coolant_loop(self),
            Scenario::ForestFire => setup_forest_fire(self),
        }
        self.refresh_chunks_public();
    }
}

fn put(sim: &mut SimulationState, x: u32, y: u32, id: u16, t: u16) {
    if x < sim.grid.width && y < sim.grid.height {
        sim.grid.set(x, y, Particle::new(id, t));
    }
}

fn floor(sim: &mut SimulationState, id: u16) {
    let w = sim.grid.width;
    let h = sim.grid.height;
    for x in 0..w {
        put(sim, x, h - 1, id, reactions::AMBIENT_TEMP);
        put(sim, x, h - 2, id, reactions::AMBIENT_TEMP);
    }
}

fn setup_controlled_reactor(sim: &mut SimulationState) {
    sim.setup_reactor_demo();
    let w = sim.grid.width;
    let h = sim.grid.height;
    // Replace boron sprinkles with two solid control rods that can be racked.
    for y in h.saturating_sub(18)..h.saturating_sub(3) {
        put(sim, w / 2 - 14, y, CONTROL_ROD, reactions::AMBIENT_TEMP);
        put(sim, w / 2 + 14, y, CONTROL_ROD, reactions::AMBIENT_TEMP);
        put(sim, w / 2 - 13, y, CONTROL_ROD, reactions::AMBIENT_TEMP);
        put(sim, w / 2 + 13, y, CONTROL_ROD, reactions::AMBIENT_TEMP);
    }
    // Coolant jacket
    for y in h.saturating_sub(14)..h.saturating_sub(4) {
        put(sim, w / 2 - 10, y, WATER, 310);
        put(sim, w / 2 + 10, y, WATER, 310);
    }
}

fn setup_bomb(sim: &mut SimulationState) {
    floor(sim, CONCRETE);
    let cx = sim.grid.width / 2;
    let cy = sim.grid.height / 2;
    for dy in -6..=6 {
        for dx in -6..=6 {
            if dx * dx + dy * dy <= 36 {
                put(sim, (cx as i32 + dx) as u32, (cy as i32 + dy) as u32, PU239, 400);
            }
        }
    }
    put(sim, cx, cy, NEUTRON_THERMAL, 400);
    put(sim, cx + 1, cy, NEUTRON_FAST, 800);
}

fn setup_ice_melt(sim: &mut SimulationState) {
    floor(sim, STONE);
    let w = sim.grid.width;
    let h = sim.grid.height;
    for y in h / 2..h - 2 {
        for x in w / 4..3 * w / 4 {
            put(sim, x, y, ICE, 250);
        }
    }
    for x in w / 2 - 8..w / 2 + 8 {
        put(sim, x, h / 2 - 1, HEATER, 900);
    }
}

fn setup_hourglass(sim: &mut SimulationState) {
    let w = sim.grid.width;
    let h = sim.grid.height;
    for y in 0..h {
        put(sim, 0, y, STONE, 293);
        put(sim, w - 1, y, STONE, 293);
    }
    for x in 0..w {
        put(sim, x, h - 1, STONE, 293);
        put(sim, x, 0, STONE, 293);
    }
    let mid = h / 2;
    for x in 0..w {
        if x < w / 2 - 2 || x > w / 2 + 2 {
            put(sim, x, mid, STONE, 293);
        }
    }
    for y in 2..mid {
        for x in 4..w - 4 {
            if (x + y) % 2 == 0 {
                put(sim, x, y, SAND, 293);
            }
        }
    }
}

fn setup_fusion_cell(sim: &mut SimulationState) {
    floor(sim, LEAD);
    let cx = sim.grid.width / 2;
    let cy = sim.grid.height / 2;
    for y in cy - 8..cy + 8 {
        put(sim, cx - 10, y, STEEL, 293);
        put(sim, cx + 10, y, STEEL, 293);
    }
    for x in cx - 10..=cx + 10 {
        put(sim, x, cy - 8, STEEL, 293);
        put(sim, x, cy + 8, STEEL, 293);
    }
    for y in cy - 4..cy + 4 {
        for x in cx - 6..cx {
            put(sim, x, y, DEUTERIUM, 1800);
        }
        for x in cx..cx + 6 {
            put(sim, x, y, TRITIUM, 1800);
        }
    }
    put(sim, cx, cy - 7, HEATER, 2000);
}

fn setup_coolant_loop(sim: &mut SimulationState) {
    floor(sim, CONCRETE);
    let x0 = sim.grid.width / 2 - 20;
    let x1 = sim.grid.width / 2 + 20;
    let y0 = sim.grid.height / 2 - 12;
    let y1 = sim.grid.height / 2 + 12;
    for x in x0..=x1 {
        put(sim, x, y0, PIPE, 293);
        put(sim, x, y1, PIPE, 293);
        if x > x0 && x < x1 {
            put(sim, x, y0 + 1, WATER, 300);
            put(sim, x, y1 - 1, WATER, 300);
        }
    }
    for y in y0..=y1 {
        put(sim, x0, y, PIPE, 293);
        put(sim, x1, y, PIPE, 293);
        if y > y0 && y < y1 {
            put(sim, x0 + 1, y, WATER, 300);
            put(sim, x1 - 1, y, WATER, 300);
        }
    }
    put(sim, x0 + 1, y1 - 1, PUMP, 293);
    put(sim, x1 - 1, y0 + 1, HEATER, 1200);
    for y in y0 + 3..y1 - 3 {
        for x in sim.grid.width / 2 - 4..sim.grid.width / 2 + 4 {
            put(sim, x, y, U235, 400);
        }
    }
    put(sim, sim.grid.width / 2, y0 + 4, NEUTRON_THERMAL, 350);
}

fn setup_forest_fire(sim: &mut SimulationState) {
    floor(sim, STONE);
    let w = sim.grid.width;
    let h = sim.grid.height;
    for y in h / 3..h - 2 {
        for x in 4..w - 4 {
            if (x * 17 + y * 13) % 5 == 0 {
                put(sim, x, y, WOOD, 293);
            } else if (x + y) % 11 == 0 {
                put(sim, x, y, COAL, 293);
            }
        }
    }
    put(sim, w / 2, h / 3, FIRE, 900);
    put(sim, w / 2 + 1, h / 3, FIRE, 900);
}
