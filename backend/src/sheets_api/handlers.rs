use actix_web::{get, web, HttpResponse, Responder, Result};
use rusqlite::{params, Connection, OptionalExtension};
use shared_types::{
    GetSheetResponse, GetSheetTabDataResponse, GetSheetTabSchemaResponse, ListSheetsResponse,
    Sheet, SheetTab, SheetTabColumn, SheetTabRow,
};

use crate::config;

use super::sheet_record::{
    list_records, AgentChatMessage, AgentChatSession, AgentToolCall, Project, SheetRecord,
};
use super::types::{GetSheetTabDataQuery, ListSheetsQuery};

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
