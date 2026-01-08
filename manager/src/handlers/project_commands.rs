use crate::command_discovery::CommandDiscovery;
use crate::error::AppError;
use crate::handlers::main_handlers::AppState;
use crate::models::{
    CreateProjectCommandRequest, DiscoveryOptionsQuery, ExecuteProjectCommandRequest, ProjectCommand, ProjectCommandExecution,
    ProjectCommandExecutionListResponse, ProjectCommandExecutionResponse,
    ProjectCommandFilterQuery, ProjectCommandResponse, UpdateProjectCommandRequest,
};
use actix_web::{web, HttpResponse, Result};
use chrono::Utc;
use std::time::Instant;
use tracing::{error, info};
use uuid::Uuid;

/// GET /api/projects/{id}/commands
/// List all commands for a project
#[allow(dead_code)]
pub async fn get_project_commands(
    project_id: web::Path<i64>,
    query: web::Query<ProjectCommandFilterQuery>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let project_id = project_id.into_inner();
    info!("Getting commands for project {}", project_id);

    // Verify project exists
    data.database
        .get_project_by_id(project_id)
        .map_err(|_| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let mut commands = data.database.get_project_commands(project_id)?;

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

    Ok(HttpResponse::Ok().json(commands))
}

/// GET /api/projects/{id}/commands/{cmd_id}
/// Get a specific command
#[allow(dead_code)]
pub async fn get_project_command(
    path: web::Path<(i64, String)>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();
    info!("Getting command {} for project {}", command_id, project_id);

    let command = data.database.get_project_command_by_id(&command_id)?;

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
/// Create new command(s) - accepts either a single command or array of commands
#[allow(dead_code)]
pub async fn create_project_command(
    project_id: web::Path<i64>,
    request: web::Json<serde_json::Value>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let project_id = project_id.into_inner();

    // Verify project exists
    data.database
        .get_project_by_id(project_id)
        .map_err(|_| AppError::NotFound(format!("Project {} not found", project_id)))?;

    // Handle both single command and array of commands
    let commands_json = if request.is_array() {
        request.clone()
    } else {
        serde_json::json!([request.clone()])
    };

    let requests: Vec<CreateProjectCommandRequest> = serde_json::from_value(commands_json)
        .map_err(|e| {
            error!("Failed to deserialize command request(s): {}", e);
            AppError::InvalidRequest(format!("Invalid command data: {}", e))
        })?;

    if requests.is_empty() {
        return Err(AppError::InvalidRequest(
            "At least one command is required".into(),
        ));
    }

    info!(
        "Creating {} command(s) for project {}",
        requests.len(),
        project_id
    );

    let mut created_commands = Vec::new();
    let now = Utc::now().timestamp();

    for request in requests {
        // Validate request
        if request.name.is_empty() {
            error!("Command creation failed: name is empty");
            return Err(AppError::InvalidRequest("Command name is required".into()));
        }
        if request.command.is_empty() {
            error!("Command creation failed: command is empty");
            return Err(AppError::InvalidRequest("Command is required".into()));
        }

        // Check if command already exists for this project
        if let Some(existing_command) =
            data.database.command_exists(project_id, &request.command)?
        {
            info!(
                "Command '{}' already exists for project {} with id {}, skipping creation",
                request.command, project_id, existing_command.id
            );
            created_commands.push(existing_command);
            continue;
        }

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

        data.database.create_project_command(&command)?;
        info!("Created command {} for project {}", command.id, project_id);
        created_commands.push(command);
    }

    // Return array of created commands
    Ok(HttpResponse::Created().json(created_commands))
}

/// PUT /api/projects/{id}/commands/{cmd_id}
/// Update a command
#[allow(dead_code)]
pub async fn update_project_command(
    path: web::Path<(i64, String)>,
    request: web::Json<UpdateProjectCommandRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();
    info!("Updating command {} for project {}", command_id, project_id);

    let mut command = data.database.get_project_command_by_id(&command_id)?;

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
            return Err(AppError::InvalidRequest(
                "Command name cannot be empty".into(),
            ));
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

    data.database.update_project_command(&command)?;

    info!("Updated command {} for project {}", command_id, project_id);
    Ok(HttpResponse::Ok().json(ProjectCommandResponse { command }))
}

/// DELETE /api/projects/{id}/commands/{cmd_id}
/// Delete a command
#[allow(dead_code)]
pub async fn delete_project_command(
    path: web::Path<(i64, String)>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();
    info!("Deleting command {} for project {}", command_id, project_id);

    let command = data.database.get_project_command_by_id(&command_id)?;

    // Verify command belongs to project
    if command.project_id != project_id {
        return Err(AppError::NotFound(format!(
            "Command {} not found in project {}",
            command_id, project_id
        )));
    }

    data.database.delete_project_command(&command_id)?;

    info!("Deleted command {} for project {}", command_id, project_id);
    Ok(HttpResponse::NoContent().finish())
}

/// POST /api/projects/{id}/commands/{cmd_id}/execute
/// Execute a command
#[allow(dead_code)]
pub async fn execute_project_command(
    path: web::Path<(i64, String)>,
    request: web::Json<ExecuteProjectCommandRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();
    info!(
        "Executing command {} for project {} on branch {:?}",
        command_id, project_id, request.git_branch
    );

    // Get command
    let command = data.database.get_project_command_by_id(&command_id)?;

    // Verify command belongs to project
    if command.project_id != project_id {
        return Err(AppError::NotFound(format!(
            "Command {} not found in project {}",
            command_id, project_id
        )));
    }

    // Get project to determine execution path
    let project = data
        .database
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
            .map_err(|e| AppError::InvalidRequest(format!("Failed to get worktree path: {}", e)))?;
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

    info!(
        "Executing: {} (timeout: {}s)",
        full_command, timeout_seconds
    );

    // Execute using tokio::process::Command
    let mut cmd = tokio::process::Command::new(shell);
    cmd.arg("-c")
        .arg(&command.command)
        .current_dir(&execution_path);

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
            return Err(AppError::Internal(format!(
                "Failed to execute command: {}",
                e
            )));
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

    let execution_id = data.database.create_project_command_execution(&execution)?;

    // Return execution with assigned ID
    let mut execution = execution;
    execution.id = execution_id;

    Ok(HttpResponse::Ok().json(ProjectCommandExecutionResponse { execution }))
}

/// GET /api/projects/{id}/commands/{cmd_id}/executions
/// Get execution history for a command
#[allow(dead_code)]
pub async fn get_command_executions(
    path: web::Path<(i64, String)>,
    query: web::Query<ProjectCommandFilterQuery>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let (project_id, command_id) = path.into_inner();
    info!(
        "Getting execution history for command {} in project {}",
        command_id, project_id
    );

    // Verify command exists and belongs to project
    let command = data.database.get_project_command_by_id(&command_id)?;

    if command.project_id != project_id {
        return Err(AppError::NotFound(format!(
            "Command {} not found in project {}",
            command_id, project_id
        )));
    }

    let limit = query.limit.unwrap_or(50);
    let executions = data
        .database
        .get_project_command_executions(&command_id, limit)?;

    Ok(HttpResponse::Ok().json(ProjectCommandExecutionListResponse { executions }))
}

/// POST /api/projects/{id}/commands/discover
/// Discover commands for a project using hybrid rule-based + LLM approach
#[allow(dead_code)]
pub async fn discover_project_commands(
    project_id: web::Path<i64>,
    _query: web::Query<DiscoveryOptionsQuery>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let project_id = project_id.into_inner();

    info!(
        "Discovering commands for project {}",
        project_id
    );

    // Get project
    let project = data
        .database
        .get_project_by_id(project_id)
        .map_err(|_| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let project_path = std::path::PathBuf::from(&project.path);

    // Rule-based discovery (LLM enhancement has been removed)
    info!("Running rule-based discovery for project {}", project_id);
    let discovery = CommandDiscovery::new(project_path.clone(), project_id);
    let rule_based_response = discovery.discover_all().await?;

    info!(
        "Rule-based discovery found {} commands for project {}",
        rule_based_response.commands.len(),
        project_id
    );

    // LLM enhancement has been removed - using only rule-based discovery
    let (final_commands, reasoning) = (
        rule_based_response.commands.clone(),
        rule_based_response.reasoning.clone(),
    );

    info!(
        "Discovered {} commands for project {}",
        final_commands.len(),
        project_id
    );

    // Convert local SuggestedCommand to shared_types::SuggestedCommand
    let commands_response: Vec<shared_types::SuggestedCommand> = final_commands
        .iter()
        .map(|cmd| shared_types::SuggestedCommand {
            name: cmd.name.clone(),
            description: cmd.description.clone(),
            command: cmd.command.clone(),
            shell: cmd.shell.clone(),
            working_directory: cmd.working_directory.clone(),
            environment: cmd.environment.clone(),
            timeout_seconds: cmd.timeout_seconds,
            os_filter: cmd.os_filter.clone(),
        })
        .collect();

    // Return suggested commands (not stored yet - user will select which ones to save)
    Ok(
        HttpResponse::Ok().json(shared_types::DiscoverCommandsResponse {
            commands: commands_response,
            project_types: rule_based_response.project_types,
            reasoning,
        }),
    )
}
