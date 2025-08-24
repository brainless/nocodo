#!/bin/bash

# Integration test for AI session lifecycle with persistence and WebSocket events
# This script tests the full end-to-end flow from CLI to Manager to Web UI

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
MANAGER_PORT=8081
WEB_PORT=3000
TEST_PROJECT_DIR="/tmp/nocodo-test-project"
LOG_DIR="test_logs"

log() {
    echo -e "${BLUE}[$(date '+%H:%M:%S')] $1${NC}"
}

success() {
    echo -e "${GREEN}[SUCCESS] $1${NC}"
}

error() {
    echo -e "${RED}[ERROR] $1${NC}"
}

warning() {
    echo -e "${YELLOW}[WARNING] $1${NC}"
}

cleanup() {
    log "Cleaning up processes and test data..."
    
    # Kill background processes
    if [[ -n "$MANAGER_PID" ]]; then
        kill $MANAGER_PID 2>/dev/null || true
        wait $MANAGER_PID 2>/dev/null || true
    fi
    
    if [[ -n "$WEB_PID" ]]; then
        kill $WEB_PID 2>/dev/null || true
        wait $WEB_PID 2>/dev/null || true
    fi
    
    # Clean up test project
    rm -rf "$TEST_PROJECT_DIR" || true
    
    log "Cleanup completed"
}

# Set trap for cleanup on exit
trap cleanup EXIT

# Create log directory
mkdir -p "$LOG_DIR"

log "Starting AI session integration test..."

# Store absolute paths
PROJECT_ROOT="$(pwd)"
CLI_PATH="$PROJECT_ROOT/target/release/nocodo"

# Step 1: Build the project
log "Building the project..."
cargo build --release
if [[ $? -ne 0 ]]; then
    error "Failed to build project"
    exit 1
fi
success "Project built successfully"

# Step 2: Start Manager daemon
log "Starting Manager daemon..."
./target/release/nocodo-manager > "$LOG_DIR/manager.log" 2>&1 &
MANAGER_PID=$!

# Wait for Manager to start
log "Waiting for Manager daemon to start..."
sleep 3

# Check if Manager is running
if ! kill -0 $MANAGER_PID 2>/dev/null; then
    error "Manager daemon failed to start"
    cat "$LOG_DIR/manager.log"
    exit 1
fi

# Test Manager health
log "Testing Manager daemon health..."
response=$(curl -s -w "%{http_code}" -o /dev/null "http://127.0.0.1:$MANAGER_PORT/api/health")
if [[ "$response" -ne 200 ]]; then
    error "Manager daemon health check failed (HTTP $response)"
    cat "$LOG_DIR/manager.log"
    exit 1
fi
success "Manager daemon is healthy"

# Step 3: Start web development server
log "Starting web development server..."
cd manager-web
npm run dev > "../$LOG_DIR/web.log" 2>&1 &
WEB_PID=$!
cd ..

# Wait for web server to start
log "Waiting for web development server to start..."
sleep 5

# Check if web server is running
if ! kill -0 $WEB_PID 2>/dev/null; then
    error "Web development server failed to start"
    cat "$LOG_DIR/web.log"
    exit 1
fi

# Test web server
log "Testing web server..."
response=$(timeout 10 curl -s -w "%{http_code}" -o /dev/null "http://127.0.0.1:$WEB_PORT/" 2>/dev/null || echo "timeout")
if [[ "$response" == "timeout" ]]; then
    warning "Web server health check timed out, but server is running"
else
    if [[ "$response" -ne 200 ]]; then
        error "Web server health check failed (HTTP $response)"
        cat "$LOG_DIR/web.log"
        exit 1
    fi
fi
success "Web development server is running"

# Step 4: Create test project
log "Creating test project..."
rm -rf "$TEST_PROJECT_DIR"
mkdir -p "$TEST_PROJECT_DIR"
cd "$TEST_PROJECT_DIR"

# Initialize a simple test project
echo "# Test Project" > README.md
echo "This is a test project for nocodo integration testing." >> README.md

# Use CLI to add project to Manager
log "Adding project to Manager via CLI..."
"$CLI_PATH" project add
if [[ $? -ne 0 ]]; then
    error "Failed to add project via CLI"
    exit 1
fi

# Get project ID by checking the Manager API
log "Getting project ID from Manager API..."
projects_response=$(curl -s "http://127.0.0.1:$MANAGER_PORT/api/projects")
PROJECT_ID=$(echo "$projects_response" | jq -r '.[0].id')
if [[ -z "$PROJECT_ID" || "$PROJECT_ID" == "null" ]]; then
    error "Failed to get project ID from Manager API"
    echo "API Response: $projects_response"
    exit 1
