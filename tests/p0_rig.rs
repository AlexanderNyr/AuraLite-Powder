//! P0 rig — property tests, the golden tick corpus, and cross-cutting physics
//! invariants. (ROADMAP Phase P0; decision D2/D7.)
//!
//! No new test-only dependencies: random inputs come from a local xorshift so
//! the property tests are deterministic per-seed, and the golden corpus uses a
//! self-computed fingerprint (count + rolling hash) so a model change shows up
//! as a *named* red line.
//!
//! Bootstrap: run with `POWDER_RECORD=1 cargo test --test p0_rig -- --nocapture`
//! to print the golden fingerprints, then paste them into `GOLDEN` below.

use aura_lite_core::{
    element_id::*, ChunkPool, NeutronEnergy, Particle, Scenario, SimulationState, CHUNK_SIZE,
};

// ───────────────────────── deterministic PRNG (local, no extra dep) ──────────
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn u32(&mut self, n: u32) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 as u32) % n
    }
    fn f(&mut self) -> f32 {
        self.u32(1 << 24) as f32 / (1 << 24) as f32
    }
}

// ───────────────────────── golden fingerprint ────────────────────────────────
/// Deterministic state fingerprint: non-empty count + a position/order-aware
/// rolling hash of (element_id, temperature). Two grids with the same
/// fingerprint have the same contents; any model change that alters a single
/// grain's path changes the hash.
fn fingerprint(sim: &SimulationState) -> (usize, u64) {
    let mut count = 0usize;
    let mut h: u64 = 0xcbf29ce484222325; // FNV offset
    for (i, p) in sim.grid.iter_particles().enumerate() {
        if p.is_empty() {
            continue;
        }
        count += 1;
        h ^= (i as u64).wrapping_mul(0x100000001b3);
        h = h.wrapping_mul(97).wrapping_add(p.element_id as u64);
        h = h.wrapping_mul(113).wrapping_add(p.temperature as u64);
    }
    (count, h)
}

const TICKS: u64 = 150;
const SEED: u64 = 42;

/// (name, builder-fn, expected (count, hash)). Recorded via POWDER_RECORD=1;
/// a model change that alters a grain's path shows up as a named drift.
/// Scenes are chosen to be *deterministic*: hand-built grids or scenarios that
/// do not call the un-seeded global `fastrand` (reactor_demo does, so it is
/// excluded — see ROADMAP §1 Fact 10 / P0 risk note).
#[allow(clippy::type_complexity)]
fn golden_scenes() -> Vec<(
    &'static str,
    Box<dyn Fn(&mut SimulationState)>,
    (usize, u64),
)> {
    vec![
        (
            "sand_pile",
            Box::new(|sim| {
                for x in 0..16 {
                    for y in 0..80 {
                        sim.grid.set(x, y, Particle::new(SAND, 293));
                    }
                }
            }),
            (1280, 0x2debc27eea498cf2),
        ),
        (
            "water_basin",
            Box::new(|sim| {
                for x in 0..sim.grid.width {
                    sim.grid
                        .set(x, sim.grid.height - 1, Particle::new(STONE, 293));
                }
                for y in 60..sim.grid.height - 1 {
                    for x in 20..60 {
                        sim.grid.set(x, y, Particle::new(WATER, 293));
                    }
                }
            }),
            (2808, 0x14d93f86e6910b6f),
        ),
        (
            "scenario_hourglass",
            Box::new(|sim| {
                sim.load_scenario(Scenario::Hourglass);
            }),
            (4349, 0x96c4c0107286234d),
        ),
        (
            "scenario_bomb",
            Box::new(|sim| {
                sim.load_scenario(Scenario::Bomb);
            }),
            (368, 0xd3ca58632f8f3f53),
        ),
        (
            "scenario_coolant_loop",
            Box::new(|sim| {
                sim.load_scenario(Scenario::CoolantLoop);
            }),
            (824, 0xec4b4fda53a81fbc),
        ),
        (
            "scenario_ice_melt",
            Box::new(|sim| {
                sim.load_scenario(Scenario::IceMelt);
            }),
            (4694, 0xc4524489bd792166),
        ),
    ]
}

