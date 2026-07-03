#!/usr/bin/env bash
# Track X: dual bench process @ C=256 on disjoint partition ranges.
# Usage: run-scylla-track-x-campaign.sh [hardware] [total_k]
set -euo pipefail

HARDWARE="${1:-aws-t3-medium}"
TOTAL_K="${2:-256}"
BENCH="${CONTINUUM_BENCH_BIN:-continuum-bench}"
STORAGE="${CONTINUUM_BENCH_STORAGE:-scylla}"
REPORTS="${CONTINUUM_BENCH_REPORTS_DIR:-$HOME/continuum-bench/reports}"
HALF=$((TOTAL_K / 2))

export CONTINUUM_BENCH_REPORTS_DIR="$REPORTS"
mkdir -p "$REPORTS"

echo "Track X: single process C=K=$TOTAL_K"
export CONTINUUM_BENCH_CLIENT_COUNT="$TOTAL_K"
export CONTINUUM_BENCH_PARTITION_COUNT="$TOTAL_K"
unset CONTINUUM_BENCH_PARTITION_OFFSET
export CONTINUUM_BENCH_REPORT_TAG=x-single
"$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE"

echo "Track X: dual process C=$HALF each offset 0/$HALF"
export CONTINUUM_BENCH_CLIENT_COUNT="$HALF"
export CONTINUUM_BENCH_PARTITION_COUNT="$HALF"

export CONTINUUM_BENCH_PARTITION_OFFSET=0
export CONTINUUM_BENCH_REPORT_TAG=x-dual-a
nohup "$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE" \
  >~/continuum-bench/track-x-a.log 2>&1 &
PID_A=$!

export CONTINUUM_BENCH_PARTITION_OFFSET="$HALF"
export CONTINUUM_BENCH_REPORT_TAG=x-dual-b
nohup "$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE" \
  >~/continuum-bench/track-x-b.log 2>&1 &
PID_B=$!

wait "$PID_A" "$PID_B" || true

report_a="$(ls -t "$REPORTS"/bm-m4-*-x-dual-a*.json 2>/dev/null | head -1 || true)"
report_b="$(ls -t "$REPORTS"/bm-m4-*-x-dual-b*.json 2>/dev/null | head -1 || true)"
single_report="$(ls -t "$REPORTS"/bm-m4-*-x-single*.json 2>/dev/null | head -1 || true)"

ops_a=0 ops_b=0
[[ -n "$report_a" && -f "$report_a" ]] && ops_a="$(python3 -c "import json; print(json.load(open('$report_a'))['metrics'].get('achieved_ops_per_sec',0))")"
[[ -n "$report_b" && -f "$report_b" ]] && ops_b="$(python3 -c "import json; print(json.load(open('$report_b'))['metrics'].get('achieved_ops_per_sec',0))")"
dual_ops="$(python3 - <<PY
a, b = float("$ops_a"), float("$ops_b")
print(a + b)
PY
)"

echo "Track X dual aggregate ops/s=$dual_ops (A=$ops_a B=$ops_b)"
echo "TRACK_X_DONE dual_ops=$dual_ops single_report=$single_report dual_a=$report_a dual_b=$report_b" >~/continuum-bench/campaign-track-x.done

unset CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT CONTINUUM_BENCH_PARTITION_OFFSET \
  CONTINUUM_BENCH_REPORT_TAG
