#!/usr/bin/env bash
# Run go-ycsb Test A (spread) + Test B (single-key) on TiKV host.
set -euo pipefail

ROOT="${HOME}/raw-engine-bench"
GOYCSB="${ROOT}/go-ycsb/bin/go-ycsb"
mkdir -p "$ROOT/logs" "$ROOT/done"

if [[ ! -x "$GOYCSB" ]]; then
  echo "go-ycsb not built at $GOYCSB — run bootstrap-tikv.sh first" >&2
  exit 1
fi

wait_tikv() {
  echo "waiting for tikv pd..."
  for _ in $(seq 1 60); do
    if curl -sf http://127.0.0.1:2379/pd/api/v1/stores 2>/dev/null | grep -qE '"state_name"[[:space:]]*:[[:space:]]*"Up"'; then
      echo "tikv ready"
      return 0
    fi
    sleep 5
  done
  echo "tikv not ready" >&2
  return 1
}

wait_tikv

run_sweep() {
  local id="$1"
  local props="$2"
  local log="$ROOT/logs/${id}.log"
  if [[ -f "$ROOT/done/$id" ]]; then
    echo "skip $id (done)"
    return 0
  fi
  echo ">>> start $id $(date -Iseconds)" | tee "$log"
  local best_tps=0
  local best_threads=0
  local best_p95=""
  for threads in 64 128 256 512 1024; do
    echo "--- threadcount=$threads ---" | tee -a "$log"
    local run_log="$ROOT/logs/${id}-t${threads}.log"
    if "$GOYCSB" run tikv -P "$props" -p "threadcount=${threads}" >"$run_log" 2>&1; then
      :
    fi
    cat "$run_log" >>"$log"
    local tps p95
    tps="$(grep -E '^TOTAL  - Takes\(s\):' "$run_log" | tail -1 | sed -n 's/.*OPS: \([0-9.]*\).*/\1/p')"
    p95="$(grep -E '^INSERT  - Takes\(s\):' "$run_log" | tail -1 | sed -n 's/.*95th(us): \([0-9]*\).*/\1/p')"
    if [[ -z "$tps" ]]; then
      tps="$(grep -E '^\[OVERALL\], Throughput\(ops/sec\)' "$run_log" | tail -1 | awk -F',' '{print $NF}' | tr -d ' ' || true)"
    fi
    if [[ -z "$p95" ]]; then
      p95="$(grep -E '^\[INSERT\], 95thPercentileLatency\(us\)' "$run_log" | tail -1 | awk -F',' '{print $NF}' | tr -d ' ' || true)"
    fi
    if [[ -n "$tps" ]] && python3 -c "import sys; sys.exit(0 if float('${tps}') > float('${best_tps}') else 1)" 2>/dev/null; then
      best_tps="$tps"
      best_threads="$threads"
      best_p95="$p95"
    fi
  done
  {
    echo "BEST_THREADS=$best_threads"
    echo "BEST_OPS_PER_SEC=$best_tps"
    echo "BEST_P95_US=$best_p95"
  } >>"$log"
  echo ">>> done $id best=${best_tps} ops/s threads=$best_threads" | tee -a "$log"
  touch "$ROOT/done/$id"
}

# Test A: spread keys — load once if needed
if [[ ! -f "$ROOT/done/tikv-a-load" ]]; then
  echo ">>> tikv-a load $(date -Iseconds)" | tee "$ROOT/logs/tikv-a-load.log"
  "$GOYCSB" load tikv -P "$ROOT/props/spread-insert.properties" \
    >"$ROOT/logs/tikv-a-load.log" 2>&1 || true
  touch "$ROOT/done/tikv-a-load"
fi
run_sweep tikv-a "$ROOT/props/spread-insert.properties"

# Test B: single key — load single record if needed
if [[ ! -f "$ROOT/done/tikv-b-load" ]]; then
  echo ">>> tikv-b load $(date -Iseconds)" | tee "$ROOT/logs/tikv-b-load.log"
  "$GOYCSB" load tikv -P "$ROOT/props/single-key-insert.properties" \
    -p recordcount=1 -p operationcount=1 -p threadcount=1 \
    >"$ROOT/logs/tikv-b-load.log" 2>&1 || true
  touch "$ROOT/done/tikv-b-load"
fi
run_sweep tikv-b "$ROOT/props/single-key-insert.properties"

echo "tikv-run-all complete"
