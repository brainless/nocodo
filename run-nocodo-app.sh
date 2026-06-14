#!/usr/bin/env bash
set -e

# Build backend
cargo build --bin nocodo-backend

# Start backend in background
RUST_LOG=nocodo_agents=debug,nocodo_backend=debug,llm_sdk=debug \
NOCODO_LLM_LOG_PAYLOADS=1 \
./target/debug/nocodo-backend &
BACKEND_PID=$!

# Start admin-gui dev server in background
npm --prefix admin-gui run dev &
ADMINGUI_PID=$!

echo "Backend PID: $BACKEND_PID"
echo "Admin-GUI PID: $ADMINGUI_PID"
echo "Admin GUI: http://127.0.0.1:6626"
echo "Press Ctrl+C to stop both."

trap "kill $BACKEND_PID $ADMINGUI_PID 2>/dev/null; exit" INT TERM

wait
