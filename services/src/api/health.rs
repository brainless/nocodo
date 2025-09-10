use actix_web::{HttpResponse, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VersionResponse {
    pub version: String,
    pub service: String,
}

pub async fn health_check() -> Result<HttpResponse> {
    let response = HealthResponse {
        status: "ok".to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn version_info() -> Result<HttpResponse> {
    let response = VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "nocodo-services".to_string(),
    };
    Ok(HttpResponse::Ok().json(response))
}
