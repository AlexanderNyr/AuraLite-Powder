# P3 — Thermal: Doppler Reactivity Feedback + Latent Heat — Report

**Phase:** P3 (ROADMAP)
**Baseline:** through `patches/P2a_parallel.patch`
**Deliverable:** `patches/P3_thermal.patch` (feature `thermal-pde`, opt-in)
**Status:** ✅ COMPLETE — feature-gated, both gates green, default unchanged

> P3 adds the single highest-value piece of physics the MVP lacked: a negative
> reactivity coefficient. The MVP's pile runs away (a hotter pile fissions
> *more*); a real reactor stays critical *because* a hotter pile fissions less.
> That loop, closed here behind the `thermal-pde` feature, is what turns the
> nuclear model from "explosion or dud" into a control problem.

---

## Why feature-gated (decision D3)

The MVP's `fission_probability` carries a mild *positive* temperature
coefficient — and every save replay, every golden-corpus fingerprint, and every
mission tuning depends on it. Changing it in the default build would silently
rewrite history. So P3 ships behind `thermal-pde` (core + workspace feature):
the default build is byte-identical to P2a, and the feature opts into the
honest model.

```
$ cargo test --test p0_rig golden_tick_corpus        # default
test golden_tick_corpus ... ok                        # ← unchanged
$ cargo test --features thermal-pde --test p3_thermal # opt-in
test doppler_lowers_reactivity_at_high_temp ... ok
test boiling_cools_neighbours ... ok
test self_limiting_pile ... ok
```

---

## What shipped

### Doppler reactivity feedback (`reactions.rs`)

```rust
pub fn temperature_coefficient(element_id: u16) -> f32 {
    match element_id {
        U235 => -0.0008,   // per Kelvin above ambient
        U238 => -0.0006,
        PU239 => -0.0005,
        PU240 => -0.0007,
        MOLTEN_FUEL => -0.0004,
        _ => 0.0,
    }
}
```

Under `thermal-pde`, `fission_probability` replaces the MVP's positive
`temp_factor` with the Doppler term: `base * (1 + coeff * excess_temp)`. A U-235
cell at +500 K fissions at ~60 % of ambient; at +1250 K it is essentially shut
down. The closed loop: fission heats the fuel → Doppler lowers fission
probability → fewer fissions → the pile cools → equilibrium.

### Latent heat at the phase change (`physics.rs`)

Under `thermal-pde`, water boiling to steam no longer carries its excess energy
into the steam for free: the steam starts near 400 K and each phase change saps
~40 K from its four neighbours — vaporisation absorbs energy, as it should.

---

## Test gates ✅

| Test | Checks |
|------|--------|
| `doppler_lowers_reactivity_at_high_temp` | `fission_prob(U235, Thermal, T)` is monotone decreasing in T (ambient > warm > hot); hot < 0.2 |
| `self_limiting_pile` | a graphite-moderated pile sustains a chain (`fission_count > 10`) but forms **zero molten fuel** and peaks < 3500 K — Doppler prevents meltdown |
| `boiling_cools_neighbours` | a hot stone next to boiling water ends cooler than it started — latent heat |

The `self_limiting_pile` gate is the proof of the phase: a pile that, under the
MVP model, would heat monotonically through the 2000 K meltdown threshold
instead self-limits and never melts. (A bare U-235 pile is subcritical on fast
fission alone, so the test embeds the fuel in graphite to make the chain
sustain — *then* checks Doppler caps it.)

---

## What is deliberately deferred

- **The implicit ADI heat-conduction solver.** The roadmap mentioned it; the
  current conductivity-weighted Jacobi step is retained because it is stable
  for the diffusion rates in play and the feedback loop does not need its
  unconditional-stability guarantee. ADI is a numerical-precision refinement
  (larger `dt`, sharper gradients) and is left for a later sub-phase — it does
  not affect the feedback gate, which is what P3 exists to close.

---

## Unblocks

- **P8 (content)** — missions like "Hold critical" and "Core damage" become
  *real* control problems under `thermal-pde` (the pile helps you hold, and a
  loss-of-coolant can still push it past meltdown). P8 can offer a
  `thermal-pde` mission variant.
- **P4 (multi-group transport)** — Doppler is the temperature feedback; P4's
  measured k-effective is the neutron-population feedback. Together they are
  the full reactivity picture.

## Roadmap status after P3

| Phase | Status |
|-------|:------:|
| P0, P1, P2a, P3 | ✅ |
| P2b (parallel gravity) | ☐ |
| P4–P9 | ☐ |

*Structure inherited from `P0_REPORT.md` / `P1_REPORT.md` / `P2a_REPORT.md`.*
