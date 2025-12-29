# Task: Implement Reflect Mode for SQLite Agent

## Status
✅ Completed

## Overview
Updated the SQLite analysis agent to use the new reflect mode for schema discovery during agent initialization and expose both query and reflect modes to the LLM for runtime use.

## Changes Made

### 1. Agent Initialization (nocodo-agents/src/sqlite_analysis/mod.rs)

#### Made Constructor Async
- Changed `SqliteAnalysisAgent::new()` from sync to `async fn`
- Allows schema discovery to happen during initialization

#### Added Schema Discovery
- Implemented `discover_schema()` function that calls reflect mode with `target: "tables"`
- Executes: `SELECT name, sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'`
- Retrieves table names and DDL statements for all user tables

#### Added Response Parsing
- Implemented `parse_schema_response()` to extract table info from ToolResponse
- Implemented `parse_tables_from_reflection()` to parse formatted table output
- Handles tabular output with name and sql columns
- Extracts table names and CREATE TABLE statements

#### Added DDL Parsing Helpers
- `extract_table_name_from_create_sql()` - Extracts table name from CREATE TABLE statement
- `extract_columns_from_ddl()` - Extracts column names from DDL (up to 10, truncates if more)
- Handles quoted table names: `[user_data]`, `"users"`, etc.
- Handles `IF NOT EXISTS` clause

#### Updated System Prompt Generation
- Changed from `generate_system_prompt()` to `generate_system_prompt_with_schema()`
- Includes discovered tables in the system prompt
- Lists available tables with their columns
- Explains both query and reflect modes with examples
- Provides clear usage instructions for LLM

#### Error Handling
- Graceful fallback if schema discovery fails
- Logs warning and continues with empty schema
- System prompt shows "No tables found" if discovery fails

### 2. Factory Update (nocodo-agents/src/factory.rs)

#### Made Factory Function Async
- Changed `create_sqlite_analysis_agent()` to `pub async fn`
- Now awaits the agent's async initialization

### 3. CLI Runner Update (nocodo-agents/bin/sqlite_analysis_runner.rs)

#### Added Async Await
- Updated agent creation to use `.await`
- Properly handles async factory function

### 4. Test Coverage (nocodo-agents/src/sqlite_analysis/tests.rs)

Added comprehensive unit tests:

- `test_parse_tables_from_reflection()` - Parses table reflection output
- `test_extract_table_name_from_create_sql()` - Extracts table names from DDL
- `test_extract_columns_from_ddl()` - Extracts column names
- `test_extract_columns_from_ddl_long()` - Handles truncation for many columns
- `test_schema_info_empty()` - Tests empty schema case
- `test_generate_system_prompt_with_schema()` - Verifies prompt includes tables
- `test_generate_system_prompt_empty_schema()` - Verifies empty schema message

All tests pass successfully.

## System Prompt Structure

The enhanced system prompt includes:

1. **Database path** (as before)
2. **DATABASE SCHEMA section** - Lists discovered tables with columns
3. **Two-mode tool description**:
   - **Query Mode**: Execute SELECT and PRAGMA statements
   - **Reflect Mode**: Introspect schema (tables, schema, table_info, indexes, views, foreign_keys, stats)
4. **Usage examples** for both modes
5. **Best practices** for query construction
6. **Allowed queries** (SELECT, PRAGMA only)

## Benefits

### Initialization-Time Context
- LLM receives complete table list upfront
- Better query planning with known schema
- Reduces unnecessary reflect calls at runtime

### Runtime Flexibility
- LLM can still call reflect mode for:
  - Detailed column information: `{"mode": "reflect", "target": "table_info", "table_name": "users"}`
  - Index inspection: `{"mode": "reflect", "target": "indexes"}`
  - Foreign key relationships: `{"mode": "reflect", "target": "foreign_keys", "table_name": "posts"}`
  - Database statistics: `{"mode": "reflect", "target": "stats"}`
  - Schema discovery if database changes

### Error Resilience
- Graceful degradation if schema discovery fails
- Agent still functional with fallback prompt
- Warning logs for debugging

## Technical Details

### Schema Discovery Flow

```
Agent::new()
  ↓
discover_schema() [async]
  ↓
ToolRequest::Sqlite3Reader(Reflect { target: "tables" })
  ↓
ToolExecutor::execute()
  ↓
manager_tools::sqlite::execute_sqlite3_reader()
  ↓
build_reflection_query("tables")
  → SELECT name, sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'
  ↓
SqlExecutor::execute()
  ↓
parse_schema_response()
  ↓
parse_tables_from_reflection()
  → Extract table names and CREATE statements
  ↓
generate_system_prompt_with_schema()
```

### Reflect Mode Targets Available

The LLM can use reflect mode with these targets:

- `tables` - List all tables with DDL (used at initialization)
- `schema` - Full schema dump (tables, indexes, views)
- `table_info` - Column information for specific table
- `indexes` - List all indexes
- `views` - List all views
- `foreign_keys` - Foreign key relationships for table
- `stats` - Database statistics and table counts

## Dependencies

This task depends on:

1. **manager-tools** SqliteMode with reflect variant (already implemented)
2. **manager-tools** sqlite3_reader tool with reflect mode (already implemented)

## Testing

### Unit Tests
- All 13 unit tests pass
- Tests cover schema parsing, DDL extraction, and prompt generation

### Integration Tests
- Existing integration tests (`test_count_users_integration`, `test_latest_user_registration_integration`)
  still pass
- These use `new_for_testing()` which skips schema discovery
- Integration tests require API keys and are ignored by default

### Manual Testing
Run the CLI to test with a real database:

```bash
cargo run --bin sqlite-analysis-runner \
  --prompt "Show me all tables and their schemas" \
  --config path/to/config.toml \
  --db-path /path/to/database.db
```

## Files Modified

1. `nocodo-agents/src/sqlite_analysis/mod.rs` - Core agent implementation
2. `nocodo-agents/src/factory.rs` - Factory function
3. `nocodo-agents/bin/sqlite_analysis_runner.rs` - CLI runner
4. `nocodo-agents/src/sqlite_analysis/tests.rs` - Unit tests

## Lines Changed

- 329 insertions
- 19 deletions
- Total: 348 lines modified

## Success Criteria Met

✅ Agent initialization calls reflect mode successfully
✅ System prompt includes discovered table information
✅ LLM tool schema exposes both query and reflect modes
✅ Agent can be created with async factory
✅ CLI runner works with async initialization
✅ Both modes are usable by LLM at runtime
✅ Error handling for schema discovery failures
✅ Empty database case handled gracefully
✅ Existing query functionality preserved
✅ Comprehensive unit test coverage

## Follow-up Enhancements (Future)

Potential improvements to consider:

1. **Schema Caching** - Cache schema info and refresh periodically for long-running sessions
2. **Change Detection** - Support database change notifications to refresh schema
3. **Enhanced DDL Parsing** - Include foreign key relationships, indexes, and constraints in schema info
4. **Statistics Integration** - Add table row counts, index usage statistics to schema info
5. **Smart Schema Refresh** - Detect when database changes and update schema automatically
6. **Multi-line DDL** - Better parsing of complex CREATE TABLE statements spanning multiple lines
