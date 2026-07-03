#!/usr/bin/env bash
# Create private S3 bucket for continuum-bench AL2023 binary artifacts.
# Usage: setup-artifact-bucket.sh [bucket-name]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/config/defaults.env"

REGION="${CONTINUUM_NATIVE_AWS_REGION}"
ACCOUNT_ID="$(aws sts get-caller-identity --query Account --output text)"
BUCKET="${1:-continuum-bench-artifacts-${ACCOUNT_ID}}"

if aws s3api head-bucket --bucket "$BUCKET" 2>/dev/null; then
  echo "Bucket already exists: $BUCKET"
else
  if [[ "$REGION" == "us-east-1" ]]; then
    aws s3api create-bucket --bucket "$BUCKET" --region "$REGION"
  else
    aws s3api create-bucket --bucket "$BUCKET" --region "$REGION" \
      --create-bucket-configuration "LocationConstraint=$REGION"
  fi
  echo "Created bucket: $BUCKET"
fi

aws s3api put-public-access-block --bucket "$BUCKET" \
  --public-access-block-configuration \
  BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true

aws s3api put-bucket-lifecycle-configuration --bucket "$BUCKET" --lifecycle-configuration '{
  "Rules": [{
    "ID": "expire-al2023-artifacts",
    "Status": "Enabled",
    "Filter": { "Prefix": "al2023/" },
    "Expiration": { "Days": 30 }
  }]
}' 2>/dev/null || true

echo "CONTINUUM_NATIVE_ARTIFACT_BUCKET=$BUCKET"
echo "Add to infra/native-aws/config/defaults.env or export before build/deploy."
