#!/usr/bin/env bash
# Launch multi-node topology from topologies/*.yaml
# Usage: provision-topology.sh <topology-name>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"

TOPO="${1:?usage: provision-topology.sh native-scylla-3n|native-tikv-ha-3|native-tikv-scale-5}"
YAML="$ROOT/topologies/${TOPO}.yaml"
[[ -f "$YAML" ]] || { echo "missing $YAML" >&2; exit 1; }

if [[ ! -f "$CONTINUUM_NATIVE_AWS_KEY_PATH" ]]; then
  echo "SSH key not found: $CONTINUUM_NATIVE_AWS_KEY_PATH" >&2
  exit 1
fi

AMI_ID="$(aws ec2 describe-images --region "$CONTINUUM_NATIVE_AWS_REGION" --owners amazon \
  --filters $CONTINUUM_NATIVE_AMI_X86_FILTER \
  --query 'sort_by(Images,&CreationDate)[-1].ImageId' --output text)"

SG_ID="$(aws ec2 describe-security-groups --region "$CONTINUUM_NATIVE_AWS_REGION" \
  --filters "Name=group-name,Values=$CONTINUUM_NATIVE_AWS_SG_NAME" \
  --query 'SecurityGroups[0].GroupId' --output text 2>/dev/null || true)"

# shellcheck disable=SC2015
[[ -n "$SG_ID" && "$SG_ID" != "None" ]] || { echo "run provision-colocated.sh first to create SG" >&2; exit 1; }

export TOPO YAML AMI_ID SG_ID CONTINUUM_NATIVE_AWS_REGION CONTINUUM_NATIVE_AWS_INSTANCE_TYPE
export CONTINUUM_NATIVE_AWS_KEY_NAME CONTINUUM_NATIVE_AWS_PROJECT_TAG CONTINUUM_NATIVE_AWS_EBS_GB

python3 - <<'PY' | manifest_write "$TOPO" "$(cat)"
import json, os, subprocess, sys, time

yaml_path = os.environ["YAML"]
topo = os.environ["TOPO"]
region = os.environ["CONTINUUM_NATIVE_AWS_REGION"]
ami = os.environ["AMI_ID"]
sg = os.environ["SG_ID"]
itype = os.environ["CONTINUUM_NATIVE_AWS_INSTANCE_TYPE"]
key = os.environ["CONTINUUM_NATIVE_AWS_KEY_NAME"]
tag = os.environ["CONTINUUM_NATIVE_AWS_PROJECT_TAG"]
ebs = os.environ["CONTINUUM_NATIVE_AWS_EBS_GB"]

def parse_yaml(path):
    roles = []
    cur, count = None, 1
    bench_topology = topo
    for line in open(path):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("bench_topology:"):
            bench_topology = line.split(":", 1)[1].strip()
        elif line.startswith("- role:"):
            if cur:
                roles.append((cur, count))
            cur = line.split(":", 1)[1].strip()
            count = 1
        elif line.startswith("count:"):
            count = int(line.split(":", 1)[1].strip())
    if cur:
        roles.append((cur, count))
    return bench_topology, roles

def launch(role, index):
    name = f"{topo}-{role}-{index}"
    out = subprocess.check_output([
        "aws", "ec2", "run-instances",
        "--region", region,
        "--image-id", ami,
        "--instance-type", itype,
        "--key-name", key,
        "--security-group-ids", sg,
        "--block-device-mappings", json.dumps([{
            "DeviceName": "/dev/xvda",
            "Ebs": {"VolumeSize": int(ebs), "VolumeType": "gp3", "DeleteOnTermination": True},
        }]),
        "--tag-specifications", json.dumps([{
            "ResourceType": "instance",
            "Tags": [
                {"Key": "Name", "Value": name},
                {"Key": "Project", "Value": tag},
                {"Key": "Role", "Value": role},
                {"Key": "Topology", "Value": topo},
            ],
        }]),
        "--query", "Instances[0].InstanceId",
        "--output", "text",
    ], text=True).strip()
    return out

def wait_ips(instance_id):
    for _ in range(60):
        row = subprocess.check_output([
            "aws", "ec2", "describe-instances",
            "--region", region,
            "--instance-ids", instance_id,
            "--query", "Reservations[0].Instances[0].[PublicIpAddress,PrivateIpAddress,State.Name]",
            "--output", "text",
        ], text=True).split()
        if len(row) >= 3 and row[2] == "running" and row[0] != "None":
            return row[0], row[1]
        time.sleep(5)
    raise RuntimeError(f"timeout waiting for {instance_id}")

bench_topology, roles = parse_yaml(yaml_path)
instances = []
for role, n in roles:
    for idx in range(n):
        iid = launch(role, idx)
        pub, priv = wait_ips(iid)
        instances.append({
            "role": role,
            "index": idx,
            "instance_id": iid,
            "public_ip": pub,
            "private_ip": priv,
        })

print(json.dumps({
    "topology": topo,
    "bench_topology": bench_topology,
    "region": region,
    "instance_type": itype,
    "instances": instances,
}, indent=2))
PY

echo "Provisioned topology $TOPO:"
manifest_read "$TOPO" | python3 -m json.tool
