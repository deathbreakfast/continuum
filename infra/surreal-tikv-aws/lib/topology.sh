#!/usr/bin/env bash
# Parse topology YAML into shell-exportable variables via python3 (no PyYAML).
set -euo pipefail

topology_parse() {
  local yaml_file="$1"
  local arch="${2:-x86}"

  if [[ ! -f "$yaml_file" ]]; then
    echo "topology file not found: $yaml_file" >&2
    return 1
  fi

  python3 - "$yaml_file" "$arch" <<'PY'
import sys, re

path, arch = sys.argv[1], sys.argv[2]

def parse_yaml_simple(text):
    root = {}
    stack = [(0, root)]
    for raw in text.splitlines():
        if not raw.strip() or raw.strip().startswith("#"):
            continue
        indent = len(raw) - len(raw.lstrip(" "))
        line = raw.strip()
        key, _, val = line.partition(":")
        key = key.strip()
        val = val.strip()
        while stack and indent < stack[-1][0]:
            stack.pop()
        parent = stack[-1][1]
        if val == "":
            node = {}
            parent[key] = node
            stack.append((indent + 2, node))
        else:
            if val.lower() in ("true", "false"):
                parent[key] = val.lower() == "true"
            elif val.isdigit():
                parent[key] = int(val)
            else:
                parent[key] = val.strip('"').strip("'")
    return root

with open(path) as f:
    t = parse_yaml_simple(f.read())

def emit(k, v):
    if isinstance(v, bool):
        v = "true" if v else "false"
    elif v is None:
        v = ""
    print(f"export TOPO_{k}={v!r}")

emit("NAME", t["name"])
emit("BENCH_TOPOLOGY", t["bench_topology"])
emit("SURREAL_INSTANCES", t.get("surreal_instances", t.get("surreal", {}).get("count", 1)))

pd = t.get("pd", {})
emit("PD_COLOCATE_WITH", pd.get("colocate_with", "tikv-0"))
emit("PD_STANDALONE", pd.get("standalone", False))

tikv = t["tikv"]
emit("TIKV_COUNT", tikv["count"])
bench = t["bench"]
surreal = t["surreal"]

tikv_type = tikv["instance_type"]
bench_type = bench["instance_type"]
surreal_type = surreal["instance_type"]

if arch == "arm":
    def armify(typ):
        return ("t4g." + typ[3:]) if typ.startswith("t3.") else typ
    tikv_type = armify(tikv_type)
    bench_type = armify(bench_type)
    surreal_type = armify(surreal_type)
    hw = t.get("component_hardware", {})
    tikv_hw = hw.get("tikv", "aws-t3-small").replace("aws-t3-", "aws-t4g-")
    surreal_hw = hw.get("surreal", "aws-t3-small").replace("aws-t3-", "aws-t4g-")
    emit("COMPONENT_TIKV_HW", tikv_hw)
    emit("COMPONENT_SURREAL_HW", surreal_hw)
    emit("BENCH_HW_DEFAULT", bench_type.replace("t4g.", "aws-t4g-").replace("t3.", "aws-t3-"))
else:
    emit("TIKV_TYPE", tikv_type)
    emit("BENCH_TYPE", bench_type)
    emit("SURREAL_TYPE", surreal_type)
    hw = t.get("component_hardware", {})
    emit("COMPONENT_TIKV_HW", hw.get("tikv", "aws-t3-small"))
    emit("COMPONENT_SURREAL_HW", hw.get("surreal", "aws-t3-small"))
    emit("BENCH_HW_DEFAULT", bench_type.replace("t3.", "aws-t3-").replace("t4g.", "aws-t4g-"))

if arch == "arm":
    emit("TIKV_TYPE", tikv_type)
    emit("BENCH_TYPE", bench_type)
    emit("SURREAL_TYPE", surreal_type)

emit("SURREAL_COUNT", surreal["count"])
emit("SURREAL_LB", surreal.get("load_balancer", False))
PY
}

topology_yaml_path() {
  local name="$1"
  echo "$CONTINUUM_TIKV_AWS_ROOT/topologies/${name}.yaml"
}
