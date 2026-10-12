pub mod handlers;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::run);
    cfg.service(handlers::run_praxis_auth);
    cfg.service(handlers::run_state_transition);
    cfg.service(handlers::list_praxis_specs);
}
