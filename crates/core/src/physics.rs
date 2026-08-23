//! Cellular material physics: powders, liquids, gases, heat, knockback.
//!
//! Designed so walls stay put, sand piles, water finds a level, and hot gas rises.

use crate::element_id::*;
use crate::grid::Grid;
use crate::particle::Particle;
use crate::reactions;

/// Per-cell velocity, parallel to `Grid::particles`. Not stored in the particle
/// itself so old save files stay compatible.
#[derive(Clone, Debug)]
pub struct VelocityField {
    pub vx: Vec<i8>,
    pub vy: Vec<i8>,
}

impl Default for VelocityField {
    fn default() -> Self {
        Self {
            vx: Vec::new(),
            vy: Vec::new(),
        }
    }
}

impl VelocityField {
    pub fn new(len: usize) -> Self {
        Self {
            vx: vec![0; len],
            vy: vec![0; len],
        }
    }

    pub fn sync_len(&mut self, len: usize) {
        if self.vx.len() != len {
            self.vx.resize(len, 0);
            self.vy.resize(len, 0);
        }
    }

    #[inline]
    fn get(&self, i: usize) -> (i8, i8) {
        (self.vx[i], self.vy[i])
    }

    #[inline]
    fn set(&mut self, i: usize, vx: i8, vy: i8) {
        self.vx[i] = vx;
        self.vy[i] = vy;
    }
}

pub fn step(grid: &mut Grid, vel: &mut VelocityField, rng: &mut fastrand::Rng) {
    let len = grid.particles.len();
    vel.sync_len(len);
    for p in &mut grid.particles {
        p.clear_flag(Particle::FLAG_MOVED);
        p.clear_flag(Particle::FLAG_REACTED);
    }

    let w = grid.width;
    let h = grid.height;
    if w == 0 || h == 0 {
        return;
    }

    // Bottom-up so fallen grains don't get updated twice (FLAG_MOVED is the belt).
    let mut xs: Vec<u32> = (0..w).collect();
    for y in (0..h).rev() {
        shuffle_row(&mut xs, rng);
        for &x in &xs {
            update_cell(grid, vel, x, y, rng);
        }
    }
}

fn shuffle_row(xs: &mut [u32], rng: &mut fastrand::Rng) {
    if rng.bool() {
        xs.reverse();
        return;
    }
    for i in 0..xs.len() {
        let j = rng.usize(0..xs.len());
        xs.swap(i, j);
    }
}

fn update_cell(
    grid: &mut Grid,
    vel: &mut VelocityField,
    x: u32,
    y: u32,
    rng: &mut fastrand::Rng,
) {
    let Some(cur) = grid.get(x, y).copied() else {
        return;
    };
    if cur.is_empty() || cur.has_flag(Particle::FLAG_MOVED) {
        return;
    }
    let id = cur.element_id;
    if is_radiation(id) {
        move_radiation(grid, x, y, rng);
        return;
    }
    if is_static_solid(id) {
        let i = grid.index(x, y);
        vel.set(i, 0, 0);
        return;
    }
    if is_powder(id) {
        update_powder(grid, vel, x, y, rng);
        return;
    }
    if is_liquid(id) {
        update_liquid(grid, vel, x, y, rng);
        return;
    }
    if is_gas(id) {
        update_gas(grid, vel, x, y, rng);
    }
}

