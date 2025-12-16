//! Centralized route configuration for the Nocodo manager API.
//! 
//! This module provides a shared function to configure all application routes,
//! allowing both the main server and test servers to use the same routing setup.

use actix_web::web;
use crate::handlers::{main_handlers, project_handlers, work_handlers, user_handlers, team_handlers, file_handlers, project_commands};
use crate::middleware::{AuthenticationMiddleware, PermissionMiddleware, PermissionRequirement};

/// Configures all application routes for the given scope.
/// 
/// # Arguments
/// 
/// * `cfg` - The web service configuration to add routes to
/// * `with_auth` - Whether to include authentication middleware
pub fn configure_routes(cfg: &mut web::ServiceConfig, with_auth: bool) {
    let api_scope = web::scope("/api")
        // Public endpoints (no auth required)
        .route("/health", web::get().to(main_handlers::health_check))
        .route("/auth/login", web::post().to(user_handlers::login))
        .route("/auth/register", web::post().to(user_handlers::register))
        // Protected endpoints with permission checks
        .service(
            web::scope("/projects")
                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                    "project", "read",
                )))
                .route("", web::get().to(project_handlers::get_projects))
                .service(
                    web::resource("")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("project", "write"),
                        ))
                        .route(web::post().to(project_handlers::create_project)),
                )
                .service(
                    web::resource("/add-existing")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("project", "write"),
                        ))
                        .route(web::post().to(project_handlers::add_existing_project)),
                )
                .service(
                    web::resource("/scan")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("project", "write"),
                        ))
                        .route(web::post().to(project_handlers::scan_projects)),
                )
                .service(
                    web::scope("/{id}")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("project", "read")
                                .with_resource_id("id"),
                        ))
                        .route("", web::get().to(project_handlers::get_project))
                        .route(
                            "/details",
                            web::get().to(project_handlers::get_project_details),
                        )
                        .service(
                            web::resource("")
                                .wrap(PermissionMiddleware::new(
                                    PermissionRequirement::new("project", "delete")
                                        .with_resource_id("id"),
                                ))
                                .route(web::delete().to(project_handlers::delete_project)),
                        )
                        // Git endpoints
                        .service(
                            web::scope("/git")
                                .wrap(PermissionMiddleware::new(
                                    PermissionRequirement::new("project", "read")
                                        .with_resource_id("id"),
                                ))
                                .route("/worktree-branches", web::get().to(work_handlers::list_worktree_branches)),
                        )
                        // Project commands endpoints
                        .service(
                            web::scope("/commands")
                                .wrap(PermissionMiddleware::new(
                                    PermissionRequirement::new("project", "read")
                                        .with_resource_id("id"),
                                ))
                                .route("", web::get().to(project_commands::get_project_commands))
                                .service(
                                    web::resource("")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("project", "write")
                                                .with_resource_id("id"),
                                        ))
                                        .route(web::post().to(project_commands::create_project_command)),
                                )
                                .service(
                                    web::resource("/discover")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("project", "write")
                                                .with_resource_id("id"),
                                        ))
                                        .route(web::post().to(project_commands::discover_project_commands)),
                                )
                                .service(
                                    web::scope("/{cmd_id}")
                                        .wrap(PermissionMiddleware::new(
                                            PermissionRequirement::new("project", "read")
                                                .with_resource_id("id"),
                                        ))
                                        .route("", web::get().to(project_commands::get_project_command))
                                        .service(
                                            web::resource("")
                                                .wrap(PermissionMiddleware::new(
                                                    PermissionRequirement::new("project", "write")
                                                        .with_resource_id("id"),
                                                ))
                                                .route(web::put().to(project_commands::update_project_command))
                                                .route(web::delete().to(project_commands::delete_project_command)),
                                        )
                                        .service(
                                            web::resource("/execute")
                                                .wrap(PermissionMiddleware::new(
                                                    PermissionRequirement::new("project", "write")
                                                        .with_resource_id("id"),
                                                ))
                                                .route(web::post().to(project_commands::execute_project_command)),
                                        )
                                        .route(
                                            "/executions",
                                            web::get().to(project_commands::get_command_executions),
                                        ),
                                ),
                        ),
                ),
        )
        .service(
            web::scope("/templates")
                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                    "project", "read",
                )))
                .route("", web::get().to(project_handlers::get_templates)),
        )
        // File operation endpoints
        .service(
            web::scope("/files")
                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                    "project", "read",
                )))
                .route("", web::get().to(file_handlers::list_files))
                .service(
                    web::resource("")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("project", "write"),
                        ))
                        .route(web::post().to(file_handlers::create_file)),
                )
                .service(
                    web::scope("/{path:.*}")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("project", "read"),
                        ))
                        .route("", web::get().to(file_handlers::get_file_content))
                        .service(
                            web::resource("")
                                .wrap(PermissionMiddleware::new(
                                    PermissionRequirement::new("project", "write"),
                                ))
                                .route(web::put().to(file_handlers::update_file))
                                .route(web::delete().to(file_handlers::delete_file)),
                        ),
                ),
        )
        // Work management endpoints
        .service(
            web::scope("/work")
                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                    "work", "read",
                )))
                .route("", web::get().to(work_handlers::list_works))
                .service(
                    web::resource("")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("work", "write"),
                        ))
                        .route(web::post().to(work_handlers::create_work)),
                )
                .service(
                    web::scope("/{id}")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("work", "read")
                                .with_resource_id("id"),
                        ))
                        .route("", web::get().to(work_handlers::get_work))
                        .route(
                            "/messages",
                            web::get().to(work_handlers::get_work_messages),
                        )
                        .route(
                            "/outputs",
                            web::get().to(work_handlers::list_ai_session_outputs),
                        )

                        .service(
                            web::resource("")
                                .wrap(PermissionMiddleware::new(
                                    PermissionRequirement::new("work", "delete")
                                        .with_resource_id("id"),
                                ))
                                .route(web::delete().to(work_handlers::delete_work)),
                        )
                        .service(
                            web::resource("/messages")
                                .wrap(PermissionMiddleware::new(
                                    PermissionRequirement::new("work", "write")
                                        .with_resource_id("id"),
                                ))
                                .route(
                                    web::post().to(work_handlers::add_message_to_work),
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
                .route("/scan", web::post().to(work_handlers::scan_workflows))
                .route("/commands", web::get().to(work_handlers::get_workflow_commands))
                .service(
                    web::scope("/commands/{command_id}")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("project", "write")
                                .with_resource_id("id"),
                        ))
                        .route(
                            "/execute",
                            web::post().to(work_handlers::execute_workflow_command),
                        )
                        .route(
                            "/executions",
                            web::get().to(work_handlers::get_command_executions),
                        ),
                ),
        )
        // Settings endpoints
        .service(
            web::scope("/settings")
                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                    "settings", "read",
                )))
                .route("", web::get().to(main_handlers::get_settings))
                .service(
                    web::resource("/api-keys")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("settings", "write"),
                        ))
                        .route(web::post().to(main_handlers::update_api_keys)),
                )
                .service(
                    web::resource("/projects-path")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("settings", "write"),
                        ))
                        .route(web::post().to(project_handlers::set_projects_default_path)),
                )
                .service(
                    web::resource("/authorized-ssh-keys")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("settings", "write"),
                        ))
                        .route(web::post().to(main_handlers::add_authorized_ssh_key)),
                ),
        )
        // Current user endpoint (get teams for current logged-in user)
        .route("/me/teams", web::get().to(team_handlers::get_current_user_teams))
        .service(
            web::resource("/models")
                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                    "settings", "read",
                )))
                .route(web::get().to(main_handlers::get_supported_models)),
        )
        // User management endpoints (admin only)
        .service(
            web::scope("/users")
                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                    "user", "admin",
                )))
                .route("", web::get().to(user_handlers::list_users))
                .route("", web::post().to(user_handlers::create_user))
                .route("/{id}", web::get().to(user_handlers::get_user))
                .route("/{id}", web::patch().to(user_handlers::update_user))
                .route("/{id}", web::delete().to(user_handlers::delete_user))
                .route("/search", web::get().to(user_handlers::search_users))
                .route("/{id}/teams", web::get().to(user_handlers::get_user_teams)),
        )
        // Team management endpoints
        .service(
            web::scope("/teams")
                .wrap(PermissionMiddleware::new(PermissionRequirement::new(
                    "team", "read",
                )))
                .route("", web::get().to(team_handlers::list_teams))
                .service(
                    web::resource("")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("team", "write"),
                        ))
                        .route(web::post().to(team_handlers::create_team)),
                )
                .service(
                    web::scope("/{id}")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("team", "read")
                                .with_resource_id("id"),
                        ))
                        .route("", web::get().to(team_handlers::get_team))
                        .route(
                            "/members",
                            web::get().to(team_handlers::get_team_members),
                        )
                        .route(
                            "/permissions",
                            web::get().to(team_handlers::get_team_permissions),
                        )
                        .service(
                            web::resource("")
                                .wrap(PermissionMiddleware::new(
                                    PermissionRequirement::new("team", "write")
                                        .with_resource_id("id"),
                                ))
                                .route(web::put().to(team_handlers::update_team))
                                .route(web::delete().to(team_handlers::delete_team)),
                        )
                        .service(
                            web::resource("/members")
                                .wrap(PermissionMiddleware::new(
                                    PermissionRequirement::new("team", "write")
                                        .with_resource_id("id"),
                                ))
                                .route(web::post().to(team_handlers::add_team_member)),
                        )
                        .service(
                            web::scope("/members/{user_id}")
                                .wrap(PermissionMiddleware::new(
                                    PermissionRequirement::new("team", "write")
                                        .with_resource_id("id"),
                                ))
                                .route(
                                    "",
                                    web::delete().to(team_handlers::remove_team_member),
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
                .route("", web::get().to(team_handlers::list_permissions))
                .route("", web::post().to(team_handlers::create_permission))
                .service(
                    web::scope("/{id}")
                        .wrap(PermissionMiddleware::new(
                            PermissionRequirement::new("team", "admin"),
                        ))
                        .route("", web::delete().to(team_handlers::delete_permission)),
                ),
        );
    
    // Add authentication middleware if required
    if with_auth {
        cfg.service(api_scope.wrap(AuthenticationMiddleware));
    } else {
        cfg.service(api_scope);
    }
    
    // WebSocket endpoints (they handle auth internally) - added to the service config directly
    cfg.route("/ws", web::get().to(crate::websocket::websocket_handler))
       .route(
           "/ws/work/{id}",
           web::get().to(crate::websocket::ai_session_websocket_handler),
       );
}