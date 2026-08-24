//! Headless replay tool (P9a): load a `.aura` save, run N ticks, print the
//! deterministic layout hash. For bug reports: "save X replays to hash Y".
//!
//! Run: cargo run --release --no-default-features --example replay -- save.aura 1000

use aura_lite_core::SimulationState;
use aura_lite_io::{load_save_from_bytes, replay_hash};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args
        .get(1)
        .expect("usage: replay <save.aura> [ticks] (default 1000)");
    let ticks: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let bytes = std::fs::read(path).expect("could not read save file");
    let save = load_save_from_bytes(&bytes, false).expect("could not decode save");
    let mut sim = SimulationState::new(8, 8, save.seed);
    save.apply_to(&mut sim).expect("could not restore save");
    let h = replay_hash(&mut sim, ticks);
    println!("replay {} ticks -> layout hash 0x{h:016x}", ticks);
}
