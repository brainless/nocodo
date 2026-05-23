pub mod handlers;

pub use handlers::{append_message, create_session, get_messages, list_sessions, poll_messages};

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(create_session)
        .service(append_message)
        .service(get_messages)
        .service(poll_messages)
        .service(list_sessions);
}