#[test]
fn golden_tick_corpus() {
    let record = std::env::var("POWDER_RECORD").is_ok();
    let mut lines: Vec<String> = Vec::new();
    let mut all_ok = true;

    for (name, build, expected) in golden_scenes() {
        let mut sim = SimulationState::new(128, 128, SEED);
        build(&mut sim);
        sim.refresh_chunks_public();
        for _ in 0..TICKS {
            sim.tick();
        }
        let (count, hash) = fingerprint(&sim);
        if record {
            lines.push(format!(
                "    ({:?}, _, ({}, 0x{:016x})),",
                name, count, hash
            ));
        } else if expected == (0, 0) {
            // No baked expectation yet — record mode not run for this scene.
            eprintln!("golden[{name}]: (count={count}, hash=0x{hash:016x}) — NOT BAKED (run with POWDER_RECORD=1)");
        } else {
            let ok = (count, hash) == expected;
            if !ok {
                all_ok = false;
                eprintln!(
                    "golden[{name}] DRIFT: got ({count}, 0x{hash:016x}) expected ({}, 0x{:016x})",
                    expected.0, expected.1
                );
            }
        }
    }

    if record {
        println!("\n=== POWDER_RECORD: paste into golden_scenes() expected fields ===\n");
        for l in &lines {
            println!("{l}");
        }
        println!("\n(set POWDER_RECORD= only to refresh the corpus; a model change is a reviewed artifact.)");
        return;
    }

    assert!(
        all_ok,
        "golden corpus drifted — see stderr; re-record only after a reviewed model change"
    );
}

// ───────────────────────── property tests (decision D7) ─────────────────────
/// Pure-core properties, exercised with a local RNG so they are deterministic
/// per seed. Each is the kind of test that would have caught a bugfixes.patch
/// bug before release: camera-zoom anchoring (property: zoom is involutive),
/// save round-trip (GIF-class bug), chunk arithmetic.

#[test]
fn prop_world_screen_roundtrip() {
    let mut rng = Rng::new(7);
    for _ in 0..200 {
        let w = 100.0 + rng.f() * 900.0;
        let h = 100.0 + rng.f() * 900.0;
        let mut cam = aura_lite_renderer::Camera::new(w, h);
        cam.offset = aura_lite_utils::Vec2::new(rng.f() * w, rng.f() * h);
        cam.scale = 0.5 + rng.f() * 10.0;
        let p = aura_lite_utils::Vec2::new(rng.f() * w, rng.f() * h);
        let world = cam.screen_to_world(p);
        let back = cam.world_to_screen(world);
        let d = ((back.x - p.x).powi(2) + (back.y - p.y).powi(2)).sqrt();
        assert!(
            d < 1e-3,
            "world<->screen not an involution: {back:?} vs {p:?}"
        );
    }
}

#[test]
fn prop_zoom_then_unzoom_is_identity() {
    // The camera-zoom bug: zoom anchored to origin, so zoom(f)∘zoom(1/f) ≠
    // identity. With the fix it is (up to the 0.1/20 clamp).
    let mut rng = Rng::new(11);
    for _ in 0..200 {
        let mut cam = aura_lite_renderer::Camera::new(800.0, 600.0);
        cam.scale = 1.0 + rng.f() * 4.0; // stay well inside the clamp for the round trip
        cam.offset = aura_lite_utils::Vec2::new(rng.f() * 400.0, rng.f() * 300.0);
        let center = aura_lite_utils::Vec2::new(rng.f() * 800.0, rng.f() * 600.0);
        let before = cam.screen_to_world(center);
        cam.zoom(2.0, Some(center));
        cam.zoom(0.5, Some(center));
        let after = cam.screen_to_world(center);
        let d = ((after.x - before.x).abs()).max((after.y - before.y).abs());
        assert!(
            d < 1e-2,
            "zoom(2)∘zoom(0.5) not identity at cursor: {before:?} -> {after:?}"
        );
    }
}

