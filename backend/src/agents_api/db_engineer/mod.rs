pub mod handlers;
pub mod types;

pub use handlers::{generate_task_schema_code, get_board, get_task_schema, list_epics, list_tasks};

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_tasks)
        .service(list_epics)
        .service(get_board)
        .service(get_task_schema)
        .service(generate_task_schema_code);
}
