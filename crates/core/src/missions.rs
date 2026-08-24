//! Short playable challenges with win / fail conditions.

use crate::element_id::*;
use crate::particle::Particle;
use crate::reactions;
use crate::scenarios::Scenario;
use crate::simulation::SimulationState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MissionId {
    HoldCritical = 0,
    PoisonRestart = 1,
    ForestFire = 2,
    CoolantLoop = 3,
    WireShot = 4,
    FilterRescue = 5,
    TritiumBreeder = 6,
    Quench = 7,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MissionStatus {
    Running = 0,
    Won = 1,
    Failed = 2,
}

/// Compact record stored in `.aura` saves.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionSave {
    pub id: u8,
    pub status: u8,
    pub started_tick: u64,
    pub hold_ticks: u64,
    pub time_limit: u64,
    pub message: String,
    pub start_pipes: u32,
    pub start_wood: u32,
    pub start_tnt: u32,
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
            MissionId::TritiumBreeder,
            MissionId::Quench,
        ]
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(MissionId::HoldCritical),
            1 => Some(MissionId::PoisonRestart),
            2 => Some(MissionId::ForestFire),
            3 => Some(MissionId::CoolantLoop),
            4 => Some(MissionId::WireShot),
            5 => Some(MissionId::FilterRescue),
            6 => Some(MissionId::TritiumBreeder),
            7 => Some(MissionId::Quench),
            _ => None,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            MissionId::HoldCritical => "Hold critical",
            MissionId::PoisonRestart => "Iodine pit",
            MissionId::ForestFire => "Forest fire",
            MissionId::CoolantLoop => "Keep the loop",
            MissionId::WireShot => "Wire shot",
            MissionId::FilterRescue => "Filter rescue",
            MissionId::TritiumBreeder => "Tritium breeder",
            MissionId::Quench => "Quench the core",
        }
    }

    pub fn brief(self) -> &'static str {
        match self {
            MissionId::HoldCritical => {
                "Keep k-eff between 0.70 and 1.40 for 10 seconds. [ ] move the rods."
            }
            MissionId::PoisonRestart => {
                "Raise the rods. Get k-eff back above 0.70 for two seconds."
            }
            MissionId::ForestFire => {
                "Paint water on the fire. A pond is on the left. Keep most of the wood."
            }
            MissionId::CoolantLoop => {
                "Let the loop run 12 s. Losing up to four pipes is OK."
            }
            MissionId::WireShot => "Paint Spark (hotbar) on the free wire end to set off the TNT.",
            MissionId::FilterRescue => "Water must fall through the filter. Sand stays on top.",
            MissionId::TritiumBreeder => "Lithium breeds tritium under neutron flux. Reach 15 tritium atoms.",
            MissionId::Quench => "Submerge the hot core in water and cool it below 900 K.",
        }
    }
}

