#!/usr/bin/env bash
# Ceiling campaign: BM-M4 Z1 on/off sweep across client counts.
# Usage: run-scylla-ceiling-campaign.sh <hardware> <c-list-space-separated>
set -euo pipefail

HARDWARE="${1:-aws-t3-medium}"
C_LIST="${2:-256}"
BENCH="${CONTINUUM_BENCH_BIN:-continuum-bench}"
STORAGE="${CONTINUUM_BENCH_STORAGE:-scylla}"
REPORTS="${CONTINUUM_BENCH_REPORTS_DIR:-$HOME/continuum-bench/reports}"
DONE="$HOME/continuum-bench/campaign-ceiling.done"

export CONTINUUM_BENCH_REPORTS_DIR="$REPORTS"
export CONTINUUM_APPEND_DEBUG_OPS=1
mkdir -p "$REPORTS" "$HOME/continuum-bench"

run_one() {
  local tag="$1"
  local ck="$2"
  local idem="$3"
  echo "=== ceiling tag=$tag C=$ck idempotency=$idem ==="
  (
    unset CONTINUUM_SCYLLA_IDEMPOTENCY CONTINUUM_SCYLLA_IDEMPOTENCY_NONE_TOPICS \
      CONTINUUM_BENCH_REPORT_TAG
    export CONTINUUM_BENCH_CLIENT_COUNT="$ck"
    export CONTINUUM_BENCH_PARTITION_COUNT="$ck"
    export CONTINUUM_BENCH_REPORT_TAG="$tag"
    export CONTINUUM_SCYLLA_IDEMPOTENCY="$idem"
    "$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE"
  )
}

for CK in $C_LIST; do
  run_one "ceil-z1on-c${CK}" "$CK" "lwt"
  run_one "ceil-z1off-c${CK}" "$CK" "none"
done

echo "CEILING_DONE hardware=$HARDWARE c_list=$C_LIST" >"$DONE"

unset CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT \
  CONTINUUM_BENCH_REPORT_TAG CONTINUUM_APPEND_DEBUG_OPS CONTINUUM_SCYLLA_IDEMPOTENCY
