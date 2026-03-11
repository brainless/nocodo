use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use shared_types::HeartbeatResponse;

mod auth;
mod config;

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
    let backend_host = std::env::var("BACKEND_HOST")
        .ok()
        .or_else(|| config::read_project_conf("BACKEND_HOST"))
        .unwrap_or_else(|| "127.0.0.1".to_string());

    let backend_port: u16 = std::env::var("BACKEND_PORT")
        .ok()
        .or_else(|| config::read_project_conf("BACKEND_PORT"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);

    let gui_port: u16 = std::env::var("GUI_PORT")
        .ok()
        .or_else(|| config::read_project_conf("GUI_PORT"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(3030);

    let admin_gui_port: u16 = std::env::var("ADMIN_GUI_PORT")
        .ok()
        .or_else(|| config::read_project_conf("ADMIN_GUI_PORT"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(3031);

    let domain_name = std::env::var("DOMAIN_NAME")
        .ok()
        .or_else(|| config::read_project_conf("DOMAIN_NAME"));

    println!(
        "Backend listening on http://{}:{}",
        backend_host, backend_port
    );

    let gui_origin_ip = format!("http://127.0.0.1:{gui_port}");
    let gui_origin_local = format!("http://localhost:{gui_port}");
    let admin_origin_ip = format!("http://127.0.0.1:{admin_gui_port}");
    let admin_origin_local = format!("http://localhost:{admin_gui_port}");
    let domain_origin_https = domain_name.as_deref().map(|d| format!("https://{d}"));
    let domain_origin_http = domain_name.as_deref().map(|d| format!("http://{d}"));

    HttpServer::new(move || {
        let mut cors = Cors::default()
            .allowed_origin(&gui_origin_ip)
            .allowed_origin(&gui_origin_local)
            .allowed_origin(&admin_origin_ip)
            .allowed_origin(&admin_origin_local);

        if let Some(ref origin) = domain_origin_https {
            cors = cors.allowed_origin(origin);
        }
        if let Some(ref origin) = domain_origin_http {
            cors = cors.allowed_origin(origin);
        }

        let cors = cors
            .allowed_methods(vec!["GET"])
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(web::JsonConfig::default())
            .service(heartbeat)
    })
    .bind((backend_host.as_str(), backend_port))?
    .run()
    .await
}
