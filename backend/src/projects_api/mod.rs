pub mod handlers;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::list_projects)
        .service(handlers::create_project);
}
