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

REMOTE_BASE_DIR="${REMOTE_BASE_DIR:-/home/${SSH_USER}/apps}"
REMOTE_PROJECT_ROOT="${REMOTE_BASE_DIR}/${PROJECT_NAME}"
DEPLOY_ROOT="/opt/${PROJECT_NAME}"
TEMP_CERT_SITE="${PROJECT_NAME}-temp-cert"

remote_exec() {
  ssh -o StrictHostKeyChecking=no "${SSH_USER}@${SERVER_IP}" "$@"
}

echo "[setup] installing base packages"
remote_exec "sudo apt-get update && sudo apt-get upgrade -y"
remote_exec "sudo apt-get install -y build-essential pkg-config libssl-dev curl git rsync nginx certbot python3-certbot-nginx ufw fail2ban"

echo "[setup] configuring firewall"
remote_exec "sudo ufw default deny incoming"
remote_exec "sudo ufw default allow outgoing"
remote_exec "sudo ufw allow 22/tcp"
remote_exec "sudo ufw allow 80/tcp"
remote_exec "sudo ufw allow 443/tcp"
remote_exec "sudo ufw --force enable"

echo "[setup] enabling fail2ban"
remote_exec "sudo systemctl enable fail2ban && sudo systemctl start fail2ban"

echo "[setup] hardening SSH"
remote_exec "sudo sed -i 's/^#*PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config"
remote_exec "sudo sed -i 's/^#*ChallengeResponseAuthentication.*/ChallengeResponseAuthentication no/' /etc/ssh/sshd_config"
remote_exec "sudo sed -i 's/^#*PubkeyAuthentication.*/PubkeyAuthentication yes/' /etc/ssh/sshd_config"
remote_exec "sudo sed -i 's/^#*PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config"
remote_exec "sudo sshd -t"
if remote_exec "sudo systemctl list-unit-files | grep -q '^ssh.service'"; then
  remote_exec "sudo systemctl restart ssh"
else
  remote_exec "sudo systemctl restart sshd"
fi

if ! remote_exec "command -v cargo >/dev/null 2>&1"; then
  echo "[setup] installing rust"
  remote_exec "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
fi

if ! remote_exec "command -v sccache >/dev/null 2>&1"; then
  echo "[setup] installing sccache"
  remote_exec "SCCACHE_VER=\$(curl -fsSL -o /dev/null -w '%{url_effective}' https://github.com/mozilla/sccache/releases/latest | grep -o 'v[0-9.]*\$') && curl -fsSL \"https://github.com/mozilla/sccache/releases/download/\${SCCACHE_VER}/sccache-\${SCCACHE_VER}-x86_64-unknown-linux-musl.tar.gz\" | tar xz -C /tmp && sudo mv /tmp/sccache-\${SCCACHE_VER}-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache && rm -rf /tmp/sccache-\${SCCACHE_VER}-x86_64-unknown-linux-musl"
fi

DB_PASSWORD=""
if [ "${DB_KIND:-}" = "postgres" ]; then
  echo "[setup] installing postgresql"
  remote_exec "sudo apt-get install -y postgresql postgresql-contrib"
  remote_exec "sudo systemctl enable postgresql && sudo systemctl start postgresql"
  echo "[setup] creating postgres role and database"
  remote_exec "sudo -u postgres createuser --no-superuser --no-createdb --no-createrole ${SSH_USER} 2>/dev/null || true"
  remote_exec "sudo -u postgres createdb -O ${SSH_USER} ${PROJECT_NAME} 2>/dev/null || true"
  echo "[setup] setting postgres role password"
  DB_PASSWORD=$(LC_ALL=C tr -dc 'A-Za-z0-9' < /dev/urandom | head -c 24)
  remote_exec "sudo -u postgres psql -c \"ALTER ROLE ${SSH_USER} PASSWORD '${DB_PASSWORD}';\""
  echo "[setup] postgres ready"
fi

remote_exec "mkdir -p ${REMOTE_PROJECT_ROOT}"
remote_exec "sudo mkdir -p ${DEPLOY_ROOT}/gui ${DEPLOY_ROOT}/admin-gui"
remote_exec "sudo chown -R ${SSH_USER}:${SSH_USER} ${DEPLOY_ROOT}"

echo "[setup] writing server.env"
{
  printf 'BACKEND_HOST=%s\n' "${BACKEND_HOST:-127.0.0.1}"
  printf 'BACKEND_PORT=%s\n' "${BACKEND_PORT:-8080}"
  if [ -n "${DB_PASSWORD}" ]; then
    printf 'DATABASE_URL=postgresql://%s:%s@localhost/%s\n' "${SSH_USER}" "${DB_PASSWORD}" "${PROJECT_NAME}"
  else
    printf 'DATABASE_URL=app.db\n'
  fi
} | ssh -o StrictHostKeyChecking=no "${SSH_USER}@${SERVER_IP}" "cat > /tmp/server.env"
remote_exec "sudo mv /tmp/server.env ${DEPLOY_ROOT}/server.env && sudo chown ${SSH_USER}:${SSH_USER} ${DEPLOY_ROOT}/server.env && chmod 600 ${DEPLOY_ROOT}/server.env"

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
