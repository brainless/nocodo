use rusqlite::Connection;
use serde_json::Value;
use std::sync::Arc;

pub mod connection;
pub mod ui_state;

pub use connection::*;
pub use ui_state::*;

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, Default)]
pub enum Page {
    Projects,
    Work,
    ProjectDetail(i64), // Project ID
    Mentions,
    #[default]
    Servers,
    Settings,
    UiReference,
    UiTwoColumnMainContent,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Server {
    pub host: String,
    pub user: String,
    pub key_path: Option<String>,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
}

fn default_ssh_port() -> u16 {
    22
}

/// Centralized application state
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AppState {
    // Connection state
    pub connection_state: ConnectionState,

    // Authentication state
    pub auth_state: AuthState,

    // Configuration
    pub config: crate::config::DesktopConfig,

    // UI state
    pub ui_state: UiState,

    // Data
    pub projects: Vec<manager_models::Project>,
    pub works: Vec<manager_models::Work>,
    pub work_messages: Vec<manager_models::WorkMessage>,
    pub ai_session_outputs: Vec<manager_models::AiSessionOutput>,
    pub ai_tool_calls: Vec<manager_models::LlmAgentToolCall>,
    pub project_details: Option<manager_models::ProjectDetailsResponse>,
    pub servers: Vec<Server>,
    pub settings: Option<manager_models::SettingsResponse>,
    pub supported_models: Vec<manager_models::SupportedModel>,

    // Favorite state
    pub favorite_projects: std::collections::HashSet<i64>,

    // Projects default path state
    pub projects_default_path_modified: bool,

    // API keys state
    pub xai_api_key_input: String,
    pub openai_api_key_input: String,
    pub anthropic_api_key_input: String,
    pub api_keys_modified: bool,

    // UI Reference state
    pub ui_reference_card_titles: Vec<String>,
    pub ui_reference_form_text: String,
    pub ui_reference_form_dropdown: Option<String>,
    pub ui_reference_readme_content: String,

    // Runtime state (not serialized)
    #[serde(skip)]
    pub connection_manager: Arc<crate::connection_manager::ConnectionManager>,
    #[serde(skip)]
    pub pending_project_details_refresh: Option<i64>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub settings_result:
        Arc<std::sync::Mutex<Option<Result<manager_models::SettingsResponse, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub project_details_result:
        Arc<std::sync::Mutex<Option<Result<manager_models::ProjectDetailsResponse, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub projects_result:
        Arc<std::sync::Mutex<Option<Result<Vec<manager_models::Project>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub works_result: Arc<std::sync::Mutex<Option<Result<Vec<manager_models::Work>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub work_messages_result:
        Arc<std::sync::Mutex<Option<Result<Vec<manager_models::WorkMessage>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub ai_session_outputs_result:
        Arc<std::sync::Mutex<Option<Result<Vec<manager_models::AiSessionOutput>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub ai_tool_calls_result:
        Arc<std::sync::Mutex<Option<Result<Vec<manager_models::LlmAgentToolCall>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub update_projects_path_result: Arc<std::sync::Mutex<Option<Result<Value, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub scan_projects_result: Arc<std::sync::Mutex<Option<Result<Value, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub supported_models_result:
        Arc<std::sync::Mutex<Option<Result<Vec<manager_models::SupportedModel>, String>>>>,
    #[serde(skip)]
    pub loading_projects: bool,
    #[serde(skip)]
    pub loading_works: bool,
    #[serde(skip)]
    pub loading_work_messages: bool,
    #[serde(skip)]
    pub loading_ai_session_outputs: bool,
    #[serde(skip)]
    pub loading_ai_tool_calls: bool,
    #[serde(skip)]
    pub loading_settings: bool,
    #[serde(skip)]
    pub loading_project_details: bool,
    #[serde(skip)]
    pub loading_supported_models: bool,
    #[serde(skip)]
    pub models_fetch_attempted: bool,
    #[serde(skip)]
    pub creating_work: bool,
    #[serde(skip)]
    pub updating_projects_path: bool,
    #[serde(skip)]
    pub scanning_projects: bool,
    #[serde(skip)]
    pub updating_api_keys: bool,
    #[serde(skip)]
    pub loading_file_list: bool,
    #[serde(skip)]
    pub loading_file_content: bool,
    #[serde(skip)]
    pub current_file_browser_project_id: Option<i64>,
    #[serde(skip)]
    pub sending_message: bool,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub create_work_result: Arc<std::sync::Mutex<Option<Result<manager_models::Work, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub send_message_result:
        Arc<std::sync::Mutex<Option<Result<manager_models::WorkMessage, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub create_ai_session_result:
        Arc<std::sync::Mutex<Option<Result<manager_models::AiSession, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub update_api_keys_result: Arc<std::sync::Mutex<Option<Result<Value, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub file_list_result:
        Arc<std::sync::Mutex<Option<Result<Vec<manager_models::FileInfo>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub file_content_result:
        Arc<std::sync::Mutex<Option<Result<manager_models::FileContentResponse, String>>>>,
    #[serde(skip)]
    pub db: Option<Connection>,
    #[serde(skip)]
    pub local_server_check_result: Arc<std::sync::Mutex<Option<bool>>>,
    #[serde(skip)]
    pub connection_result: Arc<std::sync::Mutex<Option<Result<String, String>>>>,
    #[serde(skip)]
    pub auth_required: Arc<std::sync::Mutex<bool>>, // Flag set when 401 is detected
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connection_state: ConnectionState::default(),
            auth_state: AuthState::default(),
            config: crate::config::DesktopConfig::default(),
            ui_state: UiState::default(),
            projects: Vec::new(),
            works: Vec::new(),
            work_messages: Vec::new(),
            ai_session_outputs: Vec::new(),
            ai_tool_calls: Vec::new(),
            project_details: None,
            servers: Vec::new(),
            settings: None,
            supported_models: Vec::new(),
            favorite_projects: std::collections::HashSet::new(),
            projects_default_path_modified: false,
            xai_api_key_input: String::new(),
            openai_api_key_input: String::new(),
            anthropic_api_key_input: String::new(),
            api_keys_modified: false,
            ui_reference_card_titles: Vec::new(),
            ui_reference_form_text: String::new(),
            ui_reference_form_dropdown: None,
            ui_reference_readme_content: String::new(),
            connection_manager: Arc::new(crate::connection_manager::ConnectionManager::new()),
            pending_project_details_refresh: None,
            settings_result: Arc::new(std::sync::Mutex::new(None)),
            project_details_result: Arc::new(std::sync::Mutex::new(None)),
            projects_result: Arc::new(std::sync::Mutex::new(None)),
            works_result: Arc::new(std::sync::Mutex::new(None)),
            work_messages_result: Arc::new(std::sync::Mutex::new(None)),
            ai_session_outputs_result: Arc::new(std::sync::Mutex::new(None)),
            ai_tool_calls_result: Arc::new(std::sync::Mutex::new(None)),
            update_projects_path_result: Arc::new(std::sync::Mutex::new(None)),
            scan_projects_result: Arc::new(std::sync::Mutex::new(None)),
            supported_models_result: Arc::new(std::sync::Mutex::new(None)),
            loading_projects: false,
            loading_works: false,
            loading_work_messages: false,
            loading_ai_session_outputs: false,
            loading_ai_tool_calls: false,
            loading_settings: false,
            loading_project_details: false,
            loading_supported_models: false,
            models_fetch_attempted: false,
            creating_work: false,
            updating_projects_path: false,
            scanning_projects: false,
            updating_api_keys: false,
            loading_file_list: false,
            loading_file_content: false,
            current_file_browser_project_id: None,
            sending_message: false,
            create_work_result: Arc::new(std::sync::Mutex::new(None)),
            send_message_result: Arc::new(std::sync::Mutex::new(None)),
            create_ai_session_result: Arc::new(std::sync::Mutex::new(None)),
            update_api_keys_result: Arc::new(std::sync::Mutex::new(None)),
            file_list_result: Arc::new(std::sync::Mutex::new(None)),
            file_content_result: Arc::new(std::sync::Mutex::new(None)),
            db: None,
            local_server_check_result: Arc::new(std::sync::Mutex::new(None)),
            connection_result: Arc::new(std::sync::Mutex::new(None)),
            auth_required: Arc::new(std::sync::Mutex::new(false)),
        }
    }
}
