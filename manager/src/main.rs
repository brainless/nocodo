mod browser_launcher;
mod config;
mod database;
mod embedded_web;
mod error;
mod handlers;
mod llm_agent;
mod llm_client;
mod models;
mod runner;
mod socket;
mod templates;
mod tools;
mod websocket;

use actix::Actor;
use actix_files as fs;
use actix_web::{middleware::Logger, web, App, HttpServer};
use browser_launcher::{launch_browser, print_startup_banner, wait_for_server, BrowserConfig};
use clap::{Arg, Command};
use config::AppConfig;
use database::Database;
use embedded_web::{configure_embedded_routes, get_embedded_assets_size, validate_embedded_assets};
use error::AppResult;
use handlers::AppState;
use llm_agent::LlmAgent;
use runner::Runner;
use socket::SocketServer;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use websocket::{WebSocketBroadcaster, WebSocketServer};

#[actix_web::main]
async fn main() -> AppResult<()> {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

    // Parse command line arguments
    let matches = Command::new("nocodo-manager")
        .version("0.1.0")
        .about("nocodo Manager - AI-assisted development environment daemon")
        .arg(
            Arg::new("no-browser")
                .long("no-browser")
                .help("Don't automatically open browser")
                .action(clap::ArgAction::SetTrue),
        )
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

    // Configure browser launching
    let browser_config = BrowserConfig {
        auto_launch: !matches.get_flag("no-browser"),
        ..BrowserConfig::default()
    };

    print_startup_banner(&browser_config);
    tracing::info!("Starting nocodo Manager daemon");

    // Load configuration
    let config = AppConfig::load()?;
    tracing::info!("Loaded configuration from ~/.config/nocodo/manager.toml");

    // Validate embedded web assets
    if validate_embedded_assets() {
        let asset_size = get_embedded_assets_size();
        tracing::info!("Embedded web assets loaded: {} bytes", asset_size);
    } else {
        tracing::warn!("No embedded web assets found - serving from filesystem as fallback");
    }

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
    // Optionally enable in-Manager runner via env flag
    let runner_enabled = std::env::var("NOCODO_RUNNER_ENABLED")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    tracing::info!("Runner enabled: {}", runner_enabled);

    let runner = if runner_enabled {
        tracing::info!("Initializing in-process AI runner");
        Some(Arc::new(Runner::new(
            Arc::clone(&database),
            Arc::clone(&broadcaster),
        )))
    } else {
        tracing::warn!("AI runner disabled - set NOCODO_RUNNER_ENABLED=1 to enable");
        None
    };

    // Optionally enable LLM agent via env flag
    let llm_agent_enabled = std::env::var("NOCODO_LLM_AGENT_ENABLED")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    tracing::info!("LLM agent enabled: {}", llm_agent_enabled);

    let llm_agent = if llm_agent_enabled {
        tracing::info!("Initializing LLM agent");
        Some(Arc::new(LlmAgent::new(
            Arc::clone(&database),
            Arc::clone(&broadcaster),
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        )))
    } else {
        tracing::warn!("LLM agent disabled - set NOCODO_LLM_AGENT_ENABLED=1 to enable");
        None
    };

    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: broadcaster,
        runner,
        llm_agent,
    });

    // Start HTTP server
    let server_addr = format!("{}:{}", config.server.host, config.server.port);
    let server_url = format!("http://{}:{}", config.server.host, config.server.port);
    tracing::info!("Starting HTTP server on {}", server_addr);

    // Update browser config with actual server URL
    let browser_config = BrowserConfig {
        url: server_url.clone(),
        ..browser_config
    };

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
                        // Workflow endpoints
                        .route(
                            "/projects/{id}/workflows/scan",
                            web::post().to(handlers::scan_workflows),
                        )
                        .route(
                            "/projects/{id}/workflows/commands",
                            web::get().to(handlers::get_workflow_commands),
                        )
                        .route(
                            "/projects/{project_id}/workflows/commands/{command_id}/execute",
                            web::post().to(handlers::execute_workflow_command),
                        )
                        .route(
                            "/projects/{project_id}/workflows/commands/{command_id}/executions",
                            web::get().to(handlers::get_command_executions),
                        ),
                )
                // WebSocket endpoints
                .route("/ws", web::get().to(websocket::websocket_handler))
                .route(
                    "/ws/work/{id}",
                    web::get().to(websocket::ai_session_websocket_handler),
                )
                // Serve embedded web assets (with filesystem fallback)
                .configure(|cfg| {
                    if validate_embedded_assets() {
                        // Use embedded assets
                        tracing::info!("Configuring embedded web asset routes");
                        configure_embedded_routes(cfg);
                    } else if std::path::Path::new("manager-web/dist").exists() {
                        // Fallback to filesystem (development mode)
                        tracing::info!("Using filesystem fallback for web assets");
                        cfg.service(
                            fs::Files::new("/", "manager-web/dist")
                                .index_file("index.html")
                                .use_etag(true)
                                .use_last_modified(true)
                                .prefer_utf8(true),
                        );
                    } else {
                        tracing::warn!("No web assets available - neither embedded nor filesystem");
                    }
                })
        })
        .bind(&server_addr)
        .expect("Failed to bind HTTP server")
        .run()
        .await
        .expect("HTTP server failed")
    });

    // Launch browser after server starts
    let _browser_task = tokio::spawn({
        let browser_config = browser_config.clone();
        let server_url = server_url.clone();
        async move {
            // Wait for server to be ready
            wait_for_server(&server_url, 10).await;

            // Launch browser
            launch_browser(&browser_config).await;
        }
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
