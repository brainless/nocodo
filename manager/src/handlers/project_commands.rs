use crate::command_discovery::CommandDiscovery;
use crate::database::Database;
use crate::error::AppError;
use crate::models::{
    CreateProjectCommandRequest, ExecuteProjectCommandRequest, ProjectCommand,
    ProjectCommandExecution, ProjectCommandExecutionListResponse,
    ProjectCommandExecutionResponse, ProjectCommandFilterQuery, ProjectCommandListResponse,
    ProjectCommandResponse, UpdateProjectCommandRequest,
};
use actix_web::{web, HttpResponse, Result};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};
use uuid::Uuid;

/// GET /api/projects/{id}/commands
/// List all commands for a project
pub async fn get_project_commands(
    project_id: web::Path<i64>,
    query: web::Query<ProjectCommandFilterQuery>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let project_id = project_id.into_inner();
    info!("Getting commands for project {}", project_id);

    // Verify project exists
    db.get_project_by_id(project_id)
        .map_err(|_| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let mut commands = db.get_project_commands(project_id)?;

    // Apply search filter if provided
    if let Some(search) = &query.search {
        let search_lower = search.to_lowercase();
        commands.retain(|cmd| {
            cmd.name.to_lowercase().contains(&search_lower)
                || cmd
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&search_lower))
                    .unwrap_or(false)
        });
    }

    // Apply pagination if provided
    if let Some(offset) = query.offset {
        commands = commands.into_iter().skip(offset as usize).collect();
    }

    if let Some(limit) = query.limit {
        commands.truncate(limit as usize);
    }

    Ok(HttpResponse::Ok().json(ProjectCommandListResponse { commands }))
}

/// GET /api/projects/{id}/commands/{cmd_id}
/// Get a specific command
pub async fn get_project_command(
    path: web::Path<(i64, String)>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();
    info!("Getting command {} for project {}", command_id, project_id);

    let command = db.get_project_command_by_id(&command_id)?;

    // Verify command belongs to project
    if command.project_id != project_id {
        return Err(AppError::NotFound(format!(
            "Command {} not found in project {}",
            command_id, project_id
        )));
    }

    Ok(HttpResponse::Ok().json(ProjectCommandResponse { command }))
}

/// POST /api/projects/{id}/commands
/// Create a new command
pub async fn create_project_command(
    project_id: web::Path<i64>,
    request: web::Json<CreateProjectCommandRequest>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let project_id = project_id.into_inner();
    info!("Creating command for project {}", project_id);

    // Verify project exists
    db.get_project_by_id(project_id)
        .map_err(|_| AppError::NotFound(format!("Project {} not found", project_id)))?;

    // Validate request
    if request.name.is_empty() {
        return Err(AppError::InvalidRequest("Command name is required".into()));
    }
    if request.command.is_empty() {
        return Err(AppError::InvalidRequest("Command is required".into()));
    }

    let now = Utc::now().timestamp();
    let command = ProjectCommand {
        id: Uuid::new_v4().to_string(),
        project_id,
        name: request.name.clone(),
        description: request.description.clone(),
        command: request.command.clone(),
        shell: request.shell.clone(),
        working_directory: request.working_directory.clone(),
        environment: request.environment.clone(),
        timeout_seconds: request.timeout_seconds,
        os_filter: request.os_filter.clone(),
        created_at: now,
        updated_at: now,
    };

    db.create_project_command(&command)?;

    info!("Created command {} for project {}", command.id, project_id);
    Ok(HttpResponse::Created().json(ProjectCommandResponse { command }))
}

