#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

ADMIN_GUI_PORT="$(
  awk -F= '/^ADMIN_GUI_PORT=/{print $2; exit}' project.conf 2>/dev/null | tr -d '[:space:]'
)"
if [[ -z "${ADMIN_GUI_PORT}" ]]; then
  ADMIN_GUI_PORT="6626"
fi

# Kill stale listeners on the admin GUI dev port from previous tauri dev runs.
if command -v lsof >/dev/null 2>&1; then
  STALE_PIDS="$(lsof -ti tcp:"$ADMIN_GUI_PORT" || true)"
  if [[ -n "${STALE_PIDS}" ]]; then
    echo "[tauri-dev] Killing stale process(es) on port ${ADMIN_GUI_PORT}: ${STALE_PIDS}"
    kill ${STALE_PIDS} || true
    sleep 0.2
    STALE_PIDS="$(lsof -ti tcp:"$ADMIN_GUI_PORT" || true)"
    if [[ -n "${STALE_PIDS}" ]]; then
      kill -9 ${STALE_PIDS} || true
    fi
  fi
fi

cargo build -p nocodo-backend
mkdir -p tauri/bin
cp target/debug/nocodo-backend tauri/bin/nocodo-backend-aarch64-apple-darwin

cd "$ROOT_DIR/admin-gui"
# Exec so tauri controls this process directly and can stop it on exit.
exec node ./node_modules/vite/bin/vite.js
