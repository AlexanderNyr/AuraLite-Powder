use crate::chunk::CHUNK_SIZE;
use crate::element_id::*;
use crate::grid::{Grid, GridSnapshot};
use crate::particle::Particle;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeutronEnergy {
    Thermal,
    Fast,
}

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
            fusion_threshold: 1500,
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
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.grid.resize(w, h);
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        let mut rng = fastrand::Rng::with_seed(self.seed.wrapping_add(self.tick));
        self.process_neutron_queue(&mut rng);
        self.physics_pass(&mut rng);

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
        let chunks_x = w.div_ceil(CHUNK_SIZE as u32);
        let chunks_y = h.div_ceil(CHUNK_SIZE as u32);
        let _base_seed = self.seed.wrapping_add(self.tick);

        // Collect candidates in parallel
        let chunk_results: Vec<ChunkResult> = (0..chunks_y)
            .flat_map(|cy| (0..chunks_x).map(move |cx| (cx, cy)))
            .collect::<Vec<_>>()
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
                        if matches!(id, U235 | U238 | PU239 | PU240 | TRITIUM) {
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
                                        fusion.push((x, y, nx as u32, ny as u32));
                                    }
                                }
                            }
                        }
                    }
                }
                (fissile, decay, fusion)
            })
            .collect();

        // Merge all candidates
        let mut fissile_to_check: Vec<(u32, u32)> = Vec::new();
        let mut decay_to_check: Vec<(u32, u32)> = Vec::new();
        let mut fusion_pairs: Vec<(u32, u32, u32, u32)> = Vec::new();
        for (f, d, fu) in chunk_results {
            fissile_to_check.extend(f);
            decay_to_check.extend(d);
            fusion_pairs.extend(fu);
        }

        // Process sequentially (reactions involve state changes)
        for (x, y) in fissile_to_check {
            let cur = *self.grid.get(x, y).unwrap();
            if cur.is_empty() || !is_fissile(cur.element_id) {
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
                    Self::fission_probability(cur.element_id, neutron_energy, cur.temperature);
                if rng.f32() < prob {
                    self.trigger_fission(x, y, rng);
                }
            } else if rng.f32() < 0.00001 {
                self.trigger_fission(x, y, rng);
            }
        }

        for (x1, y1, x2, y2) in fusion_pairs {
            let p1 = *self.grid.get(x1, y1).unwrap();
            let p2 = *self.grid.get(x2, y2).unwrap();
            if p1.temperature > self.settings.fusion_threshold
                && p2.temperature > self.settings.fusion_threshold
                && rng.f32() < 0.05
            {
                self.trigger_fusion(x1, y1, x2, y2, rng);
            }
        }

        for (x, y) in decay_to_check {
            if let Some(p) = self.grid.get(x, y).copied() {
                let half_life = Self::half_life_ticks(p.element_id);
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

        // Parallel temperature diffusion
        let new_temps: Vec<u16> = (0..total)
            .into_par_iter()
            .map(|idx| {
                let x = (idx as u32) % w;
                let y = (idx as u32) / w;
                let cur_temp = self.grid.particles[idx].temperature as f32;
                let mut sum = cur_temp;
                let mut count = 1.0_f32;
                for dy in -1..=1_i32 {
                    for dx in -1..=1_i32 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || ny < 0 || nx as u32 >= w || ny as u32 >= h {
                            continue;
                        }
                        let nidx = (ny as u32 * w + nx as u32) as usize;
                        sum += self.grid.particles[nidx].temperature as f32;
                        count += 1.0;
                    }
                }
                let avg = sum / count;
                let diffused = cur_temp + (avg - cur_temp) * diff_rate;
                let cooled = diffused * 0.999 + 293.0 * 0.001;
                cooled.clamp(0.0, 5000.0) as u16
            })
            .collect();

        for (i, temp) in new_temps.into_iter().enumerate() {
            self.grid.particles[i].temperature = temp;
        }

        // Meltdown, boiling, TNT checks (sequential - sparse operations)
        for y in 0..h {
            for x in 0..w {
                let p = *self.grid.get(x, y).unwrap();
                if p.is_empty() {
                    continue;
                }
                if p.temperature > 2000 && is_fissile(p.element_id) && rng.f32() < 0.01 {
                    self.grid
                        .set(x, y, Particle::new(MOLTEN_FUEL, p.temperature));
                    for dy in -1..=1_i32 {
                        for dx in -1..=1_i32 {
                            let nx = x as i32 + dx;
                            let ny = y as i32 + dy;
                            if !self.grid.in_bounds(nx, ny) {
                                continue;
                            }
                            if let Some(n) = self.grid.get_mut(nx as u32, ny as u32) {
                                n.temperature = n.temperature.saturating_add(100);
                            }
                        }
                    }
                }
                if p.temperature > 2500
                    && matches!(p.element_id, WATER | HEAVY_WATER)
                    && rng.f32() < 0.05
                {
                    self.grid.set(x, y, Particle::new(HYDROGEN, p.temperature));
                }
                if p.element_id == TNT && p.temperature > 500 {
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
                    let prob = Self::fission_probability(cell_id, ev.energy, cell_temp);
                    if rng.f32() < prob {
                        self.trigger_fission(x, y, rng);
                    } else if let Some(target) = self.grid.get_mut(x, y) {
                        target.temperature = target.temperature.saturating_add(20);
                    }
                } else if cell_id == BORON {
                    if rng.f32() < 0.8 {
                        self.grid.set(x, y, Particle::new(FALLOUT, 500));
                    }
                } else if is_moderator(cell_id)
                    && ev.energy == NeutronEnergy::Fast
                    && rng.f32() < 0.4
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
        let w = self.grid.width as i32;
        let h = self.grid.height as i32;

        for y in (0..h - 1).rev() {
            let mut xs: Vec<i32> = (0..w).collect();
            if rng.bool() {
                xs.reverse();
            } else {
                for i in 0..xs.len() {
                    let j = rng.usize(0..xs.len());
                    xs.swap(i, j);
                }
            }
            for x in xs {
                self.apply_gravity_at(x as u32, y as u32, rng);
            }
        }

        for y in 1..h {
            for x in 0..w {
                let id = if let Some(p) = self.grid.get(x as u32, y as u32) {
                    p.element_id
                } else {
                    continue;
                };
                if is_gas(id) && y > 0 {
                    let above_empty = self
                        .grid
                        .get(x as u32, (y - 1) as u32)
                        .is_some_and(|p| p.is_empty());
                    if above_empty && rng.f32() < 0.5 {
                        let p = *self.grid.get(x as u32, y as u32).unwrap();
                        self.grid.set(x as u32, (y - 1) as u32, p);
                        self.grid.set(x as u32, y as u32, Particle::air());
                    } else {
                        let dir = if rng.bool() { -1 } else { 1 };
                        let nx = x + dir;
                        if self.grid.in_bounds(nx, y)
                            && self
                                .grid
                                .get(nx as u32, y as u32)
                                .is_some_and(|p| p.is_empty())
                        {
                            let p = *self.grid.get(x as u32, y as u32).unwrap();
                            self.grid.set(nx as u32, y as u32, p);
                            self.grid.set(x as u32, y as u32, Particle::air());
                        }
                    }
                }
                if is_radiation(id) {
                    self.apply_radiation_movement(x as u32, y as u32, rng);
                }
            }
        }
    }

    fn apply_gravity_at(&mut self, x: u32, y: u32, rng: &mut fastrand::Rng) {
        let w = self.grid.width;
        let h = self.grid.height;
        if x >= w || y >= h {
            return;
        }
        let cur = *self.grid.get(x, y).unwrap();
        if cur.is_empty() {
            return;
        }
        let cur_kind = kind_for_id(cur.element_id);
        if is_radiation(cur.element_id) {
            return;
        }
        let below_y = y + 1;
        if below_y >= h {
            return;
        }
        let below = *self.grid.get(x, below_y).unwrap();
        if below.is_empty() {
            self.grid.set(x, below_y, cur);
            self.grid.set(x, y, Particle::air());
            return;
        }
        let cur_dens = density_for_id(cur.element_id);
        let below_dens = density_for_id(below.element_id);
        let can_swap = (cur_dens > below_dens + 0.1 && is_liquid(below.element_id))
            || (cur_dens > below_dens + 1.0 && below.element_id == AIR)
            || (cur_dens > below_dens && is_gas(below.element_id))
            || (is_liquid(cur.element_id) && is_liquid(below.element_id) && cur_dens > below_dens);

        if can_swap && rng.f32() < 0.9 {
            self.grid.set(x, below_y, cur);
            self.grid.set(x, y, below);
            return;
        }

        if cur_kind == ElementKind::Sand
            || cur_kind == ElementKind::Liquid
            || is_gas(cur.element_id)
            || cur_kind == ElementKind::Solid
        {
            let dirs = if rng.bool() { [-1, 1] } else { [1, -1] };
            for dx in dirs {
                let nx = x as i32 + dx;
                if nx < 0 || nx >= w as i32 {
                    continue;
                }
                let nx_u = nx as u32;
                if let Some(diag) = self.grid.get(nx_u, below_y).copied() {
                    if diag.is_empty() {
                        self.grid.set(nx_u, below_y, cur);
                        self.grid.set(x, y, Particle::air());
                        return;
                    }
                    let diag_dens = density_for_id(diag.element_id);
                    if cur_dens > diag_dens + 0.5
                        && (is_liquid(diag.element_id) || is_gas(diag.element_id))
                    {
                        self.grid.set(nx_u, below_y, cur);
                        self.grid.set(x, y, diag);
                        return;
                    }
                }
            }
            if is_liquid(cur.element_id) || cur_kind == ElementKind::Sand {
                let dirs = if rng.bool() { [-1, 1] } else { [1, -1] };
                for dx in dirs {
                    let nx = x as i32 + dx;
                    if nx < 0 || nx >= w as i32 {
                        continue;
                    }
                    let nx_u = nx as u32;
                    if self.grid.get(nx_u, y).is_some_and(|p| p.is_empty()) {
                        self.grid.set(nx_u, y, cur);
                        self.grid.set(x, y, Particle::air());
                        return;
                    }
                }
            }
        }
    }

    fn apply_radiation_movement(&mut self, x: u32, y: u32, rng: &mut fastrand::Rng) {
        let p = *self.grid.get(x, y).unwrap();
        let id = p.element_id;
        if !is_radiation(id) {
            return;
        }
        let mut new_p = p;
        new_p.lifetime = new_p.lifetime.wrapping_add(1);
        let max_lt = match id {
            NEUTRON_THERMAL => 30,
            NEUTRON_FAST => 40,
            GAMMA => 20,
            ALPHA => 8,
            BETA => 12,
            _ => 10,
        };
        if new_p.lifetime > max_lt {
            self.grid.set(x, y, Particle::air());
            return;
        }
        *self.grid.get_mut(x, y).unwrap() = new_p;

        let moves = match id {
            NEUTRON_FAST => 2,
            NEUTRON_THERMAL => 1,
            GAMMA => 3,
            _ => 1,
        };
        let mut cx = x as i32;
        let mut cy = y as i32;
        for _ in 0..moves {
            let dx = rng.i32(-1..=1);
            let dy = rng.i32(-1..=1);
            let nx = cx + dx;
            let ny = cy + dy;
            if !self.grid.in_bounds(nx, ny) {
                self.grid.set(cx as u32, cy as u32, Particle::air());
                return;
            }
            let target = *self.grid.get(nx as u32, ny as u32).unwrap();
            if target.is_empty() {
                let cur = *self.grid.get(cx as u32, cy as u32).unwrap();
                self.grid.set(nx as u32, ny as u32, cur);
                self.grid.set(cx as u32, cy as u32, Particle::air());
                cx = nx;
                cy = ny;
            } else {
                let pen = penetration_depth(id);
                if pen > 0 && rng.u32(0..pen + 1) > 0 {
                    if let Some(tgt) = self.grid.get_mut(nx as u32, ny as u32) {
                        tgt.temperature = tgt.temperature.saturating_add(match id {
                            GAMMA => 5,
                            NEUTRON_FAST => 15,
                            NEUTRON_THERMAL => 8,
                            _ => 2,
                        });
                    }
                    if id == GAMMA && rng.f32() < 0.7 {
                        continue;
                    }
                }
                break;
            }
        }
    }

    fn reaction_pass(&mut self, rng: &mut fastrand::Rng) {
        let w = self.grid.width;
        let h = self.grid.height;
        let mut fissile_to_check: Vec<(u32, u32)> = Vec::new();
        let mut decay_to_check: Vec<(u32, u32)> = Vec::new();
        let mut fusion_pairs: Vec<(u32, u32, u32, u32)> = Vec::new();

        for y in 0..h {
            for x in 0..w {
                let id = self.grid.get(x, y).unwrap().element_id;
                if is_fissile(id) {
                    fissile_to_check.push((x, y));
                }
                if matches!(id, U235 | U238 | PU239 | PU240 | TRITIUM) {
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
                                fusion_pairs.push((x, y, nx as u32, ny as u32));
                            }
                        }
                    }
                }
            }
        }

        for (x, y) in fissile_to_check {
            let cur = *self.grid.get(x, y).unwrap();
            if cur.is_empty() {
                continue;
            }
            let mut has_neutron = false;
            let mut neutron_energy = NeutronEnergy::Thermal;
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
                    if nid == NEUTRON_THERMAL {
                        has_neutron = true;
                        neutron_energy = NeutronEnergy::Thermal;
                        break;
                    } else if nid == NEUTRON_FAST {
                        has_neutron = true;
                        neutron_energy = NeutronEnergy::Fast;
                    }
                }
                if has_neutron {
                    break;
                }
            }
            if has_neutron {
                let prob =
                    Self::fission_probability(cur.element_id, neutron_energy, cur.temperature);
                if rng.f32() < prob {
                    self.trigger_fission(x, y, rng);
                }
            } else if rng.f32() < 0.00001 {
                self.trigger_fission(x, y, rng);
            }
        }

        for (x1, y1, x2, y2) in fusion_pairs {
            let p1 = *self.grid.get(x1, y1).unwrap();
            let p2 = *self.grid.get(x2, y2).unwrap();
            if p1.temperature > self.settings.fusion_threshold
                && p2.temperature > self.settings.fusion_threshold
                && rng.f32() < 0.05
            {
                self.trigger_fusion(x1, y1, x2, y2, rng);
            }
        }

        for (x, y) in decay_to_check {
            if let Some(p) = self.grid.get(x, y).copied() {
                let half_life = Self::half_life_ticks(p.element_id);
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

    fn effects_pass(&mut self, rng: &mut fastrand::Rng) {
        let w = self.grid.width;
        let h = self.grid.height;
        let mut new_temps = vec![0u16; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                let idx = self.grid.index(x, y);
                let cur_temp = self.grid.particles[idx].temperature as f32;
                let mut sum = cur_temp;
                let mut count = 1.0;
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
                        let nidx = self.grid.index(nx as u32, ny as u32);
                        let ntemp = self.grid.particles[nidx].temperature as f32;
                        sum += ntemp;
                        count += 1.0;
                    }
                }
                let avg = sum / count;
                let diffused =
                    cur_temp + (avg - cur_temp) * self.settings.temperature_diffusion_rate;
                let cooled = diffused * 0.999 + 293.0 * 0.001;
                new_temps[idx] = cooled.clamp(0.0, 5000.0) as u16;
            }
        }
        for (i, temp) in new_temps.into_iter().enumerate() {
            self.grid.particles[i].temperature = temp;
        }

        for y in 0..h {
            for x in 0..w {
                let p = *self.grid.get(x, y).unwrap();
                if p.is_empty() {
                    continue;
                }
                if p.temperature > 2000 && is_fissile(p.element_id) && rng.f32() < 0.01 {
                    self.grid
                        .set(x, y, Particle::new(MOLTEN_FUEL, p.temperature));
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let nx = x as i32 + dx;
                            let ny = y as i32 + dy;
                            if !self.grid.in_bounds(nx, ny) {
                                continue;
                            }
                            if let Some(n) = self.grid.get_mut(nx as u32, ny as u32) {
                                n.temperature = n.temperature.saturating_add(100);
                            }
                        }
                    }
                }
                if p.temperature > 2500
                    && matches!(p.element_id, WATER | HEAVY_WATER)
                    && rng.f32() < 0.05
                {
                    self.grid.set(x, y, Particle::new(HYDROGEN, p.temperature));
                }
                if p.element_id == TNT && p.temperature > 500 {
                    self.trigger_tnt(x, y, rng);
                }
            }
        }
    }

    fn trigger_fission(&mut self, x: u32, y: u32, rng: &mut fastrand::Rng) {
        let orig = *self.grid.get(x, y).unwrap();
        self.grid.set(
            x,
            y,
            Particle::new(FISSION_PRODUCTS, orig.temperature.saturating_add(500)),
        );
        self.fission_count += 1;
        self.reaction_count += 1;

        let n_count = rng.u32(2..=3);
        for _ in 0..n_count {
            let dx = rng.i32(-2..=2);
            let dy = rng.i32(-2..=2);
            let nx = (x as i32 + dx).clamp(0, self.grid.width as i32 - 1) as u32;
            let ny = (y as i32 + dy).clamp(0, self.grid.height as i32 - 1) as u32;
            self.neutron_queue.push_back(NeutronEvent {
                x: nx,
                y: ny,
                delay: rng.u8(1..=3),
                energy: NeutronEnergy::Fast,
            });
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
                if let Some(n) = self.grid.get_mut(nx as u32, ny as u32) {
                    if !n.is_empty() {
                        n.temperature = n.temperature.saturating_add(rng.u16(50..200));
                    }
                }
            }
        }
    }

    fn trigger_fusion(&mut self, x1: u32, y1: u32, x2: u32, y2: u32, rng: &mut fastrand::Rng) {
        self.grid.set(x1, y1, Particle::new(HELIUM, 3000));
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
                if let Some(n) = self.grid.get_mut(nx as u32, ny as u32) {
                    n.temperature = n.temperature.saturating_add(800);
                }
            }
        }
    }

    fn trigger_decay(&mut self, x: u32, y: u32, rng: &mut fastrand::Rng) {
        let p = *self.grid.get(x, y).unwrap();
        let daughter = Self::decay_daughter(p.element_id);
        let radiation = Self::decay_radiation(p.element_id);
        self.grid.set(x, y, Particle::new(daughter, p.temperature));
        self.decay_count += 1;

        let dx = rng.i32(-1..=1);
        let dy = rng.i32(-1..=1);
        let nx = (x as i32 + dx).clamp(0, self.grid.width as i32 - 1) as u32;
        let ny = (y as i32 + dy).clamp(0, self.grid.height as i32 - 1) as u32;
        if self.grid.get(nx, ny).unwrap().is_empty() {
            self.grid
                .set(nx, ny, Particle::new(radiation, 400).with_lifetime(0));
        }
    }

    fn trigger_tnt(&mut self, x: u32, y: u32, rng: &mut fastrand::Rng) {
        let radius = 6;
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
                } else if let Some(n) = self.grid.get_mut(nx as u32, ny as u32) {
                    n.temperature = n.temperature.saturating_add(300);
                }
            }
        }
        self.grid.set(x, y, Particle::new(FALLOUT, 800));
    }

    fn fission_probability(element_id: u16, energy: NeutronEnergy, temp: u16) -> f32 {
        let base = match element_id {
            U235 => match energy {
                NeutronEnergy::Thermal => 0.85,
                NeutronEnergy::Fast => 0.35,
            },
            PU239 => match energy {
                NeutronEnergy::Thermal => 0.90,
                NeutronEnergy::Fast => 0.40,
            },
            U238 => match energy {
                NeutronEnergy::Thermal => 0.02,
                NeutronEnergy::Fast => 0.25,
            },
            PU240 => match energy {
                NeutronEnergy::Thermal => 0.10,
                NeutronEnergy::Fast => 0.30,
            },
            _ => 0.0,
        };
        let temp_factor = 1.0 + ((temp as f32 - 293.0) / 1000.0).clamp(-0.5, 1.0);
        (base * temp_factor).clamp(0.0, 0.95)
    }

    fn half_life_ticks(element_id: u16) -> u64 {
        match element_id {
            U235 => 1_000_000,
            U238 => 2_000_000,
            PU239 => 500_000,
            PU240 => 400_000,
            TRITIUM => 100_000,
            DEUTERIUM => 0,
            _ => 0,
        }
    }

    fn decay_daughter(element_id: u16) -> u16 {
        match element_id {
            U235 => FISSION_PRODUCTS,
            U238 => DEPLETED_URANIUM,
            PU239 => U235,
            PU240 => PU239,
            TRITIUM => HELIUM,
            _ => FISSION_PRODUCTS,
        }
    }

    fn decay_radiation(element_id: u16) -> u16 {
        match element_id {
            U235 | U238 => ALPHA,
            PU239 | PU240 => ALPHA,
            TRITIUM => BETA,
            _ => GAMMA,
        }
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
