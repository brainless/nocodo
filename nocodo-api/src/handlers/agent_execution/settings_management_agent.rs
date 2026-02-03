use crate::helpers::agents::create_settings_management_agent;
use crate::models::ErrorResponse;
use crate::storage::SqliteAgentStorage;
use actix_web::{post, web, HttpResponse, Responder};
use nocodo_agents::{Agent, AgentStorage, Session, SessionStatus};
use serde_json::json;
use shared_types::{AgentConfig, AgentExecutionRequest, AgentExecutionResponse};
use std::sync::Arc;
use tracing::{error, info};

#[post("/agents/settings-management/execute")]
pub async fn execute_settings_management_agent(
    req: web::Json<AgentExecutionRequest>,
    llm_client: web::Data<Arc<dyn nocodo_llm_sdk::client::LlmClient>>,
    storage: web::Data<Arc<SqliteAgentStorage>>,
) -> impl Responder {
    let (settings_file_path, agent_schemas) = match &req.config {
        AgentConfig::SettingsManagement(config) => {
            let agent_schemas = config
                .agent_schemas
                .iter()
                .map(|s| nocodo_agents::AgentSettingsSchema {
                    agent_name: s.agent_name.clone(),
                    section_name: s.section_name.clone(),
                    settings: s
                        .settings
                        .iter()
                        .map(|setting| nocodo_agents::SettingDefinition {
                            name: setting.name.clone(),
                            label: setting.label.clone(),
                            description: setting.description.clone(),
                            setting_type: match setting.setting_type {
                                shared_types::SettingType::Text => nocodo_agents::SettingType::Text,
                                shared_types::SettingType::Password => {
                                    nocodo_agents::SettingType::Password
                                }
                                shared_types::SettingType::FilePath => {
                                    nocodo_agents::SettingType::FilePath
                                }
                                shared_types::SettingType::Email => {
                                    nocodo_agents::SettingType::Email
                                }
                                shared_types::SettingType::Url => nocodo_agents::SettingType::Url,
                                shared_types::SettingType::Boolean => {
                                    nocodo_agents::SettingType::Boolean
                                }
                            },
                            required: setting.required,
                            default_value: setting.default_value.clone(),
                        })
                        .collect(),
                })
                .collect();
            (config.settings_file_path.clone(), agent_schemas)
        }
        _ => {
            error!(config_type = ?req.config, "Invalid config type for Settings Management agent");
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Expected Settings Management agent config".to_string(),
            });
        }
    };

    info!(
        user_prompt = %req.user_prompt,
        settings_file_path = %settings_file_path,
        "Executing Settings Management agent"
    );

    let user_prompt = req.user_prompt.clone();
    let agent_name = "settings-management".to_string();

    let config = json!(&req.config);

    let provider = llm_client.provider_name().to_string();
    let model = llm_client.model_name().to_string();

    let session = Session {
        id: None,
        agent_name: agent_name.clone(),
        provider: provider.clone(),
        model: model.clone(),
        system_prompt: None,
        user_prompt: user_prompt.clone(),
        config: config.clone(),
        status: SessionStatus::Running,
        started_at: chrono::Utc::now().timestamp(),
        ended_at: None,
        result: None,
        error: None,
    };

    let session_id = match storage.create_session(session).await {
        Ok(id) => id,
        Err(e) => {
            error!(error = %e, "Failed to create session");
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to create session: {}", e),
            });
        }
    };

    let llm_client_clone = llm_client.get_ref().clone();
    let storage_clone = storage.get_ref().clone();
    let user_prompt_clone = user_prompt.clone();
    let settings_file_path_clone = settings_file_path.clone();

    tokio::spawn(async move {
        let agent = match create_settings_management_agent(
            &llm_client_clone,
            &storage_clone,
            &settings_file_path_clone,
            agent_schemas,
        ) {
            Ok(agent) => agent,
            Err(e) => {
                error!(error = %e, session_id = session_id, "Failed to create Settings Management agent");
                let mut session = Session {
                    id: Some(session_id),
                    agent_name: "settings-management".to_string(),
                    provider,
                    model,
                    system_prompt: None,
                    user_prompt: user_prompt_clone,
                    config,
                    status: SessionStatus::Failed,
                    started_at: 0,
                    ended_at: Some(chrono::Utc::now().timestamp()),
                    result: None,
                    error: Some(format!("Failed to create agent: {}", e)),
                };
                let _ = storage_clone.update_session(session).await;
                return;
            }
        };

        match agent.execute(&user_prompt_clone, session_id).await {
            Ok(result) => {
                info!(result = %result, session_id = session_id, "Agent execution completed successfully");
                if !result.contains("Waiting for user") {
                    let mut session = Session {
                        id: Some(session_id),
                        agent_name: "settings-management".to_string(),
                        provider,
                        model,
                        system_prompt: None,
                        user_prompt: user_prompt_clone,
                        config,
                        status: SessionStatus::Completed,
                        started_at: 0,
                        ended_at: Some(chrono::Utc::now().timestamp()),
                        result: Some(result.clone()),
                        error: None,
                    };
                    if let Err(e) = storage_clone.update_session(session).await {
                        error!(error = %e, session_id = session_id, "Failed to complete session");
                    }
                }
            }
            Err(e) => {
                error!(error = %e, session_id = session_id, "Agent execution failed");
                let mut session = Session {
                    id: Some(session_id),
                    agent_name: "settings-management".to_string(),
                    provider,
                    model,
                    system_prompt: None,
                    user_prompt: user_prompt_clone,
                    config,
                    status: SessionStatus::Failed,
                    started_at: 0,
                    ended_at: Some(chrono::Utc::now().timestamp()),
                    result: None,
                    error: Some(format!("Execution failed: {}", e)),
                };
                let _ = storage_clone.update_session(session).await;
            }
        }
    });

    HttpResponse::Ok().json(AgentExecutionResponse {
        session_id,
        agent_name,
        status: "running".to_string(),
        result: String::new(),
    })
}
