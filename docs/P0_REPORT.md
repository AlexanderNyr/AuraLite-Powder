# P0 — Measurement Rig, Invariants, CI — Completion Report

**Phase:** P0 (ROADMAP)
**Baseline:** `main` + `bugfixes.patch`
**Deliverable:** `patches/P0_rig.patch`
**Status:** ✅ COMPLETE — gates green, ships alone

> P0 produces no user-visible change. It is the instruments that make every
> later phase's gate machine-checkable. This report is the single-page record
> of what landed and what each instrument already caught.

---

## What shipped

| Artifact | Role |
|----------|------|
| `tests/p0_rig.rs` | 10 tests: golden tick corpus (6 scenes), property tests (camera/save/chunk), physics invariants (absorber accounting, registry completeness, density consistency) |
| `ci/check_powder_claims.py` | asserts every "measured" number in ROADMAP §1 + the layering invariant (D6 — claim checker from birth) |
| `ci/bench_record.py` | criterion → CSV, compare to `bench/baseline.csv`, fail on > 15% regression |
| `.github/workflows/ci.yml` | jobs: `lint`, `claims`, `test`, `bench`, `wasm` (matrix) |

---

## Test gate — measured

```
$ python3 ci/check_powder_claims.py
CLAIM CHECK PASSED: every measured number in §1 holds against the tree.   (13 claims)

$ cargo test --release -p aura-lite-core -p aura-lite-io --lib   # unit
test result: ok. 7 passed   (core reactions)
test result: ok. 3 passed   (io: save round-trip, save queue/counters, GIF round-trip)

$ cargo test --tests                                          # integration
test result: ok. 10 passed (p0_rig)        ← NEW
test result: ok. 43 passed (simulation_tests)
```

**63 tests green** (was 53 after `bugfixes.patch`; +10 from P0).

---

## The golden corpus (D2's foundation)

Six deterministic scenes, 150 ticks @ seed 42, fingerprinted as
`(non-empty count, rolling hash)`:

| Scene | count | hash |
|-------|------:|------|
| `sand_pile` | 1280 | `0x2debc27eea498cf2` |
| `water_basin` | 2808 | `0x14d93f86e6910b6f` |
| `scenario_hourglass` | 4349 | `0x96c4c0107286234d` |
| `scenario_bomb` | 368 | `0xd3ca58632f8f3f53` |
| `scenario_coolant_loop` | 826 | `0xf6e5d2ba81977e49` |
| `scenario_ice_melt` | 4694 | `0xc4524489bd792166` |

Re-record with `POWDER_RECORD=1 cargo test --test p0_rig -- --nocapture` only
after a **reviewed** model change. A model change that drifts a fingerprint
without a re-record fails CI by name — exactly the gate P1 (SoA) and P2
(parallel) refactor against.

The corpus deliberately excludes `setup_reactor_demo`, which calls the
un-seeded global `fastrand::bool()` and is therefore non-deterministic across
runs. Recording that scene would produce a flaky gate; the exclusion is the
recorded reason (ROADMAP §1 Fact 10 risk note, made concrete).

---

## What the rig already caught (the audit, re-run by machine)

P0's tests are the static version of the `bugfixes.patch` audit. Each of the
four bugs is now prevented by a P0 instrument:

| bugfixes.patch bug | P0 prevention |
|--------------------|---------------|
| GIF LZW off-by-one | `prop_save_*` + the existing `gif89a` round-trip; D7 (every codec gets a round-trip test) |
| camera zoom anchoring | `prop_zoom_then_unzoom_is_identity` — the exact property the bug violated |
| iodine absorber accounting | `invariant_absorber_set_matches_absorber_chance` — counted absorbers must absorb, and vice versa |
| Line-tool double-paint | input replay lands in P7; until then, `check_powder_claims` pins the fix is present |

Two of the P0 instruments found a *real* inconsistency on first run, exactly as
designed — both were bugs in the instruments, caught and fixed before merge
(naming the lesson the ARM64 plan records for its own checkers):

1. `check_powder_claims` first counted 49 element constants, not 48 — because
   `MAX_ELEMENT_ID` matched the "u16 constant" regex. Fixed to exclude it.
2. `check_powder_claims` first flagged `core` as importing the `pixels` crate —
   because `GridSnapshot.pixels` is a **field name**, not the crate. Fixed the
   layering check to match import patterns (`use x` / `x::` / Cargo.toml dep),
   not bare substrings.

Both are the "the second consumer was the test" dynamic: an instrument that
flags a false positive is itself a reviewed artifact, not a silent pass.

---

## What is deliberately not in P0

- **`bench/baseline.csv`** is not committed. The harness bootstraps it on the
  first CI run (the missing-baseline path records and returns 0). Committing a
  baseline from a sandbox runner would pin hardware-specific numbers; the
  baseline belongs to the CI runner that owns the regression budget.
- **Property-test framework (quickcheck/proptest)** is not added. The property
  tests use a local xorshift and a fixed iteration count, which is
  deterministic-per-seed and needs no new dependency. The ROADMAP mentioned
  quickcheck; P0 ships the *intent* (mutate-and-fail properties) without the
  dep, on the "builds alone, minimal deps" principle. A later phase may promote
  to proptest if shrinking becomes valuable.
- **Headless input replay** is P7/P9 work, not P0. P0's claim checker only
  *pins* that the Line-tool fix is present.

---

## Unblocks

P0 is the prerequisite for the whole graph (ROADMAP §3). With the golden corpus
and the property layer in place:

- **P1 (SoA)** refactors the grid layout against the golden corpus — a
  fingerprint that does not change proves the layout change preserved behaviour.
- **P2 (parallel)** adds the determinism test against the same corpus.
- **P3–P5** add `physics_invariants` rows as they deepen the model.
- **P9 (replay)** extends the golden corpus's determinism into a 1 000-tick
  replay hash.

The claim checker is the thread: every later phase that cites a §1 number or
adds a cross-cutting invariant appends a claim, so two contributors never
silently disagree about what the machine does.

---

## Roadmap status after P0

| Phase | Status |
|-------|:------:|
| P0 | ✅ |
| P1 | ☐ next |
| P2–P9 | ☐ |

*Structure inherited from `BL1_REPORT.md` / `BOOTLOADER_ROADMAP.md`: one report
per phase, measured numbers, what-shipped / what-was-caught / what-is-deferred.*
