#!/usr/bin/env bash
# SCP pre-built continuum-bench binary to bench instance.
# Usage: deploy-bench.sh <topology-name> <path-to-binary>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_TIKV_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"
# shellcheck disable=SC1091
source "$ROOT/lib/ssh.sh"

[[ $# -ge 2 ]] || { echo "usage: $0 <topology-name> <path-to-binary>" >&2; exit 1; }
TOPOLOGY="$1"
BINARY="$2"

if [[ ! -f "$BINARY" ]]; then
  echo "binary not found: $BINARY" >&2
  exit 1
fi

if [[ ! -f "$CONTINUUM_TIKV_AWS_KEY_PATH" ]]; then
  echo "SSH key not found: $CONTINUUM_TIKV_AWS_KEY_PATH" >&2
  exit 1
fi

MANIFEST="$(manifest_read "$TOPOLOGY")"
BENCH_PUBLIC="$(python3 -c "
import json, sys
m = json.load(sys.stdin)
print(next(i['public_ip'] for i in m['instances'] if i['role']=='bench'))
" <<< "$MANIFEST")"

ssh_wait_ready "$BENCH_PUBLIC"
ssh_cmd "$BENCH_PUBLIC" "mkdir -p ~/continuum-bench/reports"
scp_to "$BENCH_PUBLIC" "$BINARY" "~/continuum-bench/continuum-bench"
ssh_cmd "$BENCH_PUBLIC" "chmod +x ~/continuum-bench/continuum-bench"

echo "Deployed to ec2-user@${BENCH_PUBLIC}:~/continuum-bench/continuum-bench"
echo ""
echo "Smoke test:"
echo "  ssh -i $CONTINUUM_TIKV_AWS_KEY_PATH ec2-user@${BENCH_PUBLIC} '~/continuum-bench/continuum-bench experiments'"
echo ""
echo "If GLIBC mismatch, rebuild inside amazonlinux:2023 Docker — see README.md"
