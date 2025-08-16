mod config;
mod database;
mod error;
mod handlers;
mod models;

use actix_files as fs;
use actix_web::{middleware::Logger, web, App, HttpServer};
use config::AppConfig;
use database::Database;
use error::AppResult;
use handlers::AppState;
use std::sync::Arc;
use std::time::SystemTime;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

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
    
    // Create application state
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
    });
    
    // Start HTTP server
    let server_addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Starting HTTP server on {}", server_addr);
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(Logger::default())
            .service(
                web::scope("/api")
                    .route("/health", web::get().to(handlers::health_check))
                    .route("/projects", web::get().to(handlers::get_projects))
                    .route("/projects", web::post().to(handlers::create_project))
                    .route("/projects/{id}", web::get().to(handlers::get_project))
                    .route("/projects/{id}", web::delete().to(handlers::delete_project))
            )
            // Serve static files from ./web/dist if it exists
            .service(
                fs::Files::new("/", "./web/dist")
                    .index_file("index.html")
                    .use_etag(true)
                    .use_last_modified(true)
            )
    })
    .bind(&server_addr)?
    .run()
    .await
    .map_err(|e| e.into())
}
