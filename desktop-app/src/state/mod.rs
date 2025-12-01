use rusqlite::Connection;
use serde_json::Value;
use std::sync::Arc;

// Type aliases to reduce complexity
pub type UsersResult =
    Arc<std::sync::Mutex<Option<Result<Vec<manager_models::UserListItem>, String>>>>;
pub type TeamsResult =
    Arc<std::sync::Mutex<Option<Result<Vec<manager_models::TeamListItem>, String>>>>;

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
    pub worktree_branches: Vec<String>,
    pub project_detail_worktree_branches: Vec<String>,

    // Command management state
    pub project_detail_saved_commands: Vec<manager_models::ProjectCommand>,

    // Users management
    pub users: Vec<manager_models::UserListItem>,
    pub filtered_users: Vec<manager_models::UserListItem>,
    pub user_search_query: String,
    pub selected_user_ids: std::collections::HashSet<i64>,
    pub loading_users: bool,
    #[serde(skip)]
    pub users_result: UsersResult,

    // User detail modal
    pub show_user_modal: bool,
    pub editing_user: Option<manager_models::User>,
    pub editing_user_teams: Vec<i64>,

    // Teams
    pub teams: Vec<manager_models::Team>,
    pub team_list_items: Vec<manager_models::TeamListItem>,
    pub filtered_teams: Vec<manager_models::TeamListItem>,
    pub team_search_query: String,
    pub selected_team_ids: std::collections::HashSet<i64>,
    pub loading_teams: bool,
    #[serde(skip)]
    pub teams_result: TeamsResult,

    // Team detail modal
    pub show_team_modal: bool,
    pub editing_team: Option<manager_models::Team>,
    pub editing_team_permissions: Vec<i64>, // Permission IDs for team being edited

    // Update results
    #[serde(skip)]
    pub update_user_result: Arc<std::sync::Mutex<Option<Result<(), String>>>>,
    pub updating_user: bool,
    #[serde(skip)]
    pub update_team_result: Arc<std::sync::Mutex<Option<Result<(), String>>>>,
    pub updating_team: bool,

    // Favorite state - stores (server_host, server_user, server_port, project_id) tuples
    pub favorite_projects: std::collections::HashSet<(String, String, u16, i64)>,

    // Current server connection info for favorites
    pub current_server_info: Option<(String, String, u16)>,

    // Projects default path state
    pub projects_default_path_modified: bool,

    // API keys state
    pub xai_api_key_input: String,
    pub openai_api_key_input: String,
    pub anthropic_api_key_input: String,
    pub api_keys_modified: bool,

    // SSH keys state
    pub ssh_public_key_input: String,
    pub adding_ssh_key: bool,
    #[serde(skip)]
    pub add_ssh_key_result: Arc<std::sync::Mutex<Option<Result<String, String>>>>,
    pub ssh_key_message: Option<String>,

    // Current user teams
    pub current_user_teams: Vec<manager_models::TeamItem>,
    pub loading_current_user_teams: bool,
    #[serde(skip)]
    pub current_user_teams_result: Arc<std::sync::Mutex<Option<Result<Vec<manager_models::TeamItem>, String>>>>,

    // UI Reference state
    pub ui_reference_card_titles: Vec<String>,
    pub ui_reference_form_text: String,
    pub ui_reference_form_dropdown: Option<String>,
    pub ui_reference_readme_content: String,

    // Runtime state (not serialized)
    #[serde(skip)]
    pub connection_manager: Arc<crate::connection_manager::ConnectionManager>,
    #[serde(skip)]
    pub project_details_result:
        Arc<std::sync::Mutex<Option<Result<manager_models::ProjectDetailsResponse, String>>>>,
    #[serde(skip)]
    pub create_commands_result:
        Arc<std::sync::Mutex<Option<Result<Vec<manager_models::ProjectCommand>, String>>>>,
            #[serde(skip)]
            pub execute_command_result:
                Arc<std::sync::Mutex<Option<Result<serde_json::Value, String>>>>,
            #[serde(skip)]
            #[allow(clippy::type_complexity)]
            pub command_executions_result:
                Arc<std::sync::Mutex<Option<Result<Vec<manager_models::ProjectCommandExecution>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub projects_result:
        Arc<std::sync::Mutex<Option<Result<Vec<manager_models::Project>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub worktree_branches_result:
        Arc<std::sync::Mutex<Option<Result<Vec<String>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub settings_result:
        Arc<std::sync::Mutex<Option<Result<manager_models::SettingsResponse, String>>>>,
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
    pub loading_work_messages: bool,
    pub loading_ai_session_outputs: bool,
    pub loading_ai_tool_calls: bool,
    pub loading_worktree_branches: bool,
    pub loading_project_detail_worktree_branches: bool,
    pub loading_project_detail_commands: bool,
    pub project_detail_commands_fetch_attempted: bool,
    pub loading_command_discovery: bool,
    pub loading_command_executions: bool,
    pub executing_command_id: Option<String>,
    #[serde(skip)]
    pub loading_settings: bool,
    #[serde(skip)]
    pub loading_project_details: bool,
    #[serde(skip)]
    pub loading_supported_models: bool,
    #[serde(skip)]
    pub models_fetch_attempted: bool,
    pub worktree_branches_fetch_attempted: bool,
    pub project_detail_worktree_branches_fetch_attempted: bool,
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
    pub connection_result: Arc<std::sync::Mutex<Option<Result<(String, String, u16), String>>>>,
    #[serde(skip)]
    pub auth_required: Arc<std::sync::Mutex<bool>>, // Flag set when 401 is detected
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub login_result:
        Arc<std::sync::Mutex<Option<Result<manager_models::LoginResponse, String>>>>,
    #[serde(skip)]
    pub pending_project_details_refresh: Option<i64>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub project_detail_worktree_branches_result:
        Arc<std::sync::Mutex<Option<Result<Vec<String>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub project_detail_saved_commands_result:
        Arc<std::sync::Mutex<Option<Result<Vec<manager_models::ProjectCommand>, String>>>>,
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub command_discovery_result:
        Arc<std::sync::Mutex<Option<Result<manager_models::DiscoverCommandsResponse, String>>>>,
}

