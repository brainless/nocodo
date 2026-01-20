# Task: Update SQLite Agent to Use Reflect Mode

## Overview
Update the SQLite analysis agent to use the new reflect mode for schema discovery during initialization and expose both query and reflect modes to the LLM for runtime use.

## Prerequisites
- Task 1 (manager-tools) must be completed first
- SqliteMode with query and reflect variants must be available

## Files to Modify

### 1. `nocodo-agents/src/sqlite_reader/mod.rs`

#### Current Agent Structure
```rust
pub struct SqliteReaderAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
    db_path: String,
    system_prompt: String,
}

impl SqliteReaderAgent {
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
        db_path: String,
    ) -> anyhow::Result<Self> {
        validate_db_path(&db_path)?;
        let system_prompt = generate_system_prompt(&db_path);

        Ok(Self {
            client,
            database,
            tool_executor,
            db_path,
            system_prompt,
        })
    }
}
```

#### Updated Agent Structure

**Change `new()` to `async`** to allow calling reflect mode during initialization:

```rust
impl SqliteReaderAgent {
    pub async fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
        db_path: String,
    ) -> anyhow::Result<Self> {
        validate_db_path(&db_path)?;

        // Discover schema using reflect mode
        let schema_info = discover_schema(&tool_executor, &db_path).await?;

        // Generate enhanced system prompt with schema
        let system_prompt = generate_system_prompt_with_schema(&db_path, &schema_info);

        Ok(Self {
            client,
            database,
            tool_executor,
            db_path,
            system_prompt,
        })
    }
}
```

#### Add Schema Discovery Function

```rust
use manager_tools::types::{ToolRequest, Sqlite3ReaderRequest, SqliteMode};

/// Discover database schema using reflect mode during agent initialization
async fn discover_schema(
    executor: &Arc<ToolExecutor>,
    db_path: &str,
) -> anyhow::Result<SchemaInfo> {
    // Call reflect mode to get table list and schema
    let request = ToolRequest::Sqlite3Reader(Sqlite3ReaderRequest {
        db_path: db_path.to_string(),
        mode: SqliteMode::Reflect {
            target: "tables".to_string(),
            table_name: None,
        },
        limit: Some(1000),
    });

    let response = executor.execute(request).await?;

    // Parse the response to extract table information
    let schema_info = parse_schema_response(&response)?;

    Ok(schema_info)
}

#[derive(Debug)]
struct SchemaInfo {
    tables: Vec<TableInfo>,
}

#[derive(Debug)]
struct TableInfo {
    name: String,
    create_sql: Option<String>,
}

fn parse_schema_response(response: &manager_tools::types::ToolResponse) -> anyhow::Result<SchemaInfo> {
    // Parse the reflection output to extract table names and DDL
    // The output format from reflect mode will be structured
    // This is a simple parser - can be enhanced based on actual output format

    let output = &response.output;

    // TODO: Implement proper parsing based on actual reflect mode output
    // For now, create a simple structure

    Ok(SchemaInfo {
        tables: vec![], // Parse from output
    })
}
```

#### Update System Prompt Generation

**Current:**
```rust
fn generate_system_prompt(db_path: &str) -> String {
    format!(
        "You are a database analysis expert specialized in SQLite databases. \
         You are analyzing the database at: {}

Your role is to query data and provide insights about database contents. \
You have access to the sqlite3_reader tool which executes read-only SQL queries.

IMPORTANT: The database path is already configured. You do NOT need to specify \
db_path in your tool calls - just provide the SQL query.",
        db_path
    )
}
```

