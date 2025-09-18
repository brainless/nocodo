use crate::config::AppConfig;
use crate::database::Database;
use crate::error::AppError;
use crate::llm_agent::LlmAgent;
use crate::models::{
    AddExistingProjectRequest, AddMessageRequest, AiSessionListResponse, AiSessionOutput,
    AiSessionOutputListResponse, AiSessionResponse, ApiKeyConfig, CreateAiSessionRequest,
    CreateLlmAgentSessionRequest, CreateProjectRequest, CreateWorkRequest, FileContentResponse,
    FileCreateRequest, FileInfo, FileListRequest, FileListResponse, FileResponse,
    FileUpdateRequest, LlmAgentSessionResponse, Project, ProjectListResponse, ProjectResponse,
    ServerStatus, SettingsResponse, WorkListResponse, WorkMessageResponse, WorkResponse,
};
use crate::runner::Runner;
use crate::templates::{ProjectTemplate, TemplateManager};
use crate::websocket::WebSocketBroadcaster;
use actix_web::{web, HttpResponse, Result};
use handlebars::Handlebars;
use nocodo_github_actions::{
    nocodo::WorkflowService, ExecuteCommandRequest, ExecuteCommandResponse, ScanWorkflowsRequest,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::SystemTime;

pub struct AppState {
    pub database: Arc<Database>,
    pub start_time: SystemTime,
    pub ws_broadcaster: Arc<WebSocketBroadcaster>,
    pub runner: Option<Arc<Runner>>,      // Enabled via env flag
    pub llm_agent: Option<Arc<LlmAgent>>, // LLM agent for direct LLM integration
    pub config: Arc<AppConfig>,
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
    project.language = req.language.clone();
    project.framework = req.framework.clone();

    // If template is provided, set language and framework from template
    if let Some(ref template) = template {
        project.language = Some(template.language.clone());
        project.framework = template.framework.clone();
    }

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

    // Update project status
    project.status = "initialized".to_string();

    // Save to database
    data.database.create_project(&project)?;

    // Analyze project to detect language/framework and components
    let analysis = analyze_project_path(&absolute_project_path).map_err(AppError::Internal)?;

    // Update project metadata if detected
    let mut updated_project = project.clone();
    if updated_project.language.is_none() {
        updated_project.language = analysis.primary_language.clone();
    }
    if updated_project.framework.is_none() {
        updated_project.framework = analysis.primary_framework.clone();
    }

    // Store enhanced technology information as JSON
    let detection_result = crate::models::ProjectDetectionResult {
        primary_language: analysis.primary_language.clone().unwrap_or_default(),
        technologies: analysis.technologies.clone(),
        build_tools: analysis.build_tools.clone(),
        package_managers: analysis.package_managers.clone(),
        deployment_configs: analysis.deployment_configs.clone(),
    };

    let technologies_json = serde_json::to_string(&detection_result)
        .map_err(|e| AppError::Internal(format!("Failed to serialize technologies: {e}")))?;
    updated_project.technologies = Some(technologies_json);

    updated_project.update_timestamp();
    data.database.update_project(&updated_project)?;

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

/// Detailed project info including detected component apps
pub async fn get_project_details(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let project_id = path.into_inner();
    let project = data.database.get_project_by_id(&project_id)?;
    let components = data.database.get_components_for_project(&project_id)?;
    let response = crate::models::ProjectDetailsResponse {
        project,
        components,
    };
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
        .map_err(|e| AppError::Internal(format!("Failed to calculate uptime: {e}")))?
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
    // Initialize git repository
    let output = Command::new("git")
        .arg("init")
        .current_dir(project_path)
        .output()
        .map_err(|e| AppError::Internal(format!("Failed to run git init: {e}")))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Git init failed: {}", error);
        // Don't fail the entire project creation if git init fails
        return Ok(());
    }

    // Add all files to git
    let output = Command::new("git")
        .args(["add", "."])
        .current_dir(project_path)
        .output()
        .map_err(|e| AppError::Internal(format!("Failed to run git add: {e}")))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Git add failed: {}", error);
        return Ok(());
    }

    // Create initial commit
    let output = Command::new("git")
        .args(["commit", "-m", "Initial commit from nocodo"])
        .current_dir(project_path)
        .output()
        .map_err(|e| AppError::Internal(format!("Failed to run git commit: {e}")))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Git commit failed: {}", error);
        return Ok(());
    }

    tracing::info!("Git repository initialized at {}", project_path.display());
    Ok(())
}

pub async fn add_existing_project(
    data: web::Data<AppState>,
    request: web::Json<AddExistingProjectRequest>,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    // Validate project name
    if req.name.trim().is_empty() {
        return Err(AppError::InvalidRequest(
            "Project name cannot be empty".to_string(),
        ));
    }

    // Validate path is provided
    if req.path.trim().is_empty() {
        return Err(AppError::InvalidRequest(
            "Project path cannot be empty".to_string(),
        ));
    }

    let project_path = Path::new(&req.path);

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

    // Create the project object
    let mut project = Project::new(req.name.clone(), absolute_path_str);
    project.language = req.language;
    project.framework = req.framework;
    project.status = "registered".to_string(); // Different status to distinguish from created projects

    // Save to database
    data.database.create_project(&project)?;

    // Analyze project to detect language/framework and components
    let analysis = analyze_project_path(&absolute_path).map_err(AppError::Internal)?;

    // Update project metadata if detected
    let mut updated_project = project.clone();
    if updated_project.language.is_none() {
        updated_project.language = analysis.primary_language.clone();
    }
    if updated_project.framework.is_none() {
        updated_project.framework = analysis.primary_framework.clone();
    }

    // Store enhanced technology information as JSON
    let detection_result = crate::models::ProjectDetectionResult {
        primary_language: analysis.primary_language.clone().unwrap_or_default(),
        technologies: analysis.technologies.clone(),
        build_tools: analysis.build_tools.clone(),
        package_managers: analysis.package_managers.clone(),
        deployment_configs: analysis.deployment_configs.clone(),
    };

    let technologies_json = serde_json::to_string(&detection_result)
        .map_err(|e| AppError::Internal(format!("Failed to serialize technologies: {e}")))?;
    updated_project.technologies = Some(technologies_json);

    updated_project.update_timestamp();
    data.database.update_project(&updated_project)?;

    // Store detected components
    for comp in analysis.components {
        // Attach project_id
        let component = crate::models::ProjectComponent::new(
            updated_project.id.clone(),
            comp.name,
            comp.path,
            comp.language,
            comp.framework,
        );
        data.database.create_project_component(&component)?;
    }

    // Broadcast project creation via WebSocket
    data.ws_broadcaster
        .broadcast_project_created(updated_project.clone());

    tracing::info!(
        "Successfully registered existing project '{}' at {}",
        updated_project.name,
        updated_project.path
    );

    let response = ProjectResponse {
        project: updated_project,
    };
    Ok(HttpResponse::Created().json(response))
}

