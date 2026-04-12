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
export type SheetTabColumn = { id: number, sheet_tab_id: number, name: string, column_type: ColumnType, is_required: boolean, is_unique: boolean, default_value: string | null, display_order: number, created_at: number, };


/**
 * A row stores JSON data keyed by column_id
 * Example: {"1": "Alice", "2": "Acme Inc", "3": "qualified"}
 */
export type SheetTabRow = { id: number, sheet_tab_id: number, 
/**
 * JSON map: column_id (as string) -> cell value
 */
data: string, created_at: number, updated_at: number, };


/**
 * Column data types for sheet tabs
 */
export type ColumnType = { "type": "text" } | { "type": "number" } | { "type": "integer" } | { "type": "boolean" } | { "type": "date" } | { "type": "date_time" } | { "type": "currency" } | { "type": "relation", target_sheet_tab_id: number, display_column: string, } | { "type": "lookup", relation_column: string, lookup_column: string, } | { "type": "formula", expression: string, };


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
 * Get row data for a sheet tab (paginated)
 */
export type GetSheetTabDataRequest = { sheet_tab_id: number, limit: number | null, offset: number | null, };


export type GetSheetTabDataResponse = { sheet_tab_id: number, rows: Array<SheetTabRow>, total_count: number, };


export type HeartbeatResponse = { status: string, service: string, };
