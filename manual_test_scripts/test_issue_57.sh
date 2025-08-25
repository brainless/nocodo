#!/usr/bin/env bash
set -euo pipefail

BASE="http://localhost:8081/api"

echo "1) Create AI session (tool=echo)..."
CREATE_PAYLOAD='{"project_id":null,"tool_name":"echo","prompt":"Hello from nocodo runner"}'
SESSION_JSON=$(curl -sS -X POST "$BASE/ai/sessions" -H 'Content-Type: application/json' -d "$CREATE_PAYLOAD")
echo "$SESSION_JSON" | jq . || echo "$SESSION_JSON"
SESSION_ID=$(echo "$SESSION_JSON" | jq -r '.session.id' 2>/dev/null || true)
if [[ -z "${SESSION_ID:-}" || "$SESSION_ID" == "null" ]]; then
echo "Failed to extract session id. Raw response:" >&2
echo "$SESSION_JSON" >&2
exit 1
fi
echo "Session ID: $SESSION_ID"

echo "2) Poll status until completion (timeout ~10s)..."
ATTEMPTS=20
for i in $(seq 1 $ATTEMPTS); do
STATUS_JSON=$(curl -sS "$BASE/ai/sessions/$SESSION_ID")
STATUS=$(echo "$STATUS_JSON" | jq -r '.session.status' 2>/dev/null || echo "unknown")
echo "  Attempt $i: status=$STATUS"
[[ "$STATUS" == "completed" || "$STATUS" == "failed" ]] && break
sleep 0.5
done

echo "3) Fetch recorded outputs..."
OUT_JSON=$(curl -sS "$BASE/ai/sessions/$SESSION_ID/outputs")
echo "$OUT_JSON" | jq . || echo "$OUT_JSON"

echo "4) (Optional) Try input endpoint (will likely be rejected because echo exits fast)"
INP_PAYLOAD='{"content":"additional line"}'
INP_JSON=$(curl -sS -X POST "$BASE/ai/sessions/$SESSION_ID/input" -H 'Content-Type: application/json' -d "$INP_PAYLOAD" || true)
echo "$INP_JSON"

echo "Done. If outputs contain the prompt text, runner → WS broadcast → DB persistence worked."
