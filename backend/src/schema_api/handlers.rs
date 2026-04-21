use actix_web::{get, post, web, HttpResponse, Responder, Result};
use rusqlite::{params, Connection, OptionalExtension};
use shared_types::{
    Column, DataType, ForeignKey, GetSchemaResponse, GetTableColumnsResponse,
    GetTableDataResponse, ListSchemasResponse, PaginationInfo, Schema, Table, TableDataResult,
};

use crate::config;

use super::schema_cache::SchemaCache;
use super::types::{GetTableDataQuery, ListSchemasQuery};

fn open_db() -> Result<Connection, rusqlite::Error> {
    let database_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| config::read_project_conf("DATABASE_URL"))
        .unwrap_or_else(|| "nocodo.db".to_string());
    Connection::open(&database_url)
}

fn db_err(e: impl std::fmt::Display) -> actix_web::Error {
    actix_web::error::ErrorInternalServerError(format!("Database error: {}", e))
}

/// GET /api/schemas?project_id={id}
#[get("/api/schemas")]
pub async fn list_schemas(query: web::Query<ListSchemasQuery>) -> Result<impl Responder> {
    let conn = open_db().map_err(db_err)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, name, created_at
             FROM app_schema
             WHERE project_id = ?1
             ORDER BY id",
        )
        .map_err(db_err)?;

    let schemas: Vec<Schema> = stmt
        .query_map(params![query.project_id], |row| {
            Ok(Schema {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(db_err)?
        .collect::<std::result::Result<_, _>>()
        .map_err(db_err)?;

    Ok(HttpResponse::Ok().json(ListSchemasResponse { schemas }))
}

/// GET /api/schemas/{id}
#[get("/api/schemas/{schema_id}")]
pub async fn get_schema(schema_id: web::Path<i64>) -> Result<impl Responder> {
    let conn = open_db().map_err(db_err)?;
    let sid = schema_id.into_inner();

    let schema: Schema = conn
        .query_row(
            "SELECT id, project_id, name, created_at FROM app_schema WHERE id = ?1",
            params![sid],
            |row| {
                Ok(Schema {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(db_err)?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Schema not found"))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, schema_id, name, created_at
             FROM schema_table
             WHERE schema_id = ?1
             ORDER BY id",
        )
        .map_err(db_err)?;

    let tables: Vec<Table> = stmt
        .query_map(params![sid], |row| {
            Ok(Table {
                id: row.get(0)?,
                schema_id: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(db_err)?
        .collect::<std::result::Result<_, _>>()
        .map_err(db_err)?;

    Ok(HttpResponse::Ok().json(GetSchemaResponse { schema, tables }))
}

/// GET /api/tables/{id}/columns
#[get("/api/tables/{table_id}/columns")]
pub async fn get_table_columns(table_id: web::Path<i64>) -> Result<impl Responder> {
    let conn = open_db().map_err(db_err)?;
    let tid = table_id.into_inner();

    let table: Table = conn
        .query_row(
            "SELECT id, schema_id, name, created_at FROM schema_table WHERE id = ?1",
            params![tid],
            |row| {
                Ok(Table {
                    id: row.get(0)?,
                    schema_id: row.get(1)?,
                    name: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(db_err)?
        .ok_or_else(|| actix_web::error::ErrorNotFound("Table not found"))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, table_id, name, data_type, nullable, primary_key, display_order, created_at
             FROM schema_column
             WHERE table_id = ?1
             ORDER BY display_order, id",
        )
        .map_err(db_err)?;

    let columns: Vec<Column> = stmt
        .query_map(params![tid], |row| {
            let data_type_str: String = row.get(3)?;
            let data_type = super::schema_cache::data_type_from_str(&data_type_str);
            Ok(Column {
                id: row.get(0)?,
                table_id: row.get(1)?,
                name: row.get(2)?,
                data_type,
                nullable: row.get::<_, i64>(4)? != 0,
                primary_key: row.get::<_, i64>(5)? != 0,
                display_order: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(db_err)?
        .collect::<std::result::Result<_, _>>()
        .map_err(db_err)?;

    Ok(HttpResponse::Ok().json(GetTableColumnsResponse { table, columns }))
}

/// GET /api/tables/{id}/foreign-keys
#[get("/api/tables/{table_id}/foreign-keys")]
pub async fn get_table_foreign_keys(table_id: web::Path<i64>) -> Result<impl Responder> {
    let conn = open_db().map_err(db_err)?;
    let tid = table_id.into_inner();

    let mut stmt = conn
        .prepare(
            "SELECT fk.id, fk.column_id, fk.ref_table, fk.ref_column
             FROM schema_fk fk
             JOIN schema_column c ON c.id = fk.column_id
             WHERE c.table_id = ?1",
        )
        .map_err(db_err)?;

    let fks: Vec<ForeignKey> = stmt
        .query_map(params![tid], |row| {
            Ok(ForeignKey {
                id: row.get(0)?,
                column_id: row.get(1)?,
                ref_table: row.get(2)?,
                ref_column: row.get(3)?,
            })
        })
        .map_err(db_err)?
        .collect::<std::result::Result<_, _>>()
        .map_err(db_err)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "foreign_keys": fks })))
}

/// POST /api/tables/data?table_ids=1,2&limit=100&offset=0
/// Returns row data for one or more tables using dynamic SQL.
#[post("/api/tables/data")]
pub async fn get_table_data(
    query: web::Query<GetTableDataQuery>,
    cache: web::Data<SchemaCache>,
) -> Result<impl Responder> {
    let table_ids: Vec<i64> = query
        .table_ids
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    if table_ids.is_empty() {
        return Err(actix_web::error::ErrorBadRequest(
            "At least one table_id is required",
        ));
    }

    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0);

    let conn = open_db().map_err(db_err)?;
    let mut results = Vec::new();

    for table_id in table_ids {
        let _table = cache
            .get_table(table_id)
            .ok_or_else(|| actix_web::error::ErrorNotFound(format!("Table {} not found", table_id)))?;

        let columns = cache.get_table_columns(table_id);
        if columns.is_empty() {
            results.push(TableDataResult {
                table_id,
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

        let sql_table = cache
            .get_sql_table_name(table_id)
            .ok_or_else(|| actix_web::error::ErrorInternalServerError(
                format!("Cannot resolve SQL table name for table {}", table_id),
            ))?;

        let sql_columns: Vec<String> = cache.get_table_sql_column_names(table_id);
        let select_cols = sql_columns.join(", ");

        let total_count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {}", sql_table),
                [],
                |row| row.get(0),
            )
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Count query failed for {}: {}", sql_table, e),
            ))?;

        let sql = format!(
            "SELECT {} FROM {} ORDER BY id LIMIT ?1 OFFSET ?2",
            select_cols, sql_table
        );

        let mut stmt = conn.prepare(&sql).map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Prepare failed: {}", e))
        })?;

        let rows = stmt
            .query_map(params![limit, offset], |row| {
                let mut values = Vec::with_capacity(columns.len());
                for (i, col) in columns.iter().enumerate() {
                    let value = match col.data_type {
                        DataType::Text => {
                            let s: String = row.get(i)?;
                            serde_json::json!(s)
                        }
                        DataType::Real => {
                            let n: f64 = row.get(i)?;
                            serde_json::json!(n)
                        }
                        DataType::Integer => {
                            let n: i64 = row.get(i)?;
                            serde_json::json!(n)
                        }
                        DataType::Boolean => {
                            let b: i64 = row.get(i)?;
                            serde_json::json!(b != 0)
                        }
                        DataType::Date | DataType::DateTime => {
                            let n: i64 = row.get(i)?;
                            serde_json::json!(n)
                        }
                    };
                    values.push(value);
                }
                Ok(values)
            })
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Query failed for {}: {}", sql_table, e),
            ))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| actix_web::error::ErrorInternalServerError(
                format!("Row mapping failed for {}: {}", sql_table, e),
            ))?;

        let has_more = (offset + rows.len() as i64) < total_count;

        results.push(TableDataResult {
            table_id,
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

    Ok(HttpResponse::Ok().json(GetTableDataResponse { results }))
}
