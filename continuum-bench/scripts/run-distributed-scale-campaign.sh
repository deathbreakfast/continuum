#!/usr/bin/env bash
# Track T: distributed topology scaling campaign with adaptive BM-M4 C=K sweep.
# Usage: run-distributed-scale-campaign.sh [hardware] [storage]
#   hardware: aws-t3-medium (default), aws-c7i-4xlarge, etc.
#   storage: auto (default), scylla, tikv-raw
#
# Requires bench env from export-env-topology.sh or export-env.sh.
set -euo pipefail

HARDWARE="${1:-aws-t3-medium}"
STORAGE_FILTER="${2:-auto}"
BENCH="${CONTINUUM_BENCH_BIN:-continuum-bench}"
REPORTS_DIR="${CONTINUUM_BENCH_REPORTS_DIR:-$HOME/continuum-bench/reports}"
export CONTINUUM_BENCH_REPORTS_DIR="$REPORTS_DIR"

GAIN_MIN="${CONTINUUM_BENCH_SWEEP_GAIN_MIN:-0.10}"
ERR_MAX="${CONTINUUM_BENCH_SWEEP_ERR_MAX:-0.01}"
CPU_MAX="${CONTINUUM_BENCH_SWEEP_CPU_MAX:-85}"
RAM_GIB_MAX="${CONTINUUM_BENCH_SWEEP_RAM_GIB_MAX:-3.5}"

detect_storage() {
  if [[ -n "${CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS:-}" ]]; then
    echo scylla
  elif [[ -n "${CONTINUUM_BENCH_TIKV_PD_ENDPOINT:-}" ]]; then
    echo tikv-raw
  else
    echo "missing CONTINUUM_BENCH_SCYLLA_CONTACT_POINTS or CONTINUUM_BENCH_TIKV_PD_ENDPOINT" >&2
    exit 1
  fi
}

if [[ "$STORAGE_FILTER" == auto ]]; then
  STORAGE_FILTER="$(detect_storage)"
fi

case "$STORAGE_FILTER" in
  scylla)
    BASE_LADDER=(128 256 512 1024)
    FIXED_CK=128
    ;;
  tikv-raw)
    BASE_LADDER=(64 128 256 512 1024)
    FIXED_CK=64
    ;;
  *)
    echo "unknown storage: $STORAGE_FILTER" >&2
    exit 1
    ;;
esac

run_bm_m4() {
  local ck="$1"
  export CONTINUUM_BENCH_CLIENT_COUNT="$ck"
  export CONTINUUM_BENCH_PARTITION_COUNT="$ck"
  local marker report pattern
  marker="$(mktemp)"
  touch "$marker"
  pattern="bm-m4-${STORAGE_FILTER}"
  if ! "$BENCH" run bm-m4 --storage "$STORAGE_FILTER" --hardware "$HARDWARE" >/dev/null; then
    echo "BM-M4 C=K=$ck FAILED" >&2
    rm -f "$marker"
    return 1
  fi
  report="$(find "$REPORTS_DIR" -maxdepth 1 -name "${pattern}*.json" -newer "$marker" -printf '%T@ %p\n' 2>/dev/null | sort -rn | head -1 | cut -d' ' -f2-)"
  rm -f "$marker"
  if [[ -z "$report" || ! -f "$report" ]]; then
    echo "BM-M4 C=K=$ck: no report under $REPORTS_DIR matching ${pattern}*" >&2
    return 1
  fi
  echo "$report"
}

parse_metrics() {
  local report="$1"
  python3 - "$report" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
m = r.get("metrics") or {}
rp = r.get("resource_profile") or {}
print(m.get("achieved_ops_per_sec") or 0)
print(m.get("error_rate") or 0)
print(rp.get("process_cpu_percent_peak") or 0)
print((rp.get("process_rss_bytes_peak") or 0) / (1024**3))
print(m.get("client_count") or 0)
print(m.get("p99_ms") or 0)
PY
}

