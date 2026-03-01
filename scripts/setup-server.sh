#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CONFIG_FILE="${1:-${PROJECT_ROOT}/project.conf}"

if [ ! -f "$CONFIG_FILE" ]; then
  echo "Config not found: $CONFIG_FILE"
  echo "Create it from project.conf.template"
  exit 1
fi

# shellcheck disable=SC1090
source "$CONFIG_FILE"

required_vars=(PROJECT_NAME SERVER_IP SSH_USER DOMAIN_NAME LETSENCRYPT_EMAIL)
for v in "${required_vars[@]}"; do
  if [ -z "${!v:-}" ]; then
    echo "Missing required config key: $v"
    exit 1
  fi
done

REMOTE_BASE_DIR="${REMOTE_BASE_DIR:-~/apps}"
REMOTE_PROJECT_ROOT="${REMOTE_BASE_DIR}/${PROJECT_NAME}"
DEPLOY_ROOT="/opt/${PROJECT_NAME}"
TEMP_CERT_SITE="${PROJECT_NAME}-temp-cert"

remote_exec() {
  ssh -o StrictHostKeyChecking=no "${SSH_USER}@${SERVER_IP}" "$@"
}

echo "[setup] installing base packages"
remote_exec "sudo apt-get update"
remote_exec "sudo apt-get install -y build-essential pkg-config libssl-dev curl git nginx certbot python3-certbot-nginx"

if ! remote_exec "command -v cargo >/dev/null 2>&1"; then
  echo "[setup] installing rust"
  remote_exec "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
fi

remote_exec "mkdir -p ${REMOTE_PROJECT_ROOT}"
remote_exec "sudo mkdir -p ${DEPLOY_ROOT}/gui"
remote_exec "sudo chown -R ${SSH_USER}:${SSH_USER} ${DEPLOY_ROOT}"

echo "[setup] upload certbot nginx bootstrap template"
scp -o StrictHostKeyChecking=no "${PROJECT_ROOT}/scripts/configs/nginx-temp-cert.conf.template" "${SSH_USER}@${SERVER_IP}:/tmp/nginx-temp-cert.conf.template"

echo "[setup] bootstrap nginx for certbot"
remote_exec "sed 's|{{DOMAIN_NAME}}|${DOMAIN_NAME}|g' /tmp/nginx-temp-cert.conf.template > /tmp/nginx-temp-cert.conf"
remote_exec "rm -f /tmp/nginx-temp-cert.conf.template"
remote_exec "sudo mv /tmp/nginx-temp-cert.conf /etc/nginx/sites-available/${TEMP_CERT_SITE}"
remote_exec "sudo ln -sf /etc/nginx/sites-available/${TEMP_CERT_SITE} /etc/nginx/sites-enabled/${TEMP_CERT_SITE}"
remote_exec "sudo rm -f /etc/nginx/sites-enabled/default"
remote_exec "sudo nginx -t"
remote_exec "sudo systemctl restart nginx"

echo "[setup] requesting or reusing letsencrypt certificate"
if remote_exec "sudo test -f /etc/letsencrypt/live/${DOMAIN_NAME}/fullchain.pem"; then
  echo "[setup] certificate already present for ${DOMAIN_NAME}"
else
  remote_exec "sudo certbot --nginx -d ${DOMAIN_NAME} --non-interactive --agree-tos -m ${LETSENCRYPT_EMAIL} --redirect"
fi

remote_exec "sudo systemctl enable certbot.timer"
remote_exec "sudo systemctl start certbot.timer"

echo "[setup] done"
