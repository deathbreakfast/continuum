#!/usr/bin/env bash
# Provision multi-EC2 surreal-tikv topology, deploy binary, print bench commands.
#
# Usage: run-tikv-aws-topology.sh <topology> <bench-hardware> <path-to-binary> [--arch x86|arm]
# Example: run-tikv-aws-topology.sh budget-ha-3 aws-t3-medium /tmp/continuum-bench-x86
#
# Does NOT run the matrix automatically — SSH to bench host and run matrix after eval export-env.
set -euo pipefail

if [[ $# -lt 3 ]]; then
  echo "usage: $0 <topology> <bench-hardware> <path-to-binary> [--arch x86|arm]" >&2
  exit 1
fi

TOPOLOGY="$1"
HARDWARE="$2"
BINARY="$3"
shift 3

ARCH=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --arch) ARCH="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 1 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
AWS="$ROOT/infra/surreal-tikv-aws"

UP_ARGS=("$TOPOLOGY")
[[ -n "$ARCH" ]] && UP_ARGS+=("--arch" "$ARCH")

"$AWS/scripts/up.sh" "${UP_ARGS[@]}"
"$AWS/scripts/deploy-bench.sh" "$TOPOLOGY" "$BINARY"

BENCH_IP="$(python3 -c "
import json
with open('$AWS/state/${TOPOLOGY}.json') as f:
    m = json.load(f)
print(next(i['public_ip'] for i in m['instances'] if i['role']=='bench'))
")"

echo ""
echo "=== Next steps on bench host ==="
echo "ssh -i ~/.ssh/continuum-bench.pem ec2-user@${BENCH_IP}"
echo ""
echo "eval \"\$($AWS/scripts/export-env.sh $TOPOLOGY)\""
echo "export CONTINUUM_BENCH_REPORTS_DIR=~/continuum-bench/reports"
echo "curl -sf \"\${CONTINUUM_BENCH_TIKV_PD_ENDPOINT}/health\""
echo ""
echo "~/continuum-bench/continuum-bench matrix --subset tikv-topology \\"
echo "  --hardware $HARDWARE --tikv-topology \$(echo \$CONTINUUM_BENCH_TIKV_TOPOLOGY) \\"
echo "  --skip-experiments bm-c6 --skip-existing"
echo ""
echo "Teardown: $AWS/scripts/down.sh $TOPOLOGY"
