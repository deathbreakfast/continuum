#!/usr/bin/env bash
# Collect Scylla storage-node metrics (Track U).
# Usage: collect-scylla-storage-metrics.sh <topology> [phase] [output-json]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"
# shellcheck disable=SC1091
source "$ROOT/lib/ssh.sh"
# shellcheck disable=SC1091
source "$ROOT/lib/topology.sh"

TOPO="${1:?topology}"
PHASE="${2:-end}"
OUT="${3:-$ROOT/state/${TOPO}-storage-${PHASE}.json}"

mkdir -p "$(dirname "$OUT")"
NODES="[]"

while IFS= read -r pub; do
  [[ -z "$pub" ]] && continue
  cname="$(ssh_cmd "$pub" "sudo docker ps --format '{{.Names}}' | grep -E '^continuum-scylla' | head -1" | tr -d '\r' || true)"
  cname="${cname:-continuum-scylla0}"
  nodetool_status="$(ssh_cmd "$pub" "sudo docker exec $cname nodetool status 2>/dev/null" || echo "nodetool failed")"
  tablestats="$(ssh_cmd "$pub" "sudo docker exec $cname nodetool tablestats continuum 2>/dev/null | tail -20" || echo "tablestats failed")"
  docker_stats="$(ssh_cmd "$pub" "sudo docker stats --no-stream $cname 2>/dev/null" || echo "docker stats failed")"
  loadavg="$(ssh_cmd "$pub" "cat /proc/loadavg 2>/dev/null" || echo "")"
  mpstat="$(ssh_cmd "$pub" "command -v mpstat >/dev/null && mpstat 1 3 2>/dev/null | tail -5 || echo mpstat-unavailable" || echo "")"
  NODES="$(python3 - "$NODES" "$pub" "$cname" "$nodetool_status" "$tablestats" "$docker_stats" "$loadavg" "$mpstat" <<'PY'
import json, sys
nodes = json.loads(sys.argv[1])
nodes.append({
  "host": sys.argv[2],
  "container": sys.argv[3],
  "nodetool_status": sys.argv[4],
  "tablestats_tail": sys.argv[5],
  "docker_stats": sys.argv[6],
  "loadavg": sys.argv[7],
  "mpstat_tail": sys.argv[8],
})
print(json.dumps(nodes))
PY
)"
done < <(topology_scylla_public_ips "$TOPO")

python3 - "$TOPO" "$PHASE" "$NODES" "$OUT" <<'PY'
import json, sys, datetime
topo, phase, nodes_json, out = sys.argv[1:5]
doc = {
  "topology": topo,
  "phase": phase,
  "collected_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
  "nodes": json.loads(nodes_json),
}
with open(out, "w") as f:
  json.dump(doc, f, indent=2)
print(f"Wrote {out}")
PY
