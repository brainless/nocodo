use super::main_handlers::AppState;
use crate::database::Database;
use crate::error::AppError;
use crate::helpers::project;
use crate::models::{
    AddExistingProjectRequest, CreateProjectRequest, Project, ProjectListResponse, ProjectResponse,
};
use crate::templates::{ProjectTemplate, TemplateManager};
use actix_web::{web, HttpResponse, Result, HttpMessage};
use handlebars::Handlebars;
use std::path::Path;

pub async fn get_projects(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let projects = data.database.get_all_projects().map_err(|e| {
        eprintln!("get_all_projects error: {}", e);
        e
    })?;
    let response = ProjectListResponse { projects };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn create_project(
    data: web::Data<AppState>,
    request: web::Json<CreateProjectRequest>,
    http_req: actix_web::HttpRequest,
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

    // Convert to absolute path for consistency
    let absolute_project_path = if project_path.is_absolute() {
        project_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| AppError::Internal(format!("Failed to get current directory: {e}")))?
            .join(project_path)
    };

    let absolute_project_path_string = absolute_project_path.to_string_lossy().to_string();

    // Check if project directory already exists
    if absolute_project_path.exists() {
        return Err(AppError::InvalidRequest(format!(
            "Project directory already exists: {}",
            absolute_project_path.display()
        )));
    }

    // Get template if specified
    let template = if let Some(template_name) = &req.template {
        Some(TemplateManager::get_template(template_name)?)
    } else {
        None
    };

    // Create the project object with absolute path
    let mut project = Project::new(req.name.clone(), absolute_project_path_string.clone());
    project.description = req.description.clone();
    project.parent_id = req.parent_id;

    // Apply template if provided
    if let Some(template) = template {
        apply_project_template(&template, &absolute_project_path, &req.name)?;
        tracing::info!("Applied template {} to project {}", template.name, req.name);
    } else {
        // Create basic project directory structure
        std::fs::create_dir_all(&absolute_project_path)
            .map_err(|e| AppError::Internal(format!("Failed to create project directory: {e}")))?;

        // Create a basic README.md
        let readme_content = format!(
            "# {}

A new project created with nocodo.
",
            req.name
        );
        std::fs::write(absolute_project_path.join("README.md"), readme_content)
            .map_err(|e| AppError::Internal(format!("Failed to create README.md: {e}")))?;
    }

    // Initialize Git repository
    initialize_git_repository(&absolute_project_path)?;

    // Save to database
    let project_id = data.database.create_project(&project)?;
    project.id = project_id;

    // Get user ID from request and record ownership (skip in test mode)
    let is_test_mode = http_req
        .app_data::<web::Data<AppState>>()
        .and_then(|state| state.config.read().ok())
        .map(|config| config.clone())
        .and_then(|config| config.auth)
        .and_then(|auth| auth.jwt_secret)
        .is_none();

    if !is_test_mode {
        let user_id = http_req
            .extensions()
            .get::<crate::models::UserInfo>()
            .map(|u| u.id)
            .unwrap_or(1); // Use test user ID if not authenticated

        let ownership =
            crate::models::ResourceOwnership::new("project".to_string(), project_id, user_id);
        data.database.create_ownership(&ownership)?;
    }

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
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let project_id = path.into_inner();
    let project = data.database.get_project_by_id(project_id)?;
    let response = ProjectResponse { project };
    Ok(HttpResponse::Ok().json(response))
}

