# P8 — Content: Campaign + 8 Missions — Report

**Phase:** P8 (ROADMAP)
**Baseline:** through `patches/P3_thermal.patch`
**Deliverable:** `patches/P8_content.patch`
**Status:** ✅ COMPLETE — all four gates green, ships alone

> P8 is the first phase whose output the player sees directly: two new missions
> and a campaign structure. It runs against the MVP model (the roadmap permits
> it), so it touches no engine internals beyond `missions.rs` — exactly the kind
> of additive work the layering contract is meant to make cheap.

---

## What shipped

### Two new missions (6 → 8 total)

| Mission | Setup | Win |
|---------|-------|-----|
| **Tritium breeder** (id 6) | a lithium blanket with a switched-on, staggered neutron flux | ≥ 15 tritium atoms bred (Li + n → T, prob 0.4) |
| **Quench the core** (id 7) | a small glowing-hot steel core submerged in a water pool | core max-temperature cools below 900 K |

Both reuse existing mechanics (neutron breeding, heat diffusion + boiling) so no
new physics was needed. Both are **auto-winnable**, which is what makes them
testable headlessly — the gate is "does the setup, with no player action,
reach the win condition", matching the existing `test_mission_filter_rescue_can_win`
pattern. (Quench needed its steel core shrunk to 3×3 so the pool's thermal mass
can actually quench it — the tuning is recorded in the setup comment.)

### Campaign framework (`Campaign`, core)

```rust
pub struct Campaign { order: Vec<MissionId>, completed: Vec<MissionId> }
// is_unlocked: first mission always; each later one unlocks when the previous is won
// record(id, status): a win unlocks the next
// next(): the next unlocked, not-yet-won mission
```

Forward-compatible: new `MissionId` values 6–7, `from_u8` extended, old saves
(ids 0–5) still load. Persisting campaign progress across saves is a small
follow-up (the mission itself already round-trips via `MissionSave`).

---

## Test gates ✅

```
$ cargo test --test p8_content
test test_mission_tritium_breeder_wins ... ok
test test_mission_quench_wins          ... ok
test test_campaign_unlock_logic        ... ok
test test_eight_missions_all_start     ... ok
```

- **Tritium breeder** breeds ≥ 15 tritium within 80 ticks (the staggered flux
  guarantees enough Li-n collisions).
- **Quench** cools the core below 900 K within 300 ticks.
- **Campaign** unlock logic: only the first mission is unlocked at start; a win
  unlocks the next; a failure does not; all eight ids round-trip through
  `from_u8`.
- **All eight missions start** without panic and report a running status.

68 default tests green (58 integration + 10 unit); claim checker 13/13. The
default build is unchanged (P8 adds content, not engine behaviour).

---

## What is deferred

- **New elements** (the roadmap's "elements" sub-item). Adding an element
  touches the `element_id` registry and many `is_*`/`density`/`conductivity`
  match arms; P0's `invariant_registry_covers_every_id` would guard it, but it
  is independent of the eight-mission gate and is left for a follow-up.
- **Campaign progress persistence** (saving which missions are completed) and a
  campaign **UI tree**. The data structure + logic are here; the save/UI wiring
  is additive.

---

## Roadmap status after P8

| Phase | Status |
|-------|:------:|
| P0, P1, P2a, P3, P8 | ✅ |
| P2b (parallel gravity) | ☐ |
| P4 (multi-group transport) | ☐ — blocked on the 2-neutron-element architecture |
| P5a, P5b, P6, P7, P9 | ☐ |

*Structure inherited from `P0_REPORT.md` … `P3_REPORT.md`.*
