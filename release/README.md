# AuraLite Powder — Release Builds

## Артефакты

### Linux x86_64
| Файл | Размер | Описание |
|---|---|---|
| `aura_lite` | 1.6 MB | Динамическая сборка (glibc) |
| `aura_lite-static` | 1.7 MB | Статическая сборка (musl, без зависимостей) |

```bash
chmod +x aura_lite && ./aura_lite
```

### Windows x86_64
| Файл | Размер | Описание |
|---|---|---|
| `aura_lite.exe` | 2.8 MB | PE32+ executable |

### WebAssembly
| Файл | Размер | Описание |
|---|---|---|
| `aura_lite_wasm_bg.wasm` | 109 KB | WASM модуль |
| `aura_lite_wasm.js` | 14 KB | JS bindings |
| `aura_lite_wasm.d.ts` | 2.7 KB | TypeScript definitions |
| `index.html` | 1.4 KB | Демо-страница |
| `package.json` | 324 B | NPM metadata |

```bash
cd wasm && python3 -m http.server 8000
# Откройте http://localhost:8000
```

### macOS
Не скомпилировано — cross-compile из Linux требует macOS SDK (Xcode).
На macOS-хосте:
```bash
cargo build --release --target x86_64-apple-darwin    # Intel
cargo build --release --target aarch64-apple-darwin   # Apple Silicon
```

---

- **Rust:** 1.97.1 stable
- **Оптимизация:** release (LTO)
- **Режим:** headless (`--no-default-features`)
- **Лицензия:** Apache-2.0