#[test]
fn prop_pan_then_unpan_is_identity() {
    let mut rng = Rng::new(13);
    for _ in 0..200 {
        let mut cam = aura_lite_renderer::Camera::new(800.0, 600.0);
        cam.scale = 0.5 + rng.f() * 8.0;
        let d = aura_lite_utils::Vec2::new(rng.f() * 50.0, rng.f() * 50.0);
        let before = cam.offset;
        cam.pan(d);
        cam.pan(aura_lite_utils::Vec2::new(-d.x, -d.y));
        assert!((cam.offset.x - before.x).abs() < 1e-3);
        assert!((cam.offset.y - before.y).abs() < 1e-3);
    }
}

#[test]
fn prop_save_roundtrip_preserves_grid() {
    // The GIF bug class: a codec with a header-only test. Save must round-trip.
    let mut rng = Rng::new(17);
    for _ in 0..20 {
        let mut sim = SimulationState::new(24, 24, rng.u32(1000) as u64);
        for _ in 0..60 {
            let x = rng.u32(24);
            let y = rng.u32(24);
            // Non-air only: compact save mode intentionally drops air cells
            // (to_compact skips is_empty), so a round-trip of an AIR particle
            // with a non-default temperature is *supposed* to lose it. We place
            // only real material so the round-trip must be exact.
            let id = 1 + rng.u32(MAX_ELEMENT_ID as u32) as u16; // 1..=47
            sim.grid
                .set(x, y, Particle::new(id, 293 + rng.u32(2000) as u16));
        }
        let bytes = aura_lite_io::save_simulation_to_bytes(&sim, false).unwrap();
        let save = aura_lite_io::load_save_from_bytes(&bytes, false).unwrap();
        let mut loaded = SimulationState::new(8, 8, 0);
        save.apply_to(&mut loaded).unwrap();
        assert_eq!(loaded.grid.width, sim.grid.width);
        assert_eq!(loaded.grid.height, sim.grid.height);
        assert_eq!(
            loaded.grid.particles_vec(),
            sim.grid.particles_vec(),
            "save round-trip lost a cell"
        );
        assert_eq!(loaded.tick, sim.tick);
    }
}

#[test]
fn prop_save_handles_compression_roundtrip() {
    // With the `compression` feature OFF (the default), requesting compression
    // must degrade gracefully to uncompressed bytes (encode_save logs a warning
    // and returns plain bincode), so a plain decode still works.
    let mut sim = SimulationState::new(16, 16, 1);
    sim.grid.set(3, 3, Particle::new(U235, 400));
    let bytes = aura_lite_io::save_simulation_to_bytes(&sim, true).unwrap();
    let save = aura_lite_io::load_save_from_bytes(&bytes, false).unwrap();
    let mut loaded = SimulationState::new(8, 8, 0);
    save.apply_to(&mut loaded).unwrap();
    assert_eq!(loaded.grid.get(3, 3).unwrap().element_id, U235);
}

#[test]
fn prop_chunk_index_roundtrip() {
    let mut rng = Rng::new(19);
    for (w, h) in [(64u32, 64u32), (100, 100), (256, 200), (33, 17)] {
        let mut pool = ChunkPool::new(w, h);
        for _ in 0..50 {
            let cx = rng.u32(pool.chunks_x);
            let cy = rng.u32(pool.chunks_y);
            let meta = pool.get_mut(cx, cy).unwrap();
            let lx = rng.u32(CHUNK_SIZE as u32);
            let ly = rng.u32(CHUNK_SIZE as u32);
            meta.mark_dirty(lx, ly);
            assert!((meta.x, meta.y) == (cx, cy));
        }
        // expanded_active(1) is a superset of active_chunks()
        let active = pool.active_chunks();
        let expanded = pool.expanded_active(1);
        for c in active {
            assert!(
                expanded.contains(&c),
                "halo must cover active set: {c:?} missing"
            );
        }
    }
}