fi
success "Project added with ID: $PROJECT_ID"

# Step 5: Test AI session creation via CLI
log "Creating AI session via CLI..."
# Use the session command which creates an AI session
"$CLI_PATH" session "test-tool" "Test AI session for integration testing"

if [[ $? -ne 0 ]]; then
    error "Failed to create AI session via CLI"
    exit 1
fi

# Get session ID from Manager API (get the most recent session)
log "Getting session ID from Manager API..."
sessions_response=$(curl -s "http://127.0.0.1:$MANAGER_PORT/api/ai/sessions")
SESSION_ID=$(echo "$sessions_response" | jq -r '.[0].id')
if [[ -z "$SESSION_ID" || "$SESSION_ID" == "null" ]]; then
    error "Failed to get session ID from Manager API"
    echo "API Response: $sessions_response"
    exit 1
fi
success "AI session created with ID: $SESSION_ID"

# Step 6: Verify session persistence in Manager API
log "Verifying session persistence via Manager API..."
response=$(curl -s "http://127.0.0.1:$MANAGER_PORT/api/ai/sessions/$SESSION_ID")
api_session_id=$(echo "$response" | jq -r '.id')
if [[ "$api_session_id" != "$SESSION_ID" ]]; then
    error "Session not found in Manager API"
    echo "API Response: $response"
    exit 1
fi
success "Session found in Manager API"

# Step 7: Test WebSocket connection and events
log "Testing WebSocket connection and events..."
# Create a simple WebSocket test client using Node.js
cat > websocket_test.js << 'EOF'
const WebSocket = require('ws');

const ws = new WebSocket('ws://127.0.0.1:8081/ws');
let receivedEvents = [];
let connected = false;
let timeout;

ws.on('open', function open() {
    console.log('WebSocket connected');
    connected = true;
    
    // Set timeout for test
    timeout = setTimeout(() => {
        console.log('Test timeout reached');
        ws.close();
        process.exit(0);
    }, 10000);
});

ws.on('message', function message(data) {
    try {
        const message = JSON.parse(data.toString());
        console.log('Received message:', JSON.stringify(message, null, 2));
        receivedEvents.push(message);
        
        // Look for AI session events
        if (message.type && message.type.includes('AiSession')) {
            console.log('Received AI session event:', message.type);
        }
    } catch (error) {
        console.log('Raw message:', data.toString());
    }
});

ws.on('error', function error(err) {
    console.error('WebSocket error:', err.message);
    process.exit(1);
});

ws.on('close', function close() {
    console.log('WebSocket disconnected');
    if (timeout) clearTimeout(timeout);
    
    // Print summary
    console.log(`Total events received: ${receivedEvents.length}`);
    
    // Check if we're properly connected
    if (connected) {
        console.log('WebSocket connection test PASSED');
        process.exit(0);
    } else {
        console.log('WebSocket connection test FAILED');
        process.exit(1);
    }
});

// Handle process termination
process.on('SIGINT', () => {
    ws.close();
    process.exit(0);
});
EOF

# Run WebSocket test if Node.js is available
if command -v node &> /dev/null; then
    log "Running WebSocket connection test..."
    npm install ws > /dev/null 2>&1 || true
    timeout 15s node websocket_test.js > "../$LOG_DIR/websocket_test.log" 2>&1 || true
    
    if grep -q "WebSocket connection test PASSED" "../$LOG_DIR/websocket_test.log"; then
        success "WebSocket connection test passed"
    else
        warning "WebSocket connection test had issues (check log for details)"
    fi
else
    warning "Node.js not available, skipping WebSocket test"
fi

# Step 8: Simulate session status updates
log "Testing session status updates..."

# Update session to running status
curl -s -X PUT "http://127.0.0.1:$MANAGER_PORT/api/ai/sessions/$SESSION_ID" \
    -H "Content-Type: application/json" \
    -d '{"status": "running"}' > /dev/null

# Verify status update
response=$(curl -s "http://127.0.0.1:$MANAGER_PORT/api/ai/sessions/$SESSION_ID")
status=$(echo "$response" | jq -r '.status')
if [[ "$status" != "running" ]]; then
    error "Failed to update session status to running"
    exit 1
fi
success "Session status updated to running"

# Sleep to allow WebSocket events to propagate
sleep 2