**New:**
```rust
fn generate_system_prompt_with_schema(db_path: &str, schema_info: &SchemaInfo) -> String {
    let tables_section = if schema_info.tables.is_empty() {
        "No tables found in the database.".to_string()
    } else {
        let table_list = schema_info
            .tables
            .iter()
            .map(|table| {
                if let Some(sql) = &table.create_sql {
                    format!("- {} ({})", table.name, extract_columns_from_ddl(sql))
                } else {
                    format!("- {}", table.name)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("Available Tables:\n{}", table_list)
    };

    format!(
        "You are a database analysis expert specialized in SQLite databases. \
         You are analyzing the database at: {}

DATABASE SCHEMA (discovered at initialization):
{}

Your role is to query data and provide insights about database contents. \
You have access to the sqlite3_reader tool with TWO modes:

1. QUERY MODE - Execute SQL queries
   - Use for: SELECT statements, PRAGMA queries
   - Example: {{\"mode\": \"query\", \"query\": \"SELECT * FROM users LIMIT 5\"}}

2. REFLECT MODE - Introspect database schema at runtime
   - Use for: Discovering tables, getting column info, viewing indexes
   - Targets: \"tables\", \"schema\", \"table_info\", \"indexes\", \"views\"
   - Example: {{\"mode\": \"reflect\", \"target\": \"tables\"}}
   - Example: {{\"mode\": \"reflect\", \"target\": \"table_info\", \"table_name\": \"users\"}}

IMPORTANT: The database path is already configured. You do NOT need to specify \
db_path in your tool calls.

ALLOWED QUERIES (query mode):
- SELECT queries to retrieve data
- PRAGMA queries to inspect schema

You can ONLY use SELECT and PRAGMA statements in query mode. Do NOT use CREATE, \
INSERT, UPDATE, DELETE, ALTER, DROP, or any other statements.

Best Practices:
1. Use the schema information above to construct accurate queries
2. If you need detailed column information, use reflect mode with \"table_info\"
3. Keep queries simple and direct
4. Use LIMIT clauses for large result sets
5. For latest/newest records: use ORDER BY column DESC LIMIT 1
6. For counting: use SELECT COUNT(*) FROM table
7. Answer user questions concisely based on query results

Focus on answering the user's question directly using the schema provided.",
        db_path,
        tables_section
    )
}

fn extract_columns_from_ddl(create_sql: &str) -> String {
    // Simple column extraction from CREATE TABLE statement
    // This can be enhanced with proper SQL parsing if needed

    // For now, return a simplified version
    "columns available".to_string()

    // TODO: Implement proper DDL parsing to extract column names
    // Example output: "id, name, email, created_at"
}
```

### 2. `nocodo-agents/src/factory.rs`

Update the factory function to handle async initialization:

**Current:**
```rust
pub fn create_sqlite_reader_agent(
    llm_client: Arc<dyn LlmClient>,
    db_path: String,
) -> anyhow::Result<Box<dyn Agent>> {
    let database = Arc::new(Database::new(None)?);
    let tool_executor = Arc::new(ToolExecutor::new());

    let agent = SqliteReaderAgent::new(
        llm_client,
        database,
        tool_executor,
        db_path,
    )?;

    Ok(Box::new(agent))
}
```

**New:**
```rust
pub async fn create_sqlite_reader_agent(
    llm_client: Arc<dyn LlmClient>,
    db_path: String,
) -> anyhow::Result<Box<dyn Agent>> {
    let database = Arc::new(Database::new(None)?);
    let tool_executor = Arc::new(ToolExecutor::new());

    let agent = SqliteReaderAgent::new(
        llm_client,
        database,
        tool_executor,
        db_path,
    ).await?;  // Now awaits async initialization

    Ok(Box::new(agent))
}
```

**Note:** This will require updating all call sites to handle async. If this creates too many cascading changes, consider:
- Option A: Keep factory sync, do schema discovery lazily on first run
- Option B: Add a separate `new_with_schema()` async constructor
- Option C: Make the factory async (preferred for consistency)

### 3. `nocodo-agents/src/tools/llm_schemas.rs`

Update the LLM tool definition to expose both modes:

**Current (lines 54-57):**
```rust
AgentTool::Sqlite3Reader => {
    json!({
        "name": "sqlite3_reader",
        "description": "Execute read-only SQL queries on the SQLite database.
                       The database path is pre-configured - only provide the SQL query.
                       Supports SELECT and PRAGMA statements only.
                       Use LIMIT clauses to control result size.",
        "input_schema": {
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "SQL query to execute (SELECT or PRAGMA only)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of rows to return (optional, default: 100)"
                }
            },
            "required": ["query"]
        }
    })
}
```

