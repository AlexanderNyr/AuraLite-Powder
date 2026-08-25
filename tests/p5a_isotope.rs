//! P5a isotope-model gates (ROADMAP): U-238 breeds Pu-239 by neutron capture,
//! a burning pile's enrichment falls, and enrichment raises the *measured*
//! k-effective (tying into P4's measurement).

use aura_lite_core::{
    element_id::*, reactions, NeutronEnergy, NeutronEvent, Particle, SimulationState,
};

fn count_id(sim: &SimulationState, id: u16) -> usize {
    sim.grid.element_ids().iter().filter(|&&e| e == id).count()
}

/// Uranium-family enrichment: U-235 as a fraction of (U-235 + U-238).
fn enrichment(sim: &SimulationState) -> f32 {
    let u235 = count_id(sim, U235) as f32;
    let u238 = count_id(sim, U238) as f32;
    if u235 + u238 == 0.0 {
        return 0.0;
    }
    u235 / (u235 + u238)
}

/// A U-238 block under a neutron flux breeds Pu-239 (U-238 + n → Pu-239).
#[test]
fn u238_breeds_pu239() {
    let mut sim = SimulationState::new(48, 48, 1);
    for y in 18..30 {
        for x in 18..30 {
            sim.grid.set(x, y, Particle::new(U238, 350));
        }
    }
    sim.refresh_chunks_public();
    // A staggered burst of thermal neutrons into the block. U-238 thermal
    // fission is only 0.02, so most surviving interactions are captures.
    let mut i = 0u8;
    for y in 19..29 {
        for x in 19..29 {
            sim.neutron_queue.push_back(NeutronEvent {
                x,
                y,
                delay: i % 4,
                energy: NeutronEnergy::Thermal,
            });
            i = i.wrapping_add(1);
        }
    }
    for _ in 0..60 {
        sim.tick();
    }
    let pu = count_id(&sim, PU239);
    assert!(
        pu > 3,
        "U-238 under neutron flux must breed Pu-239 (bred {pu} cells, fissions {})",
        sim.fission_count
    );
    assert!(
        sim.fission_count > 0,
        "the bred/natural fuel must also fission"
    );
}

/// A burning pile's enrichment falls: U-235 fissions (0.85 thermal) far more
/// readily than U-238 fissions (0.02) or captures, so the U-235 fraction of
/// the uranium family drops as the pile burns.
#[test]
fn enrichment_drops_as_pile_burns() {
    let mut sim = SimulationState::new(48, 48, 3);
    // A graphite moderator wrapper so fission-spawned fast neutrons
    // thermalize (via P4's two-step path) and hit the 0.85-vs-0.02
    // fission asymmetry instead of the fast 0.35-vs-0.25 near-tie.
    for y in 14..34 {
        for x in 14..34 {
            sim.grid.set(x, y, Particle::new(GRAPHITE, 320));
        }
    }
    // A checkerboard of U-235 and U-238 — 50% enrichment.
    for y in 18..30 {
        for x in 18..30 {
            let id = if (x + y) % 2 == 0 { U235 } else { U238 };
            sim.grid.set(x, y, Particle::new(id, 380));
        }
    }
    sim.refresh_chunks_public();
    let initial = enrichment(&sim);
    assert!((initial - 0.5).abs() < 1e-3, "checkerboard starts at 50%");

    // A sustained thermal flux over the WHOLE pile (both isotopes): thermal
    // neutrons fission U-235 at 0.85 but U-238 at only 0.02, so U-235 burns
    // away preferentially.
    for burst in 0..6 {
        let mut i = 0u8;
        for y in 19..29 {
            for x in 19..29 {
                sim.neutron_queue.push_back(NeutronEvent {
                    x,
                    y,
                    delay: (i % 3) + (burst > 0) as u8,
                    energy: NeutronEnergy::Thermal,
                });
                i = i.wrapping_add(1);
            }
        }
        for _ in 0..20 {
            sim.tick();
        }
    }
    let final_e = enrichment(&sim);
    assert!(
        final_e < initial - 0.10,
        "enrichment must fall as the pile burns: {initial:.3} -> {final_e:.3}"
    );
}

/// Enrichment raises the measured k (P4's measurement): the same
/// graphite-moderated geometry multiplies better with a U-235 core than with a
/// 20%-enriched U-235/U-238 core — the critical-mass/enrichment tie-in.
#[test]
fn enrichment_raises_measured_k() {
    let run = |enriched: bool| -> (f32, u64) {
        let mut sim = SimulationState::new(96, 96, 7);
        let (cx, cy) = (48, 48);
        // Graphite moderator wrapper.
        for y in (cy - 15)..(cy + 15) {
            for x in (cx - 15)..(cx + 15) {
                sim.grid.set(x, y, Particle::new(GRAPHITE, 320));
            }
        }
        // Core: pure U-235 or a 20% checkerboard of U-235 in U-238.
        for y in (cy - 6)..(cy + 6) {
            for x in (cx - 6)..(cx + 6) {
                let id = if enriched || (x + y) % 5 == 0 {
                    U235
                } else {
                    U238
                };
                sim.grid.set(x, y, Particle::new(id, 400));
            }
        }
        sim.grid.set(cx, cy, Particle::new(NEUTRON_THERMAL, 350));
        sim.refresh_chunks_public();
        for _ in 0..300 {
            sim.tick();
        }
        (sim.k_measured, sim.fission_count)
    };

    let (k_pure, f_pure) = run(true);
    let (k_dilute, f_dilute) = run(false);
    assert!(
        f_pure > 0 && f_dilute > 0,
        "both piles must fission (pure {f_pure}, dilute {f_dilute})"
    );
    assert!(
        k_pure > k_dilute,
        "a pure U-235 core must out-multiply a 20%-enriched one: \
         k {k_pure:.3} vs {k_dilute:.3} (fissions {f_pure} vs {f_dilute})"
    );
}

/// The capture chance is energy-ordered like the absorbers (thermal highest).
#[test]
fn u238_capture_is_energy_ordered() {
    let t = reactions::u238_capture_chance(NeutronEnergy::Thermal);
    let e = reactions::u238_capture_chance(NeutronEnergy::Epithermal);
    let f = reactions::u238_capture_chance(NeutronEnergy::Fast);
    assert!(
        t > e && e > f && f > 0.0,
        "thermal {t} > epithermal {e} > fast {f}"
    );
}
