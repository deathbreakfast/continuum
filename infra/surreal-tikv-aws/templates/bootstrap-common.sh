#!/usr/bin/env bash
# Install Docker on Amazon Linux 2023 and pull pinned images.
set -euo pipefail

if ! command -v docker >/dev/null 2>&1; then
  sudo dnf install -y docker
  sudo systemctl enable --now docker
  sudo usermod -aG docker ec2-user
fi

sudo docker pull "${PD_IMAGE:?}"
sudo docker pull "${TIKV_IMAGE:?}"
sudo docker pull "${SURREAL_IMAGE:?}"

if [[ "${PULL_NGINX:-false}" == "true" ]]; then
  sudo docker pull "${NGINX_IMAGE:?}"
fi

echo "bootstrap-common: docker ready, images pulled"
