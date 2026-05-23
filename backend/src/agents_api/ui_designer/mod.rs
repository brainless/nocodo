pub mod handlers;
pub mod types;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::list_entities)
        .service(handlers::generate_form)
        .service(handlers::get_form)
        .service(handlers::list_forms);
}
