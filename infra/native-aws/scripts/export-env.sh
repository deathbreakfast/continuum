#!/usr/bin/env bash
# Print bench env vars for colocated native runs.
# Usage: export-env.sh <role>   (scylla|tikv)
set -euo pipefail

ROLE="${1:?usage: export-env.sh scylla|tikv}"

case "$ROLE" in
  scylla)
    echo "export CONTINUUM_BENCH_SCYLLA_TOPOLOGY=scylla-1"
    echo "export CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS=127.0.0.1:9042"
    echo "export CONTINUUM_BENCH_SCYLLA_KEYSPACE=continuum"
    ;;
  tikv)
    echo "export CONTINUUM_BENCH_TIKV_TOPOLOGY=tikv-minimal"
    echo "export CONTINUUM_BENCH_TIKV_PD_ENDPOINT=http://127.0.0.1:2379"
    ;;
  *)
    echo "unknown role: $ROLE" >&2
    exit 1
    ;;
esac
echo "export CONTINUUM_BENCH_REPORTS_DIR=\${HOME}/continuum-bench/reports"
