use crate::helpers::agents::create_pdftotext_agent;
use crate::models::ErrorResponse;
use actix_web::{post, web, HttpResponse, Responder};
use nocodo_agents::Agent;
use serde_json::json;
use shared_types::{AgentConfig, AgentExecutionRequest, AgentExecutionResponse};
use std::sync::Arc;
use tracing::{error, info};

#[post("/agents/pdftotext/execute")]
pub async fn execute_pdftotext_agent(
    req: web::Json<AgentExecutionRequest>,
    llm_client: web::Data<Arc<dyn nocodo_llm_sdk::client::LlmClient>>,
    database: web::Data<Arc<nocodo_agents::database::Database>>,
) -> impl Responder {
    let pdf_path = match &req.config {
        AgentConfig::PdfToText(config) => config.pdf_path.clone(),
        _ => {
            error!(config_type = ?req.config, "Invalid config type for PdfToText agent");
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Expected PdfToText agent config".to_string(),
            });
        }
    };

    info!(
        user_prompt = %req.user_prompt,
        pdf_path = %pdf_path,
        "Executing PdfToText agent"
    );

    let user_prompt = req.user_prompt.clone();
    let agent_name = "pdftotext".to_string();

    let config = json!(&req.config);

    let provider = llm_client.provider_name().to_string();
    let model = llm_client.model_name().to_string();

    let session_id = match database.create_session(
        &agent_name,
        &provider,
        &model,
        None,
        &user_prompt,
        Some(config),
    ) {
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
    let database_clone = database.get_ref().clone();
    let pdf_path_clone = pdf_path.clone();
    let user_prompt_clone = user_prompt.clone();

    tokio::spawn(async move {
        let agent =
            match create_pdftotext_agent(&llm_client_clone, &database_clone, &pdf_path_clone).await
            {
                Ok(agent) => agent,
                Err(e) => {
                    error!(error = %e, session_id = session_id, "Failed to create PdfToText agent");
                    let _ = database_clone
                        .fail_session(session_id, &format!("Failed to create agent: {}", e));
                    return;
                }
            };

        match agent.execute(&user_prompt_clone, session_id).await {
            Ok(result) => {
                info!(result = %result, session_id = session_id, "Agent execution completed successfully");
                if let Err(e) = database_clone.complete_session(session_id, &result) {
                    error!(error = %e, session_id = session_id, "Failed to complete session");
                }
            }
            Err(e) => {
                error!(error = %e, session_id = session_id, "Agent execution failed");
                let _ =
                    database_clone.fail_session(session_id, &format!("Execution failed: {}", e));
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
