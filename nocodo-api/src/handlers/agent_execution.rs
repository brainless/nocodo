use crate::helpers::agents::create_sqlite_agent;
use crate::models::ErrorResponse;
use actix_web::{post, web, HttpResponse, Responder};
use nocodo_agents::codebase_analysis::CodebaseAnalysisAgent;
use nocodo_agents::Agent;
use serde_json::json;
use shared_types::{AgentConfig, AgentExecutionRequest, AgentExecutionResponse};
use std::sync::Arc;
use tracing::{error, info};

#[post("/agents/sqlite/execute")]
pub async fn execute_sqlite_agent(
    req: web::Json<AgentExecutionRequest>,
    llm_client: web::Data<Arc<dyn nocodo_llm_sdk::client::LlmClient>>,
    database: web::Data<Arc<nocodo_agents::database::Database>>,
) -> impl Responder {
    let db_path = match &req.config {
        AgentConfig::Sqlite(config) => config.db_path.clone(),
        _ => {
            error!(config_type = ?req.config, "Invalid config type for SQLite agent");
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Expected SQLite agent config".to_string(),
            });
        }
    };

    info!(
        user_prompt = %req.user_prompt,
        db_path = %db_path,
        "Executing SQLite agent"
    );

    let user_prompt = req.user_prompt.clone();
    let agent_name = "sqlite".to_string();

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

    let agent = match create_sqlite_agent(&llm_client, &database, &db_path).await {
        Ok(agent) => agent,
        Err(e) => {
            error!(error = %e, "Failed to create SQLite agent");
            let _ = database.fail_session(session_id, &format!("Failed to create agent: {}", e));
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to create agent: {}", e),
            });
        }
    };

    match agent.execute(&user_prompt, session_id).await {
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

#[post("/agents/codebase-analysis/execute")]
pub async fn execute_codebase_analysis_agent(
    req: web::Json<AgentExecutionRequest>,
    llm_client: web::Data<Arc<dyn nocodo_llm_sdk::client::LlmClient>>,
    database: web::Data<Arc<nocodo_agents::database::Database>>,
) -> impl Responder {
    let (path, max_depth) = match &req.config {
        AgentConfig::CodebaseAnalysis(config) => {
            (config.path.clone(), config.max_depth.unwrap_or(3))
        }
        _ => {
            error!(config_type = ?req.config, "Invalid config type for Codebase Analysis agent");
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Expected Codebase Analysis agent config".to_string(),
            });
        }
    };

    info!(
        user_prompt = %req.user_prompt,
        path = %path,
        max_depth = max_depth,
        "Executing Codebase Analysis agent"
    );

    let user_prompt = req.user_prompt.clone();
    let agent_name = "codebase-analysis".to_string();

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

    let tool_executor = Arc::new(
        manager_tools::ToolExecutor::new(std::path::PathBuf::from(path.clone()))
            .with_max_file_size(10 * 1024 * 1024),
    );

    let agent = CodebaseAnalysisAgent::new(
        llm_client.get_ref().clone(),
        database.get_ref().clone(),
        tool_executor,
    );

    match agent.execute(&user_prompt, session_id).await {
        Ok(result) => {
            info!(result = %result, session_id = session_id, "Agent execution completed successfully");

            if let Err(e) = database.complete_session(session_id, &result) {
                error!(error = %e, session_id = session_id, "Failed to complete session");
            }

            HttpResponse::Ok().json(AgentExecutionResponse {
                session_id,
                agent_name,
                status: "completed".to_string(),
                result: result.to_string(),
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
