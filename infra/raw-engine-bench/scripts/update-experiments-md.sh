#!/usr/bin/env bash
# Patch EXPERIMENTS.md raw engine table from results/*.json
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
MD="$REPO_ROOT/continuum-bench/EXPERIMENTS.md"
RES="$ROOT/results"

read_json() {
  local f="$1" key="$2"
  python3 -c "import json; d=json.load(open('$f')); v=d.get('$key'); print(v if v is not None else '?')"
}

fmt_row() {
  local engine="$1" test="$2" id="$3" vs="$4"
  local j="$RES/${id}.json"
  if [[ ! -f "$j" ]]; then
    echo "| $engine | $test | — | — | — | $vs |"
    return
  fi
  local ops threads p95
  ops="$(read_json "$j" peak_ops_per_sec)"
  threads="$(read_json "$j" threads_at_peak)"
  p95="$(read_json "$j" p95_ms)"
  if [[ "$ops" != "?" ]]; then
    ops="$(python3 -c "ops='$ops'; print(f'{float(ops):,.0f}')")"
  fi
  if [[ "$p95" != "?" ]]; then
    p95="$(python3 -c "p95='$p95'; print(f'{float(p95):.2f}')")"
  fi
  echo "| $engine | $test | $ops | $threads | $p95 | $vs |"
}

BLOCK="$(cat <<EOF
### Raw engine max throughput (July 2026)

**Tooling:** \`cassandra-stress\` (Scylla host) and \`go-ycsb\` raw mode (TiKV host). Same Docker config as Phase A native-lab (\`--smp 1 --memory 750M\` Scylla). Test A = spread keys + auto threads (≤512); Test B = single partition/key + same thread ramp. Runs detached on EC2 via [\`infra/raw-engine-bench/\`](../infra/raw-engine-bench/).

| Engine | Test | Max ops/s | Threads at peak | p95 ms | vs Continuum hot stream |
| ------ | ---- | --------- | --------------- | ------ | ----------------------- |
$(fmt_row Scylla "A spread" scylla-a "—")
$(fmt_row Scylla "B single-key" scylla-b "vs 64/s")
$(fmt_row TiKV "A spread" tikv-a "—")
$(fmt_row TiKV "B single-key" tikv-b "vs 45/s")

EOF
)"

python3 - "$MD" "$BLOCK" <<'PY'
import sys, re
path, block = sys.argv[1], sys.argv[2]
text = open(path).read()
marker = "### Raw engine max throughput"
if marker in text:
    text = re.sub(
        r"### Raw engine max throughput \(July 2026\)\n.*?(?=\n---\n|\n### |\n## )",
        block.rstrip() + "\n\n",
        text,
        count=1,
        flags=re.DOTALL,
    )
else:
    anchor = "**Pending:** `partition-campaign`"
    if anchor not in text:
        sys.exit("anchor not found in EXPERIMENTS.md")
    text = text.replace(
        anchor,
        block.rstrip() + "\n\n" + anchor,
        1,
    )
open(path, "w").write(text)
print("Updated", path)
PY
