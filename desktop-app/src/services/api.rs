use crate::state::AppState;
use std::sync::Arc;

use crate::state::ProjectCommandsResult;

pub struct ApiService;

impl ApiService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApiService {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiService {
    // Connection methods
    pub async fn connect_to_server(&self, state: &mut AppState) -> Result<(), String> {
        // This will be implemented with actual API calls
        state.connection_state = crate::state::ConnectionState::Connected;
        state.ui_state.connected_host = Some(state.config.ssh.server.clone());
        Ok(())
    }

    pub async fn connect_to_local_server(&self, state: &mut AppState) -> Result<(), String> {
        let connection_manager = Arc::clone(&state.connection_manager);
        match connection_manager.connect_local(8081).await {
            Ok(_) => {
                state.connection_state = crate::state::ConnectionState::Connected;
                state.ui_state.connected_host = Some("localhost".to_string());
                state.current_server_info =
                    Some(("localhost".to_string(), "local".to_string(), 8081));
                tracing::info!(
                    "Set current_server_info for local connection: {:?}",
                    state.current_server_info
                );
                state.models_fetch_attempted = false;

                // Check if we need to show auth dialog
                let should_show_auth = state.auth_state.jwt_token.is_none();
                if should_show_auth {
                    tracing::info!("Local connection successful, showing auth dialog");
                    state.ui_state.show_auth_dialog = true;
                } else {
                    // Already authenticated, load favorites
                    state.load_favorites_for_current_server();
                }

                Ok(())
            }
            Err(e) => Err(format!("Failed to connect to local manager: {}", e)),
        }
    }

    // Project methods
    pub fn refresh_projects(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_projects = true;
            state.projects_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.projects_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.list_projects().await;
                    let mut projects_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *projects_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut projects_result = result_clone.lock().unwrap();
                    *projects_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    // Git methods
    pub fn refresh_worktree_branches(&self, state: &mut AppState, project_id: i64) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_worktree_branches = true;
            state.worktree_branches_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.worktree_branches_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.get_worktree_branches(project_id).await;
                    let mut branches_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *branches_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut branches_result = result_clone.lock().unwrap();
                    *branches_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_project_detail_worktree_branches(&self, state: &mut AppState, project_id: i64) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_project_detail_worktree_branches = true;
            state.project_detail_worktree_branches_fetch_attempted = true;
            state.project_detail_worktree_branches_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.project_detail_worktree_branches_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.get_worktree_branches(project_id).await;
                    let mut branches_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *branches_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut branches_result = result_clone.lock().unwrap();
                    *branches_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_works(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_works = true;
            state.works_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.works_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.list_works().await;
                    let mut works_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *works_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut works_result = result_clone.lock().unwrap();
                    *works_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_settings(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_settings = true;
            state.settings_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.settings_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.get_settings().await;
                    let mut settings_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *settings_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut settings_result = result_clone.lock().unwrap();
                    *settings_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_supported_models(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_supported_models = true;
            state.models_fetch_attempted = true;
            state.supported_models_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.supported_models_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    // Check if we have a JWT token before making the API call
                    if api_client.get_jwt_token().is_some() {
                        let result = api_client.get_supported_models().await;
                        let mut supported_models_result = result_clone.lock().unwrap();

                        // Check for 401 Unauthorized and set auth required flag
                        if let Err(ref e) = result {
                            if e.is_unauthorized() {
                                if let Ok(mut auth_required) =
                                    connection_manager.get_auth_required_flag().lock()
                                {
                                    *auth_required = true;
                                }
                            }
                        }

                        *supported_models_result = Some(result.map_err(|e| e.to_string()));
                    } else {
                        // No token, set auth required
                        if let Ok(mut auth_required) =
                            connection_manager.get_auth_required_flag().lock()
                        {
                            *auth_required = true;
                        }
                        let mut supported_models_result = result_clone.lock().unwrap();
                        *supported_models_result = Some(Err("Authentication required".to_string()));
                    }
                } else {
                    let mut supported_models_result = result_clone.lock().unwrap();
                    *supported_models_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn create_work(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.creating_work = true;
            state.create_work_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.create_work_result);

            let title = state.ui_state.new_work_title.clone();
            let project_id = state.ui_state.new_work_project_id;
            let model = state.ui_state.new_work_model.clone();
            let git_branch = state.ui_state.new_work_branch.clone();

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let request = shared_types::CreateWorkRequest {
                        title,
                        project_id,
                        model,
                        auto_start: true, // Automatically start LLM agent session
                        tool_name: Some("llm-agent".to_string()),
                        git_branch,
                    };
                    let result = api_client.create_work(request).await;
                    let mut create_work_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *create_work_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut create_work_result = result_clone.lock().unwrap();
                    *create_work_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn send_message_to_work(&self, work_id: i64, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.sending_message = true;
            state.send_message_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.send_message_result);

            let message_content = state.ui_state.continue_message_input.clone();

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let request = shared_types::AddMessageRequest {
                        content: message_content,
                        content_type: shared_types::MessageContentType::Text,
                        author_type: shared_types::MessageAuthorType::User,
                        author_id: None,
                    };
                    let result = api_client.add_message_to_work(work_id, request).await;
                    let mut send_message_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *send_message_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut send_message_result = result_clone.lock().unwrap();
                    *send_message_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_work_messages(&self, work_id: i64, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            // Fetch work messages, AI session outputs, and tool calls
            state.loading_work_messages = true;
            state.loading_ai_session_outputs = true;
            state.loading_ai_tool_calls = true;
            state.work_messages_result = Arc::new(std::sync::Mutex::new(None));
            state.ai_session_outputs_result = Arc::new(std::sync::Mutex::new(None));
            state.ai_tool_calls_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let connection_manager2 = Arc::clone(&state.connection_manager);
            let connection_manager3 = Arc::clone(&state.connection_manager);
            let messages_result_clone = Arc::clone(&state.work_messages_result);
            let outputs_result_clone = Arc::clone(&state.ai_session_outputs_result);
            let tool_calls_result_clone = Arc::clone(&state.ai_tool_calls_result);

            // Fetch work messages (user input)
            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.get_work_messages(work_id).await;
                    let mut work_messages_result = messages_result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *work_messages_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut work_messages_result = messages_result_clone.lock().unwrap();
                    *work_messages_result = Some(Err("Not connected".to_string()));
                }
            });

            // Fetch AI session outputs (AI responses and tool results)
            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager2.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.get_ai_session_outputs(work_id).await;
                    let mut ai_session_outputs_result = outputs_result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager2.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *ai_session_outputs_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut ai_session_outputs_result = outputs_result_clone.lock().unwrap();
                    *ai_session_outputs_result = Some(Err("Not connected".to_string()));
                }
            });

