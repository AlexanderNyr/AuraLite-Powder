//! Headless replay (P9a, ROADMAP). Run a simulation forward deterministically
//! and reduce the final grid to a stable hash — the foundation for reproducible
//! bug reports ("this save replays to hash X") and a long-run regression gate
//! that complements the short P0 golden corpus.
//!
//! The hash covers the **element-id layout only**, deliberately excluding
//! temperatures: the heat solver uses f32, whose rounding is not stable across
//! compilers/optimisation levels, so hashing it would make the gate flaky.
//! Element positions are driven by the integer cellular automaton plus the
//! per-tick deterministic RNG, so for a thermally-inert scene they ARE stable
//! across builds — which is exactly what a regression hash needs.

use aura_lite_core::{Grid, SimulationState};

/// FNV-1a over the element-id array.
pub fn grid_layout_hash(grid: &Grid) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &id in grid.element_ids() {
        h ^= id as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Run `ticks` simulation steps and return the resulting layout hash.
/// Deterministic for a given `(state, seed)`; stable across builds for
/// thermally-inert scenes (uniform ambient temperature, no fission/heating).
pub fn replay_hash(sim: &mut SimulationState, ticks: u64) -> u64 {
    for _ in 0..ticks {
        sim.tick();
    }
    grid_layout_hash(&sim.grid)
}

/// Decode a `.aura` save, restore it onto a fresh simulation, run `ticks`, and
/// return the layout hash — the one-call form for replay tooling.
pub fn replay_save_bytes(bytes: &[u8], ticks: u64) -> Result<u64, crate::error::IoError> {
    let save = crate::save::load_save_from_bytes(bytes, false)?;
    let mut sim = SimulationState::new(8, 8, save.seed);
    save.apply_to(&mut sim)?;
    Ok(replay_hash(&mut sim, ticks))
}
