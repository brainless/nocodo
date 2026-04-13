use actix_web::{get, post, web, HttpResponse, Responder, Result};
use rusqlite::{params, Connection, OptionalExtension};
use shared_types::{
    GetSheetDataResponse, GetSheetResponse, GetSheetTabDataResponse, GetSheetTabSchemaResponse,
    ListSheetsResponse, PaginationInfo, Project, Sheet, SheetTab, SheetTabColumn,
    SheetTabDataResult, SheetTabRow,
};

use crate::config;

use super::schema_cache::SchemaCache;
use super::sheet_record::{
    list_records, AgentChatMessage, AgentChatSession, AgentToolCall, SheetRecord,
};
use super::types::{GetSheetDataQuery, GetSheetTabDataQuery, ListSheetsQuery};

fn open_db() -> Result<Connection, rusqlite::Error> {
    let database_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| config::read_project_conf("DATABASE_URL"))
        .unwrap_or_else(|| "nocodo.db".to_string());
    Connection::open(&database_url)
}

/// GET /api/sheets?project_id={id}
/// List all sheets in a project
#[get("/api/sheets")]
pub async fn list_sheets(query: web::Query<ListSheetsQuery>) -> Result<impl Responder> {
    let conn = open_db().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Database error: {}", e))
    })?;

    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, name, created_at, updated_at 
             FROM sheet 
             WHERE project_id = ?1 
             ORDER BY id",
        )
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Prepare error: {}", e))
        })?;

    let sheets = stmt
        .query_map(params![query.project_id], |row| {
            Ok(Sheet {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Query error: {}", e))
        })?;

    let sheets: Vec<Sheet> = sheets.collect::<Result<Vec<_>, _>>().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Collect error: {}", e))
    })?;

    let response = ListSheetsResponse { sheets };
    Ok(HttpResponse::Ok().json(response))
}

/// GET /api/sheets/{id}
/// Get a sheet with all its tabs
#[get("/api/sheets/{sheet_id}")]
pub async fn get_sheet(sheet_id: web::Path<i64>) -> Result<impl Responder> {
    let conn = open_db().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Database error: {}", e))
    })?;

    // Get the sheet
    let sheet: Sheet = conn
        .query_row(
            "SELECT id, project_id, name, created_at, updated_at 
             FROM sheet 
             WHERE id = ?1",
            params![sheet_id.into_inner()],
            |row| {
                Ok(Sheet {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Query error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Sheet not found"))?;

    // Get its tabs
    let mut stmt = conn
        .prepare(
            "SELECT id, sheet_id, name, display_order, created_at, updated_at 
             FROM sheet_tab 
             WHERE sheet_id = ?1 
             ORDER BY display_order, id",
        )
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Prepare error: {}", e))
        })?;

    let sheet_tabs = stmt
        .query_map(params![sheet.id], |row| {
            Ok(SheetTab {
                id: row.get(0)?,
                sheet_id: row.get(1)?,
                name: row.get(2)?,
                display_order: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Query error: {}", e))
        })?;

    let sheet_tabs: Vec<SheetTab> = sheet_tabs.collect::<Result<Vec<_>, _>>().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Collect error: {}", e))
    })?;

    let response = GetSheetResponse {
        sheet,
        sheet_tabs,
    };
    Ok(HttpResponse::Ok().json(response))
}

/// GET /api/sheet-tabs/{id}/schema
/// Get a sheet tab's schema (columns)
#[get("/api/sheet-tabs/{sheet_tab_id}/schema")]
pub async fn get_sheet_tab_schema(sheet_tab_id: web::Path<i64>) -> Result<impl Responder> {
    let conn = open_db().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Database error: {}", e))
    })?;

    let tab_id = sheet_tab_id.into_inner();

    // Get the sheet tab
    let sheet_tab: SheetTab = conn
        .query_row(
            "SELECT id, sheet_id, name, display_order, created_at, updated_at 
             FROM sheet_tab 
             WHERE id = ?1",
            params![tab_id],
            |row| {
                Ok(SheetTab {
                    id: row.get(0)?,
                    sheet_id: row.get(1)?,
                    name: row.get(2)?,
                    display_order: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Query error: {}", e)))?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Sheet tab not found"))?;

    // Get columns
    let mut stmt = conn
        .prepare(
            "SELECT id, sheet_tab_id, name, column_type, is_required, is_unique, default_value, display_order, created_at, width 
             FROM sheet_tab_column 
             WHERE sheet_tab_id = ?1 
             ORDER BY display_order, id",
        )
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Prepare error: {}", e))
        })?;

    let columns = stmt
        .query_map(params![tab_id], |row| {
            let column_type_json: String = row.get(3)?;
            let column_type = serde_json::from_str(&column_type_json).map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(e))
            })?;

            Ok(SheetTabColumn {
                id: row.get(0)?,
                sheet_tab_id: row.get(1)?,
                name: row.get(2)?,
                column_type,
                is_required: row.get::<_, i64>(4)? != 0,
                is_unique: row.get::<_, i64>(5)? != 0,
                default_value: row.get(6)?,
                display_order: row.get(7)?,
                created_at: row.get(8)?,
                width: row.get::<_, Option<i32>>(9)?.unwrap_or(120),
            })
        })
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Query error: {}", e))
        })?;

    let columns: Vec<SheetTabColumn> = columns.collect::<Result<Vec<_>, _>>().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Collect error: {}", e))
    })?;

    let response = GetSheetTabSchemaResponse {
        sheet_tab,
        columns,
    };
    Ok(HttpResponse::Ok().json(response))
}

