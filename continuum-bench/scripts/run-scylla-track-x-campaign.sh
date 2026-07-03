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
"$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE"

echo "Track X: dual process C=$HALF each offset 0/$HALF"
export CONTINUUM_BENCH_CLIENT_COUNT="$HALF"
export CONTINUUM_BENCH_PARTITION_COUNT="$HALF"

marker="$(mktemp)"
touch "$marker"

export CONTINUUM_BENCH_PARTITION_OFFSET=0
nohup "$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE" \
  >~/continuum-bench/track-x-a.log 2>&1 &
PID_A=$!

export CONTINUUM_BENCH_PARTITION_OFFSET="$HALF"
nohup "$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE" \
  >~/continuum-bench/track-x-b.log 2>&1 &
PID_B=$!

wait "$PID_A" "$PID_B" || true

report_a="$(find "$REPORTS" -maxdepth 1 -name "bm-m4-${STORAGE}*.json" -newer "$marker" -printf '%T@ %p\n' 2>/dev/null | sort -rn | sed -n '1p' | cut -d' ' -f2-)"
report_b="$(find "$REPORTS" -maxdepth 1 -name "bm-m4-${STORAGE}*.json" -newer "$marker" -printf '%T@ %p\n' 2>/dev/null | sort -rn | sed -n '2p' | cut -d' ' -f2-)"
rm -f "$marker"

ops_a=0 ops_b=0
[[ -n "$report_a" && -f "$report_a" ]] && ops_a="$(python3 -c "import json; print(json.load(open('$report_a'))['metrics'].get('achieved_ops_per_sec',0))")"
[[ -n "$report_b" && -f "$report_b" ]] && ops_b="$(python3 -c "import json; print(json.load(open('$report_b'))['metrics'].get('achieved_ops_per_sec',0))")"
dual_ops="$(python3 - <<PY
a, b = float("$ops_a"), float("$ops_b")
print(a + b)
PY
)"

echo "Track X dual aggregate ops/s=$dual_ops (A=$ops_a B=$ops_b)"
echo "TRACK_X_DONE dual_ops=$dual_ops single_report=$report_a dual_a=$report_a dual_b=$report_b" >~/continuum-bench/campaign-track-x.done

unset CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT CONTINUUM_BENCH_PARTITION_OFFSET
