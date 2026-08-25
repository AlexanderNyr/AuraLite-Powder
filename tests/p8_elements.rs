//! P8 "elements" gate: the two new liquids behave as declared — mercury (very
//! dense) sinks through water via the existing density-based `try_sink`, and
//! oil flows under gravity like any liquid. Both reuse existing mechanics.

use aura_lite_core::{element_id::*, Particle, SimulationState};

/// A narrow stone column so liquids can only move vertically.
fn column(width: u32, height: u32) -> SimulationState {
    let mut s = SimulationState::new(width, height, 1);
    let cx = width / 2;
    for y in 0..height {
        s.grid.set(cx - 1, y, Particle::new(STONE, 293));
        s.grid.set(cx + 1, y, Particle::new(STONE, 293));
    }
    for x in 0..width {
        s.grid.set(x, height - 1, Particle::new(STONE, 293));
    }
    s
}

/// Highest y (lowest on screen) occupied by `id` anywhere in the grid.
fn lowest_y_of(sim: &SimulationState, id: u16) -> u32 {
    let w = sim.grid.width as usize;
    (0..sim.grid.len())
        .filter(|&i| sim.grid.element_at(i) == id)
        .map(|i| (i / w) as u32)
        .max()
        .unwrap_or(0)
}

/// Much denser mercury placed above water sinks through it to the bottom.
#[test]
fn mercury_sinks_through_water() {
    let mut sim = column(24, 30);
    let cx = 12;
    for y in 10..14 {
        sim.grid.set(cx, y, Particle::new(MERCURY, 293));
    }
    for y in 14..22 {
        sim.grid.set(cx, y, Particle::new(WATER, 293));
    }
    sim.refresh_chunks_public();
    for _ in 0..220 {
        sim.tick();
    }
    let mercury_below =
        (15..29).any(|y| sim.grid.get(cx, y).is_some_and(|p| p.element_id == MERCURY));
    assert!(
        mercury_below,
        "mercury should sink through water to the bottom"
    );
}

/// Oil behaves as a liquid: a blob near the top falls well below its start.
#[test]
fn oil_flows_like_a_liquid() {
    let mut sim = SimulationState::new(20, 30, 1);
    for x in 0..20 {
        sim.grid.set(x, 29, Particle::new(STONE, 293));
    }
    for y in 4..7 {
        for x in 8..12 {
            sim.grid.set(x, y, Particle::new(OIL, 293));
        }
    }
    sim.refresh_chunks_public();
    for _ in 0..80 {
        sim.tick();
    }
    let lowest = lowest_y_of(&sim, OIL);
    assert!(
        lowest > 12,
        "oil should fall under gravity: started ~6, lowest now {lowest}"
    );
}