fn update_powder(
    grid: &mut Grid,
    vel: &mut VelocityField,
    x: u32,
    y: u32,
    rng: &mut fastrand::Rng,
) {
    let i = grid.index(x, y);
    let id = grid.particles[i].element_id;
    let (vx, mut vy) = vel.get(i);
    vy = (vy + 1).clamp(-3, max_fall_speed(id));
    vel.set(i, vx, vy);

    let mut cx = x;
    let mut cy = y;
    if vx != 0 {
        let nx = cx as i32 + vx.signum() as i32;
        if try_move_i(grid, vel, cx, cy, nx, cy as i32) {
            cx = nx as u32;
        }
        vel.set(grid.index(cx, cy), vx.saturating_sub(vx.signum()), vy);
    }
    for _ in 0..vy.max(0) {
        if try_move(grid, vel, cx, cy, cx, cy + 1) {
            cy += 1;
            continue;
        }
        if try_sink(grid, vel, cx, cy, rng) {
            cy += 1;
            continue;
        }
        let dirs = rand_lr(rng);
        let mut slid = false;
        for dx in dirs {
            if rng.f32() > repose_slide(id) {
                continue;
            }
            let nx = cx as i32 + dx;
            let ny = cy + 1;
            if try_move_i(grid, vel, cx, cy, nx, ny as i32)
                || try_sink_at(grid, vel, cx, cy, nx, ny as i32, rng)
            {
                cx = nx as u32;
                cy = ny;
                slid = true;
                break;
            }
        }
        if !slid {
            vel.set(grid.index(cx, cy), 0, 0);
            break;
        }
    }
}

fn update_liquid(
    grid: &mut Grid,
    vel: &mut VelocityField,
    x: u32,
    y: u32,
    rng: &mut fastrand::Rng,
) {
    let i = grid.index(x, y);
    let id = grid.particles[i].element_id;
    let temp = grid.particles[i].temperature;
    let (_, mut vy) = vel.get(i);
    vy = (vy + 1).clamp(1, max_fall_speed(id));
    vel.set(i, 0, vy);

    let mut cx = x;
    let mut cy = y;
    let mut falling = false;
    for _ in 0..vy {
        if try_move(grid, vel, cx, cy, cx, cy + 1) || try_sink(grid, vel, cx, cy, rng) {
            cy += 1;
            falling = true;
            continue;
        }
        let dirs = rand_lr(rng);
        let mut slid = false;
        for dx in dirs {
            let nx = cx as i32 + dx;
            if try_move_i(grid, vel, cx, cy, nx, (cy + 1) as i32)
                || try_sink_at(grid, vel, cx, cy, nx, (cy + 1) as i32, rng)
            {
                cx = nx as u32;
                cy += 1;
                slid = true;
                falling = true;
                break;
            }
        }
        if !slid {
            break;
        }
    }
    if falling {
        return;
    }
    vel.set(grid.index(cx, cy), 0, 0);

    // Hydrostatic spread: prefer the side whose next cell is "downhill".
    let steps = flow_steps(id);
    let dirs = prefer_downhill(grid, cx, cy, rng);
    for _ in 0..steps {
        let mut moved = false;
        for dx in dirs {
            let nx = cx as i32 + dx;
            if !grid.in_bounds(nx, cy as i32) {
                continue;
            }
            if try_move_i(grid, vel, cx, cy, nx, cy as i32)
                || try_sink_at(grid, vel, cx, cy, nx, cy as i32, rng)
            {
                cx = nx as u32;
                moved = true;
                break;
            }
        }
        if !moved {
            break;
        }
    }

    // Hot liquid convects upward into a colder fluid / air.
    if temp > 360 && rng.f32() < convection_chance(temp) {
        let _ = try_move(grid, vel, cx, cy, cx, cy.saturating_sub(1));
    }
}

fn update_gas(
    grid: &mut Grid,
    vel: &mut VelocityField,
    x: u32,
    y: u32,
    rng: &mut fastrand::Rng,
) {
    let i = grid.index(x, y);
    let id = grid.particles[i].element_id;
    let temp = grid.particles[i].temperature;
    let rise = rise_probability(id, temp);
    let dirs = rand_lr(rng);

    if rng.f32() < rise {
        if try_move(grid, vel, x, y, x, y.saturating_sub(1)) {
            return;
        }
        for dx in dirs {
            if try_move_i(grid, vel, x, y, x as i32 + dx, y as i32 - 1) {
                return;
            }
        }
        // Displace a heavier / colder gas above.
        if y > 0 {
            let above = grid.get(x, y - 1).copied().unwrap();
            if is_gas(above.element_id) && buoyancy(id, temp) > buoyancy(above.element_id, above.temperature)
                && rng.f32() < 0.7
            {
                swap_cells(grid, vel, x, y, x, y - 1);
                return;
            }
        }
    }

    let steps = flow_steps(id).max(1);
    let mut cx = x;
    let cy = y;
    for _ in 0..steps {
        let dx = if rng.bool() { -1 } else { 1 };
        if try_move_i(grid, vel, cx, cy, cx as i32 + dx, cy as i32) {
            cx = (cx as i32 + dx) as u32;
            continue;
        }
        break;
    }
}

