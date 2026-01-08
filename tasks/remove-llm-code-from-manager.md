# Remove LLM Code from Manager

**Status**: 📋 Planned

## Summary

Complete removal of LLM client, provider, and agent code from the manager crate. This functionality has been moved to `nocodo-llm-sdk` and `nocodo-agents` crates. This is a comprehensive cleanup to prepare the manager for reconnecting with the new modular architecture.

## Scope

### Files to Delete (6 core files)
1. `manager/src/llm_client.rs` - LLM client factory (62 lines)
2. `manager/src/llm_agent.rs` - Agent implementation (911 lines)
3. `manager/src/schema_provider.rs` - Schema providers (191 lines)
4. `manager/src/handlers/ai_session_handlers.rs` - Empty placeholder
5. `manager/tests/integration/llm_agent.rs` - Agent tests
6. `manager/tests/integration/tool_calls.rs` - Tool call tests

### Files to Modify (6 files)
1. `manager/src/lib.rs` - Remove module declarations
2. `manager/src/main.rs` - Remove LlmAgent initialization
3. `manager/src/handlers/main_handlers.rs` - Remove from AppState
4. `manager/src/handlers/work_handlers.rs` - Remove LLM integration
5. `manager/src/database/common.rs` - Remove tables and methods
6. `shared-types/src/lib.rs` - Remove LLM models

### Total Removal
- **~1,164 lines** from implementation files
- **3 database tables** (llm_agent_sessions, llm_agent_messages, llm_agent_tool_calls)
- **10 database methods**
- **4 model structs** from shared-types

## Rationale

The manager crate currently contains LLM-related code that has been properly extracted into dedicated crates:
- **nocodo-llm-sdk**: Handles all LLM client implementations (ClaudeClient, OpenAIClient, etc.) with a unified LlmClient trait
- **nocodo-agents**: Provides Agent trait and specialized agents (CodebaseAnalysisAgent, SqliteAnalysisAgent) with their own database for session management

By removing this code from manager:
1. ✅ **Cleaner separation of concerns** - Manager focuses on work/project management
2. ✅ **Reusable components** - LLM SDK and agents can be used independently
3. ✅ **Easier maintenance** - Changes to LLM/agent logic happen in dedicated crates
4. ✅ **Better testing** - Each crate has focused tests
5. ✅ **Preparation for reconnection** - Clean slate for integrating the new crates properly

## Implementation Plan

### Phase 1: Remove Dead Code (No Dependencies)

**Low Risk - Can execute immediately**

**Step 1.1: Remove schema_provider.rs**
```bash
rm manager/src/schema_provider.rs
```
- Edit `manager/src/lib.rs` line 14 - Remove `pub mod schema_provider;`
- **Why**: Completely unused (marked with `#[allow(dead_code)]`)

**Step 1.2: Remove test files**
```bash
rm manager/tests/integration/llm_agent.rs
rm manager/tests/integration/tool_calls.rs
```
- **Why**: Testing old implementation that's been moved to nocodo-agents

**Step 1.3: Remove empty handler**
```bash
rm manager/src/handlers/ai_session_handlers.rs
```
- **Why**: Empty placeholder (2 lines only)

**Verification after Phase 1:**
```bash
cargo check
```

---

### Phase 2: Update Work Handlers

**Medium Risk - Test after each change**

**Step 2.1: Edit work_handlers.rs** (`manager/src/handlers/work_handlers.rs`)

Remove LLM integration code:

1. **Line 3**: Remove import
   ```rust
   // DELETE: use crate::llm_client::CLAUDE_SONNET_4_5_MODEL_ID;
   ```

2. **Lines ~164-201**: Remove LLM session creation in `create_work` function
   - Currently creates LLM agent session when work has "llm-agent" tool
   - After removal, work creation won't auto-create agent sessions

3. **Lines ~327-390**: Remove LLM message processing in `add_message_to_work` function
   - Currently processes messages through LLM agent
   - After removal, messages will be stored but not processed by LLM

4. **Lines ~556-577**: Remove LLM message fetching in `get_work` function
   - Currently fetches LLM agent messages for work
   - After removal, only work messages will be returned

5. **Lines 13-33**: Remove `infer_provider_from_model()` helper function
   - Only used for LLM provider inference
   - Verify it's not used elsewhere first

**Step 2.2: Verify project_commands.rs** (`manager/src/handlers/project_commands.rs`)
- Check if lines 5-6 imports (`LlmAgent`, `CLAUDE_SONNET_4_5_MODEL_ID`) are actually used
- Remove if unused
- If used, plan replacement with nocodo-agents

**Verification after Phase 2:**
```bash
cargo check
```

---

### Phase 3: Update AppState and Main

**Medium Risk - Core application state changes**

**Step 3.1: Edit main_handlers.rs** (`manager/src/handlers/main_handlers.rs`)