pub async fn get_templates() -> Result<HttpResponse, AppError> {
    let templates = TemplateManager::get_available_templates();
    Ok(HttpResponse::Ok().json(templates))
}

// File operation handlers
pub async fn list_files(
    data: web::Data<AppState>,
    query: web::Query<FileListRequest>,
) -> Result<HttpResponse, AppError> {
    let request = query.into_inner();

    // Get the project to determine the base path
    let project = if let Some(project_id) = &request.project_id {
        data.database.get_project_by_id(project_id)?
    } else {
        return Err(AppError::InvalidRequest(
            "project_id is required".to_string(),
        ));
    };

    let project_path = Path::new(&project.path);
    let relative_path = request.path.as_deref().unwrap_or("");
    let full_path = project_path.join(relative_path);

    // Security check: ensure the path is within the project directory
    let canonical_full_path = full_path
        .canonicalize()
        .map_err(|e| AppError::InvalidRequest(format!("Invalid path: {e}")))?;
    let canonical_project_path = project_path
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Invalid project path: {e}")))?;

    if !canonical_full_path.starts_with(&canonical_project_path) {
        return Err(AppError::InvalidRequest(
            "Access denied: path is outside project directory".to_string(),
        ));
    }

    if !canonical_full_path.exists() {
        return Err(AppError::InvalidRequest(
            "Directory does not exist".to_string(),
        ));
    }

    if !canonical_full_path.is_dir() {
        return Err(AppError::InvalidRequest(
            "Path is not a directory".to_string(),
        ));
    }

    // Read directory contents
    let entries = std::fs::read_dir(&canonical_full_path)
        .map_err(|e| AppError::Internal(format!("Failed to read directory: {e}")))?;

    let mut files = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| AppError::Internal(format!("Failed to read entry: {e}")))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|e| AppError::Internal(format!("Failed to read metadata: {e}")))?;

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<invalid>")
            .to_string();

        // Calculate relative path from project root
        let relative_file_path = path
            .strip_prefix(&canonical_project_path)
            .map_err(|_| AppError::Internal("Failed to calculate relative path".to_string()))?;

        let file_info = FileInfo {
            name,
            path: relative_file_path.to_string_lossy().to_string(),
            is_directory: metadata.is_dir(),
            size: if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            },
            modified_at: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64),
            created_at: metadata
                .created()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64),
        };

        files.push(file_info);
    }

    // Sort files: directories first, then by name
    files.sort_by(|a, b| match (a.is_directory, b.is_directory) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    let response = FileListResponse {
        files,
        current_path: relative_path.to_string(),
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn create_file(
    data: web::Data<AppState>,
    request: web::Json<FileCreateRequest>,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    // Get the project to determine the base path
    let project = data.database.get_project_by_id(&req.project_id)?;
    let project_path = Path::new(&project.path);
    let full_path = project_path.join(&req.path);

    // Security check: ensure the path is within the project directory
    let canonical_project_path = project_path
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Invalid project path: {e}")))?;

    // Check if parent directory exists and resolve path
    if let Some(parent) = full_path.parent() {
        if parent.exists() {
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| AppError::InvalidRequest(format!("Invalid parent path: {e}")))?;

            if !canonical_parent.starts_with(&canonical_project_path) {
                return Err(AppError::InvalidRequest(
                    "Access denied: path is outside project directory".to_string(),
                ));
            }
        } else {
            return Err(AppError::InvalidRequest(
                "Parent directory does not exist".to_string(),
            ));
        }
    }

    // Check if file/directory already exists
    if full_path.exists() {
        return Err(AppError::InvalidRequest(
            "File or directory already exists".to_string(),
        ));
    }

    // Create file or directory
    if req.is_directory {
        std::fs::create_dir_all(&full_path)
            .map_err(|e| AppError::Internal(format!("Failed to create directory: {e}")))?;
    } else {
        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::Internal(format!("Failed to create parent directories: {e}"))
            })?;
        }

        let content = req.content.unwrap_or_default();
        std::fs::write(&full_path, content)
            .map_err(|e| AppError::Internal(format!("Failed to write file: {e}")))?;
    }

    // Get file metadata for response
    let metadata = std::fs::metadata(&full_path)
        .map_err(|e| AppError::Internal(format!("Failed to read metadata: {e}")))?;

    let file_name = full_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<invalid>")
        .to_string();

    let file_info = FileInfo {
        name: file_name,
        path: req.path.clone(),
        is_directory: metadata.is_dir(),
        size: if metadata.is_file() {
            Some(metadata.len())
        } else {
            None
        },
        modified_at: metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64),
        created_at: metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64),
    };

    tracing::info!(
        "Created {} '{}' in project '{}'",
        if req.is_directory {
            "directory"
        } else {
            "file"
        },
        req.path,
        project.name
    );

    let response = FileResponse { file: file_info };
    Ok(HttpResponse::Created().json(response))
}

