use super::main_handlers::AppState;
use crate::error::AppError;
use crate::models::{
    AiSessionListResponse, AiSessionOutput, AiSessionOutputListResponse,
    LlmAgentToolCallListResponse,
};
use actix_web::{web, HttpResponse, Result};



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