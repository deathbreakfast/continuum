#!/usr/bin/env bash
# Export bench env for a multi-node topology manifest.
# Usage: export-env-topology.sh <topology-name>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"

TOPO="${1:?topology name}"
MANIFEST_PATH="$(manifest_path "$TOPO")"

python3 - <<PY
import json
m = json.load(open("$MANIFEST_PATH"))
topo = m.get("bench_topology", m["topology"])
if topo.startswith("scylla"):
    points = ",".join(f"{i['private_ip']}:9042" for i in m["instances"] if i["role"] == "scylla")
    print(f"export CONTINUUM_BENCH_SCYLLA_TOPOLOGY={topo}")
    print(f"export CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS={points}")
    print("export CONTINUUM_BENCH_SCYLLA_KEYSPACE=continuum")
elif topo.startswith("tikv"):
    pd = next(i["private_ip"] for i in m["instances"] if i["role"] == "pd")
    print(f"export CONTINUUM_BENCH_TIKV_TOPOLOGY={topo}")
    print(f"export CONTINUUM_BENCH_TIKV_PD_ENDPOINT=http://{pd}:2379")
else:
    raise SystemExit(f"unknown bench_topology: {topo}")
print("export CONTINUUM_BENCH_REPORTS_DIR=\${HOME}/continuum-bench/reports")
PY
