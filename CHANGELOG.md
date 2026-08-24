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
