//! Large-scale hydrodynamics on top of the cell CA:
//! connected-surface leveling, hydrostatic pressure, hollow pipes, overburden.

use crate::element_id::*;
use crate::grid::Grid;
use crate::particle::Particle;
use crate::physics::VelocityField;
use crate::devices::PressureField;

/// Depth-scaled pressure for every liquid column. Sealed walls still block diffusion.
pub fn add_hydrostatic_pressure(grid: &Grid, pressure: &mut PressureField) {
    let w = grid.width;
    let h = grid.height;
    pressure.sync_len(grid.particles.len());
    for x in 0..w {
        let mut depth = 0u16;
        for y in (0..h).rev() {
            let i = grid.index(x, y);
            let id = grid.particles[i].element_id;
            if is_liquid(id) || id == PIPE_WATER {
                depth = depth.saturating_add(1);
                let add = (depth.saturating_mul(3)).min(40);
                pressure.p[i] = pressure.p[i].saturating_add(add);
            } else if is_static_solid(id) && !is_pipe(id) {
                depth = 0;
            } else if id == AIR || is_gas(id) {
                depth = 0;
            }
        }
    }
}

/// Move free-surface liquid cells toward a lower neighboring column.
/// This is what makes a lake actually find a level on a large grid.
pub fn equalize_liquid_surface(
    grid: &mut Grid,
    vel: &mut VelocityField,
    rng: &mut fastrand::Rng,
) {
    let w = grid.width;
    let h = grid.height;
    vel.sync_len(grid.particles.len());
    let mut xs: Vec<u32> = (0..w).collect();
    if rng.bool() {
        xs.reverse();
    }
    for y in 0..h {
        for &x in &xs {
            let i = grid.index(x, y);
            let id = grid.particles[i].element_id;
            if !is_liquid(id) || grid.particles[i].has_flag(Particle::FLAG_MOVED) {
                continue;
            }
            // Surface: empty / gas above, or top of the map.
            let open_above = y == 0
                || grid
                    .get(x, y - 1)
                    .is_some_and(|p| p.is_empty() || is_gas(p.element_id));
            if !open_above {
                continue;
            }
            let look = flow_steps(id).max(2).min(8);
            let mut best: Option<(u32, u32, i32)> = None;
            for sign in rand_lr(rng) {
                for step in 1..=look {
                    let nx = x as i32 + sign * step as i32;
                    if !grid.in_bounds(nx, y as i32) {
                        break;
                    }
                    let nid = grid.get(nx as u32, y).unwrap().element_id;
                    if is_static_solid(nid) && !is_pipe(nid) {
                        break;
                    }
                    // Prefer a hole one cell down, else same-level empty.
                    if y + 1 < h {
                        let below = grid.get(nx as u32, y + 1).unwrap();
                        if below.is_empty() {
                            best = Some((nx as u32, y + 1, sign));
                            break;
                        }
                    }
                    if grid.get(nx as u32, y).unwrap().is_empty() && best.is_none() {
                        best = Some((nx as u32, y, sign));
                    }
                }
                if best.is_some() {
                    break;
                }
            }
            if let Some((nx, ny, dir)) = best {
                if rng.f32() < 0.85 {
                    let ia = grid.index(x, y);
                    let ib = grid.index(nx, ny);
                    grid.particles.swap(ia, ib);
                    vel.vx.swap(ia, ib);
                    vel.vy.swap(ia, ib);
                    vel.vx[ib] = dir as i8;
                    grid.particles[ib].set_flag(Particle::FLAG_MOVED);
                }
            }
        }
    }
}

fn rand_lr(rng: &mut fastrand::Rng) -> [i32; 2] {
    if rng.bool() {
        [-1, 1]
    } else {
        [1, -1]
    }
}

