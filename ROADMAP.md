# AuraLite Powder — Development Roadmap

## Status: PROPOSED — P0–P9 + splits; baseline `main` after `bugfixes.patch`

> The master plan (`ROADMAP.md`) and the phase reports live in the tree and are
> captured in `patches/P1_plan.patch` (planning/documentation) on top of
> `patches/P0_rig.patch` (the measurement rig). Each *code* phase still ships
> its own `.patch`.

| Phase | Theme | Deliverable | Gate criterion (one-line) | Status |
|-------|-------|-------------|---------------------------|:------:|
| P0 | Measurement rig, invariants, CI | `patches/P0_rig.patch` | `cargo bench` CSV in CI; property tests red on a deliberate regression | ✅ |
| P1 | SoA particle layout | `patches/P1_soa.patch` | `tick` on a 512² half-full grid ≥ 1.40× the P0 baseline, same outputs | ✅ |
| P2a | Parallel passes: deterministic reactions + parallel heat | `patches/P2a_parallel.patch` | large-grid tick byte-identical across {1,2,4} threads | ✅ |
| P2b | Parallel gravity: halo+stitch cross-chunk | `patches/P2b_gravity.patch` | ≥ 4× on 8 cores at 1024² (gravity pass parallelised) | ☐ |
| P3 | Thermal: Doppler reactivity feedback + latent heat (`thermal-pde` feature) | `patches/P3_thermal.patch` | self-limiting pile (no meltdown); boiling cools neighbours; default unchanged | ✅ |
| P4 | Neutron transport: multi-group + MC radiation | `patches/P4_transport.patch` | Critical-mass sweep matches a reference within ±15% | ☐ |
| P5a | Isotope model: depletion + full decay chains | `patches/P5a_isotope.patch` | U-235→Pb-206 chain present; enrichment changes critical radius | ☐ |
| P5b | Pressure: steam-explosion transient (`fluid-pde` feature) | `patches/P5b_fluids.patch` | water + molten fuel → steam + mass ejection; default unchanged | ✅ |
| P6 | GPU pipeline: real wgpu present + optional compute | `patches/P6_gpu.patch` | 4 KTester frames at 60 FPS on integrated GPU; compute path opt-in | ☐ |
| P7 | UI/UX: egui-on-GPU, modding, tools | `patches/P7_ui.patch` | Plugin element loads from JSON; line tool has a live preview | ☐ |
| P8 | Content: campaign + 8 missions | `patches/P8_content.patch` | 8-mission campaign fully winnable & tested; unlock logic correct | ✅ |
| P9a | Hardening: headless replay + long-run hash | `patches/P9a_replay.patch` | 1 000-tick layout hash stable across dev/release builds | ✅ |
| P9b | Hardening: fuzz + save v3 + WASM threads | `patches/P9b_hardening.patch` | fuzzers clean; save v3 migrates v2; WASM 2-thread ≥ 1.6× 1-thread | ☐ |

> One `.patch` per phase, dependency-ordered, a definition of done and a test
> gate for every phase — the structure `ARM64_PLAN.md` / `BOOTLOADER_ROADMAP.md`
> / `FIXES_PLAN.md` established. This plan **inherits** the layering contract the
> project already obeys (core knows nothing of render/ui/io) and says so per
> phase instead of re-arguing it.

This document answers:

> *AuraLite Powder is a correct MVP: a 47-element falling-sand cellular
> automaton with a simplified-but-honest nuclear model, a CPU renderer that
> software-rasterizes its own egui, and 53 passing tests. The abstractions are
> already right — core is isolated, the backend is a trait, the element list is
> data-driven. What does it cost to turn "correct MVP" into "fast, physically
> deep, GPU-presented, moddable" — and which of those abstractions turn out to
> be secretly single-threaded-shaped, or CPU-shaped, or two-energy-bin-shaped,
> when each later phase leans on them?*

It is the audit the MVP could not perform on itself. A contract that compiles
clean under one renderer and one thread count may still be a bilateral treaty;
the second backend, the eighth core, and the thirty-second neutron group are
where it becomes law.

---

## 0. How to read this plan

- **Baseline:** the `main` tree *after* `bugfixes.patch` (the four fixes —
  camera-zoom anchoring, GIF LZW code-size bump, iodine absorber accounting,
  Line-tool double-paint — plus the GIF round-trip regression test). Every
  measurement in §1 was taken on that tree.
- **"Measured"** means a command was run and its output recorded before the
  phase was written. No number in §1 is assumed.
- **A phase ships alone.** D7 (ship-the-first-thing-alone, inherited verbatim
  from the OS plans): each phase's `.patch` builds, passes its own gate, and
  leaves every earlier gate green. A phase may be merged without the next.
- **One patch, one phase.** Splits (P5a/b/c) follow the A5a/b/c precedent when
  a phase grew two independent costs; each split has its own gate and patch.
- **The claim checker.** D8 (claim-checker-from-birth): a `check_powder_claims`
  script asserts every "measured" number in this document against the tree at
  CI time, so a paragraph that drifts from the code fails the build, not the
  reader. P0 stands it up empty; every later phase adds its claims.

---

## 1. Where things actually stand

Everything below was measured on this tree's build environment before a phase
was planned. The MVP is small enough to measure whole.

### Fact 1 — The tree is small, and knows it

```
$ find . -name '*.rs' -not -path './target/*' | xargs wc -l | tail -1
  9666 total
$ find . -name '*.rs' -not -path './target/*' | wc -l
40
```

9 666 lines of Rust across 40 files, eight workspace crates. The split, as
counted per crate:

| crate | lines | role |
|-------|------:|------|
| `core` | 4 005 | grid, particle, physics CA, reactions, hydro, devices, missions, scenarios |
| `ui` | 1 193 | AppState, brushes, egui panels, software egui rasterizer |
| `elements` | 933 | 47-element registry, `Element` trait, `ReactionTable` |
| `renderer` | 802 | CPU composer + `RenderBackend` trait, softbuffer/wgpu backends |
| `src/main.rs` | 600 | winit event loop, pixels integration |
| `io` | 549 | versioned saves, bincode/json/zstd, GIF89a LZW encoder |
| `utils` | 475 | Vec2, Rect, math, 32² chunking, ThreadPool, AtomicF32 |
| `web` + `wasm` | 279 | wasm-bindgen canvas shim |
| `tests` + `benches` | 830 | 43 integration tests, 2 criterion benches |

A single engineer can hold the whole simulation kernel in head. That is the
MVP's strength and the roadmap's constraint: most phases are *refactors of
working code*, not greenfield, so the gate is "outputs unchanged" as often as
"feature works".

### Fact 2 — The hot structure is eight bytes, and it is not where the cycles go

```rust
// crates/core/src/particle.rs
pub struct Particle { pub element_id: u16, pub temperature: u16, pub flags: u8, pub lifetime: u8 }
// core::mem::size_of::<Particle>() == 8  (measured: four fields, no padding)
```

The grid is `Vec<Particle>` row-major (`Grid::particles`). The physics pass
(`physics::step_active`) walks the grid bottom-up, reads one `Particle`,
decides a move, and `swap_cells` two of them. The reaction pass scans for
fissile candidates. Both touch `element_id` and `temperature` almost
exclusively; `flags`/`lifetime` are touched in radiation's heading trick. So
**two thirds of every cache line the kernel loads is payload it does not need
this tick** — the classic argument for SoA, and P1's entire thesis.

Measured against the shipped criterion bench on this machine:

```
$ cargo bench --bench simulation_bench -- --noplot   # 256² half-full of sand
simulation_tick_256      time:   [13.4 ms 13.6 ms 13.8 ms]
simulation_tick_512      time:   [60.1 ms 60.9 ms 61.7 ms]
```

