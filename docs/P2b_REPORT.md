# P2b — Parallel Gravity Pass — Report

**Phase:** P2b (ROADMAP; completes P2 alongside P2a)
**Baseline:** through `patches/p8_elements.patch`
**Deliverable:** `patches/P2b_gravity.patch`
**Status:** ✅ COMPLETE — determinism gate green, safe (no `unsafe`), ships alone

> P2b parallelises the falling-sand gravity pass — the most expensive per-tick
> pass and the one P2a left sequential. The hard part was never raw
> parallelism; it was doing it **without `unsafe`** (project policy), without
> locks, and **deterministically** so a replay hash still means something.

---

## Design: per-chunk local simulation + sequential border pass

Three phases per tick, for grids ≥ 65 536 cells (smaller grids keep the
sequential path — one code path for the golden corpus and replay hash):

```
A. PARALLEL    each active chunk → simulate_chunk(): copy the region out into a
               local Grid + VelocityField, run the standard bottom-up sweep on
               it (chunk borders act as walls), return the buffer.
B. WRITE-BACK  diff each buffer against the source, write only changed cells.
               Chunks are disjoint → order irrelevant.
C. BORDER PASS sequential sweep over the edge ring of every active chunk;
               unflagged edge particles are re-run against the full grid, so
               they can cross chunk borders. FLAG_MOVED keeps a crossing to at
               most one extra tick.
```

Every update function (`update_cell`, `update_powder`, `update_liquid`,
`update_gas`, `move_radiation`, all `try_*`/`swap_cells`) is **reused
unchanged** — the local Grid is a real Grid, so the entire sequential physics
runs verbatim on 32×32 tiles. That is the design's chief virtue: one physics
implementation, two schedulers.

### Why this is safe

During phase A the shared grid is only read; every task mutates a buffer it
owns. No locks, no atomics, no `unsafe` — the project's "no unsafe unless
justified" policy holds, unlike the raw-pointer 4-colouring shortcut this
report explicitly rejects.

### Why this is deterministic

1. Per-chunk RNG seeds are drawn from the shared RNG **before** the parallel
   section, in fixed chunk order.
2. A chunk's result depends only on the start-of-pass state + its own seed.
3. Write-backs are disjoint.
4. The border pass is sequential.

Nothing depends on the rayon schedule. The same grid produces the same result
on 1, 2, or N threads.

### Cost control

- Copy-in copies **only non-empty cells** (the local grid starts as air).
- Write-back **diffs** — resting chunks, the common case, write nothing back.
- The border pass pre-filters to **occupied, unmoved** ring cells.
- Cross-border movement costs a particle at most one extra tick (the wall
  pause), which the tests accept as the parallel path's semantics.

---

## Test gates ✅

```
$ cargo test --test p0_rig
test deterministic_across_thread_counts ... ok   # 256² takes the PARALLEL path now
test parallel_physics_moves_particles    ... ok   # guards against a broken write-back
test particles_cross_chunk_borders       ... ok   # sand settles through several chunk
                                                  # boundaries into the bottom chunk
test golden_tick_corpus                   ... ok   # small grids: sequential, unchanged
```

- `deterministic_across_thread_counts` (from P2a) now exercises the parallel
  physics path — 1/2/4-thread fingerprints identical.
- `parallel_physics_moves_particles`: sand spans several chunks and must fall.
- `particles_cross_chunk_borders`: a full-width sand layer in the top chunk
  (rows 0–31) must reach the bottom chunk (rows 224+) through seven 32-row
  boundaries — proving the border pass works.
- Full suite: **64 integration + 10 unit tests green**; golden corpus and the
  P9a replay hash unchanged (both stay on the sequential path); fmt + clippy
  clean; claim checker 13/13; the feature-gated suites (`thermal-pde`,
  `fluid-pde`) still green.

---

## Perf — measured, and honest

2-core sandbox, release build, isolated physics pass, 512² half-full grid:

| | sequential | parallel | speedup |
|---|---:|---:|---:|
| physics pass | 4.34 ms | 4.60 ms | ×0.94 |
| overhead floor (static solids) | — | 2.55 ms | — |

The safe local-copy design carries a **~2.5 ms/pass overhead floor**
(copy-in + diff write-back + border pass), which on two cores exactly cancels
the halving of the sim work. The pass-level model is `work/N + floor`:
break-even at 2 cores, ~×1.8 estimated at 8 cores.

**The roadmap's "≥ 4× on 8 cores at 1024²" for the whole tick is not met by
physics parallelisation alone**, and this phase does not claim it. The tick at
512² is ~13.6 ms of which the gravity pass is ~4.3 ms (32%); the remainder —
`powder_overburden_slide` (hydro), `step_devices` (which allocates and scans
full-grid id/lifetime snapshots every tick), `refresh_chunks` (a full-grid scan
every tick), and the pressure solver — is sequential and dominates. Meeting a
4× *tick* target needs those parallelised too: that is **P2c**, and the
structure established here (disjoint chunk ownership + per-chunk RNG + a
sequential reconcile pass) is the template for it.

Two cheaper wins are also queued for P2c, both found while measuring:
`step_devices` builds two full-grid `Vec`s every tick even when no devices
exist, and `refresh_chunks` rescans the whole grid every tick despite the
chunk pool existing to avoid exactly that.

---

## Roadmap status after P2b

| Phase | Status |
|-------|:------:|
| P0, P1, P2a, P2b, P3, P5b, P8 (+elements), P9a | ✅ |
| P2c (parallel hydro/devices + scan eliminations) | ☐ next |
| P4 (multi-group) | ☐ — blocked on the 2-neutron-element architecture |
| P5a, P6, P7, P9b | ☐ |

*Structure inherited from `P0_REPORT.md` … `P9a_REPORT.md`.*
