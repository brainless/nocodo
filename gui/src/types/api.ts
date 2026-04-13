/**
 * A Project is a container for related sheets and agent chat sessions.
 * It represents a workspace with its own data storage path.
 */
export type Project = { id: number, name: string, 
/**
 * Path to folder where project data is stored
 */
path: string, created_at: number, };


/**
 * A Sheet is a collection of related tabs (like a database/schema)
 */
export type Sheet = { id: number, project_id: number, name: string, created_at: number, updated_at: number, };


/**
 * A SheetTab is a tab within a sheet (like a table/spreadsheet page)
 */
export type SheetTab = { id: number, sheet_id: number, name: string, display_order: number, created_at: number, updated_at: number, };


/**
 * Column definition (schema) for a sheet tab
 */
export type SheetTabColumn = { id: number, sheet_tab_id: number, name: string, column_type: ColumnType, is_required: boolean, is_unique: boolean, default_value: string | null, display_order: number, created_at: number, 
/**
 * Column width in pixels (user-resizable), default 120
 */
width: number, };


/**
 * Column data types for sheet tabs
 */
export type ColumnType = { "type": "text" } | { "type": "number" } | { "type": "integer" } | { "type": "boolean" } | { "type": "date" } | { "type": "date_time" } | { "type": "currency" } | { "type": "relation", target_sheet_tab_id: number, display_column: string, } | { "type": "lookup", relation_column: string, lookup_column: string, } | { "type": "formula", expression: string, };


/**
 * Create a new project
 */
export type CreateProjectRequest = { name: string, 
/**
 * Path to folder where project data is stored (optional, auto-generated if not provided)
 */
path: string | null, };


export type CreateProjectResponse = { project: Project, };


/**
 * List all projects
 */
export type ListProjectsResponse = { projects: Array<Project>, };


/**
 * List all sheets in a project
 */
export type ListSheetsRequest = { project_id: number, };


export type ListSheetsResponse = { sheets: Array<Sheet>, };


export type GetSheetResponse = { sheet: Sheet, sheet_tabs: Array<SheetTab>, };


/**
 * Get a sheet tab's schema (columns)
 */
export type GetSheetTabSchemaRequest = { sheet_tab_id: number, };


export type GetSheetTabSchemaResponse = { sheet_tab: SheetTab, columns: Array<SheetTabColumn>, };


/**
 * Request data for one or more sheet tabs
 * Returns positional row data (not column-id keyed) for flexible querying
 */
export type GetSheetDataRequest = { 
/**
 * Sheet tab IDs to query (supports multi-table in future)
 */
sheet_tab_ids: number[], limit: number | null, offset: number | null, };


export type GetSheetDataResponse = { 
/**
 * Results for each requested sheet_tab_id
 */
results: Array<SheetTabDataResult>, };


/**
 * Data result for a single sheet tab
 */
export type SheetTabDataResult = { sheet_tab_id: number, 
/**
 * Column definitions (same order as row data)
 */
columns: Array<SheetTabColumn>, 
/**
 * Rows as positional arrays (not keyed by column_id)
 * Each inner array matches the order of `columns`
 * TypeScript: unknown[][] (any JSON value)
 */
rows: unknown[][], pagination: PaginationInfo, };


/**
 * Pagination metadata
 */
export type PaginationInfo = { total_count: number, limit: number, offset: number, has_more: boolean, };


export type HeartbeatResponse = { status: string, service: string, };
