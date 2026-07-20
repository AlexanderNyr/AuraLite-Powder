# AuraLite Powder

Cross-Platform Falling-Sand Simulation with Advanced Nuclear Physics

Powder Toy inspired simulation written in Rust 1.97.1, targeting Windows, Linux, macOS, and WebAssembly.

## Features

- **Core Simulation**
  - Grid Vec<Particle> row-major, configurable 128×128 → 1024×1024+
  - Chunking 32×32 with dirty tracking and adaptive threading (rayon)
  - Physics passes: gravity, density-based swapping, liquid flow, gas buoyancy
  - Three-pass update: Physics → Reactions → Effects

- **Nuclear Physics**
  - Fissile: U-235, U-238, Pu-239, Pu-240
  - Moderator / Coolant: Heavy Water D2O, Graphite, Water
  - Structural: Lead, Concrete, Steel, Boron (absorber)
  - Radiation: thermal/fast neutron, gamma, alpha, beta
  - Fuel/Products: Depleted Uranium, Fission Products, Tritium, Deuterium, Helium, Molten Fuel
  - Reactions: fission with neutron multiplication, fusion D+T → He + n, decay chains
  - Chain reaction via neutron_queue with 1-3 tick delay
  - Temperature diffusion, criticality, meltdown, radiation penetration

- **Rendering**
  - Abstract `RenderBackend` trait
  - `SoftbufferBackend` primary (pixels + softbuffer + winit)
  - `WgpuBackend` optional (feature `wgpu-renderer`), WGSL shader in `assets/shaders/shader.wgsl`
  - Temperature glow overlay, zoom/pan camera

- **UI**
  - Component tree: SimulationController, GridView, PalettePanel, ToolPanel, PropertyInspector, SaveLoadPanel, InfoPanel
  - Drawing tools: Brush (radius), Line (Bresenham), Fill (flood-fill), Eraser, Rectangle
  - Zoom (wheel) / Pan (right-drag)
  - Egui integration (feature `native-ui`, optional) with palette search, sliders
  - Native file dialogs via `rfd`

- **IO**
  - Versioned `SaveFile` with compact (non-air) + full modes
  - Serialization via `bincode` + `serde_json`, optional `zstd` compression
  - Migration for future versions

- **Web / WASM**
  - `aura-lite-web` shim binding core to <canvas>
  - `wasm` crate exports `start_sim(canvas_id)` for browser
  - Builds with `wasm-pack build --target web`

## Project Structure

```
aura_lite/
├── Cargo.toml (workspace + feature flags)
├── crates/
│   ├── core/ - simulation kernel
│   ├── elements/ - element registry & ReactionTable
│   ├── renderer/ - RenderBackend abstraction
│   ├── ui/ - components, brushes, camera
│   ├── io/ - save/load
│   ├── utils/ - Vec2, Rect, ChunkPool, ThreadPool, math
│   └── web/ - WASM adapter
├── src/main.rs - native entry
├── wasm/
│   ├── Cargo.toml
│   └── src/lib.rs - WASM entry exports start_sim()
└── assets/shaders/shader.wgsl
```

## Building

See `BUILD.md` for per-platform instructions.

Quick native:

```bash
cargo run --release --features default
# or with wgpu backend
cargo build --features wgpu-renderer
```

Web:

```bash
wasm-pack build --target web --manifest-path wasm/Cargo.toml
# serve with e.g. python -m http.server and load pkg
```

## Controls

- Mouse left drag: paint with selected element
- Right drag: pan camera
- Wheel: zoom
- Space: pause/resume
- C: clear
- S: quick save to temp dir
- 1-6: quick select Sand, Water, U235, Neutron, Deuterium, Tritium
- In egui mode: full palette search, tool selection, temp slider, save/load dialogs

## Nuclear Demo

On start, a small reactor demo is pre-filled:

- Concrete floor/walls, graphite moderator, U-235 pile, boron absorber rods, one thermal neutron to start chain
- Plus D+T sample at top left at high temperature for fusion glimpse

Observe fission chain reactions: neuron queue propagates, gamma emission, temperature spike, fission products.

Place moderator (water, heavy water, graphite) to thermalize fast neutrons and increase U-235 fission probability. Use boron to absorb neutrons and control.

Place Deuterium + Tritium adjacent at >1500K to trigger fusion → Helium + fast neutron.

## Performance

- <65536 cells: single-threaded; >=65536: parallel ready (currently fallback to single thread for correctness, but architecture supports rayon par_chunks)
- ChunkPool with active tracking
- Per-chunk RNG via fastrand, no global lock
- Grid resize with particle migration

## License

Apache-2.0
