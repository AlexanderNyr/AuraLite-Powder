# Architecture

## Overview

AuraLite Powder follows strict modular separation:

- `core` zero rendering/UI knowledge, compiles without renderer/ui/io
- `elements` defines Element trait + registry + ReactionTable
- `renderer` abstract backend
- `ui` component framework
- `io` versioned serialization
- `utils` math/helpers
- `web` thin shim

Data flow:

```
User Input (winit/web-sys)
  -> UI (AppState, BrushTool, Camera)
  -> SimulationState.tick()
      Pass1 Physics (gravity density swap, chunking)
      Pass2 Reactions (fission, fusion, decay, neutron_queue)
      Pass3 Effects (temp diffusion, meltdown)
  -> GridSnapshot (RGBA via color_for_element)
  -> RenderBackend (softbuffer pixels blit or wgpu storage texture + wgsl)
  -> Window / <canvas>
```

## Grid

- `Vec<Particle>` row-major
- `Particle { element_id:u16, temperature:u16, flags:u8, lifetime:u8 }`
- Empty = element_id 0
- Configurable resize with migration

## Chunking & Threading

- CHUNK_SIZE = 32
- ChunkMeta with dirty bbox to skip empty
- ChunkPool tracks active chunks
- Adaptive: if area < 65536, single thread; else rayon par ready (currently sequential fallback for correctness)
- Per-chunk SmallRng via fastrand seed = global seed + tick + chunk_idx
- No grid-wide lock; chunk-to-chunk migration deferred to synchronous phase after parallel pass

## Element System

Trait:

```rust
pub trait Element: Send+Sync {
  id(), name(), color(), density(), temperature()
  update(ctx) -> UpdateResult
  react(neighbor, ctx) -> ReactionEvent
}
```

Registry holds ElementDef static list (data-driven). ReactionTable HashMap<ReactionPair, Vec<ReactionOutcome>> with probability, products, energy_change, particle_spawns.

## Nuclear Mechanics

- Fission: thermal neutron + U235 -> fission products + 2-3 fast neutrons (delayed via queue) + gamma + 500K spike. Probability depends on element, neutron energy, temperature.
- Moderator: water/heavy water/graphite slows fast -> thermal with prob 0.3-0.5
- Absorber: boron absorbs neutrons -> fallout + alpha
- Fusion: D+T at >1500K -> He + fast neutron + massive temp spike
- Decay: per-element half_life_ticks, daughter, radiation (alpha/beta/gamma)
- Chain Reaction: neutron_queue VecDeque<(x,y,delay,energy)>, processed each tick, propagation to fissile triggers fission
- Temperature: diffusion via averaging 8 neighbors * diffusion_rate, cooling towards 293K baseline
- Meltdown: fissile temp >2000 -> molten fuel, water >2500 -> hydrogen gas
- Radiation: penetration_depth per type, deposits energy on neighbor temp

## Rendering

`RenderBackend` trait:

```rust
fn init(w,h) -> Self
fn render(&mut self, pixels: &[Rgba<u8>])
fn resize(w,h)
```

SoftbufferBackend:

- Holds RGBA buffer
- main.rs creates pixels::Pixels via winit window, calls renderer to produce pixel buffer, blits to pixels frame
- Handles zoom/pan via Camera.world_to_screen transform during frame construction
- Screenshot via image crate (encode frame buffer)

WgpuBackend (optional):

- Creates a wgpu device/queue at init and compiles `assets/shaders/shader.wgsl`
- Uploads a grid-sized RGBA8 texture each frame (row pitch 256-aligned)
- Fullscreen triangle + nearest `textureLoad` with camera uniforms
- Falls back to the CPU buffer if no adapter is present

Separation Simulation vs Render threads (planned):

- Arc<RwLock<GridSnapshot>> so renderer reads latest snapshot without tearing
- Fixed tick rate 60Hz simulation, render at display refresh VSync

## UI

AppState tree:

```
AppState
├── SimulationController (speed, pause, tick_rate, resize)
├── GridView (zoom/pan)
├── Camera
├── PalettePanel (search filter via all_definitions)
├── ToolPanel (Brush radius, line start)
├── PropertyInspector (hovered particle stats)
├── SaveLoadPanel (rfd dialogs, compression toggle)
└── InfoPanel (fps, counts)
```

Drawing tools:

- Brush: circle radius scan, dx^2+dy^2 <= r^2
- Line: Bresenham
- Fill: BFS flood fill with stack depth limit 10000
- Eraser: set to AIR
- Rectangle: border or filled

Egui integration (native-ui feature):

- Palette with color preview rect
- Tool sliders
- Simulation controls
- Save/load using rfd
- Meshes are CPU-rasterized into the `pixels` framebuffer (`egui_raster`) so the panels are visible

Web Considerations:

- No rfd, use Blob
- Canvas 2D context putImageData via ImageData::new_with_u8_clamped_array
- Mouse events from web-sys
- Future eframe for egui in WASM

## Save/Load

SaveFile versioned:

```rust
struct SaveFile {
  version, timestamp, grid_width, height, tick_rate, seed,
  particles: Vec<ParticleData>, // compact
  full_grid: Option<Vec<Particle>>,
  settings: SimulationSettings
}
```

Compact mode: only non-air for small files. Full mode for debug.

bincode binary (.aura) + serde_json export (.json). Optional zstd compression behind feature.

Migration trait (future): if version < CURRENT, apply defaults.

## Extensibility

- Plugin elements via runtime registry loading .json/.rmp (future): Design ElementDef to be loadable
- Backend independence: renderer only on pixels or abstract trait; core never imports winit
- Feature gating: all optional behind flags; default produces working binary
- Testing: unit tests in core/io, simulation_tests.rs integration fission/fusion, insta snapshots
- Documentation: every public trait/module has /// docs

## Code Quality

- No unsafe unless justified for WASM interop
- clippy deny warnings in CI
- fmt 100 cols
- Prefer smallvec/arrayvec where bounded (future)

## Future Improvements

- SoA layout for particles if profiling shows cache misses
- Lock-free cross-chunk migration using crossbeam queues
- GPU compute shader for physics (wgpu)
- Proper egui texture rendering into pixels via software raster
- Decay chain visualization
- Heatmap overlay mode