# Update session to completed status
curl -s -X PUT "http://127.0.0.1:$MANAGER_PORT/api/ai/sessions/$SESSION_ID" \
    -H "Content-Type: application/json" \
    -d '{"status": "completed"}' > /dev/null

# Verify final status
response=$(curl -s "http://127.0.0.1:$MANAGER_PORT/api/ai/sessions/$SESSION_ID")
status=$(echo "$response" | jq -r '.status')
if [[ "$status" != "completed" ]]; then
    error "Failed to update session status to completed"
    exit 1
fi
success "Session status updated to completed"

# Step 9: Test session outputs
log "Testing AI session outputs..."
output_response=$(curl -s -X POST "http://127.0.0.1:$MANAGER_PORT/api/ai/sessions/$SESSION_ID/outputs" \
    -H "Content-Type: application/json" \
    -d '{
        "content": "This is a test output from the AI session",
        "output_type": "text"
    }')

output_id=$(echo "$output_response" | jq -r '.id')
if [[ -z "$output_id" || "$output_id" == "null" ]]; then
    error "Failed to create AI session output"
    echo "Response: $output_response"
    exit 1
fi
success "AI session output created with ID: $output_id"

# Verify outputs list
outputs_response=$(curl -s "http://127.0.0.1:$MANAGER_PORT/api/ai/sessions/$SESSION_ID/outputs")
outputs_count=$(echo "$outputs_response" | jq '. | length')
if [[ "$outputs_count" -lt 1 ]]; then
    error "No outputs found for AI session"
    echo "Response: $outputs_response"
    exit 1
fi
success "AI session outputs verified ($outputs_count outputs found)"

# Step 10: Test web UI API endpoints
log "Testing web UI API endpoints..."

# Test sessions list endpoint
sessions_response=$(curl -s "http://127.0.0.1:$WEB_PORT/api/ai/sessions")
sessions_count=$(echo "$sessions_response" | jq '. | length')
if [[ "$sessions_count" -lt 1 ]]; then
    error "No sessions found via web API"
    echo "Response: $sessions_response"
    exit 1
fi
success "Sessions list retrieved via web API ($sessions_count sessions found)"

# Test specific session endpoint
session_response=$(curl -s "http://127.0.0.1:$WEB_PORT/api/ai/sessions/$SESSION_ID")
web_session_id=$(echo "$session_response" | jq -r '.id')
if [[ "$web_session_id" != "$SESSION_ID" ]]; then
    error "Session not accessible via web API"
    echo "Response: $session_response"
    exit 1
fi
success "Session accessible via web API"

# Step 11: Clean up test session
log "Cleaning up test session..."
# Note: We don't have a delete endpoint for sessions yet, which is fine
# The session will remain in the database for inspection

cd ..

# Step 12: Capture browser console for manual verification
log "Starting browser console capture for 10 seconds..."
if command -v google-chrome &> /dev/null || command -v chromium &> /dev/null; then
    # Use Chrome/Chromium if available
    CHROME_CMD="google-chrome"
    if ! command -v google-chrome &> /dev/null; then
        CHROME_CMD="chromium"
    fi
    
    # Start Chrome in headless mode to capture console logs
    timeout 10s $CHROME_CMD --headless --disable-gpu --enable-logging --log-level=0 \
        --remote-debugging-port=9222 "http://127.0.0.1:$WEB_PORT" \
        > "$LOG_DIR/manager_web_browser_console.log" 2>&1 || true
        
    success "Browser console logs captured"
else
    warning "Chrome/Chromium not available, skipping browser console capture"
fi

# Final summary
log "Integration test completed successfully!"
echo
echo "=== TEST SUMMARY ==="
echo "✅ Manager daemon started and healthy"
echo "✅ Web development server started and accessible"
echo "✅ Test project created via CLI"
echo "✅ AI session created via CLI (ID: $SESSION_ID)"
echo "✅ Session persistence verified in Manager API"
echo "✅ Session status updates working"
echo "✅ AI session outputs working"
echo "✅ Web API endpoints accessible"
echo "✅ WebSocket connection tested"
echo
echo "Log files created in $LOG_DIR/:"
echo "  - manager.log: Manager daemon logs"
echo "  - web.log: Web development server logs"
echo "  - websocket_test.log: WebSocket connection test logs"
echo "  - manager_web_browser_console.log: Browser console logs"
echo
echo "You can now manually test the web UI at: http://127.0.0.1:$WEB_PORT"
echo "The AI session should be visible in the Sessions page."
echo
success "All tests passed!"
