# P5a — Isotope Model: U-238 Breeding + Depletion — Report

**Phase:** P5a (ROADMAP)
**Baseline:** through `patches/P4_transport.patch`
**Deliverable:** `patches/P5a_isotope.patch`
**Status:** ✅ COMPLETE — all 4 gates green, zero drift on every existing gate

> P5a adds the reaction that makes a fuel *cycle* rather than a fuel *pile*:
> U-238 + n → Pu-239. Before this phase, U-238 was mostly dead weight — a
> threshold fissile that barely fissioned. Now it breeds plutonium, the
> plutonium is fissile, its fissions breed more, and enrichment becomes a
> living quantity the player can watch fall as a reactor burns.

---

## What shipped

### Breeding (`reactions.rs` + two hooks in `simulation.rs`)

```rust
pub fn u238_capture_chance(energy) -> f32;   // thermal 0.25 / epi 0.20 / fast 0.15
```

When an incident neutron fails to **fission** a U-238 nucleus, it may instead
be **captured**, converting the cell to Pu-239 (+60 K capture heat). Hooked in
both neutron paths so the rate is the same whether the neutron arrives as a
queue event (moderation/direct-hit) or as a walking particle (adjacency):

- queue path: the event is consumed by the capture (it was spent either way);
- adjacency path: the particle is not consumed, mirroring how fission treats
  it there.

The closed cycle this creates, entirely from pre-existing elements:

```
U-238 + n → Pu-239        (breeding — NEW)
Pu-239 + n_thermal → fission (0.90 — existed)
fission → 2-4 fast n       (existed)
fast n → moderator → epithermal → thermal   (P4)
```

### The RNG discipline that kept the corpus byte-identical

The capture roll only draws RNG when the cell **is U-238**: the condition
short-circuits before the draw for every other isotope. No golden-corpus
scene contains U-238 (the demo/bomb/coolant piles are U-235 and Pu-239), so
the RNG stream is untouched in all six scenes — verified: **all fingerprints
and the P9a replay hash are byte-identical** after the phase.

---

## Test gates ✅ (4 new)

| Gate | Checks |
|------|--------|
| `u238_breeds_pu239` | a U-238 block under a staggered thermal flux grows > 3 Pu-239 cells (and fissions happen) |
| `enrichment_drops_as_pile_burns` | a 50%-enriched checkerboard in a graphite wrapper: the U-235 fraction falls by > 10 points — thermal flux hits the 0.85-vs-0.02 asymmetry, not the fast 0.35-vs-0.25 near-tie |
| `enrichment_raises_measured_k` | the SAME moderated geometry, pure U-235 core vs 20% checkerboard: the pure core out-multiplies (measured with P4's `k_measured`) — the enrichment/critical-mass tie-in the roadmap asked for |
| `u238_capture_is_energy_ordered` | thermal > epithermal > fast, like every absorber |

The enrichment test is the phase's proof of depletion: burning preferentially
removes the fissile isotope, exactly as real fuel burns down — and the
moderator wrapper is what makes it visible (fast neutrons would otherwise
fission both isotopes at nearly the same rate, a subtlety the first draft of
the test missed and the graphite wrapper fixed).

**Everything else stays green:** 75 integration + 10 unit tests; golden corpus
byte-identical; replay hash unchanged; fmt + clippy clean; claim checker
13/13; the feature-gated suites (thermal-pde, fluid-pde) still green.

---

## What is deliberately deferred

- **Save format v3** — the roadmap queued it for "when P4/P5a add new state";
  neither did (P4 used serde-defaults + enum-append; P5a's breeding is
  emergent from existing elements), so v2 remains fully compatible and v3 is
  deferred until a phase genuinely needs incompatible state.
- **Per-cell isotope vectors / waste signature** — heavy persistent state for
  little observable behaviour; the enrichment *observable* (element-count
  ratios) already answers "how burnt is this pile".
- **Full decay chains to lead** — would need ~6 new elements (Th, Pa, Ra, Rn,
  Po, Pb) for a mostly-invisible bookkeeping chain; the one-step chains remain
  the documented simplification.

## Roadmap status after P5a

| Phase | Status |
|-------|:------:|
| P0, P1, P2a, P2b, P3, P4, P5a, P5b, P8 (+elements), P9a | ✅ — 10 phases |
| P2c (parallel hydro/devices), P6 (GPU), P7 (UI), P9b (fuzz/WASM-threads) | ☐ |

*Structure inherited from `P0_REPORT.md` … `P4_REPORT.md`.*
