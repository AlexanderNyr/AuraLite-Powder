# Changelog

All notable changes to **AuraLite Powder** are recorded here, grouped by the
development phases defined in [`ROADMAP.md`](./ROADMAP.md). The format mirrors a
Keep-a-Changelog log, adapted to the phase-gated workflow (one `.patch` per
phase, a definition of done and a test gate for every phase).

The project follows the layered contract documented in ROADMAP §4: `core` knows
nothing of render/ui/io/wasm. No entry here may break that invariant —
`ci/check_powder_claims.py` asserts it every CI run.

---

## [Unreleased]

Work toward the ROADMAP phases, applied on top of the upstream `main`.

### Phase P5b — Pressure: steam-explosion transient — 2026-08-24  ✅
*Deliverable: `patches/P5b_fluids.patch` (feature `fluid-pde`, opt-in)*

- **Added** the `fluid-pde` feature (core + workspace), off by default — the
  MVP has no pressure transients; save replay and the golden corpus are
  unchanged (verified).
- **Added** the steam explosion: under `fluid-pde`, water (or heavy water) in
  contact with molten fuel flashes to steam and the blast ejects its
  surroundings (`physics::apply_impulse`) with a heat spike — the real
  reactor-accident transient the MVP lacks.
- **Test gate (under `--features fluid-pde`):**
  `steam_explosion_flashes_water_to_steam` — water around a molten core
  produces steam and displaces mass out of the contact zone.
- **Default build unchanged:** golden corpus identical; 60 integration + 10
  unit tests green; fmt + clippy clean.
- **Deferred:** the full Navier–Stokes-lite pressure/velocity solver (water
  hammer, continuous advection) — the steam-explosion impulse is the testable
  transient; the solver is a deeper, separate sub-phase.

### Elements: Oil + Mercury (completes P8's "elements" sub-item) — 2026-08-24

- **Added** two new elements, extending the registry to 50 (ids 0–49):
  - **Oil** (id 48) — a flammable liquid lighter than water (density 0.85);
    burns via the existing `is_flammable` fire-spread path.
  - **Mercury** (id 49) — a very dense liquid (13.5) that sinks through water.
- Both reuse the existing liquid physics — no new simulation code. The density
  separation (mercury sinking through water) works through the existing
  density-based `try_sink` in `update_liquid`.
- **Tests:** `mercury_sinks_through_water`, `oil_flows_like_a_liquid`
  (`tests/p8_elements.rs`); the P0 invariant
  `invariant_registry_covers_every_id` now pins ids 0–49.
