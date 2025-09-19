mod config;
mod database;
mod error;
mod handlers;
mod llm_agent;
mod llm_client;
mod models;
mod socket;
mod templates;
mod tools;
mod websocket;

use actix::Actor;
use actix_web::{middleware::Logger, web, App, HttpServer};
use clap::{Arg, Command};
use config::AppConfig;
use database::Database;
use error::AppResult;
use handlers::AppState;
use llm_agent::LlmAgent;
use socket::SocketServer;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use websocket::{WebSocketBroadcaster, WebSocketServer};

#[actix_web::main]
async fn main() -> AppResult<()> {
    // Parse command line arguments
    let _matches = Command::new("nocodo-manager")
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
    let config = AppConfig::load()?;
    tracing::info!("Loaded configuration from ~/.config/nocodo/manager.toml");


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

    // LLM agent is now enabled by default
    // Can be explicitly disabled with NOCODO_LLM_AGENT_ENABLED=false
    let llm_agent_disabled = std::env::var("NOCODO_LLM_AGENT_ENABLED")
        .ok()
        .map(|v| v.eq_ignore_ascii_case("false") || v == "0")
        .unwrap_or(false);

    let llm_agent = if !llm_agent_disabled {
        tracing::info!("Initializing LLM agent");
        Some(Arc::new(LlmAgent::new(
            Arc::clone(&database),
            Arc::clone(&broadcaster),
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            Arc::new(config.clone()),
        )))
    } else {
        tracing::warn!("LLM agent explicitly disabled via NOCODO_LLM_AGENT_ENABLED=false");
        None
    };

    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: broadcaster,
        llm_agent,
        config: Arc::new(config.clone()),
    });

    // Start HTTP server
    let server_addr = format!("{}:{}", config.server.host, config.server.port);
    let _server_url = format!("http://{}:{}", config.server.host, config.server.port);
    tracing::info!("Starting HTTP server on {}", server_addr);


    let http_task = tokio::spawn(async move {
        HttpServer::new(move || {
            App::new()
                .app_data(app_state.clone())
                .app_data(ws_server_data.clone())
                .wrap(Logger::default())
                .service(
                    web::scope("/api")
                        .route("/health", web::get().to(handlers::health_check))
                        .route("/projects", web::get().to(handlers::get_projects))
                        .route("/projects", web::post().to(handlers::create_project))
                        .route(
                            "/projects/add-existing",
                            web::post().to(handlers::add_existing_project),
                        )
                        .route("/projects/{id}", web::get().to(handlers::get_project))
                        .route("/projects/{id}", web::delete().to(handlers::delete_project))
                        .route(
                            "/projects/{id}/details",
                            web::get().to(handlers::get_project_details),
                        )
                        .route("/templates", web::get().to(handlers::get_templates))
                        // File operation endpoints
                        .route("/files", web::get().to(handlers::list_files))
                        .route("/files", web::post().to(handlers::create_file))
                        .route(
                            "/files/{path:.*}",
                            web::get().to(handlers::get_file_content),
                        )
                        .route("/files/{path:.*}", web::put().to(handlers::update_file))
                        .route("/files/{path:.*}", web::delete().to(handlers::delete_file))
                        // Work management endpoints
                        .route("/work", web::post().to(handlers::create_work))
                        .route("/work", web::get().to(handlers::list_works))
                        .route("/work/{id}", web::get().to(handlers::get_work))
                        .route("/work/{id}", web::delete().to(handlers::delete_work))
                        // Work message endpoints
                        .route(
                            "/work/{id}/messages",
                            web::post().to(handlers::add_message_to_work),
                        )
                        .route(
                            "/work/{id}/messages",
                            web::get().to(handlers::get_work_messages),
                        )
                        // AI session endpoints
                        .route(
                            "/work/{id}/sessions",
                            web::post().to(handlers::create_ai_session),
                        )
                        .route(
                            "/work/{id}/sessions",
                            web::get().to(handlers::list_ai_sessions),
                        )
                        .route(
                            "/work/{id}/outputs",
                            web::get().to(handlers::list_ai_session_outputs),
                        )
                        // LLM agent endpoints for direct LLM integration
                        .route(
                            "/work/{work_id}/llm-agent",
                            web::post().to(handlers::create_llm_agent_session),
                        )
                        .route(
                            "/work/{work_id}/llm-agent/sessions",
                            web::get().to(handlers::get_llm_agent_sessions),
                        )
                        .route(
                            "/llm-agent/{session_id}",
                            web::get().to(handlers::get_llm_agent_session),
                        )
                        .route(
                            "/llm-agent/{session_id}/message",
                            web::post().to(handlers::send_llm_agent_message),
                        )
                        .route(
                            "/llm-agent/{session_id}/complete",
                            web::post().to(handlers::complete_llm_agent_session),
                        )
                        // Settings endpoint
                        .route("/settings", web::get().to(handlers::get_settings)),
                )
                // WebSocket endpoints
                .route("/ws", web::get().to(websocket::websocket_handler))
                .route(
                    "/ws/work/{id}",
                    web::get().to(websocket::ai_session_websocket_handler),
                )
        })
        .bind(&server_addr)
        .expect("Failed to bind HTTP server")
        .run()
        .await
        .expect("HTTP server failed")
    });


    // Wait for servers (they should run indefinitely)
    // We don't want to exit when browser launcher completes
    // Only exit when HTTP or socket server completes
    tokio::select! {
        _ = socket_task => tracing::info!("Socket server completed"),
        _ = http_task => tracing::info!("HTTP server completed"),
    }

    Ok(())
}
