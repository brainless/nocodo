mod config;
mod database;
mod error;
mod handlers;
mod models;
mod runner;
mod socket;
mod templates;
mod terminal_runner;
mod websocket;

use actix::Actor;
use actix_files as fs;
use actix_web::{middleware::Logger, web, App, HttpServer};
use config::AppConfig;
use database::Database;
use error::AppResult;
use handlers::AppState;
use runner::Runner;
use terminal_runner::TerminalRunner;
use socket::SocketServer;
use std::sync::Arc;
use std::time::SystemTime;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use websocket::{WebSocketBroadcaster, WebSocketServer};

#[actix_web::main]
async fn main() -> AppResult<()> {
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

    // Create terminal runner for PTY-based sessions
    let terminal_runner_enabled = std::env::var("NOCODO_TERMINAL_RUNNER_ENABLED")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true); // Enabled by default

    tracing::info!("Terminal runner enabled: {}", terminal_runner_enabled);

    let terminal_runner = if terminal_runner_enabled {
        tracing::info!("Initializing PTY-based terminal runner");
        Some(Arc::new(TerminalRunner::new(
            Arc::clone(&database),
            Arc::clone(&broadcaster),
        )))
    } else {
        tracing::warn!("Terminal runner disabled - set NOCODO_TERMINAL_RUNNER_ENABLED=1 to enable");
        None
    };

    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: broadcaster,
        runner,
        terminal_runner,
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
                        // Terminal session endpoints for PTY-based interactive sessions
                        .route("/tools", web::get().to(handlers::get_tool_registry))
                        .route("/terminals", web::post().to(handlers::create_terminal_session))
                        .route(
                            "/terminals/{id}",
                            web::get().to(handlers::get_terminal_session),
                        )
                        .route(
                            "/terminals/{id}/input",
                            web::post().to(handlers::send_terminal_input),
                        )
                        .route(
                            "/terminals/{id}/resize",
                            web::post().to(handlers::resize_terminal_session),
                        )
                        .route(
                            "/terminals/{id}/transcript",
                            web::get().to(handlers::get_terminal_transcript),
                        )
                        .route(
                            "/terminals/{id}/terminate",
                            web::post().to(handlers::terminate_terminal_session),
                        ),
                )
                // WebSocket endpoints
                .route("/ws", web::get().to(websocket::websocket_handler))
                .route(
                    "/ws/work/{id}",
                    web::get().to(websocket::ai_session_websocket_handler),
                )
                .route(
                    "/ws/terminals/{id}",
                    web::get().to(websocket::terminal_websocket_handler),
                )
                // Serve static files from ./web/dist if it exists
                .configure(|cfg| {
                    if std::path::Path::new("manager-web/dist").exists() {
                        cfg.service(
                            fs::Files::new("/", "manager-web/dist")
                                .index_file("index.html")
                                .use_etag(true)
                                .use_last_modified(true)
                                .prefer_utf8(true),
                        );
                    }
                })
        })
        .bind(&server_addr)
        .expect("Failed to bind HTTP server")
        .run()
        .await
        .expect("HTTP server failed")
    });

    // Wait for either server to complete (they should run indefinitely)
    tokio::select! {
        _ = socket_task => tracing::info!("Socket server completed"),
        _ = http_task => tracing::info!("HTTP server completed"),
    }

    Ok(())
}
