# Remove LLM/Agent Code from Desktop App

**Status**: 📋 Planned

## Summary

Remove LLM/agent-related code from the desktop-app crate. The API endpoints for LLM agent sessions and tool calls are being removed from the manager (see `remove-llm-code-from-manager.md`), and the corresponding client-side code in desktop-app is currently broken/unusable.

This cleanup prepares desktop-app for future integration with the new `nocodo-agents` crate.

## Scope

### Files to Modify (3 files)
1. `desktop-app/src/api_client.rs` - Remove `get_ai_tool_calls()` method
2. `desktop-app/src/state/mod.rs` - Remove `ai_tool_calls` state fields
3. `desktop-app/src/pages/board.rs` - Remove `ToolCall` UI component usage

### Dependencies
- Must run **after** `remove-llm-code-from-manager.md` completes Phase 6 (shared-types cleanup)
- Manager removes `LlmAgentToolCall` from shared-types, breaking desktop-app builds

## Rationale

The desktop-app currently has tool calls functionality that:
1. **Calls non-existent API endpoint**: `/api/work/{id}/tool-calls` doesn't exist in manager's routes.rs
2. **Uses broken LLM types**: Depends on `LlmAgentToolCall` being removed from manager and shared-types
3. **Has no UI integration**: Tool calls are in state but not displayed in current UI

By removing this code:
1. ✅ **Fixes compilation errors** - desktop-app will build after manager cleanup
2. ✅ **Cleaner codebase** - Removes dead/broken code
3. ✅ **Prepares for future integration** - Clean slate for nocodo-agents integration

## Implementation Plan

### Phase 1: Remove API Client Method

**Step 1.1: Edit api_client.rs** (`desktop-app/src/api_client.rs`)

Remove the `get_ai_tool_calls()` method:
```rust
// DELETE lines 168-190:
pub async fn get_ai_tool_calls(
    &self,
    work_id: i64,
) -> Result<Vec<shared_types::LlmAgentToolCall>, ApiError> {
    let url = format!("{}/api/work/{}/tool-calls", self.base_url, work_id);
    // ... entire method
}
```

**Why**: The endpoint `/api/work/{id}/tool-calls` doesn't exist in manager (verified in routes.rs)

**Verification**:
```bash
cargo check --package desktop-app
```

---

### Phase 2: Remove State Fields

**Step 2.1: Edit state/mod.rs** (`desktop-app/src/state/mod.rs`)

Remove `ai_tool_calls` fields from state:

1. **Line ~71**: Remove field from `BoardState`
   ```rust
   // DELETE:
   pub ai_tool_calls: Vec<shared_types::LlmAgentToolCall>,
   ```

2. **Line ~184**: Remove result field from `BoardStateResult`
   ```rust
   // DELETE:
   pub ai_tool_calls_result: ArcResult<Vec<shared_types::LlmAgentToolCall>>,
   ```

3. **Initialize field in BoardState::new()**: Remove any initialization if present

**Why**: Tool calls data is no longer fetched from API (removed in Phase 1)

**Verification**:
```bash
cargo check --package desktop-app
```

---

### Phase 3: Remove UI Usage

**Step 3.1: Edit pages/board.rs** (`desktop-app/src/pages/board.rs`)

Remove `ToolCall` component usage around line 413:
```rust
// DELETE or comment out ToolCall rendering:
// ToolCall(shared_types::LlmAgentToolCall),
```

If this is in a match arm or conditional rendering, ensure the code still compiles.

**Note**: Check if there are any other references to `ai_tool_calls` in this file using grep.

**Verification**:
```bash
grep -n "ToolCall\|ai_tool_calls" desktop-app/src/pages/board.rs
cargo check --package desktop-app
```

---

### Phase 4: Cleanup Imports and References

**Step 4.1: Search for any remaining references**

```bash
grep -r "LlmAgentToolCall\|ai_tool_calls" desktop-app/src
```

Remove any unused imports or references found:
- Import statements like `use shared_types::LlmAgentToolCall`
- Any other state or UI references

**Step 4.2: Verify no broken references**

```bash
grep -r "get_ai_tool_calls\|tool-calls" desktop-app/src
```

Should show no results after cleanup.

---

### Phase 5: Verification

**Step 5.1: Compilation check**
```bash
cargo check --package desktop-app
```
**Expected**: Desktop app compiles without errors

**Step 5.2: Full workspace check**
```bash
cargo check --workspace
```
**Expected**: All crates compile (manager cleanup should already be complete)

**Step 5.3: Build check**
```bash
cargo build --release --package desktop-app
```
**Expected**: Desktop app builds successfully

**Step 5.4: Manual verification checklist**
- [ ] No references to `get_ai_tool_calls()` in desktop-app
- [ ] No `ai_tool_calls` fields in state
- [ ] No `ToolCall` component usage in UI
- [ ] No imports of `LlmAgentToolCall`
- [ ] Desktop app compiles and builds
- [ ] No broken references to tool-calls endpoint

---

## Risk Assessment

### Low Risk
- **No active users**: The `/api/work/{id}/tool-calls` endpoint doesn't exist in manager, so this code was never working
- **No UI impact**: Tool calls weren't being displayed in current UI implementation
- **Pure removal**: Only removing dead code, no behavioral changes

### Dependencies
- ⚠️ **Must run after manager cleanup Phase 6**: Removes `LlmAgentToolCall` from shared-types
- If run before manager cleanup, desktop-app will still break due to shared-types changes
- Running after ensures clean compilation across workspace

## Rollback Strategy

All removed code is tracked in git history:
```bash
# Before starting
git tag pre-desktop-llm-cleanup
git commit -m "checkpoint: before desktop app LLM cleanup"

# Restore single file if needed
git checkout pre-desktop-llm-cleanup -- desktop-app/src/api_client.rs
```

## Success Criteria

### Code Removal
- ✅ `get_ai_tool_calls()` method removed from api_client.rs
- ✅ `ai_tool_calls` fields removed from state/mod.rs
- ✅ `ToolCall` usage removed from pages/board.rs
- ✅ All unused imports cleaned up

### Testing
- ✅ `cargo check --package desktop-app` passes
- ✅ `cargo build --release --package desktop-app` succeeds
- ✅ `cargo check --workspace` passes
- ✅ No grep results for removed code

### Final State
- ✅ Desktop app compiles cleanly after manager cleanup
- ✅ No broken references to removed API endpoints
- ✅ No broken references to removed shared types
- ✅ Ready for future nocodo-agents integration

## Future Integration Notes

After this cleanup, tool calls functionality can be reintroduced via:

1. **New API endpoints**: When manager integrates with nocodo-agents, it will provide new endpoints for agent sessions
2. **Updated shared types**: New types from nocodo-agents crate, not manager-specific types
3. **Desktop app integration**: Connect to new agent API with proper WebSocket support for streaming responses

This integration should be tracked in a future task: `integrate-nocodo-agents-into-desktop-app.md`

## References

- **Related task**: `tasks/remove-llm-code-from-manager.md` - Removes LLM code from manager and shared-types
- **Related task**: `tasks/nocodo-llm-sdk-creation.md` - LLM SDK crate (completed)
- **Related task**: `tasks/implement-tool-execution-flow.md` - Tool execution in nocodo-agents
- **Desktop app architecture**: `specs/DESKTOP_APP_STYLING.md` - UI component patterns
