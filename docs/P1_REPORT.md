# P1 — SoA Particle Layout — Completion Report

**Phase:** P1 (ROADMAP)
**Baseline:** `main` + `bugfixes.patch` + `patches/P0_rig.patch` + `patches/P1_plan.patch`
**Deliverable:** `patches/P1_soa.patch`
**Status:** ✅ COMPLETE — both gates green, ships alone

> P1 turns the grid's storage from an array of structs into a struct of arrays.
> It is the textbook cache refactor, and its gate is the unforgiving one:
> *outputs must not change*. The P0 golden corpus — built precisely so a layout
> refactor could be proven behaviour-preserving — is the gate, and it is green.

---

## What shipped

`Grid` storage moved from `particles: Vec<Particle>` to four parallel arrays:

```rust
pub struct Grid {
    pub width: u32,
    pub height: u32,
    element_ids: Vec<u16>,
    temperatures: Vec<u16>,
    flags: Vec<u8>,
    lifetimes: Vec<u8>,
}
```

A scan that reads `element_id` for every cell now streams a contiguous `u16`
array — a 64-byte cache line carries **32 ids** instead of the **4** an AoS
line carried (one id used out of every eight-byte particle). The single-cell
physics pass keeps the same access pattern through cheap field accessors
(`Particle` is `Copy`, eight bytes).

### The accessor surface (the part that made the refactor tractable)

The blocker for SoA in this codebase was `get_mut -> &mut Particle`: a borrow of
one `Particle` cannot span four arrays. The replacement is a closure that does
read–modify–write in one call:

```rust
pub fn modify<F: FnOnce(&mut Particle)>(&mut self, x: u32, y: u32, f: F) -> bool
```

Plus index/field accessors for hot loops (`element_at`, `temperature_at`,
`add_temperature_at`, `or_flag_at`, `clear_flag_at`, …), contiguous slice
accessors for scans (`element_ids()`, `temperatures()`, `temperatures_mut()`),
`swap_particles` (the physics `swap_cells` inner op, now a four-array swap),
and `particles_vec()` / `with_particles()` / `set_particles_vec()` at the
serialize/undo boundary so **save format v2 is unchanged**.

Every consumer moved off the `particles` field and `get_mut`: `physics`,
`simulation`, `hydro`, `devices`, `missions` (core), `io/save`,
`renderer/compose`, `renderer/overlay`, `ui/app_state`.

---

## Test gate — correctness (D2) ✅

The P0 golden tick corpus is the gate, and it is **byte-for-byte unchanged**:

| Scene | count | hash | vs P0 |
|-------|------:|------|:-----:|
| `sand_pile` | 1280 | `0x2debc27eea498cf2` | identical |
| `water_basin` | 2808 | `0x14d93f86e6910b6f` | identical |
| `scenario_hourglass` | 4349 | `0x96c4c0107286234d` | identical |
| `scenario_bomb` | 368 | `0xd3ca58632f8f3f53` | identical |
| `scenario_coolant_loop` | 826 | `0xf6e5d2ba81977e49` | identical |
| `scenario_ice_melt` | 4694 | `0xc4524489bd792166` | identical |

```
$ cargo test --tests
test result: ok. 7 passed   (core)
test result: ok. 3 passed   (io)
test result: ok. 10 passed  (p0_rig — incl. golden_tick_corpus)
test result: ok. 43 passed  (simulation_tests)
```

**63 tests green.** The refactor is behaviour-preserving — this is the harder
of P1's two gates, and it is the one that matters. Full GUI path `cargo
check`s; `check_powder_claims` 13/13 (the layering invariant holds: `core`
still imports no render/ui/io/wasm types, and `GridSnapshot.pixels` the field
is still not confused with the `pixels` crate).

---

## Test gate — perf ✅

Release (no-LTO, codegen-units 16), half-full grid of sand, median per tick:

| bench | AoS (§1 baseline) | SoA (this phase) | speedup |
|-------|------------------:|-----------------:|--------:|
| `simulation_tick_256` | ~13.6 ms | **4.9 ms** | ~2.8× |
| `simulation_tick_512` | ~60.9 ms | **20.5 ms** | ~2.9× |

Both clear the **1.40×** gate by a wide margin. The §1 baseline was recorded
on this sandbox in a prior run; the authoritative comparison is
`ci/bench_record.py` against `bench/baseline.csv` in a release-LTO CI build
(the sandbox cannot produce an LTO build reproducibly inside one turn, so the
no-LTO number is the indicative measurement and the CI number is the record of
record).

The size of the win matches the cache-mechanics argument: sand physics is
scan-dominated (refresh + reaction-collection + heat all walk the active set
reading `element_id`), which is exactly the workload AoS penalises most.

---

## Bugs found and fixed during the phase

Every refactor of this size drifts in a few places; the compiler and the
golden corpus caught all of them before merge:

1. **`*grid.get_mut(x,y).unwrap() = new_p`** (physics `move_radiation`) — a
   deref-assign through a borrow, which owned-`get` cannot express. →
   `grid.set(x, y, new_p)`.
2. **`else if let Some(t) = grid.get_mut(…)` arms** (devices `tick_fire`,
   simulation `trigger_tnt`/`process_neutron_queue`) — the bulk
   `get_mut → modify` transform turned `else if let … { }` into the invalid
   `else <stmt>`. Three sites rebuilt as `else { grid.modify(…) }`.
3. **`get_mut` bodies containing `match { }` / nested `if { }`** (physics
   radiation penetration, simulation fission heat-neighbours) — the regex
   transform's "body up to first `}`" stopped at the inner brace, leaving
   unbalanced delimiters. Two sites rebuilt by hand.
4. **`Grid { width, height, particles }` struct literal** (io/save `to_grid`)
   — no longer constructible. → `Grid::with_particles(w, h, full)`.

None reached the golden corpus; the type checker found all four. The corpus's
job here was to confirm the *logic* was preserved, which it did.

---

## What the refactor does *not* change

- **Save format v2** is identical on the wire. `SaveFile` still serialises a
  `Vec<Particle>`; `with_particles`/`particles_vec` are the only places AoS is
  materialised. A v2 file from before P1 loads unchanged; the migration path
  (D9) is untouched.
- **The public `Grid` API surface used by `core` consumers** is the same shape
  (`get`/`set`/`in_bounds`/`index`/`clear`/`resize`/`count_non_empty`/
  `to_compact`/`from_compact`/`to_rgba_buffer`); `get` returns an owned
  `Particle` instead of `&Particle`, which is transparent because `Particle`
  is `Copy`.
- **Visitation order, RNG seeding, every probability** — unchanged. That is
  what the golden corpus proves.

---

## Unblocks

- **P2 (parallel physics)** — the four-array layout splits across threads with
  no aliasing concerns (a chunk owns disjoint index ranges in four arrays),
  and `swap_particles` is the unit the cross-chunk stitch will reason about.
- **P6 (GPU compute)** — the SoA buffers map almost directly to WGSL storage
  buffers; a compute shader implementing a tick becomes "the third backend"
  for the same logic the golden corpus pins.

## Roadmap status after P1

| Phase | Status |
|-------|:------:|
| P0 | ✅ |
| P1 | ✅ |
| P2–P9 | ☐ |

*Structure inherited from `BL1_REPORT.md` / `P0_REPORT.md`: measured numbers,
what-shipped / what-was-caught / what-is-unchanged, unblocks.*
