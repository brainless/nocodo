use crate::auth;
use crate::config::AppConfig;
use crate::database::Database;
use crate::error::AppError;
use crate::llm_agent::LlmAgent;
use crate::llm_client::LlmProvider;
use crate::models::LlmProviderConfig;
use crate::models::{
    AddExistingProjectRequest, AddMessageRequest, AiSessionListResponse, AiSessionOutput,
    AiSessionOutputListResponse, AiSessionResponse, ApiKeyConfig, CreateAiSessionRequest,
    CreateProjectRequest, CreateTeamRequest, CreateWorkRequest, FileContentResponse,
    FileCreateRequest, FileInfo, FileListRequest, FileListResponse, FileResponse, FileType,
    FileUpdateRequest, LlmAgentToolCallListResponse, Permission, Project, ProjectListResponse,
    ProjectResponse, ServerStatus, SettingsResponse, Team, UpdateApiKeysRequest, UpdateTeamRequest,
    UpdateUserRequest, User, UserListResponse, UserResponse, WorkListResponse, WorkMessageResponse,
    WorkResponse,
};
use crate::templates::{ProjectTemplate, TemplateManager};
use crate::websocket::WebSocketBroadcaster;
use actix_web::{web, HttpMessage, HttpResponse, Result};
use handlebars::Handlebars;
use nocodo_github_actions::{
    nocodo::WorkflowService, ExecuteCommandRequest, ExecuteCommandResponse, ScanWorkflowsRequest,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;


pub struct AppState {
    pub database: Arc<Database>,
    pub start_time: SystemTime,
    pub ws_broadcaster: Arc<WebSocketBroadcaster>,
    pub llm_agent: Option<Arc<LlmAgent>>, // LLM agent for direct LLM integration
    pub config: Arc<std::sync::RwLock<AppConfig>>,
}

/// Helper function to infer the provider from a model ID
fn infer_provider_from_model(model_id: &str) -> &str {
    let model_lower = model_id.to_lowercase();

    if model_lower.contains("gpt") || model_lower.contains("o1") || model_lower.starts_with("gpt-")
    {
        "openai"
    } else if model_lower.contains("claude")
        || model_lower.contains("opus")
        || model_lower.contains("sonnet")
        || model_lower.contains("haiku")
    {
        "anthropic"
    } else if model_lower.contains("grok") {
        "xai"
    } else {
        // Default to anthropic if we can't determine
        "anthropic"
    }
}

/// Helper function to get a user-friendly display name from a model ID by looking it up in the provider
pub async fn get_projects(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let projects = data.database.get_all_projects()?;
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

    // Get user ID from request and record ownership
    let user_id = http_req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    let ownership =
        crate::models::ResourceOwnership::new("project".to_string(), project_id, user_id);
    data.database.create_ownership(&ownership)?;

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

// User management handlers

pub async fn list_users(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let users = data.database.get_all_users()?;
    let response = UserListResponse { users };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn create_user(
    data: web::Data<AppState>,
    request: web::Json<crate::models::CreateUserRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let create_req = request.into_inner();

    // Validate username
    if create_req.username.trim().is_empty() {
        return Err(AppError::InvalidRequest(
            "Username cannot be empty".to_string(),
        ));
    }

    // Check if user already exists
    if data.database.get_user_by_username(&create_req.username).is_ok() {
        return Err(AppError::InvalidRequest(
            "Username already exists".to_string(),
        ));
    }

    // Hash password
    let password_hash = auth::hash_password(&create_req.password)?;

    // Get current user ID for created_by field (currently not used, but reserved for future audit logging)
    let _created_by = req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Create user
    let user = User {
        id: 0, // Will be set by database
        username: create_req.username,
        email: create_req.email.unwrap_or_default(),
        password_hash,
        is_active: true,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        updated_at: std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    };

    let user_id = data.database.create_user(&user)?;
    let mut user = user;
    user.id = user_id;

    let response = UserResponse { user };
    Ok(HttpResponse::Created().json(response))
}

pub async fn get_user(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let user = data.database.get_user_by_id(user_id)?;
    let response = UserResponse { user };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn update_user(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    request: web::Json<UpdateUserRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let update_req = request.into_inner();

    // Get current user for updated_by field (currently not used, but reserved for future audit logging)
    let _updated_by = req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Update user
    data.database.update_user(user_id, &update_req)?;

    let user = data.database.get_user_by_id(user_id)?;
    let response = UserResponse { user };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn delete_user(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    data.database.delete_user(user_id)?;
    Ok(HttpResponse::NoContent().finish())
}

// Team management handlers

pub async fn list_teams(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let teams = data.database.get_all_teams()?;
    Ok(HttpResponse::Ok().json(teams))
}

pub async fn create_team(
    data: web::Data<AppState>,
    request: web::Json<CreateTeamRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let create_req = request.into_inner();

    // Validate team name
    if create_req.name.trim().is_empty() {
        return Err(AppError::InvalidRequest(
            "Team name cannot be empty".to_string(),
        ));
    }

    // Get current user ID for created_by field
    let created_by = req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Create team
    let team = Team::new(create_req.name, create_req.description, created_by);
    let team_id = data.database.create_team(&team)?;
    let mut team = team;
    team.id = team_id;

    Ok(HttpResponse::Created().json(team))
}

pub async fn get_team(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    let team = data.database.get_team_by_id(team_id)?;
    Ok(HttpResponse::Ok().json(team))
}

pub async fn update_team(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    request: web::Json<UpdateTeamRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    let update_req = request.into_inner();

    // Get current user ID for updated_by field (currently not used, but reserved for future audit logging)
    let _updated_by = req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Update team
    data.database.update_team(team_id, &update_req)?;

    let team = data.database.get_team_by_id(team_id)?;
    Ok(HttpResponse::Ok().json(team))
}

pub async fn delete_team(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    data.database.delete_team(team_id)?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn get_team_members(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    let members = data.database.get_team_members(team_id)?;
    Ok(HttpResponse::Ok().json(members))
}

pub async fn add_team_member(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    request: web::Json<crate::models::AddTeamMemberRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    let add_req = request.into_inner();

    // Get current user ID for added_by field
    let added_by = req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Add team member
    data.database
        .add_team_member(team_id, add_req.user_id, Some(added_by))?;

    Ok(HttpResponse::Created().finish())
}

pub async fn remove_team_member(
    data: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, AppError> {
    let (team_id, user_id) = path.into_inner();
    data.database.remove_team_member(team_id, user_id)?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn get_team_permissions(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    let permissions = data.database.get_team_permissions(team_id)?;
    Ok(HttpResponse::Ok().json(permissions))
}

// Permission management handlers

pub async fn list_permissions(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let permissions = data.database.get_all_permissions()?;
    Ok(HttpResponse::Ok().json(permissions))
}

pub async fn create_permission(
    data: web::Data<AppState>,
    request: web::Json<crate::models::CreatePermissionRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let create_req = request.into_inner();

    // Get current user ID for granted_by field
    let granted_by = req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Create permission
    let permission = Permission::new(
        create_req.team_id,
        create_req.resource_type,
        create_req.resource_id,
        create_req.action,
        Some(granted_by),
    );

    let permission_id = data.database.create_permission(&permission)?;
    let mut permission = permission;
    permission.id = permission_id;

    Ok(HttpResponse::Created().json(permission))
}

pub async fn delete_permission(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let permission_id = path.into_inner();
    data.database.delete_permission(permission_id)?;
    Ok(HttpResponse::NoContent().finish())
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

// File operation handlers
pub async fn list_files(
    data: web::Data<AppState>,
    query: web::Query<FileListRequest>,
) -> Result<HttpResponse, AppError> {
    let request = query.into_inner();

    // Get the project to determine the base path
    let project = if let Some(project_id) = &request.project_id {
        data.database.get_project_by_id(*project_id)?
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

        let is_directory = metadata.is_dir();
        let file_info = FileInfo {
            name,
            path: relative_file_path.to_string_lossy().to_string(),
            absolute: path.to_string_lossy().to_string(),
            file_type: if is_directory {
                FileType::Directory
            } else {
                FileType::File
            },
            ignored: false, // TODO: Implement .gitignore checking for API responses
            is_directory,
            size: if is_directory {
                None
            } else {
                metadata.len().into()
            },
            modified_at: if is_directory {
                None
            } else {
                metadata.modified().ok().map(|t| {
                    t.duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .to_string()
                })
            },
        };

        files.push(file_info);
    }

    // Sort files: directories first, then by name
    files.sort_by(|a, b| match (&a.file_type, &b.file_type) {
        (FileType::Directory, FileType::File) => std::cmp::Ordering::Less,
        (FileType::File, FileType::Directory) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    let total_files = files.len() as u32;
    let response = FileListResponse {
        files,
        current_path: relative_path.to_string(),
        total_files,
        truncated: false, // API doesn't implement truncation for now
        limit: 1000,      // Default limit
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn create_file(
    data: web::Data<AppState>,
    request: web::Json<FileCreateRequest>,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    // Get the project to determine the base path
    let project = data.database.get_project_by_id(req.project_id)?;
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

    let is_directory = metadata.is_dir();
    let file_info = FileInfo {
        name: file_name,
        path: req.path.clone(),
        absolute: full_path.to_string_lossy().to_string(),
        file_type: if is_directory {
            FileType::Directory
        } else {
            FileType::File
        },
        ignored: false, // New files are not ignored
        is_directory,
        size: if is_directory {
            None
        } else {
            metadata.len().into()
        },
        modified_at: if is_directory {
            None
        } else {
            metadata.modified().ok().map(|t| format!("{:?}", t))
        },
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
    let file_path = urlencoding::decode(&path_param.into_inner())
        .map_err(|_| AppError::InvalidRequest("Invalid URL encoding in path".to_string()))?
        .to_string();
    let project_id_str = query
        .get("project_id")
        .ok_or_else(|| AppError::InvalidRequest("project_id is required".to_string()))?;
    let project_id = project_id_str
        .parse::<i64>()
        .map_err(|_| AppError::InvalidRequest("Invalid project_id".to_string()))?;

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
    let file_path = urlencoding::decode(&path_param.into_inner())
        .map_err(|_| AppError::InvalidRequest("Invalid URL encoding in path".to_string()))?
        .to_string();
    let req = request.into_inner();

    // Get the project to determine the base path
    let project = data.database.get_project_by_id(req.project_id)?;
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
    let file_path = urlencoding::decode(&path_param.into_inner())
        .map_err(|_| AppError::InvalidRequest("Invalid URL encoding in path".to_string()))?
        .to_string();
    let project_id_str = query
        .get("project_id")
        .ok_or_else(|| AppError::InvalidRequest("project_id is required".to_string()))?;
    let project_id = project_id_str
        .parse::<i64>()
        .map_err(|_| AppError::InvalidRequest("Invalid project_id".to_string()))?;

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
#[allow(dead_code)]
pub async fn create_ai_session(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    request: web::Json<CreateAiSessionRequest>,
    http_req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();
    let session_req = request.into_inner();

    // Validate required fields
    if session_req.tool_name.trim().is_empty() {
        return Err(AppError::InvalidRequest("tool_name is required".into()));
    }
    if session_req.message_id.trim().is_empty() {
        return Err(AppError::InvalidRequest("message_id is required".into()));
    }

    // Validate that work and message exist
    let work = data.database.get_work_by_id(work_id)?;
    let messages = data.database.get_work_messages(work_id)?;
    let message_id_i64 = session_req
        .message_id
        .parse::<i64>()
        .map_err(|_| AppError::InvalidRequest("Invalid message_id".to_string()))?;
    if !messages.iter().any(|m| m.id == message_id_i64) {
        return Err(AppError::InvalidRequest(
            "message_id not found in work".into(),
        ));
    }

    // Generate project context if work is associated with a project
    let project_context = if let Some(project_id) = work.project_id {
        let project = data.database.get_project_by_id(project_id)?;
        Some(format!("Project: {}\nPath: {}", project.name, project.path))
    } else {
        None
    };

    let mut session = crate::models::AiSession::new(
        work_id,
        message_id_i64,
        session_req.tool_name.clone(),
        project_context,
    );

    // Get user ID from request
    let user_id = http_req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Persist
    let session_id = data.database.create_ai_session(&session)?;
    session.id = session_id;

    // Record ownership for the AI session
    let ownership =
        crate::models::ResourceOwnership::new("ai_session".to_string(), session_id, user_id);
    data.database.create_ownership(&ownership)?;

    // Broadcast AI session creation via WebSocket
    data.ws_broadcaster
        .broadcast_ai_session_created(session.clone());

    // Response
    let response = AiSessionResponse {
        session: session.clone(),
    };

    // Handle LLM agent specially
    if session_req.tool_name == "llm-agent" {
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
                let project = data.database.get_project_by_id(*project_id)?;
                std::path::PathBuf::from(project.path)
            } else {
                std::env::current_dir().map_err(|e| {
                    AppError::Internal(format!("Failed to get current directory: {}", e))
                })?
            };

            // Determine provider and model from work.model or fall back to environment/defaults
            let (provider, model) = if let Some(ref model_id) = work.model {
                let provider = infer_provider_from_model(model_id);
                (provider.to_string(), model_id.clone())
            } else {
                // Fall back to environment variables or defaults
                let provider =
                    std::env::var("PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
                let model = std::env::var("MODEL")
                    .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
                (provider, model)
            };

            // Create LLM agent session with provider/model from environment
            let llm_session = llm_agent
                .create_session(work_id, provider, model, session.project_context.clone())
                .await?;

            // Process the message in background task to avoid blocking HTTP response
            let llm_agent_clone = llm_agent.clone();
            let session_id = llm_session.id;
            let message_content = message.content.clone();
            tokio::spawn(async move {
                if let Err(e) = llm_agent_clone
                    .process_message(session_id, message_content)
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
    // Note: AI session created but no execution backend enabled
    // Sessions can be executed externally or via LLM agent

    Ok(HttpResponse::Created().json(response))
}

pub async fn list_ai_sessions(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let sessions = data.database.get_all_ai_sessions()?;
    let response = AiSessionListResponse { sessions };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn list_ai_session_outputs(
    path: web::Path<i64>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();

    // First, get the AI session for this work
    let sessions = data.database.get_ai_sessions_by_work_id(work_id)?;

    if sessions.is_empty() {
        // No AI session found for this work, return empty outputs
        let response = AiSessionOutputListResponse { outputs: vec![] };
        return Ok(HttpResponse::Ok().json(response));
    }

    // Get the most recent AI session (in case there are multiple)
    let session = sessions.into_iter().max_by_key(|s| s.started_at).unwrap();

    // Get outputs for this session
    let mut outputs = data.database.list_ai_session_outputs(session.id)?;

    // If this is an LLM agent session, also fetch LLM agent messages
    if session.tool_name == "llm-agent" {
        if let Ok(llm_agent_session) = data.database.get_llm_agent_session_by_work_id(work_id) {
            if let Ok(llm_messages) = data.database.get_llm_agent_messages(llm_agent_session.id) {
                // Convert LLM agent messages to AiSessionOutput format
                for msg in llm_messages {
                    // Only include assistant messages (responses) and tool messages (results)
                    if msg.role == "assistant" || msg.role == "tool" {
                        let output = AiSessionOutput {
                            id: msg.id,
                            session_id: session.id,
                            content: msg.content,
                            created_at: msg.created_at,
                            role: Some(msg.role.clone()),
                            model: if msg.role == "assistant" {
                                Some(llm_agent_session.model.clone())
                            } else {
                                None
                            },
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

pub async fn list_ai_tool_calls(
    path: web::Path<i64>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();

    // First, get the AI session for this work
    let sessions = data.database.get_ai_sessions_by_work_id(work_id)?;

    if sessions.is_empty() {
        // No AI session found for this work, return empty tool calls
        let response = LlmAgentToolCallListResponse { tool_calls: vec![] };
        return Ok(HttpResponse::Ok().json(response));
    }

    // Get the most recent AI session (in case there are multiple)
    let session = sessions.into_iter().max_by_key(|s| s.started_at).unwrap();

    // Only fetch tool calls if this is an LLM agent session
    let tool_calls = if session.tool_name == "llm-agent" {
        if let Ok(llm_agent_session) = data.database.get_llm_agent_session_by_work_id(work_id) {
            data.database
                .get_llm_agent_tool_calls(llm_agent_session.id)
                .unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let response = LlmAgentToolCallListResponse { tool_calls };

    tracing::debug!(
        "Retrieved {} tool calls for work {}",
        response.tool_calls.len(),
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

// Work management handlers
pub async fn create_work(
    data: web::Data<AppState>,
    request: web::Json<CreateWorkRequest>,
    http_req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let work_req = request.into_inner();

    // Validate work title
    if work_req.title.trim().is_empty() {
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
        id: 0, // Will be set by database AUTOINCREMENT
        title: work_req.title.clone(),
        project_id: work_req.project_id,
        model: work_req.model.clone(),
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
    };

    // Get user ID from request
    let user_id = http_req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Create work with initial message in a single transaction
    let (work_id, message_id) = data
        .database
        .create_work_with_message(&work, work_req.title.clone())?;
    let mut work = work;
    work.id = work_id;

    // Record ownership
    let ownership = crate::models::ResourceOwnership::new("work".to_string(), work_id, user_id);
    data.database.create_ownership(&ownership)?;

    // Broadcast work creation via WebSocket
    data.ws_broadcaster.broadcast_project_created(Project {
        id: work.id,
        name: work.title.clone(),
        path: "".to_string(), // Works don't have a path like projects
        description: None,
        parent_id: None,
        created_at: work.created_at,
        updated_at: work.updated_at,
    });

    tracing::info!(
        "Successfully created work '{}' with ID {} and message ID {}",
        work.title,
        work.id,
        message_id
    );

    // Auto-start LLM agent session if requested (default: true)
    if work_req.auto_start {
        let tool_name = work_req
            .tool_name
            .unwrap_or_else(|| "llm-agent".to_string());

        // Generate project context if work is associated with a project
        let project_context = if let Some(project_id) = work.project_id {
            let project = data.database.get_project_by_id(project_id)?;
            Some(format!("Project: {}\nPath: {}", project.name, project.path))
        } else {
            None
        };

        // Create AI session record
        let session = crate::models::AiSession::new(
            work_id,
            message_id,
            tool_name.clone(),
            project_context.clone(),
        );
        let session_id = data.database.create_ai_session(&session)?;

        tracing::info!(
            "Auto-starting {} session {} for work {}",
            tool_name,
            session_id,
            work_id
        );

        // Handle LLM agent specially
        if tool_name == "llm-agent" {
            if let Some(ref llm_agent) = data.llm_agent {
                // Determine provider and model from work.model or fall back to environment/defaults
                let (provider, model) = if let Some(ref model_id) = work.model {
                    let provider = infer_provider_from_model(model_id);
                    (provider.to_string(), model_id.clone())
                } else {
                    // Fall back to environment variables or defaults
                    let provider =
                        std::env::var("PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
                    let model = std::env::var("MODEL")
                        .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
                    (provider, model)
                };

                // Create LLM agent session
                let llm_session = llm_agent
                    .create_session(work_id, provider, model, project_context)
                    .await?;

                // Process the message in background task to avoid blocking HTTP response
                let llm_agent_clone = llm_agent.clone();
                let session_id = llm_session.id;
                let message_content = work_req.title.clone();
                tokio::spawn(async move {
                    if let Err(e) = llm_agent_clone
                        .process_message(session_id, message_content)
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
                    "LLM agent not available - work {} will not have auto-started session",
                    work_id
                );
            }
        }
    }

    let response = WorkResponse { work };
    Ok(HttpResponse::Created().json(response))
}

pub async fn get_work(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();
    let work_with_history = data.database.get_work_with_messages(work_id)?;
    Ok(HttpResponse::Ok().json(work_with_history))
}

pub async fn list_works(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let works = data.database.get_all_works()?;
    let response = WorkListResponse { works };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn delete_work(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();

    // Delete from database
    data.database.delete_work(work_id)?;

    // Broadcast work deletion via WebSocket
    data.ws_broadcaster.broadcast_project_deleted(work_id);

    Ok(HttpResponse::NoContent().finish())
}

// Work message handlers
pub async fn add_message_to_work(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    request: web::Json<AddMessageRequest>,
    http_req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();
    let msg_req = request.into_inner();

    // Verify work exists
    let _work = data.database.get_work_by_id(work_id)?;

    // Get next sequence number
    let sequence_order = data.database.get_next_message_sequence(work_id)?;

    // Get user ID from request
    let user_id = http_req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Create the message object
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| AppError::Internal(format!("Failed to get timestamp: {e}")))?
        .as_secs() as i64;

    let mut message = crate::models::WorkMessage {
        id: 0, // Will be set by database AUTOINCREMENT
        work_id,
        content: msg_req.content,
        content_type: msg_req.content_type,
        author_type: msg_req.author_type,
        author_id: Some(user_id.to_string()), // Use authenticated user ID
        sequence_order,
        created_at: now,
    };

    // Save to database
    let message_id = data.database.create_work_message(&message)?;
    message.id = message_id;

    tracing::info!(
        "Successfully added message {} to work {}",
        message.id,
        work_id
    );

    // Auto-start AI session to continue the conversation
    let work = data.database.get_work_by_id(work_id)?;
    let tool_name = "llm-agent".to_string();

    // Generate project context if work is associated with a project
    let project_context = if let Some(project_id) = work.project_id {
        let project = data.database.get_project_by_id(project_id)?;
        Some(format!("Project: {}\nPath: {}", project.name, project.path))
    } else {
        None
    };

    // Create AI session record
    let session = crate::models::AiSession::new(
        work_id,
        message_id,
        tool_name.clone(),
        project_context.clone(),
    );
    let session_id = data.database.create_ai_session(&session)?;

    tracing::info!(
        "Auto-starting {} session {} for work {} (message continuation)",
        tool_name,
        session_id,
        work_id
    );

    // Handle LLM agent
    if let Some(ref llm_agent) = data.llm_agent {
        // Get existing LLM agent session for this work
        if let Ok(llm_session) = data.database.get_llm_agent_session_by_work_id(work_id) {
            // Use existing session - just process the new message
            let session_id = llm_session.id;
            let message_content = message.content.clone();
            let llm_agent_clone = llm_agent.clone();

            tokio::spawn(async move {
                if let Err(e) = llm_agent_clone
                    .process_message(session_id, message_content)
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
            // No existing session - create a new one
            let (provider, model) = if let Some(ref model_id) = work.model {
                let provider = infer_provider_from_model(model_id);
                (provider.to_string(), model_id.clone())
            } else {
                let provider =
                    std::env::var("PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
                let model = std::env::var("MODEL")
                    .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
                (provider, model)
            };

            let llm_session = llm_agent
                .create_session(work_id, provider, model, project_context)
                .await?;

            let llm_agent_clone = llm_agent.clone();
            let session_id = llm_session.id;
            let message_content = message.content.clone();

            tokio::spawn(async move {
                if let Err(e) = llm_agent_clone
                    .process_message(session_id, message_content)
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
        }
    } else {
        tracing::warn!(
            "LLM agent not available - work {} message {} will not be processed",
            work_id,
            message_id
        );
    }

    let response = WorkMessageResponse { message };
    Ok(HttpResponse::Created().json(response))
}

pub async fn get_work_messages(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let work_id = path.into_inner();

    // Verify work exists
    let _work = data.database.get_work_by_id(work_id)?;

    let messages = data.database.get_work_messages(work_id)?;
    let response = crate::models::WorkMessageListResponse { messages };
    Ok(HttpResponse::Ok().json(response))
}

// Workflow handlers

/// Scan workflows for a project
pub async fn scan_workflows(
    data: web::Data<AppState>,
    project_id: web::Path<i64>,
    _request: web::Json<ScanWorkflowsRequest>,
) -> Result<HttpResponse, AppError> {
    let project_id = project_id.into_inner();

    // Get project to verify it exists and get path
    let project = data.database.get_project_by_id(project_id)?;
    let project_path = PathBuf::from(&project.path);

    // Create workflow service
    let workflow_service = WorkflowService::new(data.database.connection());

    // Scan workflows
    let response = workflow_service
        .scan_workflows(&project_id.to_string(), &project_path)
        .map_err(|e| AppError::Internal(format!("Failed to scan workflows: {}", e)))?;

    Ok(HttpResponse::Ok().json(response))
}

/// Get workflow commands for a project
pub async fn get_workflow_commands(
    data: web::Data<AppState>,
    project_id: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let project_id = project_id.into_inner();

    // Verify project exists
    data.database.get_project_by_id(project_id)?;

    let commands = data
        .database
        .get_workflow_commands(project_id)
        .map_err(|e| AppError::Internal(format!("Failed to get workflow commands: {}", e)))?;

    Ok(HttpResponse::Ok().json(commands))
}

/// Execute a workflow command
pub async fn execute_workflow_command(
    data: web::Data<AppState>,
    path: web::Path<(String, String)>,
    request: web::Json<ExecuteCommandRequest>,
) -> Result<HttpResponse, AppError> {
    let (project_id_str, command_id) = path.into_inner();
    let project_id = project_id_str
        .parse::<i64>()
        .map_err(|_| AppError::InvalidRequest("Invalid project_id".to_string()))?;
    let request = request.into_inner();

    // Verify project exists
    data.database.get_project_by_id(project_id)?;

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
            project_id: project_id.to_string(),
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
    let (project_id_str, command_id) = path.into_inner();
    let project_id = project_id_str
        .parse::<i64>()
        .map_err(|_| AppError::InvalidRequest("Invalid project_id".to_string()))?;

    // Verify project exists
    data.database.get_project_by_id(project_id)?;

    let executions = data
        .database
        .get_command_executions(
            command_id
                .parse::<i64>()
                .map_err(|_| AppError::InvalidRequest("Invalid command_id".to_string()))?,
        )
        .map_err(|e| AppError::Internal(format!("Failed to get command executions: {}", e)))?;

    Ok(HttpResponse::Ok().json(executions))
}

/// Get settings information including API key configuration
pub async fn get_settings(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    // Use the in-memory config to get latest settings
    let config = data
        .config
        .read()
        .map_err(|e| AppError::Internal(format!("Failed to acquire config read lock: {}", e)))?;

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
            key: api_key_config.xai_api_key.as_ref().map(|key| {
                if key.is_empty() {
                    "".to_string()
                } else {
                    format!("{}****", &key[..key.len().min(4)])
                }
            }),
            is_configured: api_key_config.xai_api_key.is_some()
                && !api_key_config.xai_api_key.as_ref().unwrap().is_empty(),
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
        tracing::info!("No API keys config section found");
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
        projects_default_path: config
            .projects
            .as_ref()
            .and_then(|projects| projects.default_path.clone()),
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Update API keys
pub async fn update_api_keys(
    data: web::Data<AppState>,
    request: web::Json<UpdateApiKeysRequest>,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    tracing::info!("Updating API keys");

    // Load current config
    let mut config = {
        let config_read = data.config.read().map_err(|e| {
            AppError::Internal(format!("Failed to acquire config read lock: {}", e))
        })?;
        config_read.clone()
    };

    // Initialize api_keys section if it doesn't exist
    if config.api_keys.is_none() {
        config.api_keys = Some(crate::config::ApiKeysConfig {
            xai_api_key: None,
            openai_api_key: None,
            anthropic_api_key: None,
        });
    }

    // Update the API keys (only update if provided in request)
    if let Some(ref mut api_keys) = config.api_keys {
        if let Some(xai_key) = req.xai_api_key {
            if !xai_key.is_empty() {
                api_keys.xai_api_key = Some(xai_key);
                tracing::info!("Updated xAI API key");
            } else {
                api_keys.xai_api_key = None;
                tracing::info!("Cleared xAI API key");
            }
        }

        if let Some(openai_key) = req.openai_api_key {
            if !openai_key.is_empty() {
                api_keys.openai_api_key = Some(openai_key);
                tracing::info!("Updated OpenAI API key");
            } else {
                api_keys.openai_api_key = None;
                tracing::info!("Cleared OpenAI API key");
            }
        }

        if let Some(anthropic_key) = req.anthropic_api_key {
            if !anthropic_key.is_empty() {
                api_keys.anthropic_api_key = Some(anthropic_key);
                tracing::info!("Updated Anthropic API key");
            } else {
                api_keys.anthropic_api_key = None;
                tracing::info!("Cleared Anthropic API key");
            }
        }
    }

    // Save config to file
    let config_path = if let Some(home) = home::home_dir() {
        home.join(".config/nocodo/manager.toml")
    } else {
        std::path::PathBuf::from("manager.toml")
    };

    let toml_string = toml::to_string(&config)
        .map_err(|e| AppError::Internal(format!("Failed to serialize config: {}", e)))?;

    std::fs::write(&config_path, toml_string)
        .map_err(|e| AppError::Internal(format!("Failed to write config file: {}", e)))?;

    // Update the in-memory config as well
    {
        let mut config_write = data.config.write().map_err(|e| {
            AppError::Internal(format!("Failed to acquire config write lock: {}", e))
        })?;
        *config_write = config;
    }

    tracing::info!("API keys updated successfully");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "API keys updated successfully"
    })))
}

/// Set projects default path
pub async fn set_projects_default_path(
    data: web::Data<AppState>,
    request: web::Json<serde_json::Value>,
) -> Result<HttpResponse, AppError> {
    let path = request
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidRequest("path field is required".to_string()))?;

    // Expand ~ to home directory if present
    let expanded_path = if path.starts_with("~/") {
        if let Some(home) = home::home_dir() {
            path.replacen("~", &home.to_string_lossy(), 1)
        } else {
            return Err(AppError::InvalidRequest(
                "Cannot expand ~: home directory not found".to_string(),
            ));
        }
    } else {
        path.to_string()
    };

    // Validate path exists and is a directory
    let path_obj = std::path::Path::new(&expanded_path);
    if !path_obj.exists() {
        return Err(AppError::InvalidRequest(format!(
            "Path does not exist: {} (expanded from: {})",
            expanded_path, path
        )));
    }

    if !path_obj.is_dir() {
        return Err(AppError::InvalidRequest(format!(
            "Path is not a directory: {} (expanded from: {})",
            expanded_path, path
        )));
    }

    // Update config
    let mut config = {
        let config_read = data.config.read().map_err(|e| {
            AppError::Internal(format!("Failed to acquire config read lock: {}", e))
        })?;
        config_read.clone()
    };
    if let Some(ref mut projects) = config.projects {
        projects.default_path = Some(expanded_path.clone());
    } else {
        config.projects = Some(crate::config::ProjectsConfig {
            default_path: Some(expanded_path.clone()),
        });
    }

    // Save config to file
    let config_path = if let Some(home) = home::home_dir() {
        home.join(".config/nocodo/manager.toml")
    } else {
        std::path::PathBuf::from("manager.toml")
    };

    let toml_string = toml::to_string(&config)
        .map_err(|e| AppError::Internal(format!("Failed to serialize config: {}", e)))?;

    std::fs::write(&config_path, toml_string)
        .map_err(|e| AppError::Internal(format!("Failed to write config file: {}", e)))?;

    // Update the in-memory config as well
    {
        let mut config_write = data.config.write().map_err(|e| {
            AppError::Internal(format!("Failed to acquire config write lock: {}", e))
        })?;
        *config_write = config;
    }

    tracing::info!("Updated projects default path to: {}", expanded_path);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "path": expanded_path
    })))
}

/// Scan projects default path and save as Project entities
pub async fn scan_projects(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    tracing::info!("scan_projects endpoint called");

    // Reload config from file to get latest projects_default_path
    let config = AppConfig::load().map_err(|e| {
        tracing::error!("Failed to reload config: {}", e);
        AppError::Internal(format!("Failed to reload config: {}", e))
    })?;

    tracing::info!("Reloaded config api_keys: {:?}", config.api_keys);

    let projects_path = if let Some(projects) = &config.projects {
        if let Some(path) = &projects.default_path {
            path.clone()
        } else {
            tracing::error!("Projects default path not configured in reloaded config");
            return Err(AppError::InvalidRequest(
                "Projects default path not configured. Please set it in Settings first."
                    .to_string(),
            ));
        }
    } else {
        tracing::error!("Projects configuration not found in reloaded config");
        return Err(AppError::InvalidRequest(
            "Projects configuration not found. Please set the projects path in Settings."
                .to_string(),
        ));
    };

    tracing::info!("Scanning projects directory: {}", projects_path);
    let projects_dir = std::path::Path::new(&projects_path);

    if !projects_dir.exists() {
        return Err(AppError::InvalidRequest(format!(
            "Projects directory does not exist: {}",
            projects_path
        )));
    }

    if !projects_dir.is_dir() {
        return Err(AppError::InvalidRequest(format!(
            "Projects path is not a directory: {}",
            projects_path
        )));
    }

    // Read directory contents
    let entries = std::fs::read_dir(projects_dir)
        .map_err(|e| AppError::Internal(format!("Failed to read projects directory: {}", e)))?;

    let mut created_projects = Vec::new();
    let mut entry_count = 0;

    for entry in entries {
        entry_count += 1;
        let entry =
            entry.map_err(|e| AppError::Internal(format!("Failed to read entry: {}", e)))?;
        let path = entry.path();

        if path.is_dir() {
            let project_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("<invalid>")
                .to_string();

            let project_path = path.to_string_lossy().to_string();

            // Check if project already exists in database
            if let Ok(_existing) = data.database.get_project_by_path(&project_path) {
                tracing::info!("Project already exists: {}", project_name);
                continue;
            }

            // Create new project
            let mut project = Project::new(project_name.clone(), project_path.clone());

            // Try to detect project type from common files
            if path.join("package.json").exists() {
                project.description = Some("Node.js project".to_string());
            } else if path.join("Cargo.toml").exists() {
                project.description = Some("Rust project".to_string());
            } else if path.join("pyproject.toml").exists() || path.join("requirements.txt").exists()
            {
                project.description = Some("Python project".to_string());
            } else if path.join("go.mod").exists() {
                project.description = Some("Go project".to_string());
            } else {
                project.description = Some("Project".to_string());
            }

            // This function only scans the filesystem, it doesn't create projects in the database
            // The ownership recording code was misplaced here

            created_projects.push(project.clone());

            tracing::info!("Created project: {} at {}", project.name, project.path);
        }
    }

    tracing::info!(
        "Successfully scanned projects directory: {} (found {} entries, created {} projects)",
        projects_path,
        entry_count,
        created_projects.len()
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "created_projects": created_projects.len(),
        "projects": created_projects
    })))
}

/// Get list of supported and enabled models
pub async fn get_supported_models(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    tracing::info!("get_supported_models endpoint called");
    let api_key_config = data
        .config
        .read()
        .map_err(|e| AppError::Internal(format!("Failed to acquire config read lock: {}", e)))?
        .api_keys
        .clone();
    let mut models = Vec::new();

    tracing::info!("Checking for configured API keys in get_supported_models");

    // Check if API keys are configured and add enabled models
    if let Some(api_key_config) = api_key_config {
        tracing::info!("API keys config found, checking individual keys");
        // OpenAI models
        if api_key_config.openai_api_key.is_some()
            && !api_key_config.openai_api_key.as_ref().unwrap().is_empty()
        {
            tracing::info!("OpenAI API key is configured, creating provider");
            let openai_config = LlmProviderConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(), // Default model for checking
                api_key: api_key_config.openai_api_key.as_ref().unwrap().clone(),
                base_url: None,
                max_tokens: Some(1000),
                temperature: Some(0.7),
            };

            match crate::llm_providers::OpenAiProvider::new(openai_config) {
                Ok(provider) => {
                    tracing::info!("OpenAI provider created successfully");
                    match provider.list_available_models().await {
                        Ok(available_models) => {
                            tracing::info!("Found {} OpenAI models", available_models.len());
                            for model in available_models {
                                models.push(crate::models::SupportedModel {
                                    provider: "openai".to_string(),
                                    model_id: model.id().to_string(),
                                    name: model.name().to_string(),
                                    context_length: model.context_length(),
                                    supports_streaming: model.supports_streaming(),
                                    supports_tool_calling: model.supports_tool_calling(),
                                    supports_vision: model.supports_vision(),
                                    supports_reasoning: model.supports_reasoning(),
                                    input_cost_per_token: model.input_cost_per_token(),
                                    output_cost_per_token: model.output_cost_per_token(),
                                    default_temperature: model.default_temperature(),
                                    default_max_tokens: model.default_max_tokens(),
                                });
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to list OpenAI models: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create OpenAI provider: {}", e);
                }
            }
        }

        // Anthropic models
        if api_key_config.anthropic_api_key.is_some()
            && !api_key_config
                .anthropic_api_key
                .as_ref()
                .unwrap()
                .is_empty()
        {
            tracing::info!("Anthropic API key is configured, creating provider");
            let anthropic_config = LlmProviderConfig {
                provider: "anthropic".to_string(),
                model: "claude-3-opus-20240229".to_string(), // Default model for checking
                api_key: api_key_config.anthropic_api_key.as_ref().unwrap().clone(),
                base_url: None,
                max_tokens: Some(1000),
                temperature: Some(0.7),
            };

            match crate::llm_providers::AnthropicProvider::new(anthropic_config) {
                Ok(provider) => {
                    tracing::info!("Anthropic provider created successfully");
                    match provider.list_available_models().await {
                        Ok(available_models) => {
                            tracing::info!("Found {} Anthropic models", available_models.len());
                            for model in available_models {
                                models.push(crate::models::SupportedModel {
                                    provider: "anthropic".to_string(),
                                    model_id: model.id().to_string(),
                                    name: model.name().to_string(),
                                    context_length: model.context_length(),
                                    supports_streaming: model.supports_streaming(),
                                    supports_tool_calling: model.supports_tool_calling(),
                                    supports_vision: model.supports_vision(),
                                    supports_reasoning: model.supports_reasoning(),
                                    input_cost_per_token: model.input_cost_per_token(),
                                    output_cost_per_token: model.output_cost_per_token(),
                                    default_temperature: model.default_temperature(),
                                    default_max_tokens: model.default_max_tokens(),
                                });
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to list Anthropic models: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create Anthropic provider: {}", e);
                }
            }
        } else {
            tracing::info!("Anthropic API key not configured");
        }

        // xAI models
        if api_key_config.xai_api_key.is_some()
            && !api_key_config.xai_api_key.as_ref().unwrap().is_empty()
        {
            tracing::info!("xAI API key is configured, creating provider");
            let xai_config = LlmProviderConfig {
                provider: "xai".to_string(),
                model: "grok-code-fast-1".to_string(), // Default model for checking
                api_key: api_key_config.xai_api_key.as_ref().unwrap().clone(),
                base_url: Some("https://api.x.ai".to_string()),
                max_tokens: Some(1000),
                temperature: Some(0.7),
            };

            match crate::llm_providers::XaiProvider::new(xai_config) {
                Ok(provider) => {
                    tracing::info!("xAI provider created successfully");
                    match provider.list_available_models().await {
                        Ok(available_models) => {
                            tracing::info!("Found {} xAI models", available_models.len());
                            for model in available_models {
                                models.push(crate::models::SupportedModel {
                                    provider: "xai".to_string(),
                                    model_id: model.id().to_string(),
                                    name: model.name().to_string(),
                                    context_length: model.context_length(),
                                    supports_streaming: model.supports_streaming(),
                                    supports_tool_calling: model.supports_tool_calling(),
                                    supports_vision: model.supports_vision(),
                                    supports_reasoning: model.supports_reasoning(),
                                    input_cost_per_token: model.input_cost_per_token(),
                                    output_cost_per_token: model.output_cost_per_token(),
                                    default_temperature: model.default_temperature(),
                                    default_max_tokens: model.default_max_tokens(),
                                });
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to list xAI models: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create xAI provider: {}", e);
                }
            }
        } else {
            tracing::info!("xAI API key not configured");
        }
    }

    tracing::info!("Returning {} supported models", models.len());
    let response = crate::models::SupportedModelsResponse { models };
    Ok(HttpResponse::Ok().json(response))
}

// Authentication handlers

/// Registration handler - allows self-registration, with bootstrap logic for first user
pub async fn register(
    data: web::Data<AppState>,
    register_req: web::Json<crate::models::CreateUserRequest>,
) -> Result<HttpResponse, AppError> {
    let create_req = register_req.into_inner();
    tracing::info!("Registration attempt for user: {}", create_req.username);

    // Validate username
    if create_req.username.trim().is_empty() {
        return Err(AppError::InvalidRequest(
            "Username cannot be empty".to_string(),
        ));
    }

    // Check if user already exists
    if data.database.get_user_by_username(&create_req.username).is_ok() {
        return Err(AppError::InvalidRequest(
            "Username already exists".to_string(),
        ));
    }

    // Hash password
    let password_hash = auth::hash_password(&create_req.password)?;

    // Create user
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let user = User {
        id: 0, // Will be set by database
        username: create_req.username.clone(),
        email: create_req.email.unwrap_or_default(),
        password_hash,
        is_active: true,
        created_at: now,
        updated_at: now,
    };

    let user_id = data.database.create_user(&user)?;
    let mut user = user;
    user.id = user_id;
    tracing::info!("User created with ID: {}", user_id);

    // Create SSH key record for the user
    let ssh_key = crate::models::UserSshKey {
        id: 0, // Will be set by database
        user_id,
        key_type: "ssh-ed25519".to_string(), // Default assumption, could be improved
        fingerprint: create_req.ssh_fingerprint.clone(),
        public_key_data: create_req.ssh_public_key.clone(),
        label: Some("Registration Key".to_string()),
        is_active: true,
        created_at: now,
        last_used_at: None,
    };

    let ssh_key_id = data.database.create_ssh_key(&ssh_key)?;
    tracing::info!("SSH key created with ID: {} for user: {}", ssh_key_id, user.username);

    // Check if this is the first user (bootstrap logic)
    let user_count = data.database.get_all_users()?.len();
    if user_count == 1 {
        // This is the first user - create Super Admins team and grant admin permissions
        tracing::info!("First user registered: {} - creating Super Admins team", user.username);

        // Create "Super Admins" team
        let super_admin_team = Team {
            id: 0,
            name: "Super Admins".to_string(),
            description: Some("System administrators with full access".to_string()),
            created_by: user_id,
            created_at: now,
            updated_at: now,
        };

        let team_id = data.database.create_team(&super_admin_team)?;

        // Add first user to the team
        data.database.add_team_member(team_id, user_id, Some(user_id))?;

        // Grant entity-level admin permissions on all resource types
        let resource_types = ["project", "work", "settings", "user", "team"];
        for resource_type in &resource_types {
            let permission = Permission {
                id: 0,
                team_id,
                resource_type: resource_type.to_string(),
                resource_id: None, // Entity-level (all resources of this type)
                action: "admin".to_string(),
                granted_by: Some(user_id),
                granted_at: now,
            };
            data.database.create_permission(&permission)?;
        }

        tracing::info!("Bootstrap complete: Super Admins team created with full admin permissions");
    } else {
        tracing::info!("User registered: {} (not first user, no auto-permissions)", user.username);
    }

    tracing::info!("Registration successful for user: {}", user.username);
    let response = UserResponse { user };
    Ok(HttpResponse::Created().json(response))
}

/// Login handler - authenticates user with password and SSH key fingerprint
pub async fn login(
    data: web::Data<AppState>,
    login_req: web::Json<crate::models::LoginRequest>,
) -> Result<HttpResponse> {
    tracing::info!("Login attempt for user: {} with SSH fingerprint: {}", login_req.username, login_req.ssh_fingerprint);

    // 1. Get user by username
    let user = data
        .database
        .get_user_by_username(&login_req.username)
        .map_err(|_| AppError::AuthenticationFailed("Invalid username or password".to_string()))?;

    // 2. Check if user is active
    if !user.is_active {
        tracing::warn!("Login attempt for inactive user: {}", user.username);
        return Err(AppError::AuthenticationFailed("User account is inactive".to_string()).into());
    }

    // 3. Verify password
    let password_valid =
        auth::verify_password(&login_req.password, &user.password_hash).map_err(|e| {
            tracing::error!("Password verification error: {}", e);
            AppError::Internal("Authentication system error".to_string())
        })?;

    if !password_valid {
        tracing::warn!("Invalid password for user: {}", user.username);
        return Err(
            AppError::AuthenticationFailed("Invalid username or password".to_string()).into(),
        );
    }

    // 4. Verify SSH key fingerprint
    let ssh_key = data
        .database
        .get_ssh_key_by_fingerprint(&login_req.ssh_fingerprint)
        .map_err(|_| {
            tracing::warn!(
                "SSH key not found for user {}: {}",
                user.username,
                login_req.ssh_fingerprint
            );
            AppError::AuthenticationFailed("Invalid SSH key".to_string())
        })?;

    // 5. Verify SSH key belongs to this user
    if ssh_key.user_id != user.id {
        tracing::warn!(
            "SSH key {} does not belong to user {}",
            login_req.ssh_fingerprint,
            user.username
        );
        return Err(AppError::AuthenticationFailed("Invalid SSH key".to_string()).into());
    }

    // 6. Update SSH key last_used_at
    if let Err(e) = data.database.update_ssh_key_last_used(ssh_key.id) {
        tracing::error!("Failed to update SSH key last_used_at: {}", e);
        // Non-fatal error, continue with login
    }

    // 7. Generate JWT token
    let config = data.config.read().unwrap();
    let jwt_secret = config
        .jwt_secret
        .as_ref()
        .ok_or_else(|| AppError::Internal("JWT secret not configured".to_string()))?;

    let claims = auth::Claims::new(
        user.id,
        user.username.clone(),
        Some(login_req.ssh_fingerprint.clone()),
    );

    let token = auth::generate_token(&claims, jwt_secret).map_err(|e| {
        tracing::error!("Failed to generate token: {}", e);
        AppError::Internal("Failed to generate authentication token".to_string())
    })?;

    tracing::info!("Successful login for user: {}", user.username);

    // 8. Return success response
    let user_info = crate::models::UserInfo {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
    };
    let response = crate::models::LoginResponse {
        token,
        user: user_info,
    };
    tracing::info!("Login response sent for user: {}", user.username);
    Ok(HttpResponse::Ok().json(response))
}
