mod handlers;
mod helpers;

use actix_web::{App, HttpServer};
use tracing::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let bind_addr = "127.0.0.1:8080";
    info!("Starting nocodo-api server at http://{}", bind_addr);

    HttpServer::new(|| {
        App::new()
            .service(handlers::llm_providers::list_providers)
            .service(handlers::agents::list_agents)
    })
    .bind(bind_addr)?
    .run()
    .await
}
