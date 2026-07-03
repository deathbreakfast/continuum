#!/usr/bin/env bash
# Pre-flight health checks for a multi-node topology manifest.
# Usage: preflight-topology.sh <topology-name>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"
# shellcheck disable=SC1091
source "$ROOT/lib/ssh.sh"

TOPO="${1:?topology name}"
MANIFEST_PATH="$(manifest_path "$TOPO")"
BENCH_TOPO="$(python3 -c "import json; print(json.load(open('$MANIFEST_PATH'))['bench_topology'])")"

check_scylla() {
  local seed_pub
  seed_pub="$(python3 -c "
import json
m = json.load(open('$MANIFEST_PATH'))
for i in m['instances']:
    if i['role'] == 'scylla' and i['index'] == 0:
        print(i['public_ip']); break
")"
  echo "Scylla seed: $seed_pub"
  ssh_cmd "$seed_pub" "sudo docker exec continuum-scylla0 nodetool status 2>/dev/null || nodetool status 2>/dev/null || true"
}

check_tikv() {
  local pd_pub expected
  pd_pub="$(python3 -c "
import json
m = json.load(open('$MANIFEST_PATH'))
for i in m['instances']:
    if i['role'] == 'pd':
        print(i['public_ip']); break
")"
  expected="$(python3 -c "
import json
print(sum(1 for i in json.load(open('$MANIFEST_PATH'))['instances'] if i['role'] == 'tikv'))
")"
  echo "TiKV PD: $pd_pub (expect $expected stores)"
  ssh_cmd "$pd_pub" "curl -sf http://127.0.0.1:2379/pd/api/v1/health; echo"
  local count
  count="$(ssh_cmd "$pd_pub" "curl -sf http://127.0.0.1:2379/pd/api/v1/stores | python3 -c \"import json,sys; d=json.load(sys.stdin); print(d.get('count', len(d.get('stores',[]))))\" 2>/dev/null" || echo 0)"
  echo "stores $count"
  if [[ "$count" != "$expected" ]]; then
    echo "preflight FAIL: expected $expected TiKV stores, got $count" >&2
    exit 1
  fi
}

bench_pub="$(python3 -c "import json; print(next(i['public_ip'] for i in json.load(open('$MANIFEST_PATH'))['instances'] if i['role']=='bench'))")"
ENV_EXPORTS="$(bash "$ROOT/scripts/export-env-topology.sh" "$TOPO")"

echo "Bench host: $bench_pub"
ssh_cmd "$bench_pub" "bash -lc 'set -euo pipefail; $ENV_EXPORTS; echo env_ok; test -x ~/continuum-bench/continuum-bench || echo bench_binary_missing'"

if [[ "$BENCH_TOPO" == scylla* ]]; then
  check_scylla
elif [[ "$BENCH_TOPO" == tikv* ]]; then
  check_tikv
else
  echo "unknown bench_topology: $BENCH_TOPO" >&2
  exit 1
fi

echo "preflight OK: $TOPO"
