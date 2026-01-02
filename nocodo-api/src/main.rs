mod handlers;
mod helpers;
mod models;

use actix_web::{web, App, HttpServer};
use nocodo_llm_sdk::client::LlmClient;
use rusqlite::Connection;
use std::env;
use std::sync::{Arc, Mutex};
use tracing::info;

pub type DbConnection = Arc<Mutex<Connection>>;

fn create_llm_client() -> anyhow::Result<Arc<dyn LlmClient>> {
    let provider = env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let api_key = env::var("ANTHROPIC_API_KEY")
        .or_else(|_| env::var("OPENAI_API_KEY"))
        .or_else(|_| env::var("XAI_API_KEY"))
        .map_err(|_| anyhow::anyhow!("No API key found in environment variables. Set ANTHROPIC_API_KEY, OPENAI_API_KEY, or XAI_API_KEY."))?;

    let client: Arc<dyn LlmClient> = match provider.as_str() {
        "anthropic" => Arc::new(nocodo_llm_sdk::claude::ClaudeClient::new(api_key)?),
        "openai" => Arc::new(nocodo_llm_sdk::openai::OpenAIClient::new(api_key)?),
        "xai" => Arc::new(nocodo_llm_sdk::grok::xai::XaiGrokClient::new(api_key)?),
        _ => Arc::new(nocodo_llm_sdk::claude::ClaudeClient::new(api_key)?),
    };

    Ok(client)
}

fn initialize_database() -> anyhow::Result<(DbConnection, Arc<nocodo_agents::database::Database>)> {
    let db_path = helpers::agents::get_api_db_path()?;

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(&db_path)?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    let db = Arc::new(nocodo_agents::database::Database::new(&db_path)?);

    Ok((Arc::new(Mutex::new(conn)), db))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let llm_client = create_llm_client().expect("Failed to create LLM client");
    let (db_conn, db) = initialize_database().expect("Failed to initialize database");

    let bind_addr = "127.0.0.1:8080";
    info!("Starting nocodo-api server at http://{}", bind_addr);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(llm_client.clone()))
            .app_data(web::Data::new(db_conn.clone()))
            .app_data(web::Data::new(db.clone()))
            .service(handlers::llm_providers::list_providers)
            .service(handlers::agents::list_agents)
            .service(handlers::agent_execution::execute_sqlite_agent)
            .service(handlers::sessions::get_session)
    })
    .bind(bind_addr)?
    .run()
    .await
}
