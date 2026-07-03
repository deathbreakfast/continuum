#!/usr/bin/env bash
# Start nginx LB for Surreal upstreams (host network, private IPs).
# SURREAL_IPS: space-separated list of surreal node private IPs.
set -euo pipefail

SURREAL_IPS="${SURREAL_IPS:?}"
NGINX_CONF="/var/lib/continuum/nginx-surreal-lb.conf"
sudo mkdir -p /var/lib/continuum

{
  echo "events { worker_connections 1024; }"
  echo "http {"
  echo "    upstream surreal_upstream {"
  for ip in $SURREAL_IPS; do
    echo "        server ${ip}:8000;"
  done
  echo "    }"
  echo "    server {"
  echo "        listen 8000;"
  echo "        location / {"
  echo "            proxy_pass http://surreal_upstream;"
  echo "            proxy_http_version 1.1;"
  echo "            proxy_set_header Upgrade \$http_upgrade;"
  echo "            proxy_set_header Connection \"upgrade\";"
  echo "            proxy_set_header Host \$host;"
  echo "        }"
  echo "    }"
  echo "}"
} | sudo tee "$NGINX_CONF" >/dev/null

sudo docker rm -f continuum-surreal-lb 2>/dev/null || true

sudo docker run -d --name continuum-surreal-lb --restart unless-stopped --network host \
  -v "${NGINX_CONF}:/etc/nginx/nginx.conf:ro" \
  "${NGINX_IMAGE:?}"

echo "Surreal LB started on :8000 upstreams: $SURREAL_IPS"