fn rise_probability(id: u16, temp: u16) -> f32 {
    let base = match id {
        HYDROGEN => 0.85,
        STEAM | HELIUM => 0.75,
        TRITIUM | DEUTERIUM => 0.55,
        _ => 0.5,
    };
    let hot = ((temp as f32 - 293.0) / 1800.0).clamp(0.0, 0.25);
    (base + hot).clamp(0.15, 0.95)
}

fn buoyancy(id: u16, temp: u16) -> f32 {
    let d = density_for_id(id).max(0.001);
    (1.0 / d) * (1.0 + (temp as f32 - 293.0) / 2500.0)
}

fn convection_chance(temp: u16) -> f32 {
    ((temp as f32 - 360.0) / 2000.0).clamp(0.02, 0.35)
}

fn prefer_downhill(grid: &Grid, x: u32, y: u32, rng: &mut fastrand::Rng) -> [i32; 2] {
    let left = column_support(grid, x as i32 - 1, y);
    let right = column_support(grid, x as i32 + 1, y);
    if left < right {
        [-1, 1]
    } else if right < left {
        [1, -1]
    } else {
        rand_lr(rng)
    }
}

/// How "supported" a neighboring column is (lower = more empty space below).
fn column_support(grid: &Grid, x: i32, y: u32) -> i32 {
    if !grid.in_bounds(x, y as i32) {
        return 1000;
    }
    let xu = x as u32;
    if grid.get(xu, y).is_some_and(|p| !p.is_empty() && !is_fluid(p.element_id)) {
        return 50;
    }
    let mut score = 0;
    for dy in 1..=4 {
        let yy = y + dy;
        if yy >= grid.height {
            score += 2;
            break;
        }
        match grid.get(xu, yy) {
            Some(p) if p.is_empty() || is_gas(p.element_id) => score -= 3,
            Some(p) if is_liquid(p.element_id) => score -= 1,
            _ => {
                score += 2;
                break;
            }
        }
    }
    score
}

fn try_sink(grid: &mut Grid, vel: &mut VelocityField, x: u32, y: u32, rng: &mut fastrand::Rng) -> bool {
    try_sink_at(grid, vel, x, y, x as i32, y as i32 + 1, rng)
}

fn try_sink_at(
    grid: &mut Grid,
    vel: &mut VelocityField,
    x: u32,
    y: u32,
    nx: i32,
    ny: i32,
    rng: &mut fastrand::Rng,
) -> bool {
    if !grid.in_bounds(nx, ny) {
        return false;
    }
    let dest = *grid.get(nx as u32, ny as u32).unwrap();
    if dest.is_empty() || is_static_solid(dest.element_id) {
        return false;
    }
    if !is_fluid(dest.element_id) {
        return false;
    }
    let src = *grid.get(x, y).unwrap();
    let dd = density_for_id(src.element_id) - density_for_id(dest.element_id);
    if dd < 0.15 {
        return false;
    }
    // Heavier grains settle through fluids; chance scales with density gap.
    let p = (dd / 8.0).clamp(0.15, 0.95);
    if rng.f32() > p {
        return false;
    }
    swap_cells(grid, vel, x, y, nx as u32, ny as u32);
    true
}

