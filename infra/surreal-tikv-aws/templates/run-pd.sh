#!/usr/bin/env bash
# Start PD container on host network with private IP advertise URLs.
set -euo pipefail

PD_IP="${PD_IP:?}"
DATA_DIR="/var/lib/continuum/pd0"
sudo mkdir -p "$DATA_DIR"

sudo docker rm -f continuum-pd0 2>/dev/null || true

sudo docker run -d --name continuum-pd0 --restart unless-stopped --network host \
  -v "${DATA_DIR}:/data/pd0" \
  "${PD_IMAGE:?}" \
  --name=pd0 \
  --client-urls=http://0.0.0.0:2379 \
  --peer-urls=http://0.0.0.0:2380 \
  --advertise-client-urls="http://${PD_IP}:2379" \
  --advertise-peer-urls="http://${PD_IP}:2380" \
  --initial-cluster="pd0=http://${PD_IP}:2380" \
  --data-dir=/data/pd0

echo "PD started on ${PD_IP}:2379"
