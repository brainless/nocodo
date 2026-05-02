/**
 * A Project is a container for related schemas and agent chat sessions.
 */
export type Project = { id: number, name: string, 
/**
 * Path to folder where project data is stored
 */
path: string, created_at: number, };


/**
 * A Schema is a named collection of tables within a project.
 */
export type Schema = { id: number, project_id: number, name: string, created_at: number, };


/**
 * A Table is a relational table within a schema.
 */
export type Table = { id: number, schema_id: number, name: string, created_at: number, };


/**
 * Storage-level column data type.
 */
export type DataType = "text" | "integer" | "real" | "boolean" | "date" | "date_time";


/**
 * A Column in a relational table.
 */
export type Column = { id: number, table_id: number, name: string, data_type: DataType, nullable: boolean, primary_key: boolean, 
/**
 * Defines column order in SELECT queries and UI display.
 */
display_order: number, created_at: number, };


/**
 * A persisted foreign key constraint on a column.
 */
export type ForeignKey = { id: number, column_id: number, 
/**
 * SQL name of the referenced table.
 */
ref_table: string, 
/**
 * Name of the referenced column (usually "id").
 */
ref_column: string, };


/**
 * UI display metadata for a column. Decoupled from the relational schema.
 */
export type ColumnDisplay = { id: number, column_id: number, 
/**
 * Column width in pixels (user-resizable), default 120.
 */
width: number, 
/**
 * For FK columns: which column of the referenced table to show as the link label.
 */
display_column: string | null, };


/**
 * Available agent types in the multi-agent system.
 */
export type AgentType = "project_manager" | "schema_designer" | "backend_developer" | "frontend_developer";


/**
 * Foreign key reference by name — resolved to IDs on persist.
 */
export type ForeignKeyDef = { 
/**
 * SQL name of the referenced table.
 */
ref_table: string, 
/**
 * Name of the referenced column (usually "id").
 */
ref_column: string, };


/**
 * Column definition as emitted by the agent.
 */
export type ColumnDef = { name: string, label: string | null, data_type: DataType, nullable: boolean, primary_key: boolean, foreign_key: ForeignKeyDef | null, };


/**
 * Table definition as emitted by the agent.
 */
export type TableDef = { name: string, label: string | null, columns: Array<ColumnDef>, };


/**
 * Complete schema definition — the agent emits this via the `generate_schema` tool.
 * Each call produces a new versioned snapshot stored in `project_schema`.
 */
export type SchemaDef = { 
/**
 * Human-readable schema name.
 */
name: string, label: string | null, 
/**
 * Normalized set of tables that make up the schema.
 */
tables: Array<TableDef>, };


export type ListSchemasResponse = { schemas: Array<Schema>, };


export type GetSchemaResponse = { schema: Schema, tables: Array<Table>, };


export type GetTableColumnsResponse = { table: Table, columns: Array<Column>, };


/**
 * Pagination metadata
 */
export type PaginationInfo = { total_count: number, limit: number, offset: number, has_more: boolean, };


/**
 * Data result for a single table
 */
export type TableDataResult = { table_id: number, 
/**
 * Column definitions in display order
 */
columns: Array<Column>, 
/**
 * Rows as positional arrays matching the order of `columns`
 */
rows: unknown[][], pagination: PaginationInfo, };


export type GetTableDataResponse = { results: Array<TableDataResult>, };


export type CreateProjectRequest = { name: string, path: string | null, };


export type CreateProjectResponse = { project: Project, };


export type ListProjectsResponse = { projects: Array<Project>, };


export type HeartbeatResponse = { status: string, service: string, };


export type TaskItem = { id: number, project_id: number, epic_id: number | null, title: string, source_prompt: string, assigned_to_agent: string, status: string, created_at: number, updated_at: number, };


export type ListTasksResponse = { tasks: Array<TaskItem>, };


export type EpicItem = { id: number, project_id: number, title: string, description: string, status: string, created_by_agent: string, created_by_task_id: number | null, created_at: number, updated_at: number, };


export type ListEpicsResponse = { epics: Array<EpicItem>, };
