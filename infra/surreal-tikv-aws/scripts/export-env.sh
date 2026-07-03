#!/usr/bin/env bash
# Print bench env vars for active topology (eval in shell on bench host).
# Usage: export-env.sh <topology-name>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_TIKV_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"

[[ $# -ge 1 ]] || { echo "usage: $0 <topology-name>" >&2; exit 1; }
TOPOLOGY="$1"

MANIFEST="$(manifest_read "$TOPOLOGY")"
export MANIFEST

python3 - <<'PY'
import json, os, sys

m = json.loads(os.environ["MANIFEST"])
pd_ip = m.get("pd_ip")
if not pd_ip:
    for inst in m["instances"]:
        if inst["role"] == "tikv" and inst["index"] == 0:
            pd_ip = inst["private_ip"]
            break

surreal_ip = m.get("surreal_endpoint_ip")
if not surreal_ip:
    if m.get("surreal_lb_on_bench"):
        surreal_ip = "127.0.0.1"
    else:
        for inst in m["instances"]:
            if inst["role"] == "surreal" and inst["index"] == 0:
                surreal_ip = inst["private_ip"]
                break

hw = m.get("component_hardware", {})
print(f"export CONTINUUM_BENCH_TIKV_TOPOLOGY={m['bench_topology']}")
print(f"export CONTINUUM_BENCH_TIKV_PD_ENDPOINT=http://{pd_ip}:2379")
print(f"export CONTINUUM_BENCH_SURREAL_URL=ws://{surreal_ip}:8000")
print(f"export CONTINUUM_BENCH_SURREAL_INSTANCES={m['surreal_instances']}")
print(f"export CONTINUUM_BENCH_SURREAL_HARDWARE={hw.get('surreal', 'aws-t3-small')}")
print(f"export CONTINUUM_BENCH_TIKV_HARDWARE={hw.get('tikv', 'aws-t3-small')}")
PY
