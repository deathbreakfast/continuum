#!/usr/bin/env bash
# Start Scylla container on host (one node per EC2).
set -euo pipefail

SCYLLA_IP="${SCYLLA_IP:?}"
SEED_IP="${SEED_IP:?}"
SCYLLA_INDEX="${SCYLLA_INDEX:-0}"
DATA_DIR="/var/lib/continuum/scylla${SCYLLA_INDEX}"
CONTAINER="continuum-scylla${SCYLLA_INDEX}"
IMAGE="${SCYLLA_IMAGE:?}"

sudo mkdir -p "$DATA_DIR"
sudo docker rm -f "$CONTAINER" 2>/dev/null || true

EXTRA=()
if [[ "$SCYLLA_INDEX" != "0" ]]; then
  EXTRA=(--seeds="$SEED_IP")
fi

sudo docker run -d --name "$CONTAINER" --restart unless-stopped \
  -p 9042:9042 \
  -v "${DATA_DIR}:/var/lib/scylla" \
  "$IMAGE" \
  "${EXTRA[@]}" \
  --smp 1 --memory 750M --overprovisioned 1 --api-address 0.0.0.0

echo "Scylla ${SCYLLA_INDEX} on ${SCYLLA_IP}:9042 seeds=${SEED_IP}"
