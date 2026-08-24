#!/usr/bin/env python3
#bench_record.py — run the criterion benches, record a CSV row, fail CI on a
#regression beyond a recorded tolerance. (P0 deliverable; ROADMAP D2/D6.)
#
#Usage:
#  python3 ci/bench_record.py --record      # append a row to bench/bench.csv
#  python3 ci/bench_record.py                # compare to bench/baseline.csv
#  python3 ci/bench_record.py --baseline     # write bench/baseline.csv fresh
#
#A median that exceeds baseline*(1+TOLERANCE) fails with exit 1.
from __future__ import annotations
import argparse, csv, json, os, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BENCH_DIR = ROOT / "bench"
BASELINE = BENCH_DIR / "baseline.csv"
RECORD = BENCH_DIR / "bench.csv"
TOLERANCE = 0.15            # 15% — measured day-to-day noise is < 8%
BENCH_NAME = "simulation_bench"


def run_benches() -> dict[str, float]:
    """Return {bench_id: median_seconds} from criterion's JSON stream."""
    cmd = ["cargo", "bench", "--bench", BENCH_NAME, "--", "--message-format=json"]
    print(f"$ {' '.join(cmd)}")
    env = dict(os.environ, CARGO_TERM_COLOR="never")
    proc = subprocess.run(cmd, cwd=ROOT, env=env, text=True,
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    out: dict[str, float] = {}
    for line in proc.stdout.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("reason") != "benchmark":
            continue
        bid = msg.get("id") or msg.get("benchmark_id", "?")
        median_ns = msg.get("median")              # nanoseconds
        if isinstance(median_ns, (int, float)):
            out[bid] = median_ns / 1e9
    if not out:
        print("WARN: no benchmark medians parsed; criterion stdout tail:")
        print("\n".join(proc.stdout.splitlines()[-12:]))
    return out


def read_csv(path: Path) -> dict[str, float]:
    if not path.exists():
        return {}
    with path.open() as f:
        return {row["bench"]: float(row["median_s"]) for row in csv.DictReader(f)}


def write_csv(path: Path, mapping: dict[str, float]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["bench", "median_s"])
        for k in sorted(mapping):
            w.writerow([k, f"{mapping[k]:.9f}"])


def append_csv(path: Path, mapping: dict[str, float]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    new = not path.exists()
    with path.open("a", newline="") as f:
        w = csv.writer(f)
        if new:
            w.writerow(["date", "bench", "median_s"])
        from datetime import date
        today = date.today().isoformat()
        for k in sorted(mapping):
            w.writerow([today, k, f"{mapping[k]:.9f}"])


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--record", action="store_true", help="append a row to bench/bench.csv")
    ap.add_argument("--baseline", action="store_true", help="(re)write bench/baseline.csv")
    args = ap.parse_args()

    got = run_benches()
    if not got:
        return 1

    if args.baseline:
        write_csv(BASELINE, got)
        print(f"wrote baseline: {BASELINE}")
        return 0
    if args.record:
        append_csv(RECORD, got)
        print(f"appended row: {RECORD}")
        # fall through to compare as well

    base = read_csv(BASELINE)
    if not base:
        print(f"no {BASELINE}; recording current run as baseline (first run).")
        write_csv(BASELINE, got)
        return 0

    print(f"\n== compare to {BASELINE} (tolerance {TOLERANCE:.0%}) ==")
    regressions: list[str] = []
    for bid, sec in sorted(got.items()):
        b = base.get(bid)
        if b is None:
            print(f"  {bid:28} {sec*1e3:9.3f} ms   (no baseline — recorded)")
            continue
        ratio = sec / b
        flag = "OK "
        if ratio > 1 + TOLERANCE:
            flag = "REG"
            regressions.append(bid)
        elif ratio < 1 - TOLERANCE:
            flag = "improve"
        print(f"  {bid:28} {sec*1e3:9.3f} ms   baseline {b*1e3:9.3f} ms   x{ratio:.2f}  [{flag}]")

    if regressions:
        print(f"\nBENCH REGRESSION: {', '.join(regressions)} exceeded +{TOLERANCE:.0%}")
        return 1
    print("\nBENCH OK: no regression beyond tolerance.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
