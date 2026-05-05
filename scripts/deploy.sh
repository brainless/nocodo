#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CONFIG_FILE="${1:-${PROJECT_ROOT}/project.toml}"

if [ ! -f "$CONFIG_FILE" ]; then
  echo "Config not found: $CONFIG_FILE"
  echo "Create it from project.toml.template"
  exit 1
fi

# Read a value from a TOML file: toml_get <file> <section> <key>
toml_get() {
  python3 - "$1" "$2" "$3" <<'PYEOF'
import sys

file, section, key = sys.argv[1], sys.argv[2], sys.argv[3]

try:
    import tomllib
    with open(file, "rb") as f:
        data = tomllib.load(f)
    val = data.get(section, {}).get(key)
    if val is not None:
        print(val)
    sys.exit(0)
except ImportError:
    pass

# Fallback for Python < 3.11
in_section = False
with open(file) as f:
    for line in f:
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        if line.startswith('[') and line.endswith(']'):
            in_section = (line[1:-1].strip() == section)
            continue
        if in_section and '=' in line:
            k, _, v = line.partition('=')
            if k.strip() == key:
                v = v.strip().strip('"').strip("'").split('#')[0].strip()
                print(v)
                break
PYEOF
}

PROJECT_NAME="$(toml_get "$CONFIG_FILE" project name)"
PROJECT_TITLE="$(toml_get "$CONFIG_FILE" project title)"
SERVER_IP="$(toml_get "$CONFIG_FILE" deploy server_ip)"
SSH_USER="$(toml_get "$CONFIG_FILE" deploy ssh_user)"
DOMAIN_NAME="$(toml_get "$CONFIG_FILE" deploy domain_name)"
LETSENCRYPT_EMAIL="$(toml_get "$CONFIG_FILE" deploy letsencrypt_email || true)"
REMOTE_BASE_DIR="$(toml_get "$CONFIG_FILE" deploy remote_base_dir || true)"
REMOTE_BASE_DIR="${REMOTE_BASE_DIR:-/home/${SSH_USER}/apps}"
BACKEND_PORT="$(toml_get "$CONFIG_FILE" server port)"
BACKEND_PORT="${BACKEND_PORT:-8080}"
GUI_PORT="$(toml_get "$CONFIG_FILE" gui port || true)"; GUI_PORT="${GUI_PORT:-3030}"
ADMIN_GUI_PORT="$(toml_get "$CONFIG_FILE" admin_gui port || true)"; ADMIN_GUI_PORT="${ADMIN_GUI_PORT:-3031}"
DB_KIND="$(toml_get "$CONFIG_FILE" database kind || true)"; DB_KIND="${DB_KIND:-sqlite}"
DEFAULT_PROJECTS_PATH="$(toml_get "$CONFIG_FILE" projects default_path || true)"
DEFAULT_PROJECTS_PATH="${DEFAULT_PROJECTS_PATH:-/opt/${PROJECT_NAME}/projects}"
AGENT_PROVIDER="$(toml_get "$CONFIG_FILE" agents provider || true)"
AGENT_MODEL="$(toml_get "$CONFIG_FILE" agents model || true)"
PM_AGENT_PROVIDER="$(toml_get "$CONFIG_FILE" pm_agent provider || true)"
PM_AGENT_MODEL="$(toml_get "$CONFIG_FILE" pm_agent model || true)"
MANDATORY_AUTH="$(toml_get "$CONFIG_FILE" auth mandatory || true)"; MANDATORY_AUTH="${MANDATORY_AUTH:-true}"
MANDATORY_AUTH="$(echo "$MANDATORY_AUTH" | tr '[:upper:]' '[:lower:]')"

for v in PROJECT_NAME SERVER_IP SSH_USER DOMAIN_NAME; do
  if [ -z "${!v:-}" ]; then
    echo "Missing required config key: $v"
    exit 1
  fi
done

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
  --exclude='project.toml' \
  -e "ssh -o StrictHostKeyChecking=no" \
  "${PROJECT_ROOT}/" \
  "${SSH_USER}@${SERVER_IP}:${REMOTE_ROOT}/"

echo "[deploy] sync llm-sdk source tree"
LLM_SDK_ROOT="$(cd "${PROJECT_ROOT}/../llm-sdk" && pwd)"
rsync -az --delete \
  --exclude='target/' \
  --exclude='.git/' \
  --exclude='.DS_Store' \
  -e "ssh -o StrictHostKeyChecking=no" \
  "${LLM_SDK_ROOT}/" \
  "${SSH_USER}@${SERVER_IP}:${REMOTE_BASE_DIR}/llm-sdk/"

