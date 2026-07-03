#!/usr/bin/env bash
# Bootstrap all instances in a topology manifest.
# Usage: bootstrap-topology.sh <topology-name>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"
# shellcheck disable=SC1091
source "$ROOT/lib/ssh.sh"

TOPO="${1:?topology name required}"
MANIFEST="$(manifest_read "$TOPO")"

python3 - <<PY
import json, sys
m = json.loads('''$MANIFEST''')
for i in m["instances"]:
    print(f"{i['role']}\t{i['index']}\t{i['public_ip']}\t{i['private_ip']}")
PY

# Base docker + swap on all hosts (binary + campaign scripts deployed separately)
while read -r host; do
  [[ -z "$host" ]] && continue
  ssh_wait_ready "$host"
  ssh_cmd "$host" "bash -s" <<'EOF'
set -euo pipefail
if ! command -v docker >/dev/null 2>&1; then
  sudo dnf install -y docker
  sudo systemctl enable --now docker
  sudo usermod -aG docker ec2-user
fi
if ! swapon --show | grep -q swapfile; then
  sudo fallocate -l 4G /swapfile || sudo dd if=/dev/zero of=/swapfile bs=1M count=4096
  sudo chmod 600 /swapfile
  sudo mkswap /swapfile
  sudo swapon /swapfile
fi
mkdir -p ~/continuum-bench/reports
EOF
  ssh_cmd "$host" "sudo docker pull ${CONTINUUM_NATIVE_SCYLLA_IMAGE}; sudo docker pull ${CONTINUUM_NATIVE_PD_IMAGE}; sudo docker pull ${CONTINUUM_NATIVE_TIKV_IMAGE}" || true
done < <(python3 - <<PY | sort -u
import json
for i in json.loads('''$MANIFEST''')["instances"]:
    print(i["public_ip"])
PY
)

start_storage_node() {
  local role="$1" idx="$2" pub="$3" priv="$4"
  case "$role" in
    pd)
      ssh_cmd "$pub" "command -v docker >/dev/null || (sudo dnf install -y docker && sudo systemctl enable --now docker)"
      scp_to "$pub" "$REPO_ROOT/infra/surreal-tikv-aws/templates/run-pd.sh" "/tmp/run-pd.sh"
      ssh_cmd "$pub" "export PD_IP=$priv PD_IMAGE=$CONTINUUM_NATIVE_PD_IMAGE; bash /tmp/run-pd.sh"
      ;;
    tikv)
      local pd_ip
      pd_ip="$(python3 -c "import json; m=json.loads('''$MANIFEST'''); print(next(i['private_ip'] for i in m['instances'] if i['role']=='pd'))")"
      ssh_cmd "$pub" "command -v docker >/dev/null || (sudo dnf install -y docker && sudo systemctl enable --now docker)"
      scp_to "$pub" "$REPO_ROOT/infra/surreal-tikv-aws/templates/run-tikv.sh" "/tmp/run-tikv.sh"
      ssh_cmd "$pub" "export TIKV_IP=$priv PD_IP=$pd_ip TIKV_INDEX=$idx TIKV_IMAGE=$CONTINUUM_NATIVE_TIKV_IMAGE; bash /tmp/run-tikv.sh"
      ssh_cmd "$pub" "sudo docker ps --filter name=continuum-tikv${idx} --format '{{.Names}} {{.Status}}'"
      ;;
    scylla)
      local seed
      seed="$(python3 -c "import json; m=json.loads('''$MANIFEST'''); print(next(i['private_ip'] for i in m['instances'] if i['role']=='scylla' and i['index']==0))")"
      ssh_cmd "$pub" "command -v docker >/dev/null || (sudo dnf install -y docker && sudo systemctl enable --now docker)"
      scp_to "$pub" "$ROOT/templates/run-scylla.sh" "/tmp/run-scylla.sh"
      ssh_cmd "$pub" "export SCYLLA_IP=$priv SEED_IP=$seed SCYLLA_INDEX=$idx SCYLLA_IMAGE=$CONTINUUM_NATIVE_SCYLLA_IMAGE; bash /tmp/run-scylla.sh"
      ;;
    bench) ;;
    *)
      echo "unknown role: $role" >&2
      return 1
      ;;
  esac
}

# Start storage stacks (PD before TiKV; skip bench)
while IFS=$'\t' read -r role idx pub priv; do
  echo "Starting $role-$idx on $pub ($priv)..."
  start_storage_node "$role" "$idx" "$pub" "$priv"
done < <(python3 - <<PY
import json
m = json.loads('''$MANIFEST''')
prio = {"pd": 0, "scylla": 1, "tikv": 2, "bench": 9}
for i in sorted(m["instances"], key=lambda x: (prio.get(x["role"], 5), x["index"])):
    if i["role"] == "bench":
        continue
    print(i["role"], i["index"], i["public_ip"], i["private_ip"], sep="\t")
PY
)

sleep 15

# Wait for storage stacks to become ready
BENCH_TOPO="$(python3 -c "import json,sys; print(json.load(sys.stdin)['bench_topology'])" <<< "$MANIFEST")"
if [[ "$BENCH_TOPO" == scylla* ]]; then
  SEED_PUB="$(python3 -c "import json,sys; m=json.load(sys.stdin); print(next(i['public_ip'] for i in m['instances'] if i['role']=='scylla' and i['index']==0))" <<< "$MANIFEST")"
  echo "Waiting for Scylla cluster on $SEED_PUB..."
  for _ in $(seq 1 60); do
    UN="$(ssh_cmd "$SEED_PUB" "sudo docker exec continuum-scylla0 nodetool status 2>/dev/null | grep -c '^UN' || true")"
    if [[ "${UN:-0}" -ge 2 ]]; then
      break
    fi
    sleep 5
  done
elif [[ "$BENCH_TOPO" == tikv* ]]; then
  PD_PUB="$(python3 -c "import json,sys; m=json.load(sys.stdin); print(next(i['public_ip'] for i in m['instances'] if i['role']=='pd'))" <<< "$MANIFEST")"
  EXPECT="$(python3 -c "import json,sys; print(sum(1 for i in json.load(sys.stdin)['instances'] if i['role']=='tikv'))" <<< "$MANIFEST")"
  echo "Waiting for TiKV stores on $PD_PUB (expect $EXPECT)..."
  COUNT=0
  for _ in $(seq 1 90); do
    COUNT="$(ssh_cmd "$PD_PUB" "curl -sf http://127.0.0.1:2379/pd/api/v1/stores 2>/dev/null | python3 -c \"import json,sys; d=json.load(sys.stdin); print(d.get('count', len(d.get('stores',[]))))\" 2>/dev/null" || echo 0)"
    if [[ "$COUNT" == "$EXPECT" ]]; then
      echo "TiKV stores ready: $COUNT"
      break
    fi
    sleep 5
  done
  if [[ "$COUNT" != "$EXPECT" ]]; then
    echo "bootstrap FAIL: expected $EXPECT TiKV stores, got ${COUNT:-0}" >&2
    ssh_cmd "$PD_PUB" "curl -s http://127.0.0.1:2379/pd/api/v1/stores || true"
    exit 1
  fi
fi

echo "bootstrap-topology complete: $TOPO"
