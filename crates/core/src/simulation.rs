use crate::chunk::{ChunkPool, CHUNK_SIZE};
use crate::devices::{self, PressureField};
use crate::element_id::*;
use crate::grid::{Grid, GridSnapshot};
use crate::particle::Particle;
use crate::physics::{self, VelocityField};
use crate::reactions::{self, NeutronEnergy};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct NeutronEvent {
    pub x: u32,
    pub y: u32,
    pub delay: u8,
    pub energy: NeutronEnergy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationSettings {
    pub temperature_diffusion_rate: f32,
    pub gravity_enabled: bool,
    pub critical_mass_threshold: u32,
    pub fusion_threshold: u16,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            temperature_diffusion_rate: 0.08,
            gravity_enabled: true,
            critical_mass_threshold: 8,
            fusion_threshold: reactions::FUSION_THRESHOLD,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationState {
    pub grid: Grid,
    pub tick: u64,
    pub tick_rate: u32,
    pub seed: u64,
    pub settings: SimulationSettings,
    pub neutron_queue: VecDeque<NeutronEvent>,
    pub reaction_count: u64,
    pub fission_count: u64,
    pub fusion_count: u64,
    pub decay_count: u64,
    /// Approximate k-effective, refreshed every tick from live cell counts.
    #[serde(default)]
    pub k_effective: f32,
    /// Smoothed fissions-per-tick — the "power" needle.
    #[serde(default)]
    pub power: f32,
    /// Estimated doubling / e-folding time in ticks (0 = unknown).
    #[serde(default)]
    pub period_ticks: f32,
    /// +1 rising, 0 holding, −1 dying.
    #[serde(default)]
    pub trend: i8,
    #[serde(default)]
    pub iodine_count: u32,
    #[serde(default)]
    pub xenon_count: u32,
    #[serde(default)]
    pub mission: Option<crate::missions::MissionSave>,
    #[serde(skip, default)]
    fission_at_last_hud: u64,
    #[serde(skip, default)]
    pub chunk_pool: ChunkPool,
    #[serde(skip, default)]
    pub velocities: VelocityField,
    #[serde(skip, default)]
    pub pressure: PressureField,
}

impl SimulationState {
    pub fn new(width: u32, height: u32, seed: u64) -> Self {
        Self {
            grid: Grid::new(width, height),
            tick: 0,
            tick_rate: 60,
            seed,
            settings: SimulationSettings::default(),
            neutron_queue: VecDeque::new(),
            reaction_count: 0,
            fission_count: 0,
            fusion_count: 0,
            decay_count: 0,
            k_effective: 0.0,
            power: 0.0,
            period_ticks: 0.0,
            trend: 0,
            iodine_count: 0,
            xenon_count: 0,
            mission: None,
            fission_at_last_hud: 0,
            chunk_pool: ChunkPool::new(width, height),
            velocities: VelocityField::new((width * height) as usize),
            pressure: PressureField::new((width * height) as usize),
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.grid.resize(w, h);
        self.chunk_pool = ChunkPool::new(w, h);
        self.velocities = VelocityField::new((w * h) as usize);
        self.pressure = PressureField::new((w * h) as usize);
    }

    pub fn refresh_chunks_public(&mut self) {
        self.refresh_chunks();
    }

    pub fn shift_control_rods(&mut self, dy: i32) {
        devices::shift_control_rods(&mut self.grid, dy);
        self.refresh_chunks();
    }

    /// Prefill a small graphite-moderated U-235 pile plus a D+T fusion sample.
    pub fn setup_reactor_demo(&mut self) {
        let w = self.grid.width;
        let h = self.grid.height;
        if w < 40 || h < 30 {
            return;
        }
        for x in 0..w {
            self.grid
                .set(x, h - 2, Particle::new(CONCRETE, reactions::AMBIENT_TEMP));
            self.grid
                .set(x, h - 1, Particle::new(CONCRETE, reactions::AMBIENT_TEMP));
        }
        for y in h.saturating_sub(20)..h {
            self.grid
                .set(0, y, Particle::new(CONCRETE, reactions::AMBIENT_TEMP));
            self.grid
                .set(w - 1, y, Particle::new(CONCRETE, reactions::AMBIENT_TEMP));
        }
        for y in h.saturating_sub(12)..h.saturating_sub(5) {
            for x in w / 2 - 8..w / 2 + 8 {
                if fastrand::bool() {
                    self.grid.set(x, y, Particle::new(U235, 350));
                }
            }
        }
        for y in h.saturating_sub(15)..h.saturating_sub(12) {
            for x in w / 2 - 10..w / 2 + 10 {
                self.grid.set(x, y, Particle::new(GRAPHITE, 300));
            }
        }
        self.grid
            .set(w / 2, h - 14, Particle::new(NEUTRON_THERMAL, 350));
        for y in h.saturating_sub(15)..h.saturating_sub(2) {
            self.grid
                .set(w / 2 - 12, y, Particle::new(BORON, reactions::AMBIENT_TEMP));
            self.grid
                .set(w / 2 + 12, y, Particle::new(BORON, reactions::AMBIENT_TEMP));
        }
        self.grid.set(30, 30, Particle::new(DEUTERIUM, 1600));
        self.grid.set(31, 30, Particle::new(TRITIUM, 1600));
        self.refresh_chunks();
    }

    fn refresh_chunks(&mut self) {
        let expected_cx = self.grid.width.div_ceil(CHUNK_SIZE as u32);
        let expected_cy = self.grid.height.div_ceil(CHUNK_SIZE as u32);
        if self.chunk_pool.chunks_x != expected_cx || self.chunk_pool.chunks_y != expected_cy {
            self.chunk_pool = ChunkPool::new(self.grid.width, self.grid.height);
        }
        for meta in &mut self.chunk_pool.metas {
            meta.clear();
        }
        let mut fissile = 0u32;
        let mut moderator = 0u32;
        let mut absorber = 0u32;
        let mut iodine = 0u32;
        let mut xenon = 0u32;
        let w = self.grid.width;
        let h = self.grid.height;
        let cs = CHUNK_SIZE as u32;
        for y in 0..h {
            for x in 0..w {
                let id = self.grid.element_at(self.grid.index(x, y));
                if id == AIR {
                    continue;
                }
                if let Some(meta) = self.chunk_pool.get_mut(x / cs, y / cs) {
                    meta.mark_dirty(x % cs, y % cs);
                }
                if is_fissile(id) {
                    fissile += 1;
                }
                if is_moderator(id) {
                    moderator += 1;
                }
                if matches!(id, BORON | CONTROL_ROD | XENON | IODINE) {
                    absorber += 1;
                }
                if id == IODINE {
                    iodine += 1;
                }
                if id == XENON {
                    xenon += 1;
                }
            }
        }
        self.iodine_count = iodine;
        self.xenon_count = xenon;
        self.k_effective = reactions::criticality_factor(fissile, moderator, absorber);
        self.update_reactor_hud();
    }

    fn update_reactor_hud(&mut self) {
        let df = self.fission_count.saturating_sub(self.fission_at_last_hud) as f32;
        self.fission_at_last_hud = self.fission_count;
        let prev = self.power;
        self.power = prev * 0.82 + df * 0.18;
        if self.power > prev * 1.08 && self.power > 0.05 {
            self.trend = 1;
            let ratio = (self.power / prev.max(0.01)).clamp(1.01, 4.0);
            self.period_ticks = std::f32::consts::LN_2 / ratio.ln();
        } else if self.power < prev * 0.92 && prev > 0.05 {
            self.trend = -1;
            let ratio = (prev / self.power.max(0.01)).clamp(1.01, 4.0);
            self.period_ticks = -(std::f32::consts::LN_2 / ratio.ln());
        } else {
            self.trend = 0;
        }
    }

    pub fn reactor_status(&self) -> &'static str {
        match self.trend {
            1 => "rising",
            -1 => "dying",
            _ if self.k_effective >= 0.95 => "holding",
            _ => "subcritical",
        }
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        self.refresh_chunks();
        let mut rng = fastrand::Rng::with_seed(self.seed.wrapping_add(self.tick));
        self.process_neutron_queue(&mut rng);
        self.physics_pass(&mut rng);
        crate::hydro::equalize_liquid_surface(&mut self.grid, &mut self.velocities, &mut rng);
        crate::hydro::powder_overburden_slide(&mut self.grid, &mut self.velocities, &mut rng);
        devices::step_devices(
            &mut self.grid,
            &mut self.velocities,
            &mut self.pressure,
            &mut rng,
            self.k_effective,
            Some(&self.chunk_pool),
        );
        crate::hydro::add_hydrostatic_pressure(&self.grid, &mut self.pressure);
        crate::hydro::step_pipe_network(
            &mut self.grid,
            &mut self.velocities,
            &mut self.pressure,
            &mut rng,
        );

        let total_cells = self.grid.width as usize * self.grid.height as usize;
        if total_cells >= 65536 {
            self.reaction_pass_parallel(&mut rng);
            self.effects_pass_parallel(&mut rng);
        } else {
            self.reaction_pass(&mut rng);
            self.effects_pass(&mut rng);
        }
    }

    /// Chunk-based parallel version of reaction_pass using rayon.
    /// Scans for fissile/decay/fusion candidates in parallel across chunks.
    fn reaction_pass_parallel(&mut self, rng: &mut fastrand::Rng) {
        type FissileList = Vec<(u32, u32)>;
        type DecayList = Vec<(u32, u32)>;
        type FusionList = Vec<(u32, u32, u32, u32)>;
        type ChunkResult = (FissileList, DecayList, FusionList);

        let w = self.grid.width;
        let h = self.grid.height;
        let active = self.chunk_pool.active_chunks();
        let chunk_coords: Vec<(u32, u32)> = if active.is_empty() {
            let chunks_x = w.div_ceil(CHUNK_SIZE as u32);
            let chunks_y = h.div_ceil(CHUNK_SIZE as u32);
            (0..chunks_y)
                .flat_map(|cy| (0..chunks_x).map(move |cx| (cx, cy)))
                .collect()
        } else {
            active
        };

        let chunk_results: Vec<ChunkResult> = chunk_coords
            .par_iter()
            .map(|&(cx, cy)| {
                let start_x = cx * CHUNK_SIZE as u32;
                let start_y = cy * CHUNK_SIZE as u32;
                let end_x = (start_x + CHUNK_SIZE as u32).min(w);
                let end_y = (start_y + CHUNK_SIZE as u32).min(h);

                let mut fissile: Vec<(u32, u32)> = Vec::new();
                let mut decay: Vec<(u32, u32)> = Vec::new();
                let mut fusion: Vec<(u32, u32, u32, u32)> = Vec::new();

                for y in start_y..end_y {
                    for x in start_x..end_x {
                        let id = self.grid.get(x, y).unwrap().element_id;
                        if is_fissile(id) {
                            fissile.push((x, y));
                        }
                        if matches!(id, U235 | U238 | PU239 | PU240 | TRITIUM | XENON | IODINE) {
                            decay.push((x, y));
                        }
                        if id == DEUTERIUM || id == TRITIUM {
                            for dy in -1..=1_i32 {
                                for dx in -1..=1_i32 {
                                    if dx == 0 && dy == 0 {
                                        continue;
                                    }
                                    let nx = x as i32 + dx;
                                    let ny = y as i32 + dy;
                                    if !self.grid.in_bounds(nx, ny) {
                                        continue;
                                    }
                                    let nid =
                                        self.grid.get(nx as u32, ny as u32).unwrap().element_id;
                                    if (id == DEUTERIUM && nid == TRITIUM)
                                        || (id == TRITIUM && nid == DEUTERIUM)
                                    {
                                        let (ax, ay, bx, by) = if (x, y) < (nx as u32, ny as u32) {
                                            (x, y, nx as u32, ny as u32)
                                        } else {
                                            (nx as u32, ny as u32, x, y)
                                        };
                                        fusion.push((ax, ay, bx, by));
                                    }
                                }
                            }
                        }
                    }
                }
                (fissile, decay, fusion)
            })
            .collect();

        let mut fissile_to_check: Vec<(u32, u32)> = Vec::new();
        let mut decay_to_check: Vec<(u32, u32)> = Vec::new();
        let mut fusion_pairs: Vec<(u32, u32, u32, u32)> = Vec::new();
        for (f, d, fu) in chunk_results {
            fissile_to_check.extend(f);
            decay_to_check.extend(d);
            fusion_pairs.extend(fu);
        }
        // P2 determinism: par_iter returns chunk results in a thread-count-dependent
        // order, so the candidate lists must be sorted (and de-duplicated) before
        // the rng-driven application. Without this, the same grid produces different
        // fission/decay outcomes on 1 vs N threads. fusion_pairs was already sorted.
        fissile_to_check.sort_unstable();
        fissile_to_check.dedup();
        decay_to_check.sort_unstable();
        decay_to_check.dedup();
        fusion_pairs.sort_unstable();
        fusion_pairs.dedup();

        self.apply_collected_reactions(rng, fissile_to_check, decay_to_check, fusion_pairs);
    }

    fn apply_collected_reactions(
        &mut self,
        rng: &mut fastrand::Rng,
        fissile_to_check: Vec<(u32, u32)>,
        decay_to_check: Vec<(u32, u32)>,
        fusion_pairs: Vec<(u32, u32, u32, u32)>,
    ) {
        for (x, y) in fissile_to_check {
            let cur = self.grid.get(x, y).unwrap();
            if cur.is_empty() || !is_fissile(cur.element_id) || cur.has_flag(Particle::FLAG_REACTED)
            {
                continue;
            }
            let mut has_neutron = false;
            let mut neutron_energy = NeutronEnergy::Thermal;
            for dy in -1..=1_i32 {
                for dx in -1..=1_i32 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if !self.grid.in_bounds(nx, ny) {
                        continue;
                    }
                    let nid = self.grid.get(nx as u32, ny as u32).unwrap().element_id;
                    if nid == NEUTRON_THERMAL {
                        has_neutron = true;
                        neutron_energy = NeutronEnergy::Thermal;
                        break;
                    } else if nid == NEUTRON_FAST {
                        has_neutron = true;
                        neutron_energy = NeutronEnergy::Fast;
                    }
                }
                if has_neutron && neutron_energy == NeutronEnergy::Thermal {
                    break;
                }
            }
            if has_neutron {
                let prob =
                    reactions::fission_probability(cur.element_id, neutron_energy, cur.temperature);
                if rng.f32() < prob {
                    self.trigger_fission(x, y, rng);
                }
            } else if rng.f32() < reactions::spontaneous_fission_prob(self.k_effective) {
                self.trigger_fission(x, y, rng);
            }
        }

        for (x1, y1, x2, y2) in fusion_pairs {
            let p1 = self.grid.get(x1, y1).unwrap();
            let p2 = self.grid.get(x2, y2).unwrap();
            if p1.has_flag(Particle::FLAG_REACTED) || p2.has_flag(Particle::FLAG_REACTED) {
                continue;
            }
            if p1.temperature > self.settings.fusion_threshold
                && p2.temperature > self.settings.fusion_threshold
                && rng.f32() < reactions::FUSION_PROBABILITY
            {
                self.trigger_fusion(x1, y1, x2, y2, rng);
            }
        }

        for (x, y) in decay_to_check {
            if let Some(p) = self.grid.get(x, y) {
                if p.has_flag(Particle::FLAG_REACTED) {
                    continue;
                }
                let half_life = reactions::half_life_ticks(p.element_id);
                if half_life == 0 {
                    continue;
                }
                let prob = 0.693 / half_life as f32;
                if rng.f32() < prob {
                    self.trigger_decay(x, y, rng);
                }
            }
        }
    }

    /// Chunk-based parallel effects pass: temperature diffusion computed in parallel,
    /// meltdown/boiling/TNT processed sequentially.
    fn effects_pass_parallel(&mut self, rng: &mut fastrand::Rng) {
        let w = self.grid.width;
        let h = self.grid.height;
        let total = (w * h) as usize;
        let diff_rate = self.settings.temperature_diffusion_rate;

        let _ = (w, h, total);
        physics::diffuse_heat_parallel(&mut self.grid, diff_rate, Some(&self.chunk_pool));
        physics::apply_phase_changes(&mut self.grid, rng);
        self.apply_thermal_effects(rng);
    }

    fn apply_thermal_effects(&mut self, rng: &mut fastrand::Rng) {
        let w = self.grid.width;
        let h = self.grid.height;
        for y in 0..h {
            for x in 0..w {
                let p = self.grid.get(x, y).unwrap();
                if p.is_empty() {
                    continue;
                }
                if p.temperature > reactions::MELTDOWN_TEMP
                    && is_fissile(p.element_id)
                    && rng.f32() < reactions::MELTDOWN_PROB
                {
                    self.grid
                        .set(x, y, Particle::new(MOLTEN_FUEL, p.temperature));
                    for dy in -1..=1_i32 {
                        for dx in -1..=1_i32 {
                            let nx = x as i32 + dx;
                            let ny = y as i32 + dy;
                            if !self.grid.in_bounds(nx, ny) {
                                continue;
                            }
                            self.grid.modify(nx as u32, ny as u32, |n| {
                                n.temperature = n.temperature.saturating_add(100);
                            });
                        }
                    }
                }
                if p.temperature > reactions::BOIL_TEMP
                    && matches!(p.element_id, WATER | HEAVY_WATER | STEAM)
                    && rng.f32() < reactions::BOIL_PROB
                {
                    // Thermolysis of already-superheated water / steam.
                    self.grid.set(x, y, Particle::new(HYDROGEN, p.temperature));
                }
                #[cfg(feature = "fluid-pde")]
                if matches!(p.element_id, WATER | HEAVY_WATER) {
                    // P5b steam explosion: water in contact with molten fuel
                    // flashes to steam and the blast ejects its surroundings —
                    // a real reactor-accident transient the MVP has no notion of.
                    let xi = x as i32;
                    let yi = y as i32;
                    let contact = (-1..=1_i32)
                        .flat_map(|dy| (-1..=1_i32).map(move |dx| (dx, dy)))
                        .any(|(dx, dy)| {
                            dx == 0 && dy == 0
                                || self
                                    .grid
                                    .get((xi + dx) as u32, (yi + dy) as u32)
                                    .is_some_and(|q| q.element_id == MOLTEN_FUEL)
                        });
                    if contact && rng.f32() < 0.6 {
                        self.grid.set(x, y, Particle::new(STEAM, 2600));
                        physics::apply_impulse(&mut self.grid, &mut self.velocities, x, y, 4, rng);
                        for dy in -2..=2_i32 {
                            for dx in -2..=2_i32 {
                                let nx = xi + dx;
                                let ny = yi + dy;
                                if self.grid.in_bounds(nx, ny) {
                                    self.grid.modify(nx as u32, ny as u32, |q| {
                                        q.temperature = q.temperature.saturating_add(400);
                                    });
                                }
                            }
                        }
                    }
                }
                if p.element_id == TNT && p.temperature > reactions::TNT_IGNITE_TEMP {
                    self.trigger_tnt(x, y, rng);
                }
            }
        }
    }

    fn process_neutron_queue(&mut self, rng: &mut fastrand::Rng) {
        let mut remaining = VecDeque::new();
        let mut to_spawn: Vec<NeutronEvent> = Vec::new();

        while let Some(mut ev) = self.neutron_queue.pop_front() {
            if ev.delay > 0 {
                ev.delay -= 1;
                remaining.push_back(ev);
            } else {
                to_spawn.push(ev);
            }
        }
        self.neutron_queue = remaining;

        for ev in to_spawn {
            if !self.grid.in_bounds(ev.x as i32, ev.y as i32) {
                continue;
            }
            let x = ev.x;
            let y = ev.y;
            if let Some(cell) = self.grid.get(x, y) {
                let cell_id = cell.element_id;
                let cell_temp = cell.temperature;
                if cell.is_empty() {
                    let temp = match ev.energy {
                        NeutronEnergy::Thermal => 350,
                        NeutronEnergy::Fast => 800,
                    };
                    let id = match ev.energy {
                        NeutronEnergy::Thermal => NEUTRON_THERMAL,
                        NeutronEnergy::Fast => NEUTRON_FAST,
                    };
                    self.grid
                        .set(x, y, Particle::new(id, temp).with_lifetime(20));
                } else if is_fissile(cell_id) {
                    let prob = reactions::fission_probability(cell_id, ev.energy, cell_temp);
                    if rng.f32() < prob {
                        self.trigger_fission(x, y, rng);
                    } else {
                        self.grid.modify(x, y, |target| {
                            target.temperature = target.temperature.saturating_add(20);
                        });
                    }
                } else if cell_id == BORON {
                    if rng.f32() < reactions::absorber_chance(BORON, ev.energy) {
                        self.grid.set(x, y, Particle::new(FALLOUT, 500));
                        let ax = (x as i32 + rng.i32(-1..=1)).clamp(0, self.grid.width as i32 - 1)
                            as u32;
                        let ay = (y as i32 + rng.i32(-1..=1)).clamp(0, self.grid.height as i32 - 1)
                            as u32;
                        if self.grid.get(ax, ay).is_some_and(|p| p.is_empty()) {
                            self.grid
                                .set(ax, ay, Particle::new(ALPHA, 400).with_lifetime(0));
                        }
                    }
                } else if cell_id == CONTROL_ROD {
                    if rng.f32() < reactions::absorber_chance(CONTROL_ROD, ev.energy) {
                        self.grid.modify(x, y, |rod| {
                            rod.temperature = rod.temperature.saturating_add(45);
                        });
                    }
                } else if matches!(cell_id, XENON | IODINE) {
                    if rng.f32() < reactions::absorber_chance(cell_id, ev.energy) {
                        self.grid.set(x, y, Particle::air());
                    }
                } else if cell_id == LITHIUM && rng.f32() < reactions::LITHIUM_BREED_CHANCE {
                    // Li-6 + n -> T + He (simplified single-cell breeding)
                    self.grid
                        .set(x, y, Particle::new(TRITIUM, cell_temp.saturating_add(50)));
                    let hx =
                        (x as i32 + rng.i32(-1..=1)).clamp(0, self.grid.width as i32 - 1) as u32;
                    let hy =
                        (y as i32 + rng.i32(-1..=1)).clamp(0, self.grid.height as i32 - 1) as u32;
                    if self.grid.get(hx, hy).is_some_and(|p| p.is_empty()) {
                        self.grid.set(hx, hy, Particle::new(HELIUM, 400));
                    }
                    self.reaction_count += 1;
                } else if is_moderator(cell_id)
                    && ev.energy == NeutronEnergy::Fast
                    && rng.f32() < reactions::moderator_thermalize_chance(cell_id)
                {
                    self.neutron_queue.push_back(NeutronEvent {
                        x: x.saturating_add_signed(rng.i32(-1..=1)),
                        y: y.saturating_add_signed(rng.i32(-1..=1)),
                        delay: 1,
                        energy: NeutronEnergy::Thermal,
                    });
                }
            }
        }
    }

    fn physics_pass(&mut self, rng: &mut fastrand::Rng) {
        if !self.settings.gravity_enabled {
            return;
        }
        // P2b: large grids run the physics pass in parallel (per-chunk local
        // simulation + a sequential border pass). The threshold matches the
        // reaction pass, so a tick is either fully sequential or fully
        // parallel — which also keeps the golden corpus on one code path.
        let total_cells = (self.grid.width as usize) * (self.grid.height as usize);
        if total_cells >= 65536 {
            physics::step_active_parallel(
                &mut self.grid,
                &mut self.velocities,
                rng,
                &self.chunk_pool,
            );
        } else {
            physics::step_active(
                &mut self.grid,
                &mut self.velocities,
                rng,
                Some(&self.chunk_pool),
            );
        }
    }

    fn reaction_pass(&mut self, rng: &mut fastrand::Rng) {
        let w = self.grid.width;
        let h = self.grid.height;
        let mut fissile_to_check: Vec<(u32, u32)> = Vec::new();
        let mut decay_to_check: Vec<(u32, u32)> = Vec::new();
        let mut fusion_pairs: Vec<(u32, u32, u32, u32)> = Vec::new();

        let active = self.chunk_pool.active_chunks();
        let cells: Vec<(u32, u32)> = if active.is_empty() {
            (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).collect()
        } else {
            let cs = CHUNK_SIZE as u32;
            let mut out = Vec::new();
            for (cx, cy) in active {
                let start_x = cx * cs;
                let start_y = cy * cs;
                let end_x = (start_x + cs).min(w);
                let end_y = (start_y + cs).min(h);
                for y in start_y..end_y {
                    for x in start_x..end_x {
                        out.push((x, y));
                    }
                }
            }
            out
        };

        for (x, y) in cells {
            let id = self.grid.get(x, y).unwrap().element_id;
            if is_fissile(id) {
                fissile_to_check.push((x, y));
            }
            if matches!(id, U235 | U238 | PU239 | PU240 | TRITIUM | XENON | IODINE) {
                decay_to_check.push((x, y));
            }
            if id == DEUTERIUM || id == TRITIUM {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if !self.grid.in_bounds(nx, ny) {
                            continue;
                        }
                        let nid = self.grid.get(nx as u32, ny as u32).unwrap().element_id;
                        if (id == DEUTERIUM && nid == TRITIUM)
                            || (id == TRITIUM && nid == DEUTERIUM)
                        {
                            let (ax, ay, bx, by) = if (x, y) < (nx as u32, ny as u32) {
                                (x, y, nx as u32, ny as u32)
                            } else {
                                (nx as u32, ny as u32, x, y)
                            };
                            fusion_pairs.push((ax, ay, bx, by));
                        }
                    }
                }
            }
        }
        fusion_pairs.sort_unstable();
        fusion_pairs.dedup();
        self.apply_collected_reactions(rng, fissile_to_check, decay_to_check, fusion_pairs);
    }

    fn effects_pass(&mut self, rng: &mut fastrand::Rng) {
        physics::diffuse_heat_active(
            &mut self.grid,
            self.settings.temperature_diffusion_rate,
            Some(&self.chunk_pool),
        );
        physics::apply_phase_changes(&mut self.grid, rng);
        self.apply_thermal_effects(rng);
    }

    fn trigger_fission(&mut self, x: u32, y: u32, rng: &mut fastrand::Rng) {
        let orig = self.grid.get(x, y).unwrap();
        if orig.has_flag(Particle::FLAG_REACTED) || !is_fissile(orig.element_id) {
            return;
        }
        let mut product = Particle::new(
            FISSION_PRODUCTS,
            orig.temperature
                .saturating_add(reactions::FISSION_SELF_HEAT),
        );
        product.set_flag(Particle::FLAG_REACTED);
        self.grid.set(x, y, product);
        self.fission_count += 1;
        self.reaction_count += 1;

        let n_count = reactions::neutron_count(orig.element_id, rng)
            + reactions::k_extra_neutrons(self.k_effective, rng);
        for i in 0..n_count {
            let dx = rng.i32(-2..=2);
            let dy = rng.i32(-2..=2);
            let nx = (x as i32 + dx).clamp(0, self.grid.width as i32 - 1) as u32;
            let ny = (y as i32 + dy).clamp(0, self.grid.height as i32 - 1) as u32;
            // ~15% delayed neutrons — they keep a pile critical after the prompt burst.
            let delay = if i == 0 && rng.f32() < 0.15 {
                rng.u8(10..=28)
            } else {
                rng.u8(1..=3)
            };
            self.neutron_queue.push_back(NeutronEvent {
                x: nx,
                y: ny,
                delay,
                energy: NeutronEnergy::Fast,
            });
        }
        // Iodine pit: most poison is born as I-135 and later decays to Xe-135.
        let roll = rng.f32();
        if roll < 0.10 {
            let dx = rng.i32(-1..=1);
            let dy = rng.i32(-1..=1);
            let nx = (x as i32 + dx).clamp(0, self.grid.width as i32 - 1) as u32;
            let ny = (y as i32 + dy).clamp(0, self.grid.height as i32 - 1) as u32;
            if self.grid.get(nx, ny).unwrap().is_empty() {
                self.grid.set(nx, ny, Particle::new(IODINE, 400));
            }
        } else if roll < 0.14 {
            let dx = rng.i32(-1..=1);
            let dy = rng.i32(-1..=1);
            let nx = (x as i32 + dx).clamp(0, self.grid.width as i32 - 1) as u32;
            let ny = (y as i32 + dy).clamp(0, self.grid.height as i32 - 1) as u32;
            if self.grid.get(nx, ny).unwrap().is_empty() {
                self.grid.set(nx, ny, Particle::new(XENON, 400));
            }
        }
        for _ in 0..rng.u32(1..=2) {
            let dx = rng.i32(-1..=1);
            let dy = rng.i32(-1..=1);
            let nx = (x as i32 + dx).clamp(0, self.grid.width as i32 - 1) as u32;
            let ny = (y as i32 + dy).clamp(0, self.grid.height as i32 - 1) as u32;
            if self.grid.get(nx, ny).unwrap().is_empty() {
                self.grid
                    .set(nx, ny, Particle::new(GAMMA, 1000).with_lifetime(0));
            }
        }
        for dy in -2..=2 {
            for dx in -2..=2 {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if !self.grid.in_bounds(nx, ny) {
                    continue;
                }
                self.grid.modify(nx as u32, ny as u32, |n| {
                    if !n.is_empty() {
                        n.temperature = n.temperature.saturating_add(rng.u16(50..200));
                    }
                });
            }
        }
    }

    fn trigger_fusion(&mut self, x1: u32, y1: u32, x2: u32, y2: u32, rng: &mut fastrand::Rng) {
        let mut helium = Particle::new(HELIUM, 3000);
        helium.set_flag(Particle::FLAG_REACTED);
        self.grid.set(x1, y1, helium);
        self.grid.set(x2, y2, Particle::air());
        self.fusion_count += 1;
        self.reaction_count += 1;

        let nx = x1 as i32 + rng.i32(-2..=2);
        let ny = y1 as i32 + rng.i32(-2..=2);
        if self.grid.in_bounds(nx, ny) {
            self.neutron_queue.push_back(NeutronEvent {
                x: nx as u32,
                y: ny as u32,
                delay: 1,
                energy: NeutronEnergy::Fast,
            });
        }
        for dy in -3..=3 {
            for dx in -3..=3 {
                let nx = x1 as i32 + dx;
                let ny = y1 as i32 + dy;
                if !self.grid.in_bounds(nx, ny) {
                    continue;
                }
                self.grid.modify(nx as u32, ny as u32, |n| {
                    n.temperature = n.temperature.saturating_add(reactions::FUSION_RADIUS_HEAT);
                });
            }
        }
    }

    fn trigger_decay(&mut self, x: u32, y: u32, rng: &mut fastrand::Rng) {
        let p = self.grid.get(x, y).unwrap();
        let daughter = reactions::decay_daughter(p.element_id);
        let radiation = reactions::decay_radiation(p.element_id);
        let mut next = Particle::new(daughter, p.temperature);
        next.set_flag(Particle::FLAG_REACTED);
        self.grid.set(x, y, next);
        self.decay_count += 1;

        if radiation != AIR {
            let dx = rng.i32(-1..=1);
            let dy = rng.i32(-1..=1);
            let nx = (x as i32 + dx).clamp(0, self.grid.width as i32 - 1) as u32;
            let ny = (y as i32 + dy).clamp(0, self.grid.height as i32 - 1) as u32;
            if self.grid.get(nx, ny).unwrap().is_empty() {
                self.grid
                    .set(nx, ny, Particle::new(radiation, 400).with_lifetime(0));
            }
        }
    }

    fn trigger_tnt(&mut self, x: u32, y: u32, rng: &mut fastrand::Rng) {
        let radius = 6;
        physics::apply_impulse(&mut self.grid, &mut self.velocities, x, y, radius, rng);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy > radius * radius {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if !self.grid.in_bounds(nx, ny) {
                    continue;
                }
                if rng.f32() < 0.7 {
                    self.grid.set(nx as u32, ny as u32, Particle::air());
                } else {
                    self.grid.modify(nx as u32, ny as u32, |n| {
                        n.temperature = n.temperature.saturating_add(300);
                    });
                }
            }
        }
        self.grid.set(x, y, Particle::new(FALLOUT, 800));
    }

    pub fn snapshot_rgba<F>(&self, color_fn: F) -> GridSnapshot
    where
        F: Fn(u16) -> [u8; 4],
    {
        let pixels = self.grid.to_rgba_buffer(color_fn);
        GridSnapshot {
            width: self.grid.width,
            height: self.grid.height,
            pixels,
        }
    }
}
