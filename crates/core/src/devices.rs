//! Heaters, pumps, fire, acid, wires, pressure and control rods.

use crate::element_id::*;
use crate::grid::Grid;
use crate::particle::Particle;
use crate::physics::VelocityField;

#[derive(Clone, Debug, Default)]
pub struct PressureField {
    pub p: Vec<u16>,
}

impl PressureField {
    pub fn new(len: usize) -> Self {
        Self { p: vec![0; len] }
    }
    pub fn sync_len(&mut self, len: usize) {
        if self.p.len() != len {
            self.p.resize(len, 0);
        }
    }
}

pub fn step_devices(
    grid: &mut Grid,
    vel: &mut VelocityField,
    pressure: &mut PressureField,
    rng: &mut fastrand::Rng,
    k_eff: f32,
) {
    let w = grid.width;
    let h = grid.height;
    let len = grid.particles.len();
    pressure.sync_len(len);
    vel.sync_len(len);

    // Snapshot so we don't chain-react the whole grid in one pass.
    let ids: Vec<u16> = grid.particles.iter().map(|p| p.element_id).collect();
    let lives: Vec<u8> = grid.particles.iter().map(|p| p.lifetime).collect();

    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            match ids[i] {
                HEATER => heat_around(grid, x, y, 18),
                PUMP => pump_fluid(grid, vel, x, y),
                FIRE => {
                    pressure.p[i] = pressure.p[i].saturating_add(3);
                    tick_fire(grid, pressure, x, y, rng);
                }
                ACID => tick_acid(grid, x, y, rng),
                WOOD | COAL => {
                    if grid.particles[i].temperature > 650 && rng.f32() < 0.08 {
                        grid.set(x, y, Particle::new(FIRE, grid.particles[i].temperature));
                    }
                }
                HYDROGEN => {
                    if grid.particles[i].temperature > 750 && rng.f32() < 0.04 {
                        grid.set(x, y, Particle::new(FIRE, 1200));
                        pressure.p[i] = pressure.p[i].saturating_add(20);
                    }
                }
                SPARK => tick_spark(grid, x, y, rng),
                WIRE => tick_wire(grid, vel, x, y, lives[i], rng),
                SENSOR => tick_sensor(grid, x, y, k_eff, rng),
                FILTER => tick_filter(grid, vel, x, y),
                CONTROL_ROD => {
                    if grid.particles[i].temperature > 1600 && rng.f32() < 0.06 {
                        grid.set(x, y, Particle::new(SLAG, grid.particles[i].temperature));
                    }
                }
                STEAM => {
                    pressure.p[i] = pressure.p[i].saturating_add(6);
                }
                _ => {}
            }
        }
    }

    diffuse_pressure(pressure, w, h);
    apply_pressure_flow(grid, vel, pressure, rng);
    apply_overpressure(grid, pressure, rng);
}

fn heat_around(grid: &mut Grid, x: u32, y: u32, delta: u16) {
    for dy in -1..=1_i32 {
        for dx in -1..=1_i32 {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if let Some(n) = grid
                .in_bounds(nx, ny)
                .then(|| grid.get_mut(nx as u32, ny as u32))
                .flatten()
            {
                n.temperature = n.temperature.saturating_add(delta);
            }
        }
    }
}

fn pump_fluid(grid: &mut Grid, vel: &mut VelocityField, x: u32, y: u32) {
    // Push fluid from any side through to the opposite empty / fluid cell.
    for (dx, dy) in [(0i32, 1), (0, -1), (1, 0), (-1, 0)] {
        let fx = x as i32 + dx;
        let fy = y as i32 + dy;
        let tx = x as i32 - dx;
        let ty = y as i32 - dy;
        if !grid.in_bounds(fx, fy) || !grid.in_bounds(tx, ty) {
            continue;
        }
        let from = *grid.get(fx as u32, fy as u32).unwrap();
        let to = *grid.get(tx as u32, ty as u32).unwrap();
        if !is_fluid(from.element_id) {
            continue;
        }
        if !(to.is_empty() || is_fluid(to.element_id)) {
            continue;
        }
        let ia = grid.index(fx as u32, fy as u32);
        let ib = grid.index(tx as u32, ty as u32);
        grid.particles.swap(ia, ib);
        if vel.vx.len() == grid.particles.len() {
            vel.vx.swap(ia, ib);
            vel.vy.swap(ia, ib);
            vel.vx[ib] = (-dx as i8).clamp(-2, 2);
            vel.vy[ib] = (-dy as i8).clamp(-2, 2);
        }
        return;
    }
}