**New:**
```rust
AgentTool::Sqlite3Reader => {
    json!({
        "name": "sqlite3_reader",
        "description": "Execute SQL queries or reflect on database schema.
                       The database path is pre-configured.

                       TWO MODES:
                       1. QUERY MODE - Execute SELECT/PRAGMA queries
                       2. REFLECT MODE - Introspect database schema

                       Use reflect mode to discover tables, columns, indexes, etc.
                       Use query mode to retrieve actual data.",
        "input_schema": {
            "type": "object",
            "properties": {
                "mode": {
                    "type": "object",
                    "description": "Operation mode: query or reflect",
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "mode": {
                                    "type": "string",
                                    "enum": ["query"],
                                    "description": "Execute a SQL query"
                                },
                                "query": {
                                    "type": "string",
                                    "description": "SQL query to execute (SELECT or PRAGMA only)"
                                }
                            },
                            "required": ["mode", "query"]
                        },
                        {
                            "type": "object",
                            "properties": {
                                "mode": {
                                    "type": "string",
                                    "enum": ["reflect"],
                                    "description": "Reflect on database schema"
                                },
                                "target": {
                                    "type": "string",
                                    "enum": ["tables", "schema", "table_info", "indexes", "views"],
                                    "description": "What to reflect on"
                                },
                                "table_name": {
                                    "type": "string",
                                    "description": "Required for table_info target"
                                }
                            },
                            "required": ["mode", "target"]
                        }
                    ]
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of rows to return (optional)"
                }
            },
            "required": ["mode"]
        }
    })
}
```

### 4. Update CLI Runner (if needed)

**File:** `nocodo-agents/bin/sqlite_reader_runner.rs`

If the factory becomes async, update the runner to await:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ... setup code ...

    let agent = create_sqlite_reader_agent(
        llm_client,
        db_path,
    ).await?;  // Add .await

    // ... rest of code ...
}
```

## Implementation Steps

1. Add `SchemaInfo` and `TableInfo` structs
2. Implement `discover_schema()` function using reflect mode
3. Implement `parse_schema_response()` to extract table info
4. Update `generate_system_prompt()` to `generate_system_prompt_with_schema()`
5. Add `extract_columns_from_ddl()` helper (can be simple initially)
6. Make `SqliteReaderAgent::new()` async
7. Update factory to handle async initialization
8. Update LLM tool schema to expose both modes
9. Update CLI runner if needed
10. Test with real database

## Key Design Decisions

### 1. Initialization Schema Discovery
- **When:** During agent creation (in `new()`)
- **Why:** LLM has context upfront for better query planning
- **Trade-off:** One-time overhead at initialization

### 2. Runtime Reflect Mode Availability
- **LLM can call reflect mode at any time**
- **Use cases:**
  - Get detailed column info: `{"mode": "reflect", "target": "table_info", "table_name": "users"}`
  - Check for indexes: `{"mode": "reflect", "target": "indexes"}`
  - Discover new tables if database changes

### 3. System Prompt Structure
- Include discovered tables in prompt
- Explain both modes clearly
- Provide usage examples
- Keep it concise but informative

## Testing Strategy

### Test Cases

1. **Agent Initialization**
   - Create agent with valid database
   - Verify schema discovery runs
   - Check system prompt includes table list

2. **LLM Query Mode Usage**
   - Agent executes SELECT query
   - Agent executes PRAGMA query

3. **LLM Reflect Mode Usage**
   - Agent calls reflect to list tables
   - Agent calls reflect to get table info
   - Agent calls reflect with invalid target (error handling)

4. **Empty Database**
   - Agent initializes with empty database
   - System prompt shows "No tables found"
   - LLM can still use reflect mode

5. **Error Handling**
   - Invalid database path
   - Schema discovery fails
   - Malformed reflect response

## Success Criteria

- [ ] Agent initialization calls reflect mode successfully
- [ ] System prompt includes discovered table information
- [ ] LLM tool schema exposes both query and reflect modes
- [ ] Agent can be created with async factory
- [ ] CLI runner works with async initialization
- [ ] Both modes are usable by LLM at runtime
- [ ] Error handling for schema discovery failures
- [ ] Empty database case handled gracefully
- [ ] Existing query functionality preserved

## Notes

- The schema discovery happens once at initialization
- LLM can still call reflect mode at runtime for dynamic introspection
- This provides the best of both worlds: upfront context + runtime flexibility
- If schema discovery fails, consider fallback to old prompt without schema
- The reflect mode output should be user-friendly for LLM consumption

## Follow-up Enhancements (Future)

- Cache schema info and refresh periodically
- Support database change notifications
- Add more sophisticated DDL parsing for column extraction
- Include foreign key relationships in schema info
- Add statistics (row counts, index usage) to schema info
