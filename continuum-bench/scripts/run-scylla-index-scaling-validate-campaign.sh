#!/usr/bin/env bash
# Phase 2 validation: BM-M4 Z1 off with default backend config (L2 on by default).
# Usage: run-scylla-index-scaling-validate-campaign.sh [hardware]
set -euo pipefail

HARDWARE="${1:-aws-t3-medium}"
BENCH="${CONTINUUM_BENCH_BIN:-continuum-bench}"
STORAGE="${CONTINUUM_BENCH_STORAGE:-scylla}"
REPORTS="${CONTINUUM_BENCH_REPORTS_DIR:-$HOME/continuum-bench/reports}"
DONE="$HOME/continuum-bench/campaign-index-validate.done"
CK=256

export CONTINUUM_BENCH_REPORTS_DIR="$REPORTS"
export CONTINUUM_BENCH_CLIENT_COUNT="$CK"
export CONTINUUM_BENCH_PARTITION_COUNT="$CK"
export CONTINUUM_SCYLLA_IDEMPOTENCY=none
export CONTINUUM_APPEND_DEBUG_OPS=1
mkdir -p "$REPORTS" "$HOME/continuum-bench"

echo "=== index-validate bm-m4 z1off default-l2 tag=idx-validate-m4-default ==="
(
  unset CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE CONTINUUM_BENCH_REPORT_TAG
  export CONTINUUM_SCYLLA_IDEMPOTENCY=none
  export CONTINUUM_BENCH_REPORT_TAG=idx-validate-m4-default
  "$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE"
)

echo "INDEX_VALIDATE_DONE hardware=$HARDWARE" >"$DONE"

unset CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT \
  CONTINUUM_BENCH_REPORT_TAG CONTINUUM_APPEND_DEBUG_OPS CONTINUUM_SCYLLA_IDEMPOTENCY
