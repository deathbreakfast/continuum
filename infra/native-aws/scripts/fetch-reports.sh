#!/usr/bin/env bash
# SCP reports from fleet back to workspace.
# Usage: fetch-reports.sh [manifest-name] [--upload-s3]
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
# shellcheck disable=SC1091
source "$ROOT/lib/topology.sh"

MANIFEST_NAME="${1:-native-colocated}"
UPLOAD_S3=false
if [[ "${2:-}" == --upload-s3 || "${1:-}" == --upload-s3 ]]; then
  UPLOAD_S3=true
  [[ "$1" == --upload-s3 ]] && MANIFEST_NAME="native-colocated"
fi

DEST="${CONTINUUM_BENCH_REPORTS_DIR:-$REPO_ROOT/profiling/continuum-bench/reports}"
mkdir -p "$DEST"

BEFORE_SUM="$(find "$DEST" -maxdepth 1 -name '*.json' -printf '%f:%s\n' 2>/dev/null | sort | sha256sum | cut -d' ' -f1)"

MANIFEST="$(manifest_read "$MANIFEST_NAME")"
while IFS= read -r host; do
  [[ -z "$host" ]] && continue
  echo "Fetching reports from $host"
  scp_from "$host" "~/continuum-bench/reports/*.json" "$DEST/" 2>/dev/null || true
done < <(echo "$MANIFEST" | python3 -c "
import json, sys
for i in json.load(sys.stdin)['instances']:
    role = i['role']
    if role in ('bench', 'scylla', 'tikv'):
        print(i['public_ip'])
" | sort -u)

AFTER_SUM="$(find "$DEST" -maxdepth 1 -name '*.json' -printf '%f:%s\n' 2>/dev/null | sort | sha256sum | cut -d' ' -f1)"
AFTER_COUNT="$(find "$DEST" -maxdepth 1 -name '*.json' 2>/dev/null | wc -l)"
if [[ "$BEFORE_SUM" == "$AFTER_SUM" ]]; then
  echo "fetch-reports: no reports changed for $MANIFEST_NAME" >&2
  exit 1
fi

echo "Reports in $DEST ($AFTER_COUNT total, content updated)"

if $UPLOAD_S3 && [[ -n "${CONTINUUM_NATIVE_ARTIFACT_BUCKET:-}" ]]; then
  DATE_PREFIX="$(date +%Y-%m-%d)"
  PREFIX="reports/scylla-diagnosis/${DATE_PREFIX}/${MANIFEST_NAME}/"
  aws s3 sync "$DEST/" "s3://${CONTINUUM_NATIVE_ARTIFACT_BUCKET}/${PREFIX}" \
    --exclude '*' --include '*.json' --region "$CONTINUUM_NATIVE_AWS_REGION"
  echo "Uploaded reports to s3://${CONTINUUM_NATIVE_ARTIFACT_BUCKET}/${PREFIX}"
fi
