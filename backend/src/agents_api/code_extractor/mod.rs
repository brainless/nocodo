pub mod handlers;
pub mod types;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::extract_struct_handler)
        .service(handlers::extract_free_fn_handler)
        .service(handlers::extract_impl_fn_handler)
        .service(handlers::find_struct_handler)
        .service(handlers::find_free_fn_handler)
        .service(handlers::find_impl_fn_handler)
        .service(handlers::index_build_handler)
        .service(handlers::index_reindex_handler)
        .service(handlers::index_list_structs_handler)
        .service(handlers::index_list_free_fns_handler)
        .service(handlers::index_list_impl_fns_handler)
        .service(handlers::index_get_struct_handler)
        .service(handlers::index_get_free_fn_handler)
        .service(handlers::index_get_impl_fn_handler);
}
