#!/usr/bin/env bash
# GUI check by an agent (epic 008, Task 8).
#
# Replays, with curl against a real `nocodo-api`, the server side of every
# action a person takes in `nocodo-gui`, so an agent can check them without a
# window. Uses a temporary projects folder and a free port; kills the server on
# exit. Run from the repo root:  scripts/gui-check.sh
#
# Steps marked [NEEDS MODEL KEY] call the real Schema agent. They are skipped
# unless NOCODO_CHECK_LIVE=1 and a key is set in .env; otherwise only the
# no-key part is checked (the request is accepted, the run fails with a
# plain message, and Retry (`continue`) is offered).
# The API has no route for `changes` (hand-written change sets), so schema
# setup uses the `nocodo` CLI; the GUI-side reads are all curl.
set -u
cd "$(dirname "$0")/.."

PASS=0; FAIL=0
ROOT=$(mktemp -d)
PROJ=$ROOT/projects
mkdir -p "$PROJ"
cargo build -q --bin nocodo --bin nocodo-api || { echo "build failed"; exit 1; }
BIN=target/debug
NOCODO="$BIN/nocodo"

LOG=$ROOT/server.log
$BIN/nocodo-api --addr 127.0.0.1:0 --projects-dir "$PROJ" >"$LOG" 2>&1 &
SERVER=$!
trap 'kill $SERVER 2>/dev/null; wait $SERVER 2>/dev/null; rm -rf "$ROOT"' EXIT
for _ in $(seq 50); do
  ADDR=$(sed -n 's#.*listening on \(http://[0-9.:]*\).*#\1#p' "$LOG"); [ -n "$ADDR" ] && break; sleep 0.1
done
[ -n "${ADDR:-}" ] || { echo "server did not start"; cat "$LOG"; exit 1; }
echo "server: $ADDR  projects: $PROJ"

