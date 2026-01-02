mod handlers;
mod helpers;
mod models;

use actix_web::{web, App, HttpServer};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tracing::info;

pub type DbConnection = Arc<Mutex<Connection>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let llm_client = helpers::llm::create_llm_client().expect("Failed to create LLM client");
    let (db_conn, db) =
        helpers::database::initialize_database().expect("Failed to initialize database");

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
