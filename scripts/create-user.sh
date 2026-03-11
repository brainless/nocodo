#!/usr/bin/env bash
set -euo pipefail

# Create the SSH_USER from project.conf on a fresh server using initial bootstrap credentials.
# Run this once before setup-server.sh.
#
# Usage: ./create-user.sh <INITIAL_USER> [OPTIONS] [path/to/project.conf]
#
# Options:
#   --key <path>      SSH private key for initial connection (default: auto-detect ~/.ssh/id_*)
#   --password <pass> Password for initial connection (uses expect)
#
# Examples:
#   ./create-user.sh ubuntu                              # SSH key auto-detect (most cloud servers)
#   ./create-user.sh ubuntu --key ~/.ssh/my_key          # specific private key
#   ./create-user.sh ubuntu --key ~/.ssh/my_key /path/to/project.conf
#   ./create-user.sh root --password mypassword          # password auth (uses expect)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [ "$#" -lt 1 ]; then
  echo "Usage: $0 <INITIAL_USER> [--key <path> | --password <pass>] [path/to/project.conf]"
  exit 1
fi

INITIAL_USER="$1"
shift

INITIAL_KEY=""
INITIAL_PASSWORD=""
CONFIG_FILE=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --key)
      INITIAL_KEY="$2"; shift 2 ;;
    --password)
      INITIAL_PASSWORD="$2"; shift 2 ;;
    *)
      CONFIG_FILE="$1"; shift ;;
  esac
done

CONFIG_FILE="${CONFIG_FILE:-${PROJECT_ROOT}/project.conf}"

if [ ! -f "$CONFIG_FILE" ]; then
  echo "Config not found: $CONFIG_FILE"
  echo "Create it from project.conf.template"
  exit 1
fi

# shellcheck disable=SC1090
source "$CONFIG_FILE"

required_vars=(SERVER_IP SSH_USER)
for v in "${required_vars[@]}"; do
  if [ -z "${!v:-}" ]; then
    echo "Missing required config key: $v"
    exit 1
  fi
done

# Resolve the public key to install for SSH_USER.
# If --key was given, prefer its .pub counterpart; otherwise auto-detect.
if [ -n "$INITIAL_KEY" ] && [ -f "${INITIAL_KEY}.pub" ]; then
  PUB_KEY_FILE="${INITIAL_KEY}.pub"
else
  PUB_KEY_FILE=""
  for candidate in ~/.ssh/id_ed25519.pub ~/.ssh/id_rsa.pub ~/.ssh/id_ecdsa.pub; do
    if [ -f "$candidate" ]; then
      PUB_KEY_FILE="$candidate"
      break
    fi
  done
fi

if [ -z "$PUB_KEY_FILE" ]; then
  echo "No SSH public key found. Either:"
  echo "  - Generate one: ssh-keygen -t ed25519"
  echo "  - Pass --key /path/to/private_key (matching .pub must exist)"
  exit 1
fi

SSH_KEY=$(cat "$PUB_KEY_FILE")

# Build the remote provisioning commands (shared by both auth modes).
REMOTE_CMDS="
  id ${SSH_USER} &>/dev/null && echo USER_EXISTS || sudo useradd -m -s /bin/bash ${SSH_USER}
  sudo mkdir -p /home/${SSH_USER}/.ssh
  echo '${SSH_KEY}' | sudo tee /home/${SSH_USER}/.ssh/authorized_keys
  sudo chmod 700 /home/${SSH_USER}/.ssh
  sudo chmod 600 /home/${SSH_USER}/.ssh/authorized_keys
  sudo chown -R ${SSH_USER}:${SSH_USER} /home/${SSH_USER}/.ssh
  echo '${SSH_USER} ALL=(ALL) NOPASSWD:ALL' | sudo tee /etc/sudoers.d/${SSH_USER}
  sudo chmod 440 /etc/sudoers.d/${SSH_USER}
"

echo "[create-user] creating ${SSH_USER} on ${SERVER_IP}..."

if [ -n "$INITIAL_PASSWORD" ]; then
  # Password auth: drive the interactive SSH session with expect.
  expect << EOF
set timeout 30
spawn ssh -o StrictHostKeyChecking=no ${INITIAL_USER}@${SERVER_IP}
expect "password:"
send "${INITIAL_PASSWORD}\r"
expect "$ "
send "${REMOTE_CMDS}\r"
expect "$ "
send "exit\r"
expect eof
EOF
else
  # SSH key auth: run commands directly (no expect needed).
  SSH_OPTS="-o StrictHostKeyChecking=no"
  if [ -n "$INITIAL_KEY" ]; then
    SSH_OPTS="${SSH_OPTS} -i ${INITIAL_KEY}"
  fi
  # shellcheck disable=SC2086
  ssh $SSH_OPTS "${INITIAL_USER}@${SERVER_IP}" bash -s <<< "$REMOTE_CMDS"
fi

echo "[create-user] verifying SSH access as ${SSH_USER}..."
ssh -o StrictHostKeyChecking=no "${SSH_USER}@${SERVER_IP}" "whoami && sudo whoami"

echo "[create-user] done — ${SSH_USER} created on ${SERVER_IP}"
echo "[create-user] next step: run scripts/setup-server.sh"
