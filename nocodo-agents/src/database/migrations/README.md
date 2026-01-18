# Database Migrations

This directory contains database schema migrations for the nocodo-agents crate, managed using [Refinery](https://github.com/rust-db/refinery).

## Overview

Migrations are defined as **Rust modules** (not SQL files) that generate SQLite DDL statements on demand. This approach:

- Keeps schema definitions as Rust code
- Delays SQL generation until runtime
- Makes migrations easy to version control and review
- Allows for future database dialect support

## Migration Files

Migrations must follow the naming format: `V{version}__{name}.rs`

- `V` indicates a contiguous (sequential) migration
- `{version}` is the migration version number (1, 2, 3, etc.)
- `{name}` is a descriptive name using snake_case
- Each migration module must export a `migration()` function that returns a `String`

### Current Migrations

1. **V1__create_agent_sessions.rs** - Core agent session tracking
2. **V2__create_agent_messages.rs** - Conversation message storage
3. **V3__create_agent_tool_calls.rs** - Tool execution tracking
4. **V4__create_project_requirements_qna.rs** - Requirements gathering Q&A (agent-specific)

## Adding New Migrations

### For Core Agent Features

Core features used by all agents should be added to this directory:

```rust
// V5__create_new_core_table.rs

/// Description of what this migration does
pub fn migration() -> String {
    r#"
CREATE TABLE new_core_table (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- columns here
);

CREATE INDEX IF NOT EXISTS idx_new_core_table
    ON new_core_table(some_column);
"#.to_string()
}
```

### For Agent-Specific Features

Agent-specific tables should also be added here, but clearly documented:

```rust
// V6__create_agent_specific_table.rs

/// Description of what this migration does
///
/// **Agent-Specific Migration**: This table is specific to the {agent_name} agent.
/// Applications that don't use this agent can skip this migration.
pub fn migration() -> String {
    r#"
CREATE TABLE agent_specific_table (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- columns here
);
"#.to_string()
}
```

## Running Migrations

### From Within nocodo-agents

Migrations run automatically when creating a new Database instance:

```rust
use nocodo_agents::database::Database;
use std::path::PathBuf;

let db = Database::new(&PathBuf::from("path/to/db.sqlite"))?;
// Migrations have been applied
```

### From External Projects

External projects can use the public migration API:

```rust
use rusqlite::Connection;
use nocodo_agents::database::migrations::run_agent_migrations;

let mut conn = Connection::open("mydb.db")?;
run_agent_migrations(&mut conn)?;
```

### With Other Databases

Currently generates SQLite dialect only. Support for PostgreSQL, MySQL, etc. can be added by:

1. Creating database-specific migration modules
2. Detecting the database type
3. Calling the appropriate migration function

Example for future PostgreSQL support:

```rust
// V1__create_agent_sessions_postgres.rs
pub fn migration() -> String {
    r#"
CREATE TABLE agent_sessions (
    id SERIAL PRIMARY KEY,
    -- PostgreSQL specific syntax
);
"#.to_string()
}
```

## Migration Tracking

Refinery maintains a `refinery_schema_history` table that tracks:

- Migration version
- Migration name
- Checksum (to detect modifications)
- Applied timestamp

**Important**: Never modify an applied migration! Instead, create a new migration to alter the schema.

## Testing

See `../migrations_test.rs` for examples of testing migrations:

```bash
cargo test migrations
```

## Database Support

Current implementation uses Refinery 0.9, which supports:

- **SQLite** (rusqlite 0.37)
- **PostgreSQL** (sync and async)
- **MySQL** (sync and async)
- **SQL Server** (tiberius)

The same migration files can work across all databases once dialect-specific SQL is implemented.

## Best Practices

1. **Never modify applied migrations** - Create new migrations instead
2. **Use descriptive names** - Make it clear what each migration does
3. **Document agent-specific tables** - Help users understand dependencies
4. **Test thoroughly** - Verify migrations work on fresh and existing databases
5. **Keep migrations small** - One logical change per migration
6. **Add indexes** - Include relevant indexes for performance

## References

- [Refinery Documentation](https://github.com/rust-db/refinery)
- [Refinery Rust Docs](https://docs.rs/refinery/)