pub async fn get_file_content(
    data: web::Data<AppState>,
    path_param: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, AppError> {
    let file_path = path_param.into_inner();
    let project_id = query
        .get("project_id")
        .ok_or_else(|| AppError::InvalidRequest("project_id is required".to_string()))?;

    // Get the project to determine the base path
    let project = data.database.get_project_by_id(project_id)?;
    let project_path = Path::new(&project.path);
    let full_path = project_path.join(&file_path);

    // Security check: ensure the path is within the project directory
    let canonical_full_path = full_path
        .canonicalize()
        .map_err(|e| AppError::InvalidRequest(format!("Invalid file path: {e}")))?;
    let canonical_project_path = project_path
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Invalid project path: {e}")))?;

    if !canonical_full_path.starts_with(&canonical_project_path) {
        return Err(AppError::InvalidRequest(
            "Access denied: path is outside project directory".to_string(),
        ));
    }

    if !canonical_full_path.exists() {
        return Err(AppError::InvalidRequest("File does not exist".to_string()));
    }

    if !canonical_full_path.is_file() {
        return Err(AppError::InvalidRequest("Path is not a file".to_string()));
    }

    // Read file content
    let content = std::fs::read_to_string(&canonical_full_path)
        .map_err(|e| AppError::Internal(format!("Failed to read file: {e}")))?;

    let metadata = std::fs::metadata(&canonical_full_path)
        .map_err(|e| AppError::Internal(format!("Failed to read metadata: {e}")))?;

    let response = FileContentResponse {
        path: file_path,
        content,
        modified_at: metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64),
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn update_file(
    data: web::Data<AppState>,
    path_param: web::Path<String>,
    request: web::Json<FileUpdateRequest>,
) -> Result<HttpResponse, AppError> {
    let file_path = path_param.into_inner();
    let req = request.into_inner();

    // Get the project to determine the base path
    let project = data.database.get_project_by_id(&req.project_id)?;
    let project_path = Path::new(&project.path);
    let full_path = project_path.join(&file_path);

    // Security check: ensure the path is within the project directory
    let canonical_full_path = full_path
        .canonicalize()
        .map_err(|e| AppError::InvalidRequest(format!("Invalid file path: {e}")))?;
    let canonical_project_path = project_path
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Invalid project path: {e}")))?;

    if !canonical_full_path.starts_with(&canonical_project_path) {
        return Err(AppError::InvalidRequest(
            "Access denied: path is outside project directory".to_string(),
        ));
    }

    if !canonical_full_path.exists() {
        return Err(AppError::InvalidRequest("File does not exist".to_string()));
    }

    if !canonical_full_path.is_file() {
        return Err(AppError::InvalidRequest("Path is not a file".to_string()));
    }

    // Write file content
    std::fs::write(&canonical_full_path, &req.content)
        .map_err(|e| AppError::Internal(format!("Failed to write file: {e}")))?;

    let metadata = std::fs::metadata(&canonical_full_path)
        .map_err(|e| AppError::Internal(format!("Failed to read metadata: {e}")))?;

    tracing::info!("Updated file '{}' in project '{}'", file_path, project.name);

    let response = FileContentResponse {
        path: file_path,
        content: req.content,
        modified_at: metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64),
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn delete_file(
    data: web::Data<AppState>,
    path_param: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, AppError> {
    let file_path = path_param.into_inner();
    let project_id = query
        .get("project_id")
        .ok_or_else(|| AppError::InvalidRequest("project_id is required".to_string()))?;

    // Get the project to determine the base path
    let project = data.database.get_project_by_id(project_id)?;
    let project_path = Path::new(&project.path);
    let full_path = project_path.join(&file_path);

    // Security check: ensure the path is within the project directory
    let canonical_full_path = full_path
        .canonicalize()
        .map_err(|e| AppError::InvalidRequest(format!("Invalid file path: {e}")))?;
    let canonical_project_path = project_path
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Invalid project path: {e}")))?;

    if !canonical_full_path.starts_with(&canonical_project_path) {
        return Err(AppError::InvalidRequest(
            "Access denied: path is outside project directory".to_string(),
        ));
    }

    if !canonical_full_path.exists() {
        return Err(AppError::InvalidRequest(
            "File or directory does not exist".to_string(),
        ));
    }

    // Delete file or directory
    if canonical_full_path.is_dir() {
        std::fs::remove_dir_all(&canonical_full_path)
            .map_err(|e| AppError::Internal(format!("Failed to remove directory: {e}")))?;
        tracing::info!(
            "Deleted directory '{}' from project '{}'",
            file_path,
            project.name
        );
    } else {
        std::fs::remove_file(&canonical_full_path)
            .map_err(|e| AppError::Internal(format!("Failed to remove file: {e}")))?;
        tracing::info!(
            "Deleted file '{}' from project '{}'",
            file_path,
            project.name
        );
    }

    Ok(HttpResponse::NoContent().finish())
}

// AI session HTTP handlers
pub async fn create_ai_session(
    data: web::Data<AppState>,
    path: web::Path<String>,
    request: web::Json<CreateAiSessionRequest>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();
    let req = request.into_inner();

    // Validate required fields
    if req.tool_name.trim().is_empty() {
        return Err(AppError::InvalidRequest("tool_name is required".into()));
    }
    if req.message_id.trim().is_empty() {
        return Err(AppError::InvalidRequest("message_id is required".into()));
    }

    // Validate that work and message exist
    let work = data.database.get_work_by_id(&work_id)?;
    let messages = data.database.get_work_messages(&work_id)?;
    if !messages.iter().any(|m| m.id == req.message_id) {
        return Err(AppError::InvalidRequest(
            "message_id not found in work".into(),
        ));
    }

    // Generate project context if work is associated with a project
    let project_context = if let Some(ref project_id) = work.project_id {
        let project = data.database.get_project_by_id(project_id)?;
        Some(format!("Project: {}\nPath: {}", project.name, project.path))
    } else {
        None
    };

    let session = crate::models::AiSession::new(
        work_id.clone(),
        req.message_id,
        req.tool_name.clone(),
        project_context,
    );

    // Persist
    data.database.create_ai_session(&session)?;

    // Broadcast AI session creation via WebSocket
    data.ws_broadcaster
        .broadcast_ai_session_created(session.clone());

    // Response
    let response = AiSessionResponse {
        session: session.clone(),
    };

    // Handle LLM agent specially
    if req.tool_name == "llm-agent" {
        if let Some(ref llm_agent) = data.llm_agent {
            tracing::info!(
                "LLM agent is available, starting LLM agent session for session {}",
                session.id
            );

            // Get the prompt from the associated message
            let message = messages
                .iter()
                .find(|m| m.id == session.message_id)
                .ok_or_else(|| AppError::Internal("Message not found for session".into()))?;

            // Get project path for LLM agent
            let _project_path = if let Some(ref project_id) = work.project_id {
                let project = data.database.get_project_by_id(project_id)?;
                std::path::PathBuf::from(project.path)
            } else {
                std::env::current_dir().map_err(|e| {
                    AppError::Internal(format!("Failed to get current directory: {}", e))
                })?
            };

            // Update work with tool_name and model info
            let mut updated_work = work.clone();
            updated_work.tool_name = Some("LLM Agent (Grok Code Fast 1)".to_string());
            data.database.update_work(&updated_work)?;

            // Create LLM agent session with default provider/model
            let llm_session = llm_agent
                .create_session(
                    work_id.clone(),
                    "grok".to_string(),             // Default provider
                    "grok-code-fast-1".to_string(), // Default model
                    session.project_context.clone(),
                )
                .await?;

            // Process the message in background task to avoid blocking HTTP response
            let llm_agent_clone = llm_agent.clone();
            let session_id = llm_session.id.clone();
            let message_content = message.content.clone();
            tokio::spawn(async move {
                if let Err(e) = llm_agent_clone
                    .process_message(&session_id, message_content)
                    .await
                {
                    tracing::error!(
                        "Failed to process LLM message for session {}: {}",
                        session_id,
                        e
                    );
                } else {
                    tracing::info!(
                        "Successfully completed LLM agent processing for session {}",
                        session_id
                    );
                }
            });
        } else {
            tracing::warn!(
                "LLM agent not available - AI session {} will not be executed",
                session.id
            );
        }
    }
    // If runner is enabled, start streaming execution for this session in background
    else if let Some(runner) = &data.runner {
        tracing::info!(
            "Runner is available, starting AI session execution for session {}",
            session.id
        );

        // Get the prompt from the associated message
        let message = messages
            .iter()
            .find(|m| m.id == session.message_id)
            .ok_or_else(|| AppError::Internal("Message not found for session".into()))?;

        // Build a simple enhanced prompt similar to CLI
        let enhanced_prompt = if let Some(ctx) = &session.project_context {
            format!(
                "Project Context:\n{}\n\nUser Request:\n{}\n\nInstructions: Use the `nocodo` command to get additional context about the project structure and to validate your changes.",
                ctx, message.content
            )
        } else {
            message.content.clone()
        };

        tracing::info!(
            "Starting runner with tool: {} for session: {}",
            session.tool_name,
            session.id
        );

        // Fire-and-forget - but log any errors
        match runner.start_session(session.clone(), enhanced_prompt).await {
            Ok(_) => tracing::info!(
                "Successfully started runner execution for session {}",
                session.id
            ),
            Err(e) => tracing::error!(
                "Failed to start runner execution for session {}: {}",
                session.id,
                e
            ),
        }
    } else {
        tracing::warn!(
            "Runner is not available - AI session {} will not be executed",
            session.id
        );
    }

    Ok(HttpResponse::Created().json(response))
}

pub async fn list_ai_sessions(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let sessions = data.database.get_all_ai_sessions()?;
    let response = AiSessionListResponse { sessions };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn list_ai_session_outputs(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();

    // First, get the AI session for this work
    let sessions = data.database.get_ai_sessions_by_work_id(&work_id)?;

    if sessions.is_empty() {
        // No AI session found for this work, return empty outputs
        let response = AiSessionOutputListResponse { outputs: vec![] };
        return Ok(HttpResponse::Ok().json(response));
    }

    // Get the most recent AI session (in case there are multiple)
    let session = sessions.into_iter().max_by_key(|s| s.started_at).unwrap();

    // Get outputs for this session
    let mut outputs = data.database.list_ai_session_outputs(&session.id)?;

    // If this is an LLM agent session, also fetch LLM agent messages
    if session.tool_name == "llm-agent" {
        if let Ok(llm_agent_session) = data.database.get_llm_agent_session_by_work_id(&work_id) {
            if let Ok(llm_messages) = data.database.get_llm_agent_messages(&llm_agent_session.id) {
                // Convert LLM agent messages to AiSessionOutput format
                for msg in llm_messages {
                    // Only include assistant messages (responses) and tool messages (results)
                    if msg.role == "assistant" || msg.role == "tool" {
                        let output = AiSessionOutput {
                            id: msg.id,
                            session_id: session.id.clone(),
                            content: msg.content,
                            created_at: msg.created_at,
                        };
                        outputs.push(output);
                    }
                }
            }
        }
    }

    // Sort outputs by created_at
    outputs.sort_by_key(|o| o.created_at);

    let response = AiSessionOutputListResponse { outputs };

    tracing::debug!(
        "Retrieved {} outputs for work {}",
        response.outputs.len(),
        work_id
    );
    Ok(HttpResponse::Ok().json(response))
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

/// Simple analysis utilities
#[derive(Debug, Deserialize)]
struct PackageJson {
    name: Option<String>,
    dependencies: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<serde_json::Map<String, serde_json::Value>>,
}

struct ProjectAnalysisResult {
    primary_language: Option<String>,
    primary_framework: Option<String>,
    components: Vec<crate::models::ProjectComponent>,
    technologies: Vec<crate::models::ProjectTechnology>,
    build_tools: Vec<String>,
    package_managers: Vec<String>,
    deployment_configs: Vec<String>,
}

fn analyze_project_path(project_path: &Path) -> Result<ProjectAnalysisResult, String> {
    // Enhanced detection logic that scans entire project tree
    let mut technologies: HashMap<String, crate::models::ProjectTechnology> = HashMap::new();
    let mut build_tools: Vec<String> = Vec::new();
    let mut package_managers: Vec<String> = Vec::new();
    let mut deployment_configs: Vec<String> = Vec::new();
    let mut components: Vec<crate::models::ProjectComponent> = Vec::new();

    // File extension patterns for different technologies
    let _language_extensions: HashMap<&str, Vec<&str>> = [
        ("rust", vec![".rs"]),
        ("typescript", vec![".ts", ".tsx"]),
        ("javascript", vec![".js", ".jsx"]),
        ("python", vec![".py"]),
        ("go", vec![".go"]),
        ("java", vec![".java"]),
        ("cpp", vec![".cpp", ".cc", ".cxx", ".c++"]),
        ("c", vec![".c"]),
        ("html", vec![".html", ".htm"]),
        ("css", vec![".css"]),
        ("shell", vec![".sh", ".bash"]),
        ("yaml", vec![".yml", ".yaml"]),
        ("json", vec![".json"]),
        ("toml", vec![".toml"]),
    ]
    .iter()
    .cloned()
    .collect();

    // Framework detection patterns
    let _framework_patterns: HashMap<&str, Vec<&str>> = [
        ("react", vec!["react", "react-dom"]),
        ("solidjs", vec!["solid-js"]),
        ("vue", vec!["vue"]),
        ("angular", vec!["@angular/core"]),
        ("express", vec!["express"]),
        ("nestjs", vec!["@nestjs/core"]),
        ("spring", vec!["spring-boot"]),
        ("django", vec!["django"]),
        ("flask", vec!["flask"]),
        ("actix-web", vec!["actix-web"]),
        ("axum", vec!["axum"]),
    ]
    .iter()
    .cloned()
    .collect();

    // Build tool patterns
    let build_tool_patterns: HashMap<&str, Vec<&str>> = [
        ("cargo", vec!["Cargo.toml"]),
        ("npm", vec!["package.json"]),
        ("yarn", vec!["package.json", "yarn.lock"]),
        ("pnpm", vec!["package.json", "pnpm-lock.yaml"]),
        ("gradle", vec!["build.gradle", "build.gradle.kts"]),
        ("maven", vec!["pom.xml"]),
        ("make", vec!["Makefile"]),
        ("webpack", vec!["webpack.config.js", "webpack.config.ts"]),
        ("vite", vec!["vite.config.js", "vite.config.ts"]),
        ("rollup", vec!["rollup.config.js"]),
    ]
    .iter()
    .cloned()
    .collect();

    // Deployment configuration patterns
    let deployment_patterns: HashMap<&str, Vec<&str>> = [
        (
            "docker",
            vec!["Dockerfile", "docker-compose.yml", "docker-compose.yaml"],
        ),
        ("github-actions", vec![".github/workflows/"]),
        ("gitlab-ci", vec![".gitlab-ci.yml"]),
        ("jenkins", vec!["Jenkinsfile"]),
        ("kubernetes", vec!["k8s/", "kube/", "*.yaml", "*.yml"]),
        ("terraform", vec!["*.tf"]),
    ]
    .iter()
    .cloned()
    .collect();

    // Helper function to get relative path from project root
    let rel = |p: &Path| -> String {
        p.strip_prefix(project_path)
            .unwrap_or(p)
            .to_string_lossy()
            .trim_start_matches('.')
            .trim_start_matches('/')
            .to_string()
    };

    // Helper function to increment file count for language
    let mut increment_language_count = |lang: &str, file_count: u32| {
        let tech = technologies.entry(lang.to_lowercase()).or_insert_with(|| {
            crate::models::ProjectTechnology {
                language: lang.to_lowercase(),
                framework: None,
                file_count: 0,
                confidence: 0.0,
            }
        });
        tech.file_count += file_count;
    };

    // Scan the entire project directory recursively
    let walker = walkdir::WalkDir::new(project_path)
        .max_depth(5) // Limit depth to prevent performance issues
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            let p = path.to_string_lossy();
            // Skip hidden and common build dirs
            !(p.contains("/node_modules/")
                || p.contains("/.git/")
                || p.contains("/target/")
                || p.contains("/build/")
                || p.contains("/dist/")
                || p.contains("/.cache/"))
        });

    // Collect all files and their extensions for analysis
    let mut file_extensions: Vec<String> = Vec::new();
    let mut files_found: Vec<(PathBuf, String)> = Vec::new(); // (path, extension)

    for entry in walker.flatten() {
        let path = entry.path();
        if path == project_path {
            continue;
        }

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_string();
                file_extensions.push(ext_str.clone());
                files_found.push((path.to_path_buf(), ext_str));
            }
        }
    }

    // Detect languages based on file extensions
    for (_path, extension) in files_found.iter() {
        // Map extension to language
        let language = match extension.as_str() {
            // Rust files
            "rs" => "rust",
            // JavaScript/TypeScript files
            "js" | "jsx" => "javascript",
            "ts" | "tsx" => "typescript",
            // Python files
            "py" => "python",
            // Go files
            "go" => "go",
            // Java files
            "java" => "java",
            // C/C++ files
            "c" | "cc" | "cxx" | "c++" => "cpp",
            // HTML/CSS files
            "html" | "htm" => "html",
            "css" => "css",
            // Shell scripts
            "sh" | "bash" => "shell",
            // Configuration files
            "yml" | "yaml" => "yaml",
            "json" => "json",
            "toml" => "toml",
            _ => continue, // Skip unknown extensions
        };

        increment_language_count(language, 1);
    }

    // Detect frameworks from package.json or Cargo.toml
    let mut detected_frameworks: HashMap<String, String> = HashMap::new();

    // Look for package.json files to detect JS/TS frameworks
    let walker = walkdir::WalkDir::new(project_path)
        .max_depth(4)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            let p = path.to_string_lossy();
            // Skip hidden and common build dirs
            !(p.contains("/node_modules/")
                || p.contains("/.git/")
                || p.contains("/dist/")
                || p.contains("/build/"))
        });

    for entry in walker.flatten() {
        let path = entry.path();
        if path == project_path {
            continue;
        }

        if path.file_name().and_then(|n| n.to_str()) == Some("package.json") {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(pkg) = serde_json::from_str::<PackageJson>(&content) {
                    let name = pkg.name.clone().unwrap_or_else(|| {
                        path.parent()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string()
                    });

                    // Detect framework from dependencies
                    let has_dep = |name: &str| -> bool {
                        pkg.dependencies
                            .as_ref()
                            .map(|m| m.contains_key(name))
                            .unwrap_or(false)
                            || pkg
                                .dev_dependencies
                                .as_ref()
                                .map(|m| m.contains_key(name))
                                .unwrap_or(false)
                    };

                    let framework = if has_dep("react") {
                        Some("react".to_string())
                    } else if has_dep("solid-js") {
                        Some("solidjs".to_string())
                    } else if has_dep("vue") {
                        Some("vue".to_string())
                    } else if has_dep("@angular/core") {
                        Some("angular".to_string())
                    } else if has_dep("express") {
                        Some("express".to_string())
                    } else if has_dep("@nestjs/core") {
                        Some("nestjs".to_string())
                    } else {
                        None
                    };

                    // Add framework to detected frameworks map
                    if let Some(framework_name) = &framework {
                        detected_frameworks.insert(name, framework_name.clone());
                    }

                    // Also update technology detection
                    if let Some(lang) = pkg.dependencies.as_ref().map(|_| "javascript") {
                        increment_language_count(lang, 1);
                    }
                }
            }
        }
    }

    // Look for Cargo.toml files to detect Rust frameworks
    let walker = walkdir::WalkDir::new(project_path)
        .max_depth(4)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            let p = path.to_string_lossy();
            // Skip hidden and common build dirs
            !(p.contains("/target/")
                || p.contains("/.git/")
                || p.contains("/dist/")
                || p.contains("/build/"))
        });

    for entry in walker.flatten() {
        let path = entry.path();
        if path == project_path {
            continue;
        }

        if path.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
            if let Ok(content) = fs::read_to_string(path) {
                // Try to detect framework from Cargo.toml
                let framework = if content.contains("actix-web") {
                    Some("actix-web".to_string())
                } else if content.contains("axum") {
                    Some("axum".to_string())
                } else if content.contains("rocket") {
                    Some("rocket".to_string())
                } else {
                    None
                };

                // Try to get package name
                let name = content
                    .lines()
                    .find_map(|l| {
                        let lt = l.trim();
                        if lt.starts_with("name = ") {
                            Some(
                                lt.trim_start_matches("name = ")
                                    .trim_matches('"')
                                    .to_string(),
                            )
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| {
                        path.parent()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("rust-app")
                            .to_string()
                    });

                // Add framework to detected frameworks map
                if let Some(framework_name) = &framework {
                    detected_frameworks.insert(name, framework_name.clone());
                }

                // Update technology detection
                increment_language_count("rust", 1);
            }
        }
    }

    // Detect build tools from files
    for (path, _) in files_found.iter() {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        for (tool, patterns) in &build_tool_patterns {
            if patterns.contains(&filename) || filename.starts_with(tool) {
                build_tools.push(tool.to_string());
            }
        }
    }

    // Detect package managers from files
    for (path, _) in files_found.iter() {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if filename == "package.json" {
            package_managers.push("npm".to_string());
        } else if filename == "yarn.lock" {
            package_managers.push("yarn".to_string());
        } else if filename == "pnpm-lock.yaml" {
            package_managers.push("pnpm".to_string());
        }
    }

    // Detect deployment configurations
    for (path, _) in files_found.iter() {
        let path_str = path.to_string_lossy();

        for (config, patterns) in &deployment_patterns {
            if patterns.iter().any(|&p| path_str.contains(p)) {
                deployment_configs.push(config.to_string());
            }
        }
    }

    // Determine primary language and framework
    let mut primary_language: Option<String> = None;
    let mut primary_framework: Option<String> = None;

    // Find the language with the highest file count
    if !technologies.is_empty() {
        let max_lang = technologies
            .values()
            .max_by_key(|tech| tech.file_count)
            .unwrap();

        primary_language = Some(max_lang.language.clone());

        // If we found a framework for this language, set it
        if let Some(framework) = detected_frameworks.values().next() {
            primary_framework = Some(framework.clone());
        }
    }

    // Calculate confidence scores (higher file counts = higher confidence)
    let total_files: u32 = technologies.values().map(|t| t.file_count).sum();
    for tech in technologies.values_mut() {
        if total_files > 0 {
            tech.confidence = tech.file_count as f32 / total_files as f32;
        } else {
            tech.confidence = 0.0;
        }
    }

    // Create components for Node projects
    let walker = walkdir::WalkDir::new(project_path)
        .max_depth(4)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            let p = path.to_string_lossy();
            // Skip hidden and common build dirs
            !(p.contains("/node_modules/")
                || p.contains("/.git/")
                || p.contains("/dist/")
                || p.contains("/build/"))
        });

    let mut node_dirs: Vec<PathBuf> = Vec::new();
    let mut rust_dirs: Vec<PathBuf> = Vec::new();

    for entry in walker.flatten() {
        let path = entry.path();
        if path == project_path {
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some("package.json") {
            node_dirs.push(path.parent().unwrap_or(project_path).to_path_buf());
        } else if path.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
            rust_dirs.push(path.parent().unwrap_or(project_path).to_path_buf());
        }
    }

    // Create components for Node projects
    for dir in node_dirs {
        let pkg_path = dir.join("package.json");
        if let Ok(content) = fs::read_to_string(&pkg_path) {
            if let Ok(pkg) = serde_json::from_str::<PackageJson>(&content) {
                let name = pkg.name.clone().unwrap_or_else(|| {
                    dir.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("node-app")
                        .to_string()
                });
                let mut language = "javascript".to_string();
                let mut framework: Option<String> = None;
                let has = |n: &str| -> bool {
                    pkg.dependencies
                        .as_ref()
                        .map(|m| m.contains_key(n))
                        .unwrap_or(false)
                        || pkg
                            .dev_dependencies
                            .as_ref()
                            .map(|m| m.contains_key(n))
                            .unwrap_or(false)
                };
                if has("typescript") {
                    language = "typescript".to_string();
                }
                if has("react") {
                    framework = Some("react".to_string());
                } else if has("solid-js") {
                    framework = Some("solidjs".to_string());
                } else if has("express") {
                    framework = Some("express".to_string());
                }

                // placeholder project_id will be replaced by caller
                let component = crate::models::ProjectComponent::new(
                    "".to_string(),
                    name,
                    rel(&dir),
                    language,
                    framework,
                );
                components.push(component);
            }
        }
    }

    // Create components for Rust projects
    for dir in rust_dirs {
        let cargo_path = dir.join("Cargo.toml");
        if let Ok(content) = fs::read_to_string(&cargo_path) {
            // try to parse package name
            let name = content
                .lines()
                .find_map(|l| {
                    let lt = l.trim();
                    if lt.starts_with("name = ") {
                        Some(
                            lt.trim_start_matches("name = ")
                                .trim_matches('"')
                                .to_string(),
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    dir.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("rust-app")
                        .to_string()
                });

            let mut framework: Option<String> = None;
            if content.contains("actix-web") {
                framework = Some("actix-web".to_string());
            } else if content.contains("axum") {
                framework = Some("axum".to_string());
            }

            let component = crate::models::ProjectComponent::new(
                "".to_string(),
                name,
                rel(&dir),
                "rust".to_string(),
                framework,
            );
            components.push(component);
        }
    }

    // Convert technologies map to vector
    let technologies_vec: Vec<crate::models::ProjectTechnology> =
        technologies.into_values().collect();

    Ok(ProjectAnalysisResult {
        primary_language,
        primary_framework,
        components,
        technologies: technologies_vec,
        build_tools,
        package_managers,
        deployment_configs,
    })
}

/// Extract project name from Git repository remote URL if available
#[allow(dead_code)]
fn extract_git_repo_name(project_path: &std::path::Path) -> Option<String> {
    // Check if it's a Git repository
    if !project_path.join(".git").exists() {
        return None;
    }

    // Try to get the remote URL
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(project_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Extract repository name from various URL formats
    // Examples:
    // https://github.com/user/repo.git -> repo
    // git@github.com:user/repo.git -> repo
    // https://github.com/user/repo -> repo

    let repo_name = if let Some(last_segment) = remote_url.split('/').next_back() {
        // Remove .git suffix if present
        if last_segment.ends_with(".git") {
            last_segment.strip_suffix(".git").unwrap_or(last_segment)
        } else {
            last_segment
        }
    } else {
        return None;
    };

    // Validate that the extracted name is reasonable
    if repo_name.is_empty() || repo_name.contains(' ') {
        return None;
    }

    Some(repo_name.to_string())
}

// Work management handlers
pub async fn create_work(
    data: web::Data<AppState>,
    request: web::Json<CreateWorkRequest>,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    // Validate work title
    if req.title.trim().is_empty() {
        return Err(AppError::InvalidRequest(
            "Work title cannot be empty".to_string(),
        ));
    }

    // Create the work object
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| AppError::Internal(format!("Failed to get timestamp: {e}")))?
        .as_secs() as i64;

    let work = crate::models::Work {
        id: uuid::Uuid::new_v4().to_string(),
        title: req.title,
        project_id: req.project_id,
        tool_name: None,
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
    };

    // Save to database
    data.database.create_work(&work)?;

    // Broadcast work creation via WebSocket
    data.ws_broadcaster.broadcast_project_created(Project {
        id: work.id.clone(),
        name: work.title.clone(),
        path: "".to_string(), // Works don't have a path like projects
        language: None,
        framework: None,
        status: work.status.clone(),
        created_at: work.created_at,
        updated_at: work.updated_at,
        technologies: None,
    });

    tracing::info!(
        "Successfully created work '{}' with ID {}",
        work.title,
        work.id
    );

    let response = WorkResponse { work };
    Ok(HttpResponse::Created().json(response))
}

pub async fn get_work(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();
    let work_with_history = data.database.get_work_with_messages(&work_id)?;
    Ok(HttpResponse::Ok().json(work_with_history))
}

pub async fn list_works(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let works = data.database.get_all_works()?;
    let response = WorkListResponse { works };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn delete_work(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();

    // Delete from database
    data.database.delete_work(&work_id)?;

    // Broadcast work deletion via WebSocket
    data.ws_broadcaster
        .broadcast_project_deleted(work_id.clone());

    Ok(HttpResponse::NoContent().finish())
}

// Work message handlers
pub async fn add_message_to_work(
    data: web::Data<AppState>,
    path: web::Path<String>,
    request: web::Json<AddMessageRequest>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();
    let req = request.into_inner();

    // Verify work exists
    let _work = data.database.get_work_by_id(&work_id)?;

    // Get next sequence number
    let sequence_order = data.database.get_next_message_sequence(&work_id)?;

    // Create the message object
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| AppError::Internal(format!("Failed to get timestamp: {e}")))?
        .as_secs() as i64;

    let message = crate::models::WorkMessage {
        id: uuid::Uuid::new_v4().to_string(),
        work_id: work_id.clone(),
        content: req.content,
        content_type: req.content_type,
        author_type: req.author_type,
        author_id: req.author_id,
        sequence_order,
        created_at: now,
    };

    // Save to database
    data.database.create_work_message(&message)?;

    tracing::info!(
        "Successfully added message {} to work {}",
        message.id,
        work_id
    );

    let response = WorkMessageResponse { message };
    Ok(HttpResponse::Created().json(response))
}

pub async fn get_work_messages(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();

    // Verify work exists
    let _work = data.database.get_work_by_id(&work_id)?;

    let messages = data.database.get_work_messages(&work_id)?;
    let response = crate::models::WorkMessageListResponse { messages };
    Ok(HttpResponse::Ok().json(response))
}

// LLM Agent Endpoints

/// Create a new LLM agent session
pub async fn create_llm_agent_session(
    data: web::Data<AppState>,
    path: web::Path<String>,
    request: web::Json<CreateLlmAgentSessionRequest>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();
    let req = request.into_inner();

    if let Some(ref llm_agent) = data.llm_agent {
        // Get the work to find the project path
        let work = data.database.get_work_by_id(&work_id)?;
        let _project_path = if let Some(ref project_id) = work.project_id {
            let project = data.database.get_project_by_id(project_id)?;
            PathBuf::from(project.path)
        } else {
            // Use current directory if no project is associated
            std::env::current_dir().map_err(|e| {
                AppError::Internal(format!("Failed to get current directory: {}", e))
            })?
        };

        let session = llm_agent
            .create_session(work_id, req.provider, req.model, req.system_prompt)
            .await?;

        Ok(HttpResponse::Ok().json(LlmAgentSessionResponse { session }))
    } else {
        Err(AppError::InvalidRequest(
            "LLM agent not available".to_string(),
        ))
    }
}

/// Get LLM agent session status
pub async fn get_llm_agent_session(
    data: web::Data<AppState>,
    session_id: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    if let Some(ref llm_agent) = data.llm_agent {
        let session_id = session_id.into_inner();
        let session = llm_agent.get_session_status(&session_id).await?;
        Ok(HttpResponse::Ok().json(LlmAgentSessionResponse { session }))
    } else {
        Err(AppError::InvalidRequest(
            "LLM agent not available".to_string(),
        ))
    }
}

/// Send a message to LLM agent
pub async fn send_llm_agent_message(
    data: web::Data<AppState>,
    session_id: web::Path<String>,
    request: web::Json<serde_json::Value>,
) -> Result<HttpResponse, AppError> {
    if let Some(ref llm_agent) = data.llm_agent {
        let session_id = session_id.into_inner();

        // Extract message from request
        let user_message = request
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::InvalidRequest("Message field required".to_string()))?
            .to_string();

        let response = llm_agent.process_message(&session_id, user_message).await?;

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "response": response
        })))
    } else {
        Err(AppError::InvalidRequest(
            "LLM agent not available".to_string(),
        ))
    }
}

