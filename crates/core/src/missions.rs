//! Short playable challenges with win / fail conditions.

use crate::element_id::*;
use crate::particle::Particle;
use crate::reactions;
use crate::scenarios::Scenario;
use crate::simulation::SimulationState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionId {
    HoldCritical,
    PoisonRestart,
    ForestFire,
    CoolantLoop,
    WireShot,
    FilterRescue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionStatus {
    Running,
    Won,
    Failed,
}

#[derive(Clone, Debug)]
pub struct Mission {
    pub id: MissionId,
    pub status: MissionStatus,
    pub started_tick: u64,
    pub hold_ticks: u64,
    pub time_limit: u64,
    pub message: String,
    start_pipes: u32,
    start_wood: u32,
    start_tnt: u32,
}

impl MissionId {
    pub fn all() -> &'static [MissionId] {
        &[
            MissionId::HoldCritical,
            MissionId::PoisonRestart,
            MissionId::ForestFire,
            MissionId::CoolantLoop,
            MissionId::WireShot,
            MissionId::FilterRescue,
        ]
    }

    pub fn title(self) -> &'static str {
        match self {
            MissionId::HoldCritical => "Hold critical",
            MissionId::PoisonRestart => "Iodine pit",
            MissionId::ForestFire => "Forest fire",
            MissionId::CoolantLoop => "Keep the loop",
            MissionId::WireShot => "Wire shot",
            MissionId::FilterRescue => "Filter rescue",
        }
    }

    pub fn brief(self) -> &'static str {
        match self {
            MissionId::HoldCritical => "Keep k-eff between 0.85 and 1.25 for 20 seconds. Use [ ] on the rods.",
            MissionId::PoisonRestart => "Iodine is decaying into xenon. Raise the rods and get k-eff back above 0.90.",
            MissionId::ForestFire => "Put the fire out (paint water). Leave at least half the wood standing.",
            MissionId::CoolantLoop => "Run the coolant loop 15 s without bursting more than two pipes. Steam is a good sign.",
            MissionId::WireShot => "Paint a Spark on the free end of the wire to detonate the TNT.",
            MissionId::FilterRescue => "Get the water below the filter. The sand must stay above it.",
        }
    }
}

impl Mission {
    pub fn start(sim: &mut SimulationState, id: MissionId) -> Self {
        match id {
            MissionId::HoldCritical => sim.load_scenario(Scenario::ControlledReactor),
            MissionId::PoisonRestart => setup_poison(sim),
            MissionId::ForestFire => sim.load_scenario(Scenario::ForestFire),
            MissionId::CoolantLoop => sim.load_scenario(Scenario::CoolantLoop),
            MissionId::WireShot => setup_wire_shot(sim),
            MissionId::FilterRescue => setup_filter_rescue(sim),
        }
        let time_limit = match id {
            MissionId::HoldCritical => 45 * 60,
            MissionId::PoisonRestart => 30 * 60,
            MissionId::ForestFire => 30 * 60,
            MissionId::CoolantLoop => 20 * 60,
            MissionId::WireShot => 20 * 60,
            MissionId::FilterRescue => 20 * 60,
        };
        Self {
            id,
            status: MissionStatus::Running,
            started_tick: sim.tick,
            hold_ticks: 0,
            time_limit,
            message: id.brief().to_string(),
            start_pipes: count(sim, PIPE),
            start_wood: count(sim, WOOD) + count(sim, COAL),
            start_tnt: count(sim, TNT),
        }
    }

    pub fn elapsed(&self, sim: &SimulationState) -> u64 {
        sim.tick.saturating_sub(self.started_tick)
    }

    pub fn tick(&mut self, sim: &SimulationState) {
        if self.status != MissionStatus::Running {
            return;
        }
        let elapsed = self.elapsed(sim);
        if elapsed > self.time_limit {
            self.fail("Time is up.");
            return;
        }
        if count(sim, MOLTEN_FUEL) > 0 {
            self.fail("Meltdown.");
            return;
        }
        match self.id {
            MissionId::HoldCritical => {
                let k = sim.k_effective;
                if k > 2.4 {
                    self.fail("Went prompt-critical.");
                    return;
                }
                if (0.85..=1.25).contains(&k) {
                    self.hold_ticks += 1;
                } else {
                    self.hold_ticks = 0;
                }
                self.message = format!(
                    "Hold k 0.85–1.25 for 20 s  (now {:.2}, held {:.1} s)",
                    k,
                    self.hold_ticks as f32 / 60.0
                );
                if self.hold_ticks >= 20 * 60 {
                    self.win("Pile held in the band.");
                }
            }
            MissionId::PoisonRestart => {
                self.message = format!(
                    "Get k-eff ≥ 0.90  (now {:.2}, I={} Xe={})",
                    sim.k_effective, sim.iodine_count, sim.xenon_count
                );
                if sim.k_effective >= 0.90 {
                    self.hold_ticks += 1;
                } else {
                    self.hold_ticks = 0;
                }
                if self.hold_ticks >= 3 * 60 {
                    self.win("Restarted through the iodine pit.");
                }
            }
            MissionId::ForestFire => {
                let fire = count(sim, FIRE);
                let wood = count(sim, WOOD) + count(sim, COAL);
                self.message = format!("Fire cells: {fire}   wood left: {wood}/{}", self.start_wood);
                if self.start_wood > 0 && wood * 2 < self.start_wood {
                    self.fail("Too much timber burned.");
                    return;
                }
                if fire == 0 && elapsed > 30 {
                    self.win("Fire is out.");
                }
            }
            MissionId::CoolantLoop => {
                let pipes = count(sim, PIPE);
                let steam = count(sim, STEAM);
                let lost = self.start_pipes.saturating_sub(pipes);
                self.message = format!("Pipes lost: {lost}   steam: {steam}");
                if lost > 2 {
                    self.fail("Loop burst.");
                    return;
                }
                if elapsed >= 15 * 60 && lost <= 2 {
                    self.win("Loop held.");
                }
            }
            MissionId::WireShot => {
                let tnt = count(sim, TNT);
                self.message = format!("TNT left: {tnt}  (spark the free wire end)");
                if self.start_tnt > 0 && tnt < self.start_tnt {
                    self.win("Charge fired.");
                }
            }
            MissionId::FilterRescue => {
                let (water_below, sand_below, sand_above) = filter_score(sim);
                self.message = format!("Water below: {water_below}   sand still above: {sand_above}");
                if sand_below > 2 {
                    self.fail("Sand went through the filter.");
                    return;
                }
                if water_below >= 8 && sand_above >= 6 {
                    self.win("Water drained, sand held.");
                }
            }
        }
    }

