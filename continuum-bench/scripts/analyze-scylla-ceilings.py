#!/usr/bin/env python3
"""Parse Scylla ceiling reports and emit peak matrix + monthly cost table."""

from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path

HOURLY = {
    "aws-t3-medium": 0.0416,
    "aws-c7i-4xlarge": 0.816,
}

TOPO_NODES = {
    "scylla-1": 1,
    "scylla-2n": 2,
    "scylla-4n": 4,
}

TARGETS = [10_000, 50_000, 100_000, 1_000_000]


def parse_reports(reports_dir: Path) -> dict:
    best: dict[tuple[str, str, str, bool], float] = {}
    for path in reports_dir.glob("*.json"):
        try:
            data = json.loads(path.read_text())
        except (json.JSONDecodeError, OSError):
            continue
        if data.get("experiment_id") not in ("bm-m4", "bm-m5"):
            continue
        dims = data.get("dimensions") or {}
        hw = dims.get("hardware")
        storage = dims.get("storage")
        if storage != "scylla" or not hw:
            continue
        topo = dims.get("scylla_topology") or "scylla-1"
        fname = path.name
        for slug in ["scylla-4n", "scylla-2n", "scylla-1"]:
            if slug in fname and not dims.get("scylla_topology"):
                topo = slug
        tag = ""
        m = re.search(r"ceil-z1(on|off)-c\d+", fname)
        if m:
            tag = m.group(0)
        elif "z1-treatment-none" in fname or "z1off" in fname:
            tag = "ceil-z1off"
        elif "z1-baseline" in fname or "z1on" in fname:
            tag = "ceil-z1on"
        z1_on = "off" not in tag and "none" not in fname
        if "ceil-z1off" in fname or "treatment-none" in fname:
            z1_on = False
        if "ceil-z1on" in fname or "baseline-lwt" in fname:
            z1_on = True
        ops = (data.get("metrics") or {}).get("achieved_ops_per_sec", 0.0)
        key = (hw, topo, tag or fname, z1_on)
        best[key] = max(best.get(key, 0.0), float(ops))
    return best


def monthly_cost(ops_peak: float, nodes: int, hourly: float, target: float) -> float:
    if ops_peak <= 0:
        return float("inf")
    clusters = math.ceil(target / ops_peak)
    return clusters * nodes * hourly * 730


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--reports-dir", type=Path, required=True)
    ap.add_argument("--out", type=Path, default=None)
    args = ap.parse_args()
    best = parse_reports(args.reports_dir)
    lines = ["# Scylla ceiling analysis", ""]
    for hw in sorted({k[0] for k in best}):
        lines.append(f"## {hw}")
        lines.append("| topology | Z1 | peak ops/s | 10k/mo | 50k/mo | 100k/mo | 1M/mo |")
        lines.append("| --- | --- | --- | --- | --- | --- | --- |")
        hourly = HOURLY.get(hw, 0.0)
        for topo in sorted({k[1] for k in best if k[0] == hw}):
            nodes = TOPO_NODES.get(topo, 1)
            for z1_on in (True, False):
                peaks = [v for (h, t, _, z) in best if h == hw and t == topo and z == z1_on for v in [best[(h, t, _, z)]]]
                if not peaks:
                    continue
                peak = max(peaks)
                zlabel = "on" if z1_on else "off"
                costs = [monthly_cost(peak, nodes, hourly, t) for t in TARGETS]
                cost_str = " | ".join(f"${c:,.0f}" if math.isfinite(c) else "—" for c in costs)
                lines.append(f"| {topo} | {zlabel} | {peak:,.0f} | {cost_str} |")
        lines.append("")
    out = "\n".join(lines)
    if args.out:
        args.out.write_text(out)
    print(out)


if __name__ == "__main__":
    main()
