# Desktop App: Display All Tool Calls in Work Details

## Problem
Work details page shows only 1 tool call when "Show tools" is enabled, but API returns all 10 tool calls. For Work ID 106 (Claude Haiku 4.5), only "Listed 20 files" is shown instead of all 10 tool calls (1 list_files, 5 read_file, 4 write_file).

## Root Cause
Desktop app fetches tool calls from `/api/work/{id}/tool-calls` and stores in `state.ai_tool_calls`, but **never renders this data**. Instead, it only displays tool calls embedded in `ai_session_outputs` messages, which misses most tools.

## Current Implementation

**Data fetching** (✅ Works):
- `desktop-app/src/services/api.rs:392` - Fetches from `/api/work/{id}/tool-calls`
- `desktop-app/src/services/background_tasks.rs:203` - Stores in `state.ai_tool_calls`

**Data rendering** (❌ Broken):
- `desktop-app/src/pages/board.rs:465-566` - Only renders tools from `ai_session_outputs.content.tool_calls`
- `desktop-app/src/pages/board.rs:364,369` - Only checks loading/empty state of `ai_tool_calls`
- No code to iterate and render `state.ai_tool_calls` array

## Task
Add rendering logic to display all tool calls from `state.ai_tool_calls` when "Show tools" is enabled.

## Requirements
1. Iterate over `state.ai_tool_calls` in the message history section
2. Render each tool call with:
   - Tool name (from `tool_name` field)
   - Request data (from `request` JSON)
   - Response data (from `response` JSON)
   - Status, timestamps, execution time
3. Support expand/collapse like existing tool displays
4. Show tools in chronological order (sort by `created_at`)
5. Merge with existing `ai_session_outputs` timeline or display separately

## Data Structure
`manager_models::LlmAgentToolCall` (manager-models/src/lib.rs:808-821):
```rust
pub struct LlmAgentToolCall {
    pub id: i64,
    pub tool_name: String,           // "list_files", "read_file", "write_file"
    pub request: serde_json::Value,   // Tool request parameters
    pub response: Option<serde_json::Value>, // Tool response data
    pub status: String,               // "completed", "pending", "failed"
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub execution_time_ms: Option<i64>,
}
```

## Acceptance Criteria
- All 10 tool calls visible for Work 106 when "Show tools" enabled
- Each tool call shows name, request/response summary
- Expandable to see full request/response JSON
- Chronological ordering maintained
- No duplicate rendering of tools already in messages

## Files to Modify
- `desktop-app/src/pages/board.rs` - Add rendering logic in message history section (around line 406-800)