impl AppState {
    /// Check if user is authenticated (connected to server AND logged in)
    pub fn is_authenticated(&self) -> bool {
        self.connection_state == ConnectionState::Connected && self.auth_state.jwt_token.is_some()
    }

    /// Load favorites from database for the current server connection
    pub fn load_favorites_for_current_server(&mut self) {
        if let Some((server_host, server_user, server_port)) = &self.current_server_info {
            if let Some(db) = &self.db {
                // Clear existing favorites
                self.favorite_projects.clear();

                tracing::info!(
                    "DB SELECT params: server_host='{}', server_user='{}', server_port={}",
                    server_host, server_user, server_port
                );

                // First, check how many total favorites exist
                if let Ok(total_count) = db.query_row("SELECT COUNT(*) FROM favorites", [], |row| row.get::<_, i64>(0)) {
                    tracing::info!("Total favorites in database: {}", total_count);
                }

                // Check how many match our query
                if let Ok(mut stmt) = db.prepare("SELECT COUNT(*) FROM favorites WHERE server_host = ? AND server_user = ? AND server_port = ?") {
                    if let Ok(matching_count) = stmt.query_row(
                        rusqlite::params![server_host, server_user, server_port],
                        |row| row.get::<_, i64>(0)
                    ) {
                        tracing::info!("Matching favorites for this server: {}", matching_count);
                    }
                }

                // Load favorites for this specific server
                let mut stmt = db
                    .prepare("SELECT entity_type, entity_id, server_host, server_user, server_port FROM favorites WHERE server_host = ? AND server_user = ? AND server_port = ?")
                    .expect("Could not prepare favorites statement");

                let favorites_iter = stmt
                    .query_map(
                        rusqlite::params![server_host, server_user, server_port],
                        |row| {
                            let entity_type: String = row.get(0)?;
                            let entity_id: i64 = row.get(1)?;
                            let db_server_host: String = row.get(2)?;
                            let db_server_user: String = row.get(3)?;
                            let db_server_port: i64 = row.get(4)?;
                            Ok((entity_type, entity_id, db_server_host, db_server_user, db_server_port))
                        },
                    )
                    .expect("Could not query favorites");

                for (entity_type, entity_id, db_server_host, db_server_user, db_server_port) in favorites_iter.flatten() {
                    tracing::info!(
                        "Row from DB: entity_type='{}', entity_id={}, server_host='{}', server_user='{}', server_port={}",
                        entity_type, entity_id, db_server_host, db_server_user, db_server_port
                    );
                    if entity_type == "project" {
                        let favorite_key = (server_host.clone(), server_user.clone(), *server_port, entity_id);
                        tracing::info!("Loaded favorite from DB for current server: project_id={}", entity_id);
                        self.favorite_projects.insert(favorite_key);
                    }
                }

                tracing::info!(
                    "Total favorites loaded for {}@{}:{}: {}",
                    server_user,
                    server_host,
                    server_port,
                    self.favorite_projects.len()
                );
            } else {
                tracing::warn!("Cannot load favorites: database not available");
            }
        } else {
            tracing::debug!("Cannot load favorites: no current_server_info set");
        }
    }
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
            worktree_branches: Vec::new(),
            project_detail_worktree_branches: Vec::new(),

