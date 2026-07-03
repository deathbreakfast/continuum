#!/usr/bin/env bash
# Bootstrap Docker + swap + storage stack on a colocated host.
# Usage: bootstrap.sh <manifest-name> <role>
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

MANIFEST_NAME="${1:-native-colocated}"
ROLE="${2:?role required: scylla|tikv|bench}"

MANIFEST="$(manifest_read "$MANIFEST_NAME")"
HOST="$(echo "$MANIFEST" | python3 -c "
import json, sys
m = json.load(sys.stdin)
for i in m['instances']:
    if i['role'] == '$ROLE':
        print(i['public_ip'])
        break
else:
    raise SystemExit('role not in manifest')
")"

ssh_wait_ready "$HOST"
rsync_repo "$HOST" "$REPO_ROOT"

ssh_cmd "$HOST" "bash -s" <<EOF
set -euo pipefail
if ! command -v docker >/dev/null 2>&1; then
  sudo dnf install -y docker rsync
  sudo systemctl enable --now docker
  sudo usermod -aG docker ec2-user
fi
if ! swapon --show | grep -q swapfile; then
  sudo fallocate -l 4G /swapfile || sudo dd if=/dev/zero of=/swapfile bs=1M count=4096
  sudo chmod 600 /swapfile
  sudo mkswap /swapfile
  sudo swapon /swapfile
  grep -q swapfile /etc/fstab || echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
fi
sudo docker pull ${CONTINUUM_NATIVE_SCYLLA_IMAGE}
sudo docker pull ${CONTINUUM_NATIVE_PD_IMAGE}
sudo docker pull ${CONTINUUM_NATIVE_TIKV_IMAGE}
mkdir -p ~/continuum-bench/reports
EOF

case "$ROLE" in
  scylla)
    ssh_cmd "$HOST" "sudo docker rm -f continuum-scylla0 2>/dev/null || true; sudo docker run -d --name continuum-scylla0 -p 9042:9042 -v scylla0-data:/var/lib/scylla ${CONTINUUM_NATIVE_SCYLLA_IMAGE} --smp 1 --memory 750M --overprovisioned 1 --api-address 0.0.0.0"
    ;;
  tikv)
    ssh_cmd "$HOST" "sudo docker rm -f continuum-pd0 continuum-tikv0 2>/dev/null || true; \
      sudo docker run -d --name continuum-pd0 --network host -v pd0-data:/data/pd0 ${CONTINUUM_NATIVE_PD_IMAGE} \
        --name=pd0 --client-urls=http://127.0.0.1:2379 --peer-urls=http://127.0.0.1:2380 \
        --advertise-client-urls=http://127.0.0.1:2379 --advertise-peer-urls=http://127.0.0.1:2380 \
        --initial-cluster=pd0=http://127.0.0.1:2380 --data-dir=/data/pd0; \
      sleep 5; \
      sudo docker run -d --name continuum-tikv0 --network host --ulimit nofile=262144:262144 -v tikv0-data:/data/tikv0 ${CONTINUUM_NATIVE_TIKV_IMAGE} \
        --addr=127.0.0.1:20160 --advertise-addr=127.0.0.1:20160 --data-dir=/data/tikv0 --pd=127.0.0.1:2379; \
      for i in \$(seq 1 60); do curl -sf http://127.0.0.1:2379/pd/api/v1/stores 2>/dev/null | grep -q '\"state_name\":\"Up\"' && exit 0; sleep 2; done"
    ;;
  bench)
    echo "bench role: no storage stack (remote bench only)"
    ;;
  *)
    echo "unknown role: $ROLE" >&2
    exit 1
    ;;
esac

echo "bootstrap complete role=$ROLE host=$HOST"
