use crate::command_discovery::{CommandDiscovery, SuggestedCommand};
use crate::database::Database;
use crate::error::AppError;
use crate::handlers::main_handlers::AppState;
use crate::llm_agent::LlmAgent;
use crate::models::{
    CreateProjectCommandRequest, DiscoveryOptionsQuery, ExecuteProjectCommandRequest,
    ProjectCommand, ProjectCommandExecution, ProjectCommandExecutionListResponse,
    ProjectCommandExecutionResponse, ProjectCommandFilterQuery, ProjectCommandListResponse,
    ProjectCommandResponse, UpdateProjectCommandRequest,
};
use actix_web::{web, HttpResponse, Result};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};
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
/// Discover commands for a project using hybrid rule-based + LLM approach
pub async fn discover_project_commands(
    project_id: web::Path<i64>,
    query: web::Query<DiscoveryOptionsQuery>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let project_id = project_id.into_inner();
    let use_llm = query.use_llm.unwrap_or(true);

    info!(
        "Discovering commands for project {} (use_llm: {})",
        project_id, use_llm
    );

    // Get project
    let project = data
        .database
        .get_project_by_id(project_id)
        .map_err(|_| AppError::NotFound(format!("Project {} not found", project_id)))?;

    let project_path = std::path::PathBuf::from(&project.path);

    // Phase 1: Rule-based discovery (always run - it's fast and reliable)
    info!("Running rule-based discovery for project {}", project_id);
    let discovery = CommandDiscovery::new(project_path.clone(), project_id);
    let rule_based_response = discovery.discover_all().await?;

    info!(
        "Rule-based discovery found {} commands for project {}",
        rule_based_response.commands.len(),
        project_id
    );

    // Phase 2: LLM enhancement (conditional)
    let (final_commands, reasoning) = if use_llm && data.llm_agent.is_some() {
        info!("Enhancing discovery with LLM for project {}", project_id);
        match enhance_discovery_with_llm(
            data.llm_agent.as_ref().unwrap().as_ref(),
            &data.database,
            project_id,
            &project_path,
            rule_based_response.commands.clone(),
            &query,
        )
        .await
        {
            Ok((enhanced_commands, llm_reasoning)) => {
                info!(
                    "LLM enhancement completed: {} commands total",
                    enhanced_commands.len()
                );
                (enhanced_commands, Some(llm_reasoning))
            }
            Err(e) => {
                warn!(
                    "LLM enhancement failed for project {}: {}. Falling back to rule-based results.",
                    project_id, e
                );
                (
                    rule_based_response.commands.clone(),
                    Some(format!(
                        "{}. LLM enhancement failed: {}",
                        rule_based_response.reasoning.clone().unwrap_or_default(),
                        e
                    )),
                )
            }
        }
    } else {
        if use_llm && data.llm_agent.is_none() {
            warn!("LLM enhancement requested but LLM agent not available");
        }
        (rule_based_response.commands.clone(), rule_based_response.reasoning.clone())
    };

    // Store discovered commands in database
    let mut stored_commands = Vec::new();
    for suggested in &final_commands {
        let command = suggested.to_project_command(project_id);
        match data.database.create_project_command(&command) {
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
        "project_types": rule_based_response.project_types,
        "reasoning": reasoning,
        "discovered_count": final_commands.len(),
        "stored_count": stored_commands.len(),
        "llm_used": use_llm && data.llm_agent.is_some(),
    })))
}

/// Enhance command discovery using LLM
async fn enhance_discovery_with_llm(
    llm_agent: &LlmAgent,
    _db: &Arc<Database>,
    project_id: i64,
    project_path: &std::path::Path,
    rule_based_commands: Vec<SuggestedCommand>,
    query: &DiscoveryOptionsQuery,
) -> Result<(Vec<SuggestedCommand>, String), AppError> {
    info!("Starting LLM-enhanced discovery for project {}", project_id);

    // Create a temporary work session for discovery
    // (We need a work_id to create an LLM session, so we'll use project_id as a placeholder)
    let work_id = project_id; // Using project_id as work_id for discovery sessions

    // Get LLM provider and model from query or use defaults
    let provider = query
        .llm_provider
        .clone()
        .unwrap_or_else(|| "anthropic".to_string());
    let model = query
        .llm_model
        .clone()
        .unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());

    info!(
        "Creating LLM session for discovery with provider: {}, model: {}",
        provider, model
    );

    // Create discovery prompt
    let system_prompt = create_discovery_system_prompt(project_path, &rule_based_commands);

    // Create LLM session
    let session = llm_agent
        .create_session(work_id, provider.clone(), model.clone(), Some(system_prompt))
        .await
        .map_err(|e| AppError::Internal(format!("Failed to create LLM session: {}", e)))?;

    info!("LLM session created: {}", session.id);

    // Send discovery request
    let user_message = create_discovery_user_message(project_path, &rule_based_commands);

    info!("Sending discovery request to LLM");
    let llm_response = llm_agent
        .process_message(session.id, user_message)
        .await
        .map_err(|e| AppError::Internal(format!("LLM processing failed: {}", e)))?;

    info!("Received LLM response, parsing commands");

    // Parse LLM response to extract enhanced commands
    let (enhanced_commands, reasoning) = parse_llm_discovery_response(&llm_response, &rule_based_commands)?;

    info!(
        "LLM discovery completed: {} commands extracted",
        enhanced_commands.len()
    );

    Ok((enhanced_commands, reasoning))
}

