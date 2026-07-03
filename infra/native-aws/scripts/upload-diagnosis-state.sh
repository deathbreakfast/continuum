#!/usr/bin/env bash
# Upload scylla diagnosis state + metrics to S3.
# Usage: upload-diagnosis-state.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"

BUCKET="${CONTINUUM_NATIVE_ARTIFACT_BUCKET:-}"
[[ -n "$BUCKET" ]] || { echo "CONTINUUM_NATIVE_ARTIFACT_BUCKET not set; skip upload" >&2; exit 0; }

DATE_PREFIX="$(date +%Y-%m-%d)"
PREFIX="diagnosis/${DATE_PREFIX}/"
aws s3 sync "$ROOT/state/" "s3://${BUCKET}/${PREFIX}state/" \
  --exclude '*' --include '*.json' --region "$CONTINUUM_NATIVE_AWS_REGION"
echo "Uploaded state to s3://${BUCKET}/${PREFIX}state/"
