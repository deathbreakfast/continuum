#!/usr/bin/env bash
# S3 artifact key helpers for continuum-bench AL2023 binary.
set -euo pipefail

artifact_repo_root() {
  cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd
}

artifact_git_sha() {
  git -C "$(artifact_repo_root)" rev-parse HEAD 2>/dev/null || echo "unknown"
}

artifact_lock_hash() {
  sha256sum "$(artifact_repo_root)/Cargo.lock" | awk '{print substr($1,1,8)}'
}

artifact_key() {
  echo "al2023/$(artifact_git_sha)-$(artifact_lock_hash)/continuum-bench"
}

artifact_s3_uri() {
  local bucket="${1:?bucket required}"
  echo "s3://${bucket}/$(artifact_key)"
}

artifact_local_path() {
  local root="${1:-$(artifact_repo_root)}"
  echo "${root}/target/al2023/continuum-bench"
}
