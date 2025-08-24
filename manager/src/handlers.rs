use crate::database::Database;
use crate::error::AppError;
use crate::models::{
    AddExistingProjectRequest, AiSessionListResponse, AiSessionOutputListResponse,
    AiSessionResponse, CreateAiSessionRequest, CreateProjectRequest, FileContentResponse,
    FileCreateRequest, FileInfo, FileListRequest, FileListResponse, FileResponse,
    FileUpdateRequest, Project, ProjectListResponse, ProjectResponse, RecordAiOutputRequest,
    ServerStatus,
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
        std::fs::create_dir_all(&absolute_project_path).map_err(|e| {
            AppError::Internal(format!("Failed to create project directory: {e}"))
        })?;

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

    // Create the project object
    let mut project = Project::new(req.name.clone(), absolute_path_str);
    project.language = req.language;
    project.framework = req.framework;
    project.status = "registered".to_string(); // Different status to distinguish from created projects

    // Save to database
    data.database.create_project(&project)?;

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
        let entry =
            entry.map_err(|e| AppError::Internal(format!("Failed to read entry: {e}")))?;
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
    request: web::Json<CreateAiSessionRequest>,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    // Optional: Validate tool_name/prompt
    if req.tool_name.trim().is_empty() {
        return Err(AppError::InvalidRequest("tool_name is required".into()));
    }
    if req.prompt.trim().is_empty() {
        return Err(AppError::InvalidRequest("prompt is required".into()));
    }

    // Generate simple context if project_id present
    let project_context = if let Some(ref project_id) = req.project_id {
        let project = data.database.get_project_by_id(project_id)?;
        Some(format!("Project: {}\nPath: {}", project.name, project.path))
    } else {
        None
    };

    let session = crate::models::AiSession::new(
        req.project_id.clone(),
        req.tool_name,
        req.prompt,
        project_context,
    );

    // Persist
    data.database.create_ai_session(&session)?;

    // Response
    let response = AiSessionResponse {
        session: session.clone(),
    };
    Ok(HttpResponse::Created().json(response))
}

pub async fn list_ai_sessions(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let sessions = data.database.get_all_ai_sessions()?;
    let response = AiSessionListResponse { sessions };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn get_ai_session(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let session = data.database.get_ai_session_by_id(&id)?;
    let response = AiSessionResponse { session };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn record_ai_output(
    data: web::Data<AppState>,
    path: web::Path<String>,
    request: web::Json<RecordAiOutputRequest>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let req = request.into_inner();

    if req.content.trim().is_empty() {
        return Err(AppError::InvalidRequest("content is required".into()));
    }

    // Ensure session exists
    let _ = data.database.get_ai_session_by_id(&id)?;

    data.database.create_ai_session_output(&id, &req.content)?;
    Ok(HttpResponse::Created().json(serde_json::json!({"ok": true})))
}

pub async fn list_ai_outputs(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    // Ensure session exists
    let _ = data.database.get_ai_session_by_id(&id)?;

    let outputs = data.database.get_ai_session_outputs(&id)?;
    let response = AiSessionOutputListResponse { outputs };
    Ok(HttpResponse::Ok().json(response))
}
