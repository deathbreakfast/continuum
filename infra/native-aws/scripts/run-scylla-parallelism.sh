#!/usr/bin/env bash
# Master driver: topic fan-out + multi-publisher scaling (WSL-resilient).
# Usage: run-scylla-parallelism.sh [--no-teardown-on-fail] [--skip-artifact] [--from-step STEP]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"

STATE_DIR="$ROOT/state"
STATE_FILE="$STATE_DIR/scylla-parallelism.json"
BIN="$REPO_ROOT/target/al2023/continuum-bench"
TEARDOWN_ON_FAIL=true
SKIP_ARTIFACT=false
FROM_STEP=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-teardown-on-fail) TEARDOWN_ON_FAIL=false ;;
    --skip-artifact) SKIP_ARTIFACT=true ;;
    --from-step) FROM_STEP="${2:-}"; shift ;;
    *) echo "unknown arg: $1" >&2; exit 1 ;;
  esac
  shift
done

mkdir -p "$STATE_DIR"
if [[ ! -f "$STATE_FILE" ]]; then
  echo '{"completed":[],"current":null,"failed":null}' >"$STATE_FILE"
fi

state_init() {
  python3 - "$STATE_FILE" <<'PY'
import json, sys
path = sys.argv[1]
try:
  json.load(open(path))
except Exception:
  with open(path, "w") as f:
    json.dump({"completed": [], "current": None, "failed": None}, f)
PY
}

state_done() {
  local step="$1"
  python3 - "$STATE_FILE" "$step" <<'PY'
import json, sys
path, step = sys.argv[1], sys.argv[2]
s = json.load(open(path))
if step not in s["completed"]:
  s["completed"].append(step)
s["current"] = None
s["failed"] = None
json.dump(s, open(path, "w"), indent=2)
print(f"state: completed {step}")
PY
}

state_current() {
  local step="$1"
  python3 - "$STATE_FILE" "$step" <<'PY'
import json, sys
s = json.load(open(path := sys.argv[1]))
s["current"] = sys.argv[2]
json.dump(s, open(path, "w"), indent=2)
PY
}

state_failed() {
  local step="$1"
  python3 - "$STATE_FILE" "$step" <<'PY'
import json, sys
s = json.load(open(path := sys.argv[1]))
s["failed"] = sys.argv[2]
json.dump(s, open(path, "w"), indent=2)
PY
}

state_skip() {
  local step="$1"
  python3 - "$STATE_FILE" "$step" <<'PY'
import json, sys
step = sys.argv[2]
s = json.load(open(sys.argv[1]))
print(step in s.get("completed", []))
PY
}

FAILED=false
CURRENT_STEP=""
SKIPPING=false
[[ -n "$FROM_STEP" ]] && SKIPPING=true

cleanup() {
  local rc=$?
  if [[ $rc -ne 0 ]]; then
    FAILED=true
    state_failed "${CURRENT_STEP:-unknown}" "exit $rc"
    if $TEARDOWN_ON_FAIL; then
      echo ">>> trap: teardown-all after failure"
      bash "$ROOT/scripts/teardown-all.sh" || true
    fi
  fi
}
trap cleanup EXIT

run_step() {
  local step="$1"
  shift
  if $SKIPPING; then
    if [[ "$step" == "$FROM_STEP" ]]; then
      SKIPPING=false
    else
      echo ">>> skip (before --from-step): $step"
      return 0
    fi
  fi
  if state_skip "$step" | grep -q True; then
    echo ">>> skip (completed): $step"
    return 0
  fi
  CURRENT_STEP="$step"
  state_current "$step"
  echo "========== $step =========="
  "$@"
  state_done "$step"
}

state_init

if ! $SKIP_ARTIFACT; then
  run_step artifact bash -c "
    if [[ -z \"\${CONTINUUM_NATIVE_ARTIFACT_BUCKET:-}\" ]]; then
      ACCOUNT=\$(aws sts get-caller-identity --query Account --output text)
      export CONTINUUM_NATIVE_ARTIFACT_BUCKET=continuum-bench-artifacts-\${ACCOUNT}
      bash '$ROOT/scripts/setup-artifact-bucket.sh' \"\$CONTINUUM_NATIVE_ARTIFACT_BUCKET\" || true
    fi
    bash '$ROOT/scripts/artifact-fetch.sh' --build-if-missing
  "
  # shellcheck disable=SC1091
  source "$ROOT/lib/artifact.sh"
  BIN="$(artifact_local_path "$REPO_ROOT")"
fi

provision_topo() {
  local topo="$1"
  bash "$ROOT/scripts/provision-topology.sh" "$topo"
  bash "$ROOT/scripts/bootstrap-topology.sh" "$topo"
  bash "$ROOT/scripts/preflight-topology.sh" "$topo"
  CONTINUUM_NATIVE_USE_ARTIFACT=1 bash "$ROOT/scripts/deploy-bench.sh" "$topo" "$BIN" bench
}

run_parallelism_phase() {
  local prefix="$1"
  local topo="$2"
  run_step "${prefix}_provision" provision_topo "$topo"
  run_step "${prefix}_topics" bash "$ROOT/scripts/run-scylla-topics.sh" "$topo"
  for n in 1 2 4; do
    run_step "${prefix}_pub${n}" bash "$ROOT/scripts/run-scylla-publishers.sh" "$topo" "$n"
  done
  run_step "${prefix}_fetch" bash "$ROOT/scripts/fetch-reports.sh" "$topo" --upload-s3 || true
  run_step "${prefix}_teardown" bash "$ROOT/scripts/teardown.sh" "$topo"
}

run_parallelism_phase 1n native-scylla-1n
run_parallelism_phase 2n native-scylla-2n

run_step teardown_all bash "$ROOT/scripts/teardown-all.sh"

trap - EXIT
echo "SCYLLA_PARALLELISM_COMPLETE"
