# P2c — Scan Elimination: Classify-Once Gating + Parallel Pressure — Report

**Phase:** P2c (ROADMAP; the follow-up P2b's report queued)
**Baseline:** through `patches/P5a_isotope.patch`
**Deliverable:** `patches/P2c_scans.patch`
**Status:** ✅ COMPLETE — zero drift on every existing gate, ×1.65 on the 512² bench

> P2b's report ended with two named suspects: `step_devices` building two
> full-grid snapshots every tick even with no devices, and `refresh_chunks`
> rescanning the whole grid despite the chunk pool existing to avoid it. P2c
> profiled the tick, confirmed both, and fixed them — plus found the phase's
> real difficulty: skipping a pass is only free if it never draws rng.

---

## Profile first (2-core sandbox, release, 512² half-full sand)

| pass | ms/tick | note |
|------|--------:|------|
| physics (P2b parallel) | 4.11 | real work |
| **devices** | **3.52** | **pure waste** — 2 full-grid snapshot Vecs + 3 pressure scans, zero devices |
| reactions + effects | 3.89 | mostly parallel already |
| overburden | 0.86 | real work (powder) |
| pipes | 0.47 | **waste** — no pipes in scene |
| refresh | 0.38 | mandatory scan |
| equalize | 0.19 | no liquids |
| hydrostatic | 0.30 | no liquids |
| **full tick** | **13.71** | |

## What shipped

### 1. Classify-once gating

`refresh_chunks` is already the one mandatory full-grid scan. It now also
counts **liquid / powder / device / pipe** cells (`liquid_cells`,
`powder_cells`, `device_cells`, `pipe_cells` on `SimulationState`), and
`tick()` skips a whole pass when its class is absent:

```
equalize              — never gated (see the rng rule)
overburden            — gated on powder_cells
step_devices          — gated on device_cells (loop + snapshots)
  diffuse_pressure    — gated on live pressure (no rng)
  apply_pressure_flow — NEVER skipped (per-row rng.bool())
  apply_overpressure  — gated on live pressure (no rng)
add_hydrostatic       — gated on liquid/pipe cells (takes no rng)
step_pipe_network     — gated on pipe_cells (no unconditional draws)
```

The counters are an **upper bound** at gate time — nothing creates these
elements between refresh and the gated pass — so a skip is exactly a no-op
skip and a stale-positive count merely runs a no-op pass.

### 2. The rng-stream rule (the phase's real lesson)

A pass may be skipped **only if it draws no rng when its class is absent**.
Two passes violate that and are therefore never skipped:

- `equalize_liquid_surface` — draws `rng.bool()` once unconditionally;
- `apply_pressure_flow` — draws `rng.bool()` **per row** unconditionally.

The golden corpus caught both violations as drifts (first the equalize skip,
then the pressure-flow skip), each root-caused and fixed before merge — the
corpus doing precisely the job P0 built it for. The rule is now documented at
the gating site so the next phase doesn't relearn it.

### 3. Cheaper pressure

- `PressureField` gained a reused **scratch** buffer: `diffuse_pressure` no
  longer clones a full-grid Vec every tick (take → compute → swap).
- `diffuse_pressure` is **parallelised** (rayon over rows). It is a Jacobi
  sweep — every `next[i]` depends only on the *current* field and the grid's
  element ids — with pure integer arithmetic and no rng, so the result is
  **bit-identical at any thread count** (unlike an f32 heat solver).

### 4. Cheaper refresh

The per-cell `chunk_pool.get_mut(x/32, y/32)` + `mark_dirty` calls became one
`meta.activate()` per occupied chunk (a new `ChunkMeta::activate` that skips
the unused dirty-bbox bookkeeping). The active set is identical.

---

## Measured

| benchmark | before P2c | after P2c | speedup |
|---|---:|---:|---:|
| `simulation_tick_256` (criterion median) | 4.33 ms | **2.88 ms** | ×1.50 |
| `simulation_tick_512` (criterion median) | 13.6 ms | **8.23 ms** | ×1.65 |
| devices pass (device-free scene) | 3.52 ms | **0.09–0.22 ms** | ~×20 |
| water scene (live pressure) full tick | 13.6 ms | 12.7 ms | ×1.07 |

(Criterion medians, release build, 2 cores; the water scene keeps its pressure
solver because hydrostatic pressure is genuinely live there.)

---

## Gates ✅

- **Zero drift:** golden corpus byte-identical (all six scenes), P9a replay
  hash unchanged, P2a/b `deterministic_across_thread_counts` green.
- **New unit gate:** `classify_counters_match_grid` — the counters that gate
  physics must exactly match a known grid (8 water / 12 sand / 3 pipe / 3
  device cells).
- **Full suite:** 76 integration + 10 unit tests; fmt + clippy clean; claim
  checker 13/13; thermal-pde / fluid-pde feature suites green.

## What is deliberately not in P2c

- **Parallel `powder_overburden_slide`** — profiled at 0.86 ms (6% of the
  tick); the chunk-local + border-pass machinery it would need costs more
  complexity than the win justifies at 2 cores.
- **Gating inside the effects pass** (phase changes / thermal effects scans) —
  the same pattern applies, but the passes live in the reactions+effects
  bucket that is already partially parallel; left for a measured follow-up.

## Roadmap status after P2c

| Phase | Status |
|-------|:------:|
| P0, P1, P2a, P2b, P2c, P3, P4, P5a, P5b, P8 (+elements), P9a | ✅ — 11 phases |
| P6 (GPU), P7 (UI), P9b (fuzz/WASM-threads) | ☐ |

*Structure inherited from `P0_REPORT.md` … `P5a_REPORT.md`.*
