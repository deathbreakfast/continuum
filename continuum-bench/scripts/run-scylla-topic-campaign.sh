#!/usr/bin/env bash
# Topic fan-out campaign: BM-M5 across T topics x L2 on/off + mixed idempotency.
# Usage: run-scylla-topic-campaign.sh [hardware]
set -euo pipefail

HARDWARE="${1:-aws-t3-medium}"
BENCH="${CONTINUUM_BENCH_BIN:-continuum-bench}"
STORAGE="${CONTINUUM_BENCH_STORAGE:-scylla}"
REPORTS="${CONTINUUM_BENCH_REPORTS_DIR:-$HOME/continuum-bench/reports}"
DONE="$HOME/continuum-bench/campaign-topics.done"
CK=256

export CONTINUUM_BENCH_REPORTS_DIR="$REPORTS"
export CONTINUUM_BENCH_CLIENT_COUNT="$CK"
export CONTINUUM_BENCH_PARTITION_COUNT="$CK"
export CONTINUUM_APPEND_DEBUG_OPS=1
mkdir -p "$REPORTS" "$HOME/continuum-bench"

run_topic() {
  local t="$1"
  local l2="$2"
  local tag="topic-t${t}-l2${l2}"
  echo "=== topics T=$t L2=$l2 tag=$tag ==="
  (
    unset CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE CONTINUUM_BENCH_REPORT_TAG \
      CONTINUUM_SCYLLA_IDEMPOTENCY_NONE_TOPICS CONTINUUM_BENCH_TOPIC_COUNT \
      CONTINUUM_BENCH_TOPIC_OFFSET
    export CONTINUUM_BENCH_TOPIC_COUNT="$t"
    export CONTINUUM_BENCH_REPORT_TAG="$tag"
    if [[ "$l2" == "on" ]]; then
      export CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE=1
    else
      export CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE=0
    fi
    "$BENCH" run bm-m5 --storage "$STORAGE" --hardware "$HARDWARE"
  )
}

for T in 1 8 64; do
  for L2 in off on; do
    run_topic "$T" "$L2"
  done
done

echo "=== mixed per-topic idempotency T=64 ==="
(
  unset CONTINUUM_SCYLLA_TOPIC_INDEX_CACHE CONTINUUM_BENCH_REPORT_TAG \
    CONTINUUM_SCYLLA_IDEMPOTENCY_NONE_TOPICS CONTINUUM_BENCH_TOPIC_COUNT
  export CONTINUUM_BENCH_TOPIC_COUNT=64
  export CONTINUUM_BENCH_REPORT_TAG=idem-mixed-t64
  NONE_TOPICS="$(python3 - <<'PY'
print(",".join(f"bm-m5-t{i}" for i in range(32)))
PY
)"
  export CONTINUUM_SCYLLA_IDEMPOTENCY_NONE_TOPICS="$NONE_TOPICS"
  "$BENCH" run bm-m5 --storage "$STORAGE" --hardware "$HARDWARE"
)

echo "TOPICS_DONE" >"$DONE"

unset CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT \
  CONTINUUM_BENCH_REPORT_TAG CONTINUUM_APPEND_DEBUG_OPS