/// Create system prompt for LLM discovery
fn create_discovery_system_prompt(
    project_path: &std::path::Path,
    rule_based_commands: &[SuggestedCommand],
) -> String {
    format!(
        r#"You are an expert software development assistant analyzing a project to discover and validate development commands.

Project Path: {:?}
Rule-based Discovery: Found {} commands through automated detection

Your task:
1. Review the commands discovered through rule-based analysis
2. Use available tools (list_files, read_file, grep) to explore the project
3. Validate the discovered commands and suggest improvements
4. Identify any missing commands that would be useful for developers
5. Provide enhanced command descriptions and proper environment variables
6. Suggest working directories if commands should run in specific subdirectories

Guidelines:
- Prioritize accuracy over completeness
- Include environment variables when relevant (NODE_ENV, DEBUG, DATABASE_URL, etc.)
- Specify working_directory only if the command must run in a subdirectory
- Use descriptive names and clear descriptions
- Consider the project's technology stack and common development workflows

Return your response as a JSON object with this structure:
{{
  "commands": [
    {{
      "name": "command-name",
      "command": "actual command to run",
      "description": "Clear description of what this does",
      "shell": "bash",
      "working_directory": null,
      "environment": {{"KEY": "value"}},
      "timeout_seconds": 120,
      "os_filter": null
    }}
  ],
  "reasoning": "Explanation of your analysis and decisions"
}}

Be thorough but concise. Focus on commands that developers will actually use."#,
        project_path,
        rule_based_commands.len()
    )
}

/// Create user message for LLM discovery request
fn create_discovery_user_message(
    project_path: &std::path::Path,
    rule_based_commands: &[SuggestedCommand],
) -> String {
    let commands_json = serde_json::to_string_pretty(rule_based_commands)
        .unwrap_or_else(|_| "[]".to_string());

    format!(
        r#"Please analyze this project and enhance the discovered commands.

Project path: {:?}

Commands found through rule-based analysis:
```json
{}
```

Tasks:
1. Explore the project structure using list_files to understand the layout
2. Read key configuration files (package.json, Cargo.toml, etc.) to validate the tech stack
3. Review and enhance the discovered commands:
   - Validate each command is correct for this project
   - Add or improve descriptions
   - Add environment variables where appropriate
   - Suggest any missing important commands (e.g., database migrations, linting, etc.)
4. Return the final command list as JSON

Please provide your enhanced command discovery results."#,
        project_path, commands_json
    )
}

/// Parse LLM response to extract enhanced commands
fn parse_llm_discovery_response(
    llm_response: &str,
    fallback_commands: &[SuggestedCommand],
) -> Result<(Vec<SuggestedCommand>, String), AppError> {
    // Try to find JSON in the response
    // LLM might wrap it in markdown code blocks or include explanatory text

    // First, try to extract JSON from code blocks
    let json_str = if let Some(start) = llm_response.find("```json") {
        if let Some(end) = llm_response[start..].find("```") {
            let json_start = start + 7; // Length of "```json"
            &llm_response[json_start..start + end].trim()
        } else {
            llm_response
        }
    } else if let Some(start) = llm_response.find('{') {
        // Try to find JSON object
        &llm_response[start..]
    } else {
        llm_response
    };

    // Try to parse as JSON
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(json) => {
            let commands = json["commands"]
                .as_array()
                .ok_or_else(|| AppError::Internal("LLM response missing 'commands' array".to_string()))?;

            let mut enhanced_commands = Vec::new();
            for cmd_value in commands {
                match serde_json::from_value::<SuggestedCommand>(cmd_value.clone()) {
                    Ok(cmd) => enhanced_commands.push(cmd),
                    Err(e) => {
                        warn!("Failed to parse LLM command: {}. Skipping.", e);
                    }
                }
            }

            let reasoning = json["reasoning"]
                .as_str()
                .unwrap_or("LLM-enhanced discovery completed")
                .to_string();

            // If LLM didn't return any commands, fall back to rule-based
            if enhanced_commands.is_empty() {
                warn!("LLM returned no valid commands, using fallback");
                Ok((fallback_commands.to_vec(), format!("{} (no valid LLM commands, using fallback)", reasoning)))
            } else {
                Ok((enhanced_commands, reasoning))
            }
        }
        Err(e) => {
            warn!("Failed to parse LLM response as JSON: {}. Using fallback commands.", e);
            Ok((
                fallback_commands.to_vec(),
                format!("Rule-based discovery (LLM response parsing failed: {})", e),
            ))
        }
    }
}