fn try_move(grid: &mut Grid, vel: &mut VelocityField, x: u32, y: u32, nx: u32, ny: u32) -> bool {
    try_move_i(grid, vel, x, y, nx as i32, ny as i32)
}

fn try_move_i(
    grid: &mut Grid,
    vel: &mut VelocityField,
    x: u32,
    y: u32,
    nx: i32,
    ny: i32,
) -> bool {
    if !grid.in_bounds(nx, ny) {
        return false;
    }
    let dest = *grid.get(nx as u32, ny as u32).unwrap();
    if !dest.is_empty() || dest.has_flag(Particle::FLAG_MOVED) {
        return false;
    }
    swap_cells(grid, vel, x, y, nx as u32, ny as u32);
    true
}

fn swap_cells(grid: &mut Grid, vel: &mut VelocityField, ax: u32, ay: u32, bx: u32, by: u32) {
    let ia = grid.index(ax, ay);
    let ib = grid.index(bx, by);
    grid.particles.swap(ia, ib);
    vel.vx.swap(ia, ib);
    vel.vy.swap(ia, ib);
    grid.particles[ib].set_flag(Particle::FLAG_MOVED);
}

fn rand_lr(rng: &mut fastrand::Rng) -> [i32; 2] {
    if rng.bool() {
        [-1, 1]
    } else {
        [1, -1]
    }
}

fn move_radiation(grid: &mut Grid, x: u32, y: u32, rng: &mut fastrand::Rng) {
    let p = *grid.get(x, y).unwrap();
    let id = p.element_id;
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
        grid.set(x, y, Particle::air());
        return;
    }
    // Persist a preferred heading in unused flag bits so tracks are less Brownian.
    let (pdx, pdy) = heading_from_flags(new_p.flags);
    *grid.get_mut(x, y).unwrap() = new_p;

    let moves = match id {
        NEUTRON_FAST => 2,
        NEUTRON_THERMAL => 1,
        GAMMA => 3,
        _ => 1,
    };
    let mut cx = x as i32;
    let mut cy = y as i32;
    let mut last_dx = pdx;
    let mut last_dy = pdy;
    for _ in 0..moves {
        let (dx, dy) = if rng.f32() < 0.65 && (pdx != 0 || pdy != 0) {
            (pdx, pdy)
        } else {
            (rng.i32(-1..=1), rng.i32(-1..=1))
        };
        last_dx = dx;
        last_dy = dy;
        let nx = cx + dx;
        let ny = cy + dy;
        if !grid.in_bounds(nx, ny) {
            grid.set(cx as u32, cy as u32, Particle::air());
            return;
        }
        let target = *grid.get(nx as u32, ny as u32).unwrap();
        if target.is_empty() {
            let mut cur = *grid.get(cx as u32, cy as u32).unwrap();
            cur.flags = heading_to_flags(cur.flags, dx, dy);
            grid.set(nx as u32, ny as u32, cur);
            grid.set(cx as u32, cy as u32, Particle::air());
            cx = nx;
            cy = ny;
        } else {
            let pen = penetration_depth(id);
            // Dense shielding (lead, steel, concrete) absorbs more readily.
            let shield = density_for_id(target.element_id);
            let blocked = shield > 7.0 && rng.f32() < (shield / 20.0);
            if !blocked && pen > 0 && rng.u32(0..pen + 1) > 0 {
                if let Some(tgt) = grid.get_mut(nx as u32, ny as u32) {
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
    let _ = last_dx;
    let _ = last_dy;
}

fn heading_from_flags(flags: u8) -> (i32, i32) {
    if flags & (1 << 6) == 0 {
        return (0, 0);
    }
    let dx = (flags >> 2) & 0b11;
    let dy = (flags >> 4) & 0b11;
    (dx as i32 - 1, dy as i32 - 1)
}

fn heading_to_flags(flags: u8, dx: i32, dy: i32) -> u8 {
    let dxb = (dx.clamp(-1, 1) + 1) as u8;
    let dyb = (dy.clamp(-1, 1) + 1) as u8;
    (flags & 0b11) | (dxb << 2) | (dyb << 4) | (1 << 6)
}

/// Conductivity-weighted Jacobi heat step + ambient leak.
pub fn diffuse_heat(grid: &mut Grid, rate: f32) {
    let w = grid.width;
    let h = grid.height;
    let mut next = vec![0u16; grid.particles.len()];
    for y in 0..h {
        for x in 0..w {
            let idx = grid.index(x, y);
            let cur = grid.particles[idx];
            let k0 = conductivity(cur.element_id);
            let t0 = cur.temperature as f32;
            let mut acc = t0;
            let mut wsum = 1.0;
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if !grid.in_bounds(nx, ny) {
                    continue;
                }
                let n = grid.particles[grid.index(nx as u32, ny as u32)];
                let k = 0.5 * (k0 + conductivity(n.element_id));
                acc += n.temperature as f32 * k;
                wsum += k;
            }
            let mixed = acc / wsum;
            let diffused = t0 + (mixed - t0) * rate;
            let leak = if cur.is_empty() { 0.004 } else { 0.001 };
            let cooled = diffused * (1.0 - leak) + reactions::AMBIENT_TEMP as f32 * leak;
            next[idx] = cooled.clamp(0.0, 5000.0) as u16;
        }
    }
    for (i, t) in next.into_iter().enumerate() {
        grid.particles[i].temperature = t;
    }
}

/// Knock neighboring movable cells away from an explosion center.
pub fn apply_impulse(
    grid: &mut Grid,
    vel: &mut VelocityField,
    cx: u32,
    cy: u32,
    radius: i32,
    rng: &mut fastrand::Rng,
) {
    vel.sync_len(grid.particles.len());
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if !grid.in_bounds(nx, ny) {
                continue;
            }
            let xu = nx as u32;
            let yu = ny as u32;
            let id = grid.get(xu, yu).unwrap().element_id;
            if id == AIR || is_static_solid(id) || is_radiation(id) {
                continue;
            }
            let i = grid.index(xu, yu);
            let ivx = (dx.signum() as i8).saturating_mul(1 + rng.i32(0..=1) as i8);
            let ivy = (dy.signum() as i8).saturating_mul(1 + rng.i32(0..=1) as i8);
            vel.set(i, ivx, ivy);
        }
    }
}

