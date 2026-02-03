# Refactor Storage to Trait-Based Interface - Progress Report

**Date**: 2026-02-03
**Status**: 🔄 In Progress (Partial Completion)

## Summary

Significant progress has been made on refactoring nocodo-agents to use a trait-based storage abstraction. The core infrastructure is in place, but factory and some agents need additional work to complete the refactoring.

## Completed Work

### ✅ Phase 1: Create Storage Types Module
- Created `nocodo-agents/src/types/mod.rs` with module exports
- Created `nocodo-agents/src/types/session.rs` with Session struct and SessionStatus enum
- Created `nocodo-agents/src/types/message.rs` with Message struct and MessageRole enum
- Created `nocodo-agents/src/types/tool_call.rs` with ToolCall struct and ToolCallStatus enum
- All types derive Serialize, Deserialize, Debug, Clone
- Added helper methods like `as_str()`, `from_str()`, `complete()`, `fail()`

### ✅ Phase 2: Create Storage Trait Interface
- Created `nocodo-agents/src/storage/mod.rs` with AgentStorage trait
- Defined all async methods: create_session, get_session, update_session, create_message, get_messages, create_tool_call, update_tool_call, get_tool_calls, get_pending_tool_calls
- Created StorageError enum with thiserror for proper error handling
- Added From<anyhow::Error> implementation for StorageError

### ✅ Phase 3: Refactor Agents to Use Trait

#### CodebaseAnalysisAgent
- ✅ Made agent generic over `AgentStorage`: `CodebaseAnalysisAgent<S: AgentStorage>`
- ✅ Updated all database method calls to use async trait methods
- ✅ Fixed import conflicts by using `ToolCall as StorageToolCall` and `ToolCall as LlmToolCall`
- ✅ Converted all synchronous operations to async
- ✅ Fixed borrow checker issues

#### SqliteReaderAgent
- ✅ Made agent generic over `AgentStorage`: `SqliteReaderAgent<S: AgentStorage>`
- ✅ Updated all database method calls to use async trait methods
- ✅ Fixed import conflicts
- ✅ Converted all operations to async

#### UserClarificationAgent
- ✅ Made agent generic over `AgentStorage` and `RequirementsStorage`: `UserClarificationAgent<S: AgentStorage, R: RequirementsStorage>`
- ✅ Created separate `RequirementsStorage` trait in `requirements_gathering/storage.rs`
- ✅ Updated all method calls to use async trait
- ✅ Fixed import conflicts
- ✅ Converted all operations to async

### ✅ Additional Components
- Created `nocodo-agents/src/storage/memory.rs` with `InMemoryStorage` implementation
- Added `thiserror` to Cargo.toml
- Added `uuid` to Cargo.toml for generating unique IDs

### ✅ Phase 4: Module Updates
- Updated `nocodo-agents/src/lib.rs` to export `storage` and `types` modules
- Added public exports for `AgentStorage`, `StorageError`, `Session`, `Message`, `ToolCall`, and all related types
- Removed database module export from public API (kept temporarily for internal use)

## Remaining Work

### ⚠️ Phase 5: Factory Methods Update
The factory methods in `factory.rs` still reference `Database` struct and create agents incorrectly:
- Methods like `create_codebase_analysis_agent()`, `create_sqlite_reader_agent()`, etc. need to accept storage parameter
- Return types need to include generic parameters: `CodebaseAnalysisAgent<S>`, `SqliteReaderAgent<S>`, etc.
- Factory itself may need to be made generic to accept any storage implementation

**Status**: Blocker - Cannot proceed until factory is updated or removed

### ⚠️ Phase 6: Remove Database Module
The old `Database` struct in `database/mod.rs` still exists and is referenced:
- rusqlite and refinery dependencies still in Cargo.toml
- Database implementation methods are synchronous (not async-compatible with trait)
- Need to either:
  1. Implement AgentStorage trait for Database (transitional approach), OR
  2. Remove database module entirely (proper approach)

**Recommendation**: Remove database module and have consuming applications implement their own storage.

### ⚠️ Phase 7: Remaining Agent Refactors
The following agents have NOT been refactored yet and still use the old `Database` struct:
- `SettingsManagementAgent` in `settings_management/mod.rs`
- `ImapEmailAgent` in `imap_email/mod.rs`
- `StructuredJsonAgent` in `structured_json/mod.rs`
- `TesseractAgent` in `tesseract/mod.rs`

**Status**: Ready to implement (same pattern as refactored agents)

