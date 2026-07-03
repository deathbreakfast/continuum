#!/usr/bin/env bash
# SSH/SCP helpers for native-aws EC2 workers.
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
  ssh "${ssh_opts[@]}" -i "$CONTINUUM_NATIVE_AWS_KEY_PATH" "ec2-user@${host}" "$@"
}

scp_to() {
  local host="$1"
  local src="$2"
  local dst="$3"
  scp "${ssh_opts[@]}" -i "$CONTINUUM_NATIVE_AWS_KEY_PATH" "$src" "ec2-user@${host}:${dst}"
}

scp_from() {
  local host="$1"
  local src="$2"
  local dst="$3"
  scp "${ssh_opts[@]}" -i "$CONTINUUM_NATIVE_AWS_KEY_PATH" "ec2-user@${host}:${src}" "$dst"
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
  rsync -az --delete \
    -e "ssh ${ssh_opts[*]} -i $CONTINUUM_NATIVE_AWS_KEY_PATH" \
    --exclude target --exclude .git \
    "$repo_root/" "ec2-user@${host}:~/continuum/"
}
