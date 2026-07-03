#!/usr/bin/env python3
"""Parse Track AA index-scaling reports and emit verdict vs pre-registered criteria."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

TOPO_NODES = {
    "scylla-1": 1,
    "scylla-2n": 2,
    "scylla-4n": 4,
}

BASELINE_2N = 18_517.0
BASELINE_4N = 20_118.0
RAW_2N = 29_524.0


def parse_reports(reports_dir: Path) -> dict[tuple[str, str, str], dict]:
    """Key: (topology, variant, l2) -> {ops, rt, p99}."""
    out: dict[tuple[str, str, str], dict] = {}
    for path in reports_dir.glob("*.json"):
        if "idx-z1off" not in path.name and "idx-validate" not in path.name:
            continue
        try:
            data = json.loads(path.read_text())
        except (json.JSONDecodeError, OSError):
            continue
        if data.get("experiment_id") not in ("bm-m4", "bm-m5"):
            continue
        dims = data.get("dimensions") or {}
        if dims.get("storage") != "scylla":
            continue
        topo = dims.get("scylla_topology") or "scylla-1"
        m = re.search(
            r"idx-z1off-(m4|t8|t64)-l2(on|off)|idx-validate-m4-default",
            path.name,
        )
        if not m:
            continue
        if m.group(0).startswith("idx-validate"):
            variant, l2 = "m4", "default"
        else:
            variant, l2 = m.group(1), m.group(2)
        metrics = data.get("metrics") or {}
        notes = data.get("notes") or ""
        rt = None
        rm = re.search(r"rt_per_append=([\d.]+)", notes)
        if rm:
            rt = float(rm.group(1))
        key = (topo, variant, l2)
        ops = float(metrics.get("achieved_ops_per_sec", 0.0))
        prev = out.get(key, {})
        if ops >= prev.get("ops", 0.0):
            out[key] = {
                "ops": ops,
                "rt": rt,
                "p99": metrics.get("p99_ms"),
                "file": path.name,
            }
    return out


def verdict(rows: dict) -> list[str]:
    lines: list[str] = []
    m4_2n_off = rows.get(("scylla-2n", "m4", "off"), {}).get("ops", 0.0)
    m4_2n_on = rows.get(("scylla-2n", "m4", "on"), {}).get("ops", 0.0)
    t64_2n_off = rows.get(("scylla-2n", "t64", "off"), {}).get("ops", 0.0)
    t64_2n_on = rows.get(("scylla-2n", "t64", "on"), {}).get("ops", 0.0)
    m4_4n_off = rows.get(("scylla-4n", "m4", "off"), {}).get("ops", 0.0)
    m4_1n_off = rows.get(("scylla-1", "m4", "off"), {}).get("ops", 0.0)

    lines.append("## Pre-registered verdict")
    if m4_2n_off and t64_2n_off and t64_2n_off >= 1.5 * m4_2n_off:
        lines.append(
            "- **Index partition confirmed (multi-node):** T=64 L2 off "
            f"({t64_2n_off:.0f}) >= 1.5× single-topic ({m4_2n_off:.0f}) on 2n."
        )
        lines.append("- **Next action:** Phase 2 minimal fix (default L2 on).")
    elif m4_2n_off and abs(m4_2n_on - m4_2n_off) / max(m4_2n_off, 1) < 0.1:
        lines.append(
            "- **L2 insufficient on Z1 off:** m4 l2on ≈ l2off on 2n "
            f"({m4_2n_on:.0f} vs {m4_2n_off:.0f})."
        )
        lines.append("- **Next action:** Investigate bench/driver/VPC; retry Track W 4n.")
    else:
        lines.append("- **Mixed/partial:** Review matrix below.")

    if m4_1n_off and m4_2n_off:
        eff = m4_2n_off / TOPO_NODES["scylla-2n"]
        lines.append(
            f"- 1n m4 l2off: {m4_1n_off:.0f} ops/s; 2n per-node efficiency: {eff:.0f} ops/s/node."
        )
    if m4_4n_off:
        lines.append(f"- 4n m4 l2off: {m4_4n_off:.0f} ops/s (baseline {BASELINE_4N:.0f}).")
    if t64_2n_on and t64_2n_off and abs(t64_2n_on - t64_2n_off) / max(t64_2n_off, 1) < 0.1:
        lines.append(
            "- **L2 effective after warmup:** t64 l2on ≈ t64 l2off at Z1 off — default L2 likely enough."
        )
    return lines


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--reports-dir", type=Path, required=True)
    ap.add_argument("--out", type=Path, default=None)
    args = ap.parse_args()
    rows = parse_reports(args.reports_dir)

    lines = ["# Track AA — Index scaling analysis", ""]
    lines.append("| topology | variant | L2 | ops/s | rt/append | p99 ms |")
    lines.append("| --- | --- | --- | --- | --- | --- |")
    for topo in sorted({k[0] for k in rows}):
        for variant in ("m4", "t8", "t64"):
            for l2 in ("off", "on", "default"):
                r = rows.get((topo, variant, l2))
                if not r:
                    continue
                rt = f"{r['rt']:.2f}" if r.get("rt") is not None else "—"
                p99 = f"{r['p99']:.1f}" if r.get("p99") is not None else "—"
                lines.append(
                    f"| {topo} | {variant} | {l2} | {r['ops']:.0f} | {rt} | {p99} |"
                )
    lines.append("")
    lines.extend(verdict(rows))
    lines.append("")
    lines.append(f"Reference baselines: 2n ceiling m4 z1off {BASELINE_2N:.0f}; raw 2n {RAW_2N:.0f}.")

    text = "\n".join(lines) + "\n"
    if args.out:
        args.out.write_text(text)
    print(text)


if __name__ == "__main__":
    main()
