#!/usr/bin/env bash
# Partitioning sweeps for aws-t3-medium native campaign (Track P).
set -euo pipefail

HARDWARE="${1:-aws-t3-medium}"
STORAGE_FILTER="${2:-}"
BENCH="${CONTINUUM_BENCH_BIN:-cargo run --release -p continuum-bench --}"

export CONTINUUM_BENCH_REPORTS_DIR="${CONTINUUM_BENCH_REPORTS_DIR:-$(cd "$(dirname "$0")/../../profiling/continuum-bench/reports" && pwd)}"

storage_args=()
if [[ -n "$STORAGE_FILTER" ]]; then
  storage_args=(--storages "$STORAGE_FILTER")
fi

run_matrix() {
  local subset="$1"
  shift
  echo ">>> matrix subset=$subset $*"
  eval "$BENCH matrix --hardware \"$HARDWARE\" --subset \"$subset\" --skip-existing $* ${storage_args[*]:-}"
}

echo "Partition campaign hardware=$HARDWARE reports=$CONTINUUM_BENCH_REPORTS_DIR"

for K in 10 64 256; do
  export CONTINUUM_BENCH_LOAD_PARTITION_COUNT=$K
  run_matrix native-lab-partitioned
done
unset CONTINUUM_BENCH_LOAD_PARTITION_COUNT

for K in 10 64 128; do
  export CONTINUUM_BENCH_PARTITION_COUNT=$K
  run_matrix native-scale --skip-experiments bm-m1,bm-m2
done

for C in 8 64 128; do
  export CONTINUUM_BENCH_CLIENT_COUNT=$C
  export CONTINUUM_BENCH_PARTITION_COUNT=$C
  run_matrix native-scale --skip-experiments bm-p1,bm-p2,bm-m1,bm-m2
done

unset CONTINUUM_BENCH_PARTITION_COUNT CONTINUUM_BENCH_CLIENT_COUNT

echo "PARTITION_CAMPAIGN_DONE"
