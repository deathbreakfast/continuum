#!/usr/bin/env bash
# Master driver: Scylla bottleneck diagnosis Tracks U–Y (WSL-resilient, teardown guaranteed).
# Usage: run-scylla-diagnosis.sh [--no-teardown-on-fail] [--skip-artifact] [--from-step STEP]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"

STATE_DIR="$ROOT/state"
STATE_FILE="$STATE_DIR/scylla-diagnosis.json"
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

# --- artifact ---
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

provision_colocated() {
  bash "$ROOT/scripts/provision-colocated.sh" --storage scylla
  CONTINUUM_NATIVE_SKIP_REPO_SYNC=1 bash "$ROOT/scripts/bootstrap.sh" native-colocated scylla
  for _ in $(seq 1 30); do
    HOST="$(manifest_read native-colocated | python3 -c "import json,sys; print(next(i['public_ip'] for i in json.load(sys.stdin)['instances'] if i['role']=='scylla'))")"
    if ssh -n -o StrictHostKeyChecking=no -i "$CONTINUUM_NATIVE_AWS_KEY_PATH" "ec2-user@$HOST" \
      "sudo docker exec continuum-scylla0 nodetool status 2>/dev/null | grep -q '^UN'" 2>/dev/null; then
      break
    fi
    sleep 5
  done
  CONTINUUM_NATIVE_USE_ARTIFACT=1 bash "$ROOT/scripts/deploy-bench.sh" native-colocated "$BIN" scylla
}

provision_topo() {
  local topo="$1"
  bash "$ROOT/scripts/provision-topology.sh" "$topo"
  bash "$ROOT/scripts/bootstrap-topology.sh" "$topo"
  bash "$ROOT/scripts/preflight-topology.sh" "$topo"
  CONTINUUM_NATIVE_USE_ARTIFACT=1 bash "$ROOT/scripts/deploy-bench.sh" "$topo" "$BIN" bench
}

run_colocated_tracks() {
  run_step colocated_provision provision_colocated
  run_step U_colocated bash "$ROOT/scripts/run-scylla-track-u.sh" native-colocated 256
  run_step V_colocated bash "$ROOT/scripts/run-scylla-track-v.sh" native-colocated
  run_step W_colocated bash "$ROOT/scripts/run-scylla-track-w.sh" native-colocated
  run_step Y_colocated bash "$ROOT/scripts/run-scylla-track-y.sh" native-colocated
  run_step fetch_colocated bash "$ROOT/scripts/fetch-reports.sh" native-colocated --upload-s3 || true
  bash "$ROOT/scripts/upload-diagnosis-state.sh" || true
  run_step teardown_colocated bash "$ROOT/scripts/teardown.sh" native-colocated
}

run_2n_tracks() {
  run_step 2n_provision provision_topo native-scylla-2n
  run_step U_2n bash "$ROOT/scripts/run-scylla-track-u.sh" native-scylla-2n 256
  run_step V_2n bash "$ROOT/scripts/run-scylla-track-v.sh" native-scylla-2n
  run_step W_2n bash "$ROOT/scripts/run-scylla-track-w.sh" native-scylla-2n
  run_step X_2n bash "$ROOT/scripts/run-scylla-track-x.sh" native-scylla-2n 256
  run_step fetch_2n bash "$ROOT/scripts/fetch-reports.sh" native-scylla-2n --upload-s3 || true
  bash "$ROOT/scripts/upload-diagnosis-state.sh" || true
  run_step teardown_2n bash "$ROOT/scripts/teardown.sh" native-scylla-2n
}

run_4n_tracks() {
  run_step 4n_provision provision_topo native-scylla-4n
  run_step U_4n bash "$ROOT/scripts/run-scylla-track-u.sh" native-scylla-4n 128
  run_step W_4n bash "$ROOT/scripts/run-scylla-track-w.sh" native-scylla-4n
  run_step fetch_4n bash "$ROOT/scripts/fetch-reports.sh" native-scylla-4n --upload-s3 || true
  bash "$ROOT/scripts/upload-diagnosis-state.sh" || true
  run_step teardown_4n bash "$ROOT/scripts/teardown.sh" native-scylla-4n
}

run_colocated_tracks
run_2n_tracks
run_4n_tracks

run_step teardown_all bash "$ROOT/scripts/teardown-all.sh"

trap - EXIT
echo "SCYLLA_DIAGNOSIS_COMPLETE"