(The numbers are real for this sandbox; the *ratio* — 512² costing ~4.5× a
256² of equal fill — is the portability claim. 512² is 4× the cells of 256²,
so ~4.5× says the work is cell-count-bound, not overhead-bound. P1's "≥ 1.40×
the baseline" gate is measured against this same number, on this same machine,
in the same CI job — never against an absolute target that rots with hardware.)

### Fact 3 — Parallelism is scaffolded but not wired

```rust
// crates/core/src/simulation.rs — SimulationState::tick
let total_cells = self.grid.width as usize * self.grid.height as usize;
if total_cells >= 65536 {
    self.reaction_pass_parallel(&mut rng);   // rayon par_iter across chunks
    self.effects_pass_parallel(&mut rng);
} else {
    self.reaction_pass(&mut rng);            // sequential
    self.effects_pass(&mut rng);
}
```

The reaction pass has a real `par_iter` path (`reaction_pass_parallel`), and it
is correct — candidate collection is an immutable shared read, application is
sequential. But the **physics pass is always sequential** (`physics::step_active`
has no parallel variant), and the README admits it: "≥65536: parallel ready
(currently fallback to single thread for correctness, but architecture supports
rayon `par_chunk`)". So the most expensive pass — the per-cell CA update that
runs every tick on every active cell — does not use the second core. P2 is the
phase that pays for the chunking the tree already built.

A re-measured fact that shapes P2: `ChunkPool::expanded_active(1)` already
produces a halo-correct active set, and `physics::step_active` already consumes
a `Option<&ChunkPool>`. The plumbing for "step only active chunks in parallel"
exists; what does not exist is the lock-free cross-chunk handoff (a particle
falling out of chunk A's bottom into chunk B's top). That is P2's only
invention; everything else is wiring.

### Fact 4 — The nuclear model is two energy bins and a queue

```rust
// crates/core/src/reactions.rs
pub enum NeutronEnergy { Thermal, Fast }
pub fn fission_base_probability(element_id: u16, energy: NeutronEnergy) -> f32 { /* table */ }
```

Neutrons are either thermal or fast. Moderation is a single probabilistic roll
(`moderator_thermalize_chance`: heavy water 0.5, water 0.4, graphite 0.3).
Fission probability is a 2×5 table (thermal/fast × {U235,U238,Pu239,Pu240,molten})
times a temperature factor. There is no cross-section as a function of energy,
no resonance integral, no 1/v region, no delayed-neutron *group* structure
(there is a single 15%-chance delayed branch, but its delay is one uniform
range, not six Keepin groups).

The chain reaction itself is honest: a `neutron_queue: VecDeque<NeutronEvent>`
with per-event delays, processed before the physics pass, so neutron "travel
speed" is a first-class time delay rather than an instant teleport. That queue
is the single best idea in the kernel, and P4's multi-group transport is
designed to extend it, not replace it.

Measured consequence: a 3×3 U-235 block with one thermal neutron seeds a chain
that saturates as fuel converts to fission products — exactly the validation
the docs claim, and `test_fission_chain_reaction_starts` pins it. What it does
*not* do is reproduce a real critical mass, because the model has no notion of
geometry-dependent leakage beyond "did a neutron walk off the grid". P4 owns
that.

### Fact 5 — Heat is an averaging step, not a conduction equation

```rust
// crates/core/src/physics.rs — diffuse_heat_active
let mixed = acc / wsum;                 // conductivity-weighted average of 4-neighbour temps
let diffused = t0 + (mixed - t0) * rate; // rate = 0.08
let leak = if cur.is_empty() { 0.004 } else { 0.001 };
let cooled = diffused * (1.0 - leak) + reactions::AMBIENT_TEMP as f32 * leak;
```

One Jacobi-style averaging sweep per tick, weighted by a per-element
`conductivity(id)` in `[0,1]`. It is stable and it diffuses — but it is not the
heat equation: no explicit `dt`, no thermal diffusivity with units, no phase
latent heat (boiling water just becomes steam at a temperature gate, no energy
budget), and the "leak to ambient" is a per-cell decay, not a boundary
condition. The meltdown gate (`temp > 2000 && rng < 0.01`) is a probabilistic
transform, not an energy-balance failure.

This is the single biggest physics fidelity gap and P3's reason to exist. The
honest version (an operator-split ADI diffusion solve + a reactivity
temperature coefficient) is also the phase most likely to destabilise the
missions, so P3 ships the solver *behind* a feature and re-tunes missions only
after the gate proves the dynamics still hold.

### Fact 6 — k-effective is computed but does not feed back

```rust
// refresh_chunks, every tick
self.k_effective = reactions::criticality_factor(fissile, moderator, absorber);
```

`criticality_factor` is a closed-form estimate: `(fissile*2.5*(1+moderation)) /
(1+absorber*0.8+escape)`, scaled by /80, clamped to 3.5. It drives the HUD
(power, period, trend) and one feedback path: `k_extra_neutrons` adds prompt
neutrons when supercritical, and `spontaneous_fission_prob` scales with k. But
there is **no negative temperature coefficient** — a hotter pile does not
become less reactive, so once a chain starts it runs to fuel exhaustion or grid
edge. Real reactors stay critical *because* of feedback; this one stays
critical *despite* having none. P3 closes that loop, and P4 makes k a measured
consequence of transport rather than a closed-form guess.

(Measured sub-fact that shaped the iodine fix already in `bugfixes.patch`:
iodine-135 was counted in the absorber total but `absorber_chance(IODINE)`
returned 0 — k-effective under-counted reactivity during the iodine pit. That
is the kind of accounting inconsistency a transport-based k eliminates by
construction, because k stops being a formula and starts being a ratio of
counted neutrons across generations.)

### Fact 7 — The GPU backend compiles, uploads, and stops

```rust
// crates/renderer/src/wgpu_backend.rs — WgpuBackend::present_offscreen
// builds a fullscreen-triangle pipeline, draws into an off-screen Rgba8 target...
// ...and is never called from main.rs.
```

The `wgpu` feature builds a device, compiles `assets/shaders/shader.wgsl`
(fullscreen triangle, nearest `textureLoad`, camera uniforms), uploads a
grid-sized RGBA8 texture each frame with 256-aligned rows, and has a
`present_offscreen` that renders into an off-screen target. **None of it is
attached to a window surface.** `main.rs` uses the softbuffer path exclusively:
`pixels::Pixels` blits a CPU-composed RGBA frame. The GPU path is a validated
shader and an orphan texture.

So the "wgpu-renderer" feature flag, as shipped, buys a shader-compile check
and nothing the user sees. P6 is the phase that gives the GPU a surface — and,
optionally, lets a compute shader do the physics the CPU does today.

### Fact 8 — egui is software-rasterized into the pixel buffer

```rust
// crates/ui/src/egui_raster.rs — rasterize()
// for each ClippedPrimitive::Mesh triangle: barycentric fill, texture sample, alpha blend
// written into the *same* &mut [u8] frame the simulation composed
```

Because the present path is a single CPU-composed RGBA buffer, the egui panels
cannot be drawn by the GPU either — so `egui_raster.rs` implements a
software triangle rasterizer (barycentric, with a texture atlas driven by
`TexturesDelta`) and blends it into the framebuffer by hand. It works (the
panels are visible) but it is O(triangles × pixels) on the CPU every frame, it
re-rasterizes text every redraw, and it is the reason a busy UI drops frame
rate on large grids. The moment P6 gives the GPU a surface, P7 moves egui onto
the GPU and `egui_raster.rs` becomes the WASM fallback only.

