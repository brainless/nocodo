use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use nocodo_agents::{SqliteTaskStorage, TaskStorage};
use shared_types::HeartbeatResponse;

mod agents_api;
mod auth;
mod config;
mod db;
mod projects_api;
mod repo_api;
mod schema_api;

#[get("/api/heartbeat")]
async fn heartbeat(auth_config: web::Data<auth::AuthConfig>) -> impl Responder {
    let payload = HeartbeatResponse {
        status: "ok".to_string(),
        service: env!("CARGO_PKG_NAME").to_string(),
        auth_required: auth_config.mandatory,
    };

    HttpResponse::Ok().json(payload)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    let config = config::Config::load()
        .unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1) });

    if let Some(path) = config::resolved_config_path() {
        println!("Config file resolved to {}", path.display());
    }

    // Export API keys and agent config to env vars for the agents crate.
    config.export_to_env();

    let default_projects_path = config
        .projects
        .as_ref()
        .and_then(|p| p.default_path.clone())
        .unwrap_or_else(|| "./projects".to_string());
    println!("Default projects path resolved to {}", default_projects_path);

    if let Err(e) = db::run_startup_migrations(&config.database.url) {
        eprintln!("Warning: Failed to run migrations: {}", e);
    } else {
        println!("Database migrations applied successfully");
    }

    let mandatory_auth = config
        .auth
        .as_ref()
        .map(|a| a.mandatory)
        .unwrap_or(true);
    let resend_api_key = config.auth.as_ref().and_then(|a| a.resend_api_key.clone());
    let from_email = config.auth.as_ref().and_then(|a| a.from_email.clone());

    println!(
        "Mandatory authentication: {}",
        if mandatory_auth { "enabled" } else { "disabled" }
    );

    let auth_config = web::Data::new(auth::AuthConfig {
        db_url: config.database.url.clone(),
        mandatory: mandatory_auth,
        resend_api_key,
        from_email,
    });

    println!(
        "Backend listening on http://{}:{}",
        config.server.host, config.server.port
    );

    let gui_origin_ip = format!("http://127.0.0.1:{}", config.gui.port);
    let gui_origin_local = format!("http://localhost:{}", config.gui.port);
    let admin_origin_ip = format!("http://127.0.0.1:{}", config.admin_gui.port);
    let admin_origin_local = format!("http://localhost:{}", config.admin_gui.port);
    let domain_origin_https = config
        .deploy
        .as_ref()
        .map(|d| format!("https://{}", d.domain_name));
    let domain_origin_http = config
        .deploy
        .as_ref()
        .map(|d| format!("http://{}", d.domain_name));

    let agent_state = match agents_api::AgentState::new(config.database.url.clone()) {
        Ok(state) => web::Data::new(state),
        Err(e) => {
            eprintln!("Warning: Failed to initialize agent state: {}", e);
            panic!("Failed to initialize agent state: {}", e);
        }
    };

    if let Ok(ts) = SqliteTaskStorage::open(&config.database.url) {
        match ts.list_open_dispatchable_tasks().await {
            Ok(tasks) => {
                if !tasks.is_empty() {
                    log::info!("Reconciling {} open dispatchable task(s)", tasks.len());
                }
                for task in tasks {
                    let _ = agent_state.dispatch_tx.send(
                        agents_api::dispatcher::DispatchEvent {
                            task_id: task.id.unwrap_or(0),
                            project_id: task.project_id,
                            assigned_to_agent: task.assigned_to_agent,
                            source_prompt: task.source_prompt,
                        },
                    );
                }
            }
            Err(e) => log::warn!("Startup reconciliation failed: {}", e),
        }
    }

    let schema_cache = {
        let conn = rusqlite::Connection::open(&config.database.url)
            .expect("Failed to open database");
        match schema_api::schema_cache::SchemaCache::load(&conn) {
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
            .wrap(auth::middleware::RequireAuth)
            .app_data(web::JsonConfig::default())
            .app_data(agent_state.clone())
            .app_data(schema_cache.clone())
            .app_data(auth_config.clone())
            .service(heartbeat)
            .service(auth::handlers::request_otp)
            .service(auth::handlers::verify_otp)
            .service(auth::handlers::logout)
            .service(auth::handlers::me)
            .service(agents_api::schema_designer::list_tasks)
            .service(agents_api::schema_designer::list_epics)
            .service(agents_api::schema_designer::get_board)
            .service(agents_api::schema_designer::send_chat_message)
            .service(agents_api::schema_designer::get_task_messages)
            .service(agents_api::schema_designer::get_task_schema)
            .service(agents_api::schema_designer::get_message_response)
            .service(agents_api::schema_designer::generate_task_schema_code)
            .service(agents_api::pm_agent::init_pm_project)
            .service(agents_api::pm_agent::send_pm_chat_message)
            .service(agents_api::pm_agent::get_pm_message_response)
            .service(agents_api::pm_agent::get_pm_task_messages)
            .service(agents_api::ui_designer::handlers::list_entities)
            .service(agents_api::ui_designer::handlers::generate_form)
            .service(agents_api::ui_designer::handlers::get_form)
            .service(agents_api::ui_designer::handlers::list_forms)
            .service(projects_api::handlers::list_projects)
            .service(projects_api::handlers::create_project)
            .service(schema_api::handlers::list_schemas)
            .service(schema_api::handlers::get_schema)
            .service(schema_api::handlers::get_table_columns)
            .service(schema_api::handlers::get_table_foreign_keys)
            .service(schema_api::handlers::get_table_data)
    })
    .bind((config.server.host.as_str(), config.server.port))?
    .run()
    .await
}
