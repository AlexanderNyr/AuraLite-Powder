# P9b — Hardening: Codec Fuzzing — Report

**Phase:** P9b (ROADMAP)
**Baseline:** through `patches/P2c_scans.patch`
**Deliverable:** `patches/P9b_fuzz.patch`
**Status:** ✅ COMPLETE — the fuzzer found three real bugs; all fixed and gated

> The roadmap's argument for this phase was the GIF bug itself: "a codec that
> had a round-trip test *for the header only* shipped broken for two years."
> P9b threw fuzzing at every codec — and the codecs obliged: **three real,
> crash-grade bugs** surfaced on the first runs, exactly the class the phase
> existed to catch.

---

## The fuzzer

Deterministic (local xorshift, seeded — no libFuzzer, no nightly, runs in the
plain `cargo test` suite in CI). Inputs:

| test | input |
|------|-------|
| `save_decode_fuzz_random_buffers` | 2000 random buffers (0–600 B), both compression flags, plus the replay path |
| `save_decode_and_apply_fuzz_mutations` | every byte of a valid save flipped to `0x00`/`0xFF`/`0x7F`; decode **and** `apply_to` |
| `save_decode_fuzz_truncations` | every prefix of a valid save (the torn-write case) |
| `save_with_absurd_grid_dimensions_is_rejected` | crafted save: `u32::MAX × u32::MAX` grid, zero payload |
| `save_with_absurd_length_claim_is_rejected_at_decode` | a 2³²-particle length varint spliced into an otherwise valid save |
| `gif89a::tests::encode_fuzz_arbitrary_frames` | 200 random frames incl. deliberately wrong lengths |
| `gif89a::tests::roundtrip_fuzz_random_frames` | 60 random frames → encode → decode → exact match |

## The three bugs (found in order, fixed in order)

### 1. i8 velocity overflow — `physics.rs`

A crafted save with `vel_y = 127` hit `vy + 1` in `update_powder` /
`update_liquid`: **debug builds panicked** ("attempt to add with overflow"),
release builds silently wrapped 127 → −128. Three sites fixed with
`saturating_add`. The velocities are serialized save state, so this was
reachable from any corrupted file.

### 2. The grid allocation bomb — `save.rs`

Compact saves store only non-empty particles, so the *bytes say nothing about
the claimed area*. A save with `grid_width = grid_height = u32::MAX` and zero
particles is a few dozen bytes on disk — and demanded exabytes in `Grid::new`,
aborting with "capacity overflow" (or, for dimensions that fit `isize`,
OOM-killing the process). Fixed with dimension guards:

```rust
pub const MAX_GRID_SIDE: u32 = 8192;          // gameplay tops out at 1024²
pub const MAX_GRID_AREA: u32 = 16_777_216;    // 4096²
// -> IoError::GridTooLarge { width, height }
```

### 3. GIF encoder partial-pixel panic — `gif89a.rs`

A frame whose length is not a multiple of 4 leaves a trailing partial pixel;
`px[2]` indexed out of bounds → panic in `encode_rgba_frames`. Now indexed
defensively (`px.get()` with 0 defaults).

## Plus the decode limit

bincode 2.0 pre-allocates `Vec::with_capacity(len)` from a container's length
varint **before decoding a single element** — a crafted length claims its
bytes up front (64 GiB for a 2³²-particle claim; fatal wherever overcommit
doesn't save you, e.g. WASM). The save decoder now runs
`bincode::config::standard().with_limit::<256 MiB>()`; the claim is rejected
at the decoder. 256 MiB is ~30× a full 1024² save, so nothing legitimate is
affected.

---

## Gates ✅

- 5 save-fuzz + 2 GIF-fuzz tests, all green (each was red against the bug it
  names — the honest fix cycle: fuzz → fail → fix → green).
- Full suite: **81 integration + 12 unit tests** green; golden corpus and
  P9a replay hash unchanged; fmt + clippy clean; claim checker 13/13; the
  thermal-pde / fluid-pde feature suites green.

## What is deferred

- **WASM threads** (`wasm-bindgen-rayon` + COOP/COEP): needs browser
  verification the sandbox cannot provide; the WASM build itself stays green
  in CI.
- **Save v3:** still no incompatible state — P4 (enum-append) and P5a
  (emergent behaviour) both stayed v2-compatible by design, so a v3 with a
  migration path has nothing to migrate.

## Roadmap status after P9b

| Phase | Status |
|-------|:------:|
| P0, P1, P2a, P2b, P2c, P3, P4, P5a, P5b, P8 (+elements), P9a, P9b | ✅ — 12 phases |
| P6 (GPU — needs hardware), P7 (UI — needs a display) | ☐ |

*Structure inherited from `P0_REPORT.md` … `P2c_REPORT.md`.*
