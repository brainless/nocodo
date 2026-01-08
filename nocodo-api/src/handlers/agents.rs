use crate::helpers::agents::list_supported_agents;
use actix_web::{get, HttpResponse, Responder};
use shared_types::AgentsResponse;

#[get("/agents")]
pub async fn list_agents() -> impl Responder {
    let agents = list_supported_agents();
    HttpResponse::Ok().json(AgentsResponse { agents })
}
