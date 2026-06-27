#!/usr/bin/env bash
# Run SQL adapter benchmark subset (sqlite + postgres) for one hardware profile.
set -euo pipefail

HARDWARE="${1:-dev-wsl}"
BENCH="${CONTINUUM_BENCH_BIN:-cargo run --release -p continuum-bench --}"

if [[ -z "${CONTINUUM_BENCH_POSTGRES_URL:-}" ]]; then
  echo "CONTINUUM_BENCH_POSTGRES_URL not set — postgres runs will be omitted from matrix"
fi

export CONTINUUM_BENCH_REPORTS_DIR="${CONTINUUM_BENCH_REPORTS_DIR:-$(cd "$(dirname "$0")/../../profiling/continuum-bench/reports" && pwd)}"

echo "SQL matrix hardware=$HARDWARE reports=$CONTINUUM_BENCH_REPORTS_DIR"

eval "$BENCH matrix --hardware \"$HARDWARE\" --subset sql --skip-existing"

echo "SQL_MATRIX_DONE"
