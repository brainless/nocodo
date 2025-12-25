use super::main_handlers::AppState;
use crate::error::AppError;
use crate::models::{
    AddMessageRequest, AiSessionOutputListResponse, CreateWorkRequest, WorkListResponse,
    WorkResponse,
};

use actix_web::{web, HttpMessage, HttpResponse, Result};
use nocodo_github_actions::{ExecuteCommandRequest, ScanWorkflowsRequest};
use std::time::SystemTime;

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

    // Resolve working_directory from git_branch if provided
    let working_directory =
        if let (Some(git_branch), Some(project_id)) = (&work_req.git_branch, work_req.project_id) {
            // Get project to find project path
            let project = data.database.get_project_by_id(project_id)?;
            let project_path = std::path::Path::new(&project.path);

            // Resolve the working directory for the given branch
            match crate::git::get_working_directory_for_branch(project_path, git_branch) {
                Ok(path) => Some(path),
                Err(e) => {
                    tracing::warn!(
                    "Failed to resolve working directory for branch '{}': {}. Using project path.",
                    git_branch,
                    e
                );
                    Some(project.path.clone())
                }
            }
        } else if let Some(project_id) = work_req.project_id {
            // No git_branch specified, use project path as working_directory
            let project = data.database.get_project_by_id(project_id)?;
            Some(project.path.clone())
        } else {
            None
        };

    let work = crate::models::Work {
        id: 0, // Will be set by database AUTOINCREMENT
        title: work_req.title.clone(),
        project_id: work_req.project_id,
        model: work_req.model.clone(),
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
        git_branch: work_req.git_branch.clone(),
        working_directory,
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
    data.ws_broadcaster
        .broadcast_project_created(crate::models::Project {
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

    let response = crate::models::WorkMessageResponse { message };
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

// TODO: Implement scan_workflows - requires WorkflowService with DB connection
pub async fn scan_workflows(
    _data: web::Data<AppState>,
    request: web::Json<ScanWorkflowsRequest>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let scan_request = request.into_inner();

    // Get user ID from request for authorization
    let _user_id = _req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // For now, return a placeholder response since WorkflowService needs DB connection
    let scan_result = serde_json::json!({
        "project_id": scan_request.project_id,
        "status": "scanned",
        "message": "Workflow scanning not yet implemented"
    });

    Ok(HttpResponse::Ok().json(scan_result))
}

// TODO: Implement get_workflow_commands - requires WorkflowService with DB connection
pub async fn get_workflow_commands(
    _data: web::Data<AppState>,
    request: web::Json<ExecuteCommandRequest>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let _command_request = request.into_inner();

    // Get user ID from request for authorization
    let _user_id = _req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // For now, return empty commands since WorkflowService needs DB connection
    let commands = serde_json::json!({
        "commands": [],
        "message": "Workflow scanning not yet implemented"
    });

    Ok(HttpResponse::Ok().json(commands))
}

// TODO: Implement execute_workflow_command - requires WorkflowService with DB connection
pub async fn execute_workflow_command(
    _data: web::Data<AppState>,
    request: web::Json<ExecuteCommandRequest>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let _command_request = request.into_inner();

    // Get user ID from request for authorization
    let _user_id = _req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // For now, return a placeholder response since WorkflowService needs DB connection
    let execution_result = serde_json::json!({
        "status": "not_implemented",
        "message": "Workflow command execution not yet implemented"
    });

    Ok(HttpResponse::Ok().json(execution_result))
}

// TODO: Implement get_command_executions - requires WorkflowService with DB connection
pub async fn get_command_executions(
    _data: web::Data<AppState>,
    request: web::Json<ExecuteCommandRequest>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let _command_request = request.into_inner();

    // Get user ID from request for authorization
    let _user_id = _req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // For now, return empty executions since WorkflowService needs DB connection
    let executions = serde_json::json!({
        "executions": []
    });

    Ok(HttpResponse::Ok().json(executions))
}

pub async fn list_worktree_branches(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let project_id = path.into_inner();

    // Get project from database
    let project = data
        .database
        .get_project_by_id(project_id)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // List branches using the helper function
    let project_path = std::path::Path::new(&project.path);
    let git_branches = crate::helpers::git_operations::list_project_branches(project_path)?;

    let response = crate::models::GitBranchListResponse {
        branches: git_branches,
    };

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
        let response = AiSessionOutputListResponse { outputs: vec![] };
        return Ok(HttpResponse::Ok().json(response));
    }

    // Get the most recent AI session
    let session = sessions.into_iter().max_by_key(|s| s.started_at).unwrap();

    // Get outputs for this session
    let mut outputs = data.database.list_ai_session_outputs(session.id)?;

    outputs.sort_by_key(|o| o.created_at);
    let response = AiSessionOutputListResponse { outputs };
    Ok(HttpResponse::Ok().json(response))
}
