#!/usr/bin/env bash
# Track U: BM-M4 @ fixed C=K with storage metrics collection.
# Usage: run-scylla-track-u-campaign.sh <C=K> [hardware]
set -euo pipefail

CK="${1:?C=K value}"
HARDWARE="${2:-aws-t3-medium}"
BENCH="${CONTINUUM_BENCH_BIN:-continuum-bench}"
STORAGE="${CONTINUUM_BENCH_STORAGE:-scylla}"

export CONTINUUM_BENCH_CLIENT_COUNT="$CK"
export CONTINUUM_BENCH_PARTITION_COUNT="$CK"
export CONTINUUM_BENCH_REPORTS_DIR="${CONTINUUM_BENCH_REPORTS_DIR:-$HOME/continuum-bench/reports}"

echo "Track U: BM-M4 C=K=$CK hardware=$HARDWARE storage=$STORAGE"
"$BENCH" run bm-m4 --storage "$STORAGE" --hardware "$HARDWARE"

unset CONTINUUM_BENCH_CLIENT_COUNT CONTINUUM_BENCH_PARTITION_COUNT
echo "TRACK_U_DONE C=K=$CK" >~/continuum-bench/campaign-track-u.done