fn tick_fire(
    grid: &mut Grid,
    pressure: &mut PressureField,
    x: u32,
    y: u32,
    rng: &mut fastrand::Rng,
) {
    let p = *grid.get(x, y).unwrap();
    let life = p.lifetime.saturating_add(1);
    let mut air = 0u32;
    let mut wet = false;
    let mut fuel_h2 = false;
    for dy in -1..=1_i32 {
        for dx in -1..=1_i32 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if !grid.in_bounds(nx, ny) {
                continue;
            }
            let n = *grid.get(nx as u32, ny as u32).unwrap();
            if n.is_empty() {
                air += 1;
            } else if matches!(n.element_id, WATER | HEAVY_WATER | STEAM) {
                wet = true;
            } else if n.element_id == HYDROGEN {
                fuel_h2 = true;
            }
        }
    }
    if wet && rng.f32() < 0.45 {
        grid.set(x, y, Particle::air());
        return;
    }
    if air == 0 && rng.f32() < 0.55 {
        // Smothered — no free oxygen (empty cells).
        grid.set(x, y, Particle::air());
        return;
    }
    if life > 24 || rng.f32() < 0.08 {
        grid.set(x, y, Particle::air());
        return;
    }
    if let Some(c) = grid.get_mut(x, y) {
        c.lifetime = life;
        c.temperature = c.temperature.saturating_add(12);
    }
    if fuel_h2 {
        let i = grid.index(x, y);
        if i < pressure.p.len() {
            pressure.p[i] = pressure.p[i].saturating_add(18);
        }
    }
    for dy in -1..=1_i32 {
        for dx in -1..=1_i32 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if !grid.in_bounds(nx, ny) {
                continue;
            }
            let n = *grid.get(nx as u32, ny as u32).unwrap();
            if n.element_id == HYDROGEN && rng.f32() < 0.55 {
                grid.set(nx as u32, ny as u32, Particle::new(FIRE, 1300).with_lifetime(0));
                continue;
            }
            if is_flammable(n.element_id) && rng.f32() < 0.18 {
                if n.element_id == TNT {
                    if let Some(t) = grid.get_mut(nx as u32, ny as u32) {
                        t.temperature = t.temperature.saturating_add(80);
                    }
                } else {
                    grid.set(nx as u32, ny as u32, Particle::new(FIRE, 1100).with_lifetime(0));
                }
            } else if n.is_empty() && air > 0 && rng.f32() < 0.04 {
                grid.set(nx as u32, ny as u32, Particle::new(FIRE, 900).with_lifetime(0));
            } else if let Some(t) = grid.get_mut(nx as u32, ny as u32) {
                t.temperature = t.temperature.saturating_add(8);
            }
        }
    }
}

fn tick_acid(grid: &mut Grid, x: u32, y: u32, rng: &mut fastrand::Rng) {
    for dy in -1..=1_i32 {
        for dx in -1..=1_i32 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if !grid.in_bounds(nx, ny) {
                continue;
            }
            let id = grid.get(nx as u32, ny as u32).unwrap().element_id;
            let eats = matches!(id, STONE | CONCRETE | STEEL | WOOD | ICE | PIPE | FILTER);
            if eats && rng.f32() < 0.08 {
                grid.set(nx as u32, ny as u32, Particle::new(SLAG, 350));
            }
        }
    }
}

fn tick_spark(grid: &mut Grid, x: u32, y: u32, rng: &mut fastrand::Rng) {
    let p = *grid.get(x, y).unwrap();
    if p.lifetime > 4 {
        grid.set(x, y, Particle::air());
        return;
    }
    if let Some(c) = grid.get_mut(x, y) {
        c.lifetime = c.lifetime.saturating_add(1);
    }
    // Charge neighbouring wire — the pulse then walks along the cable.
    for dy in -1..=1_i32 {
        for dx in -1..=1_i32 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if !grid.in_bounds(nx, ny) {
                continue;
            }
            if let Some(n) = grid.get_mut(nx as u32, ny as u32) {
                if n.element_id == WIRE && n.lifetime == 0 {
                    n.lifetime = 8;
                    n.temperature = n.temperature.saturating_add(20);
                }
            }
        }
    }
    if rng.f32() < 0.04 {
        // occasional stray spark into air
        for dy in -1..=1_i32 {
            for dx in -1..=1_i32 {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if grid.in_bounds(nx, ny) && grid.get(nx as u32, ny as u32).unwrap().is_empty() {
                    grid.set(
                        nx as u32,
                        ny as u32,
                        Particle::new(SPARK, 800).with_lifetime(0),
                    );
                    break;
                }
            }
        }
    }
}