should_escalate() {
  local prev_ops="$1" ops="$2" err="$3" cpu="$4" rss_gib="$5"
  python3 - "$prev_ops" "$ops" "$err" "$cpu" "$rss_gib" "$GAIN_MIN" "$ERR_MAX" "$CPU_MAX" "$RAM_GIB_MAX" <<'PY'
import sys
prev, ops, err, cpu, rss = map(float, sys.argv[1:6])
gain_min, err_max, cpu_max, ram_max = map(float, sys.argv[6:10])
if err >= err_max:
    print("stop:errors")
    sys.exit(1)
if cpu >= cpu_max or rss >= ram_max:
    print("stop:bench-bound")
    sys.exit(2)
if prev <= 0:
    sys.exit(0)
gain = ops / prev - 1.0
if gain < gain_min:
    print("stop:plateau")
    sys.exit(1)
sys.exit(0)
PY
}

adaptive_sweep() {
  local ck prev_ops=0 ops err cpu rss_gib peak_ops=0 peak_ck=0 report reason=""
  local -a ladder=("${BASE_LADDER[@]}")
  local next_extra=2048

  echo "=== BM-M4 adaptive sweep storage=$STORAGE_FILTER hardware=$HARDWARE ==="

  while true; do
    for ck in "${ladder[@]}"; do
      echo "--- C=K=$ck ---"
      report="$(run_bm_m4 "$ck")"
      read -r ops err cpu rss_gib client_count p99_ms < <(parse_metrics "$report")
      echo "  ops/s=$ops err=$err cpu_peak=${cpu}% rss_peak=${rss_gib}GiB p99=${p99_ms}ms report=$(basename "$report")"

      if python3 - "$ops" "$peak_ops" <<'PY'
import sys
ops, peak = map(float, sys.argv[1:3])
sys.exit(0 if ops > peak else 1)
PY
      then
        peak_ops="$ops"
        peak_ck="$ck"
      fi

      if [[ "$prev_ops" != "0" ]]; then
        set +e
        reason="$(should_escalate "$prev_ops" "$ops" "$err" "$cpu" "$rss_gib")"
        rc=$?
        set -e
        if [[ $rc -ne 0 ]]; then
          echo "  sweep stop: $reason (peak C=K=$peak_ck @ ${peak_ops} ops/s)"
          if [[ "$reason" == "stop:bench-bound" ]]; then
            echo "  BENCH-BOUND -> Phase 5 candidate (larger bench instance)"
          fi
          echo "PEAK_CK=$peak_ck PEAK_OPS=$peak_ops"
          return 0
        fi
      fi
      prev_ops="$ops"
    done

    # Extend ladder while top tier still passes
    if [[ "$prev_ops" == "0" ]]; then
      break
    fi
    echo "--- extending ladder to C=K=$next_extra ---"
    ladder=("$next_extra")
    next_extra=$((next_extra * 2))
  done

  echo "PEAK_CK=$peak_ck PEAK_OPS=$peak_ops"
}

run_experiment() {
  local exp="$1"
  echo ">>> $exp"
  "$BENCH" run "$exp" --storage "$STORAGE_FILTER" --hardware "$HARDWARE"
}

echo "DISTRIBUTED_SCALE_CAMPAIGN storage=$STORAGE_FILTER hardware=$HARDWARE reports=$REPORTS_DIR"

adaptive_sweep

echo "=== fixed-load reference C=K=$FIXED_CK ==="
export CONTINUUM_BENCH_CLIENT_COUNT="$FIXED_CK"
export CONTINUUM_BENCH_PARTITION_COUNT="$FIXED_CK"
run_experiment bm-m4

echo "=== BM-L3 partitioned K=64 ==="
unset CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT
export CONTINUUM_BENCH_LOAD_PARTITION_COUNT=64
run_experiment bm-l3
unset CONTINUUM_BENCH_LOAD_PARTITION_COUNT

echo "=== BM-P1 K=128 ==="
export CONTINUUM_BENCH_PARTITION_COUNT=128
run_experiment bm-p1
unset CONTINUUM_BENCH_PARTITION_COUNT

echo "=== BM-L3 hot stream (control) ==="
run_experiment bm-l3

echo "DISTRIBUTED_SCALE_CAMPAIGN_DONE"
