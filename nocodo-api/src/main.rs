mod config;
mod handlers;
mod helpers;
mod models;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use clap::Parser;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    log_file_path: Option<String>,
}

pub type DbConnection = Arc<Mutex<Connection>>;

#[actix_web::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    if let Some(log_path) = args.log_file_path {
        let log_path = std::path::Path::new(&log_path);
        let file_appender = tracing_appender::rolling::never(
            log_path.parent().unwrap_or(std::path::Path::new(".")),
            log_path
                .file_name()
                .unwrap_or(std::ffi::OsStr::new("nocodo-api.log")),
        );
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        std::mem::forget(guard);

        tracing_subscriber::registry()
            .with(env_filter.clone())
            .with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(true)
                    .with_writer(std::io::stdout),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_writer(non_blocking),
            )
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }

    let (api_config, config_path) = config::ApiConfig::load().expect("Failed to load config");
    info!("Loaded config from: {}", config_path.display());
    let app_config = Arc::new(std::sync::RwLock::new(api_config));

    let config = app_config
        .read()
        .expect("Failed to acquire config read lock");
    let llm_client = helpers::llm::create_llm_client(&config).expect("Failed to create LLM client");
    let db_path = config.database.path.clone();
    drop(config);
    let (db_conn, db) =
        helpers::database::initialize_database(&db_path).expect("Failed to initialize database");

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
            .service(handlers::agent_execution::sqlite_agent::execute_sqlite_agent)
            .service(handlers::agent_execution::codebase_analysis_agent::execute_codebase_analysis_agent)
            .service(handlers::agent_execution::tesseract_agent::execute_tesseract_agent)
            .service(handlers::agent_execution::requirements_gathering_agent::execute_requirements_gathering_agent)
            .service(
                handlers::agent_execution::workflow_creation_agent::execute_workflow_creation_agent,
            )
            .service(handlers::sessions::list_sessions)
            .service(handlers::sessions::get_session)
            .service(handlers::sessions::get_pending_questions)
            .service(handlers::sessions::submit_answers)
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