/// GET /api/sheet-tabs/{id}/data?limit={n}&offset={n}
/// Get row data for a sheet tab
/// 
/// Uses the SheetRecord trait to query actual SQL tables directly.
/// Virtual sheet tab IDs: 6=Projects, 7=Sessions, 8=Messages, 9=Tool Calls
#[get("/api/sheet-tabs/{sheet_tab_id}/data")]
pub async fn get_sheet_tab_data(
    sheet_tab_id: web::Path<i64>,
    query: web::Query<GetSheetTabDataQuery>,
) -> Result<impl Responder> {
    let conn = open_db().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Database error: {}", e))
    })?;

    let tab_id = sheet_tab_id.into_inner();
    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0);

    // Route to the appropriate SheetRecord implementation
    match tab_id {
        6 => {
            // Projects tab
            let (records, total_count) =
                list_records::<Project>(&conn, Some(limit), Some(offset)).map_err(|e| {
                    actix_web::error::ErrorInternalServerError(format!("Query error: {}", e))
                })?;
            let rows = records_to_sheet_rows(tab_id, records);
            Ok(HttpResponse::Ok().json(GetSheetTabDataResponse {
                sheet_tab_id: tab_id,
                rows,
                total_count,
            }))
        }
        7 => {
            // Sessions tab
            let (records, total_count) =
                list_records::<AgentChatSession>(&conn, Some(limit), Some(offset)).map_err(|e| {
                    actix_web::error::ErrorInternalServerError(format!("Query error: {}", e))
                })?;
            let rows = records_to_sheet_rows(tab_id, records);
            Ok(HttpResponse::Ok().json(GetSheetTabDataResponse {
                sheet_tab_id: tab_id,
                rows,
                total_count,
            }))
        }
        8 => {
            // Messages tab
            let (records, total_count) =
                list_records::<AgentChatMessage>(&conn, Some(limit), Some(offset)).map_err(|e| {
                    actix_web::error::ErrorInternalServerError(format!("Query error: {}", e))
                })?;
            let rows = records_to_sheet_rows(tab_id, records);
            Ok(HttpResponse::Ok().json(GetSheetTabDataResponse {
                sheet_tab_id: tab_id,
                rows,
                total_count,
            }))
        }
        9 => {
            // Tool Calls tab
            let (records, total_count) =
                list_records::<AgentToolCall>(&conn, Some(limit), Some(offset)).map_err(|e| {
                    actix_web::error::ErrorInternalServerError(format!("Query error: {}", e))
                })?;
            let rows = records_to_sheet_rows(tab_id, records);
            Ok(HttpResponse::Ok().json(GetSheetTabDataResponse {
                sheet_tab_id: tab_id,
                rows,
                total_count,
            }))
        }
        _ => Err(actix_web::error::ErrorNotFound("Sheet tab not found")),
    }
}

/// Convert SheetRecord structs to SheetTabRow format for API compatibility
///
/// The data field contains JSON with column_id -> value mapping.
/// Uses the SheetRecord's to_column_json method to get properly keyed data.
fn records_to_sheet_rows<T: SheetRecord>(sheet_tab_id: i64, records: Vec<T>) -> Vec<SheetTabRow> {
    records
        .into_iter()
        .map(|record| {
            let data = serde_json::to_string(&record.to_column_json())
                .unwrap_or_else(|_| "{}".to_string());
            let id = record.id();
            let created_at = record.created_at();
            SheetTabRow {
                id,
                sheet_tab_id,
                data,
                created_at,
                updated_at: created_at, // Using created_at as updated_at for now
            }
        })
        .collect()
}