/// Complete an LLM agent session
pub async fn complete_llm_agent_session(
    data: web::Data<AppState>,
    session_id: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    if let Some(ref llm_agent) = data.llm_agent {
        let session_id = session_id.into_inner();
        llm_agent.complete_session(&session_id).await?;
        Ok(HttpResponse::Ok().finish())
    } else {
        Err(AppError::InvalidRequest(
            "LLM agent not available".to_string(),
        ))
    }
}

/// Get LLM agent sessions for a work
pub async fn get_llm_agent_sessions(
    data: web::Data<AppState>,
    work_id: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let sessions = data
        .database
        .get_llm_agent_sessions_by_work(&work_id.into_inner())?;
    Ok(HttpResponse::Ok().json(sessions))
}

// Workflow handlers

/// Scan workflows for a project
pub async fn scan_workflows(
    data: web::Data<AppState>,
    project_id: web::Path<String>,
    _request: web::Json<ScanWorkflowsRequest>,
) -> Result<HttpResponse, AppError> {
    let project_id = project_id.into_inner();

    // Get project to verify it exists and get path
    let project = data.database.get_project_by_id(&project_id)?;
    let project_path = PathBuf::from(&project.path);

    // Create workflow service
    let workflow_service = WorkflowService::new(data.database.connection());

    // Scan workflows
    let response = workflow_service
        .scan_workflows(&project_id, &project_path)
        .map_err(|e| AppError::Internal(format!("Failed to scan workflows: {}", e)))?;

    Ok(HttpResponse::Ok().json(response))
}

