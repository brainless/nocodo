use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("LLM agent error: {0}")]
    LlmAgent(#[from] anyhow::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let error_response = ErrorResponse {
            error: self.error_type(),
            message: self.to_string(),
        };

        match self {
            AppError::ProjectNotFound(_) | AppError::NotFound(_) => {
                HttpResponse::NotFound().json(error_response)
            }
            AppError::InvalidRequest(_) => HttpResponse::BadRequest().json(error_response),
            AppError::Unauthorized(_) | AppError::AuthenticationFailed(_) => {
                HttpResponse::Unauthorized().json(error_response)
            }
            AppError::Forbidden(_) => HttpResponse::Forbidden().json(error_response),
            AppError::Database(_)
            | AppError::Config(_)
            | AppError::Io(_)
            | AppError::Internal(_)
            | AppError::LlmAgent(_)
            | AppError::Git(_) => HttpResponse::InternalServerError().json(error_response),
        }
    }
}

impl AppError {
    fn error_type(&self) -> String {
        match self {
            AppError::Database(_) => "database_error".to_string(),
            AppError::Config(_) => "config_error".to_string(),
            AppError::Io(_) => "io_error".to_string(),
            AppError::ProjectNotFound(_) => "project_not_found".to_string(),
            AppError::NotFound(_) => "not_found".to_string(),
            AppError::InvalidRequest(_) => "invalid_request".to_string(),
            AppError::Internal(_) => "internal_error".to_string(),
            AppError::LlmAgent(_) => "llm_agent_error".to_string(),
            AppError::Git(_) => "git_error".to_string(),
            AppError::Unauthorized(_) => "unauthorized".to_string(),
            AppError::AuthenticationFailed(_) => "authentication_failed".to_string(),
            AppError::Forbidden(_) => "forbidden".to_string(),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
