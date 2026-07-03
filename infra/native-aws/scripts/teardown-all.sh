#!/usr/bin/env bash
# Terminate ALL running EC2 instances tagged Project=continuum-bench and clear manifests.
# Usage: teardown-all.sh [--dry-run]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"

DRY_RUN=false
[[ "${1:-}" == --dry-run ]] && DRY_RUN=true

TAG="${CONTINUUM_NATIVE_AWS_PROJECT_TAG}"
REGION="${CONTINUUM_NATIVE_AWS_REGION}"

IDS="$(aws ec2 describe-instances --region "$REGION" \
  --filters "Name=tag:Project,Values=$TAG" "Name=instance-state-name,Values=running,pending,stopping,stopped" \
  --query 'Reservations[].Instances[].InstanceId' --output text | tr '\t' ' ')"

if [[ -n "${IDS// /}" ]]; then
  if $DRY_RUN; then
    echo "Would terminate: $IDS"
  else
    aws ec2 terminate-instances --region "$REGION" --instance-ids $IDS
    echo "Terminated: $IDS"
  fi
else
  echo "No continuum-bench instances found."
fi

if [[ -d "$ROOT/manifests" ]]; then
  count="$(find "$ROOT/manifests" -maxdepth 1 -name '*.json' 2>/dev/null | wc -l)"
  if $DRY_RUN; then
    echo "Would remove $count manifest(s) under $ROOT/manifests/"
  else
    rm -f "$ROOT/manifests"/*.json 2>/dev/null || true
    echo "Cleared manifests ($count removed)."
  fi
fi

REMAINING="$(aws ec2 describe-instances --region "$REGION" \
  --filters "Name=tag:Project,Values=$TAG" "Name=instance-state-name,Values=running,pending" \
  --query 'length(Reservations[].Instances[])' --output text 2>/dev/null || echo 0)"
echo "Remaining active instances (Project=$TAG): ${REMAINING:-0}"
