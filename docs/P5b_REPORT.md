# P5b — Pressure: Steam-Explosion Transient — Report

**Phase:** P5b (ROADMAP)
**Deliverable:** `patches/P5b_fluids.patch` (feature `fluid-pde`, opt-in)
**Status:** ✅ COMPLETE — gate green, default unchanged, ships alone

> P5b adds the first pressure *transient*: a steam explosion when water meets
> molten fuel. The MVP's fluid model is a cellular automaton with hydrostatic
> band-aids — it can level a lake, but it cannot do the violent, pressure-driven
> events (a fuel-coolant interaction) that make a reactor accident an accident.
> Feature-gated (`fluid-pde`), so the default model and the golden corpus are
> untouched.

---

## What shipped

### `fluid-pde` feature (core + workspace)

Off by default. The MVP has no pressure transients; enabling `fluid-pde` opts
into the steam explosion. Verified: the default `golden_tick_corpus` is
byte-identical.

### Steam explosion (`simulation.rs`, under `fluid-pde`)

When a water (or heavy-water) cell is 8-adjacent to molten fuel, it flashes to
steam and the blast ejects its surroundings:

```rust
#[cfg(feature = "fluid-pde")]
if matches!(p.element_id, WATER | HEAVY_WATER) {
    let contact = neighbors_8.any(|n| n.element_id == MOLTEN_FUEL);
    if contact && rng.f32() < 0.6 {
        self.grid.set(x, y, Particle::new(STEAM, 2600));
        physics::apply_impulse(&mut self.grid, &mut self.velocities, x, y, 4, rng);
        // + heat spike to the 5x5 around the blast
    }
}
```

`apply_impulse` (already used by TNT) is the ejection mechanism — a real
fuel–coolant interaction that displaces mass, the thing the MVP structurally
could not do.

---

## Test gate ✅

```
$ cargo test --features fluid-pde --no-default-features --test p5b_fluid
test steam_explosion_flashes_water_to_steam ... ok
```

A water jacket around a molten core produces **steam** and the blast clears /
displaces cells out of the contact zone (`molten + water < original`). Under
the default model nothing happens.

**Default build unchanged:** golden corpus identical; 60 integration + 10 unit
tests green; `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
both clean.

---

## What is deferred

- **The full Navier–Stokes-lite solver** — continuous pressure/velocity
  advection (water hammer in a pipe, sustained pressure-driven flow). The
  existing `VelocityField` / `PressureField` are underused scaffolding the
  solver would activate. The steam-explosion *impulse* is the testable
  transient; the solver is a deeper, separate sub-phase (and its gate —
  "water hammer propagates" — needs the solver to exist).

## Roadmap status after P5b

| Phase | Status |
|-------|:------:|
| P0, P1, P2a, P3, P5b, P8, P9a | ✅ |
| P2b (parallel gravity) | ☐ |
| P4 (multi-group) | ☐ — blocked on the 2-neutron-element architecture |
| P5a, P6, P7, P9b | ☐ |

*Structure inherited from `P0_REPORT.md` … `P9a_REPORT.md`.*
