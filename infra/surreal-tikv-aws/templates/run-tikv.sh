#!/usr/bin/env bash
# Start TiKV container on host network.
set -euo pipefail

TIKV_IP="${TIKV_IP:?}"
PD_IP="${PD_IP:?}"
TIKV_INDEX="${TIKV_INDEX:-0}"
DATA_DIR="/var/lib/continuum/tikv${TIKV_INDEX}"
CONTAINER="continuum-tikv${TIKV_INDEX}"

sudo mkdir -p "$DATA_DIR"
sudo docker rm -f "$CONTAINER" 2>/dev/null || true

sudo docker run -d --name "$CONTAINER" --restart unless-stopped --network host \
  --ulimit nofile=262144:262144 \
  -v "${DATA_DIR}:/data/tikv${TIKV_INDEX}" \
  "${TIKV_IMAGE:?}" \
  --addr=0.0.0.0:20160 \
  --advertise-addr="${TIKV_IP}:20160" \
  --data-dir="/data/tikv${TIKV_INDEX}" \
  --pd="${PD_IP}:2379"

echo "TiKV ${TIKV_INDEX} started on ${TIKV_IP}:20160 -> PD ${PD_IP}:2379"
