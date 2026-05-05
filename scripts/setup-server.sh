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
SERVER_IP="$(toml_get "$CONFIG_FILE" deploy server_ip)"
SSH_USER="$(toml_get "$CONFIG_FILE" deploy ssh_user)"
DOMAIN_NAME="$(toml_get "$CONFIG_FILE" deploy domain_name)"
LETSENCRYPT_EMAIL="$(toml_get "$CONFIG_FILE" deploy letsencrypt_email)"
REMOTE_BASE_DIR="$(toml_get "$CONFIG_FILE" deploy remote_base_dir || true)"
REMOTE_BASE_DIR="${REMOTE_BASE_DIR:-/home/${SSH_USER}/apps}"
DB_KIND="$(toml_get "$CONFIG_FILE" database kind)"
BACKEND_PORT="$(toml_get "$CONFIG_FILE" server port)"
BACKEND_PORT="${BACKEND_PORT:-8080}"
GUI_PORT="$(toml_get "$CONFIG_FILE" gui port)"
GUI_PORT="${GUI_PORT:-3030}"
ADMIN_GUI_PORT="$(toml_get "$CONFIG_FILE" admin_gui port)"
ADMIN_GUI_PORT="${ADMIN_GUI_PORT:-3031}"
PROJECT_TITLE="$(toml_get "$CONFIG_FILE" project title)"
DEFAULT_PROJECTS_PATH="$(toml_get "$CONFIG_FILE" projects default_path || true)"
DEFAULT_PROJECTS_PATH="${DEFAULT_PROJECTS_PATH:-/opt/${PROJECT_NAME}/projects}"
AGENT_PROVIDER="$(toml_get "$CONFIG_FILE" agents provider || true)"
AGENT_MODEL="$(toml_get "$CONFIG_FILE" agents model || true)"
PM_AGENT_PROVIDER="$(toml_get "$CONFIG_FILE" pm_agent provider || true)"
PM_AGENT_MODEL="$(toml_get "$CONFIG_FILE" pm_agent model || true)"
OPENAI_API_KEY="$(toml_get "$CONFIG_FILE" api_keys openai_api_key || true)"
GROQ_API_KEY="$(toml_get "$CONFIG_FILE" api_keys groq_api_key || true)"
CEREBRAS_API_KEY="$(toml_get "$CONFIG_FILE" api_keys cerebras_api_key || true)"
ANTHROPIC_API_KEY="$(toml_get "$CONFIG_FILE" api_keys anthropic_api_key || true)"
RESEND_API_KEY="$(toml_get "$CONFIG_FILE" auth resend_api_key || true)"
AUTH_FROM_EMAIL="$(toml_get "$CONFIG_FILE" auth from_email || true)"
MANDATORY_AUTH="$(toml_get "$CONFIG_FILE" auth mandatory || true)"
MANDATORY_AUTH="${MANDATORY_AUTH:-true}"

for v in PROJECT_NAME SERVER_IP SSH_USER DOMAIN_NAME LETSENCRYPT_EMAIL; do
  if [ -z "${!v:-}" ]; then
    echo "Missing required config key: $v"
    exit 1
  fi
done

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
remote_exec "sudo sed -i 's/^#*KbdInteractiveAuthentication.*/KbdInteractiveAuthentication no/' /etc/ssh/sshd_config"
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
  remote_exec "SARCH=\$(uname -m); case \"\$SARCH\" in x86_64) ST=x86_64-unknown-linux-musl ;; aarch64) ST=aarch64-unknown-linux-musl ;; *) echo \"unsupported arch: \$SARCH\"; exit 1 ;; esac && SCCACHE_VER=\$(curl -fsSL -o /dev/null -w '%{url_effective}' https://github.com/mozilla/sccache/releases/latest | grep -o 'v[0-9.]*\$') && curl -fsSL \"https://github.com/mozilla/sccache/releases/download/\${SCCACHE_VER}/sccache-\${SCCACHE_VER}-\${ST}.tar.gz\" | tar xz -C /tmp && sudo mv /tmp/sccache-\${SCCACHE_VER}-\${ST}/sccache /usr/local/bin/sccache && rm -rf /tmp/sccache-\${SCCACHE_VER}-\${ST}"
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

echo "[setup] writing server.env (secrets only)"
{
  if [ -n "${DB_PASSWORD}" ]; then
    printf 'DATABASE_URL=postgresql://%s:%s@localhost/%s\n' "${SSH_USER}" "${DB_PASSWORD}" "${PROJECT_NAME}"
  else
    printf 'DATABASE_URL=%s\n' "nocodo.db"
  fi
  [ -n "${OPENAI_API_KEY:-}" ]    && printf 'OPENAI_API_KEY=%s\n'    "${OPENAI_API_KEY}"
  [ -n "${GROQ_API_KEY:-}" ]      && printf 'GROQ_API_KEY=%s\n'      "${GROQ_API_KEY}"
  [ -n "${CEREBRAS_API_KEY:-}" ]  && printf 'CEREBRAS_API_KEY=%s\n'  "${CEREBRAS_API_KEY}"
  [ -n "${ANTHROPIC_API_KEY:-}" ] && printf 'ANTHROPIC_API_KEY=%s\n' "${ANTHROPIC_API_KEY}"
  [ -n "${RESEND_API_KEY:-}" ]    && printf 'RESEND_API_KEY=%s\n'    "${RESEND_API_KEY}"
  [ -n "${AUTH_FROM_EMAIL:-}" ]   && printf 'AUTH_FROM_EMAIL=%s\n'   "${AUTH_FROM_EMAIL}"
} | ssh -o StrictHostKeyChecking=no "${SSH_USER}@${SERVER_IP}" "cat > /tmp/server.env"
remote_exec "sudo mv /tmp/server.env ${DEPLOY_ROOT}/server.env && sudo chown ${SSH_USER}:${SSH_USER} ${DEPLOY_ROOT}/server.env && chmod 600 ${DEPLOY_ROOT}/server.env"

echo "[setup] writing project.toml (non-secret config)"
{
  printf '[project]\nname  = "%s"\ntitle = "%s"\n\n' "${PROJECT_NAME}" "${PROJECT_TITLE}"
  printf '[database]\nkind = "%s"\nurl  = "nocodo.db"\n\n' "${DB_KIND:-sqlite}"
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
    "${SERVER_IP}" "${SSH_USER}" "${DOMAIN_NAME}" "${LETSENCRYPT_EMAIL}"
} | ssh -o StrictHostKeyChecking=no "${SSH_USER}@${SERVER_IP}" "cat > /tmp/project.toml"
remote_exec "sudo mv /tmp/project.toml ${DEPLOY_ROOT}/project.toml && sudo chown ${SSH_USER}:${SSH_USER} ${DEPLOY_ROOT}/project.toml"

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
