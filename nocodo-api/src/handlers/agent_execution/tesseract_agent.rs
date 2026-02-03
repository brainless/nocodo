use crate::helpers::agents::create_tesseract_agent;
use crate::models::ErrorResponse;
use crate::storage::SqliteAgentStorage;
use actix_web::{post, web, HttpResponse, Responder};
use nocodo_agents::{Agent, AgentStorage, Session, SessionStatus};
use serde_json::json;
use shared_types::{AgentConfig, AgentExecutionRequest, AgentExecutionResponse};
use std::sync::Arc;
use tracing::{error, info};

#[post("/agents/tesseract/execute")]
pub async fn execute_tesseract_agent(
    req: web::Json<AgentExecutionRequest>,
    llm_client: web::Data<Arc<dyn nocodo_llm_sdk::client::LlmClient>>,
    storage: web::Data<Arc<SqliteAgentStorage>>,
) -> impl Responder {
    let image_path = match &req.config {
        AgentConfig::Tesseract(config) => config.image_path.clone(),
        _ => {
            error!(config_type = ?req.config, "Invalid config type for Tesseract agent");
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Expected Tesseract agent config".to_string(),
            });
        }
    };

    info!(
        user_prompt = %req.user_prompt,
        image_path = %image_path,
        "Executing Tesseract agent"
    );

    let user_prompt = req.user_prompt.clone();
    let agent_name = "tesseract".to_string();

    let config = json!(&req.config);

    let provider = llm_client.provider_name().to_string();
    let model = llm_client.model_name().to_string();

    let session = Session {
        id: None,
        agent_name: agent_name.clone(),
        provider: provider.clone(),
        model: model.clone(),
        system_prompt: None,
        user_prompt: user_prompt.clone(),
        config: config.clone(),
        status: SessionStatus::Running,
        started_at: chrono::Utc::now().timestamp(),
        ended_at: None,
        result: None,
        error: None,
    };

    let session_id = match storage.create_session(session).await {
        Ok(id) => id,
        Err(e) => {
            error!(error = %e, "Failed to create session");
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to create session: {}", e),
            });
        }
    };

    // Return immediately with session_id and spawn background task
    let llm_client_clone = llm_client.get_ref().clone();
    let storage_clone = storage.get_ref().clone();
    let image_path_clone = image_path.clone();
    let user_prompt_clone = user_prompt.clone();

    tokio::spawn(async move {
        let agent = match create_tesseract_agent(
            &llm_client_clone,
            &storage_clone,
            &image_path_clone,
        )
        .await
        {
            Ok(agent) => agent,
            Err(e) => {
                error!(error = %e, session_id = session_id, "Failed to create Tesseract agent");
                let mut session = Session {
                    id: Some(session_id),
                    agent_name: "tesseract".to_string(),
                    provider,
                    model,
                    system_prompt: None,
                    user_prompt: user_prompt_clone,
                    config,
                    status: SessionStatus::Failed,
                    started_at: 0,
                    ended_at: Some(chrono::Utc::now().timestamp()),
                    result: None,
                    error: Some(format!("Failed to create agent: {}", e)),
                };
                let _ = storage_clone.update_session(session).await;
                return;
            }
        };

        match agent.execute(&user_prompt_clone, session_id).await {
            Ok(result) => {
                info!(result = %result, session_id = session_id, "Agent execution completed successfully");
                let mut session = Session {
                    id: Some(session_id),
                    agent_name: "tesseract".to_string(),
                    provider,
                    model,
                    system_prompt: None,
                    user_prompt: user_prompt_clone,
                    config,
                    status: SessionStatus::Completed,
                    started_at: 0,
                    ended_at: Some(chrono::Utc::now().timestamp()),
                    result: Some(result.clone()),
                    error: None,
                };
                if let Err(e) = storage_clone.update_session(session).await {
                    error!(error = %e, session_id = session_id, "Failed to complete session");
                }
            }
            Err(e) => {
                error!(error = %e, session_id = session_id, "Agent execution failed");
                let mut session = Session {
                    id: Some(session_id),
                    agent_name: "tesseract".to_string(),
                    provider,
                    model,
                    system_prompt: None,
                    user_prompt: user_prompt_clone,
                    config,
                    status: SessionStatus::Failed,
                    started_at: 0,
                    ended_at: Some(chrono::Utc::now().timestamp()),
                    result: None,
                    error: Some(format!("Execution failed: {}", e)),
                };
                let _ = storage_clone.update_session(session).await;
            }
        }
    });

    HttpResponse::Ok().json(AgentExecutionResponse {
        session_id,
        agent_name,
        status: "running".to_string(),
        result: String::new(),
    })
}
