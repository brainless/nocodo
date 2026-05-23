pub mod handlers;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::list_epic_comments)
        .service(handlers::add_epic_comment)
        .service(handlers::list_task_comments)
        .service(handlers::add_task_comment);
}