impl Mission {
    pub fn start(sim: &mut SimulationState, id: MissionId) -> Self {
        match id {
            MissionId::HoldCritical => setup_hold(sim),
            MissionId::PoisonRestart => setup_poison(sim),
            MissionId::ForestFire => setup_forest(sim),
            MissionId::CoolantLoop => setup_loop(sim),
            MissionId::WireShot => setup_wire_shot(sim),
            MissionId::FilterRescue => setup_filter_rescue(sim),
            MissionId::TritiumBreeder => setup_tritium_breeder(sim),
            MissionId::Quench => setup_quench(sim),
        }
        let time_limit = match id {
            MissionId::HoldCritical => 40 * 60,
            MissionId::PoisonRestart => 40 * 60,
            MissionId::ForestFire => 35 * 60,
            MissionId::CoolantLoop => 25 * 60,
            MissionId::WireShot => 30 * 60,
            MissionId::FilterRescue => 25 * 60,
            MissionId::TritiumBreeder => 30 * 60,
            MissionId::Quench => 30 * 60,
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

    pub fn to_save(&self) -> MissionSave {
        MissionSave {
            id: self.id as u8,
            status: self.status as u8,
            started_tick: self.started_tick,
            hold_ticks: self.hold_ticks,
            time_limit: self.time_limit,
            message: self.message.clone(),
            start_pipes: self.start_pipes,
            start_wood: self.start_wood,
            start_tnt: self.start_tnt,
        }
    }

    pub fn from_save(s: &MissionSave) -> Option<Self> {
        Some(Self {
            id: MissionId::from_u8(s.id)?,
            status: match s.status {
                1 => MissionStatus::Won,
                2 => MissionStatus::Failed,
                _ => MissionStatus::Running,
            },
            started_tick: s.started_tick,
            hold_ticks: s.hold_ticks,
            time_limit: s.time_limit,
            message: s.message.clone(),
            start_pipes: s.start_pipes,
            start_wood: s.start_wood,
            start_tnt: s.start_tnt,
        })
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
                if k > 2.8 {
                    self.fail("Went prompt-critical.");
                    return;
                }
                if (0.70..=1.40).contains(&k) {
                    self.hold_ticks += 1;
                } else {
                    self.hold_ticks = self.hold_ticks.saturating_sub(2);
                }
                self.message = format!(
                    "Hold k 0.70–1.40 for 10 s   now {:.2}   held {:.1}s",
                    k,
                    self.hold_ticks as f32 / 60.0
                );
                if self.hold_ticks >= 10 * 60 {
                    self.win("Pile held in the band.");
                }
            }
            MissionId::PoisonRestart => {
                self.message = format!(
                    "Get k-eff ≥ 0.70 for 2 s   now {:.2}   I={} Xe={}",
                    sim.k_effective, sim.iodine_count, sim.xenon_count
                );
                if sim.k_effective >= 0.70 {
                    self.hold_ticks += 1;
                } else {
                    self.hold_ticks = 0;
                }
                if self.hold_ticks >= 2 * 60 {
                    self.win("Restarted through the iodine pit.");
                }
            }
            MissionId::ForestFire => {
                let fire = count(sim, FIRE);
                let wood = count(sim, WOOD) + count(sim, COAL);
                self.message = format!("Fire: {fire}   wood left: {wood}/{}", self.start_wood);
                if self.start_wood > 0 && wood * 5 < self.start_wood * 2 {
                    self.fail("Too much timber burned.");
                    return;
                }
                if fire == 0 && elapsed > 20 {
                    self.win("Fire is out.");
                }
            }
            MissionId::CoolantLoop => {
                let pipes = count(sim, PIPE);
                let steam = count(sim, STEAM);
                let lost = self.start_pipes.saturating_sub(pipes);
                self.message = format!("Pipes lost: {lost}/4   steam: {steam}");
                if lost > 4 {
                    self.fail("Loop burst.");
                    return;
                }
                if elapsed >= 12 * 60 && lost <= 4 {
                    self.win("Loop held.");
                }
            }
            MissionId::WireShot => {
                let tnt = count(sim, TNT);
                self.message = format!("TNT left: {tnt}  — paint Spark on the free wire end");
                if self.start_tnt > 0 && tnt < self.start_tnt {
                    self.win("Charge fired.");
                }
            }
            MissionId::FilterRescue => {
                let (water_below, sand_below, sand_above) = filter_score(sim);
                self.message =
                    format!("Water below: {water_below}   sand still above: {sand_above}");
                if sand_below > 6 {
                    self.fail("Sand went through the filter.");
                    return;
                }
                if water_below >= 5 && sand_above >= 4 {
                    self.win("Water drained, sand held.");
                }
            }
            MissionId::TritiumBreeder => {
                let t = count(sim, TRITIUM);
                self.message = format!("Tritium bred: {t}/15");
                if t >= 15 {
                    self.win("Bred 15 tritium atoms from lithium.");
                }
            }
            MissionId::Quench => {
                let core = max_temp_of(sim, STEEL);
                self.message = format!("Core temperature: {core} K   target < 900");
                if core < 900 {
                    self.win("Core quenched below 900 K.");
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
        .iter_particles()
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

fn put(sim: &mut SimulationState, x: u32, y: u32, id: u16, t: u16) {
    if x < sim.grid.width && y < sim.grid.height {
        sim.grid.set(x, y, Particle::new(id, t));
    }
}

fn setup_hold(sim: &mut SimulationState) {
    // Compact, rod-controlled pile that sits near k ≈ 0.8–1.2.
    sim.grid.clear();
    sim.neutron_queue.clear();
    sim.tick = 0;
    let w = sim.grid.width;
    let h = sim.grid.height;
    for x in 0..w {
        put(sim, x, h - 1, CONCRETE, reactions::AMBIENT_TEMP);
        put(sim, x, h - 2, CONCRETE, reactions::AMBIENT_TEMP);
    }
    let cx = w / 2;
    for y in h.saturating_sub(16)..h.saturating_sub(4) {
        for x in cx.saturating_sub(6)..cx + 6 {
            put(sim, x, y, GRAPHITE, 300);
        }
    }
    for y in h.saturating_sub(14)..h.saturating_sub(6) {
        for x in cx.saturating_sub(4)..cx + 4 {
            put(sim, x, y, U235, 360);
        }
    }
    for y in h.saturating_sub(16)..h.saturating_sub(4) {
        put(sim, cx.saturating_sub(8), y, CONTROL_ROD, 293);
        put(sim, cx.saturating_sub(7), y, CONTROL_ROD, 293);
        put(sim, cx + 7, y, CONTROL_ROD, 293);
        put(sim, cx + 8, y, CONTROL_ROD, 293);
    }
    for y in h.saturating_sub(12)..h.saturating_sub(6) {
        put(sim, cx.saturating_sub(5), y, WATER, 310);
        put(sim, cx + 5, y, WATER, 310);
    }
    put(sim, cx, h.saturating_sub(15), NEUTRON_THERMAL, 350);
    sim.refresh_chunks_public();
}

fn setup_poison(sim: &mut SimulationState) {
    setup_hold(sim);
    let w = sim.grid.width;
    let h = sim.grid.height;
    let cx = w / 2;
    // A light iodine cloud — enough to feel, not enough to brick the pile.
    for y in h.saturating_sub(13)..h.saturating_sub(8) {
        for x in cx.saturating_sub(3)..cx + 3 {
            if sim.grid.get(x, y).map(|p| p.is_empty()).unwrap_or(false) {
                put(sim, x, y, IODINE, 340);
            }
        }
    }
    sim.shift_control_rods(3);
    sim.refresh_chunks_public();
}

fn setup_forest(sim: &mut SimulationState) {
    sim.load_scenario(Scenario::ForestFire);
    let h = sim.grid.height;
    // Water pond on the left so the player has something to paint with.
    for y in h.saturating_sub(10)..h.saturating_sub(2) {
        for x in 1..8 {
            put(sim, x, y, WATER, 293);
        }
    }
}

fn setup_loop(sim: &mut SimulationState) {
    sim.load_scenario(Scenario::CoolantLoop);
    // Tone the heaters down so four-pipe tolerance is enough.
    let w = sim.grid.width;
    let h = sim.grid.height;
    for y in 0..h {
        for x in 0..w {
            if sim.grid.get(x, y).map(|p| p.element_id) == Some(HEATER) {
                sim.grid.modify(x, y, |p| {
                    p.temperature = 900;
                });
            }
        }
    }
}

fn setup_wire_shot(sim: &mut SimulationState) {
    sim.grid.clear();
    sim.neutron_queue.clear();
    sim.tick = 0;
    let w = sim.grid.width;
    let h = sim.grid.height;
    let y = h / 2;
    for x in 0..w {
        put(sim, x, h - 1, STONE, 293);
    }
    for x in w / 4..w / 2 + 8 {
        put(sim, x, y, WIRE, 293);
    }
    for dy in -2..=2_i32 {
        for dx in -2..=2_i32 {
            let tx = w as i32 / 2 + 10 + dx;
            let ty = y as i32 + dy;
            if tx >= 0 && ty >= 0 {
                put(sim, tx as u32, ty as u32, TNT, 293);
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
        put(sim, x, h - 1, STONE, 293);
        put(sim, x, h / 2, FILTER, 293);
    }
    // Mostly water, some sand — drain is obvious, sand still has to stay up.
    for y in 3..h / 2 {
        for x in 4..w - 4 {
            if (x + y) % 5 == 0 {
                put(sim, x, y, SAND, 293);
            } else {
                put(sim, x, y, WATER, 293);
            }
        }
    }
    sim.refresh_chunks_public();
}

fn setup_tritium_breeder(sim: &mut SimulationState) {
    sim.grid.clear();
    sim.neutron_queue.clear();
    sim.tick = 0;
    let w = sim.grid.width;
    let h = sim.grid.height;
    for x in 0..w {
        put(sim, x, h - 1, CONCRETE, reactions::AMBIENT_TEMP);
    }
    let cx = w / 2;
    let cy = h / 2;
    // A lithium blanket.
    for y in (cy - 4)..(cy + 4) {
        for x in (cx - 12)..(cx + 12) {
            put(sim, x, y, LITHIUM, 300);
        }
    }
    // Switch on a neutron flux into the blanket (one neutron per Li cell,
    // staggered delays). Breeding is probabilistic (0.4), so ~70 cells breed.
    let mut i = 0u8;
    for y in (cy - 4)..(cy + 4) {
        for x in (cx - 12)..(cx + 12) {
            sim.neutron_queue.push_back(crate::simulation::NeutronEvent {
                x: x as u32,
                y: y as u32,
                delay: i % 4,
                energy: crate::reactions::NeutronEnergy::Thermal,
            });
            i = i.wrapping_add(1);
        }
    }
    sim.refresh_chunks_public();
}

fn setup_quench(sim: &mut SimulationState) {
    sim.grid.clear();
    sim.neutron_queue.clear();
    sim.tick = 0;
    let w = sim.grid.width;
    let h = sim.grid.height;
    for x in 0..w {
        put(sim, x, h - 1, CONCRETE, reactions::AMBIENT_TEMP);
    }
    let cx = w / 2;
    let cy = h / 2;
    // A small glowing-hot steel core (small enough that the pool can quench it).
    for y in (cy - 1)..(cy + 2) {
        for x in (cx - 1)..(cx + 2) {
            put(sim, x, y, STEEL, 2400);
        }
    }
    // ...submerged in a large water pool (the player's coolant).
    for y in (cy - 14)..(h - 1) {
        for x in (cx.saturating_sub(24))..(cx + 24).min(w) {
            if sim.grid.get(x, y).map(|p| p.is_empty()).unwrap_or(true) {
                put(sim, x, y, WATER, 293);
            }
        }
    }
    sim.refresh_chunks_public();
}

/// Highest temperature among all cells of a given element (Quench gate).
fn max_temp_of(sim: &SimulationState, id: u16) -> u16 {
    let mut m = 0u16;
    for i in 0..sim.grid.len() {
        if sim.grid.element_at(i) == id {
            let t = sim.grid.temperature_at(i);
            if t > m {
                m = t;
            }
        }
    }
    m
}

/// Ordered campaign (P8): missions unlock as the previous one is won. The first
/// mission is always available. Progress is held in `completed`; persisting it
/// across saves is a follow-up (the mission itself already round-trips).
#[derive(Clone, Debug, Default)]
pub struct Campaign {
    pub order: Vec<MissionId>,
    pub completed: Vec<MissionId>,
}

impl Campaign {
    pub fn new() -> Self {
        Self {
            order: MissionId::all().to_vec(),
            completed: Vec::new(),
        }
    }

    /// The first mission is always unlocked; every later one unlocks when the
    /// mission immediately before it in `order` is completed.
    pub fn is_unlocked(&self, id: MissionId) -> bool {
        let Some(i) = self.order.iter().position(|&m| m == id) else {
            return false;
        };
        i == 0 || self.completed.contains(&self.order[i - 1])
    }

    /// Record a mission result; a win unlocks the next mission in the order.
    pub fn record(&mut self, id: MissionId, status: MissionStatus) {
        if status == MissionStatus::Won && !self.completed.contains(&id) {
            self.completed.push(id);
        }
    }

    /// The next mission the player should attempt (unlocked and not yet won).
    pub fn next(&self) -> Option<MissionId> {
        self.order
            .iter()
            .copied()
            .find(|&m| self.is_unlocked(m) && !self.completed.contains(&m))
    }
}
