#!/usr/bin/env bash
# Track V: BM-M4 @ C=64 and C=256 with append debug ops.
# Usage: run-scylla-track-v-campaign.sh [hardware]
set -euo pipefail

HARDWARE="${1:-aws-t3-medium}"
BENCH="${CONTINUUM_BENCH_BIN:-continuum-bench}"
STORAGE="${CONTINUUM_BENCH_STORAGE:-scylla}"

export CONTINUUM_APPEND_DEBUG_OPS=1
export CONTINUUM_BENCH_REPORTS_DIR="${CONTINUUM_BENCH_REPORTS_DIR:-$HOME/continuum-bench/reports}"

for CK in 64 256; do
  export CONTINUUM_BENCH_CLIENT_COUNT="$CK"
  export CONTINUUM_BENCH_PARTITION_COUNT="$CK"
  echo "Track V: BM-M4 C=K=$CK debug=1"
  "$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE"
done

unset CONTINUUM_APPEND_DEBUG_OPS CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT
echo "TRACK_V_DONE" >~/continuum-bench/campaign-track-v.done