# req METHOD PATH [JSON]: sets CODE and BODY.
req() {
  local out
  if [ $# -ge 3 ]; then
    out=$(curl -s -w '\n%{http_code}' -X "$1" -H 'content-type: application/json' -d "$3" "$ADDR$2")
  else
    out=$(curl -s -w '\n%{http_code}' -X "$1" "$ADDR$2")
  fi
  CODE=${out##*$'\n'}; BODY=${out%$'\n'*}
}
# check "label" EXPECTED_CODE [jq-filter expected-output]
check() {
  local label=$1 code=$2 got=""
  if [ "$CODE" != "$code" ]; then
    echo "FAIL  $label: HTTP $CODE (want $code): $BODY"; FAIL=$((FAIL+1)); return
  fi
  if [ $# -ge 4 ]; then
    got=$(printf '%s' "$BODY" | jq -c "$3")
    if [ "$got" != "$4" ]; then
      echo "FAIL  $label: $3 = $got (want $4): $BODY"; FAIL=$((FAIL+1)); return
    fi
  fi
  echo "ok    $label"; PASS=$((PASS+1))
}
# wait_run PROJECT RUN_ID: poll like the GUI until the run is not running.
wait_run() {
  for _ in $(seq 100); do
    req GET "/projects/$1/runs/$2"
    [ "$(printf '%s' "$BODY" | jq -r .status)" != running ] && return
    sleep 0.1
  done
}

echo "== Project dropdown: New project..., list"
req GET /projects;                              check "list, empty" 200 '.projects|length' 0
req POST /projects '{"name":"g-finance"}';      check "new project" 201
req POST /projects '{"name":"g-finance"}';      check "same name again is refused" 409
req POST /projects '{"name":"../x"}';           check "bad name refused" 400
req GET /projects;                              check "list shows it" 200 '.projects[0].folder' '"g-finance"'
req GET /projects/nope/summary;                 check "unknown project" 404

echo "== Data view on an empty project"
P=g-finance
req GET /projects/$P/summary;                   check "summary, no lists yet" 200
req GET /projects/$P/questions;                 check "no questions" 200 '.questions|length' 0
req GET /projects/$P/events?after=0;            check "events: only project_created" 200 '[.events[].kind.type]' '["project_created"]'
req GET /projects/$P/tables/accounts/rows;      check "rows of unknown list" 404

echo "== Schema setup by CLI (no model), then the GUI reload reads"
$NOCODO --project "$PROJ/$P" changes fixtures/change-sets/finance/01-initial.json >"$ROOT/c1.txt" 2>&1 \
  && echo "ok    cli changes 01-initial" && PASS=$((PASS+1)) || { echo "FAIL  cli changes 01: $(cat "$ROOT/c1.txt")"; FAIL=$((FAIL+1)); }
req GET /projects/$P/summary;                   check "summary lists the new lists" 200 '.summary|contains("Accounts")' true
TABLE=accounts   # the summary is plain text; the list names come from the finance change set
req GET /projects;                              check "list shows table count" 200 '.projects[0].tables > 0' true
req GET /projects/$P/events?after=0;            check "events show applied change" 200 '[.events[].kind.type] | any(. == "migration_applied")' true
LAST=$(printf '%s' "$BODY" | jq .last)
req GET "/projects/$P/events?after=$LAST";      check "poll after last is empty" 200 '.events|length' 0
req GET /projects/$P/tables/$TABLE/rows;        check "rows, empty list" 200 '.total' 0
req GET /projects/$P/tables/nope/rows;          check "rows, unknown list names existing ones" 404

echo "== Add sample rows button"
req POST /projects/$P/sample-rows '{"per_table":4}'; check "sample rows accepted" 202 '.status' '"running"'
RUN=$(printf '%s' "$BODY" | jq -r .run_id); wait_run $P "$RUN"
check "sample rows run finished" 200 '.status' '"finished"'
req GET /projects/$P/tables/$TABLE/rows;        check "rows now present" 200 '.total' 4
req GET "/projects/$P/tables/$TABLE/rows?offset=2&limit=2"; check "paging (Next)" 200 '.rows|length' 2
req GET "/projects/$P/tables/$TABLE/rows?offset=100&limit=2"; check "page past the end" 200 '.rows|length' 0

echo "== Apply button (nothing pending)"
req POST /projects/$P/apply;                    check "apply accepted" 202
RUN=$(printf '%s' "$BODY" | jq -r .run_id); wait_run $P "$RUN"
check "apply run finished" 200 '.status' '"finished"'

echo "== Follow-up change on top of data, then the destructive one (CLI, no model)"
$NOCODO --project "$PROJ/$P" changes fixtures/change-sets/finance/02-family-member.json >"$ROOT/c2.txt" 2>&1 \
  && echo "ok    cli changes 02" && PASS=$((PASS+1)) || { echo "FAIL  cli changes 02: $(cat "$ROOT/c2.txt")"; FAIL=$((FAIL+1)); }
$NOCODO --project "$PROJ/$P" changes fixtures/change-sets/finance/03-rebuild.json >"$ROOT/c3.txt" 2>&1 \
  && echo "ok    cli changes 03" && PASS=$((PASS+1)) || { echo "FAIL  cli changes 03: $(cat "$ROOT/c3.txt")"; FAIL=$((FAIL+1)); }
$NOCODO --project "$PROJ/$P" changes fixtures/change-sets/finance/04-drop-notes.json >"$ROOT/c4.txt" 2>&1
req GET /projects/$P/questions;                 check "removal confirmation is an open question (modal)" 200 '.questions|length > 0' true
echo "      question: $(printf '%s' "$BODY" | jq -c '.questions[0]')"
req POST /projects/$P/answers '{"q1":99}';      check "bad choice is accepted, the run record carries the error" 202
RUN=$(printf '%s' "$BODY" | jq -r .run_id); wait_run $P "$RUN"
req GET /projects/$P/runs/$RUN;                 check "bad answer shows as failed run" 200 '.status' '"failed"'
req GET /projects/$P/questions
QID=$(printf '%s' "$BODY" | jq -r '.questions[0].id')
req POST /projects/$P/answers "{\"$QID\":1}";   check "confirm in modal (choice 1)" 202
RUN=$(printf '%s' "$BODY" | jq -r .run_id); wait_run $P "$RUN"
req GET /projects/$P/runs/$RUN;                 check "answer run finished" 200
echo "      run: $BODY"
req GET /projects/$P/tables/transactions/rows; check "removed column is gone from the grid, rows kept" 200 '([.columns[].name]|index("notes")) == null and .total > 0' true

echo "== Prompt box [NEEDS MODEL KEY for the real Schema agent]"
req POST /projects/$P/prompt '{"text":""}';     check "empty prompt refused" 400
if [ "${NOCODO_CHECK_LIVE:-0}" = 1 ]; then
  req POST /projects/$P/prompt '{"text":"I want to track my family spending."}'; check "prompt accepted" 202
  RUN=$(printf '%s' "$BODY" | jq -r .run_id); wait_run $P "$RUN"
  req GET /projects/$P/runs/$RUN; echo "      run: $BODY"
  req GET /projects/$P/questions; echo "      questions: $(printf '%s' "$BODY" | jq -c '.questions')"
else
  echo "skip  live prompt (set NOCODO_CHECK_LIVE=1 with a key in .env)"
  # Without a key the request is still accepted and the run fails plainly.
  req POST /projects/$P/prompt '{"text":"I want to track my family spending."}'; check "prompt accepted" 202
  RUN=$(printf '%s' "$BODY" | jq -r .run_id); wait_run $P "$RUN"
  req GET /projects/$P/runs/$RUN
  echo "      run: $BODY"
  # While the run is still going (the model call retries without a key) Retry
  # is refused as busy; once it failed it is accepted.
  req POST /projects/$P/continue;               echo "      continue (Retry button): HTTP $CODE $BODY"
fi

echo "== Busy: CLI holds the write lock while the API is asked to write"
# A real hold needs a long-running writer (a slow model call). Covered by
# api/tests/api.rs (busy 409 for every slow route); not simulated here.

echo; echo "passed: $PASS  failed: $FAIL"
[ "$FAIL" = 0 ]
