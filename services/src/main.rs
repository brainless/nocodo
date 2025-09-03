use actix_web::{web, App, HttpServer, Result};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;

use api::health;
use config::Config;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load configuration first to get logging settings
    let config = Config::load().expect("Failed to load configuration");
    
    // Initialize structured logging
    let log_level = config.logging.level.parse().unwrap_or(tracing::Level::INFO);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::filter::LevelFilter::from_level(log_level))
        .init();
    
    info!("Starting nocodo-services on {}:{}", config.server.host, config.server.port);

    HttpServer::new(|| {
        App::new()
            .route("/api/health", web::get().to(health::health_check))
            .route("/api/version", web::get().to(health::version_info))
    })
    .bind(format!("{}:{}", config.server.host, config.server.port))?
    .run()
    .await
}