            // Fetch AI tool calls (tool requests and responses)
            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager3.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.get_ai_tool_calls(work_id).await;
                    let mut ai_tool_calls_result = tool_calls_result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager3.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *ai_tool_calls_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut ai_tool_calls_result = tool_calls_result_clone.lock().unwrap();
                    *ai_tool_calls_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_project_details(&self, project_id: i64, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            tracing::info!(
                "refresh_project_details called for project_id={}",
                project_id
            );
            state.loading_project_details = true;
            state.project_details_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.project_details_result);

            tokio::spawn(async move {
                tracing::info!("Fetching project details for project_id={}", project_id);
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.get_project_details(project_id).await;
                    match &result {
                        Ok(details) => tracing::info!(
                            "Project details loaded: {} components",
                            details.components.len()
                        ),
                        Err(e) => {
                            tracing::error!("Failed to load project details: {}", e);
                            // Check for 401 Unauthorized and set auth required flag
                            if e.is_unauthorized() {
                                if let Ok(mut auth_required) =
                                    connection_manager.get_auth_required_flag().lock()
                                {
                                    *auth_required = true;
                                }
                            }
                        }
                    }
                    let mut project_details_result = result_clone.lock().unwrap();
                    *project_details_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    tracing::error!("No API client available for project details");
                    let mut project_details_result = result_clone.lock().unwrap();
                    *project_details_result = Some(Err("Not connected".to_string()));
                }
            });
        } else {
            tracing::warn!("refresh_project_details called but not connected");
        }
    }

    pub fn update_projects_default_path(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.updating_projects_path = true;
            state.update_projects_path_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.update_projects_path_result);
            let path = state.ui_state.projects_default_path.clone();

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.set_projects_default_path(path).await;
                    let mut update_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *update_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut update_result = result_clone.lock().unwrap();
                    *update_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn scan_projects(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.scanning_projects = true;
            state.scan_projects_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.scan_projects_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.scan_projects().await;
                    let mut scan_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *scan_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut scan_result = result_clone.lock().unwrap();
                    *scan_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn update_api_keys(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.updating_api_keys = true;
            state.update_api_keys_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.update_api_keys_result);

            let xai_key = if !state.xai_api_key_input.is_empty() {
                Some(state.xai_api_key_input.clone())
            } else {
                None
            };
            let openai_key = if !state.openai_api_key_input.is_empty() {
                Some(state.openai_api_key_input.clone())
            } else {
                None
            };
            let anthropic_key = if !state.anthropic_api_key_input.is_empty() {
                Some(state.anthropic_api_key_input.clone())
            } else {
                None
            };

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let request = shared_types::UpdateApiKeysRequest {
                        xai_api_key: xai_key,
                        openai_api_key: openai_key,
                        anthropic_api_key: anthropic_key,
                        zai_api_key: None,
                        zai_coding_plan: None,
                    };
                    let result = api_client.update_api_keys(request).await;
                    let mut update_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *update_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut update_result = result_clone.lock().unwrap();
                    *update_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    // Local server methods
    pub fn check_local_server(&self, state: &mut AppState) -> Result<(), String> {
        state.ui_state.checking_local_server = true;
        state.local_server_check_result = Arc::new(std::sync::Mutex::new(None));

        let result_clone = Arc::clone(&state.local_server_check_result);

        tokio::spawn(async move {
            // Try to connect to localhost:8081/api/health
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(3))
                .build()
                .unwrap();
            let url = "http://localhost:8081/api/health";

            let is_running = matches!(client.get(url).send().await, Ok(response) if response.status().is_success());

            let mut result = result_clone.lock().unwrap();
            *result = Some(is_running);
        });

        Ok(())
    }

    // User management methods
    pub fn refresh_users(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_users = true;
            state.users_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.users_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.list_users().await;
                    let mut users_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *users_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut users_result = result_clone.lock().unwrap();
                    *users_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_teams(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_teams = true;
            state.teams_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.teams_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.list_teams().await;
                    let mut teams_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *teams_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut teams_result = result_clone.lock().unwrap();
                    *teams_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn update_user(
        &self,
        state: &mut AppState,
        user_id: i64,
        request: shared_types::UpdateUserRequest,
    ) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.updating_user = true;
            state.update_user_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.update_user_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.update_user(user_id, request).await;
                    let mut update_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *update_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut update_result = result_clone.lock().unwrap();
                    *update_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_user_teams(&self, state: &mut AppState, user_id: i64) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            let connection_manager = Arc::clone(&state.connection_manager);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    if let Ok(_user_teams) = api_client.get_user_teams(user_id).await {
                        // Update editing_user_teams in the main thread
                        // Note: This would need to be handled differently in a real async context
                        // For now, we'll handle this in the UI layer
                    }
                }
            });
        }
    }

    pub fn apply_user_search_filter(&self, state: &mut AppState) {
        let query = state.user_search_query.to_lowercase();
        if query.is_empty() {
            state.filtered_users = state.users.clone();
        } else {
            state.filtered_users = state
                .users
                .iter()
                .filter(|u| {
                    u.name.to_lowercase().contains(&query)
                        || u.email.to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }
    }

    pub fn refresh_team_list(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_teams = true;
            state.teams_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.teams_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.list_teams().await;
                    let mut teams_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *teams_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut teams_result = result_clone.lock().unwrap();
                    *teams_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn update_team(
        &self,
        state: &mut AppState,
        team_id: i64,
        request: shared_types::UpdateTeamRequest,
    ) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.updating_team = true;
            state.update_team_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.update_team_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.update_team(team_id, request).await;
                    let mut update_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *update_result = Some(result.map(|_| ()).map_err(|e| e.to_string()));
                } else {
                    let mut update_result = result_clone.lock().unwrap();
                    *update_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn refresh_team_permissions(&self, state: &mut AppState, team_id: i64) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            let connection_manager = Arc::clone(&state.connection_manager);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    if let Ok(_permissions) = api_client.get_team_permissions(team_id).await {
                        // Update state - this needs to be handled in UI thread
                        // For now, permissions will be loaded when modal opens
                    }
                }
            });
        }
    }

    pub fn apply_team_search_filter(&self, state: &mut AppState) {
        let query = state.team_search_query.to_lowercase();
        if query.is_empty() {
            state.filtered_teams = state.team_list_items.clone();
        } else {
            state.filtered_teams = state
                .team_list_items
                .iter()
                .filter(|t| {
                    t.name.to_lowercase().contains(&query)
                        || t.description
                            .as_ref()
                            .is_some_and(|d| d.to_lowercase().contains(&query))
                })
                .cloned()
                .collect();
        }
    }

    pub fn refresh_current_user_teams(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_current_user_teams = true;
            state.current_user_teams_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.current_user_teams_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.get_current_user_teams().await;
                    let mut teams_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *teams_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut teams_result = result_clone.lock().unwrap();
                    *teams_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn add_authorized_ssh_key(&self, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.adding_ssh_key = true;
            state.add_ssh_key_result = Arc::new(std::sync::Mutex::new(None));
            state.ssh_key_message = None;

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.add_ssh_key_result);
            let public_key = state.ssh_public_key_input.clone();

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.add_authorized_ssh_key(public_key).await;
                    let mut add_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *add_result = Some(result.map(|r| r.message).map_err(|e| e.to_string()));
                } else {
                    let mut add_result = result_clone.lock().unwrap();
                    *add_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    // Command management methods
    pub fn discover_project_commands(
        &self,
        project_id: i64,
        use_llm: Option<bool>,
        state: &mut AppState,
    ) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_command_discovery = true;
            state.command_discovery_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone: Arc<
                std::sync::Mutex<Option<Result<shared_types::DiscoverCommandsResponse, String>>>,
            > = Arc::clone(&state.command_discovery_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client
                        .discover_project_commands(project_id, use_llm)
                        .await;
                    let mut discovery_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *discovery_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut discovery_result = result_clone.lock().unwrap();
                    *discovery_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn list_project_commands(&self, project_id: i64, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_project_detail_commands = true;
            state.project_detail_commands_fetch_attempted = true;
            state.project_detail_saved_commands_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone: ProjectCommandsResult =
                Arc::clone(&state.project_detail_saved_commands_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client.list_project_commands(project_id).await;
                    let mut commands_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *commands_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut commands_result = result_clone.lock().unwrap();
                    *commands_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn create_project_commands(
        &self,
        project_id: i64,
        commands: Vec<shared_types::ProjectCommand>,
        state: &mut AppState,
    ) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.create_commands_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.create_commands_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client
                        .create_project_commands(project_id, commands)
                        .await;
                    let mut create_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *create_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut create_result = result_clone.lock().unwrap();
                    *create_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn execute_project_command(
        &self,
        project_id: i64,
        command_id: &str,
        git_branch: Option<&str>,
        state: &mut AppState,
    ) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.executing_command_id = Some(command_id.to_string());
            state.execute_command_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let result_clone = Arc::clone(&state.execute_command_result);
            let command_id = command_id.to_string();
            let git_branch = git_branch.map(|s| s.to_string());

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client
                        .execute_project_command(project_id, &command_id, git_branch.as_deref())
                        .await;
                    let mut execute_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *execute_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut execute_result = result_clone.lock().unwrap();
                    *execute_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }

    pub fn get_command_executions(&self, project_id: i64, command_id: &str, state: &mut AppState) {
        if state.connection_state == crate::state::ConnectionState::Connected {
            state.loading_command_executions = true;
            state.command_executions_result = Arc::new(std::sync::Mutex::new(None));

            let connection_manager = Arc::clone(&state.connection_manager);
            let command_id = command_id.to_string();
            let result_clone = Arc::clone(&state.command_executions_result);

            tokio::spawn(async move {
                if let Some(api_client_arc) = connection_manager.get_api_client().await {
                    let api_client = api_client_arc.read().await;
                    let result = api_client
                        .get_command_executions(project_id, &command_id, Some(10))
                        .await;
                    let mut executions_result = result_clone.lock().unwrap();

                    // Check for 401 Unauthorized and set auth required flag
                    if let Err(ref e) = result {
                        if e.is_unauthorized() {
                            if let Ok(mut auth_required) =
                                connection_manager.get_auth_required_flag().lock()
                            {
                                *auth_required = true;
                            }
                        }
                    }

                    *executions_result = Some(result.map_err(|e| e.to_string()));
                } else {
                    let mut executions_result = result_clone.lock().unwrap();
                    *executions_result = Some(Err("Not connected".to_string()));
                }
            });
        }
    }
}
