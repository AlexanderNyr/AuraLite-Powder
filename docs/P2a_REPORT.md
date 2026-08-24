# P2a — Parallel Passes: Deterministic Reactions + Parallel Heat — Report

**Phase:** P2a (split from P2; ROADMAP)
**Baseline:** through `patches/P1_soa.patch`
**Deliverable:** `patches/P2a_parallel.patch`
**Status:** ✅ COMPLETE — determinism gate green, ships alone

> P2 was always the hard phase, and it has two independent costs: making the
> *parallel* passes reproducible, and parallelising the *order-dependent*
> gravity pass. The first is done here (P2a); the gravity pass's halo+stitch
> is P2b. The split follows the A5a/b/c precedent — a phase grew two costs, so
> each gets its own gate and patch.

---

## The headline: a reproducibility bug, found and fixed

`reaction_pass_parallel` collected fissile/decay candidates with a `par_iter`
over chunks:

```rust
let chunk_results: Vec<ChunkResult> = chunk_coords.par_iter().map(...).collect();
for (f, d, fu) in chunk_results { fissile_to_check.extend(f); decay_to_check.extend(d); ... }
fusion_pairs.sort_unstable(); fusion_pairs.dedup();   // ← only fusion was sorted
```

`par_iter`'s `collect` preserves each chunk's own order, but the *inter-chunk*
order is whatever the rayon scheduler produced — which changes with the thread
count. `fusion_pairs` was sorted (so fusion was fine), but `fissile_to_check`
and `decay_to_check` were extended in scheduler order, then iterated by the
rng-driven `apply_collected_reactions`. **Same grid, different thread count →
different fission/decay outcomes.** A 1024² reactor run was non-reproducible.

The fix is one line each — sort and de-dup the candidate lists before the
application — but the *test* is the point: without it, the bug is invisible
until a user reports "my save plays back differently".

---

## The second piece: parallel heat

`diffuse_heat_active` is a Jacobi sweep: every cell's next temperature is a
function of the *current* (read-only) neighbour temperatures, written to a
disjoint `next[idx]`. That is the textbook embarrassingly-parallel pattern:

```rust
let grid_ref: &Grid = grid;                 // shared, read-only during compute
next.par_iter_mut().enumerate().for_each(|(idx, slot)| {
    *slot = heat_step_cell(grid_ref, x, y, idx, rate);   // each thread writes its own slot
});
```

`heat_step_cell` is factored out so the sequential (`< 65536` cells) and
parallel (`≥ 65536`) solvers share one implementation and cannot drift. Heat
has no RNG, so it is deterministic by construction.

---

## Test gate — determinism (the P2 gate that matters) ✅

```rust
#[test]
fn deterministic_across_thread_counts() {
    // 256² = 65 536 cells → parallel path. Ticked 25× under rayon pools of
    // 1, 2, 4 threads via ThreadPoolBuilder + pool.install(); fingerprints must match.
}
```

```
$ cargo test --test p0_rig
test deterministic_across_thread_counts ... ok
test golden_tick_corpus ... ok            // 128² scenes still sequential, unchanged
... 11 passed
```

The golden corpus is unchanged because the 128² scenes (16 384 cells) take the
sequential path (`< 65536`), which P2a never touches. **64 tests green** (7
core + 3 io + 11 p0_rig + 43 simulation); claim checker 13/13.

---

## Perf — measured

Release (no-LTO, codegen-units 16, ~2 cores), median per tick:

| bench | original AoS (§1) | P1 SoA | **P2a** | P2a vs P1 | P2a vs AoS |
|-------|------------------:|-------:|--------:|----------:|-----------:|
| `simulation_tick_256` | ~13.6 ms | 4.9 ms | **4.6 ms** | ~1.07× | ~2.96× |
| `simulation_tick_512` | ~60.9 ms | 20.5 ms | **13.2 ms** | ~1.55× | ~4.61× |

The 512² win is larger because the heat pass is O(all cells) every tick and is
a bigger fraction of a 512² tick than a 256² one; parallelising it pays more
there. (This sandbox has ~2 cores, capping the parallel speedup; on the 8-core
runner the gate targets, both parallel passes scale further.)

---

## What is deliberately NOT in P2a

- **The gravity (falling-sand) pass is still sequential.** It is order-dependent
  (the `FLAG_MOVED` bottom-up cascade) and cannot be parallelised the way heat
  can — a sand grain falling out of chunk A into chunk B races with B's own
  update. The safe, no-`unsafe` way is the halo+stitch: each chunk processes its
  interior, cross-border moves are queued, a sequential stitch resolves them.
  That is **P2b**, and it is the pass that owns the "≥ 4× on 8 cores" perf gate.
- **No `unsafe`.** The project policy ("no unsafe unless justified for WASM
  interop") rules out the 4-colouring-with-raw-pointers shortcut; P2b's
  halo+stitch stays in safe Rust by construction.

---

## Unblocks

- **P9 (replay)** — reproducibility across thread counts is exactly what a
  headless replay hash needs. P2a's determinism is P9's foundation; P9 extends
  it to "same hash across machines".
- **P2b (parallel gravity)** — the halo+stitch now has a determinism test to
  lean on: once the gravity pass is parallelised, the same
  `deterministic_across_thread_counts` gate proves it stayed reproducible.

## Roadmap status after P2a

| Phase | Status |
|-------|:------:|
| P0 | ✅ |
| P1 | ✅ |
| P2a | ✅ |
| P2b | ☐ next |
| P3–P9 | ☐ |

*Structure inherited from `P0_REPORT.md` / `P1_REPORT.md`.*
