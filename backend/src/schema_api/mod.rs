pub mod handlers;
pub mod schema_cache;
pub mod types;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::list_schemas)
        .service(handlers::get_schema)
        .service(handlers::get_table_columns)
        .service(handlers::get_table_foreign_keys)
        .service(handlers::get_table_data);
}
