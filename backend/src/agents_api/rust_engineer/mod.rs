pub mod handlers;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::run);
    cfg.service(handlers::run_praxis_auth);
}
