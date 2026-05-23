pub mod handlers;
pub mod types;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::gather_context)
        .service(handlers::get_context);
}
