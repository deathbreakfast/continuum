#!/usr/bin/env bash
# Cross-build continuum-bench for Amazon Linux 2023 (glibc match).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
OUT="${1:-$ROOT/target/al2023/continuum-bench}"
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"

mkdir -p "$(dirname "$OUT")"
mkdir -p "$ROOT/target" "$CARGO_HOME/registry" "$CARGO_HOME/git"

docker run --rm \
  -v "${ROOT}:/work:rw" \
  -v "${CARGO_HOME}/registry:/usr/local/cargo/registry:rw" \
  -v "${CARGO_HOME}/git:/usr/local/cargo/git:rw" \
  -w /work \
  amazonlinux:2023 \
  bash -lc '
    set -euo pipefail
    if ! command -v cargo >/dev/null 2>&1; then
      dnf install -y gcc openssl-devel pkgconfig git clang
      curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    fi
    source "$HOME/.cargo/env"
    export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
    cargo build --release -p continuum-bench -j "${CARGO_BUILD_JOBS}"
  '

cp "$ROOT/target/release/continuum-bench" "$OUT"
chmod +x "$OUT"
test -f "$OUT"
echo "Built $OUT"

SCRIPT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_ROOT/config/defaults.env" 2>/dev/null || true
if [[ -n "${CONTINUUM_NATIVE_ARTIFACT_BUCKET:-}" ]]; then
  bash "$SCRIPT_ROOT/scripts/artifact-upload.sh" "$OUT" || true
fi