    fn win(&mut self, msg: &str) {
        self.status = MissionStatus::Won;
        self.message = format!("WIN — {msg}");
    }

    fn fail(&mut self, msg: &str) {
        self.status = MissionStatus::Failed;
        self.message = format!("FAIL — {msg}");
    }
}

fn count(sim: &SimulationState, id: u16) -> u32 {
    sim.grid
        .particles
        .iter()
        .filter(|p| p.element_id == id)
        .count() as u32
}

fn filter_score(sim: &SimulationState) -> (u32, u32, u32) {
    let mut fy = None;
    for y in 0..sim.grid.height {
        for x in 0..sim.grid.width {
            if sim.grid.get(x, y).unwrap().element_id == FILTER {
                fy = Some(y);
                break;
            }
        }
        if fy.is_some() {
            break;
        }
    }
    let fy = fy.unwrap_or(sim.grid.height / 2);
    let mut water_below = 0;
    let mut sand_below = 0;
    let mut sand_above = 0;
    for y in 0..sim.grid.height {
        for x in 0..sim.grid.width {
            let id = sim.grid.get(x, y).unwrap().element_id;
            if y > fy && id == WATER {
                water_below += 1;
            }
            if y > fy && id == SAND {
                sand_below += 1;
            }
            if y < fy && id == SAND {
                sand_above += 1;
            }
        }
    }
    (water_below, sand_below, sand_above)
}

fn setup_poison(sim: &mut SimulationState) {
    sim.load_scenario(Scenario::ControlledReactor);
    let w = sim.grid.width;
    let h = sim.grid.height;
    for y in h.saturating_sub(12)..h.saturating_sub(5) {
        for x in w / 2 - 6..w / 2 + 6 {
            if sim.grid.get(x, y).map(|p| p.is_empty()).unwrap_or(false) {
                sim.grid.set(x, y, Particle::new(IODINE, 350));
            }
        }
    }
    // Rods start inserted so the player has to raise them.
    sim.shift_control_rods(4);
}

fn setup_wire_shot(sim: &mut SimulationState) {
    sim.grid.clear();
    sim.neutron_queue.clear();
    sim.tick = 0;
    let w = sim.grid.width;
    let h = sim.grid.height;
    let y = h / 2;
    for x in 0..w {
        sim.grid.set(x, h - 1, Particle::new(STONE, 293));
    }
    for x in w / 4..w / 2 + 8 {
        sim.grid.set(x, y, Particle::new(WIRE, 293));
    }
    for dy in -2..=2_i32 {
        for dx in -2..=2_i32 {
            let tx = w as i32 / 2 + 10 + dx;
            let ty = y as i32 + dy;
            if tx >= 0 && ty >= 0 {
                sim.grid
                    .set(tx as u32, ty as u32, Particle::new(TNT, 293));
            }
        }
    }
    sim.refresh_chunks_public();
}

fn setup_filter_rescue(sim: &mut SimulationState) {
    sim.grid.clear();
    sim.neutron_queue.clear();
    sim.tick = 0;
    let w = sim.grid.width;
    let h = sim.grid.height;
    for x in 0..w {
        sim.grid.set(x, h - 1, Particle::new(STONE, 293));
        sim.grid.set(x, h / 2, Particle::new(FILTER, 293));
    }
    for y in 2..h / 2 {
        for x in 4..w - 4 {
            if (x + y) % 2 == 0 {
                sim.grid.set(x, y, Particle::new(WATER, 293));
            } else {
                sim.grid.set(x, y, Particle::new(SAND, 293));
            }
        }
    }
    sim.refresh_chunks_public();
}

/// One-line remaining-time helper for the HUD.
pub fn format_clock(left_ticks: u64) -> String {
    let s = left_ticks / 60;
    format!("{s}s")
}

#[allow(dead_code)]
fn _ambient() -> u16 {
    reactions::AMBIENT_TEMP
}