fn tick_wire(
    grid: &mut Grid,
    vel: &mut VelocityField,
    x: u32,
    y: u32,
    snap_life: u8,
    rng: &mut fastrand::Rng,
) {
    if snap_life == 0 {
        return;
    }
    // Powered: hop the pulse to unpowered neighbours, then drive devices.
    for dy in -1..=1_i32 {
        for dx in -1..=1_i32 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if !grid.in_bounds(nx, ny) {
                continue;
            }
            let n = *grid.get(nx as u32, ny as u32).unwrap();
            match n.element_id {
                WIRE if n.lifetime == 0 => {
                    if let Some(w) = grid.get_mut(nx as u32, ny as u32) {
                        w.lifetime = snap_life.saturating_sub(1).max(1);
                        w.temperature = w.temperature.saturating_add(12);
                    }
                }
                HEATER => {
                    heat_around(grid, nx as u32, ny as u32, 30);
                }
                PUMP => pump_fluid(grid, vel, nx as u32, ny as u32),
                TNT => {
                    if let Some(t) = grid.get_mut(nx as u32, ny as u32) {
                        t.temperature = t.temperature.saturating_add(120);
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(w) = grid.get_mut(x, y) {
        w.lifetime = snap_life.saturating_sub(1);
        w.temperature = w.temperature.saturating_add(6);
    }
    let _ = rng;
}

fn tick_sensor(grid: &mut Grid, x: u32, y: u32, k_eff: f32, rng: &mut fastrand::Rng) {
    // Temperature is the analog readout; a spark is the trip contact.
    let mut flux = 0u16;
    for dy in -2..=2_i32 {
        for dx in -2..=2_i32 {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if !grid.in_bounds(nx, ny) {
                continue;
            }
            let id = grid.get(nx as u32, ny as u32).unwrap().element_id;
            if is_radiation(id) || matches!(id, XENON | FIRE | IODINE) {
                flux = flux.saturating_add(1);
            }
        }
    }
    let t = 293u16
        .saturating_add(flux.saturating_mul(45))
        .saturating_add((k_eff * 220.0) as u16);
    if let Some(s) = grid.get_mut(x, y) {
        s.temperature = t;
    }
    let tripped = k_eff > 1.05 || flux >= 3;
    if tripped && rng.f32() < 0.25 {
        for dy in -1..=1_i32 {
            for dx in -1..=1_i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if !grid.in_bounds(nx, ny) {
                    continue;
                }
                let n = *grid.get(nx as u32, ny as u32).unwrap();
                if n.is_empty() {
                    grid.set(
                        nx as u32,
                        ny as u32,
                        Particle::new(SPARK, 800).with_lifetime(0),
                    );
                    return;
                }
                if n.element_id == WIRE && n.lifetime == 0 {
                    if let Some(w) = grid.get_mut(nx as u32, ny as u32) {
                        w.lifetime = 8;
                    }
                    return;
                }
            }
        }
    }
}

fn tick_filter(grid: &mut Grid, vel: &mut VelocityField, x: u32, y: u32) {
    // Pass fluids through; powders and solids stay put.
    for (dx, dy) in [(0i32, 1), (0, -1), (1, 0), (-1, 0)] {
        let fx = x as i32 + dx;
        let fy = y as i32 + dy;
        let tx = x as i32 - dx;
        let ty = y as i32 - dy;
        if !grid.in_bounds(fx, fy) || !grid.in_bounds(tx, ty) {
            continue;
        }
        let from = *grid.get(fx as u32, fy as u32).unwrap();
        let to = *grid.get(tx as u32, ty as u32).unwrap();
        if !is_fluid(from.element_id) {
            continue;
        }
        if !(to.is_empty() || (is_fluid(to.element_id) && to.element_id == from.element_id)) {
            continue;
        }
        let ia = grid.index(fx as u32, fy as u32);
        let ib = grid.index(tx as u32, ty as u32);
        grid.particles.swap(ia, ib);
        if vel.vx.len() == grid.particles.len() {
            vel.vx.swap(ia, ib);
            vel.vy.swap(ia, ib);
        }
        return;
    }
}

fn diffuse_pressure(pressure: &mut PressureField, w: u32, h: u32) {
    let mut next = pressure.p.clone();
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let mut acc = pressure.p[i] as u32;
            let mut n = 1u32;
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                acc += pressure.p[(ny as u32 * w + nx as u32) as usize] as u32;
                n += 1;
            }
            next[i] = ((acc / n) as u16).saturating_sub(1);
        }
    }
    pressure.p = next;
}

