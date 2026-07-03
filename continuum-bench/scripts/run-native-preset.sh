#!/usr/bin/env bash
# Run native adapter benchmark subset (scylla + tikv-raw when env configured).
set -euo pipefail

HARDWARE="${1:-aws-t3-medium}"
SUBSET="${2:-native-lab}"
BENCH="${CONTINUUM_BENCH_BIN:-cargo run --release -p continuum-bench --}"

export CONTINUUM_BENCH_REPORTS_DIR="${CONTINUUM_BENCH_REPORTS_DIR:-$(cd "$(dirname "$0")/../../profiling/continuum-bench/reports" && pwd)}"

echo "Native matrix hardware=$HARDWARE subset=$SUBSET reports=$CONTINUUM_BENCH_REPORTS_DIR"

eval "$BENCH matrix --hardware \"$HARDWARE\" --subset \"$SUBSET\" --skip-existing"

echo "NATIVE_MATRIX_DONE"
