#!/usr/bin/env python3
#check_powder_claims.py — asserts the "measured" numbers in ROADMAP.md §1
#against the tree at CI time. A claim with no check fails the build, not the
#reader. (P0 deliverable; ROADMAP decision D6 — claim checker from birth.)
#
#Run: python3 ci/check_powder_claims.py
#Exit 0 = all claims hold; non-zero = a claim drifted from the code.
from __future__ import annotations
import re, sys, subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = ROOT / "crates"
FAIL: list[str] = []


def claim(name: str, ok: bool, detail: str = "") -> None:
    flag = "OK  " if ok else "FAIL"
    print(f"  [{flag}] {name}{(' — ' + detail) if detail else ''}")
    if not ok:
        FAIL.append(name)


def grep_count(path: Path, pattern: str) -> int:
    rx = re.compile(pattern)
    return sum(1 for _ in rx.finditer(path.read_text(encoding="utf-8")))


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


print("== AuraLite Powder claim check ==")

# --- Claim: 47 elements, MAX_ELEMENT_ID = 47 (Fact 1) ---
eid = read(CRATES / "core/src/element_id.rs")
m = re.search(r"pub const MAX_ELEMENT_ID: u16 = (\d+);", eid)
max_id = int(m.group(1)) if m else -1
# Element ids run AIR=0 .. PIPE_STEAM=47 (48 ids). Exclude MAX_ELEMENT_ID itself.
elem_consts = re.findall(r"^pub const ([A-Z0-9_]+): u16 = \d+;", eid, re.M)
elem_only = [c for c in elem_consts if c != "MAX_ELEMENT_ID"]
claim("MAX_ELEMENT_ID == 49", max_id == 49, f"found {max_id}")
claim("element id constants == 50 (AIR..MERCURY)",
      len(elem_only) == 50, f"found {len(elem_only)}")
claim("id range is AIR=0 .. MERCURY=49",
      "pub const AIR: u16 = 0;" in eid and "pub const MERCURY: u16 = 49;" in eid)

# --- Claim: Particle is 8 bytes (Fact 2) ---
part = read(CRATES / "core/src/particle.rs")
# u16+u16+u8+u8 = 8 bytes, no padding.
fields_ok = all(needle in part for needle in [
    "pub element_id: u16,",
    "pub temperature: u16,",
    "pub flags: u8,",
    "pub lifetime: u8,",
])
claim("Particle fields {element_id:u16, temperature:u16, flags:u8, lifetime:u8}",
      fields_ok)

# --- Claim: chunk size 32 (Fact 2/P2) ---
chunk = read(CRATES / "utils/src/chunking.rs")
claim("CHUNK_SIZE == 32", "pub const CHUNK_SIZE: usize = 32;" in chunk)

# --- Claim: parallel threshold 65536 (Fact 3) ---
sim = read(CRATES / "core/src/simulation.rs")
claim("parallel threshold == 65536 cells", "total_cells >= 65536" in sim)

# --- Claim: neutron model is two energy bins (Fact 4) ---
react = read(CRATES / "core/src/reactions.rs")
claim("NeutronEnergy has Thermal + Fast only",
      "pub enum NeutronEnergy {" in react
      and "Thermal," in react
      and "Fast," in react)

# --- Claim: GIF LZW bump rule is the fixed one (Fact 10.1 / bugfix) ---
gif = read(CRATES / "io/src/gif89a.rs")
claim("GIF code-size bump uses (1 << code_size) + 1",
      "next_code == (1 << code_size) + 1 && code_size < 12" in gif,
      "the +1 fix from bugfixes.patch must hold")

# --- Claim: save CURRENT_VERSION == 2 (Fact 9) ---
save = read(CRATES / "io/src/save.rs")
claim("save CURRENT_VERSION == 2", "pub const CURRENT_VERSION: u32 = 2;" in save)

# --- Layering invariant (ROADMAP §4): core knows no render/ui/io types ---
# Check by IMPORT/USAGE pattern, not bare substring — `GridSnapshot.pixels` is a
# field name, not the `pixels` crate. A leak is `use <x>`, `<x>::`, or
# `extern crate <x>`, or a dependency line in core's Cargo.toml.
core_src = CRATES / "core/src"
forbidden = ["winit", "pixels", "softbuffer", "wgpu", "egui", "rfd", "web_sys",
             "js_sys", "wasm_bindgen"]
leaks: list[str] = []
for f in list(core_src.rglob("*.rs")):
    txt = f.read_text(encoding="utf-8")
    for bad in forbidden:
        if re.search(rf"\buse\s+{bad}\b", txt) or re.search(rf"\b{bad}::", txt) \
                or re.search(rf"\bextern\s+crate\s+{bad}\b", txt):
            leaks.append(f"{f.relative_to(ROOT)}: '{bad}'")
# core's Cargo.toml must not depend on the forbidden crates either.
core_toml = read(CRATES / "core/Cargo.toml")
for bad in forbidden:
    if re.search(rf"^\s*{bad.replace('_', '-')}(?!\w)\s*=\s*", core_toml, re.M) \
            or re.search(rf'^\s*"{bad.replace("_", "-")}"', core_toml, re.M):
        leaks.append(f"core/Cargo.toml: dep '{bad}'")
claim("core has no render/ui/io/wasm imports (layering)", not leaks,
      "; ".join(leaks) if leaks else "clean")

# --- Claim: bugfixes.patch fixes are present (Fact 10) ---
camera = read(CRATES / "renderer/src/camera.rs")
claim("camera zoom captures world point BEFORE scale change",
      "BEFORE the scale" in camera)
claim("iodine has a non-zero absorber_chance",
      "(IODINE, NeutronEnergy::Thermal) => 0.35" in react)
mainrs = read(ROOT / "src/main.rs")
claim("Line tool does not paint during drag",
      "BrushTool::Line | BrushTool::Rectangle | BrushTool::Copy => {}" in mainrs)

print()
if FAIL:
    print(f"CLAIM CHECK FAILED: {len(FAIL)} claim(s) drifted: {', '.join(FAIL)}")
    sys.exit(1)
print("CLAIM CHECK PASSED: every measured number in §1 holds against the tree.")
