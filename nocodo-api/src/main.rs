mod config;
mod handlers;
mod helpers;
mod models;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tracing::info;

pub type DbConnection = Arc<Mutex<Connection>>;

#[actix_web::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let (api_config, config_path) = config::ApiConfig::load().expect("Failed to load config");
    info!("Loaded config from: {}", config_path.display());
    let app_config = Arc::new(std::sync::RwLock::new(api_config));

    let config = app_config
        .read()
        .expect("Failed to acquire config read lock");
    let llm_client = helpers::llm::create_llm_client(&config).expect("Failed to create LLM client");
    drop(config);
    let (db_conn, db) =
        helpers::database::initialize_database().expect("Failed to initialize database");

    let bind_addr = "127.0.0.1:8080";
    info!("Starting nocodo-api server at http://{}", bind_addr);

    let cors_config = app_config
        .read()
        .expect("Failed to acquire config read lock")
        .cors
        .clone()
        .ok_or_else(|| anyhow::anyhow!("CORS configuration is missing from config file"))?;

    let allowed_origins = cors_config
        .allowed_origins
        .into_iter()
        .map(|origin| {
            origin
                .parse::<actix_web::http::header::HeaderValue>()
                .map_err(|e| anyhow::anyhow!("Invalid CORS origin '{}': {}", origin, e))
        })
        .collect::<Result<Vec<_>, _>>()?;

    HttpServer::new(move || {
        let origins = allowed_origins.clone();
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req_head| origins.contains(origin))
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(llm_client.clone()))
            .app_data(web::Data::new(db_conn.clone()))
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(handlers::settings::SettingsAppState {
                config: app_config.clone(),
            }))
            .service(handlers::llm_providers::list_providers)
            .service(handlers::agents::list_agents)
            .service(handlers::agent_execution::execute_sqlite_agent)
            .service(handlers::agent_execution::execute_codebase_analysis_agent)
            .service(handlers::sessions::list_sessions)
            .service(handlers::sessions::get_session)
            .service(
                web::scope("/settings")
                    .route("", web::get().to(handlers::settings::get_settings))
                    .route(
                        "/api-keys",
                        web::post().to(handlers::settings::update_api_keys),
                    ),
            )
    })
    .bind(bind_addr)?
    .run()
    .await
    .map_err(|e| anyhow::anyhow!("Server error: {}", e))
}