/// Get workflow commands for a project
pub async fn get_workflow_commands(
    data: web::Data<AppState>,
    project_id: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let project_id = project_id.into_inner();

    // Verify project exists
    data.database.get_project_by_id(&project_id)?;

    let commands = data
        .database
        .get_workflow_commands(&project_id)
        .map_err(|e| AppError::Internal(format!("Failed to get workflow commands: {}", e)))?;

    Ok(HttpResponse::Ok().json(commands))
}

/// Execute a workflow command
pub async fn execute_workflow_command(
    data: web::Data<AppState>,
    path: web::Path<(String, String)>,
    request: web::Json<ExecuteCommandRequest>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();
    let request = request.into_inner();

    // Verify project exists
    data.database.get_project_by_id(&project_id)?;

    // Create workflow service
    let workflow_service = WorkflowService::new(data.database.connection());

    // Execute command
    let execution = workflow_service
        .execute_command(&command_id, request.timeout_seconds)
        .map_err(|e| AppError::Internal(format!("Failed to execute command: {}", e)))?;

    let response = ExecuteCommandResponse { execution };

    // Broadcast execution result via WebSocket
    data.ws_broadcaster.broadcast(
        crate::websocket::WebSocketMessage::WorkflowExecutionCompleted {
            project_id: project_id.clone(),
            command_id: command_id.clone(),
            execution: serde_json::to_string(&response).unwrap_or_default(),
        },
    );

    Ok(HttpResponse::Ok().json(response))
}