/// Ingest / hop / eject fluid inside PIPE cells so a long run behaves like a duct.
pub fn step_pipe_network(
    grid: &mut Grid,
    vel: &mut VelocityField,
    pressure: &mut PressureField,
    rng: &mut fastrand::Rng,
) {
    let w = grid.width;
    let h = grid.height;
    vel.sync_len(grid.particles.len());
    pressure.sync_len(grid.particles.len());
    let ids: Vec<u16> = grid.particles.iter().map(|p| p.element_id).collect();

    for y in 0..h {
        for x in 0..w {
            let i = grid.index(x, y);
            match ids[i] {
                PIPE => ingest_pipe(grid, vel, pressure, x, y, rng),
                PIPE_WATER | PIPE_STEAM => {
                    if !eject_pipe(grid, vel, pressure, x, y, rng) {
                        hop_pipe(grid, vel, pressure, x, y, rng);
                    }
                }
                _ => {}
            }
        }
    }
}

fn ingest_pipe(
    grid: &mut Grid,
    vel: &mut VelocityField,
    pressure: &mut PressureField,
    x: u32,
    y: u32,
    rng: &mut fastrand::Rng,
) {
    // Only ingest at a "port": fluid neighbor + at least one pipe neighbor.
    let mut pipe_n = 0;
    let mut fluid: Option<(u32, u32, u16)> = None;
    for (dx, dy) in [(0i32, 1), (0, -1), (1, 0), (-1, 0)] {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if !grid.in_bounds(nx, ny) {
            continue;
        }
        let id = grid.get(nx as u32, ny as u32).unwrap().element_id;
        if is_pipe(id) {
            pipe_n += 1;
        } else if is_fluid(id) && fluid.is_none() {
            fluid = Some((nx as u32, ny as u32, id));
        }
    }
    if pipe_n == 0 || fluid.is_none() {
        return;
    }
    if rng.f32() > 0.40 {
        return;
    }
    let (fx, fy, fid) = fluid.unwrap();
    let fi = grid.index(fx, fy);
    let pi = grid.index(x, y);
    let t = grid.particles[fi].temperature;
    grid.set(x, y, Particle::new(pipe_with(fid), t));
    grid.set(fx, fy, Particle::air());
    if fi < vel.vx.len() && pi < vel.vx.len() {
        vel.vx[pi] = vel.vx[fi];
        vel.vy[pi] = vel.vy[fi];
        vel.vx[fi] = 0;
        vel.vy[fi] = 0;
    }
    if fi < pressure.p.len() && pi < pressure.p.len() {
        pressure.p[pi] = pressure.p[pi].saturating_add(pressure.p[fi] / 2 + 4);
    }
}

fn eject_pipe(
    grid: &mut Grid,
    vel: &mut VelocityField,
    pressure: &mut PressureField,
    x: u32,
    y: u32,
    rng: &mut fastrand::Rng,
) -> bool {
    let payload = match pipe_payload(grid.get(x, y).unwrap().element_id) {
        Some(p) => p,
        None => return false,
    };
    let t = grid.get(x, y).unwrap().temperature;
    // Prefer an opening (empty cell with few pipe neighbors) — a true outlet.
    let mut outlet = None;
    let mut fallback = None;
    for (dx, dy) in [(0i32, 1), (0, -1), (1, 0), (-1, 0)] {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if !grid.in_bounds(nx, ny) {
            continue;
        }
        let n = *grid.get(nx as u32, ny as u32).unwrap();
        if !n.is_empty() {
            continue;
        }
        let pipes_around = count_pipe_neighbors(grid, nx as u32, ny as u32);
        if pipes_around <= 1 {
            outlet = Some((nx as u32, ny as u32));
            break;
        }
        fallback = Some((nx as u32, ny as u32));
    }
    let dest = outlet.or(fallback);
    let Some((dx, dy)) = dest else {
        return false;
    };
    let force = pressure
        .p
        .get(grid.index(x, y))
        .copied()
        .unwrap_or(0);
    if outlet.is_none() && force < 18 && rng.f32() > 0.15 {
        return false;
    }
    grid.set(dx, dy, Particle::new(payload, t));
    grid.set(x, y, Particle::new(PIPE, t));
    let ib = grid.index(dx, dy);
    if ib < vel.vx.len() {
        vel.vy[ib] = if payload == STEAM { -1 } else { 1 };
    }
    true
}

