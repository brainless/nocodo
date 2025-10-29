mod auth;
mod config;
mod database;
mod error;
mod handlers;
mod llm_agent;
mod llm_client;
mod llm_providers;
mod middleware;
mod models;
mod permissions;
mod socket;
mod templates;
mod tools;
mod websocket;

use actix::Actor;
use actix_web::{middleware::Logger, web, App, HttpServer};
use clap::{Arg, Command};
use config::AppConfig;
use database::Database;
use error::AppResult;
use handlers::AppState;
use llm_agent::LlmAgent;
use middleware::{AuthenticationMiddleware, PermissionMiddleware, PermissionRequirement};
use socket::SocketServer;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use websocket::{WebSocketBroadcaster, WebSocketServer};

#[actix_web::main]
async fn main() -> AppResult<()> {
    // Parse command line arguments
    let matches = Command::new("nocodo-manager")
        .version("0.1.0")
        .about("nocodo Manager - AI-assisted development environment daemon")
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

    tracing::info!("Starting nocodo Manager daemon");

    // Load configuration
    let config = if let Some(config_path) = matches.get_one::<String>("config") {
        let path = PathBuf::from(config_path);
        tracing::info!("Loading configuration from {}", path.display());
        AppConfig::load_from_file(&path)?
    } else {
        tracing::info!("Loading configuration from default location");
        AppConfig::load()?
    };

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

    // Initialize LLM agent (always enabled)
    tracing::info!("Initializing LLM agent");
    let llm_agent = Some(Arc::new(LlmAgent::new(
        Arc::clone(&database),
        Arc::clone(&broadcaster),
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        Arc::new(config.clone()),
    )));

    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: broadcaster,
        llm_agent,
        config: Arc::new(std::sync::RwLock::new(config.clone())),
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
                .wrap(AuthenticationMiddleware)
                .service(
                    web::scope("/api")
                        // Public endpoints (no auth required)
                        .route("/health", web::get().to(handlers::health_check))
                        .route("/auth/login", web::post().to(handlers::login))
                        // Protected endpoints with permission checks
                        .service(
                            web::scope("/projects")
                                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                                    "project", "read",
                                )))
                                .route("", web::get().to(handlers::get_projects))
                                .service(
                                    web::resource("")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("project", "write"),
                                        ))
                                        .route(web::post().to(handlers::create_project)),
                                )
                                .service(
                                    web::resource("/add-existing")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("project", "write"),
                                        ))
                                        .route(web::post().to(handlers::add_existing_project)),
                                )
                                .service(
                                    web::scope("/{id}")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("project", "read")
                                                .with_resource_id("id"),
                                        ))
                                        .route("", web::get().to(handlers::get_project))
                                        .route(
                                            "/details",
                                            web::get().to(handlers::get_project_details),
                                        )
                                        .service(
                                            web::resource("")
                                                .wrap(PermissionMiddleware::new(
                                                    PermissionRequirement::new("project", "delete")
                                                        .with_resource_id("id"),
                                                ))
                                                .route(web::delete().to(handlers::delete_project)),
                                        ),
                                ),
                        )
                        .service(
                            web::scope("/templates")
                                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                                    "project", "read",
                                )))
                                .route("", web::get().to(handlers::get_templates)),
                        )
                        // File operation endpoints
                        .service(
                            web::scope("/files")
                                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                                    "project", "read",
                                )))
                                .route("", web::get().to(handlers::list_files))
                                .service(
                                    web::resource("")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("project", "write"),
                                        ))
                                        .route(web::post().to(handlers::create_file)),
                                )
                                .service(
                                    web::scope("/{path:.*}")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("project", "read"),
                                        ))
                                        .route("", web::get().to(handlers::get_file_content))
                                        .service(
                                            web::resource("")
                                                .wrap(PermissionMiddleware::new(
                                                    PermissionRequirement::new("project", "write"),
                                                ))
                                                .route(web::put().to(handlers::update_file))
                                                .route(web::delete().to(handlers::delete_file)),
                                        ),
                                ),
                        )
                        // Work management endpoints
                        .service(
                            web::scope("/work")
                                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                                    "work", "read",
                                )))
                                .route("", web::get().to(handlers::list_works))
                                .service(
                                    web::resource("")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("work", "write"),
                                        ))
                                        .route(web::post().to(handlers::create_work)),
                                )
                                .service(
                                    web::scope("/{id}")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("work", "read")
                                                .with_resource_id("id"),
                                        ))
                                        .route("", web::get().to(handlers::get_work))
                                        .route(
                                            "/messages",
                                            web::get().to(handlers::get_work_messages),
                                        )
                                        .route(
                                            "/sessions",
                                            web::get().to(handlers::list_ai_sessions),
                                        )
                                        .route(
                                            "/outputs",
                                            web::get().to(handlers::list_ai_session_outputs),
                                        )
                                        .route(
                                            "/tool-calls",
                                            web::get().to(handlers::list_ai_tool_calls),
                                        )
                                        .service(
                                            web::resource("")
                                                .wrap(PermissionMiddleware::new(
                                                    PermissionRequirement::new("work", "delete")
                                                        .with_resource_id("id"),
                                                ))
                                                .route(web::delete().to(handlers::delete_work)),
                                        )
                                        .service(
                                            web::resource("/messages")
                                                .wrap(PermissionMiddleware::new(
                                                    PermissionRequirement::new("work", "write")
                                                        .with_resource_id("id"),
                                                ))
                                                .route(
                                                    web::post().to(handlers::add_message_to_work),
                                                ),
                                        ),
                                ),
                        )
                        // Workflow endpoints
                        .service(
                            web::scope("/projects/{id}/workflows")
                                .wrap(PermissionMiddleware::new(
                                    PermissionRequirement::new("project", "read")
                                        .with_resource_id("id"),
                                ))
                                .route("/scan", web::post().to(handlers::scan_workflows))
                                .route("/commands", web::get().to(handlers::get_workflow_commands))
                                .service(
                                    web::scope("/commands/{command_id}")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("project", "write")
                                                .with_resource_id("id"),
                                        ))
                                        .route(
                                            "/execute",
                                            web::post().to(handlers::execute_workflow_command),
                                        )
                                        .route(
                                            "/executions",
                                            web::get().to(handlers::get_command_executions),
                                        ),
                                ),
                        )
                        // Settings endpoints
                        .service(
                            web::scope("/settings")
                                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                                    "settings", "read",
                                )))
                                .route("", web::get().to(handlers::get_settings))
                                .service(
                                    web::resource("/api-keys")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("settings", "write"),
                                        ))
                                        .route(web::post().to(handlers::update_api_keys)),
                                )
                                .service(
                                    web::resource("/projects-path")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("settings", "write"),
                                        ))
                                        .route(web::post().to(handlers::set_projects_default_path)),
                                ),
                        )
                        .service(
                            web::resource("/projects/scan")
                                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                                    "project", "write",
                                )))
                                .route(web::post().to(handlers::scan_projects)),
                        )
                        .service(
                            web::resource("/models")
                                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                                    "settings", "read",
                                )))
                                .route(web::get().to(handlers::get_supported_models)),
                        )
                        // User management endpoints (admin only)
                        .service(
                            web::scope("/users")
                                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                                    "user", "admin",
                                )))
                                .route("", web::get().to(handlers::list_users))
                                .route("", web::post().to(handlers::create_user))
                                .route("/{id}", web::get().to(handlers::get_user))
                                .route("/{id}", web::put().to(handlers::update_user))
                                .route("/{id}", web::delete().to(handlers::delete_user)),
                        )
                        // Team management endpoints
                        .service(
                            web::scope("/teams")
                                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                                    "team", "read",
                                )))
                                .route("", web::get().to(handlers::list_teams))
                                .service(
                                    web::resource("")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("team", "write"),
                                        ))
                                        .route(web::post().to(handlers::create_team)),
                                )
                                .service(
                                    web::scope("/{id}")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("team", "read")
                                                .with_resource_id("id"),
                                        ))
                                        .route("", web::get().to(handlers::get_team))
                                        .route(
                                            "/members",
                                            web::get().to(handlers::get_team_members),
                                        )
                                        .route(
                                            "/permissions",
                                            web::get().to(handlers::get_team_permissions),
                                        )
                                        .service(
                                            web::resource("")
                                                .wrap(PermissionMiddleware::new(
                                                    PermissionRequirement::new("team", "write")
                                                        .with_resource_id("id"),
                                                ))
                                                .route(web::put().to(handlers::update_team))
                                                .route(web::delete().to(handlers::delete_team)),
                                        )
                                        .service(
                                            web::resource("/members")
                                                .wrap(PermissionMiddleware::new(
                                                    PermissionRequirement::new("team", "write")
                                                        .with_resource_id("id"),
                                                ))
                                                .route(web::post().to(handlers::add_team_member)),
                                        )
                                        .service(
                                            web::scope("/members/{user_id}")
                                                .wrap(PermissionMiddleware::new(
                                                    PermissionRequirement::new("team", "write")
                                                        .with_resource_id("id"),
                                                ))
                                                .route(
                                                    "",
                                                    web::delete().to(handlers::remove_team_member),
                                                ),
                                        ),
                                ),
                        )
                        // Permission management endpoints
                        .service(
                            web::scope("/permissions")
                                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                                    "team", "admin",
                                )))
                                .route("", web::get().to(handlers::list_permissions))
                                .route("", web::post().to(handlers::create_permission))
                                .service(
                                    web::scope("/{id}")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("team", "admin"),
                                        ))
                                        .route("", web::delete().to(handlers::delete_permission)),
                                ),
                        ),
                )
                // WebSocket endpoints (they handle auth internally)
                .route("/ws", web::get().to(websocket::websocket_handler))
                .route(
                    "/ws/work/{id}",
                    web::get().to(websocket::ai_session_websocket_handler),
                )
        })
        .bind(&server_addr)
        .expect("Failed to bind HTTP server")
        .run()
        .await
        .expect("HTTP server failed")
    });

    // Wait for servers (they should run indefinitely)
    tokio::select! {
        _ = socket_task => tracing::info!("Socket server completed"),
        _ = http_task => tracing::info!("HTTP server completed"),
    }

    Ok(())
}
