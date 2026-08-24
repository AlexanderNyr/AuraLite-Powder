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
