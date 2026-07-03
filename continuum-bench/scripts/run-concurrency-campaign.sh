#!/usr/bin/env bash
# Concurrency sweeps for aws-t3-medium native campaign (Track M — BM-M3).
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

echo "Concurrency campaign hardware=$HARDWARE reports=$CONTINUUM_BENCH_REPORTS_DIR"

for C in 8 64 128; do
  export CONTINUUM_BENCH_CLIENT_COUNT=$C
  run_matrix native-concurrency
done

unset CONTINUUM_BENCH_CLIENT_COUNT

echo "CONCURRENCY_CAMPAIGN_DONE" | tee ~/continuum-bench/campaign-concurrency.done
