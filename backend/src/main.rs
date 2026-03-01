use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use shared_types::HeartbeatResponse;

mod auth;
mod db;

#[get("/api/heartbeat")]
async fn heartbeat() -> impl Responder {
    let payload = HeartbeatResponse {
        status: "ok".to_string(),
        service: env!("CARGO_PKG_NAME").to_string(),
    };

    HttpResponse::Ok().json(payload)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    db::run_startup_migrations()?;

    let bind_addr = ("127.0.0.1", 8080);
    println!("Backend listening on http://{}:{}", bind_addr.0, bind_addr.1);

    HttpServer::new(|| {
        let cors = Cors::default()
            .allowed_origin("http://127.0.0.1:3030")
            .allowed_origin("http://localhost:3030")
            .allowed_methods(vec!["GET"])
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(web::JsonConfig::default())
            .service(heartbeat)
    })
    .bind(bind_addr)?
    .run()
    .await
}