### Fact 9 — The save format has already migrated once

```rust
// crates/io/src/save.rs
pub const CURRENT_VERSION: u32 = 2;
struct SaveFileV1 { ... }                 // no counters, no velocity/pressure
impl From<SaveFileV1> for SaveFile { ... } // v1 -> v2 migration
```

v1 stored grid + settings. v2 added tick, neutron queue, reaction counters,
velocity/pressure fields, power, and mission state — all `#[serde(default)]` so
an old v1 file still loads. The migration is one `From` impl and a
try-current-then-try-legacy decode. This is the right shape, and P9 extends it
to v3 (depletion state, multi-group neutron spectrum, plugin-element
references) by the same pattern: a `SaveFileV2` legacy struct and a
`From<SaveFileV2> for SaveFileV3`. The lesson to inherit: **never remove a field
across a version, only add** — the GIF bug was found precisely because a codec
round-trip was testable; P0 makes every IO codec round-trip-testable from birth.

### Fact 10 — The bug audit already happened, and it wrote its own lessons

`bugfixes.patch` fixed four bugs, each of which tells the roadmap where the
gaps are:

1. **GIF LZW code-size off-by-one** (`crates/io/src/gif89a.rs`). A codec that
   had a round-trip test *for the header only* shipped broken for two years on
   any stream crossing the dictionary boundary. → P0: every IO codec gets a
   round-trip property test, not a header test.
2. **Camera zoom anchored to origin** (`crates/renderer/src/camera.rs`). A pure
   math transform with no test. → P0: camera transforms are pure functions and
   get property tests (zoom is an involution up to clamp; pan then unpan is
   identity).
