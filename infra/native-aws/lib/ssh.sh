#!/usr/bin/env bash
# SSH/SCP helpers for native-aws EC2 workers.
set -euo pipefail

ssh_opts=(
  -o StrictHostKeyChecking=no
  -o UserKnownHostsFile=/dev/null
  -o ConnectTimeout=15
  -o BatchMode=yes
)

scp_opts=(
  -o StrictHostKeyChecking=no
  -o UserKnownHostsFile=/dev/null
  -o ConnectTimeout=15
  -o BatchMode=yes
)

ssh_cmd() {
  local host="$1"
  shift
  ssh "${ssh_opts[@]}" -i "$CONTINUUM_NATIVE_AWS_KEY_PATH" "ec2-user@${host}" "$@"
}

scp_to() {
  local host="$1"
  local src="$2"
  local dst="$3"
  scp "${scp_opts[@]}" -i "$CONTINUUM_NATIVE_AWS_KEY_PATH" "$src" "ec2-user@${host}:${dst}"
}

scp_from() {
  local host="$1"
  local src="$2"
  local dst="$3"
  scp "${scp_opts[@]}" -i "$CONTINUUM_NATIVE_AWS_KEY_PATH" "ec2-user@${host}:${src}" "$dst"
}

ssh_wait_ready() {
  local host="$1"
  local tries="${2:-40}"
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

rsync_repo() {
  local host="$1"
  local repo_root="$2"
  local attempt
  ssh_wait_ready "$host"
  for attempt in 1 2 3 4 5 6 7 8 9 10; do
    if tar czf - -C "$repo_root" --exclude target --exclude .git . | \
      ssh "${ssh_opts[@]}" -i "$CONTINUUM_NATIVE_AWS_KEY_PATH" "ec2-user@${host}" \
        "mkdir -p ~/continuum && tar xzf - -C ~/continuum"; then
      return 0
    fi
    echo "repo sync to $host failed (attempt $attempt); retrying..." >&2
    sleep 15
  done
  echo "repo sync to $host failed after 10 attempts" >&2
  return 1
}
