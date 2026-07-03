#!/usr/bin/env bash
# SSH/SCP helpers for EC2 workers.
set -euo pipefail

ssh_opts=(
  -o StrictHostKeyChecking=no
  -o UserKnownHostsFile=/dev/null
  -o ConnectTimeout=15
  -o BatchMode=yes
)

ssh_cmd() {
  local host="$1"
  shift
  ssh "${ssh_opts[@]}" -i "$CONTINUUM_TIKV_AWS_KEY_PATH" "ec2-user@${host}" "$@"
}

scp_to() {
  local host="$1"
  local src="$2"
  local dst="$3"
  scp "${ssh_opts[@]}" -i "$CONTINUUM_TIKV_AWS_KEY_PATH" "$src" "ec2-user@${host}:${dst}"
}

ssh_wait_ready() {
  local host="$1"
  local tries="${2:-30}"
  local i
  for ((i = 1; i <= tries; i++)); do
    if ssh_cmd "$host" "echo ready" >/dev/null 2>&1; then
      return 0
    fi
    sleep 5
  done
  echo "SSH not ready on $host after ${tries} attempts" >&2
  return 1
}

# Run a local script on remote with optional env exports prepended.
ssh_run_local_script() {
  local host="$1"
  local env_file="$2"
  local script_file="$3"
  {
    if [[ -f "$env_file" ]]; then
      cat "$env_file"
    fi
    cat "$script_file"
  } | ssh_cmd "$host" "bash -s"
}