/// PUT /api/projects/{id}/commands/{cmd_id}
/// Update a command
pub async fn update_project_command(
    path: web::Path<(i64, String)>,
    request: web::Json<UpdateProjectCommandRequest>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();
    info!("Updating command {} for project {}", command_id, project_id);

    let mut command = db.get_project_command_by_id(&command_id)?;

    // Verify command belongs to project
    if command.project_id != project_id {
        return Err(AppError::NotFound(format!(
            "Command {} not found in project {}",
            command_id, project_id
        )));
    }

    // Apply updates
    if let Some(name) = &request.name {
        if name.is_empty() {
            return Err(AppError::InvalidRequest("Command name cannot be empty".into()));
        }
        command.name = name.clone();
    }
    if let Some(description) = &request.description {
        command.description = Some(description.clone());
    }
    if let Some(cmd) = &request.command {
        if cmd.is_empty() {
            return Err(AppError::InvalidRequest("Command cannot be empty".into()));
        }
        command.command = cmd.clone();
    }
    if let Some(shell) = &request.shell {
        command.shell = Some(shell.clone());
    }
    if let Some(working_directory) = &request.working_directory {
        command.working_directory = Some(working_directory.clone());
    }
    if let Some(environment) = &request.environment {
        command.environment = Some(environment.clone());
    }
    if let Some(timeout_seconds) = request.timeout_seconds {
        command.timeout_seconds = Some(timeout_seconds);
    }
    if let Some(os_filter) = &request.os_filter {
        command.os_filter = Some(os_filter.clone());
    }

    command.updated_at = Utc::now().timestamp();

    db.update_project_command(&command)?;

    info!("Updated command {} for project {}", command_id, project_id);
    Ok(HttpResponse::Ok().json(ProjectCommandResponse { command }))
}

/// DELETE /api/projects/{id}/commands/{cmd_id}
/// Delete a command
pub async fn delete_project_command(
    path: web::Path<(i64, String)>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();
    info!("Deleting command {} for project {}", command_id, project_id);

    let command = db.get_project_command_by_id(&command_id)?;

    // Verify command belongs to project
    if command.project_id != project_id {
        return Err(AppError::NotFound(format!(
            "Command {} not found in project {}",
            command_id, project_id
        )));
    }

    db.delete_project_command(&command_id)?;

    info!("Deleted command {} for project {}", command_id, project_id);
    Ok(HttpResponse::NoContent().finish())
}

/// POST /api/projects/{id}/commands/{cmd_id}/execute
/// Execute a command
pub async fn execute_project_command(
    path: web::Path<(i64, String)>,
    request: web::Json<ExecuteProjectCommandRequest>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();
    info!(
        "Executing command {} for project {} on branch {:?}",
        command_id, project_id, request.git_branch
    );

    // Get command
    let command = db.get_project_command_by_id(&command_id)?;

    // Verify command belongs to project
    if command.project_id != project_id {
        return Err(AppError::NotFound(format!(
            "Command {} not found in project {}",
            command_id, project_id
        )));
    }

    // Get project to determine execution path
    let project = db
        .get_project_by_id(project_id)
        .map_err(|_| AppError::NotFound(format!("Project {} not found", project_id)))?;

    // Determine working directory based on git_branch
    let base_path = match &request.git_branch {
        None => {
            // Execute in main branch
            std::path::PathBuf::from(&project.path)
        }
        Some(branch) => {
            // Execute in worktree
            let worktree_path = crate::git::get_working_directory_for_branch(
                &std::path::PathBuf::from(&project.path),
                branch,
            )
            .map_err(|e| {
                AppError::InvalidRequest(format!("Failed to get worktree path: {}", e))
            })?;
            std::path::PathBuf::from(worktree_path)
        }
    };

    // Append command's working_directory if specified
    let execution_path = if let Some(working_dir) = &command.working_directory {
        base_path.join(working_dir)
    } else {
        base_path
    };

    // Verify execution path exists
    if !execution_path.exists() {
        return Err(AppError::InvalidRequest(format!(
            "Execution path does not exist: {:?}",
            execution_path
        )));
    }

    // Merge environment variables
    let mut env = command.environment.clone().unwrap_or_default();
    if let Some(request_env) = &request.environment {
        env.extend(request_env.clone());
    }

    // Determine timeout
    let timeout_seconds = request
        .timeout_seconds
        .or(command.timeout_seconds)
        .unwrap_or(120);

    // Execute command
    let start_time = Instant::now();

    // Build shell command
    let shell = command.shell.as_deref().unwrap_or("bash");
    let full_command = format!("cd {:?} && {}", execution_path, command.command);

    info!("Executing: {} (timeout: {}s)", full_command, timeout_seconds);

    // Execute using tokio::process::Command
    let mut cmd = tokio::process::Command::new(shell);
    cmd.arg("-c").arg(&command.command).current_dir(&execution_path);

    // Set environment variables
    for (key, value) in env {
        cmd.env(key, value);
    }

    let output = match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_seconds),
        cmd.output(),
    )
    .await
    {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            error!("Failed to execute command: {}", e);
            return Err(AppError::Internal(format!("Failed to execute command: {}", e)));
        }
        Err(_) => {
            error!("Command execution timed out after {}s", timeout_seconds);
            return Err(AppError::Internal(format!(
                "Command execution timed out after {}s",
                timeout_seconds
            )));
        }
    };

    let duration_ms = start_time.elapsed().as_millis() as u64;
    let exit_code = output.status.code();
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    info!(
        "Command execution completed: exit_code={:?}, duration={}ms",
        exit_code, duration_ms
    );

    // Record execution
    let execution = ProjectCommandExecution::new(
        command_id.clone(),
        request.git_branch.clone(),
        exit_code,
        stdout,
        stderr,
        duration_ms,
        success,
    );

    let execution_id = db.create_project_command_execution(&execution)?;

    // Return execution with the assigned ID
    let mut execution = execution;
    execution.id = execution_id;

    Ok(HttpResponse::Ok().json(ProjectCommandExecutionResponse { execution }))
}

