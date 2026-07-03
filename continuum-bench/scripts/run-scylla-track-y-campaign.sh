#!/usr/bin/env bash
# Track Y: BM-M4 @ C=256 comparing seq block sizes 64 vs 256.
# Usage: run-scylla-track-y-campaign.sh [hardware]
set -euo pipefail

HARDWARE="${1:-aws-t3-medium}"
BENCH="${CONTINUUM_BENCH_BIN:-continuum-bench}"
STORAGE="${CONTINUUM_BENCH_STORAGE:-scylla}"
CK=256

export CONTINUUM_BENCH_CLIENT_COUNT="$CK"
export CONTINUUM_BENCH_PARTITION_COUNT="$CK"
export CONTINUUM_APPEND_DEBUG_OPS=1
export CONTINUUM_BENCH_REPORTS_DIR="${CONTINUUM_BENCH_REPORTS_DIR:-$HOME/continuum-bench/reports}"

for BLOCK in 64 256; do
  export CONTINUUM_SCYLLA_SEQ_BLOCK_SIZE="$BLOCK"
  export CONTINUUM_BENCH_REPORT_TAG="y-blk${BLOCK}"
  echo "Track Y: BM-M4 C=K=$CK seq_block=$BLOCK"
  "$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE"
done

unset CONTINUUM_SCYLLA_SEQ_BLOCK_SIZE CONTINUUM_APPEND_DEBUG_OPS \
  CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT CONTINUUM_BENCH_REPORT_TAG
echo "TRACK_Y_DONE" >~/continuum-bench/campaign-track-y.done
