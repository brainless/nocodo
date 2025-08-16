use crate::database::Database;
use crate::error::AppError;
use crate::models::{CreateProjectRequest, Project, ProjectListResponse, ProjectResponse, ServerStatus};
use actix_web::{web, HttpResponse, Result};
use std::sync::Arc;
use std::time::SystemTime;

pub struct AppState {
    pub database: Arc<Database>,
    pub start_time: SystemTime,
}

pub async fn get_projects(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let projects = data.database.get_all_projects()?;
    let response = ProjectListResponse { projects };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn create_project(
    data: web::Data<AppState>,
    request: web::Json<CreateProjectRequest>,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();
    
    // Validate project name
    if req.name.trim().is_empty() {
        return Err(AppError::InvalidRequest("Project name cannot be empty".to_string()));
    }
    
    // Generate project path if not provided
    let project_path = if let Some(path) = req.path {
        path
    } else {
        // Default to ~/projects/{project_name}
        if let Some(home) = home::home_dir() {
            home.join("projects").join(&req.name).to_string_lossy().to_string()
        } else {
            format!("./projects/{}", req.name)
        }
    };
    
    // Create the project
    let mut project = Project::new(req.name, project_path);
    project.language = req.language;
    project.framework = req.framework;
    
    data.database.create_project(&project)?;
    
    let response = ProjectResponse { project };
    Ok(HttpResponse::Created().json(response))
}

pub async fn get_project(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let project_id = path.into_inner();
    let project = data.database.get_project_by_id(&project_id)?;
    let response = ProjectResponse { project };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn delete_project(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let project_id = path.into_inner();
    data.database.delete_project(&project_id)?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn health_check(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let uptime = data
        .start_time
        .elapsed()
        .map_err(|e| AppError::Internal(format!("Failed to calculate uptime: {}", e)))?
        .as_secs();
    
    let status = ServerStatus {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime,
    };
    
    Ok(HttpResponse::Ok().json(status))
}
