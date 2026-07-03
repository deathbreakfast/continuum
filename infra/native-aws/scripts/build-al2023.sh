#!/usr/bin/env bash
# Cross-build continuum-bench for Amazon Linux 2023 (glibc match).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
OUT="${1:-$ROOT/target/al2023/continuum-bench}"

mkdir -p "$(dirname "$OUT")"

docker run --rm \
  -v "${ROOT}:/work:rw" \
  -w /work \
  amazonlinux:2023 \
  bash -lc '
    set -euo pipefail
    dnf install -y gcc openssl-devel pkgconfig git clang
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
    cargo build --release -p continuum-bench
  '

cp "$ROOT/target/release/continuum-bench" "$OUT"
chmod +x "$OUT"
test -f "$OUT"
echo "Built $OUT"
