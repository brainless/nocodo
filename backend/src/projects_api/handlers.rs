use actix_web::{get, post, web, HttpResponse, Responder, Result};
use rusqlite::{params, Connection};
use shared_types::{CreateProjectRequest, CreateProjectResponse, ListProjectsResponse, Project};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config;
use crate::repo_api;

fn open_db(database_url: &str) -> Result<Connection, rusqlite::Error> {
    Connection::open(database_url)
}

fn generate_project_path(base_path: &str, name: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let sanitized_name = name
        .to_lowercase()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "");
    let dir_name = format!("{}-{}", sanitized_name, timestamp);
    PathBuf::from(base_path)
        .join(dir_name)
        .to_string_lossy()
        .into_owned()
}

/// GET /api/projects
/// List all projects
#[get("/api/projects")]
pub async fn list_projects(config: web::Data<config::Config>) -> Result<impl Responder> {
    let conn = open_db(&config.database.url).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Database error: {}", e))
    })?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, path, created_at 
             FROM project 
             ORDER BY created_at DESC",
        )
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Prepare error: {}", e))
        })?;

    let projects = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Query error: {}", e))
        })?;

    let projects: Vec<Project> = projects.collect::<Result<Vec<_>, _>>().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Collect error: {}", e))
    })?;

    let response = ListProjectsResponse { projects };
    Ok(HttpResponse::Ok().json(response))
}

/// POST /api/projects
/// Create a new project
#[post("/api/projects")]
pub async fn create_project(
    body: web::Json<CreateProjectRequest>,
    config: web::Data<config::Config>,
) -> Result<impl Responder> {
    let conn = open_db(&config.database.url).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Database error: {}", e))
    })?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let default_projects_path = config
        .projects
        .as_ref()
        .and_then(|p| p.default_path.clone())
        .unwrap_or_else(|| "./projects".to_string());

    let path = body
        .path
        .clone()
        .unwrap_or_else(|| generate_project_path(&default_projects_path, &body.name));

    // Create the project directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&path) {
        return Err(actix_web::error::ErrorInternalServerError(format!(
            "Failed to create project directory: {}",
            e
        )));
    }

    let result = conn.execute(
        "INSERT INTO project (name, path, created_at) VALUES (?1, ?2, ?3)",
        params![&body.name, &path, now],
    );

    match result {
        Ok(_) => {
            let project_id = conn.last_insert_rowid();
            let project = Project {
                id: project_id,
                name: body.name.clone(),
                path: path.clone(),
                created_at: now,
            };

            // Clone template repo in background
            tokio::spawn(async move {
                repo_api::handlers::clone_template_repo(path).await;
            });

            let response = CreateProjectResponse { project };
            Ok(HttpResponse::Created().json(response))
        }
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!(
            "Failed to create project: {}",
            e
        ))),
    }
}
