# Fix GPT-5-Codex E2E Test - Tool Call History Reconstruction Issue

## Quick Summary

**Issue**: E2E test fails with `400 Bad Request: Unknown parameter: 'input[1].tool_calls'`
**File**: `manager/src/llm_client/adapters/responses_api.rs` lines 104-130
**Fix**: Remove `tool_calls` field from assistant messages; add function calls as separate `input` items
**Test**: `./run_llm_e2e_test.sh openai gpt-5-codex` must pass without errors

## Problem

The E2E test `./run_llm_e2e_test.sh openai gpt-5-codex` fails with:

```
ERROR: API error: 400 Bad Request - {
  "error": {
    "message": "Unknown parameter: 'input[1].tool_calls'.",
    "type": "invalid_request_error",
    "param": "input[1].tool_calls",
    "code": "unknown_parameter"
  }
}
```

**Root Cause**: The `ResponsesApiAdapter` incorrectly includes `tool_calls` field in assistant messages when reconstructing conversation history. OpenAI's Responses API does NOT support the `tool_calls` field in the `input` array - it expects function calls to be represented differently.

**Flow that triggers the bug**:
1. LLM makes a tool call (e.g., `list_files`)
2. Tool executes successfully
3. System tries to send tool result back to LLM
4. Adapter reconstructs conversation history including previous assistant message
5. Adapter includes `tool_calls` field in assistant message → **API rejects this**

## Expected Behavior

According to OpenAI Responses API documentation:
- **Current request** function calls: Returned in `output` array as `ResponseItem::FunctionCall`
- **Conversation history** function calls: Should be represented as separate items in the `input` array with `type: "function_call"`

## Files to Fix

### `manager/src/llm_client/adapters/responses_api.rs`

**Location**: Lines 104-130 (assistant message conversion)

**Current INCORRECT code**:
```rust
"assistant" => {
    let mut msg_obj = serde_json::json!({"role": "assistant"});
    let content = message.content.as_deref().unwrap_or("");
    msg_obj["content"] = Value::String(content.to_string());

    // ❌ WRONG: tool_calls field is not supported in input array
    if let Some(tool_calls) = &message.tool_calls {
        msg_obj["tool_calls"] = Value::Array(tool_calls_json);
    }

    input.push(msg_obj);
}
```

**Required FIX**:
```rust
"assistant" => {
    // 1. Add assistant message with text content (if any)
    if let Some(content) = &message.content {
        if !content.is_empty() {
            input.push(serde_json::json!({
                "role": "assistant",
                "content": content
            }));
        }
    }

    // 2. Add function calls as SEPARATE items with type "function_call"
    if let Some(tool_calls) = &message.tool_calls {
        for tc in tool_calls {
            input.push(serde_json::json!({
                "type": "function_call",
                "call_id": tc.id,
                "name": tc.function.name,
                "arguments": tc.function.arguments
            }));
        }
    }
}
```

**Key differences**:
1. Assistant messages should NOT contain `tool_calls` field
2. Function calls must be separate items in `input` array
3. Function call items use `type: "function_call"` (not `role`)
4. Function call items have `call_id`, `name`, `arguments` at top level
5. Assistant messages with no content should be skipped (only if empty)

## Testing

**Run the E2E test**:
```bash
./run_llm_e2e_test.sh openai gpt-5-codex
```

**Expected success indicators**:
1. ✅ No `400 Bad Request` errors
2. ✅ LLM makes initial tool call (e.g., `list_files`)
3. ✅ Tool result sent back successfully
4. ✅ LLM continues conversation and makes more tool calls if needed
5. ✅ Final text response received
6. ✅ Response contains required keywords: Django, Python, PostgreSQL, GraphQL
7. ✅ Test completes within 120 seconds

**Expected test flow**:
```
1. User prompt: "What is the tech stack..."
2. LLM → tool call: list_files
3. System → tool result: "saleor/, manage.py, ..."
4. LLM → tool call: read_file(manage.py)
5. System → tool result: "#!/usr/bin/env python..."
6. LLM → tool call: read_file(pyproject.toml)
7. System → tool result: "[tool.poetry]..."
8. LLM → text response: "The project uses Django, Python, PostgreSQL, GraphQL..."
9. Test validates keywords → PASS ✅
```

## Verification Checklist

After fixing:
- [ ] Code compiles without errors: `cd manager && cargo build`
- [ ] Unit tests pass: `cargo test --lib`
- [ ] E2E test passes: `./run_llm_e2e_test.sh openai gpt-5-codex`
- [ ] No regression in other tests: `cargo test`
- [ ] Check logs show successful follow-up calls (no 400 errors)

## Reference: Responses API Format

**Request format** (what we send):
```json
{
  "model": "gpt-5-codex",
  "instructions": "You are Codex...",
  "input": [
    {"role": "user", "content": "What is the tech stack?"},
    {"role": "assistant", "content": "Let me check the files."},
    {"type": "function_call", "call_id": "call_123", "name": "list_files", "arguments": "{}"},
    {"type": "function_call_output", "call_id": "call_123", "output": "manage.py\nsaleor/\n..."}
  ],
  "tools": [...],
  "tool_choice": "auto"
}
```

**Response format** (what we receive):
```json
{
  "id": "resp_abc",
  "model": "gpt-5-codex",
  "output": [
    {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "I'll read manage.py"}]},
    {"type": "function_call", "call_id": "call_456", "name": "read_file", "arguments": "{\"path\":\"manage.py\"}"}
  ]
}
```

## Debug Tips

If the test still fails:
1. Check error logs for `400 Bad Request` → Still have format issue
2. Check for timeout → Tool calls working but no final response
3. Add debug logging in `convert_to_responses_request()` to print the `input` array
4. Verify `input` array items match the format above (no `tool_calls` field in assistant messages)

## Success Criteria

The fix is complete when:
1. E2E test runs without `400 Bad Request` errors
2. Tool calls execute successfully in conversation history
3. Test completes with keyword validation passing
4. All unit tests still pass (no regressions)
