//! P9b fuzz gates (ROADMAP): the IO codecs must be panic-free on arbitrary
//! input. The original GIF LZW bug shipped because the codec's only test
//! checked the header — these tests throw thousands of random and
//! systematically mutated inputs at the save decoder and apply path.
//!
//! Deterministic by construction: a local xorshift, no external fuzzer, runs
//! in the normal `cargo test` suite.

use aura_lite_core::{Particle, SimulationState};
use aura_lite_io::{load_save_from_bytes, replay_save_bytes, save_simulation_to_bytes};

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u64() as u8).collect()
    }
}

fn a_valid_save() -> Vec<u8> {
    let mut sim = SimulationState::new(12, 12, 7);
    sim.grid.set(3, 3, Particle::new(4, 400));
    sim.grid.set(5, 7, Particle::new(1, 293));
    save_simulation_to_bytes(&sim, false).expect("encode a valid save")
}

/// Decode must never panic on arbitrary bytes (random buffers of many sizes,
/// both compression flags).
#[test]
fn save_decode_fuzz_random_buffers() {
    let mut r = Rng::new(0xa11ce);
    for i in 0..2000 {
        let len = (r.u64() % 600) as usize;
        let buf = r.bytes(len);
        let _ = load_save_from_bytes(&buf, false);
        let _ = load_save_from_bytes(&buf, true);
        if i % 500 == 0 {
            // Also exercise the one-call replay path.
            let _ = replay_save_bytes(&buf, 3);
        }
    }
}

/// Systematic mutation: flip every byte of a valid save to several values.
/// This reaches the crafted-save cases random bytes rarely hit (e.g. a huge
/// grid width field) — exactly the input that used to blow up the grid
/// allocation. Decode and — when the mutation still decodes — apply must both
/// be panic-free.
#[test]
fn save_decode_and_apply_fuzz_mutations() {
    let valid = a_valid_save();
    for pos in 0..valid.len() {
        for &val in &[0x00u8, 0xFF, 0x7F] {
            let mut buf = valid.clone();
            buf[pos] = val;
            let save = match load_save_from_bytes(&buf, false) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let mut sim = SimulationState::new(8, 8, 0);
            let _ = save.apply_to(&mut sim);
            // The replay path takes the same route.
            let _ = replay_save_bytes(&buf, 2);
        }
    }
}

/// Truncations of a valid save (every prefix) — the classic "torn write".
#[test]
fn save_decode_fuzz_truncations() {
    let valid = a_valid_save();
    for cut in 0..valid.len() {
        let buf = &valid[..cut];
        if let Ok(save) = load_save_from_bytes(buf, false) {
            let mut sim = SimulationState::new(8, 8, 0);
            let _ = save.apply_to(&mut sim);
        }
    }
}

/// The allocation bomb: a compact save claiming a u32::MAX × u32::MAX grid
/// with zero particles is tiny on disk but demands exabytes on load. Without
/// a dimension guard, `Grid::new` panics with "capacity overflow" (or worse,
/// OOM-aborts on dimensions that fit isize).
#[test]
fn save_with_absurd_grid_dimensions_is_rejected() {
    let valid = a_valid_save();
    let mut save = load_save_from_bytes(&valid, false).expect("valid save decodes");
    save.grid_width = u32::MAX;
    save.grid_height = u32::MAX;
    save.particles.clear(); // compact mode: no payload at all
    let bytes = bincode::serde::encode_to_vec(&save, bincode::config::standard())
        .expect("re-encode crafted save");
    match load_save_from_bytes(&bytes, false) {
        Err(_) => {} // rejected at decode — fine
        Ok(s) => {
            let mut sim = SimulationState::new(8, 8, 0);
            // Must be rejected (or applied with sane dimensions) — NOT panic
            // with capacity overflow inside Grid::new.
            let _ = s.apply_to(&mut sim);
            assert!(
                sim.grid.width <= 8192 && sim.grid.height <= 8192,
                "absurd dimensions must not be honoured: {}x{}",
                sim.grid.width,
                sim.grid.height
            );
        }
    }
}

/// The length-claim bomb: splice a huge particles-count varint into an
/// otherwise valid save. bincode pre-allocates `Vec::with_capacity(len)`
/// before reading a single element, so without a decode limit the claimed
/// bytes are allocated up front. The limit must reject the claim at the
/// decoder, before any allocation.
#[test]
fn save_with_absurd_length_claim_is_rejected_at_decode() {
    let one = {
        let mut save = load_save_from_bytes(&a_valid_save(), false).unwrap();
        save.particles = vec![save.particles[0]];
        bincode::serde::encode_to_vec(&save, bincode::config::standard()).unwrap()
    };
    let two = {
        let mut save = load_save_from_bytes(&a_valid_save(), false).unwrap();
        save.particles = vec![save.particles[0], save.particles[0]];
        bincode::serde::encode_to_vec(&save, bincode::config::standard()).unwrap()
    };
    // The two encodings differ first at the particles-length varint (both are
    // single-byte for 1 and 2).
    let pos = one
        .iter()
        .zip(two.iter())
        .position(|(a, b)| a != b)
        .expect("encodings differ");
    // Claim 2^32 particles (2^32 * 16 B = 64 GiB) — far past any sane budget.
    let claim: u64 = 1 << 32;
    let varint = bincode::serde::encode_to_vec(claim, bincode::config::standard()).unwrap();
    let mut bomb = one[..pos].to_vec();
    bomb.extend_from_slice(&varint);
    bomb.extend_from_slice(&one[pos + 1..]);
    // Must return an Err (the claim exceeds the decode limit) — and must not
    // have allocated 64 GiB to get there.
    assert!(
        load_save_from_bytes(&bomb, false).is_err(),
        "an absurd length claim must be rejected at decode"
    );
}
