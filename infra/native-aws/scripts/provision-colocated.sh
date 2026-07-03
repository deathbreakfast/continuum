#!/usr/bin/env bash
# Launch colocated t3.medium instance(s) for Phase A native benchmarks.
# Usage: provision-colocated.sh [--storage scylla|tikv|both]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CONTINUUM_NATIVE_AWS_ROOT="$ROOT"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"
# shellcheck disable=SC1091
source "$ROOT/lib/manifest.sh"

STORAGE="${1:-both}"
case "$STORAGE" in
  scylla|tikv|both) ;;
  --storage)
    STORAGE="${2:-both}"
    ;;
  *)
    echo "usage: $0 [--storage scylla|tikv|both]" >&2
    exit 1
    ;;
esac

if [[ ! -f "$CONTINUUM_NATIVE_AWS_KEY_PATH" ]]; then
  echo "SSH key not found: $CONTINUUM_NATIVE_AWS_KEY_PATH" >&2
  exit 1
fi

MY_IP="$(curl -sf https://checkip.amazonaws.com || true)"
if [[ -z "$MY_IP" ]]; then
  echo "could not detect public IP for security group" >&2
  exit 1
fi

AMI_ID="$(aws ec2 describe-images \
  --region "$CONTINUUM_NATIVE_AWS_REGION" \
  --owners amazon \
  --filters $CONTINUUM_NATIVE_AMI_X86_FILTER \
  --query 'sort_by(Images,&CreationDate)[-1].ImageId' \
  --output text)"

SG_ID="$(aws ec2 describe-security-groups \
  --region "$CONTINUUM_NATIVE_AWS_REGION" \
  --filters "Name=group-name,Values=$CONTINUUM_NATIVE_AWS_SG_NAME" \
  --query 'SecurityGroups[0].GroupId' \
  --output text 2>/dev/null || true)"

if [[ -z "$SG_ID" || "$SG_ID" == "None" ]]; then
  SG_ID="$(aws ec2 create-security-group \
    --region "$CONTINUUM_NATIVE_AWS_REGION" \
    --group-name "$CONTINUUM_NATIVE_AWS_SG_NAME" \
    --description "continuum native-aws bench" \
    --query GroupId --output text)"
  aws ec2 authorize-security-group-ingress \
    --region "$CONTINUUM_NATIVE_AWS_REGION" \
    --group-id "$SG_ID" \
    --protocol tcp --port 22 --cidr "${MY_IP}/32"
  # bench -> storage on private IPs within VPC (same SG members)
  aws ec2 authorize-security-group-ingress \
    --region "$CONTINUUM_NATIVE_AWS_REGION" \
    --group-id "$SG_ID" \
    --protocol tcp --port 9042 --source-group "$SG_ID"
  aws ec2 authorize-security-group-ingress \
    --region "$CONTINUUM_NATIVE_AWS_REGION" \
    --group-id "$SG_ID" \
    --protocol tcp --port 2379 --source-group "$SG_ID"
  aws ec2 authorize-security-group-ingress \
    --region "$CONTINUUM_NATIVE_AWS_REGION" \
    --group-id "$SG_ID" \
    --protocol tcp --port 20160 --source-group "$SG_ID"
fi

launch_one() {
  local role="$1"
  local name="native-colocated-${role}-$(date +%Y%m%d%H%M%S)"
  local instance_id
  instance_id="$(aws ec2 run-instances \
    --region "$CONTINUUM_NATIVE_AWS_REGION" \
    --image-id "$AMI_ID" \
    --instance-type "$CONTINUUM_NATIVE_AWS_INSTANCE_TYPE" \
    --key-name "$CONTINUUM_NATIVE_AWS_KEY_NAME" \
    --security-group-ids "$SG_ID" \
    --block-device-mappings "[{\"DeviceName\":\"/dev/xvda\",\"Ebs\":{\"VolumeSize\":${CONTINUUM_NATIVE_AWS_EBS_GB},\"VolumeType\":\"gp3\",\"DeleteOnTermination\":true}}]" \
    --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=${name}},{Key=Project,Value=${CONTINUUM_NATIVE_AWS_PROJECT_TAG}},{Key=Role,Value=${role}}]" \
    --query 'Instances[0].InstanceId' --output text)"
  echo "$instance_id"
}

wait_public_ip() {
  local instance_id="$1"
  local ip=""
  for _ in $(seq 1 40); do
    ip="$(aws ec2 describe-instances \
      --region "$CONTINUUM_NATIVE_AWS_REGION" \
      --instance-ids "$instance_id" \
      --query 'Reservations[0].Instances[0].PublicIpAddress' \
      --output text 2>/dev/null || true)"
    if [[ -n "$ip" && "$ip" != "None" ]]; then
      echo "$ip"
      return 0
    fi
    sleep 5
  done
  return 1
}

instances_json="[]"
if [[ "$STORAGE" == "scylla" || "$STORAGE" == "both" ]]; then
  id="$(launch_one scylla)"
  pub="$(wait_public_ip "$id")"
  instances_json="$(python3 - <<PY
import json
print(json.dumps([{"role":"scylla","instance_id":"$id","public_ip":"$pub","private_ip":""}]))
PY
)"
fi
if [[ "$STORAGE" == "tikv" || "$STORAGE" == "both" ]]; then
  id="$(launch_one tikv)"
  pub="$(wait_public_ip "$id")"
  tikv_entry="{\"role\":\"tikv\",\"instance_id\":\"$id\",\"public_ip\":\"$pub\",\"private_ip\":\"\"}"
  if [[ "$STORAGE" == "both" ]]; then
    instances_json="$(python3 - <<PY
import json
a=json.loads('$instances_json')
a.append(json.loads('$tikv_entry'))
print(json.dumps(a))
PY
)"
  else
    instances_json="[$tikv_entry]"
  fi
fi

manifest_write "native-colocated" "$(python3 - <<PY
import json, datetime
print(json.dumps({
  "topology": "native-colocated",
  "region": "$CONTINUUM_NATIVE_AWS_REGION",
  "instance_type": "$CONTINUUM_NATIVE_AWS_INSTANCE_TYPE",
  "created_at": datetime.datetime.utcnow().isoformat() + "Z",
  "instances": json.loads('$instances_json'),
}, indent=2))
PY
)"

echo "Provisioned native-colocated fleet:"
manifest_read native-colocated | python3 -m json.tool
