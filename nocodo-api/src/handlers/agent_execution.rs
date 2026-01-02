use crate::helpers::agents::create_sqlite_agent;
use crate::models::{AgentExecutionRequest, AgentExecutionResponse, ErrorResponse};
use actix_web::{post, web, HttpResponse, Responder};
use nocodo_agents::Agent;
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info};

#[post("/agents/sqlite/execute")]
pub async fn execute_sqlite_agent(
    req: web::Json<AgentExecutionRequest>,
    llm_client: web::Data<Arc<dyn nocodo_llm_sdk::client::LlmClient>>,
    database: web::Data<Arc<nocodo_agents::database::Database>>,
) -> impl Responder {
    info!(
        user_prompt = %req.user_prompt,
        db_path = %req.db_path,
        "Executing SQLite agent"
    );

    let db_path = req.db_path.clone();
    let user_prompt = req.user_prompt.clone();
    let agent_name = "sqlite".to_string();

    let config = json!({
        "db_path": db_path
    });

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

    let agent = match create_sqlite_agent(&llm_client, &database, &req.db_path).await {
        Ok(agent) => agent,
        Err(e) => {
            error!(error = %e, "Failed to create SQLite agent");
            let _ = database.fail_session(session_id, &format!("Failed to create agent: {}", e));
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to create agent: {}", e),
            });
        }
    };

    match agent.execute(&user_prompt).await {
        Ok(result) => {
            info!(result = %result, session_id = session_id, "Agent execution completed successfully");

            if let Err(e) = database.complete_session(session_id, &result) {
                error!(error = %e, session_id = session_id, "Failed to complete session");
            }

            HttpResponse::Ok().json(AgentExecutionResponse {
                session_id,
                agent_name,
                status: "completed".to_string(),
                result,
            })
        }
        Err(e) => {
            error!(error = %e, session_id = session_id, "Agent execution failed");
            let _ = database.fail_session(session_id, &format!("Execution failed: {}", e));
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Agent execution failed: {}", e),
            })
        }
    }
}