Remove from AppState:
```rust
// Line 4 - DELETE
use crate::llm_agent::LlmAgent;

// Line 16 - DELETE from AppState struct
pub struct AppState {
    pub database: Arc<Database>,
    pub start_time: SystemTime,
    pub ws_broadcaster: Arc<WebSocketBroadcaster>,
    pub llm_agent: Option<Arc<LlmAgent>>,  // <-- DELETE THIS LINE
    pub config: Arc<RwLock<AppConfig>>,
}
```

**Step 3.2: Edit main.rs** (`manager/src/main.rs`)

Remove LlmAgent initialization:
```rust
// Line 27 - DELETE
use llm_agent::LlmAgent;

// Lines 95-102 - DELETE entire block
// Initialize LLM agent (always enabled)
tracing::info!("Initializing LLM agent");
let llm_agent = Some(Arc::new(LlmAgent::new(
    Arc::clone(&database),
    Arc::clone(&broadcaster),
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    Arc::new(config.clone()),
)));

// Line 108 - DELETE from AppState initialization
let app_state = web::Data::new(AppState {
    database,
    start_time: SystemTime::now(),
    ws_broadcaster: broadcaster,
    llm_agent,  // <-- DELETE THIS LINE
    config: Arc::new(std::sync::RwLock::new(config.clone())),
});
```

**Step 3.3: Edit lib.rs** (`manager/src/lib.rs`)

Remove module declarations:
```rust
// Line 8 - DELETE
pub mod llm_agent;

// Line 9 - DELETE
pub mod llm_client;

// Line 14 - DELETE (already done in Phase 1)
pub mod schema_provider;
```

**Verification after Phase 3:**
```bash
cargo check
```

---

### Phase 4: Remove Implementation Files

**Low Risk - All references removed in previous phases**

**Step 4.1: Delete core files**
```bash
rm manager/src/llm_agent.rs
rm manager/src/llm_client.rs
```

**What's being removed:**
- `llm_agent.rs` (911 lines): Main LlmAgent struct with session/message processing
- `llm_client.rs` (62 lines): Factory function for creating LLM clients, re-exports from SDK