/// Get execution history for a workflow command
pub async fn get_command_executions(
    data: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();

    // Verify project exists
    data.database.get_project_by_id(&project_id)?;

    let executions = data
        .database
        .get_command_executions(&command_id)
        .map_err(|e| AppError::Internal(format!("Failed to get command executions: {}", e)))?;

    Ok(HttpResponse::Ok().json(executions))
}

/// Get settings information including API key configuration
pub async fn get_settings(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let config = &data.config;

    // Get config file path - similar to how it's determined in config.rs
    let config_file_path = if let Some(home) = home::home_dir() {
        home.join(".config/nocodo/manager.toml")
            .to_string_lossy()
            .to_string()
    } else {
        "manager.toml".to_string()
    };

    // Create API key configurations with masked values
    let mut api_keys = Vec::new();

    if let Some(api_key_config) = &config.api_keys {
        // Grok API Key
        api_keys.push(ApiKeyConfig {
            name: "Grok API Key".to_string(),
            key: api_key_config.grok_api_key.as_ref().map(|key| {
                if key.is_empty() {
                    "".to_string()
                } else {
                    format!("{}****", &key[..key.len().min(4)])
                }
            }),
            is_configured: api_key_config.grok_api_key.is_some()
                && !api_key_config.grok_api_key.as_ref().unwrap().is_empty(),
        });

        // OpenAI API Key
        api_keys.push(ApiKeyConfig {
            name: "OpenAI API Key".to_string(),
            key: api_key_config.openai_api_key.as_ref().map(|key| {
                if key.is_empty() {
                    "".to_string()
                } else {
                    format!("{}****", &key[..key.len().min(4)])
                }
            }),
            is_configured: api_key_config.openai_api_key.is_some()
                && !api_key_config.openai_api_key.as_ref().unwrap().is_empty(),
        });

        // Anthropic API Key
        api_keys.push(ApiKeyConfig {
            name: "Anthropic API Key".to_string(),
            key: api_key_config.anthropic_api_key.as_ref().map(|key| {
                if key.is_empty() {
                    "".to_string()
                } else {
                    format!("{}****", &key[..key.len().min(4)])
                }
            }),
            is_configured: api_key_config.anthropic_api_key.is_some()
                && !api_key_config
                    .anthropic_api_key
                    .as_ref()
                    .unwrap()
                    .is_empty(),
        });
    } else {
        // If no API keys config section exists, show as not configured
        api_keys.push(ApiKeyConfig {
            name: "Grok API Key".to_string(),
            key: None,
            is_configured: false,
        });
        api_keys.push(ApiKeyConfig {
            name: "OpenAI API Key".to_string(),
            key: None,
            is_configured: false,
        });
        api_keys.push(ApiKeyConfig {
            name: "Anthropic API Key".to_string(),
            key: None,
            is_configured: false,
        });
    }

    let response = SettingsResponse {
        config_file_path,
        api_keys,
    };

    Ok(HttpResponse::Ok().json(response))
}
