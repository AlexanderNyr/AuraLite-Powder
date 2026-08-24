# P9a — Hardening: Headless Replay + Long-Run Hash — Report

**Phase:** P9a (split from P9; ROADMAP)
**Baseline:** through `patches/ci_green.patch`
**Deliverable:** `patches/P9a_replay.patch`
**Status:** ✅ COMPLETE — gate green across dev + release, ships alone

> P9a turns the determinism P2a paid for into a tool and a gate. The golden
> corpus (P0) covers 150 ticks; this phase covers 1 000, and ships a replay
> binary so a bug report can say "save X replays to hash Y" and a reviewer can
> reproduce it bit-for-bit.

---

## What shipped

### `aura_lite_io::replay`

```rust
pub fn replay_hash(&mut SimulationState, ticks: u64) -> u64;   // run + hash
pub fn grid_layout_hash(&Grid) -> u64;                         // hash of a grid
pub fn replay_save_bytes(&[u8], ticks: u64) -> Result<u64, IoError>; // one-call
```

### The f32 question — answered by hashing ids only

The heat solver is f32; its rounding is **not** stable across compilers /
optimisation levels, so hashing temperatures would make the gate flaky across
builds. Element positions, by contrast, are driven by the integer cellular
automaton plus the per-tick deterministic RNG — and for a thermally-inert scene
(uniform 293 K, no fission) no temperature threshold ever fires, so the layout
is f32-independent. `grid_layout_hash` therefore hashes the **element-id array
only**:

| build | 1 000-tick hash |
|-------|----------------:|
| dev (`cargo test`) | `0x86bf17c0b45557f3` |
| release (`cargo test --release`) | `0x86bf17c0b45557f3` |

Identical — the gate is build-robust, which is what a regression hash must be.

### `examples/replay.rs`

A headless tool: `cargo run --release --no-default-features --example replay --
save.aura 1000` → `replay 1000 ticks -> layout hash 0x…`. For reproducible bug
reports.

---

## Test gate ✅

```
$ cargo test --test p9_replay
test replay_hash_stable_1000_ticks ... ok      # baked 0x86bf17c0b45557f3
test replay_hash_is_deterministic   ... ok
```

- The 1 000-tick hash is a **long-run regression gate**: any model change that
  alters the element layout over 1 000 ticks flips it (re-record with
  `POWDER_RECORD=1` only after a reviewed change — the same discipline as the
  golden corpus).
- `replay_hash` is a pure function of `(state, seed, ticks)` — same input,
  same hash.

70 default tests green (60 integration + 10 unit); `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` both clean.

---

## What is deferred (P9b)

- **`cargo fuzz` targets** (save/plugin/gif decode) — the GIF round-trip bug
  class argues for them.
- **Save format v3** — needed only when P4/P5a add new state (isotope vectors,
  group structure); v1→v2 migration pattern is the template.
- **Cross-machine replay** — within a build the hash is stable; *across*
  compilers it can still drift if a thermally-active scene is replayed. True
  cross-machine determinism needs a deterministic-accumulation heat solver (or
  fixed-point), a deeper change.
- **WASM threads** (`wasm-bindgen-rayon` + SharedArrayBuffer/COOP-COEP).

## Roadmap status after P9a

| Phase | Status |
|-------|:------:|
| P0, P1, P2a, P3, P8, P9a | ✅ |
| P2b (parallel gravity) | ☐ |
| P4 (multi-group) | ☐ — blocked on the 2-neutron-element architecture |
| P5a, P5b, P6, P7, P9b | ☐ |

*Structure inherited from `P0_REPORT.md` … `P8_REPORT.md`.*
