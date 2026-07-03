#!/usr/bin/env bash
# Upload AL2023 continuum-bench binary to S3.
# Usage: artifact-upload.sh [path-to-binary]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/artifact.sh"

BINARY="${1:-$(artifact_local_path "$REPO_ROOT")}"
BUCKET="${CONTINUUM_NATIVE_ARTIFACT_BUCKET:?set CONTINUUM_NATIVE_ARTIFACT_BUCKET}"

[[ -f "$BINARY" ]] || { echo "binary not found: $BINARY" >&2; exit 1; }

URI="$(artifact_s3_uri "$BUCKET")"
aws s3 cp "$BINARY" "$URI" --region "${CONTINUUM_NATIVE_AWS_REGION}"
echo "Uploaded $URI"
