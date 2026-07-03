#!/usr/bin/env bash
# A/B campaign for Scylla append-path levers (Tracks Z1–Z5).
# Usage: run-scylla-lever-campaign.sh <z1|z2|z3|z4|z5> [hardware] [c=k]
set -euo pipefail

LEVER="${1:?z1|z2|z3|z4|z5}"
HARDWARE="${2:-aws-t3-medium}"
CK="${3:-256}"
BENCH="${CONTINUUM_BENCH_BIN:-continuum-bench}"
STORAGE="${CONTINUUM_BENCH_STORAGE:-scylla}"
REPORTS="${CONTINUUM_BENCH_REPORTS_DIR:-$HOME/continuum-bench/reports}"
DONE="$HOME/continuum-bench/campaign-${LEVER}.done"

export CONTINUUM_BENCH_CLIENT_COUNT="$CK"
export CONTINUUM_BENCH_PARTITION_COUNT="$CK"
export CONTINUUM_APPEND_DEBUG_OPS=1
export CONTINUUM_BENCH_REPORTS_DIR="$REPORTS"
mkdir -p "$REPORTS" "$HOME/continuum-bench"

run_one() {
  local tag="$1"
  shift
  echo "=== lever $LEVER tag=$tag ==="
  (
    unset CONTINUUM_SCYLLA_IDEMPOTENCY \
      CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE \
      CONTINUUM_SCYLLA_PIPELINE_WRITES \
      CONTINUUM_SCYLLA_WRITE_CONSISTENCY \
      CONTINUUM_SCYLLA_POOL_PER_SHARD \
      CONTINUUM_BENCH_REPORT_TAG \
      CONTINUUM_BENCH_PARTITION_OFFSET
    export CONTINUUM_BENCH_REPORT_TAG="$tag"
    for kv in "$@"; do
      export "$kv"
    done
    "$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE"
  )
}

case "$LEVER" in
  z1)
    run_one "${LEVER}-baseline-lwt" "CONTINUUM_SCYLLA_IDEMPOTENCY=lwt"
    run_one "${LEVER}-treatment-none" "CONTINUUM_SCYLLA_IDEMPOTENCY=none"
    ;;
  z2)
    run_one "${LEVER}-baseline" "CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE=0"
    run_one "${LEVER}-treatment" "CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE=1"
    ;;
  z3)
    run_one "${LEVER}-baseline"
    run_one "${LEVER}-treatment" "CONTINUUM_SCYLLA_PIPELINE_WRITES=1"
    ;;
  z4)
    run_one "${LEVER}-baseline"
    run_one "${LEVER}-treatment" "CONTINUUM_SCYLLA_WRITE_CONSISTENCY=one"
    ;;
  z5)
    run_one "${LEVER}-baseline"
    run_one "${LEVER}-treatment" "CONTINUUM_SCYLLA_POOL_PER_SHARD=4"
    ;;
  *)
    echo "unknown lever: $LEVER (use z1-z5)" >&2
    exit 1
    ;;
esac

echo "${LEVER^^}_DONE" >"$DONE"

unset CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT CONTINUUM_APPEND_DEBUG_OPS \
  CONTINUUM_BENCH_REPORT_TAG