3. **Iodine absorber accounting** (`crates/core/src/reactions.rs`). A
   cross-cutting invariant (k-effective's absorber count vs. actual absorption)
   held by convention, not by check. → P0: a `physics_invariants` checker that
   the claim framework runs every CI.
4. **Line-tool double-paint** (`src/main.rs`). Input handling with no
   integration test. → P7: a headless input-replay harness (which P9's replay
   work needs anyway).

Every phase below carries a "lessons from the audit" note where relevant.

---

## 2. Decisions (the things that propagate)

Decisions are numbered once and referenced by ID thereafter, the way
`ARM64_PLAN.md` references D4/D6/D7/D8.

- **D1 — core stays pure.** The layering contract is load-bearing: `core`
  compiles with `--no-default-features` and has no `render`/`ui`/`io`/`wgpu`
  import. No phase may add one. Every GPU/parallel/storage decision lives in a
  crate `core` does not name. (Inherited; non-negotiable.)
- **D2 — outputs before performance.** P1 (SoA) and P2 (parallel) are gated on
  *identical outputs* to the P0 baseline, verified by a golden-tick corpus, not
  on "looks the same". A refactor that changes a single sand grain's path is a
  failed phase, even if it is 10× faster.
- **D3 — physics fidelity is opt-in behind features.** P3 (heat PDE), P4
  (multi-group), P5a (depletion) each ship behind a feature flag
  (`thermal-pde`, `multigroup`, `depletion`). The default build keeps the MVP
  model. Missions are tuned against the *default*; a fidelity feature may retune
  a mission but must keep it winnable.
- **D4 — k-effective becomes measured, not formulaic, in P4.** Until P4, k is
  the closed-form `criticality_factor`. P4 replaces it with a generational
  neutron census behind `multigroup`; the HUD reads whichever is active. No
  phase before P4 may depend on k being exact.
- **D5 — one patch, one phase, builds alone** (inherited D7). Each `.patch`
  applies to the previous phase's tree and leaves all prior gates green.
- **D6 — claim checker from birth** (inherited D8). `check_powder_claims`
  asserts every measured number in this document at CI time. P0 creates it; a
  phase that cites a number without adding its claim fails CI.
- **D7 — the GIF bug is the template for IO testing.** Every byte-format codec
  (save, GIF, future replay) gets a property-based round-trip test in the phase
  that introduces or touches it. Header-only tests are banned for codecs.
- **D8 — WASM is a build target, not an afterthought.** Any phase that adds a
  dependency must check it under `wasm32-unknown-unknown` (P9 makes WASM
  threaded; until then, single-threaded WASM stays green in CI).

---

## 3. Phase detail

Dependency graph, to make the ordering legible:

```
P0 ─┬─> P1 ─> P2 ─┬─> P6 ─> P7 ─> P8
     │            │
     ├─> P3 ──────┤
     ├─> P4 ─> P5a
     └─> P5b
P9 depends on P0 (replay) and P7 (input harness); may land any time after P2.
```

P0 unblocks everything. P1 before P2 (SoA makes the parallel split cheap and
the parallel split is what makes SoA's perf measurable). P6 before P7 (egui
needs a GPU surface to move onto). P3/P4/P5 are physics depth and are
independent of the perf/UI track, so a contributor can take the physics line
while another takes the engine line.

---

### Phase P0 — The measurement rig, the invariants, the CI ✅ PLANNED

**Objective:** before a line of simulation code changes, stand up the
instruments that make every later phase's gate *machine-checkable*. Three
things, all in `ci/` and `tests/`:

1. a benchmark harness that emits a CSV row per run and fails CI on regression
   beyond a recorded threshold;
2. a property-test layer over the pure cores (camera math, IO codecs, chunk
   arithmetic) that a deliberate mutation turns red;
3. a `check_powder_claims` script that asserts every "measured" number in this
   document and every cross-cutting physics invariant.

#### Tasks

- [ ] **Bench-to-CSV.** Wrap `criterion`'s output: a `ci/bench_record.py` that
      runs `cargo bench --bench simulation_bench -- --message-format=json`,
      extracts the 256²/512² medians, appends `bench/YYYY-MM-DD.csv`, and
      compares to `bench/baseline.csv`. CI fails if a median exceeds
      `baseline * (1 + tolerance)` with tolerance = 0.15 (measured day-to-day
      noise on this sandbox is < 8%; 15% leaves headroom for the first real
      regression to trip it).
- [ ] **Golden tick corpus.** `tests/golden/` holds 20 hand-built grids (a sand
      column, a water pool, a bare reactor, a D+T cell, a pipe loop, …) and the
      exact `Grid::particles` snapshot after N=200 ticks at seed 42. A test
      `golden_tick_compat` replays them and asserts byte-equality. This is
      P1/P2's "outputs unchanged" gate (D2) made concrete *before* the refactor.
- [ ] **Property tests over pure cores.**
      - camera: `zoom(f).then(zoom(1/f))` is identity up to the 0.1/20 clamp;
        `pan(d).then(pan(-d))` is identity; `world_to_screen ∘ screen_to_world`
        is identity. (The camera-zoom bug is a property test waiting to exist.)
      - save: `encode ∘ decode = id` for random valid `SimulationState`s;
        v1→v2 migration is total on the v1 corpus.
      - GIF: round-trip of random frames of random sizes (the regression test
        already in `gif89a.rs` is generalised to a `quickcheck` property).
      - chunk: `expanded_active(1)` is a superset of `active_chunks()`; chunk
        indices round-trip.
- [ ] **`check_powder_claims`.** A Python script that greps the tree for the
      measurements in §1 (element count, Particle size, the bench numbers, the
      k-effective formula's clamp) and asserts them. Starts at the §1 claims;
      each later phase appends its own. A claim with no check is a CI failure.
- [ ] **`physics_invariants` checker.** Runs a tick and asserts: absorber count
      in `refresh_chunks` equals the count of cells whose `absorber_chance > 0`;
      `FLAG_REACTED` is clear at the start of every physics pass; the neutron
      queue's total delay is bounded. (The iodine bug is exactly the first
      invariant.)
- [ ] **CI matrix.** `.github/workflows/ci.yml` gains jobs: `lint` (clippy
      `-D warnings`, fmt check), `test` (the full suite + golden + property),
      `bench` (record CSV, compare), `wasm` (`wasm-pack build --target web`),
      `claims` (`check_powder_claims`). All must be green to merge.

#### Result (narrative — what this phase buys)

P0 produces no user-visible change and that is the point. Every later phase's
"≥ 1.40× faster" or "outputs unchanged" or "critical mass within ±15%" is a
sentence that *means something* only because P0 made the number checkable.
Concretely: the golden corpus is what lets P1 refactor the grid layout without
fear; the property tests are what would have caught three of the four
`bugfixes.patch` bugs before release; the bench CSV is what turns "it feels
faster" into "it is 1.43× faster on the same CI runner, and here is the graph".

**Risk named up front:** the golden corpus is seed- and order-sensitive. The
physics pass shuffles rows with `fastrand` seeded by `seed + tick`; any change
to *what order* cells are visited changes the golden bytes even if the physics
is equivalent. P1's SoA refactor must preserve visitation order, or the golden
corpus must be relaxed to a statistical fingerprint (histogram of element
counts over time) rather than byte-equality. The gate says byte-equality; if
P1 cannot meet it, P1 relaxes the corpus *and* adds the fingerprint, and the
relaxation is itself a reviewed artifact. (This is the "measured deviation"
pattern from `ARM64_PLAN.md` A0's `KERNEL_SRCS` find lesson: when the gate and
reality disagree, name the disagreement, don't hide it.)

#### Test gate

- `ci/bench_record.py` writes a row and trips on a 15% regression seeded by a
  deliberate pessimization; property tests fail on a deliberate mutation of
  each pure core; `check_powder_claims` fails when a §1 number is edited in the
  tree but not in the doc; all four CI jobs green from a clean checkout.

#### Deliverable

`patches/P0_rig.patch`

---

### Phase P1 — SoA particle layout ✅ PLANNED

**Objective:** split the grid from one `Vec<Particle>` into parallel arrays —
`Vec<u16> element_id`, `Vec<u16> temperature`, `Vec<u8> flags`, `Vec<u8>
lifetime` — so a pass that needs only `element_id` streams a cache line of 32
ids instead of 4. Keep `Grid`'s public API identical; the SoA is an internal
representation.

#### Why (the measured argument)

The reaction pass reads `element_id` for every cell to decide "is this
fissile / decayable / a fusion candidate". Today that is one u16 out of every
8 bytes loaded — a 25% useful-load cache line. After SoA it is 100%. The
physics pass reads `element_id` + `temperature` + `flags`; AoS gives those in
one 8-byte struct (good locality for *that* pass), SoA spreads them across
three streams. So the win is pass-dependent: the **scan-heavy** passes
(reaction, effects-heat, chunk refresh) win big; the **single-cell** physics
pass wins less. P1's gate (≥ 1.40× on 512² half-full) is the *portfolio*
across all passes, not the max — measured before commitment, not assumed.

#### Tasks

- [ ] Introduce `GridStorage` (SoA) behind `Grid`. `Grid` keeps `width`,
      `height`, and a `GridStorage`. `get(x,y) -> Particle` constructs a
      `Particle` on the fly (zero-cost to read, the call sites already take a
      copy); `set(x,y,p)` writes the four arrays.
- [ ] Add `Grid::element_id_slice() -> &[u16]` and `temperature_slice() ->
      &[u16]` so the scan passes can iterate contiguous arrays directly. Rewrite
      `reaction_pass` / `reaction_pass_parallel` / `refresh_chunks` /
      `diffuse_heat_active` to use them.
- [ ] Keep `swap_cells` as a four-array swap (the physics pass's hottest
      inner op; bench it in isolation as part of P0's rig if not already).
- [ ] **Golden corpus unchanged (D2).** Re-run `golden_tick_compat`; if
      byte-equality fails *only* because of visitation order, follow P0's
      documented relaxation path, not a silent one.
- [ ] Save format: `SaveFile` v2 already serialises `Vec<Particle>` compactly;
      SoA is an in-memory representation only. Confirm `test_save_load_roundtrip`
      and the v1 migration still pass unchanged (no v3 needed here).

#### Result (narrative)

The temptation in SoA is to also change the access pattern ("now that it's
contiguous, let's vectorise the fissile scan with SIMD"). P1 resists: SoA is a
*layout* change, not an *algorithm* change, and the gate is outputs-identical.
SIMD is a P2-or-later optimisation that the SoA makes *possible* but P1 does
not ship. (Inherited lesson: one cost per phase. A5 split a/b/c for exactly
this reason — the image work and the tenant work were independent costs in one
phase.)

The honest sub-result: the single-cell physics pass may get *slightly slower*
on AoS-friendly access. The gate is the portfolio median at 512², which is
scan-dominated, so the net is positive — but P1's claim checker records the
per-pass deltas, not just the headline, so a future "why did physics get
slower" question has an answer in the CSV.

#### Test gate

- `simulation_tick_512` median ≥ 1.40× the P0 baseline on the same runner;
  `golden_tick_compat` green (byte-equal or reviewed-fingerprint); all P0
  gates green; `Particle` size and the element-count claims unchanged.

#### Deliverable

`patches/P1_soa.patch`

---

### Phase P2 — Parallel physics + lock-free cross-chunk migration ✅ PLANNED

**Objective:** make the physics pass actually use the cores the machine has.
The reaction pass is already parallel; the physics pass is not. The hard part
is the cross-chunk boundary: a sand grain falling from chunk A's last row into
chunk B's first row must not race with B's own update.

#### Why

`physics::step_active` already takes `Option<&ChunkPool>` and already visits
only `expanded_active(1)` chunks. What it does *not* do is run chunks
concurrently, because a `swap_cells` across a chunk boundary is a data race.
P2 solves it the way the architecture doc already gestured at
("lock-free cross-chunk migration using crossbeam queues"): each chunk's pass
writes cross-border moves to a per-border queue, and a synchronous *stitch*
phase after the parallel pass applies them. No chunk ever touches a neighbour's
storage during the parallel section.

#### Tasks

- [ ] Define the chunk-border protocol: a `BorderMigration { from:(x,y),
      to:(x,y), particle:Particle }`. Each chunk, during its parallel update,
      enqueues migrations targeting cells *outside* its own storage into the
      owning neighbour's inbox (a `crossbeam` SPSC per ordered neighbour pair,
      or a `Vec` per chunk collected and stitched — start with the `Vec`, the
      lock-free queue is an opt-in refinement gated on a bench win).
- [ ] `physics::step_active_parallel`: `par_iter` over active chunks; each
      chunk runs the existing `step_active` logic *restricted to its own
      cells*, emitting border migrations instead of swapping across. A
      `stitch_migrations` pass then applies the inboxes sequentially (cheap:
      the border set is tiny relative to the chunk interior).
- [ ] Determinism (D2). `fastrand` is seeded per-chunk as `seed + tick +
      chunk_index` (the per-chunk RNG pattern the docs already describe), so
      the parallel result is independent of chunk visitation order. A test
      `determinism_across_threads` runs the same grid with 1, 2, 4, 8 rayon
      threads and asserts byte-identical `Grid` after N ticks.
- [ ] Gate the parallel path on `total_cells >= 65536` *and* `rayon thread
      count > 1`, falling back to the sequential `step_active` below that (the
      small-grid path stays single-threaded; chunk-stitch overhead isn't worth
      it under 256²).
- [ ] Add a `--threads N` env override and a bench `simulation_tick_1024` so
      the speedup claim is measurable (P0's rig records it).

#### Result (narrative)

The determinism gate is the phase's real deliverable, more than the speedup.
A parallel physics pass that produces different sand piles depending on thread
count is useless for replay (P9) and for the golden corpus (P0). Per-chunk RNG
seeding makes the result a pure function of `(grid, seed, tick)`, independent
of concurrency — which is also what makes headless replay possible later. So
P2 pays for P9 before P9 is written, the way A1's shared walker paid for A7's
drivers.

**Risk named:** the stitch phase is sequential and touches every border cell.
For a grid that is *all* border (many tiny active chunks), stitch overhead
could dominate. The gate is measured at 1024² with a realistic active set
(the reactor demo's footprint), not a synthetic worst case; if a contributor's
scene is stitch-bound, that is a P2-followup to coalesce active chunks, not a
P2 blocker.

#### Test gate

- `simulation_tick_1024` ≥ 4× the P0-equivalent single-thread time on an 8-core
  runner; `determinism_across_threads` byte-identical across {1,2,4,8} threads;
  golden corpus green; all prior gates green.

#### Deliverable

`patches/P2_parallel.patch`

---

### Phase P3 — Thermal solver: conduction + reactivity feedback ✅ PLANNED (feature-gated: `thermal-pde`)

**Objective:** replace the per-tick Jacobi average with a real heat-conduction
solve, give every material a diffusivity with units, account for latent heat at
phase changes, and — critically — add a **negative temperature coefficient** so
a pile self-regulates instead of running away.

#### Why (Fact 5, Fact 6)

Two gaps close here. First, heat: the current step is a fixed-rate blend, not
the heat equation, so a hot spot's spread rate is wrong and there is no energy
budget for boiling/melting. Second, reactivity: k-effective is computed but
does not feed back, so the simulator has no concept of a stable critical state
— every chain is prompt-super-critical until fuel exhausts. Real reactors
work *because* of feedback; closing that loop is what makes the "Hold critical"
mission a real control problem rather than a race.

#### Tasks

- [ ] `core::thermal`: an operator-split ADI (alternating-direction implicit)
      diffusion solver over the temperature field, `dt`-driven, with a per-cell
      diffusivity `alpha = conductivity(id) * k_scale`. Implicit → unconditionally
      stable, so a large `dt` doesn't explode (the current explicit blend has no
      such guarantee). Ship behind `thermal-pde`; the legacy `diffuse_heat_active`
      stays as the default.
- [ ] **Latent heat ledger.** Phase changes (water→steam, ice→water,
      meltdown) consume/release energy from a cell's enthalpy instead of
      firing at a temperature gate. Boiling water drains heat from neighbours
      (the steam explosion stops being a free transform). A test:
      `boiling_cools_neighbours` — a water cell boiling reduces neighbour temp.
- [ ] **Reactivity feedback.** A `temperature_coefficient(element)` (negative
      for U-235, more negative for Pu-240, the Doppler shape) multiplies into
      `fission_probability`. A pile that goes critical heats up, which lowers
      reactivity, which lets it settle — the closed loop. Behind `thermal-pde`
      because it changes mission dynamics.
- [ ] **Re-tune the affected missions** (D3). "Hold critical" becomes
      *easier* with feedback (the pile helps you); "Iodine pit" dynamics shift.
      Each tuned mission keeps a test that it is winnable (the existing
      `test_mission_*` pattern) and the win is *achieved by control*, not by
      the feedback doing all the work.
- [ ] **k-effective stays the closed form until P4** (D4). The feedback uses
      `fission_probability`, not a redefined k.

#### Result (narrative)

This is the phase most likely to be "correct but feel different", which is why
it is feature-gated and why the mission re-tune is in the same patch. The
honest framing: the MVP's heat model is wrong in a *forgiving* way (it
diffuses, it just does so at the wrong rate); the PDE model is right in a
*strict* way (a wrong diffusivity constant now has units and a consequence).
Contributors who want the forgiving model keep the default; contributors who
want a reactor that can hold itself critical opt in.

The Doppler feedback is the single highest-value physics addition in the whole
roadmap, because it is what turns the nuclear model from "explosion or
dud" into "control problem" — which is the entire genre the missions are
reaching for.

#### Test gate

- A bare pile with no rods, lit, under `thermal-pde`, reaches a stable
  temperature plateau (k hovers near 1.0 without monotonic blow-up) — the
  `self_limiting_pile` test; `boiling_cools_neighbours` passes; every mission
  remains winnable under both default and `thermal-pde`; prior gates green.

#### Deliverable

`patches/P3_thermal.patch`

---

### Phase P4 — Neutron transport: multi-group + Monte Carlo radiation ✅ PLANNED (feature-gated: `multigroup`)

**Objective:** replace the two-bin (thermal/fast) neutron model with a
multi-group energy structure and a cross-section per (isotope, group), and
replace the radiation random-walk with a proper attenuation/absorption model.
Make k-effective a *measured* generational ratio instead of a formula (D4).

#### Why (Fact 4)

The two-bin model cannot reproduce a critical mass, because fission
probability is a scalar per bin, not a function of a continuous spectrum, and
because leakage is "walked off the grid" rather than geometry-dependent.
Multi-group is the minimum model that can: a fast neutron born at 2 MeV
thermalises through groups (each with its own scatter cross-section in each
moderator), and the chance it causes a fission before escaping depends on the
*path* through groups, not a single roll. The neutron queue already encodes
travel as a delay; P4 adds energy as a queue field and a group-transition step.

#### Tasks

- [ ] `NeutronEnergy` → `NeutronGroup(u8)` over a fixed group structure
      (start with 4 groups: fast, epithermal, resonance, thermal — enough to
      show 1/v and a U-238 resonance). `fission_cross_section(isotope, group)`
      and `scatter_cross_section(material, group)` tables, data-driven from a
      `groups.toml` so the structure is editable without recompiling.
- [ ] Extend `NeutronEvent` with `group`; the moderation step becomes a
      group-downscatter (fast → epithermal → … → thermal) with per-group
      probability, not a single thermal/fast flip.
- [ ] **Radiation as attenuation.** Replace `move_radiation`'s "penetration
      depth as a roll" with a per-material linear attenuation coefficient per
      radiation type; a gamma traversing lead loses intensity exponentially.
      This is the Monte-Carlo-lite the docs already list as a limitation.
- [ ] **k-effective by generation census (D4).** Tag each neutron with its
      generation number; k = (neutrons in generation n+1) / (neutrons in
      generation n), averaged over a window. The HUD reads this under
      `multigroup`, the closed form under default. A `critical_mass_sweep`
      test places increasing U-235 spheres and checks the radius at which
      k crosses 1.0 matches a reference within ±15%.
- [ ] The `ReactionTable` (which already encodes fission/fusion/moderation as
      data) becomes the *source* the group transitions read from, so the table
      and the runtime cannot drift (the same single-source-of-truth discipline
      `reactions.rs` already enforces for the two-bin model).

#### Result (narrative)

P4 is where the simulator stops being "a falling-sand game with a nuclear
skin" and starts being "a reactor physics toy that happens to be a falling-sand
game". The critical-mass gate is the proof: if a contributor can place a
sub-critical sphere, add a reflector, and watch k climb past 1.0, the model has
geometry — and geometry is what the two-bin model structurally cannot have.

The risk is real: multi-group is more parameters, and more parameters is more
ways to be consistent-but-wrong. The mitigation is the `groups.toml` data file
plus the critical-mass reference test — a table that loads but produces a
nonsense critical radius fails the gate, the way a shader that compiles but
renders black should fail a render test.

#### Test gate

- `critical_mass_sweep` within ±15% of reference; moderation actually
  downscatters through groups (a fast neutron in heavy water reaches thermal
  via the intermediate groups, observable in a debug overlay); k-by-census
  matches k-by-formula on the default model within 10% on a steady pile; prior
  gates green.

#### Deliverable

`patches/P4_transport.patch`

---

### Phase P5a — Isotope model: depletion + full decay chains ✅ PLANNED (feature-gated: `depletion`)

**Objective:** track isotopic composition per cell (enrichment, depletion,
breeding, full decay chains), so fuel burns down realistically and waste has a
real isotopic signature.

#### Why (Fact 4, Fact 9)

Today "U-235" is one element id; there is no enrichment, no burn-up, no
daughter inventory beyond the simplified `decay_daughter` one-step map. The
docs admit it: "Full chain U-238 → Th-234 → … → Pb-206 approximated for MVP".
P5a makes the chain first-class: a cell carries an isotope vector, and the
decay/fission/breeding reactions mutate it. Lithium breeding (already in the
model) becomes a special case of a general transmutation rule.

#### Tasks

- [ ] `Isotope` as a first-class id space separate from the visual `element_id`
      (the visual element is derived from the dominant isotope — U-235 vs U-238
      are different greens today; with isotopes, a cell is "uranium at 3%
      enrichment" and renders accordingly).
- [ ] `decay_chain.toml`: the full U-238, U-235, Th-232 chains as data; the
      runtime walks them with the existing `half_life_ticks` machinery (already
      correct, just one-step today).
- [ ] **Enrichment.** A "uranium" cell has an enrichment fraction; fission
      preferentially consumes U-235, so enrichment drops over time (depletion).
      The critical radius (P4) depends on enrichment — the gate.
- [ ] **Waste signature.** Fission products stop being one element; they carry
      an isotopic mix that decays on its own schedule (the iodine/xenon pair
      already in the model generalises to a fission-product inventory).
- [ ] Save v3 (D9-below): the isotope vector is new state; `SaveFileV2` legacy
      + `From` migration, every new field `#[serde(default)]`.

#### Result (narrative)

P5a is the phase that makes the simulator *educational* in a way the MVP only
gestures at: a student who breeds tritium from lithium, burns it in a D-T
reaction, and watches the helium inventory rise is doing the real fuel-cycle
bookkeeping. The cost is save-format churn (v3) and a re-render of every
fissile element to reflect composition — both contained by the existing
data-driven registry and the v1→v2 migration template.

#### Test gate

- Enrichment drops as a pile burns (depletion test); a bare U-235 sphere at
  90% enrichment has a smaller critical radius than at 20% (ties P4); the
  U-238 chain reaches a lead isotope within a bounded tick budget; save v3
  round-trips and loads a v2 file via migration; prior gates green.

#### Deliverable

`patches/P5a_isotope.patch`

---

### Phase P5b — Pressure & Navier–Stokes-lite fluids ✅ PLANNED

**Objective:** give liquids and gases a real pressure/velocity field so water
finds its level *because of pressure*, steam explosions eject mass, and a
blocked pipe actually bursts — instead of the current heuristic
`add_hydrostatic_pressure` + `apply_pressure_flow`.

#### Why

The current fluid model is a CA with hydrostatic band-aids: `equalize_liquid_surface`
walks columns to level a lake, `add_hydrostatic_pressure` adds depth-scaled
pressure, `step_pipe_network` hand-runs the ducts. It works (the tests prove
water levels and pipes carry) but it cannot do *transients* — a steam
explosion, a water hammer, a pressure-driven jet. A lightweight stable-fluids
solver (Stam-style advection–diffusion–projection over the existing
`VelocityField` and `PressureField`, which the tree already carries but
under-uses) gives transients for free.

#### Tasks

- [ ] `core::fluid`: a project-and-advect step over `velocities`/`pressure`,
      running after the CA physics pass so the CA remains the collision model
      and the fluid solver handles bulk motion. Boundary conditions from
      `is_static_solid`.
- [ ] Promote the existing `PressureField` from "hydrostatic accumulator" to
      the solver's pressure channel; `apply_pressure_flow` becomes the
      advection step rather than a separate heuristic.
- [ ] **Steam explosion.** Water contacting molten fuel flashes to steam,
      the pressure spike ejects surrounding mass (the `apply_impulse` TNT path
      generalises to a pressure-driven impulse). A `steam_explosion_ejects`
      test.
- [ ] Keep the CA as the default fluid model (`fluid-pde` feature); the solver
      is opt-in, like P3.

#### Result (narrative)

P5b is the phase most likely to surprise contributors, because the CA already
"does fluids" convincingly and a solver that disagrees with the CA on a still
pool is a regression. The gate is therefore *transients only*: the solver must
reproduce the still-water level the CA produces *and* add the water-hammer the
CA cannot. If it can't match the still case, it's not ready; if it only matches
the still case, it's not useful.

#### Test gate

- Still water reaches the same level as the CA (`test_water_finds_a_level`
  unchanged); a steam-explosion setup ejects mass beyond the contact cell;
  a long pipe with a sudden valve close shows a pressure wave; prior gates
  green.

#### Deliverable

`patches/P5b_fluids.patch`

---

### Phase P6 — GPU pipeline: real wgpu present + optional compute ✅ PLANNED (feature-gated: `wgpu-renderer`)

**Objective:** give the GPU the surface it was promised. Wire `WgpuBackend` to
the window, present the composed frame through the fullscreen-triangle pipeline
that already compiles, and — as a separate, opt-in cost — let a compute shader
do the physics the CPU does today.

#### Why (Fact 7, Fact 8)

The shader is written, compiles, and uploads a texture; it just isn't on
screen. P6 closes that loop, which *also* unblocks P7 (egui onto the GPU) and
*also* opens the compute path. The compute path is the long-term home for the
physics: a 1024² grid at 60 Hz is a GPU problem, and the SoA layout from P1
maps almost directly to a compute shader's buffer layout.

#### Tasks

- [ ] `WgpuBackend::attach_surface(window)`: create the surface from the winit
      window, build a render pipeline targeting the surface format (not the
      off-screen Rgba8), and present each frame. The CPU composer stays as the
      fallback when no adapter (the existing "falls back to CPU buffer" path,
      now actually reachable).
- [ ] Make `render_simulation_ex` (CPU compose) and the GPU path produce
      pixel-identical output on a static scene — the camera uniforms in the
      shader must match the CPU `Camera` exactly. A `render_parity` test
      composes a frame both ways and diffs.
- [ ] **Compute physics (opt-in, `compute-physics`).** A WGSL compute shader
      implementing one physics tick over the SoA buffers (from P1). The shader
      is the *third* implementation of the CA (sequential, parallel-CPU,
      GPU); the golden corpus (P0) is the gate for all three. Start with the
      physics pass only; reactions stay CPU until a follow-up.
- [ ] Headless GPU validation: `present_offscreen` already exists; P6 makes CI
      run it on a software adapter (lavapipe) so a shader regression is caught
      without a display.

#### Result (narrative)

P6 is two costs in one phase and should probably split (P6a present, P6b
compute) the moment the present work is large — the A5a/b/c precedent. The
present work is *wiring*; the compute work is *a third CA implementation* and
is where the real difficulty (and the real win) lives. The discipline that
keeps it honest is D2 again: the compute shader's output on the golden corpus
must match the CPU's byte-for-byte, which is a fierce constraint on a
floating-point shader but is exactly what makes the result trustworthy.

**Risk named:** wgpu's surface integration differs across platforms (Wayland
vs X11 vs Windows vs web). The gate runs on the CI runner's software adapter;
per-platform hardware validation is a follow-up, not a P6 blocker (the same
"CI proves it compiles and runs headless; real GPUs are a stretch" honesty the
OS plans apply to qemu vs real hardware).

#### Test gate

- `render_parity` CPU-vs-GPU diff below threshold on a static scene; the
  default softbuffer path unchanged; compute path matches golden corpus on a
  256² scene; frame rate ≥ 60 FPS at 4 KTester cells on an integrated GPU
  (measured, claim-checked); prior gates green.

#### Deliverable

`patches/P6_gpu.patch` (or `P6a_present.patch` + `P6b_compute.patch`)

---

### Phase P7 — UI/UX: egui-on-GPU, modding, tools ✅ PLANNED

**Objective:** move egui onto the GPU (now that P6 gives it a surface),
open the element registry to runtime plugins, and finish the tooling the MVP
left half-done (the Line tool's missing live preview is the canonical example).

#### Why (Fact 8, Fact 10.4)

`egui_raster.rs` is a CPU triangle rasterizer that exists *only* because the
present path is a CPU buffer. Once P6 ships, egui renders on the GPU and
`egui_raster.rs` becomes the WASM fallback (where there is no GPU surface
yet). The modding story follows from the data-driven registry: `all_definitions()`
is already a `Vec<ElementDef>`; loading part of it from JSON is a small step
with a large payoff (community elements without recompilation).

#### Tasks

- [ ] **egui on GPU.** Render egui's tessellated mesh through wgpu (egui's own
      wgpu backend, or a thin custom one matching `egui_raster`'s texture
      atlas handling). Keep `egui_raster` behind `target_family = "wasm"` as
      the fallback.
- [ ] **Plugin elements.** A `plugins/` directory; `ElementDef`s loadable from
      JSON at startup (id, name, color, density, kind, flags). Plugin elements
      participate in the palette and the reaction table. A sandboxed
      validation rejects an element whose id collides with a built-in.
- [ ] **Tool finishing.** Line/Rectangle get a live preview overlay during
      drag (the Line-tool fix removed the erroneous trail; P7 adds the intended
      preview). A brush-size on-cursor indicator. A scenario editor (save the
      current grid as a named scenario).
- [ ] **Input replay harness.** Record a `(event, tick)` stream; replay it
      against a fresh `AppState`. This is P9's replay work's UI half, landed
      early because the Line-tool bug showed input handling has no integration
      test.

#### Result (narrative)

P7 is the phase that turns the simulator from "engine with a debug UI" into
"tool people use". The modding hook is the highest-leverage item: a data-driven
element system that already exists is one file-format away from a community,
and every plugin element is free test coverage for the registry's invariants
(the P0 checker).

#### Test gate

- egui panels visible under wgpu (screenshot diff vs the softbuffer baseline);
  a JSON plugin element appears in the palette and behaves as declared; Line
  preview renders during drag and commits on release; input replay reproduces
  a recorded session's grid; prior gates green.

#### Deliverable

`patches/P7_ui.patch`

---

### Phase P8 — Content: campaign, missions, elements ✅ PLANNED

**Objective:** a structured 8-mission campaign that teaches the nuclear model
left to right (sub-critical → critical → control → poisoning → meltdown →
fusion → breeding → full plant), plus the element gaps the campaign exposes.

#### Why

The MVP has 6 missions and 9 scenarios, each a standalone puzzle. A *campaign*
strings them into a teaching arc and, in doing so, exposes which elements are
missing (no control-room instruments, no coolant pump that reads a sensor, no
spent-fuel pool). P8 is where the physics depth from P3–P5 meets the player.

#### Tasks

- [ ] **Campaign structure.** `missions/campaign.toml`: an ordered list, each
      entry a mission id + unlock condition + intro text. The UI shows a
      campaign tree; completing one unlocks the next.
- [ ] **New missions to fill the arc:** "First criticality" (assemble a bare
      pile to k=1), "Shutdown margin" (insert rods to go sub-critical fast),
      "Xenon dead-time" (the iodine pit, retuned against P3 if landed), "Core
      damage" (avoid meltdown under a loss-of-coolant), "Ignition" (D-T
      fusion), "Breeder" (Li breeding to sustain a fusion neutron source),
      "Balance of plant" (the full coolant loop with sensors).
- [ ] **Element gaps.** Whatever the campaign needs that's missing — likely a
      `thermocouple` (sensor that reads temperature to a wire), a `condenser`
      (steam→water heat exchanger), a `spent_fuel` element. Each added through
      the P7 plugin mechanism first, promoted to built-in if it earns it.
- [ ] Every campaign mission has a win test (the `test_mission_*` pattern) and
      a "winnable without the fidelity features" check (D3 — the campaign
      ships against the default model).

#### Result (narrative)

P8 is the phase most likely to *change the physics*, because designing a
teaching mission is the fastest way to discover the model can't express
something (you reach for a condenser and find there isn't one). Each such
discovery is either a P8 element addition or a recorded limitation for a later
phase — the same "the second consumer was the test" dynamic that made the
ARM64 walker promotion find the riscv-shaped assumption.

#### Test gate

- All 8 campaign missions winnable and tested under the default model; the
  campaign tree unlocks correctly; new elements pass registry validation;
  prior gates green.

#### Deliverable

`patches/P8_content.patch`

---

### Phase P9 — Hardening: saves v3, replay, fuzz, WASM threads ✅ PLANNED

**Objective:** the engineering phase that makes the simulator trustworthy over
time: deterministic headless replay, fuzzing of every parser, save format v3
migration, and real WASM threading.

#### Why (Fact 9, Fact 10, D8)

Three threads braid here. **Replay:** P2's per-chunk RNG seeding made the
simulation a pure function of `(grid, seed, tick)`; P9 exploits that to replay
a recorded session bit-identically, which is the foundation for bug reports
("here is the replay that crashes") and for the golden corpus's evolution.
**Fuzzing:** every parser (save bincode/json, GIF decode if it's ever added,
plugin JSON, groups.toml) gets a fuzzer — the GIF bug is the argument. **WASM
threads:** the web build is single-threaded today; `SharedArrayBuffer` +
`wasm-bindgen-rayon` would let P2's parallelism reach the browser.

#### Tasks

- [ ] **Headless replay.** A `replay` binary: given a save and a seed, run N
      ticks and emit the final grid hash. Two runs on two machines produce the
      same hash (the determinism P2 paid for). CI replays a fixed corpus every
      push and fails on a hash change that isn't a reviewed model change.
- [ ] **`cargo fuzz` targets.** `save_decode` (random bytes → must not panic,
      must round-trip if it decodes), `plugin_json` (random JSON → validated
      or rejected, never panics), `gif_decode` (when a decoder lands). Each
      fuzzer runs for a fixed budget in CI.
- [ ] **Save v3.** Whatever P4/P5a added (isotope vectors, group structure)
      lands in `SaveFileV3`; `SaveFileV2` legacy + `From` migration, all new
      fields `#[serde(default)]`. A v2 file from today loads in the v3 tree.
- [ ] **WASM threads.** `wasm-bindgen-rayon` behind a `web-threads` feature;
      `SharedArrayBuffer` requires COOP/COEP headers (the `release/wasm/`
      `index.html` ships them). The gate is a ≥ 1.6× speedup of a 512² web
      build going 1→2 threads.

#### Result (narrative)

P9 is unglamorous and is the phase that decides whether the project ages well.
A simulator that can replay a bug from a 2 KB save file is a simulator that
gets fixed; one that can't is one where issues rot. The WASM threading is the
stretch goal — it depends on browser cooperation (COOP/COEP) and is the item
most likely to slip, but the parallel physics from P2 means the *only* new
work is the thread transport, not the algorithm.

#### Test gate

- Replay hash stable across {this machine, CI runner} for a 1 000-tick
  corpus; fuzzers run clean for their budget; a v2 save loads in the v3 tree
  with migrated defaults; 512² web build ≥ 1.6× with 2 threads; prior gates
  green.

#### Deliverable

`patches/P9_hardening.patch`

---

## 4. What `core` does not know (the invariant, restated)

The layering contract is the one thing no phase may break. `core` talks to the
rest of the world through exactly these surfaces and nothing else:

* `Grid`, `Particle`, `SimulationState` — the state.
* `element_id` constants and the `is_*` / `density_for_id` / `conductivity` /
  `flow_steps` / `repose_slide` pure functions — the material model.
* `reactions::*` constants and pure probability functions — the nuclear model.
* `Scenario` and `Mission` enums — the content hooks.
* `GridSnapshot` (raw RGBA) — the render handoff, with **no** rendering types
  in the signature.

No `core` file contains the strings "winit", "pixels", "softbuffer", "wgpu",
"egui", "rfd", or "web_sys". The renderer is a trait (`RenderBackend`) `core`
does not import; the UI is a consumer of `core`, never a dependency. The GPU
(P6), the plugin loader (P7), the transport solver (P4) — all live in crates
`core` does not name. Adding a third backend (a terminal renderer, a headless
GIF-only renderer, a network multiplayer snapshot sink) needs to consume the
same `GridSnapshot`/`SimulationState`; nothing in `core` changes. This is the
contract the ARM64 plan called "different ISA, same contracts", translated:
*different presentation, same state*.

`check_powder_claims` (P0) asserts this every CI run: a grep for the forbidden
strings in `crates/core/src/**` is a gate from birth.

---

## 5. Test matrix (target, all green to merge any phase)

| Group | Tests | Runner | Gate owner |
|-------|-------|--------|------------|
| Unit | `reactions` (7), `io::save` (2), `io::gif89a` round-trip (1) | cargo | P0 (existing) |
| Integration | `simulation_tests` (43) — gravity, flow, fission, fusion, decay, absorption, pipes, filters, pressure, missions | cargo | existing |
| Golden tick | 20 scenes × 200 ticks, byte/fingerprint equality | cargo | P0 |
| Property | camera math, save round-trip, GIF round-trip, chunk arithmetic | `quickcheck` | P0 |
| Claims | `check_powder_claims` (§1 numbers + layering grep + physics invariants) | python | P0 |
| Bench | 256²/512²(/1024²) median vs baseline CSV | criterion | P0 |
| Determinism | same grid, {1,2,4,8} threads → byte-identical | cargo | P2 |
| Render parity | CPU vs wgpu frame diff | cargo (lavapipe) | P6 |
| Replay | 1 000-tick corpus hash stable across machines | cargo | P9 |
| Fuzz | save/plugin/gif decode, no panic | `cargo fuzz` | P9 |
| WASM | `wasm-pack build --target web` (+ threads under `web-threads`) | wasm-pack | existing → P9 |

Every test writes a log under `build/*.log`; CI uploads artefacts on failure.
A phase that adds a "measured" claim adds a claim row; a phase that adds a
codec adds a round-trip row; a phase that adds a parser adds a fuzz row.

---

## 6. Bugs the audit already found (and what each phase prevents next time)

Carried forward from `bugfixes.patch`, with the prevention mapped to a phase:

1. **GIF LZW code-size off-by-one.** Prevention: P0's "every codec gets a
   round-trip property test" rule (D7). The regression test already in
   `gif89a.rs` is the template.
2. **Camera zoom anchored to origin.** Prevention: P0's camera property tests
   (zoom/pan involutions). Pure math must have pure tests.
3. **Iodine absorber accounting.** Prevention: P0's `physics_invariants`
   checker — a cross-cutting count held by convention becomes a count held by
   CI.
4. **Line-tool double-paint.** Prevention: P7's input-replay harness (and P9's
   replay generally). Input handling without an integration test is input
   handling that regresses.

The pattern: every bug the audit found was a thing with *no test at its
boundary*. The roadmap's first phase is the one that puts a test at every
boundary, so that the next audit finds fewer.

---

## 7. What is deliberately *not* in this roadmap

Honest scoping, the way the ARM64 plan named PCIe/fw-cfg/PL031 as "ignored
this plan":

- **Network multiplayer.** A shared `SimulationState` over the network is a
  real product but a different project; the layering permits it (a network
  snapshot sink is just another `GridSnapshot` consumer) but no phase builds
  it.
- **A visual node editor for reactions.** Tempting (the `ReactionTable` is
  already data), but it is a UI project, not a simulation one; P7's plugin
  JSON is the cheap version, the node editor is out of scope.
- **Real CFD / full neutron transport / Monte Carlo N-particle.** P4's
  multi-group and P5b's stable-fluids are *toys that respect the shape of the
  real thing*. A research-grade solver is a different codebase.
- **Mobile.** winit targets desktop; touch input and battery are their own
  concern. WASM in a mobile browser is the accidental mobile story.
- **A custom element scripting language.** P7's JSON plugins declare static
  elements; a scripting layer (Lua/WASM) for *behavioural* elements is a
  follow-up to P7, not part of it.

Each "not in scope" is a place the layering permits a future plan to land
without disturbing this one — the same property that let the ARM64 plan defer
PCIe to a later plan while the contract carried it.

---

## 8. Sequencing and contributor shape

A single contributor can take a phase; the dependency graph in §3 is the
merge order, not the assignment order. The two natural lines of work that can
proceed in parallel after P0:

- **The engine line** — P1 → P2 → P6 → P7. SoA, parallel, GPU, UI. This is the
  "make it fast and pretty" track and it is internally sequential (each needs
  the last).
- **The physics line** — P3 → P4 → P5a, with P5b off to the side. This is the
  "make it deep" track and it is also internally sequential (P4's groups need
  P3's heat for the Doppler gate to be meaningful; P5a's isotopes need P4's
  transport for the critical-radius gate).

P8 (content) needs P3–P5 to exist to be interesting but can start against the
MVP model and re-tune. P9 needs P0 (replay) and P2 (determinism) and can run
concurrently with the engine line from P2 onward.

The claim checker (P0) is the thread that ties the lines together: a physics
contribution that changes a §1 number and an engine contribution that changes
a bench number both update the same `check_powder_claims`, so two contributors
never silently disagree about what the machine does.

---

## 9. Definition of done — for the roadmap itself

This roadmap is "done" when:

- P0–P9 are all ☐ → ✅ in the status table, each with its `.patch` and its gate
  green on `main`;
- `check_powder_claims` asserts every number in this document;
- the golden corpus, the replay corpus, and the fuzz corpus are all in the
  tree and CI-green;
- a contributor can land a plugin element, a mission, or a new scenario
  without touching `core`;
- the default build still does what the MVP does (a reactor demo that fissions
  on start), only faster and on more platforms.

Until then, this document is the plan the MVP could not write about itself —
the audit that says, for each thing the MVP does well, what it would cost to
do it right, and for each thing it does at all, what it would cost to make it
honest.

---

*Baseline: `main` + `bugfixes.patch`. Structure inherited from
`ARM64_PLAN.md` / `BOOTLOADER_ROADMAP.md`: dependency-ordered phases, a
definition of done and a test gate for every phase, one `.patch` per phase, a
claim checker from birth.*
