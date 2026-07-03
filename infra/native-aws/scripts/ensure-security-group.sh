#!/usr/bin/env bash
# Ensure continuum-bench SG allows intra-group storage traffic (idempotent).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"

SG_ID="$(aws ec2 describe-security-groups --region "$CONTINUUM_NATIVE_AWS_REGION" \
  --filters "Name=group-name,Values=$CONTINUUM_NATIVE_AWS_SG_NAME" \
  --query 'SecurityGroups[0].GroupId' --output text 2>/dev/null || true)"

[[ -n "$SG_ID" && "$SG_ID" != "None" ]] || {
  echo "security group $CONTINUUM_NATIVE_AWS_SG_NAME not found; run provision-colocated.sh first" >&2
  exit 1
}

authorize_port() {
  local port="$1"
  aws ec2 authorize-security-group-ingress \
    --region "$CONTINUUM_NATIVE_AWS_REGION" \
    --group-id "$SG_ID" \
    --protocol tcp --port "$port" --source-group "$SG_ID" 2>/dev/null || true
}

for port in 9042 7000 7001 2379 2380 20160; do
  authorize_port "$port"
done

echo "SG $CONTINUUM_NATIVE_AWS_SG_NAME ($SG_ID) storage ports ensured"