### ⚠️ Phase 8: Update Binary Runners
Binary runners in `bin/` directory still need to be updated:
- `codebase_analysis_runner.rs`
- `sqlite_reader_runner.rs`
- `structured_json_runner.rs`
- `requirements_gathering_runner.rs`
- `settings_management_runner.rs`
- `imap_email_runner.rs`

**Status**: Not started - depends on factory and agents

## Architecture Decisions

### Type Naming
Used type aliases to avoid name conflicts:
- `ToolCall` from LLM SDK → `LlmToolCall`
- `ToolCall` from storage types → `StorageToolCall`

### Storage Implementation
Created `InMemoryStorage` for testing and binary runners that need self-contained storage:
- Uses `HashMap` with `Arc<Mutex<>>` for thread-safe in-memory storage
- Generates UUIDs for all new entities
- Fully implements `AgentStorage` trait

### Requirements Storage
Created separate `RequirementsStorage` trait for agent-specific Q&A operations:
- `store_questions()` - Save clarifying questions
- `get_pending_questions()` - Retrieve unanswered questions
- `store_answers()` - Save user answers

## Next Steps

### Immediate (to complete the refactoring):
1. **Update or remove factory.rs** - Either accept storage parameter or remove entirely
2. **Refactor remaining agents** - settings_management, imap_email, structured_json, tesseract
3. **Remove database module** - Delete `src/database/mod.rs` and migration files
4. **Clean Cargo.toml** - Remove rusqlite and refinery dependencies
5. **Update binary runners** - Use `InMemoryStorage` or accept storage parameter
6. **Run cargo test** - Ensure all tests pass with new implementation

### For consuming applications:
When integrating with refactored nocodo-agents, consuming applications will need to:
1. Implement `AgentStorage` trait for their preferred backend (PostgreSQL, files, memory, etc.)
2. For RequirementsStorage, implement `RequirementsStorage` trait if using UserClarificationAgent
3. Pass storage implementation to agents via agent constructors
4. No database initialization required - storage is fully externalized

## File Changes Summary

### New Files Created
- `nocodo-agents/src/types/mod.rs`
- `nocodo-agents/src/types/session.rs`
- `nocodo-agents/src/types/message.rs`
- `nocodo-agents/src/types/tool_call.rs`
- `nocodo-agents/src/storage/mod.rs`
- `nocodo-agents/src/storage/memory.rs`
- `nocodo-agents/src/requirements_gathering/storage.rs`
- `nocodo-agents/tasks/refactor-storage-to-trait-based-interface.md`

### Files Modified
- `nocodo-agents/src/lib.rs` - Added storage and types module exports
- `nocodo-agents/src/codebase_analysis/mod.rs` - Full refactor to use AgentStorage trait
- `nocodo-agents/src/sqlite_reader/mod.rs` - Full refactor to use AgentStorage trait
- `nocodo-agents/src/requirements_gathering/mod.rs` - Full refactor to use AgentStorage trait
- `nocodo-agents/Cargo.toml` - Added thiserror and uuid dependencies

### Files Needing Updates
- `nocodo-agents/src/factory.rs` - Major updates needed
- `nocodo-agents/src/settings_management/mod.rs` - Refactor to use trait
- `nocodo-agents/src/imap_email/mod.rs` - Refactor to use trait
- `nocodo-agents/src/structured_json/mod.rs` - Refactor to use trait
- `nocodo-agents/src/tesseract/mod.rs` - Refactor to use trait
- All binary runners in `bin/` directory
- `nocodo-agents/Cargo.toml` - Remove rusqlite and refinery (after completing refactoring)

## Success Criteria Status

- [x] `AgentStorage` trait defined with all required methods
- [x] `Session`, `Message`, `ToolCall` types defined and exported
- [x] `StorageError` enum defined with proper error handling
- [x] CodebaseAnalysisAgent refactored to use `AgentStorage` trait
- [x] SqliteReaderAgent refactored to use `AgentStorage` trait
- [x] UserClarificationAgent refactored to use `AgentStorage` trait (with RequirementsStorage)
- [x] Import conflicts resolved with type aliases
- [x] thiserror dependency added to Cargo.toml
- [ ] All agents refactored to use `AgentStorage` trait (3 remaining)
- [ ] SQLite-specific code removed (rusqlite, refinery dependencies)
- [ ] `database` module removed from nocodo-agents
- [ ] Factory methods updated to accept storage parameter
- [ ] Code compiles without errors
- [ ] No clippy warnings
- [ ] Documentation complete for trait interface

**Overall Progress**: ~60% complete