**Verification after Phase 4:**
```bash
cargo check
grep -r "llm_agent\|llm_client\|LlmAgent" manager/src --exclude-dir=target
```
Should show no results (except in database/common.rs which we'll handle next)

---

### Phase 5: Database Cleanup

**⚠️ USER DATA AFFECTED - Backup recommended**

The manager database currently has 3 LLM agent tables that are separate from nocodo-agents' own database.

**Step 5.1: Create database migration** (`manager/src/database/common.rs`)

Update the `create_tables()` function to drop the LLM agent tables:

**Tables to remove from schema (lines 252-307):**
```sql
-- DELETE these table creation statements:
CREATE TABLE IF NOT EXISTS llm_agent_sessions (...)
CREATE TABLE IF NOT EXISTS llm_agent_messages (...)
CREATE TABLE IF NOT EXISTS llm_agent_tool_calls (...)
```

**Add migration code:**
```rust
// Drop legacy LLM agent tables
conn.execute("DROP TABLE IF EXISTS llm_agent_tool_calls", [])?;
conn.execute("DROP TABLE IF EXISTS llm_agent_messages", [])?;
conn.execute("DROP TABLE IF EXISTS llm_agent_sessions", [])?;
```

**Step 5.2: Remove database methods**

Delete these methods from `manager/src/database/common.rs`:

1. `create_llm_agent_session` (~line 1797)
2. `get_llm_agent_session` (~line 1762)
3. `get_llm_agent_sessions_by_work` (~line 1830)
4. `get_llm_agent_session_by_work_id` (~line 1858)
5. `update_llm_agent_session` (~line 1870)
6. `create_llm_agent_message` (~line 1896)
7. `get_llm_agent_messages` (~line 1923)
8. `create_llm_agent_tool_call` (~line 1948)
9. `update_llm_agent_tool_call` (~line 1981)
10. `get_llm_agent_tool_calls` (~line 2012)

**Total deletion**: ~300 lines of database code

**⚠️ Important Note:**
- Agent-related schema is now in the `nocodo-agents` crate with its own separate database
- The nocodo-agents database is located at a different path and managed independently
- No data migration needed since they're separate databases

**Verification after Phase 5:**
```bash
cargo check
cargo test --package manager --lib database::tests
```

---

### Phase 6: Shared Types Cleanup

**⚠️ Cross-package Impact - Verify no other crates depend on these types**

**Step 6.1: Edit shared-types/src/lib.rs**

Remove LLM-related model structs and their implementations:

**Structs to remove:**
1. `LlmProviderConfig` - LLM provider configuration
2. `LlmAgentSession` - Agent session model
3. `LlmAgentMessage` - Agent message model
4. `LlmAgentToolCall` - Tool call tracking model

**Also remove:**
- All Serialize/Deserialize/Clone implementations for these types
- Any associated enums or helper types
- Re-export statements in `lib.rs`

**Why safe to remove:**
- Manager no longer uses these (removed in previous phases)
- nocodo-agents crate has its own session models in `nocodo-agents/src/database/mod.rs`
- No other crates in workspace depend on these manager-specific models

**Verification after Phase 6:**
```bash
cd shared-types && cargo check
cd .. && cargo check --workspace
```

---

### Phase 7: Verification and Testing

**Step 7.1: Compilation check**
```bash
cargo check --workspace
```
**Expected**: All crates compile without errors

**Step 7.2: Run tests**
```bash
cargo test --workspace
```
**Expected**: All tests pass (LLM agent tests already removed)

**Step 7.3: Lint check**
```bash
cargo clippy --workspace
```
**Expected**: No clippy warnings related to removed code

**Step 7.4: Manual verification checklist**
- [ ] No references to `llm_agent` module in manager
- [ ] No references to `llm_client` module in manager
- [ ] No references to `schema_provider` module in manager
- [ ] AppState no longer has `llm_agent` field
- [ ] Database has no LLM agent tables
- [ ] shared-types has no LLM models
- [ ] All tests pass
- [ ] Workspace compiles cleanly

**Step 7.5: Search for any remaining references**
```bash
grep -r "LlmAgent\|llm_agent\|llm_client" . --exclude-dir=target --exclude-dir=.git
```
Should only show references in:
- `nocodo-llm-sdk` crate (expected)
- `nocodo-agents` crate (expected)
- This task file itself
- Git history

---

## Risk Mitigation

### Low Risk (Execute immediately)
- ✅ Phase 1: Removing dead code and tests
  - No active dependencies
  - Marked as dead code

### Medium Risk (Test after each step)
- ⚠️ Phase 2-3: Updating handlers and AppState
  - Affects work creation and message handling
  - No API routes currently expose this (verified)
  - Tool calling already disabled

- ⚠️ Phase 4: Removing implementation files
  - All usages removed in Phase 2-3
  - Verify with grep before deletion

### High Risk (User data affected)
- 🔴 Phase 5: Database table removal
  - Existing LLM session data will be lost
  - **Mitigation**: Backup database before execution
  - **Note**: Agent sessions are separate in nocodo-agents DB

### Cross-package Impact
- ⚠️ Phase 6: Removing from shared-types
  - Affects any crate using those models
  - **Mitigation**: Verified no other crates depend on these
  - nocodo-agents uses its own models

## Rollback Strategy

1. **Git History**: All removed code preserved
   ```bash
   git tag pre-llm-cleanup
   git checkout pre-llm-cleanup -- manager/src/llm_agent.rs  # restore single file
   ```

2. **Phase Independence**: Each phase can be rolled back individually
   - Phases 1-4: Simple file restoration
   - Phase 5: Restore from database backup
   - Phase 6: Restore shared-types models

3. **Database Backup** (before Phase 5):
   ```bash
   cp ~/.local/share/nocodo/manager/manager.db ~/.local/share/nocodo/manager/manager.db.backup
   ```

4. **Workspace State**: Tag before starting
   ```bash
   git tag pre-llm-cleanup
   git commit -m "checkpoint: before LLM cleanup"
   ```

## Success Criteria

### Code Removal
- ✅ All 6 implementation files deleted
- ✅ All module declarations removed from lib.rs
- ✅ LlmAgent removed from AppState and main.rs
- ✅ Work handlers no longer reference LLM code

### Database
- ✅ 3 LLM agent tables dropped
- ✅ 10 database methods removed
- ✅ Migration code added

### Shared Types
- ✅ 4 LLM model structs removed
- ✅ No dependent crates broken

### Testing
- ✅ `cargo check --workspace` passes
- ✅ `cargo test --workspace` passes
- ✅ `cargo clippy --workspace` has no warnings
- ✅ No grep results for removed modules

### Final State
- ✅ Manager ready for fresh nocodo-agents integration
- ✅ Clean separation between work management and agent functionality
- ✅ All LLM operations delegated to nocodo-llm-sdk
- ✅ All agent functionality delegated to nocodo-agents

## Next Steps (Post-Cleanup)

After this cleanup is complete:

1. **Reconnect nocodo-agents**
   - Add nocodo-agents as dependency to manager's Cargo.toml
   - Create new API handlers using AgentFactory
   - Implement work-to-agent integration using nocodo-agents patterns

2. **Create Unified Agent API**
   - New REST endpoints for agent sessions
   - Integration with work items (optional)
   - WebSocket broadcasting for agent responses

3. **Data Migration Tool** (Optional)
   - Tool to migrate old llm_agent_sessions to nocodo-agents database
   - One-time migration script for existing users

4. **Documentation Updates**
   - Update README to reflect new architecture
   - Document agent integration points
   - Add examples of using nocodo-agents with manager

## Notes

- Tool calling is currently disabled in the existing LlmAgent implementation
- No API routes currently expose LLM agent endpoints (verified in routes.rs)
- The nocodo-agents crate has its own separate database at a different path
- This is a preparation step - actual integration with new crates will follow

## References

- **nocodo-llm-sdk**: `/nocodo-llm-sdk/` - LLM client implementations
- **nocodo-agents**: `/nocodo-agents/` - Agent trait and specialized agents
- **Related tasks**:
  - `integrate-sdk-into-manager.md` - Previous SDK integration (completed)
  - `nocodo-llm-sdk-creation.md` - SDK creation task (completed)