echo "[deploy] build backend on server"
remote_exec "command -v sccache >/dev/null 2>&1 || (SARCH=\$(uname -m); case \"\$SARCH\" in x86_64) ST=x86_64-unknown-linux-musl ;; aarch64) ST=aarch64-unknown-linux-musl ;; *) echo \"unsupported arch: \$SARCH\"; exit 1 ;; esac && SCCACHE_VER=\$(curl -fsSL -o /dev/null -w '%{url_effective}' https://github.com/mozilla/sccache/releases/latest | grep -o 'v[0-9.]*\$') && curl -fsSL \"https://github.com/mozilla/sccache/releases/download/\${SCCACHE_VER}/sccache-\${SCCACHE_VER}-\${ST}.tar.gz\" | tar xz -C /tmp && sudo mv /tmp/sccache-\${SCCACHE_VER}-\${ST}/sccache /usr/local/bin/sccache && rm -rf /tmp/sccache-\${SCCACHE_VER}-\${ST})"
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

echo "[deploy] upload project.toml (non-secret config)"
{
  printf '[project]\nname  = "%s"\ntitle = "%s"\n\n' "${PROJECT_NAME}" "${PROJECT_TITLE}"
  printf '[database]\nkind = "%s"\nurl  = "nocodo.db"\n\n' "${DB_KIND}"
  printf '[server]\nhost = "127.0.0.1"\nport = %s\n\n' "${BACKEND_PORT}"
  printf '[gui]\nport = %s\n\n' "${GUI_PORT}"
  printf '[admin_gui]\nport = %s\n\n' "${ADMIN_GUI_PORT}"
  printf '[projects]\ndefault_path = "%s"\n\n' "${DEFAULT_PROJECTS_PATH}"
  printf '[auth]\nmandatory = %s\n\n' "${MANDATORY_AUTH}"
  if [ -n "${AGENT_PROVIDER:-}" ] && [ -n "${AGENT_MODEL:-}" ]; then
    printf '[agents]\nprovider = "%s"\nmodel    = "%s"\n\n' "${AGENT_PROVIDER}" "${AGENT_MODEL}"
  fi
  if [ -n "${PM_AGENT_PROVIDER:-}" ] && [ -n "${PM_AGENT_MODEL:-}" ]; then
    printf '[pm_agent]\nprovider = "%s"\nmodel    = "%s"\n\n' "${PM_AGENT_PROVIDER}" "${PM_AGENT_MODEL}"
  fi
  printf '[deploy]\nserver_ip         = "%s"\nssh_user          = "%s"\ndomain_name       = "%s"\nletsencrypt_email = "%s"\n' \
    "${SERVER_IP}" "${SSH_USER}" "${DOMAIN_NAME}" "${LETSENCRYPT_EMAIL:-}"
} | ssh -o StrictHostKeyChecking=no "${SSH_USER}@${SERVER_IP}" "cat > /tmp/project.toml"
remote_exec "sudo mv /tmp/project.toml ${DEPLOY_ROOT}/project.toml && sudo chown ${SSH_USER}:${SSH_USER} ${DEPLOY_ROOT}/project.toml"

echo "[deploy] run database migrations"
remote_exec "bash -c 'set -a; . ${DEPLOY_ROOT}/server.env; cd ${DEPLOY_ROOT}; ./${MIGRATE_BIN}'"

echo "[deploy] upload gui dist"
remote_exec "sudo rm -rf ${DEPLOY_ROOT}/gui/* && sudo mkdir -p ${DEPLOY_ROOT}/gui"
remote_exec "mkdir -p /tmp/${PROJECT_NAME}-gui-dist"
scp -o StrictHostKeyChecking=no -r "$PROJECT_ROOT/gui/dist/"* "${SSH_USER}@${SERVER_IP}:/tmp/${PROJECT_NAME}-gui-dist/"
remote_exec "sudo mv /tmp/${PROJECT_NAME}-gui-dist/* ${DEPLOY_ROOT}/gui/ && rmdir /tmp/${PROJECT_NAME}-gui-dist"
remote_exec "sudo chown -R ${SSH_USER}:${SSH_USER} ${DEPLOY_ROOT}/gui"

echo "[deploy] upload admin-gui dist"
remote_exec "sudo rm -rf ${DEPLOY_ROOT}/admin-gui/* && sudo mkdir -p ${DEPLOY_ROOT}/admin-gui"
remote_exec "mkdir -p /tmp/${PROJECT_NAME}-admin-gui-dist"
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
