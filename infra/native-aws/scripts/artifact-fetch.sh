#!/usr/bin/env bash
# Fetch AL2023 continuum-bench binary from S3 or build locally.
# Usage: artifact-fetch.sh [--build-if-missing]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/artifact.sh"

BUILD_IF_MISSING=0
for arg in "$@"; do
  [[ "$arg" == "--build-if-missing" ]] && BUILD_IF_MISSING=1
done

OUT="$(artifact_local_path "$REPO_ROOT")"
mkdir -p "$(dirname "$OUT")"

if [[ -z "${CONTINUUM_NATIVE_ARTIFACT_BUCKET:-}" ]]; then
  echo "CONTINUUM_NATIVE_ARTIFACT_BUCKET not set" >&2
  if [[ "$BUILD_IF_MISSING" -eq 1 ]]; then
    bash "$ROOT/scripts/build-al2023.sh" "$OUT"
    exit 0
  fi
  exit 1
fi

URI="$(artifact_s3_uri "$CONTINUUM_NATIVE_ARTIFACT_BUCKET")"
if aws s3 cp "$URI" "$OUT" --region "${CONTINUUM_NATIVE_AWS_REGION}" 2>/dev/null; then
  chmod +x "$OUT"
  echo "Fetched $URI -> $OUT"
  exit 0
fi

if [[ "$BUILD_IF_MISSING" -eq 1 ]]; then
  echo "S3 artifact missing; building locally..."
  bash "$ROOT/scripts/build-al2023.sh" "$OUT"
  bash "$ROOT/scripts/artifact-upload.sh" "$OUT" || true
  exit 0
fi

echo "Artifact not found: $URI (use --build-if-missing)" >&2
exit 1
