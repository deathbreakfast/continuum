#!/usr/bin/env bash
# Read/write state/*.json manifests.
set -euo pipefail

manifest_path() {
  echo "$CONTINUUM_TIKV_AWS_ROOT/state/${1}.json"
}

manifest_write() {
  local topology="$1"
  local json="$2"
  local path
  path="$(manifest_path "$topology")"
  echo "$json" > "$path"
}

manifest_read() {
  local topology="$1"
  local path
  path="$(manifest_path "$topology")"
  if [[ ! -f "$path" ]]; then
    echo "state not found for topology: $topology (run up.sh first)" >&2
    return 1
  fi
  cat "$path"
}

manifest_exists() {
  [[ -f "$(manifest_path "$1")" ]]
}

manifest_get() {
  local topology="$1"
  local jq_expr="$2"
  manifest_read "$topology" | python3 -c "
import json, sys
data = json.load(sys.stdin)
expr = sys.argv[1]
# minimal jq-like: dot path only
parts = expr.lstrip('.').split('.')
v = data
for p in parts:
    if p.isdigit():
        v = v[int(p)]
    else:
        v = v[p]
if isinstance(v, (dict, list)):
    print(json.dumps(v))
else:
    print(v)
" "$jq_expr"
}

manifest_build_json() {
  python3 - "$@" <<'PY'
import json, sys, os
from datetime import datetime, timezone

# args: topology name arch bench_topology surreal_instances component hw...
topology = sys.argv[1]
arch = sys.argv[2]
bench_topology = sys.argv[3]
surreal_instances = int(sys.argv[4])
component_tikv_hw = sys.argv[5]
component_surreal_hw = sys.argv[6]
instances_json = sys.argv[7]

data = {
    "topology": topology,
    "arch": arch,
    "bench_topology": bench_topology,
    "surreal_instances": surreal_instances,
    "component_hardware": {
        "tikv": component_tikv_hw,
        "surreal": component_surreal_hw,
    },
    "created_at": datetime.now(timezone.utc).isoformat(),
    "instances": json.loads(instances_json),
}
print(json.dumps(data, indent=2))
PY
}
