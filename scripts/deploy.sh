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

required_vars=(PROJECT_NAME SERVER_IP SSH_USER DOMAIN_NAME)
for v in "${required_vars[@]}"; do
  if [ -z "${!v:-}" ]; then
    echo "Missing required config key: $v"
    exit 1
  fi
done

REMOTE_BASE_DIR="${REMOTE_BASE_DIR:-/home/${SSH_USER}/apps}"
BACKEND_PORT="${BACKEND_PORT:-8080}"
REMOTE_ROOT="${REMOTE_BASE_DIR}/${PROJECT_NAME}"
DEPLOY_ROOT="/opt/${PROJECT_NAME}"
BACKEND_BIN="${PROJECT_NAME}-backend"
MIGRATE_BIN="migrate"
SERVICE_NAME="${PROJECT_NAME}-backend"
NGINX_SITE_NAME="${PROJECT_NAME}"
TEMP_CERT_SITE="${PROJECT_NAME}-temp-cert"

remote_exec() {
  ssh -o StrictHostKeyChecking=no "${SSH_USER}@${SERVER_IP}" "$@"
}

echo "[deploy] build gui locally"
cd "$PROJECT_ROOT/gui"
npm install
npm run build

echo "[deploy] build admin-gui locally"
cd "$PROJECT_ROOT/admin-gui"
npm install
npm run build

echo "[deploy] sync source tree"
remote_exec "command -v rsync >/dev/null 2>&1 || sudo apt-get install -y rsync"
remote_exec "mkdir -p ${REMOTE_ROOT}"
rsync -az --delete \
  --exclude='target/' \
  --exclude='gui/node_modules/' \
  --exclude='gui/dist/' \
  --exclude='admin-gui/node_modules/' \
  --exclude='admin-gui/dist/' \
  --exclude='.git/' \
  --exclude='.DS_Store' \
  -e "ssh -o StrictHostKeyChecking=no" \
  "$PROJECT_ROOT/" \
  "${SSH_USER}@${SERVER_IP}:${REMOTE_ROOT}/"

echo "[deploy] build backend on server"
remote_exec "command -v sccache >/dev/null 2>&1 || (SCCACHE_VER=\$(curl -fsSL -o /dev/null -w '%{url_effective}' https://github.com/mozilla/sccache/releases/latest | grep -o 'v[0-9.]*\$') && curl -fsSL \"https://github.com/mozilla/sccache/releases/download/\${SCCACHE_VER}/sccache-\${SCCACHE_VER}-x86_64-unknown-linux-musl.tar.gz\" | tar xz -C /tmp && sudo mv /tmp/sccache-\${SCCACHE_VER}-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache && rm -rf /tmp/sccache-\${SCCACHE_VER}-x86_64-unknown-linux-musl)"
remote_exec "cd ${REMOTE_ROOT} && source ~/.cargo/env && RUSTC_WRAPPER=sccache cargo build --release -p ${BACKEND_BIN} --bin ${BACKEND_BIN} --bin ${MIGRATE_BIN}"

echo "[deploy] install backend binary"
remote_exec "sudo mkdir -p ${DEPLOY_ROOT}"
remote_exec "sudo systemctl stop ${SERVICE_NAME} 2>/dev/null || true"
remote_exec "sudo cp ${REMOTE_ROOT}/target/release/${BACKEND_BIN} ${DEPLOY_ROOT}/${BACKEND_BIN}"
remote_exec "sudo cp ${REMOTE_ROOT}/target/release/${MIGRATE_BIN} ${DEPLOY_ROOT}/${MIGRATE_BIN}"
remote_exec "sudo chmod +x ${DEPLOY_ROOT}/${BACKEND_BIN}"
remote_exec "sudo chmod +x ${DEPLOY_ROOT}/${MIGRATE_BIN}"
remote_exec "sudo chown ${SSH_USER}:${SSH_USER} ${DEPLOY_ROOT}/${BACKEND_BIN}"
remote_exec "sudo chown ${SSH_USER}:${SSH_USER} ${DEPLOY_ROOT}/${MIGRATE_BIN}"

echo "[deploy] run database migrations"
remote_exec "bash -c 'set -a; . ${DEPLOY_ROOT}/server.env; cd ${DEPLOY_ROOT}; ./${MIGRATE_BIN}'"

