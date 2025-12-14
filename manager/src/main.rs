mod auth;
mod command_discovery;
mod config;
mod database;
mod error;
mod git;
mod handlers;
mod helpers;
mod llm_agent;
mod llm_client;
mod middleware;
mod models;
mod permissions;
mod routes;
mod schema_provider;
mod socket;
mod templates;
mod websocket;

use actix::Actor;
use actix_web::{middleware::Logger, web, App, HttpServer};
use clap::{Arg, Command};
use config::AppConfig;
use database::Database;
use error::AppResult;
use handlers::AppState;
use llm_agent::LlmAgent;

use crate::routes::configure_routes;
use socket::SocketServer;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use websocket::{WebSocketBroadcaster, WebSocketServer};

#[actix_web::main]
async fn main() -> AppResult<()> {
    // Parse command line arguments
    let matches = Command::new("nocodo-manager")
        .version("0.1.0")
        .about("nocodo Manager - AI-assisted development environment daemon")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("Path to configuration file")
                .value_name("FILE"),
        )
        .get_matches();

    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("nocodo_manager=info".parse().unwrap()))
        .init();

    tracing::info!("Starting nocodo Manager daemon");

    // Load configuration
    let config = if let Some(config_path) = matches.get_one::<String>("config") {
        let path = PathBuf::from(config_path);
        tracing::info!("Loading configuration from {}", path.display());
        AppConfig::load_from_file(&path)?
    } else {
        tracing::info!("Loading configuration from default location");
        AppConfig::load()?
    };

    // Initialize database
    let database = Arc::new(Database::new(&config.database.path)?);
    tracing::info!("Database initialized at {:?}", config.database.path);

    // Start WebSocket server
    let ws_server = WebSocketServer::default().start();
    let ws_server_data = web::Data::new(ws_server.clone());
    let broadcaster = Arc::new(WebSocketBroadcaster::new(ws_server));
    tracing::info!("WebSocket server started");

    // Start Unix socket server
    let socket_server = SocketServer::new(
        &config.socket.path,
        Arc::clone(&database),
        Arc::clone(&broadcaster),
    )
    .await?;
    let socket_task = tokio::spawn(async move {
        if let Err(e) = socket_server.run().await {
            tracing::error!("Socket server error: {}", e);
        }
    });

    // Create application state with WebSocket broadcaster

    // Initialize LLM agent (always enabled)
    tracing::info!("Initializing LLM agent");
    let llm_agent = Some(Arc::new(LlmAgent::new(
        Arc::clone(&database),
        Arc::clone(&broadcaster),
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        Arc::new(config.clone()),
    )));

    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: broadcaster,
        llm_agent,
        config: Arc::new(std::sync::RwLock::new(config.clone())),
    });

    // Start HTTP server
    let server_addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Starting HTTP server on {}", server_addr);
    let http_task = tokio::spawn(async move {
        HttpServer::new(move || {
            App::new()
                .app_data(app_state.clone())
                .app_data(ws_server_data.clone())
                .wrap(Logger::default())
                .configure(|cfg| configure_routes(cfg, true))
        })
        .bind(&server_addr)
        .expect("Failed to bind HTTP server")
        .run()
        .await
        .expect("HTTP server failed")
    });

    // Wait for servers (they should run indefinitely)
    tokio::select! {
        _ = socket_task => tracing::info!("Socket server completed"),
        _ = http_task => tracing::info!("HTTP server completed"),
    }

    Ok(())
}
