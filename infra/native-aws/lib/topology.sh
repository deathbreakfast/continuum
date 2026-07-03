#!/usr/bin/env bash
# Topology manifest helpers (bench host, scylla nodes).
set -euo pipefail

topology_bench_host() {
  local topo="$1"
  local manifest
  manifest="$(manifest_read "$topo")"
  echo "$manifest" | python3 -c "
import json, sys
m = json.load(sys.stdin)
for i in m['instances']:
    if i['role'] == 'bench':
        print(i['public_ip'])
        break
else:
    for i in m['instances']:
        if i['role'] == 'scylla':
            print(i['public_ip'])
            break
"
}

topology_scylla_nodes_csv() {
  local topo="$1"
  local manifest
  manifest="$(manifest_read "$topo")"
  echo "$manifest" | python3 -c "
import json, sys
m = json.load(sys.stdin)
ips = [i['private_ip'] or i['public_ip'] for i in sorted(m['instances'], key=lambda x: x.get('index', 0)) if i['role'] == 'scylla']
print(','.join(ips))
"
}

topology_scylla_public_ips() {
  local topo="$1"
  local manifest
  manifest="$(manifest_read "$topo")"
  echo "$manifest" | python3 -c "
import json, sys
m = json.load(sys.stdin)
for i in sorted(m['instances'], key=lambda x: x.get('index', 0)):
    if i['role'] == 'scylla':
        print(i['public_ip'])
"
}