/// POST /api/sheets/data
/// Get row data for one or more sheet tabs using dynamic SQL queries
///
/// This is an alternative to the trait-based get_sheet_tab_data endpoint.
/// It queries actual SQL tables directly based on sheet/sheet_tab metadata.
/// Returns positional row data (not column-id keyed) for flexible querying.
#[post("/api/sheets/data")]
pub async fn get_sheet_data(
    query: web::Query<GetSheetDataQuery>,
    cache: web::Data<SchemaCache>,
) -> Result<impl Responder> {
    // Parse sheet_tab_ids from comma-separated string
    let sheet_tab_ids: Vec<i64> = query
        .sheet_tab_ids
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    if sheet_tab_ids.is_empty() {
        return Err(actix_web::error::ErrorBadRequest(
            "At least one sheet_tab_id is required",
        ));
    }

    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0);

    let conn = open_db().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Database error: {}", e))
    })?;

    let mut results = Vec::new();

    for tab_id in sheet_tab_ids {
        // Get schema info from cache
        let _tab = cache
            .get_sheet_tab(tab_id)
            .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Sheet tab {} not found", tab_id)))?;

        let columns = cache.get_tab_columns(tab_id);
        if columns.is_empty() {
            // Return empty result for tabs with no columns
            results.push(SheetTabDataResult {
                sheet_tab_id: tab_id,
                columns: vec![],
                rows: vec![],
                pagination: PaginationInfo {
                    total_count: 0,
                    limit,
                    offset,
                    has_more: false,
                },
            });
            continue;
        }

        // Get SQL table name
        let table_name = cache
            .get_sql_table_name(tab_id)
            .ok_or_else(|| actix_web::error::ErrorInternalServerError(format!(
                "Could not determine SQL table name for sheet tab {}",
                tab_id
            )))?;

        // Build column list for SELECT using sql_name from cache
        let sql_column_names = cache.get_tab_sql_column_names(tab_id);
        let select_columns: String = sql_column_names.join(", ");

        // Get total count
        let total_count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {}", table_name),
                [],
                |row| row.get(0),
            )
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Count query failed for {}: {}",
                    table_name, e
                ))
            })?;

        // Build and execute SELECT query
        let sql = format!(
            "SELECT {} FROM {} ORDER BY id LIMIT ?1 OFFSET ?2",
            select_columns, table_name
        );

        let mut stmt = conn.prepare(&sql).map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Prepare failed for {}: {}",
                table_name, e
            ))
        })?;

        // Execute query and collect results as positional arrays
        let rows = stmt
            .query_map(params![limit, offset], |row| {
                let mut values = Vec::with_capacity(columns.len());
                for (i, col) in columns.iter().enumerate() {
                    let value = match col.column_type {
                        shared_types::ColumnType::Text => {
                            let s: String = row.get(i)?;
                            serde_json::json!(s)
                        }
                        shared_types::ColumnType::Number | shared_types::ColumnType::Currency => {
                            let n: f64 = row.get(i)?;
                            serde_json::json!(n)
                        }
                        shared_types::ColumnType::Integer => {
                            let n: i64 = row.get(i)?;
                            serde_json::json!(n)
                        }
                        shared_types::ColumnType::Boolean => {
                            let b: i64 = row.get(i)?;
                            serde_json::json!(b != 0)
                        }
                        shared_types::ColumnType::Date | shared_types::ColumnType::DateTime => {
                            let n: i64 = row.get(i)?;
                            serde_json::json!(n)
                        }
                        _ => {
                            // For Relation, Lookup, Formula - get as text or number
                            let s: Result<String, _> = row.get(i);
                            match s {
                                Ok(s) => serde_json::json!(s),
                                Err(_) => {
                                    let n: Result<i64, _> = row.get(i);
                                    match n {
                                        Ok(n) => serde_json::json!(n),
                                        Err(_) => serde_json::Value::Null,
                                    }
                                }
                            }
                        }
                    };
                    values.push(value);
                }
                Ok(values)
            })
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Query failed for {}: {}",
                    table_name, e
                ))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Row mapping failed for {}: {}",
                    table_name, e
                ))
            })?;

        let has_more = (offset + rows.len() as i64) < total_count;

        results.push(SheetTabDataResult {
            sheet_tab_id: tab_id,
            columns: columns.into_iter().cloned().collect(),
            rows,
            pagination: PaginationInfo {
                total_count,
                limit,
                offset,
                has_more,
            },
        });
    }

    Ok(HttpResponse::Ok().json(GetSheetDataResponse { results }))
}
