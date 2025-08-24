mod config;
mod database;
mod error;
mod handlers;
mod models;
mod socket;
mod templates;
mod websocket;

use actix::Actor;
use actix_files as fs;
use actix_web::{middleware::Logger, web, App, HttpServer};
use config::AppConfig;
use database::Database;
use error::AppResult;
use handlers::AppState;
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
    let socket_server = SocketServer::new(&config.socket.path, Arc::clone(&database)).await?;
    let socket_task = tokio::spawn(async move {
        if let Err(e) = socket_server.run().await {
            tracing::error!("Socket server error: {}", e);
        }
    });

    // Create application state with WebSocket broadcaster
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: broadcaster,
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
                        // AI session endpoints
                        .route("/ai/sessions", web::post().to(handlers::create_ai_session))
                        .route("/ai/sessions", web::get().to(handlers::list_ai_sessions))
                        .route("/ai/sessions/{id}", web::get().to(handlers::get_ai_session))
                        .route(
                            "/ai/sessions/{id}/outputs",
                            web::post().to(handlers::record_ai_output),
                        )
                        .route(
                            "/ai/sessions/{id}/outputs",
                            web::get().to(handlers::list_ai_outputs),
                        ),
                )
                // WebSocket endpoint
                .route("/ws", web::get().to(websocket::websocket_handler))
                // Serve static files from ./web/dist if it exists
                .service(
                    fs::Files::new("/", "./web/dist")
                        .index_file("index.html")
                        .use_etag(true)
                        .use_last_modified(true),
                )
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
