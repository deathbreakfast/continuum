#!/usr/bin/env bash
# Run cassandra-stress Test A (spread) + Test B (single-key) on Scylla host.
set -euo pipefail

ROOT="${HOME}/raw-engine-bench"
mkdir -p "$ROOT/logs" "$ROOT/done"

wait_scylla() {
  echo "waiting for scylla on host :9042..."
  for _ in $(seq 1 60); do
    if timeout 1 bash -c 'cat < /dev/null > /dev/tcp/127.0.0.1/9042' 2>/dev/null; then
      echo "scylla ready"
      return 0
    fi
    sleep 5
  done
  echo "scylla not ready after 300s" >&2
  return 1
}

wait_scylla

stress_cmd() {
  sudo docker run --rm --network host scylladb/cassandra-stress "$@"
}

run_test() {
  local id="$1"
  shift
  if [[ -f "$ROOT/done/$id" ]]; then
    echo "skip $id (done)"
    return 0
  fi
  local log="$ROOT/logs/${id}.log"
  echo ">>> start $id $(date -Iseconds)"
  stress_cmd "$@" >"$log" 2>&1
  echo ">>> done $id $(date -Iseconds)"
  touch "$ROOT/done/$id"
}

RATE=( -rate 'threads>=16' 'threads<=512' auto )

# Test A: spread keys — 500k ops enough for auto-rate saturation
run_test scylla-a write n=500000 cl=ONE \
  -mode native cql3 \
  -node 127.0.0.1 \
  -col 'size=fixed(256)' \
  "${RATE[@]}"

# Test B: single key — duration-based (2M ops on one partition would take hours)
run_test scylla-b write duration=180s cl=ONE \
  -mode native cql3 \
  -node 127.0.0.1 \
  -col 'size=fixed(256)' \
  -pop seq=1..1 \
  "${RATE[@]}"

echo "scylla-run-all complete"