/// Detailed project info including detected component apps
pub async fn get_project_details(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let project_id = path.into_inner();
    let project = data.database.get_project_by_id(project_id)?;
    let components = data.database.get_components_for_project(project_id)?;
    let response = crate::models::ProjectDetailsResponse {
        project,
        components,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn delete_project(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let project_id = path.into_inner();

    // Delete from database
    data.database.delete_project(project_id)?;

    // Broadcast project deletion via WebSocket
    data.ws_broadcaster.broadcast_project_deleted(project_id);

    Ok(HttpResponse::NoContent().finish())
}

pub async fn add_existing_project(
    data: web::Data<AppState>,
    request: web::Json<AddExistingProjectRequest>,
    http_req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let project_req = request.into_inner();

    // Validate project name
    if project_req.name.trim().is_empty() {
        return Err(AppError::InvalidRequest(
            "Project name cannot be empty".to_string(),
        ));
    }

    // Validate path is provided
    if project_req.path.trim().is_empty() {
        return Err(AppError::InvalidRequest(
            "Project path cannot be empty".to_string(),
        ));
    }

    let project_path = Path::new(&project_req.path);

    // Validate directory exists and is accessible
    if !project_path.exists() {
        return Err(AppError::InvalidRequest(format!(
            "Project directory does not exist: {}",
            project_path.display()
        )));
    }

    if !project_path.is_dir() {
        return Err(AppError::InvalidRequest(format!(
            "Path is not a directory: {}",
            project_path.display()
        )));
    }

    // Convert to absolute path for consistency
    let absolute_path = project_path.canonicalize().map_err(|e| {
        AppError::InvalidRequest(format!(
            "Failed to resolve absolute path for {}: {}",
            project_path.display(),
            e
        ))
    })?;

    let absolute_path_str = absolute_path.to_string_lossy().to_string();

    // Check if project with this path already exists in database
    if let Ok(_existing) = data.database.get_project_by_path(&absolute_path_str) {
        return Err(AppError::InvalidRequest(format!(
            "Project already registered at path: {}",
            absolute_path.display()
        )));
    }

    // Check if project exists in current or parent paths
    if let Err(err_msg) = check_project_path_conflicts(&data.database, &absolute_path).await {
        return Err(AppError::InvalidRequest(err_msg));
    }

    // Get user ID from request
    let user_id = http_req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Create the project object
    let mut project = Project::new(project_req.name.clone(), absolute_path_str);
    project.description = project_req.description;
    project.parent_id = project_req.parent_id;

    // Save to database
    let project_id = data.database.create_project(&project)?;
    project.id = project_id;

    // Record ownership
    let ownership =
        crate::models::ResourceOwnership::new("project".to_string(), project_id, user_id);
    data.database.create_ownership(&ownership)?;

    // Broadcast project creation via WebSocket
    data.ws_broadcaster
        .broadcast_project_created(project.clone());

    tracing::info!(
        "Successfully registered existing project '{}' at {}",
        project.name,
        project.path
    );

    let response = ProjectResponse { project };
    Ok(HttpResponse::Created().json(response))
}

pub async fn get_templates() -> Result<HttpResponse, AppError> {
    let templates = TemplateManager::get_available_templates();
    Ok(HttpResponse::Ok().json(templates))
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
        .map_err(|e| AppError::Internal(format!("Failed to create project directory: {e}")))?;

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
            .map_err(|e| AppError::Internal(format!("Template processing error: {e}")))?;

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
    use git2::{Repository, Signature, Time};
    
    // Initialize git repository
    let repo = match Repository::init(project_path) {
        Ok(repo) => repo,
        Err(e) => {
            tracing::warn!("Failed to initialize git repository: {}", e);
            // Don't fail the entire project creation if git init fails
            return Ok(());
        }
    };

    // Add all files to git
    let mut index = match repo.index() {
        Ok(index) => index,
        Err(e) => {
            tracing::warn!("Failed to get git index: {}", e);
            return Ok(());
        }
    };

    // Add all files in the directory
    if let Err(e) = index.add_all(["."].iter(), git2::IndexAddOption::DEFAULT, None) {
        tracing::warn!("Failed to add files to git index: {}", e);
        return Ok(());
    }

    // Write the index to get a tree
    let tree_id = match index.write_tree() {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!("Failed to write git tree: {}", e);
            return Ok(());
        }
    };

    let tree = match repo.find_tree(tree_id) {
        Ok(tree) => tree,
        Err(e) => {
            tracing::warn!("Failed to find git tree: {}", e);
            return Ok(());
        }
    };

    // Create a signature for the commit
    let time = Time::new(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
        0,
    );
    
    let signature = match Signature::new("nocodo", "nocodo@localhost", &time) {
        Ok(sig) => sig,
        Err(e) => {
            tracing::warn!("Failed to create git signature: {}", e);
            return Ok(());
        }
    };

    // Create initial commit
    if let Err(e) = repo.commit(
        Some("HEAD"), // Update HEAD to point to this commit
        &signature,   // Author
        &signature,   // Committer (same as author)
        "Initial commit from nocodo",
        &tree,
        &[], // No parents for initial commit
    ) {
        tracing::warn!("Failed to create initial git commit: {}", e);
        return Ok(());
    }

    tracing::info!("Git repository initialized at {}", project_path.display());
    Ok(())
}

pub async fn scan_projects(
    data: web::Data<AppState>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    // NOTE: Permission middleware already checked permissions, no need to check UserInfo here
    // The explicit UserInfo check was causing issues as extensions don't transfer correctly
    // from ServiceRequest (middleware) to HttpRequest (handler) in some middleware configurations

    // Get the configured projects path
    let scan_path = {
        let config = data.config.read().map_err(|e| {
            AppError::Internal(format!("Failed to acquire config read lock: {}", e))
        })?;
        
        config.projects
            .as_ref()
            .and_then(|p| p.default_path.as_ref())
            .ok_or_else(|| {
                AppError::InvalidRequest("No projects default path configured".to_string())
            })?
            .clone()
    };

    // Expand ~ to home directory if present
    let expanded_path = if scan_path.starts_with("~/") {
        if let Some(home) = home::home_dir() {
            scan_path.replacen('~', &home.to_string_lossy(), 1)
        } else {
            scan_path
        }
    } else {
        scan_path
    };

    let scan_path = std::path::Path::new(&expanded_path);

    // Use the project scanner to discover projects
    let discovered_projects = project::scan_filesystem_for_projects(
        scan_path,
        &data.database,
    )
    .await
    .map_err(|e| {
        AppError::Internal(format!("Failed to scan projects: {}", e))
    })?;

    // Convert discovered projects to JSON response
    let scan_results: Vec<serde_json::Value> = discovered_projects
        .into_iter()
        .map(|project| {
            serde_json::json!({
                "project_name": project.name,
                "project_path": project.path,
                "project_type": project.project_type,
                "status": project.status
            })
        })
        .collect();

    let response = serde_json::json!({
        "results": scan_results
    });

    Ok(HttpResponse::Ok().json(response))
}

pub async fn set_projects_default_path(
    data: web::Data<AppState>,
    request: web::Json<serde_json::Value>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    // Get the default path from request
    let default_path = req.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::InvalidRequest("Invalid path parameter".to_string())
        })?;

    // Expand ~ to home directory
    let expanded_path = if default_path.starts_with("~/") {
        if let Some(home) = home::home_dir() {
            default_path.replacen('~', &home.to_string_lossy(), 1)
        } else {
            default_path.to_string()
        }
    } else {
        default_path.to_string()
    };

    // Update config
    let mut config = data.config.write().map_err(|e| {
        AppError::Internal(format!("Failed to acquire config write lock: {}", e))
    })?;

    if let Some(ref mut projects) = config.projects {
        projects.default_path = Some(expanded_path.clone());
    } else {
        config.projects = Some(crate::config::ProjectsConfig {
            default_path: Some(expanded_path.clone()),
        });
    }

    // Clone config for serialization
    let config_clone = config.clone();

    // Save config to file
    let toml_string = toml::to_string(&config_clone).map_err(|e| {
        AppError::Internal(format!("Failed to serialize config: {}", e))
    })?;

    let config_path = if let Some(home) = home::home_dir() {
        home.join(".config/nocodo/manager.toml")
    } else {
        std::path::PathBuf::from("manager.toml")
    };

    std::fs::write(&config_path, toml_string)
        .map_err(|e| AppError::Internal(format!("Failed to write config file: {}", e)))?;

    tracing::info!("Set projects default path to: {}", expanded_path);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "path": expanded_path
    })))
}

pub async fn check_project_path_conflicts(
    database: &Database,
    requested_path: &std::path::Path,
) -> Result<(), String> {
    // Get all existing projects
    let existing_projects = database
        .get_all_projects()
        .map_err(|e| format!("Failed to fetch existing projects: {e}"))?;

    let requested_path_str = requested_path.to_string_lossy();

    for project in existing_projects {
        let existing_path = std::path::Path::new(&project.path);

        // Check if requested path is inside an existing project
        if requested_path.starts_with(existing_path) && requested_path != existing_path {
            return Err(format!(
                "Cannot add project at '{}' because it is inside existing project '{}' at '{}'",
                requested_path_str, project.name, project.path
            ));
        }

        // Check if existing project is inside requested path
        if existing_path.starts_with(requested_path) && existing_path != requested_path {
            return Err(format!(
                "Cannot add project at '{}' because it contains existing project '{}' at '{}'",
                requested_path_str, project.name, project.path
            ));
        }
    }

    Ok(())
}