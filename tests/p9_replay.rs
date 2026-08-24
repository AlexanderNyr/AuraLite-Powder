//! P9a replay gate (ROADMAP): a 1000-tick deterministic layout hash. This is a
//! LONG-run regression gate that complements the short P0 golden corpus — any
//! model change that alters the 1000-tick element layout flips the hash.
//!
//! The scene is thermally inert (uniform 293 K, no fission), so the hash is
//! stable across build configurations (f32 rounding in the heat solver does not
//! reach element positions). Re-bake with `POWDER_RECORD=1` after a reviewed
//! change.

use aura_lite_core::{element_id::*, Particle, SimulationState};
use aura_lite_io::replay_hash;

fn scene() -> SimulationState {
    let mut s = SimulationState::new(96, 96, 42);
    // Stone basin walls.
    for x in 0..96 {
        s.grid.set(x, 95, Particle::new(STONE, 293));
    }
    for y in 0..96 {
        s.grid.set(0, y, Particle::new(STONE, 293));
        s.grid.set(95, y, Particle::new(STONE, 293));
    }
    // A sand block that will collapse and spread.
    for y in 8..38 {
        for x in 20..50 {
            s.grid.set(x, y, Particle::new(SAND, 293));
        }
    }
    // A water pool it can run into.
    for y in 64..94 {
        for x in 30..70 {
            s.grid.set(x, y, Particle::new(WATER, 293));
        }
    }
    s.refresh_chunks_public();
    s
}

#[test]
fn replay_hash_stable_1000_ticks() {
    let mut s = scene();
    let h = replay_hash(&mut s, 1000);
    if std::env::var("POWDER_RECORD").is_ok() {
        println!("BAKED replay_hash_1000 = 0x{h:016x}");
        return;
    }
    assert_eq!(
        h, 0x86bf17c0b45557f3,
        "1000-tick replay hash drifted (got 0x{h:016x}) — re-record with POWDER_RECORD=1 only \
         after a reviewed model change"
    );
}

/// Short-run determinism: the same scene replayed twice must hash identically
/// (the function is a pure function of state + seed + ticks).
#[test]
fn replay_hash_is_deterministic() {
    let h1 = replay_hash(&mut scene(), 200);
    let h2 = replay_hash(&mut scene(), 200);
    assert_eq!(h1, h2, "replay_hash must be deterministic");
    assert_ne!(h1, 0, "a non-empty scene must produce a non-zero hash");
}