/// Water / D2O / steam / ice phase changes.
pub fn apply_phase_changes(grid: &mut Grid, rng: &mut fastrand::Rng) {
    let w = grid.width;
    let h = grid.height;
    for y in 0..h {
        for x in 0..w {
            let p = *grid.get(x, y).unwrap();
            if p.is_empty() {
                continue;
            }
            match p.element_id {
                WATER => {
                    if p.temperature < 273 && rng.f32() < 0.15 {
                        grid.set(x, y, Particle::new(ICE, p.temperature));
                    } else if p.temperature > 373 && p.temperature <= reactions::BOIL_TEMP && rng.f32() < 0.20
                    {
                        grid.set(x, y, Particle::new(STEAM, p.temperature));
                    }
                }
                HEAVY_WATER => {
                    if p.temperature < 277 && rng.f32() < 0.12 {
                        grid.set(x, y, Particle::new(ICE, p.temperature));
                    } else if p.temperature > 375 && p.temperature <= reactions::BOIL_TEMP && rng.f32() < 0.18
                    {
                        grid.set(x, y, Particle::new(STEAM, p.temperature));
                    }
                }
                STEAM => {
                    if p.temperature < 368 && rng.f32() < 0.18 {
                        grid.set(x, y, Particle::new(WATER, p.temperature));
                    }
                }
                ICE => {
                    if p.temperature > 273 && rng.f32() < 0.20 {
                        grid.set(x, y, Particle::new(WATER, p.temperature));
                    }
                }
                _ => {}
            }
        }
    }
}
