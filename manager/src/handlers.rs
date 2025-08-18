use crate::database::Database;
use crate::error::AppError;
use crate::models::{
    CreateProjectRequest, Project, ProjectListResponse, ProjectResponse, ServerStatus,
};
use crate::templates::{ProjectTemplate, TemplateManager};
use crate::websocket::WebSocketBroadcaster;
use actix_web::{web, HttpResponse, Result};
use handlebars::Handlebars;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::SystemTime;

pub struct AppState {
    pub database: Arc<Database>,
    pub start_time: SystemTime,
    pub ws_broadcaster: Arc<WebSocketBroadcaster>,
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
        return Err(AppError::InvalidRequest(
            "Project name cannot be empty".to_string(),
        ));
    }

    // Generate project path if not provided
    let project_path_string = if let Some(path) = req.path {
        path
    } else {
        // Default to ~/projects/{project_name}
        if let Some(home) = home::home_dir() {
            home.join("projects")
                .join(&req.name)
                .to_string_lossy()
                .to_string()
        } else {
            format!("./projects/{}", req.name)
        }
    };

    let project_path = Path::new(&project_path_string);

    // Check if project directory already exists
    if project_path.exists() {
        return Err(AppError::InvalidRequest(format!(
            "Project directory already exists: {}",
            project_path.display()
        )));
    }

    // Get template if specified
    let template = if let Some(template_name) = &req.template {
        Some(TemplateManager::get_template(template_name)?)
    } else {
        None
    };

    // Create the project object
    let mut project = Project::new(req.name.clone(), project_path_string.clone());
    project.language = req.language.clone();
    project.framework = req.framework.clone();

    // If template is provided, set language and framework from template
    if let Some(ref template) = template {
        project.language = Some(template.language.clone());
        project.framework = template.framework.clone();
    }

    // Apply template if provided
    if let Some(template) = template {
        apply_project_template(&template, project_path, &req.name)?;
        tracing::info!("Applied template {} to project {}", template.name, req.name);
    } else {
        // Create basic project directory structure
        std::fs::create_dir_all(project_path).map_err(|e| {
            AppError::Internal(format!("Failed to create project directory: {}", e))
        })?;

        // Create a basic README.md
        let readme_content = format!(
            "# {}

A new project created with nocodo.
",
            req.name
        );
        std::fs::write(project_path.join("README.md"), readme_content)
            .map_err(|e| AppError::Internal(format!("Failed to create README.md: {}", e)))?;
    }

    // Initialize Git repository
    initialize_git_repository(project_path)?;

    // Update project status
    project.status = "initialized".to_string();

    // Save to database
    data.database.create_project(&project)?;

    // Broadcast project creation via WebSocket
    data.ws_broadcaster
        .broadcast_project_created(project.clone());

    tracing::info!(
        "Successfully created project '{}' at {}",
        project.name,
        project.path
    );

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

    // Delete from database
    data.database.delete_project(&project_id)?;

    // Broadcast project deletion via WebSocket
    data.ws_broadcaster.broadcast_project_deleted(project_id);

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

fn apply_project_template(
    template: &ProjectTemplate,
    project_path: &Path,
    project_name: &str,
) -> Result<(), AppError> {
    // Create handlebars registry for template processing
    let handlebars = Handlebars::new();
    let mut context = std::collections::HashMap::new();
    context.insert("project_name", project_name);

    // Create project directory
    std::fs::create_dir_all(project_path)
        .map_err(|e| AppError::Internal(format!("Failed to create project directory: {}", e)))?;

    // Apply template files
    for file in &template.files {
        let file_path = project_path.join(&file.path);

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::Internal(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        // Process template content
        let processed_content = handlebars
            .render_template(&file.content, &context)
            .map_err(|e| AppError::Internal(format!("Template processing error: {}", e)))?;

        // Write the file content
        std::fs::write(&file_path, processed_content).map_err(|e| {
            AppError::Internal(format!(
                "Failed to write file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        // Set executable permissions if needed (Unix only)
        #[cfg(unix)]
        if file.executable {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&file_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&file_path, perms).map_err(|e| {
                AppError::Internal(format!(
                    "Failed to set permissions for {}: {}",
                    file_path.display(),
                    e
                ))
            })?;
        }
    }

    Ok(())
}

fn initialize_git_repository(project_path: &Path) -> Result<(), AppError> {
    // Initialize git repository
    let output = Command::new("git")
        .arg("init")
        .current_dir(project_path)
        .output()
        .map_err(|e| AppError::Internal(format!("Failed to run git init: {}", e)))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Git init failed: {}", error);
        // Don't fail the entire project creation if git init fails
        return Ok(());
    }

    // Add all files to git
    let output = Command::new("git")
        .args(&["add", "."])
        .current_dir(project_path)
        .output()
        .map_err(|e| AppError::Internal(format!("Failed to run git add: {}", e)))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Git add failed: {}", error);
        return Ok(());
    }

    // Create initial commit
    let output = Command::new("git")
        .args(&["commit", "-m", "Initial commit from nocodo"])
        .current_dir(project_path)
        .output()
        .map_err(|e| AppError::Internal(format!("Failed to run git commit: {}", e)))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Git commit failed: {}", error);
        return Ok(());
    }

    tracing::info!("Git repository initialized at {}", project_path.display());
    Ok(())
}

pub async fn get_templates() -> Result<HttpResponse, AppError> {
    let templates = TemplateManager::get_available_templates();
    Ok(HttpResponse::Ok().json(templates))
}
