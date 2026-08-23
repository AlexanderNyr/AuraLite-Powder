# Build Instructions

## Requirements

- Rust 1.97.1+
- `wasm-pack` for web target: `cargo install wasm-pack`
- On Linux: dependencies for winit softbuffer (libxcb, libxkbcommon)
  - Ubuntu: `sudo apt install libxcb1-dev libxkbcommon-dev`

## Native Default (softbuffer + egui)

```bash
cargo run --release
# or explicit
cargo run --release --features "softbuffer-renderer native-ui"
```

This uses:

- `pixels` 0.15
- `winit` 0.29
- `softbuffer` 0.4
- `egui` 0.28 + `rfd` for dialogs

## WGPU Backend

```bash
cargo build --features wgpu-renderer
cargo run --release --features wgpu-renderer
```

The `WgpuBackend` loads `assets/shaders/shader.wgsl`. Validation is performed at run. In MVP it falls back to softbuffer blitting if needed, but architecture supports full GPU pipeline.

Ensure feature combinations:

- `softbuffer-renderer` is default primary
- `wgpu-renderer` optional; can be combined: `--features "softbuffer-renderer wgpu-renderer"`

## Compression

```bash
cargo build --features compression
```

Enables `zstd` for save files.

## WebAssembly

### Build

```bash
wasm-pack build --target web --manifest-path wasm/Cargo.toml --release
# or
cd wasm && wasm-pack build --target web
```

Output in `wasm/pkg/`.

### Run in browser

Create an `index.html`:

```html
<!DOCTYPE html>
<html>
<body>
<canvas id="canvas" width="512" height="512"></canvas>
<script type="module">
import init, { start_sim } from "./pkg/aura_lite_wasm.js";
async function run() {
  await init();
  start_sim("canvas");
}
run();
</script>
</body>
</html>
```

Serve:

```bash
python -m http.server 8000
# open http://localhost:8000
```

Note: web build excludes `rfd`; use Blob download/upload for saves.

### WASM threading

Current WASM build is single-threaded for compatibility. ThreadPool is no-op in WASM. Future: SharedArrayBuffer + WebWorker with `wasm-bindgen-rayon`.

## Testing & Lint

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo bench
```

## CI

Recommended CI checks:

- `cargo test --all-features` on Linux, Windows, macOS
- `cargo clippy`
- `wasm-pack build --target web`

## Features Matrix

| Feature | Crate | Description |
|---|---|--|
| softbuffer-renderer | renderer | Primary CPU renderer |
| wgpu-renderer | renderer | GPU renderer optional |
| native-ui | ui | egui + file dialogs |
| web | web + wasm | WASM bindings |
| compression | io | zstd save compression |

## Troubleshooting

- `pixels` requires window surface; headless environment -> use `cargo run --no-default-features` to run headless nuclear benchmark
- If `egui-winit` reports clipboard error on Linux Wayland, install `libclipboard`
