#!/usr/bin/env bash
# Parse cassandra-stress or go-ycsb log → JSON summary on stdout.
# Usage: parse-stress-log.sh <cassandra-stress|ycsb> <log-file>
set -euo pipefail

KIND="${1:?cassandra-stress|ycsb}"
LOG="${2:?log file}"

python3 - "$KIND" "$LOG" <<'PY'
import json, re, sys

kind, path = sys.argv[1], sys.argv[2]
text = open(path).read()

if kind == "cassandra-stress":
    rates = [float(m.group(1).replace(",", "")) for m in re.finditer(r"Op rate\s*:\s*([\d,.]+)\s*op/s", text)]
    threads = [int(m.group(1)) for m in re.finditer(r"Running with\s*(\d+)\s*thread", text)]
    p95 = [float(m.group(1)) for m in re.finditer(r"95th percentile\s*:\s*([\d.]+)\s*ms", text, re.I)]
    if not rates:
        # Per-interval throughput lines from auto-rate runs
        rates = [
            float(m.group(1).replace(",", ""))
            for m in re.finditer(
                r"total,\s*\d+,\s*([\d,]+),\s*[\d,]+,\s*[\d,]+,", text
            )
        ]
    peak = max(rates) if rates else None
    idx = rates.index(peak) if peak is not None and rates else 0
    out = {
        "tool": "cassandra-stress",
        "peak_ops_per_sec": peak,
        "threads_at_peak": threads[idx] if threads and idx < len(threads) else (threads[-1] if threads else None),
        "p95_ms": p95[idx] if p95 and idx < len(p95) else (max(p95) if p95 else None),
        "all_op_rates": rates,
    }
elif kind == "ycsb":
    best_t = re.search(r"BEST_THREADS=(\d+)", text)
    best_ops = re.search(r"BEST_OPS_PER_SEC=([\d.]+)", text)
    best_p95 = re.search(r"BEST_P95_US=([\d.]+)", text)
    peak = float(best_ops.group(1)) if best_ops and best_ops.group(1) not in ("", "0") else None
    threads = int(best_t.group(1)) if best_t and best_t.group(1) not in ("", "0") else None
    p95_us = float(best_p95.group(1)) if best_p95 and best_p95.group(1) else None
    if peak is None:
        runs = re.findall(
            r"--- threadcount=(\d+) ---.*?^TOTAL  - Takes\(s\): [^,]+, Count: (\d+), OPS: ([\d.]+).*?95th\(us\): (\d+)",
            text,
            re.MULTILINE | re.DOTALL,
        )
        if not runs:
            runs = [
                (None, m.group(1), m.group(2), m.group(3))
                for m in re.finditer(
                    r"^TOTAL  - Takes\(s\): [^,]+, Count: (\d+), OPS: ([\d.]+).*?95th\(us\): (\d+)",
                    text,
                    re.MULTILINE,
                )
            ]
            runs = [(None, r[0], r[1], r[2]) for r in runs]
        best = None
        for tcount, count, ops, p95 in runs:
            if count != "100000":
                continue
            ops_f = float(ops)
            if best is None or ops_f > best[0]:
                best = (ops_f, int(tcount) if tcount else None, float(p95))
        if best:
            peak, threads, p95_us = best[0], best[1], best[2]
    p95_ms = p95_us / 1000.0 if p95_us else None
    out = {
        "tool": "go-ycsb",
        "peak_ops_per_sec": peak,
        "threads_at_peak": threads,
        "p95_ms": p95_ms,
    }
else:
    raise SystemExit(f"unknown kind: {kind}")

print(json.dumps(out, indent=2))
PY
