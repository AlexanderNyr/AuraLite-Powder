//! P3 thermal-model gates (ROADMAP). Compiled only under `--features thermal-pde`:
//! the Doppler reactivity feedback (negative temperature coefficient) and latent
//! heat at phase changes. Run with `cargo test --features thermal-pde --test p3_thermal`.

#![cfg(feature = "thermal-pde")]

use aura_lite_core::{element_id::*, reactions, NeutronEnergy, Particle, SimulationState};

/// The unit-level statement of Doppler: fission probability must FALL as fuel
/// temperature rises. (Under the default model it rises — so this test is
/// meaningful only under `thermal-pde`, which is why the file is gated.)
#[test]
fn doppler_lowers_reactivity_at_high_temp() {
    let ambient = reactions::fission_probability(U235, NeutronEnergy::Thermal, 300);
    let warm = reactions::fission_probability(U235, NeutronEnergy::Thermal, 900);
    let hot = reactions::fission_probability(U235, NeutronEnergy::Thermal, 1600);
    assert!(
        ambient > warm,
        "ambient {ambient} should exceed warm {warm}"
    );
    assert!(warm > hot, "warm {warm} should exceed hot {hot}");
    assert!(
        hot < 0.2,
        "very hot U-235 must be strongly suppressed, got {hot}"
    );
    assert!(
        ambient > 0.7,
        "ambient U-235 thermal fission should be near base 0.85, got {ambient}"
    );
}

/// The integration gate: a graphite-moderated pile, lit with no control rods,
/// must NOT run away to meltdown. Under the MVP model a sustained chain heats
/// monotonically past the 2000 K meltdown threshold; under Doppler the negative
/// temperature coefficient self-limits it — no molten fuel forms and the peak
/// temperature stays bounded. (A bare U-235 pile is subcritical on fast fission
/// alone, so the test embeds fuel in a graphite moderator to make the chain
/// actually sustain — then checks Doppler caps it.)
#[test]
fn self_limiting_pile() {
    let mut sim = SimulationState::new(96, 96, 3);
    // Concrete floor.
    for x in 0..96 {
        sim.grid.set(x, 94, Particle::new(CONCRETE, 293));
        sim.grid.set(x, 95, Particle::new(CONCRETE, 293));
    }
    // Graphite moderator block with an embedded U-235 core.
    for y in 30..80 {
        for x in 30..66 {
            sim.grid.set(x, y, Particle::new(GRAPHITE, 320));
        }
    }
    for y in 44..66 {
        for x in 42..54 {
            sim.grid.set(x, y, Particle::new(U235, 400));
        }
    }
    sim.grid.set(48, 50, Particle::new(NEUTRON_THERMAL, 350));
    sim.refresh_chunks_public();

    let mut peak: u16 = 0;
    for _ in 0..260 {
        sim.tick();
        for i in 0..sim.grid.len() {
            let t = sim.grid.temperature_at(i);
            if t > peak {
                peak = t;
            }
        }
    }
    let molten = sim
        .grid
        .element_ids()
        .iter()
        .filter(|&&id| id == MOLTEN_FUEL)
        .count();
    assert!(
        sim.fission_count > 10,
        "the moderated pile must sustain a chain"
    );
    assert_eq!(molten, 0, "Doppler must prevent meltdown (no molten fuel)");
    assert!(
        peak < 3500,
        "Doppler must cap the temperature: peak reached {peak} K"
    );
}

/// Latent heat: water boiling to steam must cool its neighbours (the phase
/// change absorbs energy), not carry the excess heat into the steam for free.
#[test]
fn boiling_cools_neighbours() {
    let mut sim = SimulationState::new(10, 10, 1);
    sim.grid.set(5, 5, Particle::new(WATER, 700)); // well above the 373 K boil gate
    sim.grid.set(4, 5, Particle::new(STONE, 700)); // hot neighbour
    let before = sim.grid.get(4, 5).unwrap().temperature;
    for _ in 0..40 {
        sim.tick();
    }
    let neighbour = sim.grid.get(4, 5).unwrap();
    assert!(
        neighbour.element_id == STONE && neighbour.temperature < before,
        "latent heat should cool the stone neighbour: {before} -> {}",
        neighbour.temperature
    );
}
