use crate::config::AppConfig;
use crate::database::Database;
use crate::error::AppError;
use crate::llm_agent::LlmAgent;
use crate::models::{
    ServerStatus, SettingsResponse, SupportedModel, SupportedModelsResponse, UpdateApiKeysRequest,
};
use actix_web::{web, HttpResponse, Result};
use std::sync::Arc;
use std::time::SystemTime;

pub struct AppState {
    pub database: Arc<Database>,
    pub start_time: SystemTime,
    pub ws_broadcaster: Arc<crate::websocket::WebSocketBroadcaster>,
    pub llm_agent: Option<Arc<LlmAgent>>,
    pub config: Arc<std::sync::RwLock<AppConfig>>,
}

pub async fn health_check(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let uptime = data
        .start_time
        .elapsed()
        .map_err(|e| AppError::Internal(format!("Failed to calculate uptime: {e}")))?
        .as_secs();

    let status = ServerStatus {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime,
    };

    Ok(HttpResponse::Ok().json(status))
}

pub async fn get_settings(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let config = data
        .config
        .read()
        .map_err(|e| AppError::Internal(format!("Failed to acquire config read lock: {}", e)))?;

    let api_keys = if let Some(ref keys) = config.api_keys {
        vec![
            crate::models::ApiKeyConfig {
                name: "xai".to_string(),
                key: keys.xai_api_key.clone(),
                is_configured: keys.xai_api_key.is_some(),
            },
            crate::models::ApiKeyConfig {
                name: "openai".to_string(),
                key: keys.openai_api_key.clone(),
                is_configured: keys.openai_api_key.is_some(),
            },
            crate::models::ApiKeyConfig {
                name: "anthropic".to_string(),
                key: keys.anthropic_api_key.clone(),
                is_configured: keys.anthropic_api_key.is_some(),
            },
            crate::models::ApiKeyConfig {
                name: "zai".to_string(),
                key: keys.zai_api_key.clone(),
                is_configured: keys.zai_api_key.is_some(),
            },
        ]
    } else {
        vec![]
    };

    let response = SettingsResponse {
        config_file_path: "manager.toml".to_string(),
        api_keys,
        projects_default_path: config
            .projects
            .as_ref()
            .and_then(|p| p.default_path.clone()),
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn update_api_keys(
    data: web::Data<AppState>,
    request: web::Json<UpdateApiKeysRequest>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    // Update config
    let mut config = data
        .config
        .write()
        .map_err(|e| AppError::Internal(format!("Failed to acquire config write lock: {}", e)))?;

    // Update API keys from request
    if let Some(ref mut keys) = config.api_keys {
        if let Some(xai_key) = req.xai_api_key {
            keys.xai_api_key = Some(xai_key);
        }
        if let Some(openai_key) = req.openai_api_key {
            keys.openai_api_key = Some(openai_key);
        }
        if let Some(anthropic_key) = req.anthropic_api_key {
            keys.anthropic_api_key = Some(anthropic_key);
        }
        if let Some(zai_key) = req.zai_api_key {
            keys.zai_api_key = Some(zai_key);
        }
        if let Some(zai_coding_plan) = req.zai_coding_plan {
            keys.zai_coding_plan = Some(zai_coding_plan);
        }
    } else {
        config.api_keys = Some(crate::config::ApiKeysConfig {
            xai_api_key: req.xai_api_key,
            openai_api_key: req.openai_api_key,
            anthropic_api_key: req.anthropic_api_key,
            zai_api_key: req.zai_api_key,
            zai_coding_plan: req.zai_coding_plan,
            cerebras_api_key: None,
            zen_api_key: None,
        });
    }

    // Clone config for serialization
    let config_clone = config.clone();

    // Save config to file
    let toml_string = toml::to_string(&config_clone)
        .map_err(|e| AppError::Internal(format!("Failed to serialize config: {}", e)))?;

    let config_path = if let Some(home) = home::home_dir() {
        home.join(".config/nocodo/manager.toml")
    } else {
        std::path::PathBuf::from("manager.toml")
    };

    std::fs::write(&config_path, toml_string)
        .map_err(|e| AppError::Internal(format!("Failed to write config file: {}", e)))?;

    tracing::info!("Updated API keys in settings");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "path": config_path.to_string_lossy()
    })))
}

pub async fn add_authorized_ssh_key(
    _data: web::Data<AppState>,
    request: web::Json<serde_json::Value>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    // Get the SSH key from request
    let _ssh_key = req
        .get("ssh_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::InvalidRequest("Invalid ssh_key parameter".to_string()))?;

    // For now, just return success - SSH key management would need to be added to config
    tracing::info!("SSH key addition requested (not implemented in config yet)");

    tracing::info!("Added authorized SSH key");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true
    })))
}

pub async fn get_supported_models(_data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    // Get all model metadata from the LLM SDK
    let sdk_models = nocodo_llm_sdk::model_metadata::get_all_models();

    // Convert to SupportedModel format
    let models: Vec<SupportedModel> = sdk_models
        .into_iter()
        .map(|m| SupportedModel {
            provider: m.provider.to_string(),
            model_id: m.model_id.to_string(),
            name: m.name.to_string(),
            context_length: m.context_length,
            supports_streaming: m.supports_streaming,
            supports_tool_calling: m.supports_tool_calling,
            supports_vision: m.supports_vision,
            supports_reasoning: m.supports_reasoning,
            input_cost_per_token: m.input_cost_per_token,
            output_cost_per_token: m.output_cost_per_token,
            default_temperature: m.default_temperature,
            default_max_tokens: m.default_max_tokens,
        })
        .collect();

    let response = SupportedModelsResponse { models };

    Ok(HttpResponse::Ok().json(response))
}
