#!/usr/bin/env bash
# Multi-publisher campaign: N processes across 64 topics, aggregate ops/s.
# Usage: run-scylla-publishers-campaign.sh <N> [total_topics] [c_proc] [hardware]
set -euo pipefail

N="${1:?N publishers}"
TOTAL_TOPICS="${2:-64}"
C_PROC="${3:-256}"
HARDWARE="${4:-aws-t3-medium}"
BENCH="${CONTINUUM_BENCH_BIN:-continuum-bench}"
STORAGE="${CONTINUUM_BENCH_STORAGE:-scylla}"
REPORTS="${CONTINUUM_BENCH_REPORTS_DIR:-$HOME/continuum-bench/reports}"
DONE="$HOME/continuum-bench/campaign-publishers-N${N}.done"

export CONTINUUM_BENCH_REPORTS_DIR="$REPORTS"
export CONTINUUM_BENCH_PARTITION_COUNT="$C_PROC"
mkdir -p "$REPORTS" "$HOME/continuum-bench"

TOPICS_PER=$((TOTAL_TOPICS / N))
PIDS=()

for ((p = 0; p < N; p++)); do
  OFFSET=$((p * TOPICS_PER))
  TAG="pub-N${N}-p${p}"
  echo "publisher p=$p topics=$TOPICS_PER offset=$OFFSET tag=$TAG"
  (
    unset CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_TOPIC_COUNT \
      CONTINUUM_BENCH_TOPIC_OFFSET CONTINUUM_BENCH_REPORT_TAG
    export CONTINUUM_BENCH_CLIENT_COUNT="$C_PROC"
    export CONTINUUM_BENCH_TOPIC_COUNT="$TOPICS_PER"
    export CONTINUUM_BENCH_TOPIC_OFFSET="$OFFSET"
    export CONTINUUM_BENCH_REPORT_TAG="$TAG"
    "$BENCH" run bm-m5 --storage "$STORAGE" --hardware "$HARDWARE"
  ) &
  PIDS+=($!)
done

for pid in "${PIDS[@]}"; do
  wait "$pid" || true
done

aggregate=0
for ((p = 0; p < N; p++)); do
  report="$(ls -t "$REPORTS"/bm-m5-*-pub-N"${N}"-p"${p}"*.json 2>/dev/null | head -1 || true)"
  if [[ -n "$report" && -f "$report" ]]; then
    ops="$(python3 -c "import json; print(json.load(open('$report'))['metrics'].get('achieved_ops_per_sec',0))")"
    aggregate="$(python3 - <<PY
print(float("$aggregate") + float("$ops"))
PY
)"
  fi
done

echo "PUBLISHERS_DONE N=$N aggregate_ops=$aggregate" >"$DONE"
echo "aggregate ops/s=$aggregate (N=$N)"

unset CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT \
  CONTINUUM_BENCH_TOPIC_COUNT CONTINUUM_BENCH_TOPIC_OFFSET CONTINUUM_BENCH_REPORT_TAG
