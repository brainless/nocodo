#!/bin/bash

# Demo script to show WebSocket connection persistence fix
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[$(date '+%H:%M:%S')] $1${NC}"
}

success() {
    echo -e "${GREEN}[SUCCESS] $1${NC}"
}

cleanup() {
    log "Cleaning up demo processes..."
    kill $MANAGER_PID 2>/dev/null || true
    kill $WEB_PID 2>/dev/null || true
    log "Demo completed"
}

trap cleanup EXIT

log "🚀 Starting WebSocket persistence demo..."

# Start Manager daemon
log "Starting Manager daemon..."
./target/release/nocodo-manager > test_logs/demo_manager.log 2>&1 &
MANAGER_PID=$!
sleep 3

# Start web development server
log "Starting web development server..."
cd manager-web
npm run dev > ../test_logs/demo_web.log 2>&1 &
WEB_PID=$!
cd ..
sleep 5

success "✅ Services started successfully!"
echo
echo "🔧 WHAT WE FIXED:"
echo "=================="
echo "❌ BEFORE: Multiple WebSocketProvider instances per route"
echo "   - Each route had its own <WebSocketProvider>"
echo "   - Navigation caused provider unmount → disconnect"
echo "   - New route mounted new provider → reconnect"
echo "   - Result: Disconnect/reconnect cycle on every navigation"
echo
echo "✅ AFTER: Single global WebSocketProvider"
echo "   - One <WebSocketProvider> wraps entire app"
echo "   - Navigation only changes route components"
echo "   - WebSocket connection persists across all routes"
echo "   - Result: Stable connection throughout app usage"
echo
echo "📱 TEST THE FIX:"
echo "==============="
echo "1. Open http://localhost:3000 in your browser"
echo "2. Open Developer Console (F12)"
echo "3. Watch WebSocket messages in Console or Network tab"
echo "4. Navigate between: Dashboard → Projects → AI Sessions → back to Dashboard"
echo "5. Observe: NO disconnection/reconnection messages!"
echo
echo "🔍 WHAT TO LOOK FOR:"
echo "===================="
echo "✅ GOOD: Single 'WebSocket connected' message when page loads"
echo "✅ GOOD: Heartbeat/ping messages continue uninterrupted during navigation"
echo "✅ GOOD: Connection status indicator stays 'Connected' (green dot)"
echo "❌ BAD: Multiple 'WebSocket connected/disconnected' messages during navigation"
echo
success "Demo is ready! Press Ctrl+C when done testing."

# Wait for user to finish testing
wait
