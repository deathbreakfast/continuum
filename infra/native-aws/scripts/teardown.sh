#!/usr/bin/env bash
# Terminate EC2 instances from a manifest.
# Usage: teardown.sh [manifest-name]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"

MANIFEST_NAME="${1:-native-colocated}"
MANIFEST="$(manifest_read "$MANIFEST_NAME")"

IDS="$(echo "$MANIFEST" | python3 -c "
import json, sys
m = json.load(sys.stdin)
print(' '.join(i['instance_id'] for i in m['instances']))
")"

if [[ -n "$IDS" ]]; then
  aws ec2 terminate-instances --region "$CONTINUUM_NATIVE_AWS_REGION" --instance-ids $IDS
  echo "Terminated: $IDS"
fi

rm -f "$(manifest_path "$MANIFEST_NAME")"
