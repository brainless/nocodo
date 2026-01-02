use crate::helpers::agents::{list_supported_agents, AgentInfo};
use actix_web::{get, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentsResponse {
    pub agents: Vec<AgentInfo>,
}

#[get("/agents")]
pub async fn list_agents() -> impl Responder {
    let agents = list_supported_agents();
    HttpResponse::Ok().json(AgentsResponse { agents })
}
