use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use shared_types::HeartbeatResponse;

mod agents_api;
mod auth;
mod config;
mod db;
mod projects_api;
mod sheets_api;

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
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    // Run database migrations on startup
    let database_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| config::read_project_conf("DATABASE_URL"))
        .unwrap_or_else(|| "nocodo.db".to_string());

    if let Err(e) = db::run_startup_migrations(&database_url) {
        eprintln!("Warning: Failed to run migrations: {}", e);
    } else {
        println!("Database migrations applied successfully");
    }

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

    // Create one shared agent state for all Actix workers.
    let agent_state = match agents_api::AgentState::new(database_url.clone()) {
        Ok(state) => web::Data::new(state),
        Err(e) => {
            eprintln!("Warning: Failed to initialize agent state: {}", e);
            panic!("Failed to initialize agent state: {}", e);
        }
    };

    // Load schema cache for dynamic SQL queries
    let schema_cache = {
        let conn = rusqlite::Connection::open(&database_url).expect("Failed to open database");
        match sheets_api::schema_cache::SchemaCache::load(&conn) {
            Ok(cache) => {
                println!("Schema cache loaded successfully");
                web::Data::new(cache)
            }
            Err(e) => {
                eprintln!("Warning: Failed to load schema cache: {}", e);
                panic!("Failed to load schema cache: {}", e);
            }
        }
    };

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

        let cors = cors.allowed_methods(vec!["GET", "POST"]).allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(web::JsonConfig::default())
            .app_data(agent_state.clone())
            .app_data(schema_cache.clone())
            .service(heartbeat)
            // Agent API routes
            .service(agents_api::schema_designer::send_chat_message)
            .service(agents_api::schema_designer::get_session_messages)
            .service(agents_api::schema_designer::get_message_response)
            // Project API routes
            .service(projects_api::handlers::list_projects)
            .service(projects_api::handlers::create_project)
            // Sheets API routes (read-only)
            .service(sheets_api::handlers::list_sheets)
            .service(sheets_api::handlers::get_sheet)
            .service(sheets_api::handlers::get_sheet_tab_schema)
            .service(sheets_api::handlers::get_sheet_data)
    })
    .bind((backend_host.as_str(), backend_port))?
    .run()
    .await
}
