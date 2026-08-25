# P4 — Neutron Transport: 3-Group Moderation + Measured k-effective — Report

**Phase:** P4 (ROADMAP)
**Baseline:** through `patches/P2b_gravity.patch`
**Deliverable:** `patches/P4_transport.patch`
**Status:** ✅ COMPLETE — all 7 gates green, old saves decode unchanged

> P4 is where the nuclear model stops being two bins and becomes a spectrum,
> and where k-effective stops being a formula and becomes a *measurement*. The
> unlocking insight: the epithermal group never needs to exist as a grid
> particle — it is a queue-transient state between fast and thermal, so the
> 47-element registry is untouched and old saves decode byte-compatibly.

---

## What shipped

### 1. Three-group moderation (`reactions.rs`)

```rust
pub enum NeutronEnergy { Thermal, Fast, Epithermal }   // order preserves saves
pub fn moderator_downscatter(e) -> Option<NeutronEnergy>   // one group per collision
```

Before P4, a fast neutron hitting water had a chance to become thermal **in
one collision**. Now it steps down one group: fast → epithermal → thermal,
one moderator collision each. Two collisions to thermalize — as many as real
neutrons need. The thermal/fast probability constants are byte-identical to
pre-P4; epithermal sits between them everywhere:

| fission prob | thermal | epithermal | fast |
|---|---:|---:|---:|
| U-235 | 0.85 | 0.55 | 0.35 |
| U-238 (threshold) | 0.02 | 0.12 | 0.25 |
| Pu-239 | 0.90 | 0.60 | 0.40 |
| Pu-240 (threshold) | 0.10 | 0.18 | 0.30 |

Absorbers (boron / control rod / xenon / iodine) all gained an epithermal arm
between their thermal and fast values.

**Save compatibility:** the enum is bincode-encoded by variant index, so
`Thermal`=0 and `Fast`=1 keep their pre-P4 encodings and `Epithermal`=2 is
appended — old saves with queued neutrons decode unchanged. Verified by the
save round-trip property test.

### 2. Measured k-effective (`simulation.rs`)

`k_effective` (the closed-form estimate from cell counts) answers "what does
the pile *look like*". `k_measured` answers "what does the chain *do*": the
fission-rate ratio between consecutive 12-tick windows, exponent-corrected
back to one neutron generation (≈ 3 ticks):

```
k_measured = (fissions_window / fissions_prev_window) ^ (3 / 12)
```

1.0 means self-sustaining **by construction** — no formula to calibrate. The
estimate is only trusted when both windows carry ≥ 3 fissions; a dying
chain's late windows are statistical noise, and freezing the last trusted
value is the honest answer there. Both k values are shown in the HUD.

---

## Test gates ✅ (7 new)

| Gate | Checks |
|------|--------|
| `epithermal_sits_between_fast_and_thermal` | per-isotope ordering (both fissile and threshold shapes) |
| `absorbers_absorb_epithermal_between_thermal_and_fast` | all four absorbers |
| `downscatter_steps_one_group_per_collision` | the chain fast→epi→thermal, thermal→none |
| `moderation_steps_through_epithermal` | **queue inspection**: after one water collision the queue holds an *epithermal* event, not thermal; a second collision reaches thermal |
| `k_measured_subcritical_for_a_small_bare_pile` | 6×6 bare U-235: fissions happen, measured k < 0.95 |
| `k_measured_grows_with_moderated_pile_size` | graphite-moderated cores 6/12/18: the largest out-multiplies the smallest (the critical-mass sweep, gated self-consistently — no external reference exists for this toy model) |
| `graphite_reflector_raises_measured_k` | the SAME fuel load multiplies better wrapped in graphite — leaking neutrons come back |

The reflector gate is the proof the model has geometry: a pile that
multiplies better with a moderator blanket than without is doing neutron
*transport*, not just counting cells.

---

## Reviewed model change

Two-step moderation alters chain dynamics in moderated scenes. The golden
corpus re-recorded for **one** scene:

| scene | before | after |
|-------|-------:|-------:|
| `scenario_coolant_loop` | (826, `0xf6e5…`) | (824, `0xec4b…`) |
| other five scenes | — | **byte-identical** |

The P9a replay hash (a sand/water scene — no neutrons) is unchanged.

**All suites green:** 71 integration + 10 unit tests; fmt + clippy clean;
claim checker 13/13; the feature-gated suites (`thermal-pde`, `fluid-pde`)
still green.

---

## What is deliberately deferred

- **Monte-Carlo radiation transport** (the roadmap's "replace the random-walk
  with attenuation"): the existing penetration-depth model already implements
  shielding (dense materials absorb more readily, gamma passes 70%); a
  full MC rewrite of `move_radiation` is a refinement with no new gate.
- **Per-particle neutron generation tagging** (a true generational census):
  the windowed fission-ratio estimator measures the same quantity — k —
  without threading generation state through every neutron particle. If a
  future phase needs per-generation data (delayed-neutron groups), the
  estimator is the thing to replace.

## Unblocks

- **P5a (isotope model)** — the roadmap's P5a gate ("enrichment changes the
  critical radius") needs a *measured* k to be meaningful, and now has one:
  `k_measured` can observe enrichment-driven reactivity changes directly.

## Roadmap status after P4

| Phase | Status |
|-------|:------:|
| P0, P1, P2a, P2b, P3, P4, P5b, P8 (+elements), P9a | ✅ |
| P5a (isotopes — now unblocked) | ☐ next |
| P2c, P6, P7, P9b | ☐ |

*Structure inherited from `P0_REPORT.md` … `P2b_REPORT.md`.*