echo "[deploy] upload gui dist"
remote_exec "sudo rm -rf ${DEPLOY_ROOT}/gui/* && sudo mkdir -p ${DEPLOY_ROOT}/gui"
scp -o StrictHostKeyChecking=no -r "$PROJECT_ROOT/gui/dist/"* "${SSH_USER}@${SERVER_IP}:/tmp/${PROJECT_NAME}-gui-dist/"
remote_exec "sudo mv /tmp/${PROJECT_NAME}-gui-dist/* ${DEPLOY_ROOT}/gui/ && rmdir /tmp/${PROJECT_NAME}-gui-dist"
remote_exec "sudo chown -R ${SSH_USER}:${SSH_USER} ${DEPLOY_ROOT}/gui"

echo "[deploy] upload admin-gui dist"
remote_exec "sudo rm -rf ${DEPLOY_ROOT}/admin-gui/* && sudo mkdir -p ${DEPLOY_ROOT}/admin-gui"
scp -o StrictHostKeyChecking=no -r "$PROJECT_ROOT/admin-gui/dist/"* "${SSH_USER}@${SERVER_IP}:/tmp/${PROJECT_NAME}-admin-gui-dist/"
remote_exec "sudo mv /tmp/${PROJECT_NAME}-admin-gui-dist/* ${DEPLOY_ROOT}/admin-gui/ && rmdir /tmp/${PROJECT_NAME}-admin-gui-dist"
remote_exec "sudo chown -R ${SSH_USER}:${SSH_USER} ${DEPLOY_ROOT}/admin-gui"

echo "[deploy] install systemd service"
remote_exec "sed -e 's|{{SSH_USER}}|${SSH_USER}|g' -e 's|{{PROJECT_NAME}}|${PROJECT_NAME}|g' ${REMOTE_ROOT}/scripts/configs/backend.service.template > /tmp/${SERVICE_NAME}.service"
remote_exec "sudo mv /tmp/${SERVICE_NAME}.service /etc/systemd/system/${SERVICE_NAME}.service"
remote_exec "sudo systemctl daemon-reload"
remote_exec "sudo systemctl enable ${SERVICE_NAME}"
remote_exec "sudo systemctl restart ${SERVICE_NAME}"

echo "[deploy] install nginx config"
remote_exec "sed -e 's|{{DOMAIN_NAME}}|${DOMAIN_NAME}|g' -e 's|{{PROJECT_NAME}}|${PROJECT_NAME}|g' -e 's|{{BACKEND_PORT}}|${BACKEND_PORT}|g' ${REMOTE_ROOT}/scripts/configs/nginx.conf.template > /tmp/${NGINX_SITE_NAME}.conf"
remote_exec "sudo mv /tmp/${NGINX_SITE_NAME}.conf /etc/nginx/sites-available/${NGINX_SITE_NAME}"
remote_exec "sudo ln -sf /etc/nginx/sites-available/${NGINX_SITE_NAME} /etc/nginx/sites-enabled/${NGINX_SITE_NAME}"
remote_exec "sudo rm -f /etc/nginx/sites-enabled/default"
remote_exec "sudo rm -f /etc/nginx/sites-enabled/${TEMP_CERT_SITE}"
remote_exec "sudo nginx -t"
remote_exec "sudo systemctl restart nginx"

echo "[deploy] ensure tls cert and renew timer"
if remote_exec "sudo test -f /etc/letsencrypt/live/${DOMAIN_NAME}/fullchain.pem"; then
  echo "[deploy] tls cert present"
else
  echo "[deploy] tls cert missing; run setup-server.sh first"
  exit 1
fi

if remote_exec "test -f ${DEPLOY_ROOT}/server.env"; then
  echo "[deploy] server.env present"
else
  echo "[deploy] server.env missing; run setup-server.sh first"
  exit 1
fi
remote_exec "sudo systemctl enable certbot.timer"
remote_exec "sudo systemctl start certbot.timer"

echo "[deploy] done"
echo "[deploy] backend: systemctl status ${SERVICE_NAME}"
echo "[deploy] app url: https://${DOMAIN_NAME}"
