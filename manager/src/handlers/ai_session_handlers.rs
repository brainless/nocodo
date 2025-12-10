use super::main_handlers::AppState;
use crate::error::AppError;
use crate::llm_client::CLAUDE_SONNET_4_5_MODEL_ID;
use crate::models::{
    AiSessionListResponse, AiSessionOutput, AiSessionOutputListResponse,
    AiSessionResponse, CreateAiSessionRequest, LlmAgentToolCallListResponse,
};
use actix_web::{web, HttpMessage, HttpResponse, Result};

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
    } else if model_lower.contains("glm") {
        "zai"
    } else {
        // Default to anthropic if we can't determine
        "anthropic"
    }
}

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
        // Use work's working_directory if available, otherwise fall back to project.path
        let working_path = work.working_directory
            .as_ref()
            .map(|wd| wd.as_str())
            .unwrap_or(&project.path);
        Some(format!("Project: {}\nPath: {}", project.name, working_path))
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
                .find(|m| m.id == message_id_i64)
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
                    .unwrap_or_else(|_| CLAUDE_SONNET_4_5_MODEL_ID.to_string());
                (provider, model)
            };

            // Create LLM agent session with provider/model from environment
            // Note: Not passing project_context as system prompt - system prompt should be None
            // to allow the LLM agent to use its default behavior
            let llm_session = llm_agent
                .create_session(work_id, provider, model, None)
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
    let session = sessions
        .into_iter()
        .max_by_key(|s| s.started_at)
        .unwrap();

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
    let session = sessions
        .into_iter()
        .max_by_key(|s| s.started_at)
        .unwrap();

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