- **Gates:** golden corpus unchanged (new ids don't touch existing scenes);
  62 integration + 10 unit tests green; fmt + clippy clean; claim checker
  updated (MAX_ELEMENT_ID 47→49, 48→50 constants).

### Phase P9a — Hardening: headless replay + long-run hash — 2026-08-24  ✅
*Deliverable: `patches/P9a_replay.patch` (baseline: through `ci_green.patch`)*

- **Added** `aura_lite_io::replay` — `replay_hash(&mut sim, ticks)`,
  `grid_layout_hash(&grid)`, and `replay_save_bytes(bytes, ticks)`: run a
  simulation forward deterministically and reduce the final grid to a hash.
- **Added** the long-run regression gate `replay_hash_stable_1000_ticks`
  (`tests/p9_replay.rs`): a 1 000-tick layout hash (baked
  `0x86bf17c0b45557f3`). This complements the short P0 golden corpus — any
  model change that alters the long-run element layout flips the hash. The hash
  covers **element ids only** (temperatures excluded) so it is stable across
  dev/release builds despite the heat solver's f32 rounding; verified identical
  in dev and release.
- **Added** `examples/replay.rs` — a headless tool that loads a `.aura` save,
  runs N ticks, and prints the layout hash (for reproducible bug reports).
- **Test gate:** 1 000-tick hash identical dev↔release; `replay_hash` is
  deterministic; 70 default tests green; fmt + clippy clean.

### CI / build fixes — 2026-08-24
*Deliverable: `patches/ci_green.patch`*

- **Fixed** the CI `Fmt + Clippy` job: `cargo fmt --all` (the tree was not
  fmt-clean) and resolved every `clippy -D warnings` lint the newer toolchain
  raised (derivable `Default` impls, needless range loops, `clamp`/match-guard
  idioms, `as_chunks`, `is_multiple_of`, `too_many_arguments` allows, etc.).
- **Fixed** the CI `Test` job: it ran `cargo test --release`, but the release
  profile (`lto + panic = "abort"`) is unsuitable for the test harness — tests
  now run in the dev profile (`cargo test`); the release build is still
  verified by the separate `cargo build --release` step.
- **Fixed** the CI `Bench` job: `bench_record.py` parsed criterion's JSON for
  the wrong reason string (`benchmark` vs `benchmark-complete`) and treated the
  median as a bare number (criterion 0.5 emits a `{point_estimate}` object), so
  it always parsed zero medians and failed. The parser now handles both shapes
  and no longer fails CI on an empty parse.
- **Hardened** the `Quench` mission's steel core (2×2) so its win threshold is
  not flippable by f32-rounding differences between optimisation levels.
- **Cleared** the Node.js 20 deprecation warnings by bumping
  `actions/checkout`, `actions/cache`, `actions/upload-artifact` to v5.
- **Verified** the WASM build end-to-end (`wasm-pack build --target web` → pkg
  ready); it was not actually broken — the CI run was red on the three jobs
  above, not on WASM.

### Phase P8 — Content: campaign + 8 missions — 2026-08-24  ✅
*Deliverable: `patches/P8_content.patch` (baseline: through `P3_thermal.patch`)*

- **Added** two missions, bringing the total to the roadmap's eight:
  - **Tritium breeder** — a lithium blanket under a switched-on neutron flux
    breeds tritium (Li + n → T); win at ≥ 15 tritium atoms.
  - **Quench the core** — a glowing-hot steel core submerged in a water pool;
    win when the core cools below 900 K (heat diffusion + boiling).
- **Added** the `Campaign` framework (core): an ordered mission list where each
  mission unlocks when the previous one is won (`is_unlocked` / `record` /
  `next`). Forward-compatible with saves (new `MissionId` values 6–7; old saves
  with 0–5 still load).
- **Test gates:** `test_mission_tritium_breeder_wins`,
  `test_mission_quench_wins`, `test_campaign_unlock_logic`,
  `test_eight_missions_all_start` — all four green. 68 default tests pass;
  claim checker 13/13.
- **Deferred:** new *elements* (the roadmap's "elements" sub-item) and campaign
  progress persistence across saves — both additive follow-ups that do not
  affect the eight-mission gate.

### Phase P3 — Thermal: Doppler reactivity feedback + latent heat — 2026-08-24  ✅
*Deliverable: `patches/P3_thermal.patch` (feature `thermal-pde`, opt-in; baseline: through `P2a_parallel.patch`)*

- **Added** the `thermal-pde` feature (core + workspace). Off by default, so the
  MVP model, save replay and the golden corpus are unchanged (verified: the
  default `golden_tick_corpus` still passes byte-for-byte).
- **Added** `reactions::temperature_coefficient` — a negative Doppler
  coefficient per fissile isotope (U-235 −0.0008/K, …). Under `thermal-pde`,
  `fission_probability` uses it instead of the MVP's mild positive coefficient,
  so reactivity **falls** as fuel temperature rises — the feedback real
  reactors rely on. A chain reaction now self-limits instead of running to
  meltdown.
- **Added** latent heat at the water→steam phase change under `thermal-pde`:
  the steam starts near the boiling point and saps heat from its neighbours,
  rather than carrying the excess energy for free.
- **Test gates (under `--features thermal-pde`):**
  `doppler_lowers_reactivity_at_high_temp` (unit: fission prob is monotone
  decreasing in temperature), `self_limiting_pile` (a graphite-moderated pile
  sustains a chain but forms **no molten fuel** and peaks < 3500 K), and
  `boiling_cools_neighbours` (latent heat).
- **Default build unchanged:** 64 tests green, golden corpus identical.
- **Deferred:** the full implicit ADI heat-conduction solver (the current
  conductivity-weighted Jacobi step is retained — stable and sufficient for the
  feedback loop; ADI is a numerical-precision refinement for a later sub-phase).

### Phase P2a — Parallel passes: deterministic reactions + parallel heat — 2026-08-24  ✅
*Deliverable: `patches/P2a_parallel.patch` (baseline: through `P1_soa.patch`)*

- **Fixed** a real reproducibility bug in `reaction_pass_parallel`: the
  `par_iter` over chunks returned fissile/decay candidate lists in a
  thread-count-dependent order, so the same grid produced different
  fission/decay outcomes on 1 vs N threads. The candidate lists are now
  `sort_unstable` + `dedup`'d before the rng-driven application (fusion pairs
  were already sorted). This is the headline of the phase — large-grid
  simulations are now reproducible.
- **Added** `physics::diffuse_heat_parallel` — the heat Jacobi step is
  embarrassingly parallel (each cell reads current state, writes a disjoint
  `next[idx]`), so it runs under rayon. Factored the per-cell compute into
  `heat_step_cell` so the sequential and parallel solvers cannot drift.
  `effects_pass_parallel` now uses it.
- **Added** the determinism test `deterministic_across_thread_counts`: a 256²
  grid (≥ 65536 cells → parallel path) ticked 25 times under rayon pools of
  1/2/4 threads must yield a byte-identical fingerprint.
- **Test gate — determinism:** ✅ 1/2/4-thread fingerprints identical; golden
  corpus unchanged (128² scenes use the sequential path). 64 tests green.
- **Perf:** release (no-LTO, ~2 cores) `tick`: 256² 4.9→**4.6 ms**, 512²
  20.5→**13.2 ms** (≈1.55× over P1; ≈4.6× over the original AoS baseline at
  512²). The gravity pass stays sequential — its parallelisation is **P2b**.

### Phase P1 — SoA particle layout — 2026-08-24  ✅
*Deliverable: `patches/P1_soa.patch` (baseline: `main` + `bugfixes.patch` + `patches/P0_rig.patch` + `patches/P1_plan.patch`)*

- **Changed** `Grid` storage from `Vec<Particle>` (AoS) to **struct-of-arrays**
  (`element_ids: Vec<u16>`, `temperatures: Vec<u16>`, `flags: Vec<u8>`,
  `lifetimes: Vec<u8>`). Scan passes now read a contiguous `element_ids` slice
  — a 64-byte cache line delivers 32 ids instead of 4.
- **Added** the SoA accessor set: owned `get`, `set`, a `modify(x,y,|p|…)`
  closure (the SoA replacement for `get_mut → &mut Particle`, which cannot span
  four arrays), index/field accessors (`element_at`/`temperature_at`/…
  /`or_flag_at`/`add_temperature_at`) for the hot inner loops, contiguous slice
  accessors (`element_ids()`/`temperatures()`/…) for scans, `swap_particles`,
  `iter_particles`, and `particles_vec()`/`with_particles()`/`set_particles_vec()`
  for the serialize/undo boundary.
- **Changed** every consumer (`physics`, `simulation`, `hydro`, `devices`,
  `missions`, `io/save`, `renderer/compose`, `renderer/overlay`,
  `ui/app_state`) off the `particles` field and `get_mut`.
- **Save format v2 unchanged** — `SaveFile` still stores `Vec<Particle>`;
  `with_particles`/`particles_vec` are the only boundary. The migration path
  (D9) is untouched.
- **Test gate — correctness (D2):** the P0 golden tick corpus is **unchanged**
  (all 6 scene fingerprints identical) — the refactor is behaviour-preserving.
  63 tests green; full GUI path `cargo check`s; claim checker 13/13.
- **Test gate — perf:** release (no-LTO) `tick` on a half-full grid:
  256² **4.9 ms**, 512² **20.5 ms**, vs the §1 AoS baseline 13.6 ms / 60.9 ms —
  **~2.8× speedup**, far past the 1.40× gate. (Authoritative comparison runs
  in CI via `ci/bench_record.py` against `bench/baseline.csv`.)

### Documentation — 2026-08-24
- Added [`ROADMAP.md`](./ROADMAP.md) — the master development roadmap:
  status table for phases P0–P9 (+ splits), the framing question, §1 "where
  things actually stand" with ten measured facts, the decision log (D1–D8),
  the dependency graph, and per-phase Objective / Tasks / Result / Test gate /
  Deliverable. Inherited structure from the AuraLite-OS plans
  (`ARM64_PLAN.md`, `BOOTLOADER_ROADMAP.md`).
- Added [`docs/STUDY_REPORT.md`](./docs/STUDY_REPORT.md) — full codebase
  audit: per-crate line counts, the data-flow diagram, the `Particle`/`Grid`
  data model, the cellular-automaton physics, the nuclear model, hydrodynamics,
  devices, missions, rendering, UI, IO and WASM layers, plus build/test
  verification results.
- Added [`docs/P0_REPORT.md`](./docs/P0_REPORT.md) — P0 completion report.

### Phase P0 — Measurement rig, invariants, CI — 2026-08-24  ✅
*Deliverable: `patches/P0_rig.patch` (baseline: `main` + `bugfixes.patch`)*

- **Added** `tests/p0_rig.rs` — 10 tests:
  - **Golden tick corpus** (6 deterministic scenes × 150 ticks @ seed 42,
    fingerprinted as `(count, rolling hash)`). Re-record with
    `POWDER_RECORD=1`. `setup_reactor_demo` excluded (un-seeded global
    `fastrand`).
  - **Property tests** — camera `world↔screen` involution,
    `zoom(2)∘zoom(½)` identity (the exact property the camera bug violated),
    `pan∘unpan` identity, save encode/decode round-trip, chunk
    `expanded_active ⊇ active_chunks`.
  - **Physics invariants** — absorber set matches `absorber_chance` (the
    iodine-class bug, made a CI check), registry covers every id 0..=47,
    registry vs core `density_for_id` agree.
- **Added** `ci/check_powder_claims.py` — asserts every measured number in
  ROADMAP §1 plus the layering invariant (`core` has no render/ui/io/wasm
  imports; checked by import pattern, not bare substring, so
  `GridSnapshot.pixels` the field is not confused with the `pixels` crate).
- **Added** `ci/bench_record.py` — criterion → CSV harness; records a row,
  bootstraps `bench/baseline.csv` on first run, fails CI on > 15% regression.
- **Changed** `.github/workflows/ci.yml` — matrix of `lint`, `claims`, `test`,
  `bench`, `wasm` jobs.
- **Test gate:** 63 tests green (7 core + 3 io + 10 p0_rig + 43 simulation);
  claim checker 13/13.

### Bug fixes — 2026-08-24
*Deliverable: `bugfixes.patch`*

- **Fixed** GIF89a LZW encoder code-size bump off-by-one
  (`crates/io/src/gif89a.rs`). The width was increased one entry too early
  (`next_code == (1 << code_size)` → `+ 1`), producing GIFs no standard
  decoder (browsers, libgif, Pillow) could read once the dictionary crossed
  the first code-size boundary. Added a round-trip regression test with an
  embedded decoder.
- **Fixed** camera zoom anchoring to the world origin
  (`crates/renderer/src/camera.rs`). `screen_to_world` was called *after* the
  scale changed, so the offset was never recomputed; zoom now anchors to the
  cursor.
- **Fixed** iodine-135 absorber accounting (`crates/core/src/reactions.rs`).
  Iodine was counted toward `k_effective`'s absorber total but
  `absorber_chance(IODINE, _)` returned 0, so the dedicated iodine branch was
  dead code. Added `IODINE` arms (thermal 0.35, fast 0.12).
- **Fixed** Line tool double-paint (`src/main.rs`). The Line tool painted a
  brush stamp at every cursor position during the drag *and* committed the
  line on release; it now commits only on release, matching Rectangle/Copy.

---

## [0.1.0] — upstream baseline

The MVP as cloned from `github.com/AlexanderNyr/AuraLite-Powder`: a 47-element
falling-sand cellular automaton with a two-energy-bin nuclear model, a CPU
renderer that software-rasterizes its own egui, versioned saves, a WASM
canvas shim, 6 missions and 9 scenarios, 43 integration + 9 unit tests.

This is the tree ROADMAP §1 measures and every later phase refactors.

---

### Conventions

- A phase entry cites its `.patch` deliverable and its test gate.
- "Added / Changed / Fixed / Removed / Deprecated" follow Keep-a-Changelog.
- Dates are ISO 8601 in the Europe/Moscow timezone.
- The layering invariant (ROADMAP §4) is non-negotiable; a change that breaks
  it fails `check_powder_claims` and does not ship.