/// GET /api/projects/{id}/commands/{cmd_id}/executions
/// Get execution history for a command
pub async fn get_command_executions(
    path: web::Path<(i64, String)>,
    query: web::Query<ProjectCommandFilterQuery>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();
    info!(
        "Getting execution history for command {} in project {}",
        command_id, project_id
    );

    // Verify command exists and belongs to project
    let command = db.get_project_command_by_id(&command_id)?;

    if command.project_id != project_id {
        return Err(AppError::NotFound(format!(
            "Command {} not found in project {}",
            command_id, project_id
        )));
    }

    let limit = query.limit.unwrap_or(50);
    let executions = db.get_project_command_executions(&command_id, limit)?;

    Ok(HttpResponse::Ok().json(ProjectCommandExecutionListResponse { executions }))
}

/// POST /api/projects/{id}/commands/discover
/// Discover commands for a project
pub async fn discover_project_commands(
    project_id: web::Path<i64>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, AppError> {
    let project_id = project_id.into_inner();
    info!("Discovering commands for project {}", project_id);

    // Get project
    let project = db
        .get_project_by_id(project_id)
        .map_err(|_| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let project_path = std::path::PathBuf::from(&project.path);

    // Create discovery engine
    let discovery = CommandDiscovery::new(project_path, project_id);

    // Discover commands
    let response = discovery.discover_all().await?;

    // Store discovered commands in database
    let mut stored_commands = Vec::new();
    for suggested in &response.commands {
        let command = suggested.to_project_command(project_id);
        match db.create_project_command(&command) {
            Ok(_) => {
                stored_commands.push(command);
                info!("Stored discovered command: {}", suggested.name);
            }
            Err(e) => {
                error!("Failed to store command {}: {}", suggested.name, e);
            }
        }
    }

    info!(
        "Discovered and stored {} commands for project {}",
        stored_commands.len(),
        project_id
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "commands": stored_commands,
        "project_types": response.project_types,
        "reasoning": response.reasoning,
        "discovered_count": response.commands.len(),
        "stored_count": stored_commands.len(),
    })))
}