fn hop_pipe(
    grid: &mut Grid,
    vel: &mut VelocityField,
    pressure: &mut PressureField,
    x: u32,
    y: u32,
    rng: &mut fastrand::Rng,
) {
    let src_id = grid.get(x, y).unwrap().element_id;
    let t = grid.get(x, y).unwrap().temperature;
    let dirs = if rng.bool() {
        [(1i32, 0), (-1, 0), (0, 1), (0, -1)]
    } else {
        [(-1, 0), (1, 0), (0, -1), (0, 1)]
    };
    for (dx, dy) in dirs {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if !grid.in_bounds(nx, ny) {
            continue;
        }
        if grid.get(nx as u32, ny as u32).unwrap().element_id != PIPE {
            continue;
        }
        if rng.f32() > 0.55 {
            continue;
        }
        let ia = grid.index(x, y);
        let ib = grid.index(nx as u32, ny as u32);
        grid.set(nx as u32, ny as u32, Particle::new(src_id, t));
        grid.set(x, y, Particle::new(PIPE, t));
        if ia < vel.vx.len() && ib < vel.vx.len() {
            vel.vx.swap(ia, ib);
            vel.vy.swap(ia, ib);
            vel.vx[ib] = dx as i8;
            vel.vy[ib] = dy as i8;
        }
        if ia < pressure.p.len() && ib < pressure.p.len() {
            let carry = pressure.p[ia] / 2;
            pressure.p[ib] = pressure.p[ib].saturating_add(carry);
            pressure.p[ia] = pressure.p[ia].saturating_sub(carry);
        }
        return;
    }
}

fn count_pipe_neighbors(grid: &Grid, x: u32, y: u32) -> u32 {
    let mut n = 0;
    for (dx, dy) in [(0i32, 1), (0, -1), (1, 0), (-1, 0)] {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if grid.in_bounds(nx, ny) && is_pipe(grid.get(nx as u32, ny as u32).unwrap().element_id) {
            n += 1;
        }
    }
    n
}

/// Tall powder columns develop a weak lateral failure (landslide), not a liquid run.
pub fn powder_overburden_slide(
    grid: &mut Grid,
    vel: &mut VelocityField,
    rng: &mut fastrand::Rng,
) {
    let w = grid.width;
    let h = grid.height;
    vel.sync_len(grid.particles.len());
    for y in (0..h).rev() {
        for x in 0..w {
            let i = grid.index(x, y);
            let id = grid.particles[i].element_id;
            if !is_powder(id) || grid.particles[i].has_flag(Particle::FLAG_MOVED) {
                continue;
            }
            let mut above = 0u32;
            let mut yy = y;
            while yy > 0 {
                yy -= 1;
                let nid = grid.get(x, yy).unwrap().element_id;
                if is_powder(nid) {
                    above += 1;
                } else {
                    break;
                }
                if above > 10 {
                    break;
                }
            }
            if above < 6 {
                continue;
            }
            // Rare, one cell sideways only — sand piles still hold a slope.
            if rng.f32() > 0.08 {
                continue;
            }
            let dir = if rng.bool() { -1 } else { 1 };
            let nx = x as i32 + dir;
            if !grid.in_bounds(nx, y as i32) {
                continue;
            }
            if grid.get(nx as u32, y).unwrap().is_empty() {
                let ia = grid.index(x, y);
                let ib = grid.index(nx as u32, y);
                grid.particles.swap(ia, ib);
                vel.vx.swap(ia, ib);
                vel.vy.swap(ia, ib);
                vel.vx[ib] = dir as i8;
                grid.particles[ib].set_flag(Particle::FLAG_MOVED);
            }
        }
    }
}
