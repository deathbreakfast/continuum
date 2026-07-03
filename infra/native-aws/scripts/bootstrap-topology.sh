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

# Base docker + swap on all hosts
python3 - <<PY
import json
for i in json.loads('''$MANIFEST''')["instances"]:
    print(i["public_ip"])
PY
| sort -u | while read -r host; do
  [[ -z "$host" ]] && continue
  ssh_wait_ready "$host"
  rsync_repo "$host" "$REPO_ROOT"
  ssh_cmd "$host" "bash -s" <<'EOF'
set -euo pipefail
if ! command -v docker >/dev/null 2>&1; then
  sudo dnf install -y docker docker-compose-plugin
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
done

# Start storage stacks
python3 - <<PY | while IFS=$'\t' read -r role idx pub priv; do
import json
m = json.loads('''$MANIFEST''')
for i in m["instances"]:
    print(i["role"], i["index"], i["public_ip"], i["private_ip"], sep="\t")
PY
  case "$role" in
    pd)
      ssh_cmd "$pub" "export PD_IP=$priv PD_IMAGE=$CONTINUUM_NATIVE_PD_IMAGE; bash ~/continuum/infra/surreal-tikv-aws/templates/run-pd.sh"
      ;;
    tikv)
      PD_IP="$(python3 -c "import json; m=json.loads('''$MANIFEST'''); print(next(i['private_ip'] for i in m['instances'] if i['role']=='pd'))")"
      ssh_cmd "$pub" "export TIKV_IP=$priv PD_IP=$PD_IP TIKV_INDEX=$idx TIKV_IMAGE=$CONTINUUM_NATIVE_TIKV_IMAGE; bash ~/continuum/infra/surreal-tikv-aws/templates/run-tikv.sh"
      ;;
    scylla)
      SEED="$(python3 -c "import json; m=json.loads('''$MANIFEST'''); print(next(i['private_ip'] for i in m['instances'] if i['role']=='scylla' and i['index']==0))")"
      ssh_cmd "$pub" "export SCYLLA_IP=$priv SEED_IP=$SEED SCYLLA_INDEX=$idx SCYLLA_IMAGE=$CONTINUUM_NATIVE_SCYLLA_IMAGE; bash ~/continuum/infra/native-aws/templates/run-scylla.sh"
      ;;
    bench) ;;
  esac
done

echo "bootstrap-topology complete: $TOPO"
