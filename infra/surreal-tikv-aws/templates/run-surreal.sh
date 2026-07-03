#!/usr/bin/env bash
# Start SurrealDB container pointing at remote PD.
set -euo pipefail

PD_IP="${PD_IP:?}"
SURREAL_INDEX="${SURREAL_INDEX:-0}"
CONTAINER="continuum-surreal${SURREAL_INDEX}"

sudo docker rm -f "$CONTAINER" 2>/dev/null || true

sudo docker run -d --name "$CONTAINER" --restart unless-stopped --network host \
  "${SURREAL_IMAGE:?}" \
  start --log info --user root --pass root --bind 0.0.0.0:8000 "tikv://${PD_IP}:2379"

echo "Surreal ${SURREAL_INDEX} started on :8000 -> PD ${PD_IP}:2379"