            // Command management state
            project_detail_saved_commands: Vec::new(),
            users: Vec::new(),
            filtered_users: Vec::new(),
            user_search_query: String::new(),
            selected_user_ids: std::collections::HashSet::new(),
            loading_users: false,
            users_result: Arc::new(std::sync::Mutex::new(None)),
            show_user_modal: false,
            editing_user: None,
            editing_user_teams: Vec::new(),
            teams: Vec::new(),
            team_list_items: Vec::new(),
            filtered_teams: Vec::new(),
            team_search_query: String::new(),
            selected_team_ids: std::collections::HashSet::new(),
            loading_teams: false,
            teams_result: Arc::new(std::sync::Mutex::new(None)),
            show_team_modal: false,
            editing_team: None,
            editing_team_permissions: Vec::new(),
            update_user_result: Arc::new(std::sync::Mutex::new(None)),
            updating_user: false,
            update_team_result: Arc::new(std::sync::Mutex::new(None)),
            updating_team: false,
            favorite_projects: std::collections::HashSet::new(),
            current_server_info: None,
            projects_default_path_modified: false,
            xai_api_key_input: String::new(),
            openai_api_key_input: String::new(),
            anthropic_api_key_input: String::new(),
            api_keys_modified: false,
            ssh_public_key_input: String::new(),
            adding_ssh_key: false,
            add_ssh_key_result: Arc::new(std::sync::Mutex::new(None)),
            ssh_key_message: None,
            current_user_teams: Vec::new(),
            loading_current_user_teams: false,
            current_user_teams_result: Arc::new(std::sync::Mutex::new(None)),
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
            worktree_branches_result: Arc::new(std::sync::Mutex::new(None)),
            project_detail_worktree_branches_result: Arc::new(std::sync::Mutex::new(None)),
            project_detail_saved_commands_result: Arc::new(std::sync::Mutex::new(None)),
            command_discovery_result: Arc::new(std::sync::Mutex::new(None)),
            create_commands_result: Arc::new(std::sync::Mutex::new(None)),
            execute_command_result: Arc::new(std::sync::Mutex::new(None)),
            command_executions_result: Arc::new(std::sync::Mutex::new(None)),
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
            loading_worktree_branches: false,
            loading_project_detail_worktree_branches: false,
            loading_project_detail_commands: false,
            project_detail_commands_fetch_attempted: false,
            loading_command_discovery: false,
            loading_command_executions: false,
            executing_command_id: None,
            loading_settings: false,
            loading_project_details: false,
            loading_supported_models: false,
            models_fetch_attempted: false,
            worktree_branches_fetch_attempted: false,
            project_detail_worktree_branches_fetch_attempted: false,
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
            login_result: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}
