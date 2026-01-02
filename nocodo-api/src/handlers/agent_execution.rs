use crate::helpers::agents::create_sqlite_agent;
use crate::models::{AgentExecutionRequest, AgentExecutionResponse, ErrorResponse};
use crate::DbConnection;
use actix_web::{post, web, HttpResponse, Responder};
use nocodo_agents::Agent;
use std::sync::Arc;
use tracing::{error, info};

#[post("/agents/sqlite/execute")]
pub async fn execute_sqlite_agent(
    req: web::Json<AgentExecutionRequest>,
    llm_client: web::Data<Arc<dyn nocodo_llm_sdk::client::LlmClient>>,
    database: web::Data<Arc<nocodo_agents::database::Database>>,
    db_conn: web::Data<DbConnection>,
) -> impl Responder {
    info!(
        user_prompt = %req.user_prompt,
        db_path = %req.db_path,
        "Executing SQLite agent"
    );

    let db_path = req.db_path.clone();
    let user_prompt = req.user_prompt.clone();

    match create_sqlite_agent(&llm_client, &database, &db_path).await {
        Ok(agent) => match agent.execute(&user_prompt).await {
            Ok(result) => {
                info!(result = %result, "Agent execution completed successfully");

                let session_id = get_latest_session_id(&db_conn, &user_prompt).unwrap_or(0);

                HttpResponse::Ok().json(AgentExecutionResponse {
                    session_id,
                    agent_name: "sqlite".to_string(),
                    status: "completed".to_string(),
                    result,
                })
            }
            Err(e) => {
                error!(error = %e, "Agent execution failed");
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: format!("Agent execution failed: {}", e),
                })
            }
        },
        Err(e) => {
            error!(error = %e, "Failed to create SQLite agent");
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to create agent: {}", e),
            })
        }
    }
}

fn get_latest_session_id(db_conn: &DbConnection, user_prompt: &str) -> Option<i64> {
    let conn = db_conn.lock().ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT id FROM agent_sessions WHERE user_prompt = ?1 ORDER BY started_at DESC LIMIT 1",
        )
        .ok()?;

    let session_id: i64 = stmt.query_row([user_prompt], |row| row.get(0)).ok()?;
    Some(session_id)
}
