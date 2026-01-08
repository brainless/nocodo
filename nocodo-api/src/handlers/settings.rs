use crate::config::ApiConfig;
use actix_web::{web, HttpResponse, Result};
use shared_types::{ApiKeyConfig, SettingsResponse, UpdateApiKeysRequest};
use std::sync::Arc;
use tracing::info;

pub struct SettingsAppState {
    pub config: Arc<std::sync::RwLock<ApiConfig>>,
}

fn mask_api_key(key: &Option<String>) -> Option<String> {
    key.as_ref().map(|k| {
        if k.len() <= 6 {
            k.clone()
        } else {
            let masked = format!("{}{}", &k[..6], "*".repeat(k.len() - 6));
            if masked.len() > 40 {
                format!("{}...", &masked[..37])
            } else {
                masked
            }
        }
    })
}

pub async fn get_settings(data: web::Data<SettingsAppState>) -> Result<HttpResponse> {
    let config = data.config.read().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to acquire config read lock: {}",
            e
        ))
    })?;

    let api_keys = if let Some(ref keys) = config.api_keys {
        vec![
            ApiKeyConfig {
                name: "xai".to_string(),
                key: mask_api_key(&keys.xai_api_key),
                is_configured: keys.xai_api_key.is_some(),
            },
            ApiKeyConfig {
                name: "openai".to_string(),
                key: mask_api_key(&keys.openai_api_key),
                is_configured: keys.openai_api_key.is_some(),
            },
            ApiKeyConfig {
                name: "anthropic".to_string(),
                key: mask_api_key(&keys.anthropic_api_key),
                is_configured: keys.anthropic_api_key.is_some(),
            },
            ApiKeyConfig {
                name: "zai".to_string(),
                key: mask_api_key(&keys.zai_api_key),
                is_configured: keys.zai_api_key.is_some(),
            },
        ]
    } else {
        vec![]
    };

    let response = SettingsResponse {
        config_file_path: "api.toml".to_string(),
        api_keys,
        projects_default_path: None,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn update_api_keys(
    data: web::Data<SettingsAppState>,
    request: web::Json<UpdateApiKeysRequest>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let req = request.into_inner();

    let mut config = data.config.write().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to acquire config write lock: {}",
            e
        ))
    })?;

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

    let config_clone = config.clone();

    let toml_string = toml::to_string(&config_clone).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to serialize config: {}", e))
    })?;

    let config_path = if let Some(home) = home::home_dir() {
        home.join(".config/nocodo/api.toml")
    } else {
        std::path::PathBuf::from("api.toml")
    };

    std::fs::write(&config_path, toml_string).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to write config file: {}", e))
    })?;

    info!("Updated API keys in settings");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "path": config_path.to_string_lossy()
    })))
}