// ───────────────────────── physics invariants (decision D6/D8) ───────────────
/// The iodine-bug class: a cross-cutting count held by convention must be held
/// by CI. refresh_chunks counts {BORON, CONTROL_ROD, XENON, IODINE} as absorbers
/// for k-effective; every one of those must actually absorb, and nothing else
/// may silently absorb.
#[test]
fn invariant_absorber_set_matches_absorber_chance() {
    let counted_absorbers = [BORON, CONTROL_ROD, XENON, IODINE];
    for &id in &counted_absorbers {
        let thermal = aura_lite_core::reactions::absorber_chance(id, NeutronEnergy::Thermal);
        let fast = aura_lite_core::reactions::absorber_chance(id, NeutronEnergy::Fast);
        assert!(
            thermal > 0.0 || fast > 0.0,
            "element {id} is counted as an absorber in k-effective but never absorbs (iodine-class bug)"
        );
    }
    // Conversely, an element that absorbs must be in the counted set (no hidden
    // absorbers that k-effective ignores).
    for id in 0..=MAX_ELEMENT_ID {
        let absorbs = aura_lite_core::reactions::absorber_chance(id, NeutronEnergy::Thermal) > 0.0
            || aura_lite_core::reactions::absorber_chance(id, NeutronEnergy::Fast) > 0.0;
        if absorbs {
            assert!(
                counted_absorbers.contains(&id),
                "element {id} absorbs but is not counted in k-effective's absorber total"
            );
        }
    }
}

/// Registry completeness: every id in 0..=MAX_ELEMENT_ID has a definition,
/// a name, and a color. A new element added to element_id.rs without a
/// registry entry is caught here, not at render time.
#[test]
fn invariant_registry_covers_every_id() {
    for id in 0..=MAX_ELEMENT_ID {
        assert!(
            aura_lite_elements::registry::get_definition(id).is_some(),
            "element id {id} has no registry definition"
        );
        assert!(
            !aura_lite_elements::registry::name_for_id(id).is_empty(),
            "element id {id} has no name"
        );
        let col = aura_lite_elements::registry::color_for_id(id);
        assert_eq!(col.len(), 4, "element id {id} has no color");
    }
}

/// Density consistency: the registry's ElementDef.density and core's
/// density_for_id must agree (the renderer/HUD read one, the physics reads the
/// other — a drift changes buoyancy silently).
#[test]
fn invariant_registry_and_core_density_agree() {
    for id in 0..=MAX_ELEMENT_ID {
        let Some(def) = aura_lite_elements::registry::get_definition(id) else {
            continue;
        };
        let core_d = density_for_id(id);
        assert!(
            (def.density - core_d).abs() < 1e-4,
            "element {id} ({}) density drift: registry {} vs core {}",
            def.name,
            def.density,
            core_d
        );
    }
}

// ───────────────────────── P2: determinism across thread counts ─────────────
/// P2 gate: a grid large enough to take the parallel passes (>= 65536 cells)
/// must produce a byte-identical result whether the rayon pool has 1, 2, or 4
/// threads. Before P2 the reaction pass's par_iter collected candidates in a
/// thread-count-dependent order, so fission/decay outcomes diverged.
#[test]
fn deterministic_across_thread_counts() {
    use rayon::ThreadPoolBuilder;
    let build = || {
        let mut s = SimulationState::new(256, 256, SEED);
        // U-235 pile + water moderator + a starter neutron: exercises the
        // parallel reaction pass (fission) and the parallel heat pass.
        for y in 96..144 {
            for x in 96..128 {
                s.grid.set(x, y, Particle::new(U235, 400));
            }
            for x in 128..160 {
                s.grid.set(x, y, Particle::new(WATER, 320));
            }
        }
        s.grid.set(112, 95, Particle::new(NEUTRON_THERMAL, 350));
        s
    };
    let run = |threads: usize| -> (usize, u64) {
        let pool = ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        let mut s = build();
        let ptr = &mut s;
        pool.install(move || {
            for _ in 0..25 {
                ptr.tick();
            }
        });
        fingerprint(&s)
    };
    let base = run(1);
    let t2 = run(2);
    let t4 = run(4);
    assert_eq!(base, t2, "P2 determinism broken: 1-thread != 2-thread");
    assert_eq!(base, t4, "P2 determinism broken: 1-thread != 4-thread");
}