/// High-pressure fluid shoves into a lower-pressure neighbour.
fn apply_pressure_flow(
    grid: &mut Grid,
    vel: &mut VelocityField,
    pressure: &PressureField,
    rng: &mut fastrand::Rng,
) {
    let w = grid.width;
    let h = grid.height;
    if pressure.p.len() != grid.particles.len() {
        return;
    }
    vel.sync_len(grid.particles.len());
    let mut xs: Vec<u32> = (0..w).collect();
    for y in 0..h {
        if rng.bool() {
            xs.reverse();
        }
        for &x in &xs {
            let i = (y * w + x) as usize;
            let src_p = pressure.p[i];
            if src_p < 12 {
                continue;
            }
            let id = grid.particles[i].element_id;
            if !(is_fluid(id) || is_powder(id)) {
                continue;
            }
            if grid.particles[i].has_flag(Particle::FLAG_MOVED) {
                continue;
            }
            let mut best = None;
            let mut best_dp = 8u16;
            for (dx, dy) in [(0i32, -1), (0, 1), (-1, 0), (1, 0)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if !grid.in_bounds(nx, ny) {
                    continue;
                }
                let j = grid.index(nx as u32, ny as u32);
                let dest = grid.particles[j];
                let dp = src_p.saturating_sub(pressure.p[j]);
                if dp < best_dp {
                    continue;
                }
                let can = dest.is_empty()
                    || (is_fluid(dest.element_id)
                        && density_for_id(id) >= density_for_id(dest.element_id));
                if can {
                    best_dp = dp;
                    best = Some((nx as u32, ny as u32, dx, dy));
                }
            }
            if let Some((nx, ny, dx, dy)) = best {
                if rng.f32() < (best_dp as f32 / 80.0).clamp(0.15, 0.9) {
                    let ia = grid.index(x, y);
                    let ib = grid.index(nx, ny);
                    grid.particles.swap(ia, ib);
                    vel.vx.swap(ia, ib);
                    vel.vy.swap(ia, ib);
                    vel.vx[ib] = dx as i8;
                    vel.vy[ib] = dy as i8;
                    grid.particles[ib].set_flag(Particle::FLAG_MOVED);
                }
            }
        }
    }
}

fn apply_overpressure(grid: &mut Grid, pressure: &PressureField, rng: &mut fastrand::Rng) {
    let w = grid.width;
    let h = grid.height;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if pressure.p[i] < 90 {
                continue;
            }
            let id = grid.particles[i].element_id;
            if matches!(id, PIPE | ICE | WOOD) && rng.f32() < 0.12 {
                grid.set(x, y, Particle::air());
            }
        }
    }
}

/// Move every CONTROL_ROD cell by `dy` (−1 raise, +1 insert) if the dest is empty.
pub fn shift_control_rods(grid: &mut Grid, dy: i32) {
    if dy == 0 {
        return;
    }
    let w = grid.width;
    let h = grid.height;
    let mut rods = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if grid.get(x, y).unwrap().element_id == CONTROL_ROD {
                rods.push((x, y, *grid.get(x, y).unwrap()));
            }
        }
    }
    if dy > 0 {
        rods.sort_by_key(|r| std::cmp::Reverse(r.1));
    } else {
        rods.sort_by_key(|r| r.1);
    }
    for (x, y, p) in rods {
        let ny = y as i32 + dy;
        if !grid.in_bounds(x as i32, ny) {
            continue;
        }
        if grid.get(x, ny as u32).unwrap().is_empty() {
            grid.set(x, ny as u32, p);
            grid.set(x, y, Particle::air());
        }
    }
}
