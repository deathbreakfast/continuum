#!/usr/bin/env bash
# Deploy raw-engine-bench scripts to EC2 host(s).
# Usage: bootstrap.sh [scylla|tikv|both]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/lib/common.sh"

TARGET="${1:-both}"

deploy_host() {
  local role="$1"
  local host
  host="$(manifest_host "$role")"
  echo ">>> bootstrap $role ($host)"
  ssh_cmd "$host" "mkdir -p ~/${REMOTE_DIR}/{logs,done,props,remote}"

  scp_to "$host" "$ROOT/remote/scylla-run-all.sh" "~/${REMOTE_DIR}/remote/"
  scp_to "$host" "$ROOT/remote/tikv-run-all.sh" "~/${REMOTE_DIR}/remote/"
  scp_to "$host" "$ROOT/props/spread-insert.properties" "~/${REMOTE_DIR}/props/"
  scp_to "$host" "$ROOT/props/single-key-insert.properties" "~/${REMOTE_DIR}/props/"
  ssh_cmd "$host" "chmod +x ~/${REMOTE_DIR}/remote/*.sh"

  if [[ "$role" == "tikv" ]]; then
    ssh_cmd "$host" "bash -s" <<'EOF'
set -euo pipefail
ROOT="${HOME}/raw-engine-bench"
if [[ ! -x "${ROOT}/go-ycsb/bin/go-ycsb" ]]; then
  sudo dnf install -y golang git make
  rm -rf "${ROOT}/go-ycsb-src"
  git clone --depth 1 https://github.com/pingcap/go-ycsb.git "${ROOT}/go-ycsb-src"
  cd "${ROOT}/go-ycsb-src"
  make
  ln -sfn "${ROOT}/go-ycsb-src" "${ROOT}/go-ycsb"
fi
EOF
  fi
  echo "bootstrap $role done"
}

case "$TARGET" in
  scylla) deploy_host scylla ;;
  tikv) deploy_host tikv ;;
  both)
    deploy_host scylla
    deploy_host tikv
    ;;
  *) echo "usage: $0 [scylla|tikv|both]" >&2; exit 1 ;;
esac
