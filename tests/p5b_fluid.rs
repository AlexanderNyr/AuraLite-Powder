//! P5b fluid-transient gate (ROADMAP). Compiled only under `--features fluid-pde`:
//! a steam explosion when water meets molten fuel. Run with
//! `cargo test --features fluid-pde --no-default-features --test p5b_fluid`.

#![cfg(feature = "fluid-pde")]

use aura_lite_core::{element_id::*, Particle, SimulationState};

fn count_id(sim: &SimulationState, id: u16) -> usize {
    sim.grid.element_ids().iter().filter(|&&e| e == id).count()
}

/// Water in contact with molten fuel must flash to steam (and the blast ejects
/// surroundings). Under the default model nothing happens; under `fluid-pde`
/// the transient fires.
#[test]
fn steam_explosion_flashes_water_to_steam() {
    let mut sim = SimulationState::new(40, 40, 1);
    // Molten core.
    for y in 18..22 {
        for x in 18..22 {
            sim.grid.set(x, y, Particle::new(MOLTEN_FUEL, 2500));
        }
    }
    // Water jacket in contact with it.
    for y in 16..24 {
        for x in 16..24 {
            if sim.grid.get(x, y).is_some_and(|p| p.is_empty()) {
                sim.grid.set(x, y, Particle::new(WATER, 300));
            }
        }
    }
    sim.refresh_chunks_public();
    assert_eq!(count_id(&sim, STEAM), 0, "no steam at start");

    for _ in 0..50 {
        sim.tick();
    }
    assert!(
        count_id(&sim, STEAM) > 0,
        "water contacting molten fuel must flash to steam (P5b steam explosion)"
    );
    // The blast should also have displaced mass out of the contact zone:
    // some of the original molten/water cells are no longer molten/water.
    let molten = count_id(&sim, MOLTEN_FUEL);
    let water = count_id(&sim, WATER);
    assert!(
        molten + water < 16 + 40,
        "steam explosion should have cleared/displaced some cells (molten={molten} water={water})"
    );
}