// ───────────────────────── P2b: parallel physics gates ──────────────────────
/// A 256² grid takes the P2b parallel physics path (≥ 65 536 cells). The
/// particles must actually move — this guards against a broken write-back
/// silently turning the parallel pass into a no-op.
#[test]
fn parallel_physics_moves_particles() {
    let mut sim = SimulationState::new(256, 256, SEED);
    // A sand block near the top, spanning several chunks.
    for y in 8..40 {
        for x in 64..192 {
            sim.grid.set(x, y, Particle::new(SAND, 293));
        }
    }
    sim.refresh_chunks_public();
    for _ in 0..60 {
        sim.tick();
    }
    let lowest = sim
        .grid
        .element_ids()
        .iter()
        .enumerate()
        .filter(|&(_, &id)| id == SAND)
        .map(|(i, _)| (i / sim.grid.width as usize) as u32)
        .max()
        .unwrap_or(0);
    assert!(
        lowest > 60,
        "parallel physics must move particles: sand lowest row {lowest} after 60 ticks"
    );
}

/// Particles must be able to cross chunk borders (the border pass works):
/// sand placed in the top chunk settles across several 32-cell chunk
/// boundaries into the bottom chunk.
#[test]
fn particles_cross_chunk_borders() {
    let mut sim = SimulationState::new(256, 256, SEED);
    // A full-width sand layer in the top chunk (rows 0..31).
    for y in 0..32 {
        for x in 0..256 {
            sim.grid.set(x, y, Particle::new(SAND, 293));
        }
    }
    // A floor in the very bottom row.
    for x in 0..256 {
        sim.grid.set(x, 255, Particle::new(STONE, 293));
    }
    sim.refresh_chunks_public();
    for _ in 0..400 {
        sim.tick();
    }
    // Sand must have reached well past several chunk borders (each 32 rows).
    let in_bottom_chunk = (224..255)
        .flat_map(|y| (0..256).map(move |x| (x, y)))
        .filter(|&(x, y)| sim.grid.get(x, y).is_some_and(|p| p.element_id == SAND))
        .count();
    assert!(
        in_bottom_chunk > 100,
        "sand must cross chunk borders and reach the bottom chunk (found {in_bottom_chunk} cells)"
    );
}

// ───────────────────────── P2c: classify-once counters ──────────────────────
/// The refresh scan's element-class counters must match the grid contents —
/// they gate whole passes in `tick()`, so a mis-count would silently skip
/// physics.
#[test]
fn classify_counters_match_grid() {
    let mut sim = SimulationState::new(64, 64, 1);
    // 8 water (liquid), 12 sand (powder), 3 pipes, 1 heater (device),
    // 1 wire (device), 1 fire (device), 5 stone (neither).
    for x in 0..8 {
        sim.grid.set(x, 10, Particle::new(WATER, 293));
    }
    for x in 0..12 {
        sim.grid.set(x, 20, Particle::new(SAND, 293));
    }
    for x in 0..3 {
        sim.grid.set(x, 30, Particle::new(PIPE, 293));
    }
    sim.grid.set(5, 40, Particle::new(HEATER, 293));
    sim.grid.set(6, 40, Particle::new(WIRE, 293));
    sim.grid.set(7, 40, Particle::new(FIRE, 1100));
    for x in 0..5 {
        sim.grid.set(x, 50, Particle::new(STONE, 293));
    }
    sim.refresh_chunks_public();
    assert_eq!(sim.liquid_cells, 8, "water cells");
    assert_eq!(sim.powder_cells, 12, "sand cells");
    assert_eq!(sim.pipe_cells, 3, "pipe cells");
    assert_eq!(sim.device_cells, 3, "heater + wire + fire cells");
    // And the chunk active set still covers every non-empty chunk.
    let active = sim.chunk_pool.active_chunks();
    assert!(
        active.contains(&(0, 0)),
        "the chunk holding all particles must be active"
    );
}
