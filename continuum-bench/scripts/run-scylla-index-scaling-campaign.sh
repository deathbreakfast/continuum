#!/usr/bin/env bash
# Track AA: index scaling matrix — BM-M4/M5 × L2 on/off × Z1 off @ C=K=256.
# Usage: run-scylla-index-scaling-campaign.sh [hardware]
set -euo pipefail

HARDWARE="${1:-aws-t3-medium}"
BENCH="${CONTINUUM_BENCH_BIN:-continuum-bench}"
STORAGE="${CONTINUUM_BENCH_STORAGE:-scylla}"
REPORTS="${CONTINUUM_BENCH_REPORTS_DIR:-$HOME/continuum-bench/reports}"
DONE="$HOME/continuum-bench/campaign-index-scaling.done"
CK=256

export CONTINUUM_BENCH_REPORTS_DIR="$REPORTS"
export CONTINUUM_BENCH_CLIENT_COUNT="$CK"
export CONTINUUM_BENCH_PARTITION_COUNT="$CK"
export CONTINUUM_SCYLLA_IDEMPOTENCY=none
export CONTINUUM_APPEND_DEBUG_OPS=1
mkdir -p "$REPORTS" "$HOME/continuum-bench"

run_case() {
  local exp="$1"
  local tag="$2"
  local t_count="${3:-}"
  echo "=== index-scaling exp=$exp tag=$tag T=${t_count:-1} ==="
  (
    unset CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE CONTINUUM_BENCH_REPORT_TAG \
      CONTINUUM_BENCH_TOPIC_COUNT CONTINUUM_BENCH_TOPIC_OFFSET \
      CONTINUUM_SCYLLA_IDEMPOTENCY_NONE_TOPICS
    export CONTINUUM_SCYLLA_IDEMPOTENCY=none
    export CONTINUUM_BENCH_REPORT_TAG="$tag"
    if [[ -n "$t_count" ]]; then
      export CONTINUUM_BENCH_TOPIC_COUNT="$t_count"
    fi
    if [[ "$tag" == *"-l2on" ]]; then
      export CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE=1
    else
      export CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE=0
    fi
    "$BENCH" run "$exp" --storage "$STORAGE" --hardware "$HARDWARE"
  )
}

for L2 in off on; do
  run_case bm-m4 "idx-z1off-m4-l2${L2}"
done

for T in 8 64; do
  for L2 in off on; do
    run_case bm-m5 "idx-z1off-t${T}-l2${L2}" "$T"
  done
done

echo "INDEX_SCALING_DONE hardware=$HARDWARE" >"$DONE"

unset CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT \
  CONTINUUM_BENCH_REPORT_TAG CONTINUUM_APPEND_DEBUG_OPS \
  CONTINUUM_SCYLLA_IDEMPOTENCY CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE \
  CONTINUUM_BENCH_TOPIC_COUNT
